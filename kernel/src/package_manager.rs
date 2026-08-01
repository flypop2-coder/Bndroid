//! Trusted, allocation-free AndroidBox Install-0 package host.
//!
//! This module owns one bounded boot transaction plus a single-slot, read-only
//! runtime relaunch service. Boot accepts an APK from a trusted kernel source,
//! admits the complete source before any disk mutation, commits it through
//! `bndr-package-store`, reads the committed APK back into a separate kernel
//! buffer, and publishes only evidence derived from that durable readback. A
//! later launcher request is serviced outside its IRQ-masked SVC frame by
//! freshly recovering and rereading the durable APK into independent scratch,
//! then repeating complete Resources-1 Activity validation and execution. It
//! grants no Android bytecode a Bndroid handle, device capability, network
//! path, or EL0 storage authority.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

#[cfg(feature = "androidbox-el0-runtime0")]
use bndr_abi::AndroidPackageImageClaim;
#[cfg(feature = "androidbox-runtime-uninstall1")]
use bndr_abi::AndroidPackageUninstallRequest;
use bndr_abi::{AndroidPackageCompatibilityProfile, AndroidPackageRelaunchRequest};
#[cfg(feature = "androidbox-runtime-install2")]
use bndr_abi::{
    AndroidPackageInstallAction, AndroidPackageInstallCandidate,
    AndroidPackageInstallCandidateMetadata, AndroidPackageInstallRequest,
};
#[cfg(feature = "androidbox-manifest-catalog3")]
use bndr_androidbox::AndroidBoxManifestCatalogScratch;
#[cfg(feature = "androidbox-icon-resources5")]
use bndr_androidbox::AndroidLauncherIcon;
use bndr_androidbox::{
    ActivityLaunch, AndroidBox, ApkSignatureError, ApkSignerInfo, Error as AndroidBoxError,
    verify_apk_v2,
};
#[cfg(feature = "androidbox-apk-envelope4")]
use bndr_androidbox::{AndroidBoxEnvelopeScratch, ApkEnvelopeInfo};
#[cfg(not(feature = "androidbox-multipackage4"))]
use bndr_package_store::VOLUME_SECTORS;
use bndr_package_store::{
    CompatibilityProfile, DataDisposition, Error as PackageStoreError, InstallMetadata,
    InstalledPackage, IoError, MAX_ACTIVITY_NAME_BYTES, MAX_APK_BYTES, MAX_PACKAGE_NAME_BYTES,
    MetadataError, PackageState, RemovedPackage, Sector, SectorIo,
};
#[cfg(feature = "androidbox-multipackage4")]
use bndr_package_store::{
    MULTI_PACKAGE_CAPACITY, MULTI_PACKAGE_VOLUME_SECTORS, MultiInstalledPackage,
    MultiPackageCatalog, MultiPackageError, ensure_multi_formatted, inspect_multi, install_multi,
    read_multi_blob, recover_multi, uninstall_multi,
};
#[cfg(not(feature = "androidbox-multipackage4"))]
use bndr_package_store::{format, install, read_blob, recover_state, uninstall};
use bndr_sm::verified_manifest::{Sha256, sha256};
#[cfg(feature = "androidbox-el0-runtime0")]
use bndroid_kernel::vmo::{Vmo, VmoCreateError};
use bndroid_kernel::{
    fw_cfg::{
        PackageUninstallDataDisposition, PackageUninstallRequest, PackageUninstallRequestError,
        parse_package_uninstall_request,
    },
    gpt::{BNDROID_PACKAGES_PARTITION_TYPE_GUID, GptEvidence},
};

use crate::arch::aarch64;
use crate::driver::virtio::block::{BlockError, PhysicalCounter, deadline_after};
use crate::storage::{self, StorageError};

pub const PACKAGES_DEVICE_SECTORS: u64 = 32_768;
pub const PACKAGES_PARTITION_FIRST_LBA: u64 = 16_384;
#[cfg(feature = "androidbox-multipackage4")]
pub const PACKAGES_PARTITION_SECTORS: u64 = MULTI_PACKAGE_VOLUME_SECTORS;
#[cfg(not(feature = "androidbox-multipackage4"))]
pub const PACKAGES_PARTITION_SECTORS: u64 = VOLUME_SECTORS;
pub const PACKAGES_PARTITION_LAST_LBA: u64 =
    PACKAGES_PARTITION_FIRST_LBA + PACKAGES_PARTITION_SECTORS - 1;
pub const PACKAGES_PARTITION_ENTRY_INDEX: u32 = 3;

/// GPT raw byte order for
/// `d25cf174-a879-4e5a-9364-67b2d8905801`.
pub const PACKAGES_PARTITION_UNIQUE_GUID: [u8; 16] = [
    0x74, 0xf1, 0x5c, 0xd2, 0x79, 0xa8, 0x5a, 0x4e, 0x93, 0x64, 0x67, 0xb2, 0xd8, 0x90, 0x58, 0x01,
];

const PACKAGES_PARTITION_NAME: [u16; 16] = [
    b'B' as u16,
    b'N' as u16,
    b'D' as u16,
    b'R' as u16,
    b'O' as u16,
    b'I' as u16,
    b'D' as u16,
    b'_' as u16,
    b'P' as u16,
    b'A' as u16,
    b'C' as u16,
    b'K' as u16,
    b'A' as u16,
    b'G' as u16,
    b'E' as u16,
    b'S' as u16,
];

const STATE_EMPTY: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_READY: u8 = 2;
const STATE_FAILED: u8 = 3;

static PUBLICATION_STATE: AtomicU8 = AtomicU8::new(STATE_EMPTY);

struct ApkPublication {
    bytes: UnsafeCell<[u8; MAX_APK_BYTES]>,
}

impl ApkPublication {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; MAX_APK_BYTES]),
        }
    }
}

// SAFETY: `initialize` obtains the only write lease through the publication
// state transition. The byte array is never mutated after the Release store of
// STATE_READY, and readers first perform an Acquire load of that state.
unsafe impl Sync for ApkPublication {}

static INSTALLED_APK: ApkPublication = ApkPublication::new();

#[cfg(feature = "androidbox-apk-envelope4")]
struct AndroidEnvelopeScratchPublication {
    value: UnsafeCell<AndroidBoxEnvelopeScratch>,
}

#[cfg(feature = "androidbox-apk-envelope4")]
impl AndroidEnvelopeScratchPublication {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(AndroidBoxEnvelopeScratch::new()),
        }
    }
}

// SAFETY: package admission, durable readback validation, and the relaunch
// monitor all execute synchronously on Bndroid's single core. None retains a
// borrow into this workspace: `load_with_scratch` returns an image borrowing
// only the immutable APK and wipes the workspace before returning.
#[cfg(feature = "androidbox-apk-envelope4")]
unsafe impl Sync for AndroidEnvelopeScratchPublication {}

#[cfg(feature = "androidbox-apk-envelope4")]
static ANDROID_ENVELOPE_SCRATCH: AndroidEnvelopeScratchPublication =
    AndroidEnvelopeScratchPublication::new();

#[cfg(feature = "androidbox-manifest-catalog3")]
struct AndroidManifestCatalogScratchPublication {
    value: UnsafeCell<AndroidBoxManifestCatalogScratch>,
}

// SAFETY: the package manager is single-core and synchronous. The catalog
// loader wipes this non-retained workspace before returning on every path.
#[cfg(feature = "androidbox-manifest-catalog3")]
unsafe impl Sync for AndroidManifestCatalogScratchPublication {}

#[cfg(feature = "androidbox-manifest-catalog3")]
static ANDROID_MANIFEST_CATALOG_SCRATCH: AndroidManifestCatalogScratchPublication =
    AndroidManifestCatalogScratchPublication {
        value: UnsafeCell::new(AndroidBoxManifestCatalogScratch::new()),
    };

#[derive(Clone, Copy)]
struct LivePackageCatalog {
    formatted: bool,
    installed: Option<InstalledPackage>,
    package: Option<InstalledPackageSnapshot>,
    removed: Option<RemovedPackageSnapshot>,
    #[cfg(feature = "androidbox-multipackage4")]
    multi_installed: [Option<MultiInstalledPackage>; MULTI_PACKAGE_CAPACITY],
    #[cfg(feature = "androidbox-multipackage4")]
    multi_packages: [Option<InstalledPackageSnapshot>; MULTI_PACKAGE_CAPACITY],
    #[cfg(feature = "androidbox-multipackage4")]
    revision: u64,
}

impl LivePackageCatalog {
    const fn new() -> Self {
        Self {
            formatted: false,
            installed: None,
            package: None,
            removed: None,
            #[cfg(feature = "androidbox-multipackage4")]
            multi_installed: [None; MULTI_PACKAGE_CAPACITY],
            #[cfg(feature = "androidbox-multipackage4")]
            multi_packages: [None; MULTI_PACKAGE_CAPACITY],
            #[cfg(feature = "androidbox-multipackage4")]
            revision: 0,
        }
    }
}

struct LivePackageCatalogPublication {
    value: UnsafeCell<LivePackageCatalog>,
}

impl LivePackageCatalogPublication {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(LivePackageCatalog::new()),
        }
    }
}

// SAFETY: boot initialization owns the pre-publication write lease. ABI 52
// runtime mutation and every live reader are serialized by the local IRQ mask
// on Bndroid's single core; durable I/O happens before the short publication
// critical section.
unsafe impl Sync for LivePackageCatalogPublication {}

static LIVE_PACKAGE_CATALOG: LivePackageCatalogPublication = LivePackageCatalogPublication::new();

#[cfg(feature = "androidbox-runtime-install2")]
struct InstallCandidatePublication {
    value: UnsafeCell<Option<AndroidPackageInstallCandidate>>,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl InstallCandidatePublication {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }
}

// SAFETY: boot publication, ABI 53 runtime consumption, and candidate readers
// are serialized by the single-core IRQ-masked publication domain.
#[cfg(feature = "androidbox-runtime-install2")]
unsafe impl Sync for InstallCandidatePublication {}

#[cfg(feature = "androidbox-runtime-install2")]
static INSTALL_CANDIDATE: InstallCandidatePublication = InstallCandidatePublication::new();

struct EvidencePublication {
    value: UnsafeCell<PackageBootEvidence>,
}

impl EvidencePublication {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(PackageBootEvidence::EMPTY),
        }
    }
}

// SAFETY: identical one-writer/Release-publication discipline to
// `INSTALLED_APK`; a published evidence value is immutable.
unsafe impl Sync for EvidencePublication {}

static BOOT_EVIDENCE: EvidencePublication = EvidencePublication::new();

/// Fixed ASCII retained in a `Copy` boot snapshot without allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedText<const N: usize> {
    length: u16,
    bytes: [u8; N],
}

impl<const N: usize> FixedText<N> {
    const EMPTY: Self = Self {
        length: 0,
        bytes: [0; N],
    };

    fn try_from_ascii(value: &str) -> Result<Self, Error> {
        if value.len() > N || !value.is_ascii() {
            return Err(Error::SnapshotText);
        }
        let mut result = Self::EMPTY;
        result.length = value.len() as u16;
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(result)
    }

    pub const fn len(&self) -> usize {
        self.length as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("package snapshot contains validated ASCII")
    }
}

#[cfg(feature = "androidbox-icon-resources5")]
const INSTALLED_LAUNCHER_ICON_PALETTE_CAPACITY: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
const INSTALLED_LAUNCHER_ICON_PACKED_BYTES: usize = bndr_abi::ANDROID_PACKAGE_ICON_PIXEL_COUNT / 2;

/// Stack-safe retained form of one exact signed APK launcher icon.
///
/// ABI 56 expands these four-bit indices into canonical ARGB pixels only while
/// copying into its separately leased static wire workspace.
#[cfg(feature = "androidbox-icon-resources5")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstalledLauncherIcon {
    resource_id: u32,
    png_crc32: u32,
    palette: [u32; INSTALLED_LAUNCHER_ICON_PALETTE_CAPACITY],
    palette_len: u8,
    indices: [u8; INSTALLED_LAUNCHER_ICON_PACKED_BYTES],
    #[cfg(feature = "androidbox-density-icons7")]
    source_width: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    source_height: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    source_density_dpi: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    color_quantized: bool,
}

#[cfg(feature = "androidbox-icon-resources5")]
impl InstalledLauncherIcon {
    pub const fn resource_id(self) -> u32 {
        self.resource_id
    }

    pub const fn png_crc32(self) -> u32 {
        self.png_crc32
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_width(self) -> u16 {
        self.source_width
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_height(self) -> u16 {
        self.source_height
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_density_dpi(self) -> u16 {
        self.source_density_dpi
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn color_quantized(self) -> bool {
        self.color_quantized
    }

    pub fn pixel(self, index: usize) -> Option<u32> {
        if index >= bndr_abi::ANDROID_PACKAGE_ICON_PIXEL_COUNT {
            return None;
        }
        let packed = self.indices[index / 2];
        let palette_index = if index.is_multiple_of(2) {
            packed >> 4
        } else {
            packed & 0x0f
        };
        (usize::from(palette_index) < usize::from(self.palette_len))
            .then_some(self.palette[usize::from(palette_index)])
    }
}

/// Complete identity and Resources-1 launch evidence for the durable APK.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstalledPackageSnapshot {
    pub generation: u64,
    pub slot: u8,
    pub apk_length: u32,
    pub version_code: u64,
    pub package: FixedText<MAX_PACKAGE_NAME_BYTES>,
    pub activity: FixedText<MAX_ACTIVITY_NAME_BYTES>,
    pub signer_cert_sha256: [u8; 32],
    pub apk_sha256: [u8; 32],
    pub compatibility_profile: CompatibilityProfile,
    pub launch_title: FixedText<128>,
    pub launch_text: FixedText<128>,
    pub resources_arsc_crc32: u32,
    pub layout_xml_crc32: u32,
    pub layout_resource_id: u32,
    pub text_resource_id: u32,
    pub launch_constructor_method_index: u32,
    pub launch_constructor_code_offset: u32,
    pub launch_constructor_instruction_count: u16,
    pub launch_on_create_method_index: u32,
    pub launch_on_create_code_offset: u32,
    pub launch_instruction_count: u16,
    /// Exact signed APK launcher artwork, decoded during durable validation.
    #[cfg(feature = "androidbox-icon-resources5")]
    pub launcher_icon: Option<InstalledLauncherIcon>,
    /// ZIP/DEFLATE shape proven from the exact durable APK readback.
    #[cfg(feature = "androidbox-apk-envelope4")]
    pub envelope: ApkEnvelopeInfo,
}

/// Durable logical-removal evidence retained without exposing stale APK bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemovedPackageSnapshot {
    pub operation_id: u64,
    pub generation: u64,
    pub last_generation: u64,
    pub last_version_code: u64,
    pub last_apk_length: u32,
    pub last_blob_slot: u8,
    pub package: FixedText<MAX_PACKAGE_NAME_BYTES>,
    pub last_apk_sha256: [u8; 32],
    pub last_signer_cert_sha256: [u8; 32],
    pub data_disposition: DataDisposition,
}

/// Exact synchronous I/O completed by the package-store boot transaction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PackageIoEvidence {
    pub reads: u64,
    pub writes: u64,
    pub flushes: u64,
}

/// Terminal evidence from one fresh, read-only durable Activity relaunch.
#[cfg_attr(
    not(feature = "androidbox-el0-runtime0"),
    derive(Clone, Copy, Eq, PartialEq)
)]
#[derive(Debug)]
pub struct RelaunchCompletion {
    pub request_sequence: u64,
    pub package: InstalledPackageSnapshot,
    pub io: PackageIoEvidence,
    #[cfg(feature = "androidbox-el0-runtime0")]
    pub image: Vmo,
}

/// Public result of polling the single boot-local relaunch slot.
#[allow(clippy::large_enum_variant)]
pub enum RelaunchPoll {
    Pending,
    Ready(Result<RelaunchCompletion, RelaunchError>),
}

#[cfg(feature = "androidbox-el0-runtime0")]
struct DurableRelaunch {
    package: InstalledPackageSnapshot,
    image: Vmo,
}

#[cfg(not(feature = "androidbox-el0-runtime0"))]
type DurableRelaunch = InstalledPackageSnapshot;

#[cfg(feature = "androidbox-el0-runtime0")]
fn durable_relaunch_package(relaunch: &DurableRelaunch) -> InstalledPackageSnapshot {
    relaunch.package
}

#[cfg(not(feature = "androidbox-el0-runtime0"))]
fn durable_relaunch_package(relaunch: &DurableRelaunch) -> InstalledPackageSnapshot {
    *relaunch
}

/// Stable failure classes exposed to the syscall boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelaunchError {
    /// The caller generation or requested installed identity is no longer
    /// current.
    Stale,
    /// Boot or fresh recovery exposes no installed package.
    NotInstalled,
    /// The request sequence is zero, replayed, or has a gap.
    Sequence,
    /// The sole slot belongs to a different exact request.
    Busy,
    /// Durable bytes or package metadata failed complete revalidation.
    Corrupt,
    /// The durable package is outside the supported Resources-1 profile.
    Unsupported,
    /// The block path cannot safely continue before device recovery/reset.
    RequiresReset,
    /// A verified immutable package image could not be allocated.
    OutOfMemory,
}

impl RelaunchError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::NotInstalled => "not-installed",
            Self::Sequence => "sequence",
            Self::Busy => "busy",
            Self::Corrupt => "corrupt",
            Self::Unsupported => "unsupported",
            Self::RequiresReset => "requires-reset",
            Self::OutOfMemory => "out-of-memory",
        }
    }
}

const RELAUNCH_EMPTY: u8 = 0;
const RELAUNCH_PENDING: u8 = 1;
const RELAUNCH_RUNNING: u8 = 2;
const RELAUNCH_COMPLETE: u8 = 3;

struct RelaunchWorkspace {
    compatible_session_id: u64,
    request: Option<AndroidPackageRelaunchRequest>,
    outcome: Option<Result<RelaunchCompletion, RelaunchError>>,
}

impl RelaunchWorkspace {
    const fn new() -> Self {
        Self {
            compatible_session_id: 0,
            request: None,
            outcome: None,
        }
    }
}

struct RelaunchService {
    state: AtomicU8,
    owner: AtomicU64,
    value: UnsafeCell<RelaunchWorkspace>,
}

impl RelaunchService {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(RELAUNCH_EMPTY),
            owner: AtomicU64::new(0),
            value: UnsafeCell::new(RelaunchWorkspace::new()),
        }
    }
}

// SAFETY: the IRQ-masked syscall domain is the sole PENDING/COMPLETE
// publisher/consumer. The IRQ-enabled monitor acquires exclusive mutable
// access only after PENDING -> RUNNING and Release-publishes COMPLETE.
unsafe impl Sync for RelaunchService {}

struct RelaunchScratch {
    bytes: UnsafeCell<[u8; MAX_APK_BYTES]>,
}

impl RelaunchScratch {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; MAX_APK_BYTES]),
        }
    }
}

// SAFETY: only the monitor owning RELAUNCH_RUNNING can dereference this
// independent scratch buffer. It is wiped before COMPLETE is published.
unsafe impl Sync for RelaunchScratch {}

const _: () = assert!(MAX_APK_BYTES == 65_024);

static RELAUNCH_SERVICE: RelaunchService = RelaunchService::new();
static RELAUNCH_SCRATCH: RelaunchScratch = RelaunchScratch::new();
static RELAUNCH_LEDGER_OWNER: AtomicU64 = AtomicU64::new(0);
static RELAUNCH_LEDGER_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static RELAUNCH_DEVICE_SECTORS: AtomicU64 = AtomicU64::new(0);
static RELAUNCH_DEADLINE_BUDGET: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeUninstallCompletion {
    pub request_sequence: u64,
    pub operation_id: u64,
    pub last_generation: u64,
    pub removal_generation: u64,
    pub io: PackageIoEvidence,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeUninstallError {
    Stale,
    NotInstalled,
    Sequence,
    Busy,
    Corrupt,
    Unsupported,
    RequiresReset,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl RuntimeUninstallError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::NotInstalled => "not-installed",
            Self::Sequence => "sequence",
            Self::Busy => "busy",
            Self::Corrupt => "corrupt",
            Self::Unsupported => "unsupported",
            Self::RequiresReset => "requires-reset",
        }
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
pub enum RuntimeUninstallPoll {
    Pending,
    Ready(Result<RuntimeUninstallCompletion, RuntimeUninstallError>),
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
const UNINSTALL_EMPTY: u8 = 0;
#[cfg(feature = "androidbox-runtime-uninstall1")]
const UNINSTALL_PENDING: u8 = 1;
#[cfg(feature = "androidbox-runtime-uninstall1")]
const UNINSTALL_RUNNING: u8 = 2;
#[cfg(feature = "androidbox-runtime-uninstall1")]
const UNINSTALL_COMPLETE: u8 = 3;

#[cfg(feature = "androidbox-runtime-uninstall1")]
struct RuntimeUninstallWorkspace {
    request: Option<AndroidPackageUninstallRequest>,
    outcome: Option<Result<RuntimeUninstallCompletion, RuntimeUninstallError>>,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl RuntimeUninstallWorkspace {
    const fn new() -> Self {
        Self {
            request: None,
            outcome: None,
        }
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
struct RuntimeUninstallService {
    state: AtomicU8,
    owner: AtomicU64,
    value: UnsafeCell<RuntimeUninstallWorkspace>,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl RuntimeUninstallService {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(UNINSTALL_EMPTY),
            owner: AtomicU64::new(0),
            value: UnsafeCell::new(RuntimeUninstallWorkspace::new()),
        }
    }
}

// SAFETY: syscall submission/collection is IRQ-masked. The IRQ-enabled
// monitor alone transitions PENDING -> RUNNING and publishes COMPLETE.
#[cfg(feature = "androidbox-runtime-uninstall1")]
unsafe impl Sync for RuntimeUninstallService {}

#[cfg(feature = "androidbox-runtime-uninstall1")]
static RUNTIME_UNINSTALL_SERVICE: RuntimeUninstallService = RuntimeUninstallService::new();
#[cfg(feature = "androidbox-runtime-uninstall1")]
static RUNTIME_UNINSTALL_LEDGER_OWNER: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "androidbox-runtime-uninstall1")]
static RUNTIME_UNINSTALL_LEDGER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeInstallCompletion {
    pub request_sequence: u64,
    pub operation_id: u64,
    pub candidate_id: u64,
    pub previous_generation: u64,
    pub installed_generation: u64,
    pub installed_version_code: u64,
    pub apk_length: u32,
    pub action: AndroidPackageInstallAction,
    pub apk_sha256: [u8; 32],
    pub signer_sha256: [u8; 32],
    pub io: PackageIoEvidence,
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallError {
    Stale,
    NoCandidate,
    Sequence,
    Busy,
    Corrupt,
    Unsupported,
    RequiresReset,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl RuntimeInstallError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::NoCandidate => "no-candidate",
            Self::Sequence => "sequence",
            Self::Busy => "busy",
            Self::Corrupt => "corrupt",
            Self::Unsupported => "unsupported",
            Self::RequiresReset => "requires-reset",
        }
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
pub enum RuntimeInstallPoll {
    Pending,
    Ready(Result<RuntimeInstallCompletion, RuntimeInstallError>),
}

#[cfg(feature = "androidbox-runtime-install2")]
const INSTALL_EMPTY: u8 = 0;
#[cfg(feature = "androidbox-runtime-install2")]
const INSTALL_PENDING: u8 = 1;
#[cfg(feature = "androidbox-runtime-install2")]
const INSTALL_RUNNING: u8 = 2;
#[cfg(feature = "androidbox-runtime-install2")]
const INSTALL_COMPLETE: u8 = 3;

#[cfg(feature = "androidbox-runtime-install2")]
struct RuntimeInstallWorkspace {
    request: Option<AndroidPackageInstallRequest>,
    outcome: Option<Result<RuntimeInstallCompletion, RuntimeInstallError>>,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl RuntimeInstallWorkspace {
    const fn new() -> Self {
        Self {
            request: None,
            outcome: None,
        }
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
struct RuntimeInstallService {
    state: AtomicU8,
    owner: AtomicU64,
    value: UnsafeCell<RuntimeInstallWorkspace>,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl RuntimeInstallService {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(INSTALL_EMPTY),
            owner: AtomicU64::new(0),
            value: UnsafeCell::new(RuntimeInstallWorkspace::new()),
        }
    }
}

// SAFETY: syscall submission/collection is IRQ-masked. The IRQ-enabled
// monitor alone owns RUNNING and Release-publishes COMPLETE.
#[cfg(feature = "androidbox-runtime-install2")]
unsafe impl Sync for RuntimeInstallService {}

#[cfg(feature = "androidbox-runtime-install2")]
struct RuntimeInstallScratch {
    bytes: UnsafeCell<[u8; MAX_APK_BYTES]>,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl RuntimeInstallScratch {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; MAX_APK_BYTES]),
        }
    }
}

// SAFETY: only the IRQ-enabled monitor owning INSTALL_RUNNING dereferences the
// scratch, and it wipes the complete buffer before publishing completion.
#[cfg(feature = "androidbox-runtime-install2")]
unsafe impl Sync for RuntimeInstallScratch {}

#[cfg(feature = "androidbox-runtime-install2")]
static RUNTIME_INSTALL_SERVICE: RuntimeInstallService = RuntimeInstallService::new();
#[cfg(feature = "androidbox-runtime-install2")]
static RUNTIME_INSTALL_SCRATCH: RuntimeInstallScratch = RuntimeInstallScratch::new();
#[cfg(feature = "androidbox-runtime-install2")]
static RUNTIME_INSTALL_LEDGER_OWNER: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "androidbox-runtime-install2")]
static RUNTIME_INSTALL_LEDGER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// One exact, one-shot handoff from Launcher verification to the package
/// execution process (App in ABI 45/46, AndroidApp in ABI 47).
///
/// The grant is never published through a Launcher handle table. Both owner
/// PIDs are generation-qualified and the claim binds the compatible session,
/// package generation, length, and both digests.
#[cfg(feature = "androidbox-el0-runtime0")]
pub struct AndroidPackageImageGrant {
    launcher_owner: u64,
    execution_owner: u64,
    claim: AndroidPackageImageClaim,
    image: Vmo,
}

#[cfg(feature = "androidbox-el0-runtime0")]
struct AndroidPackageImageGrantSlot {
    value: UnsafeCell<Option<AndroidPackageImageGrant>>,
}

#[cfg(feature = "androidbox-el0-runtime0")]
impl AndroidPackageImageGrantSlot {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }
}

// SAFETY: grant publication and consumption occur only in the IRQ-masked
// syscall domain. The IRQ-enabled relaunch monitor transfers its VMO through
// RelaunchCompletion and never accesses this slot.
#[cfg(feature = "androidbox-el0-runtime0")]
unsafe impl Sync for AndroidPackageImageGrantSlot {}

#[cfg(feature = "androidbox-el0-runtime0")]
static ANDROID_PACKAGE_IMAGE_GRANT: AndroidPackageImageGrantSlot =
    AndroidPackageImageGrantSlot::new();

#[cfg(feature = "androidbox-restart0")]
const RESTART_IMAGE_GRANT_EMPTY: u8 = 0;
#[cfg(feature = "androidbox-restart0")]
const RESTART_IMAGE_GRANT_ESCROWED: u8 = 1;
#[cfg(feature = "androidbox-restart0")]
const RESTART_IMAGE_GRANT_REISSUED: u8 = 2;
#[cfg(feature = "androidbox-restart0")]
const RESTART_IMAGE_GRANT_CONSUMED: u8 = 3;

/// One kernel-private strong reference to the exact immutable VMO consumed by
/// the initial AndroidApp generation.
///
/// The reference is captured only after syscall 61 has authenticated and
/// consumed the ordinary one-shot grant. It can move once to the immediately
/// following generation of the same process slot after the restart transcript
/// has authenticated the old generation's fault. It is never exposed through
/// a handle table while held here and is not replenished after that move.
#[cfg(feature = "androidbox-restart0")]
struct AndroidPackageRestartImageGrant {
    state: u8,
    launcher_owner: u64,
    old_execution_owner: u64,
    new_execution_owner: u64,
    claim: Option<AndroidPackageImageClaim>,
    image: Option<Vmo>,
    escrows: u64,
    reissues: u64,
    replacement_claims: u64,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidPackageRestartImageGrant {
    const fn new() -> Self {
        Self {
            state: RESTART_IMAGE_GRANT_EMPTY,
            launcher_owner: 0,
            old_execution_owner: 0,
            new_execution_owner: 0,
            claim: None,
            image: None,
            escrows: 0,
            reissues: 0,
            replacement_claims: 0,
        }
    }
}

#[cfg(feature = "androidbox-restart0")]
struct AndroidPackageRestartImageGrantSlot {
    value: UnsafeCell<AndroidPackageRestartImageGrant>,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidPackageRestartImageGrantSlot {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(AndroidPackageRestartImageGrant::new()),
        }
    }
}

// SAFETY: every mutation is serialized by the same IRQ-masked domain as the
// ordinary grant slot. Read-only monitor snapshots mask IRQs first.
#[cfg(feature = "androidbox-restart0")]
unsafe impl Sync for AndroidPackageRestartImageGrantSlot {}

#[cfg(feature = "androidbox-restart0")]
static ANDROID_PACKAGE_RESTART_IMAGE_GRANT: AndroidPackageRestartImageGrantSlot =
    AndroidPackageRestartImageGrantSlot::new();

#[cfg(feature = "androidbox-restart0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageRestartImageGrantSnapshot {
    pub state: u8,
    pub launcher_owner: u64,
    pub old_execution_owner: u64,
    pub new_execution_owner: u64,
    pub compatible_session_id: u64,
    pub package_generation: u64,
    pub escrows: u64,
    pub reissues: u64,
    pub replacement_claims: u64,
    pub image_held: bool,
    pub ordinary_grant_pending: bool,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidPackageRestartImageGrantSnapshot {
    pub const fn complete(&self) -> bool {
        self.state == RESTART_IMAGE_GRANT_CONSUMED
            && self.launcher_owner != 0
            && self.old_execution_owner != 0
            && self.new_execution_owner != 0
            && self.old_execution_owner != self.new_execution_owner
            && self.compatible_session_id != 0
            && self.package_generation != 0
            && self.escrows == 1
            && self.reissues == 1
            && self.replacement_claims == 1
            && !self.image_held
            && !self.ordinary_grant_pending
    }
}

#[cfg(feature = "androidbox-el0-runtime0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageImageGrantError {
    Missing,
    Mismatch,
    StaleOwner,
    Busy,
}

/// Exact package-store decision made by the one trusted boot pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageBootOperation {
    /// No source and no committed package were present.
    EmptyRecovery,
    /// A source created generation one on an empty volume.
    Install,
    /// A same-package, same-signer, higher-version source replaced the
    /// previous committed generation.
    Update,
    /// A source reinstalled the retained package identity after removal.
    Reinstall,
    /// The exact already-committed source transaction was supplied again.
    SourceReplay,
    /// No source was supplied and the committed package was recovered.
    Recovery,
    /// A canonical request revoked the installed package and launcher entry.
    Uninstall,
    /// A complete canonical uninstall request was replayed without mutation.
    UninstallReplay,
    /// An interrupted one-copy tombstone was repaired to the required mirror.
    UninstallRepair,
    /// No input was supplied and a durable removal tombstone was recovered.
    RemovedRecovery,
    /// ABI 53 admitted a source for a user-confirmed first installation
    /// without mutating the package volume during boot.
    #[cfg(feature = "androidbox-runtime-install2")]
    InstallCandidate,
    /// ABI 53 admitted a same-package, same-signer higher version without
    /// mutating the current installed generation during boot.
    #[cfg(feature = "androidbox-runtime-install2")]
    UpdateCandidate,
    /// ABI 53 admitted a retained-identity reinstall after removal without
    /// mutating the package volume during boot.
    #[cfg(feature = "androidbox-runtime-install2")]
    ReinstallCandidate,
}

impl PackageBootOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyRecovery => "empty-recovery",
            Self::Install => "install",
            Self::Update => "update",
            Self::Reinstall => "reinstall",
            Self::SourceReplay => "source-replay",
            Self::Recovery => "recovery",
            Self::Uninstall => "uninstall",
            Self::UninstallReplay => "uninstall-replay",
            Self::UninstallRepair => "uninstall-repair",
            Self::RemovedRecovery => "removed-recovery",
            #[cfg(feature = "androidbox-runtime-install2")]
            Self::InstallCandidate => "install-candidate",
            #[cfg(feature = "androidbox-runtime-install2")]
            Self::UpdateCandidate => "update-candidate",
            #[cfg(feature = "androidbox-runtime-install2")]
            Self::ReinstallCandidate => "reinstall-candidate",
        }
    }
}

/// Release-published result of the one permitted package-manager boot pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageBootEvidence {
    /// The package-store superblock was valid after this boot pass.
    pub formatted: bool,
    /// This boot pass formatted a previously virgin package volume.
    pub format_performed: bool,
    pub installed: bool,
    /// A supplied source was fully admitted and selected for installation.
    pub source_used: bool,
    /// A supplied canonical removal request was selected for mutation/replay.
    pub uninstall_request_used: bool,
    /// The exact install/update/replay/recovery classification.
    pub operation: PackageBootOperation,
    /// The committed generation observed before a source decision, or zero.
    pub previous_generation: u64,
    /// The committed version observed before a source decision, or zero.
    pub previous_version_code: u64,
    pub package: Option<InstalledPackageSnapshot>,
    pub removed: Option<RemovedPackageSnapshot>,
    #[cfg(feature = "androidbox-runtime-install2")]
    pub install_candidate: Option<AndroidPackageInstallCandidate>,
    pub io: PackageIoEvidence,
}

/// Current authority-free package catalog plus immutable boot provenance.
///
/// ABI 51 and earlier publish the same package for the whole boot. ABI 52 can
/// transition the live package exactly once from Installed to Removed after a
/// durable runtime tombstone; boot-source facts and boot I/O remain unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageCatalogSnapshot {
    pub formatted: bool,
    pub format_performed: bool,
    pub source_used: bool,
    pub package: Option<InstalledPackageSnapshot>,
    pub removed: Option<RemovedPackageSnapshot>,
    pub boot_io: PackageIoEvidence,
}

/// ABI 55's complete, authority-free installed-package directory.
#[cfg(feature = "androidbox-multipackage4")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageDirectorySnapshot {
    pub formatted: bool,
    pub revision: u64,
    pub count: u8,
    pub packages: [Option<InstalledPackageSnapshot>; MULTI_PACKAGE_CAPACITY],
}

impl PackageBootEvidence {
    const EMPTY: Self = Self {
        formatted: false,
        format_performed: false,
        installed: false,
        source_used: false,
        uninstall_request_used: false,
        operation: PackageBootOperation::EmptyRecovery,
        previous_generation: 0,
        previous_version_code: 0,
        package: None,
        removed: None,
        #[cfg(feature = "androidbox-runtime-install2")]
        install_candidate: None,
        io: PackageIoEvidence {
            reads: 0,
            writes: 0,
            flushes: 0,
        },
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GptPolicyError {
    DeviceSize,
    PartitionType,
    PartitionUniqueGuid,
    PartitionIndex,
    PartitionName,
    PartitionBounds,
    PartitionAttributes,
}

impl GptPolicyError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeviceSize => "installed-package GPT has the wrong device size",
            Self::PartitionType => "installed-package GPT type GUID is invalid",
            Self::PartitionUniqueGuid => "installed-package GPT unique GUID is invalid",
            Self::PartitionIndex => "installed-package GPT entry index is invalid",
            Self::PartitionName => "installed-package GPT name is invalid",
            Self::PartitionBounds => "installed-package GPT bounds are invalid",
            Self::PartitionAttributes => "installed-package GPT attributes are unsupported",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized,
    InvalidDeadlineBudget,
    Gpt(GptPolicyError),
    DeviceContract(StorageError),
    DeviceSizeMismatch,
    ReadOnlyDevice,
    SourceEmpty,
    SourceTooLarge,
    Signature(ApkSignatureError),
    AndroidBox(AndroidBoxError),
    Resources1Required,
    Metadata(MetadataError),
    Store(PackageStoreError),
    #[cfg(feature = "androidbox-multipackage4")]
    MultiStore(MultiPackageError),
    DurableLengthMismatch,
    DurableApkDigestMismatch,
    DurableSignerMismatch,
    DurableManifestMismatch,
    DurableTransactionMismatch,
    DurableCompatibilityMismatch,
    ConflictingPackageInputs,
    UninstallRequest(PackageUninstallRequestError),
    UninstallTargetMismatch,
    DurableRemovalMismatch,
    SnapshotText,
    #[cfg(feature = "androidbox-icon-resources5")]
    LauncherIconPalette,
}

impl Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyInitialized => "package manager was already initialized",
            Self::InvalidDeadlineBudget => "package I/O deadline budget is invalid",
            Self::Gpt(error) => error.as_str(),
            Self::DeviceContract(error) => error.as_str(),
            Self::DeviceSizeMismatch => "GPT and live package device sizes do not match",
            Self::ReadOnlyDevice => "installed-package block device is read-only",
            Self::SourceEmpty => "APK source is empty",
            Self::SourceTooLarge => "APK source exceeds the durable package slot",
            Self::Signature(_) => "APK v2 signature admission failed",
            Self::AndroidBox(_) => "AndroidBox Resources-1 admission failed",
            Self::Resources1Required => "APK does not launch through Resources-1",
            Self::Metadata(_) => "APK install metadata is invalid",
            Self::Store(error) => error.as_str(),
            #[cfg(feature = "androidbox-multipackage4")]
            Self::MultiStore(error) => error.as_str(),
            Self::DurableLengthMismatch => "durable APK length does not match its registry",
            Self::DurableApkDigestMismatch => "durable APK digest does not match its registry",
            Self::DurableSignerMismatch => "durable APK signer does not match its registry",
            Self::DurableManifestMismatch => "durable APK manifest does not match its registry",
            Self::DurableTransactionMismatch => {
                "durable APK transaction identity does not match its registry"
            }
            Self::DurableCompatibilityMismatch => {
                "durable APK compatibility profile does not match its registry"
            }
            Self::ConflictingPackageInputs => {
                "APK and package-uninstall inputs are mutually exclusive"
            }
            Self::UninstallRequest(error) => error.as_str(),
            Self::UninstallTargetMismatch => {
                "package-uninstall request does not match durable package state"
            }
            Self::DurableRemovalMismatch => {
                "durable package removal does not match the admitted request"
            }
            Self::SnapshotText => "durable APK text does not fit the package snapshot",
            #[cfg(feature = "androidbox-icon-resources5")]
            Self::LauncherIconPalette => {
                "launcher icon exceeds the bounded sixteen-color runtime palette"
            }
        }
    }
}

impl From<PackageStoreError> for Error {
    fn from(value: PackageStoreError) -> Self {
        Self::Store(value)
    }
}

#[cfg(feature = "androidbox-multipackage4")]
impl From<MultiPackageError> for Error {
    fn from(value: MultiPackageError) -> Self {
        Self::MultiStore(value)
    }
}

impl From<ApkSignatureError> for Error {
    fn from(value: ApkSignatureError) -> Self {
        Self::Signature(value)
    }
}

impl From<AndroidBoxError> for Error {
    fn from(value: AndroidBoxError) -> Self {
        Self::AndroidBox(value)
    }
}

impl From<MetadataError> for Error {
    fn from(value: MetadataError) -> Self {
        Self::Metadata(value)
    }
}

#[derive(Clone, Copy)]
struct AdmittedSource {
    metadata: InstallMetadata,
    apk_length: u32,
    launch_title: FixedText<128>,
    launch_text: FixedText<128>,
    resources_arsc_crc32: u32,
    layout_xml_crc32: u32,
    layout_resource_id: u32,
    text_resource_id: u32,
    instruction_count: u16,
}

/// Runs the one package transaction permitted during this boot.
///
/// Supplying neither input is recovery-only. Supplying both is always an
/// error. An APK is fully admitted before `PackageIo` can write, while a
/// canonical uninstall request is bound to the recovered package generation,
/// version, length, package name, APK digest, and signer before its first
/// tombstone write.
pub fn initialize(
    gpt: GptEvidence,
    source: Option<&[u8]>,
    uninstall_request: Option<&[u8]>,
    deadline_budget: u64,
) -> Result<PackageBootEvidence, Error> {
    if PUBLICATION_STATE
        .compare_exchange(
            STATE_EMPTY,
            STATE_INITIALIZING,
            Ordering::Acquire,
            Ordering::Acquire,
        )
        .is_err()
    {
        return Err(Error::AlreadyInitialized);
    }

    let result = initialize_once(gpt, source, uninstall_request, deadline_budget);
    match result {
        Ok(evidence) => {
            // SAFETY: the successful EMPTY -> INITIALIZING transition grants
            // this call the only write lease. Readers cannot observe either
            // publication until the following Release store.
            unsafe {
                *BOOT_EVIDENCE.value.get() = evidence;
                let live = &mut *LIVE_PACKAGE_CATALOG.value.get();
                live.formatted = evidence.formatted;
                if evidence.package.is_none() {
                    live.installed = None;
                }
                live.package = evidence.package;
                live.removed = evidence.removed;
                #[cfg(feature = "androidbox-runtime-install2")]
                {
                    *INSTALL_CANDIDATE.value.get() = evidence.install_candidate;
                }
            }
            PUBLICATION_STATE.store(STATE_READY, Ordering::Release);
            Ok(evidence)
        }
        Err(error) => {
            // A failed or unknown-outcome boot pass is deliberately not
            // retryable in-place. A reboot must rebuild the block driver and
            // recover the transaction before another installation decision.
            PUBLICATION_STATE.store(STATE_FAILED, Ordering::Release);
            Err(error)
        }
    }
}

/// Returns the immutable boot evidence after successful Release publication.
pub fn snapshot() -> Option<PackageBootEvidence> {
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return None;
    }
    // SAFETY: STATE_READY is stored only after the sole writer completed, and
    // the value is never mutated afterward.
    Some(unsafe { *BOOT_EVIDENCE.value.get() })
}

/// Returns the current authority-free package catalog.
pub fn catalog_snapshot() -> Option<PackageCatalogSnapshot> {
    let saved_daif = aarch64::save_and_mask_irq();
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        aarch64::restore_daif(saved_daif);
        return None;
    }
    // SAFETY: boot initialization completed before STATE_READY. Runtime
    // changes share this same IRQ-masked single-core publication domain.
    let live = unsafe { *LIVE_PACKAGE_CATALOG.value.get() };
    let boot = unsafe { *BOOT_EVIDENCE.value.get() };
    let snapshot = PackageCatalogSnapshot {
        formatted: live.formatted,
        format_performed: boot.format_performed,
        source_used: boot.source_used,
        package: live.package,
        removed: live.removed,
        boot_io: boot.io,
    };
    aarch64::restore_daif(saved_daif);
    Some(snapshot)
}

/// Returns all live installed packages in stable volume order.
#[cfg(feature = "androidbox-multipackage4")]
pub fn directory_snapshot() -> Option<PackageDirectorySnapshot> {
    let saved_daif = aarch64::save_and_mask_irq();
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        aarch64::restore_daif(saved_daif);
        return None;
    }
    // SAFETY: boot and runtime mutations share this IRQ-masked single-core
    // publication domain.
    let live = unsafe { *LIVE_PACKAGE_CATALOG.value.get() };
    let mut packages = [None; MULTI_PACKAGE_CAPACITY];
    let mut count = 0_usize;
    for package in live.multi_packages.iter().flatten().copied() {
        packages[count] = Some(package);
        count += 1;
    }
    let snapshot = PackageDirectorySnapshot {
        formatted: live.formatted,
        revision: live.revision,
        count: count as u8,
        packages,
    };
    aarch64::restore_daif(saved_daif);
    Some(snapshot)
}

/// Returns the current boot-local, authority-free ABI 53 install candidate.
#[cfg(feature = "androidbox-runtime-install2")]
pub fn install_candidate_snapshot() -> Option<AndroidPackageInstallCandidate> {
    let saved_daif = aarch64::save_and_mask_irq();
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        aarch64::restore_daif(saved_daif);
        return None;
    }
    // SAFETY: candidate publication and consumption share this IRQ-masked
    // single-core domain.
    let candidate = unsafe { *INSTALL_CANDIDATE.value.get() };
    aarch64::restore_daif(saved_daif);
    candidate
}

/// Returns only the APK bytes read back from the current committed package.
///
/// No fw_cfg/source slice is ever returned through this API.
pub fn installed_apk_bytes() -> Option<&'static [u8]> {
    let catalog = catalog_snapshot()?;
    let length = catalog.package?.apk_length as usize;
    if length == 0 || length > MAX_APK_BYTES {
        return None;
    }
    // SAFETY: the Acquire in `snapshot` observes all writes preceding the
    // STATE_READY Release store. The buffer is immutable thereafter.
    let bytes = unsafe { &*INSTALLED_APK.bytes.get() };
    Some(&bytes[..length])
}

/// Publishes or exact-retries one generation-bound durable relaunch request.
///
/// This entry point is intentionally confined to the syscall IRQ domain. A
/// canonical enqueue consumes its boot-local sequence before the later
/// IRQ-enabled monitor may perform any I/O.
pub fn poll_relaunch(
    owner: u64,
    compatible_session_id: u64,
    request: AndroidPackageRelaunchRequest,
) -> RelaunchPoll {
    if !aarch64::irq_is_masked() {
        panic!("Android package relaunch polled with IRQ enabled");
    }
    if owner == 0 || !crate::process::android_package_relaunch_owner_can_retry_irq_masked(owner) {
        return RelaunchPoll::Ready(Err(RelaunchError::Stale));
    }
    #[cfg(feature = "androidbox-el0-runtime0")]
    if compatible_session_id == 0 {
        return RelaunchPoll::Ready(Err(RelaunchError::Stale));
    }
    #[cfg(not(feature = "androidbox-el0-runtime0"))]
    if compatible_session_id != 0 {
        return RelaunchPoll::Ready(Err(RelaunchError::Stale));
    }

    match RELAUNCH_SERVICE.state.load(Ordering::Acquire) {
        RELAUNCH_EMPTY => {
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            if RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) != UNINSTALL_EMPTY {
                return RelaunchPoll::Ready(Err(RelaunchError::Busy));
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            if RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) != INSTALL_EMPTY {
                return RelaunchPoll::Ready(Err(RelaunchError::Busy));
            }
            if next_relaunch_sequence(owner, request.request_sequence()).is_err() {
                return RelaunchPoll::Ready(Err(RelaunchError::Sequence));
            }
            if let Err(error) = boot_relaunch_identity(&request) {
                return RelaunchPoll::Ready(Err(error));
            }
            enqueue_relaunch(owner, compatible_session_id, request);
            RelaunchPoll::Pending
        }
        RELAUNCH_PENDING | RELAUNCH_RUNNING => {
            if published_relaunch_matches(owner, compatible_session_id, &request) {
                RelaunchPoll::Pending
            } else {
                RelaunchPoll::Ready(Err(RelaunchError::Busy))
            }
        }
        RELAUNCH_COMPLETE => {
            if !published_relaunch_matches(owner, compatible_session_id, &request) {
                return RelaunchPoll::Ready(Err(RelaunchError::Busy));
            }
            // SAFETY: COMPLETE is immutable to the monitor and local syscall
            // handling is serialized by the IRQ mask.
            let workspace = unsafe { &mut *RELAUNCH_SERVICE.value.get() };
            let outcome = workspace
                .outcome
                .take()
                .unwrap_or_else(|| panic!("package relaunch completed without an outcome"));
            workspace.compatible_session_id = 0;
            workspace.request = None;
            RELAUNCH_SERVICE.owner.store(0, Ordering::Relaxed);
            RELAUNCH_SERVICE
                .state
                .store(RELAUNCH_EMPTY, Ordering::Release);
            RelaunchPoll::Ready(outcome)
        }
        _ => panic!("Android package relaunch queue entered an invalid state"),
    }
}

/// Publishes or exact-retries one ABI 52 generation-bound removal request.
///
/// Submission is IRQ-masked and performs no block I/O. Only the mobile
/// monitor can execute the durable tombstone transaction.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub fn poll_runtime_uninstall(
    owner: u64,
    request: AndroidPackageUninstallRequest,
) -> RuntimeUninstallPoll {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall polled with IRQ enabled");
    }
    if owner == 0 || !crate::process::android_package_uninstall_owner_can_retry_irq_masked(owner) {
        return RuntimeUninstallPoll::Ready(Err(RuntimeUninstallError::Stale));
    }

    match RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) {
        UNINSTALL_EMPTY => {
            if next_runtime_uninstall_sequence(owner, request.request_sequence()).is_err() {
                return RuntimeUninstallPoll::Ready(Err(RuntimeUninstallError::Sequence));
            }
            if RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_EMPTY
                || {
                    #[cfg(feature = "androidbox-runtime-install2")]
                    {
                        RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) != INSTALL_EMPTY
                    }
                    #[cfg(not(feature = "androidbox-runtime-install2"))]
                    {
                        false
                    }
                }
                || !runtime_uninstall_grants_quiescent_irq_masked()
            {
                return RuntimeUninstallPoll::Ready(Err(RuntimeUninstallError::Busy));
            }
            if let Err(error) = runtime_uninstall_identity(&request) {
                return RuntimeUninstallPoll::Ready(Err(error));
            }
            enqueue_runtime_uninstall(owner, request);
            RuntimeUninstallPoll::Pending
        }
        UNINSTALL_PENDING | UNINSTALL_RUNNING => {
            if published_runtime_uninstall_matches(owner, &request) {
                RuntimeUninstallPoll::Pending
            } else {
                RuntimeUninstallPoll::Ready(Err(RuntimeUninstallError::Busy))
            }
        }
        UNINSTALL_COMPLETE => {
            if !published_runtime_uninstall_matches(owner, &request) {
                return RuntimeUninstallPoll::Ready(Err(RuntimeUninstallError::Busy));
            }
            // SAFETY: COMPLETE is immutable to the monitor and the IRQ mask
            // excludes every other syscall-side consumer.
            let workspace = unsafe { &mut *RUNTIME_UNINSTALL_SERVICE.value.get() };
            let outcome = workspace
                .outcome
                .take()
                .unwrap_or_else(|| panic!("package uninstall completed without an outcome"));
            workspace.request = None;
            RUNTIME_UNINSTALL_SERVICE.owner.store(0, Ordering::Relaxed);
            RUNTIME_UNINSTALL_SERVICE
                .state
                .store(UNINSTALL_EMPTY, Ordering::Release);
            RuntimeUninstallPoll::Ready(outcome)
        }
        _ => panic!("Android package uninstall queue entered an invalid state"),
    }
}

/// Publishes or exact-retries one ABI 53 install/update request.
///
/// Submission is IRQ-masked and performs no block I/O. The mobile monitor
/// re-admits the immutable boot source and owns the durable transaction.
#[cfg(feature = "androidbox-runtime-install2")]
pub fn poll_runtime_install(
    owner: u64,
    request: AndroidPackageInstallRequest,
) -> RuntimeInstallPoll {
    if !aarch64::irq_is_masked() {
        panic!("Android package install polled with IRQ enabled");
    }
    if owner == 0 || !crate::process::android_package_uninstall_owner_can_retry_irq_masked(owner) {
        return RuntimeInstallPoll::Ready(Err(RuntimeInstallError::Stale));
    }

    match RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) {
        INSTALL_EMPTY => {
            if next_runtime_install_sequence(owner, request.request_sequence()).is_err() {
                return RuntimeInstallPoll::Ready(Err(RuntimeInstallError::Sequence));
            }
            if RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_EMPTY
                || RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) != UNINSTALL_EMPTY
                || !runtime_uninstall_grants_quiescent_irq_masked()
            {
                return RuntimeInstallPoll::Ready(Err(RuntimeInstallError::Busy));
            }
            if let Err(error) = runtime_install_identity(&request) {
                return RuntimeInstallPoll::Ready(Err(error));
            }
            enqueue_runtime_install(owner, request);
            RuntimeInstallPoll::Pending
        }
        INSTALL_PENDING | INSTALL_RUNNING => {
            if published_runtime_install_matches(owner, &request) {
                RuntimeInstallPoll::Pending
            } else {
                RuntimeInstallPoll::Ready(Err(RuntimeInstallError::Busy))
            }
        }
        INSTALL_COMPLETE => {
            if !published_runtime_install_matches(owner, &request) {
                return RuntimeInstallPoll::Ready(Err(RuntimeInstallError::Busy));
            }
            // SAFETY: COMPLETE is immutable to the monitor and the IRQ mask
            // excludes every other syscall-side consumer.
            let workspace = unsafe { &mut *RUNTIME_INSTALL_SERVICE.value.get() };
            let outcome = workspace
                .outcome
                .take()
                .unwrap_or_else(|| panic!("package install completed without an outcome"));
            workspace.request = None;
            RUNTIME_INSTALL_SERVICE.owner.store(0, Ordering::Relaxed);
            RUNTIME_INSTALL_SERVICE
                .state
                .store(INSTALL_EMPTY, Ordering::Release);
            RuntimeInstallPoll::Ready(outcome)
        }
        _ => panic!("Android package install queue entered an invalid state"),
    }
}

/// Replaces the bounded one-shot image grant after syscall 60 has collected a
/// successful fresh relaunch and validated its canonical response.
#[cfg(feature = "androidbox-el0-runtime0")]
pub fn publish_image_grant(
    launcher_owner: u64,
    execution_owner: u64,
    claim: AndroidPackageImageClaim,
    image: Vmo,
) -> Result<(), AndroidPackageImageGrantError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package image grant published with IRQ enabled");
    }
    if launcher_owner == 0
        || execution_owner == 0
        || !crate::process::android_package_relaunch_owner_can_retry_irq_masked(launcher_owner)
        || crate::process::android_package_execution_process_id_irq_masked()
            != Some(execution_owner)
        || image.len() != claim.apk_length() as usize
    {
        return Err(AndroidPackageImageGrantError::StaleOwner);
    }
    #[cfg(feature = "androidbox-restart0")]
    {
        // SAFETY: publication shares the IRQ-masked grant domain.
        let restart = unsafe { &*ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
        if matches!(
            restart.state,
            RESTART_IMAGE_GRANT_ESCROWED | RESTART_IMAGE_GRANT_REISSUED
        ) {
            // A fresh Launcher publication must not replace either the held
            // old-worker escrow or the grant awaiting replacement syscall 61.
            return Err(AndroidPackageImageGrantError::Busy);
        }
    }
    // SAFETY: every caller is serialized in the IRQ-masked syscall domain.
    // Replacing an unconsumed prior grant is bounded revocation, not a second
    // publication: the old VMO is dropped without ever entering a handle table.
    unsafe {
        *ANDROID_PACKAGE_IMAGE_GRANT.value.get() = Some(AndroidPackageImageGrant {
            launcher_owner,
            execution_owner,
            claim,
            image,
        });
    }
    Ok(())
}

/// Consumes the exact current package executor's matching image grant once.
///
/// The syscall caller must reserve handle-table capacity before invoking this
/// function. A mismatched claim is preserved so malformed or replayed input
/// cannot revoke the valid grant.
#[cfg(feature = "androidbox-el0-runtime0")]
pub fn take_image_grant(
    execution_owner: u64,
    claim: AndroidPackageImageClaim,
) -> Result<Vmo, AndroidPackageImageGrantError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package image grant consumed with IRQ enabled");
    }
    // SAFETY: the IRQ mask serializes publication and consumption.
    let slot = unsafe { &mut *ANDROID_PACKAGE_IMAGE_GRANT.value.get() };
    let Some(grant) = slot.as_ref() else {
        return Err(AndroidPackageImageGrantError::Missing);
    };
    if grant.execution_owner != execution_owner
        || crate::process::android_package_execution_process_id_irq_masked()
            != Some(execution_owner)
        || !crate::process::android_package_relaunch_owner_can_retry_irq_masked(
            grant.launcher_owner,
        )
    {
        *slot = None;
        return Err(AndroidPackageImageGrantError::StaleOwner);
    }
    if grant.claim != claim || grant.image.len() != claim.apk_length() as usize {
        return Err(AndroidPackageImageGrantError::Mismatch);
    }
    let grant = slot
        .take()
        .unwrap_or_else(|| panic!("validated Android package image grant disappeared"));
    #[cfg(feature = "androidbox-restart0")]
    capture_or_complete_restart_image_grant(&grant);
    Ok(grant.image)
}

#[cfg(feature = "androidbox-restart0")]
fn capture_or_complete_restart_image_grant(grant: &AndroidPackageImageGrant) {
    if !aarch64::irq_is_masked() {
        panic!("Android package restart image grant captured with IRQ enabled");
    }
    // SAFETY: syscall 61 runs in the IRQ-masked grant domain.
    let restart = unsafe { &mut *ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
    match restart.state {
        RESTART_IMAGE_GRANT_EMPTY => {
            restart.state = RESTART_IMAGE_GRANT_ESCROWED;
            restart.launcher_owner = grant.launcher_owner;
            restart.old_execution_owner = grant.execution_owner;
            restart.new_execution_owner = 0;
            restart.claim = Some(grant.claim);
            restart.image = Some(grant.image.clone());
            restart.escrows = restart
                .escrows
                .checked_add(1)
                .unwrap_or_else(|| panic!("Android package restart escrow counter overflowed"));
        }
        RESTART_IMAGE_GRANT_REISSUED => {
            if restart.launcher_owner != grant.launcher_owner
                || restart.new_execution_owner != grant.execution_owner
                || restart.claim != Some(grant.claim)
                || restart
                    .image
                    .as_ref()
                    .is_none_or(|proof| !proof.same_vmo(&grant.image))
                || restart.escrows != 1
                || restart.reissues != 1
                || restart.replacement_claims != 0
            {
                panic!("replacement AndroidApp consumed a mismatched restart image grant");
            }
            restart.image = None;
            restart.state = RESTART_IMAGE_GRANT_CONSUMED;
            restart.replacement_claims = 1;
        }
        RESTART_IMAGE_GRANT_ESCROWED => {
            panic!("initial AndroidApp consumed a second image grant before restart");
        }
        RESTART_IMAGE_GRANT_CONSUMED => {
            // The one-shot recovery budget is exhausted. Later ordinary
            // package launches retain their normal one-shot semantics without
            // recreating a restart escrow.
        }
        _ => panic!("Android package restart image grant entered an invalid state"),
    }
}

#[cfg(feature = "androidbox-restart0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageRestartImageGrantError {
    Missing,
    Mismatch,
    StaleOwner,
    AlreadyIssued,
}

/// Moves the exact initial worker's immutable package VMO to one authenticated
/// replacement generation. No bytes are copied, reread, or exposed to Init,
/// App, or Launcher. One same-identity proof reference is retained only until
/// the replacement consumes syscall 61, then dropped.
#[cfg(feature = "androidbox-restart0")]
pub fn reissue_image_grant_for_restart(
    old_execution_owner: u64,
    new_execution_owner: u64,
    compatible_session_id: u64,
    package_generation: u64,
) -> Result<(), AndroidPackageRestartImageGrantError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package restart image grant reissued with IRQ enabled");
    }
    let old_slot = old_execution_owner as u32;
    let new_slot = new_execution_owner as u32;
    let old_generation = (old_execution_owner >> 32) as u32;
    let new_generation = (new_execution_owner >> 32) as u32;
    if old_execution_owner == 0
        || new_execution_owner == 0
        || old_execution_owner == new_execution_owner
        || old_slot != new_slot
        || old_generation.checked_add(1) != Some(new_generation)
        || crate::process::android_package_execution_process_id_irq_masked()
            != Some(new_execution_owner)
    {
        return Err(AndroidPackageRestartImageGrantError::StaleOwner);
    }

    // SAFETY: the authenticated replacement-Open ChannelRead and both grant
    // syscalls share this IRQ-masked serialization domain.
    let grant_slot = unsafe { &mut *ANDROID_PACKAGE_IMAGE_GRANT.value.get() };
    if grant_slot.is_some() {
        return Err(AndroidPackageRestartImageGrantError::AlreadyIssued);
    }
    let restart = unsafe { &mut *ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
    if restart.state != RESTART_IMAGE_GRANT_ESCROWED {
        return Err(AndroidPackageRestartImageGrantError::Missing);
    }
    let Some(claim) = restart.claim else {
        return Err(AndroidPackageRestartImageGrantError::Missing);
    };
    if restart.old_execution_owner != old_execution_owner
        || restart.new_execution_owner != 0
        || restart.escrows != 1
        || restart.reissues != 0
        || restart.replacement_claims != 0
        || claim.compatible_session_id() != compatible_session_id
        || claim.package_generation() != package_generation
    {
        return Err(AndroidPackageRestartImageGrantError::Mismatch);
    }
    if !crate::process::android_package_relaunch_owner_can_retry_irq_masked(restart.launcher_owner)
    {
        return Err(AndroidPackageRestartImageGrantError::StaleOwner);
    }
    let Some(package) = catalog_snapshot().and_then(|catalog| catalog.package) else {
        return Err(AndroidPackageRestartImageGrantError::Mismatch);
    };
    if package.generation != package_generation
        || package.apk_length != claim.apk_length()
        || package.apk_sha256 != *claim.apk_sha256()
        || package.signer_cert_sha256 != *claim.signer_sha256()
    {
        return Err(AndroidPackageRestartImageGrantError::Mismatch);
    }
    let Some(image) = restart.image.take() else {
        return Err(AndroidPackageRestartImageGrantError::Missing);
    };
    if image.len() != claim.apk_length() as usize {
        panic!("Android package restart escrow changed image length");
    }
    *grant_slot = Some(AndroidPackageImageGrant {
        launcher_owner: restart.launcher_owner,
        execution_owner: new_execution_owner,
        claim,
        image,
    });
    restart.state = RESTART_IMAGE_GRANT_REISSUED;
    restart.new_execution_owner = new_execution_owner;
    restart.image = grant_slot.as_ref().map(|grant| grant.image.clone());
    restart.reissues = 1;
    Ok(())
}

/// Drops a held restart escrow, or a reissued-but-unclaimed replacement grant,
/// when its exact worker exits outside the single authenticated restart path.
#[cfg(feature = "androidbox-restart0")]
pub fn revoke_restart_image_grant_for_execution_owner(execution_owner: u64) -> bool {
    if !aarch64::irq_is_masked() || execution_owner == 0 {
        panic!("Android package restart image grant revoked outside its serialized domain");
    }
    // SAFETY: the process reaper shares the IRQ-masked grant domain with both
    // the ordinary grant slot and the restart proof. In the REISSUED case the
    // ordinary slot is normally removed immediately before this function by
    // `revoke_image_grant_for_execution_owner`; clearing a matching value here
    // as well keeps this operation complete if that call ordering changes.
    let grant_slot = unsafe { &mut *ANDROID_PACKAGE_IMAGE_GRANT.value.get() };
    let restart = unsafe { &mut *ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
    if restart.state == RESTART_IMAGE_GRANT_ESCROWED
        && restart.old_execution_owner == execution_owner
    {
        *restart = AndroidPackageRestartImageGrant::new();
        true
    } else if restart.state == RESTART_IMAGE_GRANT_REISSUED
        && restart.new_execution_owner == execution_owner
        && restart.replacement_claims == 0
    {
        if grant_slot
            .as_ref()
            .is_some_and(|grant| grant.execution_owner == execution_owner)
        {
            *grant_slot = None;
        }
        // Drop the same-VMO proof clone and every owner/claim field together.
        // EMPTY is intentionally incomplete, so the monitor cannot publish a
        // restart success marker and a later ordinary grant is not held Busy.
        *restart = AndroidPackageRestartImageGrant::new();
        true
    } else {
        false
    }
}

#[cfg(feature = "androidbox-restart0")]
pub fn restart_image_grant_snapshot() -> AndroidPackageRestartImageGrantSnapshot {
    let saved_daif = aarch64::save_and_mask_irq();
    // SAFETY: the IRQ mask excludes all grant mutation.
    let restart = unsafe { &*ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
    let claim = restart.claim;
    let snapshot = AndroidPackageRestartImageGrantSnapshot {
        state: restart.state,
        launcher_owner: restart.launcher_owner,
        old_execution_owner: restart.old_execution_owner,
        new_execution_owner: restart.new_execution_owner,
        compatible_session_id: claim.map_or(0, |claim| claim.compatible_session_id()),
        package_generation: claim.map_or(0, |claim| claim.package_generation()),
        escrows: restart.escrows,
        reissues: restart.reissues,
        replacement_claims: restart.replacement_claims,
        image_held: restart.image.is_some(),
        ordinary_grant_pending: unsafe { &*ANDROID_PACKAGE_IMAGE_GRANT.value.get() }.is_some(),
    };
    aarch64::restore_daif(saved_daif);
    snapshot
}

/// Revokes an unconsumed ABI 47 grant when its exact AndroidApp generation is
/// reaped. A consumed grant is already owned by the dying process handle table.
#[cfg(feature = "androidbox-process0")]
pub fn revoke_image_grant_for_execution_owner(execution_owner: u64) -> bool {
    if !aarch64::irq_is_masked() || execution_owner == 0 {
        panic!("Android package image grant revoked outside its serialized domain");
    }
    // SAFETY: the process reaper and both grant syscalls run with IRQ masked.
    let slot = unsafe { &mut *ANDROID_PACKAGE_IMAGE_GRANT.value.get() };
    if slot
        .as_ref()
        .is_some_and(|grant| grant.execution_owner == execution_owner)
    {
        *slot = None;
        true
    } else {
        false
    }
}

/// Executes at most one package operation from the IRQ-enabled monitor.
pub fn service_pending() {
    service_pending_relaunch();
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    service_pending_runtime_uninstall();
    #[cfg(feature = "androidbox-runtime-install2")]
    service_pending_runtime_install();
}

fn service_pending_relaunch() {
    if aarch64::irq_is_masked() {
        panic!("Android package relaunch service attempted I/O with IRQ masked");
    }

    let saved_daif = aarch64::save_and_mask_irq();
    match RELAUNCH_SERVICE.state.load(Ordering::Acquire) {
        RELAUNCH_PENDING if !published_relaunch_owner_can_retry() => {
            discard_published_relaunch();
            aarch64::restore_daif(saved_daif);
            return;
        }
        RELAUNCH_COMPLETE if !published_relaunch_owner_can_retry() => {
            discard_published_relaunch();
            aarch64::restore_daif(saved_daif);
            return;
        }
        RELAUNCH_PENDING => {}
        RELAUNCH_EMPTY | RELAUNCH_RUNNING | RELAUNCH_COMPLETE => {
            aarch64::restore_daif(saved_daif);
            return;
        }
        _ => panic!("Android package relaunch queue entered an invalid state"),
    }

    let owner = RELAUNCH_SERVICE.owner.load(Ordering::Relaxed);
    // SAFETY: PENDING is immutable after Release publication. The local IRQ
    // mask excludes its sole syscall consumer while the monitor copies it.
    let (compatible_session_id, request) = unsafe {
        let workspace = &*RELAUNCH_SERVICE.value.get();
        (
            workspace.compatible_session_id,
            workspace
                .request
                .unwrap_or_else(|| panic!("pending package relaunch has no request")),
        )
    };
    if RELAUNCH_SERVICE
        .state
        .compare_exchange(
            RELAUNCH_PENDING,
            RELAUNCH_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        aarch64::restore_daif(saved_daif);
        return;
    }
    aarch64::restore_daif(saved_daif);

    let (result, io) = execute_durable_relaunch(&request);
    match &result {
        Ok(relaunch) => {
            let package = durable_relaunch_package(relaunch);
            crate::kprintln!(
                "ANDROID_PACKAGE_DURABLE_LAUNCH_OK owner={} request_sequence={} generation={} version_code={} apk_length={} profile=Resources-1 reads={} writes=0 flushes=0 package={} activity={} constructor_instructions={} on_create_instructions={}",
                owner,
                request.request_sequence(),
                package.generation,
                package.version_code,
                package.apk_length,
                io.reads,
                package.package.as_str(),
                package.activity.as_str(),
                package.launch_constructor_instruction_count,
                package.launch_instruction_count,
            );
        }
        Err(error) => {
            crate::kprintln!(
                "ANDROID_PACKAGE_DURABLE_LAUNCH_FAIL owner={} request_sequence={} reason={} reads={} writes=0 flushes=0",
                owner,
                request.request_sequence(),
                error.as_str(),
                io.reads,
            );
        }
    }
    #[cfg(feature = "androidbox-multipackage4")]
    let selected_package = result.as_ref().ok().map(durable_relaunch_package);
    let outcome = result.map(|relaunch| {
        let package = durable_relaunch_package(&relaunch);
        RelaunchCompletion {
            request_sequence: request.request_sequence(),
            package,
            io,
            #[cfg(feature = "androidbox-el0-runtime0")]
            image: relaunch.image,
        }
    });

    let saved_daif = aarch64::save_and_mask_irq();
    if RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_RUNNING
        || RELAUNCH_SERVICE.owner.load(Ordering::Relaxed) != owner
        || !published_relaunch_matches(owner, compatible_session_id, &request)
    {
        panic!("running package relaunch lost its exact request identity");
    }
    #[cfg(feature = "androidbox-multipackage4")]
    if let Some(package) = selected_package {
        // ABI 55 retains syscall 59 as the compatibility worker's view of the
        // package selected by Launcher. A successful exact durable relaunch
        // changes only that boot-local selection; the complete directory,
        // its revision, and persistent storage remain untouched.
        let live = unsafe { &mut *LIVE_PACKAGE_CATALOG.value.get() };
        let index = live
            .multi_packages
            .iter()
            .position(|entry| *entry == Some(package))
            .unwrap_or_else(|| panic!("durable relaunch lost its directory package"));
        let installed = live.multi_installed[index]
            .unwrap_or_else(|| panic!("durable relaunch lost its directory handle"))
            .package();
        if !installed_matches_snapshot(&installed, &package) {
            panic!("durable relaunch selected an inconsistent compatibility package");
        }
        live.installed = Some(installed);
        live.package = Some(package);
        live.removed = None;
    }
    // SAFETY: RUNNING grants the monitor sole mutable access. COMPLETE is
    // Release-published only after the terminal value is fully written.
    unsafe {
        (*RELAUNCH_SERVICE.value.get()).outcome = Some(outcome);
    }
    RELAUNCH_SERVICE
        .state
        .store(RELAUNCH_COMPLETE, Ordering::Release);
    aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn service_pending_runtime_uninstall() {
    if aarch64::irq_is_masked() {
        panic!("Android package uninstall service attempted I/O with IRQ masked");
    }

    let saved_daif = aarch64::save_and_mask_irq();
    match RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) {
        UNINSTALL_PENDING if !published_runtime_uninstall_owner_can_retry() => {
            discard_published_runtime_uninstall();
            aarch64::restore_daif(saved_daif);
            return;
        }
        UNINSTALL_COMPLETE if !published_runtime_uninstall_owner_can_retry() => {
            discard_published_runtime_uninstall();
            aarch64::restore_daif(saved_daif);
            return;
        }
        UNINSTALL_PENDING => {}
        UNINSTALL_EMPTY | UNINSTALL_RUNNING | UNINSTALL_COMPLETE => {
            aarch64::restore_daif(saved_daif);
            return;
        }
        _ => panic!("Android package uninstall queue entered an invalid state"),
    }
    if RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_EMPTY
        || {
            #[cfg(feature = "androidbox-runtime-install2")]
            {
                RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) != INSTALL_EMPTY
            }
            #[cfg(not(feature = "androidbox-runtime-install2"))]
            {
                false
            }
        }
        || !runtime_uninstall_grants_quiescent_irq_masked()
    {
        panic!("published Android package uninstall lost its quiescent authority boundary");
    }
    let owner = RUNTIME_UNINSTALL_SERVICE.owner.load(Ordering::Relaxed);
    // SAFETY: PENDING is immutable after Release publication and the IRQ mask
    // excludes its syscall consumer while the monitor copies it.
    let request = unsafe {
        (*RUNTIME_UNINSTALL_SERVICE.value.get())
            .request
            .unwrap_or_else(|| panic!("pending package uninstall has no request"))
    };
    if RUNTIME_UNINSTALL_SERVICE
        .state
        .compare_exchange(
            UNINSTALL_PENDING,
            UNINSTALL_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        aarch64::restore_daif(saved_daif);
        return;
    }
    aarch64::restore_daif(saved_daif);

    let (result, io) = execute_runtime_uninstall(&request);
    match &result {
        Ok(removed) => {
            crate::kprintln!(
                "ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_OK owner={} request_sequence={} operation_id={} last_generation={} removal_generation={} package={} reads={} writes={} flushes={} logical_apk_reachable=0 apk_blob_erased=0 data_disposition=no-managed-package-data",
                owner,
                request.request_sequence(),
                request.operation_id(),
                removed.last_generation,
                removed.generation,
                removed.package.as_str(),
                io.reads,
                io.writes,
                io.flushes,
            );
        }
        Err(error) => {
            if *error == RuntimeUninstallError::RequiresReset {
                // An I/O or outcome-unknown failure invalidates all live
                // package publication until reboot recovery proves the disk.
                PUBLICATION_STATE.store(STATE_FAILED, Ordering::Release);
            }
            crate::kprintln!(
                "ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_FAIL owner={} request_sequence={} operation_id={} reason={} reads={} writes={} flushes={}",
                owner,
                request.request_sequence(),
                request.operation_id(),
                error.as_str(),
                io.reads,
                io.writes,
                io.flushes,
            );
        }
    }

    let saved_daif = aarch64::save_and_mask_irq();
    if RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) != UNINSTALL_RUNNING
        || RUNTIME_UNINSTALL_SERVICE.owner.load(Ordering::Relaxed) != owner
        || !published_runtime_uninstall_matches(owner, &request)
    {
        panic!("running package uninstall lost its exact request identity");
    }
    let outcome = match result {
        Ok(removed) => {
            let live = unsafe { &mut *LIVE_PACKAGE_CATALOG.value.get() };
            #[cfg(not(feature = "androidbox-multipackage4"))]
            {
                if live.installed.is_none()
                    || live.package.is_none()
                    || !runtime_uninstall_request_matches_snapshot(
                        &request,
                        &live
                            .package
                            .unwrap_or_else(|| panic!("validated live package vanished")),
                    )
                    || !runtime_uninstall_request_matches_installed(
                        &request,
                        &live
                            .installed
                            .unwrap_or_else(|| panic!("validated live handle vanished")),
                    )
                {
                    panic!("durable package uninstall raced a live catalog mutation");
                }
                *live = LivePackageCatalog {
                    formatted: true,
                    installed: None,
                    package: None,
                    removed: Some(removed),
                    ..LivePackageCatalog::new()
                };
            }
            #[cfg(feature = "androidbox-multipackage4")]
            {
                let Some(index) = live.multi_packages.iter().position(|package| {
                    package.is_some_and(|package| {
                        runtime_uninstall_request_matches_snapshot(&request, &package)
                    })
                }) else {
                    panic!("durable package uninstall lost its directory entry");
                };
                let installed = live.multi_installed[index]
                    .unwrap_or_else(|| panic!("validated directory handle vanished"));
                if !runtime_uninstall_request_matches_installed(&request, &installed.package()) {
                    panic!("durable package uninstall raced a directory mutation");
                }
                live.multi_installed[index] = None;
                live.multi_packages[index] = None;
                let replacement_index = live.multi_packages.iter().position(Option::is_some);
                live.installed = replacement_index
                    .and_then(|index| live.multi_installed[index].map(|value| value.package()));
                live.package = replacement_index.and_then(|index| live.multi_packages[index]);
                live.removed = if replacement_index.is_none() {
                    Some(removed)
                } else {
                    None
                };
                live.formatted = true;
                live.revision = live
                    .revision
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("package directory revision exhausted"));
            }
            Ok(RuntimeUninstallCompletion {
                request_sequence: request.request_sequence(),
                operation_id: request.operation_id(),
                last_generation: removed.last_generation,
                removal_generation: removed.generation,
                io,
            })
        }
        Err(error) => Err(error),
    };
    // SAFETY: RUNNING grants the monitor sole mutable access. COMPLETE is
    // published only after the durable outcome and live catalog are final.
    unsafe {
        (*RUNTIME_UNINSTALL_SERVICE.value.get()).outcome = Some(outcome);
    }
    RUNTIME_UNINSTALL_SERVICE
        .state
        .store(UNINSTALL_COMPLETE, Ordering::Release);
    aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn next_runtime_uninstall_sequence(owner: u64, sequence: u64) -> Result<(), RuntimeUninstallError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall sequence checked with IRQ enabled");
    }
    if owner == 0 || sequence == 0 {
        return Err(RuntimeUninstallError::Sequence);
    }
    let ledger_owner = RUNTIME_UNINSTALL_LEDGER_OWNER.load(Ordering::Relaxed);
    let last_sequence = RUNTIME_UNINSTALL_LEDGER_SEQUENCE.load(Ordering::Relaxed);
    if ledger_owner != owner {
        return if sequence == 1 {
            Ok(())
        } else {
            Err(RuntimeUninstallError::Sequence)
        };
    }
    if last_sequence.checked_add(1) == Some(sequence) {
        Ok(())
    } else {
        Err(RuntimeUninstallError::Sequence)
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn enqueue_runtime_uninstall(owner: u64, request: AndroidPackageUninstallRequest) {
    if !aarch64::irq_is_masked()
        || owner == 0
        || RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) != UNINSTALL_EMPTY
    {
        panic!("invalid Android package uninstall publication");
    }
    // SAFETY: EMPTY plus the IRQ-masked syscall domain grants the sole write.
    let workspace = unsafe { &mut *RUNTIME_UNINSTALL_SERVICE.value.get() };
    workspace.request = Some(request);
    workspace.outcome = None;
    RUNTIME_UNINSTALL_SERVICE
        .owner
        .store(owner, Ordering::Relaxed);
    RUNTIME_UNINSTALL_LEDGER_OWNER.store(owner, Ordering::Relaxed);
    RUNTIME_UNINSTALL_LEDGER_SEQUENCE.store(request.request_sequence(), Ordering::Relaxed);
    RUNTIME_UNINSTALL_SERVICE
        .state
        .store(UNINSTALL_PENDING, Ordering::Release);
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn published_runtime_uninstall_matches(
    owner: u64,
    request: &AndroidPackageUninstallRequest,
) -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall identity compared with IRQ enabled");
    }
    let state = RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire);
    if !matches!(
        state,
        UNINSTALL_PENDING | UNINSTALL_RUNNING | UNINSTALL_COMPLETE
    ) || RUNTIME_UNINSTALL_SERVICE.owner.load(Ordering::Relaxed) != owner
    {
        return false;
    }
    // SAFETY: the request is immutable in every published state.
    unsafe { (*RUNTIME_UNINSTALL_SERVICE.value.get()).request == Some(*request) }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn published_runtime_uninstall_owner_can_retry() -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall liveness checked with IRQ enabled");
    }
    let owner = RUNTIME_UNINSTALL_SERVICE.owner.load(Ordering::Relaxed);
    owner != 0 && crate::process::android_package_uninstall_owner_can_retry_irq_masked(owner)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn discard_published_runtime_uninstall() {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall discarded with IRQ enabled");
    }
    let workspace = unsafe { &mut *RUNTIME_UNINSTALL_SERVICE.value.get() };
    workspace.request = None;
    workspace.outcome = None;
    RUNTIME_UNINSTALL_SERVICE.owner.store(0, Ordering::Relaxed);
    RUNTIME_UNINSTALL_SERVICE
        .state
        .store(UNINSTALL_EMPTY, Ordering::Release);
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn runtime_uninstall_grants_quiescent_irq_masked() -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package uninstall grant state checked with IRQ enabled");
    }
    let ordinary_empty = unsafe { &*ANDROID_PACKAGE_IMAGE_GRANT.value.get() }.is_none();
    let restart = unsafe { &*ANDROID_PACKAGE_RESTART_IMAGE_GRANT.value.get() };
    ordinary_empty
        && !matches!(
            restart.state,
            RESTART_IMAGE_GRANT_ESCROWED | RESTART_IMAGE_GRANT_REISSUED
        )
        && restart.image.is_none()
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[cfg(not(feature = "androidbox-multipackage4"))]
fn runtime_uninstall_identity(
    request: &AndroidPackageUninstallRequest,
) -> Result<(InstalledPackageSnapshot, LiveInstalledHandle), RuntimeUninstallError> {
    if request.profile() != AndroidPackageCompatibilityProfile::Resources1 {
        return Err(RuntimeUninstallError::Unsupported);
    }
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let catalog = catalog_snapshot().ok_or(RuntimeUninstallError::RequiresReset)?;
    let package = catalog.package.ok_or(RuntimeUninstallError::NotInstalled)?;
    if !runtime_uninstall_request_matches_snapshot(request, &package) {
        return Err(RuntimeUninstallError::Stale);
    }
    let saved_daif = aarch64::save_and_mask_irq();
    let installed = unsafe { (*LIVE_PACKAGE_CATALOG.value.get()).installed };
    aarch64::restore_daif(saved_daif);
    let installed = installed.ok_or(RuntimeUninstallError::NotInstalled)?;
    if !runtime_uninstall_request_matches_installed(request, &installed)
        || !installed_matches_snapshot(&installed, &package)
    {
        return Err(RuntimeUninstallError::Stale);
    }
    Ok((package, installed))
}

#[cfg(feature = "androidbox-multipackage4")]
fn runtime_uninstall_identity(
    request: &AndroidPackageUninstallRequest,
) -> Result<(InstalledPackageSnapshot, LiveInstalledHandle), RuntimeUninstallError> {
    if request.profile() != AndroidPackageCompatibilityProfile::Resources1 {
        return Err(RuntimeUninstallError::Unsupported);
    }
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let saved_daif = aarch64::save_and_mask_irq();
    let live = unsafe { *LIVE_PACKAGE_CATALOG.value.get() };
    let match_index = live.multi_packages.iter().position(|package| {
        package.is_some_and(|package| runtime_uninstall_request_matches_snapshot(request, &package))
    });
    let result = match_index
        .and_then(|index| Some((live.multi_packages[index]?, live.multi_installed[index]?)));
    aarch64::restore_daif(saved_daif);
    let (package, installed) = result.ok_or(RuntimeUninstallError::NotInstalled)?;
    let inner = installed.package();
    if !runtime_uninstall_request_matches_installed(request, &inner)
        || !installed_matches_snapshot(&inner, &package)
    {
        return Err(RuntimeUninstallError::Stale);
    }
    Ok((package, installed))
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn runtime_uninstall_request_matches_snapshot(
    request: &AndroidPackageUninstallRequest,
    package: &InstalledPackageSnapshot,
) -> bool {
    request.expected_generation() == package.generation
        && request.expected_version_code() == package.version_code
        && request.expected_apk_length() == package.apk_length
        && request.expected_apk_sha256() == &package.apk_sha256
        && request.expected_signer_sha256() == &package.signer_cert_sha256
        && request.package_name_bytes() == package.package.as_bytes()
        && package.compatibility_profile == CompatibilityProfile::Resources1
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn runtime_uninstall_request_matches_installed(
    request: &AndroidPackageUninstallRequest,
    installed: &InstalledPackage,
) -> bool {
    let metadata = installed.metadata();
    request.expected_generation() == installed.generation()
        && request.expected_version_code() == metadata.version_code()
        && request.expected_apk_length() as usize == installed.apk_length()
        && request.expected_apk_sha256() == metadata.apk_sha256()
        && request.expected_signer_sha256() == metadata.signer_cert_sha256()
        && request.package_name() == metadata.package()
        && metadata.compatibility_profile() == CompatibilityProfile::Resources1
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn execute_runtime_uninstall(
    request: &AndroidPackageUninstallRequest,
) -> (
    Result<RemovedPackageSnapshot, RuntimeUninstallError>,
    PackageIoEvidence,
) {
    let mut evidence = PackageIoEvidence::default();
    let result = execute_runtime_uninstall_once(request, &mut evidence);
    (result, evidence)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[cfg(not(feature = "androidbox-multipackage4"))]
fn execute_runtime_uninstall_once(
    request: &AndroidPackageUninstallRequest,
    evidence: &mut PackageIoEvidence,
) -> Result<RemovedPackageSnapshot, RuntimeUninstallError> {
    let (live_package, live_installed) = runtime_uninstall_identity(request)?;
    let expected_device_sectors = RELAUNCH_DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = RELAUNCH_DEADLINE_BUDGET.load(Ordering::Acquire);
    if expected_device_sectors == 0 || deadline_budget == 0 {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let (device_sectors, read_only) =
        storage::device_contract().map_err(|_| RuntimeUninstallError::RequiresReset)?;
    if read_only
        || device_sectors != expected_device_sectors
        || device_sectors != PACKAGES_DEVICE_SECTORS
    {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let mut io = PackageIo::new(device_sectors, deadline_budget);
    let recovered = match recover_state(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(PackageState::Installed(installed)) => installed,
        Ok(PackageState::Empty | PackageState::Removed(_)) => {
            *evidence = io.evidence();
            return Err(RuntimeUninstallError::NotInstalled);
        }
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_uninstall_store_error(error));
        }
    };
    if recovered != live_installed
        || !runtime_uninstall_request_matches_installed(request, &recovered)
        || !installed_matches_snapshot(&recovered, &live_package)
    {
        *evidence = io.evidence();
        return Err(RuntimeUninstallError::Stale);
    }
    let removed = match uninstall(
        &mut io,
        PACKAGES_PARTITION_FIRST_LBA,
        request.operation_id(),
        &recovered,
        DataDisposition::NoManagedPackageData,
    ) {
        Ok(removed) => removed,
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_uninstall_store_error(error));
        }
    };
    *evidence = io.evidence();
    if evidence.reads == 0 || evidence.writes == 0 || evidence.flushes == 0 {
        return Err(RuntimeUninstallError::Corrupt);
    }
    if removed.operation_id() != request.operation_id()
        || removed.expected_generation() != request.expected_generation()
        || removed.generation()
            != request
                .expected_generation()
                .checked_add(1)
                .ok_or(RuntimeUninstallError::Corrupt)?
        || removed.version_code() != request.expected_version_code()
        || removed.apk_length() != request.expected_apk_length() as usize
        || removed.package() != request.package_name()
        || removed.apk_sha256() != request.expected_apk_sha256()
        || removed.signer_cert_sha256() != request.expected_signer_sha256()
        || removed.disposition() != DataDisposition::NoManagedPackageData
    {
        return Err(RuntimeUninstallError::Corrupt);
    }
    snapshot_removed_package(&removed).map_err(|_| RuntimeUninstallError::Corrupt)
}

#[cfg(feature = "androidbox-multipackage4")]
fn execute_runtime_uninstall_once(
    request: &AndroidPackageUninstallRequest,
    evidence: &mut PackageIoEvidence,
) -> Result<RemovedPackageSnapshot, RuntimeUninstallError> {
    let (live_package, live_handle) = runtime_uninstall_identity(request)?;
    let live_installed = live_handle.package();
    let expected_device_sectors = RELAUNCH_DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = RELAUNCH_DEADLINE_BUDGET.load(Ordering::Acquire);
    if expected_device_sectors == 0 || deadline_budget == 0 {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let (device_sectors, read_only) =
        storage::device_contract().map_err(|_| RuntimeUninstallError::RequiresReset)?;
    if read_only
        || device_sectors != expected_device_sectors
        || device_sectors != PACKAGES_DEVICE_SECTORS
    {
        return Err(RuntimeUninstallError::RequiresReset);
    }
    let mut io = PackageIo::new(device_sectors, deadline_budget);
    let recovered = match recover_multi(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(catalog) => match catalog.find_installed(request.package_name()) {
            Some(installed) => installed,
            None => {
                *evidence = io.evidence();
                return Err(RuntimeUninstallError::NotInstalled);
            }
        },
        Err(MultiPackageError::Store(error)) => {
            *evidence = io.evidence();
            return Err(map_runtime_uninstall_store_error(error));
        }
        Err(_) => {
            *evidence = io.evidence();
            return Err(RuntimeUninstallError::Corrupt);
        }
    };
    let recovered_inner = recovered.package();
    if recovered != live_handle
        || recovered_inner != live_installed
        || !runtime_uninstall_request_matches_installed(request, &recovered_inner)
        || !installed_matches_snapshot(&recovered_inner, &live_package)
    {
        *evidence = io.evidence();
        return Err(RuntimeUninstallError::Stale);
    }
    let removed = match uninstall_multi(
        &mut io,
        PACKAGES_PARTITION_FIRST_LBA,
        request.operation_id(),
        recovered,
        DataDisposition::NoManagedPackageData,
    ) {
        Ok(removed) => removed,
        Err(MultiPackageError::Store(error)) => {
            *evidence = io.evidence();
            return Err(map_runtime_uninstall_store_error(error));
        }
        Err(MultiPackageError::PackageNotFound) => {
            *evidence = io.evidence();
            return Err(RuntimeUninstallError::Stale);
        }
        Err(_) => {
            *evidence = io.evidence();
            return Err(RuntimeUninstallError::Corrupt);
        }
    };
    *evidence = io.evidence();
    if evidence.reads == 0 || evidence.writes == 0 || evidence.flushes == 0 {
        return Err(RuntimeUninstallError::Corrupt);
    }
    if removed.operation_id() != request.operation_id()
        || removed.expected_generation() != request.expected_generation()
        || removed.generation()
            != request
                .expected_generation()
                .checked_add(1)
                .ok_or(RuntimeUninstallError::Corrupt)?
        || removed.version_code() != request.expected_version_code()
        || removed.apk_length() != request.expected_apk_length() as usize
        || removed.package() != request.package_name()
        || removed.apk_sha256() != request.expected_apk_sha256()
        || removed.signer_cert_sha256() != request.expected_signer_sha256()
        || removed.disposition() != DataDisposition::NoManagedPackageData
    {
        return Err(RuntimeUninstallError::Corrupt);
    }
    snapshot_removed_package(&removed).map_err(|_| RuntimeUninstallError::Corrupt)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn map_runtime_uninstall_store_error(error: PackageStoreError) -> RuntimeUninstallError {
    match error {
        PackageStoreError::Unformatted | PackageStoreError::NotInstalled => {
            RuntimeUninstallError::NotInstalled
        }
        PackageStoreError::StalePackage => RuntimeUninstallError::Stale,
        PackageStoreError::Io(_) | PackageStoreError::OutcomeUnknown => {
            RuntimeUninstallError::RequiresReset
        }
        PackageStoreError::VolumeBounds
        | PackageStoreError::AlreadyFormatted
        | PackageStoreError::NotVirgin
        | PackageStoreError::InvalidMetadata(_)
        | PackageStoreError::EmptyApk
        | PackageStoreError::ApkTooLarge
        | PackageStoreError::ApkDigestMismatch
        | PackageStoreError::PackageChanged
        | PackageStoreError::SignerChanged
        | PackageStoreError::VersionNotIncreasing
        | PackageStoreError::TransactionConflict
        | PackageStoreError::GenerationExhausted
        | PackageStoreError::BufferTooSmall { .. }
        | PackageStoreError::Corrupt(_)
        | PackageStoreError::VerificationFailed => RuntimeUninstallError::Corrupt,
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
struct RuntimeInstallDurable {
    installed: InstalledPackage,
    package: InstalledPackageSnapshot,
    previous_generation: u64,
    #[cfg(feature = "androidbox-multipackage4")]
    volume_index: u8,
}

#[cfg(feature = "androidbox-runtime-install2")]
fn service_pending_runtime_install() {
    if aarch64::irq_is_masked() {
        panic!("Android package install service attempted I/O with IRQ masked");
    }

    let saved_daif = aarch64::save_and_mask_irq();
    match RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) {
        INSTALL_PENDING if !published_runtime_install_owner_can_retry() => {
            discard_published_runtime_install();
            aarch64::restore_daif(saved_daif);
            return;
        }
        INSTALL_COMPLETE if !published_runtime_install_owner_can_retry() => {
            discard_published_runtime_install();
            aarch64::restore_daif(saved_daif);
            return;
        }
        INSTALL_PENDING => {}
        INSTALL_EMPTY | INSTALL_RUNNING | INSTALL_COMPLETE => {
            aarch64::restore_daif(saved_daif);
            return;
        }
        _ => panic!("Android package install queue entered an invalid state"),
    }
    if RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_EMPTY
        || RUNTIME_UNINSTALL_SERVICE.state.load(Ordering::Acquire) != UNINSTALL_EMPTY
        || !runtime_uninstall_grants_quiescent_irq_masked()
    {
        panic!("published Android package install lost its quiescent authority boundary");
    }
    let owner = RUNTIME_INSTALL_SERVICE.owner.load(Ordering::Relaxed);
    // SAFETY: PENDING is immutable after Release publication and the IRQ mask
    // excludes its syscall consumer while the monitor copies it.
    let request = unsafe {
        (*RUNTIME_INSTALL_SERVICE.value.get())
            .request
            .unwrap_or_else(|| panic!("pending package install has no request"))
    };
    if RUNTIME_INSTALL_SERVICE
        .state
        .compare_exchange(
            INSTALL_PENDING,
            INSTALL_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        aarch64::restore_daif(saved_daif);
        return;
    }
    aarch64::restore_daif(saved_daif);

    let (result, io) = execute_runtime_install(&request);
    match &result {
        Ok(durable) => {
            crate::kprintln!(
                "ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK owner={} request_sequence={} operation_id={} candidate_id={} action={} previous_generation={} installed_generation={} version_code={} package={} reads={} writes={} flushes={} source=immutable-fw-cfg network=disabled",
                owner,
                request.request_sequence(),
                request.operation_id(),
                request.candidate_id(),
                runtime_install_action_name(request.action()),
                durable.previous_generation,
                durable.installed.generation(),
                durable.package.version_code,
                durable.package.package.as_str(),
                io.reads,
                io.writes,
                io.flushes,
            );
        }
        Err(error) => {
            if *error == RuntimeInstallError::RequiresReset {
                PUBLICATION_STATE.store(STATE_FAILED, Ordering::Release);
            }
            crate::kprintln!(
                "ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_FAIL owner={} request_sequence={} operation_id={} candidate_id={} reason={} reads={} writes={} flushes={}",
                owner,
                request.request_sequence(),
                request.operation_id(),
                request.candidate_id(),
                error.as_str(),
                io.reads,
                io.writes,
                io.flushes,
            );
        }
    }

    let saved_daif = aarch64::save_and_mask_irq();
    if RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) != INSTALL_RUNNING
        || RUNTIME_INSTALL_SERVICE.owner.load(Ordering::Relaxed) != owner
        || !published_runtime_install_matches(owner, &request)
    {
        panic!("running package install lost its exact request identity");
    }
    let outcome = match result {
        Ok(durable) => {
            let live = unsafe { &mut *LIVE_PACKAGE_CATALOG.value.get() };
            if !runtime_install_live_base_matches(&request, live)
                || unsafe { *INSTALL_CANDIDATE.value.get() }.is_none_or(|candidate| {
                    !runtime_install_request_matches_candidate(&request, &candidate)
                })
            {
                panic!("durable package install raced a live catalog mutation");
            }
            live.formatted = true;
            live.installed = Some(durable.installed);
            live.package = Some(durable.package);
            live.removed = None;
            #[cfg(feature = "androidbox-multipackage4")]
            {
                let index = usize::from(durable.volume_index);
                if index >= MULTI_PACKAGE_CAPACITY {
                    panic!("durable package install returned an invalid volume");
                }
                live.multi_installed[index] = Some(MultiInstalledPackage::from_parts(
                    durable.volume_index,
                    durable.installed,
                ));
                live.multi_packages[index] = Some(durable.package);
                live.revision = live
                    .revision
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("package directory revision exhausted"));
            }
            unsafe {
                *INSTALL_CANDIDATE.value.get() = None;
            }
            Ok(RuntimeInstallCompletion {
                request_sequence: request.request_sequence(),
                operation_id: request.operation_id(),
                candidate_id: request.candidate_id(),
                previous_generation: durable.previous_generation,
                installed_generation: durable.installed.generation(),
                installed_version_code: durable.package.version_code,
                apk_length: durable.package.apk_length,
                action: request
                    .action()
                    .unwrap_or_else(|| panic!("validated install action disappeared")),
                apk_sha256: durable.package.apk_sha256,
                signer_sha256: durable.package.signer_cert_sha256,
                io,
            })
        }
        Err(error) => Err(error),
    };
    // SAFETY: RUNNING grants the monitor sole mutable access.
    unsafe {
        (*RUNTIME_INSTALL_SERVICE.value.get()).outcome = Some(outcome);
    }
    RUNTIME_INSTALL_SERVICE
        .state
        .store(INSTALL_COMPLETE, Ordering::Release);
    aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "androidbox-runtime-install2")]
fn runtime_install_action_name(action: Option<AndroidPackageInstallAction>) -> &'static str {
    match action {
        Some(AndroidPackageInstallAction::Install) => "install",
        Some(AndroidPackageInstallAction::Update) => "update",
        Some(AndroidPackageInstallAction::Reinstall) => "reinstall",
        None => "invalid",
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn next_runtime_install_sequence(owner: u64, sequence: u64) -> Result<(), RuntimeInstallError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package install sequence checked with IRQ enabled");
    }
    if owner == 0 || sequence == 0 {
        return Err(RuntimeInstallError::Sequence);
    }
    let ledger_owner = RUNTIME_INSTALL_LEDGER_OWNER.load(Ordering::Relaxed);
    let last_sequence = RUNTIME_INSTALL_LEDGER_SEQUENCE.load(Ordering::Relaxed);
    if ledger_owner != owner {
        return if sequence == 1 {
            Ok(())
        } else {
            Err(RuntimeInstallError::Sequence)
        };
    }
    if last_sequence.checked_add(1) == Some(sequence) {
        Ok(())
    } else {
        Err(RuntimeInstallError::Sequence)
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn enqueue_runtime_install(owner: u64, request: AndroidPackageInstallRequest) {
    if !aarch64::irq_is_masked()
        || owner == 0
        || RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire) != INSTALL_EMPTY
    {
        panic!("invalid Android package install publication");
    }
    // SAFETY: EMPTY plus the IRQ-masked syscall domain grants the sole write.
    let workspace = unsafe { &mut *RUNTIME_INSTALL_SERVICE.value.get() };
    workspace.request = Some(request);
    workspace.outcome = None;
    RUNTIME_INSTALL_SERVICE
        .owner
        .store(owner, Ordering::Relaxed);
    RUNTIME_INSTALL_LEDGER_OWNER.store(owner, Ordering::Relaxed);
    RUNTIME_INSTALL_LEDGER_SEQUENCE.store(request.request_sequence(), Ordering::Relaxed);
    RUNTIME_INSTALL_SERVICE
        .state
        .store(INSTALL_PENDING, Ordering::Release);
}

#[cfg(feature = "androidbox-runtime-install2")]
fn published_runtime_install_matches(owner: u64, request: &AndroidPackageInstallRequest) -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package install identity compared with IRQ enabled");
    }
    let state = RUNTIME_INSTALL_SERVICE.state.load(Ordering::Acquire);
    if !matches!(state, INSTALL_PENDING | INSTALL_RUNNING | INSTALL_COMPLETE)
        || RUNTIME_INSTALL_SERVICE.owner.load(Ordering::Relaxed) != owner
    {
        return false;
    }
    // SAFETY: the request is immutable in every published state.
    unsafe { (*RUNTIME_INSTALL_SERVICE.value.get()).request == Some(*request) }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn published_runtime_install_owner_can_retry() -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package install liveness checked with IRQ enabled");
    }
    let owner = RUNTIME_INSTALL_SERVICE.owner.load(Ordering::Relaxed);
    owner != 0 && crate::process::android_package_uninstall_owner_can_retry_irq_masked(owner)
}

#[cfg(feature = "androidbox-runtime-install2")]
fn discard_published_runtime_install() {
    if !aarch64::irq_is_masked() {
        panic!("Android package install discarded with IRQ enabled");
    }
    let workspace = unsafe { &mut *RUNTIME_INSTALL_SERVICE.value.get() };
    workspace.request = None;
    workspace.outcome = None;
    RUNTIME_INSTALL_SERVICE.owner.store(0, Ordering::Relaxed);
    RUNTIME_INSTALL_SERVICE
        .state
        .store(INSTALL_EMPTY, Ordering::Release);
}

#[cfg(feature = "androidbox-runtime-install2")]
fn runtime_install_identity(
    request: &AndroidPackageInstallRequest,
) -> Result<AndroidPackageInstallCandidate, RuntimeInstallError> {
    if request.profile() != Some(AndroidPackageCompatibilityProfile::Resources1)
        || request.action().is_none()
    {
        return Err(RuntimeInstallError::Unsupported);
    }
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return Err(RuntimeInstallError::RequiresReset);
    }
    // SAFETY: caller is in the IRQ-masked syscall domain.
    let candidate =
        unsafe { *INSTALL_CANDIDATE.value.get() }.ok_or(RuntimeInstallError::NoCandidate)?;
    if !runtime_install_request_matches_candidate(request, &candidate) {
        return Err(RuntimeInstallError::Stale);
    }
    Ok(candidate)
}

#[cfg(feature = "androidbox-runtime-install2")]
fn runtime_install_request_matches_candidate(
    request: &AndroidPackageInstallRequest,
    candidate: &AndroidPackageInstallCandidate,
) -> bool {
    candidate.present()
        && request.candidate_id() == candidate.candidate_id()
        && request.action() == candidate.action()
        && request.expected_generation() == candidate.expected_generation()
        && request.expected_version_code() == candidate.expected_version_code()
        && request.version_code() == candidate.version_code()
        && request.apk_length() == candidate.apk_length()
        && request.apk_sha256() == candidate.apk_sha256()
        && request.signer_sha256() == candidate.signer_sha256()
        && request.package_name_bytes() == candidate.package_name_bytes()
        && candidate.profile() == Some(AndroidPackageCompatibilityProfile::Resources1)
}

#[cfg(all(
    feature = "androidbox-runtime-install2",
    not(feature = "androidbox-multipackage4")
))]
fn runtime_install_live_base_matches(
    request: &AndroidPackageInstallRequest,
    live: &LivePackageCatalog,
) -> bool {
    match request.action() {
        Some(AndroidPackageInstallAction::Install) => {
            live.installed.is_none()
                && live.package.is_none()
                && live.removed.is_none()
                && request.expected_generation() == 0
                && request.expected_version_code() == 0
        }
        Some(AndroidPackageInstallAction::Update) => {
            live.formatted
                && live.removed.is_none()
                && live.installed.is_some_and(|installed| {
                    installed.generation() == request.expected_generation()
                        && installed.metadata().version_code() == request.expected_version_code()
                })
                && live.package.is_some_and(|package| {
                    package.generation == request.expected_generation()
                        && package.version_code == request.expected_version_code()
                        && package.package.as_bytes() == request.package_name_bytes()
                })
        }
        Some(AndroidPackageInstallAction::Reinstall) => {
            live.formatted
                && live.installed.is_none()
                && live.package.is_none()
                && live.removed.is_some_and(|removed| {
                    removed.generation == request.expected_generation()
                        && removed.last_version_code == request.expected_version_code()
                        && removed.package.as_bytes() == request.package_name_bytes()
                })
        }
        None => false,
    }
}

#[cfg(feature = "androidbox-multipackage4")]
fn runtime_install_live_base_matches(
    request: &AndroidPackageInstallRequest,
    live: &LivePackageCatalog,
) -> bool {
    let matching = live
        .multi_packages
        .iter()
        .enumerate()
        .find_map(|(index, package)| {
            package
                .filter(|package| package.package.as_bytes() == request.package_name_bytes())
                .map(|package| (index, package))
        });
    match request.action() {
        Some(AndroidPackageInstallAction::Install) => {
            matching.is_none()
                && request.expected_generation() == 0
                && request.expected_version_code() == 0
        }
        Some(AndroidPackageInstallAction::Update) => matching.is_some_and(|(index, package)| {
            package.generation == request.expected_generation()
                && package.version_code == request.expected_version_code()
                && live.multi_installed[index].is_some_and(|installed| {
                    installed.package().generation() == request.expected_generation()
                        && installed.package().metadata().version_code()
                            == request.expected_version_code()
                })
        }),
        Some(AndroidPackageInstallAction::Reinstall) => {
            matching.is_none()
                && request.expected_generation() != 0
                && request.expected_version_code() != 0
        }
        None => false,
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn execute_runtime_install(
    request: &AndroidPackageInstallRequest,
) -> (
    Result<RuntimeInstallDurable, RuntimeInstallError>,
    PackageIoEvidence,
) {
    let mut evidence = PackageIoEvidence::default();
    let result = execute_runtime_install_once(request, &mut evidence);
    (result, evidence)
}

#[cfg(all(
    feature = "androidbox-runtime-install2",
    not(feature = "androidbox-multipackage4")
))]
fn execute_runtime_install_once(
    request: &AndroidPackageInstallRequest,
    evidence: &mut PackageIoEvidence,
) -> Result<RuntimeInstallDurable, RuntimeInstallError> {
    let published = runtime_install_identity(request)?;
    let source = crate::package_source::bytes().ok_or(RuntimeInstallError::NoCandidate)?;
    let admitted = admit_source(source).map_err(map_runtime_install_admission_error)?;
    let expected_device_sectors = RELAUNCH_DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = RELAUNCH_DEADLINE_BUDGET.load(Ordering::Acquire);
    if expected_device_sectors == 0 || deadline_budget == 0 {
        return Err(RuntimeInstallError::RequiresReset);
    }
    let (device_sectors, read_only) =
        storage::device_contract().map_err(|_| RuntimeInstallError::RequiresReset)?;
    if read_only
        || device_sectors != expected_device_sectors
        || device_sectors != PACKAGES_DEVICE_SECTORS
    {
        return Err(RuntimeInstallError::RequiresReset);
    }

    let mut io = PackageIo::new(device_sectors, deadline_budget);
    let (current, unformatted) = match recover_state(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(state) => (state, false),
        Err(PackageStoreError::Unformatted) => (PackageState::Empty, true),
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_install_store_error(error));
        }
    };
    let rebuilt = classify_runtime_install_candidate(&admitted, current)
        .map_err(map_runtime_install_admission_error)?
        .ok_or(RuntimeInstallError::Stale)?;
    if rebuilt != published || !runtime_install_request_matches_candidate(request, &rebuilt) {
        *evidence = io.evidence();
        return Err(RuntimeInstallError::Stale);
    }
    if unformatted {
        if request.action() != Some(AndroidPackageInstallAction::Install) {
            *evidence = io.evidence();
            return Err(RuntimeInstallError::Stale);
        }
        if let Err(error) = format(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
            *evidence = io.evidence();
            return Err(map_runtime_install_store_error(error));
        }
    }
    let installed = match install(
        &mut io,
        PACKAGES_PARTITION_FIRST_LBA,
        admitted.metadata,
        source,
    ) {
        Ok(installed) => installed,
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_install_store_error(error));
        }
    };
    // SAFETY: INSTALL_RUNNING grants this monitor the only scratch lease.
    let output = unsafe { &mut *RUNTIME_INSTALL_SCRATCH.bytes.get() };
    let read_length = match read_blob(&mut io, PACKAGES_PARTITION_FIRST_LBA, &installed, output) {
        Ok(length) => length,
        Err(error) => {
            output.fill(0);
            *evidence = io.evidence();
            return Err(map_runtime_install_store_error(error));
        }
    };
    let package = match validate_durable_package(&installed, &output[..read_length]) {
        Ok(package) => package,
        Err(_) => {
            output.fill(0);
            *evidence = io.evidence();
            return Err(RuntimeInstallError::Corrupt);
        }
    };
    output.fill(0);
    *evidence = io.evidence();
    if evidence.reads == 0 || evidence.writes == 0 || evidence.flushes == 0 {
        return Err(RuntimeInstallError::Corrupt);
    }
    if installed.generation()
        != request
            .expected_generation()
            .checked_add(1)
            .ok_or(RuntimeInstallError::Corrupt)?
        || installed.metadata().version_code() != request.version_code()
        || installed.apk_length() != request.apk_length() as usize
        || installed.metadata().package() != request.package_name()
        || installed.metadata().apk_sha256() != request.apk_sha256()
        || installed.metadata().signer_cert_sha256() != request.signer_sha256()
        || !installed_matches_snapshot(&installed, &package)
    {
        return Err(RuntimeInstallError::Corrupt);
    }
    Ok(RuntimeInstallDurable {
        installed,
        package,
        previous_generation: request.expected_generation(),
    })
}

#[cfg(feature = "androidbox-multipackage4")]
fn execute_runtime_install_once(
    request: &AndroidPackageInstallRequest,
    evidence: &mut PackageIoEvidence,
) -> Result<RuntimeInstallDurable, RuntimeInstallError> {
    let published = runtime_install_identity(request)?;
    let source = crate::package_source::bytes().ok_or(RuntimeInstallError::NoCandidate)?;
    let admitted = admit_source(source).map_err(map_runtime_install_admission_error)?;
    let expected_device_sectors = RELAUNCH_DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = RELAUNCH_DEADLINE_BUDGET.load(Ordering::Acquire);
    if expected_device_sectors == 0 || deadline_budget == 0 {
        return Err(RuntimeInstallError::RequiresReset);
    }
    let (device_sectors, read_only) =
        storage::device_contract().map_err(|_| RuntimeInstallError::RequiresReset)?;
    if read_only
        || device_sectors != expected_device_sectors
        || device_sectors != PACKAGES_DEVICE_SECTORS
    {
        return Err(RuntimeInstallError::RequiresReset);
    }

    let mut io = PackageIo::new(device_sectors, deadline_budget);
    let inspected = match inspect_multi(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(catalog) => catalog,
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_install_multi_error(error));
        }
    };
    let (_, current) = multi_target_state(&inspected, admitted.metadata.package())
        .map_err(map_runtime_install_admission_error)?;
    let rebuilt = classify_runtime_install_candidate(&admitted, current)
        .map_err(map_runtime_install_admission_error)?
        .ok_or(RuntimeInstallError::Stale)?;
    if rebuilt != published || !runtime_install_request_matches_candidate(request, &rebuilt) {
        *evidence = io.evidence();
        return Err(RuntimeInstallError::Stale);
    }
    if let Err(error) = ensure_multi_formatted(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        *evidence = io.evidence();
        return Err(map_runtime_install_multi_error(error));
    }
    let multi = match install_multi(
        &mut io,
        PACKAGES_PARTITION_FIRST_LBA,
        admitted.metadata,
        source,
    ) {
        Ok(installed) => installed,
        Err(error) => {
            *evidence = io.evidence();
            return Err(map_runtime_install_multi_error(error));
        }
    };
    let installed = multi.package();
    // SAFETY: INSTALL_RUNNING grants this monitor the only scratch lease.
    let output = unsafe { &mut *RUNTIME_INSTALL_SCRATCH.bytes.get() };
    let read_length = match read_multi_blob(&mut io, PACKAGES_PARTITION_FIRST_LBA, multi, output) {
        Ok(length) => length,
        Err(error) => {
            output.fill(0);
            *evidence = io.evidence();
            return Err(map_runtime_install_multi_error(error));
        }
    };
    let package = match validate_durable_package(&installed, &output[..read_length]) {
        Ok(package) => package,
        Err(_) => {
            output.fill(0);
            *evidence = io.evidence();
            return Err(RuntimeInstallError::Corrupt);
        }
    };
    output.fill(0);
    *evidence = io.evidence();
    if evidence.reads == 0 || evidence.writes == 0 || evidence.flushes == 0 {
        return Err(RuntimeInstallError::Corrupt);
    }
    if installed.generation()
        != request
            .expected_generation()
            .checked_add(1)
            .ok_or(RuntimeInstallError::Corrupt)?
        || installed.metadata().version_code() != request.version_code()
        || installed.apk_length() != request.apk_length() as usize
        || installed.metadata().package() != request.package_name()
        || installed.metadata().apk_sha256() != request.apk_sha256()
        || installed.metadata().signer_cert_sha256() != request.signer_sha256()
        || !installed_matches_snapshot(&installed, &package)
    {
        return Err(RuntimeInstallError::Corrupt);
    }
    Ok(RuntimeInstallDurable {
        installed,
        package,
        previous_generation: request.expected_generation(),
        volume_index: multi.volume_index(),
    })
}

#[cfg(feature = "androidbox-multipackage4")]
fn map_runtime_install_multi_error(error: MultiPackageError) -> RuntimeInstallError {
    match error {
        MultiPackageError::Store(error) => map_runtime_install_store_error(error),
        MultiPackageError::Capacity => RuntimeInstallError::Unsupported,
        MultiPackageError::PackageNotFound => RuntimeInstallError::Stale,
        MultiPackageError::DuplicatePackage | MultiPackageError::SlotOutOfRange => {
            RuntimeInstallError::Corrupt
        }
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn map_runtime_install_admission_error(error: Error) -> RuntimeInstallError {
    match error {
        Error::Store(
            PackageStoreError::PackageChanged
            | PackageStoreError::SignerChanged
            | PackageStoreError::VersionNotIncreasing
            | PackageStoreError::TransactionConflict,
        ) => RuntimeInstallError::Stale,
        Error::Signature(_)
        | Error::AndroidBox(_)
        | Error::Resources1Required
        | Error::SourceEmpty
        | Error::SourceTooLarge => RuntimeInstallError::Unsupported,
        Error::DeviceContract(_)
        | Error::DeviceSizeMismatch
        | Error::ReadOnlyDevice
        | Error::Store(PackageStoreError::Io(_) | PackageStoreError::OutcomeUnknown) => {
            RuntimeInstallError::RequiresReset
        }
        _ => RuntimeInstallError::Corrupt,
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn map_runtime_install_store_error(error: PackageStoreError) -> RuntimeInstallError {
    match error {
        PackageStoreError::PackageChanged
        | PackageStoreError::SignerChanged
        | PackageStoreError::VersionNotIncreasing
        | PackageStoreError::TransactionConflict
        | PackageStoreError::NotInstalled
        | PackageStoreError::StalePackage => RuntimeInstallError::Stale,
        PackageStoreError::Io(_) | PackageStoreError::OutcomeUnknown => {
            RuntimeInstallError::RequiresReset
        }
        PackageStoreError::Unformatted
        | PackageStoreError::VolumeBounds
        | PackageStoreError::AlreadyFormatted
        | PackageStoreError::NotVirgin
        | PackageStoreError::InvalidMetadata(_)
        | PackageStoreError::EmptyApk
        | PackageStoreError::ApkTooLarge
        | PackageStoreError::ApkDigestMismatch
        | PackageStoreError::GenerationExhausted
        | PackageStoreError::BufferTooSmall { .. }
        | PackageStoreError::Corrupt(_)
        | PackageStoreError::VerificationFailed => RuntimeInstallError::Corrupt,
    }
}

fn next_relaunch_sequence(owner: u64, sequence: u64) -> Result<(), RelaunchError> {
    if !aarch64::irq_is_masked() {
        panic!("Android package relaunch sequence checked with IRQ enabled");
    }
    if owner == 0 || sequence == 0 {
        return Err(RelaunchError::Sequence);
    }
    let ledger_owner = RELAUNCH_LEDGER_OWNER.load(Ordering::Relaxed);
    let last_sequence = RELAUNCH_LEDGER_SEQUENCE.load(Ordering::Relaxed);
    if ledger_owner != owner {
        return if sequence == 1 {
            Ok(())
        } else {
            Err(RelaunchError::Sequence)
        };
    }
    if last_sequence.checked_add(1) == Some(sequence) {
        Ok(())
    } else {
        Err(RelaunchError::Sequence)
    }
}

fn enqueue_relaunch(
    owner: u64,
    compatible_session_id: u64,
    request: AndroidPackageRelaunchRequest,
) {
    if !aarch64::irq_is_masked()
        || owner == 0
        || RELAUNCH_SERVICE.state.load(Ordering::Acquire) != RELAUNCH_EMPTY
    {
        panic!("invalid Android package relaunch publication");
    }
    // SAFETY: EMPTY plus the IRQ-masked syscall domain grants the sole write.
    let workspace = unsafe { &mut *RELAUNCH_SERVICE.value.get() };
    workspace.compatible_session_id = compatible_session_id;
    workspace.request = Some(request);
    workspace.outcome = None;
    RELAUNCH_SERVICE.owner.store(owner, Ordering::Relaxed);
    // The canonical enqueue, rather than completion, consumes the sequence.
    RELAUNCH_LEDGER_OWNER.store(owner, Ordering::Relaxed);
    RELAUNCH_LEDGER_SEQUENCE.store(request.request_sequence(), Ordering::Relaxed);
    RELAUNCH_SERVICE
        .state
        .store(RELAUNCH_PENDING, Ordering::Release);
}

fn published_relaunch_matches(
    owner: u64,
    compatible_session_id: u64,
    request: &AndroidPackageRelaunchRequest,
) -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package relaunch identity compared with IRQ enabled");
    }
    let state = RELAUNCH_SERVICE.state.load(Ordering::Acquire);
    if !matches!(
        state,
        RELAUNCH_PENDING | RELAUNCH_RUNNING | RELAUNCH_COMPLETE
    ) || RELAUNCH_SERVICE.owner.load(Ordering::Relaxed) != owner
    {
        return false;
    }
    // SAFETY: every published state keeps the request immutable. On this
    // single-core kernel the caller is either the IRQ-masked syscall path or
    // the monitor that exclusively owns RUNNING.
    unsafe {
        let workspace = &*RELAUNCH_SERVICE.value.get();
        workspace.compatible_session_id == compatible_session_id
            && workspace.request == Some(*request)
    }
}

fn published_relaunch_owner_can_retry() -> bool {
    if !aarch64::irq_is_masked() {
        panic!("Android package relaunch liveness checked with IRQ enabled");
    }
    let owner = RELAUNCH_SERVICE.owner.load(Ordering::Relaxed);
    if owner == 0 {
        panic!("published Android package relaunch has no owner");
    }
    crate::process::android_package_relaunch_owner_can_retry_irq_masked(owner)
}

fn discard_published_relaunch() {
    if !aarch64::irq_is_masked() {
        panic!("Android package relaunch discarded with IRQ enabled");
    }
    let state = RELAUNCH_SERVICE.state.load(Ordering::Acquire);
    if !matches!(
        state,
        RELAUNCH_PENDING | RELAUNCH_RUNNING | RELAUNCH_COMPLETE
    ) {
        panic!("Android package relaunch discard observed no request");
    }
    // SAFETY: syscall access is serialized by the IRQ mask and the monitor
    // never calls this while its own RUNNING operation is in flight.
    let workspace = unsafe { &mut *RELAUNCH_SERVICE.value.get() };
    workspace.compatible_session_id = 0;
    workspace.request = None;
    workspace.outcome = None;
    RELAUNCH_SERVICE.owner.store(0, Ordering::Relaxed);
    RELAUNCH_SERVICE
        .state
        .store(RELAUNCH_EMPTY, Ordering::Release);
}

#[cfg(not(feature = "androidbox-multipackage4"))]
type LiveInstalledHandle = InstalledPackage;
#[cfg(feature = "androidbox-multipackage4")]
type LiveInstalledHandle = MultiInstalledPackage;

#[cfg(not(feature = "androidbox-multipackage4"))]
fn live_installed_package(handle: LiveInstalledHandle) -> InstalledPackage {
    handle
}

#[cfg(feature = "androidbox-multipackage4")]
fn live_installed_package(handle: LiveInstalledHandle) -> InstalledPackage {
    handle.package()
}

#[cfg(not(feature = "androidbox-multipackage4"))]
fn boot_relaunch_identity(
    request: &AndroidPackageRelaunchRequest,
) -> Result<(InstalledPackageSnapshot, LiveInstalledHandle), RelaunchError> {
    if request.profile() != AndroidPackageCompatibilityProfile::Resources1 {
        return Err(RelaunchError::Unsupported);
    }
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return Err(RelaunchError::RequiresReset);
    }
    let catalog = catalog_snapshot().ok_or(RelaunchError::RequiresReset)?;
    let package = catalog.package.ok_or(RelaunchError::NotInstalled)?;
    if !request_matches_snapshot(request, &package) {
        return Err(RelaunchError::Stale);
    }
    let saved_daif = aarch64::save_and_mask_irq();
    // SAFETY: the IRQ mask serializes the live handle with ABI 52 removal.
    let installed = unsafe { (*LIVE_PACKAGE_CATALOG.value.get()).installed };
    aarch64::restore_daif(saved_daif);
    let installed = installed.ok_or(RelaunchError::NotInstalled)?;
    if !request_matches_installed(request, &installed)
        || !installed_matches_snapshot(&installed, &package)
    {
        return Err(RelaunchError::Stale);
    }
    Ok((package, installed))
}

#[cfg(feature = "androidbox-multipackage4")]
fn boot_relaunch_identity(
    request: &AndroidPackageRelaunchRequest,
) -> Result<(InstalledPackageSnapshot, LiveInstalledHandle), RelaunchError> {
    if request.profile() != AndroidPackageCompatibilityProfile::Resources1 {
        return Err(RelaunchError::Unsupported);
    }
    if PUBLICATION_STATE.load(Ordering::Acquire) != STATE_READY {
        return Err(RelaunchError::RequiresReset);
    }
    let saved_daif = aarch64::save_and_mask_irq();
    // SAFETY: the IRQ mask serializes directory lookup with runtime mutation.
    let live = unsafe { *LIVE_PACKAGE_CATALOG.value.get() };
    let match_index = live.multi_packages.iter().position(|package| {
        package.is_some_and(|package| request_matches_snapshot(request, &package))
    });
    let result = match_index
        .and_then(|index| Some((live.multi_packages[index]?, live.multi_installed[index]?)));
    aarch64::restore_daif(saved_daif);
    let (package, installed) = result.ok_or(RelaunchError::NotInstalled)?;
    let inner = installed.package();
    if !request_matches_installed(request, &inner) || !installed_matches_snapshot(&inner, &package)
    {
        return Err(RelaunchError::Stale);
    }
    Ok((package, installed))
}

fn request_matches_snapshot(
    request: &AndroidPackageRelaunchRequest,
    package: &InstalledPackageSnapshot,
) -> bool {
    request.expected_generation() == package.generation
        && request.expected_version_code() == package.version_code
        && request.apk_length() == package.apk_length
        && request.apk_sha256() == &package.apk_sha256
        && request.signer_sha256() == &package.signer_cert_sha256
        && request.package_name_bytes() == package.package.as_bytes()
        && request.activity_name_bytes() == package.activity.as_bytes()
        && package.compatibility_profile == CompatibilityProfile::Resources1
}

fn request_matches_installed(
    request: &AndroidPackageRelaunchRequest,
    installed: &InstalledPackage,
) -> bool {
    let metadata = installed.metadata();
    request.expected_generation() == installed.generation()
        && request.expected_version_code() == metadata.version_code()
        && request.apk_length() as usize == installed.apk_length()
        && request.apk_sha256() == metadata.apk_sha256()
        && request.signer_sha256() == metadata.signer_cert_sha256()
        && request.package_name() == metadata.package()
        && request.activity_name() == metadata.activity()
        && metadata.compatibility_profile() == CompatibilityProfile::Resources1
}

fn installed_matches_snapshot(
    installed: &InstalledPackage,
    package: &InstalledPackageSnapshot,
) -> bool {
    let metadata = installed.metadata();
    installed.generation() == package.generation
        && installed.slot() == package.slot
        && installed.apk_length() == package.apk_length as usize
        && metadata.version_code() == package.version_code
        && metadata.package().as_bytes() == package.package.as_bytes()
        && metadata.activity().as_bytes() == package.activity.as_bytes()
        && metadata.apk_sha256() == &package.apk_sha256
        && metadata.signer_cert_sha256() == &package.signer_cert_sha256
        && metadata.compatibility_profile() == package.compatibility_profile
}

fn execute_durable_relaunch(
    request: &AndroidPackageRelaunchRequest,
) -> (Result<DurableRelaunch, RelaunchError>, PackageIoEvidence) {
    let mut evidence = PackageIoEvidence::default();
    let result = execute_durable_relaunch_once(request, &mut evidence);
    (result, evidence)
}

fn execute_durable_relaunch_once(
    request: &AndroidPackageRelaunchRequest,
    evidence: &mut PackageIoEvidence,
) -> Result<DurableRelaunch, RelaunchError> {
    let (boot_package, boot_handle) = boot_relaunch_identity(request)?;
    let boot_installed = live_installed_package(boot_handle);
    let expected_device_sectors = RELAUNCH_DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = RELAUNCH_DEADLINE_BUDGET.load(Ordering::Acquire);
    if expected_device_sectors == 0 || deadline_budget == 0 {
        return Err(RelaunchError::RequiresReset);
    }
    let (live_device_sectors, _) =
        storage::device_contract().map_err(|_| RelaunchError::RequiresReset)?;
    if live_device_sectors != expected_device_sectors
        || live_device_sectors != PACKAGES_DEVICE_SECTORS
    {
        return Err(RelaunchError::RequiresReset);
    }

    let mut io = RelaunchReadIo::new(live_device_sectors, deadline_budget);
    #[cfg(not(feature = "androidbox-multipackage4"))]
    let recovered_handle = {
        let recovered = match recover_state(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
            Ok(recovered) => recovered,
            Err(error) => {
                *evidence = io.evidence();
                return Err(map_relaunch_store_error(error));
            }
        };
        match recovered {
            PackageState::Installed(installed) => installed,
            PackageState::Empty | PackageState::Removed(_) => {
                *evidence = io.evidence();
                return Err(RelaunchError::NotInstalled);
            }
        }
    };
    #[cfg(feature = "androidbox-multipackage4")]
    let recovered_handle = match recover_multi(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(catalog) => match catalog.find_installed(request.package_name()) {
            Some(installed) => installed,
            None => {
                *evidence = io.evidence();
                return Err(RelaunchError::NotInstalled);
            }
        },
        Err(MultiPackageError::Store(error)) => {
            *evidence = io.evidence();
            return Err(map_relaunch_store_error(error));
        }
        Err(_) => {
            *evidence = io.evidence();
            return Err(RelaunchError::Corrupt);
        }
    };
    let installed = live_installed_package(recovered_handle);
    if recovered_handle != boot_handle
        || installed != boot_installed
        || !request_matches_installed(request, &installed)
        || !installed_matches_snapshot(&installed, &boot_package)
    {
        *evidence = io.evidence();
        return Err(RelaunchError::Stale);
    }

    // SAFETY: RELAUNCH_RUNNING grants this monitor turn the only scratch
    // lease. No completion contains a reference to these bytes.
    let scratch = unsafe { &mut *RELAUNCH_SCRATCH.bytes.get() };
    scratch.fill(0);
    #[cfg(not(feature = "androidbox-multipackage4"))]
    let read_result = read_blob(&mut io, PACKAGES_PARTITION_FIRST_LBA, &installed, scratch);
    #[cfg(feature = "androidbox-multipackage4")]
    let read_result = read_multi_blob(
        &mut io,
        PACKAGES_PARTITION_FIRST_LBA,
        recovered_handle,
        scratch,
    )
    .map_err(|error| match error {
        MultiPackageError::Store(error) => error,
        MultiPackageError::Capacity
        | MultiPackageError::DuplicatePackage
        | MultiPackageError::PackageNotFound
        | MultiPackageError::SlotOutOfRange => PackageStoreError::VerificationFailed,
    });
    let result = match read_result {
        Ok(length) if length == installed.apk_length() => {
            validate_durable_package(&installed, &scratch[..length])
                .map_err(map_relaunch_validation_error)
        }
        Ok(_) => Err(RelaunchError::Corrupt),
        Err(error) => Err(map_relaunch_store_error(error)),
    };
    let io_evidence = io.evidence();
    let result = result.and_then(|fresh| {
        if io_evidence.reads == 0 || io_evidence.writes != 0 || io_evidence.flushes != 0 {
            return Err(RelaunchError::Corrupt);
        }
        if fresh != boot_package || !request_matches_snapshot(request, &fresh) {
            return Err(RelaunchError::Stale);
        }
        #[cfg(feature = "androidbox-el0-runtime0")]
        {
            let image = Vmo::try_from_package_image(&scratch[..fresh.apk_length as usize])
                .map_err(|error| match error {
                    VmoCreateError::OutOfMemory => RelaunchError::OutOfMemory,
                    VmoCreateError::TooLarge => RelaunchError::Corrupt,
                })?;
            Ok(DurableRelaunch {
                package: fresh,
                image,
            })
        }
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        {
            Ok(fresh)
        }
    });
    // The monitor retains only the immutable VMO in EL0Runtime-0. The mutable
    // disk scratch is wiped on every terminal path before publication.
    scratch.fill(0);
    *evidence = io_evidence;
    result
}

fn map_relaunch_store_error(error: PackageStoreError) -> RelaunchError {
    match error {
        PackageStoreError::Unformatted | PackageStoreError::NotInstalled => {
            RelaunchError::NotInstalled
        }
        PackageStoreError::StalePackage => RelaunchError::Stale,
        PackageStoreError::Io(_) | PackageStoreError::OutcomeUnknown => {
            RelaunchError::RequiresReset
        }
        PackageStoreError::VolumeBounds
        | PackageStoreError::AlreadyFormatted
        | PackageStoreError::NotVirgin
        | PackageStoreError::InvalidMetadata(_)
        | PackageStoreError::EmptyApk
        | PackageStoreError::ApkTooLarge
        | PackageStoreError::ApkDigestMismatch
        | PackageStoreError::PackageChanged
        | PackageStoreError::SignerChanged
        | PackageStoreError::VersionNotIncreasing
        | PackageStoreError::TransactionConflict
        | PackageStoreError::GenerationExhausted
        | PackageStoreError::BufferTooSmall { .. }
        | PackageStoreError::Corrupt(_)
        | PackageStoreError::VerificationFailed => RelaunchError::Corrupt,
    }
}

fn map_relaunch_validation_error(error: Error) -> RelaunchError {
    match error {
        Error::Resources1Required | Error::DurableCompatibilityMismatch => {
            RelaunchError::Unsupported
        }
        Error::Store(error) => map_relaunch_store_error(error),
        #[cfg(feature = "androidbox-multipackage4")]
        Error::MultiStore(MultiPackageError::Store(error)) => map_relaunch_store_error(error),
        #[cfg(feature = "androidbox-multipackage4")]
        Error::MultiStore(
            MultiPackageError::Capacity
            | MultiPackageError::DuplicatePackage
            | MultiPackageError::PackageNotFound
            | MultiPackageError::SlotOutOfRange,
        ) => RelaunchError::Corrupt,
        Error::DeviceContract(_) | Error::DeviceSizeMismatch => RelaunchError::RequiresReset,
        Error::AlreadyInitialized
        | Error::InvalidDeadlineBudget
        | Error::Gpt(_)
        | Error::ReadOnlyDevice
        | Error::SourceEmpty
        | Error::SourceTooLarge
        | Error::Signature(_)
        | Error::AndroidBox(_)
        | Error::Metadata(_)
        | Error::DurableLengthMismatch
        | Error::DurableApkDigestMismatch
        | Error::DurableSignerMismatch
        | Error::DurableManifestMismatch
        | Error::DurableTransactionMismatch
        | Error::ConflictingPackageInputs
        | Error::UninstallRequest(_)
        | Error::UninstallTargetMismatch
        | Error::DurableRemovalMismatch
        | Error::SnapshotText => RelaunchError::Corrupt,
        #[cfg(feature = "androidbox-icon-resources5")]
        Error::LauncherIconPalette => RelaunchError::Corrupt,
    }
}

#[cfg(feature = "androidbox-multipackage4")]
#[derive(Clone, Copy)]
struct MultiCatalogLoad {
    installed: [Option<MultiInstalledPackage>; MULTI_PACKAGE_CAPACITY],
    packages: [Option<InstalledPackageSnapshot>; MULTI_PACKAGE_CAPACITY],
    selected_installed: Option<InstalledPackage>,
    selected_package: Option<InstalledPackageSnapshot>,
}

#[cfg(feature = "androidbox-multipackage4")]
fn load_multi_catalog(
    io: &mut PackageIo,
    catalog: &MultiPackageCatalog,
    preferred_package: Option<&str>,
) -> Result<MultiCatalogLoad, Error> {
    let selected_index = preferred_package
        .and_then(|preferred| {
            catalog
                .states()
                .iter()
                .position(|state| matches!(state, PackageState::Installed(installed) if installed.metadata().package() == preferred))
        })
        .or_else(|| {
            catalog
                .states()
                .iter()
                .position(|state| matches!(state, PackageState::Installed(_)))
        });
    let mut installed_entries = [None; MULTI_PACKAGE_CAPACITY];
    let mut package_entries = [None; MULTI_PACKAGE_CAPACITY];
    let mut selected_installed = None;
    let mut selected_package = None;
    for (index, state) in catalog.states().iter().copied().enumerate() {
        let PackageState::Installed(installed) = state else {
            continue;
        };
        let multi = MultiInstalledPackage::from_parts(index as u8, installed);
        // SAFETY: boot initialization or a monitor-owned mutation holds the
        // only lease to this bounded, non-retained scratch area.
        let output = unsafe { &mut *RELAUNCH_SCRATCH.bytes.get() };
        output.fill(0);
        let read_length = read_multi_blob(io, PACKAGES_PARTITION_FIRST_LBA, multi, output)?;
        if read_length != installed.apk_length() {
            output.fill(0);
            return Err(Error::DurableLengthMismatch);
        }
        let package = validate_durable_package(&installed, &output[..read_length])?;
        if selected_index == Some(index) {
            // Keep the legacy one-package immutable publication coherent with
            // the selected compatibility view.
            let selected_bytes = unsafe { &mut *INSTALLED_APK.bytes.get() };
            selected_bytes.fill(0);
            selected_bytes[..read_length].copy_from_slice(&output[..read_length]);
            selected_installed = Some(installed);
            selected_package = Some(package);
        }
        output.fill(0);
        installed_entries[index] = Some(multi);
        package_entries[index] = Some(package);
    }
    Ok(MultiCatalogLoad {
        installed: installed_entries,
        packages: package_entries,
        selected_installed,
        selected_package,
    })
}

#[cfg(feature = "androidbox-multipackage4")]
fn publish_multi_catalog(
    formatted: bool,
    load: MultiCatalogLoad,
    removed: Option<RemovedPackageSnapshot>,
) {
    // SAFETY: callers are either the sole boot initializer or the IRQ-masked
    // single-core runtime publication owner.
    unsafe {
        *LIVE_PACKAGE_CATALOG.value.get() = LivePackageCatalog {
            formatted,
            installed: load.selected_installed,
            package: load.selected_package,
            removed: if load.selected_package.is_none() {
                removed
            } else {
                None
            },
            multi_installed: load.installed,
            multi_packages: load.packages,
            revision: if formatted { 1 } else { 0 },
        };
    }
}

#[cfg(feature = "androidbox-multipackage4")]
fn multi_target_state(
    catalog: &MultiPackageCatalog,
    package: &str,
) -> Result<(usize, PackageState), Error> {
    let mut empty_index = None;
    for (index, state) in catalog.states().iter().copied().enumerate() {
        let identity_matches = match state {
            PackageState::Empty => {
                if empty_index.is_none() {
                    empty_index = Some(index);
                }
                false
            }
            PackageState::Installed(installed) => installed.metadata().package() == package,
            PackageState::Removed(removed) => removed.package() == package,
        };
        if identity_matches {
            return Ok((index, state));
        }
    }
    let index = empty_index.ok_or(Error::MultiStore(MultiPackageError::Capacity))?;
    Ok((index, PackageState::Empty))
}

#[cfg(feature = "androidbox-multipackage4")]
fn first_removed_snapshot(
    catalog: &MultiPackageCatalog,
) -> Result<Option<RemovedPackageSnapshot>, Error> {
    catalog
        .states()
        .iter()
        .find_map(|state| match state {
            PackageState::Removed(removed) => Some(snapshot_removed_package(removed)),
            PackageState::Empty | PackageState::Installed(_) => None,
        })
        .transpose()
}

#[cfg(feature = "androidbox-multipackage4")]
fn initialize_once(
    gpt: GptEvidence,
    source: Option<&[u8]>,
    uninstall_request_bytes: Option<&[u8]>,
    deadline_budget: u64,
) -> Result<PackageBootEvidence, Error> {
    if deadline_budget == 0 || deadline_budget > i64::MAX as u64 {
        return Err(Error::InvalidDeadlineBudget);
    }
    if source.is_some() && uninstall_request_bytes.is_some() {
        return Err(Error::ConflictingPackageInputs);
    }
    validate_gpt(gpt)?;
    let admitted = source.map(admit_source).transpose()?;
    let uninstall_request = uninstall_request_bytes
        .map(parse_package_uninstall_request)
        .transpose()
        .map_err(Error::UninstallRequest)?;
    let (live_sectors, read_only) = storage::device_contract().map_err(Error::DeviceContract)?;
    if live_sectors != PACKAGES_DEVICE_SECTORS || live_sectors != gpt.device_sectors {
        return Err(Error::DeviceSizeMismatch);
    }
    if read_only {
        return Err(Error::ReadOnlyDevice);
    }
    RELAUNCH_DEVICE_SECTORS.store(live_sectors, Ordering::Relaxed);
    RELAUNCH_DEADLINE_BUDGET.store(deadline_budget, Ordering::Relaxed);

    let mut io = PackageIo::new(live_sectors, deadline_budget);
    let inspected = inspect_multi(&mut io, PACKAGES_PARTITION_FIRST_LBA)?;

    if let Some(request) = uninstall_request {
        let (index, state) = multi_target_state(&inspected, request.package)?;
        let (expected, previous_generation, previous_version_code, was_removed) = match state {
            PackageState::Installed(installed) => (
                installed,
                installed.generation(),
                installed.metadata().version_code(),
                false,
            ),
            PackageState::Removed(removed) => (
                removed.last_installed(),
                removed.generation(),
                removed.version_code(),
                true,
            ),
            PackageState::Empty => return Err(Error::UninstallTargetMismatch),
        };
        validate_uninstall_request_target(&request, &expected)?;
        let removed = uninstall_multi(
            &mut io,
            PACKAGES_PARTITION_FIRST_LBA,
            request.operation_id,
            MultiInstalledPackage::from_parts(index as u8, expected),
            DataDisposition::NoManagedPackageData,
        )?;
        validate_durable_removal(&request, &removed)?;
        let recovered = inspect_multi(&mut io, PACKAGES_PARTITION_FIRST_LBA)?;
        let load = load_multi_catalog(&mut io, &recovered, None)?;
        let removed_snapshot = snapshot_removed_package(&removed)?;
        let formatted = recovered.any_formatted();
        publish_multi_catalog(formatted, load, Some(removed_snapshot));
        let operation = if was_removed {
            if io.evidence.writes == 0 {
                PackageBootOperation::UninstallReplay
            } else {
                PackageBootOperation::UninstallRepair
            }
        } else {
            PackageBootOperation::Uninstall
        };
        return Ok(PackageBootEvidence {
            formatted,
            format_performed: false,
            installed: false,
            source_used: false,
            uninstall_request_used: true,
            operation,
            previous_generation,
            previous_version_code,
            package: None,
            removed: Some(removed_snapshot),
            install_candidate: None,
            io: io.evidence(),
        });
    }

    if let (Some(admitted), Some(_)) = (admitted, source) {
        let (_, current) = multi_target_state(&inspected, admitted.metadata.package())?;
        let candidate = classify_runtime_install_candidate(&admitted, current)?;
        let action = candidate.and_then(|value| value.action());
        let operation = match action {
            Some(AndroidPackageInstallAction::Install) => PackageBootOperation::InstallCandidate,
            Some(AndroidPackageInstallAction::Update) => PackageBootOperation::UpdateCandidate,
            Some(AndroidPackageInstallAction::Reinstall) => {
                PackageBootOperation::ReinstallCandidate
            }
            None if matches!(current, PackageState::Installed(_)) => {
                PackageBootOperation::SourceReplay
            }
            None => return Err(Error::DurableManifestMismatch),
        };
        let (previous_generation, previous_version_code) = match current {
            PackageState::Empty => (0, 0),
            PackageState::Installed(installed) => {
                (installed.generation(), installed.metadata().version_code())
            }
            PackageState::Removed(removed) => (removed.generation(), removed.version_code()),
        };
        let load = load_multi_catalog(&mut io, &inspected, Some(admitted.metadata.package()))?;
        let live_selected = load.selected_package;
        let target_removed = match current {
            PackageState::Removed(removed) => Some(snapshot_removed_package(&removed)?),
            PackageState::Empty | PackageState::Installed(_) => None,
        };
        let (selected, removed) = match action {
            Some(AndroidPackageInstallAction::Install) => (None, None),
            Some(AndroidPackageInstallAction::Reinstall) => (None, target_removed),
            Some(AndroidPackageInstallAction::Update) | None => (live_selected, None),
        };
        let formatted = inspected.any_formatted();
        publish_multi_catalog(formatted, load, removed);
        let evidence = io.evidence();
        if evidence.writes != 0 || evidence.flushes != 0 {
            return Err(Error::Store(PackageStoreError::VerificationFailed));
        }
        return Ok(PackageBootEvidence {
            formatted,
            format_performed: false,
            installed: selected.is_some(),
            source_used: false,
            uninstall_request_used: false,
            operation,
            previous_generation,
            previous_version_code,
            package: selected,
            removed,
            install_candidate: candidate,
            io: evidence,
        });
    }

    if admitted.is_some() || source.is_some() {
        return Err(Error::SourceEmpty);
    }
    let load = load_multi_catalog(&mut io, &inspected, None)?;
    let selected = load.selected_package;
    let removed = if selected.is_none() {
        first_removed_snapshot(&inspected)?
    } else {
        None
    };
    let formatted = inspected.any_formatted();
    publish_multi_catalog(formatted, load, removed);
    let (operation, previous_generation, previous_version_code) = match (selected, removed) {
        (Some(package), _) => (
            PackageBootOperation::Recovery,
            package.generation,
            package.version_code,
        ),
        (None, Some(removed)) => (
            PackageBootOperation::RemovedRecovery,
            removed.generation,
            removed.last_version_code,
        ),
        (None, None) => (PackageBootOperation::EmptyRecovery, 0, 0),
    };
    Ok(PackageBootEvidence {
        formatted,
        format_performed: false,
        installed: selected.is_some(),
        source_used: false,
        uninstall_request_used: false,
        operation,
        previous_generation,
        previous_version_code,
        package: selected,
        removed,
        install_candidate: None,
        io: io.evidence(),
    })
}

#[cfg(not(feature = "androidbox-multipackage4"))]
fn initialize_once(
    gpt: GptEvidence,
    source: Option<&[u8]>,
    uninstall_request_bytes: Option<&[u8]>,
    deadline_budget: u64,
) -> Result<PackageBootEvidence, Error> {
    if deadline_budget == 0 || deadline_budget > i64::MAX as u64 {
        return Err(Error::InvalidDeadlineBudget);
    }
    if source.is_some() && uninstall_request_bytes.is_some() {
        return Err(Error::ConflictingPackageInputs);
    }
    validate_gpt(gpt)?;

    // This admission value is intentionally constructed before `PackageIo`.
    // Consequently no format/install write is reachable until the whole APK
    // has passed v2 signature, AndroidBox load, manifest, onCreate, and
    // Resources-1 resolution.
    let admitted = source.map(admit_source).transpose()?;
    let uninstall_request = uninstall_request_bytes
        .map(parse_package_uninstall_request)
        .transpose()
        .map_err(Error::UninstallRequest)?;

    let (live_sectors, read_only) = storage::device_contract().map_err(Error::DeviceContract)?;
    if live_sectors != PACKAGES_DEVICE_SECTORS || live_sectors != gpt.device_sectors {
        return Err(Error::DeviceSizeMismatch);
    }
    if read_only {
        return Err(Error::ReadOnlyDevice);
    }
    RELAUNCH_DEVICE_SECTORS.store(live_sectors, Ordering::Relaxed);
    RELAUNCH_DEADLINE_BUDGET.store(deadline_budget, Ordering::Relaxed);

    let mut io = PackageIo::new(live_sectors, deadline_budget);
    let mut formatted = true;
    #[cfg(not(feature = "androidbox-runtime-install2"))]
    let mut format_performed = false;
    #[cfg(feature = "androidbox-runtime-install2")]
    let format_performed = false;
    let recovered = match recover_state(&mut io, PACKAGES_PARTITION_FIRST_LBA) {
        Ok(state) => state,
        Err(PackageStoreError::Unformatted) => {
            if uninstall_request.is_some() {
                return Err(Error::UninstallTargetMismatch);
            }
            if admitted.is_none() {
                return Ok(empty_evidence(false, false, io.evidence()));
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            {
                formatted = false;
                PackageState::Empty
            }
            #[cfg(not(feature = "androidbox-runtime-install2"))]
            {
                format(&mut io, PACKAGES_PARTITION_FIRST_LBA)?;
                format_performed = true;
                PackageState::Empty
            }
        }
        Err(error) => return Err(error.into()),
    };

    if let Some(request) = uninstall_request {
        let (expected, previous_generation, previous_version_code, was_removed) = match recovered {
            PackageState::Installed(package) => (
                package,
                package.generation(),
                package.metadata().version_code(),
                false,
            ),
            PackageState::Removed(removed) => (
                removed.last_installed(),
                removed.generation(),
                removed.version_code(),
                true,
            ),
            PackageState::Empty => return Err(Error::UninstallTargetMismatch),
        };
        validate_uninstall_request_target(&request, &expected)?;
        let removed = uninstall(
            &mut io,
            PACKAGES_PARTITION_FIRST_LBA,
            request.operation_id,
            &expected,
            DataDisposition::NoManagedPackageData,
        )?;
        validate_durable_removal(&request, &removed)?;
        let operation = if was_removed {
            if io.evidence.writes == 0 {
                PackageBootOperation::UninstallReplay
            } else {
                PackageBootOperation::UninstallRepair
            }
        } else {
            PackageBootOperation::Uninstall
        };
        return Ok(PackageBootEvidence {
            formatted: true,
            format_performed: false,
            installed: false,
            source_used: false,
            uninstall_request_used: true,
            operation,
            previous_generation,
            previous_version_code,
            package: None,
            removed: Some(snapshot_removed_package(&removed)?),
            #[cfg(feature = "androidbox-runtime-install2")]
            install_candidate: None,
            io: io.evidence(),
        });
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    if let (Some(admitted), Some(_)) = (admitted, source) {
        return initialize_runtime_install_candidate(&mut io, formatted, recovered, admitted);
    }

    let previous = recovered;
    let installed = match (admitted, source) {
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        (Some(admitted), Some(apk)) => Some(install(
            &mut io,
            PACKAGES_PARTITION_FIRST_LBA,
            admitted.metadata,
            apk,
        )?),
        (None, None) => match recovered {
            PackageState::Installed(package) => Some(package),
            PackageState::Removed(removed) => {
                return Ok(PackageBootEvidence {
                    formatted: true,
                    format_performed: false,
                    installed: false,
                    source_used: false,
                    uninstall_request_used: false,
                    operation: PackageBootOperation::RemovedRecovery,
                    previous_generation: removed.generation(),
                    previous_version_code: removed.version_code(),
                    package: None,
                    removed: Some(snapshot_removed_package(&removed)?),
                    #[cfg(feature = "androidbox-runtime-install2")]
                    install_candidate: None,
                    io: io.evidence(),
                });
            }
            PackageState::Empty => None,
        },
        // `Option::map(...).transpose()` preserves the source/admission shape.
        _ => return Err(Error::SourceEmpty),
    };

    let Some(installed) = installed else {
        return Ok(empty_evidence(true, format_performed, io.evidence()));
    };

    // SAFETY: `initialize` owns the sole INITIALIZING write lease. No caller
    // can obtain this buffer until the final STATE_READY Release store.
    let output = unsafe { &mut *INSTALLED_APK.bytes.get() };
    let read_length = read_blob(&mut io, PACKAGES_PARTITION_FIRST_LBA, &installed, output)?;
    if read_length != installed.apk_length() {
        return Err(Error::DurableLengthMismatch);
    }
    let package = validate_durable_package(&installed, &output[..read_length])?;
    let operation = match (source.is_some(), previous) {
        (false, PackageState::Installed(_)) => PackageBootOperation::Recovery,
        (true, PackageState::Empty) => PackageBootOperation::Install,
        (true, PackageState::Installed(previous)) if previous == installed => {
            PackageBootOperation::SourceReplay
        }
        (true, PackageState::Installed(_)) => PackageBootOperation::Update,
        (true, PackageState::Removed(_)) => PackageBootOperation::Reinstall,
        (false, PackageState::Empty | PackageState::Removed(_)) => {
            return Err(Error::DurableManifestMismatch);
        }
    };
    let (previous_generation, previous_version_code) = match previous {
        PackageState::Empty => (0, 0),
        PackageState::Installed(package) => {
            (package.generation(), package.metadata().version_code())
        }
        PackageState::Removed(removed) => (removed.generation(), removed.version_code()),
    };
    // SAFETY: the successful `initialize` caller owns the sole INITIALIZING
    // publication lease. ABI 52 may later replace this exact live catalog
    // under the IRQ mask only after a durable uninstall transaction.
    unsafe {
        *LIVE_PACKAGE_CATALOG.value.get() = LivePackageCatalog {
            formatted: true,
            installed: Some(installed),
            package: Some(package),
            removed: None,
            ..LivePackageCatalog::new()
        };
    }
    Ok(PackageBootEvidence {
        formatted: true,
        format_performed,
        installed: true,
        source_used: admitted.is_some(),
        uninstall_request_used: false,
        operation,
        previous_generation,
        previous_version_code,
        package: Some(package),
        removed: None,
        #[cfg(feature = "androidbox-runtime-install2")]
        install_candidate: None,
        io: io.evidence(),
    })
}

#[cfg(all(
    feature = "androidbox-runtime-install2",
    not(feature = "androidbox-multipackage4")
))]
fn initialize_runtime_install_candidate(
    io: &mut PackageIo,
    formatted: bool,
    recovered: PackageState,
    admitted: AdmittedSource,
) -> Result<PackageBootEvidence, Error> {
    let candidate = classify_runtime_install_candidate(&admitted, recovered)?;
    let (previous_generation, previous_version_code) = match recovered {
        PackageState::Empty => (0, 0),
        PackageState::Installed(package) => {
            (package.generation(), package.metadata().version_code())
        }
        PackageState::Removed(removed) => (removed.generation(), removed.version_code()),
    };
    let operation = match candidate.and_then(|candidate| candidate.action()) {
        Some(AndroidPackageInstallAction::Install) => PackageBootOperation::InstallCandidate,
        Some(AndroidPackageInstallAction::Update) => PackageBootOperation::UpdateCandidate,
        Some(AndroidPackageInstallAction::Reinstall) => PackageBootOperation::ReinstallCandidate,
        None if matches!(recovered, PackageState::Installed(_)) => {
            PackageBootOperation::SourceReplay
        }
        None => return Err(Error::DurableManifestMismatch),
    };

    let (installed, package, removed) = match recovered {
        PackageState::Installed(installed) => {
            // SAFETY: initialize owns the sole pre-publication write lease.
            let output = unsafe { &mut *INSTALLED_APK.bytes.get() };
            let read_length = read_blob(io, PACKAGES_PARTITION_FIRST_LBA, &installed, output)?;
            if read_length != installed.apk_length() {
                return Err(Error::DurableLengthMismatch);
            }
            let package = validate_durable_package(&installed, &output[..read_length])?;
            // SAFETY: the outer initialize call has not published STATE_READY.
            unsafe {
                *LIVE_PACKAGE_CATALOG.value.get() = LivePackageCatalog {
                    formatted: true,
                    installed: Some(installed),
                    package: Some(package),
                    removed: None,
                    ..LivePackageCatalog::new()
                };
            }
            (true, Some(package), None)
        }
        PackageState::Removed(removed) => (false, None, Some(snapshot_removed_package(&removed)?)),
        PackageState::Empty => (false, None, None),
    };
    let evidence = io.evidence();
    if evidence.writes != 0 || evidence.flushes != 0 {
        return Err(Error::Store(PackageStoreError::VerificationFailed));
    }
    Ok(PackageBootEvidence {
        formatted,
        format_performed: false,
        installed,
        source_used: false,
        uninstall_request_used: false,
        operation,
        previous_generation,
        previous_version_code,
        package,
        removed,
        install_candidate: candidate,
        io: evidence,
    })
}

fn empty_evidence(
    formatted: bool,
    format_performed: bool,
    io: PackageIoEvidence,
) -> PackageBootEvidence {
    PackageBootEvidence {
        formatted,
        format_performed,
        installed: false,
        source_used: false,
        uninstall_request_used: false,
        operation: PackageBootOperation::EmptyRecovery,
        previous_generation: 0,
        previous_version_code: 0,
        package: None,
        removed: None,
        #[cfg(feature = "androidbox-runtime-install2")]
        install_candidate: None,
        io,
    }
}

fn validate_uninstall_request_target(
    request: &PackageUninstallRequest<'_>,
    installed: &InstalledPackage,
) -> Result<(), Error> {
    let metadata = installed.metadata();
    let disposition_matches = matches!(
        request.disposition,
        PackageUninstallDataDisposition::NoManagedPackageData
    );
    if request.expected_generation != installed.generation()
        || request.expected_version_code != metadata.version_code()
        || request.expected_apk_length as usize != installed.apk_length()
        || request.package != metadata.package()
        || request.expected_apk_sha256 != *metadata.apk_sha256()
        || request.expected_signer_sha256 != *metadata.signer_cert_sha256()
        || !disposition_matches
    {
        return Err(Error::UninstallTargetMismatch);
    }
    Ok(())
}

fn validate_durable_removal(
    request: &PackageUninstallRequest<'_>,
    removed: &RemovedPackage,
) -> Result<(), Error> {
    if removed.operation_id() != request.operation_id
        || removed.expected_generation() != request.expected_generation
        || removed.version_code() != request.expected_version_code
        || removed.apk_length() != request.expected_apk_length as usize
        || removed.package() != request.package
        || removed.apk_sha256() != &request.expected_apk_sha256
        || removed.signer_cert_sha256() != &request.expected_signer_sha256
        || removed.disposition() != DataDisposition::NoManagedPackageData
    {
        return Err(Error::DurableRemovalMismatch);
    }
    Ok(())
}

fn snapshot_removed_package(removed: &RemovedPackage) -> Result<RemovedPackageSnapshot, Error> {
    Ok(RemovedPackageSnapshot {
        operation_id: removed.operation_id(),
        generation: removed.generation(),
        last_generation: removed.last_generation(),
        last_version_code: removed.version_code(),
        last_apk_length: removed.apk_length() as u32,
        last_blob_slot: removed.last_blob_slot(),
        package: FixedText::try_from_ascii(removed.package())?,
        last_apk_sha256: *removed.apk_sha256(),
        last_signer_cert_sha256: *removed.signer_cert_sha256(),
        data_disposition: removed.disposition(),
    })
}

#[cfg(feature = "androidbox-runtime-install2")]
fn classify_runtime_install_candidate(
    admitted: &AdmittedSource,
    current: PackageState,
) -> Result<Option<AndroidPackageInstallCandidate>, Error> {
    let metadata = admitted.metadata;
    let (action, expected_generation, expected_version_code) = match current {
        PackageState::Empty => (AndroidPackageInstallAction::Install, 0, 0),
        PackageState::Installed(installed) => {
            let previous = installed.metadata();
            if metadata.transaction_id() == previous.transaction_id() {
                if metadata == *previous && admitted.apk_length as usize == installed.apk_length() {
                    return Ok(None);
                }
                return Err(Error::Store(PackageStoreError::TransactionConflict));
            }
            if metadata.package() != previous.package() {
                return Err(Error::Store(PackageStoreError::PackageChanged));
            }
            if metadata.signer_cert_sha256() != previous.signer_cert_sha256() {
                return Err(Error::Store(PackageStoreError::SignerChanged));
            }
            if metadata.version_code() <= previous.version_code() {
                return Err(Error::Store(PackageStoreError::VersionNotIncreasing));
            }
            (
                AndroidPackageInstallAction::Update,
                installed.generation(),
                previous.version_code(),
            )
        }
        PackageState::Removed(removed) => {
            let previous = removed.last_metadata();
            if metadata.package() != previous.package() {
                return Err(Error::Store(PackageStoreError::PackageChanged));
            }
            if metadata.signer_cert_sha256() != previous.signer_cert_sha256() {
                return Err(Error::Store(PackageStoreError::SignerChanged));
            }
            if metadata.version_code() < previous.version_code()
                || (metadata.version_code() == previous.version_code()
                    && metadata.apk_sha256() != previous.apk_sha256())
            {
                return Err(Error::Store(PackageStoreError::VersionNotIncreasing));
            }
            (
                AndroidPackageInstallAction::Reinstall,
                removed.generation(),
                previous.version_code(),
            )
        }
    };
    let candidate_id = runtime_install_candidate_id(
        action,
        expected_generation,
        expected_version_code,
        &metadata,
        admitted.apk_length,
    );
    AndroidPackageInstallCandidate::new(AndroidPackageInstallCandidateMetadata {
        candidate_id,
        action,
        expected_generation,
        expected_version_code,
        version_code: metadata.version_code(),
        apk_length: admitted.apk_length,
        resources_table_crc32: admitted.resources_arsc_crc32,
        layout_xml_crc32: admitted.layout_xml_crc32,
        layout_resource_id: admitted.layout_resource_id,
        text_resource_id: admitted.text_resource_id,
        instruction_count: admitted.instruction_count,
        apk_sha256: *metadata.apk_sha256(),
        signer_sha256: *metadata.signer_cert_sha256(),
        package_name: metadata.package().as_bytes(),
        activity_name: metadata.activity().as_bytes(),
        title: admitted.launch_title.as_bytes(),
        text: admitted.launch_text.as_bytes(),
    })
    .map(Some)
    .map_err(|_| Error::SnapshotText)
}

#[cfg(feature = "androidbox-runtime-install2")]
fn runtime_install_candidate_id(
    action: AndroidPackageInstallAction,
    expected_generation: u64,
    expected_version_code: u64,
    metadata: &InstallMetadata,
    apk_length: u32,
) -> u64 {
    let mut digest = Sha256::new();
    digest.update(b"Bndroid AndroidBox Runtime Install-2 candidate v1\0");
    digest.update(&action.raw().to_le_bytes());
    digest.update(&expected_generation.to_le_bytes());
    digest.update(&expected_version_code.to_le_bytes());
    digest.update(&metadata.transaction_id().to_le_bytes());
    digest.update(&metadata.version_code().to_le_bytes());
    digest.update(&apk_length.to_le_bytes());
    digest.update(metadata.apk_sha256());
    digest.update(metadata.signer_cert_sha256());
    digest.update(metadata.package().as_bytes());
    digest.update(metadata.activity().as_bytes());
    let bytes = digest.finalize();
    let mut raw = [0; 8];
    raw.copy_from_slice(&bytes[..8]);
    let value = u64::from_le_bytes(raw);
    if value == 0 { 1 } else { value }
}

fn validate_gpt(gpt: GptEvidence) -> Result<(), Error> {
    if gpt.device_sectors != PACKAGES_DEVICE_SECTORS {
        return Err(Error::Gpt(GptPolicyError::DeviceSize));
    }
    let partition = gpt.partition;
    if partition.type_guid != BNDROID_PACKAGES_PARTITION_TYPE_GUID {
        return Err(Error::Gpt(GptPolicyError::PartitionType));
    }
    if partition.unique_guid != PACKAGES_PARTITION_UNIQUE_GUID {
        return Err(Error::Gpt(GptPolicyError::PartitionUniqueGuid));
    }
    if partition.entry_index != PACKAGES_PARTITION_ENTRY_INDEX {
        return Err(Error::Gpt(GptPolicyError::PartitionIndex));
    }
    if partition.name() != PACKAGES_PARTITION_NAME {
        return Err(Error::Gpt(GptPolicyError::PartitionName));
    }
    if partition.first_lba != PACKAGES_PARTITION_FIRST_LBA
        || partition.last_lba != PACKAGES_PARTITION_LAST_LBA
        || partition.sector_count() != PACKAGES_PARTITION_SECTORS
    {
        return Err(Error::Gpt(GptPolicyError::PartitionBounds));
    }
    if partition.attributes != 0 {
        return Err(Error::Gpt(GptPolicyError::PartitionAttributes));
    }
    Ok(())
}

fn admit_source(apk: &[u8]) -> Result<AdmittedSource, Error> {
    if apk.is_empty() {
        return Err(Error::SourceEmpty);
    }
    if apk.len() > MAX_APK_BYTES {
        return Err(Error::SourceTooLarge);
    }
    let signer = verify_apk_v2(apk)?;
    let image = load_android_box(apk)?;
    #[cfg(feature = "androidbox-runtime-install2")]
    {
        // ABI 53 publishes an actionable install candidate only when the
        // current isolated AndroidApp profile can actually open the package.
        // A static Resources-1 package remains valid historical input, but is
        // not installable into this interactive runtime.
        image.launch_activity_session()?;
    }
    let manifest = image.manifest_info();
    let launch = image.launch_activity()?;
    let resources = launch.resources.ok_or(Error::Resources1Required)?;
    let apk_digest = sha256(apk);
    let transaction_id = transaction_id(apk, signer, &launch);
    let metadata = InstallMetadata::try_new(
        transaction_id,
        manifest.package.as_str(),
        manifest.activity_descriptor.as_str(),
        u64::from(manifest.version_code),
        apk_digest,
        signer.certificate_sha256,
        CompatibilityProfile::Resources1,
    )?;
    Ok(AdmittedSource {
        metadata,
        apk_length: apk.len() as u32,
        launch_title: FixedText::try_from_ascii(launch.view.title.as_str())?,
        launch_text: FixedText::try_from_ascii(launch.view.text.as_str())?,
        resources_arsc_crc32: resources.resources_arsc_crc32,
        layout_xml_crc32: resources.layout_xml_crc32,
        layout_resource_id: resources.layout_resource_id,
        text_resource_id: resources.text_resource_id,
        instruction_count: launch.instruction_count,
    })
}

#[cfg(feature = "androidbox-icon-resources5")]
fn compact_launcher_icon(
    icon: Option<AndroidLauncherIcon>,
) -> Result<Option<InstalledLauncherIcon>, Error> {
    let Some(icon) = icon else {
        return Ok(None);
    };
    let mut palette = [0; INSTALLED_LAUNCHER_ICON_PALETTE_CAPACITY];
    let mut palette_len = 0usize;
    let mut indices = [0; INSTALLED_LAUNCHER_ICON_PACKED_BYTES];
    for (index, pixel) in icon.pixels().iter().copied().enumerate() {
        let palette_index = if let Some(existing) = palette[..palette_len]
            .iter()
            .position(|value| *value == pixel)
        {
            existing
        } else {
            if palette_len == palette.len() {
                return Err(Error::LauncherIconPalette);
            }
            palette[palette_len] = pixel;
            palette_len += 1;
            palette_len - 1
        };
        let packed = &mut indices[index / 2];
        if index.is_multiple_of(2) {
            *packed = (palette_index as u8) << 4;
        } else {
            *packed |= palette_index as u8;
        }
    }
    Ok(Some(InstalledLauncherIcon {
        resource_id: icon.resource_id(),
        png_crc32: icon.png_crc32(),
        palette,
        palette_len: palette_len as u8,
        indices,
        #[cfg(feature = "androidbox-density-icons7")]
        source_width: icon.source_width(),
        #[cfg(feature = "androidbox-density-icons7")]
        source_height: icon.source_height(),
        #[cfg(feature = "androidbox-density-icons7")]
        source_density_dpi: icon.source_density_dpi(),
        #[cfg(feature = "androidbox-density-icons7")]
        color_quantized: icon.color_quantized(),
    }))
}

fn validate_durable_package(
    installed: &InstalledPackage,
    apk: &[u8],
) -> Result<InstalledPackageSnapshot, Error> {
    if apk.len() != installed.apk_length() {
        return Err(Error::DurableLengthMismatch);
    }
    let metadata = installed.metadata();
    let apk_digest = sha256(apk);
    if apk_digest != *metadata.apk_sha256() {
        return Err(Error::DurableApkDigestMismatch);
    }

    let signer = verify_apk_v2(apk)?;
    if signer.certificate_sha256 != *metadata.signer_cert_sha256() {
        return Err(Error::DurableSignerMismatch);
    }
    let image = load_android_box(apk)?;
    #[cfg(feature = "androidbox-runtime-install2")]
    {
        // Keep durable recovery/relaunch at the same executable profile as
        // source admission; otherwise Settings could install a package that
        // the AndroidApp worker can never open.
        image.launch_activity_session()?;
    }
    #[cfg(feature = "androidbox-apk-envelope4")]
    let envelope = image
        .envelope_info()
        .ok_or(Error::DurableCompatibilityMismatch)?;
    let manifest = image.manifest_info();
    if manifest.package.as_str() != metadata.package()
        || manifest.activity_descriptor.as_str() != metadata.activity()
        || u64::from(manifest.version_code) != metadata.version_code()
    {
        return Err(Error::DurableManifestMismatch);
    }
    if metadata.compatibility_profile() != CompatibilityProfile::Resources1 {
        return Err(Error::DurableCompatibilityMismatch);
    }

    // This is the launch whose output may be exposed to the product. It is
    // intentionally performed only over the separately read-back APK buffer.
    let launch = image.launch_activity()?;
    let resources = launch.resources.ok_or(Error::Resources1Required)?;
    if transaction_id(apk, signer, &launch) != metadata.transaction_id() {
        return Err(Error::DurableTransactionMismatch);
    }

    Ok(InstalledPackageSnapshot {
        generation: installed.generation(),
        slot: installed.slot(),
        apk_length: installed.apk_length() as u32,
        version_code: metadata.version_code(),
        package: FixedText::try_from_ascii(metadata.package())?,
        activity: FixedText::try_from_ascii(metadata.activity())?,
        signer_cert_sha256: signer.certificate_sha256,
        apk_sha256: apk_digest,
        compatibility_profile: metadata.compatibility_profile(),
        launch_title: FixedText::try_from_ascii(launch.view.title.as_str())?,
        launch_text: FixedText::try_from_ascii(launch.view.text.as_str())?,
        resources_arsc_crc32: resources.resources_arsc_crc32,
        layout_xml_crc32: resources.layout_xml_crc32,
        layout_resource_id: resources.layout_resource_id,
        text_resource_id: resources.text_resource_id,
        launch_constructor_method_index: launch.constructor.method_index,
        launch_constructor_code_offset: launch.constructor.code_offset,
        launch_constructor_instruction_count: launch.constructor_instruction_count,
        launch_on_create_method_index: launch.method.method_index,
        launch_on_create_code_offset: launch.method.code_offset,
        launch_instruction_count: launch.instruction_count,
        #[cfg(feature = "androidbox-icon-resources5")]
        launcher_icon: compact_launcher_icon(image.launcher_icon())?,
        #[cfg(feature = "androidbox-apk-envelope4")]
        envelope,
    })
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, AndroidBoxError> {
    // SAFETY: every package-manager caller is synchronous on the single core,
    // and the returned AndroidBox borrows only `apk`. The catalog loader wipes
    // this bounded workspace, enumerates declarations without granting them,
    // and binds only the first enabled exported launcher Activity.
    let envelope_scratch = unsafe { &mut *ANDROID_ENVELOPE_SCRATCH.value.get() };
    let catalog_scratch = unsafe { &mut *ANDROID_MANIFEST_CATALOG_SCRATCH.value.get() };
    AndroidBox::load_with_manifest_catalog_scratch(apk, envelope_scratch, catalog_scratch)
}

#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-manifest-catalog3")
))]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, AndroidBoxError> {
    // SAFETY: every package-manager caller is synchronous on the single core,
    // and the returned AndroidBox borrows only `apk`. The AndroidBox contract
    // wipes this bounded workspace on every success and failure path.
    let scratch = unsafe { &mut *ANDROID_ENVELOPE_SCRATCH.value.get() };
    AndroidBox::load_with_scratch(apk, scratch)
}

#[cfg(not(feature = "androidbox-apk-envelope4"))]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, AndroidBoxError> {
    AndroidBox::load(apk)
}

/// Stable nonzero identity over the exact APK content and admitted metadata.
fn transaction_id(apk: &[u8], signer: ApkSignerInfo, launch: &ActivityLaunch) -> u64 {
    let manifest = launch.manifest;
    let mut digest = Sha256::new();
    digest.update(b"Bndroid AndroidBox Install-0 transaction v1\0");
    digest.update(&(apk.len() as u64).to_le_bytes());
    digest.update(apk);
    digest.update(&signer.certificate_sha256);
    digest.update(&manifest.version_code.to_le_bytes());
    digest.update(&(manifest.package.as_bytes().len() as u16).to_le_bytes());
    digest.update(manifest.package.as_bytes());
    digest.update(&(manifest.activity_descriptor.as_bytes().len() as u16).to_le_bytes());
    digest.update(manifest.activity_descriptor.as_bytes());
    digest.update(&CompatibilityProfile::Resources1.raw().to_le_bytes());
    let bytes = digest.finalize();
    let mut transaction = [0_u8; 8];
    transaction.copy_from_slice(&bytes[..8]);
    let value = u64::from_le_bytes(transaction);
    if value == 0 { 1 } else { value }
}

/// Runtime package adapter with no storage mutation capability.
///
/// `SectorIo` requires write/flush methods, but these implementations fail
/// closed without calling any storage mutation primitive. The only evidence
/// counters this adapter can increment are reads.
struct RelaunchReadIo {
    clock: PhysicalCounter,
    deadline_budget: u64,
    sector_count: u64,
    evidence: PackageIoEvidence,
}

impl RelaunchReadIo {
    const fn new(sector_count: u64, deadline_budget: u64) -> Self {
        Self {
            clock: PhysicalCounter,
            deadline_budget,
            sector_count,
            evidence: PackageIoEvidence {
                reads: 0,
                writes: 0,
                flushes: 0,
            },
        }
    }

    const fn evidence(&self) -> PackageIoEvidence {
        self.evidence
    }

    fn deadline(&mut self) -> Result<u64, IoError> {
        deadline_after(&mut self.clock, self.deadline_budget).ok_or(IoError::Device)
    }

    fn package_lba(lba: u64) -> bool {
        (PACKAGES_PARTITION_FIRST_LBA..=PACKAGES_PARTITION_LAST_LBA).contains(&lba)
    }
}

impl SectorIo for RelaunchReadIo {
    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
        if lba >= self.sector_count || !Self::package_lba(lba) {
            return Err(IoError::OutOfBounds);
        }
        let deadline = self.deadline()?;
        storage::read_sector_irq(lba, output, &mut self.clock, deadline).map_err(map_read_error)?;
        self.evidence.reads = self.evidence.reads.saturating_add(1);
        Ok(())
    }

    fn write(&mut self, _lba: u64, _input: &Sector) -> Result<(), IoError> {
        Err(IoError::Device)
    }

    fn flush(&mut self) -> Result<(), IoError> {
        Err(IoError::Device)
    }
}

struct PackageIo {
    clock: PhysicalCounter,
    deadline_budget: u64,
    sector_count: u64,
    evidence: PackageIoEvidence,
}

impl PackageIo {
    const fn new(sector_count: u64, deadline_budget: u64) -> Self {
        Self {
            clock: PhysicalCounter,
            deadline_budget,
            sector_count,
            evidence: PackageIoEvidence {
                reads: 0,
                writes: 0,
                flushes: 0,
            },
        }
    }

    const fn evidence(&self) -> PackageIoEvidence {
        self.evidence
    }

    fn deadline(&mut self) -> Result<u64, IoError> {
        deadline_after(&mut self.clock, self.deadline_budget).ok_or(IoError::Device)
    }

    fn package_lba(lba: u64) -> bool {
        (PACKAGES_PARTITION_FIRST_LBA..=PACKAGES_PARTITION_LAST_LBA).contains(&lba)
    }
}

impl SectorIo for PackageIo {
    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
        if lba >= self.sector_count || !Self::package_lba(lba) {
            return Err(IoError::OutOfBounds);
        }
        let deadline = self.deadline()?;
        storage::read_sector_irq(lba, output, &mut self.clock, deadline).map_err(map_read_error)?;
        self.evidence.reads = self.evidence.reads.saturating_add(1);
        Ok(())
    }

    fn write(&mut self, lba: u64, input: &Sector) -> Result<(), IoError> {
        if lba >= self.sector_count || !Self::package_lba(lba) {
            return Err(IoError::OutOfBounds);
        }
        let deadline = self.deadline()?;
        storage::write_packages_sector_irq(lba, input, &mut self.clock, deadline)
            .map_err(map_mutation_error)?;
        self.evidence.writes = self.evidence.writes.saturating_add(1);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        let deadline = self.deadline()?;
        storage::flush_irq(&mut self.clock, deadline).map_err(map_mutation_error)?;
        self.evidence.flushes = self.evidence.flushes.saturating_add(1);
        Ok(())
    }
}

fn map_read_error(error: StorageError) -> IoError {
    match error {
        StorageError::Block(BlockError::SectorOutOfBounds)
        | StorageError::WriteOutsidePackagesPartition => IoError::OutOfBounds,
        StorageError::RecoveryRequired
        | StorageError::InterruptFailure
        | StorageError::SubmittedMutationOutcomeUnknown
        | StorageError::Block(
            BlockError::DeviceNeedsReset
            | BlockError::ResetTimeout
            | BlockError::DeadlineExpired
            | BlockError::DriverStopped,
        ) => IoError::RequiresReset,
        _ => IoError::Device,
    }
}

fn map_mutation_error(error: StorageError) -> IoError {
    match error {
        StorageError::Block(BlockError::SectorOutOfBounds)
        | StorageError::WriteOutsidePackagesPartition => IoError::OutOfBounds,
        StorageError::SubmittedMutationOutcomeUnknown => IoError::OutcomeUnknown,
        StorageError::RecoveryRequired
        | StorageError::InterruptFailure
        | StorageError::Block(
            BlockError::DeviceNeedsReset
            | BlockError::ResetTimeout
            | BlockError::DeadlineExpired
            | BlockError::DriverStopped,
        ) => IoError::RequiresReset,
        _ => IoError::Device,
    }
}
