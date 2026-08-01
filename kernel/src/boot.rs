unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

pub fn kernel_start() -> usize {
    core::ptr::addr_of!(__kernel_start) as usize
}

pub fn kernel_end() -> usize {
    core::ptr::addr_of!(__kernel_end) as usize
}
