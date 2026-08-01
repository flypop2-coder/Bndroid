use core::fmt;
use core::hint::spin_loop;
use core::ptr::{read_volatile, write_volatile};

const UART0_BASE: usize = 0x0900_0000;
const DATA_REGISTER: usize = 0x00;
const FLAG_REGISTER: usize = 0x18;
const FLAG_TX_FIFO_FULL: u32 = 1 << 5;

pub struct Uart;

impl Uart {
    fn write_byte(&mut self, byte: u8) {
        let flags = (UART0_BASE + FLAG_REGISTER) as *const u32;
        while unsafe { read_volatile(flags) } & FLAG_TX_FIFO_FULL != 0 {
            spin_loop();
        }
        unsafe {
            write_volatile((UART0_BASE + DATA_REGISTER) as *mut u32, byte as u32);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for byte in value.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}
