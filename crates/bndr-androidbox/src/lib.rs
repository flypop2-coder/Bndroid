#![no_std]
#![deny(unsafe_code)]

//! AndroidBox: deliberately small, allocation-free APK/DEX execution profiles.
//!
//! DEX-0 retains two optional, fixed pure-static integer diagnostics for the
//! historical repository fixture. They are not APK admission requirements.
//! ActivityLifecycle-1 parses a binary Android manifest and executes its
//! selected public no-argument constructor followed by `onCreate` against a
//! tiny pure-data framework model. Resources-1
//! additionally admits one resource-backed `setContentView(int)` path through
//! `resources.arsc` and a compiled `TextView` layout. This is not ART, Dalvik,
//! general Android Framework compatibility, JNI, or a general DEX VM.
//! InteractiveActivity-1 adds one fixed-capacity compiled
//! LinearLayout/TextView/Button scene and a standard listener callback path.
//! MultiActionActivity-3 extends that bounded model to several registered
//! buttons and acyclic ID-based callback branches; neither profile is ART or
//! general Android Framework compatibility.

mod dex;
mod digest;
#[cfg(feature = "androidbox-icon-resources5")]
mod icon;
mod manifest;
mod resources;
mod signature;
mod zip;

#[cfg(feature = "androidbox-apk-envelope4")]
use miniz_oxide::inflate::core::DecompressorOxide;

#[cfg(feature = "androidbox-icon-resources5")]
pub use icon::{
    ANDROID_LAUNCHER_ICON_HEIGHT, ANDROID_LAUNCHER_ICON_PIXEL_COUNT, ANDROID_LAUNCHER_ICON_WIDTH,
    AndroidLauncherIcon,
};
pub use resources::{
    ActivityScene, ActivitySceneNode, ApkResources, DEFAULT_ACTIVITY_LAYOUT_ENTRY,
    LayoutOrientation, LayoutSize, LayoutTextReference, MAX_ACTIVITY_SCENE_DEPTH,
    MAX_ACTIVITY_SCENE_NODES, MAX_BINARY_LAYOUT_BYTES, MAX_RESOURCE_STRING_BYTES,
    MAX_RESOURCE_TABLE_BYTES, RESOURCES_ARSC_ENTRY, ResolvedActivityScene, ResolvedTextView,
    ResourceString, ResourceTable, ViewKind, parse_layout_text_reference, resolve_apk_text_view,
};
pub use signature::{
    APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID, APK_SIGNATURE_SCHEME_V2_BLOCK_ID, ApkSignatureError,
    ApkSignerInfo, verify_apk_v2,
};

/// Maximum APK size accepted by DEX-0.
pub const MAX_APK_BYTES: usize = 1024 * 1024;
/// Maximum uncompressed `classes.dex` size accepted by DEX-0.
pub const MAX_DEX_BYTES: usize = 256 * 1024;
/// Maximum uncompressed binary `AndroidManifest.xml` size accepted.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
/// Caller-owned bounded output workspace for one deflated binary XML entry.
///
/// Stored Manifest/layout entries retain their historical larger bounds.
/// Envelope-4 applies this smaller cap only while inflating method-8 XML.
#[cfg(feature = "androidbox-apk-envelope4")]
pub const MAX_ENVELOPE_XML_INFLATED_BYTES: usize = 32 * 1024;
/// Maximum canonical Java package name bytes retained from the manifest.
pub const MAX_PACKAGE_BYTES: usize = 128;
/// Maximum normalized DEX activity descriptor bytes.
pub const MAX_ACTIVITY_DESCRIPTOR_BYTES: usize = 192;
/// Maximum application label bytes retained from the manifest.
pub const MAX_APPLICATION_LABEL_BYTES: usize = 128;
/// Maximum component records retained by ManifestCatalog-3.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub const MAX_MANIFEST_COMPONENTS: usize = 16;
/// Maximum manifest-level requested permission names retained by
/// ManifestCatalog-3.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub const MAX_MANIFEST_PERMISSIONS: usize = 16;
/// Maximum requested permission name bytes retained by ManifestCatalog-3.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub const MAX_PERMISSION_NAME_BYTES: usize = 192;
/// Maximum `TextView` text bytes emitted by the ActivityLifecycle-1 model.
pub const MAX_VIEW_TEXT_BYTES: usize = 128;
/// Maximum distinct `Button` listeners retained by MultiActionActivity-3.
#[cfg(feature = "androidbox-multiaction3")]
pub const MAX_INTERACTIVE_ACTIVITY_CALLBACKS: usize = 4;
/// InteractiveActivity-1 retains exactly one registered `Button`.
#[cfg(not(feature = "androidbox-multiaction3"))]
pub const MAX_INTERACTIVE_ACTIVITY_CALLBACKS: usize = 1;
/// Historical fixture class containing both optional DEX-0 diagnostics.
pub const ENTRY_CLASS: &[u8] = b"Lorg/bndroid/demo/Main;";
/// Historical no-argument diagnostic entry point.
pub const BOOT_METHOD: &[u8] = b"boot";
/// Historical one-integer diagnostic entry point.
pub const ON_TAP_METHOD: &[u8] = b"onTap";

/// Reusable, non-retained workspace for Envelope-4 binary XML inflation.
///
/// `AndroidBox::load_with_scratch` clears this workspace before and after
/// every attempt. It contains both the 32-KiB XML output buffer and inflater
/// state, so small EL0 stacks should keep it in static or other long-lived
/// storage. The returned `AndroidBox<'apk>` borrows only the APK.
#[cfg(feature = "androidbox-apk-envelope4")]
pub struct AndroidBoxEnvelopeScratch {
    bytes: [u8; MAX_ENVELOPE_XML_INFLATED_BYTES],
    decompressor: DecompressorOxide,
}

/// Total caller-owned workspace size, including XML output and inflater state.
#[cfg(feature = "androidbox-apk-envelope4")]
pub const ANDROIDBOX_ENVELOPE_SCRATCH_BYTES: usize =
    core::mem::size_of::<AndroidBoxEnvelopeScratch>();

#[cfg(all(feature = "androidbox-apk-envelope4", target_pointer_width = "64"))]
const _: () = assert!(
    core::mem::size_of::<DecompressorOxide>() == 10_504,
    "re-audit zero initialization when the exact miniz state layout changes"
);

#[cfg(feature = "androidbox-apk-envelope4")]
impl AndroidBoxEnvelopeScratch {
    #[allow(unsafe_code)]
    pub const fn new() -> Self {
        // SAFETY: Cargo pins miniz_oxide exactly to 0.8.9. In that version
        // DecompressorOxide::default() assigns zero to every integer/array
        // field and State::Start has discriminant zero, so the all-zero bit
        // pattern is a valid, initialized decompressor. This in-place const
        // construction avoids placing its roughly 10-KiB state on an EL0
        // stack. The exact pin prevents that audited representation from
        // changing under this code.
        unsafe { core::mem::MaybeUninit::<Self>::zeroed().assume_init() }
    }

    #[allow(unsafe_code)]
    pub fn clear(&mut self) {
        // SAFETY: The same exact-version validity argument as `new` applies.
        // `self` is exclusively borrowed and `write_bytes` covers the entire
        // caller-owned workspace, wiping both decoded XML and inflater state
        // without a large temporary stack value.
        unsafe { core::ptr::write_bytes(self, 0, 1) };
    }

    pub fn wipe(&mut self) {
        self.clear();
    }

    #[inline(always)]
    fn inflate_parts_mut(&mut self) -> (&mut DecompressorOxide, &mut [u8]) {
        self.decompressor.init();
        (&mut self.decompressor, &mut self.bytes)
    }
}

#[cfg(feature = "androidbox-apk-envelope4")]
impl Default for AndroidBoxEnvelopeScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// Caller-owned, non-retained ManifestCatalog-3 parsing state.
///
/// Real manifests can declare many components and permissions, so this
/// workspace deliberately lives outside small EL0 stacks. The catalog loader
/// clears it before and after every attempt and retains only the selected
/// launcher's compact [`ManifestInfo`].
#[cfg(feature = "androidbox-manifest-catalog3")]
pub struct AndroidBoxManifestCatalogScratch {
    state: manifest::CatalogParseState,
}

#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-manifest-catalog3")
))]
struct AndroidBoxManifestCatalogScratch;

#[cfg(feature = "androidbox-manifest-catalog3")]
pub const ANDROIDBOX_MANIFEST_CATALOG_SCRATCH_BYTES: usize =
    core::mem::size_of::<AndroidBoxManifestCatalogScratch>();

#[cfg(feature = "androidbox-manifest-catalog3")]
impl AndroidBoxManifestCatalogScratch {
    pub const fn new() -> Self {
        Self {
            state: manifest::CatalogParseState::new(),
        }
    }

    pub fn clear(&mut self) {
        self.state.clear();
    }

    pub fn wipe(&mut self) {
        self.clear();
    }

    pub fn is_clear(&self) -> bool {
        self.state.is_clear()
    }
}

#[cfg(feature = "androidbox-manifest-catalog3")]
impl Default for AndroidBoxManifestCatalogScratch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "androidbox-apk-envelope4")]
#[derive(Clone, Copy)]
enum EnvelopeManifestProfile {
    Strict,
    #[cfg(feature = "androidbox-manifest-catalog3")]
    Catalog,
}

/// Inspect one already-decoded binary Android Manifest as a bounded component
/// directory.
///
/// This does not verify an APK signature, grant any declared permission, or
/// prove that listed components are executable. Callers performing package
/// admission must verify the enclosing APK first and separately bind a
/// selected Activity to an execution runtime.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub fn inspect_binary_manifest_catalog(bytes: &[u8]) -> Result<ManifestCatalog, Error> {
    manifest::parse_catalog(bytes)
}

/// Inspect the binary Manifest inside a structurally validated APK envelope.
///
/// The Manifest may be STORED or raw-DEFLATE. Caller-owned scratch is wiped
/// before and after every attempt, and the returned catalog owns all retained
/// metadata. APK signature verification remains an explicit caller
/// responsibility.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub fn inspect_apk_manifest_catalog_with_scratch(
    apk: &[u8],
    scratch: &mut AndroidBoxEnvelopeScratch,
) -> Result<ManifestCatalog, Error> {
    scratch.clear();
    let result = (|| {
        let entry = zip::required_envelope_entry(
            apk,
            b"AndroidManifest.xml",
            MAX_MANIFEST_BYTES,
            Error::ZipMissingManifest,
            Error::ZipDuplicateManifest,
            Error::ManifestTooLarge,
        )?;
        let decoded = zip::decode_envelope_entry(entry, scratch)?;
        manifest::parse_catalog(decoded.as_bytes())
    })();
    scratch.clear();
    result
}

/// Structurally validated evidence about an Envelope-4 APK container.
#[cfg(feature = "androidbox-apk-envelope4")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApkEnvelopeInfo {
    manifest_deflated: bool,
    layout_deflated: bool,
    data_descriptor_count: u8,
    unconsumed_deflated_count: u8,
}

#[cfg(feature = "androidbox-apk-envelope4")]
impl ApkEnvelopeInfo {
    pub const fn manifest_deflated(self) -> bool {
        self.manifest_deflated
    }

    pub const fn layout_deflated(self) -> bool {
        self.layout_deflated
    }

    pub const fn data_descriptor_count(self) -> u8 {
        self.data_descriptor_count
    }

    pub const fn unconsumed_deflated_count(self) -> u8 {
        self.unconsumed_deflated_count
    }
}

/// A reason an APK, DEX file, entry point, or execution was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ApkTooLarge,
    ZipEndRecordMissing,
    ZipEndRecordInvalid,
    Zip64Unsupported,
    ZipTooManyEntries,
    ZipCentralDirectoryBounds,
    ZipCentralHeaderInvalid,
    ZipLocalHeaderInvalid,
    ZipEntryBounds,
    ZipNameTooLong,
    ZipExtraInvalid,
    ZipEncrypted,
    ZipDataDescriptorUnsupported,
    ZipDataDescriptorInvalid,
    ZipCompressionUnsupported,
    ZipCompressedDex,
    ZipDeflateInvalid,
    ZipInflatedEntryTooLarge,
    ZipEntryOverlap,
    ZipEntryMismatch,
    ZipCrcMismatch,
    ZipMissingDex,
    ZipDuplicateDex,
    ZipMissingManifest,
    ZipDuplicateManifest,
    ZipCompressedManifest,
    ZipMissingResources,
    ZipDuplicateResources,
    ZipMissingLayout,
    ZipDuplicateLayout,
    ZipMissingLauncherIcon,
    ZipDuplicateLauncherIcon,
    DexTooLarge,
    DexMagic,
    DexHeader,
    DexEndian,
    DexChecksum,
    DexSignature,
    DexBounds,
    DexMap,
    DexString,
    DexClassMissing,
    DexClassDuplicate,
    DexMethodMissing,
    DexMethodDuplicate,
    DexPrototype,
    DexMethodFlags,
    DexNativeOrAbstract,
    DexCode,
    DexExceptionsUnsupported,
    DexRegisterLimit,
    DexInstructionLimit,
    DexUnknownOpcode(u8),
    DexNoReturn,
    ManifestTooLarge,
    ManifestHeader,
    ManifestStringPool,
    ManifestString,
    ManifestResourceMap,
    ManifestUnknownChunk(u16),
    ManifestStructure,
    ManifestUnknownElement,
    ManifestAttribute,
    ManifestPackageMissing,
    ManifestPackageDuplicate,
    ManifestPackageInvalid,
    ManifestVersionCodeMissing,
    ManifestVersionCodeDuplicate,
    ManifestVersionCodeInvalid,
    ManifestApplicationMissing,
    ManifestApplicationDuplicate,
    ManifestActivityMissing,
    ManifestActivityDuplicate,
    ManifestActivityInvalid,
    ManifestActivityNotExported,
    ManifestLauncherInvalid,
    ManifestCatalogTooManyComponents,
    ManifestCatalogTooManyPermissions,
    ManifestCatalogComponentInvalid,
    ManifestCatalogPermissionInvalid,
    ManifestCatalogIntentFilterInvalid,
    ResourceTableTooLarge,
    ResourceTableHeader,
    ResourceStringPool,
    ResourceString,
    ResourceStringTooLong,
    ResourcePackage,
    ResourceType,
    ResourceEntry,
    ResourceIdInvalid,
    ResourceNotFound,
    ResourceValueType,
    ResourceReference,
    ResourceConfigurationAmbiguous,
    LauncherIconTooLarge,
    LauncherIconEntryName,
    LauncherIconPng,
    LayoutTooLarge,
    LayoutEntryName,
    LayoutHeader,
    LayoutStringPool,
    LayoutResourceMap,
    LayoutStructure,
    LayoutUnknownChunk(u16),
    LayoutAttribute,
    LayoutTextViewMissing,
    LayoutTextViewDuplicate,
    DexActivityClassMissing,
    DexActivityClassDuplicate,
    DexActivityClassInvalid,
    DexActivityMethodMissing,
    DexActivityMethodDuplicate,
    DexActivityPrototype,
    DexFrameworkUnknownClass,
    DexFrameworkUnknownMethod,
    DexFrameworkState,
    DexViewTextTooLong,
}

/// Fixed-capacity ASCII text produced without allocation.
///
/// Constructors are intentionally private to preserve the invariant that
/// `as_bytes()` is always valid ASCII and `len <= N`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct BoundedText<const N: usize> {
    bytes: [u8; N],
    len: u16,
}

impl<const N: usize> BoundedText<N> {
    const fn empty() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn from_ascii(bytes: &[u8], too_long: Error, invalid: Error) -> Result<Self, Error> {
        if bytes.len() > N || bytes.len() > usize::from(u16::MAX) {
            return Err(too_long);
        }
        if bytes.is_empty() || !bytes.iter().all(u8::is_ascii) {
            return Err(invalid);
        }
        let mut value = Self::empty();
        value.bytes[..bytes.len()].copy_from_slice(bytes);
        value.len = u16::try_from(bytes.len()).map_err(|_| too_long)?;
        Ok(value)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("BoundedText contains validated ASCII")
    }

    pub const fn len(&self) -> u16 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> core::fmt::Debug for BoundedText<N> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("BoundedText")
            .field(&self.as_str())
            .finish()
    }
}

pub type PackageName = BoundedText<MAX_PACKAGE_BYTES>;
pub type ActivityDescriptor = BoundedText<MAX_ACTIVITY_DESCRIPTOR_BYTES>;
pub type ApplicationLabel = BoundedText<MAX_APPLICATION_LABEL_BYTES>;
pub type ViewText = BoundedText<MAX_VIEW_TEXT_BYTES>;
#[cfg(feature = "androidbox-manifest-catalog3")]
pub type ComponentDescriptor = BoundedText<MAX_ACTIVITY_DESCRIPTOR_BYTES>;
#[cfg(feature = "androidbox-manifest-catalog3")]
pub type PermissionName = BoundedText<MAX_PERMISSION_NAME_BYTES>;

/// Android component kinds enumerated by ManifestCatalog-3.
#[cfg(feature = "androidbox-manifest-catalog3")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestComponentKind {
    Activity,
    ActivityAlias,
    Service,
    Receiver,
    Provider,
}

/// One bounded component declaration from a binary Android Manifest.
///
/// This is package metadata, not proof that the corresponding Java/native
/// implementation can execute. Only the separately selected launcher
/// Activity is bound to the current bounded DEX interpreter.
#[cfg(feature = "androidbox-manifest-catalog3")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestComponent {
    kind: ManifestComponentKind,
    descriptor: ComponentDescriptor,
    alias_target: ComponentDescriptor,
    exported: bool,
    exported_explicit: bool,
    enabled: bool,
    intent_filter_count: u16,
    action_count: u16,
    category_count: u16,
    data_count: u16,
    launcher: bool,
    authority_declared: bool,
    permission_declared: bool,
}

#[cfg(feature = "androidbox-manifest-catalog3")]
impl ManifestComponent {
    const fn empty() -> Self {
        Self {
            kind: ManifestComponentKind::Activity,
            descriptor: ComponentDescriptor::empty(),
            alias_target: ComponentDescriptor::empty(),
            exported: false,
            exported_explicit: false,
            enabled: true,
            intent_filter_count: 0,
            action_count: 0,
            category_count: 0,
            data_count: 0,
            launcher: false,
            authority_declared: false,
            permission_declared: false,
        }
    }

    pub const fn kind(self) -> ManifestComponentKind {
        self.kind
    }

    pub const fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }

    pub const fn alias_target(&self) -> Option<&ComponentDescriptor> {
        if self.alias_target.is_empty() {
            None
        } else {
            Some(&self.alias_target)
        }
    }

    pub const fn exported(self) -> bool {
        self.exported
    }

    pub const fn exported_explicit(self) -> bool {
        self.exported_explicit
    }

    pub const fn enabled(self) -> bool {
        self.enabled
    }

    pub const fn intent_filter_count(self) -> u16 {
        self.intent_filter_count
    }

    pub const fn action_count(self) -> u16 {
        self.action_count
    }

    pub const fn category_count(self) -> u16 {
        self.category_count
    }

    pub const fn data_count(self) -> u16 {
        self.data_count
    }

    pub const fn launcher(self) -> bool {
        self.launcher
    }

    pub const fn authority_declared(self) -> bool {
        self.authority_declared
    }

    pub const fn permission_declared(self) -> bool {
        self.permission_declared
    }
}

/// Fixed-capacity package/component directory parsed from binary
/// `AndroidManifest.xml`.
///
/// It intentionally records declarations without granting permissions,
/// resolving general intents, or claiming that every component is executable.
#[cfg(feature = "androidbox-manifest-catalog3")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestCatalog {
    package: PackageName,
    version_code: u32,
    application_label: ApplicationLabel,
    application_label_resource_id: Option<u32>,
    application_icon_resource_id: Option<u32>,
    components: [ManifestComponent; MAX_MANIFEST_COMPONENTS],
    component_count: u8,
    permissions: [PermissionName; MAX_MANIFEST_PERMISSIONS],
    permission_count: u8,
    launcher_count: u8,
    primary_launcher_index: Option<u8>,
    intent_filter_count: u16,
    action_count: u16,
    category_count: u16,
    data_count: u16,
}

#[cfg(feature = "androidbox-manifest-catalog3")]
impl ManifestCatalog {
    pub const fn package(&self) -> &PackageName {
        &self.package
    }

    pub const fn version_code(self) -> u32 {
        self.version_code
    }

    pub const fn application_label(&self) -> &ApplicationLabel {
        &self.application_label
    }

    pub const fn application_label_resource_id(self) -> Option<u32> {
        self.application_label_resource_id
    }

    pub const fn application_icon_resource_id(self) -> Option<u32> {
        self.application_icon_resource_id
    }

    pub fn components(&self) -> &[ManifestComponent] {
        &self.components[..usize::from(self.component_count)]
    }

    pub fn permissions(&self) -> &[PermissionName] {
        &self.permissions[..usize::from(self.permission_count)]
    }

    pub const fn launcher_count(self) -> u8 {
        self.launcher_count
    }

    pub fn primary_launcher(&self) -> Option<&ManifestComponent> {
        self.primary_launcher_index
            .and_then(|index| self.components().get(usize::from(index)))
    }

    pub const fn intent_filter_count(self) -> u16 {
        self.intent_filter_count
    }

    pub const fn action_count(self) -> u16 {
        self.action_count
    }

    pub const fn category_count(self) -> u16 {
        self.category_count
    }

    pub const fn data_count(self) -> u16 {
        self.data_count
    }
}

/// Identity proven by the APK's binary manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestInfo {
    pub android_manifest_crc32: u32,
    pub package: PackageName,
    /// Unique nonzero `android:versionCode` from the binary manifest root.
    pub version_code: u32,
    pub activity_descriptor: ActivityDescriptor,
    pub application_label: ApplicationLabel,
    /// Resource ID used to resolve `application/@android:label`, or `None`
    /// when the binary manifest carries an inline string.
    pub application_label_resource_id: Option<u32>,
    /// Resource ID used to resolve `application/@android:icon`, or `None`
    /// when the package does not declare a launcher icon.
    pub application_icon_resource_id: Option<u32>,
}

/// Metadata for the manifest-selected ActivityLifecycle-1 constructor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityConstructorInfo {
    pub method_index: u32,
    pub code_offset: u32,
    pub registers: u16,
    pub parameters: u16,
    pub outgoing: u16,
    pub instruction_units: u32,
}

/// Metadata for the manifest-selected ActivityLifecycle-1 `onCreate` method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityMethodInfo {
    pub method_index: u32,
    pub code_offset: u32,
    pub registers: u16,
    pub parameters: u16,
    pub outgoing: u16,
    pub instruction_units: u32,
}

/// Pure data emitted by the ActivityLifecycle-1 framework model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityView {
    pub title: ApplicationLabel,
    pub text: ViewText,
}

/// Exact APK resource evidence used by one Resources-1 Activity launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityResourceInfo {
    pub resources_arsc_crc32: u32,
    pub layout_xml_crc32: u32,
    pub layout_resource_id: u32,
    pub text_resource_id: u32,
}

/// Evidence and output from executing one manifest-selected
/// constructor -> `onCreate` lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityLaunch {
    pub image: ImageInfo,
    pub manifest: ManifestInfo,
    /// Exact launcher `<init>()V` code item interpreted for this lifecycle.
    pub constructor: ActivityConstructorInfo,
    /// Exact launcher `onCreate(Bundle)V` code item interpreted afterward.
    pub method: ActivityMethodInfo,
    pub view: ActivityView,
    /// Present only when `onCreate` selected a compiled APK layout and its
    /// `TextView/android:text` reference was resolved through the same APK.
    pub resources: Option<ActivityResourceInfo>,
    /// Instructions executed by the constructor (exactly two in this profile).
    pub constructor_instruction_count: u16,
    /// Instructions executed by `onCreate`.
    pub instruction_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InteractiveActivityLaunch {
    pub image: ImageInfo,
    pub manifest: ManifestInfo,
    pub constructor: ActivityConstructorInfo,
    pub on_create: ActivityMethodInfo,
    pub on_click: ActivityMethodInfo,
    pub resources_arsc_crc32: u32,
    pub layout_xml_crc32: u32,
    pub layout_resource_id: u32,
    /// First listener, retained for the InteractiveActivity-1 ABI.
    pub listener_button_id: u32,
    /// Complete bounded listener set in layout-registration order.
    pub listener_button_ids: [u32; MAX_INTERACTIVE_ACTIVITY_CALLBACKS],
    pub listener_button_count: u8,
    pub constructor_instruction_count: u16,
    pub on_create_instruction_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityUpdate {
    pub method: ActivityMethodInfo,
    pub clicked_view_id: u32,
    pub changed_view_id: u32,
    pub text_resource_id: u32,
    /// True when `setText(CharSequence)` selected an APK-owned DEX string
    /// rather than resolving an Android resource ID.
    #[cfg(feature = "androidbox-string-text12")]
    pub direct_string_text: bool,
    /// True when the direct text was produced by the admitted bounded
    /// `StringBuilder(String).append(int).toString()` execution path.
    #[cfg(feature = "androidbox-string-builder13")]
    pub dynamic_string_text: bool,
    pub instruction_count: u16,
    #[cfg(feature = "androidbox-dex-methods8")]
    pub app_defined_call_count: u8,
    #[cfg(feature = "androidbox-dex-instance9")]
    pub app_defined_instance_call_count: u8,
    #[cfg(feature = "androidbox-activity-fields10")]
    pub activity_field_read_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    pub activity_field_write_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    pub activity_int_state_value: u8,
    pub revision: u32,
}

/// Immutable evidence about the exact APK and DEX admitted by DEX-0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageInfo {
    pub classes_dex_crc32: u32,
    pub dex_header_checksum: u32,
    pub dex_file_size: u32,
}

/// Immutable metadata for one admitted DEX method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodInfo {
    pub method_index: u32,
    pub code_offset: u32,
    pub registers: u16,
    pub parameters: u16,
    pub instruction_units: u32,
}

/// Result and evidence from one real, bounded DEX code-item execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Execution {
    pub image: ImageInfo,
    pub method: MethodInfo,
    pub result: i32,
    pub instruction_count: u16,
}

#[derive(Clone, Copy)]
// Allocation is forbidden, so the owned resolved scene is intentionally
// retained inline instead of hiding the larger variant behind a box.
#[allow(clippy::large_enum_variant)]
enum CachedActivityLayout {
    Static {
        view: ActivityView,
        evidence: ActivityResourceInfo,
    },
    Interactive(ResolvedActivityScene),
}

/// A validated, manifest-component-driven AndroidBox image borrowed directly
/// from its APK bytes.
#[derive(Clone, Copy)]
pub struct AndroidBox<'a> {
    program: dex::Program<'a>,
    image: ImageInfo,
    manifest: ManifestInfo,
    resources: Option<ApkResources<'a>>,
    cached_layout: Option<CachedActivityLayout>,
    #[cfg(feature = "androidbox-icon-resources5")]
    launcher_icon: Option<AndroidLauncherIcon>,
    #[cfg(feature = "androidbox-apk-envelope4")]
    envelope_info: Option<ApkEnvelopeInfo>,
}

pub struct ActivitySession<'a> {
    // Retain only the executable/resource views needed after launch. Keeping
    // the whole AndroidBox here would duplicate its owned cached scene beside
    // `scene`, needlessly inflating every fixed-capacity session and its EL0
    // construction stack.
    program: dex::Program<'a>,
    resources: ApkResources<'a>,
    scene: ActivityScene,
    listener_view_index: u8,
    listener_button_ids: [u32; MAX_INTERACTIVE_ACTIVITY_CALLBACKS],
    listener_button_count: u8,
    activity_field: Option<dex::ActivityFieldValue>,
    launch: InteractiveActivityLaunch,
    revision: u32,
}

// AndroidApp currently runs on a guarded 64-KiB EL0 stack. Keep one retained
// session below a page so fixed-capacity probe/return values cannot silently
// reintroduce the duplicated cached scene that this representation avoids.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(core::mem::size_of::<ActivitySession<'static>>() <= 4 * 1024);

impl ActivitySession<'_> {
    pub const fn scene(&self) -> &ActivityScene {
        &self.scene
    }

    pub const fn launch_info(&self) -> InteractiveActivityLaunch {
        self.launch
    }

    pub const fn listener_button_id(&self) -> u32 {
        self.launch.listener_button_id
    }

    pub fn listener_button_ids(&self) -> &[u32] {
        &self.listener_button_ids[..usize::from(self.listener_button_count)]
    }

    pub const fn listener_button_count(&self) -> u8 {
        self.listener_button_count
    }

    pub fn is_listener_button(&self, id: u32) -> bool {
        id != 0 && self.listener_button_ids().contains(&id)
    }

    pub const fn revision(&self) -> u32 {
        self.revision
    }

    pub fn dispatch_click(&mut self, view_id: u32) -> Result<ActivityUpdate, Error> {
        let mutation = self.program.dispatch_click(
            &self.scene,
            self.listener_view_index,
            self.activity_field,
            view_id,
        )?;
        #[cfg(feature = "androidbox-string-text12")]
        let (text, text_resource_id, direct_string_text) =
            if let Some(text) = mutation.text.direct_text() {
                (text, 0, true)
            } else {
                let resource_id = mutation.text.resource_id();
                (
                    self.resources.resolve_string(resource_id)?,
                    resource_id,
                    false,
                )
            };
        #[cfg(not(feature = "androidbox-string-text12"))]
        let (text, text_resource_id) = {
            let resource_id = mutation.text.resource_id();
            (self.resources.resolve_string(resource_id)?, resource_id)
        };
        let mut next_scene = self.scene;
        let target = next_scene
            .node_mut(mutation.target_view_index)
            .ok_or(Error::DexFrameworkState)?;
        if target.kind != ViewKind::TextView || target.id != mutation.target_view_id {
            return Err(Error::DexFrameworkState);
        }
        target.text = text;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::DexInstructionLimit)?;
        self.scene = next_scene;
        self.activity_field = mutation.activity_field;
        self.revision = revision;
        Ok(ActivityUpdate {
            method: mutation.method,
            clicked_view_id: view_id,
            changed_view_id: mutation.target_view_id,
            text_resource_id,
            #[cfg(feature = "androidbox-string-text12")]
            direct_string_text,
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic_string_text: mutation.dynamic_string_text,
            instruction_count: mutation.instruction_count,
            #[cfg(feature = "androidbox-dex-methods8")]
            app_defined_call_count: mutation.app_defined_call_count,
            #[cfg(feature = "androidbox-dex-instance9")]
            app_defined_instance_call_count: mutation.app_defined_instance_call_count,
            #[cfg(feature = "androidbox-activity-fields10")]
            activity_field_read_count: mutation.activity_field_read_count,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_field_write_count: mutation.activity_field_write_count,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_int_state_value: mutation.activity_int_state_value,
            revision,
        })
    }
}

impl<'a> AndroidBox<'a> {
    /// Validate a bounded APK, its binary manifest, and its single stored
    /// `classes.dex`, then bind the unique exported `MAIN`/`LAUNCHER` component
    /// to its exact DEX Activity class, public no-argument constructor, and
    /// `onCreate` method.
    ///
    /// The historical `Lorg/bndroid/demo/Main;` boot/onTap diagnostics are
    /// discovered when present, but never participate in component admission.
    pub fn load(apk: &'a [u8]) -> Result<Self, Error> {
        let entries = zip::apk_entries(apk)?;
        let mut manifest = manifest::parse(entries.manifest.bytes)?;
        manifest.android_manifest_crc32 = entries.manifest.crc32;
        let program =
            dex::Program::load(entries.dex.bytes, manifest.activity_descriptor.as_bytes())?;
        let activity_content = program.admitted_activity_content();
        let needs_resources = manifest.application_label_resource_id.is_some()
            || matches!(activity_content, dex::ActivityContent::LayoutResource(_));
        let resources = if needs_resources {
            Some(ApkResources::load(apk)?)
        } else {
            None
        };
        if let Some(resource_id) = manifest.application_label_resource_id {
            let label = resources
                .ok_or(Error::ZipMissingResources)?
                .resolve_string(resource_id)?;
            manifest.application_label = ApplicationLabel::from_ascii(
                label.as_bytes(),
                Error::ResourceStringTooLong,
                Error::ResourceString,
            )?;
        }
        let cached_layout =
            if let dex::ActivityContent::LayoutResource(layout_resource_id) = activity_content {
                let resources = resources.ok_or(Error::ZipMissingResources)?;
                if program.is_interactive() {
                    let layout_entry = resources.resolve_layout_entry(layout_resource_id)?;
                    let resolved = resources.resolve_activity_scene(layout_entry.as_bytes())?;
                    validate_interactive_layout(
                        program,
                        resources,
                        layout_resource_id,
                        resolved.scene,
                    )?;
                    Some(CachedActivityLayout::Interactive(resolved))
                } else {
                    let (view, evidence) = resolve_resource_activity(
                        manifest.application_label,
                        resources,
                        layout_resource_id,
                    )?;
                    Some(CachedActivityLayout::Static { view, evidence })
                }
            } else {
                None
            };
        Ok(Self {
            program,
            image: ImageInfo {
                classes_dex_crc32: entries.dex.crc32,
                dex_header_checksum: program.header_checksum(),
                dex_file_size: program.file_size(),
            },
            manifest,
            resources,
            cached_layout,
            #[cfg(feature = "androidbox-icon-resources5")]
            launcher_icon: None,
            #[cfg(feature = "androidbox-apk-envelope4")]
            envelope_info: None,
        })
    }

    /// Load the Envelope-4 APK profile using caller-owned binary-XML scratch.
    ///
    /// The returned image borrows only `apk`; all Manifest/layout results are
    /// parsed into owned bounded values before this function returns. The
    /// scratch bytes are wiped on both success and failure.
    #[cfg(feature = "androidbox-apk-envelope4")]
    pub fn load_with_scratch(
        apk: &'a [u8],
        scratch: &mut AndroidBoxEnvelopeScratch,
    ) -> Result<Self, Error> {
        scratch.clear();
        let result = Self::load_envelope_inner(apk, scratch, EnvelopeManifestProfile::Strict, None);
        scratch.clear();
        result
    }

    /// Load a real APK envelope through ManifestCatalog-3, then bind the first
    /// enabled exported launcher declaration to the bounded Activity runtime.
    ///
    /// Multiple Activities, aliases, services, receivers, providers,
    /// permissions, and ordinary intent filters may coexist in the Manifest.
    /// Only the selected Activity (or an alias's target Activity) is executed;
    /// every other declaration remains inert package metadata. As with the
    /// other loaders, package admission must verify the APK signature before
    /// calling this execution-stage API.
    #[cfg(feature = "androidbox-manifest-catalog3")]
    pub fn load_with_manifest_catalog_scratch(
        apk: &'a [u8],
        envelope_scratch: &mut AndroidBoxEnvelopeScratch,
        catalog_scratch: &mut AndroidBoxManifestCatalogScratch,
    ) -> Result<Self, Error> {
        envelope_scratch.clear();
        catalog_scratch.clear();
        let result = Self::load_envelope_inner(
            apk,
            envelope_scratch,
            EnvelopeManifestProfile::Catalog,
            Some(catalog_scratch),
        );
        catalog_scratch.clear();
        envelope_scratch.clear();
        result
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn load_envelope_inner(
        apk: &'a [u8],
        scratch: &mut AndroidBoxEnvelopeScratch,
        manifest_profile: EnvelopeManifestProfile,
        catalog_scratch: Option<&mut AndroidBoxManifestCatalogScratch>,
    ) -> Result<Self, Error> {
        #[cfg(not(feature = "androidbox-manifest-catalog3"))]
        let _ = catalog_scratch;
        let entries = zip::apk_entries_envelope(apk)?;
        let manifest_deflated = entries.manifest.is_deflated();
        let mut unconsumed_deflated_count = entries
            .deflated_entry_count
            .checked_sub(u8::from(manifest_deflated))
            .ok_or(Error::ZipEntryMismatch)?;
        let mut manifest = {
            let decoded = zip::decode_envelope_entry(entries.manifest, scratch)?;
            match manifest_profile {
                EnvelopeManifestProfile::Strict => manifest::parse(decoded.as_bytes())?,
                #[cfg(feature = "androidbox-manifest-catalog3")]
                EnvelopeManifestProfile::Catalog => {
                    let workspace =
                        catalog_scratch.ok_or(Error::ManifestCatalogComponentInvalid)?;
                    manifest::parse_catalog_into(decoded.as_bytes(), &mut workspace.state)?;
                    manifest::selected_manifest_info(&workspace.state, entries.manifest.crc32())?
                }
            }
        };
        manifest.android_manifest_crc32 = entries.manifest.crc32();
        let program =
            dex::Program::load(entries.dex.bytes, manifest.activity_descriptor.as_bytes())?;
        let activity_content = program.admitted_activity_content();
        let needs_resources = manifest.application_label_resource_id.is_some()
            || manifest.application_icon_resource_id.is_some()
            || matches!(activity_content, dex::ActivityContent::LayoutResource(_));
        let resources = if needs_resources {
            Some(ApkResources::load_envelope(apk)?)
        } else {
            None
        };
        if let Some(resource_id) = manifest.application_label_resource_id {
            let label = resources
                .ok_or(Error::ZipMissingResources)?
                .resolve_string(resource_id)?;
            manifest.application_label = ApplicationLabel::from_ascii(
                label.as_bytes(),
                Error::ResourceStringTooLong,
                Error::ResourceString,
            )?;
        }
        #[cfg(feature = "androidbox-icon-resources5")]
        let launcher_icon = match manifest.application_icon_resource_id {
            Some(resource_id) => Some(
                resources
                    .ok_or(Error::ZipMissingResources)?
                    .resolve_launcher_icon(resource_id, scratch)?,
            ),
            None => None,
        };

        let mut layout_deflated = false;
        let cached_layout =
            if let dex::ActivityContent::LayoutResource(layout_resource_id) = activity_content {
                let resources = resources.ok_or(Error::ZipMissingResources)?;
                let layout_entry = resources.resolve_layout_entry(layout_resource_id)?;
                if program.is_interactive() {
                    let (resolved, was_deflated) = resources
                        .resolve_activity_scene_envelope(layout_entry.as_bytes(), scratch)?;
                    layout_deflated = was_deflated;
                    validate_interactive_layout(
                        program,
                        resources,
                        layout_resource_id,
                        resolved.scene,
                    )?;
                    Some(CachedActivityLayout::Interactive(resolved))
                } else {
                    let (resolved, was_deflated) =
                        resources.resolve_text_view_envelope(layout_entry.as_bytes(), scratch)?;
                    layout_deflated = was_deflated;
                    let text = ViewText::from_ascii(
                        resolved.text.as_bytes(),
                        Error::DexViewTextTooLong,
                        Error::ResourceString,
                    )?;
                    Some(CachedActivityLayout::Static {
                        view: ActivityView {
                            title: manifest.application_label,
                            text,
                        },
                        evidence: ActivityResourceInfo {
                            resources_arsc_crc32: resolved.resources_arsc_crc32,
                            layout_xml_crc32: resolved.layout_crc32,
                            layout_resource_id,
                            text_resource_id: resolved.reference.resource_id,
                        },
                    })
                }
            } else {
                None
            };
        if layout_deflated {
            unconsumed_deflated_count = unconsumed_deflated_count
                .checked_sub(1)
                .ok_or(Error::ZipEntryMismatch)?;
        }

        Ok(Self {
            program,
            image: ImageInfo {
                classes_dex_crc32: entries.dex.crc32,
                dex_header_checksum: program.header_checksum(),
                dex_file_size: program.file_size(),
            },
            manifest,
            resources,
            cached_layout,
            #[cfg(feature = "androidbox-icon-resources5")]
            launcher_icon,
            envelope_info: Some(ApkEnvelopeInfo {
                manifest_deflated,
                layout_deflated,
                data_descriptor_count: entries.data_descriptor_count,
                unconsumed_deflated_count,
            }),
        })
    }

    pub const fn image_info(&self) -> ImageInfo {
        self.image
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    pub const fn envelope_info(&self) -> Option<ApkEnvelopeInfo> {
        self.envelope_info
    }

    /// Return metadata for the optional historical DEX-0 boot diagnostic.
    pub const fn boot_method(&self) -> Result<MethodInfo, Error> {
        self.program.boot_info()
    }

    /// Return metadata for the optional historical DEX-0 tap diagnostic.
    pub const fn on_tap_method(&self) -> Result<MethodInfo, Error> {
        self.program.on_tap_info()
    }

    pub const fn manifest_info(&self) -> ManifestInfo {
        self.manifest
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    pub const fn launcher_icon(&self) -> Option<AndroidLauncherIcon> {
        self.launcher_icon
    }

    pub const fn activity_method(&self) -> ActivityMethodInfo {
        self.program.activity_info()
    }

    pub const fn activity_on_click_method(&self) -> Option<ActivityMethodInfo> {
        self.program.on_click_info()
    }

    /// Return metadata for the required manifest Activity constructor.
    pub const fn activity_constructor(&self) -> ActivityConstructorInfo {
        self.program.activity_constructor_info()
    }

    /// Execute the optional historical
    /// `Lorg/bndroid/demo/Main;->boot()I` diagnostic.
    pub fn boot(&self) -> Result<Execution, Error> {
        let (result, instruction_count) = self.program.execute_boot()?;
        Ok(Execution {
            image: self.image,
            method: self.program.boot_info()?,
            result,
            instruction_count,
        })
    }

    /// Execute the optional historical
    /// `Lorg/bndroid/demo/Main;->onTap(I)I` diagnostic.
    pub fn on_tap(&self, input: i32) -> Result<Execution, Error> {
        let (result, instruction_count) = self.program.execute_on_tap(input)?;
        Ok(Execution {
            image: self.image,
            method: self.program.on_tap_info()?,
            result,
            instruction_count,
        })
    }

    /// Execute one fresh manifest-selected Activity constructor followed by
    /// its `onCreate(Landroid/os/Bundle;)V` against the fail-closed
    /// ActivityLifecycle-1 framework model.
    pub fn launch_activity(&self) -> Result<ActivityLaunch, Error> {
        if self.program.is_interactive() {
            let session = self.launch_activity_session()?;
            let text_node = session
                .scene()
                .nodes()
                .iter()
                .find(|node| node.kind == ViewKind::TextView)
                .ok_or(Error::LayoutTextViewMissing)?;
            let text = ViewText::from_ascii(
                text_node.text.as_bytes(),
                Error::DexViewTextTooLong,
                Error::ResourceString,
            )?;
            let launch = session.launch_info();
            return Ok(ActivityLaunch {
                image: self.image,
                manifest: self.manifest,
                constructor: launch.constructor,
                method: launch.on_create,
                view: ActivityView {
                    title: self.manifest.application_label,
                    text,
                },
                resources: Some(ActivityResourceInfo {
                    resources_arsc_crc32: launch.resources_arsc_crc32,
                    layout_xml_crc32: launch.layout_xml_crc32,
                    layout_resource_id: launch.layout_resource_id,
                    text_resource_id: text_node.text_resource_id,
                }),
                constructor_instruction_count: launch.constructor_instruction_count,
                instruction_count: launch.on_create_instruction_count,
            });
        }
        let (content, constructor_instruction_count, instruction_count) =
            self.program.execute_activity()?;
        let (view, resources) = match content {
            dex::ActivityContent::InlineText(text) => (
                ActivityView {
                    title: self.manifest.application_label,
                    text,
                },
                None,
            ),
            dex::ActivityContent::LayoutResource(layout_resource_id) => match self.cached_layout {
                Some(CachedActivityLayout::Static { view, evidence })
                    if evidence.layout_resource_id == layout_resource_id =>
                {
                    (view, Some(evidence))
                }
                _ => return Err(Error::DexFrameworkState),
            },
        };
        Ok(ActivityLaunch {
            image: self.image,
            manifest: self.manifest,
            constructor: self.program.activity_constructor_info(),
            method: self.program.activity_info(),
            view,
            resources,
            constructor_instruction_count,
            instruction_count,
        })
    }

    pub fn launch_activity_session(&self) -> Result<ActivitySession<'a>, Error> {
        if !self.program.is_interactive() {
            return Err(Error::DexActivityMethodMissing);
        }
        let layout_resource_id = match self.program.admitted_activity_content() {
            dex::ActivityContent::LayoutResource(id) => id,
            dex::ActivityContent::InlineText(_) => return Err(Error::DexFrameworkState),
        };
        let resolved = match self.cached_layout {
            Some(CachedActivityLayout::Interactive(resolved)) => resolved,
            _ => return Err(Error::DexFrameworkState),
        };
        let executed = self.program.launch_interactive(resolved.scene)?;
        if executed.layout_resource_id != layout_resource_id {
            return Err(Error::DexFrameworkState);
        }
        let on_click = self
            .program
            .on_click_info()
            .ok_or(Error::DexActivityMethodMissing)?;
        let resources = self.resources.ok_or(Error::ZipMissingResources)?;
        Ok(ActivitySession {
            program: self.program,
            resources,
            scene: executed.scene,
            listener_view_index: executed.listener_view_index,
            listener_button_ids: executed.listener_view_ids,
            listener_button_count: executed.listener_count,
            activity_field: executed.activity_field,
            launch: InteractiveActivityLaunch {
                image: self.image,
                manifest: self.manifest,
                constructor: self.program.activity_constructor_info(),
                on_create: self.program.activity_info(),
                on_click,
                resources_arsc_crc32: resolved.resources_arsc_crc32,
                layout_xml_crc32: resolved.layout_crc32,
                layout_resource_id,
                listener_button_id: executed.listener_view_id,
                listener_button_ids: executed.listener_view_ids,
                listener_button_count: executed.listener_count,
                constructor_instruction_count: executed.constructor_instruction_count,
                on_create_instruction_count: executed.on_create_instruction_count,
            },
            revision: 0,
        })
    }
}

fn validate_interactive_layout(
    program: dex::Program<'_>,
    resources: ApkResources<'_>,
    layout_resource_id: u32,
    scene: ActivityScene,
) -> Result<(), Error> {
    let launch = program.launch_interactive(scene)?;
    if launch.layout_resource_id != layout_resource_id {
        return Err(Error::DexFrameworkState);
    }
    for listener_id in &launch.listener_view_ids[..usize::from(launch.listener_count)] {
        let mutation = program.dispatch_click(
            &scene,
            launch.listener_view_index,
            launch.activity_field,
            *listener_id,
        )?;
        #[cfg(feature = "androidbox-string-text12")]
        if mutation.text.direct_text().is_none() {
            resources.resolve_string(mutation.text.resource_id())?;
        }
        #[cfg(not(feature = "androidbox-string-text12"))]
        {
            resources.resolve_string(mutation.text.resource_id())?;
        }
        let target = scene
            .node(mutation.target_view_index)
            .ok_or(Error::DexFrameworkState)?;
        if target.kind != ViewKind::TextView || target.id != mutation.target_view_id {
            return Err(Error::DexFrameworkState);
        }
    }
    Ok(())
}

fn resolve_resource_activity(
    title: ApplicationLabel,
    resources: ApkResources<'_>,
    layout_resource_id: u32,
) -> Result<(ActivityView, ActivityResourceInfo), Error> {
    let layout_entry = resources.resolve_layout_entry(layout_resource_id)?;
    let resolved = resources.resolve_text_view(layout_entry.as_bytes())?;
    let text = ViewText::from_ascii(
        resolved.text.as_bytes(),
        Error::DexViewTextTooLong,
        Error::ResourceString,
    )?;
    Ok((
        ActivityView { title, text },
        ActivityResourceInfo {
            resources_arsc_crc32: resolved.resources_arsc_crc32,
            layout_xml_crc32: resolved.layout_crc32,
            layout_resource_id,
            text_resource_id: resolved.reference.resource_id,
        },
    ))
}

/// Load an APK and execute its optional historical `boot()I` diagnostic.
pub fn boot(apk: &[u8]) -> Result<Execution, Error> {
    AndroidBox::load(apk)?.boot()
}

/// Load an APK and execute its optional historical `onTap(I)I` diagnostic.
pub fn on_tap(apk: &[u8], input: i32) -> Result<Execution, Error> {
    AndroidBox::load(apk)?.on_tap(input)
}

/// Load an APK and launch its unique exported `MAIN`/`LAUNCHER` activity
/// without allocation or Bndroid capabilities.
pub fn launch_activity(apk: &[u8]) -> Result<ActivityLaunch, Error> {
    AndroidBox::load(apk)?.launch_activity()
}

pub fn launch_activity_session(apk: &[u8]) -> Result<ActivitySession<'_>, Error> {
    AndroidBox::load(apk)?.launch_activity_session()
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::ops::Range;

    use super::digest::{adler32, crc32, sha1};
    #[cfg(feature = "androidbox-apk-envelope4")]
    use super::{
        ANDROIDBOX_ENVELOPE_SCRATCH_BYTES, AndroidBoxEnvelopeScratch,
        MAX_ENVELOPE_XML_INFLATED_BYTES,
    };
    #[cfg(feature = "androidbox-manifest-catalog3")]
    use super::{
        ANDROIDBOX_MANIFEST_CATALOG_SCRATCH_BYTES, AndroidBoxManifestCatalogScratch,
        ManifestComponentKind, inspect_apk_manifest_catalog_with_scratch,
        inspect_binary_manifest_catalog,
    };
    use super::{
        AndroidBox, DEFAULT_ACTIVITY_LAYOUT_ENTRY, ENTRY_CLASS, Error, LayoutOrientation,
        LayoutSize, ViewKind, boot, launch_activity, launch_activity_session, on_tap,
        verify_apk_v2,
    };

    const DEMO_APK: &[u8] = include_bytes!("../../../fixtures/androidbox-demo/androidbox-demo.apk");
    const COMPONENT_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-mac-demo/androidbox-mac-demo.apk");
    const INTERACTIVE_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-interactive-demo/androidbox-interactive-demo.apk"
    );
    const PROFILE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-profile-demo/androidbox-profile-demo.apk");
    const MULTIACTION_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-multiaction-demo/androidbox-multiaction-demo.apk"
    );
    #[cfg(feature = "androidbox-apk-envelope4")]
    const ENVELOPE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-envelope-demo/androidbox-envelope-demo.apk");
    #[cfg(feature = "androidbox-manifest-catalog3")]
    const MANIFEST_CATALOG_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-manifest-catalog-demo/androidbox-manifest-catalog-demo.apk"
    );

    #[test]
    fn real_d8_apk_executes_both_fixed_entries() {
        let image = AndroidBox::load(DEMO_APK).expect("generated APK must validate");
        let boot_result = image.boot().expect("boot code_item must execute");
        assert_eq!(boot_result.result, 20_260_729);
        assert_eq!(boot_result.instruction_count, 2);
        assert_eq!(boot_result.image.classes_dex_crc32, 0x27e1_618c);
        assert_eq!(boot_result.image.dex_header_checksum, 0xac51_d7c3);
        assert_eq!(boot_result.image.dex_file_size, 1_312);
        assert_eq!(boot_result.method.method_index, 6);
        assert_eq!(boot_result.method.code_offset, 0x228);
        assert_eq!(boot_result.method.registers, 1);
        assert_eq!(boot_result.method.parameters, 0);
        assert_eq!(boot_result.method.instruction_units, 4);
        assert_eq!(boot_result.image, image.image_info());
        assert_eq!(
            boot_result.method,
            image.boot_method().expect("boot metadata")
        );

        let tap_result = image.on_tap(35).expect("onTap code_item must execute");
        assert_eq!(tap_result.result, 42);
        assert_eq!(tap_result.instruction_count, 2);
        assert_eq!(tap_result.method.method_index, 7);
        assert_eq!(tap_result.method.code_offset, 0x240);
        assert_eq!(tap_result.method.registers, 1);
        assert_eq!(tap_result.method.parameters, 1);
        assert_eq!(tap_result.method.instruction_units, 3);
        assert_eq!(
            tap_result.method,
            image.on_tap_method().expect("tap metadata")
        );
    }

    #[test]
    fn real_binary_manifest_selects_and_launches_real_d8_activity() {
        let image = AndroidBox::load(DEMO_APK).expect("generated APK must validate");
        let manifest = image.manifest_info();
        assert_eq!(manifest.android_manifest_crc32, 0xc376_5412);
        assert_eq!(manifest.package.as_bytes(), b"org.bndroid.demo");
        assert_eq!(manifest.version_code, 1);
        assert_eq!(
            manifest.activity_descriptor.as_bytes(),
            b"Lorg/bndroid/demo/MainActivity;"
        );
        assert_eq!(
            manifest.application_label.as_bytes(),
            b"AndroidBox DEX-0 Demo"
        );

        let method = image.activity_method();
        assert_eq!(method.method_index, 9);
        assert_eq!(method.code_offset, 0x1f4);
        assert_eq!(method.registers, 3);
        assert_eq!(method.parameters, 2);
        assert_eq!(method.outgoing, 2);
        assert_eq!(method.instruction_units, 17);

        let constructor = image.activity_constructor();
        assert_eq!(constructor.method_index, 8);
        assert_eq!(constructor.code_offset, 0x1dc);
        assert_eq!(constructor.registers, 1);
        assert_eq!(constructor.parameters, 1);
        assert_eq!(constructor.outgoing, 1);
        assert_eq!(constructor.instruction_units, 4);

        let launched = image
            .launch_activity()
            .expect("real constructor -> onCreate lifecycle must execute");
        assert_eq!(launched.image, image.image_info());
        assert_eq!(launched.manifest, manifest);
        assert_eq!(launched.constructor, constructor);
        assert_eq!(launched.method, method);
        assert_eq!(launched.view.title.as_bytes(), b"AndroidBox DEX-0 Demo");
        assert_eq!(launched.view.text.as_bytes(), b"AndroidBox DEX-0 fixture");
        assert_eq!(launched.constructor_instruction_count, 2);
        assert_eq!(launched.instruction_count, 7);

        let relaunched = image
            .launch_activity()
            .expect("each launch must execute one fresh bounded lifecycle");
        assert_eq!(relaunched.constructor, constructor);
        assert_eq!(relaunched.constructor_instruction_count, 2);
        assert_eq!(relaunched.method, method);
        assert_eq!(relaunched.instruction_count, 7);
    }

    #[test]
    fn manifest_component_launches_without_repository_specific_dex0_probes() {
        let image = AndroidBox::load(COMPONENT_APK).expect("component-only APK must validate");
        let manifest = image.manifest_info();
        assert_eq!(manifest.package.as_bytes(), b"org.bndroid.macdemo");
        assert_eq!(
            manifest.activity_descriptor.as_bytes(),
            b"Lorg/bndroid/macdemo/MainActivity;"
        );
        assert_eq!(
            manifest.application_label.as_bytes(),
            b"Mac-built Android app"
        );
        assert_eq!(image.boot_method(), Err(Error::DexClassMissing));
        assert_eq!(image.on_tap_method(), Err(Error::DexClassMissing));
        assert_eq!(image.boot(), Err(Error::DexClassMissing));
        assert_eq!(image.on_tap(7), Err(Error::DexClassMissing));

        let constructor = image.activity_constructor();
        assert_eq!(constructor.code_offset, 0x24c);
        assert_eq!(constructor.registers, 1);
        assert_eq!(constructor.parameters, 1);
        assert_eq!(constructor.outgoing, 1);
        assert_eq!(constructor.instruction_units, 4);

        let launched = image
            .launch_activity()
            .expect("manifest-selected constructor -> onCreate must execute");
        assert_eq!(launched.manifest, manifest);
        assert_eq!(launched.constructor, constructor);
        assert_eq!(launched.constructor_instruction_count, 2);
        assert_eq!(launched.view.text.as_bytes(), b"Hello from a Mac-built APK");
        assert_eq!(launched.instruction_count, 4);
        let resources = launched.resources.expect("Resources-1 evidence");
        assert_eq!(resources.layout_resource_id, 0x7f02_0000);
        assert_eq!(resources.text_resource_id, 0x7f03_0000);
    }

    #[test]
    fn real_standard_button_listener_dispatches_exact_on_click_transactionally() {
        assert_ne!(
            verify_apk_v2(INTERACTIVE_APK)
                .expect("v2-only interactive fixture")
                .certificate_sha256,
            [0; 32]
        );
        let image = AndroidBox::load(INTERACTIVE_APK).expect("interactive APK admission");
        assert_eq!(
            image.manifest_info().activity_descriptor.as_str(),
            "Lorg/bndroid/interactive/MainActivity;"
        );
        let callback = image
            .activity_on_click_method()
            .expect("retained onClick(View)");
        assert_eq!(callback.code_offset, 0x2f8);
        assert_eq!(callback.registers, 3);
        assert_eq!(callback.parameters, 2);
        assert_eq!(callback.outgoing, 2);
        assert_eq!(callback.instruction_units, 16);

        let mut session = image
            .launch_activity_session()
            .expect("constructor/onCreate/listener binding");
        let launch = session.launch_info();
        assert_eq!(launch.layout_resource_id, 0x7f02_0000);
        assert_eq!(launch.listener_button_id, 0x7f01_0000);
        assert_eq!(launch.constructor_instruction_count, 2);
        assert_eq!(launch.on_create_instruction_count, 8);
        assert_eq!(launch.on_click, callback);
        assert_eq!(session.revision(), 0);
        assert_eq!(session.scene().len(), 3);
        assert_eq!(session.scene().nodes()[0].kind, ViewKind::LinearLayout);
        assert_eq!(session.scene().nodes()[1].kind, ViewKind::TextView);
        assert_eq!(
            session.scene().nodes()[1].text.as_str(),
            "Ready for a real APK click"
        );
        assert_eq!(session.scene().nodes()[2].kind, ViewKind::Button);
        assert_eq!(session.scene().nodes()[2].text.as_str(), "Update text");

        let before = *session.scene();
        assert_eq!(
            session.dispatch_click(0x7f01_0001),
            Err(Error::DexFrameworkState)
        );
        assert_eq!(*session.scene(), before);
        assert_eq!(session.revision(), 0);

        let update = session
            .dispatch_click(0x7f01_0000)
            .expect("real Button callback");
        assert_eq!(update.method, callback);
        assert_eq!(update.clicked_view_id, 0x7f01_0000);
        assert_eq!(update.changed_view_id, 0x7f01_0001);
        assert_eq!(update.text_resource_id, 0x7f03_0003);
        assert_eq!(update.instruction_count, 7);
        assert_eq!(update.revision, 1);
        assert_eq!(
            session.scene().nodes()[1].text.as_str(),
            "Button callback executed"
        );
        assert_eq!(session.scene().nodes()[2].text.as_str(), "Update text");

        let repeated = session
            .dispatch_click(0x7f01_0000)
            .expect("deterministic repeated callback");
        assert_eq!(repeated.revision, 2);
        assert_eq!(
            session.scene().nodes()[1].text.as_str(),
            "Button callback executed"
        );
    }

    #[test]
    fn profile_apk_preserves_nested_five_node_scene_and_status_only_callback() {
        assert_ne!(
            verify_apk_v2(PROFILE_APK)
                .expect("v2-only profile fixture")
                .certificate_sha256,
            [0; 32]
        );
        let image = AndroidBox::load(PROFILE_APK).expect("profile APK admission");
        let manifest = image.manifest_info();
        assert_eq!(manifest.package.as_str(), "org.bndroid.profile");
        assert_eq!(manifest.version_code, 1);
        assert_eq!(
            manifest.activity_descriptor.as_str(),
            "Lorg/bndroid/profile/MainActivity;"
        );
        assert_eq!(manifest.application_label.as_str(), "Profile Android app");
        assert_eq!(image.boot(), Err(Error::DexClassMissing));
        assert_eq!(image.on_tap(1), Err(Error::DexClassMissing));

        let callback = image
            .activity_on_click_method()
            .expect("profile onClick(View)");
        let mut session = image
            .launch_activity_session()
            .expect("profile constructor/onCreate/listener binding");
        let launch = session.launch_info();
        assert_eq!(launch.layout_resource_id, 0x7f02_0000);
        assert_eq!(launch.listener_button_id, 0x7f01_0000);
        assert_eq!(launch.constructor_instruction_count, 2);
        assert_eq!(launch.on_create_instruction_count, 8);
        assert_eq!(launch.on_click, callback);
        assert_eq!(session.listener_button_id(), 0x7f01_0000);
        assert_eq!(session.revision(), 0);

        let nodes = session.scene().nodes();
        assert_eq!(nodes.len(), 5);

        assert_eq!(nodes[0].kind, ViewKind::LinearLayout);
        assert_eq!(nodes[0].parent, None);
        assert_eq!(nodes[0].id, 0);
        assert_eq!(nodes[0].width, LayoutSize::MatchParent);
        assert_eq!(nodes[0].height, LayoutSize::MatchParent);
        assert_eq!(nodes[0].orientation, LayoutOrientation::Vertical);
        assert_eq!(nodes[0].text_resource_id, 0);
        assert!(nodes[0].text.is_empty());

        assert_eq!(nodes[1].kind, ViewKind::TextView);
        assert_eq!(nodes[1].parent, Some(0));
        assert_eq!(nodes[1].id, 0x7f01_0002);
        assert_eq!(nodes[1].width, LayoutSize::MatchParent);
        assert_eq!(nodes[1].height, LayoutSize::WrapContent);
        assert_eq!(nodes[1].orientation, LayoutOrientation::None);
        assert_eq!(nodes[1].text_resource_id, 0x7f03_0004);
        assert_eq!(nodes[1].text.as_str(), "Account profile");

        assert_eq!(nodes[2].kind, ViewKind::LinearLayout);
        assert_eq!(nodes[2].parent, Some(0));
        assert_eq!(nodes[2].id, 0);
        assert_eq!(nodes[2].width, LayoutSize::MatchParent);
        assert_eq!(nodes[2].height, LayoutSize::WrapContent);
        assert_eq!(nodes[2].orientation, LayoutOrientation::Vertical);
        assert_eq!(nodes[2].text_resource_id, 0);
        assert!(nodes[2].text.is_empty());

        assert_eq!(nodes[3].kind, ViewKind::TextView);
        assert_eq!(nodes[3].parent, Some(2));
        assert_eq!(nodes[3].id, 0x7f01_0001);
        assert_eq!(nodes[3].width, LayoutSize::MatchParent);
        assert_eq!(nodes[3].height, LayoutSize::WrapContent);
        assert_eq!(nodes[3].orientation, LayoutOrientation::None);
        assert_eq!(nodes[3].text_resource_id, 0x7f03_0003);
        assert_eq!(nodes[3].text.as_str(), "Profile status: pending");

        assert_eq!(nodes[4].kind, ViewKind::Button);
        assert_eq!(nodes[4].parent, Some(2));
        assert_eq!(nodes[4].id, 0x7f01_0000);
        assert_eq!(nodes[4].width, LayoutSize::MatchParent);
        assert_eq!(nodes[4].height, LayoutSize::WrapContent);
        assert_eq!(nodes[4].orientation, LayoutOrientation::None);
        assert_eq!(nodes[4].text_resource_id, 0x7f03_0001);
        assert_eq!(nodes[4].text.as_str(), "Verify profile");

        let initial = *session.scene();
        assert_eq!(
            session.dispatch_click(0x7f01_0002),
            Err(Error::DexFrameworkState)
        );
        assert_eq!(*session.scene(), initial);
        assert_eq!(session.revision(), 0);

        let first = session
            .dispatch_click(0x7f01_0000)
            .expect("profile status callback");
        assert_eq!(first.method, callback);
        assert_eq!(first.clicked_view_id, 0x7f01_0000);
        assert_eq!(first.changed_view_id, 0x7f01_0001);
        assert_eq!(first.text_resource_id, 0x7f03_0002);
        assert_eq!(first.instruction_count, 7);
        assert_eq!(first.revision, 1);
        assert_eq!(session.revision(), 1);
        let after_first = *session.scene();
        for index in [0, 1, 2, 4] {
            assert_eq!(after_first.nodes()[index], initial.nodes()[index]);
        }
        assert_eq!(
            after_first.nodes()[3].text.as_str(),
            "Profile status: verified"
        );
        assert_eq!(
            after_first.nodes()[3].text_resource_id,
            initial.nodes()[3].text_resource_id
        );
        assert_eq!(after_first.nodes()[1].text.as_str(), "Account profile");

        let repeated = session
            .dispatch_click(0x7f01_0000)
            .expect("repeated profile status callback");
        assert_eq!(repeated.revision, 2);
        assert_eq!(session.revision(), 2);
        assert_eq!(*session.scene(), after_first);
        assert_eq!(session.scene().nodes()[1].text.as_str(), "Account profile");
        assert_eq!(
            session.scene().nodes()[3].text.as_str(),
            "Profile status: verified"
        );
    }

    #[cfg(feature = "androidbox-multiaction3")]
    #[test]
    fn multiaction_apk_dispatches_each_registered_button_branch() {
        let image = AndroidBox::load(MULTIACTION_APK).expect("multi-action APK admission");
        assert_eq!(
            image.manifest_info().activity_descriptor.as_str(),
            "Lorg/bndroid/multiaction/MainActivity;"
        );
        let mut session = image
            .launch_activity_session()
            .expect("multi-action constructor/onCreate/listeners");
        assert_eq!(session.listener_button_count(), 2);
        assert_eq!(session.listener_button_id(), 0x7f01_0000);
        assert_eq!(session.listener_button_ids(), &[0x7f01_0000, 0x7f01_0001]);
        assert!(session.is_listener_button(0x7f01_0000));
        assert!(session.is_listener_button(0x7f01_0001));
        assert!(!session.is_listener_button(0x7f01_0002));
        let launch = session.launch_info();
        assert_eq!(launch.listener_button_count, 2);
        assert_eq!(
            &launch.listener_button_ids[..usize::from(launch.listener_button_count)],
            &[0x7f01_0000, 0x7f01_0001]
        );

        let approved = session
            .dispatch_click(0x7f01_0000)
            .expect("approve callback branch");
        assert_eq!(approved.clicked_view_id, 0x7f01_0000);
        assert_eq!(approved.changed_view_id, 0x7f01_0002);
        assert_eq!(approved.text_resource_id, 0x7f03_0003);
        assert_eq!(approved.revision, 1);
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .expect("status TextView")
                .1
                .text
                .as_str(),
            "Decision: approved"
        );

        let rejected = session
            .dispatch_click(0x7f01_0001)
            .expect("reject callback branch");
        assert_eq!(rejected.clicked_view_id, 0x7f01_0001);
        assert_eq!(rejected.changed_view_id, 0x7f01_0002);
        assert_eq!(rejected.text_resource_id, 0x7f03_0005);
        assert_eq!(rejected.revision, 2);
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .expect("status TextView")
                .1
                .text
                .as_str(),
            "Decision: rejected"
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn envelope4_scratch_layout_keeps_inflater_off_the_call_stack() {
        assert_eq!(ANDROIDBOX_ENVELOPE_SCRATCH_BYTES, 43_272);
        assert!(
            ANDROIDBOX_ENVELOPE_SCRATCH_BYTES > MAX_ENVELOPE_XML_INFLATED_BYTES,
            "the 32-KiB bound is the XML output cap, not the total workspace"
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_loads_deflated_xml_descriptors_and_opaque_deflate() {
        let apk = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_method: 8,
                layout_method: 8,
                manifest_descriptor: true,
                opaque_deflated: true,
                opaque_descriptor: true,
                ..EnvelopeFixtureOptions::new()
            },
            None,
        );
        assert_eq!(
            AndroidBox::load(&apk).err(),
            Some(Error::ZipDataDescriptorUnsupported),
            "the ABI-50 loader must retain its STORED-only contract"
        );

        let mut scratch = AndroidBoxEnvelopeScratch::new();
        scratch.bytes.fill(0xa5);
        let image = AndroidBox::load_with_scratch(&apk, &mut scratch)
            .expect("Envelope-4 compressed XML APK");
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));
        let info = image.envelope_info().expect("envelope evidence");
        assert!(info.manifest_deflated());
        assert!(info.layout_deflated());
        assert_eq!(info.data_descriptor_count(), 2);
        assert_eq!(info.unconsumed_deflated_count(), 1);
        assert_eq!(
            image.manifest_info().activity_descriptor.as_str(),
            "Lorg/bndroid/multiaction/MainActivity;"
        );

        // Prove the returned image/session has no borrow into the workspace.
        scratch.bytes.fill(0x5a);
        scratch.wipe();
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));
        let mut session = image
            .launch_activity_session()
            .expect("cached scene after scratch wipe");
        session.dispatch_click(0x7f01_0000).expect("approve branch");
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .expect("status")
                .1
                .text
                .as_str(),
            "Decision: approved"
        );
        session.dispatch_click(0x7f01_0001).expect("reject branch");
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .expect("status")
                .1
                .text
                .as_str(),
            "Decision: rejected"
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_loads_the_real_aapt2_fixture() {
        assert_eq!(
            verify_apk_v2(ENVELOPE_APK)
                .expect("real Envelope-4 fixture has one valid v2 signer")
                .certificate_sha256,
            [
                0xe7, 0x41, 0x2e, 0x1c, 0xc0, 0xff, 0xbd, 0x21, 0x00, 0x0e, 0xce, 0x51, 0x76, 0xd0,
                0x08, 0x37, 0xce, 0x27, 0x6e, 0x42, 0xe6, 0xb5, 0x2b, 0xed, 0x99, 0xcb, 0x5e, 0x62,
                0x85, 0x7b, 0x77, 0xbf,
            ]
        );
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        let image = AndroidBox::load_with_scratch(ENVELOPE_APK, &mut scratch)
            .expect("real aapt2/zipalign/apksigner Envelope-4 fixture");
        let info = image.envelope_info().expect("Envelope-4 evidence");
        assert!(info.manifest_deflated());
        assert!(info.layout_deflated());
        assert_eq!(info.data_descriptor_count(), 3);
        assert_eq!(info.unconsumed_deflated_count(), 1);
        assert_eq!(
            image.manifest_info().activity_descriptor.as_str(),
            "Lorg/bndroid/envelope/MainActivity;"
        );
        let mut local_tuple_tamper = ENVELOPE_APK.to_vec();
        let manifest = compressed_entry_layout(&local_tuple_tamper, b"AndroidManifest.xml");
        let wrong_local_crc = read_u32(&local_tuple_tamper, manifest.local + 14) ^ 1;
        write_u32(
            &mut local_tuple_tamper,
            manifest.local + 14,
            wrong_local_crc,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&local_tuple_tamper, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );
        let mut session = image
            .launch_activity_session()
            .expect("cached real aapt2 scene");
        session
            .dispatch_click(0x7f01_0000)
            .expect("first real callback branch");
        session
            .dispatch_click(0x7f01_0001)
            .expect("second real callback branch");
    }

    #[cfg(feature = "androidbox-manifest-catalog3")]
    #[test]
    fn manifest_catalog3_enumerates_and_runs_a_real_sdk_multi_component_apk() {
        assert_eq!(
            verify_apk_v2(MANIFEST_CATALOG_APK)
                .expect("catalog fixture has one valid v2 signer")
                .certificate_sha256,
            [
                0xe7, 0x41, 0x2e, 0x1c, 0xc0, 0xff, 0xbd, 0x21, 0x00, 0x0e, 0xce, 0x51, 0x76, 0xd0,
                0x08, 0x37, 0xce, 0x27, 0x6e, 0x42, 0xe6, 0xb5, 0x2b, 0xed, 0x99, 0xcb, 0x5e, 0x62,
                0x85, 0x7b, 0x77, 0xbf,
            ]
        );

        let mut scratch = AndroidBoxEnvelopeScratch::new();
        scratch.bytes.fill(0xa5);
        let catalog = inspect_apk_manifest_catalog_with_scratch(MANIFEST_CATALOG_APK, &mut scratch)
            .expect("real aapt2 binary Manifest component directory");
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));
        assert_eq!(catalog.package().as_str(), "org.bndroid.catalog");
        assert_eq!(catalog.version_code(), 3);
        assert!(catalog.application_label().is_empty());
        assert_eq!(catalog.application_label_resource_id(), Some(0x7f03_0000));
        assert_eq!(catalog.launcher_count(), 2);
        assert_eq!(catalog.intent_filter_count(), 4);
        assert_eq!(catalog.action_count(), 4);
        assert_eq!(catalog.category_count(), 3);
        assert_eq!(catalog.data_count(), 0);
        assert_eq!(
            catalog
                .permissions()
                .iter()
                .map(|permission| permission.as_str())
                .collect::<std::vec::Vec<_>>(),
            [
                "android.permission.INTERNET",
                "android.permission.RECEIVE_BOOT_COMPLETED",
            ]
        );

        let components = catalog.components();
        assert_eq!(components.len(), 6);
        assert_eq!(components[0].kind(), ManifestComponentKind::Activity);
        assert_eq!(
            components[0].descriptor().as_str(),
            "Lorg/bndroid/catalog/MainActivity;"
        );
        assert!(components[0].launcher());
        assert!(components[0].exported());
        assert!(components[0].exported_explicit());
        assert!(components[0].enabled());
        assert_eq!(components[0].intent_filter_count(), 1);
        assert_eq!(components[0].action_count(), 1);
        assert_eq!(components[0].category_count(), 2);

        assert_eq!(components[1].kind(), ManifestComponentKind::Activity);
        assert_eq!(
            components[1].descriptor().as_str(),
            "Lorg/bndroid/catalog/DetailActivity;"
        );
        assert!(!components[1].launcher());
        assert!(!components[1].exported());

        assert_eq!(components[2].kind(), ManifestComponentKind::ActivityAlias);
        assert_eq!(
            components[2].descriptor().as_str(),
            "Lorg/bndroid/catalog/AliasActivity;"
        );
        assert_eq!(
            components[2].alias_target().expect("alias target").as_str(),
            "Lorg/bndroid/catalog/MainActivity;"
        );
        assert!(components[2].launcher());

        assert_eq!(components[3].kind(), ManifestComponentKind::Service);
        assert_eq!(components[3].intent_filter_count(), 1);
        assert!(!components[3].exported());
        assert_eq!(components[4].kind(), ManifestComponentKind::Receiver);
        assert!(components[4].exported());
        assert!(components[4].permission_declared());
        assert_eq!(components[5].kind(), ManifestComponentKind::Provider);
        assert!(components[5].authority_declared());
        assert!(components[5].permission_declared());
        assert_eq!(
            catalog
                .primary_launcher()
                .expect("declaration-order launcher")
                .descriptor()
                .as_str(),
            "Lorg/bndroid/catalog/MainActivity;"
        );

        let manifest_layout = compressed_entry_layout(MANIFEST_CATALOG_APK, b"AndroidManifest.xml");
        let decoded_manifest = super::zip::decode_envelope_entry(
            super::zip::required_envelope_entry(
                MANIFEST_CATALOG_APK,
                b"AndroidManifest.xml",
                super::MAX_MANIFEST_BYTES,
                Error::ZipMissingManifest,
                Error::ZipDuplicateManifest,
                Error::ManifestTooLarge,
            )
            .expect("catalog manifest envelope entry"),
            &mut scratch,
        )
        .expect("catalog manifest inflate");
        assert!(
            inspect_binary_manifest_catalog(decoded_manifest.as_bytes()).is_ok(),
            "the decoded binary-XML API must share the same catalog semantics"
        );
        scratch.clear();
        assert!(
            manifest_layout.data.start < manifest_layout.data.end,
            "compressed Manifest evidence must have a nonempty payload"
        );

        assert_eq!(
            AndroidBox::load_with_scratch(MANIFEST_CATALOG_APK, &mut scratch).err(),
            Some(Error::ManifestResourceMap),
            "the old single-Activity profile remains strict"
        );
        let mut catalog_scratch = AndroidBoxManifestCatalogScratch::new();
        let image = AndroidBox::load_with_manifest_catalog_scratch(
            MANIFEST_CATALOG_APK,
            &mut scratch,
            &mut catalog_scratch,
        )
        .expect("catalog-selected launcher binds to the bounded DEX runtime");
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));
        assert_eq!(
            ANDROIDBOX_MANIFEST_CATALOG_SCRATCH_BYTES,
            core::mem::size_of_val(&catalog_scratch)
        );
        assert!(catalog_scratch.is_clear());
        assert_eq!(
            image.manifest_info().activity_descriptor.as_str(),
            "Lorg/bndroid/catalog/MainActivity;"
        );
        assert_eq!(
            image.manifest_info().application_label.as_str(),
            "Component catalog"
        );
        let envelope = image.envelope_info().expect("catalog envelope evidence");
        assert!(envelope.manifest_deflated());
        assert!(envelope.layout_deflated());
        assert_eq!(envelope.unconsumed_deflated_count(), 0);
        let mut session = image
            .launch_activity_session()
            .expect("selected real SDK Activity executes");
        assert_eq!(session.listener_button_count(), 2);
        assert_eq!(
            session.scene().nodes()[1].text.as_str(),
            "Android components"
        );
        assert_eq!(session.scene().nodes()[2].text.as_str(), "Catalog ready");
        session
            .dispatch_click(0x7f01_0000)
            .expect("component button callback");
        assert_eq!(
            session.scene().nodes()[2].text.as_str(),
            "Components: 6 discovered"
        );
        session
            .dispatch_click(0x7f01_0001)
            .expect("permission button callback");
        assert_eq!(
            session.scene().nodes()[2].text.as_str(),
            "Permissions: declared only"
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_also_accepts_the_historical_stored_aligned_shape() {
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        let image = AndroidBox::load_with_scratch(MULTIACTION_APK, &mut scratch)
            .expect("stored zipalign fixture through Envelope-4");
        assert_eq!(
            image.envelope_info(),
            Some(super::ApkEnvelopeInfo {
                manifest_deflated: false,
                layout_deflated: false,
                data_descriptor_count: 0,
                unconsumed_deflated_count: 0,
            })
        );
        assert_eq!(
            image
                .launch_activity_session()
                .expect("cached stored layout")
                .listener_button_count(),
            2
        );
        scratch.clear();
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_keeps_dex_and_resource_table_stored() {
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        let compressed_dex = build_envelope_fixture(
            EnvelopeFixtureOptions {
                dex_method: 8,
                ..EnvelopeFixtureOptions::new()
            },
            None,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&compressed_dex, &mut scratch).err(),
            Some(Error::ZipCompressedDex)
        );

        let compressed_resources = build_envelope_fixture(
            EnvelopeFixtureOptions {
                resources_method: 8,
                ..EnvelopeFixtureOptions::new()
            },
            None,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&compressed_resources, &mut scratch).err(),
            Some(Error::ZipCompressionUnsupported)
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_descriptor_and_deflate_fail_closed() {
        let options = EnvelopeFixtureOptions {
            manifest_method: 8,
            manifest_descriptor: true,
            ..EnvelopeFixtureOptions::new()
        };
        let mut scratch = AndroidBoxEnvelopeScratch::new();

        let mut noncanonical_local = build_envelope_fixture(options, None);
        let manifest = compressed_entry_layout(&noncanonical_local, b"AndroidManifest.xml");
        write_u32(&mut noncanonical_local, manifest.local + 14, 1);
        assert_eq!(
            AndroidBox::load_with_scratch(&noncanonical_local, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );

        let mut mismatched_descriptor = build_envelope_fixture(options, None);
        let manifest = compressed_entry_layout(&mismatched_descriptor, b"AndroidManifest.xml");
        let descriptor_crc = manifest.data.end + 4;
        let wrong_crc = read_u32(&mismatched_descriptor, descriptor_crc) ^ 1;
        write_u32(&mut mismatched_descriptor, descriptor_crc, wrong_crc);
        assert_eq!(
            AndroidBox::load_with_scratch(&mismatched_descriptor, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );

        let truncated_descriptor = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_descriptor_truncated: true,
                ..options
            },
            None,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&truncated_descriptor, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );

        let missing_descriptor = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_descriptor_omitted: true,
                ..options
            },
            None,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&missing_descriptor, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );

        let trailing_deflate = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_deflate_trailing_byte: true,
                ..options
            },
            None,
        );
        scratch.bytes.fill(0x33);
        assert_eq!(
            AndroidBox::load_with_scratch(&trailing_deflate, &mut scratch).err(),
            Some(Error::ZipDeflateInvalid)
        );
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));

        let bad_crc = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_crc_xor: 1,
                ..options
            },
            None,
        );
        assert_eq!(
            AndroidBox::load_with_scratch(&bad_crc, &mut scratch).err(),
            Some(Error::ZipCrcMismatch)
        );

        let mut deflate_options = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_method: 8,
                ..EnvelopeFixtureOptions::new()
            },
            None,
        );
        let manifest = compressed_entry_layout(&deflate_options, b"AndroidManifest.xml");
        write_u16(&mut deflate_options, manifest.local + 6, 0x0006);
        write_u16(&mut deflate_options, manifest.central + 8, 0x0006);
        AndroidBox::load_with_scratch(&deflate_options, &mut scratch)
            .expect("DEFLATE option bits are legal only with method 8");

        let mut stored_deflate_options =
            build_envelope_fixture(EnvelopeFixtureOptions::new(), None);
        let resources = compressed_entry_layout(&stored_deflate_options, b"resources.arsc");
        write_u16(&mut stored_deflate_options, resources.local + 6, 0x0002);
        write_u16(&mut stored_deflate_options, resources.central + 8, 0x0002);
        assert_eq!(
            AndroidBox::load_with_scratch(&stored_deflate_options, &mut scratch).err(),
            Some(Error::ZipEntryMismatch)
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_accepts_signatureless_descriptor_and_rejects_bad_fields() {
        let options = EnvelopeFixtureOptions {
            manifest_method: 8,
            manifest_descriptor: true,
            manifest_descriptor_signature: false,
            ..EnvelopeFixtureOptions::new()
        };
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        let apk = build_envelope_fixture(options, None);
        let image = AndroidBox::load_with_scratch(&apk, &mut scratch)
            .expect("signatureless descriptor is a valid ZIP shape");
        assert_eq!(
            image
                .envelope_info()
                .expect("Envelope-4 evidence")
                .data_descriptor_count(),
            1
        );

        let mut mismatched = build_envelope_fixture(options, None);
        let manifest = compressed_entry_layout(&mismatched, b"AndroidManifest.xml");
        let wrong_crc = read_u32(&mismatched, manifest.data.end) ^ 1;
        write_u32(&mut mismatched, manifest.data.end, wrong_crc);
        assert_eq!(
            AndroidBox::load_with_scratch(&mismatched, &mut scratch).err(),
            Some(Error::ZipDataDescriptorInvalid)
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_rejects_duplicate_local_record_ranges() {
        let apk = build_envelope_fixture(
            EnvelopeFixtureOptions {
                duplicate_manifest_central: true,
                ..EnvelopeFixtureOptions::new()
            },
            None,
        );
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        assert_eq!(
            AndroidBox::load_with_scratch(&apk, &mut scratch).err(),
            Some(Error::ZipEntryOverlap)
        );
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_inflated_xml_has_a_separate_32k_cap() {
        let oversized = std::vec![0u8; MAX_ENVELOPE_XML_INFLATED_BYTES + 1];
        let apk = build_envelope_fixture(
            EnvelopeFixtureOptions {
                manifest_method: 8,
                ..EnvelopeFixtureOptions::new()
            },
            Some(&oversized),
        );
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        assert_eq!(
            AndroidBox::load_with_scratch(&apk, &mut scratch).err(),
            Some(Error::ZipInflatedEntryTooLarge)
        );
        assert!(scratch.bytes.iter().all(|byte| *byte == 0));
    }

    #[cfg(not(feature = "androidbox-multiaction3"))]
    #[test]
    fn multiaction_apk_requires_the_explicit_multiaction_profile() {
        assert_eq!(
            AndroidBox::load(MULTIACTION_APK).err(),
            Some(Error::DexFrameworkState)
        );
    }

    #[test]
    fn interactive_sessions_are_fresh_and_isolated() {
        let mut first =
            launch_activity_session(INTERACTIVE_APK).expect("first interactive session");
        let second = launch_activity_session(INTERACTIVE_APK).expect("second interactive session");
        first
            .dispatch_click(first.listener_button_id())
            .expect("first callback");
        assert_eq!(
            first.scene().nodes()[1].text.as_str(),
            "Button callback executed"
        );
        assert_eq!(
            second.scene().nodes()[1].text.as_str(),
            "Ready for a real APK click"
        );
        assert_eq!(first.revision(), 1);
        assert_eq!(second.revision(), 0);
    }

    #[test]
    fn interactive_activity_rejects_wrong_interface_and_callback_opcode() {
        let mut wrong_interface = INTERACTIVE_APK.to_vec();
        let dex = dex_layout(&wrong_interface);
        let descriptor_relative = find_bytes(
            &wrong_interface[dex.data.clone()],
            b"Landroid/view/View$OnClickListener;",
        );
        wrong_interface[dex.data.start + descriptor_relative + 1] = b'x';
        reseal_dex_and_zip(&mut wrong_interface);
        assert_load_error(&wrong_interface, Error::DexActivityClassInvalid);

        let image = AndroidBox::load(INTERACTIVE_APK).expect("baseline");
        let callback = image.activity_on_click_method().expect("callback");
        let mut unknown_callback_opcode = INTERACTIVE_APK.to_vec();
        let dex = dex_layout(&unknown_callback_opcode);
        let first_instruction =
            dex.data.start + usize::try_from(callback.code_offset).unwrap() + 16;
        unknown_callback_opcode[first_instruction] = 0x13;
        reseal_dex_and_zip(&mut unknown_callback_opcode);
        assert_load_error(&unknown_callback_opcode, Error::DexUnknownOpcode(0x13));

        let mut missing_check_cast = INTERACTIVE_APK.to_vec();
        let dex = dex_layout(&missing_check_cast);
        let check_cast =
            dex.data.start + usize::try_from(callback.code_offset).unwrap() + 16 + 7 * 2;
        assert_eq!(missing_check_cast[check_cast], 0x1f);
        missing_check_cast[check_cast] = 0x15;
        reseal_dex_and_zip(&mut missing_check_cast);
        assert_load_error(&missing_check_cast, Error::DexFrameworkState);
    }

    #[test]
    fn interactive_activity_rejects_listener_bound_to_the_wrong_receiver() {
        let image = AndroidBox::load(INTERACTIVE_APK).expect("baseline");
        let on_create = image.activity_method();
        let mut wrong_receiver = INTERACTIVE_APK.to_vec();
        let dex = dex_layout(&wrong_receiver);
        let packed_registers =
            dex.data.start + usize::try_from(on_create.code_offset).unwrap() + 16 + 16 * 2;
        assert_eq!(read_u16(&wrong_receiver, packed_registers), 0x0001);
        write_u16(&mut wrong_receiver, packed_registers, 0x0010);
        reseal_dex_and_zip(&mut wrong_receiver);
        assert_load_error(&wrong_receiver, Error::DexFrameworkState);
    }

    #[test]
    fn interactive_callback_requires_exact_public_view_proto() {
        let image = AndroidBox::load(INTERACTIVE_APK).expect("baseline");
        let callback = image.activity_on_click_method().expect("callback");

        let mut wrong_access = INTERACTIVE_APK.to_vec();
        let mut cursor = activity_class_data_offset(INTERACTIVE_APK);
        assert_eq!(read_test_uleb128(INTERACTIVE_APK, &mut cursor), 0);
        assert_eq!(read_test_uleb128(INTERACTIVE_APK, &mut cursor), 0);
        assert_eq!(read_test_uleb128(INTERACTIVE_APK, &mut cursor), 1);
        assert_eq!(read_test_uleb128(INTERACTIVE_APK, &mut cursor), 2);
        read_test_uleb128(INTERACTIVE_APK, &mut cursor);
        read_test_uleb128(INTERACTIVE_APK, &mut cursor);
        read_test_uleb128(INTERACTIVE_APK, &mut cursor);
        read_test_uleb128(INTERACTIVE_APK, &mut cursor);
        assert_eq!(wrong_access[cursor], 1);
        wrong_access[cursor] = 4;
        reseal_dex_and_zip(&mut wrong_access);
        assert_load_error(&wrong_access, Error::DexMethodFlags);

        let mut wrong_proto = INTERACTIVE_APK.to_vec();
        let callback_method_id = method_id_offset(&wrong_proto, callback.method_index);
        let on_create_method_id =
            method_id_offset(&wrong_proto, image.activity_method().method_index);
        let on_create_proto = read_u16(&wrong_proto, on_create_method_id + 2);
        write_u16(&mut wrong_proto, callback_method_id + 2, on_create_proto);
        reseal_dex_and_zip(&mut wrong_proto);
        assert_load_error(&wrong_proto, Error::DexActivityPrototype);
    }

    #[test]
    fn interactive_layout_rejects_an_id_missing_from_the_resource_table() {
        let mut missing_id = INTERACTIVE_APK.to_vec();
        let layout = entry_layout(&missing_id, DEFAULT_ACTIVITY_LAYOUT_ENTRY);
        let relative = find_bytes(
            &missing_id[layout.data.clone()],
            &0x7f01_0001_u32.to_le_bytes(),
        );
        let absolute = layout.data.start + relative;
        missing_id[absolute..absolute + 4].copy_from_slice(&0x7f01_0002_u32.to_le_bytes());
        update_entry_zip_crc(&mut missing_id, DEFAULT_ACTIVITY_LAYOUT_ENTRY);
        assert_load_error(&missing_id, Error::ResourceNotFound);
    }

    #[test]
    fn allocation_free_activity_wrapper_returns_owned_bounded_view() {
        let launched = launch_activity(DEMO_APK).expect("activity launch");
        assert_eq!(launched.view.title.as_str(), "AndroidBox DEX-0 Demo");
        assert_eq!(launched.view.text.as_str(), "AndroidBox DEX-0 fixture");
        assert_eq!(launched.manifest.package.len(), 16);
        assert!(!launched.manifest.activity_descriptor.is_empty());
    }

    #[test]
    fn allocation_free_convenience_wrappers_execute_real_dex() {
        assert_eq!(boot(DEMO_APK).expect("boot").result, 20_260_729);
        assert_eq!(on_tap(DEMO_APK, -10).expect("tap").result, -3);
    }

    #[test]
    fn tap_input_changes_the_executed_result() {
        let image = AndroidBox::load(DEMO_APK).expect("load");
        assert_eq!(image.on_tap(0).expect("tap zero").result, 7);
        assert_eq!(image.on_tap(100).expect("tap hundred").result, 107);
        assert_eq!(
            image.on_tap(i32::MAX).expect("wrapping tap").result,
            i32::MIN + 6
        );
    }

    #[test]
    fn wrong_zip_crc_is_rejected_before_dex_parsing() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        let wrong = read_u32(&apk, layout.central + 16) ^ 1;
        write_u32(&mut apk, layout.central + 16, wrong);
        write_u32(&mut apk, layout.local + 14, wrong);
        assert_load_error(&apk, Error::ZipCrcMismatch);
    }

    #[test]
    fn compressed_classes_dex_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        write_u16(&mut apk, layout.central + 10, 8);
        write_u16(&mut apk, layout.local + 8, 8);
        assert_load_error(&apk, Error::ZipCompressedDex);
    }

    #[test]
    fn compressed_binary_manifest_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        write_u16(&mut apk, layout.central + 10, 8);
        write_u16(&mut apk, layout.local + 8, 8);
        assert_load_error(&apk, Error::ZipCompressedManifest);
    }

    #[test]
    fn encrypted_entry_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        write_u16(&mut apk, layout.central + 8, 1);
        write_u16(&mut apk, layout.local + 6, 1);
        assert_load_error(&apk, Error::ZipEncrypted);
    }

    #[test]
    fn zip64_sentinel_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let eocd = find_signature_from_end(&apk, 0x0605_4b50);
        write_u16(&mut apk, eocd + 8, u16::MAX);
        write_u16(&mut apk, eocd + 10, u16::MAX);
        assert_load_error(&apk, Error::Zip64Unsupported);
    }

    #[test]
    fn duplicate_classes_dex_is_rejected() {
        let layout = dex_layout(DEMO_APK);
        let eocd = find_signature_from_end(DEMO_APK, 0x0605_4b50);
        let central_len = central_record_len(DEMO_APK, layout.central);
        let duplicate = &DEMO_APK[layout.central..layout.central + central_len];
        let mut apk = DEMO_APK[..eocd].to_vec();
        apk.extend_from_slice(duplicate);
        let new_eocd = apk.len();
        apk.extend_from_slice(&DEMO_APK[eocd..]);
        write_u16(&mut apk, new_eocd + 8, 3);
        write_u16(&mut apk, new_eocd + 10, 3);
        let old_size = read_u32(DEMO_APK, eocd + 12);
        write_u32(
            &mut apk,
            new_eocd + 12,
            old_size + u32::try_from(central_len).expect("central record size"),
        );
        assert_load_error(&apk, Error::ZipDuplicateDex);
    }

    #[test]
    fn duplicate_binary_manifest_is_rejected() {
        let layout = manifest_layout(DEMO_APK);
        let eocd = find_signature_from_end(DEMO_APK, 0x0605_4b50);
        let central_len = central_record_len(DEMO_APK, layout.central);
        let duplicate = &DEMO_APK[layout.central..layout.central + central_len];
        let mut apk = DEMO_APK[..eocd].to_vec();
        apk.extend_from_slice(duplicate);
        let new_eocd = apk.len();
        apk.extend_from_slice(&DEMO_APK[eocd..]);
        write_u16(&mut apk, new_eocd + 8, 3);
        write_u16(&mut apk, new_eocd + 10, 3);
        let old_size = read_u32(DEMO_APK, eocd + 12);
        write_u32(
            &mut apk,
            new_eocd + 12,
            old_size + u32::try_from(central_len).expect("central record size"),
        );
        assert_load_error(&apk, Error::ZipDuplicateManifest);
    }

    #[test]
    fn renamed_binary_manifest_is_rejected_as_missing() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        apk[layout.local + 30] = b'X';
        apk[layout.central + 46] = b'X';
        assert_load_error(&apk, Error::ZipMissingManifest);
    }

    #[test]
    fn wrong_manifest_zip_crc_is_rejected_before_xml_parsing() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        apk[layout.data.end - 1] ^= 1;
        assert_load_error(&apk, Error::ZipCrcMismatch);
    }

    #[test]
    fn manifest_string_pool_bounds_corruption_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        // The string-pool chunk follows the eight-byte XML document header.
        write_u32(&mut apk, layout.data.start + 8 + 20, u32::MAX);
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::ManifestStringPool);
    }

    #[test]
    fn manifest_resource_map_corruption_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        // Resource map follows the 0x458-byte string-pool chunk.
        assert_eq!(read_u16(&apk, layout.data.start + 0x460), 0x0180);
        write_u32(&mut apk, layout.data.start + 0x468, 0x0101_ffff);
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::ManifestResourceMap);
    }

    #[test]
    fn duplicate_package_attribute_is_rejected() {
        let mut xml = raw_manifest();
        // Change platformBuildVersionCode's no-namespace name index to the
        // package string index, producing two package attributes.
        write_u32(&mut xml, 0x4b0 + 36 + 5 * 20 + 4, 27);
        assert_manifest_error(&xml, Error::ManifestPackageDuplicate);
    }

    #[test]
    fn missing_package_attribute_is_rejected() {
        let mut xml = raw_manifest();
        let root = 0x4b0;
        let package_attribute = root + 36 + 4 * 20;
        write_u32(&mut xml, 4, 1_984 - 20);
        write_u32(&mut xml, root + 4, 176 - 20);
        write_u16(&mut xml, root + 28, 6);
        xml.drain(package_attribute..package_attribute + 20);
        assert_manifest_error(&xml, Error::ManifestPackageMissing);
    }

    #[test]
    fn duplicate_version_code_attribute_is_rejected() {
        let mut xml = raw_manifest();
        let root = 0x4b0;
        let compile_sdk_name = root + 36 + 2 * 20 + 4;
        // Resource-map string index 5 is android:versionCode. Reusing it for
        // compileSdkVersion creates a duplicate in the same namespace.
        write_u32(&mut xml, compile_sdk_name, 5);
        assert_manifest_error(&xml, Error::ManifestVersionCodeDuplicate);
    }

    #[test]
    fn missing_version_code_attribute_is_rejected() {
        let mut xml = raw_manifest();
        let root = 0x4b0;
        let version_code_attribute = root + 36;
        write_u32(&mut xml, 4, 1_984 - 20);
        write_u32(&mut xml, root + 4, 176 - 20);
        write_u16(&mut xml, root + 28, 6);
        xml.drain(version_code_attribute..version_code_attribute + 20);
        assert_manifest_error(&xml, Error::ManifestVersionCodeMissing);
    }

    #[test]
    fn zero_version_code_is_rejected() {
        let mut xml = raw_manifest();
        let root = 0x4b0;
        write_u32(&mut xml, root + 36 + 16, 0);
        assert_manifest_error(&xml, Error::ManifestVersionCodeInvalid);
    }

    #[test]
    fn duplicate_main_launcher_activity_is_rejected() {
        let mut xml = raw_manifest();
        let activity_start = 0x638;
        let activity_end = 0x778;
        let duplicate = xml[activity_start..activity_end].to_vec();
        write_u32(
            &mut xml,
            4,
            1_984 + u32::try_from(duplicate.len()).expect("activity event bytes"),
        );
        xml.splice(activity_end..activity_end, duplicate);
        assert_manifest_error(&xml, Error::ManifestActivityDuplicate);
    }

    #[test]
    fn noncanonical_manifest_attribute_width_is_rejected() {
        let mut xml = raw_manifest();
        write_u16(&mut xml, 0x4b0 + 26, 19);
        assert_manifest_error(&xml, Error::ManifestStructure);
    }

    #[test]
    fn unknown_manifest_event_chunk_is_rejected() {
        let mut xml = raw_manifest();
        write_u16(&mut xml, 0x4b0, 0x0104);
        assert_manifest_error(&xml, Error::ManifestUnknownChunk(0x0104));
    }

    #[test]
    fn manifest_unknown_element_is_rejected_fail_closed() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        let activity = find_utf16_ascii(&apk[layout.data.clone()], b"activity");
        apk[layout.data.start + activity] = b'x';
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::ManifestUnknownElement);
    }

    #[test]
    fn non_exported_launcher_activity_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        // aapt2's generated activity start is at 0x638. Its second
        // attribute is android:exported and its typed data word is at 0x680.
        assert_eq!(read_u16(&apk, layout.data.start + 0x638), 0x0102);
        write_u32(&mut apk, layout.data.start + 0x680, 0);
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::ManifestActivityNotExported);
    }

    #[test]
    fn incomplete_main_launcher_filter_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        // Point LAUNCHER's string typed value at the MAIN action string.
        assert_eq!(read_u16(&apk, layout.data.start + 0x6f8), 0x0102);
        write_u32(&mut apk, layout.data.start + 0x724, 19);
        write_u32(&mut apk, layout.data.start + 0x72c, 19);
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::ManifestLauncherInvalid);
    }

    #[test]
    fn manifest_activity_descriptor_must_resolve_to_a_real_dex_class() {
        let mut apk = DEMO_APK.to_vec();
        let layout = manifest_layout(&apk);
        let activity = find_utf16_ascii(&apk[layout.data.clone()], b".MainActivity");
        apk[layout.data.start + activity + 2] = b'X';
        update_manifest_zip_crc(&mut apk);
        assert_load_error(&apk, Error::DexActivityClassMissing);
    }

    #[test]
    fn changed_dex_bytes_with_only_zip_crc_resealed_fail_dex_checksum() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        apk[layout.data.end - 1] ^= 1;
        update_zip_crc(&mut apk);
        assert_load_error(&apk, Error::DexChecksum);
    }

    #[test]
    fn changed_dex_signature_with_valid_adler_fails_signature() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        apk[layout.data.start + 12] ^= 1;
        reseal_adler_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexSignature);
    }

    #[test]
    fn wrong_legacy_class_descriptor_does_not_block_activity_admission() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        let relative = find_bytes(&apk[layout.data.clone()], ENTRY_CLASS);
        apk[layout.data.start + relative + 19] = b'X';
        reseal_dex_and_zip(&mut apk);
        let image = AndroidBox::load(&apk).expect("component admission");
        assert_eq!(image.boot(), Err(Error::DexClassMissing));
        image
            .launch_activity()
            .expect("manifest Activity must remain launchable");
    }

    #[test]
    fn out_of_bounds_dex_table_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        let file_size = read_u32(&apk, layout.data.start + 32);
        write_u32(&mut apk, layout.data.start + 60, file_size - 2);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexBounds);
    }

    #[test]
    fn out_of_bounds_map_count_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let layout = dex_layout(&apk);
        let map_offset = read_u32(&apk, layout.data.start + 52);
        write_u32(
            &mut apk,
            layout.data.start + usize::try_from(map_offset).expect("map offset"),
            65,
        );
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexMap);
    }

    #[test]
    fn exception_structure_on_entry_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk)
            .expect("load")
            .boot_method()
            .expect("boot metadata");
        let layout = dex_layout(&apk);
        let tries_offset =
            layout.data.start + usize::try_from(method.code_offset).expect("code offset") + 6;
        write_u16(&mut apk, tries_offset, 1);
        reseal_dex_and_zip(&mut apk);
        let image = AndroidBox::load(&apk).expect("component admission");
        assert_eq!(image.boot(), Err(Error::DexExceptionsUnsupported));
        image
            .launch_activity()
            .expect("legacy diagnostic failure must not block Activity");
    }

    #[test]
    fn invoke_opcode_is_not_in_the_dex_zero_allowlist() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk)
            .expect("load")
            .boot_method()
            .expect("boot metadata");
        let layout = dex_layout(&apk);
        let first_opcode =
            layout.data.start + usize::try_from(method.code_offset).expect("code offset") + 16;
        apk[first_opcode] = 0x6e;
        reseal_dex_and_zip(&mut apk);
        let image = AndroidBox::load(&apk).expect("component admission");
        assert_eq!(image.boot(), Err(Error::DexUnknownOpcode(0x6e)));
        image
            .launch_activity()
            .expect("legacy diagnostic failure must not block Activity");
    }

    #[test]
    fn manifest_activity_constructor_is_required_for_admission() {
        let mut apk = DEMO_APK.to_vec();
        let (constructor_index, boot_index) = {
            let image = AndroidBox::load(&apk).expect("load");
            (
                image.activity_constructor().method_index,
                image.boot_method().expect("boot metadata").method_index,
            )
        };
        let constructor_id = method_id_offset(&apk, constructor_index);
        let boot_id = method_id_offset(&apk, boot_index);
        let boot_name_index = read_u32(&apk, boot_id + 4);
        write_u32(&mut apk, constructor_id + 4, boot_name_index);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexMethodMissing);
    }

    #[test]
    fn manifest_activity_constructor_must_be_unique() {
        let mut apk = DEMO_APK.to_vec();
        let class_data = activity_class_data_offset(&apk);
        assert_eq!(&apk[class_data..class_data + 4], &[0, 0, 1, 1]);
        let mut second_method = class_data + 4;
        read_test_uleb128(&apk, &mut second_method);
        read_test_uleb128(&apk, &mut second_method);
        read_test_uleb128(&apk, &mut second_method);

        // Reinterpret the virtual-method record as a second direct record and
        // give it a zero method-index delta plus the exact constructor flags.
        // The scanner must reject the duplicate identity before trusting its
        // deliberately zero code offset.
        apk[class_data + 2] = 2;
        apk[class_data + 3] = 0;
        apk[second_method..second_method + 5].copy_from_slice(&[0, 0x81, 0x80, 0x04, 0]);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexMethodDuplicate);
    }

    #[test]
    fn manifest_activity_constructor_requires_exact_void_no_arg_proto() {
        let mut apk = DEMO_APK.to_vec();
        let (constructor_index, on_create_index) = {
            let image = AndroidBox::load(&apk).expect("load");
            (
                image.activity_constructor().method_index,
                image.activity_method().method_index,
            )
        };
        let constructor_id = method_id_offset(&apk, constructor_index);
        let on_create_id = method_id_offset(&apk, on_create_index);
        let on_create_proto = read_u16(&apk, on_create_id + 2);
        write_u16(&mut apk, constructor_id + 2, on_create_proto);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexPrototype);
    }

    #[test]
    fn manifest_activity_constructor_requires_exact_public_constructor_flags() {
        let mut apk = DEMO_APK.to_vec();
        let flags = activity_constructor_access_flags_offset(&apk);
        // 0x10001 is encoded as 81 80 04. Preserve its ULEB128 width while
        // changing PUBLIC to PRIVATE.
        assert_eq!(&apk[flags..flags + 3], &[0x81, 0x80, 0x04]);
        apk[flags] = 0x82;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexMethodFlags);
    }

    #[test]
    fn manifest_activity_fields_are_rejected_before_lifecycle_admission() {
        let mut apk = DEMO_APK.to_vec();
        let class_data = activity_class_data_offset(&apk);
        assert_eq!(apk[class_data], 0);
        apk[class_data] = 1;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexActivityClassInvalid);
    }

    #[test]
    fn manifest_activity_interfaces_are_rejected_before_lifecycle_admission() {
        let mut apk = DEMO_APK.to_vec();
        let class_definition = activity_class_definition_offset(&apk);
        assert_eq!(read_u32(&apk, class_definition + 12), 0);
        write_u32(&mut apk, class_definition + 12, 1);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexBounds);
    }

    #[test]
    fn constructor_must_invoke_the_activity_super_constructor() {
        let mut apk = DEMO_APK.to_vec();
        let constructor = AndroidBox::load(&apk).expect("load").activity_constructor();
        let layout = dex_layout(&apk);
        let method_reference = layout.data.start
            + usize::try_from(constructor.code_offset).expect("code offset")
            + 16
            + 2;
        // method@4 is java.lang.Object.<init>()V in this fixture.
        write_u16(&mut apk, method_reference, 4);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkUnknownClass);
    }

    #[test]
    fn constructor_code_item_requires_one_outgoing_argument_slot() {
        let mut apk = DEMO_APK.to_vec();
        let constructor = AndroidBox::load(&apk).expect("load").activity_constructor();
        let layout = dex_layout(&apk);
        let outgoing =
            layout.data.start + usize::try_from(constructor.code_offset).expect("code offset") + 4;
        assert_eq!(read_u16(&apk, outgoing), 1);
        write_u16(&mut apk, outgoing, 0);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexCode);
    }

    #[test]
    fn constructor_rejects_a_known_framework_method_with_the_wrong_proto() {
        let mut apk = DEMO_APK.to_vec();
        let constructor = AndroidBox::load(&apk).expect("load").activity_constructor();
        let layout = dex_layout(&apk);
        let method_reference = layout.data.start
            + usize::try_from(constructor.code_offset).expect("code offset")
            + 16
            + 2;
        // method@1 is Activity.onCreate(Bundle)V, not Activity.<init>()V.
        write_u16(&mut apk, method_reference, 1);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkUnknownMethod);
    }

    #[test]
    fn constructor_must_call_super_before_returning() {
        let mut apk = DEMO_APK.to_vec();
        let constructor = AndroidBox::load(&apk).expect("load").activity_constructor();
        let layout = dex_layout(&apk);
        let first_instruction =
            layout.data.start + usize::try_from(constructor.code_offset).expect("code offset") + 16;
        write_u16(&mut apk, first_instruction, 0x000e);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkState);
    }

    #[test]
    fn unknown_constructor_opcode_is_rejected_during_admission_dry_run() {
        let mut apk = DEMO_APK.to_vec();
        let constructor = AndroidBox::load(&apk).expect("load").activity_constructor();
        let layout = dex_layout(&apk);
        let first_opcode =
            layout.data.start + usize::try_from(constructor.code_offset).expect("code offset") + 16;
        apk[first_opcode] = 0xff;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexUnknownOpcode(0xff));
    }

    #[test]
    fn unknown_activity_framework_invoke_is_rejected_during_load() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let method_reference =
            layout.data.start + usize::try_from(method.code_offset).expect("code offset") + 16 + 2;
        // method@4 is java.lang.Object.<init>(), which is not admitted in
        // ActivityLifecycle-1's onCreate framework allowlist.
        write_u16(&mut apk, method_reference, 4);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkUnknownClass);
    }

    #[test]
    fn unknown_method_on_known_framework_class_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let method_reference =
            layout.data.start + usize::try_from(method.code_offset).expect("code offset") + 16 + 2;
        // method@0 is Activity.<init>(); the owner is known but the call is
        // not the required Activity.onCreate(Bundle) super invocation.
        write_u16(&mut apk, method_reference, 0);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkUnknownMethod);
    }

    #[test]
    fn activity_register_bounds_are_enforced() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let new_instance_high_byte = layout.data.start
            + usize::try_from(method.code_offset).expect("code offset")
            + 16
            + 3 * 2
            + 1;
        apk[new_instance_high_byte] = 7;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexCode);
    }

    #[test]
    fn move_result_without_immediate_framework_result_is_rejected() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let return_opcode = layout.data.start
            + usize::try_from(method.code_offset).expect("code offset")
            + 16
            + 16 * 2;
        apk[return_opcode] = 0x0c;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkState);
    }

    #[test]
    fn activity_lifecycle_call_order_is_enforced() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let set_text_method_reference = layout.data.start
            + usize::try_from(method.code_offset).expect("code offset")
            + 16
            + 10 * 2
            + 2;
        // Replace TextView.setText method@3 with setContentView method@10
        // while retaining the original registers and position.
        write_u16(&mut apk, set_text_method_reference, 10);
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexFrameworkState);
    }

    #[test]
    fn unknown_activity_opcode_is_rejected_during_load() {
        let mut apk = DEMO_APK.to_vec();
        let method = AndroidBox::load(&apk).expect("load").activity_method();
        let layout = dex_layout(&apk);
        let first_opcode =
            layout.data.start + usize::try_from(method.code_offset).expect("code offset") + 16;
        apk[first_opcode] = 0xff;
        reseal_dex_and_zip(&mut apk);
        assert_load_error(&apk, Error::DexUnknownOpcode(0xff));
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[derive(Clone, Copy)]
    struct EnvelopeFixtureOptions {
        manifest_method: u16,
        resources_method: u16,
        layout_method: u16,
        dex_method: u16,
        manifest_descriptor: bool,
        manifest_descriptor_signature: bool,
        opaque_deflated: bool,
        opaque_descriptor: bool,
        opaque_descriptor_signature: bool,
        duplicate_manifest_central: bool,
        manifest_descriptor_omitted: bool,
        manifest_descriptor_truncated: bool,
        manifest_deflate_trailing_byte: bool,
        manifest_crc_xor: u32,
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    impl EnvelopeFixtureOptions {
        const fn new() -> Self {
            Self {
                manifest_method: 0,
                resources_method: 0,
                layout_method: 0,
                dex_method: 0,
                manifest_descriptor: false,
                manifest_descriptor_signature: true,
                opaque_deflated: false,
                opaque_descriptor: false,
                opaque_descriptor_signature: true,
                duplicate_manifest_central: false,
                manifest_descriptor_omitted: false,
                manifest_descriptor_truncated: false,
                manifest_deflate_trailing_byte: false,
                manifest_crc_xor: 0,
            }
        }
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[derive(Clone)]
    struct TestCentralEntry {
        name: std::vec::Vec<u8>,
        flags: u16,
        method: u16,
        crc32: u32,
        compressed_size: u32,
        uncompressed_size: u32,
        local_offset: u32,
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[derive(Clone)]
    struct CompressedEntryLayout {
        local: usize,
        central: usize,
        data: Range<usize>,
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn build_envelope_fixture(
        options: EnvelopeFixtureOptions,
        manifest_override: Option<&[u8]>,
    ) -> std::vec::Vec<u8> {
        let manifest = manifest_override.map_or_else(
            || stored_fixture_entry(MULTIACTION_APK, b"AndroidManifest.xml"),
            |bytes| bytes.to_vec(),
        );
        let resources = stored_fixture_entry(MULTIACTION_APK, b"resources.arsc");
        let layout = stored_fixture_entry(MULTIACTION_APK, b"res/layout/activity_multiaction.xml");
        let dex = stored_fixture_entry(MULTIACTION_APK, b"classes.dex");
        let mut apk = std::vec::Vec::new();
        let mut central = std::vec::Vec::new();
        append_test_zip_entry(
            &mut apk,
            &mut central,
            b"AndroidManifest.xml",
            &manifest,
            options.manifest_method,
            options.manifest_descriptor,
            options.manifest_descriptor_signature,
            options.manifest_descriptor_omitted,
            options.manifest_descriptor_truncated,
            options.manifest_deflate_trailing_byte,
            options.manifest_crc_xor,
        );
        append_test_zip_entry(
            &mut apk,
            &mut central,
            b"resources.arsc",
            &resources,
            options.resources_method,
            false,
            true,
            false,
            false,
            false,
            0,
        );
        append_test_zip_entry(
            &mut apk,
            &mut central,
            b"res/layout/activity_multiaction.xml",
            &layout,
            options.layout_method,
            false,
            true,
            false,
            false,
            false,
            0,
        );
        append_test_zip_entry(
            &mut apk,
            &mut central,
            b"classes.dex",
            &dex,
            options.dex_method,
            false,
            true,
            false,
            false,
            false,
            0,
        );
        if options.opaque_deflated {
            append_test_zip_entry(
                &mut apk,
                &mut central,
                b"assets/envelope-note.txt",
                b"opaque compressed bytes are authenticated but not interpreted",
                8,
                options.opaque_descriptor,
                options.opaque_descriptor_signature,
                false,
                false,
                false,
                0,
            );
        }
        if options.duplicate_manifest_central {
            let duplicate = central[0].clone();
            central.push(duplicate);
        }

        let central_offset = u32::try_from(apk.len()).expect("test central offset");
        for entry in &central {
            push_u32(&mut apk, 0x0201_4b50);
            push_u16(&mut apk, 20);
            push_u16(&mut apk, 20);
            push_u16(&mut apk, entry.flags);
            push_u16(&mut apk, entry.method);
            push_u16(&mut apk, 0);
            push_u16(&mut apk, 0);
            push_u32(&mut apk, entry.crc32);
            push_u32(&mut apk, entry.compressed_size);
            push_u32(&mut apk, entry.uncompressed_size);
            push_u16(
                &mut apk,
                u16::try_from(entry.name.len()).expect("test entry name"),
            );
            push_u16(&mut apk, 0);
            push_u16(&mut apk, 0);
            push_u16(&mut apk, 0);
            push_u16(&mut apk, 0);
            push_u32(&mut apk, 0);
            push_u32(&mut apk, entry.local_offset);
            apk.extend_from_slice(&entry.name);
        }
        let central_size = u32::try_from(apk.len())
            .expect("test archive size")
            .checked_sub(central_offset)
            .expect("test central size");
        let entry_count = u16::try_from(central.len()).expect("test entry count");
        push_u32(&mut apk, 0x0605_4b50);
        push_u16(&mut apk, 0);
        push_u16(&mut apk, 0);
        push_u16(&mut apk, entry_count);
        push_u16(&mut apk, entry_count);
        push_u32(&mut apk, central_size);
        push_u32(&mut apk, central_offset);
        push_u16(&mut apk, 0);
        apk
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[allow(clippy::too_many_arguments)]
    fn append_test_zip_entry(
        apk: &mut std::vec::Vec<u8>,
        central: &mut std::vec::Vec<TestCentralEntry>,
        name: &[u8],
        data: &[u8],
        method: u16,
        descriptor: bool,
        descriptor_signature: bool,
        omit_descriptor: bool,
        truncate_descriptor: bool,
        trailing_deflate_byte: bool,
        crc_xor: u32,
    ) {
        assert!(method == 0 || method == 8);
        let mut compressed = if method == 8 {
            raw_deflate_stored_blocks(data)
        } else {
            data.to_vec()
        };
        if trailing_deflate_byte {
            assert_eq!(method, 8);
            compressed.push(0);
        }
        let flags = if descriptor { 1 << 3 } else { 0 };
        let crc = crc32(data) ^ crc_xor;
        let compressed_size = u32::try_from(compressed.len()).expect("test compressed size");
        let uncompressed_size = u32::try_from(data.len()).expect("test uncompressed size");
        let local_offset = u32::try_from(apk.len()).expect("test local offset");
        push_u32(apk, 0x0403_4b50);
        push_u16(apk, 20);
        push_u16(apk, flags);
        push_u16(apk, method);
        push_u16(apk, 0);
        push_u16(apk, 0);
        push_u32(apk, if descriptor { 0 } else { crc });
        push_u32(apk, if descriptor { 0 } else { compressed_size });
        push_u32(apk, if descriptor { 0 } else { uncompressed_size });
        push_u16(apk, u16::try_from(name.len()).expect("test entry name"));
        push_u16(apk, 0);
        apk.extend_from_slice(name);
        apk.extend_from_slice(&compressed);
        if descriptor && !omit_descriptor {
            if descriptor_signature {
                push_u32(apk, 0x0807_4b50);
            }
            push_u32(apk, crc);
            push_u32(apk, compressed_size);
            if !truncate_descriptor {
                push_u32(apk, uncompressed_size);
            }
        }
        central.push(TestCentralEntry {
            name: name.to_vec(),
            flags,
            method,
            crc32: crc,
            compressed_size,
            uncompressed_size,
            local_offset,
        });
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn raw_deflate_stored_blocks(bytes: &[u8]) -> std::vec::Vec<u8> {
        let mut encoded = std::vec::Vec::new();
        if bytes.is_empty() {
            encoded.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
            return encoded;
        }
        let mut cursor = 0usize;
        while cursor < bytes.len() {
            let remaining = bytes.len() - cursor;
            let length = remaining.min(u16::MAX as usize);
            let final_block = cursor + length == bytes.len();
            encoded.push(u8::from(final_block));
            let length = u16::try_from(length).expect("stored DEFLATE block");
            encoded.extend_from_slice(&length.to_le_bytes());
            encoded.extend_from_slice(&(!length).to_le_bytes());
            encoded.extend_from_slice(&bytes[cursor..cursor + usize::from(length)]);
            cursor += usize::from(length);
        }
        encoded
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn stored_fixture_entry(apk: &[u8], name: &[u8]) -> std::vec::Vec<u8> {
        let layout = entry_layout(apk, name);
        apk[layout.data].to_vec()
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn compressed_entry_layout(apk: &[u8], expected_name: &[u8]) -> CompressedEntryLayout {
        let eocd = find_signature_from_end(apk, 0x0605_4b50);
        let mut central = usize::try_from(read_u32(apk, eocd + 16)).expect("central offset");
        while central < eocd {
            assert_eq!(read_u32(apk, central), 0x0201_4b50);
            let name_len = usize::from(read_u16(apk, central + 28));
            let name = &apk[central + 46..central + 46 + name_len];
            if name == expected_name {
                let local = usize::try_from(read_u32(apk, central + 42)).expect("local offset");
                let local_name_len = usize::from(read_u16(apk, local + 26));
                let local_extra_len = usize::from(read_u16(apk, local + 28));
                let data_start = local + 30 + local_name_len + local_extra_len;
                let data_len =
                    usize::try_from(read_u32(apk, central + 20)).expect("compressed size");
                return CompressedEntryLayout {
                    local,
                    central,
                    data: data_start..data_start + data_len,
                };
            }
            central += central_record_len(apk, central);
        }
        panic!("requested compressed central entry");
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn push_u16(bytes: &mut std::vec::Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn push_u32(bytes: &mut std::vec::Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[derive(Clone)]
    struct DexLayout {
        local: usize,
        central: usize,
        data: Range<usize>,
    }

    fn dex_layout(apk: &[u8]) -> DexLayout {
        entry_layout(apk, b"classes.dex")
    }

    fn manifest_layout(apk: &[u8]) -> DexLayout {
        entry_layout(apk, b"AndroidManifest.xml")
    }

    fn method_id_offset(apk: &[u8], method_index: u32) -> usize {
        let layout = dex_layout(apk);
        let dex = layout.data.start;
        let method_count = read_u32(apk, dex + 88);
        assert!(method_index < method_count);
        let method_ids = usize::try_from(read_u32(apk, dex + 92)).expect("method_ids offset");
        dex + method_ids + usize::try_from(method_index).expect("method index") * 8
    }

    fn activity_class_definition_offset(apk: &[u8]) -> usize {
        let constructor_index = AndroidBox::load(apk)
            .expect("valid lifecycle fixture")
            .activity_constructor()
            .method_index;
        let class_type = read_u16(apk, method_id_offset(apk, constructor_index));
        let layout = dex_layout(apk);
        let dex = layout.data.start;
        let class_count = read_u32(apk, dex + 96);
        let class_definitions =
            usize::try_from(read_u32(apk, dex + 100)).expect("class_defs offset");
        let mut index = 0u32;
        while index < class_count {
            let definition =
                dex + class_definitions + usize::try_from(index).expect("class index") * 32;
            if read_u32(apk, definition) == u32::from(class_type) {
                return definition;
            }
            index += 1;
        }
        panic!("manifest Activity class definition");
    }

    fn activity_class_data_offset(apk: &[u8]) -> usize {
        let definition = activity_class_definition_offset(apk);
        let layout = dex_layout(apk);
        layout.data.start
            + usize::try_from(read_u32(apk, definition + 24)).expect("class_data offset")
    }

    fn activity_constructor_access_flags_offset(apk: &[u8]) -> usize {
        let mut cursor = activity_class_data_offset(apk);
        assert_eq!(read_test_uleb128(apk, &mut cursor), 0);
        assert_eq!(read_test_uleb128(apk, &mut cursor), 0);
        assert_eq!(read_test_uleb128(apk, &mut cursor), 1);
        assert_eq!(read_test_uleb128(apk, &mut cursor), 1);
        read_test_uleb128(apk, &mut cursor);
        cursor
    }

    fn read_test_uleb128(bytes: &[u8], cursor: &mut usize) -> u32 {
        let mut value = 0u32;
        let mut shift = 0u32;
        loop {
            let byte = bytes[*cursor];
            *cursor += 1;
            value |= u32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
            shift += 7;
            assert!(shift < 35);
        }
    }

    fn entry_layout(apk: &[u8], expected_name: &[u8]) -> DexLayout {
        let eocd = find_signature_from_end(apk, 0x0605_4b50);
        let mut central = usize::try_from(read_u32(apk, eocd + 16)).expect("central offset");
        while central < eocd {
            assert_eq!(read_u32(apk, central), 0x0201_4b50);
            let name_len = usize::from(read_u16(apk, central + 28));
            let name = &apk[central + 46..central + 46 + name_len];
            if name == expected_name {
                let local = usize::try_from(read_u32(apk, central + 42)).expect("local offset");
                assert_eq!(read_u32(apk, local), 0x0403_4b50);
                let local_name_len = usize::from(read_u16(apk, local + 26));
                let local_extra_len = usize::from(read_u16(apk, local + 28));
                let data_start = local + 30 + local_name_len + local_extra_len;
                let data_len =
                    usize::try_from(read_u32(apk, local + 22)).expect("uncompressed size");
                return DexLayout {
                    local,
                    central,
                    data: data_start..data_start + data_len,
                };
            }
            central += central_record_len(apk, central);
        }
        panic!("requested central entry");
    }

    fn central_record_len(apk: &[u8], central: usize) -> usize {
        46 + usize::from(read_u16(apk, central + 28))
            + usize::from(read_u16(apk, central + 30))
            + usize::from(read_u16(apk, central + 32))
    }

    fn update_zip_crc(apk: &mut [u8]) {
        update_entry_zip_crc(apk, b"classes.dex");
    }

    fn update_entry_zip_crc(apk: &mut [u8], name: &[u8]) {
        let layout = entry_layout(apk, name);
        let crc = crc32(&apk[layout.data]);
        write_u32(apk, layout.local + 14, crc);
        write_u32(apk, layout.central + 16, crc);
    }

    fn update_manifest_zip_crc(apk: &mut [u8]) {
        let layout = manifest_layout(apk);
        let crc = crc32(&apk[layout.data]);
        write_u32(apk, layout.local + 14, crc);
        write_u32(apk, layout.central + 16, crc);
    }

    fn reseal_adler_and_zip(apk: &mut [u8]) {
        let layout = dex_layout(apk);
        let checksum = adler32(&apk[layout.data.start + 12..layout.data.end]);
        write_u32(apk, layout.data.start + 8, checksum);
        update_zip_crc(apk);
    }

    fn reseal_dex_and_zip(apk: &mut [u8]) {
        let layout = dex_layout(apk);
        let signature = sha1(&apk[layout.data.start + 32..layout.data.end]);
        apk[layout.data.start + 12..layout.data.start + 32].copy_from_slice(&signature);
        reseal_adler_and_zip(apk);
    }

    fn assert_load_error(apk: &[u8], expected: Error) {
        match AndroidBox::load(apk) {
            Ok(_) => panic!("APK unexpectedly loaded"),
            Err(actual) => assert_eq!(actual, expected),
        }
    }

    fn find_signature_from_end(bytes: &[u8], signature: u32) -> usize {
        let pattern = signature.to_le_bytes();
        bytes
            .windows(4)
            .rposition(|window| window == pattern)
            .expect("signature")
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("fixture bytes")
    }

    fn find_utf16_ascii(haystack: &[u8], needle: &[u8]) -> usize {
        let byte_len = needle.len() * 2;
        let mut offset = 0usize;
        while offset + byte_len <= haystack.len() {
            let mut index = 0usize;
            while index < needle.len()
                && haystack[offset + index * 2] == needle[index]
                && haystack[offset + index * 2 + 1] == 0
            {
                index += 1;
            }
            if index == needle.len() {
                return offset;
            }
            offset += 1;
        }
        panic!("UTF-16 fixture string");
    }

    fn raw_manifest() -> std::vec::Vec<u8> {
        let layout = manifest_layout(DEMO_APK);
        DEMO_APK[layout.data].to_vec()
    }

    fn assert_manifest_error(xml: &[u8], expected: Error) {
        match super::manifest::parse(xml) {
            Ok(_) => panic!("binary manifest unexpectedly parsed"),
            Err(actual) => assert_eq!(actual, expected),
        }
    }

    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
