//! SELinux service-query ABI projected from Init's immutable native grants.
//! This is not a Linux SELinux engine or an Android policy binary interpreter.
use crate::{linux_rootfs, process, userboot};
use alloc::{format, vec::Vec};
use bndroid_kernel::{
    linux_control::Kind as ControlKind,
    linux_fd::FileRef,
    linux_fs::{Kind, Node, Path},
};

use bndroid_kernel::linux_launch::SECURITY_CONTEXTS as CONTEXTS;
const fn status() -> [u8; 4096] {
    let mut bytes = [0; 4096];
    bytes[0] = 1; // ABI version
    bytes[8] = 1; // enforcing native grants
    bytes[12] = 1; // immutable boot policy generation
    bytes[16] = 1; // unknown classes/permissions denied
    bytes
}
static STATUS: [u8; 4096] = status();
pub fn nodes() -> impl Iterator<Item = Node<'static>> {
    [
        ("sys/fs/selinux/status", STATUS.as_slice(), 0o444),
        ("sys/fs/selinux/enforce", b"1".as_slice(), 0o444),
        ("sys/fs/selinux/deny_unknown", b"1".as_slice(), 0o444),
        ("sys/fs/selinux/reject_unknown", b"1".as_slice(), 0o444),
        ("sys/fs/selinux/mls", b"0".as_slice(), 0o444),
        ("sys/fs/selinux/access", b"".as_slice(), 0o666),
        ("sys/fs/selinux/context", b"".as_slice(), 0o666),
        (
            "sys/fs/selinux/class/service_manager/index",
            b"1".as_slice(),
            0o444,
        ),
        (
            "sys/fs/selinux/class/service_manager/perms/add",
            b"1".as_slice(),
            0o444,
        ),
        (
            "sys/fs/selinux/class/service_manager/perms/find",
            b"2".as_slice(),
            0o444,
        ),
        (
            "sys/fs/selinux/class/service_manager/perms/list",
            b"3".as_slice(),
            0o444,
        ),
        (
            "sys/fs/selinux/class/bndroid_service/index",
            b"2".as_slice(),
            0o444,
        ),
        (
            "sys/fs/selinux/class/bndroid_service/perms/stop",
            b"1".as_slice(),
            0o444,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (path, data, mode))| Node {
        path,
        data,
        kind: Kind::File,
        mode,
        inode: 0x20000 + i as u64,
    })
}
pub fn proc_node(path: &str) -> Result<Option<Node<'static>>, i64> {
    let profile = if matches!(
        path,
        "proc/self/attr/current" | "proc/thread-self/attr/current"
    ) {
        process::linux_security_profile(None)
    } else if let Some(pid) = path
        .strip_prefix("proc/")
        .and_then(|s| s.strip_suffix("/attr/current"))
    {
        process::linux_security_profile(Some(pid.parse::<u32>().map_err(|_| 2)?))
    } else {
        return Ok(None);
    };
    let context = CONTEXTS.get(profile.ok_or(2)?).ok_or(2)?;
    Ok(Some(Node {
        path: "proc/thread-self/attr/current",
        data: context.as_bytes(),
        kind: Kind::File,
        mode: 0o444,
        inode: 0x20100 + profile.unwrap() as u64,
    }))
}
fn known_context(context: &str) -> bool {
    if CONTEXTS
        .iter()
        .position(|c| *c == context)
        .is_some_and(|i| userboot::linux_launch(i).is_some())
    {
        return true;
    }
    for path in [
        "/system/etc/selinux/plat_service_contexts",
        "/system_ext/etc/selinux/system_ext_service_contexts",
        "/product/etc/selinux/product_service_contexts",
    ] {
        let Ok(node) = linux_rootfs::resolve(&Path::normalize("", path).unwrap(), true) else {
            continue;
        };
        let Ok(text) = core::str::from_utf8(node.data) else {
            continue;
        };
        if text.lines().any(|l| {
            !l.trim_start().starts_with('#') && l.split_ascii_whitespace().nth(1) == Some(context)
        }) {
            return true;
        }
    }
    false
}
pub fn write(file: &FileRef, kind: ControlKind, bytes: &[u8]) -> Result<u64, i64> {
    if bytes.is_empty() || bytes.len() > 512 {
        return Err(22);
    }
    let text = core::str::from_utf8(bytes).map_err(|_| 22)?;
    let response = match kind {
        ControlKind::Context => {
            let context = text.strip_suffix('\0').unwrap_or(text);
            if !known_context(context) {
                return Err(22);
            }
            context.as_bytes().to_vec()
        }
        ControlKind::Access => {
            let mut fields = text.split_ascii_whitespace();
            let source = fields.next().ok_or(22)?;
            let target = fields.next().ok_or(22)?;
            let class = fields.next().ok_or(22)?.parse::<u16>().map_err(|_| 22)?;
            let _requested = u32::from_str_radix(fields.next().ok_or(22)?, 16).map_err(|_| 22)?;
            if fields.next().is_some() || !known_context(source) || !known_context(target) {
                return Err(22);
            }
            let allowed = if class == 1 {
                CONTEXTS
                    .iter()
                    .position(|c| *c == source)
                    .and_then(userboot::linux_launch)
                    .map_or(0, |p| {
                        bndr_sm::service_authority::allowed(p.service_grants, target)
                    })
            } else if class == 2 {
                CONTEXTS
                    .iter()
                    .position(|c| *c == source)
                    .and_then(userboot::linux_launch)
                    .map_or(0, |p| {
                        u32::from(bndr_sm::service_authority::may_stop_service(
                            p.control_grants,
                            target,
                        ))
                    })
            } else {
                0
            };
            format!("{allowed:x} ffffffff 0 ffffffff 1 0").into_bytes()
        }
        ControlKind::Property => return Err(22),
    };
    file.control().with(|state| {
        if state.response.is_some() {
            return Err(16);
        }
        state.response = Some(response);
        Ok(bytes.len() as u64)
    })
}
pub fn read(file: &FileRef, address: u64, count: u64) -> Result<u64, i64> {
    if count == 0 {
        return Ok(0);
    }
    file.control().with(|state| {
        let bytes = state.response.as_ref().ok_or(11)?;
        let n = (count as usize).min(bytes.len() - state.read);
        let result =
            crate::linux_runtime::copy_out(address, &bytes[state.read..state.read + n]) as i64;
        if result < 0 {
            return Err(-result);
        }
        state.read += n;
        Ok(n as u64)
    })
}
pub fn directory(file: &FileRef, address: u64, count: u64) -> Result<u64, i64> {
    let bndroid_kernel::linux_fd::Target::Node(node) = file.target() else {
        return Err(20);
    };
    if !file.readable() || node.kind != Kind::Directory {
        return Err(20);
    }
    let entries = linux_rootfs::children(node.path)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve((count as usize).min(16384))
        .map_err(|_| 12)?;
    let start = usize::try_from(file.position()).map_err(|_| 22)?;
    let mut position = start;
    for (name, child) in entries.iter().skip(start) {
        let length = (19 + name.len() + 1 + 7) & !7;
        if bytes.len() + length > (count as usize).min(16384) {
            if bytes.is_empty() {
                return Err(22);
            }
            break;
        }
        let offset = bytes.len();
        bytes.resize(offset + length, 0);
        position += 1;
        bytes[offset..offset + 8].copy_from_slice(&child.inode.to_le_bytes());
        bytes[offset + 8..offset + 16].copy_from_slice(&(position as u64).to_le_bytes());
        bytes[offset + 16..offset + 18].copy_from_slice(&(length as u16).to_le_bytes());
        bytes[offset + 18] = match child.kind {
            Kind::File => 8,
            Kind::Directory => 4,
            Kind::Symlink => 10,
        };
        bytes[offset + 19..offset + 19 + name.len()].copy_from_slice(name.as_bytes());
    }
    let result = crate::linux_runtime::copy_out(address, &bytes) as i64;
    if result < 0 {
        return Err(-result);
    }
    file.set_position(position as u64);
    Ok(bytes.len() as u64)
}
