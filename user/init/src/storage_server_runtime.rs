use core::arch::asm;
use core::cell::UnsafeCell;

#[cfg(all(
    feature = "storage-server-shutdown-orchestration-runtime",
    not(feature = "resident-platform-shutdown-runtime")
))]
use bndr_abi::SYSTEM_SHUTDOWN_COMMIT;
use bndr_abi::{
    APP_DATA_VOLUME_SECTORS, AppDataPrincipal, CHANNEL_MESSAGE_MAX_BYTES, ChannelMessageKind,
    IPC_BUFFER_CREATE_FLAGS_NONE, ObjectSignals, PROCESS_KILLED_EXIT_CODE,
    PROCESS_SPAWN_FLAGS_NONE, PROCESS_TERMINATE_FLAGS_NONE, ProcessTerminationReason, Rights,
    STORAGE_BLOCK_MAX_SECTORS, STORAGE_BLOCK_REQUEST_WIRE_SIZE, STORAGE_CONNECT_RIGHTS_MASK,
    STORAGE_SECTOR_SIZE, STORAGE_SESSION_BINDING_WIRE_SIZE, Status as KernelStatus,
    StorageBlockRequest, StorageSessionBinding, SyscallNumber, UserImageId, VMO_READ_MAX_BYTES,
    pack_vmo_read,
};
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
use bndr_abi::{OBJECT_WAIT_TIMEOUT_INFINITE, SYSTEM_SHUTDOWN_FLAGS_NONE, SYSTEM_SHUTDOWN_PREPARE};
use bndr_appdata::{
    AppDataVolume, DirEntry, DurableVolumeIo, EntryKind as VolumeEntryKind, Error as VolumeError,
    IoError, MAX_ENTRIES, PolicyError, PrincipalPolicy, PrincipalQuota,
    ReplaceCondition as VolumeReplaceCondition, Sector,
};
#[cfg(feature = "unified-product-liveness-runtime")]
use bndr_sm::health::{HEALTH_FRAME_SIZE, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind};
use bndr_storage::{
    EntryKind, FRAME_SIZE, Opcode, REQUEST_PAYLOAD_MAX_BYTES, ReplaceCondition, Request, Response,
    ResultKind, Status as ProtocolStatus, VALUE_MAX_BYTES,
};

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
use super::object_wait_many;
use super::{
    INIT_READY_MAGIC, SyscallResult, assert_stale_handle, exit_child, fail, object_wait,
    read_channel_envelope_now, read_scalar_envelope, syscall, transfer_write, write_scalar,
};

pub(super) const SERVER_READY_TAG: u64 = 0x4d35_3553_5252_4459;
pub(super) const SERVER_IDLE_TAG: u64 = 0x4d35_3553_4944_4c45;
const SERVER_RECOVERY_EXIT_CODE: u64 = 0x4d35_3653_5253_5401;
const SERVER_RECOVERY_COMMAND_TAG: u64 = 0x4d35_3652_434d_4421;
const SERVER_RECOVERY_RESULT_TAG: u64 = 0x4d35_3652_4553_554c;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const SERVER_OFFLINE_EXIT_CODE: u64 = 0x4d36_3053_4f46_0001;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const SERVER_OFFLINE_RESULT_TAG: u64 = 0x4d36_3053_4f46_464c;
const CLIENT_COMMAND_TAG: u64 = 0x4d35_3543_434d_4421;
const CLIENT_DONE_TAG: u64 = 0x4d35_3543_444f_4e45;
const CLIENT_EXIT_CODE: u64 = 0x4d35_3543_4f4b_0001;
const M55_STORAGE_READY_PREFIX: u64 = 0x4d35_3547_0000_0000;
const M55_STORAGE_PROOF_PREFIX: u64 = 0x4d35_3550_0000_0000;
#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
const M56_STORAGE_READY_PREFIX: u64 = 0x4d35_3647_0000_0000;
#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
const M56_STORAGE_PROOF: u64 = 0x4d35_3650_0003_0004;
#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
const M57_STORAGE_READY_PREFIX: u64 = 0x4d35_3747_0000_0000;
#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
const M57_STORAGE_PROOF: u64 = 0x4d35_3750_0006_0007;
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
const M58_STORAGE_READY_PREFIX: u64 = 0x4d35_3847_0000_0000;
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
const M58_STORAGE_PROOF: u64 = 0x4d35_3850_0006_0007;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const M60_STORAGE_READY_PREFIX: u64 = 0x4d36_3047_0000_0000;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const M60_STORAGE_PROOF: u64 = 0x4d36_3050_0007_0008;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
const M65_STORAGE_READY_PREFIX: u64 = 0x4d36_3547_0000_0000;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
const M65_STORAGE_PROOF: u64 = 0x4d36_3550_0002_0003;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
pub(super) const SERVER_SHUTDOWN_COMMAND_TAG: u64 = 0x4d36_3553_4855_5444;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
pub(super) const SERVER_SHUTDOWN_ACK_TAG: u64 = 0x4d36_3553_4855_5441;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
pub(super) const SERVER_SHUTDOWN_EXIT_CODE: u64 = 0x4d36_3553_4800_0001;
#[cfg(feature = "unified-product-liveness-runtime")]
const PRODUCT_STORAGE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;

const RECOVERY_STAGE_WRITE: u64 = 1;
const RECOVERY_STAGE_READ: u64 = 2;
const RECOVERY_STAGE_FLUSH: u64 = 3;
const RECOVERY_STAGE_FINAL: u64 = 4;
const RECOVERY_STAGE_PERMANENT_READ: u64 = 5;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const RECOVERY_STAGE_OFFLINE_PROBE: u64 = 6;
#[cfg(feature = "storage-server-owner-liveness-runtime")]
const RECOVERY_STAGE_OWNER_STALL_FLUSH: u64 = 7;
const RECOVERY_RESULT_OUTCOME_UNKNOWN: u64 = 1;
const RECOVERY_RESULT_REQUIRES_RESET: u64 = 2;
const RECOVERY_TEST_LBA: u64 = APP_DATA_VOLUME_SECTORS - 5;
#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
const RECOVERY_NAMESPACE: &str = "m56-recovery";
#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
const RECOVERY_NAMESPACE: &str = "m57-repeated-recovery";
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
const RECOVERY_NAMESPACE: &str = "m58-async-recovery";
#[cfg(feature = "storage-server-fault-policy-runtime")]
const RECOVERY_NAMESPACE: &str = "m60-fault-policy";

const PHASE_ONE: u64 = 1;
const PHASE_TWO: u64 = 2;
const STAGE_ONE: u64 = 1;
const STAGE_TWO: u64 = 2;

const NAMESPACE_PATH: &str = "state";
const STATE_PATH: &str = "state/counter";
const STATE_VALUE_BYTES: usize = 16;

const FAIL_INIT_ABI: u64 = 0x5501;
const FAIL_INIT_AUTHORITY: u64 = 0x5502;
const FAIL_INIT_CHANNEL: u64 = 0x5503;
const FAIL_INIT_SPAWN: u64 = 0x5504;
const FAIL_INIT_PROTOCOL: u64 = 0x5505;
const FAIL_INIT_WAIT: u64 = 0x5506;
const FAIL_INIT_RESTART: u64 = 0x5507;
const FAIL_INIT_PROOF: u64 = 0x5508;
const FAIL_INIT_READY: u64 = 0x5509;
const FAIL_SERVER_ACQUIRE: u64 = 0x5510;
const FAIL_SERVER_LEGACY_ABI: u64 = 0x5511;
const FAIL_SERVER_VOLUME: u64 = 0x5512;
const FAIL_SERVER_ACCEPT: u64 = 0x5513;
const FAIL_SERVER_BINDING: u64 = 0x5514;
const FAIL_SERVER_PROTOCOL: u64 = 0x5515;
const FAIL_SERVER_RESPONSE: u64 = 0x5516;
const FAIL_CLIENT_COMMAND: u64 = 0x5520;
const FAIL_CLIENT_CONNECT: u64 = 0x5521;
const FAIL_CLIENT_AUTHORITY: u64 = 0x5522;
const FAIL_CLIENT_PROTOCOL: u64 = 0x5523;
const FAIL_CLIENT_STATE: u64 = 0x5524;

struct StorageScratch {
    request: [u8; REQUEST_PAYLOAD_MAX_BYTES],
    file: [u8; VALUE_MAX_BYTES],
    entries: [DirEntry; MAX_ENTRIES],
}

impl StorageScratch {
    const fn new() -> Self {
        Self {
            request: [0; REQUEST_PAYLOAD_MAX_BYTES],
            file: [0; VALUE_MAX_BYTES],
            entries: [DirEntry::EMPTY; MAX_ENTRIES],
        }
    }
}

struct SharedScratch(UnsafeCell<StorageScratch>);

// Each ELF has one EL0 thread and a private address space. The cell keeps the
// 11 KiB protocol workspace separate from bndr-appdata's deep recovery stack.
unsafe impl Sync for SharedScratch {}

static SCRATCH: SharedScratch = SharedScratch(UnsafeCell::new(StorageScratch::new()));

fn scratch() -> &'static mut StorageScratch {
    unsafe { &mut *SCRATCH.0.get() }
}

struct VolumeIo {
    handle: u64,
    sectors: u64,
    read_cache_start: u64,
    read_cache_count: usize,
    read_cache: [Sector; STORAGE_BLOCK_MAX_SECTORS],
    pending_write_start: u64,
    pending_write_count: usize,
    pending_writes: [Sector; STORAGE_BLOCK_MAX_SECTORS],
}

impl VolumeIo {
    fn new(handle: u64, sectors: u64) -> Self {
        Self {
            handle,
            sectors,
            read_cache_start: 0,
            read_cache_count: 0,
            read_cache: [[0; STORAGE_SECTOR_SIZE]; STORAGE_BLOCK_MAX_SECTORS],
            pending_write_start: 0,
            pending_write_count: 0,
            pending_writes: [[0; STORAGE_SECTOR_SIZE]; STORAGE_BLOCK_MAX_SECTORS],
        }
    }

    fn execute(
        &mut self,
        request: StorageBlockRequest,
        read_output: Option<&mut [u8]>,
    ) -> Result<(), IoError> {
        let wire = request.encode();
        let token = loop {
            let submitted = syscall(
                SyscallNumber::StorageSubmit,
                self.handle,
                wire.as_ptr() as u64,
                STORAGE_BLOCK_REQUEST_WIRE_SIZE as u64,
            );
            if submitted.status == KernelStatus::Ok.raw()
                && submitted.out1 != 0
                && submitted.out2 == 0
            {
                break submitted.out1;
            }
            if submitted.status != KernelStatus::ShouldWait.raw()
                || submitted.out1 != 0
                || submitted.out2 != 0
                || !wait_for_signal(self.handle, ObjectSignals::WRITABLE)
            {
                return self.fail_io(map_block_error(submitted.status));
            }
        };

        let expected_read_bytes = request.byte_len();
        let destination = match read_output {
            Some(output) if output.len() == expected_read_bytes => output.as_mut_ptr() as u64,
            Some(_) => return Err(IoError::Device),
            None => 0,
        };
        loop {
            let taken = syscall(SyscallNumber::StorageTake, self.handle, token, destination);
            if taken.status == KernelStatus::ShouldWait.raw() {
                if taken.out1 != 0
                    || taken.out2 != 0
                    || !wait_for_signal(self.handle, ObjectSignals::READABLE)
                {
                    return self.fail_io(IoError::Device);
                }
                continue;
            }
            if taken.status != KernelStatus::Ok.raw() {
                return self.fail_io(map_block_error(taken.status));
            }
            let expected = if destination == 0 {
                0
            } else {
                expected_read_bytes as u64
            };
            if taken.out1 != expected || taken.out2 != 0 {
                return self.fail_io(IoError::Device);
            }
            return Ok(());
        }
    }

    fn fail_io(&mut self, error: IoError) -> Result<(), IoError> {
        // No cached read may cross any failed device transaction. More
        // importantly, an uncertain mutation must never leave a coalesced
        // write batch behind for an implicit retry: the replacement
        // StorageServer will reconcile the durable volume in a new epoch.
        self.read_cache_start = 0;
        self.read_cache_count = 0;
        self.read_cache.fill([0; STORAGE_SECTOR_SIZE]);
        if matches!(error, IoError::OutcomeUnknown | IoError::RequiresReset) {
            self.pending_write_start = 0;
            self.pending_write_count = 0;
            self.pending_writes.fill([0; STORAGE_SECTOR_SIZE]);
        }
        Err(error)
    }

    fn flush_pending_writes(&mut self) -> Result<(), IoError> {
        if self.pending_write_count == 0 {
            return Ok(());
        }
        let request = StorageBlockRequest::write_batch(
            self.pending_write_start,
            &self.pending_writes[..self.pending_write_count],
        )
        .map_err(|_| IoError::OutOfBounds)?;
        self.execute(request, None)?;
        self.pending_write_count = 0;
        Ok(())
    }

    fn refill_read_cache(&mut self, lba: u64) -> Result<(), IoError> {
        self.flush_pending_writes()?;
        let remaining = usize::try_from(self.sectors - lba).unwrap_or(usize::MAX);
        let count = remaining.min(STORAGE_BLOCK_MAX_SECTORS);
        let request =
            StorageBlockRequest::read_batch(lba, count).map_err(|_| IoError::OutOfBounds)?;
        // SAFETY: an array of `Sector` values is contiguous, `count` is at
        // most its fixed eight-sector length, and the borrow lasts only for
        // this synchronous request.
        let output = unsafe {
            core::slice::from_raw_parts_mut(
                self.read_cache.as_mut_ptr().cast::<u8>(),
                count * STORAGE_SECTOR_SIZE,
            )
        };
        self.execute(request, Some(output))?;
        self.read_cache_start = lba;
        self.read_cache_count = count;
        Ok(())
    }
}

impl DurableVolumeIo for VolumeIo {
    fn sector_count(&self) -> u64 {
        self.sectors
    }

    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
        if lba >= self.sectors {
            return Err(IoError::OutOfBounds);
        }
        let cached = lba
            .checked_sub(self.read_cache_start)
            .and_then(|offset| usize::try_from(offset).ok())
            .filter(|offset| *offset < self.read_cache_count);
        let index = match cached {
            Some(index) if self.pending_write_count == 0 => index,
            _ => {
                self.refill_read_cache(lba)?;
                0
            }
        };
        *output = self.read_cache[index];
        Ok(())
    }

    fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), IoError> {
        if lba >= self.sectors {
            return Err(IoError::OutOfBounds);
        }
        self.read_cache_count = 0;
        let expected = self
            .pending_write_start
            .checked_add(self.pending_write_count as u64);
        if self.pending_write_count != 0
            && (self.pending_write_count == STORAGE_BLOCK_MAX_SECTORS || expected != Some(lba))
        {
            self.flush_pending_writes()?;
        }
        if self.pending_write_count == 0 {
            self.pending_write_start = lba;
        }
        self.pending_writes[self.pending_write_count] = *input;
        self.pending_write_count += 1;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.flush_pending_writes()?;
        self.execute(StorageBlockRequest::flush(), None)
    }
}

fn map_block_error(status: u64) -> IoError {
    if status == KernelStatus::RequiresReset.raw() {
        IoError::RequiresReset
    } else if status == KernelStatus::OutcomeUnknown.raw() {
        IoError::OutcomeUnknown
    } else if status == KernelStatus::InvalidArgument.raw() {
        IoError::OutOfBounds
    } else {
        // `Unavailable` is deliberately not promoted to RequiresReset. Only
        // the explicit ABI status proves that reset is required.
        IoError::Device
    }
}

#[cfg(not(feature = "storage-server-recovery-runtime"))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    expect_abi(FAIL_INIT_ABI);
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        KernelStatus::PermissionDenied,
        FAIL_INIT_AUTHORITY,
    );
    if system_root != 0 {
        close_handle(system_root, FAIL_INIT_PROTOCOL);
    }

    let server_one = spawn(UserImageId::StorageServer);
    let boot_generation = expect_server_ready(server_one);
    expect_result(
        syscall(
            SyscallNumber::StorageConnect,
            u64::from(Rights::READ.bits()),
            0,
            0,
        ),
        KernelStatus::PermissionDenied,
        FAIL_INIT_AUTHORITY,
    );

    let launcher_stage = run_client(UserImageId::Launcher, PHASE_ONE, 0);
    let app_stage = run_client(UserImageId::App, PHASE_ONE, 0);
    let first_idle_generation = expect_server_idle(server_one);
    validate_phase_one_proof(
        boot_generation,
        first_idle_generation,
        launcher_stage,
        app_stage,
    );
    terminate_server(server_one);

    let server_two = spawn(UserImageId::StorageServer);
    let recovered_generation = expect_server_ready(server_two);
    if recovered_generation != first_idle_generation {
        fail(FAIL_INIT_RESTART);
    }
    let launcher_read_stage = run_client(UserImageId::Launcher, PHASE_TWO, launcher_stage);
    let app_read_stage = run_client(UserImageId::App, PHASE_TWO, app_stage);
    if launcher_read_stage != launcher_stage || app_read_stage != app_stage {
        fail(FAIL_INIT_PROOF);
    }
    let final_generation = expect_server_idle(server_two);
    if final_generation != recovered_generation {
        fail(FAIL_INIT_PROOF);
    }

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        M55_STORAGE_READY_PREFIX | final_generation,
        M55_STORAGE_PROOF_PREFIX | (launcher_stage << 8) | app_stage,
    );
    if ready.status != KernelStatus::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_INIT_READY);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    init_recovery_runtime(
        system_root,
        &[
            (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
            (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        ],
        M56_STORAGE_READY_PREFIX,
        M56_STORAGE_PROOF,
    )
}

#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    init_recovery_runtime(
        system_root,
        &[
            (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
            (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
            (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        ],
        M57_STORAGE_READY_PREFIX,
        M57_STORAGE_PROOF,
    )
}

#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    init_recovery_runtime(
        system_root,
        &[
            (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
            (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
            (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
            (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        ],
        M58_STORAGE_READY_PREFIX,
        M58_STORAGE_PROOF,
    )
}

#[cfg(all(
    feature = "storage-server-shutdown-orchestration-runtime",
    not(feature = "resident-platform-shutdown-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    expect_abi(FAIL_INIT_ABI);
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        KernelStatus::PermissionDenied,
        FAIL_INIT_AUTHORITY,
    );
    if system_root != 0 {
        close_handle(system_root, FAIL_INIT_PROTOCOL);
    }

    let server = spawn(UserImageId::StorageServer);
    let boot_generation = expect_server_ready(server);
    if boot_generation == 0 || boot_generation & !u64::from(u32::MAX) != 0 {
        fail(FAIL_INIT_PROOF);
    }
    expect_result(
        syscall(
            SyscallNumber::SystemShutdown,
            SYSTEM_SHUTDOWN_PREPARE,
            boot_generation,
            SYSTEM_SHUTDOWN_FLAGS_NONE,
        ),
        KernelStatus::InvalidState,
        FAIL_INIT_AUTHORITY,
    );

    let launcher_stage = run_client(UserImageId::Launcher, PHASE_ONE, 0);
    let app_stage = run_client(UserImageId::App, PHASE_ONE, 0);
    let final_generation = expect_server_idle(server);
    if final_generation == 0 || final_generation & !u64::from(u32::MAX) != 0 {
        fail(FAIL_INIT_PROOF);
    }
    validate_phase_one_proof(boot_generation, final_generation, launcher_stage, app_stage);

    let prepared = syscall(
        SyscallNumber::SystemShutdown,
        SYSTEM_SHUTDOWN_PREPARE,
        final_generation,
        SYSTEM_SHUTDOWN_FLAGS_NONE,
    );
    if prepared.status != KernelStatus::Ok.raw()
        || prepared.out1 != final_generation
        || prepared.out2 != 0
    {
        fail(FAIL_INIT_READY);
    }
    exercise_shutdown_spawn_barrier();
    write_scalar(
        server.control,
        SERVER_SHUTDOWN_COMMAND_TAG,
        final_generation,
    );
    let (sender, (tag, acknowledged_generation)) = read_scalar_envelope(server.control);
    if sender != server.pid
        || tag != SERVER_SHUTDOWN_ACK_TAG
        || acknowledged_generation != final_generation
    {
        fail(FAIL_INIT_PROOF);
    }
    let waited = syscall(SyscallNumber::ProcessWait, server.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != SERVER_SHUTDOWN_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_INIT_WAIT);
    }
    close_handle(server.control, FAIL_INIT_PROTOCOL);

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        M65_STORAGE_READY_PREFIX | final_generation,
        M65_STORAGE_PROOF,
    );
    if ready.status != KernelStatus::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_INIT_READY);
    }
    let committed = syscall(
        SyscallNumber::SystemShutdown,
        SYSTEM_SHUTDOWN_COMMIT,
        final_generation,
        SYSTEM_SHUTDOWN_FLAGS_NONE,
    );
    if committed.status != KernelStatus::Ok.raw()
        || committed.out1 != final_generation
        || committed.out2 != 0
    {
        fail(FAIL_INIT_READY);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
fn exercise_shutdown_spawn_barrier() {
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != KernelStatus::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_INIT_CHANNEL);
    }
    expect_result(
        syscall(
            SyscallNumber::ProcessSpawn,
            channel.out2,
            UserImageId::Launcher.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        ),
        KernelStatus::InvalidState,
        FAIL_INIT_AUTHORITY,
    );
    close_handle(channel.out1, FAIL_INIT_PROTOCOL);
    close_handle(channel.out2, FAIL_INIT_PROTOCOL);
}

#[cfg(all(
    feature = "storage-server-fault-policy-runtime",
    not(feature = "storage-server-shutdown-orchestration-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {
    expect_abi(FAIL_INIT_ABI);
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        KernelStatus::PermissionDenied,
        FAIL_INIT_AUTHORITY,
    );
    if system_root != 0 {
        close_handle(system_root, FAIL_INIT_PROTOCOL);
    }

    #[cfg(not(feature = "storage-server-owner-liveness-runtime"))]
    let sequence = [
        (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
        (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
        (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
    ];
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    let sequence = [
        (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
        (RECOVERY_STAGE_FLUSH, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_WRITE, RECOVERY_RESULT_OUTCOME_UNKNOWN),
        (RECOVERY_STAGE_READ, RECOVERY_RESULT_REQUIRES_RESET),
        (
            RECOVERY_STAGE_OWNER_STALL_FLUSH,
            RECOVERY_RESULT_OUTCOME_UNKNOWN,
        ),
    ];
    let mut generation = 0;
    for (stage, expected) in sequence {
        let server = spawn(UserImageId::StorageServer);
        // M60 sends the private orchestration command before mount. The first
        // six owners still mount before executing WRFWRF. The later permanent
        // stage selects only an ordinary-read workload; the kernel has already
        // armed that campaign before epoch seven becomes acquirable.
        write_scalar(server.control, SERVER_RECOVERY_COMMAND_TAG, stage);
        let mounted = expect_server_ready(server);
        if generation != 0 && mounted != generation {
            fail(FAIL_INIT_RESTART);
        }
        generation = mounted;
        expect_recovery_result(server, stage, expected);
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        if stage == RECOVERY_STAGE_OWNER_STALL_FLUSH {
            wait_owner_liveness_retirement(server);
        } else {
            wait_recovery_server_exit(server);
        }
        #[cfg(not(feature = "storage-server-owner-liveness-runtime"))]
        wait_recovery_server_exit(server);
    }

    let permanent = spawn(UserImageId::StorageServer);
    if generation == 0 {
        fail(FAIL_INIT_RESTART);
    }
    write_scalar(
        permanent.control,
        SERVER_RECOVERY_COMMAND_TAG,
        RECOVERY_STAGE_PERMANENT_READ,
    );
    expect_recovery_result(
        permanent,
        RECOVERY_STAGE_PERMANENT_READ,
        RECOVERY_RESULT_REQUIRES_RESET,
    );
    wait_recovery_server_exit(permanent);

    // This eighth StorageServer has no recovery authority. During bounded
    // backoff it observes ShouldWait and sleeps; after the kernel makes the
    // boot-local Offline transition it receives one terminal Unavailable.
    let probe = spawn(UserImageId::StorageServer);
    write_scalar(
        probe.control,
        SERVER_RECOVERY_COMMAND_TAG,
        RECOVERY_STAGE_OFFLINE_PROBE,
    );
    let (sender, (tag, result)) = read_scalar_envelope(probe.control);
    if sender != probe.pid || tag != SERVER_OFFLINE_RESULT_TAG || result != 0 {
        fail(FAIL_INIT_PROOF);
    }
    let waited = syscall(SyscallNumber::ProcessWait, probe.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != SERVER_OFFLINE_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_INIT_WAIT);
    }
    close_handle(probe.control, FAIL_INIT_PROTOCOL);

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        M60_STORAGE_READY_PREFIX | generation,
        M60_STORAGE_PROOF,
    );
    if ready.status != KernelStatus::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_INIT_READY);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn init_recovery_runtime(
    system_root: u64,
    sequence: &[(u64, u64)],
    ready_prefix: u64,
    proof: u64,
) -> ! {
    expect_abi(FAIL_INIT_ABI);
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        KernelStatus::PermissionDenied,
        FAIL_INIT_AUTHORITY,
    );
    if system_root != 0 {
        close_handle(system_root, FAIL_INIT_PROTOCOL);
    }

    let mut generation = 0;
    for &(stage, expected) in sequence {
        let server = spawn(UserImageId::StorageServer);
        let mounted = expect_server_ready(server);
        if generation != 0 && mounted != generation {
            fail(FAIL_INIT_RESTART);
        }
        generation = mounted;
        write_scalar(server.control, SERVER_RECOVERY_COMMAND_TAG, stage);
        expect_recovery_result(server, stage, expected);
        wait_recovery_server_exit(server);
    }

    let final_server = spawn(UserImageId::StorageServer);
    let final_generation = expect_server_ready(final_server);
    if final_generation == 0 || final_generation != generation {
        fail(FAIL_INIT_RESTART);
    }
    write_scalar(
        final_server.control,
        SERVER_RECOVERY_COMMAND_TAG,
        RECOVERY_STAGE_FINAL,
    );
    expect_recovery_result(final_server, RECOVERY_STAGE_FINAL, 0);

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        ready_prefix | final_generation,
        proof,
    );
    if ready.status != KernelStatus::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_INIT_READY);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn expect_recovery_result(server: Child, stage: u64, expected: u64) {
    let (sender, (tag, result)) = read_scalar_envelope(server.control);
    if sender != server.pid
        || tag != SERVER_RECOVERY_RESULT_TAG
        || result != (stage << 32) | expected
    {
        fail(FAIL_INIT_PROOF);
    }
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn wait_recovery_server_exit(server: Child) {
    let waited = syscall(SyscallNumber::ProcessWait, server.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != SERVER_RECOVERY_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_INIT_WAIT);
    }
    close_handle(server.control, FAIL_INIT_PROTOCOL);
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn wait_owner_liveness_retirement(server: Child) {
    // Init supplies no ProcessTerminate authority. The server is blocked on
    // its control Channel after publishing the fatal flush result; only the
    // kernel-owned physical-counter ticket may make it terminal.
    let waited = syscall(SyscallNumber::ProcessWait, server.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(FAIL_INIT_WAIT);
    }
    close_handle(server.control, FAIL_INIT_PROTOCOL);
}

fn validate_phase_one_proof(before: u64, after: u64, launcher_stage: u64, app_stage: u64) {
    let Some(delta) = after.checked_sub(before) else {
        fail(FAIL_INIT_PROOF);
    };
    let valid = launcher_stage == STAGE_TWO
        && match delta {
            // Virgin boot: two namespace directories and two files.
            4 => app_stage == STAGE_ONE,
            // Second boot: the App alone advances stage one to stage two.
            1 => app_stage == STAGE_TWO,
            // Stable third and later boots: no mutation is permitted.
            0 => app_stage == STAGE_TWO,
            _ => false,
        };
    if !valid {
        fail(FAIL_INIT_PROOF);
    }
}

#[derive(Clone, Copy)]
struct Child {
    control: u64,
    pid: u64,
}

fn spawn(image: UserImageId) -> Child {
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != KernelStatus::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_INIT_CHANNEL);
    }
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        channel.out2,
        image.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != KernelStatus::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        close_handle(channel.out1, FAIL_INIT_PROTOCOL);
        close_handle(channel.out2, FAIL_INIT_PROTOCOL);
        fail(FAIL_INIT_SPAWN);
    }
    assert_stale_handle(channel.out2);
    Child {
        control: channel.out1,
        pid: spawned.out1,
    }
}

fn expect_server_ready(server: Child) -> u64 {
    let (sender, (tag, generation)) = read_scalar_envelope(server.control);
    if sender != server.pid || tag != SERVER_READY_TAG {
        fail(FAIL_INIT_PROTOCOL);
    }
    generation
}

fn expect_server_idle(server: Child) -> u64 {
    let (sender, (tag, generation)) = read_scalar_envelope(server.control);
    if sender != server.pid || tag != SERVER_IDLE_TAG {
        fail(FAIL_INIT_PROTOCOL);
    }
    generation
}

fn run_client(image: UserImageId, phase: u64, expected_stage: u64) -> u64 {
    let child = spawn(image);
    write_scalar(
        child.control,
        CLIENT_COMMAND_TAG,
        pack_client_command(phase, expected_stage),
    );
    let (sender, (tag, proof)) = read_scalar_envelope(child.control);
    let expected_principal = match image {
        UserImageId::Launcher => AppDataPrincipal::LAUNCHER.raw(),
        UserImageId::App => AppDataPrincipal::PRIMARY_APP.raw(),
        _ => fail(FAIL_INIT_PROTOCOL),
    };
    let (proof_phase, proof_principal, stage) = unpack_client_proof(proof);
    if sender != child.pid
        || tag != CLIENT_DONE_TAG
        || proof_phase != phase
        || proof_principal != expected_principal
        || !matches!(stage, STAGE_ONE | STAGE_TWO)
    {
        fail(FAIL_INIT_PROOF);
    }
    let waited = syscall(SyscallNumber::ProcessWait, child.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != CLIENT_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_INIT_WAIT);
    }
    close_handle(child.control, FAIL_INIT_PROTOCOL);
    stage
}

fn terminate_server(server: Child) {
    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        server.pid,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if terminated.status != KernelStatus::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
        fail(FAIL_INIT_RESTART);
    }
    let waited = syscall(SyscallNumber::ProcessWait, server.pid, 0, 0);
    if waited.status != KernelStatus::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(FAIL_INIT_RESTART);
    }
    close_handle(server.control, FAIL_INIT_PROTOCOL);
}

pub(super) fn server_runtime(startup: u64) -> ! {
    expect_abi(FAIL_SERVER_ACQUIRE);
    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-shutdown-orchestration-runtime")
    ))]
    let recovery_stage = read_recovery_stage_command(startup);
    let acquired = acquire_storage_volume();
    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-shutdown-orchestration-runtime")
    ))]
    if acquired.status == KernelStatus::Unavailable.raw()
        && acquired.out1 == 0
        && acquired.out2 == 0
    {
        if recovery_stage != RECOVERY_STAGE_OFFLINE_PROBE {
            fail(FAIL_SERVER_ACQUIRE);
        }
        write_scalar(startup, SERVER_OFFLINE_RESULT_TAG, 0);
        exit_child(SERVER_OFFLINE_EXIT_CODE);
    }
    if acquired.status != KernelStatus::Ok.raw()
        || acquired.out1 == 0
        || acquired.out2 != APP_DATA_VOLUME_SECTORS
    {
        fail(FAIL_SERVER_ACQUIRE);
    }
    let mut io = VolumeIo::new(acquired.out1, acquired.out2);
    expect_legacy_appdata_unsupported();
    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-shutdown-orchestration-runtime")
    ))]
    {
        if recovery_stage == RECOVERY_STAGE_OFFLINE_PROBE {
            fail(FAIL_SERVER_ACQUIRE);
        }
        if recovery_stage == RECOVERY_STAGE_PERMANENT_READ {
            run_m60_permanent_read(startup, &mut io);
        }
    }
    let volume = AppDataVolume::new(0);
    let policy = storage_policy();
    let recovery = match volume.recover_with_policy(&mut io, &policy) {
        Ok(recovery) => recovery,
        Err(PolicyError::Volume(VolumeError::Unformatted)) => {
            volume
                .format_virgin(&mut io)
                .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
            volume
                .recover_with_policy(&mut io, &policy)
                .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME))
        }
        Err(_) => fail(FAIL_SERVER_VOLUME),
    };
    #[cfg(feature = "storage-server-recovery-runtime")]
    let mut mounted_generation = recovery.generation;
    #[cfg(not(feature = "storage-server-recovery-runtime"))]
    let mounted_generation = recovery.generation;
    #[cfg(feature = "storage-server-recovery-runtime")]
    if mounted_generation == 0 {
        let mutation = volume
            .create_dir_with_policy(
                &mut io,
                &policy,
                AppDataPrincipal::PRIMARY_APP.raw(),
                RECOVERY_NAMESPACE,
            )
            .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
        mounted_generation = mutation.committed_generation;
        let verified = volume
            .recover_with_policy(&mut io, &policy)
            .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
        if mounted_generation == 0 || verified.generation != mounted_generation {
            fail(FAIL_SERVER_VOLUME);
        }
    }
    write_scalar(startup, SERVER_READY_TAG, mounted_generation);

    #[cfg(all(
        feature = "storage-server-recovery-runtime",
        not(feature = "storage-server-fault-policy-runtime")
    ))]
    run_recovery_stage(startup, &mut io);
    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-shutdown-orchestration-runtime")
    ))]
    run_mounted_recovery_stage(startup, &mut io, recovery_stage);

    let mut current_generation = mounted_generation;
    let mut epoch = 0_u64;
    let mut last_session_id = 0_u64;
    let mut principal_mask = 0_u8;
    #[cfg(feature = "unified-product-liveness-runtime")]
    let mut product_health = ProductStorageHealth::new();
    loop {
        #[cfg(all(
            feature = "storage-server-shutdown-orchestration-runtime",
            feature = "unified-product-liveness-runtime"
        ))]
        let work = accept_session_or_shutdown(io.handle, startup, &mut product_health);
        #[cfg(all(
            feature = "storage-server-shutdown-orchestration-runtime",
            not(feature = "unified-product-liveness-runtime")
        ))]
        let work = accept_session_or_shutdown(io.handle, startup);
        #[cfg(not(feature = "storage-server-shutdown-orchestration-runtime"))]
        let work = ServerWork::Session(accept_session(io.handle));
        let (session, binding) = match work {
            ServerWork::Session(session) => session,
            #[cfg(feature = "unified-product-liveness-runtime")]
            ServerWork::HealthProbe => continue,
            #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
            ServerWork::Shutdown(generation) => {
                if generation != current_generation {
                    fail(FAIL_SERVER_PROTOCOL);
                }
                io.flush().unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
                let verified = volume
                    .recover_with_policy(&mut io, &policy)
                    .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
                if verified.generation != current_generation {
                    fail(FAIL_SERVER_VOLUME);
                }
                write_scalar(startup, SERVER_SHUTDOWN_ACK_TAG, current_generation);
                exit_child(SERVER_SHUTDOWN_EXIT_CODE);
            }
        };
        if binding.server_epoch() == 0
            || (epoch != 0 && binding.server_epoch() != epoch)
            || binding.session_id() <= last_session_id
            || !matches!(
                binding.principal(),
                AppDataPrincipal::PRIMARY_APP | AppDataPrincipal::LAUNCHER
            )
        {
            close_handle(session, FAIL_SERVER_BINDING);
            fail(FAIL_SERVER_BINDING);
        }
        epoch = binding.server_epoch();
        last_session_id = binding.session_id();
        let outcome = serve_session(
            session,
            binding,
            volume,
            &policy,
            &mut io,
            &mut current_generation,
        );
        close_handle(session, FAIL_SERVER_PROTOCOL);
        if outcome.fatal_storage {
            // Keep the volume capability live until process teardown so the
            // kernel can atomically revoke this owner, retain the poisoned
            // broker state, and reset only after every accepted endpoint is
            // closed by the handle-table teardown.
            exit_child(SERVER_RECOVERY_EXIT_CODE);
        }
        if outcome.bound {
            principal_mask |= principal_bit(binding.principal());
        }
        if principal_mask == 0b11 {
            write_scalar(startup, SERVER_IDLE_TAG, current_generation);
            principal_mask = 0;
        }
    }
}

#[cfg(not(feature = "storage-server-recovery-runtime"))]
fn acquire_storage_volume() -> SyscallResult {
    syscall(SyscallNumber::StorageAcquire, 0, 0, 0)
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn acquire_storage_volume() -> SyscallResult {
    loop {
        let acquired = syscall(SyscallNumber::StorageAcquire, 0, 0, 0);
        if acquired.status == KernelStatus::ShouldWait.raw()
            && acquired.out1 == 0
            && acquired.out2 == 0
        {
            // The kernel monitor owns recovery and will make the ownerless
            // broker acquirable only after reset, queue rebuild, and IRQ
            // commit. This retry grants EL0 no reset authority.
            unsafe {
                asm!("wfi", options(nomem, nostack, preserves_flags));
            }
            continue;
        }
        return acquired;
    }
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn read_recovery_stage_command(startup: u64) -> u64 {
    let (sender, (tag, stage)) = read_scalar_envelope(startup);
    let historical_stage_valid = matches!(
        stage,
        RECOVERY_STAGE_WRITE | RECOVERY_STAGE_READ | RECOVERY_STAGE_FLUSH | RECOVERY_STAGE_FINAL
    );
    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-owner-liveness-runtime")
    ))]
    let stage_valid = historical_stage_valid
        || matches!(
            stage,
            RECOVERY_STAGE_PERMANENT_READ | RECOVERY_STAGE_OFFLINE_PROBE
        );
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    let stage_valid = historical_stage_valid
        || matches!(
            stage,
            RECOVERY_STAGE_PERMANENT_READ
                | RECOVERY_STAGE_OFFLINE_PROBE
                | RECOVERY_STAGE_OWNER_STALL_FLUSH
        );
    #[cfg(not(feature = "storage-server-fault-policy-runtime"))]
    let stage_valid = historical_stage_valid;
    if sender == 0 || tag != SERVER_RECOVERY_COMMAND_TAG || !stage_valid {
        fail(FAIL_SERVER_PROTOCOL);
    }
    stage
}

#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
fn run_recovery_stage(startup: u64, io: &mut VolumeIo) {
    let stage = read_recovery_stage_command(startup);
    run_mounted_recovery_stage(startup, io, stage);
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn run_mounted_recovery_stage(startup: u64, io: &mut VolumeIo, stage: u64) {
    const CONTROL_PREFIX_LBA: u64 = APP_DATA_VOLUME_SECTORS - 1;
    const CONTROL_WRITE_LBA: u64 = APP_DATA_VOLUME_SECTORS - 2;
    const CONTROL_READ_LBA: u64 = APP_DATA_VOLUME_SECTORS - 3;
    const CONTROL_FLUSH_LBA: u64 = APP_DATA_VOLUME_SECTORS - 4;

    let historical_stage_valid = matches!(
        stage,
        RECOVERY_STAGE_WRITE | RECOVERY_STAGE_READ | RECOVERY_STAGE_FLUSH | RECOVERY_STAGE_FINAL
    );
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    let stage_valid = historical_stage_valid || stage == RECOVERY_STAGE_OWNER_STALL_FLUSH;
    #[cfg(not(feature = "storage-server-owner-liveness-runtime"))]
    let stage_valid = historical_stage_valid;
    if !stage_valid {
        fail(FAIL_SERVER_PROTOCOL);
    }

    let sector =
        recovery_raw_read(io, RECOVERY_TEST_LBA).unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
    if sector.iter().any(|byte| *byte != 0) {
        fail(FAIL_SERVER_VOLUME);
    }

    let result = match stage {
        RECOVERY_STAGE_WRITE => {
            recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_WRITE_LBA);
            match io.execute(
                StorageBlockRequest::write(RECOVERY_TEST_LBA, [0; STORAGE_SECTOR_SIZE])
                    .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL)),
                None,
            ) {
                Err(IoError::OutcomeUnknown) => RECOVERY_RESULT_OUTCOME_UNKNOWN,
                _ => fail(FAIL_SERVER_VOLUME),
            }
        }
        RECOVERY_STAGE_READ => {
            recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_READ_LBA);
            match recovery_raw_read(io, RECOVERY_TEST_LBA) {
                Err(IoError::RequiresReset) => RECOVERY_RESULT_REQUIRES_RESET,
                _ => fail(FAIL_SERVER_VOLUME),
            }
        }
        RECOVERY_STAGE_FLUSH => {
            recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_FLUSH_LBA);
            match io.execute(StorageBlockRequest::flush(), None) {
                Err(IoError::OutcomeUnknown) => RECOVERY_RESULT_OUTCOME_UNKNOWN,
                _ => fail(FAIL_SERVER_VOLUME),
            }
        }
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        RECOVERY_STAGE_OWNER_STALL_FLUSH => {
            recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_FLUSH_LBA);
            match io.execute(StorageBlockRequest::flush(), None) {
                Err(IoError::OutcomeUnknown) => RECOVERY_RESULT_OUTCOME_UNKNOWN,
                _ => fail(FAIL_SERVER_VOLUME),
            }
        }
        RECOVERY_STAGE_FINAL => {
            io.execute(
                StorageBlockRequest::write(RECOVERY_TEST_LBA, [0; STORAGE_SECTOR_SIZE])
                    .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL)),
                None,
            )
            .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
            io.execute(StorageBlockRequest::flush(), None)
                .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
            0
        }
        _ => unreachable!(),
    };
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    if stage == RECOVERY_STAGE_OWNER_STALL_FLUSH {
        // A live owner cannot turn broker unbinding into a retirement proof.
        // The process-lifetime volume capability remains valid until the
        // monitor kills this exact generation and the ordinary reaper runs.
        let close = syscall(SyscallNumber::HandleClose, io.handle, 0, 0);
        if close.status != KernelStatus::InvalidState.raw() || close.out1 != 0 || close.out2 != 0 {
            fail(FAIL_SERVER_PROTOCOL);
        }
    }
    write_scalar(startup, SERVER_RECOVERY_RESULT_TAG, (stage << 32) | result);
    if stage != RECOVERY_STAGE_FINAL {
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        if stage == RECOVERY_STAGE_OWNER_STALL_FLUSH {
            let waited = object_wait(startup, ObjectSignals::READABLE);
            let _ = waited;
            fail(FAIL_SERVER_PROTOCOL);
        }
        exit_child(SERVER_RECOVERY_EXIT_CODE);
    }
}

/// The seventh M60 owner deliberately does not mount AppData. Its first block
/// request is an ordinary single-sector read. The kernel recovery coordinator
/// has already armed the sealed campaign before this owner can acquire the
/// broker; this EL0 stage selects workload only and cannot arm a fault. There
/// is no permanent selector LBA or driver-control syscall.
#[cfg(feature = "storage-server-fault-policy-runtime")]
fn run_m60_permanent_read(startup: u64, io: &mut VolumeIo) -> ! {
    let result = match recovery_raw_read(io, RECOVERY_TEST_LBA) {
        Err(IoError::RequiresReset) => RECOVERY_RESULT_REQUIRES_RESET,
        _ => fail(FAIL_SERVER_VOLUME),
    };
    write_scalar(
        startup,
        SERVER_RECOVERY_RESULT_TAG,
        (RECOVERY_STAGE_PERMANENT_READ << 32) | result,
    );
    exit_child(SERVER_RECOVERY_EXIT_CODE);
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn recovery_fault_control(io: &mut VolumeIo, prefix_lba: u64, selector_lba: u64) {
    recovery_raw_read(io, prefix_lba).unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
    recovery_raw_read(io, selector_lba).unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn recovery_raw_read(io: &mut VolumeIo, lba: u64) -> Result<Sector, IoError> {
    let mut sector = [0; STORAGE_SECTOR_SIZE];
    io.execute(
        StorageBlockRequest::read(lba).map_err(|_| IoError::OutOfBounds)?,
        Some(&mut sector),
    )?;
    Ok(sector)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SessionOutcome {
    bound: bool,
    fatal_storage: bool,
}

impl SessionOutcome {
    const fn closed(bound: bool) -> Self {
        Self {
            bound,
            fatal_storage: false,
        }
    }

    const fn fatal(bound: bool) -> Self {
        Self {
            bound,
            fatal_storage: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DispatchOutcome {
    Continue,
    SessionClosed,
    FatalStorage,
}

fn expect_legacy_appdata_unsupported() {
    for number in [
        SyscallNumber::AppDataRootOpen,
        SyscallNumber::FileReplaceAt,
        SyscallNumber::DirectoryCreateAt,
        SyscallNumber::UnlinkAt,
        SyscallNumber::DirectoryReadAt,
    ] {
        expect_result(
            syscall(number, 0, 0, 0),
            KernelStatus::Unsupported,
            FAIL_SERVER_LEGACY_ABI,
        );
    }
}

enum ServerWork {
    Session((u64, StorageSessionBinding)),
    #[cfg(feature = "unified-product-liveness-runtime")]
    HealthProbe,
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    Shutdown(u64),
}

#[cfg(feature = "unified-product-liveness-runtime")]
#[derive(Clone, Copy)]
struct ProductStorageHealth {
    init_pid: u64,
    identity: Option<ServiceIdentity>,
    probes: u64,
}

#[cfg(feature = "unified-product-liveness-runtime")]
impl ProductStorageHealth {
    const fn new() -> Self {
        Self {
            init_pid: 0,
            identity: None,
            probes: 0,
        }
    }
}

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
fn accept_session_or_shutdown(
    volume_handle: u64,
    startup: u64,
    #[cfg(feature = "unified-product-liveness-runtime")] product_health: &mut ProductStorageHealth,
) -> ServerWork {
    let signals = ObjectSignals::from_bits(
        ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits(),
    )
    .unwrap_or_else(|| fail(FAIL_SERVER_PROTOCOL));
    let ready = object_wait_many(
        startup,
        signals,
        volume_handle,
        signals,
        OBJECT_WAIT_TIMEOUT_INFINITE,
    );
    if ready.status != KernelStatus::Ok.raw()
        || ready.out1 > 1
        || ready.out2 & u64::from(signals.bits()) == 0
    {
        fail(FAIL_SERVER_PROTOCOL);
    }
    if ready.out1 == 0 {
        if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            fail(FAIL_SERVER_PROTOCOL);
        }
        let envelope = read_channel_envelope_now(startup);
        #[cfg(feature = "unified-product-liveness-runtime")]
        if envelope.kind() == ChannelMessageKind::Bytes {
            process_product_health_probe(startup, &envelope, product_health);
            return ServerWork::HealthProbe;
        }
        let Some((tag, generation)) = envelope.scalar_values() else {
            close_received_handle(&envelope);
            fail(FAIL_SERVER_PROTOCOL);
        };
        if envelope.kind() != ChannelMessageKind::Scalar
            || envelope.sender_pid() == 0
            || envelope.received_handle().is_valid()
            || tag != SERVER_SHUTDOWN_COMMAND_TAG
            || generation == 0
        {
            close_received_handle(&envelope);
            fail(FAIL_SERVER_PROTOCOL);
        }
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        let health_profile_valid = product_health.probes >= 2;
        #[cfg(all(
            feature = "unified-product-continuous-supervision-runtime",
            not(feature = "unified-product-event-supervision-runtime")
        ))]
        let expected_health_probes = 20;
        #[cfg(all(
            feature = "unified-product-multiservice-liveness-runtime",
            not(feature = "unified-product-continuous-supervision-runtime")
        ))]
        let expected_health_probes = 2;
        #[cfg(all(
            feature = "unified-product-liveness-runtime",
            not(feature = "unified-product-multiservice-liveness-runtime")
        ))]
        let expected_health_probes = 1;
        #[cfg(all(
            feature = "unified-product-liveness-runtime",
            not(feature = "unified-product-event-supervision-runtime")
        ))]
        let health_profile_valid = product_health.probes == expected_health_probes;
        #[cfg(feature = "unified-product-liveness-runtime")]
        if !health_profile_valid
            || product_health.init_pid != envelope.sender_pid()
            || product_health.identity.is_none()
        {
            fail(FAIL_SERVER_PROTOCOL);
        }
        ServerWork::Shutdown(generation)
    } else {
        if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            fail(FAIL_SERVER_PROTOCOL);
        }
        ServerWork::Session(accept_session(volume_handle))
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn process_product_health_probe(
    startup: u64,
    envelope: &bndr_abi::ChannelReadEnvelope,
    health: &mut ProductStorageHealth,
) {
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != HEALTH_FRAME_SIZE
        || envelope.sender_pid() == 0
        || envelope.received_handle().is_valid()
    {
        close_received_handle(envelope);
        fail(FAIL_SERVER_PROTOCOL);
    }
    let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
    let identity = probe.identity();
    if probe.opcode() != HealthOpcode::Probe
        || probe.interval_ns() != PRODUCT_STORAGE_HEALTH_TIMEOUT_NS
        || !probe.flags().ack_required()
        || identity.kind() != ServiceKind::StorageServer
        || identity.generation() != process_generation(identity.pid())
        || process_slot(identity.pid()) == 0
    {
        fail(FAIL_SERVER_PROTOCOL);
    }

    #[cfg(feature = "unified-product-event-supervision-runtime")]
    {
        let expected_sequence = health
            .probes
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_SERVER_PROTOCOL));
        if probe.sequence() != expected_sequence {
            fail(FAIL_SERVER_PROTOCOL);
        }
        if health.probes == 0 {
            if health.init_pid != 0 || health.identity.is_some() {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.init_pid = envelope.sender_pid();
            health.identity = Some(identity);
        } else if health.init_pid != envelope.sender_pid() || health.identity != Some(identity) {
            fail(FAIL_SERVER_PROTOCOL);
        }
        health.probes = expected_sequence;
        let healthy = HealthFrame::healthy(expected_sequence, identity)
            .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
        write_product_health(startup, healthy);
    }

    #[cfg(all(
        feature = "unified-product-continuous-supervision-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    match health.probes {
        0 | 1 => {
            let expected_sequence = u64::from(health.probes) + 1;
            if probe.sequence() != expected_sequence {
                fail(FAIL_SERVER_PROTOCOL);
            }
            if health.probes == 0 {
                if health.init_pid != 0 || health.identity.is_some() {
                    fail(FAIL_SERVER_PROTOCOL);
                }
                health.init_pid = envelope.sender_pid();
                health.identity = Some(identity);
            } else if health.init_pid != envelope.sender_pid() || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes += 1;
            let healthy = HealthFrame::healthy(expected_sequence, identity)
                .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
            write_product_health(startup, healthy);
        }
        2 if identity.generation() == 1 => {
            if probe.sequence() != 3
                || health.init_pid != envelope.sender_pid()
                || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes = 3;
            // Preserve M69's real initial StorageServer failure: generation one
            // consumes its third probe and withholds Healthy.
        }
        2..=19 => {
            let expected_sequence = u64::from(health.probes) + 1;
            if identity.generation() < 2
                || probe.sequence() != expected_sequence
                || health.init_pid != envelope.sender_pid()
                || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes += 1;
            let healthy = HealthFrame::healthy(expected_sequence, identity)
                .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
            write_product_health(startup, healthy);
        }
        _ => fail(FAIL_SERVER_PROTOCOL),
    }

    #[cfg(all(
        feature = "unified-product-multiservice-liveness-runtime",
        not(feature = "unified-product-continuous-supervision-runtime"),
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    match health.probes {
        0 | 1 => {
            let expected_sequence = u64::from(health.probes) + 1;
            if probe.sequence() != expected_sequence {
                fail(FAIL_SERVER_PROTOCOL);
            }
            if health.probes == 0 {
                if health.init_pid != 0 || health.identity.is_some() {
                    fail(FAIL_SERVER_PROTOCOL);
                }
                health.init_pid = envelope.sender_pid();
                health.identity = Some(identity);
            } else if health.init_pid != envelope.sender_pid() || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes += 1;
            let healthy = HealthFrame::healthy(expected_sequence, identity)
                .unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
            write_product_health(startup, healthy);
        }
        2 => {
            if probe.sequence() != 3
                || health.init_pid != envelope.sender_pid()
                || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes = 3;
            // The old M69 StorageServer deliberately withholds only its third
            // probe, after two real healthy cadence rounds. A replacement
            // receives two probes and therefore reaches shutdown with
            // `probes == 2`.
        }
        _ => fail(FAIL_SERVER_PROTOCOL),
    }

    #[cfg(all(
        not(feature = "unified-product-multiservice-liveness-runtime"),
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    match health.probes {
        0 => {
            if probe.sequence() != 1 || health.init_pid != 0 || health.identity.is_some() {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.init_pid = envelope.sender_pid();
            health.identity = Some(identity);
            health.probes = 1;
            let healthy =
                HealthFrame::healthy(1, identity).unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
            write_product_health(startup, healthy);
        }
        1 => {
            if probe.sequence() != 2
                || health.init_pid != envelope.sender_pid()
                || health.identity != Some(identity)
            {
                fail(FAIL_SERVER_PROTOCOL);
            }
            health.probes = 2;
            // Keep the process and its volume capability live while
            // intentionally withholding Probe 2. Init's finite ObjectWait,
            // rather than process exit or channel closure, must classify the
            // M68 health timeout.
        }
        _ => fail(FAIL_SERVER_PROTOCOL),
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn write_product_health(startup: u64, frame: HealthFrame) {
    let wire = frame.encode();
    if !wait_for_signal(startup, ObjectSignals::WRITABLE) {
        fail(FAIL_SERVER_PROTOCOL);
    }
    let written = syscall(
        SyscallNumber::ChannelWriteBytes,
        startup,
        wire.as_ptr() as u64,
        wire.len() as u64,
    );
    if written.status != KernelStatus::Ok.raw()
        || written.out1 != wire.len() as u64
        || written.out2 != 0
    {
        fail(FAIL_SERVER_PROTOCOL);
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
const fn process_slot(pid: u64) -> u32 {
    pid as u32
}

#[cfg(feature = "unified-product-liveness-runtime")]
const fn process_generation(pid: u64) -> u32 {
    (pid >> 32) as u32
}

fn accept_session(volume_handle: u64) -> (u64, StorageSessionBinding) {
    loop {
        let mut wire = [0_u8; STORAGE_SESSION_BINDING_WIRE_SIZE];
        let accepted = syscall(
            SyscallNumber::StorageAccept,
            volume_handle,
            wire.as_mut_ptr() as u64,
            STORAGE_SESSION_BINDING_WIRE_SIZE as u64,
        );
        if accepted.status == KernelStatus::ShouldWait.raw()
            && accepted.out1 == 0
            && accepted.out2 == 0
        {
            if !wait_for_signal(volume_handle, ObjectSignals::READABLE) {
                fail(FAIL_SERVER_ACCEPT);
            }
            continue;
        }
        if accepted.status == KernelStatus::RequiresReset.raw()
            && accepted.out1 == 0
            && accepted.out2 == 0
        {
            exit_child(SERVER_RECOVERY_EXIT_CODE);
        }
        if accepted.status != KernelStatus::Ok.raw() || accepted.out1 == 0 || accepted.out2 != 0 {
            fail(FAIL_SERVER_ACCEPT);
        }
        let binding =
            StorageSessionBinding::decode(&wire).unwrap_or_else(|_| fail(FAIL_SERVER_BINDING));
        return (accepted.out1, binding);
    }
}

fn serve_session(
    session: u64,
    binding: StorageSessionBinding,
    volume: AppDataVolume,
    policy: &PrincipalPolicy,
    io: &mut VolumeIo,
    current_generation: &mut u64,
) -> SessionOutcome {
    let mut bound = false;
    loop {
        if !wait_channel_readable(session) {
            return SessionOutcome::closed(bound);
        }
        let envelope = read_channel_envelope_now(session);
        if envelope.sender_pid() != binding.client_pid() {
            close_received_handle(&envelope);
            return SessionOutcome::closed(false);
        }
        if envelope.kind() != ChannelMessageKind::Transfer
            || envelope.logical_length() != FRAME_SIZE
            || !envelope.received_handle().is_valid()
        {
            close_received_handle(&envelope);
            return SessionOutcome::closed(false);
        }
        let request = match Request::decode(envelope.data()) {
            Ok(request) => request,
            Err(_) => {
                close_received_handle(&envelope);
                return SessionOutcome::closed(false);
            }
        };
        let received = u64::from(envelope.received_handle().raw());
        let StorageScratch {
            request: request_bytes,
            file,
            entries,
        } = scratch();
        let payload_length = request.payload_length();
        let payload_read = read_vmo_exact(
            received,
            payload_length,
            &mut request_bytes[..payload_length],
        );
        close_handle(received, FAIL_SERVER_PROTOCOL);
        if !payload_read {
            if !send_error(
                session,
                request.opcode(),
                request.transaction_id(),
                ProtocolStatus::ProtocolError,
            ) {
                return SessionOutcome::closed(bound);
            }
            continue;
        }
        let payload = match request.validate_payload(&request_bytes[..payload_length]) {
            Ok(payload) => payload,
            Err(_) => {
                if !send_error(
                    session,
                    request.opcode(),
                    request.transaction_id(),
                    ProtocolStatus::ProtocolError,
                ) {
                    return SessionOutcome::closed(bound);
                }
                continue;
            }
        };

        if request.opcode() == Opcode::Bind {
            if bound {
                if !send_error(
                    session,
                    request.opcode(),
                    request.transaction_id(),
                    ProtocolStatus::InvalidState,
                ) {
                    return SessionOutcome::closed(bound);
                }
            } else {
                let response = Response::bound(request.transaction_id())
                    .unwrap_or_else(|_| fail(FAIL_SERVER_RESPONSE));
                if !send_response(session, response, &[]) {
                    return SessionOutcome::closed(false);
                }
                bound = true;
            }
            continue;
        }
        if !bound {
            if !send_error(
                session,
                request.opcode(),
                request.transaction_id(),
                ProtocolStatus::PermissionDenied,
            ) {
                return SessionOutcome::closed(false);
            }
            continue;
        }
        let required = required_right(request.opcode());
        if !binding.granted_rights().contains(required) {
            if !send_error(
                session,
                request.opcode(),
                request.transaction_id(),
                ProtocolStatus::PermissionDenied,
            ) {
                return SessionOutcome::closed(bound);
            }
            continue;
        }
        match dispatch_request(
            session,
            binding.principal().raw(),
            request,
            payload.path(),
            payload.value(),
            volume,
            policy,
            io,
            current_generation,
            file,
            entries,
        ) {
            DispatchOutcome::Continue => {}
            DispatchOutcome::SessionClosed => return SessionOutcome::closed(bound),
            DispatchOutcome::FatalStorage => return SessionOutcome::fatal(bound),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_request(
    session: u64,
    principal: u64,
    request: Request,
    path: &str,
    value: &[u8],
    volume: AppDataVolume,
    policy: &PrincipalPolicy,
    io: &mut VolumeIo,
    current_generation: &mut u64,
    file: &mut [u8; VALUE_MAX_BYTES],
    entries: &mut [DirEntry; MAX_ENTRIES],
) -> DispatchOutcome {
    let transaction_id = request.transaction_id();
    let response = match request.opcode() {
        Opcode::Bind => fail(FAIL_SERVER_PROTOCOL),
        Opcode::CreateDirectory => match volume.create_dir_with_policy(io, policy, principal, path)
        {
            Ok(mutation) => {
                *current_generation = mutation.committed_generation;
                Response::mutation(
                    Opcode::CreateDirectory,
                    transaction_id,
                    mutation.committed_generation,
                    0,
                )
            }
            Err(error) => return send_policy_error(session, request, error),
        },
        Opcode::Replace => {
            let condition = match request
                .replace_condition()
                .unwrap_or_else(|| fail(FAIL_SERVER_PROTOCOL))
            {
                ReplaceCondition::Any => VolumeReplaceCondition::Any,
                ReplaceCondition::CreateOnly => VolumeReplaceCondition::Absent,
                ReplaceCondition::Exact(generation) => {
                    VolumeReplaceCondition::Generation(generation)
                }
            };
            match volume.replace_with_policy(io, policy, principal, path, value, condition) {
                Ok(mutation) => {
                    *current_generation = mutation.committed_generation;
                    Response::mutation(
                        Opcode::Replace,
                        transaction_id,
                        mutation
                            .entry_revision
                            .unwrap_or(mutation.committed_generation),
                        value.len(),
                    )
                }
                Err(error) => return send_policy_error(session, request, error),
            }
        }
        Opcode::Read => match volume.read(io, principal, path, file) {
            Ok(read) => {
                *current_generation = read.snapshot_generation;
                let response = Response::file(transaction_id, read.revision, read.bytes)
                    .unwrap_or_else(|_| fail(FAIL_SERVER_RESPONSE));
                return dispatch_from_send(send_response(session, response, &file[..read.bytes]));
            }
            Err(error) => return send_volume_error(session, request, error),
        },
        Opcode::List => {
            entries.fill(DirEntry::EMPTY);
            let list = match volume.list(io, principal, path, entries) {
                Ok(list) => list,
                Err(error) => return send_volume_error(session, request, error),
            };
            *current_generation = list.snapshot_generation;
            let cursor =
                usize::try_from(request.cursor()).unwrap_or_else(|_| fail(FAIL_SERVER_PROTOCOL));
            if cursor >= list.written {
                Response::end_of_directory(transaction_id, request.cursor())
            } else {
                let entry = entries[cursor];
                let (kind, generation, size) = match entry.kind {
                    VolumeEntryKind::File => {
                        (EntryKind::File, entry.revision, entry.length as usize)
                    }
                    VolumeEntryKind::Directory => (EntryKind::Directory, 0, 0),
                };
                let name = entry.name().as_bytes();
                let response = Response::directory_entry(
                    transaction_id,
                    request.cursor() + 1,
                    name.len(),
                    kind,
                    generation,
                    size,
                )
                .unwrap_or_else(|_| fail(FAIL_SERVER_RESPONSE));
                return dispatch_from_send(send_response(session, response, name));
            }
        }
        Opcode::Unlink => match volume.unlink(io, principal, path) {
            Ok(mutation) => {
                *current_generation = mutation.committed_generation;
                Response::mutation(
                    Opcode::Unlink,
                    transaction_id,
                    mutation.committed_generation,
                    0,
                )
            }
            Err(error) => return send_volume_error(session, request, error),
        },
    }
    .unwrap_or_else(|_| fail(FAIL_SERVER_RESPONSE));
    dispatch_from_send(send_response(session, response, &[]))
}

fn send_volume_error(session: u64, request: Request, error: VolumeError) -> DispatchOutcome {
    send_dispatch_error(
        session,
        request.opcode(),
        request.transaction_id(),
        map_volume_error(error),
    )
}

fn send_policy_error(session: u64, request: Request, error: PolicyError) -> DispatchOutcome {
    let status = match error {
        PolicyError::Volume(error) => map_volume_error(error),
        PolicyError::UnknownPrincipal { .. } => ProtocolStatus::PermissionDenied,
        PolicyError::PrincipalEntryQuotaExceeded { .. }
        | PolicyError::PrincipalLivePayloadQuotaExceeded { .. }
        | PolicyError::AggregateEntryQuotaExceeded { .. }
        | PolicyError::AggregateLivePayloadQuotaExceeded { .. } => ProtocolStatus::NoSpace,
        PolicyError::EmptyPolicy
        | PolicyError::TooManyPrincipals { .. }
        | PolicyError::InvalidPrincipal
        | PolicyError::InvalidEntryQuota { .. }
        | PolicyError::InvalidLivePayloadQuota { .. }
        | PolicyError::DuplicatePrincipal { .. }
        | PolicyError::InvalidUnusedSlot => fail(FAIL_SERVER_VOLUME),
    };
    send_dispatch_error(session, request.opcode(), request.transaction_id(), status)
}

fn send_dispatch_error(
    session: u64,
    opcode: Opcode,
    transaction_id: u64,
    status: ProtocolStatus,
) -> DispatchOutcome {
    let fatal = status.is_session_fatal();
    let sent = send_error(session, opcode, transaction_id, status);
    if fatal {
        // The error remains fatal even when the peer disappears before the
        // terminal response. A mutation without a received terminal response
        // is conservatively reconciled by the client in the next epoch.
        DispatchOutcome::FatalStorage
    } else {
        dispatch_from_send(sent)
    }
}

const fn dispatch_from_send(sent: bool) -> DispatchOutcome {
    if sent {
        DispatchOutcome::Continue
    } else {
        DispatchOutcome::SessionClosed
    }
}

fn map_volume_error(error: VolumeError) -> ProtocolStatus {
    match error {
        VolumeError::InvalidPrincipal
        | VolumeError::InvalidPath(_)
        | VolumeError::FileTooLarge
        | VolumeError::VolumeBounds => ProtocolStatus::InvalidArgument,
        VolumeError::NotFound => ProtocolStatus::NotFound,
        VolumeError::AlreadyExists => ProtocolStatus::AlreadyExists,
        VolumeError::NotDirectory => ProtocolStatus::NotDirectory,
        VolumeError::IsDirectory => ProtocolStatus::IsDirectory,
        VolumeError::NotEmpty => ProtocolStatus::NotEmpty,
        VolumeError::BufferTooSmall { .. } => ProtocolStatus::BufferTooSmall,
        VolumeError::OutOfSpace | VolumeError::GenerationExhausted => ProtocolStatus::NoSpace,
        VolumeError::Conflict { .. } => ProtocolStatus::Conflict,
        VolumeError::OutcomeUnknown | VolumeError::Io(IoError::OutcomeUnknown) => {
            ProtocolStatus::OutcomeUnknown
        }
        VolumeError::RequiresReset | VolumeError::Io(IoError::RequiresReset) => {
            ProtocolStatus::RequiresReset
        }
        VolumeError::Corrupt(_) | VolumeError::VerificationFailed => ProtocolStatus::DataCorrupt,
        VolumeError::Io(IoError::OutOfBounds) => ProtocolStatus::InvalidArgument,
        VolumeError::Io(IoError::Device) => ProtocolStatus::Unavailable,
        VolumeError::Unformatted | VolumeError::AlreadyFormatted | VolumeError::NotVirgin => {
            ProtocolStatus::InvalidState
        }
    }
}

fn required_right(opcode: Opcode) -> Rights {
    match opcode {
        Opcode::Read | Opcode::List => Rights::READ,
        Opcode::CreateDirectory | Opcode::Replace | Opcode::Unlink => Rights::WRITE,
        Opcode::Bind => Rights::NONE,
    }
}

fn storage_policy() -> PrincipalPolicy {
    let app = PrincipalQuota::try_new(AppDataPrincipal::PRIMARY_APP.raw(), 16, 65_536)
        .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
    let launcher = PrincipalQuota::try_new(AppDataPrincipal::LAUNCHER.raw(), 16, 65_536)
        .unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME));
    PrincipalPolicy::try_new(&[app, launcher]).unwrap_or_else(|_| fail(FAIL_SERVER_VOLUME))
}

fn send_error(session: u64, opcode: Opcode, transaction_id: u64, status: ProtocolStatus) -> bool {
    let response = Response::error(opcode, transaction_id, status)
        .unwrap_or_else(|_| fail(FAIL_SERVER_RESPONSE));
    send_response(session, response, &[])
}

fn send_response(session: u64, response: Response, payload: &[u8]) -> bool {
    if response.payload_length() != payload.len()
        || (!response.has_payload_vmo() && !payload.is_empty())
    {
        fail(FAIL_SERVER_RESPONSE);
    }
    let wire = response.encode();
    if !wait_channel_writable(session) {
        return false;
    }
    if !response.has_payload_vmo() {
        let written = syscall(
            SyscallNumber::ChannelWriteBytes,
            session,
            wire.as_ptr() as u64,
            FRAME_SIZE as u64,
        );
        if written.status == KernelStatus::PeerClosed.raw() {
            return false;
        }
        if written.status != KernelStatus::Ok.raw()
            || written.out1 != FRAME_SIZE as u64
            || written.out2 != 0
        {
            fail(FAIL_SERVER_RESPONSE);
        }
        return true;
    }

    let buffer = create_ipc_buffer(payload, FAIL_SERVER_RESPONSE);
    let written = transfer_write(session, wire.as_ptr() as u64, buffer, FRAME_SIZE);
    if written.status == KernelStatus::PeerClosed.raw() {
        close_handle(buffer, FAIL_SERVER_RESPONSE);
        return false;
    }
    if written.status != KernelStatus::Ok.raw()
        || written.out1 != FRAME_SIZE as u64
        || written.out2 != 0
    {
        close_handle(buffer, FAIL_SERVER_RESPONSE);
        fail(FAIL_SERVER_RESPONSE);
    }
    assert_stale_handle(buffer);
    true
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
pub(super) fn resident_client_work(startup: u64, principal: AppDataPrincipal) -> u64 {
    expect_abi(FAIL_CLIENT_CONNECT);
    exercise_client_authority(startup);
    let connected = syscall(
        SyscallNumber::StorageConnect,
        u64::from(Rights::READ.bits() | Rights::WRITE.bits()),
        0,
        0,
    );
    if connected.status != KernelStatus::Ok.raw() || connected.out1 == 0 || connected.out2 != 0 {
        fail(FAIL_CLIENT_CONNECT);
    }
    let session_handle = connected.out1;
    expect_result(
        syscall(
            SyscallNumber::HandleDuplicate,
            session_handle,
            u64::from(Rights::READ.bits()),
            0,
        ),
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );
    let transfer_denied = transfer_write(startup, u64::MAX, session_handle, 0);
    expect_result(
        transfer_denied,
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );

    let mut session = ClientSession {
        handle: session_handle,
        server_pid: 0,
        next_transaction_id: 1,
    };
    client_bind(&mut session);
    let stage = client_phase_one(principal, &mut session, scratch());
    close_handle(session.handle, FAIL_CLIENT_PROTOCOL);
    stage
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
pub(super) fn resident_storage_admission_probe() {
    expect_result(
        syscall(
            SyscallNumber::StorageConnect,
            u64::from(Rights::READ.bits() | Rights::WRITE.bits()),
            0,
            0,
        ),
        KernelStatus::Unavailable,
        FAIL_CLIENT_AUTHORITY,
    );
}

pub(super) fn client_runtime(startup: u64, principal: AppDataPrincipal) -> ! {
    expect_abi(FAIL_CLIENT_CONNECT);
    let (supervisor_pid, (tag, command)) = read_scalar_envelope(startup);
    let (phase, expected_stage) = unpack_client_command(command);
    if supervisor_pid == 0
        || tag != CLIENT_COMMAND_TAG
        || !matches!(phase, PHASE_ONE | PHASE_TWO)
        || (phase == PHASE_ONE && expected_stage != 0)
        || (phase == PHASE_TWO && !matches!(expected_stage, STAGE_ONE | STAGE_TWO))
    {
        fail(FAIL_CLIENT_COMMAND);
    }
    exercise_client_authority(startup);
    let connected = syscall(
        SyscallNumber::StorageConnect,
        u64::from(Rights::READ.bits() | Rights::WRITE.bits()),
        0,
        0,
    );
    if connected.status != KernelStatus::Ok.raw() || connected.out1 == 0 || connected.out2 != 0 {
        fail(FAIL_CLIENT_CONNECT);
    }
    let session_handle = connected.out1;
    expect_result(
        syscall(
            SyscallNumber::HandleDuplicate,
            session_handle,
            u64::from(Rights::READ.bits()),
            0,
        ),
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );
    let transfer_denied = transfer_write(startup, u64::MAX, session_handle, 0);
    expect_result(
        transfer_denied,
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );

    let mut session = ClientSession {
        handle: session_handle,
        server_pid: 0,
        next_transaction_id: 1,
    };
    client_bind(&mut session);
    let workspace = scratch();
    let stage = if phase == PHASE_ONE {
        client_phase_one(principal, &mut session, workspace)
    } else {
        client_phase_two(principal, expected_stage, &mut session, workspace)
    };
    close_handle(session.handle, FAIL_CLIENT_PROTOCOL);
    write_scalar(
        startup,
        CLIENT_DONE_TAG,
        pack_client_proof(phase, principal.raw(), stage),
    );
    exit_child(CLIENT_EXIT_CODE)
}

fn exercise_client_authority(startup: u64) {
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    expect_result(
        syscall(
            SyscallNumber::SystemShutdown,
            SYSTEM_SHUTDOWN_PREPARE,
            1,
            SYSTEM_SHUTDOWN_FLAGS_NONE,
        ),
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );
    for rights in [0, u64::from(Rights::TRANSFER.bits()), u64::from(u32::MAX)] {
        expect_result(
            syscall(SyscallNumber::StorageConnect, rights, 0, 0),
            KernelStatus::InvalidArgument,
            FAIL_CLIENT_AUTHORITY,
        );
    }
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        KernelStatus::PermissionDenied,
        FAIL_CLIENT_AUTHORITY,
    );
    // The startup channel remains usable after every rejected connect.
    if object_wait(startup, ObjectSignals::WRITABLE).status != KernelStatus::Ok.raw() {
        fail(FAIL_CLIENT_AUTHORITY);
    }
}

struct ClientSession {
    handle: u64,
    server_pid: u64,
    next_transaction_id: u64,
}

impl ClientSession {
    fn transaction_id(&mut self) -> u64 {
        let current = self.next_transaction_id;
        self.next_transaction_id = current
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_CLIENT_PROTOCOL));
        current
    }
}

fn client_bind(session: &mut ClientSession) {
    let request =
        Request::bind(session.transaction_id()).unwrap_or_else(|_| fail(FAIL_CLIENT_PROTOCOL));
    let mut ignored = [0_u8; 1];
    let response = client_exchange(session, request, &[], &mut ignored);
    if response.status() != ProtocolStatus::Ok
        || response.result_kind() != ResultKind::Bound
        || response.has_payload_vmo()
    {
        fail(FAIL_CLIENT_PROTOCOL);
    }
}

fn client_phase_one(
    principal: AppDataPrincipal,
    session: &mut ClientSession,
    workspace: &mut StorageScratch,
) -> u64 {
    let state = read_client_state(principal, session, workspace);
    if state.is_none() {
        let transaction_id = session.transaction_id();
        let request = Request::create_directory(transaction_id, NAMESPACE_PATH.len())
            .unwrap_or_else(|_| fail(FAIL_CLIENT_PROTOCOL));
        let response = client_exchange(
            session,
            request,
            NAMESPACE_PATH.as_bytes(),
            &mut workspace.file,
        );
        if response.status() == ProtocolStatus::Ok {
            if response.result_kind() != ResultKind::Mutation || response.generation() == 0 {
                fail(FAIL_CLIENT_PROTOCOL);
            }
        } else if response.status() != ProtocolStatus::AlreadyExists {
            fail(FAIL_CLIENT_PROTOCOL);
        }
    }

    let target = match (principal, state) {
        (AppDataPrincipal::LAUNCHER, None) => Some(STAGE_TWO),
        (AppDataPrincipal::LAUNCHER, Some((STAGE_TWO, _))) => None,
        (AppDataPrincipal::PRIMARY_APP, None) => Some(STAGE_ONE),
        (AppDataPrincipal::PRIMARY_APP, Some((STAGE_ONE, _))) => Some(STAGE_TWO),
        (AppDataPrincipal::PRIMARY_APP, Some((STAGE_TWO, _))) => None,
        _ => fail(FAIL_CLIENT_STATE),
    };
    if let Some(target_stage) = target {
        let condition = state.map_or(ReplaceCondition::CreateOnly, |(_, generation)| {
            ReplaceCondition::Exact(generation)
        });
        replace_client_state(principal, target_stage, condition, session, workspace);
    }
    let (stage, _) =
        read_client_state(principal, session, workspace).unwrap_or_else(|| fail(FAIL_CLIENT_STATE));
    stage
}

fn client_phase_two(
    principal: AppDataPrincipal,
    expected_stage: u64,
    session: &mut ClientSession,
    workspace: &mut StorageScratch,
) -> u64 {
    let (stage, _) =
        read_client_state(principal, session, workspace).unwrap_or_else(|| fail(FAIL_CLIENT_STATE));
    if stage != expected_stage {
        fail(FAIL_CLIENT_STATE);
    }
    stage
}

fn read_client_state(
    principal: AppDataPrincipal,
    session: &mut ClientSession,
    workspace: &mut StorageScratch,
) -> Option<(u64, u64)> {
    let request = Request::read(session.transaction_id(), STATE_PATH.len())
        .unwrap_or_else(|_| fail(FAIL_CLIENT_PROTOCOL));
    let response = client_exchange(session, request, STATE_PATH.as_bytes(), &mut workspace.file);
    if response.status() == ProtocolStatus::NotFound {
        if response.result_kind() != ResultKind::None || response.has_payload_vmo() {
            fail(FAIL_CLIENT_PROTOCOL);
        }
        return None;
    }
    if response.status() != ProtocolStatus::Ok
        || response.result_kind() != ResultKind::File
        || response.payload_length() != STATE_VALUE_BYTES
        || response.generation() == 0
    {
        fail(FAIL_CLIENT_PROTOCOL);
    }
    let stored_principal = read_u64(&workspace.file[..8]);
    let stage = read_u64(&workspace.file[8..STATE_VALUE_BYTES]);
    if stored_principal != principal.raw() || !matches!(stage, STAGE_ONE | STAGE_TWO) {
        fail(FAIL_CLIENT_STATE);
    }
    Some((stage, response.generation()))
}

fn replace_client_state(
    principal: AppDataPrincipal,
    stage: u64,
    condition: ReplaceCondition,
    session: &mut ClientSession,
    workspace: &mut StorageScratch,
) {
    let mut value = [0_u8; STATE_VALUE_BYTES];
    value[..8].copy_from_slice(&principal.raw().to_le_bytes());
    value[8..].copy_from_slice(&stage.to_le_bytes());
    let request = Request::replace(
        session.transaction_id(),
        STATE_PATH.len(),
        value.len(),
        condition,
    )
    .unwrap_or_else(|_| fail(FAIL_CLIENT_PROTOCOL));
    let payload_length = STATE_PATH.len() + value.len();
    workspace.request[..STATE_PATH.len()].copy_from_slice(STATE_PATH.as_bytes());
    workspace.request[STATE_PATH.len()..payload_length].copy_from_slice(&value);
    let response = client_exchange(
        session,
        request,
        &workspace.request[..payload_length],
        &mut workspace.file,
    );
    if response.status() != ProtocolStatus::Ok
        || response.result_kind() != ResultKind::Mutation
        || response.generation() == 0
        || response.object_size() != value.len()
        || response.has_payload_vmo()
    {
        fail(FAIL_CLIENT_PROTOCOL);
    }
}

fn client_exchange(
    session: &mut ClientSession,
    request: Request,
    payload: &[u8],
    response_payload: &mut [u8],
) -> Response {
    if payload.len() != request.payload_length() {
        fail(FAIL_CLIENT_PROTOCOL);
    }
    let buffer = create_ipc_buffer(payload, FAIL_CLIENT_PROTOCOL);
    if !wait_channel_writable(session.handle) {
        close_handle(buffer, FAIL_CLIENT_PROTOCOL);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    let wire = request.encode();
    let written = transfer_write(session.handle, wire.as_ptr() as u64, buffer, FRAME_SIZE);
    if written.status != KernelStatus::Ok.raw()
        || written.out1 != FRAME_SIZE as u64
        || written.out2 != 0
    {
        close_handle(buffer, FAIL_CLIENT_PROTOCOL);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    assert_stale_handle(buffer);
    if !wait_channel_readable(session.handle) {
        fail(FAIL_CLIENT_PROTOCOL);
    }
    let envelope = read_channel_envelope_now(session.handle);
    if envelope.logical_length() != FRAME_SIZE || envelope.sender_pid() == 0 {
        close_received_handle(&envelope);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    if session.server_pid == 0 {
        session.server_pid = envelope.sender_pid();
    } else if envelope.sender_pid() != session.server_pid {
        close_received_handle(&envelope);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    let response = Response::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_CLIENT_PROTOCOL));
    if response.opcode() != request.opcode()
        || response.transaction_id() != request.transaction_id()
    {
        close_received_handle(&envelope);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    if response.has_payload_vmo() {
        if envelope.kind() != ChannelMessageKind::Transfer
            || !envelope.received_handle().is_valid()
            || response.payload_length() > response_payload.len()
        {
            close_received_handle(&envelope);
            fail(FAIL_CLIENT_PROTOCOL);
        }
        let received = u64::from(envelope.received_handle().raw());
        let length = response.payload_length();
        let valid = read_vmo_exact(received, length, &mut response_payload[..length]);
        close_handle(received, FAIL_CLIENT_PROTOCOL);
        if !valid
            || response
                .validate_payload(&response_payload[..length])
                .is_err()
        {
            fail(FAIL_CLIENT_PROTOCOL);
        }
    } else if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.received_handle().is_valid()
        || response.payload_length() != 0
    {
        close_received_handle(&envelope);
        fail(FAIL_CLIENT_PROTOCOL);
    }
    response
}

fn create_ipc_buffer(payload: &[u8], reason: u64) -> u64 {
    let source = if payload.is_empty() {
        0
    } else {
        payload.as_ptr() as u64
    };
    let created = syscall(
        SyscallNumber::IpcBufferCreate,
        source,
        payload.len() as u64,
        IPC_BUFFER_CREATE_FLAGS_NONE,
    );
    if created.status != KernelStatus::Ok.raw()
        || created.out1 == 0
        || created.out2 != payload.len() as u64
    {
        fail(reason);
    }
    created.out1
}

fn read_vmo_exact(handle: u64, length: usize, output: &mut [u8]) -> bool {
    if length > output.len() {
        return false;
    }
    if length == 0 {
        let read = syscall(SyscallNumber::VmoRead, handle, 0, pack_vmo_read(0, 0));
        return read.status == KernelStatus::Ok.raw() && read.out1 == 0 && read.out2 == 0;
    }
    let mut offset = 0_usize;
    while offset < length {
        let amount = core::cmp::min(VMO_READ_MAX_BYTES, length - offset);
        let read = syscall(
            SyscallNumber::VmoRead,
            handle,
            output[offset..].as_mut_ptr() as u64,
            pack_vmo_read(offset as u32, amount as u32),
        );
        if read.status != KernelStatus::Ok.raw()
            || read.out1 != amount as u64
            || read.out2 != length as u64
        {
            return false;
        }
        offset += amount;
    }
    true
}

fn wait_for_signal(handle: u64, desired: ObjectSignals) -> bool {
    let requested = ObjectSignals::from_bits(desired.bits() | ObjectSignals::PEER_CLOSED.bits())
        .unwrap_or_else(|| fail(FAIL_SERVER_PROTOCOL));
    let waited = object_wait(handle, requested);
    if waited.status != KernelStatus::Ok.raw() || waited.out2 != 0 {
        return false;
    }
    waited.out1 & u64::from(desired.bits()) != 0
}

fn wait_channel_readable(handle: u64) -> bool {
    wait_for_signal(handle, ObjectSignals::READABLE)
}

fn wait_channel_writable(handle: u64) -> bool {
    wait_for_signal(handle, ObjectSignals::WRITABLE)
}

fn close_received_handle(envelope: &bndr_abi::ChannelReadEnvelope) {
    if envelope.received_handle().is_valid() {
        close_handle(
            u64::from(envelope.received_handle().raw()),
            FAIL_SERVER_PROTOCOL,
        );
    }
}

fn close_handle(handle: u64, reason: u64) {
    let closed = syscall(SyscallNumber::HandleClose, handle, 0, 0);
    if closed.status != KernelStatus::Ok.raw() || closed.out1 != 0 || closed.out2 != 0 {
        fail(reason);
    }
}

fn expect_abi(reason: u64) {
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != KernelStatus::Ok.raw() || abi.out1 != bndr_abi::ABI_VERSION || abi.out2 != 0 {
        fail(reason);
    }
}

fn expect_result(result: SyscallResult, status: KernelStatus, reason: u64) {
    if result.status != status.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

const fn principal_bit(principal: AppDataPrincipal) -> u8 {
    if principal.raw() == AppDataPrincipal::PRIMARY_APP.raw() {
        0b01
    } else if principal.raw() == AppDataPrincipal::LAUNCHER.raw() {
        0b10
    } else {
        0
    }
}

const fn pack_client_command(phase: u64, expected_stage: u64) -> u64 {
    (phase << 32) | expected_stage
}

const fn unpack_client_command(command: u64) -> (u64, u64) {
    (command >> 32, command as u32 as u64)
}

const fn pack_client_proof(phase: u64, principal: u64, stage: u64) -> u64 {
    (phase << 56) | (principal << 48) | stage
}

const fn unpack_client_proof(proof: u64) -> (u64, u64, u64) {
    (
        proof >> 56,
        (proof >> 48) & 0xff,
        proof & 0x0000_ffff_ffff_ffff,
    )
}

fn read_u64(bytes: &[u8]) -> u64 {
    let mut encoded = [0_u8; 8];
    encoded.copy_from_slice(bytes);
    u64::from_le_bytes(encoded)
}

const _: () = assert!(FRAME_SIZE == CHANNEL_MESSAGE_MAX_BYTES);
const _: () = assert!(REQUEST_PAYLOAD_MAX_BYTES == 4_160);
const _: () = assert!(VALUE_MAX_BYTES == 4_096);
const _: () = assert!(STORAGE_CONNECT_RIGHTS_MASK == 0b11);
