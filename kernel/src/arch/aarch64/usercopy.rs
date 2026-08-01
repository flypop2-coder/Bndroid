use core::arch::asm;
use core::ptr::addr_of;
use core::slice;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::exceptions::TrapFrame;

pub const MAX_USER_COPY_BYTES: usize = bndr_abi::IPC_BUFFER_PAYLOAD_MAX_BYTES;
const LOWER_48_END: u64 = 1_u64 << 48;
const DATA_ABORT_CURRENT_EL: u64 = 0x25;
const ESR_EC_SHIFT: u64 = 26;
const ESR_EC_MASK: u64 = 0x3f;
const ESR_IL: u64 = 1 << 25;
const ESR_ISV: u64 = 1 << 24;
const ESR_SAS_MASK: u64 = 0b11 << 22;
const ESR_SSE: u64 = 1 << 21;
const ESR_SRT_SHIFT: u64 = 16;
const ESR_SRT_MASK: u64 = 0x1f << ESR_SRT_SHIFT;
const ESR_SF: u64 = 1 << 15;
const ESR_AR: u64 = 1 << 14;
const ESR_WNR_SHIFT: u64 = 6;
const ESR_S1PTW: u64 = 1 << 7;
const ESR_CM: u64 = 1 << 8;
const ESR_EA: u64 = 1 << 9;
const ESR_FNV: u64 = 1 << 10;
const ESR_DFSC_MASK: u64 = 0x3f;
const UAO_FEATURE_SHIFT: u64 = 4;
const UAO_FEATURE_MASK: u64 = 0xf;
const PSTATE_UAO_BIT: u64 = 1 << 23;
const NESTED_EXCEPTION_HEADROOM: usize = 4096;
const EXPECTED_EXCEPTION_ENTRIES: usize = 2;

const DIRECTION_NONE: u64 = 0;
const DIRECTION_FROM_USER: u64 = 1;
const DIRECTION_TO_USER: u64 = 2;

#[repr(C)]
struct UserCopyExceptionEntry {
    fault_pc: u64,
    fixup_pc: u64,
    expected_write: u64,
    far_register: u64,
    direction: u64,
}

const _: [(); 40] = [(); core::mem::size_of::<UserCopyExceptionEntry>()];
const _: [(); 8] = [(); core::mem::align_of::<UserCopyExceptionEntry>()];

unsafe extern "C" {
    fn __bndroid_copy_from_user(
        kernel_destination: *mut u8,
        user_source: *const u8,
        bytes: usize,
    ) -> usize;
    fn __bndroid_copy_to_user(
        user_destination: *mut u8,
        kernel_source: *const u8,
        bytes: usize,
    ) -> usize;
    static __usercopy_ex_table_start: u8;
    static __usercopy_ex_table_end: u8;
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserCopyError {
    InvalidRange,
    TooLong,
    Fault { copied: usize },
}

#[derive(Clone, Copy)]
pub struct UserCopySnapshot {
    pub from_calls: u64,
    pub to_calls: u64,
    pub from_bytes: u64,
    pub to_bytes: u64,
    pub from_faults: u64,
    pub to_faults: u64,
    pub range_rejections: u64,
    pub fixups: u64,
    pub misses: u64,
    pub from_esr: u64,
    pub from_far: u64,
    pub to_esr: u64,
    pub to_far: u64,
    pub pan_failures: u64,
    pub uao_failures: u64,
}

struct ActiveCopy {
    direction: u64,
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static UAO_SUPPORTED: AtomicBool = AtomicBool::new(false);
static ACTIVE_DIRECTION: AtomicU64 = AtomicU64::new(DIRECTION_NONE);
static ACTIVE_START: AtomicU64 = AtomicU64::new(0);
static ACTIVE_END: AtomicU64 = AtomicU64::new(0);
static FROM_CALLS: AtomicU64 = AtomicU64::new(0);
static TO_CALLS: AtomicU64 = AtomicU64::new(0);
static FROM_BYTES: AtomicU64 = AtomicU64::new(0);
static TO_BYTES: AtomicU64 = AtomicU64::new(0);
static FROM_FAULTS: AtomicU64 = AtomicU64::new(0);
static TO_FAULTS: AtomicU64 = AtomicU64::new(0);
static RANGE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static FIXUPS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);
static FROM_ESR: AtomicU64 = AtomicU64::new(0);
static FROM_FAR: AtomicU64 = AtomicU64::new(0);
static TO_ESR: AtomicU64 = AtomicU64::new(0);
static TO_FAR: AtomicU64 = AtomicU64::new(0);
static PAN_FAILURES: AtomicU64 = AtomicU64::new(0);
static UAO_FAILURES: AtomicU64 = AtomicU64::new(0);

/// Validates the immutable exception table and establishes UAO policy.
///
/// Must run exactly once at EL1 before any user context becomes runnable.
pub fn init() -> usize {
    if INITIALIZED.load(Ordering::Acquire) {
        panic!("user-copy exception table initialized twice");
    }
    let entries = exception_entries_unchecked();
    if entries.len() != EXPECTED_EXCEPTION_ENTRIES {
        panic!("user-copy exception table has the wrong number of entries");
    }

    let text_start = addr_of!(__text_start) as u64;
    let text_end = addr_of!(__text_end) as u64;
    let table_start = addr_of!(__usercopy_ex_table_start) as usize;
    let table_end = addr_of!(__usercopy_ex_table_end) as usize;
    let rodata_start = addr_of!(__rodata_start) as usize;
    let rodata_end = addr_of!(__rodata_end) as usize;
    if !table_start.is_multiple_of(8)
        || table_start < rodata_start
        || table_end > rodata_end
        || table_start >= table_end
    {
        panic!("user-copy exception table escaped read-only kernel memory");
    }

    let mut saw_from = false;
    let mut saw_to = false;
    for (index, entry) in entries.iter().enumerate() {
        if !entry.fault_pc.is_multiple_of(4)
            || !entry.fixup_pc.is_multiple_of(4)
            || entry.fault_pc < text_start
            || entry.fault_pc >= text_end
            || entry.fixup_pc < text_start
            || entry.fixup_pc >= text_end
            || entry.fault_pc == entry.fixup_pc
            || entries[..index]
                .iter()
                .any(|previous| previous.fault_pc == entry.fault_pc)
        {
            panic!("user-copy exception table contains an invalid PC");
        }
        match entry.direction {
            DIRECTION_FROM_USER
                if entry.expected_write == 0 && entry.far_register == 1 && !saw_from =>
            {
                saw_from = true;
            }
            DIRECTION_TO_USER
                if entry.expected_write == 1 && entry.far_register == 0 && !saw_to =>
            {
                saw_to = true;
            }
            _ => panic!("user-copy exception table metadata is invalid"),
        }
    }
    if !saw_from || !saw_to {
        panic!("user-copy exception table is incomplete");
    }

    let memory_features_2: u64;
    unsafe {
        asm!(
            "mrs {features}, ID_AA64MMFR2_EL1",
            features = out(reg) memory_features_2,
            options(nomem, nostack, preserves_flags)
        );
    }
    let uao_supported = (memory_features_2 >> UAO_FEATURE_SHIFT) & UAO_FEATURE_MASK != 0;
    UAO_SUPPORTED.store(uao_supported, Ordering::Release);
    ensure_uao_disabled();
    reset_counters();
    INITIALIZED.store(true, Ordering::Release);
    entries.len()
}

pub fn copy_from_user(
    kernel_destination: &mut [u8],
    user_source: u64,
) -> Result<usize, UserCopyError> {
    FROM_CALLS.fetch_add(1, Ordering::Relaxed);
    let bytes = kernel_destination.len();
    let user_source = match validate_user_range(user_source, bytes) {
        Ok(address) => address,
        Err(error) => {
            kernel_destination.fill(0);
            RANGE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
    };
    if bytes == 0 {
        return Ok(0);
    }
    assert_copy_context();
    let active = ActiveCopy::enter(DIRECTION_FROM_USER, user_source as u64, bytes);
    let remaining = unsafe {
        __bndroid_copy_from_user(
            kernel_destination.as_mut_ptr(),
            user_source as *const u8,
            bytes,
        )
    };
    drop(active);
    assert_protection_state();
    if remaining > bytes {
        panic!("copy-from-user helper returned an invalid remaining length");
    }
    let copied = bytes - remaining;
    FROM_BYTES.fetch_add(copied as u64, Ordering::Relaxed);
    if remaining != 0 {
        kernel_destination.fill(0);
        FROM_FAULTS.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::Fault { copied });
    }
    Ok(copied)
}

pub fn copy_to_user(user_destination: u64, kernel_source: &[u8]) -> Result<usize, UserCopyError> {
    TO_CALLS.fetch_add(1, Ordering::Relaxed);
    let bytes = kernel_source.len();
    let user_destination = match validate_user_range(user_destination, bytes) {
        Ok(address) => address,
        Err(error) => {
            RANGE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
    };
    if bytes == 0 {
        return Ok(0);
    }
    assert_copy_context();
    let active = ActiveCopy::enter(DIRECTION_TO_USER, user_destination as u64, bytes);
    let remaining = unsafe {
        __bndroid_copy_to_user(user_destination as *mut u8, kernel_source.as_ptr(), bytes)
    };
    drop(active);
    assert_protection_state();
    if remaining > bytes {
        panic!("copy-to-user helper returned an invalid remaining length");
    }
    let copied = bytes - remaining;
    TO_BYTES.fetch_add(copied as u64, Ordering::Relaxed);
    if remaining != 0 {
        TO_FAULTS.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::Fault { copied });
    }
    Ok(copied)
}

/// Attempts an exact exception-table fixup for a nested current-EL abort.
pub(crate) fn try_fixup_data_abort(frame: &mut TrapFrame, esr: u64, far: u64) -> bool {
    if (esr >> ESR_EC_SHIFT) & ESR_EC_MASK != DATA_ABORT_CURRENT_EL {
        return false;
    }
    if !INITIALIZED.load(Ordering::Acquire) {
        record_miss(frame.elr_el1, esr, far);
        return false;
    }
    let entry = exception_entries_unchecked()
        .iter()
        .find(|entry| entry.fault_pc == frame.elr_el1);
    let Some(entry) = entry else {
        record_miss(frame.elr_el1, esr, far);
        return false;
    };

    let direction = ACTIVE_DIRECTION.load(Ordering::Acquire);
    let active_start = ACTIVE_START.load(Ordering::Relaxed);
    let active_end = ACTIVE_END.load(Ordering::Relaxed);
    let expected_far = frame.x[entry.far_register as usize];
    let dfsc = esr & ESR_DFSC_MASK;
    let required_pan = u64::from(crate::arch::aarch64::mmu::pan_supported()) << 22;
    let recoverable = frame.spsr_el1 & 0x1f == 0x5
        && frame.spsr_el1 & (1 << 7) != 0
        && frame.spsr_el1 & (1 << 22) == required_pan
        && frame.spsr_el1 & PSTATE_UAO_BIT == 0
        && current_user_translation_matches()
        && direction == entry.direction
        && (esr >> ESR_WNR_SHIFT) & 1 == entry.expected_write
        && esr & ESR_IL != 0
        && esr & (ESR_S1PTW | ESR_CM | ESR_EA | ESR_FNV) == 0
        && dfsc <= 0xf
        && far == expected_far
        && far >= active_start
        && far < active_end
        && valid_instruction_syndrome(esr);
    if !recoverable {
        record_miss(frame.elr_el1, esr, far);
        return false;
    }

    frame.elr_el1 = entry.fixup_pc;
    FIXUPS.fetch_add(1, Ordering::Relaxed);
    match direction {
        DIRECTION_FROM_USER => {
            FROM_ESR.store(esr, Ordering::Relaxed);
            FROM_FAR.store(far, Ordering::Relaxed);
        }
        DIRECTION_TO_USER => {
            TO_ESR.store(esr, Ordering::Relaxed);
            TO_FAR.store(far, Ordering::Relaxed);
        }
        _ => unreachable!(),
    }
    true
}

pub fn snapshot() -> UserCopySnapshot {
    UserCopySnapshot {
        from_calls: FROM_CALLS.load(Ordering::Acquire),
        to_calls: TO_CALLS.load(Ordering::Acquire),
        from_bytes: FROM_BYTES.load(Ordering::Acquire),
        to_bytes: TO_BYTES.load(Ordering::Acquire),
        from_faults: FROM_FAULTS.load(Ordering::Acquire),
        to_faults: TO_FAULTS.load(Ordering::Acquire),
        range_rejections: RANGE_REJECTIONS.load(Ordering::Acquire),
        fixups: FIXUPS.load(Ordering::Acquire),
        misses: MISSES.load(Ordering::Acquire),
        from_esr: FROM_ESR.load(Ordering::Acquire),
        from_far: FROM_FAR.load(Ordering::Acquire),
        to_esr: TO_ESR.load(Ordering::Acquire),
        to_far: TO_FAR.load(Ordering::Acquire),
        pan_failures: PAN_FAILURES.load(Ordering::Acquire),
        uao_failures: UAO_FAILURES.load(Ordering::Acquire),
    }
}

/// Reports whether this CPU implements FEAT_UAO. Initialization always leaves
/// PSTATE.UAO clear before publishing this capability.
pub fn uao_supported() -> bool {
    UAO_SUPPORTED.load(Ordering::Acquire)
}

impl ActiveCopy {
    fn enter(direction: u64, start: u64, bytes: usize) -> Self {
        let end = start
            .checked_add(bytes as u64)
            .unwrap_or_else(|| panic!("validated user-copy range overflowed"));
        ACTIVE_START.store(start, Ordering::Relaxed);
        ACTIVE_END.store(end, Ordering::Relaxed);
        if ACTIVE_DIRECTION
            .compare_exchange(
                DIRECTION_NONE,
                direction,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            panic!("nested or concurrent user-copy operation");
        }
        Self { direction }
    }
}

impl Drop for ActiveCopy {
    fn drop(&mut self) {
        if ACTIVE_DIRECTION
            .compare_exchange(
                self.direction,
                DIRECTION_NONE,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            panic!("user-copy active direction was corrupted");
        }
        ACTIVE_START.store(0, Ordering::Relaxed);
        ACTIVE_END.store(0, Ordering::Relaxed);
    }
}

fn assert_copy_context() {
    if !INITIALIZED.load(Ordering::Acquire) {
        panic!("user copy attempted before exception-table initialization");
    }
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("user copy attempted with IRQ enabled");
    }
    if !current_user_translation_matches() {
        panic!("user copy attempted outside the current process address space");
    }
    let stack_pointer: usize;
    unsafe {
        asm!(
            "mov {stack_pointer}, sp",
            stack_pointer = out(reg) stack_pointer,
            options(nomem, nostack, preserves_flags)
        );
    }
    crate::scheduler::assert_current_kernel_stack_headroom(
        stack_pointer,
        NESTED_EXCEPTION_HEADROOM,
    );
    ensure_uao_disabled();
    assert_protection_state();
}

fn current_user_translation_matches() -> bool {
    crate::scheduler::current_user_translation().is_some_and(|expected| {
        expected.asid() != crate::arch::aarch64::mmu::KERNEL_ASID
            && crate::arch::aarch64::mmu::current_translation_context() == expected
    })
}

fn assert_protection_state() {
    if crate::arch::aarch64::mmu::pan_supported() && !crate::arch::aarch64::mmu::pan_is_enabled() {
        PAN_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("user copy observed PAN disabled");
    }
    if uao_is_enabled() {
        UAO_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("user copy observed UAO enabled");
    }
}

fn ensure_uao_disabled() {
    if !UAO_SUPPORTED.load(Ordering::Acquire) {
        return;
    }
    unsafe {
        asm!(
            "msr S3_0_C4_C2_4, xzr",
            "isb",
            options(nomem, nostack, preserves_flags)
        );
    }
    if uao_is_enabled() {
        UAO_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("failed to clear PSTATE.UAO before user copy");
    }
}

fn uao_is_enabled() -> bool {
    if !UAO_SUPPORTED.load(Ordering::Acquire) {
        return false;
    }
    let uao: u64;
    unsafe {
        asm!(
            "mrs {uao}, S3_0_C4_C2_4",
            uao = out(reg) uao,
            options(nomem, nostack, preserves_flags)
        );
    }
    uao & PSTATE_UAO_BIT != 0
}

fn validate_user_range(address: u64, bytes: usize) -> Result<usize, UserCopyError> {
    if bytes > MAX_USER_COPY_BYTES {
        return Err(UserCopyError::TooLong);
    }
    if bytes == 0 {
        return usize::try_from(address).map_err(|_| UserCopyError::InvalidRange);
    }
    let last = address
        .checked_add((bytes - 1) as u64)
        .ok_or(UserCopyError::InvalidRange)?;
    if address >= LOWER_48_END || last >= LOWER_48_END {
        return Err(UserCopyError::InvalidRange);
    }
    usize::try_from(address).map_err(|_| UserCopyError::InvalidRange)
}

fn valid_instruction_syndrome(esr: u64) -> bool {
    if esr & ESR_ISV == 0 {
        return true;
    }
    esr & (ESR_SAS_MASK | ESR_SSE | ESR_SF | ESR_AR) == 0
        && (esr & ESR_SRT_MASK) >> ESR_SRT_SHIFT == 3
}

fn exception_entries_unchecked() -> &'static [UserCopyExceptionEntry] {
    let start = addr_of!(__usercopy_ex_table_start) as usize;
    let end = addr_of!(__usercopy_ex_table_end) as usize;
    let bytes = end
        .checked_sub(start)
        .unwrap_or_else(|| panic!("user-copy exception table symbols are reversed"));
    if !bytes.is_multiple_of(core::mem::size_of::<UserCopyExceptionEntry>()) {
        panic!("user-copy exception table has a partial entry");
    }
    unsafe {
        slice::from_raw_parts(
            start as *const UserCopyExceptionEntry,
            bytes / core::mem::size_of::<UserCopyExceptionEntry>(),
        )
    }
}

fn record_miss(_pc: u64, _esr: u64, _far: u64) {
    MISSES.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "vm-unmapped-access-self-test")]
    crate::kprintln!(
        "COPYIO_FIXUP_MISS pc={:#018x} ec={} dfsc={} wnr={} far={:#018x} match=0",
        _pc,
        (_esr >> ESR_EC_SHIFT) & ESR_EC_MASK,
        _esr & ESR_DFSC_MASK,
        (_esr >> ESR_WNR_SHIFT) & 1,
        _far
    );
}

fn reset_counters() {
    FROM_CALLS.store(0, Ordering::Relaxed);
    TO_CALLS.store(0, Ordering::Relaxed);
    FROM_BYTES.store(0, Ordering::Relaxed);
    TO_BYTES.store(0, Ordering::Relaxed);
    FROM_FAULTS.store(0, Ordering::Relaxed);
    TO_FAULTS.store(0, Ordering::Relaxed);
    RANGE_REJECTIONS.store(0, Ordering::Relaxed);
    FIXUPS.store(0, Ordering::Relaxed);
    MISSES.store(0, Ordering::Relaxed);
    FROM_ESR.store(0, Ordering::Relaxed);
    FROM_FAR.store(0, Ordering::Relaxed);
    TO_ESR.store(0, Ordering::Relaxed);
    TO_FAR.store(0, Ordering::Relaxed);
    PAN_FAILURES.store(0, Ordering::Relaxed);
    UAO_FAILURES.store(0, Ordering::Relaxed);
}
