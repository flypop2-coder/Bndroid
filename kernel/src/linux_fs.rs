//! A bounded, immutable runtime root. Paths are resolved inside this catalog,
//! never against the host filesystem. File bytes retain the archive lifetime.

pub const PATH_MAX: usize = 256;
pub const FILE_MAX: usize = if cfg!(feature = "art-runtime") {
    128
} else {
    64
};
pub const ENOENT: i64 = 2;
pub const EINVAL: i64 = 22;
pub const ENAMETOOLONG: i64 = 36;
pub const ELOOP: i64 = 40;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    bytes: [u8; PATH_MAX],
    len: usize,
    directory: bool,
}
impl Path {
    pub fn normalize(base: &str, input: &str) -> Result<Self, i64> {
        if input.is_empty() {
            return Err(ENOENT);
        }
        if input.as_bytes().contains(&0) {
            return Err(EINVAL);
        }
        let mut result = Self {
            bytes: [0; PATH_MAX],
            len: 0,
            directory: input.ends_with('/') || matches!(input.rsplit('/').next(), Some("." | "..")),
        };
        for source in [if input.starts_with('/') { "" } else { base }, input] {
            for component in source.split('/') {
                match component {
                    "" | "." => continue,
                    _ => {}
                }
                result.push(component)?;
            }
        }
        Ok(result)
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len])
            .expect("path contains complete UTF-8 components")
    }
    fn push(&mut self, component: &str) -> Result<(), i64> {
        let separator = usize::from(self.len != 0);
        let end = self
            .len
            .checked_add(separator + component.len())
            .ok_or(ENAMETOOLONG)?;
        if end > PATH_MAX {
            return Err(ENAMETOOLONG);
        }
        if separator != 0 {
            self.bytes[self.len] = b'/';
        }
        self.bytes[self.len + separator..end].copy_from_slice(component.as_bytes());
        self.len = end;
        Ok(())
    }
    fn parent(&mut self) {
        self.len = self.bytes[..self.len]
            .iter()
            .rposition(|byte| *byte == b'/')
            .unwrap_or(0);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    File,
    Directory,
    Symlink,
}

#[derive(Clone, Copy, Debug)]
pub struct Node<'a> {
    pub path: &'a str,
    pub data: &'a [u8],
    pub kind: Kind,
    pub mode: u32,
    pub inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveError {
    Truncated,
    Format,
    Checksum,
    Path,
    Duplicate,
    TooManyFiles,
    Overflow,
}

pub struct Archive<'a> {
    entries: [Option<Node<'a>>; FILE_MAX],
    count: usize,
}
impl<'a> Archive<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ArchiveError> {
        if !bytes.len().is_multiple_of(512) {
            return Err(ArchiveError::Truncated);
        }
        let mut result = Self {
            entries: [None; FILE_MAX],
            count: 0,
        };
        let mut at = 0;
        loop {
            let header = bytes.get(at..at + 512).ok_or(ArchiveError::Truncated)?;
            if header.iter().all(|byte| *byte == 0) {
                let trailer = bytes.get(at..at + 1024).ok_or(ArchiveError::Truncated)?;
                if trailer.iter().any(|byte| *byte != 0)
                    || bytes[at..].iter().any(|byte| *byte != 0)
                {
                    return Err(ArchiveError::Format);
                }
                result.validate_namespace()?;
                return Ok(result);
            }
            if &header[257..263] != b"ustar\0"
                || &header[263..265] != b"00"
                || header[345..500].iter().any(|byte| *byte != 0)
            {
                return Err(ArchiveError::Format);
            }
            let checksum = number(&header[148..156])?;
            let sum: u64 = header
                .iter()
                .enumerate()
                .map(|(index, byte)| {
                    if (148..156).contains(&index) {
                        32
                    } else {
                        u64::from(*byte)
                    }
                })
                .sum();
            if checksum != sum {
                return Err(ArchiveError::Checksum);
            }
            let path = string(&header[..100])?.trim_end_matches('/');
            let normalized = Path::normalize("", path).map_err(|_| ArchiveError::Path)?;
            if path.is_empty()
                || path.starts_with('/')
                || path != normalized.as_str()
                || path.split('/').any(|part| part == "..")
                || (header[156] != b'5' && string(&header[..100])?.ends_with('/'))
            {
                return Err(ArchiveError::Path);
            }
            if result.entries[..result.count]
                .iter()
                .flatten()
                .any(|entry| entry.path == path)
            {
                return Err(ArchiveError::Duplicate);
            }
            if result.count == FILE_MAX {
                return Err(ArchiveError::TooManyFiles);
            }
            let mode =
                u32::try_from(number(&header[100..108])?).map_err(|_| ArchiveError::Format)?;
            if mode & !0o777 != 0 {
                return Err(ArchiveError::Format);
            }
            let size =
                usize::try_from(number(&header[124..136])?).map_err(|_| ArchiveError::Overflow)?;
            let start = at.checked_add(512).ok_or(ArchiveError::Overflow)?;
            let end = start.checked_add(size).ok_or(ArchiveError::Overflow)?;
            let data = bytes.get(start..end).ok_or(ArchiveError::Truncated)?;
            let (kind, data) = match header[156] {
                0 | b'0' => (Kind::File, data),
                b'5' if size == 0 => (Kind::Directory, &[][..]),
                b'2' if size == 0 => {
                    let target = string(&header[157..257])?;
                    if target.is_empty() {
                        return Err(ArchiveError::Path);
                    }
                    (Kind::Symlink, target.as_bytes())
                }
                _ => return Err(ArchiveError::Format),
            };
            result.entries[result.count] = Some(Node {
                path,
                data,
                kind,
                mode,
                inode: 16 + result.count as u64,
            });
            result.count += 1;
            at = end.checked_add(511).ok_or(ArchiveError::Overflow)? & !511;
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn file_bytes(&self) -> usize {
        self.entries[..self.count]
            .iter()
            .flatten()
            .map(|node| node.data.len())
            .sum()
    }

    pub fn insert(&mut self, node: Node<'a>) -> Result<(), ArchiveError> {
        if self.count == FILE_MAX {
            return Err(ArchiveError::TooManyFiles);
        }
        if self.exact(node.path).is_some() {
            return Err(ArchiveError::Duplicate);
        }
        let path = Path::normalize("", node.path).map_err(|_| ArchiveError::Path)?;
        if path.as_str() != node.path
            || node.path.is_empty()
            || node.path.split('/').any(|part| part == "..")
        {
            return Err(ArchiveError::Path);
        }
        self.entries[self.count] = Some(node);
        self.count += 1;
        if let Err(error) = self.validate_namespace() {
            self.count -= 1;
            self.entries[self.count] = None;
            return Err(error);
        }
        Ok(())
    }

    fn validate_namespace(&self) -> Result<(), ArchiveError> {
        for entry in self.entries[..self.count].iter().flatten() {
            if entry.kind == Kind::Directory {
                continue;
            }
            if self.entries[..self.count].iter().flatten().any(|other| {
                other
                    .path
                    .strip_prefix(entry.path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            }) {
                return Err(ArchiveError::Path);
            }
        }
        Ok(())
    }

    pub fn resolve(&self, path: &Path, follow_final: bool) -> Result<Node<'a>, i64> {
        let mut pending = path.clone();
        let mut links = 0;
        'resolve: loop {
            let name = pending.as_str();
            let mut current = Path::normalize("", "/")?;
            let mut start = 0;
            for end in name
                .match_indices('/')
                .map(|(index, _)| index)
                .chain(core::iter::once(name.len()))
            {
                let component = &name[start..end];
                start = end + 1;
                if component == ".." {
                    current.parent();
                    continue;
                }
                if component.is_empty() {
                    continue;
                }
                current.push(component)?;
                let node = self.node(current.as_str()).ok_or(ENOENT)?;
                if node.kind == Kind::Symlink
                    && (end != name.len() || follow_final || pending.directory)
                {
                    if links == 40 {
                        return Err(ELOOP);
                    }
                    links += 1;
                    current.parent();
                    let target = core::str::from_utf8(node.data).map_err(|_| EINVAL)?;
                    let target = Path::normalize(current.as_str(), target)?;
                    let mut next = if end == name.len() {
                        target
                    } else {
                        Path::normalize(target.as_str(), &name[end + 1..])?
                    };
                    next.directory |= pending.directory;
                    pending = next;
                    continue 'resolve;
                }
                if end < name.len() && node.kind != Kind::Directory {
                    return Err(20);
                }
            }
            let node = self.node(current.as_str()).ok_or(ENOENT)?;
            if pending.directory && node.kind != Kind::Directory {
                return Err(20);
            }
            return Ok(node);
        }
    }

    fn node(&self, name: &str) -> Option<Node<'a>> {
        if name.is_empty() {
            return Some(Node {
                path: "",
                data: &[],
                kind: Kind::Directory,
                mode: 0o555,
                inode: 1,
            });
        }
        if let Some(node) = self.exact(name) {
            return Some(node);
        }
        for (index, entry) in self.entries[..self.count].iter().flatten().enumerate() {
            if entry
                .path
                .strip_prefix(name)
                .is_some_and(|suffix| suffix.starts_with('/'))
            {
                return Some(Node {
                    path: &entry.path[..name.len()],
                    data: &[],
                    kind: Kind::Directory,
                    mode: 0o555,
                    inode: 0x1_0000_0000 | ((index as u64) << 16) | name.len() as u64,
                });
            }
        }
        None
    }

    fn exact(&self, path: &str) -> Option<Node<'a>> {
        self.entries[..self.count]
            .iter()
            .flatten()
            .find(|entry| entry.path == path)
            .copied()
    }
}

fn string(bytes: &[u8]) -> Result<&str, ArchiveError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if bytes[end..].iter().any(|byte| *byte != 0) {
        return Err(ArchiveError::Format);
    }
    core::str::from_utf8(&bytes[..end]).map_err(|_| ArchiveError::Path)
}

fn number(bytes: &[u8]) -> Result<u64, ArchiveError> {
    let mut result = 0_u64;
    let first = bytes
        .iter()
        .position(|byte| *byte != 0 && *byte != b' ')
        .unwrap_or(bytes.len());
    let last = bytes
        .iter()
        .rposition(|byte| *byte != 0 && *byte != b' ')
        .map_or(first, |index| index + 1);
    for &byte in &bytes[first..last] {
        if !(b'0'..=b'7').contains(&byte) {
            return Err(ArchiveError::Format);
        }
        result = result
            .checked_mul(8)
            .and_then(|n| n.checked_add(u64::from(byte - b'0')))
            .ok_or(ArchiveError::Overflow)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    fn tar(files: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut result = Vec::new();
        for (name, kind, data) in files {
            let mut header = [0; 512];
            header[..name.len()].copy_from_slice(name.as_bytes());
            header[100..108].copy_from_slice(b"0000555\0");
            let length = if *kind == b'2' { 0 } else { data.len() };
            let size = alloc::format!("{length:011o}\0");
            header[124..136].copy_from_slice(size.as_bytes());
            header[148..156].fill(b' ');
            header[156] = *kind;
            if *kind == b'2' {
                header[157..157 + data.len()].copy_from_slice(data);
            }
            header[257..265].copy_from_slice(b"ustar\000");
            let checksum: u64 = header.iter().map(|b| u64::from(*b)).sum();
            header[148..156].copy_from_slice(alloc::format!("{checksum:06o}\0 ").as_bytes());
            result.extend_from_slice(&header);
            if length != 0 {
                result.extend_from_slice(data);
            }
            result.resize((result.len() + 511) & !511, 0);
        }
        result.resize(result.len() + 1024, 0);
        result
    }
    #[test]
    fn canonical_paths_stay_inside_the_runtime_root() {
        assert_eq!(
            Path::normalize("apex/runtime", "../lib/./x")
                .unwrap()
                .as_str(),
            "apex/runtime/../lib/x"
        );
        assert_eq!(
            Path::normalize("ignored", "/../../system//bin/app")
                .unwrap()
                .as_str(),
            "../../system/bin/app"
        );
        assert!(Path::normalize("", "").is_err());
        assert!(Path::normalize("", &"a".repeat(PATH_MAX + 1)).is_err());
    }
    #[test]
    fn resolves_files_implicit_directories_and_symlinks() {
        let bytes = tar(&[
            ("apex/runtime/bin/linker64", b'0', b"ELF bytes"),
            ("system/bin/linker64", b'2', b"/apex/runtime/bin/linker64"),
            ("alias", b'2', b"apex/runtime"),
        ]);
        let archive = Archive::parse(&bytes).unwrap();
        let file = archive
            .resolve(&Path::normalize("", "/system/bin/linker64").unwrap(), true)
            .unwrap();
        assert_eq!(file.data, b"ELF bytes");
        assert_eq!(file.path, "apex/runtime/bin/linker64");
        assert_eq!(
            archive
                .resolve(&Path::normalize("", "alias/bin/linker64").unwrap(), true)
                .unwrap()
                .inode,
            file.inode
        );
        assert_eq!(
            archive
                .resolve(&Path::normalize("", "apex/runtime").unwrap(), true)
                .unwrap()
                .kind,
            Kind::Directory
        );
        assert_eq!(
            archive
                .resolve(&Path::normalize("", "system/bin/linker64").unwrap(), false)
                .unwrap()
                .kind,
            Kind::Symlink
        );
    }
    #[test]
    fn rejects_corruption_duplicate_names_and_symlink_loops() {
        let mut bytes = tar(&[("file", b'0', b"contents")]);
        bytes[0] ^= 1;
        assert!(matches!(
            Archive::parse(&bytes),
            Err(ArchiveError::Checksum)
        ));
        assert!(matches!(
            Archive::parse(&tar(&[("file", b'0', b"a"), ("file", b'0', b"b")])),
            Err(ArchiveError::Duplicate)
        ));
        assert!(matches!(
            Archive::parse(&tar(&[("../escape", b'0', b"a")])),
            Err(ArchiveError::Path)
        ));
        let bytes = tar(&[("a", b'2', b"b"), ("b", b'2', b"a")]);
        assert_eq!(
            Archive::parse(&bytes)
                .unwrap()
                .resolve(&Path::normalize("", "a").unwrap(), true)
                .err(),
            Some(ELOOP)
        );
    }

    #[test]
    fn resolves_dot_dot_after_symlinks_and_checks_every_prefix() {
        let bytes = tar(&[
            ("a/b/file", b'0', b"inside"),
            ("a/file", b'0', b"parent"),
            ("alias", b'2', b"a/b"),
            ("file", b'0', b"root"),
        ]);
        let archive = Archive::parse(&bytes).unwrap();
        let resolve = |name| archive.resolve(&Path::normalize("", name).unwrap(), true);
        assert_eq!(resolve("alias/../file").unwrap().data, b"parent");
        assert_eq!(resolve("/../../file").unwrap().data, b"root");
        assert_eq!(resolve("missing/../file").err(), Some(ENOENT));
        assert_eq!(resolve("file/../file").err(), Some(20));
        assert_eq!(resolve("file/").err(), Some(20));
        assert_eq!(resolve("file/.").err(), Some(20));
        assert_eq!(resolve("alias/").unwrap().kind, Kind::Directory);
        assert!(matches!(
            Archive::parse(&tar(&[("a", b'0', b"x"), ("a/b", b'0', b"y")])),
            Err(ArchiveError::Path)
        ));
    }
}
