//! LP64 Binder wire values. Decode bytes explicitly: user buffers need not be
//! Rust-aligned, and no guest pointer is dereferenced by the protocol model.
pub const WRITE_READ: u64 = 0xc030_6201;
pub const SET_MAX_THREADS: u64 = 0x4004_6205;
pub const SET_CONTEXT_MGR: u64 = 0x4004_6207;
pub const THREAD_EXIT: u64 = 0x4004_6208;
pub const VERSION: u64 = 0xc004_6209;
pub const SET_CONTEXT_MGR_EXT: u64 = 0x4018_620d;
pub const BC_TRANSACTION: u32 = 0x4040_6300;
pub const BC_REPLY: u32 = 0x4040_6301;
pub const BC_FREE_BUFFER: u32 = 0x4008_6303;
pub const BC_INCREFS: u32 = 0x4004_6304;
pub const BC_ACQUIRE: u32 = 0x4004_6305;
pub const BC_RELEASE: u32 = 0x4004_6306;
pub const BC_DECREFS: u32 = 0x4004_6307;
pub const BC_REGISTER_LOOPER: u32 = 0x0000_630b;
pub const BC_ENTER_LOOPER: u32 = 0x0000_630c;
pub const BC_EXIT_LOOPER: u32 = 0x0000_630d;
pub const BC_REQUEST_DEATH: u32 = 0x400c_630e;
pub const BC_CLEAR_DEATH: u32 = 0x400c_630f;
pub const BC_DEAD_DONE: u32 = 0x4008_6310;
pub const BR_TRANSACTION: u32 = 0x8040_7202;
pub const BR_REPLY: u32 = 0x8040_7203;
pub const BR_DEAD_REPLY: u32 = 0x0000_7205;
pub const BR_COMPLETE: u32 = 0x0000_7206;
pub const BR_NOOP: u32 = 0x0000_720c;
pub const BR_SPAWN_LOOPER: u32 = 0x0000_720d;
pub const BR_DEAD_BINDER: u32 = 0x8008_720f;
pub const BR_CLEAR_DEATH_DONE: u32 = 0x8008_7210;
pub const BR_FAILED_REPLY: u32 = 0x0000_7211;
pub const ONE_WAY: u32 = 1;
pub const MAX_MAPPING: u64 = 4 * 1024 * 1024;

pub fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
pub fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
pub fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug)]
pub struct WriteRead {
    pub write_size: u64,
    pub write_consumed: u64,
    pub write_buffer: u64,
    pub read_size: u64,
    pub read_consumed: u64,
    pub read_buffer: u64,
}
impl WriteRead {
    pub fn decode(bytes: &[u8; 48]) -> Result<Self, i64> {
        let result = Self {
            write_size: u64_at(bytes, 0),
            write_consumed: u64_at(bytes, 8),
            write_buffer: u64_at(bytes, 16),
            read_size: u64_at(bytes, 24),
            read_consumed: u64_at(bytes, 32),
            read_buffer: u64_at(bytes, 40),
        };
        if result.write_consumed > result.write_size
            || result.read_consumed > result.read_size
            || result.write_size > MAX_MAPPING
            || result.read_size > MAX_MAPPING
            || result.write_buffer.checked_add(result.write_size).is_none()
            || result.read_buffer.checked_add(result.read_size).is_none()
        {
            return Err(22);
        }
        Ok(result)
    }
    pub fn encode(self) -> [u8; 48] {
        let mut result = [0; 48];
        for (index, value) in [
            self.write_size,
            self.write_consumed,
            self.write_buffer,
            self.read_size,
            self.read_consumed,
            self.read_buffer,
        ]
        .into_iter()
        .enumerate()
        {
            put64(&mut result, index * 8, value);
        }
        result
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transaction {
    pub target: u64,
    pub cookie: u64,
    pub code: u32,
    pub flags: u32,
    pub sender_pid: u32,
    pub sender_uid: u32,
    pub data_size: u64,
    pub offsets_size: u64,
    pub data: u64,
    pub offsets: u64,
}
impl Transaction {
    pub fn decode(bytes: &[u8; 64]) -> Self {
        Self {
            target: u64_at(bytes, 0),
            cookie: u64_at(bytes, 8),
            code: u32_at(bytes, 16),
            flags: u32_at(bytes, 20),
            sender_pid: u32_at(bytes, 24),
            sender_uid: u32_at(bytes, 28),
            data_size: u64_at(bytes, 32),
            offsets_size: u64_at(bytes, 40),
            data: u64_at(bytes, 48),
            offsets: u64_at(bytes, 56),
        }
    }
    pub fn encode(self) -> [u8; 64] {
        let mut b = [0; 64];
        for (at, value) in [
            (0, self.target),
            (8, self.cookie),
            (32, self.data_size),
            (40, self.offsets_size),
            (48, self.data),
            (56, self.offsets),
        ] {
            put64(&mut b, at, value);
        }
        for (at, value) in [
            (16, self.code),
            (20, self.flags),
            (24, self.sender_pid),
            (28, self.sender_uid),
        ] {
            put32(&mut b, at, value);
        }
        b
    }
    pub fn validate_payload(self) -> Result<(), i64> {
        // Object/FD translation and SG buffers need capability ownership. They
        // are rejected explicitly until that transport is implemented.
        if self.offsets_size != 0 || self.flags & !(ONE_WAY | 8 | 16 | 32) != 0 {
            return Err(95);
        }
        if self.data_size > MAX_MAPPING || self.data.checked_add(self.data_size).is_none() {
            return Err(22);
        }
        Ok(())
    }
}

pub fn command_bytes(command: u32) -> Result<usize, i64> {
    Ok(match command {
        BC_TRANSACTION | BC_REPLY => 64,
        BC_FREE_BUFFER | BC_DEAD_DONE => 8,
        BC_INCREFS | BC_ACQUIRE | BC_RELEASE | BC_DECREFS => 4,
        BC_REGISTER_LOOPER | BC_ENTER_LOOPER | BC_EXIT_LOOPER => 0,
        BC_REQUEST_DEATH | BC_CLEAR_DEATH => 12,
        _ => return Err(22),
    })
}
