//! Suspended Binder reads retain their original open file, including when a
//! sibling closes/reuses the fd. No user copy occurs in a foreign process.
use super::*;
use crate::linux_binder_driver::{self as driver, Call, ReadCall};
use bndroid_kernel::linux_binder::Delivery;

struct Pending {
    call: ReadCall,
    ready: Option<Result<Delivery, i64>>,
}
struct Reads(UnsafeCell<[Option<Pending>; CONTEXT_COUNT]>);
unsafe impl Sync for Reads {}
static READS: Reads = Reads(UnsafeCell::new([const { None }; CONTEXT_COUNT]));
fn slot(context: usize) -> &'static mut Option<Pending> {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut (*READS.0.get())[context] }
}
pub(super) fn dispatch(frame: *mut TrapFrame, r: &[u64; 31]) -> Option<*mut TrapFrame> {
    let result = driver::ioctl(r, current_tid() as u32)?;
    let context = current_running_context();
    let value = match result {
        Ok(Call::Complete(value)) => value,
        Err(error) => errno(error),
        Ok(Call::Wait(call)) => {
            if let Err(error) = queue().insert(
                context,
                Wait {
                    process: owner(context),
                    address: 0,
                    bitset: !0,
                    deadline: None,
                },
            ) {
                driver::cancel(&call, None);
                return Some(runtime::complete(frame, errno(error)));
            }
            assert!(slot(context).is_none());
            *slot(context) = Some(Pending { call, ready: None });
            WAITS.fetch_add(1, Ordering::Relaxed);
            return Some(block(frame, ThreadState::Waiting));
        }
    };
    Some(runtime::complete(frame, value))
}
pub(super) fn wake_ready() {
    for (context, state) in CONTEXTS.iter().enumerate().skip(DYNAMIC_USER_CONTEXT_START) {
        if state.state.load(Ordering::Relaxed) != STATE_WAITING {
            continue;
        }
        let Some(mut pending) = slot(context).take() else {
            continue;
        };
        pending.ready = match driver::poll(&pending.call) {
            Ok(None) => None,
            Ok(Some(d)) => Some(Ok(d)),
            Err(e) => Some(Err(e)),
        };
        let ready = pending.ready.is_some();
        *slot(context) = Some(pending);
        if ready {
            assert!(queue().cancel(context));
            wake(1 << context, 0);
            WAKES.fetch_add(1, Ordering::Relaxed);
        }
    }
}
pub(crate) fn complete_ready(frame: *mut TrapFrame) {
    let Some(pending) = slot(current_running_context()).take() else {
        return;
    };
    let result = match pending
        .ready
        .expect("only a prepared Binder read becomes runnable")
    {
        Ok(delivery) => driver::finish(&pending.call, delivery),
        Err(error) => Err(error),
    };
    runtime::complete(frame, result.map(|_| 0).unwrap_or_else(errno));
}
pub(super) fn cancel(context: usize) {
    if let Some(pending) = slot(context).take() {
        driver::cancel(&pending.call, pending.ready.and_then(Result::ok));
    }
}
pub(super) fn live() -> usize {
    (0..CONTEXT_COUNT).filter(|c| slot(*c).is_some()).count()
}
