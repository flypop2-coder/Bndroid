use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    crate::kprintln!("kernel panic: {}", info);
    crate::arch::aarch64::halt()
}
