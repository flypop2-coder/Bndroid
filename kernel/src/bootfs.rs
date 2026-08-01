use core::fmt;

use crate::vmo::Vmo;

/// Maximum UTF-8 byte length of a canonical capability-root-relative path.
pub const BOOTFS_MAX_PATH_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootfsError {
    EmptyPath,
    AbsolutePath,
    PathTooLong,
    NonCanonical,
    DuplicatePath,
    NotFound,
}

/// One immutable bootfs entry with a validated, root-relative static path.
#[derive(Clone)]
pub struct BootFile {
    path: &'static str,
    vmo: Vmo,
}

impl BootFile {
    pub fn try_new(path: &'static str, vmo: Vmo) -> Result<Self, BootfsError> {
        validate_path(path)?;
        Ok(Self { path, vmo })
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub fn len(&self) -> usize {
        self.vmo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vmo.is_empty()
    }

    pub const fn vmo(&self) -> &Vmo {
        &self.vmo
    }
}

impl fmt::Debug for BootFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BootFile")
            .field("path", &self.path)
            .field("size", &self.len())
            .field("vmo", &self.vmo)
            .finish()
    }
}

/// An allocation-free, compile-time-capacity catalog of immutable boot files.
#[derive(Clone, Debug)]
pub struct BootfsCatalog<const N: usize> {
    files: [BootFile; N],
}

impl<const N: usize> BootfsCatalog<N> {
    pub fn try_new(files: [BootFile; N]) -> Result<Self, BootfsError> {
        for (index, file) in files.iter().enumerate() {
            // Keep this validation at the catalog boundary so future internal
            // construction paths cannot bypass the canonical-path invariant.
            validate_path(file.path)?;
            if files[..index]
                .iter()
                .any(|existing| existing.path == file.path)
            {
                return Err(BootfsError::DuplicatePath);
            }
        }
        Ok(Self { files })
    }

    /// Builds a catalog directly from static path/VMO entries.
    pub fn try_from_entries(entries: [(&'static str, Vmo); N]) -> Result<Self, BootfsError> {
        for (index, (path, _)) in entries.iter().enumerate() {
            validate_path(path)?;
            if entries[..index]
                .iter()
                .any(|(existing, _)| existing == path)
            {
                return Err(BootfsError::DuplicatePath);
            }
        }
        let files = entries.map(|(path, vmo)| BootFile { path, vmo });
        Ok(Self { files })
    }

    pub const fn len(&self) -> usize {
        N
    }

    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    /// Opens only an exact path beneath the catalog's capability root and
    /// returns shared VMO identity plus the immutable byte size.
    pub fn open(&self, path: &str) -> Result<(Vmo, usize), BootfsError> {
        self.files
            .iter()
            .find(|file| file.path == path)
            .map(|file| (file.vmo.clone(), file.len()))
            .ok_or(BootfsError::NotFound)
    }
}

/// Validates one capability-root-relative path without opening a catalog.
pub fn validate_path(path: &str) -> Result<(), BootfsError> {
    if path.is_empty() {
        return Err(BootfsError::EmptyPath);
    }
    if path.len() > BOOTFS_MAX_PATH_BYTES {
        return Err(BootfsError::PathTooLong);
    }
    if path.starts_with('/') {
        return Err(BootfsError::AbsolutePath);
    }
    if path.ends_with('/') || path.as_bytes().contains(&0) {
        return Err(BootfsError::NonCanonical);
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(BootfsError::NonCanonical);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BOOTFS_MAX_PATH_BYTES, BootFile, BootfsCatalog, BootfsError};
    use crate::vmo::Vmo;

    fn vmo(bytes: &[u8]) -> Vmo {
        Vmo::try_from_slice(bytes).unwrap()
    }

    #[test]
    fn opens_exact_paths_with_shared_identity_and_size() {
        let hello = vmo(b"hello\n");
        let build = vmo(b"M24\n");
        let catalog = BootfsCatalog::try_new([
            BootFile::try_new("HELLO.TXT", hello.clone()).unwrap(),
            BootFile::try_new("SYSTEM/BUILD.TXT", build.clone()).unwrap(),
        ])
        .unwrap();

        assert_eq!(catalog.len(), 2);
        assert!(!catalog.is_empty());
        let (opened, size) = catalog.open("HELLO.TXT").unwrap();
        assert_eq!(size, 6);
        assert!(opened.same_vmo(&hello));
        assert_eq!(opened.read_range(0, size).unwrap(), b"hello\n");

        let (opened_again, _) = catalog.open("HELLO.TXT").unwrap();
        assert!(opened.same_vmo(&opened_again));
        let (opened_build, _) = catalog.open("SYSTEM/BUILD.TXT").unwrap();
        assert!(build.same_vmo(&opened_build));
    }

    #[test]
    fn direct_entry_construction_preserves_static_paths() {
        let source = vmo(b"configuration");
        let catalog = BootfsCatalog::try_from_entries([("CONFIG.TXT", source.clone())]).unwrap();
        let (opened, size) = catalog.open("CONFIG.TXT").unwrap();
        assert_eq!(size, source.len());
        assert!(opened.same_vmo(&source));
    }

    #[test]
    fn rejects_empty_absolute_and_overlong_paths() {
        const SIXTY_FOUR: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const TOO_LONG: &str = concat!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "a"
        );
        assert_eq!(TOO_LONG.len(), BOOTFS_MAX_PATH_BYTES + 1);

        assert_eq!(
            BootFile::try_new("", vmo(b"x")).unwrap_err(),
            BootfsError::EmptyPath
        );
        assert_eq!(
            BootFile::try_new("/SYSTEM/FILE", vmo(b"x")).unwrap_err(),
            BootfsError::AbsolutePath
        );
        assert_eq!(
            BootFile::try_new(TOO_LONG, vmo(b"x")).unwrap_err(),
            BootfsError::PathTooLong
        );
        assert_eq!(SIXTY_FOUR.len(), 64);
    }

    #[test]
    fn rejects_noncanonical_path_segments_and_trailing_slashes() {
        for path in [
            "SYSTEM//HELLO.TXT",
            "SYSTEM/./HELLO.TXT",
            "SYSTEM/../HELLO.TXT",
            "SYSTEM/.",
            "SYSTEM/..",
            "SYSTEM/HELLO.TXT/",
            "SYSTEM/HELLO\0.TXT",
        ] {
            assert_eq!(
                BootFile::try_new(path, vmo(b"x")).unwrap_err(),
                BootfsError::NonCanonical,
                "path {path:?} must be rejected"
            );
        }
    }

    #[test]
    fn catalog_rejects_duplicate_canonical_paths() {
        let result = BootfsCatalog::try_new([
            BootFile::try_new("SAME.TXT", vmo(b"first")).unwrap(),
            BootFile::try_new("SAME.TXT", vmo(b"second")).unwrap(),
        ]);
        assert_eq!(result.unwrap_err(), BootfsError::DuplicatePath);

        let result = BootfsCatalog::try_from_entries([
            ("SAME.TXT", vmo(b"first")),
            ("SAME.TXT", vmo(b"second")),
        ]);
        assert_eq!(result.unwrap_err(), BootfsError::DuplicatePath);
    }

    #[test]
    fn open_is_case_sensitive_exact_match_and_unknown_is_not_found() {
        let catalog = BootfsCatalog::try_from_entries([("HELLO.TXT", vmo(b"hello"))]).unwrap();
        for unknown in [
            "hello.txt",
            "HELLO.TXT/",
            "SYSTEM//HELLO.TXT",
            "/HELLO.TXT",
            "",
        ] {
            assert!(matches!(catalog.open(unknown), Err(BootfsError::NotFound)));
        }
    }

    #[test]
    fn empty_fixed_capacity_catalog_is_valid_and_never_opens() {
        let catalog = BootfsCatalog::<0>::try_new([]).unwrap();
        assert_eq!(catalog.len(), 0);
        assert!(catalog.is_empty());
        assert!(matches!(
            catalog.open("HELLO.TXT"),
            Err(BootfsError::NotFound)
        ));
    }
}
