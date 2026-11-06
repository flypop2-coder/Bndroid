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
pub const BC_INCREFS_DONE: u32 = 0x4010_6308;
pub const BC_ACQUIRE_DONE: u32 = 0x4010_6309;
pub const BC_REGISTER_LOOPER: u32 = 0x0000_630b;
pub const BC_ENTER_LOOPER: u32 = 0x0000_630c;
pub const BC_EXIT_LOOPER: u32 = 0x0000_630d;
pub const BC_REQUEST_DEATH: u32 = 0x400c_630e;
pub const BC_CLEAR_DEATH: u32 = 0x400c_630f;
pub const BC_DEAD_DONE: u32 = 0x4008_6310;
pub const BR_TRANSACTION: u32 = 0x8040_7202;
pub const BR_TRANSACTION_SEC_CTX: u32 = 0x8048_7202;
pub const BR_REPLY: u32 = 0x8040_7203;
pub const BR_DEAD_REPLY: u32 = 0x0000_7205;
pub const BR_COMPLETE: u32 = 0x0000_7206;
pub const BR_INCREFS: u32 = 0x8010_7207;
pub const BR_ACQUIRE: u32 = 0x8010_7208;
pub const BR_RELEASE: u32 = 0x8010_7209;
pub const BR_DECREFS: u32 = 0x8010_720a;
pub const BR_NOOP: u32 = 0x0000_720c;
pub const BR_SPAWN_LOOPER: u32 = 0x0000_720d;
pub const BR_DEAD_BINDER: u32 = 0x8008_720f;
pub const BR_CLEAR_DEATH_DONE: u32 = 0x8008_7210;
pub const BR_FAILED_REPLY: u32 = 0x0000_7211;
pub const ONE_WAY: u32 = 1;
/// Framework AppOps collection metadata. Forward to userspace unchanged;
/// this bit grants no native permission and changes no driver scheduling.
pub const COLLECT_NOTED_APP_OPS: u32 = 2;
pub const ACCEPT_FDS: u32 = 0x10;
pub const MAX_MAPPING: u64 = 4 * 1024 * 1024;
pub const MAX_OBJECTS: u64 = 256;
pub const TYPE_BINDER: u32 = 0x7362_2a85;
pub const TYPE_WEAK_BINDER: u32 = 0x7762_2a85;
pub const TYPE_HANDLE: u32 = 0x7368_2a85;
pub const TYPE_WEAK_HANDLE: u32 = 0x7768_2a85;
pub const TYPE_FD: u32 = 0x6664_2a85;

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
    pub fn object_offsets(self, bytes: &[u8]) -> Result<alloc::vec::Vec<usize>, i64> {
        self.validate_payload()?;
        if bytes.len() != self.payload_length()? {
            return Err(22);
        }
        let mut result = alloc::vec::Vec::new();
        result
            .try_reserve_exact(self.offsets_size as usize / 8)
            .map_err(|_| 12)?;
        let offsets = (self.data_size as usize + 7) & !7;
        let mut minimum = 0;
        for i in 0..self.offsets_size as usize / 8 {
            let at = usize::try_from(u64_at(bytes, offsets + i * 8)).map_err(|_| 22)?;
            if !at.is_multiple_of(4)
                || at < minimum
                || at
                    .checked_add(24)
                    .is_none_or(|end| end > self.data_size as usize)
            {
                return Err(22);
            }
            if !matches!(
                u32_at(bytes, at),
                TYPE_BINDER | TYPE_WEAK_BINDER | TYPE_HANDLE | TYPE_WEAK_HANDLE | TYPE_FD
            ) {
                return Err(95);
            }
            result.push(at);
            minimum = at + 24;
        }
        Ok(result)
    }
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
        if self.flags & !(ONE_WAY | COLLECT_NOTED_APP_OPS | 8 | ACCEPT_FDS | 32) != 0 {
            return Err(95);
        }
        if self.data_size > MAX_MAPPING
            || self.data.checked_add(self.data_size).is_none()
            || !self.offsets_size.is_multiple_of(8)
            || self.offsets_size > MAX_OBJECTS * 8
            || self.offsets.checked_add(self.offsets_size).is_none()
            || self.payload_length()? as u64 > MAX_MAPPING
        {
            return Err(22);
        }
        Ok(())
    }
    pub fn payload_length(self) -> Result<usize, i64> {
        if self.offsets_size == 0 {
            return usize::try_from(self.data_size).map_err(|_| 22);
        }
        self.data_size
            .checked_add(7)
            .map(|n| n & !7)
            .and_then(|n| n.checked_add(self.offsets_size))
            .and_then(|n| usize::try_from(n).ok())
            .ok_or(22)
    }
}

pub fn command_bytes(command: u32) -> Result<usize, i64> {
    Ok(match command {
        BC_TRANSACTION | BC_REPLY => 64,
        BC_FREE_BUFFER | BC_DEAD_DONE => 8,
        BC_INCREFS | BC_ACQUIRE | BC_RELEASE | BC_DECREFS => 4,
        BC_INCREFS_DONE | BC_ACQUIRE_DONE => 16,
        BC_REGISTER_LOOPER | BC_ENTER_LOOPER | BC_EXIT_LOOPER => 0,
        BC_REQUEST_DEATH | BC_CLEAR_DEATH => 12,
        _ => return Err(22),
    })
}
