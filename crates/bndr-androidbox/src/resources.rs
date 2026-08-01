//! Allocation-free parsing for one deliberately bounded Android resource
//! profile.
//!
//! The parser accepts a stored `resources.arsc` entry, resolves direct
//! `TYPE_STRING` values from a package's default configuration, and reads one
//! unambiguous `android:text` resource reference from a compiled binary XML
//! `TextView`. It does not inflate arbitrary layouts, select qualified
//! configurations, follow aliases, or implement Android's resource runtime.

#[cfg(feature = "androidbox-apk-envelope4")]
use crate::AndroidBoxEnvelopeScratch;
use crate::Error;
use crate::zip;

pub const RESOURCES_ARSC_ENTRY: &[u8] = b"resources.arsc";
pub const DEFAULT_ACTIVITY_LAYOUT_ENTRY: &[u8] = b"res/layout/activity_main.xml";
pub const MAX_RESOURCE_TABLE_BYTES: usize = 512 * 1024;
pub const MAX_BINARY_LAYOUT_BYTES: usize = 128 * 1024;
pub const MAX_RESOURCE_STRING_BYTES: usize = 256;
#[cfg(feature = "androidbox-icon-resources5")]
pub const MAX_LAUNCHER_ICON_PNG_BYTES: usize = 16 * 1024;

const NO_INDEX: u32 = u32::MAX;
const NO_ENTRY: u32 = u32::MAX;
const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_TABLE_TYPE: u16 = 0x0002;
const RES_XML_TYPE: u16 = 0x0003;
const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
const RES_XML_END_NAMESPACE_TYPE: u16 = 0x0101;
const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;
const RES_XML_RESOURCE_MAP_TYPE: u16 = 0x0180;
const RES_TABLE_PACKAGE_TYPE: u16 = 0x0200;
const RES_TABLE_TYPE_TYPE: u16 = 0x0201;
const RES_TABLE_TYPE_SPEC_TYPE: u16 = 0x0202;
const STRING_POOL_SORTED_FLAG: u32 = 1;
const STRING_POOL_UTF8_FLAG: u32 = 1 << 8;
const TYPE_REFERENCE: u8 = 0x01;
const TYPE_STRING: u8 = 0x03;
const TYPE_FLOAT: u8 = 0x04;
const TYPE_DIMENSION: u8 = 0x05;
const TYPE_INT_DEC: u8 = 0x10;
const TYPE_INT_BOOLEAN: u8 = 0x12;
const ENTRY_FLAG_PUBLIC: u16 = 1 << 1;
const ENTRY_FLAG_WEAK: u16 = 1 << 2;
const ENTRY_LAYOUT_NEUTRAL_FLAGS: u16 = ENTRY_FLAG_PUBLIC | ENTRY_FLAG_WEAK;
const TYPE_FLAG_SPARSE: u8 = 1;
const TYPE_FLAG_OFFSET16: u8 = 2;
const ANDROID_URI: &[u8] = b"http://schemas.android.com/apk/res/android";
const ANDROID_PREFIX: &[u8] = b"android";
const TEXT_VIEW: &[u8] = b"TextView";
const BUTTON: &[u8] = b"Button";
const LINEAR_LAYOUT: &[u8] = b"LinearLayout";
const ID_ATTRIBUTE: &[u8] = b"id";
const LAYOUT_WIDTH_ATTRIBUTE: &[u8] = b"layout_width";
const LAYOUT_HEIGHT_ATTRIBUTE: &[u8] = b"layout_height";
const LAYOUT_WEIGHT_ATTRIBUTE: &[u8] = b"layout_weight";
const LAYOUT_MARGIN_ATTRIBUTE: &[u8] = b"layout_margin";
const LAYOUT_MARGIN_LEFT_ATTRIBUTE: &[u8] = b"layout_marginLeft";
const LAYOUT_MARGIN_TOP_ATTRIBUTE: &[u8] = b"layout_marginTop";
const LAYOUT_MARGIN_RIGHT_ATTRIBUTE: &[u8] = b"layout_marginRight";
const LAYOUT_MARGIN_BOTTOM_ATTRIBUTE: &[u8] = b"layout_marginBottom";
const PADDING_ATTRIBUTE: &[u8] = b"padding";
const PADDING_LEFT_ATTRIBUTE: &[u8] = b"paddingLeft";
const PADDING_TOP_ATTRIBUTE: &[u8] = b"paddingTop";
const PADDING_RIGHT_ATTRIBUTE: &[u8] = b"paddingRight";
const PADDING_BOTTOM_ATTRIBUTE: &[u8] = b"paddingBottom";
const ORIENTATION_ATTRIBUTE: &[u8] = b"orientation";
const TEXT_ATTRIBUTE: &[u8] = b"text";
const ANDROID_ID_ATTRIBUTE_ID: u32 = 0x0101_00d0;
const ANDROID_ORIENTATION_ATTRIBUTE_ID: u32 = 0x0101_00c4;
const ANDROID_LAYOUT_WIDTH_ATTRIBUTE_ID: u32 = 0x0101_00f4;
const ANDROID_LAYOUT_HEIGHT_ATTRIBUTE_ID: u32 = 0x0101_00f5;
const ANDROID_LAYOUT_MARGIN_ATTRIBUTE_ID: u32 = 0x0101_00f6;
const ANDROID_LAYOUT_MARGIN_LEFT_ATTRIBUTE_ID: u32 = 0x0101_00f7;
const ANDROID_LAYOUT_MARGIN_TOP_ATTRIBUTE_ID: u32 = 0x0101_00f8;
const ANDROID_LAYOUT_MARGIN_RIGHT_ATTRIBUTE_ID: u32 = 0x0101_00f9;
const ANDROID_LAYOUT_MARGIN_BOTTOM_ATTRIBUTE_ID: u32 = 0x0101_00fa;
const ANDROID_LAYOUT_WEIGHT_ATTRIBUTE_ID: u32 = 0x0101_0181;
const ANDROID_PADDING_ATTRIBUTE_ID: u32 = 0x0101_00d5;
const ANDROID_PADDING_LEFT_ATTRIBUTE_ID: u32 = 0x0101_00d6;
const ANDROID_PADDING_TOP_ATTRIBUTE_ID: u32 = 0x0101_00d7;
const ANDROID_PADDING_RIGHT_ATTRIBUTE_ID: u32 = 0x0101_00d8;
const ANDROID_PADDING_BOTTOM_ATTRIBUTE_ID: u32 = 0x0101_00d9;
const ANDROID_TEXT_ATTRIBUTE_ID: u32 = 0x0101_014f;
const MAX_POOL_STRINGS: u32 = 1_024;
const MAX_POOL_STRING_UNITS: usize = 1_024;
const MAX_PACKAGES: u32 = 4;
const MAX_PACKAGE_CHUNKS: usize = 128;
const MAX_TYPE_ENTRIES: u32 = 4_096;
const MAX_LAYOUT_ATTRIBUTES: u16 = 32;
// Resources-1 admits exactly one root TextView and no child elements. Keeping
// the structural depth bound at one makes that compatibility boundary part of
// the parser contract instead of relying on the repository fixture alone.
const MAX_LAYOUT_DEPTH: usize = 1;
pub const MAX_ACTIVITY_SCENE_NODES: usize = 8;
pub const MAX_ACTIVITY_SCENE_DEPTH: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewKind {
    LinearLayout,
    TextView,
    Button,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutSize {
    MatchParent,
    WrapContent,
    #[cfg(feature = "androidbox-layout-weight15")]
    Zero,
    #[cfg(feature = "androidbox-layout-size18")]
    Exact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutOrientation {
    None,
    Vertical,
    #[cfg(feature = "androidbox-layout-row14")]
    Horizontal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivitySceneNode {
    pub kind: ViewKind,
    pub parent: Option<u8>,
    pub id: u32,
    pub width: LayoutSize,
    pub height: LayoutSize,
    pub orientation: LayoutOrientation,
    pub layout_weight: u8,
    pub layout_margin_dp: u8,
    pub padding_dp: u8,
    pub layout_margin_left_dp: u8,
    pub layout_margin_top_dp: u8,
    pub layout_margin_right_dp: u8,
    pub layout_margin_bottom_dp: u8,
    pub padding_left_dp: u8,
    pub padding_top_dp: u8,
    pub padding_right_dp: u8,
    pub padding_bottom_dp: u8,
    pub exact_width_dp: u8,
    pub exact_height_dp: u8,
    pub text_resource_id: u32,
    pub text: ResourceString,
}

impl ActivitySceneNode {
    const fn empty() -> Self {
        Self {
            kind: ViewKind::LinearLayout,
            parent: None,
            id: 0,
            width: LayoutSize::WrapContent,
            height: LayoutSize::WrapContent,
            orientation: LayoutOrientation::None,
            layout_weight: 0,
            layout_margin_dp: 0,
            padding_dp: 0,
            layout_margin_left_dp: 0,
            layout_margin_top_dp: 0,
            layout_margin_right_dp: 0,
            layout_margin_bottom_dp: 0,
            padding_left_dp: 0,
            padding_top_dp: 0,
            padding_right_dp: 0,
            padding_bottom_dp: 0,
            exact_width_dp: 0,
            exact_height_dp: 0,
            text_resource_id: 0,
            text: ResourceString::empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityScene {
    nodes: [ActivitySceneNode; MAX_ACTIVITY_SCENE_NODES],
    len: u8,
}

impl ActivityScene {
    const fn empty() -> Self {
        Self {
            nodes: [ActivitySceneNode::empty(); MAX_ACTIVITY_SCENE_NODES],
            len: 0,
        }
    }

    pub fn nodes(&self) -> &[ActivitySceneNode] {
        &self.nodes[..usize::from(self.len)]
    }

    pub const fn len(&self) -> u8 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn find_by_id(&self, id: u32) -> Option<(u8, &ActivitySceneNode)> {
        self.nodes()
            .iter()
            .position(|node| id != 0 && node.id == id)
            .and_then(|index| u8::try_from(index).ok())
            .map(|index| (index, &self.nodes[usize::from(index)]))
    }

    pub(crate) fn node(&self, index: u8) -> Option<&ActivitySceneNode> {
        self.nodes().get(usize::from(index))
    }

    pub(crate) fn node_mut(&mut self, index: u8) -> Option<&mut ActivitySceneNode> {
        self.nodes[..usize::from(self.len)].get_mut(usize::from(index))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedActivityScene {
    pub resources_arsc_crc32: u32,
    pub layout_crc32: u32,
    pub scene: ActivityScene,
}

/// A validated resource string copied into fixed-capacity UTF-8 storage.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ResourceString {
    bytes: [u8; MAX_RESOURCE_STRING_BYTES],
    len: u16,
}

impl ResourceString {
    const fn empty() -> Self {
        Self {
            bytes: [0; MAX_RESOURCE_STRING_BYTES],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes())
            .expect("ResourceString contains parser-validated UTF-8")
    }

    pub const fn len(&self) -> u16 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[cfg(feature = "androidbox-string-text12")]
    pub(crate) fn from_dex_ascii(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_RESOURCE_STRING_BYTES || bytes.len() > usize::from(u16::MAX) {
            return Err(Error::DexViewTextTooLong);
        }
        if bytes.is_empty() || !bytes.iter().all(u8::is_ascii) {
            return Err(Error::DexString);
        }
        let mut value = Self::empty();
        value.bytes[..bytes.len()].copy_from_slice(bytes);
        value.len = u16::try_from(bytes.len()).map_err(|_| Error::DexViewTextTooLong)?;
        Ok(value)
    }

    #[cfg(feature = "androidbox-string-builder13")]
    pub(crate) fn from_dex_ascii_with_u8_suffix(prefix: &[u8], value: u8) -> Result<Self, Error> {
        if value == 0 {
            return Err(Error::DexFrameworkState);
        }
        let mut output = Self::from_dex_ascii(prefix)?;
        let mut digits = [0u8; 3];
        let mut remaining = value;
        let mut start = digits.len();
        while remaining != 0 {
            start -= 1;
            digits[start] = b'0' + remaining % 10;
            remaining /= 10;
        }
        let suffix = &digits[start..];
        let old_len = usize::from(output.len);
        let new_len = old_len
            .checked_add(suffix.len())
            .filter(|length| *length <= MAX_RESOURCE_STRING_BYTES)
            .ok_or(Error::DexViewTextTooLong)?;
        output.bytes[old_len..new_len].copy_from_slice(suffix);
        output.len = u16::try_from(new_len).map_err(|_| Error::DexViewTextTooLong)?;
        Ok(output)
    }
}

impl core::fmt::Debug for ResourceString {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("ResourceString")
            .field(&self.as_str())
            .finish()
    }
}

/// The exact resource reference carried by one compiled `TextView`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutTextReference {
    pub resource_id: u32,
}

/// APK entry evidence plus the text resolved through the binary layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedTextView {
    pub resources_arsc_crc32: u32,
    pub layout_crc32: u32,
    pub reference: LayoutTextReference,
    pub text: ResourceString,
}

/// A validated resource table borrowed directly from its stored APK entry.
#[derive(Clone, Copy)]
pub struct ResourceTable<'a> {
    bytes: &'a [u8],
    global_strings: StringPool<'a>,
    package_count: u32,
    packages_start: usize,
}

impl<'a> ResourceTable<'a> {
    /// Parse and structurally validate the supported `resources.arsc` subset.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_RESOURCE_TABLE_BYTES {
            return Err(Error::ResourceTableTooLarge);
        }
        let table = chunk_header(bytes, 0, bytes.len(), Error::ResourceTableHeader)?;
        if table.kind != RES_TABLE_TYPE
            || table.header_size != 12
            || usize_from(table.size, Error::ResourceTableHeader)? != bytes.len()
        {
            return Err(Error::ResourceTableHeader);
        }
        let package_count = read_u32(bytes, 8, Error::ResourceTableHeader)?;
        if package_count == 0 || package_count > MAX_PACKAGES {
            return Err(Error::ResourcePackage);
        }
        let (global_strings, mut cursor) =
            StringPool::parse(bytes, 12, table.end, Error::ResourceStringPool)?;
        let packages_start = cursor;
        let mut packages_seen = 0u32;
        while cursor < table.end {
            let package = chunk_header(bytes, cursor, table.end, Error::ResourcePackage)?;
            if package.kind != RES_TABLE_PACKAGE_TYPE {
                return Err(Error::ResourcePackage);
            }
            validate_package(bytes, cursor, package)?;
            packages_seen = packages_seen.checked_add(1).ok_or(Error::ResourcePackage)?;
            if packages_seen > package_count {
                return Err(Error::ResourcePackage);
            }
            cursor = package.end;
        }
        if cursor != table.end || packages_seen != package_count {
            return Err(Error::ResourcePackage);
        }
        Ok(Self {
            bytes,
            global_strings,
            package_count,
            packages_start,
        })
    }

    /// Resolve one direct string resource in the default configuration.
    pub fn resolve_string(&self, resource_id: u32) -> Result<ResourceString, Error> {
        self.resolve_typed_string(resource_id, b"string")
    }

    /// Resolve one compiled layout resource to its canonical APK entry name.
    pub fn resolve_layout_entry(&self, resource_id: u32) -> Result<ResourceString, Error> {
        let entry = self.resolve_typed_string(resource_id, b"layout")?;
        validate_layout_entry_name(entry.as_bytes())?;
        Ok(entry)
    }

    #[cfg(all(
        feature = "androidbox-icon-resources5",
        not(feature = "androidbox-density-icons7")
    ))]
    pub fn resolve_launcher_icon_entry(
        &self,
        resource_id: u32,
    ) -> Result<(ResourceString, u16), Error> {
        let entry = match self.resolve_typed_string(resource_id, b"drawable") {
            Ok(entry) => entry,
            Err(Error::ResourceValueType) => self.resolve_typed_string(resource_id, b"mipmap")?,
            Err(error) => return Err(error),
        };
        validate_launcher_icon_entry_name(entry.as_bytes(), 0)?;
        Ok((entry, 0))
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub fn resolve_launcher_icon_entry(
        &self,
        resource_id: u32,
    ) -> Result<(ResourceString, u16), Error> {
        let (entry, density_dpi) =
            match self.resolve_launcher_icon_typed_string(resource_id, b"drawable") {
                Ok(entry) => entry,
                Err(Error::ResourceValueType) => {
                    self.resolve_launcher_icon_typed_string(resource_id, b"mipmap")?
                }
                Err(error) => return Err(error),
            };
        validate_launcher_icon_entry_name(entry.as_bytes(), density_dpi)?;
        Ok((entry, density_dpi))
    }

    pub fn validate_id(&self, resource_id: u32) -> Result<(), Error> {
        let package_id = resource_id >> 24;
        let type_id = (resource_id >> 16) & 0xff;
        if package_id == 0 || type_id == 0 {
            return Err(Error::ResourceIdInvalid);
        }
        let mut cursor = self.packages_start;
        let mut packages_seen = 0u32;
        let mut found = false;
        while cursor < self.bytes.len() {
            let package =
                chunk_header(self.bytes, cursor, self.bytes.len(), Error::ResourcePackage)?;
            packages_seen = packages_seen.checked_add(1).ok_or(Error::ResourcePackage)?;
            if read_u32(self.bytes, cursor + 8, Error::ResourcePackage)? == package_id {
                if found {
                    return Err(Error::ResourcePackage);
                }
                validate_id_in_package(self.bytes, cursor, package, type_id, resource_id & 0xffff)?;
                found = true;
            }
            cursor = package.end;
        }
        if packages_seen != self.package_count {
            return Err(Error::ResourcePackage);
        }
        found.then_some(()).ok_or(Error::ResourceNotFound)
    }

    fn resolve_typed_string(
        &self,
        resource_id: u32,
        expected_type_name: &[u8],
    ) -> Result<ResourceString, Error> {
        let package_id = resource_id >> 24;
        let type_id = (resource_id >> 16) & 0xff;
        if package_id == 0 || type_id == 0 {
            return Err(Error::ResourceIdInvalid);
        }

        let mut cursor = self.packages_start;
        let mut packages_seen = 0u32;
        let mut resolved: Option<ResourceString> = None;
        while cursor < self.bytes.len() {
            let package =
                chunk_header(self.bytes, cursor, self.bytes.len(), Error::ResourcePackage)?;
            packages_seen = packages_seen.checked_add(1).ok_or(Error::ResourcePackage)?;
            if read_u32(self.bytes, cursor + 8, Error::ResourcePackage)? == package_id {
                let value = resolve_in_package(
                    self.bytes,
                    cursor,
                    package,
                    type_id,
                    resource_id & 0xffff,
                    self.global_strings,
                    expected_type_name,
                )?;
                if resolved.replace(value).is_some() {
                    return Err(Error::ResourcePackage);
                }
            }
            cursor = package.end;
        }
        if packages_seen != self.package_count {
            return Err(Error::ResourcePackage);
        }
        resolved.ok_or(Error::ResourceNotFound)
    }

    #[cfg(feature = "androidbox-density-icons7")]
    fn resolve_launcher_icon_typed_string(
        &self,
        resource_id: u32,
        expected_type_name: &[u8],
    ) -> Result<(ResourceString, u16), Error> {
        let package_id = resource_id >> 24;
        let type_id = (resource_id >> 16) & 0xff;
        if package_id == 0 || type_id == 0 {
            return Err(Error::ResourceIdInvalid);
        }

        let mut cursor = self.packages_start;
        let mut packages_seen = 0u32;
        let mut resolved: Option<(ResourceString, u16)> = None;
        while cursor < self.bytes.len() {
            let package =
                chunk_header(self.bytes, cursor, self.bytes.len(), Error::ResourcePackage)?;
            packages_seen = packages_seen.checked_add(1).ok_or(Error::ResourcePackage)?;
            if read_u32(self.bytes, cursor + 8, Error::ResourcePackage)? == package_id {
                let value = resolve_launcher_icon_in_package(
                    self.bytes,
                    cursor,
                    package,
                    type_id,
                    resource_id & 0xffff,
                    self.global_strings,
                    expected_type_name,
                )?;
                if resolved.replace(value).is_some() {
                    return Err(Error::ResourcePackage);
                }
            }
            cursor = package.end;
        }
        if packages_seen != self.package_count {
            return Err(Error::ResourcePackage);
        }
        resolved.ok_or(Error::ResourceNotFound)
    }
}

/// Resource-table state borrowed from one bounded APK.
#[derive(Clone, Copy)]
pub struct ApkResources<'a> {
    apk: &'a [u8],
    table: ResourceTable<'a>,
    resources_arsc_crc32: u32,
}

impl<'a> ApkResources<'a> {
    /// Locate the unique stored `resources.arsc` entry and validate it.
    pub fn load(apk: &'a [u8]) -> Result<Self, Error> {
        let entry = zip::required_stored_entry(
            apk,
            RESOURCES_ARSC_ENTRY,
            MAX_RESOURCE_TABLE_BYTES,
            Error::ZipMissingResources,
            Error::ZipDuplicateResources,
            Error::ResourceTableTooLarge,
        )?;
        Ok(Self {
            apk,
            table: ResourceTable::parse(entry.bytes)?,
            resources_arsc_crc32: entry.crc32,
        })
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    pub(crate) fn load_envelope(apk: &'a [u8]) -> Result<Self, Error> {
        let entry = zip::required_envelope_stored_entry(
            apk,
            RESOURCES_ARSC_ENTRY,
            MAX_RESOURCE_TABLE_BYTES,
            Error::ZipMissingResources,
            Error::ZipDuplicateResources,
            Error::ResourceTableTooLarge,
        )?;
        Ok(Self {
            apk,
            table: ResourceTable::parse(entry.bytes)?,
            resources_arsc_crc32: entry.crc32,
        })
    }

    pub const fn resources_arsc_crc32(&self) -> u32 {
        self.resources_arsc_crc32
    }

    pub fn resolve_string(&self, resource_id: u32) -> Result<ResourceString, Error> {
        self.table.resolve_string(resource_id)
    }

    pub fn resolve_layout_entry(&self, resource_id: u32) -> Result<ResourceString, Error> {
        self.table.resolve_layout_entry(resource_id)
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    pub fn resolve_launcher_icon(
        &self,
        resource_id: u32,
        scratch: &mut AndroidBoxEnvelopeScratch,
    ) -> Result<crate::AndroidLauncherIcon, Error> {
        let (icon_entry, density_dpi) = self.table.resolve_launcher_icon_entry(resource_id)?;
        let entry = zip::required_envelope_stored_entry(
            self.apk,
            icon_entry.as_bytes(),
            MAX_LAUNCHER_ICON_PNG_BYTES,
            Error::ZipMissingLauncherIcon,
            Error::ZipDuplicateLauncherIcon,
            Error::LauncherIconTooLarge,
        )?;
        crate::icon::decode_launcher_icon(
            resource_id,
            entry.crc32,
            density_dpi,
            entry.bytes,
            scratch,
        )
    }

    pub fn validate_id(&self, resource_id: u32) -> Result<(), Error> {
        self.table.validate_id(resource_id)
    }

    /// Parse a unique stored binary XML layout and return its one admitted
    /// `TextView/android:text` reference.
    pub fn layout_text_reference(&self, layout_entry: &[u8]) -> Result<LayoutTextReference, Error> {
        let entry = self.layout_entry(layout_entry)?;
        parse_layout_text_reference(entry.bytes)
    }

    /// Resolve the admitted `TextView/android:text` reference through this
    /// APK's own `resources.arsc` default configuration.
    pub fn resolve_text_view(&self, layout_entry: &[u8]) -> Result<ResolvedTextView, Error> {
        let layout = self.layout_entry(layout_entry)?;
        let reference = parse_layout_text_reference(layout.bytes)?;
        let text = self.table.resolve_string(reference.resource_id)?;
        Ok(ResolvedTextView {
            resources_arsc_crc32: self.resources_arsc_crc32,
            layout_crc32: layout.crc32,
            reference,
            text,
        })
    }

    pub fn resolve_activity_scene(
        &self,
        layout_entry: &[u8],
    ) -> Result<ResolvedActivityScene, Error> {
        let layout = self.layout_entry(layout_entry)?;
        let mut scene = parse_activity_scene(layout.bytes)?;
        let mut index = 0usize;
        while index < usize::from(scene.len) {
            let node = scene.nodes[index];
            if node.id != 0 {
                self.table.validate_id(node.id)?;
            }
            if node.text_resource_id != 0 {
                scene.nodes[index].text = self.table.resolve_string(node.text_resource_id)?;
            }
            index += 1;
        }
        Ok(ResolvedActivityScene {
            resources_arsc_crc32: self.resources_arsc_crc32,
            layout_crc32: layout.crc32,
            scene,
        })
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    pub(crate) fn resolve_text_view_envelope(
        &self,
        layout_entry: &[u8],
        scratch: &mut AndroidBoxEnvelopeScratch,
    ) -> Result<(ResolvedTextView, bool), Error> {
        let layout = self.layout_entry_envelope(layout_entry)?;
        let was_deflated = layout.is_deflated();
        let decoded = zip::decode_envelope_entry(layout, scratch)?;
        let reference = parse_layout_text_reference(decoded.as_bytes())?;
        let text = self.table.resolve_string(reference.resource_id)?;
        Ok((
            ResolvedTextView {
                resources_arsc_crc32: self.resources_arsc_crc32,
                layout_crc32: layout.crc32(),
                reference,
                text,
            },
            was_deflated,
        ))
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    pub(crate) fn resolve_activity_scene_envelope(
        &self,
        layout_entry: &[u8],
        scratch: &mut AndroidBoxEnvelopeScratch,
    ) -> Result<(ResolvedActivityScene, bool), Error> {
        let layout = self.layout_entry_envelope(layout_entry)?;
        let was_deflated = layout.is_deflated();
        let decoded = zip::decode_envelope_entry(layout, scratch)?;
        let mut scene = parse_activity_scene(decoded.as_bytes())?;
        let mut index = 0usize;
        while index < usize::from(scene.len) {
            let node = scene.nodes[index];
            if node.id != 0 {
                self.table.validate_id(node.id)?;
            }
            if node.text_resource_id != 0 {
                scene.nodes[index].text = self.table.resolve_string(node.text_resource_id)?;
            }
            index += 1;
        }
        Ok((
            ResolvedActivityScene {
                resources_arsc_crc32: self.resources_arsc_crc32,
                layout_crc32: layout.crc32(),
                scene,
            },
            was_deflated,
        ))
    }

    fn layout_entry(&self, layout_entry: &[u8]) -> Result<zip::StoredEntry<'a>, Error> {
        validate_layout_entry_name(layout_entry)?;
        zip::required_stored_entry(
            self.apk,
            layout_entry,
            MAX_BINARY_LAYOUT_BYTES,
            Error::ZipMissingLayout,
            Error::ZipDuplicateLayout,
            Error::LayoutTooLarge,
        )
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn layout_entry_envelope(&self, layout_entry: &[u8]) -> Result<zip::EnvelopeEntry<'a>, Error> {
        validate_layout_entry_name(layout_entry)?;
        zip::required_envelope_entry(
            self.apk,
            layout_entry,
            MAX_BINARY_LAYOUT_BYTES,
            Error::ZipMissingLayout,
            Error::ZipDuplicateLayout,
            Error::LayoutTooLarge,
        )
    }
}

/// Resolve a stored binary XML layout's `TextView/android:text` reference
/// through the same APK's stored `resources.arsc`.
pub fn resolve_apk_text_view(apk: &[u8], layout_entry: &[u8]) -> Result<ResolvedTextView, Error> {
    ApkResources::load(apk)?.resolve_text_view(layout_entry)
}

/// Parse one bounded binary XML layout without accessing an APK container.
pub fn parse_layout_text_reference(bytes: &[u8]) -> Result<LayoutTextReference, Error> {
    if bytes.len() > MAX_BINARY_LAYOUT_BYTES {
        return Err(Error::LayoutTooLarge);
    }
    let document = chunk_header(bytes, 0, bytes.len(), Error::LayoutHeader)?;
    if document.kind != RES_XML_TYPE
        || document.header_size != 8
        || usize_from(document.size, Error::LayoutHeader)? != bytes.len()
    {
        return Err(Error::LayoutHeader);
    }
    let (strings, mut cursor) = StringPool::parse(bytes, 8, document.end, Error::LayoutStringPool)?;
    let (resource_map, next) = ResourceMap::parse(bytes, cursor, document.end, strings.count)?;
    cursor = next;
    let namespace = parse_namespace(bytes, cursor, document.end, strings, true)?;
    cursor = namespace.end;

    let mut stack = [ElementName::empty(); MAX_LAYOUT_DEPTH];
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut text_reference: Option<LayoutTextReference> = None;
    let mut namespace_closed = false;

    while cursor < document.end {
        let chunk = chunk_header(bytes, cursor, document.end, Error::LayoutStructure)?;
        match chunk.kind {
            RES_XML_START_ELEMENT_TYPE => {
                if root_closed || depth >= MAX_LAYOUT_DEPTH {
                    return Err(Error::LayoutStructure);
                }
                let parsed = parse_start_element(bytes, cursor, chunk, strings, resource_map)?;
                if depth == 0 {
                    if root_seen {
                        return Err(Error::LayoutStructure);
                    }
                    if parsed.namespace != NO_INDEX {
                        return Err(Error::LayoutStructure);
                    }
                    if !strings.equals_ascii(parsed.name, TEXT_VIEW)? {
                        return Err(Error::LayoutTextViewMissing);
                    }
                    root_seen = true;
                }
                stack[depth] = ElementName {
                    namespace: parsed.namespace,
                    name: parsed.name,
                };
                depth += 1;
                if let Some(reference) = parsed.text_reference
                    && text_reference.replace(reference).is_some()
                {
                    return Err(Error::LayoutTextViewDuplicate);
                }
                cursor = chunk.end;
            }
            RES_XML_END_ELEMENT_TYPE => {
                if depth == 0 {
                    return Err(Error::LayoutStructure);
                }
                let ended = parse_end_element(bytes, cursor, chunk, strings)?;
                if ended != stack[depth - 1] {
                    return Err(Error::LayoutStructure);
                }
                depth -= 1;
                stack[depth] = ElementName::empty();
                if depth == 0 {
                    root_closed = true;
                }
                cursor = chunk.end;
            }
            RES_XML_END_NAMESPACE_TYPE => {
                if depth != 0 || !root_seen || !root_closed {
                    return Err(Error::LayoutStructure);
                }
                let ended = parse_namespace(bytes, cursor, document.end, strings, false)?;
                if ended.prefix != namespace.prefix || ended.uri != namespace.uri {
                    return Err(Error::LayoutStructure);
                }
                cursor = ended.end;
                namespace_closed = true;
                if cursor != document.end {
                    return Err(Error::LayoutStructure);
                }
            }
            kind => return Err(Error::LayoutUnknownChunk(kind)),
        }
    }
    if cursor != document.end || depth != 0 || !root_seen || !root_closed || !namespace_closed {
        return Err(Error::LayoutStructure);
    }
    text_reference.ok_or(Error::LayoutTextViewMissing)
}

fn parse_activity_scene(bytes: &[u8]) -> Result<ActivityScene, Error> {
    if bytes.len() > MAX_BINARY_LAYOUT_BYTES {
        return Err(Error::LayoutTooLarge);
    }
    let document = chunk_header(bytes, 0, bytes.len(), Error::LayoutHeader)?;
    if document.kind != RES_XML_TYPE
        || document.header_size != 8
        || usize_from(document.size, Error::LayoutHeader)? != bytes.len()
    {
        return Err(Error::LayoutHeader);
    }
    let (strings, mut cursor) = StringPool::parse(bytes, 8, document.end, Error::LayoutStringPool)?;
    let (resource_map, next) = ResourceMap::parse(bytes, cursor, document.end, strings.count)?;
    cursor = next;
    let namespace = parse_namespace(bytes, cursor, document.end, strings, true)?;
    cursor = namespace.end;

    let mut scene = ActivityScene::empty();
    let mut element_stack = [ElementName::empty(); MAX_ACTIVITY_SCENE_DEPTH];
    let mut node_stack = [0u8; MAX_ACTIVITY_SCENE_DEPTH];
    let mut depth = 0usize;
    let mut root_closed = false;
    let mut namespace_closed = false;
    let mut text_views = 0u8;
    let mut buttons = 0u8;

    while cursor < document.end {
        let chunk = chunk_header(bytes, cursor, document.end, Error::LayoutStructure)?;
        match chunk.kind {
            RES_XML_START_ELEMENT_TYPE => {
                if root_closed
                    || depth >= MAX_ACTIVITY_SCENE_DEPTH
                    || usize::from(scene.len) >= MAX_ACTIVITY_SCENE_NODES
                {
                    return Err(Error::LayoutStructure);
                }
                let parsed =
                    parse_activity_scene_start(bytes, cursor, chunk, strings, resource_map)?;
                if parsed.namespace != NO_INDEX {
                    return Err(Error::LayoutStructure);
                }
                let parent = if depth == 0 {
                    if scene.len != 0
                        || parsed.node.kind != ViewKind::LinearLayout
                        || parsed.node.orientation != LayoutOrientation::Vertical
                    {
                        return Err(Error::LayoutStructure);
                    }
                    None
                } else {
                    let parent_index = node_stack[depth - 1];
                    if scene.node(parent_index).ok_or(Error::LayoutStructure)?.kind
                        != ViewKind::LinearLayout
                    {
                        return Err(Error::LayoutStructure);
                    }
                    Some(parent_index)
                };
                #[cfg(feature = "androidbox-layout-weight15")]
                if parsed.node.layout_weight != 0 {
                    let parent_index = parent.ok_or(Error::LayoutAttribute)?;
                    let parent_node = scene.node(parent_index).ok_or(Error::LayoutStructure)?;
                    if parsed.node.kind != ViewKind::Button
                        || parent_node.orientation != LayoutOrientation::Horizontal
                    {
                        return Err(Error::LayoutAttribute);
                    }
                }
                #[cfg(feature = "androidbox-layout-mixed19")]
                if let Some(parent_index) = parent {
                    let parent_node = scene.node(parent_index).ok_or(Error::LayoutStructure)?;
                    if parent_node.orientation == LayoutOrientation::Horizontal {
                        if !mixed_horizontal_child_is_canonical(parsed.node) {
                            return Err(Error::LayoutAttribute);
                        }
                    }
                }
                if parsed.node.id != 0 && scene.nodes().iter().any(|node| node.id == parsed.node.id)
                {
                    return Err(Error::LayoutAttribute);
                }
                let node_index = scene.len;
                let mut node = parsed.node;
                node.parent = parent;
                scene.nodes[usize::from(node_index)] = node;
                scene.len = scene.len.checked_add(1).ok_or(Error::LayoutStructure)?;
                match node.kind {
                    ViewKind::TextView => {
                        text_views = text_views.checked_add(1).ok_or(Error::LayoutStructure)?;
                    }
                    ViewKind::Button => {
                        buttons = buttons.checked_add(1).ok_or(Error::LayoutStructure)?;
                    }
                    ViewKind::LinearLayout => {}
                }
                element_stack[depth] = ElementName {
                    namespace: parsed.namespace,
                    name: parsed.name,
                };
                node_stack[depth] = node_index;
                depth += 1;
                cursor = chunk.end;
            }
            RES_XML_END_ELEMENT_TYPE => {
                if depth == 0 {
                    return Err(Error::LayoutStructure);
                }
                let ended = parse_end_element(bytes, cursor, chunk, strings)?;
                if ended != element_stack[depth - 1] {
                    return Err(Error::LayoutStructure);
                }
                depth -= 1;
                element_stack[depth] = ElementName::empty();
                node_stack[depth] = 0;
                if depth == 0 {
                    root_closed = true;
                }
                cursor = chunk.end;
            }
            RES_XML_END_NAMESPACE_TYPE => {
                if depth != 0 || scene.is_empty() || !root_closed {
                    return Err(Error::LayoutStructure);
                }
                let ended = parse_namespace(bytes, cursor, document.end, strings, false)?;
                if ended.prefix != namespace.prefix || ended.uri != namespace.uri {
                    return Err(Error::LayoutStructure);
                }
                cursor = ended.end;
                namespace_closed = true;
                if cursor != document.end {
                    return Err(Error::LayoutStructure);
                }
            }
            kind => return Err(Error::LayoutUnknownChunk(kind)),
        }
    }
    if cursor != document.end
        || depth != 0
        || !root_closed
        || !namespace_closed
        || text_views == 0
        || buttons == 0
    {
        return Err(Error::LayoutStructure);
    }
    Ok(scene)
}

struct ActivitySceneStart {
    namespace: u32,
    name: u32,
    node: ActivitySceneNode,
}

#[derive(Clone, Copy)]
struct SceneAttributes {
    id: Option<u32>,
    id_attribute_index: Option<u16>,
    width: Option<(LayoutSize, u8)>,
    height: Option<(LayoutSize, u8)>,
    orientation: Option<LayoutOrientation>,
    layout_weight: Option<u8>,
    layout_margin_dp: Option<u8>,
    padding_dp: Option<u8>,
    layout_margin_left_dp: Option<u8>,
    layout_margin_top_dp: Option<u8>,
    layout_margin_right_dp: Option<u8>,
    layout_margin_bottom_dp: Option<u8>,
    padding_left_dp: Option<u8>,
    padding_top_dp: Option<u8>,
    padding_right_dp: Option<u8>,
    padding_bottom_dp: Option<u8>,
    text: Option<u32>,
}

impl SceneAttributes {
    const fn empty() -> Self {
        Self {
            id: None,
            id_attribute_index: None,
            width: None,
            height: None,
            orientation: None,
            layout_weight: None,
            layout_margin_dp: None,
            padding_dp: None,
            layout_margin_left_dp: None,
            layout_margin_top_dp: None,
            layout_margin_right_dp: None,
            layout_margin_bottom_dp: None,
            padding_left_dp: None,
            padding_top_dp: None,
            padding_right_dp: None,
            padding_bottom_dp: None,
            text: None,
        }
    }
}

fn parse_activity_scene_start(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    strings: StringPool<'_>,
    resource_map: ResourceMap<'_>,
) -> Result<ActivitySceneStart, Error> {
    if chunk.header_size != 16 || chunk.size < 36 {
        return Err(Error::LayoutStructure);
    }
    validate_node(bytes, offset, Error::LayoutStructure)?;
    let namespace = read_u32(bytes, offset + 16, Error::LayoutStructure)?;
    let name = read_u32(bytes, offset + 20, Error::LayoutStructure)?;
    if namespace != NO_INDEX {
        strings.get(namespace)?;
    }
    strings.get(name)?;
    let kind = if strings.equals_ascii(name, LINEAR_LAYOUT)? {
        ViewKind::LinearLayout
    } else if strings.equals_ascii(name, TEXT_VIEW)? {
        ViewKind::TextView
    } else if strings.equals_ascii(name, BUTTON)? {
        ViewKind::Button
    } else {
        return Err(Error::LayoutStructure);
    };
    let attribute_start = read_u16(bytes, offset + 24, Error::LayoutStructure)?;
    let attribute_size = read_u16(bytes, offset + 26, Error::LayoutStructure)?;
    let attribute_count = read_u16(bytes, offset + 28, Error::LayoutStructure)?;
    let id_index = read_u16(bytes, offset + 30, Error::LayoutStructure)?;
    let class_index = read_u16(bytes, offset + 32, Error::LayoutStructure)?;
    let style_index = read_u16(bytes, offset + 34, Error::LayoutStructure)?;
    if attribute_start != 20
        || attribute_size != 20
        || attribute_count > MAX_LAYOUT_ATTRIBUTES
        || id_index > attribute_count
        || class_index != 0
        || style_index != 0
    {
        return Err(Error::LayoutStructure);
    }
    let attributes_start = offset
        .checked_add(16 + usize::from(attribute_start))
        .ok_or(Error::LayoutStructure)?;
    let expected_end = attributes_start
        .checked_add(
            usize::from(attribute_count)
                .checked_mul(usize::from(attribute_size))
                .ok_or(Error::LayoutStructure)?,
        )
        .ok_or(Error::LayoutStructure)?;
    if expected_end != chunk.end {
        return Err(Error::LayoutStructure);
    }
    let mut parsed = SceneAttributes::empty();
    let mut attribute_index = 0u16;
    while attribute_index < attribute_count {
        let attribute_offset = attributes_start
            .checked_add(
                usize::from(attribute_index)
                    .checked_mul(20)
                    .ok_or(Error::LayoutAttribute)?,
            )
            .ok_or(Error::LayoutAttribute)?;
        let attribute = parse_layout_attribute(bytes, attribute_offset, strings, resource_map)?;
        if attribute.namespace == NO_INDEX
            || !strings.equals_ascii(attribute.namespace, ANDROID_URI)?
        {
            return Err(Error::LayoutAttribute);
        }
        let attribute_id = resource_map.get(attribute.name)?;
        if strings.equals_ascii(attribute.name, ID_ATTRIBUTE)?
            && attribute_id == ANDROID_ID_ATTRIBUTE_ID
        {
            let id = strict_reference(attribute)?;
            if parsed.id.replace(id).is_some()
                || parsed
                    .id_attribute_index
                    .replace(attribute_index + 1)
                    .is_some()
            {
                return Err(Error::LayoutAttribute);
            }
        } else if strings.equals_ascii(attribute.name, LAYOUT_WIDTH_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_WIDTH_ATTRIBUTE_ID
        {
            let width = strict_layout_size_and_dp(attribute)?;
            if parsed.width.replace(width).is_some() {
                return Err(Error::LayoutAttribute);
            }
        } else if strings.equals_ascii(attribute.name, LAYOUT_HEIGHT_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_HEIGHT_ATTRIBUTE_ID
        {
            let height = strict_layout_size_and_dp(attribute)?;
            if parsed.height.replace(height).is_some() {
                return Err(Error::LayoutAttribute);
            }
        } else if cfg!(feature = "androidbox-layout-weight15")
            && strings.equals_ascii(attribute.name, LAYOUT_WEIGHT_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_WEIGHT_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-weight15")]
            {
                let weight = strict_layout_weight(attribute)?;
                if parsed.layout_weight.replace(weight).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-spacing16")
            && strings.equals_ascii(attribute.name, LAYOUT_MARGIN_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_MARGIN_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-spacing16")]
            {
                let margin = strict_uniform_dp(attribute)?;
                if parsed.layout_margin_dp.replace(margin).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, LAYOUT_MARGIN_LEFT_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_MARGIN_LEFT_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let margin = strict_uniform_dp(attribute)?;
                if parsed.layout_margin_left_dp.replace(margin).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, LAYOUT_MARGIN_TOP_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_MARGIN_TOP_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let margin = strict_uniform_dp(attribute)?;
                if parsed.layout_margin_top_dp.replace(margin).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, LAYOUT_MARGIN_RIGHT_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_MARGIN_RIGHT_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let margin = strict_uniform_dp(attribute)?;
                if parsed.layout_margin_right_dp.replace(margin).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, LAYOUT_MARGIN_BOTTOM_ATTRIBUTE)?
            && attribute_id == ANDROID_LAYOUT_MARGIN_BOTTOM_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let margin = strict_uniform_dp(attribute)?;
                if parsed.layout_margin_bottom_dp.replace(margin).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-spacing16")
            && strings.equals_ascii(attribute.name, PADDING_ATTRIBUTE)?
            && attribute_id == ANDROID_PADDING_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-spacing16")]
            {
                let padding = strict_uniform_dp(attribute)?;
                if parsed.padding_dp.replace(padding).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, PADDING_LEFT_ATTRIBUTE)?
            && attribute_id == ANDROID_PADDING_LEFT_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let padding = strict_uniform_dp(attribute)?;
                if parsed.padding_left_dp.replace(padding).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, PADDING_TOP_ATTRIBUTE)?
            && attribute_id == ANDROID_PADDING_TOP_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let padding = strict_uniform_dp(attribute)?;
                if parsed.padding_top_dp.replace(padding).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, PADDING_RIGHT_ATTRIBUTE)?
            && attribute_id == ANDROID_PADDING_RIGHT_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let padding = strict_uniform_dp(attribute)?;
                if parsed.padding_right_dp.replace(padding).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if cfg!(feature = "androidbox-layout-directional17")
            && strings.equals_ascii(attribute.name, PADDING_BOTTOM_ATTRIBUTE)?
            && attribute_id == ANDROID_PADDING_BOTTOM_ATTRIBUTE_ID
        {
            #[cfg(feature = "androidbox-layout-directional17")]
            {
                let padding = strict_uniform_dp(attribute)?;
                if parsed.padding_bottom_dp.replace(padding).is_some() {
                    return Err(Error::LayoutAttribute);
                }
            }
        } else if strings.equals_ascii(attribute.name, ORIENTATION_ATTRIBUTE)?
            && attribute_id == ANDROID_ORIENTATION_ATTRIBUTE_ID
        {
            if attribute.raw_value != NO_INDEX || attribute.value_type != TYPE_INT_DEC {
                return Err(Error::LayoutAttribute);
            }
            let orientation = match attribute.data {
                1 => LayoutOrientation::Vertical,
                #[cfg(feature = "androidbox-layout-row14")]
                0 => LayoutOrientation::Horizontal,
                _ => return Err(Error::LayoutAttribute),
            };
            if parsed.orientation.replace(orientation).is_some() {
                return Err(Error::LayoutAttribute);
            }
        } else if strings.equals_ascii(attribute.name, TEXT_ATTRIBUTE)?
            && attribute_id == ANDROID_TEXT_ATTRIBUTE_ID
        {
            let text = strict_reference(attribute)?;
            if parsed.text.replace(text).is_some() {
                return Err(Error::LayoutAttribute);
            }
        } else {
            return Err(Error::LayoutAttribute);
        }
        attribute_index += 1;
    }
    let (width, exact_width_dp) = parsed.width.ok_or(Error::LayoutAttribute)?;
    let (height, exact_height_dp) = parsed.height.ok_or(Error::LayoutAttribute)?;
    let layout_weight = parsed.layout_weight.unwrap_or(0);
    let layout_margin_dp = parsed.layout_margin_dp.unwrap_or(0);
    let padding_dp = parsed.padding_dp.unwrap_or(0);
    let layout_margin_left_dp = parsed.layout_margin_left_dp.unwrap_or(layout_margin_dp);
    let layout_margin_top_dp = parsed.layout_margin_top_dp.unwrap_or(layout_margin_dp);
    let layout_margin_right_dp = parsed.layout_margin_right_dp.unwrap_or(layout_margin_dp);
    let layout_margin_bottom_dp = parsed.layout_margin_bottom_dp.unwrap_or(layout_margin_dp);
    let padding_left_dp = parsed.padding_left_dp.unwrap_or(padding_dp);
    let padding_top_dp = parsed.padding_top_dp.unwrap_or(padding_dp);
    let padding_right_dp = parsed.padding_right_dp.unwrap_or(padding_dp);
    let padding_bottom_dp = parsed.padding_bottom_dp.unwrap_or(padding_dp);
    let has_margin = (layout_margin_left_dp
        | layout_margin_top_dp
        | layout_margin_right_dp
        | layout_margin_bottom_dp)
        != 0;
    let has_padding =
        (padding_left_dp | padding_top_dp | padding_right_dp | padding_bottom_dp) != 0;
    #[cfg(feature = "androidbox-layout-weight15")]
    if height == LayoutSize::Zero || (width == LayoutSize::Zero) != (layout_weight != 0) {
        return Err(Error::LayoutAttribute);
    }
    if id_index != parsed.id_attribute_index.unwrap_or(0) {
        return Err(Error::LayoutStructure);
    }
    let (id, orientation, text_resource_id) = match kind {
        ViewKind::LinearLayout => {
            if parsed.text.is_some()
                || parsed.orientation.is_none()
                || layout_weight != 0
                || has_margin
            {
                return Err(Error::LayoutAttribute);
            }
            (parsed.id.unwrap_or(0), parsed.orientation.unwrap(), 0)
        }
        ViewKind::TextView | ViewKind::Button => {
            if parsed.orientation.is_some()
                || has_padding
                || (kind == ViewKind::TextView && (layout_weight != 0 || has_margin))
            {
                return Err(Error::LayoutAttribute);
            }
            (
                parsed.id.ok_or(Error::LayoutAttribute)?,
                LayoutOrientation::None,
                parsed.text.ok_or(Error::LayoutAttribute)?,
            )
        }
    };
    Ok(ActivitySceneStart {
        namespace,
        name,
        node: ActivitySceneNode {
            kind,
            parent: None,
            id,
            width,
            height,
            orientation,
            layout_weight,
            layout_margin_dp,
            padding_dp,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            exact_width_dp,
            exact_height_dp,
            text_resource_id,
            text: ResourceString::empty(),
        },
    })
}

#[cfg(feature = "androidbox-layout-mixed19")]
fn mixed_horizontal_child_is_canonical(node: ActivitySceneNode) -> bool {
    let fixed =
        node.kind == ViewKind::Button && node.layout_weight == 0 && node.width == LayoutSize::Exact;
    let weighted = node.kind == ViewKind::Button
        && (1..=8).contains(&node.layout_weight)
        && node.width == LayoutSize::Zero;
    fixed || weighted
}

fn strict_reference(attribute: LayoutAttribute) -> Result<u32, Error> {
    if attribute.raw_value != NO_INDEX
        || attribute.value_type != TYPE_REFERENCE
        || attribute.data == 0
    {
        return Err(Error::LayoutAttribute);
    }
    Ok(attribute.data)
}

#[cfg(test)]
fn strict_layout_size(attribute: LayoutAttribute) -> Result<LayoutSize, Error> {
    strict_layout_size_and_dp(attribute).map(|(size, _)| size)
}

fn strict_layout_size_and_dp(attribute: LayoutAttribute) -> Result<(LayoutSize, u8), Error> {
    if attribute.raw_value != NO_INDEX {
        return Err(Error::LayoutAttribute);
    }
    #[cfg(feature = "androidbox-layout-weight15")]
    if attribute.value_type == TYPE_DIMENSION && attribute.data == 1 {
        return Ok((LayoutSize::Zero, 0));
    }
    #[cfg(feature = "androidbox-layout-size18")]
    if attribute.value_type == TYPE_DIMENSION && attribute.data & 0xff == 1 {
        let value = attribute.data >> 8;
        let exact_dp = u8::try_from(value)
            .ok()
            .filter(|value| *value != 0)
            .ok_or(Error::LayoutAttribute)?;
        return Ok((LayoutSize::Exact, exact_dp));
    }
    if attribute.value_type != TYPE_INT_DEC {
        return Err(Error::LayoutAttribute);
    }
    match attribute.data {
        u32::MAX => Ok((LayoutSize::MatchParent, 0)),
        value if value == (-2_i32) as u32 => Ok((LayoutSize::WrapContent, 0)),
        _ => Err(Error::LayoutAttribute),
    }
}

#[cfg(feature = "androidbox-layout-weight15")]
fn strict_layout_weight(attribute: LayoutAttribute) -> Result<u8, Error> {
    if attribute.raw_value != NO_INDEX || attribute.value_type != TYPE_FLOAT {
        return Err(Error::LayoutAttribute);
    }
    match attribute.data {
        0x3f80_0000 => Ok(1),
        0x4000_0000 => Ok(2),
        0x4040_0000 => Ok(3),
        0x4080_0000 => Ok(4),
        0x40a0_0000 => Ok(5),
        0x40c0_0000 => Ok(6),
        0x40e0_0000 => Ok(7),
        0x4100_0000 => Ok(8),
        _ => Err(Error::LayoutAttribute),
    }
}

#[cfg(feature = "androidbox-layout-spacing16")]
fn strict_uniform_dp(attribute: LayoutAttribute) -> Result<u8, Error> {
    if attribute.raw_value != NO_INDEX
        || attribute.value_type != TYPE_DIMENSION
        || attribute.data & 0xff != 1
    {
        return Err(Error::LayoutAttribute);
    }
    let value = attribute.data >> 8;
    u8::try_from(value)
        .ok()
        .filter(|value| *value <= 16)
        .ok_or(Error::LayoutAttribute)
}

#[derive(Clone, Copy)]
enum Encoding {
    Utf8,
    Utf16,
}

#[derive(Clone, Copy)]
struct PoolString<'a> {
    bytes: &'a [u8],
    utf16_units: usize,
    encoding: Encoding,
}

impl PoolString<'_> {
    fn equals_ascii(self, expected: &[u8]) -> bool {
        match self.encoding {
            Encoding::Utf8 => self.bytes == expected,
            Encoding::Utf16 => {
                if self.utf16_units != expected.len() {
                    return false;
                }
                let mut index = 0usize;
                while index < expected.len() {
                    let offset = index * 2;
                    if self.bytes[offset] != expected[index] || self.bytes[offset + 1] != 0 {
                        return false;
                    }
                    index += 1;
                }
                true
            }
        }
    }

    fn to_resource_string(self) -> Result<ResourceString, Error> {
        let mut output = ResourceString::empty();
        let mut length = 0usize;
        match self.encoding {
            Encoding::Utf8 => {
                if self.bytes.len() > MAX_RESOURCE_STRING_BYTES {
                    return Err(Error::ResourceStringTooLong);
                }
                core::str::from_utf8(self.bytes).map_err(|_| Error::ResourceString)?;
                output.bytes[..self.bytes.len()].copy_from_slice(self.bytes);
                length = self.bytes.len();
            }
            Encoding::Utf16 => {
                let mut cursor = 0usize;
                while cursor < self.bytes.len() {
                    let first = u16::from_le_bytes([self.bytes[cursor], self.bytes[cursor + 1]]);
                    cursor += 2;
                    let scalar = if (0xd800..=0xdbff).contains(&first) {
                        if cursor >= self.bytes.len() {
                            return Err(Error::ResourceString);
                        }
                        let second =
                            u16::from_le_bytes([self.bytes[cursor], self.bytes[cursor + 1]]);
                        cursor += 2;
                        if !(0xdc00..=0xdfff).contains(&second) {
                            return Err(Error::ResourceString);
                        }
                        0x1_0000
                            + ((u32::from(first) - 0xd800) << 10)
                            + (u32::from(second) - 0xdc00)
                    } else {
                        if (0xdc00..=0xdfff).contains(&first) {
                            return Err(Error::ResourceString);
                        }
                        u32::from(first)
                    };
                    let character = char::from_u32(scalar).ok_or(Error::ResourceString)?;
                    let mut encoded = [0u8; 4];
                    let utf8 = character.encode_utf8(&mut encoded).as_bytes();
                    let end = length
                        .checked_add(utf8.len())
                        .ok_or(Error::ResourceStringTooLong)?;
                    if end > MAX_RESOURCE_STRING_BYTES {
                        return Err(Error::ResourceStringTooLong);
                    }
                    output.bytes[length..end].copy_from_slice(utf8);
                    length = end;
                }
            }
        }
        output.len = u16::try_from(length).map_err(|_| Error::ResourceStringTooLong)?;
        Ok(output)
    }
}

#[derive(Clone, Copy)]
struct StringPool<'a> {
    bytes: &'a [u8],
    end: usize,
    offsets_start: usize,
    strings_start: usize,
    count: u32,
    encoding: Encoding,
    error: Error,
}

impl<'a> StringPool<'a> {
    fn parse(
        bytes: &'a [u8],
        offset: usize,
        limit: usize,
        error: Error,
    ) -> Result<(Self, usize), Error> {
        let chunk = chunk_header(bytes, offset, limit, error)?;
        if chunk.kind != RES_STRING_POOL_TYPE || chunk.header_size != 28 {
            return Err(error);
        }
        let count = read_u32(bytes, offset + 8, error)?;
        let style_count = read_u32(bytes, offset + 12, error)?;
        let flags = read_u32(bytes, offset + 16, error)?;
        let strings_offset = read_u32(bytes, offset + 20, error)?;
        let styles_offset = read_u32(bytes, offset + 24, error)?;
        if count > MAX_POOL_STRINGS
            || style_count != 0
            || styles_offset != 0
            || flags & !(STRING_POOL_SORTED_FLAG | STRING_POOL_UTF8_FLAG) != 0
        {
            return Err(error);
        }
        let offsets_size = usize_from(count, error)?.checked_mul(4).ok_or(error)?;
        let minimum_strings_offset = 28usize.checked_add(offsets_size).ok_or(error)?;
        let relative_strings = usize_from(strings_offset, error)?;
        if relative_strings < minimum_strings_offset
            || relative_strings >= usize_from(chunk.size, error)?
        {
            return Err(error);
        }
        let offsets_start = offset.checked_add(28).ok_or(error)?;
        let strings_start = offset.checked_add(relative_strings).ok_or(error)?;
        if bytes[offsets_start + offsets_size..strings_start]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(error);
        }
        let pool = Self {
            bytes,
            end: chunk.end,
            offsets_start,
            strings_start,
            count,
            encoding: if flags & STRING_POOL_UTF8_FLAG != 0 {
                Encoding::Utf8
            } else {
                Encoding::Utf16
            },
            error,
        };
        pool.validate()?;
        Ok((pool, chunk.end))
    }

    fn validate(self) -> Result<(), Error> {
        if self.count == 0 {
            if self.strings_start != self.end {
                return Err(self.error);
            }
            return Ok(());
        }
        let mut expected = 0usize;
        let mut index = 0u32;
        while index < self.count {
            let relative = self.relative_offset(index)?;
            if relative != expected {
                return Err(self.error);
            }
            let (_, end) = parse_pool_string(
                self.bytes,
                self.strings_start.checked_add(relative).ok_or(self.error)?,
                self.end,
                self.encoding,
                self.error,
            )?;
            expected = end.checked_sub(self.strings_start).ok_or(self.error)?;
            index += 1;
        }
        if expected > self.end - self.strings_start
            || self.end - self.strings_start - expected > 3
            || self.bytes[self.strings_start + expected..self.end]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(self.error);
        }
        Ok(())
    }

    fn relative_offset(self, index: u32) -> Result<usize, Error> {
        if index >= self.count {
            return Err(self.error);
        }
        let offset = self
            .offsets_start
            .checked_add(
                usize_from(index, self.error)?
                    .checked_mul(4)
                    .ok_or(self.error)?,
            )
            .ok_or(self.error)?;
        usize_from(read_u32(self.bytes, offset, self.error)?, self.error)
    }

    fn get(self, index: u32) -> Result<PoolString<'a>, Error> {
        let relative = self.relative_offset(index)?;
        let (value, _) = parse_pool_string(
            self.bytes,
            self.strings_start.checked_add(relative).ok_or(self.error)?,
            self.end,
            self.encoding,
            self.error,
        )?;
        Ok(value)
    }

    fn equals_ascii(self, index: u32, expected: &[u8]) -> Result<bool, Error> {
        Ok(self.get(index)?.equals_ascii(expected))
    }
}

fn parse_pool_string<'a>(
    bytes: &'a [u8],
    offset: usize,
    end: usize,
    encoding: Encoding,
    error: Error,
) -> Result<(PoolString<'a>, usize), Error> {
    match encoding {
        Encoding::Utf8 => {
            let (utf16_units, next) = read_length8(bytes, offset, end, error)?;
            let (byte_length, data_start) = read_length8(bytes, next, end, error)?;
            if utf16_units > MAX_POOL_STRING_UNITS || byte_length > MAX_POOL_STRING_UNITS * 4 {
                return Err(error);
            }
            let data_end = data_start.checked_add(byte_length).ok_or(error)?;
            if data_end >= end || *bytes.get(data_end).ok_or(error)? != 0 {
                return Err(error);
            }
            let value = bytes.get(data_start..data_end).ok_or(error)?;
            let text = core::str::from_utf8(value).map_err(|_| error)?;
            if text.chars().any(|character| character == '\0')
                || text.encode_utf16().count() != utf16_units
            {
                return Err(error);
            }
            Ok((
                PoolString {
                    bytes: value,
                    utf16_units,
                    encoding,
                },
                data_end + 1,
            ))
        }
        Encoding::Utf16 => {
            let (utf16_units, data_start) = read_length16(bytes, offset, end, error)?;
            if utf16_units > MAX_POOL_STRING_UNITS {
                return Err(error);
            }
            let byte_length = utf16_units.checked_mul(2).ok_or(error)?;
            let data_end = data_start.checked_add(byte_length).ok_or(error)?;
            let terminator_end = data_end.checked_add(2).ok_or(error)?;
            if terminator_end > end || read_u16(bytes, data_end, error)? != 0 {
                return Err(error);
            }
            let value = bytes.get(data_start..data_end).ok_or(error)?;
            validate_utf16(value, error)?;
            Ok((
                PoolString {
                    bytes: value,
                    utf16_units,
                    encoding,
                },
                terminator_end,
            ))
        }
    }
}

fn validate_utf16(bytes: &[u8], error: Error) -> Result<(), Error> {
    if !bytes.len().is_multiple_of(2) {
        return Err(error);
    }
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let first = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
        cursor += 2;
        if first == 0 || (0xdc00..=0xdfff).contains(&first) {
            return Err(error);
        }
        if (0xd800..=0xdbff).contains(&first) {
            if cursor >= bytes.len() {
                return Err(error);
            }
            let second = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            cursor += 2;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(error);
            }
        }
    }
    Ok(())
}

fn read_length8(
    bytes: &[u8],
    offset: usize,
    end: usize,
    error: Error,
) -> Result<(usize, usize), Error> {
    let first = *bytes.get(offset).filter(|_| offset < end).ok_or(error)?;
    if first & 0x80 == 0 {
        return Ok((usize::from(first), offset + 1));
    }
    let second_offset = offset.checked_add(1).ok_or(error)?;
    let second = *bytes
        .get(second_offset)
        .filter(|_| second_offset < end)
        .ok_or(error)?;
    let value = (usize::from(first & 0x7f) << 8) | usize::from(second);
    if value < 0x80 {
        return Err(error);
    }
    Ok((value, second_offset + 1))
}

fn read_length16(
    bytes: &[u8],
    offset: usize,
    end: usize,
    error: Error,
) -> Result<(usize, usize), Error> {
    let first_end = offset.checked_add(2).ok_or(error)?;
    if first_end > end {
        return Err(error);
    }
    let first = read_u16(bytes, offset, error)?;
    if first & 0x8000 == 0 {
        return Ok((usize::from(first), first_end));
    }
    let second_end = offset.checked_add(4).ok_or(error)?;
    if second_end > end {
        return Err(error);
    }
    let second = read_u16(bytes, offset + 2, error)?;
    let value = (usize::from(first & 0x7fff) << 16) | usize::from(second);
    if value < 0x8000 {
        return Err(error);
    }
    Ok((value, second_end))
}

fn validate_package(bytes: &[u8], offset: usize, package: Chunk) -> Result<(), Error> {
    if !matches!(package.header_size, 284 | 288) {
        return Err(Error::ResourcePackage);
    }
    let package_id = read_u32(bytes, offset + 8, Error::ResourcePackage)?;
    if package_id == 0 || package_id > 0xff {
        return Err(Error::ResourcePackage);
    }
    validate_package_name(bytes, offset + 12)?;
    let type_pool_relative = usize_from(
        read_u32(bytes, offset + 268, Error::ResourcePackage)?,
        Error::ResourcePackage,
    )?;
    let key_pool_relative = usize_from(
        read_u32(bytes, offset + 276, Error::ResourcePackage)?,
        Error::ResourcePackage,
    )?;
    let type_id_offset = if package.header_size == 288 {
        read_u32(bytes, offset + 284, Error::ResourcePackage)?
    } else {
        0
    };
    if type_id_offset > 0xfe {
        return Err(Error::ResourcePackage);
    }
    let type_pool_offset = offset
        .checked_add(type_pool_relative)
        .ok_or(Error::ResourcePackage)?;
    let key_pool_offset = offset
        .checked_add(key_pool_relative)
        .ok_or(Error::ResourcePackage)?;
    if type_pool_relative < usize::from(package.header_size)
        || key_pool_relative < usize::from(package.header_size)
        || type_pool_offset >= package.end
        || key_pool_offset >= package.end
        || type_pool_offset == key_pool_offset
    {
        return Err(Error::ResourcePackage);
    }
    let (type_pool, _) = StringPool::parse(
        bytes,
        type_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let (key_pool, _) = StringPool::parse(
        bytes,
        key_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    if type_pool.count == 0 || key_pool.count == 0 {
        return Err(Error::ResourcePackage);
    }

    let mut cursor = offset + usize::from(package.header_size);
    let mut chunks_seen = 0usize;
    let mut type_pool_seen = false;
    let mut key_pool_seen = false;
    while cursor < package.end {
        chunks_seen = chunks_seen.checked_add(1).ok_or(Error::ResourcePackage)?;
        if chunks_seen > MAX_PACKAGE_CHUNKS {
            return Err(Error::ResourcePackage);
        }
        let child = chunk_header(bytes, cursor, package.end, Error::ResourcePackage)?;
        match child.kind {
            RES_STRING_POOL_TYPE => {
                if cursor == type_pool_offset {
                    if type_pool_seen {
                        return Err(Error::ResourcePackage);
                    }
                    type_pool_seen = true;
                } else if cursor == key_pool_offset {
                    if key_pool_seen {
                        return Err(Error::ResourcePackage);
                    }
                    key_pool_seen = true;
                } else {
                    return Err(Error::ResourcePackage);
                }
            }
            RES_TABLE_TYPE_SPEC_TYPE => {
                validate_type_spec(bytes, cursor, child, type_id_offset, type_pool)?;
            }
            RES_TABLE_TYPE_TYPE => {
                validate_type_chunk(bytes, cursor, child, type_id_offset, type_pool, key_pool)?;
            }
            _ => return Err(Error::ResourcePackage),
        }
        cursor = child.end;
    }
    if cursor != package.end || !type_pool_seen || !key_pool_seen {
        return Err(Error::ResourcePackage);
    }
    Ok(())
}

fn validate_package_name(bytes: &[u8], offset: usize) -> Result<(), Error> {
    let mut terminated = false;
    let mut index = 0usize;
    while index < 128 {
        let unit = read_u16(
            bytes,
            offset
                .checked_add(index.checked_mul(2).ok_or(Error::ResourcePackage)?)
                .ok_or(Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?;
        if unit == 0 {
            terminated = true;
        } else if terminated
            || unit > 0x7f
            || !(u8::try_from(unit)
                .map_err(|_| Error::ResourcePackage)?
                .is_ascii_alphanumeric()
                || matches!(unit, 0x2e | 0x5f))
        {
            return Err(Error::ResourcePackage);
        }
        index += 1;
    }
    if !terminated {
        return Err(Error::ResourcePackage);
    }
    Ok(())
}

fn validate_type_spec(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    type_id_offset: u32,
    type_pool: StringPool<'_>,
) -> Result<(), Error> {
    if chunk.header_size != 16 {
        return Err(Error::ResourceType);
    }
    let type_id = *bytes.get(offset + 8).ok_or(Error::ResourceType)?;
    if type_id == 0 || u32::from(type_id) + type_id_offset > 0xff {
        return Err(Error::ResourceType);
    }
    type_pool.get(u32::from(type_id) - 1)?;
    let entry_count = read_u32(bytes, offset + 12, Error::ResourceType)?;
    if entry_count == 0 || entry_count > MAX_TYPE_ENTRIES {
        return Err(Error::ResourceType);
    }
    let expected = 16usize
        .checked_add(
            usize_from(entry_count, Error::ResourceType)?
                .checked_mul(4)
                .ok_or(Error::ResourceType)?,
        )
        .ok_or(Error::ResourceType)?;
    if expected != usize_from(chunk.size, Error::ResourceType)? {
        return Err(Error::ResourceType);
    }
    Ok(())
}

fn validate_type_chunk(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    type_id_offset: u32,
    type_pool: StringPool<'_>,
    key_pool: StringPool<'_>,
) -> Result<(), Error> {
    let metadata = type_chunk_metadata(bytes, offset, chunk, type_id_offset, type_pool)?;
    let mut index = 0u32;
    while index < metadata.entry_count {
        if let Some(entry_offset) = metadata.entry_offset(bytes, index)? {
            let entry = metadata
                .entries_start
                .checked_add(entry_offset)
                .ok_or(Error::ResourceEntry)?;
            validate_simple_entry(bytes, entry, chunk.end, key_pool)?;
        }
        index += 1;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct TypeChunk {
    offsets_start: usize,
    entries_start: usize,
    entry_count: u32,
}

impl TypeChunk {
    fn entry_offset(self, bytes: &[u8], index: u32) -> Result<Option<usize>, Error> {
        if index >= self.entry_count {
            return Err(Error::ResourceEntry);
        }
        let table_offset = self
            .offsets_start
            .checked_add(
                usize_from(index, Error::ResourceEntry)?
                    .checked_mul(4)
                    .ok_or(Error::ResourceEntry)?,
            )
            .ok_or(Error::ResourceEntry)?;
        let relative = read_u32(bytes, table_offset, Error::ResourceEntry)?;
        if relative == NO_ENTRY {
            Ok(None)
        } else {
            Ok(Some(usize_from(relative, Error::ResourceEntry)?))
        }
    }
}

fn type_chunk_metadata(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    type_id_offset: u32,
    type_pool: StringPool<'_>,
) -> Result<TypeChunk, Error> {
    if chunk.header_size < 24 {
        return Err(Error::ResourceType);
    }
    let type_id = *bytes.get(offset + 8).ok_or(Error::ResourceType)?;
    let flags = *bytes.get(offset + 9).ok_or(Error::ResourceType)?;
    if type_id == 0
        || u32::from(type_id) + type_id_offset > 0xff
        || flags & (TYPE_FLAG_SPARSE | TYPE_FLAG_OFFSET16) != 0
        || flags & !(TYPE_FLAG_SPARSE | TYPE_FLAG_OFFSET16) != 0
        || read_u16(bytes, offset + 10, Error::ResourceType)? != 0
    {
        return Err(Error::ResourceType);
    }
    type_pool.get(u32::from(type_id) - 1)?;
    let entry_count = read_u32(bytes, offset + 12, Error::ResourceType)?;
    if entry_count == 0 || entry_count > MAX_TYPE_ENTRIES {
        return Err(Error::ResourceType);
    }
    let entries_relative = usize_from(
        read_u32(bytes, offset + 16, Error::ResourceType)?,
        Error::ResourceType,
    )?;
    let config_size = usize_from(
        read_u32(bytes, offset + 20, Error::ResourceType)?,
        Error::ResourceType,
    )?;
    if config_size < 4
        || usize::from(chunk.header_size)
            != 20usize
                .checked_add(config_size)
                .ok_or(Error::ResourceType)?
    {
        return Err(Error::ResourceType);
    }
    let offsets_start = offset + usize::from(chunk.header_size);
    let offsets_bytes = usize_from(entry_count, Error::ResourceType)?
        .checked_mul(4)
        .ok_or(Error::ResourceType)?;
    let offsets_end = offsets_start
        .checked_add(offsets_bytes)
        .ok_or(Error::ResourceType)?;
    let entries_start = offset
        .checked_add(entries_relative)
        .ok_or(Error::ResourceType)?;
    if offsets_end > entries_start || entries_start > chunk.end {
        return Err(Error::ResourceType);
    }
    if bytes[offsets_end..entries_start]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(Error::ResourceType);
    }
    Ok(TypeChunk {
        offsets_start,
        entries_start,
        entry_count,
    })
}

fn validate_simple_entry(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    key_pool: StringPool<'_>,
) -> Result<(u8, u32), Error> {
    let entry_size = read_u16(bytes, offset, Error::ResourceEntry)?;
    let flags = read_u16(bytes, offset + 2, Error::ResourceEntry)?;
    let key_index = read_u32(bytes, offset + 4, Error::ResourceEntry)?;
    // Resources-1 implements only the canonical, non-compact simple
    // ResTable_entry + Res_value encoding. PUBLIC and WEAK annotate the entry
    // without changing that wire layout (aapt2 marks the fixture's generated
    // id entry WEAK); COMPLEX, COMPACT, and every unknown bit are unsupported.
    if entry_size != 8 || flags & !ENTRY_LAYOUT_NEUTRAL_FLAGS != 0 {
        return Err(Error::ResourceEntry);
    }
    key_pool.get(key_index)?;
    let value_offset = offset.checked_add(8).ok_or(Error::ResourceEntry)?;
    if value_offset.checked_add(8).ok_or(Error::ResourceEntry)? > limit
        || read_u16(bytes, value_offset, Error::ResourceEntry)? != 8
        || *bytes.get(value_offset + 2).ok_or(Error::ResourceEntry)? != 0
    {
        return Err(Error::ResourceEntry);
    }
    let value_type = *bytes.get(value_offset + 3).ok_or(Error::ResourceEntry)?;
    let data = read_u32(bytes, value_offset + 4, Error::ResourceEntry)?;
    Ok((value_type, data))
}

fn resolve_in_package(
    bytes: &[u8],
    package_offset: usize,
    package: Chunk,
    requested_type: u32,
    requested_entry: u32,
    global_strings: StringPool<'_>,
    expected_type_name: &[u8],
) -> Result<ResourceString, Error> {
    let type_id_offset = if package.header_size == 288 {
        read_u32(bytes, package_offset + 284, Error::ResourcePackage)?
    } else {
        0
    };
    let type_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 268, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let key_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 276, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let (type_pool, _) = StringPool::parse(
        bytes,
        type_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let (key_pool, _) = StringPool::parse(
        bytes,
        key_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let mut cursor = package_offset + usize::from(package.header_size);
    let mut found: Option<ResourceString> = None;
    while cursor < package.end {
        let child = chunk_header(bytes, cursor, package.end, Error::ResourcePackage)?;
        if child.kind == RES_TABLE_TYPE_TYPE {
            let local_type = u32::from(*bytes.get(cursor + 8).ok_or(Error::ResourceType)?);
            if local_type
                .checked_add(type_id_offset)
                .ok_or(Error::ResourceType)?
                == requested_type
                && is_default_configuration(bytes, cursor, child)?
            {
                if !type_pool.equals_ascii(local_type - 1, expected_type_name)? {
                    return Err(Error::ResourceValueType);
                }
                let metadata =
                    type_chunk_metadata(bytes, cursor, child, type_id_offset, type_pool)?;
                if requested_entry >= metadata.entry_count {
                    return Err(Error::ResourceNotFound);
                }
                if let Some(relative) = metadata.entry_offset(bytes, requested_entry)? {
                    let entry_offset = metadata
                        .entries_start
                        .checked_add(relative)
                        .ok_or(Error::ResourceEntry)?;
                    let (value_type, data) =
                        validate_simple_entry(bytes, entry_offset, child.end, key_pool)?;
                    if value_type != TYPE_STRING {
                        return Err(Error::ResourceValueType);
                    }
                    let value = global_strings
                        .get(data)
                        .map_err(|_| Error::ResourceReference)?
                        .to_resource_string()?;
                    if found.replace(value).is_some() {
                        return Err(Error::ResourceConfigurationAmbiguous);
                    }
                }
            }
        }
        cursor = child.end;
    }
    found.ok_or(Error::ResourceNotFound)
}

#[cfg(feature = "androidbox-density-icons7")]
fn resolve_launcher_icon_in_package(
    bytes: &[u8],
    package_offset: usize,
    package: Chunk,
    requested_type: u32,
    requested_entry: u32,
    global_strings: StringPool<'_>,
    expected_type_name: &[u8],
) -> Result<(ResourceString, u16), Error> {
    const MDP_DENSITY_DPI: u16 = 160;

    let type_id_offset = if package.header_size == 288 {
        read_u32(bytes, package_offset + 284, Error::ResourcePackage)?
    } else {
        0
    };
    let type_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 268, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let key_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 276, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let (type_pool, _) = StringPool::parse(
        bytes,
        type_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let (key_pool, _) = StringPool::parse(
        bytes,
        key_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;

    let mut cursor = package_offset + usize::from(package.header_size);
    let mut default_value: Option<ResourceString> = None;
    let mut mdpi_value: Option<ResourceString> = None;
    while cursor < package.end {
        let child = chunk_header(bytes, cursor, package.end, Error::ResourcePackage)?;
        if child.kind == RES_TABLE_TYPE_TYPE {
            let local_type = u32::from(*bytes.get(cursor + 8).ok_or(Error::ResourceType)?);
            if local_type
                .checked_add(type_id_offset)
                .ok_or(Error::ResourceType)?
                == requested_type
            {
                if !type_pool.equals_ascii(local_type - 1, expected_type_name)? {
                    return Err(Error::ResourceValueType);
                }
                let density_dpi = if is_default_configuration(bytes, cursor, child)? {
                    Some(0)
                } else if is_exact_density_configuration(bytes, cursor, child, MDP_DENSITY_DPI)? {
                    Some(MDP_DENSITY_DPI)
                } else {
                    None
                };
                if let Some(density_dpi) = density_dpi {
                    let metadata =
                        type_chunk_metadata(bytes, cursor, child, type_id_offset, type_pool)?;
                    if requested_entry >= metadata.entry_count {
                        return Err(Error::ResourceNotFound);
                    }
                    if let Some(relative) = metadata.entry_offset(bytes, requested_entry)? {
                        let entry_offset = metadata
                            .entries_start
                            .checked_add(relative)
                            .ok_or(Error::ResourceEntry)?;
                        let (value_type, data) =
                            validate_simple_entry(bytes, entry_offset, child.end, key_pool)?;
                        if value_type != TYPE_STRING {
                            return Err(Error::ResourceValueType);
                        }
                        let value = global_strings
                            .get(data)
                            .map_err(|_| Error::ResourceReference)?
                            .to_resource_string()?;
                        let target = if density_dpi == 0 {
                            &mut default_value
                        } else {
                            &mut mdpi_value
                        };
                        if target.replace(value).is_some() {
                            return Err(Error::ResourceConfigurationAmbiguous);
                        }
                    }
                }
            }
        }
        cursor = child.end;
    }
    if let Some(value) = mdpi_value {
        Ok((value, MDP_DENSITY_DPI))
    } else {
        default_value
            .map(|value| (value, 0))
            .ok_or(Error::ResourceNotFound)
    }
}

fn validate_id_in_package(
    bytes: &[u8],
    package_offset: usize,
    package: Chunk,
    requested_type: u32,
    requested_entry: u32,
) -> Result<(), Error> {
    let type_id_offset = if package.header_size == 288 {
        read_u32(bytes, package_offset + 284, Error::ResourcePackage)?
    } else {
        0
    };
    let type_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 268, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let key_pool_offset = package_offset
        .checked_add(usize_from(
            read_u32(bytes, package_offset + 276, Error::ResourcePackage)?,
            Error::ResourcePackage,
        )?)
        .ok_or(Error::ResourcePackage)?;
    let (type_pool, _) = StringPool::parse(
        bytes,
        type_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let (key_pool, _) = StringPool::parse(
        bytes,
        key_pool_offset,
        package.end,
        Error::ResourceStringPool,
    )?;
    let local_requested_type = requested_type
        .checked_sub(type_id_offset)
        .filter(|type_id| *type_id != 0)
        .ok_or(Error::ResourceValueType)?;
    if !type_pool.equals_ascii(local_requested_type - 1, b"id")? {
        return Err(Error::ResourceValueType);
    }
    let mut cursor = package_offset + usize::from(package.header_size);
    let mut found = false;
    while cursor < package.end {
        let child = chunk_header(bytes, cursor, package.end, Error::ResourcePackage)?;
        if child.kind == RES_TABLE_TYPE_TYPE
            && u32::from(*bytes.get(cursor + 8).ok_or(Error::ResourceType)?)
                .checked_add(type_id_offset)
                .ok_or(Error::ResourceType)?
                == requested_type
            && is_default_configuration(bytes, cursor, child)?
        {
            if found {
                return Err(Error::ResourceConfigurationAmbiguous);
            }
            let metadata = type_chunk_metadata(bytes, cursor, child, type_id_offset, type_pool)?;
            if requested_entry >= metadata.entry_count {
                return Err(Error::ResourceNotFound);
            }
            let relative = metadata
                .entry_offset(bytes, requested_entry)?
                .ok_or(Error::ResourceNotFound)?;
            let entry_offset = metadata
                .entries_start
                .checked_add(relative)
                .ok_or(Error::ResourceEntry)?;
            let (value_type, data) =
                validate_simple_entry(bytes, entry_offset, child.end, key_pool)?;
            if value_type != TYPE_INT_BOOLEAN || data != 0 {
                return Err(Error::ResourceValueType);
            }
            found = true;
        }
        cursor = child.end;
    }
    found.then_some(()).ok_or(Error::ResourceNotFound)
}

fn is_default_configuration(bytes: &[u8], offset: usize, chunk: Chunk) -> Result<bool, Error> {
    if chunk.header_size < 24 {
        return Err(Error::ResourceType);
    }
    let config_size = usize_from(
        read_u32(bytes, offset + 20, Error::ResourceType)?,
        Error::ResourceType,
    )?;
    let config_start = offset + 20;
    let config_end = config_start
        .checked_add(config_size)
        .ok_or(Error::ResourceType)?;
    if config_end != offset + usize::from(chunk.header_size) {
        return Err(Error::ResourceType);
    }
    Ok(bytes[config_start + 4..config_end]
        .iter()
        .all(|byte| *byte == 0))
}

#[cfg(feature = "androidbox-density-icons7")]
fn is_exact_density_configuration(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    expected_density_dpi: u16,
) -> Result<bool, Error> {
    if chunk.header_size < 48 {
        return Ok(false);
    }
    let config_size = usize_from(
        read_u32(bytes, offset + 20, Error::ResourceType)?,
        Error::ResourceType,
    )?;
    let config_start = offset + 20;
    let config_end = config_start
        .checked_add(config_size)
        .ok_or(Error::ResourceType)?;
    if config_end != offset + usize::from(chunk.header_size) {
        return Err(Error::ResourceType);
    }
    if read_u16(bytes, config_start + 14, Error::ResourceType)? != expected_density_dpi {
        return Ok(false);
    }
    let sdk_version = read_u16(bytes, config_start + 24, Error::ResourceType)?;
    if !matches!(sdk_version, 0 | 4)
        || read_u16(bytes, config_start + 26, Error::ResourceType)? != 0
    {
        return Ok(false);
    }
    Ok(bytes[config_start + 4..config_end]
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 10 | 11 | 20 | 21) || *byte == 0))
}

#[derive(Clone, Copy)]
struct ResourceMap<'a> {
    bytes: &'a [u8],
    entries_start: usize,
    count: u32,
}

impl<'a> ResourceMap<'a> {
    fn parse(
        bytes: &'a [u8],
        offset: usize,
        limit: usize,
        string_count: u32,
    ) -> Result<(Self, usize), Error> {
        let chunk = chunk_header(bytes, offset, limit, Error::LayoutResourceMap)?;
        if chunk.kind != RES_XML_RESOURCE_MAP_TYPE || chunk.header_size != 8 {
            return Err(Error::LayoutResourceMap);
        }
        let payload = usize_from(chunk.size, Error::LayoutResourceMap)?
            .checked_sub(8)
            .ok_or(Error::LayoutResourceMap)?;
        if !payload.is_multiple_of(4) {
            return Err(Error::LayoutResourceMap);
        }
        let count = u32::try_from(payload / 4).map_err(|_| Error::LayoutResourceMap)?;
        if count == 0 || count > string_count {
            return Err(Error::LayoutResourceMap);
        }
        let map = Self {
            bytes,
            entries_start: offset + 8,
            count,
        };
        let mut index = 0u32;
        while index < count {
            if map.get(index)? == 0 {
                return Err(Error::LayoutResourceMap);
            }
            index += 1;
        }
        Ok((map, chunk.end))
    }

    fn get(self, string_index: u32) -> Result<u32, Error> {
        if string_index >= self.count {
            return Err(Error::LayoutResourceMap);
        }
        read_u32(
            self.bytes,
            self.entries_start
                .checked_add(
                    usize_from(string_index, Error::LayoutResourceMap)?
                        .checked_mul(4)
                        .ok_or(Error::LayoutResourceMap)?,
                )
                .ok_or(Error::LayoutResourceMap)?,
            Error::LayoutResourceMap,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Namespace {
    prefix: u32,
    uri: u32,
    end: usize,
}

fn parse_namespace(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    strings: StringPool<'_>,
    start: bool,
) -> Result<Namespace, Error> {
    let chunk = chunk_header(bytes, offset, limit, Error::LayoutStructure)?;
    let expected = if start {
        RES_XML_START_NAMESPACE_TYPE
    } else {
        RES_XML_END_NAMESPACE_TYPE
    };
    if chunk.kind != expected || chunk.header_size != 16 || chunk.size != 24 {
        return Err(Error::LayoutStructure);
    }
    validate_node(bytes, offset, Error::LayoutStructure)?;
    let prefix = read_u32(bytes, offset + 16, Error::LayoutStructure)?;
    let uri = read_u32(bytes, offset + 20, Error::LayoutStructure)?;
    if !strings.equals_ascii(prefix, ANDROID_PREFIX)? || !strings.equals_ascii(uri, ANDROID_URI)? {
        return Err(Error::LayoutStructure);
    }
    Ok(Namespace {
        prefix,
        uri,
        end: chunk.end,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ElementName {
    namespace: u32,
    name: u32,
}

impl ElementName {
    const fn empty() -> Self {
        Self {
            namespace: NO_INDEX,
            name: NO_INDEX,
        }
    }
}

struct StartElement {
    namespace: u32,
    name: u32,
    text_reference: Option<LayoutTextReference>,
}

fn parse_start_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    strings: StringPool<'_>,
    resource_map: ResourceMap<'_>,
) -> Result<StartElement, Error> {
    if chunk.header_size != 16 || chunk.size < 36 {
        return Err(Error::LayoutStructure);
    }
    validate_node(bytes, offset, Error::LayoutStructure)?;
    let namespace = read_u32(bytes, offset + 16, Error::LayoutStructure)?;
    let name = read_u32(bytes, offset + 20, Error::LayoutStructure)?;
    if namespace != NO_INDEX {
        strings.get(namespace)?;
    }
    strings.get(name)?;
    let attribute_start = read_u16(bytes, offset + 24, Error::LayoutStructure)?;
    let attribute_size = read_u16(bytes, offset + 26, Error::LayoutStructure)?;
    let attribute_count = read_u16(bytes, offset + 28, Error::LayoutStructure)?;
    let id_index = read_u16(bytes, offset + 30, Error::LayoutStructure)?;
    let class_index = read_u16(bytes, offset + 32, Error::LayoutStructure)?;
    let style_index = read_u16(bytes, offset + 34, Error::LayoutStructure)?;
    if attribute_start != 20
        || attribute_size != 20
        || attribute_count > MAX_LAYOUT_ATTRIBUTES
        || id_index > attribute_count
        || class_index > attribute_count
        || style_index > attribute_count
    {
        return Err(Error::LayoutStructure);
    }
    let attributes_start = offset
        .checked_add(16 + usize::from(attribute_start))
        .ok_or(Error::LayoutStructure)?;
    let expected_end = attributes_start
        .checked_add(
            usize::from(attribute_count)
                .checked_mul(usize::from(attribute_size))
                .ok_or(Error::LayoutStructure)?,
        )
        .ok_or(Error::LayoutStructure)?;
    if expected_end != chunk.end {
        return Err(Error::LayoutStructure);
    }

    let is_text_view = strings.equals_ascii(name, TEXT_VIEW)?;
    let mut text_reference: Option<LayoutTextReference> = None;
    let mut attribute_index = 0u16;
    while attribute_index < attribute_count {
        let attribute_offset = attributes_start
            .checked_add(
                usize::from(attribute_index)
                    .checked_mul(20)
                    .ok_or(Error::LayoutAttribute)?,
            )
            .ok_or(Error::LayoutAttribute)?;
        let attribute = parse_layout_attribute(bytes, attribute_offset, strings, resource_map)?;
        let is_android_text = is_text_view
            && attribute.namespace != NO_INDEX
            && strings.equals_ascii(attribute.namespace, ANDROID_URI)?
            && strings.equals_ascii(attribute.name, TEXT_ATTRIBUTE)?;
        if is_android_text {
            let reference = validate_text_reference(attribute, resource_map)?;
            if text_reference.replace(reference).is_some() {
                return Err(Error::LayoutAttribute);
            }
        }
        attribute_index += 1;
    }
    Ok(StartElement {
        namespace,
        name,
        text_reference,
    })
}

fn validate_text_reference(
    attribute: LayoutAttribute,
    resource_map: ResourceMap<'_>,
) -> Result<LayoutTextReference, Error> {
    if resource_map.get(attribute.name)? != ANDROID_TEXT_ATTRIBUTE_ID
        || attribute.raw_value != NO_INDEX
        || attribute.value_type != TYPE_REFERENCE
        || attribute.data == 0
    {
        return Err(Error::LayoutAttribute);
    }
    Ok(LayoutTextReference {
        resource_id: attribute.data,
    })
}

fn parse_end_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    strings: StringPool<'_>,
) -> Result<ElementName, Error> {
    if chunk.header_size != 16 || chunk.size != 24 {
        return Err(Error::LayoutStructure);
    }
    validate_node(bytes, offset, Error::LayoutStructure)?;
    let namespace = read_u32(bytes, offset + 16, Error::LayoutStructure)?;
    let name = read_u32(bytes, offset + 20, Error::LayoutStructure)?;
    if namespace != NO_INDEX {
        strings.get(namespace)?;
    }
    strings.get(name)?;
    Ok(ElementName { namespace, name })
}

#[derive(Clone, Copy)]
struct LayoutAttribute {
    namespace: u32,
    name: u32,
    raw_value: u32,
    value_type: u8,
    data: u32,
}

fn parse_layout_attribute(
    bytes: &[u8],
    offset: usize,
    strings: StringPool<'_>,
    resource_map: ResourceMap<'_>,
) -> Result<LayoutAttribute, Error> {
    let namespace = read_u32(bytes, offset, Error::LayoutAttribute)?;
    let name = read_u32(bytes, offset + 4, Error::LayoutAttribute)?;
    let raw_value = read_u32(bytes, offset + 8, Error::LayoutAttribute)?;
    if namespace != NO_INDEX {
        strings.get(namespace)?;
    }
    strings.get(name)?;
    if raw_value != NO_INDEX {
        strings.get(raw_value)?;
    }
    if read_u16(bytes, offset + 12, Error::LayoutAttribute)? != 8
        || *bytes.get(offset + 14).ok_or(Error::LayoutAttribute)? != 0
    {
        return Err(Error::LayoutAttribute);
    }
    let value_type = *bytes.get(offset + 15).ok_or(Error::LayoutAttribute)?;
    let data = read_u32(bytes, offset + 16, Error::LayoutAttribute)?;
    if namespace != NO_INDEX && strings.equals_ascii(namespace, ANDROID_URI)? {
        resource_map.get(name)?;
    }
    Ok(LayoutAttribute {
        namespace,
        name,
        raw_value,
        value_type,
        data,
    })
}

fn validate_layout_entry_name(name: &[u8]) -> Result<(), Error> {
    const PREFIX: &[u8] = b"res/layout/";
    const SUFFIX: &[u8] = b".xml";
    if name.len() > 128
        || !name.starts_with(PREFIX)
        || !name.ends_with(SUFFIX)
        || name.len() <= PREFIX.len() + SUFFIX.len()
    {
        return Err(Error::LayoutEntryName);
    }
    let stem = &name[PREFIX.len()..name.len() - SUFFIX.len()];
    if !stem[0].is_ascii_lowercase()
        || !stem
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(Error::LayoutEntryName);
    }
    Ok(())
}

#[cfg(feature = "androidbox-icon-resources5")]
fn validate_launcher_icon_entry_name(name: &[u8], density_dpi: u16) -> Result<(), Error> {
    const DRAWABLE_PREFIX: &[u8] = b"res/drawable/";
    const MIPMAP_PREFIX: &[u8] = b"res/mipmap/";
    #[cfg(feature = "androidbox-density-icons7")]
    const DRAWABLE_MDPI_PREFIX: &[u8] = b"res/drawable-mdpi/";
    #[cfg(feature = "androidbox-density-icons7")]
    const DRAWABLE_MDPI_V4_PREFIX: &[u8] = b"res/drawable-mdpi-v4/";
    #[cfg(feature = "androidbox-density-icons7")]
    const MIPMAP_MDPI_PREFIX: &[u8] = b"res/mipmap-mdpi/";
    #[cfg(feature = "androidbox-density-icons7")]
    const MIPMAP_MDPI_V4_PREFIX: &[u8] = b"res/mipmap-mdpi-v4/";
    const SUFFIX: &[u8] = b".png";
    let prefix = match density_dpi {
        0 if name.starts_with(DRAWABLE_PREFIX) => DRAWABLE_PREFIX,
        0 if name.starts_with(MIPMAP_PREFIX) => MIPMAP_PREFIX,
        #[cfg(feature = "androidbox-density-icons7")]
        160 if name.starts_with(DRAWABLE_MDPI_PREFIX) => DRAWABLE_MDPI_PREFIX,
        #[cfg(feature = "androidbox-density-icons7")]
        160 if name.starts_with(DRAWABLE_MDPI_V4_PREFIX) => DRAWABLE_MDPI_V4_PREFIX,
        #[cfg(feature = "androidbox-density-icons7")]
        160 if name.starts_with(MIPMAP_MDPI_PREFIX) => MIPMAP_MDPI_PREFIX,
        #[cfg(feature = "androidbox-density-icons7")]
        160 if name.starts_with(MIPMAP_MDPI_V4_PREFIX) => MIPMAP_MDPI_V4_PREFIX,
        _ => return Err(Error::LauncherIconEntryName),
    };
    if name.len() > 128 || !name.ends_with(SUFFIX) || name.len() <= prefix.len() + SUFFIX.len() {
        return Err(Error::LauncherIconEntryName);
    }
    let stem = &name[prefix.len()..name.len() - SUFFIX.len()];
    if !stem[0].is_ascii_lowercase()
        || !stem
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(Error::LauncherIconEntryName);
    }
    Ok(())
}

fn validate_node(bytes: &[u8], offset: usize, error: Error) -> Result<(), Error> {
    if read_u32(bytes, offset + 8, error)? == 0 || read_u32(bytes, offset + 12, error)? != NO_INDEX
    {
        return Err(error);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Chunk {
    kind: u16,
    header_size: u16,
    size: u32,
    end: usize,
}

fn chunk_header(bytes: &[u8], offset: usize, limit: usize, error: Error) -> Result<Chunk, Error> {
    let kind = read_u16(bytes, offset, error)?;
    let header_size = read_u16(bytes, offset + 2, error)?;
    let size = read_u32(bytes, offset + 4, error)?;
    if header_size < 8 || u32::from(header_size) > size || !size.is_multiple_of(4) {
        return Err(error);
    }
    let end = offset.checked_add(usize_from(size, error)?).ok_or(error)?;
    if end > limit || limit > bytes.len() {
        return Err(error);
    }
    Ok(Chunk {
        kind,
        header_size,
        size,
        end,
    })
}

fn read_u16(bytes: &[u8], offset: usize, error: Error) -> Result<u16, Error> {
    let end = offset.checked_add(2).ok_or(error)?;
    let value = bytes.get(offset..end).ok_or(error)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize, error: Error) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(error)?;
    let value = bytes.get(offset..end).ok_or(error)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn usize_from(value: u32, error: Error) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| error)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::ops::Range;

    #[cfg(feature = "androidbox-layout-mixed19")]
    use super::mixed_horizontal_child_is_canonical;
    #[cfg(feature = "androidbox-layout-size18")]
    use super::strict_layout_size_and_dp;
    #[cfg(feature = "androidbox-layout-spacing16")]
    use super::strict_uniform_dp;
    use super::{
        ANDROID_ORIENTATION_ATTRIBUTE_ID, ApkResources, DEFAULT_ACTIVITY_LAYOUT_ENTRY,
        ENTRY_FLAG_PUBLIC, Encoding, Error, LayoutOrientation, LayoutSize,
        MAX_ACTIVITY_SCENE_DEPTH, MAX_ACTIVITY_SCENE_NODES, MAX_RESOURCE_STRING_BYTES, PoolString,
        RESOURCES_ARSC_ENTRY, ResourceMap, ResourceTable, StringPool, ViewKind, chunk_header,
        parse_activity_scene, parse_layout_text_reference, parse_namespace, resolve_apk_text_view,
    };
    #[cfg(feature = "androidbox-layout-weight15")]
    use super::{
        LayoutAttribute, NO_INDEX, TYPE_DIMENSION, TYPE_FLOAT, strict_layout_size,
        strict_layout_weight,
    };
    use crate::{AndroidBox, verify_apk_v2};

    const RESOURCE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-resource-demo/androidbox-resource-demo.apk");
    const UPDATE_RESOURCE_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-resource-update-demo/androidbox-resource-update-demo.apk"
    );
    const INLINE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-demo/androidbox-demo.apk");
    const INTERACTIVE_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-interactive-demo/androidbox-interactive-demo.apk"
    );
    const STRING_MESSAGE_ID: u32 = 0x7f03_0000;
    const STRING_APP_NAME_ID: u32 = 0x7f03_0001;

    #[cfg(feature = "androidbox-layout-weight15")]
    #[test]
    fn compiled_zero_dp_and_integer_weights_have_one_strict_encoding() {
        let attribute = |value_type, data| LayoutAttribute {
            namespace: 0,
            name: 0,
            raw_value: NO_INDEX,
            value_type,
            data,
        };
        assert_eq!(
            strict_layout_size(attribute(TYPE_DIMENSION, 1)),
            Ok(LayoutSize::Zero)
        );
        assert_eq!(
            strict_layout_weight(attribute(TYPE_FLOAT, 0x3f80_0000)),
            Ok(1)
        );
        assert_eq!(
            strict_layout_weight(attribute(TYPE_FLOAT, 0x4100_0000)),
            Ok(8)
        );
        assert_eq!(
            strict_layout_size(attribute(TYPE_DIMENSION, 0)).err(),
            Some(Error::LayoutAttribute)
        );
        assert_eq!(
            strict_layout_weight(attribute(TYPE_FLOAT, 0x3f00_0000)).err(),
            Some(Error::LayoutAttribute)
        );
        assert_eq!(
            strict_layout_weight(attribute(TYPE_FLOAT, 0x4110_0000)).err(),
            Some(Error::LayoutAttribute)
        );
    }

    #[cfg(feature = "androidbox-layout-size18")]
    #[test]
    fn compiled_exact_dp_sizes_are_positive_bounded_and_unit_strict() {
        let attribute = |raw_value, value_type, data| LayoutAttribute {
            namespace: 0,
            name: 0,
            raw_value,
            value_type,
            data,
        };
        assert_eq!(
            strict_layout_size_and_dp(attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_f001)),
            Ok((LayoutSize::Exact, 240))
        );
        assert_eq!(
            strict_layout_size_and_dp(attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0101)),
            Ok((LayoutSize::Exact, 1))
        );
        for malformed in [
            attribute(0, TYPE_DIMENSION, 0x0000_f001),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0100),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0111),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0001_0001),
            attribute(NO_INDEX, TYPE_FLOAT, 0x0000_f001),
        ] {
            assert_eq!(
                strict_layout_size_and_dp(malformed).err(),
                Some(Error::LayoutAttribute)
            );
        }
    }

    #[cfg(feature = "androidbox-layout-mixed19")]
    #[test]
    fn mixed_horizontal_children_accept_only_fixed_exact_or_weighted_zero_buttons() {
        let mut fixed = super::ActivitySceneNode::empty();
        fixed.kind = ViewKind::Button;
        fixed.width = LayoutSize::Exact;
        fixed.exact_width_dp = 112;
        assert!(mixed_horizontal_child_is_canonical(fixed));

        let mut weighted = fixed;
        weighted.width = LayoutSize::Zero;
        weighted.exact_width_dp = 0;
        weighted.layout_weight = 1;
        assert!(mixed_horizontal_child_is_canonical(weighted));

        for malformed in [
            super::ActivitySceneNode {
                width: LayoutSize::WrapContent,
                exact_width_dp: 0,
                layout_weight: 0,
                ..fixed
            },
            super::ActivitySceneNode {
                width: LayoutSize::Exact,
                layout_weight: 1,
                ..fixed
            },
            super::ActivitySceneNode {
                kind: ViewKind::TextView,
                ..fixed
            },
        ] {
            assert!(!mixed_horizontal_child_is_canonical(malformed));
        }
    }

    #[cfg(feature = "androidbox-layout-spacing16")]
    #[test]
    fn compiled_uniform_dp_spacing_has_one_bounded_strict_encoding() {
        let attribute = |raw_value, value_type, data| LayoutAttribute {
            namespace: 0,
            name: 0,
            raw_value,
            value_type,
            data,
        };
        assert_eq!(
            strict_uniform_dp(attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0401)),
            Ok(4)
        );
        assert_eq!(
            strict_uniform_dp(attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_1001)),
            Ok(16)
        );
        for malformed in [
            attribute(0, TYPE_DIMENSION, 0x0000_0401),
            attribute(NO_INDEX, TYPE_FLOAT, 0x0000_0401),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0400),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_0411),
            attribute(NO_INDEX, TYPE_DIMENSION, 0x0000_1101),
        ] {
            assert_eq!(
                strict_uniform_dp(malformed).err(),
                Some(Error::LayoutAttribute)
            );
        }
    }

    #[test]
    fn real_aapt2_apk_resolves_stored_resource_layout_end_to_end() {
        let resources = ApkResources::load(RESOURCE_APK).expect("real resources.arsc");
        assert_eq!(resources.resources_arsc_crc32(), 0x9f67_c7b4);
        assert_eq!(
            resources
                .resolve_string(STRING_MESSAGE_ID)
                .expect("message")
                .as_str(),
            "AndroidBox resource-backed view"
        );
        assert_eq!(
            resources
                .resolve_string(STRING_APP_NAME_ID)
                .expect("app name")
                .as_str(),
            "AndroidBox Resources-1 Demo"
        );
        assert_eq!(
            resources
                .resolve_layout_entry(0x7f02_0000)
                .expect("layout entry")
                .as_bytes(),
            DEFAULT_ACTIVITY_LAYOUT_ENTRY
        );
        assert_eq!(
            resources.resolve_string(0x7f02_0000).err(),
            Some(Error::ResourceValueType)
        );
        assert_eq!(
            resources.resolve_layout_entry(STRING_MESSAGE_ID).err(),
            Some(Error::ResourceValueType)
        );

        let reference = resources
            .layout_text_reference(DEFAULT_ACTIVITY_LAYOUT_ENTRY)
            .expect("compiled layout reference");
        assert_eq!(reference.resource_id, STRING_MESSAGE_ID);

        let resolved = resources
            .resolve_text_view(DEFAULT_ACTIVITY_LAYOUT_ENTRY)
            .expect("resolved TextView");
        assert_eq!(resolved.resources_arsc_crc32, 0x9f67_c7b4);
        assert_eq!(resolved.layout_crc32, 0x36ce_95c4);
        assert_eq!(resolved.reference, reference);
        assert_eq!(resolved.text.as_str(), "AndroidBox resource-backed view");
        assert_eq!(resolved.text.len(), 31);
        assert!(!resolved.text.is_empty());
        assert_eq!(
            resolve_apk_text_view(RESOURCE_APK, DEFAULT_ACTIVITY_LAYOUT_ENTRY),
            Ok(resolved)
        );
    }

    #[test]
    fn interactive_layout_resolves_a_strict_fixed_capacity_scene() {
        assert_eq!(MAX_ACTIVITY_SCENE_NODES, 8);
        assert_eq!(MAX_ACTIVITY_SCENE_DEPTH, 3);
        let resources = ApkResources::load(INTERACTIVE_APK).expect("interactive resources");
        resources.validate_id(0x7f01_0000).expect("Button id");
        resources.validate_id(0x7f01_0001).expect("TextView id");
        assert_eq!(
            resources.validate_id(0x7f03_0000),
            Err(Error::ResourceValueType)
        );
        let resolved = resources
            .resolve_activity_scene(DEFAULT_ACTIVITY_LAYOUT_ENTRY)
            .expect("strict resolved scene");
        let nodes = resolved.scene.nodes();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].kind, ViewKind::LinearLayout);
        assert_eq!(nodes[0].parent, None);
        assert_eq!(nodes[0].width, LayoutSize::MatchParent);
        assert_eq!(nodes[0].height, LayoutSize::MatchParent);
        assert_eq!(nodes[0].orientation, LayoutOrientation::Vertical);
        assert_eq!(nodes[1].kind, ViewKind::TextView);
        assert_eq!(nodes[1].parent, Some(0));
        assert_eq!(nodes[1].id, 0x7f01_0001);
        assert_eq!(nodes[1].text_resource_id, 0x7f03_0000);
        assert_eq!(nodes[1].text.as_str(), "Ready for a real APK click");
        assert_eq!(nodes[2].kind, ViewKind::Button);
        assert_eq!(nodes[2].parent, Some(0));
        assert_eq!(nodes[2].id, 0x7f01_0000);
        assert_eq!(nodes[2].text_resource_id, 0x7f03_0002);
        assert_eq!(nodes[2].text.as_str(), "Update text");
    }

    #[test]
    fn interactive_scene_rejects_unknown_tags_attributes_and_duplicate_ids() {
        let layout = raw_entry(INTERACTIVE_APK, DEFAULT_ACTIVITY_LAYOUT_ENTRY);

        let mut unknown_tag = layout.to_vec();
        let tag = find_bytes(&unknown_tag, b"LinearLayout");
        unknown_tag[tag] = b'X';
        assert_eq!(
            parse_activity_scene(&unknown_tag).err(),
            Some(Error::LayoutStructure)
        );

        let mut unknown_attribute = layout.to_vec();
        let attribute_id = find_bytes(
            &unknown_attribute,
            &ANDROID_ORIENTATION_ATTRIBUTE_ID.to_le_bytes(),
        );
        unknown_attribute[attribute_id] ^= 1;
        assert_eq!(
            parse_activity_scene(&unknown_attribute).err(),
            Some(Error::LayoutAttribute)
        );

        let mut duplicate_id = layout.to_vec();
        let label_id = find_bytes(&duplicate_id, &0x7f01_0001_u32.to_le_bytes());
        duplicate_id[label_id..label_id + 4].copy_from_slice(&0x7f01_0000_u32.to_le_bytes());
        assert_eq!(
            parse_activity_scene(&duplicate_id).err(),
            Some(Error::LayoutAttribute)
        );

        let document = chunk_header(layout, 0, layout.len(), Error::LayoutHeader).unwrap();
        let (strings, pool_end) =
            StringPool::parse(layout, 8, document.end, Error::LayoutStringPool).unwrap();
        let (_, map_end) =
            ResourceMap::parse(layout, pool_end, document.end, strings.count).unwrap();
        let namespace = parse_namespace(layout, map_end, document.end, strings, true).unwrap();
        let root =
            chunk_header(layout, namespace.end, document.end, Error::LayoutStructure).unwrap();
        let root_bytes = &layout[namespace.end..root.end];
        let mut too_deep = layout[..root.end].to_vec();
        too_deep.extend_from_slice(root_bytes);
        too_deep.extend_from_slice(root_bytes);
        too_deep.extend_from_slice(root_bytes);
        too_deep.extend_from_slice(&layout[root.end..]);
        let too_deep_len = u32::try_from(too_deep.len()).unwrap();
        write_u32(&mut too_deep, 4, too_deep_len);
        assert_eq!(
            parse_activity_scene(&too_deep).err(),
            Some(Error::LayoutStructure)
        );
    }

    #[test]
    fn real_resource_activity_executes_dex_and_renders_resolved_text() {
        let image = AndroidBox::load(RESOURCE_APK).expect("resource-backed AndroidBox image");
        assert_eq!(
            image.manifest_info().application_label_resource_id,
            Some(STRING_APP_NAME_ID)
        );
        assert_eq!(image.manifest_info().version_code, 2);
        assert_eq!(
            image.manifest_info().application_label.as_str(),
            "AndroidBox Resources-1 Demo"
        );
        let method = image.activity_method();
        assert_eq!(method.method_index, 7);
        assert_eq!(method.code_offset, 0x2c8);
        assert_eq!(method.registers, 2);
        assert_eq!(method.parameters, 2);
        assert_eq!(method.outgoing, 2);
        assert_eq!(method.instruction_units, 9);

        let launched = image.launch_activity().expect("resource Activity launch");
        assert_eq!(launched.view.title.as_str(), "AndroidBox Resources-1 Demo");
        assert_eq!(
            launched.view.text.as_str(),
            "AndroidBox resource-backed view"
        );
        assert_eq!(launched.instruction_count, 4);
        let evidence = launched.resources.expect("resource evidence");
        assert_eq!(evidence.resources_arsc_crc32, 0x9f67_c7b4);
        assert_eq!(evidence.layout_xml_crc32, 0x36ce_95c4);
        assert_eq!(evidence.layout_resource_id, 0x7f02_0000);
        assert_eq!(evidence.text_resource_id, STRING_MESSAGE_ID);
    }

    #[test]
    fn real_v3_update_keeps_signer_identity_and_launches_updated_resources() {
        let installed_signer = verify_apk_v2(RESOURCE_APK).expect("installed v2 signer");
        let update_signer = verify_apk_v2(UPDATE_RESOURCE_APK).expect("update v2 signer");
        assert_eq!(
            update_signer.certificate_sha256,
            installed_signer.certificate_sha256
        );

        let image = AndroidBox::load(UPDATE_RESOURCE_APK).expect("v3 AndroidBox update");
        let manifest = image.manifest_info();
        assert_eq!(manifest.android_manifest_crc32, 0xd095_0f3c);
        assert_eq!(manifest.package.as_str(), "org.bndroid.demo");
        assert_eq!(
            manifest.activity_descriptor.as_str(),
            "Lorg/bndroid/demo/MainActivity;"
        );
        assert_eq!(manifest.version_code, 3);
        assert_eq!(
            manifest.application_label_resource_id,
            Some(STRING_APP_NAME_ID)
        );
        assert_eq!(
            manifest.application_label.as_str(),
            "AndroidBox Resources-1 Demo v3"
        );

        let launched = image.launch_activity().expect("updated resource Activity");
        assert_eq!(
            launched.view.title.as_str(),
            "AndroidBox Resources-1 Demo v3"
        );
        assert_eq!(
            launched.view.text.as_str(),
            "AndroidBox updated resource view"
        );
        assert_eq!(launched.instruction_count, 4);
        let evidence = launched.resources.expect("updated resource evidence");
        assert_eq!(evidence.resources_arsc_crc32, 0xba0a_d990);
        assert_eq!(evidence.layout_xml_crc32, 0x36ce_95c4);
        assert_eq!(evidence.layout_resource_id, 0x7f02_0000);
        assert_eq!(evidence.text_resource_id, STRING_MESSAGE_ID);
    }

    #[test]
    fn existing_inline_fixture_does_not_gain_implicit_resources() {
        assert_eq!(
            ApkResources::load(INLINE_APK).err(),
            Some(Error::ZipMissingResources)
        );
    }

    #[test]
    fn apk_resource_entries_are_unique_stored_and_crc_checked() {
        let renamed_resources = rename_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY, b"xesources.arsc");
        assert_eq!(
            ApkResources::load(&renamed_resources).err(),
            Some(Error::ZipMissingResources)
        );

        let duplicate_resources = duplicate_central(RESOURCE_APK, RESOURCES_ARSC_ENTRY);
        assert_eq!(
            ApkResources::load(&duplicate_resources).err(),
            Some(Error::ZipDuplicateResources)
        );

        let duplicate_layout = duplicate_central(RESOURCE_APK, DEFAULT_ACTIVITY_LAYOUT_ENTRY);
        let resources =
            ApkResources::load(&duplicate_layout).expect("resource table remains unique");
        assert_eq!(
            resources
                .resolve_text_view(DEFAULT_ACTIVITY_LAYOUT_ENTRY)
                .err(),
            Some(Error::ZipDuplicateLayout)
        );

        let compressed_resources = change_compression_method(RESOURCE_APK, RESOURCES_ARSC_ENTRY, 8);
        assert_eq!(
            ApkResources::load(&compressed_resources).err(),
            Some(Error::ZipCompressionUnsupported)
        );

        let mut bad_crc = RESOURCE_APK.to_vec();
        let layout = entry_layout(&bad_crc, RESOURCES_ARSC_ENTRY);
        bad_crc[layout.data.start + 32] ^= 1;
        assert_eq!(
            ApkResources::load(&bad_crc).err(),
            Some(Error::ZipCrcMismatch)
        );
    }

    #[test]
    fn layout_entry_name_is_canonical_and_must_exist() {
        let resources = ApkResources::load(RESOURCE_APK).expect("resources");
        for invalid in [
            b"activity_main.xml".as_slice(),
            b"res/layout/../layout/activity_main.xml",
            b"res/layout/ActivityMain.xml",
            b"res/layout/activity-main.xml",
            b"res/layout/.xml",
            b"res/layout/activity_main.bin",
        ] {
            assert_eq!(
                resources.layout_text_reference(invalid).err(),
                Some(Error::LayoutEntryName)
            );
        }
        assert_eq!(
            resources
                .layout_text_reference(b"res/layout/missing.xml")
                .err(),
            Some(Error::ZipMissingLayout)
        );
    }

    #[test]
    fn resource_table_header_pool_and_package_bounds_fail_closed() {
        let table = raw_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY);
        assert_eq!(
            ResourceTable::parse(&table[..table.len() - 1]).err(),
            Some(Error::ResourceTableHeader)
        );

        let mut bad_size = table.to_vec();
        write_u32(&mut bad_size, 4, u32::MAX);
        assert_eq!(
            ResourceTable::parse(&bad_size).err(),
            Some(Error::ResourceTableHeader)
        );

        let mut bad_pool_offset = table.to_vec();
        write_u32(&mut bad_pool_offset, 0x20, u32::MAX);
        assert_eq!(
            ResourceTable::parse(&bad_pool_offset).err(),
            Some(Error::ResourceStringPool)
        );

        let mut wrong_package_count = table.to_vec();
        write_u32(&mut wrong_package_count, 8, 2);
        assert_eq!(
            ResourceTable::parse(&wrong_package_count).err(),
            Some(Error::ResourcePackage)
        );

        let mut bad_package_id = table.to_vec();
        write_u32(&mut bad_package_id, 0x9c, 0);
        assert_eq!(
            ResourceTable::parse(&bad_package_id).err(),
            Some(Error::ResourcePackage)
        );
    }

    #[test]
    fn resource_type_entry_and_configuration_bounds_fail_closed() {
        let table = raw_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY);

        let mut sparse_offsets = table.to_vec();
        sparse_offsets[0x389] = 1;
        assert_eq!(
            ResourceTable::parse(&sparse_offsets).err(),
            Some(Error::ResourceType)
        );

        let mut complex_entry = table.to_vec();
        write_u16(&mut complex_entry, 0x3de, 1);
        assert_eq!(
            ResourceTable::parse(&complex_entry).err(),
            Some(Error::ResourceEntry)
        );

        let mut public_entry = table.to_vec();
        write_u16(&mut public_entry, 0x3de, ENTRY_FLAG_PUBLIC);
        ResourceTable::parse(&public_entry).expect("PUBLIC keeps the simple entry layout");

        let mut compact_entry = table.to_vec();
        write_u16(&mut compact_entry, 0x3de, 0x0008);
        assert_eq!(
            ResourceTable::parse(&compact_entry).err(),
            Some(Error::ResourceEntry)
        );

        let mut unknown_entry_flag = table.to_vec();
        write_u16(&mut unknown_entry_flag, 0x3de, 0x8000);
        assert_eq!(
            ResourceTable::parse(&unknown_entry_flag).err(),
            Some(Error::ResourceEntry)
        );

        let mut wrong_value_type = table.to_vec();
        wrong_value_type[0x3e7] = 0x10;
        let parsed = ResourceTable::parse(&wrong_value_type).expect("structure remains valid");
        assert_eq!(
            parsed.resolve_string(STRING_MESSAGE_ID).err(),
            Some(Error::ResourceValueType)
        );

        let mut qualified_only = table.to_vec();
        qualified_only[0x398] = 1;
        let parsed = ResourceTable::parse(&qualified_only).expect("qualified table is structural");
        assert_eq!(
            parsed.resolve_string(STRING_MESSAGE_ID).err(),
            Some(Error::ResourceNotFound)
        );
    }

    #[test]
    fn resource_ids_and_string_indices_are_bounded() {
        let table = ResourceTable::parse(raw_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY))
            .expect("real table");
        assert_eq!(
            table.resolve_string(0).err(),
            Some(Error::ResourceIdInvalid)
        );
        assert_eq!(
            table.resolve_string(0x7f00_0000).err(),
            Some(Error::ResourceIdInvalid)
        );
        assert_eq!(
            table.resolve_string(0x7f03_ffff).err(),
            Some(Error::ResourceNotFound)
        );
        assert_eq!(
            table.resolve_string(0x8003_0000).err(),
            Some(Error::ResourceNotFound)
        );
        assert_eq!(
            table.resolve_string(0x7f02_0000).err(),
            Some(Error::ResourceValueType)
        );
        assert_eq!(
            table.resolve_layout_entry(STRING_MESSAGE_ID).err(),
            Some(Error::ResourceValueType)
        );

        let mut bad_global_string_index = raw_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY).to_vec();
        write_u32(&mut bad_global_string_index, 0x3e8, u32::MAX);
        let parsed =
            ResourceTable::parse(&bad_global_string_index).expect("entry remains structural");
        assert_eq!(
            parsed.resolve_string(STRING_MESSAGE_ID).err(),
            Some(Error::ResourceReference)
        );

        let mut invalid_layout_path = raw_entry(RESOURCE_APK, RESOURCES_ARSC_ENTRY).to_vec();
        let path = find_bytes(&invalid_layout_path, DEFAULT_ACTIVITY_LAYOUT_ENTRY);
        invalid_layout_path[path] = b'x';
        let parsed = ResourceTable::parse(&invalid_layout_path).expect("valid string pool");
        assert_eq!(
            parsed.resolve_layout_entry(0x7f02_0000).err(),
            Some(Error::LayoutEntryName)
        );
    }

    #[test]
    fn compiled_layout_resource_map_namespace_and_reference_are_proven() {
        let layout = raw_entry(RESOURCE_APK, DEFAULT_ACTIVITY_LAYOUT_ENTRY);
        assert_eq!(
            parse_layout_text_reference(layout)
                .expect("real compiled layout")
                .resource_id,
            STRING_MESSAGE_ID
        );
        assert_eq!(
            parse_layout_text_reference(&layout[..layout.len() - 1]).err(),
            Some(Error::LayoutHeader)
        );

        let mut wrong_text_attribute_id = layout.to_vec();
        write_u32(&mut wrong_text_attribute_id, 0xc4, 0x0101_0001);
        assert_eq!(
            parse_layout_text_reference(&wrong_text_attribute_id).err(),
            Some(Error::LayoutAttribute)
        );

        let mut missing_resource_map_value = layout.to_vec();
        write_u32(&mut missing_resource_map_value, 0xc4, 0);
        assert_eq!(
            parse_layout_text_reference(&missing_resource_map_value).err(),
            Some(Error::LayoutResourceMap)
        );

        let mut wrong_namespace = layout.to_vec();
        wrong_namespace[0x82] = b'x';
        assert_eq!(
            parse_layout_text_reference(&wrong_namespace).err(),
            Some(Error::LayoutStructure)
        );

        let mut unknown_event = layout.to_vec();
        write_u16(&mut unknown_event, 0xe0, 0x0104);
        assert_eq!(
            parse_layout_text_reference(&unknown_event).err(),
            Some(Error::LayoutUnknownChunk(0x0104))
        );
    }

    #[test]
    fn compiled_text_attribute_must_be_one_nonzero_resource_reference() {
        let layout = raw_entry(RESOURCE_APK, DEFAULT_ACTIVITY_LAYOUT_ENTRY);

        let mut raw_value_present = layout.to_vec();
        write_u32(&mut raw_value_present, 0x148, 0);
        assert_eq!(
            parse_layout_text_reference(&raw_value_present).err(),
            Some(Error::LayoutAttribute)
        );

        let mut inline_integer = layout.to_vec();
        inline_integer[0x14f] = 0x10;
        assert_eq!(
            parse_layout_text_reference(&inline_integer).err(),
            Some(Error::LayoutAttribute)
        );

        let mut zero_reference = layout.to_vec();
        write_u32(&mut zero_reference, 0x150, 0);
        assert_eq!(
            parse_layout_text_reference(&zero_reference).err(),
            Some(Error::LayoutAttribute)
        );

        let mut no_text_view = layout.to_vec();
        no_text_view[0x68] = b'X';
        assert_eq!(
            parse_layout_text_reference(&no_text_view).err(),
            Some(Error::LayoutTextViewMissing)
        );

        let mut nested_text_view = layout.to_vec();
        let nested_copy = nested_text_view[0xe0..0x16c].to_vec();
        nested_text_view.splice(0x154..0x154, nested_copy);
        let new_size = u32::try_from(nested_text_view.len()).expect("layout size");
        write_u32(&mut nested_text_view, 4, new_size);
        assert_eq!(
            parse_layout_text_reference(&nested_text_view).err(),
            Some(Error::LayoutStructure)
        );

        let mut namespaced_root = layout.to_vec();
        write_u32(&mut namespaced_root, 0xf0, 0);
        assert_eq!(
            parse_layout_text_reference(&namespaced_root).err(),
            Some(Error::LayoutStructure)
        );
    }

    #[test]
    fn resource_string_conversion_is_utf8_bounded_without_allocation() {
        let utf8 = "你好🦀";
        let utf8_value = PoolString {
            bytes: utf8.as_bytes(),
            utf16_units: utf8.encode_utf16().count(),
            encoding: Encoding::Utf8,
        }
        .to_resource_string()
        .expect("UTF-8 resource");
        assert_eq!(utf8_value.as_str(), utf8);

        let utf16_bytes = [0x60, 0x4f, 0x7d, 0x59, 0x3e, 0xd8, 0x80, 0xdd];
        let utf16_value = PoolString {
            bytes: &utf16_bytes,
            utf16_units: 4,
            encoding: Encoding::Utf16,
        }
        .to_resource_string()
        .expect("UTF-16 resource");
        assert_eq!(utf16_value, utf8_value);

        let malformed_surrogate = [0x3e, 0xd8, b'A', 0];
        assert_eq!(
            PoolString {
                bytes: &malformed_surrogate,
                utf16_units: 2,
                encoding: Encoding::Utf16,
            }
            .to_resource_string()
            .err(),
            Some(Error::ResourceString)
        );

        let too_long = [b'a'; MAX_RESOURCE_STRING_BYTES + 1];
        assert_eq!(
            PoolString {
                bytes: &too_long,
                utf16_units: too_long.len(),
                encoding: Encoding::Utf8,
            }
            .to_resource_string()
            .err(),
            Some(Error::ResourceStringTooLong)
        );
    }

    #[derive(Clone)]
    struct EntryLayout {
        local: usize,
        central: usize,
        data: Range<usize>,
    }

    fn raw_entry<'a>(apk: &'a [u8], name: &[u8]) -> &'a [u8] {
        let layout = entry_layout(apk, name);
        &apk[layout.data]
    }

    fn entry_layout(apk: &[u8], expected: &[u8]) -> EntryLayout {
        let eocd = find_signature_from_end(apk, 0x0605_4b50);
        let entry_count = usize::from(read_u16(apk, eocd + 10));
        let mut central = usize::try_from(read_u32(apk, eocd + 16)).expect("central offset");
        let mut index = 0usize;
        while index < entry_count {
            assert_eq!(read_u32(apk, central), 0x0201_4b50);
            let name_len = usize::from(read_u16(apk, central + 28));
            let extra_len = usize::from(read_u16(apk, central + 30));
            let comment_len = usize::from(read_u16(apk, central + 32));
            let name = &apk[central + 46..central + 46 + name_len];
            if name == expected {
                let local = usize::try_from(read_u32(apk, central + 42)).expect("local offset");
                assert_eq!(read_u32(apk, local), 0x0403_4b50);
                let local_name_len = usize::from(read_u16(apk, local + 26));
                let local_extra_len = usize::from(read_u16(apk, local + 28));
                let data_start = local + 30 + local_name_len + local_extra_len;
                let data_len = usize::try_from(read_u32(apk, central + 20)).expect("stored size");
                return EntryLayout {
                    local,
                    central,
                    data: data_start..data_start + data_len,
                };
            }
            central += 46 + name_len + extra_len + comment_len;
            index += 1;
        }
        panic!("missing test fixture entry");
    }

    fn rename_entry(apk: &[u8], old: &[u8], new: &[u8]) -> std::vec::Vec<u8> {
        assert_eq!(old.len(), new.len());
        let mut changed = apk.to_vec();
        let layout = entry_layout(&changed, old);
        let local_name = layout.local + 30;
        changed[local_name..local_name + old.len()].copy_from_slice(new);
        let central_name = layout.central + 46;
        changed[central_name..central_name + old.len()].copy_from_slice(new);
        changed
    }

    fn change_compression_method(apk: &[u8], name: &[u8], method: u16) -> std::vec::Vec<u8> {
        let mut changed = apk.to_vec();
        let layout = entry_layout(&changed, name);
        write_u16(&mut changed, layout.local + 8, method);
        write_u16(&mut changed, layout.central + 10, method);
        changed
    }

    fn duplicate_central(apk: &[u8], name: &[u8]) -> std::vec::Vec<u8> {
        let layout = entry_layout(apk, name);
        let record_len = central_record_len(apk, layout.central);
        let duplicate = &apk[layout.central..layout.central + record_len];
        let eocd = find_signature_from_end(apk, 0x0605_4b50);
        let mut changed = apk[..eocd].to_vec();
        changed.extend_from_slice(duplicate);
        let new_eocd = changed.len();
        changed.extend_from_slice(&apk[eocd..]);
        let entries = read_u16(apk, eocd + 10);
        write_u16(&mut changed, new_eocd + 8, entries + 1);
        write_u16(&mut changed, new_eocd + 10, entries + 1);
        let central_size = read_u32(apk, eocd + 12);
        write_u32(
            &mut changed,
            new_eocd + 12,
            central_size + u32::try_from(record_len).expect("central record"),
        );
        changed
    }

    fn central_record_len(apk: &[u8], offset: usize) -> usize {
        46 + usize::from(read_u16(apk, offset + 28))
            + usize::from(read_u16(apk, offset + 30))
            + usize::from(read_u16(apk, offset + 32))
    }

    fn find_signature_from_end(bytes: &[u8], signature: u32) -> usize {
        bytes
            .windows(4)
            .rposition(|window| {
                u32::from_le_bytes([window[0], window[1], window[2], window[3]]) == signature
            })
            .expect("signature")
    }

    fn find_bytes(bytes: &[u8], needle: &[u8]) -> usize {
        bytes
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("fixture bytes")
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
