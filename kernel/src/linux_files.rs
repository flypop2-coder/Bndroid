//! Read-only runtime files and POSIX descriptor operations for the dynamic
//! Bionic profile. Every pointer is checked against the calling address space.
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
        44 => file(r[0]).and_then(|_| statfs(r[1])),
        48 => access(r[0], r[1], r[2], 0),
        56 => open(r[0], r[1], r[2]),
        57 => process::with_linux_files(|t| t.close(r[0])).map(|_| 0),
        62 => file(r[0]).and_then(|f| f.seek(r[1] as i64, r[2])),
        63 => read(r[0], r[1], r[2], None),
        64 => write(r[0], r[1], r[2]),
        66 => writev(r[0], r[1], r[2]),
        67 => read(r[0], r[1], r[2], Some(r[3])),
        78 => readlink(r[0], r[1], r[2], r[3]),
        79 => statat(r[0], r[1], r[2], r[3]),
        80 => file(r[0]).and_then(|f| stat(f.target(), r[1])),
        222 if r[3] & 0x20 == 0 => mmap_file(r[..6].try_into().unwrap()),
        233 => {
            process::with_current_linux_address_space(|space| space.linux_madvise(r[0], r[1], r[2]))
        }
        439 => access(r[0], r[1], r[2], r[3]),
        _ => return None,
    };
    Some(result.unwrap_or_else(errno))
}

fn file(fd: u64) -> Result<FileRef, i64> {
    process::with_linux_files(|table| table.file(fd))
}
fn statfs_path(pathname: u64, output: u64) -> Result<u64, i64> {
    let mut buffer = [0; PATH_MAX + 1];
    node(AT_FDCWD as u64, name(pathname, &mut buffer)?, true)?;
    statfs(output)
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
        _ => file(fd).and(Err(EINVAL)),
    }
}
fn read(fd: u64, address: u64, count: u64, offset: Option<u64>) -> Result<u64, i64> {
    let file = file(fd)?;
    if !file.readable() {
        return Err(EBADF);
    }
    let position = offset.unwrap_or_else(|| file.position());
    if position > i64::MAX as u64 {
        return Err(EINVAL);
    }
    let limit = count.min(65536) as usize;
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
fn write(fd: u64, address: u64, count: u64) -> Result<u64, i64> {
    let file = file(fd)?;
    if !file.writable() {
        return Err(EBADF);
    }
    match file.target() {
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
fn writev(fd: u64, address: u64, count: u64) -> Result<u64, i64> {
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
    };
    bytes[..8].copy_from_slice(&1_u64.to_le_bytes());
    bytes[8..16].copy_from_slice(&inode.to_le_bytes());
    bytes[16..20].copy_from_slice(&mode.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
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
        let Target::Node(node) = opened.target() else {
            return Err(2);
        };
        let mut absolute = [0; PATH_MAX + 1];
        absolute[0] = b'/';
        absolute[1..1 + node.path.len()].copy_from_slice(node.path.as_bytes());
        let length = (node.path.len() + 1).min(capacity as usize);
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
    let Target::Node(node) = opened.target() else {
        return Err(19);
    };
    if node.kind != Kind::File {
        return Err(19);
    }
    if args[2] & 4 != 0 && node.mode & 0o111 == 0 {
        return Err(13);
    }
    process::with_current_linux_address_space(|space| space.linux_mmap_file(args, node.data))
}
