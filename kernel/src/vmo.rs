use alloc::alloc::{alloc, dealloc};
use core::alloc::Layout;
use core::fmt;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{AtomicUsize, Ordering, fence};

/// M55's opt-in IPC constructor carries a maximum 64-byte path next to a
/// 4096-byte whole-file value. Historical profiles retain the exact 4096-byte
/// ceiling; `VmoRead` remains chunked at 4096 bytes in every profile.
#[cfg(feature = "storage-server-runtime")]
pub const VMO_MAX_BYTES: usize = bndr_abi::IPC_BUFFER_PAYLOAD_MAX_BYTES;
#[cfg(not(feature = "storage-server-runtime"))]
pub const VMO_MAX_BYTES: usize = 4096;
/// Private ceiling used only while publishing one verified installed-package
/// image to syscall 61. It does not enlarge any pre-existing VMO constructor.
#[cfg(feature = "androidbox-el0-runtime0")]
pub const PACKAGE_IMAGE_MAX_BYTES: usize = bndr_abi::ANDROID_PACKAGE_APK_MAX_BYTES as usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmoCreateError {
    TooLarge,
    OutOfMemory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmoReadError {
    OutOfRange,
    RangeOverflow,
}

type Deallocate = unsafe fn(*mut u8, Layout);

#[repr(C)]
struct VmoInner {
    strong: AtomicUsize,
    len: usize,
    deallocate: Deallocate,
}

/// An immutable, reference-counted kernel byte object.
///
/// The payload is stored immediately after the allocation header and is never
/// mutated after publication. Clones therefore share both identity and bytes
/// without another allocation or copy.
pub struct Vmo {
    inner: NonNull<VmoInner>,
}

// The allocation is immutable after construction except for its atomic strong
// count. Every Vmo owns one strong count, so the allocation outlives access.
unsafe impl Send for Vmo {}
unsafe impl Sync for Vmo {}

impl Vmo {
    pub const MAX_BYTES: usize = VMO_MAX_BYTES;

    /// Copies `bytes` into a new VMO without invoking the global OOM handler.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, VmoCreateError> {
        Self::try_from_slice_with_limit(
            bytes,
            VMO_MAX_BYTES,
            |layout| unsafe { alloc(layout) },
            global_deallocate,
        )
    }

    /// Copies one already-verified installed APK into the private syscall-61
    /// handoff object without widening any public or pre-existing VMO source.
    #[cfg(feature = "androidbox-el0-runtime0")]
    pub fn try_from_package_image(bytes: &[u8]) -> Result<Self, VmoCreateError> {
        Self::try_from_slice_with_limit(
            bytes,
            PACKAGE_IMAGE_MAX_BYTES,
            |layout| unsafe { alloc(layout) },
            global_deallocate,
        )
    }

    #[cfg(test)]
    fn try_from_slice_with(
        bytes: &[u8],
        allocate: impl FnOnce(Layout) -> *mut u8,
        deallocate: Deallocate,
    ) -> Result<Self, VmoCreateError> {
        Self::try_from_slice_with_limit(bytes, VMO_MAX_BYTES, allocate, deallocate)
    }

    fn try_from_slice_with_limit(
        bytes: &[u8],
        maximum: usize,
        allocate: impl FnOnce(Layout) -> *mut u8,
        deallocate: Deallocate,
    ) -> Result<Self, VmoCreateError> {
        if bytes.len() > maximum {
            return Err(VmoCreateError::TooLarge);
        }

        let (layout, payload_offset) = allocation_layout(bytes.len());
        let inner =
            NonNull::new(allocate(layout).cast::<VmoInner>()).ok_or(VmoCreateError::OutOfMemory)?;
        unsafe {
            inner.as_ptr().write(VmoInner {
                strong: AtomicUsize::new(1),
                len: bytes.len(),
                deallocate,
            });
            inner
                .as_ptr()
                .cast::<u8>()
                .add(payload_offset)
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
        }
        Ok(Self { inner })
    }

    pub fn len(&self) -> usize {
        self.inner().len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a borrowed range, truncating a valid request at EOF.
    ///
    /// An offset exactly at EOF is valid and returns an empty slice. An offset
    /// beyond EOF or an overflowing `offset + requested_len` is rejected.
    pub fn read_range(&self, offset: usize, requested_len: usize) -> Result<&[u8], VmoReadError> {
        if offset > self.len() {
            return Err(VmoReadError::OutOfRange);
        }
        let requested_end = offset
            .checked_add(requested_len)
            .ok_or(VmoReadError::RangeOverflow)?;
        let end = requested_end.min(self.len());
        Ok(&self.bytes()[offset..end])
    }

    /// Copies a bounded range into `out`, returning the number of bytes read.
    ///
    /// Short reads at EOF are successful and bytes after the returned length
    /// in `out` are left untouched.
    pub fn read(&self, offset: usize, out: &mut [u8]) -> Result<usize, VmoReadError> {
        let bytes = self.read_range(offset, out.len())?;
        out[..bytes.len()].copy_from_slice(bytes);
        Ok(bytes.len())
    }

    pub fn same_vmo(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn inner(&self) -> &VmoInner {
        unsafe { self.inner.as_ref() }
    }

    fn bytes(&self) -> &[u8] {
        let (_, payload_offset) = allocation_layout(self.len());
        unsafe {
            slice::from_raw_parts(
                self.inner.as_ptr().cast::<u8>().add(payload_offset),
                self.len(),
            )
        }
    }

    #[cfg(test)]
    fn reference_count(&self) -> usize {
        self.inner().strong.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for Vmo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Vmo")
            .field("inner", &self.inner)
            .field("len", &self.len())
            .finish()
    }
}

impl Clone for Vmo {
    fn clone(&self) -> Self {
        self.inner()
            .strong
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |strong| {
                strong
                    .checked_add(1)
                    .filter(|next| *next <= isize::MAX as usize)
            })
            .unwrap_or_else(|_| panic!("VMO strong-reference overflow"));
        Self { inner: self.inner }
    }
}

impl Drop for Vmo {
    fn drop(&mut self) {
        let inner = self.inner();
        let previous = inner.strong.fetch_sub(1, Ordering::Release);
        assert!(previous != 0, "VMO strong-reference underflow");
        if previous == 1 {
            fence(Ordering::Acquire);
            let (layout, _) = allocation_layout(inner.len);
            let deallocate = inner.deallocate;
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
                deallocate(self.inner.as_ptr().cast::<u8>(), layout);
            }
        }
    }
}

fn allocation_layout(payload_len: usize) -> (Layout, usize) {
    let payload = Layout::array::<u8>(payload_len)
        .unwrap_or_else(|_| panic!("bounded VMO payload layout overflowed"));
    let (layout, payload_offset) = Layout::new::<VmoInner>()
        .extend(payload)
        .unwrap_or_else(|_| panic!("bounded VMO allocation layout overflowed"));
    (layout.pad_to_align(), payload_offset)
}

unsafe fn global_deallocate(address: *mut u8, layout: Layout) {
    unsafe { dealloc(address, layout) };
}

#[cfg(test)]
mod tests {
    use alloc::alloc::{alloc, dealloc};
    use core::alloc::Layout;
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(feature = "androidbox-el0-runtime0")]
    use super::PACKAGE_IMAGE_MAX_BYTES;
    use super::{VMO_MAX_BYTES, Vmo, VmoCreateError, VmoReadError, global_deallocate};

    static COUNTED_DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

    unsafe fn counted_deallocate(address: *mut u8, layout: Layout) {
        COUNTED_DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { dealloc(address, layout) };
    }

    #[test]
    fn fallible_creation_reports_allocator_exhaustion() {
        assert!(matches!(
            Vmo::try_from_slice_with(
                b"allocation must fail",
                |_| core::ptr::null_mut(),
                global_deallocate,
            ),
            Err(VmoCreateError::OutOfMemory)
        ));
    }

    #[test]
    fn rejects_an_oversized_payload_before_allocating() {
        let bytes = [0xa5; VMO_MAX_BYTES + 1];
        let allocation_attempts = AtomicUsize::new(0);
        let result = Vmo::try_from_slice_with(
            &bytes,
            |_| {
                allocation_attempts.fetch_add(1, Ordering::Relaxed);
                core::ptr::null_mut()
            },
            global_deallocate,
        );
        assert!(matches!(result, Err(VmoCreateError::TooLarge)));
        assert_eq!(allocation_attempts.load(Ordering::Relaxed), 0);
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    #[test]
    fn private_package_constructor_does_not_widen_the_normal_vmo_limit() {
        assert_eq!(VMO_MAX_BYTES, 4096);
        assert_eq!(
            PACKAGE_IMAGE_MAX_BYTES,
            bndr_abi::ANDROID_PACKAGE_APK_MAX_BYTES as usize
        );
        let private_only = std::vec![0x5a; VMO_MAX_BYTES + 1];
        assert!(matches!(
            Vmo::try_from_slice(&private_only),
            Err(VmoCreateError::TooLarge)
        ));
        assert_eq!(
            Vmo::try_from_package_image(&private_only).unwrap().len(),
            private_only.len()
        );
        let oversized = std::vec![0; PACKAGE_IMAGE_MAX_BYTES + 1];
        assert!(matches!(
            Vmo::try_from_package_image(&oversized),
            Err(VmoCreateError::TooLarge)
        ));
    }

    #[test]
    fn accepts_the_maximum_payload_and_reports_length() {
        let bytes = [0x5a; VMO_MAX_BYTES];
        let vmo = Vmo::try_from_slice(&bytes).unwrap();
        assert_eq!(vmo.len(), VMO_MAX_BYTES);
        assert!(!vmo.is_empty());
        assert_eq!(vmo.read_range(0, usize::MAX).unwrap(), bytes);
    }

    #[test]
    fn empty_vmo_has_a_valid_zero_length_read() {
        let vmo = Vmo::try_from_slice(&[]).unwrap();
        let mut out = [];
        assert!(vmo.is_empty());
        assert_eq!(vmo.read(0, &mut out), Ok(0));
        assert_eq!(vmo.read_range(0, 0), Ok(&[][..]));
    }

    #[test]
    fn clones_share_identity_and_the_last_drop_frees_once() {
        COUNTED_DEALLOCATIONS.store(0, Ordering::Relaxed);
        let original = Vmo::try_from_slice_with(
            b"shared",
            |layout| unsafe { alloc(layout) },
            counted_deallocate,
        )
        .unwrap();
        let first = original.clone();
        let second = first.clone();

        assert!(original.same_vmo(&first));
        assert!(first.same_vmo(&second));
        assert_eq!(original.reference_count(), 3);
        drop(first);
        assert_eq!(original.reference_count(), 2);
        assert_eq!(COUNTED_DEALLOCATIONS.load(Ordering::Relaxed), 0);
        drop(second);
        assert_eq!(original.reference_count(), 1);
        assert_eq!(COUNTED_DEALLOCATIONS.load(Ordering::Relaxed), 0);
        drop(original);
        assert_eq!(COUNTED_DEALLOCATIONS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn distinct_allocations_do_not_share_identity() {
        let left = Vmo::try_from_slice(b"same bytes").unwrap();
        let right = Vmo::try_from_slice(b"same bytes").unwrap();
        assert!(!left.same_vmo(&right));
    }

    #[test]
    fn read_range_is_zero_copy_and_truncates_at_eof() {
        let vmo = Vmo::try_from_slice(b"012345").unwrap();
        assert_eq!(vmo.read_range(2, 3), Ok(&b"234"[..]));
        assert_eq!(vmo.read_range(4, 20), Ok(&b"45"[..]));
        assert_eq!(vmo.read_range(6, 20), Ok(&[][..]));

        let whole = vmo.read_range(0, 6).unwrap();
        let middle = vmo.read_range(2, 2).unwrap();
        assert_eq!(unsafe { whole.as_ptr().add(2) }, middle.as_ptr());
    }

    #[test]
    fn read_rejects_bad_ranges_without_touching_output() {
        let vmo = Vmo::try_from_slice(b"abc").unwrap();
        let mut out = [0xcc; 4];
        assert_eq!(vmo.read(4, &mut out), Err(VmoReadError::OutOfRange));
        assert_eq!(out, [0xcc; 4]);
        assert_eq!(
            vmo.read_range(2, usize::MAX),
            Err(VmoReadError::RangeOverflow)
        );
        assert_eq!(vmo.read_range(4, 0), Err(VmoReadError::OutOfRange));
    }

    #[test]
    fn read_supports_zero_and_partial_copies() {
        let vmo = Vmo::try_from_slice(b"abcdef").unwrap();
        let mut empty = [];
        assert_eq!(vmo.read(3, &mut empty), Ok(0));

        let mut out = [0xcc; 4];
        assert_eq!(vmo.read(4, &mut out), Ok(2));
        assert_eq!(out, [b'e', b'f', 0xcc, 0xcc]);
        assert_eq!(vmo.read(6, &mut out), Ok(0));
        assert_eq!(out, [b'e', b'f', 0xcc, 0xcc]);
    }

    #[test]
    fn debug_reports_identity_and_length_without_dumping_bytes() {
        let vmo = Vmo::try_from_slice(b"secret").unwrap();
        let rendered = std::format!("{vmo:?}");
        assert!(rendered.starts_with("Vmo { inner: "));
        assert!(rendered.contains("len: 6"));
        assert!(!rendered.contains("secret"));
    }
}
