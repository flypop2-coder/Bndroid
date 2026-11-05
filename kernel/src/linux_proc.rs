//! Bounded formatting of process-owned Linux proc snapshots.
use alloc::vec::Vec;
use core::fmt::{self, Write};

pub const MAX_SNAPSHOT: usize = 128 * 1024;
pub struct Text(Vec<u8>);
impl Text {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}
impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}
impl Write for Text {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self
            .0
            .len()
            .checked_add(text.len())
            .is_none_or(|n| n > MAX_SNAPSHOT)
        {
            return Err(fmt::Error);
        }
        self.0.try_reserve(text.len()).map_err(|_| fmt::Error)?;
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

pub struct Stat {
    pub pid: u32,
    pub parent: u32,
    pub name: [u8; 16],
    pub state: char,
    pub user_ticks: u64,
    pub kernel_ticks: u64,
    pub threads: u64,
    pub start_ticks: u64,
    pub virtual_bytes: u64,
    pub resident_pages: u64,
    pub resident_limit: u64,
    pub code_start: u64,
    pub code_end: u64,
    pub stack_start: u64,
    pub blocked_signals: u64,
}
impl Stat {
    pub fn text(&self) -> Result<Vec<u8>, i64> {
        let mut text = Text::new();
        let mut name = self.name;
        // proc comm is a byte string. Keep delimiters parseable, including
        // spaces/parentheses, and escape newlines/control bytes in this view.
        for byte in &mut name {
            if *byte != 0 && !byte.is_ascii_graphic() && *byte != b' ' {
                *byte = b'?';
            }
        }
        let n = name.iter().position(|b| *b == 0).unwrap_or(name.len());
        let comm = core::str::from_utf8(&name[..n]).map_err(|_| 22)?;
        write!(text, "{} ({}) {}", self.pid, comm, self.state).map_err(|_| 12)?;
        let mut fields = [0u64; 49]; // fields 4..52, inclusive.
        for (field, value) in [
            (4, self.parent as u64),
            (5, self.pid as u64),
            (6, self.pid as u64),
            (14, self.user_ticks),
            (15, self.kernel_ticks),
            (18, 20),
            (20, self.threads),
            (22, self.start_ticks),
            (23, self.virtual_bytes),
            (24, self.resident_pages),
            (25, self.resident_limit),
            (26, self.code_start),
            (27, self.code_end),
            (28, self.stack_start),
            (32, self.blocked_signals),
        ] {
            fields[field - 4] = value;
        }
        for value in fields {
            write!(text, " {value}").map_err(|_| 12)?;
        }
        writeln!(text).map_err(|_| 12)?;
        Ok(text.finish())
    }
}

#[derive(Clone, Copy)]
pub struct Map {
    pub start: u64,
    pub end: u64,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub shared: bool,
    pub offset: u64,
    pub inode: u64,
    pub name: &'static str,
}
impl Map {
    pub fn append(&self, text: &mut Text) -> Result<(), i64> {
        if self.start >= self.end
            || !self.start.is_multiple_of(4096)
            || !self.end.is_multiple_of(4096)
        {
            return Err(22);
        }
        write!(
            text,
            "{:x}-{:x} {}{}{}{} {:08x} {} {}",
            self.start,
            self.end,
            if self.read { 'r' } else { '-' },
            if self.write { 'w' } else { '-' },
            if self.execute { 'x' } else { '-' },
            if self.shared { 's' } else { 'p' },
            self.offset,
            if self.inode == 0 { "00:00" } else { "00:01" },
            self.inode
        )
        .map_err(|_| 12)?;
        if !self.name.is_empty() {
            write!(
                text,
                " {}{}",
                if self.name.starts_with('[') { "" } else { "/" },
                self.name
            )
            .map_err(|_| 12)?;
        }
        writeln!(text).map_err(|_| 12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stat_field_positions_and_parenthesized_names_follow_proc_abi() {
        let mut name = [0; 16];
        name[..9].copy_from_slice(b"a ) name!");
        let stat = Stat {
            pid: 42,
            parent: 1,
            name,
            state: 'R',
            user_ticks: 17,
            kernel_ticks: 23,
            threads: 4,
            start_ticks: 8,
            virtual_bytes: 0x100000,
            resident_pages: 9,
            resident_limit: 0x200000,
            code_start: 0x2000,
            code_end: 0x3000,
            stack_start: 0x7000,
            blocked_signals: 0x20,
        };
        let bytes = stat.text().unwrap();
        let line = core::str::from_utf8(&bytes).unwrap();
        assert!(line.starts_with("42 (a ) name!) R "));
        let fields: alloc::vec::Vec<_> = line
            .rsplit_once(") ")
            .unwrap()
            .1
            .split_whitespace()
            .collect();
        assert_eq!(fields.len(), 50);
        for (number, expected) in [
            (4, "1"),
            (14, "17"),
            (15, "23"),
            (20, "4"),
            (22, "8"),
            (24, "9"),
            (28, "28672"),
            (32, "32"),
        ] {
            assert_eq!(fields[number - 3], expected);
        }
    }
    #[test]
    fn maps_encode_effective_rights_file_offsets_and_snapshot_bound() {
        let mut text = Text::new();
        Map {
            start: 0x1000,
            end: 0x3000,
            read: true,
            write: false,
            execute: false,
            shared: true,
            offset: 0x4000,
            inode: 29,
            name: "system/lib64/libc.so",
        }
        .append(&mut text)
        .unwrap();
        assert_eq!(
            text.finish(),
            b"1000-3000 r--s 00004000 00:01 29 /system/lib64/libc.so\n"
        );
        let mut text = Text::new();
        let huge = alloc::vec![b'x'; MAX_SNAPSHOT];
        text.write_str(core::str::from_utf8(&huge).unwrap())
            .unwrap();
        assert!(text.write_str("x").is_err());
        assert_eq!(text.finish().len(), MAX_SNAPSHOT);
    }
}
