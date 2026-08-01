// Kernel-wide process/scheduler capacity contract.
//
// A dynamic process slot owns the scheduler context, object-wait state, and
// kernel stack with the same zero-based dynamic index.  Keeping this limit in
// one module prevents the process table from accepting an identity for which
// the scheduler cannot construct (or later prove stopped) an execution slot.
#[cfg(feature = "resident-platform-shutdown-runtime")]
pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 9;
#[cfg(all(
    not(feature = "resident-platform-shutdown-runtime"),
    feature = "androidbox-process0"
))]
pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 8;
#[cfg(all(
    not(feature = "resident-platform-shutdown-runtime"),
    not(feature = "androidbox-process0"),
    not(feature = "input-server-runtime")
))]
pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 7;
#[cfg(all(
    not(feature = "resident-platform-shutdown-runtime"),
    not(feature = "androidbox-process0"),
    feature = "input-server-runtime"
))]
pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 8;
pub(crate) const PROCESS_CAPACITY: usize = 1 + DYNAMIC_PROCESS_CAPACITY;
