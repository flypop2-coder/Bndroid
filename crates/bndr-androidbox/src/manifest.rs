use crate::{
    ActivityDescriptor, ApplicationLabel, BoundedText, Error, MAX_MANIFEST_BYTES, ManifestInfo,
    PackageName,
};
#[cfg(feature = "androidbox-manifest-catalog3")]
use crate::{
    ComponentDescriptor, MAX_MANIFEST_COMPONENTS, MAX_MANIFEST_PERMISSIONS, ManifestCatalog,
    ManifestComponent, ManifestComponentKind, PermissionName,
};

const NO_INDEX: u32 = u32::MAX;
const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_XML_TYPE: u16 = 0x0003;
const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
const RES_XML_END_NAMESPACE_TYPE: u16 = 0x0101;
const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;
const RES_XML_RESOURCE_MAP_TYPE: u16 = 0x0180;
const STRING_POOL_UTF8_FLAG: u32 = 1 << 8;
const TYPE_REFERENCE: u8 = 0x01;
const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;
#[cfg(feature = "androidbox-manifest-catalog3")]
const TYPE_INT_HEX: u8 = 0x11;
const TYPE_INT_BOOLEAN: u8 = 0x12;
#[cfg(not(feature = "androidbox-manifest-catalog3"))]
const MAX_XML_STRINGS: u32 = 128;
#[cfg(feature = "androidbox-manifest-catalog3")]
const MAX_XML_STRINGS: u32 = 512;
const MAX_XML_STRING_BYTES: usize = 256;
#[cfg(not(feature = "androidbox-manifest-catalog3"))]
const MAX_ATTRIBUTES: u16 = 16;
#[cfg(feature = "androidbox-manifest-catalog3")]
const MAX_ATTRIBUTES: u16 = 32;
#[cfg(not(feature = "androidbox-manifest-catalog3"))]
const MAX_DEPTH: usize = 8;
#[cfg(feature = "androidbox-manifest-catalog3")]
const MAX_DEPTH: usize = 12;
const ANDROID_URI: &[u8] = b"http://schemas.android.com/apk/res/android";
const RESOURCE_ATTRIBUTES: [(&[u8], u32); 12] = [
    (b"label", 0x0101_0001),
    (b"name", 0x0101_0003),
    (b"hasCode", 0x0101_000c),
    (b"exported", 0x0101_0010),
    (b"minSdkVersion", 0x0101_020c),
    (b"versionCode", 0x0101_021b),
    (b"versionName", 0x0101_021c),
    (b"targetSdkVersion", 0x0101_0270),
    (b"allowBackup", 0x0101_0280),
    (b"supportsRtl", 0x0101_03af),
    (b"compileSdkVersion", 0x0101_0572),
    (b"compileSdkVersionCodename", 0x0101_0573),
];
const ANDROID_ICON_ATTRIBUTE_ID: u32 = 0x0101_0002;

#[derive(Clone, Copy)]
enum Encoding {
    Utf8,
    Utf16,
}

#[derive(Clone, Copy)]
struct XmlString<'a> {
    bytes: &'a [u8],
    length: usize,
    encoding: Encoding,
}

impl XmlString<'_> {
    fn equals(self, expected: &[u8]) -> bool {
        if self.length != expected.len() {
            return false;
        }
        let mut index = 0usize;
        while index < self.length {
            if self.ascii_at(index) != Some(expected[index]) {
                return false;
            }
            index += 1;
        }
        true
    }

    fn ascii_at(self, index: usize) -> Option<u8> {
        if index >= self.length {
            return None;
        }
        match self.encoding {
            Encoding::Utf8 => self.bytes.get(index).copied(),
            Encoding::Utf16 => {
                let offset = index.checked_mul(2)?;
                let low = *self.bytes.get(offset)?;
                let high = *self.bytes.get(offset + 1)?;
                (high == 0).then_some(low)
            }
        }
    }

    fn bounded<const N: usize>(
        self,
        too_long: Error,
        invalid: Error,
    ) -> Result<BoundedText<N>, Error> {
        if self.length > N || self.length > usize::from(u16::MAX) {
            return Err(too_long);
        }
        if self.length == 0 {
            return Err(invalid);
        }
        let mut result = BoundedText::empty();
        let mut index = 0usize;
        while index < self.length {
            let byte = self.ascii_at(index).ok_or(invalid)?;
            if !byte.is_ascii() || byte == 0 {
                return Err(invalid);
            }
            result.bytes[index] = byte;
            index += 1;
        }
        result.len = u16::try_from(self.length).map_err(|_| too_long)?;
        Ok(result)
    }
}

#[derive(Clone, Copy)]
struct StringPool<'a> {
    bytes: &'a [u8],
    chunk_end: usize,
    offsets_start: usize,
    strings_start: usize,
    count: u32,
    encoding: Encoding,
}

impl<'a> StringPool<'a> {
    fn parse(bytes: &'a [u8], offset: usize) -> Result<(Self, usize), Error> {
        let header = chunk_header(bytes, offset, Error::ManifestStringPool)?;
        if header.kind != RES_STRING_POOL_TYPE || header.header_size != 28 {
            return Err(Error::ManifestStringPool);
        }
        let string_count = read_u32(bytes, offset + 8, Error::ManifestStringPool)?;
        let style_count = read_u32(bytes, offset + 12, Error::ManifestStringPool)?;
        let flags = read_u32(bytes, offset + 16, Error::ManifestStringPool)?;
        let strings_start = read_u32(bytes, offset + 20, Error::ManifestStringPool)?;
        let styles_start = read_u32(bytes, offset + 24, Error::ManifestStringPool)?;
        if string_count == 0
            || string_count > MAX_XML_STRINGS
            || style_count != 0
            || styles_start != 0
            || flags & !STRING_POOL_UTF8_FLAG != 0
        {
            return Err(Error::ManifestStringPool);
        }
        let offsets_bytes = string_count
            .checked_mul(4)
            .ok_or(Error::ManifestStringPool)?;
        let canonical_strings_start = 28u32
            .checked_add(offsets_bytes)
            .ok_or(Error::ManifestStringPool)?;
        if strings_start != canonical_strings_start || strings_start >= header.size {
            return Err(Error::ManifestStringPool);
        }
        let offsets_start = offset + 28;
        let data_start = offset
            .checked_add(usize_from(strings_start, Error::ManifestStringPool)?)
            .ok_or(Error::ManifestStringPool)?;
        if data_start >= header.end {
            return Err(Error::ManifestStringPool);
        }
        let pool = Self {
            bytes,
            chunk_end: header.end,
            offsets_start,
            strings_start: data_start,
            count: string_count,
            encoding: if flags == STRING_POOL_UTF8_FLAG {
                Encoding::Utf8
            } else {
                Encoding::Utf16
            },
        };
        pool.validate()?;
        Ok((pool, header.end))
    }

    fn validate(self) -> Result<(), Error> {
        let mut expected_relative = 0usize;
        let mut index = 0u32;
        while index < self.count {
            let relative = usize_from(
                read_u32(
                    self.bytes,
                    self.offsets_start
                        .checked_add(
                            usize_from(index, Error::ManifestStringPool)?
                                .checked_mul(4)
                                .ok_or(Error::ManifestStringPool)?,
                        )
                        .ok_or(Error::ManifestStringPool)?,
                    Error::ManifestStringPool,
                )?,
                Error::ManifestStringPool,
            )?;
            if relative != expected_relative {
                return Err(Error::ManifestStringPool);
            }
            let (value, end) = parse_pool_string(
                self.bytes,
                self.strings_start
                    .checked_add(relative)
                    .ok_or(Error::ManifestStringPool)?,
                self.chunk_end,
                self.encoding,
            )?;
            if value.length == 0 || value.length > MAX_XML_STRING_BYTES {
                return Err(Error::ManifestString);
            }
            let mut byte_index = 0usize;
            while byte_index < value.length {
                let byte = value.ascii_at(byte_index).ok_or(Error::ManifestString)?;
                if !byte.is_ascii() || byte == 0 {
                    return Err(Error::ManifestString);
                }
                byte_index += 1;
            }
            let mut earlier = 0u32;
            while earlier < index {
                if self.get(earlier)?.equals_xml(value) {
                    return Err(Error::ManifestStringPool);
                }
                earlier += 1;
            }
            expected_relative = end
                .checked_sub(self.strings_start)
                .ok_or(Error::ManifestStringPool)?;
            index += 1;
        }
        if self.chunk_end - self.strings_start - expected_relative > 3
            || self.bytes[self.strings_start + expected_relative..self.chunk_end]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(Error::ManifestStringPool);
        }
        Ok(())
    }

    fn get(self, index: u32) -> Result<XmlString<'a>, Error> {
        if index >= self.count {
            return Err(Error::ManifestString);
        }
        let table_offset = self
            .offsets_start
            .checked_add(
                usize_from(index, Error::ManifestString)?
                    .checked_mul(4)
                    .ok_or(Error::ManifestString)?,
            )
            .ok_or(Error::ManifestString)?;
        let relative = usize_from(
            read_u32(self.bytes, table_offset, Error::ManifestString)?,
            Error::ManifestString,
        )?;
        let (value, _) = parse_pool_string(
            self.bytes,
            self.strings_start
                .checked_add(relative)
                .ok_or(Error::ManifestString)?,
            self.chunk_end,
            self.encoding,
        )?;
        Ok(value)
    }

    fn equals(self, index: u32, expected: &[u8]) -> Result<bool, Error> {
        Ok(self.get(index)?.equals(expected))
    }
}

trait XmlStringEquality {
    fn equals_xml(self, other: XmlString<'_>) -> bool;
}

impl XmlStringEquality for XmlString<'_> {
    fn equals_xml(self, other: XmlString<'_>) -> bool {
        if self.length != other.length {
            return false;
        }
        let mut index = 0usize;
        while index < self.length {
            if self.ascii_at(index) != other.ascii_at(index) {
                return false;
            }
            index += 1;
        }
        true
    }
}

#[derive(Clone, Copy)]
struct Chunk {
    kind: u16,
    header_size: u16,
    size: u32,
    end: usize,
}

#[derive(Clone, Copy)]
struct Attribute {
    namespace: u32,
    name: u32,
    raw_value: u32,
    value_type: u8,
    data: u32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Element {
    Empty,
    Manifest,
    UsesSdk,
    Application,
    Activity,
    IntentFilter,
    Action,
    Category,
}

struct ParseState {
    stack: [Element; MAX_DEPTH],
    depth: usize,
    root_closed: bool,
    uses_sdk_seen: bool,
    application_seen: bool,
    activity_count: u16,
    package: Option<PackageName>,
    version_code: Option<u32>,
    application_label: Option<ApplicationLabelValue>,
    application_icon_resource_id: Option<u32>,
    current_activity: Option<ActivityDescriptor>,
    current_exported: Option<bool>,
    current_activity_launcher: bool,
    filter_main: bool,
    filter_launcher: bool,
    launcher: Option<ActivityDescriptor>,
}

#[derive(Clone, Copy)]
enum ApplicationLabelValue {
    Inline(ApplicationLabel),
    Resource(u32),
}

impl ParseState {
    const fn new() -> Self {
        Self {
            stack: [Element::Empty; MAX_DEPTH],
            depth: 0,
            root_closed: false,
            uses_sdk_seen: false,
            application_seen: false,
            activity_count: 0,
            package: None,
            version_code: None,
            application_label: None,
            application_icon_resource_id: None,
            current_activity: None,
            current_exported: None,
            current_activity_launcher: false,
            filter_main: false,
            filter_launcher: false,
            launcher: None,
        }
    }

    fn parent(&self) -> Element {
        if self.depth == 0 {
            Element::Empty
        } else {
            self.stack[self.depth - 1]
        }
    }

    fn push(&mut self, element: Element) -> Result<(), Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::ManifestStructure);
        }
        self.stack[self.depth] = element;
        self.depth += 1;
        Ok(())
    }

    fn pop(&mut self, element: Element) -> Result<(), Error> {
        if self.depth == 0 || self.stack[self.depth - 1] != element {
            return Err(Error::ManifestStructure);
        }
        self.depth -= 1;
        self.stack[self.depth] = Element::Empty;
        Ok(())
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<ManifestInfo, Error> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error::ManifestTooLarge);
    }
    let document = chunk_header(bytes, 0, Error::ManifestHeader)?;
    if document.kind != RES_XML_TYPE
        || document.header_size != 8
        || usize_from(document.size, Error::ManifestHeader)? != bytes.len()
    {
        return Err(Error::ManifestHeader);
    }
    let (pool, mut cursor) = StringPool::parse(bytes, 8)?;
    cursor = validate_resource_map(bytes, cursor, pool)?;
    cursor = parse_namespace(bytes, cursor, pool, true)?;

    let mut state = ParseState::new();
    let mut namespace_closed = false;
    while cursor < bytes.len() {
        let chunk = chunk_header(bytes, cursor, Error::ManifestStructure)?;
        match chunk.kind {
            RES_XML_START_ELEMENT_TYPE => {
                parse_start_element(bytes, cursor, chunk, pool, &mut state)?;
            }
            RES_XML_END_ELEMENT_TYPE => {
                parse_end_element(bytes, cursor, chunk, pool, &mut state)?;
            }
            RES_XML_END_NAMESPACE_TYPE => {
                if state.depth != 0 || !state.root_closed {
                    return Err(Error::ManifestStructure);
                }
                cursor = parse_namespace(bytes, cursor, pool, false)?;
                namespace_closed = true;
                if cursor != bytes.len() {
                    return Err(Error::ManifestStructure);
                }
                break;
            }
            kind => return Err(Error::ManifestUnknownChunk(kind)),
        }
        cursor = chunk.end;
    }
    if cursor != bytes.len() || state.depth != 0 || !state.root_closed || !namespace_closed {
        return Err(Error::ManifestStructure);
    }
    let package = state.package.ok_or(Error::ManifestPackageMissing)?;
    let version_code = state
        .version_code
        .ok_or(Error::ManifestVersionCodeMissing)?;
    if !state.application_seen {
        return Err(Error::ManifestApplicationMissing);
    }
    let application_label = state
        .application_label
        .ok_or(Error::ManifestApplicationMissing)?;
    if state.activity_count == 0 {
        return Err(Error::ManifestActivityMissing);
    }
    let activity_descriptor = state.launcher.ok_or(Error::ManifestActivityMissing)?;
    let (application_label, application_label_resource_id) = match application_label {
        ApplicationLabelValue::Inline(label) => (label, None),
        ApplicationLabelValue::Resource(resource_id) => {
            (ApplicationLabel::empty(), Some(resource_id))
        }
    };
    Ok(ManifestInfo {
        android_manifest_crc32: 0,
        package,
        version_code,
        activity_descriptor,
        application_label,
        application_label_resource_id,
        application_icon_resource_id: state.application_icon_resource_id,
    })
}

#[cfg(feature = "androidbox-manifest-catalog3")]
#[derive(Clone, Copy, Eq, PartialEq)]
enum CatalogElement {
    Empty,
    Manifest,
    UsesSdk,
    UsesPermission,
    UsesFeature,
    Application,
    Activity,
    ActivityAlias,
    Service,
    Receiver,
    Provider,
    IntentFilter,
    Action,
    Category,
    Data,
    MetaData,
}

#[cfg(feature = "androidbox-manifest-catalog3")]
pub(crate) struct CatalogParseState {
    stack: [CatalogElement; MAX_DEPTH],
    depth: usize,
    root_closed: bool,
    application_seen: bool,
    package: Option<PackageName>,
    version_code: Option<u32>,
    application_label: Option<ApplicationLabelValue>,
    application_icon_resource_id: Option<u32>,
    components: [ManifestComponent; MAX_MANIFEST_COMPONENTS],
    component_count: u8,
    permissions: [PermissionName; MAX_MANIFEST_PERMISSIONS],
    permission_count: u8,
    launcher_count: u8,
    primary_launcher_index: Option<u8>,
    current_component_index: Option<u8>,
    filter_main: bool,
    filter_launcher: bool,
    filter_action_count: u16,
    filter_category_count: u16,
    filter_data_count: u16,
    intent_filter_count: u16,
    action_count: u16,
    category_count: u16,
    data_count: u16,
}

#[cfg(feature = "androidbox-manifest-catalog3")]
impl CatalogParseState {
    pub(crate) const fn new() -> Self {
        Self {
            stack: [CatalogElement::Empty; MAX_DEPTH],
            depth: 0,
            root_closed: false,
            application_seen: false,
            package: None,
            version_code: None,
            application_label: None,
            application_icon_resource_id: None,
            components: [ManifestComponent::empty(); MAX_MANIFEST_COMPONENTS],
            component_count: 0,
            permissions: [PermissionName::empty(); MAX_MANIFEST_PERMISSIONS],
            permission_count: 0,
            launcher_count: 0,
            primary_launcher_index: None,
            current_component_index: None,
            filter_main: false,
            filter_launcher: false,
            filter_action_count: 0,
            filter_category_count: 0,
            filter_data_count: 0,
            intent_filter_count: 0,
            action_count: 0,
            category_count: 0,
            data_count: 0,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.stack.fill(CatalogElement::Empty);
        self.depth = 0;
        self.root_closed = false;
        self.application_seen = false;
        self.package = None;
        self.version_code = None;
        self.application_label = None;
        self.application_icon_resource_id = None;
        self.components.fill(ManifestComponent::empty());
        self.component_count = 0;
        self.permissions.fill(PermissionName::empty());
        self.permission_count = 0;
        self.launcher_count = 0;
        self.primary_launcher_index = None;
        self.current_component_index = None;
        self.filter_main = false;
        self.filter_launcher = false;
        self.filter_action_count = 0;
        self.filter_category_count = 0;
        self.filter_data_count = 0;
        self.intent_filter_count = 0;
        self.action_count = 0;
        self.category_count = 0;
        self.data_count = 0;
    }

    pub(crate) fn is_clear(&self) -> bool {
        self.stack
            .iter()
            .all(|element| *element == CatalogElement::Empty)
            && self.depth == 0
            && !self.root_closed
            && !self.application_seen
            && self.package.is_none()
            && self.version_code.is_none()
            && self.application_label.is_none()
            && self.application_icon_resource_id.is_none()
            && self
                .components
                .iter()
                .all(|component| *component == ManifestComponent::empty())
            && self.component_count == 0
            && self.permissions.iter().all(BoundedText::is_empty)
            && self.permission_count == 0
            && self.launcher_count == 0
            && self.primary_launcher_index.is_none()
            && self.current_component_index.is_none()
            && !self.filter_main
            && !self.filter_launcher
            && self.filter_action_count == 0
            && self.filter_category_count == 0
            && self.filter_data_count == 0
            && self.intent_filter_count == 0
            && self.action_count == 0
            && self.category_count == 0
            && self.data_count == 0
    }

    fn catalog(&self) -> Result<ManifestCatalog, Error> {
        let package = self.package.ok_or(Error::ManifestPackageMissing)?;
        let version_code = self.version_code.ok_or(Error::ManifestVersionCodeMissing)?;
        if !self.application_seen {
            return Err(Error::ManifestApplicationMissing);
        }
        let (application_label, application_label_resource_id) = match self.application_label {
            Some(ApplicationLabelValue::Inline(label)) => (label, None),
            Some(ApplicationLabelValue::Resource(resource_id)) => {
                (ApplicationLabel::empty(), Some(resource_id))
            }
            None => (ApplicationLabel::empty(), None),
        };
        Ok(ManifestCatalog {
            package,
            version_code,
            application_label,
            application_label_resource_id,
            application_icon_resource_id: self.application_icon_resource_id,
            components: self.components,
            component_count: self.component_count,
            permissions: self.permissions,
            permission_count: self.permission_count,
            launcher_count: self.launcher_count,
            primary_launcher_index: self.primary_launcher_index,
            intent_filter_count: self.intent_filter_count,
            action_count: self.action_count,
            category_count: self.category_count,
            data_count: self.data_count,
        })
    }

    fn parent(&self) -> CatalogElement {
        if self.depth == 0 {
            CatalogElement::Empty
        } else {
            self.stack[self.depth - 1]
        }
    }

    fn push(&mut self, element: CatalogElement) -> Result<(), Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::ManifestStructure);
        }
        self.stack[self.depth] = element;
        self.depth += 1;
        Ok(())
    }

    fn pop(&mut self, element: CatalogElement) -> Result<(), Error> {
        if self.depth == 0 || self.stack[self.depth - 1] != element {
            return Err(Error::ManifestStructure);
        }
        self.depth -= 1;
        self.stack[self.depth] = CatalogElement::Empty;
        Ok(())
    }

    fn append_component(&mut self, component: ManifestComponent) -> Result<(), Error> {
        if self.current_component_index.is_some()
            || usize::from(self.component_count) >= MAX_MANIFEST_COMPONENTS
        {
            return Err(Error::ManifestCatalogTooManyComponents);
        }
        let index = self.component_count;
        self.components[usize::from(index)] = component;
        self.component_count = self
            .component_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogTooManyComponents)?;
        self.current_component_index = Some(index);
        Ok(())
    }

    fn current_component_mut(&mut self) -> Result<&mut ManifestComponent, Error> {
        let index = self
            .current_component_index
            .ok_or(Error::ManifestCatalogComponentInvalid)?;
        self.components
            .get_mut(usize::from(index))
            .ok_or(Error::ManifestCatalogComponentInvalid)
    }

    fn append_permission(&mut self, permission: PermissionName) -> Result<(), Error> {
        if self.permissions[..usize::from(self.permission_count)]
            .iter()
            .any(|existing| existing == &permission)
        {
            return Ok(());
        }
        if usize::from(self.permission_count) >= MAX_MANIFEST_PERMISSIONS {
            return Err(Error::ManifestCatalogTooManyPermissions);
        }
        self.permissions[usize::from(self.permission_count)] = permission;
        self.permission_count = self
            .permission_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogTooManyPermissions)?;
        Ok(())
    }
}

/// Parse a bounded component directory from binary `AndroidManifest.xml`.
///
/// Unlike the historical execution-profile parser, this accepts several
/// component kinds, non-launcher intent filters, multiple launcher
/// Activities, manifest permissions, common metadata leaves, and a
/// non-fixture-specific resource map. It still rejects malformed binary XML,
/// capacity overflow, encrypted authority, and non-ASCII identity strings.
#[cfg(feature = "androidbox-manifest-catalog3")]
pub(crate) fn parse_catalog(bytes: &[u8]) -> Result<ManifestCatalog, Error> {
    let mut state = CatalogParseState::new();
    parse_catalog_into(bytes, &mut state)?;
    state.catalog()
}

#[cfg(feature = "androidbox-manifest-catalog3")]
pub(crate) fn parse_catalog_into(bytes: &[u8], state: &mut CatalogParseState) -> Result<(), Error> {
    state.clear();
    let result = parse_catalog_inner(bytes, state);
    if result.is_err() {
        state.clear();
    }
    result
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_inner(bytes: &[u8], state: &mut CatalogParseState) -> Result<(), Error> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error::ManifestTooLarge);
    }
    let document = chunk_header(bytes, 0, Error::ManifestHeader)?;
    if document.kind != RES_XML_TYPE
        || document.header_size != 8
        || usize_from(document.size, Error::ManifestHeader)? != bytes.len()
    {
        return Err(Error::ManifestHeader);
    }
    let (pool, mut cursor) = StringPool::parse(bytes, 8)?;
    cursor = validate_catalog_resource_map(bytes, cursor, pool)?;
    cursor = parse_namespace(bytes, cursor, pool, true)?;

    let mut namespace_closed = false;
    while cursor < bytes.len() {
        let chunk = chunk_header(bytes, cursor, Error::ManifestStructure)?;
        match chunk.kind {
            RES_XML_START_ELEMENT_TYPE => {
                parse_catalog_start_element(bytes, cursor, chunk, pool, state)?;
            }
            RES_XML_END_ELEMENT_TYPE => {
                parse_catalog_end_element(bytes, cursor, chunk, pool, state)?;
            }
            RES_XML_END_NAMESPACE_TYPE => {
                if state.depth != 0 || !state.root_closed {
                    return Err(Error::ManifestStructure);
                }
                cursor = parse_namespace(bytes, cursor, pool, false)?;
                namespace_closed = true;
                if cursor != bytes.len() {
                    return Err(Error::ManifestStructure);
                }
                break;
            }
            kind => return Err(Error::ManifestUnknownChunk(kind)),
        }
        cursor = chunk.end;
    }
    if cursor != bytes.len() || state.depth != 0 || !state.root_closed || !namespace_closed {
        return Err(Error::ManifestStructure);
    }
    state.package.ok_or(Error::ManifestPackageMissing)?;
    state
        .version_code
        .ok_or(Error::ManifestVersionCodeMissing)?;
    if !state.application_seen {
        return Err(Error::ManifestApplicationMissing);
    }
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
pub(crate) fn selected_manifest_info(
    state: &CatalogParseState,
    android_manifest_crc32: u32,
) -> Result<ManifestInfo, Error> {
    let package = state.package.ok_or(Error::ManifestPackageMissing)?;
    let version_code = state
        .version_code
        .ok_or(Error::ManifestVersionCodeMissing)?;
    let launcher = state.components[..usize::from(state.component_count)]
        .iter()
        .find(|component| component.launcher && component.enabled && component.exported)
        .ok_or(Error::ManifestActivityMissing)?;
    let activity_descriptor = match launcher.kind {
        ManifestComponentKind::Activity => launcher.descriptor,
        ManifestComponentKind::ActivityAlias => {
            if launcher.alias_target.is_empty() {
                return Err(Error::ManifestCatalogComponentInvalid);
            }
            launcher.alias_target
        }
        ManifestComponentKind::Service
        | ManifestComponentKind::Receiver
        | ManifestComponentKind::Provider => return Err(Error::ManifestLauncherInvalid),
    };
    let (application_label, application_label_resource_id) = match state.application_label {
        Some(ApplicationLabelValue::Inline(label)) => (label, None),
        Some(ApplicationLabelValue::Resource(resource_id)) => {
            (ApplicationLabel::empty(), Some(resource_id))
        }
        None => (ApplicationLabel::empty(), None),
    };
    Ok(ManifestInfo {
        android_manifest_crc32,
        package,
        version_code,
        activity_descriptor,
        application_label,
        application_label_resource_id,
        application_icon_resource_id: state.application_icon_resource_id,
    })
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn validate_catalog_resource_map(
    bytes: &[u8],
    offset: usize,
    pool: StringPool<'_>,
) -> Result<usize, Error> {
    let chunk = chunk_header(bytes, offset, Error::ManifestResourceMap)?;
    if chunk.kind != RES_XML_RESOURCE_MAP_TYPE {
        return Ok(offset);
    }
    if chunk.header_size != 8 || chunk.size < 8 || !chunk.size.is_multiple_of(4) {
        return Err(Error::ManifestResourceMap);
    }
    let count = (chunk.size - 8) / 4;
    if count > pool.count {
        return Err(Error::ManifestResourceMap);
    }
    let mut index = 0u32;
    while index < count {
        let value = read_u32(
            bytes,
            offset
                .checked_add(8)
                .and_then(|base| {
                    base.checked_add(usize_from(index, Error::ManifestResourceMap).ok()? * 4)
                })
                .ok_or(Error::ManifestResourceMap)?,
            Error::ManifestResourceMap,
        )?;
        if value != 0 {
            let name = pool.get(index)?;
            if let Some(expected) = catalog_known_android_resource_id(name)
                && value != expected
            {
                return Err(Error::ManifestResourceMap);
            }
            let mut earlier = 0u32;
            while earlier < index {
                if read_u32(
                    bytes,
                    offset + 8 + usize_from(earlier, Error::ManifestResourceMap)? * 4,
                    Error::ManifestResourceMap,
                )? == value
                {
                    return Err(Error::ManifestResourceMap);
                }
                earlier += 1;
            }
        }
        index += 1;
    }
    Ok(chunk.end)
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn catalog_known_android_resource_id(name: XmlString<'_>) -> Option<u32> {
    for (known, resource_id) in RESOURCE_ATTRIBUTES {
        if name.equals(known) {
            return Some(resource_id);
        }
    }
    for (known, resource_id) in [
        (&b"icon"[..], ANDROID_ICON_ATTRIBUTE_ID),
        (&b"permission"[..], 0x0101_0006),
        (&b"enabled"[..], 0x0101_000e),
        (&b"authorities"[..], 0x0101_0018),
        (&b"priority"[..], 0x0101_001c),
        (&b"targetActivity"[..], 0x0101_0202),
        (&b"scheme"[..], 0x0101_0027),
        (&b"host"[..], 0x0101_0028),
        (&b"port"[..], 0x0101_0029),
        (&b"path"[..], 0x0101_002a),
        (&b"mimeType"[..], 0x0101_0026),
    ] {
        if name.equals(known) {
            return Some(resource_id);
        }
    }
    None
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_start_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    pool: StringPool<'_>,
    state: &mut CatalogParseState,
) -> Result<(), Error> {
    if chunk.header_size != 16 || chunk.size < 36 {
        return Err(Error::ManifestStructure);
    }
    validate_node_header(bytes, offset)?;
    let namespace = read_u32(bytes, offset + 16, Error::ManifestStructure)?;
    let name = read_u32(bytes, offset + 20, Error::ManifestStructure)?;
    let attributes_start = read_u16(bytes, offset + 24, Error::ManifestStructure)?;
    let attribute_size = read_u16(bytes, offset + 26, Error::ManifestStructure)?;
    let attribute_count = read_u16(bytes, offset + 28, Error::ManifestStructure)?;
    let id_index = read_u16(bytes, offset + 30, Error::ManifestStructure)?;
    let class_index = read_u16(bytes, offset + 32, Error::ManifestStructure)?;
    let style_index = read_u16(bytes, offset + 34, Error::ManifestStructure)?;
    if namespace != NO_INDEX
        || attributes_start != 20
        || attribute_size != 20
        || attribute_count > MAX_ATTRIBUTES
        || id_index != 0
        || class_index != 0
        || style_index != 0
        || chunk.size != 36 + u32::from(attribute_count) * 20
    {
        return Err(Error::ManifestStructure);
    }
    let attributes = offset + 36;
    validate_attribute_uniqueness(bytes, attributes, attribute_count, pool)?;

    let parent = state.parent();
    let element = if pool.equals(name, b"manifest")? {
        if parent != CatalogElement::Empty || state.root_closed {
            return Err(Error::ManifestStructure);
        }
        parse_catalog_manifest_attributes(bytes, attributes, attribute_count, pool, state)?;
        CatalogElement::Manifest
    } else if pool.equals(name, b"uses-sdk")? {
        if parent != CatalogElement::Manifest {
            return Err(Error::ManifestStructure);
        }
        validate_catalog_leaf_attributes(bytes, attributes, attribute_count, pool)?;
        CatalogElement::UsesSdk
    } else if pool.equals(name, b"uses-permission")? {
        if parent != CatalogElement::Manifest {
            return Err(Error::ManifestStructure);
        }
        let permission =
            parse_catalog_named_leaf(bytes, attributes, attribute_count, pool, b"name")?;
        state.append_permission(validate_permission_name(permission.bounded(
            Error::ManifestCatalogPermissionInvalid,
            Error::ManifestCatalogPermissionInvalid,
        )?)?)?;
        CatalogElement::UsesPermission
    } else if pool.equals(name, b"uses-feature")? {
        if parent != CatalogElement::Manifest {
            return Err(Error::ManifestStructure);
        }
        validate_catalog_leaf_attributes(bytes, attributes, attribute_count, pool)?;
        CatalogElement::UsesFeature
    } else if pool.equals(name, b"application")? {
        if parent != CatalogElement::Manifest || state.application_seen {
            return Err(Error::ManifestApplicationDuplicate);
        }
        parse_catalog_application_attributes(bytes, attributes, attribute_count, pool, state)?;
        state.application_seen = true;
        CatalogElement::Application
    } else if pool.equals(name, b"activity")? {
        begin_catalog_component(
            bytes,
            attributes,
            attribute_count,
            pool,
            state,
            parent,
            ManifestComponentKind::Activity,
        )?;
        CatalogElement::Activity
    } else if pool.equals(name, b"activity-alias")? {
        begin_catalog_component(
            bytes,
            attributes,
            attribute_count,
            pool,
            state,
            parent,
            ManifestComponentKind::ActivityAlias,
        )?;
        CatalogElement::ActivityAlias
    } else if pool.equals(name, b"service")? {
        begin_catalog_component(
            bytes,
            attributes,
            attribute_count,
            pool,
            state,
            parent,
            ManifestComponentKind::Service,
        )?;
        CatalogElement::Service
    } else if pool.equals(name, b"receiver")? {
        begin_catalog_component(
            bytes,
            attributes,
            attribute_count,
            pool,
            state,
            parent,
            ManifestComponentKind::Receiver,
        )?;
        CatalogElement::Receiver
    } else if pool.equals(name, b"provider")? {
        begin_catalog_component(
            bytes,
            attributes,
            attribute_count,
            pool,
            state,
            parent,
            ManifestComponentKind::Provider,
        )?;
        CatalogElement::Provider
    } else if pool.equals(name, b"intent-filter")? {
        if !catalog_component_element(parent) || state.current_component_index.is_none() {
            return Err(Error::ManifestStructure);
        }
        validate_catalog_leaf_attributes(bytes, attributes, attribute_count, pool)?;
        state.filter_main = false;
        state.filter_launcher = false;
        state.filter_action_count = 0;
        state.filter_category_count = 0;
        state.filter_data_count = 0;
        CatalogElement::IntentFilter
    } else if pool.equals(name, b"action")? {
        if parent != CatalogElement::IntentFilter {
            return Err(Error::ManifestStructure);
        }
        let value = parse_catalog_named_leaf(bytes, attributes, attribute_count, pool, b"name")?;
        state.filter_main |= value.equals(b"android.intent.action.MAIN");
        state.filter_action_count = state
            .filter_action_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        CatalogElement::Action
    } else if pool.equals(name, b"category")? {
        if parent != CatalogElement::IntentFilter {
            return Err(Error::ManifestStructure);
        }
        let value = parse_catalog_named_leaf(bytes, attributes, attribute_count, pool, b"name")?;
        state.filter_launcher |= value.equals(b"android.intent.category.LAUNCHER");
        state.filter_category_count = state
            .filter_category_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        CatalogElement::Category
    } else if pool.equals(name, b"data")? {
        if parent != CatalogElement::IntentFilter {
            return Err(Error::ManifestStructure);
        }
        validate_catalog_leaf_attributes(bytes, attributes, attribute_count, pool)?;
        state.filter_data_count = state
            .filter_data_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        CatalogElement::Data
    } else if pool.equals(name, b"meta-data")? {
        if parent != CatalogElement::Application && !catalog_component_element(parent) {
            return Err(Error::ManifestStructure);
        }
        validate_catalog_leaf_attributes(bytes, attributes, attribute_count, pool)?;
        CatalogElement::MetaData
    } else {
        return Err(Error::ManifestUnknownElement);
    };
    state.push(element)
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_end_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    pool: StringPool<'_>,
    state: &mut CatalogParseState,
) -> Result<(), Error> {
    if chunk.header_size != 16 || chunk.size != 24 {
        return Err(Error::ManifestStructure);
    }
    validate_node_header(bytes, offset)?;
    if read_u32(bytes, offset + 16, Error::ManifestStructure)? != NO_INDEX {
        return Err(Error::ManifestStructure);
    }
    let name = read_u32(bytes, offset + 20, Error::ManifestStructure)?;
    let element = catalog_element_from_name(pool, name)?;
    state.pop(element)?;
    match element {
        CatalogElement::IntentFilter => finish_catalog_intent_filter(state)?,
        CatalogElement::Activity
        | CatalogElement::ActivityAlias
        | CatalogElement::Service
        | CatalogElement::Receiver
        | CatalogElement::Provider => {
            if state.current_component_index.take().is_none() {
                return Err(Error::ManifestCatalogComponentInvalid);
            }
        }
        CatalogElement::Manifest => {
            if !state.application_seen {
                return Err(Error::ManifestApplicationMissing);
            }
            state.root_closed = true;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn catalog_element_from_name(pool: StringPool<'_>, name: u32) -> Result<CatalogElement, Error> {
    for (expected, element) in [
        (&b"manifest"[..], CatalogElement::Manifest),
        (&b"uses-sdk"[..], CatalogElement::UsesSdk),
        (&b"uses-permission"[..], CatalogElement::UsesPermission),
        (&b"uses-feature"[..], CatalogElement::UsesFeature),
        (&b"application"[..], CatalogElement::Application),
        (&b"activity"[..], CatalogElement::Activity),
        (&b"activity-alias"[..], CatalogElement::ActivityAlias),
        (&b"service"[..], CatalogElement::Service),
        (&b"receiver"[..], CatalogElement::Receiver),
        (&b"provider"[..], CatalogElement::Provider),
        (&b"intent-filter"[..], CatalogElement::IntentFilter),
        (&b"action"[..], CatalogElement::Action),
        (&b"category"[..], CatalogElement::Category),
        (&b"data"[..], CatalogElement::Data),
        (&b"meta-data"[..], CatalogElement::MetaData),
    ] {
        if pool.equals(name, expected)? {
            return Ok(element);
        }
    }
    Err(Error::ManifestUnknownElement)
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn catalog_component_element(element: CatalogElement) -> bool {
    matches!(
        element,
        CatalogElement::Activity
            | CatalogElement::ActivityAlias
            | CatalogElement::Service
            | CatalogElement::Receiver
            | CatalogElement::Provider
    )
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_manifest_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut CatalogParseState,
) -> Result<(), Error> {
    let mut package = None;
    let mut version_code = None;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if attribute.namespace == NO_INDEX && pool.equals(attribute.name, b"package")? {
            package = Some(attribute_string(attribute, pool)?);
        } else if is_android_attribute(attribute, pool, b"versionCode")? {
            let value = attribute_int(attribute)?;
            if value == 0 {
                return Err(Error::ManifestVersionCodeInvalid);
            }
            if version_code.replace(value).is_some() {
                return Err(Error::ManifestVersionCodeDuplicate);
            }
        }
        index += 1;
    }
    if state.package.is_some() {
        return Err(Error::ManifestPackageDuplicate);
    }
    state.package = Some(validate_package(
        package.ok_or(Error::ManifestPackageMissing)?,
    )?);
    state.version_code = Some(version_code.ok_or(Error::ManifestVersionCodeMissing)?);
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_application_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut CatalogParseState,
) -> Result<(), Error> {
    let mut label = None;
    let mut icon = None;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if is_android_attribute(attribute, pool, b"label")? {
            label = Some(if attribute.value_type == TYPE_REFERENCE {
                ApplicationLabelValue::Resource(attribute_reference(attribute)?)
            } else {
                ApplicationLabelValue::Inline(
                    attribute_string(attribute, pool)?
                        .bounded(Error::ManifestAttribute, Error::ManifestAttribute)?,
                )
            });
        } else if is_android_attribute(attribute, pool, b"icon")?
            && icon.replace(attribute_reference(attribute)?).is_some()
        {
            return Err(Error::ManifestAttribute);
        }
        index += 1;
    }
    state.application_label = label;
    state.application_icon_resource_id = icon;
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn begin_catalog_component(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut CatalogParseState,
    parent: CatalogElement,
    kind: ManifestComponentKind,
) -> Result<(), Error> {
    if parent != CatalogElement::Application || state.current_component_index.is_some() {
        return Err(Error::ManifestStructure);
    }
    let package = state.package.ok_or(Error::ManifestPackageMissing)?;
    let component = parse_catalog_component_attributes(bytes, offset, count, pool, package, kind)?;
    state.append_component(component)
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_component_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    package: PackageName,
    kind: ManifestComponentKind,
) -> Result<ManifestComponent, Error> {
    let mut name = None;
    let mut alias_target = None;
    let mut exported = None;
    let mut enabled = true;
    let mut authority_declared = false;
    let mut permission_declared = false;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if is_android_attribute(attribute, pool, b"name")? {
            name = Some(attribute_string(attribute, pool)?);
        } else if is_android_attribute(attribute, pool, b"targetActivity")? {
            alias_target = Some(attribute_string(attribute, pool)?);
        } else if is_android_attribute(attribute, pool, b"exported")? {
            exported = Some(attribute_bool(attribute)?);
        } else if is_android_attribute(attribute, pool, b"enabled")? {
            enabled = attribute_bool(attribute)?;
        } else if is_android_attribute(attribute, pool, b"authorities")? {
            let value = attribute_string(attribute, pool)?;
            if value.length == 0 {
                return Err(Error::ManifestCatalogComponentInvalid);
            }
            authority_declared = true;
        } else if is_android_attribute(attribute, pool, b"permission")? {
            validate_permission_name(attribute_string(attribute, pool)?.bounded(
                Error::ManifestCatalogPermissionInvalid,
                Error::ManifestCatalogPermissionInvalid,
            )?)?;
            permission_declared = true;
        }
        index += 1;
    }
    let descriptor =
        normalize_activity(package, name.ok_or(Error::ManifestCatalogComponentInvalid)?)?;
    let alias_target = if kind == ManifestComponentKind::ActivityAlias {
        normalize_activity(
            package,
            alias_target.ok_or(Error::ManifestCatalogComponentInvalid)?,
        )?
    } else {
        if alias_target.is_some() {
            return Err(Error::ManifestCatalogComponentInvalid);
        }
        ComponentDescriptor::empty()
    };
    if kind == ManifestComponentKind::Provider && !authority_declared {
        return Err(Error::ManifestCatalogComponentInvalid);
    }
    Ok(ManifestComponent {
        kind,
        descriptor,
        alias_target,
        exported: exported.unwrap_or(false),
        exported_explicit: exported.is_some(),
        enabled,
        intent_filter_count: 0,
        action_count: 0,
        category_count: 0,
        data_count: 0,
        launcher: false,
        authority_declared,
        permission_declared,
    })
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn finish_catalog_intent_filter(state: &mut CatalogParseState) -> Result<(), Error> {
    if state.filter_action_count == 0 {
        return Err(Error::ManifestCatalogIntentFilterInvalid);
    }
    let filter_action_count = state.filter_action_count;
    let filter_category_count = state.filter_category_count;
    let filter_data_count = state.filter_data_count;
    let index = state
        .current_component_index
        .ok_or(Error::ManifestCatalogComponentInvalid)?;
    let launcher = state.filter_main && state.filter_launcher;
    {
        let component = state.current_component_mut()?;
        component.intent_filter_count = component
            .intent_filter_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        component.action_count = component
            .action_count
            .checked_add(filter_action_count)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        component.category_count = component
            .category_count
            .checked_add(filter_category_count)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        component.data_count = component
            .data_count
            .checked_add(filter_data_count)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        if launcher {
            if !matches!(
                component.kind,
                ManifestComponentKind::Activity | ManifestComponentKind::ActivityAlias
            ) {
                return Err(Error::ManifestLauncherInvalid);
            }
            if !component.exported {
                return Err(Error::ManifestActivityNotExported);
            }
            component.launcher = true;
        }
    }
    state.intent_filter_count = state
        .intent_filter_count
        .checked_add(1)
        .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
    state.action_count = state
        .action_count
        .checked_add(filter_action_count)
        .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
    state.category_count = state
        .category_count
        .checked_add(filter_category_count)
        .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
    state.data_count = state
        .data_count
        .checked_add(filter_data_count)
        .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
    if launcher {
        state.launcher_count = state
            .launcher_count
            .checked_add(1)
            .ok_or(Error::ManifestCatalogIntentFilterInvalid)?;
        if state.primary_launcher_index.is_none() {
            state.primary_launcher_index = Some(index);
        }
    }
    state.filter_main = false;
    state.filter_launcher = false;
    state.filter_action_count = 0;
    state.filter_category_count = 0;
    state.filter_data_count = 0;
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn validate_catalog_leaf_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
) -> Result<(), Error> {
    let mut index = 0u16;
    while index < count {
        read_attribute(bytes, offset, index, pool)?;
        index += 1;
    }
    Ok(())
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn parse_catalog_named_leaf<'a>(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'a>,
    expected_name: &[u8],
) -> Result<XmlString<'a>, Error> {
    if count != 1 {
        return Err(Error::ManifestCatalogIntentFilterInvalid);
    }
    let attribute = read_attribute(bytes, offset, 0, pool)?;
    if !is_android_attribute(attribute, pool, expected_name)? {
        return Err(Error::ManifestCatalogIntentFilterInvalid);
    }
    attribute_string(attribute, pool)
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn validate_permission_name(value: PermissionName) -> Result<PermissionName, Error> {
    let bytes = value.as_bytes();
    let mut segment_start = true;
    let mut dots = 0usize;
    for &byte in bytes {
        if byte == b'.' {
            if segment_start {
                return Err(Error::ManifestCatalogPermissionInvalid);
            }
            segment_start = true;
            dots += 1;
        } else if segment_start {
            if !(byte.is_ascii_alphabetic() || byte == b'_') {
                return Err(Error::ManifestCatalogPermissionInvalid);
            }
            segment_start = false;
        } else if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return Err(Error::ManifestCatalogPermissionInvalid);
        }
    }
    if segment_start || dots == 0 {
        return Err(Error::ManifestCatalogPermissionInvalid);
    }
    Ok(value)
}

fn validate_resource_map(
    bytes: &[u8],
    offset: usize,
    pool: StringPool<'_>,
) -> Result<usize, Error> {
    let chunk = chunk_header(bytes, offset, Error::ManifestResourceMap)?;
    if chunk.kind != RES_XML_RESOURCE_MAP_TYPE
        || chunk.header_size != 8
        || chunk.size < 12
        || !chunk.size.is_multiple_of(4)
    {
        return Err(Error::ManifestResourceMap);
    }
    let count = (chunk.size - 8) / 4;
    let count_usize = usize_from(count, Error::ManifestResourceMap)?;
    let carries_icon = count_usize == RESOURCE_ATTRIBUTES.len() + 1;
    if (count_usize != RESOURCE_ATTRIBUTES.len() && !carries_icon) || count > pool.count {
        return Err(Error::ManifestResourceMap);
    }
    let mut index = 0u32;
    while index < count {
        let value = read_u32(
            bytes,
            offset
                .checked_add(8)
                .and_then(|base| {
                    base.checked_add(usize_from(index, Error::ManifestResourceMap).ok()? * 4)
                })
                .ok_or(Error::ManifestResourceMap)?,
            Error::ManifestResourceMap,
        )?;
        let index_usize = usize_from(index, Error::ManifestResourceMap)?;
        let expected = if carries_icon && index_usize == 1 {
            (&b"icon"[..], ANDROID_ICON_ATTRIBUTE_ID)
        } else {
            let legacy_index = if carries_icon && index_usize > 1 {
                index_usize - 1
            } else {
                index_usize
            };
            *RESOURCE_ATTRIBUTES
                .get(legacy_index)
                .ok_or(Error::ManifestResourceMap)?
        };
        if value != expected.1 || !pool.equals(index, expected.0)? {
            return Err(Error::ManifestResourceMap);
        }
        let mut earlier = 0u32;
        while earlier < index {
            let earlier_value = read_u32(
                bytes,
                offset + 8 + usize_from(earlier, Error::ManifestResourceMap)? * 4,
                Error::ManifestResourceMap,
            )?;
            if earlier_value == value {
                return Err(Error::ManifestResourceMap);
            }
            earlier += 1;
        }
        index += 1;
    }
    Ok(chunk.end)
}

fn parse_namespace(
    bytes: &[u8],
    offset: usize,
    pool: StringPool<'_>,
    start: bool,
) -> Result<usize, Error> {
    let chunk = chunk_header(bytes, offset, Error::ManifestStructure)?;
    let expected = if start {
        RES_XML_START_NAMESPACE_TYPE
    } else {
        RES_XML_END_NAMESPACE_TYPE
    };
    if chunk.kind != expected || chunk.header_size != 16 || chunk.size != 24 {
        return Err(Error::ManifestStructure);
    }
    validate_node_header(bytes, offset)?;
    let prefix = read_u32(bytes, offset + 16, Error::ManifestStructure)?;
    let uri = read_u32(bytes, offset + 20, Error::ManifestStructure)?;
    if !pool.equals(prefix, b"android")? || !pool.equals(uri, ANDROID_URI)? {
        return Err(Error::ManifestStructure);
    }
    Ok(chunk.end)
}

fn parse_start_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    pool: StringPool<'_>,
    state: &mut ParseState,
) -> Result<(), Error> {
    if chunk.header_size != 16 || chunk.size < 36 {
        return Err(Error::ManifestStructure);
    }
    validate_node_header(bytes, offset)?;
    let namespace = read_u32(bytes, offset + 16, Error::ManifestStructure)?;
    let name = read_u32(bytes, offset + 20, Error::ManifestStructure)?;
    let attributes_start = read_u16(bytes, offset + 24, Error::ManifestStructure)?;
    let attribute_size = read_u16(bytes, offset + 26, Error::ManifestStructure)?;
    let attribute_count = read_u16(bytes, offset + 28, Error::ManifestStructure)?;
    let id_index = read_u16(bytes, offset + 30, Error::ManifestStructure)?;
    let class_index = read_u16(bytes, offset + 32, Error::ManifestStructure)?;
    let style_index = read_u16(bytes, offset + 34, Error::ManifestStructure)?;
    if namespace != NO_INDEX
        || attributes_start != 20
        || attribute_size != 20
        || attribute_count > MAX_ATTRIBUTES
        || id_index != 0
        || class_index != 0
        || style_index != 0
        || chunk.size != 36 + u32::from(attribute_count) * 20
    {
        return Err(Error::ManifestStructure);
    }
    let attributes = offset + 36;
    validate_attribute_uniqueness(bytes, attributes, attribute_count, pool)?;

    let element = if pool.equals(name, b"manifest")? {
        if state.parent() != Element::Empty || state.root_closed {
            return Err(Error::ManifestStructure);
        }
        parse_manifest_attributes(bytes, attributes, attribute_count, pool, state)?;
        Element::Manifest
    } else if pool.equals(name, b"uses-sdk")? {
        if state.parent() != Element::Manifest || state.uses_sdk_seen || state.application_seen {
            return Err(Error::ManifestStructure);
        }
        parse_uses_sdk_attributes(bytes, attributes, attribute_count, pool)?;
        state.uses_sdk_seen = true;
        Element::UsesSdk
    } else if pool.equals(name, b"application")? {
        if state.parent() != Element::Manifest || state.application_seen {
            return Err(Error::ManifestApplicationDuplicate);
        }
        parse_application_attributes(bytes, attributes, attribute_count, pool, state)?;
        state.application_seen = true;
        Element::Application
    } else if pool.equals(name, b"activity")? {
        if state.parent() != Element::Application || state.current_activity.is_some() {
            return Err(Error::ManifestStructure);
        }
        parse_activity_attributes(bytes, attributes, attribute_count, pool, state)?;
        state.activity_count = state
            .activity_count
            .checked_add(1)
            .ok_or(Error::ManifestActivityDuplicate)?;
        state.current_activity_launcher = false;
        Element::Activity
    } else if pool.equals(name, b"intent-filter")? {
        if state.parent() != Element::Activity {
            return Err(Error::ManifestStructure);
        }
        if attribute_count != 0 || state.current_activity_launcher {
            return Err(Error::ManifestLauncherInvalid);
        }
        state.filter_main = false;
        state.filter_launcher = false;
        Element::IntentFilter
    } else if pool.equals(name, b"action")? {
        if state.parent() != Element::IntentFilter {
            return Err(Error::ManifestStructure);
        }
        parse_filter_name(
            bytes,
            attributes,
            attribute_count,
            pool,
            b"android.intent.action.MAIN",
            &mut state.filter_main,
        )?;
        Element::Action
    } else if pool.equals(name, b"category")? {
        if state.parent() != Element::IntentFilter {
            return Err(Error::ManifestStructure);
        }
        parse_filter_name(
            bytes,
            attributes,
            attribute_count,
            pool,
            b"android.intent.category.LAUNCHER",
            &mut state.filter_launcher,
        )?;
        Element::Category
    } else {
        return Err(Error::ManifestUnknownElement);
    };
    state.push(element)
}

fn parse_end_element(
    bytes: &[u8],
    offset: usize,
    chunk: Chunk,
    pool: StringPool<'_>,
    state: &mut ParseState,
) -> Result<(), Error> {
    if chunk.header_size != 16 || chunk.size != 24 {
        return Err(Error::ManifestStructure);
    }
    validate_node_header(bytes, offset)?;
    if read_u32(bytes, offset + 16, Error::ManifestStructure)? != NO_INDEX {
        return Err(Error::ManifestStructure);
    }
    let name = read_u32(bytes, offset + 20, Error::ManifestStructure)?;
    let element = if pool.equals(name, b"manifest")? {
        Element::Manifest
    } else if pool.equals(name, b"uses-sdk")? {
        Element::UsesSdk
    } else if pool.equals(name, b"application")? {
        Element::Application
    } else if pool.equals(name, b"activity")? {
        Element::Activity
    } else if pool.equals(name, b"intent-filter")? {
        Element::IntentFilter
    } else if pool.equals(name, b"action")? {
        Element::Action
    } else if pool.equals(name, b"category")? {
        Element::Category
    } else {
        return Err(Error::ManifestUnknownElement);
    };
    state.pop(element)?;
    match element {
        Element::IntentFilter => {
            if !state.filter_main || !state.filter_launcher {
                return Err(Error::ManifestLauncherInvalid);
            }
            state.current_activity_launcher = true;
        }
        Element::Activity => {
            let activity = state
                .current_activity
                .take()
                .ok_or(Error::ManifestActivityInvalid)?;
            let exported = state
                .current_exported
                .take()
                .ok_or(Error::ManifestActivityInvalid)?;
            if state.current_activity_launcher {
                if !exported {
                    return Err(Error::ManifestActivityNotExported);
                }
                if state.launcher.replace(activity).is_some() {
                    return Err(Error::ManifestActivityDuplicate);
                }
            }
            state.current_activity_launcher = false;
        }
        Element::Manifest => {
            if !state.application_seen {
                return Err(Error::ManifestApplicationMissing);
            }
            state.root_closed = true;
        }
        _ => {}
    }
    Ok(())
}

fn parse_manifest_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut ParseState,
) -> Result<(), Error> {
    let mut package = None;
    let mut version_code = None;
    let mut known = 0u16;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if attribute.namespace == NO_INDEX && pool.equals(attribute.name, b"package")? {
            package = Some(attribute_string(attribute, pool)?);
        } else if is_android_attribute(attribute, pool, b"versionCode")? {
            let value = attribute_int(attribute)?;
            if value == 0 {
                return Err(Error::ManifestVersionCodeInvalid);
            }
            if version_code.replace(value).is_some() {
                return Err(Error::ManifestVersionCodeDuplicate);
            }
        } else if (attribute.namespace == NO_INDEX
            && (pool.equals(attribute.name, b"platformBuildVersionCode")?
                || pool.equals(attribute.name, b"platformBuildVersionName")?))
            || is_android_attribute(attribute, pool, b"compileSdkVersion")?
        {
            attribute_int(attribute)?;
        } else if is_android_attribute(attribute, pool, b"versionName")?
            || is_android_attribute(attribute, pool, b"compileSdkVersionCodename")?
        {
            attribute_string(attribute, pool)?;
        } else {
            return Err(Error::ManifestAttribute);
        }
        known += 1;
        index += 1;
    }
    if known != count {
        return Err(Error::ManifestAttribute);
    }
    let package = package.ok_or(Error::ManifestPackageMissing)?;
    if state.package.is_some() {
        return Err(Error::ManifestPackageDuplicate);
    }
    state.package = Some(validate_package(package)?);
    state.version_code = Some(version_code.ok_or(Error::ManifestVersionCodeMissing)?);
    Ok(())
}

fn parse_uses_sdk_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
) -> Result<(), Error> {
    if count == 0 {
        return Err(Error::ManifestAttribute);
    }
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if is_android_attribute(attribute, pool, b"minSdkVersion")?
            || is_android_attribute(attribute, pool, b"targetSdkVersion")?
        {
            attribute_int(attribute)?;
        } else {
            return Err(Error::ManifestAttribute);
        }
        index += 1;
    }
    Ok(())
}

fn parse_application_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut ParseState,
) -> Result<(), Error> {
    let mut label = None;
    let mut has_code = None;
    let mut icon = None;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if is_android_attribute(attribute, pool, b"label")? {
            label = Some(if attribute.value_type == TYPE_REFERENCE {
                ApplicationLabelValue::Resource(attribute_reference(attribute)?)
            } else {
                ApplicationLabelValue::Inline(
                    attribute_string(attribute, pool)?
                        .bounded(Error::ManifestAttribute, Error::ManifestAttribute)?,
                )
            });
        } else if is_android_attribute(attribute, pool, b"icon")?
            && icon.replace(attribute_reference(attribute)?).is_some()
        {
            return Err(Error::ManifestAttribute);
        } else if is_android_attribute(attribute, pool, b"hasCode")? {
            has_code = Some(attribute_bool(attribute)?);
        } else if is_android_attribute(attribute, pool, b"allowBackup")?
            || is_android_attribute(attribute, pool, b"supportsRtl")?
        {
            attribute_bool(attribute)?;
        } else {
            return Err(Error::ManifestAttribute);
        }
        index += 1;
    }
    if has_code != Some(true) {
        return Err(Error::ManifestAttribute);
    }
    state.application_label = Some(label.ok_or(Error::ManifestAttribute)?);
    state.application_icon_resource_id = icon;
    Ok(())
}

fn parse_activity_attributes(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    state: &mut ParseState,
) -> Result<(), Error> {
    let mut name = None;
    let mut exported = None;
    let mut index = 0u16;
    while index < count {
        let attribute = read_attribute(bytes, offset, index, pool)?;
        if is_android_attribute(attribute, pool, b"name")? {
            name = Some(attribute_string(attribute, pool)?);
        } else if is_android_attribute(attribute, pool, b"exported")? {
            exported = Some(attribute_bool(attribute)?);
        } else {
            return Err(Error::ManifestAttribute);
        }
        index += 1;
    }
    let package = state.package.ok_or(Error::ManifestPackageMissing)?;
    state.current_activity = Some(normalize_activity(
        package,
        name.ok_or(Error::ManifestActivityInvalid)?,
    )?);
    state.current_exported = Some(exported.ok_or(Error::ManifestActivityInvalid)?);
    Ok(())
}

fn parse_filter_name(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
    expected: &[u8],
    seen: &mut bool,
) -> Result<(), Error> {
    if count != 1 || *seen {
        return Err(Error::ManifestLauncherInvalid);
    }
    let attribute = read_attribute(bytes, offset, 0, pool)?;
    if !is_android_attribute(attribute, pool, b"name")?
        || !attribute_string(attribute, pool)?.equals(expected)
    {
        return Err(Error::ManifestLauncherInvalid);
    }
    *seen = true;
    Ok(())
}

fn validate_attribute_uniqueness(
    bytes: &[u8],
    offset: usize,
    count: u16,
    pool: StringPool<'_>,
) -> Result<(), Error> {
    let mut index = 0u16;
    while index < count {
        let current = read_attribute(bytes, offset, index, pool)?;
        let mut earlier = 0u16;
        while earlier < index {
            let previous = read_attribute(bytes, offset, earlier, pool)?;
            if current.namespace == previous.namespace && current.name == previous.name {
                return if current.namespace == NO_INDEX && pool.equals(current.name, b"package")? {
                    Err(Error::ManifestPackageDuplicate)
                } else if is_android_attribute(current, pool, b"versionCode")? {
                    Err(Error::ManifestVersionCodeDuplicate)
                } else {
                    Err(Error::ManifestAttribute)
                };
            }
            earlier += 1;
        }
        index += 1;
    }
    Ok(())
}

fn read_attribute(
    bytes: &[u8],
    offset: usize,
    index: u16,
    pool: StringPool<'_>,
) -> Result<Attribute, Error> {
    let base = offset
        .checked_add(
            usize::from(index)
                .checked_mul(20)
                .ok_or(Error::ManifestAttribute)?,
        )
        .ok_or(Error::ManifestAttribute)?;
    let attribute = Attribute {
        namespace: read_u32(bytes, base, Error::ManifestAttribute)?,
        name: read_u32(bytes, base + 4, Error::ManifestAttribute)?,
        raw_value: read_u32(bytes, base + 8, Error::ManifestAttribute)?,
        value_type: *bytes.get(base + 15).ok_or(Error::ManifestAttribute)?,
        data: read_u32(bytes, base + 16, Error::ManifestAttribute)?,
    };
    if read_u16(bytes, base + 12, Error::ManifestAttribute)? != 8
        || *bytes.get(base + 14).ok_or(Error::ManifestAttribute)? != 0
    {
        return Err(Error::ManifestAttribute);
    }
    pool.get(attribute.name)?;
    if attribute.namespace != NO_INDEX {
        pool.get(attribute.namespace)?;
    }
    match attribute.value_type {
        TYPE_REFERENCE => {
            if attribute.raw_value != NO_INDEX || attribute.data == 0 {
                return Err(Error::ManifestAttribute);
            }
        }
        TYPE_STRING => {
            if attribute.raw_value == NO_INDEX || attribute.raw_value != attribute.data {
                return Err(Error::ManifestAttribute);
            }
            pool.get(attribute.data)?;
        }
        TYPE_INT_DEC | TYPE_INT_BOOLEAN => {
            if attribute.raw_value != NO_INDEX {
                return Err(Error::ManifestAttribute);
            }
        }
        #[cfg(feature = "androidbox-manifest-catalog3")]
        TYPE_INT_HEX => {
            if attribute.raw_value != NO_INDEX {
                return Err(Error::ManifestAttribute);
            }
        }
        _ => return Err(Error::ManifestAttribute),
    }
    Ok(attribute)
}

fn attribute_reference(attribute: Attribute) -> Result<u32, Error> {
    if attribute.value_type != TYPE_REFERENCE
        || attribute.raw_value != NO_INDEX
        || attribute.data == 0
    {
        return Err(Error::ManifestAttribute);
    }
    Ok(attribute.data)
}

fn is_android_attribute(
    attribute: Attribute,
    pool: StringPool<'_>,
    name: &[u8],
) -> Result<bool, Error> {
    Ok(attribute.namespace != NO_INDEX
        && pool.equals(attribute.namespace, ANDROID_URI)?
        && pool.equals(attribute.name, name)?)
}

fn attribute_string<'a>(
    attribute: Attribute,
    pool: StringPool<'a>,
) -> Result<XmlString<'a>, Error> {
    if attribute.value_type != TYPE_STRING
        || attribute.raw_value == NO_INDEX
        || attribute.raw_value != attribute.data
    {
        return Err(Error::ManifestAttribute);
    }
    pool.get(attribute.data)
}

fn attribute_int(attribute: Attribute) -> Result<u32, Error> {
    if attribute.value_type != TYPE_INT_DEC || attribute.raw_value != NO_INDEX {
        return Err(Error::ManifestAttribute);
    }
    Ok(attribute.data)
}

fn attribute_bool(attribute: Attribute) -> Result<bool, Error> {
    if attribute.value_type != TYPE_INT_BOOLEAN
        || attribute.raw_value != NO_INDEX
        || !matches!(attribute.data, 0 | u32::MAX)
    {
        return Err(Error::ManifestAttribute);
    }
    Ok(attribute.data != 0)
}

fn validate_package(value: XmlString<'_>) -> Result<PackageName, Error> {
    let package = value.bounded(Error::ManifestPackageInvalid, Error::ManifestPackageInvalid)?;
    let bytes = package.as_bytes();
    let mut segment_start = true;
    let mut dots = 0usize;
    for &byte in bytes {
        if byte == b'.' {
            if segment_start {
                return Err(Error::ManifestPackageInvalid);
            }
            segment_start = true;
            dots += 1;
        } else if segment_start {
            if !(byte.is_ascii_alphabetic() || byte == b'_') {
                return Err(Error::ManifestPackageInvalid);
            }
            segment_start = false;
        } else if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return Err(Error::ManifestPackageInvalid);
        }
    }
    if segment_start || dots == 0 {
        return Err(Error::ManifestPackageInvalid);
    }
    Ok(package)
}

fn normalize_activity(
    package: PackageName,
    name: XmlString<'_>,
) -> Result<ActivityDescriptor, Error> {
    if name.length == 0 {
        return Err(Error::ManifestActivityInvalid);
    }
    let mut result = ActivityDescriptor::empty();
    let mut output = 0usize;
    push_descriptor_byte(&mut result, &mut output, b'L')?;

    let relative = name.ascii_at(0) == Some(b'.') || !xml_contains(name, b'.');
    if relative {
        for &byte in package.as_bytes() {
            push_class_byte(&mut result, &mut output, byte)?;
        }
        push_descriptor_byte(&mut result, &mut output, b'/')?;
    }
    let mut index = usize::from(name.ascii_at(0) == Some(b'.'));
    let mut segment_start = true;
    while index < name.length {
        let byte = name.ascii_at(index).ok_or(Error::ManifestActivityInvalid)?;
        if byte == b'.' {
            if segment_start {
                return Err(Error::ManifestActivityInvalid);
            }
            push_descriptor_byte(&mut result, &mut output, b'/')?;
            segment_start = true;
        } else {
            if segment_start {
                if !(byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$') {
                    return Err(Error::ManifestActivityInvalid);
                }
                segment_start = false;
            } else if !(byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$') {
                return Err(Error::ManifestActivityInvalid);
            }
            push_descriptor_byte(&mut result, &mut output, byte)?;
        }
        index += 1;
    }
    if segment_start {
        return Err(Error::ManifestActivityInvalid);
    }
    push_descriptor_byte(&mut result, &mut output, b';')?;
    result.len = u16::try_from(output).map_err(|_| Error::ManifestActivityInvalid)?;
    Ok(result)
}

fn push_class_byte(
    output: &mut ActivityDescriptor,
    index: &mut usize,
    byte: u8,
) -> Result<(), Error> {
    push_descriptor_byte(output, index, if byte == b'.' { b'/' } else { byte })
}

fn push_descriptor_byte(
    output: &mut ActivityDescriptor,
    index: &mut usize,
    byte: u8,
) -> Result<(), Error> {
    let slot = output
        .bytes
        .get_mut(*index)
        .ok_or(Error::ManifestActivityInvalid)?;
    *slot = byte;
    *index = index.checked_add(1).ok_or(Error::ManifestActivityInvalid)?;
    Ok(())
}

fn xml_contains(value: XmlString<'_>, expected: u8) -> bool {
    let mut index = 0usize;
    while index < value.length {
        if value.ascii_at(index) == Some(expected) {
            return true;
        }
        index += 1;
    }
    false
}

fn parse_pool_string<'a>(
    bytes: &'a [u8],
    offset: usize,
    end: usize,
    encoding: Encoding,
) -> Result<(XmlString<'a>, usize), Error> {
    match encoding {
        Encoding::Utf8 => {
            let (utf16_length, cursor) = read_length8(bytes, offset, end)?;
            let (byte_length, data_start) = read_length8(bytes, cursor, end)?;
            if utf16_length != byte_length || byte_length > MAX_XML_STRING_BYTES {
                return Err(Error::ManifestString);
            }
            let data_end = data_start
                .checked_add(byte_length)
                .ok_or(Error::ManifestString)?;
            if data_end >= end || *bytes.get(data_end).ok_or(Error::ManifestString)? != 0 {
                return Err(Error::ManifestString);
            }
            Ok((
                XmlString {
                    bytes: &bytes[data_start..data_end],
                    length: byte_length,
                    encoding,
                },
                data_end + 1,
            ))
        }
        Encoding::Utf16 => {
            let (length, data_start) = read_length16(bytes, offset, end)?;
            if length > MAX_XML_STRING_BYTES {
                return Err(Error::ManifestString);
            }
            let byte_length = length.checked_mul(2).ok_or(Error::ManifestString)?;
            let data_end = data_start
                .checked_add(byte_length)
                .ok_or(Error::ManifestString)?;
            if data_end + 2 > end || read_u16(bytes, data_end, Error::ManifestString)? != 0 {
                return Err(Error::ManifestString);
            }
            Ok((
                XmlString {
                    bytes: &bytes[data_start..data_end],
                    length,
                    encoding,
                },
                data_end + 2,
            ))
        }
    }
}

fn read_length8(bytes: &[u8], offset: usize, end: usize) -> Result<(usize, usize), Error> {
    let first = *bytes
        .get(offset)
        .filter(|_| offset < end)
        .ok_or(Error::ManifestString)?;
    if first & 0x80 == 0 {
        return Ok((usize::from(first), offset + 1));
    }
    let second_offset = offset.checked_add(1).ok_or(Error::ManifestString)?;
    let second = *bytes
        .get(second_offset)
        .filter(|_| second_offset < end)
        .ok_or(Error::ManifestString)?;
    let value = (usize::from(first & 0x7f) << 8) | usize::from(second);
    if value < 0x80 {
        return Err(Error::ManifestString);
    }
    Ok((value, second_offset + 1))
}

fn read_length16(bytes: &[u8], offset: usize, end: usize) -> Result<(usize, usize), Error> {
    if offset + 2 > end {
        return Err(Error::ManifestString);
    }
    let first = read_u16(bytes, offset, Error::ManifestString)?;
    if first & 0x8000 == 0 {
        return Ok((usize::from(first), offset + 2));
    }
    if offset + 4 > end {
        return Err(Error::ManifestString);
    }
    let second = read_u16(bytes, offset + 2, Error::ManifestString)?;
    let value = (usize::from(first & 0x7fff) << 16) | usize::from(second);
    if value < 0x8000 {
        return Err(Error::ManifestString);
    }
    Ok((value, offset + 4))
}

fn validate_node_header(bytes: &[u8], offset: usize) -> Result<(), Error> {
    let line = read_u32(bytes, offset + 8, Error::ManifestStructure)?;
    let comment = read_u32(bytes, offset + 12, Error::ManifestStructure)?;
    if line == 0 || comment != NO_INDEX {
        return Err(Error::ManifestStructure);
    }
    Ok(())
}

fn chunk_header(bytes: &[u8], offset: usize, error: Error) -> Result<Chunk, Error> {
    let kind = read_u16(bytes, offset, error)?;
    let header_size = read_u16(bytes, offset + 2, error)?;
    let size = read_u32(bytes, offset + 4, error)?;
    if header_size < 8 || u32::from(header_size) > size || !size.is_multiple_of(4) {
        return Err(error);
    }
    let end = offset.checked_add(usize_from(size, error)?).ok_or(error)?;
    if end > bytes.len() {
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
    let value = bytes
        .get(offset..offset.checked_add(2).ok_or(error)?)
        .ok_or(error)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize, error: Error) -> Result<u32, Error> {
    let value = bytes
        .get(offset..offset.checked_add(4).ok_or(error)?)
        .ok_or(error)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn usize_from(value: u32, error: Error) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| error)
}

#[cfg(all(test, feature = "androidbox-manifest-catalog3"))]
mod catalog_capacity_tests {
    use super::*;

    #[test]
    fn component_directory_capacity_is_exact_and_transactional() {
        let mut state = CatalogParseState::new();
        for _ in 0..MAX_MANIFEST_COMPONENTS {
            state
                .append_component(ManifestComponent::empty())
                .expect("component within bound");
            state.current_component_index = None;
        }
        let before = state.component_count;
        assert_eq!(
            state.append_component(ManifestComponent::empty()),
            Err(Error::ManifestCatalogTooManyComponents)
        );
        assert_eq!(state.component_count, before);
        assert!(state.current_component_index.is_none());
    }

    #[test]
    fn permission_directory_deduplicates_and_fails_closed_at_capacity() {
        const NAMES: [&[u8]; MAX_MANIFEST_PERMISSIONS + 1] = [
            b"org.bndroid.permission.P00",
            b"org.bndroid.permission.P01",
            b"org.bndroid.permission.P02",
            b"org.bndroid.permission.P03",
            b"org.bndroid.permission.P04",
            b"org.bndroid.permission.P05",
            b"org.bndroid.permission.P06",
            b"org.bndroid.permission.P07",
            b"org.bndroid.permission.P08",
            b"org.bndroid.permission.P09",
            b"org.bndroid.permission.P10",
            b"org.bndroid.permission.P11",
            b"org.bndroid.permission.P12",
            b"org.bndroid.permission.P13",
            b"org.bndroid.permission.P14",
            b"org.bndroid.permission.P15",
            b"org.bndroid.permission.P16",
        ];
        let mut state = CatalogParseState::new();
        for name in &NAMES[..MAX_MANIFEST_PERMISSIONS] {
            let permission = PermissionName::from_ascii(
                name,
                Error::ManifestCatalogPermissionInvalid,
                Error::ManifestCatalogPermissionInvalid,
            )
            .expect("bounded permission");
            state
                .append_permission(permission)
                .expect("permission within bound");
        }
        let duplicate = state.permissions[0];
        state
            .append_permission(duplicate)
            .expect("duplicate is idempotent");
        assert_eq!(
            usize::from(state.permission_count),
            MAX_MANIFEST_PERMISSIONS
        );

        let overflow = PermissionName::from_ascii(
            NAMES[MAX_MANIFEST_PERMISSIONS],
            Error::ManifestCatalogPermissionInvalid,
            Error::ManifestCatalogPermissionInvalid,
        )
        .expect("bounded overflow value");
        assert_eq!(
            state.append_permission(overflow),
            Err(Error::ManifestCatalogTooManyPermissions)
        );
        assert_eq!(
            usize::from(state.permission_count),
            MAX_MANIFEST_PERMISSIONS
        );
    }
}
