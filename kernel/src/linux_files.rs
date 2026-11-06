//! Immutable runtime files, bounded anonymous memory files and POSIX descriptors
//! for dynamic Bionic. Every pointer is checked against the calling address space.
use crate::{linux_runtime as runtime, process};
use bndroid_kernel::{
    linux_abi::errno,
    linux_fd::{FileRef, Target},
    linux_fs::{Kind, Node, PATH_MAX, Path},
};

const EBADF: i64 = 9;
const EINVAL: i64 = 22;
const AT_FDCWD: u32 = (-100_i32) as u32;
// AArch64 overrides these three asm-generic fcntl bits.
const O_DIRECTORY: u32 = 0x4000;
const O_NOFOLLOW: u32 = 0x8000;
const O_LARGEFILE: u32 = 0x20000;
const O_PATH: u32 = 0x200000;

#[cfg(feature = "art-runtime")]
fn unix_socket(domain: u64, kind: u64, protocol: u64) -> Result<u64, i64> {
    if domain != 1 {
        return Err(97);
    }
    if kind & !(0x80000 | 0x800) != 2 {
        return Err(94);
    }
    if protocol != 0 {
        return Err(93);
    }
    let socket = FileRef::try_new(Target::UnixDatagram, 2 | (kind as u32 & 0x800))?;
    process::with_linux_files(|table| table.insert(socket, 0, kind & 0x80000 != 0))
}
#[cfg(feature = "art-runtime")]
fn unix_connect(fd: u64, address: u64, length: u64) -> Result<u64, i64> {
    let socket = file(fd)?;
    if !matches!(socket.target(), Target::UnixDatagram) {
        return Err(88);
    }
    if !(3..=110).contains(&length) {
        return Err(EINVAL);
    }
    let mut bytes = [0; 110];
    input(address, &mut bytes[..length as usize])?;
    if u16::from_le_bytes(bytes[..2].try_into().unwrap()) != 1 {
        return Err(97);
    }
    let path = &bytes[2..length as usize];
    let end = path
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(path.len());
    // A registered Bndroid log endpoint. Other Unix endpoints remain absent;
    // this is not an implementation of arbitrary Unix-domain IPC/networking.
    if &path[..end] != b"/dev/socket/logdw" {
        return Err(2);
    }
    socket.connect_log()?;
    Ok(0)
}
#[cfg(feature = "art-runtime")]
fn log_datagram(bytes: &[u8]) -> Result<u64, i64> {
    // android_log_header_t is packed: id/u16 tid/u32 seconds/u32 nanoseconds.
    // Attribute records to the authenticated process, not user-supplied IDs.
    if bytes.len() < 11 {
        return Err(EINVAL);
    }
    let pid = process::current_live_user_process_id().unwrap();
    let id = bytes[0];
    if matches!(id, 0 | 1 | 3 | 4 | 7) && bytes.len() > 12 {
        let priority = bytes[11];
        let payload = &bytes[12..];
        let tag_end = payload.iter().position(|byte| *byte == 0).ok_or(EINVAL)?;
        let message = &payload[tag_end + 1..];
        crate::kprint!(
            "ANDROID_LOG pid={} buffer={} priority={} tag=",
            pid,
            id,
            priority
        );
        for part in [&payload[..tag_end], b" message=", message] {
            for &byte in part {
                if byte == 0 {
                    break;
                }
                // Preserve message text while preventing forged proof lines.
                crate::uart::Uart.write_bytes(&[if matches!(byte, b'\n' | b'\r') {
                    b' '
                } else {
                    byte
                }]);
            }
        }
        crate::kprintln!();
    } else {
        crate::kprintln!(
            "ANDROID_LOG_BINARY pid={} buffer={} bytes={}",
            pid,
            id,
            bytes.len() - 11
        );
    }
    Ok(bytes.len() as u64)
}

// Keep packet buffers out of console write/writev frames: ART's error path
// uses writev, whose per-vector write must not stack two 4 KiB log buffers.
#[cfg(feature = "art-runtime")]
#[inline(never)]
fn write_log_packet(file: &FileRef, address: u64, count: u64) -> Result<u64, i64> {
    if !file.log_connected() {
        return Err(107);
    }
    if count > 4096 {
        return Err(90);
    }
    let mut bytes = [0; 4096];
    input(address, &mut bytes[..count as usize])?;
    log_datagram(&bytes[..count as usize])
}

#[cfg(feature = "art-runtime")]
#[inline(never)]
fn writev_log_packet(fd: u64, address: u64, count: u64) -> Result<u64, i64> {
    if !file(fd)?.log_connected() {
        return Err(107);
    }
    if count > 64 {
        return Err(EINVAL);
    }
    let mut packet = [0; 4096];
    let mut length = 0usize;
    for index in 0..count {
        let mut vector = [0; 16];
        input(address.checked_add(index * 16).ok_or(14)?, &mut vector)?;
        let base = u64::from_le_bytes(vector[..8].try_into().unwrap());
        let requested = u64::from_le_bytes(vector[8..].try_into().unwrap());
        if requested > (packet.len() - length) as u64 {
            return Err(90);
        }
        let end = length + requested as usize;
        input(base, &mut packet[length..end])?;
        length = end;
    }
    log_datagram(&packet[..length])
}

// File I/O's bounded gather buffer must not remain live beneath signal frames.
#[cfg_attr(feature = "art-runtime", inline(never))]
pub fn dispatch(r: &[u64; 31]) -> Option<u64> {
    let result = match r[8] {
        17 => {
            // getcwd; this initial profile starts at the immutable root.
            if r[1] < 2 {
                Err(34)
            } else {
                out(r[0], b"/\0").map(|_| 2)
            }
        }
        23 => process::with_linux_files(|t| t.duplicate(r[0], 0, false)),
        24 => {
            if r[2] & !0x80000 != 0 {
                Err(EINVAL)
            } else {
                process::with_linux_files(|t| t.duplicate_to(r[0], r[1], r[2] != 0))
            }
        }
        25 => fcntl(r[0], r[1], r[2]),
        29 => file(r[0]).and(Err(25)), // ENOTTY; no terminal ioctls.
        43 => statfs_path(r[0], r[1]),
        44 => file_statfs(r[0], r[1]),
        #[cfg(feature = "art-runtime")]
        46 => truncate(r[0], r[1]),
        48 => access(r[0], r[1], r[2], 0),
        56 => open(r[0], r[1], r[2]),
        57 => process::with_linux_files(|t| t.close(r[0])).map(|_| 0),
        62 => file(r[0]).and_then(|f| f.seek(r[1] as i64, r[2])),
        63 => read(r[0], r[1], r[2], None),
        64 => write(r[0], r[1], r[2]),
        66 => writev(r[0], r[1], r[2]),
        67 => read(r[0], r[1], r[2], Some(r[3])),
        #[cfg(feature = "art-runtime")]
        68 => file(r[0]).and_then(|f| write_memory(&f, r[1], r[2], Some(r[3]))),
        78 => readlink(r[0], r[1], r[2], r[3]),
        79 => statat(r[0], r[1], r[2], r[3]),
        80 => file(r[0]).and_then(|f| stat(f.target(), r[1])),
        #[cfg(feature = "art-runtime")]
        198 => unix_socket(r[0], r[1], r[2]),
        #[cfg(feature = "art-runtime")]
        203 => unix_connect(r[0], r[1], r[2]),
        222 if r[3] & 0x20 == 0 => mmap_file(r[..6].try_into().unwrap()),
        233 => {
            process::with_current_linux_address_space(|space| space.linux_madvise(r[0], r[1], r[2]))
        }
        #[cfg(feature = "art-runtime")]
        279 => memfd_create(r[0], r[1]),
        439 => access(r[0], r[1], r[2], r[3]),
        _ => return None,
    };
    Some(result.unwrap_or_else(errno))
}

fn file(fd: u64) -> Result<FileRef, i64> {
    process::with_linux_files(|table| table.file(fd))
}
#[cfg(feature = "art-runtime")]
fn memfd_create(address: u64, flags: u64) -> Result<u64, i64> {
    if flags & !3 != 0 {
        return Err(EINVAL);
    }
    let mut name = [0; 250];
    for index in 0..name.len() {
        input(
            address.checked_add(index as u64).ok_or(14)?,
            &mut name[index..index + 1],
        )?;
        if name[index] == 0 {
            let file = FileRef::try_memory(&name[..index], flags as u32)?;
            return process::with_linux_files(|t| t.insert(file, 0, flags & 1 != 0));
        }
    }
    Err(EINVAL)
}
#[cfg(feature = "art-runtime")]
fn truncate(fd: u64, length: u64) -> Result<u64, i64> {
    let opened = file(fd)?;
    if !opened.writable() {
        return Err(EINVAL);
    }
    let memory = opened.memory().ok_or(EINVAL)?;
    process::with_current_linux_address_space(|space| space.linux_memfd_resize(memory, length))
}
#[cfg(feature = "art-runtime")]
#[inline(never)]
fn read_memory(file: &FileRef, address: u64, count: u64, offset: Option<u64>) -> Result<u64, i64> {
    let memory = file.memory().ok_or(EINVAL)?;
    let position = offset.unwrap_or_else(|| file.position());
    if position > i64::MAX as u64 {
        return Err(EINVAL);
    }
    let count = count
        .min(0x7ffff000)
        .min(memory.len().saturating_sub(position));
    let mut bytes = [0; 4096];
    let mut total = 0;
    while total < count {
        let length = memory.read(
            position + total,
            &mut bytes[..(count - total).min(4096) as usize],
        );
        if let Err(error) = address
            .checked_add(total)
            .ok_or(14)
            .and_then(|p| out(p, &bytes[..length]))
        {
            if total == 0 {
                return Err(error);
            }
            break;
        }
        total += length as u64;
    }
    if offset.is_none() {
        file.set_position(position + total);
    }
    Ok(total)
}
#[cfg(feature = "art-runtime")]
#[inline(never)]
fn write_memory(file: &FileRef, address: u64, count: u64, offset: Option<u64>) -> Result<u64, i64> {
    if !file.writable() {
        return Err(EBADF);
    }
    let memory = file.memory().ok_or(29)?;
    let position = if file.flags() & 0x400 != 0 {
        memory.len()
    } else {
        offset.unwrap_or_else(|| file.position())
    };
    if position > i64::MAX as u64 {
        return Err(EINVAL);
    }
    let count = count.min(0x7ffff000);
    if count == 0 {
        return Ok(0);
    }
    if !memory.may_write() {
        return Err(1);
    }
    if position
        .checked_add(count)
        .is_none_or(|end| end > bndroid_kernel::linux_memfd::MAX_BYTES)
    {
        return Err(27);
    }
    let mut bytes = [0; 4096];
    let mut total = 0;
    while total < count {
        let length = (count - total).min(4096) as usize;
        let result = address
            .checked_add(total)
            .ok_or(14)
            .and_then(|p| input(p, &mut bytes[..length]))
            .and_then(|_| {
                let end = position + total + length as u64;
                if end > memory.len() {
                    process::with_current_linux_address_space(|space| {
                        space.linux_memfd_resize(memory, end)
                    })?;
                }
                memory.write(position + total, &bytes[..length])
            });
        match result {
            Ok(written) => total += written as u64,
            Err(error) => {
                if total == 0 {
                    return Err(error);
                }
                break;
            }
        }
    }
    if offset.is_none() {
        file.set_position(position + total);
    }
    Ok(total)
}
fn statfs_path(pathname: u64, output: u64) -> Result<u64, i64> {
    let mut buffer = [0; PATH_MAX + 1];
    node(AT_FDCWD as u64, name(pathname, &mut buffer)?, true)?;
    statfs(output)
}
fn file_statfs(fd: u64, output: u64) -> Result<u64, i64> {
    let opened = file(fd)?;
    if opened.memory().is_none() {
        return statfs(output);
    }
    let mut bytes = [0; 120];
    let (live, _, _) = bndroid_kernel::linux_pages::snapshot();
    let capacity = bndroid_kernel::linux_pages::MAX_PAGES as u64;
    for (offset, value) in [
        (0, 0x0102_1994_u64),
        (8, 4096),
        (16, capacity),
        (24, capacity.saturating_sub(live)),
        (32, capacity.saturating_sub(live)),
        (40, bndroid_kernel::linux_fd::CAPACITY as u64),
        (56, 2),
        (64, 255),
        (72, 4096),
    ] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    out(output, &bytes).map(|_| 0)
}
fn statfs(output: u64) -> Result<u64, i64> {
    let (length, files) = crate::linux_rootfs::usage();
    let mut bytes = [0; 120];
    // Own immutable filesystem magic, 4 KiB blocks, no writable capacity.
    for (offset, value) in [
        (0, 0x424e4452_u64),
        (8, 4096),
        (16, length.div_ceil(4096) as u64),
        (40, files as u64),
        (56, 1),
        (64, PATH_MAX as u64),
        (72, 4096),
        (80, 3),
    ] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    out(output, &bytes).map(|_| 0)
}
fn out(address: u64, bytes: &[u8]) -> Result<(), i64> {
    let result = runtime::copy_out(address, bytes) as i64;
    if result < 0 { Err(-result) } else { Ok(()) }
}
fn input(address: u64, bytes: &mut [u8]) -> Result<(), i64> {
    runtime::copy_in(address, bytes)
}
fn name(address: u64, bytes: &mut [u8; PATH_MAX + 1]) -> Result<&str, i64> {
    for index in 0..=PATH_MAX {
        input(
            address.checked_add(index as u64).ok_or(14)?,
            &mut bytes[index..index + 1],
        )?;
        if bytes[index] == 0 {
            return core::str::from_utf8(&bytes[..index]).map_err(|_| EINVAL);
        }
    }
    Err(36)
}
fn path(dir: u64, name: &str) -> Result<Path, i64> {
    if name.starts_with('/') || dir as u32 == AT_FDCWD {
        return Path::normalize("", name);
    }
    let opened = file(dir)?;
    match opened.target() {
        Target::Node(node) if node.kind == Kind::Directory => Path::normalize(node.path, name),
        _ => Err(20),
    }
}
fn node(dir: u64, name: &str, follow: bool) -> Result<Node<'static>, i64> {
    let path = path(dir, name)?;
    // The kernel image for a direct linker invocation remains the interpreter.
    if path.as_str() == "proc/self/exe" {
        if follow {
            return crate::linux_rootfs::resolve(
                &Path::normalize("", "/apex/com.android.runtime/bin/linker64")?,
                true,
            );
        }
        return Ok(Node {
            path: "proc/self/exe",
            data: b"/apex/com.android.runtime/bin/linker64",
            kind: Kind::Symlink,
            mode: 0o777,
            inode: 5,
        });
    }
    crate::linux_rootfs::resolve(&path, follow)
}
fn target(node: Node<'static>) -> Target {
    match node.path {
        "dev/null" => Target::Null,
        "dev/urandom" => Target::Random,
        _ => Target::Node(node),
    }
}

fn open(dir: u64, address: u64, raw_flags: u64) -> Result<u64, i64> {
    let flags = u32::try_from(raw_flags).map_err(|_| EINVAL)?;
    // RDONLY/WRONLY/RDWR, NONBLOCK, LARGEFILE, DIRECTORY, NOFOLLOW, CLOEXEC, PATH.
    if flags & !(3 | 0x800 | O_LARGEFILE | O_DIRECTORY | O_NOFOLLOW | 0x80000 | O_PATH) != 0 {
        return Err(if flags & (0x40 | 0x200 | 0x400) != 0 {
            30
        } else {
            EINVAL
        });
    }
    if flags & 3 == 3 {
        return Err(EINVAL);
    }
    let mut buffer = [0; PATH_MAX + 1];
    let value = name(address, &mut buffer)?;
    let node = node(dir, value, flags & O_NOFOLLOW == 0)?;
    if node.kind == Kind::Symlink && flags & O_PATH == 0 {
        return Err(40);
    }
    if flags & O_DIRECTORY != 0 && node.kind != Kind::Directory {
        return Err(20);
    }
    let target = target(node);
    if flags & O_PATH == 0 && flags & 3 != 0 {
        match target {
            Target::Null => {}
            Target::Node(n) if n.kind == Kind::Directory => return Err(21),
            _ => return Err(30),
        }
    }
    let stored = if flags & O_PATH != 0 {
        flags & (O_PATH | O_DIRECTORY | O_NOFOLLOW)
    } else {
        flags & !0x80000
    };
    #[cfg(feature = "art-runtime")]
    let file = if matches!(node.path, "proc/self/stat" | "proc/self/maps") {
        FileRef::try_snapshot(
            node.path,
            node.inode,
            stored,
            process::linux_proc_snapshot(node.path == "proc/self/maps")?,
        )?
    } else {
        FileRef::try_new(target, stored)?
    };
    #[cfg(not(feature = "art-runtime"))]
    let file = FileRef::try_new(target, stored)?;
    process::with_linux_files(|table| table.insert(file, 0, flags & 0x80000 != 0))
}

fn fcntl(fd: u64, command: u64, argument: u64) -> Result<u64, i64> {
    match command {
        0 | 1030 => process::with_linux_files(|t| t.duplicate(fd, argument, command == 1030)),
        1 => process::with_linux_files(|t| t.fd_flags(fd)),
        2 => process::with_linux_files(|t| t.set_fd_flags(fd, argument)).map(|_| 0),
        3 => file(fd).map(|f| f.flags() as u64),
        4 => file(fd)
            .and_then(|f| f.set_status_flags(u32::try_from(argument).map_err(|_| EINVAL)?))
            .map(|_| 0),
        #[cfg(feature = "art-runtime")]
        1033 => {
            let opened = file(fd)?;
            if !opened.writable() {
                return Err(1);
            }
            let memory = opened.memory().ok_or(EINVAL)?;
            process::with_current_linux_address_space(|space| {
                space.linux_memfd_seal(memory, argument)
            })
        }
        #[cfg(feature = "art-runtime")]
        1034 => file(fd)?.memory().map(|m| m.seals() as u64).ok_or(EINVAL),
        _ => file(fd).and(Err(EINVAL)),
    }
}
fn read(fd: u64, address: u64, count: u64, offset: Option<u64>) -> Result<u64, i64> {
    let file = file(fd)?;
    if !file.readable() {
        return Err(EBADF);
    }
    #[cfg(feature = "art-runtime")]
    if file.memory().is_some() {
        return read_memory(&file, address, count, offset);
    }
    let position = offset.unwrap_or_else(|| file.position());
    if position > i64::MAX as u64 {
        return Err(EINVAL);
    }
    // Regular immutable files can satisfy a full read up to EOF. Android's
    // ZIP reader expects its central-directory window in one pread operation.
    // The catalog and caller's owned mappings bound the actual copy length.
    let limit = count.min(0x7ffff000) as usize;
    let bytes = match file.target() {
        Target::Null if offset.is_none() => return Ok(0),
        Target::Random if offset.is_none() => {
            let result = runtime::getrandom(address, count, 0) as i64;
            return if result < 0 {
                Err(-result)
            } else {
                Ok(result as u64)
            };
        }
        Target::Node(node) if node.kind == Kind::File => node.data,
        Target::Snapshot { .. } => file.snapshot().ok_or(EBADF)?,
        Target::Node(node) if node.kind == Kind::Directory => return Err(21),
        _ => return Err(29),
    };
    let start = (position as usize).min(bytes.len());
    let length = limit.min(bytes.len() - start);
    out(address, &bytes[start..start + length])?;
    if offset.is_none() {
        file.set_position(position + length as u64);
    }
    Ok(length as u64)
}
#[cfg_attr(feature = "art-runtime", inline(never))]
fn write(fd: u64, address: u64, count: u64) -> Result<u64, i64> {
    let file = file(fd)?;
    if !file.writable() {
        return Err(EBADF);
    }
    match file.target() {
        #[cfg(feature = "art-runtime")]
        Target::Memory { .. } => write_memory(&file, address, count, None),
        #[cfg(feature = "art-runtime")]
        Target::UnixDatagram => write_log_packet(&file, address, count),
        Target::Null => Ok(count.min(0x7ffff000)),
        Target::Console => {
            let count = count.min(256) as usize;
            let mut bytes = [0; 256];
            input(address, &mut bytes[..count])?;
            crate::uart::Uart.write_bytes(&bytes[..count]);
            Ok(count as u64)
        }
        _ => Err(30),
    }
}
#[cfg_attr(feature = "art-runtime", inline(never))]
fn writev(fd: u64, address: u64, count: u64) -> Result<u64, i64> {
    #[cfg(feature = "art-runtime")]
    if matches!(file(fd)?.target(), Target::UnixDatagram) {
        return writev_log_packet(fd, address, count);
    }
    if !file(fd)?.writable() {
        return Err(EBADF);
    }
    if count > 64 {
        return Err(EINVAL);
    }
    let mut total = 0;
    for index in 0..count {
        let mut vector = [0; 16];
        if let Err(error) = input(address.checked_add(index * 16).ok_or(14)?, &mut vector) {
            return if total == 0 { Err(error) } else { Ok(total) };
        }
        let base = u64::from_le_bytes(vector[..8].try_into().unwrap());
        let requested = u64::from_le_bytes(vector[8..].try_into().unwrap());
        let written = match write(fd, base, requested.min(256 - total)) {
            Ok(bytes) => bytes,
            Err(error) => return if total == 0 { Err(error) } else { Ok(total) },
        };
        total += written;
        if written < requested || total == 256 {
            break;
        }
    }
    Ok(total)
}
fn stat(target: Target, address: u64) -> Result<u64, i64> {
    let mut bytes = [0; 128]; // Linux asm-generic stat, LP64.
    let (inode, mode, size, device) = match target {
        Target::Node(node) => (
            node.inode,
            node.mode
                | match node.kind {
                    Kind::File => 0o100000,
                    Kind::Directory => 0o040000,
                    Kind::Symlink => 0o120000,
                },
            node.data.len() as u64,
            0_u64,
        ),
        Target::Null => (3, 0o020666, 0, 0x103),
        Target::Random => (4, 0o020444, 0, 0x109),
        Target::Console => (6, 0o020222, 0, 0),
        Target::Snapshot { inode, .. } => (inode, 0o100444, 0, 0),
        Target::UnixDatagram => (0, 0o140777, 0, 0),
        Target::Memory { inode, length } => (inode, 0o100777, length, 0),
    };
    bytes[..8].copy_from_slice(&1_u64.to_le_bytes());
    bytes[8..16].copy_from_slice(&inode.to_le_bytes());
    bytes[16..20].copy_from_slice(&mode.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    if matches!(target, Target::Memory { .. }) {
        bytes[20..24].copy_from_slice(&0_u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&10000_u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&10000_u32.to_le_bytes());
    }
    bytes[32..40].copy_from_slice(&device.to_le_bytes());
    bytes[48..56].copy_from_slice(&size.to_le_bytes());
    bytes[56..60].copy_from_slice(&4096_u32.to_le_bytes());
    bytes[64..72].copy_from_slice(&size.div_ceil(512).to_le_bytes());
    out(address, &bytes).map(|_| 0)
}
fn statat(dir: u64, pathname: u64, address: u64, flags: u64) -> Result<u64, i64> {
    if flags & !(0x100 | 0x800 | 0x1000) != 0 {
        return Err(EINVAL);
    }
    let mut buffer = [0; PATH_MAX + 1];
    let value = name(pathname, &mut buffer)?;
    let target = if value.is_empty() && flags & 0x1000 != 0 {
        file(dir)?.target()
    } else {
        target(node(dir, value, flags & 0x100 == 0)?)
    };
    stat(target, address)
}
fn access(dir: u64, pathname: u64, mode: u64, flags: u64) -> Result<u64, i64> {
    if mode & !7 != 0 || flags & !(0x100 | 0x200) != 0 {
        return Err(EINVAL);
    }
    let mut buffer = [0; PATH_MAX + 1];
    let node = node(dir, name(pathname, &mut buffer)?, flags & 0x100 == 0)?;
    if mode & 2 != 0 && node.path != "dev/null" {
        return Err(30);
    }
    if node.mode as u64 & mode != mode {
        return Err(13);
    }
    Ok(0)
}
fn readlink(dir: u64, pathname: u64, output: u64, capacity: u64) -> Result<u64, i64> {
    if capacity == 0 {
        return Err(EINVAL);
    }
    let mut buffer = [0; PATH_MAX + 1];
    let value = name(pathname, &mut buffer)?;
    let path = path(dir, value)?;
    if let Some(number) = path.as_str().strip_prefix("proc/self/fd/") {
        let fd = number.parse::<u64>().map_err(|_| 2)?;
        let opened = file(fd).map_err(|_| 2)?;
        if let Some(memory) = opened.memory() {
            let mut bytes = [0; 280];
            let prefix = b"/memfd:";
            let suffix = b" (deleted)";
            bytes[..prefix.len()].copy_from_slice(prefix);
            let end = prefix.len() + memory.name().len();
            bytes[prefix.len()..end].copy_from_slice(memory.name());
            bytes[end..end + suffix.len()].copy_from_slice(suffix);
            let length = (end + suffix.len()).min(capacity as usize);
            return out(output, &bytes[..length]).map(|_| length as u64);
        }
        let path = match opened.target() {
            Target::Node(node) => node.path,
            Target::Snapshot { path, .. } => path,
            _ => return Err(2),
        };
        let mut absolute = [0; PATH_MAX + 1];
        absolute[0] = b'/';
        absolute[1..1 + path.len()].copy_from_slice(path.as_bytes());
        let length = (path.len() + 1).min(capacity as usize);
        return out(output, &absolute[..length]).map(|_| length as u64);
    }
    let node = node(dir, value, false)?;
    if node.kind != Kind::Symlink {
        return Err(EINVAL);
    }
    let length = node.data.len().min(capacity as usize);
    out(output, &node.data[..length]).map(|_| length as u64)
}
fn mmap_file(args: [u64; 6]) -> Result<u64, i64> {
    let opened = file(args[4])?;
    if !opened.readable() {
        return Err(13);
    }
    #[cfg(feature = "art-runtime")]
    if opened.memory().is_some() {
        return process::with_current_linux_address_space(|space| {
            space.linux_mmap_memory(args, &opened)
        });
    }
    let Target::Node(node) = opened.target() else {
        return Err(19);
    };
    if node.kind != Kind::File {
        return Err(19);
    }
    if args[2] & 4 != 0 && node.mode & 0o111 == 0 {
        return Err(13);
    }
    process::with_current_linux_address_space(|space| space.linux_mmap_file(args, node))
}
