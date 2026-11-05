//! Linux AArch64 signal wire data. No kernel pointer is serialized to userspace.
use alloc::vec::Vec;

pub const SIGKILL: u8 = 9;
pub const SIGSEGV: u8 = 11;
pub const SIGSTOP: u8 = 19;
pub const UNBLOCKABLE: u64 = (1 << (SIGKILL - 1)) | (1 << (SIGSTOP - 1));
pub const SIGINFO: u64 = 4;
pub const RESTORER: u64 = 0x0400_0000;
pub const ONSTACK: u64 = 0x0800_0000;
pub const RESTART: u64 = 0x1000_0000;
pub const NODEFER: u64 = 0x4000_0000;
pub const RESETHAND: u64 = 0x8000_0000;
pub const EXPOSE_TAGBITS: u64 = 0x800;
pub const SUPPORTED_FLAGS: u64 =
    3 | SIGINFO | RESTORER | ONSTACK | RESTART | NODEFER | RESETHAND | EXPOSE_TAGBITS;
pub const DISABLE: u32 = 2;
pub const AUTODISARM: u32 = 0x8000_0000;
pub const MIN_STACK: u64 = 5120;
pub const FRAME_BYTES: usize = 4688;
pub const RECORD_BYTES: usize = FRAME_BYTES + 16;
pub const UC_OFFSET: usize = 128;
pub const CONTEXT_OFFSET: usize = 304;
pub const RESERVED_OFFSET: usize = 592;
pub const FPSIMD_MAGIC: u32 = 0x4650_8001;
pub const ESR_MAGIC: u32 = 0x4553_5201;
pub const FPSIMD_SIZE: usize = 528;

pub fn bit(signal: u8) -> u64 {
    1u64 << (signal - 1)
}
pub fn number(raw: u64) -> Result<u8, i64> {
    if (1..=64).contains(&raw) {
        Ok(raw as u8)
    } else {
        Err(22)
    }
}
pub fn ignored_by_default(signal: u8) -> bool {
    matches!(signal, 17 | 23 | 28)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Action {
    pub handler: u64,
    pub flags: u64,
    pub restorer: u64,
    pub mask: u64,
}
impl Action {
    pub const EMPTY: Self = Self {
        handler: 0,
        flags: 0,
        restorer: 0,
        mask: 0,
    };
    pub fn decode(bytes: &[u8; 32]) -> Self {
        Self {
            handler: u64_at(bytes, 0),
            flags: u64_at(bytes, 8) & SUPPORTED_FLAGS,
            restorer: u64_at(bytes, 16),
            mask: u64_at(bytes, 24) & !UNBLOCKABLE,
        }
    }
    pub fn encode(self) -> [u8; 32] {
        let mut out = [0; 32];
        for (offset, value) in [
            (0, self.handler),
            (8, self.flags),
            (16, self.restorer),
            (24, self.mask),
        ] {
            put64(&mut out, offset, value);
        }
        out
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AltStack {
    pub address: u64,
    pub size: u64,
    pub flags: u32,
}
impl AltStack {
    pub const EMPTY: Self = Self {
        address: 0,
        size: 0,
        flags: DISABLE,
    };
    pub fn contains(self, sp: u64) -> bool {
        self.flags & DISABLE == 0 && sp >= self.address && sp - self.address < self.size
    }
    pub fn encode(self, sp: u64) -> [u8; 24] {
        let mut out = [0; 24];
        put64(&mut out, 0, self.address);
        put32(&mut out, 8, self.flags | u32::from(self.contains(sp)));
        put64(&mut out, 16, self.size);
        out
    }
    pub fn decode(bytes: &[u8], limit: u64, restoring: bool) -> Result<Self, i64> {
        let mut flags = u32_at(bytes, 8);
        if restoring {
            flags &= !1;
        }
        if flags & !(DISABLE | AUTODISARM) != 0 {
            return Err(22);
        }
        if flags & DISABLE != 0 {
            return Ok(Self::EMPTY);
        }
        let address = u64_at(bytes, 0);
        let size = u64_at(bytes, 16);
        if size < MIN_STACK {
            return Err(12);
        }
        if address == 0 || address.checked_add(size).is_none_or(|end| end > limit) {
            return Err(14);
        }
        Ok(Self {
            address,
            size,
            flags,
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Info {
    pub signal: u8,
    pub code: i32,
    pub pid: u32,
    pub uid: u32,
    pub address: u64,
    pub value: u64,
}
impl Info {
    pub fn encode(self, tagged: bool) -> [u8; 128] {
        let mut out = [0; 128];
        put32(&mut out, 0, self.signal as u32);
        put32(&mut out, 8, self.code as u32);
        if self.code > 0 {
            put64(
                &mut out,
                16,
                if tagged {
                    self.address
                } else {
                    self.address & 0x00ff_ffff_ffff_ffff
                },
            );
        } else {
            put32(&mut out, 16, self.pid);
            put32(&mut out, 20, self.uid);
            put64(&mut out, 24, self.value);
        }
        out
    }
}
#[derive(Clone, Copy)]
pub struct Pending {
    entries: [Option<Info>; 32],
    kill: Option<Info>,
}
impl Pending {
    pub const EMPTY: Self = Self {
        entries: [None; 32],
        kill: None,
    };
    pub fn mask(&self) -> u64 {
        self.entries.iter().flatten().fold(
            if self.kill.is_some() { bit(SIGKILL) } else { 0 },
            |mask, i| mask | bit(i.signal),
        )
    }
    pub fn push(&mut self, info: Info) -> Result<(), i64> {
        // SIGKILL must remain deliverable even when queued realtime signals
        // exhaust the bounded queue. Its slot never evicts another signal.
        if info.signal == SIGKILL {
            self.kill.get_or_insert(info);
            return Ok(());
        }
        if info.signal < 32 && self.mask() & bit(info.signal) != 0 {
            return Ok(());
        }
        *self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(11)? = Some(info);
        Ok(())
    }
    pub fn pop(&mut self, eligible: u64) -> Option<Info> {
        if eligible & bit(SIGKILL) != 0 && self.kill.is_some() {
            return self.kill.take();
        }
        let signal = (self.mask() & eligible).trailing_zeros();
        if signal >= 64 {
            return None;
        }
        let index = self
            .entries
            .iter()
            .position(|e| e.is_some_and(|i| i.signal as u32 == signal + 1))?;
        let result = self.entries[index].take();
        self.entries.copy_within(index + 1.., index);
        self.entries[31] = None;
        result
    }
    pub fn discard(&mut self, signal: u8) {
        while self.pop(bit(signal)).is_some() {}
    }
}

pub struct Context {
    pub registers: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub pstate: u64,
    pub vectors: [u128; 32],
    pub fpsr: u32,
    pub fpcr: u32,
    pub mask: u64,
    pub stack: AltStack,
}
impl Context {
    pub fn encode(&self, info: Info, esr: u64, tagged: bool) -> Result<Vec<u8>, i64> {
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(RECORD_BYTES).map_err(|_| 12)?;
        bytes.resize(RECORD_BYTES, 0);
        bytes[..128].copy_from_slice(&info.encode(tagged));
        bytes[144..168].copy_from_slice(&self.stack.encode(self.sp));
        put64(&mut bytes, 168, self.mask);
        put64(&mut bytes, CONTEXT_OFFSET, info.address);
        for (n, value) in self.registers.iter().enumerate() {
            put64(&mut bytes, 312 + n * 8, *value);
        }
        put64(&mut bytes, 560, self.sp);
        put64(&mut bytes, 568, self.pc);
        put64(&mut bytes, 576, self.pstate);
        put32(&mut bytes, RESERVED_OFFSET, FPSIMD_MAGIC);
        put32(&mut bytes, RESERVED_OFFSET + 4, FPSIMD_SIZE as u32);
        put32(&mut bytes, RESERVED_OFFSET + 8, self.fpsr);
        put32(&mut bytes, RESERVED_OFFSET + 12, self.fpcr);
        for (n, value) in self.vectors.iter().enumerate() {
            bytes[608 + n * 16..624 + n * 16].copy_from_slice(&value.to_le_bytes());
        }
        if esr != 0 {
            put32(&mut bytes, 1120, ESR_MAGIC);
            put32(&mut bytes, 1124, 16);
            put64(&mut bytes, 1128, esr);
        }
        put64(&mut bytes, FRAME_BYTES, self.registers[29]);
        put64(&mut bytes, FRAME_BYTES + 8, self.registers[30]);
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8], limit: u64) -> Result<Self, i64> {
        if bytes.len() < FRAME_BYTES {
            return Err(14);
        }
        let pstate = u64_at(bytes, 576);
        // Retain only the supported user condition flags; EL, DAIF, PAN/UAO,
        // single-step and other privileged controls cannot come from a frame.
        if pstate & !0xf000_0000 != 0x340 {
            return Err(22);
        }
        let stack = AltStack::decode(&bytes[144..168], limit, true)?;
        let mut vectors = [0; 32];
        let (mut fpcr, mut fpsr) = (0, 0);
        let (mut at, mut seen, mut esr_seen, mut terminated) =
            (RESERVED_OFFSET, false, false, false);
        while at + 8 <= FRAME_BYTES {
            let magic = u32_at(bytes, at);
            let size = u32_at(bytes, at + 4) as usize;
            if magic == 0 && size == 0 {
                terminated = true;
                break;
            }
            if size < 16 || !size.is_multiple_of(16) || at + size > FRAME_BYTES {
                return Err(22);
            }
            match magic {
                FPSIMD_MAGIC if !seen && size == FPSIMD_SIZE => {
                    seen = true;
                    fpsr = u32_at(bytes, at + 8);
                    fpcr = u32_at(bytes, at + 12);
                    for (n, value) in vectors.iter_mut().enumerate() {
                        *value = u128::from_le_bytes(
                            bytes[at + 16 + n * 16..at + 32 + n * 16]
                                .try_into()
                                .unwrap(),
                        );
                    }
                }
                ESR_MAGIC if !esr_seen && size == 16 => {
                    esr_seen = true;
                }
                _ => return Err(22),
            }
            at += size;
        }
        if !seen || !terminated {
            return Err(22);
        }
        let mut registers = [0; 31];
        for (n, value) in registers.iter_mut().enumerate() {
            *value = u64_at(bytes, 312 + n * 8);
        }
        Ok(Self {
            registers,
            sp: u64_at(bytes, 560),
            pc: u64_at(bytes, 568),
            pstate,
            vectors,
            fpsr,
            fpcr,
            mask: u64_at(bytes, 168) & !UNBLOCKABLE,
            stack,
        })
    }
}
pub fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
pub fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    fn info(signal: u8, value: u64) -> Info {
        Info {
            signal,
            code: -1,
            pid: 2,
            uid: 10000,
            address: 0,
            value,
        }
    }
    #[test]
    fn standard_coalescing_realtime_fifo_priority_and_capacity_are_distinct() {
        let mut p = Pending::EMPTY;
        p.push(info(12, 1)).unwrap();
        p.push(info(12, 2)).unwrap();
        p.push(info(40, 3)).unwrap();
        p.push(info(40, 4)).unwrap();
        p.push(info(35, 5)).unwrap();
        assert_eq!(p.pop(!0), Some(info(12, 1)));
        assert_eq!(p.pop(bit(40)), Some(info(40, 3)));
        assert_eq!(p.pop(!0), Some(info(35, 5)));
        assert_eq!(p.pop(!0), Some(info(40, 4)));
        for n in 0..32 {
            p.push(info(40, n)).unwrap();
        }
        assert_eq!(p.push(info(41, 0)), Err(11));
        p.push(info(SIGKILL, 7)).unwrap();
        assert_eq!(p.pop(!0), Some(info(SIGKILL, 7)));
        assert_eq!(p.pop(!0), Some(info(40, 0)));
        p.discard(40);
        assert_eq!(p.mask(), 0);
    }
    #[test]
    fn frame_roundtrip_preserves_simd_mask_and_rejects_privilege_or_malformed_extensions() {
        let c = Context {
            registers: [123; 31],
            sp: 0x8800,
            pc: 0x9000,
            pstate: 0x60000340,
            vectors: [u128::MAX - 7; 32],
            fpsr: 27,
            fpcr: 0x400000,
            mask: !0,
            stack: AltStack::EMPTY,
        };
        let bytes = c.encode(info(35, 7), 0x9000000f, false).unwrap();
        let decoded = Context::decode(&bytes, 1 << 39).unwrap();
        assert_eq!(decoded.registers, c.registers);
        assert_eq!(decoded.vectors, c.vectors);
        assert_eq!(decoded.sp, c.sp);
        assert_eq!(decoded.pc, c.pc);
        assert_eq!(decoded.mask, !UNBLOCKABLE);
        assert_eq!(decoded.fpcr, c.fpcr);
        assert_eq!(decoded.fpsr, c.fpsr);
        let mut corrupt = bytes.clone();
        put64(&mut corrupt, 576, 0x345);
        assert!(Context::decode(&corrupt, 1 << 39).is_err());
        let mut corrupt = bytes.clone();
        put32(&mut corrupt, 596, 16);
        assert!(Context::decode(&corrupt, 1 << 39).is_err());
        let mut corrupt = bytes.clone();
        put32(&mut corrupt, 1120, FPSIMD_MAGIC);
        assert!(Context::decode(&corrupt, 1 << 39).is_err());
        assert!(Context::decode(&bytes[..500], 1 << 39).is_err());
    }
}
