//! Boot-kernel ownership wrapper for the physical-input broker.

use core::cell::UnsafeCell;

use bndroid_kernel::input_broker::{
    InputAcquireError, InputBroker, InputBrokerError, InputBrokerSnapshot, InputCapability,
    InputEnqueueEvidence, InputReleaseEvidence, InputSessionInfo,
};

pub const KEYBOARD_DEVICE_ID: u8 = 1;
pub const TABLET_DEVICE_ID: u8 = 2;

struct GlobalInputBroker(UnsafeCell<InputBroker>);

// Every access is serialized on the boot CPU with local IRQ masking. The
// wrapper asserts that invariant before exposing the inner mutable reference.
unsafe impl Sync for GlobalInputBroker {}

static INPUT_BROKER: GlobalInputBroker = GlobalInputBroker(UnsafeCell::new(InputBroker::new()));

pub fn acquire(process_id: u64) -> Result<InputCapability, InputAcquireError> {
    with_broker(|broker| broker.acquire(process_id))
}

pub fn enqueue_key(code: u16, value: u8) -> Result<InputEnqueueEvidence, InputBrokerError> {
    with_broker(|broker| broker.enqueue_key(KEYBOARD_DEVICE_ID, code, value))
}

pub fn enqueue_pointer(
    x: u16,
    y: u16,
    pressed: bool,
) -> Result<InputEnqueueEvidence, InputBrokerError> {
    with_broker(|broker| broker.enqueue_pointer(TABLET_DEVICE_ID, x, y, pressed))
}

pub fn read(
    capability: &InputCapability,
    process_id: u64,
) -> Result<Option<bndr_abi::InputEvent>, InputBrokerError> {
    with_broker(|broker| broker.read(capability, process_id))
}

pub fn session_info(
    capability: &InputCapability,
    process_id: u64,
) -> Result<InputSessionInfo, InputBrokerError> {
    with_broker(|broker| broker.session_info(capability, process_id))
}

pub fn release(
    capability: &InputCapability,
    process_id: u64,
) -> Result<InputReleaseEvidence, InputBrokerError> {
    with_broker(|broker| broker.release(capability, process_id))
}

pub fn release_process(process_id: u64) -> Option<InputReleaseEvidence> {
    with_broker(|broker| broker.release_process(process_id))
}

pub fn snapshot() -> InputBrokerSnapshot {
    with_broker(|broker| broker.snapshot())
}

fn with_broker<R>(operation: impl FnOnce(&mut InputBroker) -> R) -> R {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("physical-input broker accessed with IRQ enabled");
    }
    operation(unsafe { &mut *INPUT_BROKER.0.get() })
}
