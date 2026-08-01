use core::fmt::{self, Write};

use crate::uart::Uart;

pub fn write(args: fmt::Arguments<'_>) {
    // Formatting emits many individual MMIO bytes. Keep one logical record
    // indivisible with respect to timer/device IRQs and therefore scheduler
    // preemption on the current single boot CPU. Restoring the exact saved
    // DAIF value is required because logging is also used from IRQ-masked
    // syscall, reaper, and panic paths.
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let mut uart = Uart;
    let _ = uart.write_fmt(args);
    crate::arch::aarch64::restore_daif(saved_daif);
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::log::write(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($($arg:tt)*) => {
        $crate::kprint!("{}\n", format_args!($($arg)*))
    };
}
