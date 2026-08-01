#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text._start")]
pub extern "C" fn _start(startup_handle: u64, raw_image_id: u64) -> ! {
    bndroid_init::client_entry(startup_handle, raw_image_id)
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    bndroid_init::child_panic()
}
