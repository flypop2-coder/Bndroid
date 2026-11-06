//! Bounded per-open request/response state for native service adapters. Dup
//! shares the exchange; a newly opened descriptor never inherits a response.
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Access,
    Context,
    Property,
}
#[derive(Default)]
pub struct State {
    pub connected: bool,
    pub request: Vec<u8>,
    pub response: Option<Vec<u8>>,
    pub read: usize,
}
pub struct Control {
    locked: AtomicBool,
    state: UnsafeCell<State>,
}
unsafe impl Send for Control {}
unsafe impl Sync for Control {}
impl Default for Control {
    fn default() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(State::default()),
        }
    }
}
impl Control {
    pub fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        while self.locked.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        let result = f(unsafe { &mut *self.state.get() });
        self.locked.store(false, Ordering::Release);
        result
    }
}
