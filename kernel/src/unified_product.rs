//! Kernel-owned M67 interactive convergence seal.
//!
//! Userspace cannot set this bit. The boot monitor publishes it only after the
//! complete M45 pointer, keyboard, text, soft-keyboard, graphics, capability,
//! process, and wait ledgers have converged in one live system instance.

use core::sync::atomic::{AtomicBool, Ordering};

static UI_CONVERGED: AtomicBool = AtomicBool::new(false);

pub fn publish_ui_converged() -> bool {
    UI_CONVERGED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

pub fn ui_converged() -> bool {
    UI_CONVERGED.load(Ordering::Acquire)
}
