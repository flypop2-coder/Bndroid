use crate::digest::{adler32, sha1};
use crate::resources::{ActivityScene, ResourceString, ViewKind};
use crate::{
    ActivityConstructorInfo, ActivityMethodInfo, BOOT_METHOD, ENTRY_CLASS, Error, MAX_DEX_BYTES,
    MAX_INTERACTIVE_ACTIVITY_CALLBACKS, MethodInfo, ON_TAP_METHOD, ViewText,
};

const HEADER_SIZE: usize = 0x70;
const ENDIAN_CONSTANT: u32 = 0x1234_5678;
const NO_INDEX: u32 = u32::MAX;
const MAX_STRINGS: u32 = 512;
const MAX_TYPES: u32 = 256;
const MAX_PROTOS: u32 = 128;
const MAX_FIELDS: u32 = 256;
const MAX_METHODS: u32 = 512;
const MAX_CLASSES: u32 = 32;
const MAX_MAP_ITEMS: u32 = 64;
const MAX_STRING_BYTES: usize = 256;
const MAX_CLASS_MEMBERS: u32 = 128;
const MAX_REGISTERS: u16 = 8;
// Four listener registrations plus a four-way callback branch no longer fit
// in the original 32-unit InteractiveActivity-1 envelope. Keep the expansion
// fixed and small enough for allocation-free validation and execution.
const MAX_INSTRUCTION_UNITS: u32 = 96;
const MAX_EXECUTED_INSTRUCTIONS: u16 = 96;
const MAX_INTERACTIVE_CALLBACKS: usize = MAX_INTERACTIVE_ACTIVITY_CALLBACKS;
const ACTIVITY_CLASS: &[u8] = b"Landroid/app/Activity;";
const BUNDLE_CLASS: &[u8] = b"Landroid/os/Bundle;";
const CONTEXT_CLASS: &[u8] = b"Landroid/content/Context;";
const TEXT_VIEW_CLASS: &[u8] = b"Landroid/widget/TextView;";
const CHAR_SEQUENCE_CLASS: &[u8] = b"Ljava/lang/CharSequence;";
#[cfg(feature = "androidbox-string-builder13")]
const STRING_BUILDER_CLASS: &[u8] = b"Ljava/lang/StringBuilder;";
#[cfg(feature = "androidbox-string-builder13")]
const STRING_CLASS: &[u8] = b"Ljava/lang/String;";
const VIEW_CLASS: &[u8] = b"Landroid/view/View;";
const ON_CLICK_LISTENER_CLASS: &[u8] = b"Landroid/view/View$OnClickListener;";
const INT_CLASS: &[u8] = b"I";
const VOID_CLASS: &[u8] = b"V";
const ON_CREATE_METHOD: &[u8] = b"onCreate";
const ON_CLICK_METHOD: &[u8] = b"onClick";
const INIT_METHOD: &[u8] = b"<init>";
const FIND_VIEW_BY_ID_METHOD: &[u8] = b"findViewById";
const GET_ID_METHOD: &[u8] = b"getId";
const SET_ON_CLICK_LISTENER_METHOD: &[u8] = b"setOnClickListener";
const SET_TEXT_METHOD: &[u8] = b"setText";
const SET_CONTENT_VIEW_METHOD: &[u8] = b"setContentView";
#[cfg(feature = "androidbox-string-builder13")]
const APPEND_METHOD: &[u8] = b"append";
#[cfg(feature = "androidbox-string-builder13")]
const TO_STRING_METHOD: &[u8] = b"toString";

const TYPE_HEADER_ITEM: u16 = 0x0000;
const TYPE_STRING_ID_ITEM: u16 = 0x0001;
const TYPE_TYPE_ID_ITEM: u16 = 0x0002;
const TYPE_PROTO_ID_ITEM: u16 = 0x0003;
const TYPE_FIELD_ID_ITEM: u16 = 0x0004;
const TYPE_METHOD_ID_ITEM: u16 = 0x0005;
const TYPE_CLASS_DEF_ITEM: u16 = 0x0006;
const TYPE_CALL_SITE_ID_ITEM: u16 = 0x0007;
const TYPE_METHOD_HANDLE_ITEM: u16 = 0x0008;
const TYPE_MAP_LIST: u16 = 0x1000;
const TYPE_TYPE_LIST: u16 = 0x1001;
const TYPE_ANNOTATION_SET_REF_LIST: u16 = 0x1002;
const TYPE_ANNOTATION_SET_ITEM: u16 = 0x1003;
const TYPE_CLASS_DATA_ITEM: u16 = 0x2000;
const TYPE_CODE_ITEM: u16 = 0x2001;
const TYPE_STRING_DATA_ITEM: u16 = 0x2002;
const TYPE_DEBUG_INFO_ITEM: u16 = 0x2003;
const TYPE_ANNOTATION_ITEM: u16 = 0x2004;
const TYPE_ENCODED_ARRAY_ITEM: u16 = 0x2005;
const TYPE_ANNOTATIONS_DIRECTORY_ITEM: u16 = 0x2006;
const TYPE_HIDDENAPI_CLASS_DATA_ITEM: u16 = 0xf000;

const ACC_PUBLIC: u32 = 0x0001;
#[cfg(feature = "androidbox-dex-methods8")]
const ACC_PRIVATE: u32 = 0x0002;
const ACC_PROTECTED: u32 = 0x0004;
const ACC_STATIC: u32 = 0x0008;
const ACC_FINAL: u32 = 0x0010;
const ACC_NATIVE: u32 = 0x0100;
const ACC_ABSTRACT: u32 = 0x0400;
const ACC_CONSTRUCTOR: u32 = 0x0001_0000;

#[derive(Clone, Copy)]
struct Header {
    file_size: u32,
    map_off: u32,
    string_ids: Section,
    type_ids: Section,
    proto_ids: Section,
    field_ids: Section,
    method_ids: Section,
    class_defs: Section,
    data: Section,
}

#[derive(Clone, Copy)]
struct Section {
    size: u32,
    offset: u32,
}

#[derive(Clone, Copy)]
struct Method {
    info: MethodInfo,
}

#[derive(Clone, Copy)]
struct ActivityMethod {
    info: ActivityMethodInfo,
    class_type_index: u32,
}

#[derive(Clone, Copy)]
struct ActivityConstructor {
    info: ActivityConstructorInfo,
}

#[derive(Clone, Copy)]
struct ActivityClass {
    constructor: ActivityConstructor,
    on_create: ActivityMethod,
    on_click: Option<ActivityMethod>,
    activity_field: Option<ActivityFieldDeclaration>,
}

struct ActivityMethods {
    constructor: Option<ActivityConstructor>,
    on_create: Option<ActivityMethod>,
    on_click: Option<ActivityMethod>,
}

#[derive(Clone, Copy)]
struct Code {
    registers: u16,
    parameters: u16,
    outgoing: u16,
    units_offset: usize,
    units: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActivityFieldDeclaration {
    text_view_field_index: u32,
    int_field_index: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ActivityFieldValue {
    text_view_field_index: u32,
    text_view_index: u8,
    int_field_index: Option<u32>,
    int_value: i32,
}

fn retained_activity_field_identity_matches(
    initial: Option<ActivityFieldValue>,
    retained: Option<ActivityFieldValue>,
) -> bool {
    match (initial, retained) {
        (None, None) => true,
        (Some(initial), Some(retained)) => {
            initial.text_view_field_index == retained.text_view_field_index
                && initial.text_view_index == retained.text_view_index
                && initial.int_field_index == retained.int_field_index
                && initial.int_value == 0
                && retained.int_value >= 0
        }
        _ => false,
    }
}

#[cfg(feature = "androidbox-dex-instance9")]
#[derive(Clone, Copy)]
struct InstanceCallbackHelper {
    method: ActivityMethod,
    code: Code,
}

#[derive(Clone, Copy)]
pub(crate) struct Program<'a> {
    bytes: &'a [u8],
    header: Header,
    header_checksum: u32,
    // DEX-0's repository-specific integer probes are legacy diagnostics, not
    // Android components. Keep their independently validated result so an APK
    // can be admitted and launched solely through its manifest-selected
    // Activity while the historical boot/onTap API remains fail-closed.
    boot: Result<Method, Error>,
    on_tap: Result<Method, Error>,
    constructor: ActivityConstructor,
    activity: ActivityMethod,
    on_click: Option<ActivityMethod>,
    activity_field: Option<ActivityFieldDeclaration>,
    admitted_activity_content: ActivityContent,
}

#[derive(Clone, Copy)]
pub(crate) struct InteractiveLaunch {
    pub(crate) scene: ActivityScene,
    pub(crate) layout_resource_id: u32,
    // The first listener fields preserve the InteractiveActivity-1 caller
    // contract. MultiActionActivity-3 callers may use the bounded complete
    // listener arrays below.
    pub(crate) listener_view_index: u8,
    pub(crate) listener_view_id: u32,
    pub(crate) listener_view_ids: [u32; MAX_INTERACTIVE_CALLBACKS],
    pub(crate) listener_count: u8,
    pub(crate) activity_field: Option<ActivityFieldValue>,
    pub(crate) constructor_instruction_count: u16,
    pub(crate) on_create_instruction_count: u16,
}

#[derive(Clone, Copy)]
pub(crate) struct InteractiveMutation {
    pub(crate) target_view_index: u8,
    pub(crate) target_view_id: u32,
    pub(crate) text: InteractiveText,
    pub(crate) activity_field: Option<ActivityFieldValue>,
    pub(crate) method: ActivityMethodInfo,
    pub(crate) instruction_count: u16,
    #[cfg(feature = "androidbox-dex-methods8")]
    pub(crate) app_defined_call_count: u8,
    #[cfg(feature = "androidbox-dex-instance9")]
    pub(crate) app_defined_instance_call_count: u8,
    #[cfg(feature = "androidbox-activity-fields10")]
    pub(crate) activity_field_read_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    pub(crate) activity_field_write_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    pub(crate) activity_int_state_value: u8,
    #[cfg(feature = "androidbox-string-builder13")]
    pub(crate) dynamic_string_text: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct InteractiveText {
    resource_id: u32,
    #[cfg(feature = "androidbox-string-text12")]
    direct: Option<ResourceString>,
    #[cfg(feature = "androidbox-string-builder13")]
    dynamic: bool,
}

impl InteractiveText {
    const fn resource(resource_id: u32) -> Self {
        Self {
            resource_id,
            #[cfg(feature = "androidbox-string-text12")]
            direct: None,
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic: false,
        }
    }

    #[cfg(feature = "androidbox-string-text12")]
    const fn direct(text: ResourceString) -> Self {
        Self {
            resource_id: 0,
            direct: Some(text),
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic: false,
        }
    }

    #[cfg(feature = "androidbox-string-builder13")]
    const fn dynamic(text: ResourceString) -> Self {
        Self {
            resource_id: 0,
            direct: Some(text),
            dynamic: true,
        }
    }

    pub(crate) const fn resource_id(self) -> u32 {
        self.resource_id
    }

    #[cfg(feature = "androidbox-string-text12")]
    pub(crate) const fn direct_text(self) -> Option<ResourceString> {
        self.direct
    }

    #[cfg(feature = "androidbox-string-builder13")]
    pub(crate) const fn is_dynamic(self) -> bool {
        self.dynamic
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActivityContent {
    InlineText(ViewText),
    LayoutResource(u32),
}

impl<'a> Program<'a> {
    pub(crate) fn load(bytes: &'a [u8], activity_descriptor: &[u8]) -> Result<Self, Error> {
        let (header, header_checksum) = validate_header(bytes)?;
        validate_map(bytes, header)?;
        validate_tables(bytes, header)?;
        let (boot, on_tap) = match locate_entries(bytes, header) {
            Ok((boot, on_tap)) => (
                execute(bytes, boot, None).map(|_| boot),
                execute(bytes, on_tap, Some(0)).map(|_| on_tap),
            ),
            Err(error) => (Err(error), Err(error)),
        };
        let activity_class = locate_activity(bytes, header, activity_descriptor)?;
        // ActivityLifecycle-1 interprets one fresh constructor -> onCreate
        // sequence as a pure admission dry-run, so invalid construction,
        // framework calls, or lifecycle order cannot be deferred until launch.
        // Preserve only its pure-data content; a real launch executes its own
        // fresh constructor -> onCreate sequence below.
        let admitted_activity_content = if activity_class.on_click.is_some() {
            ActivityContent::LayoutResource(discover_interactive_layout_resource(
                bytes,
                header,
                activity_class.on_create,
            )?)
        } else {
            execute_activity_lifecycle(bytes, header, activity_class)?.0
        };
        Ok(Self {
            bytes,
            header,
            header_checksum,
            boot,
            on_tap,
            constructor: activity_class.constructor,
            activity: activity_class.on_create,
            on_click: activity_class.on_click,
            activity_field: activity_class.activity_field,
            admitted_activity_content,
        })
    }

    pub(crate) const fn header_checksum(&self) -> u32 {
        self.header_checksum
    }

    pub(crate) const fn file_size(&self) -> u32 {
        self.header.file_size
    }

    pub(crate) const fn boot_info(&self) -> Result<MethodInfo, Error> {
        match self.boot {
            Ok(method) => Ok(method.info),
            Err(error) => Err(error),
        }
    }

    pub(crate) const fn on_tap_info(&self) -> Result<MethodInfo, Error> {
        match self.on_tap {
            Ok(method) => Ok(method.info),
            Err(error) => Err(error),
        }
    }

    pub(crate) const fn activity_info(&self) -> ActivityMethodInfo {
        self.activity.info
    }

    pub(crate) const fn activity_constructor_info(&self) -> ActivityConstructorInfo {
        self.constructor.info
    }

    pub(crate) const fn admitted_activity_content(&self) -> ActivityContent {
        self.admitted_activity_content
    }

    pub(crate) const fn is_interactive(&self) -> bool {
        self.on_click.is_some()
    }

    pub(crate) const fn on_click_info(&self) -> Option<ActivityMethodInfo> {
        match self.on_click {
            Some(method) => Some(method.info),
            None => None,
        }
    }

    pub(crate) fn execute_boot(&self) -> Result<(i32, u16), Error> {
        execute(self.bytes, self.boot?, None)
    }

    pub(crate) fn execute_on_tap(&self, input: i32) -> Result<(i32, u16), Error> {
        execute(self.bytes, self.on_tap?, Some(input))
    }

    pub(crate) fn execute_activity(&self) -> Result<(ActivityContent, u16, u16), Error> {
        if self.is_interactive() {
            return Err(Error::DexFrameworkState);
        }
        execute_activity_lifecycle(
            self.bytes,
            self.header,
            ActivityClass {
                constructor: self.constructor,
                on_create: self.activity,
                on_click: None,
                activity_field: self.activity_field,
            },
        )
    }

    pub(crate) fn launch_interactive(
        &self,
        scene: ActivityScene,
    ) -> Result<InteractiveLaunch, Error> {
        let on_click = self.on_click.ok_or(Error::DexActivityMethodMissing)?;
        let constructor_instruction_count =
            execute_activity_constructor(self.bytes, self.header, self.constructor)?;
        let on_create = execute_interactive_on_create(
            self.bytes,
            self.header,
            self.activity,
            self.activity_field,
            &scene,
        )?;
        if self.admitted_activity_content
            != ActivityContent::LayoutResource(on_create.layout_resource_id)
        {
            return Err(Error::DexFrameworkState);
        }
        let listener_view_index = on_create.listeners.first()?;
        let listener_view_id = scene
            .node(listener_view_index)
            .ok_or(Error::DexFrameworkState)?
            .id;
        let mut listener_view_ids = [0; MAX_INTERACTIVE_CALLBACKS];
        for (slot, index) in on_create.listeners.as_slice().iter().copied().enumerate() {
            let listener = scene.node(index).ok_or(Error::DexFrameworkState)?;
            if listener.kind != ViewKind::Button || listener.id == 0 {
                return Err(Error::DexFrameworkState);
            }
            listener_view_ids[slot] = listener.id;
            // Dry-run every registered callback input. This proves that every
            // reachable bounded branch returns exactly one valid TextView
            // mutation before the retained session can be published.
            execute_on_click(
                self.bytes,
                self.header,
                on_click,
                &scene,
                on_create.listeners,
                on_create.activity_field,
                index,
            )?;
        }
        Ok(InteractiveLaunch {
            scene,
            layout_resource_id: on_create.layout_resource_id,
            listener_view_index,
            listener_view_id,
            listener_view_ids,
            listener_count: on_create.listeners.len,
            activity_field: on_create.activity_field,
            constructor_instruction_count,
            on_create_instruction_count: on_create.instruction_count,
        })
    }

    pub(crate) fn dispatch_click(
        &self,
        scene: &ActivityScene,
        listener_view_index: u8,
        retained_activity_field: Option<ActivityFieldValue>,
        clicked_view_id: u32,
    ) -> Result<InteractiveMutation, Error> {
        let on_create = execute_interactive_on_create(
            self.bytes,
            self.header,
            self.activity,
            self.activity_field,
            scene,
        )?;
        if on_create.listeners.first()? != listener_view_index
            || !retained_activity_field_identity_matches(
                on_create.activity_field,
                retained_activity_field,
            )
        {
            return Err(Error::DexFrameworkState);
        }
        let (clicked_view_index, listener) = scene
            .find_by_id(clicked_view_id)
            .ok_or(Error::DexFrameworkState)?;
        if listener.kind != ViewKind::Button
            || listener.id == 0
            || !on_create.listeners.contains(clicked_view_index)
        {
            return Err(Error::DexFrameworkState);
        }
        let method = self.on_click.ok_or(Error::DexActivityMethodMissing)?;
        let callback = execute_on_click(
            self.bytes,
            self.header,
            method,
            scene,
            on_create.listeners,
            retained_activity_field,
            clicked_view_index,
        )?;
        Ok(InteractiveMutation {
            target_view_index: callback.target_view_index,
            target_view_id: callback.target_view_id,
            text: callback.text,
            activity_field: callback.activity_field,
            method: method.info,
            instruction_count: callback.instruction_count,
            #[cfg(feature = "androidbox-dex-methods8")]
            app_defined_call_count: callback.app_defined_call_count,
            #[cfg(feature = "androidbox-dex-instance9")]
            app_defined_instance_call_count: callback.app_defined_instance_call_count,
            #[cfg(feature = "androidbox-activity-fields10")]
            activity_field_read_count: callback.activity_field_read_count,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_field_write_count: callback.activity_field_write_count,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_int_state_value: callback.activity_int_state_value,
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic_string_text: callback.text.is_dynamic(),
        })
    }
}

fn validate_header(bytes: &[u8]) -> Result<(Header, u32), Error> {
    if bytes.len() > MAX_DEX_BYTES {
        return Err(Error::DexTooLarge);
    }
    if bytes.len() < HEADER_SIZE {
        return Err(Error::DexHeader);
    }
    let magic = bytes.get(..8).ok_or(Error::DexMagic)?;
    if &magic[..4] != b"dex\n"
        || !matches!(
            &magic[4..7],
            b"035" | b"037" | b"038" | b"039" | b"040" | b"041"
        )
        || magic[7] != 0
    {
        return Err(Error::DexMagic);
    }
    let stored_checksum = read_u32(bytes, 8)?;
    if adler32(&bytes[12..]) != stored_checksum {
        return Err(Error::DexChecksum);
    }
    let stored_signature = bytes.get(12..32).ok_or(Error::DexHeader)?;
    if sha1(&bytes[32..]) != stored_signature {
        return Err(Error::DexSignature);
    }
    let file_size = read_u32(bytes, 32)?;
    if usize::try_from(file_size).map_err(|_| Error::DexBounds)? != bytes.len()
        || read_u32(bytes, 36)? != HEADER_SIZE as u32
    {
        return Err(Error::DexHeader);
    }
    if read_u32(bytes, 40)? != ENDIAN_CONSTANT {
        return Err(Error::DexEndian);
    }
    let link_size = read_u32(bytes, 44)?;
    let link_off = read_u32(bytes, 48)?;
    if link_size != 0 || link_off != 0 {
        return Err(Error::DexHeader);
    }

    let header = Header {
        file_size,
        map_off: read_u32(bytes, 52)?,
        string_ids: Section {
            size: read_u32(bytes, 56)?,
            offset: read_u32(bytes, 60)?,
        },
        type_ids: Section {
            size: read_u32(bytes, 64)?,
            offset: read_u32(bytes, 68)?,
        },
        proto_ids: Section {
            size: read_u32(bytes, 72)?,
            offset: read_u32(bytes, 76)?,
        },
        field_ids: Section {
            size: read_u32(bytes, 80)?,
            offset: read_u32(bytes, 84)?,
        },
        method_ids: Section {
            size: read_u32(bytes, 88)?,
            offset: read_u32(bytes, 92)?,
        },
        class_defs: Section {
            size: read_u32(bytes, 96)?,
            offset: read_u32(bytes, 100)?,
        },
        data: Section {
            size: read_u32(bytes, 104)?,
            offset: read_u32(bytes, 108)?,
        },
    };
    validate_section(bytes, header.string_ids, 4, 4, MAX_STRINGS)?;
    validate_section(bytes, header.type_ids, 4, 4, MAX_TYPES)?;
    validate_section(bytes, header.proto_ids, 12, 4, MAX_PROTOS)?;
    validate_section(bytes, header.field_ids, 8, 4, MAX_FIELDS)?;
    validate_section(bytes, header.method_ids, 8, 4, MAX_METHODS)?;
    validate_section(bytes, header.class_defs, 32, 4, MAX_CLASSES)?;
    if header.data.size == 0
        || header.data.offset < HEADER_SIZE as u32
        || !header.data.offset.is_multiple_of(4)
        || checked_range(bytes, header.data.offset, header.data.size, 1)?.end != bytes.len()
        || header.map_off < header.data.offset
        || header.map_off >= file_size
        || !header.map_off.is_multiple_of(4)
    {
        return Err(Error::DexBounds);
    }
    Ok((header, stored_checksum))
}

fn validate_section(
    bytes: &[u8],
    section: Section,
    item_size: u32,
    alignment: u32,
    maximum: u32,
) -> Result<(), Error> {
    if section.size > maximum {
        return Err(Error::DexBounds);
    }
    if section.size == 0 {
        return if section.offset == 0 {
            Ok(())
        } else {
            Err(Error::DexBounds)
        };
    }
    if section.offset < HEADER_SIZE as u32 || !section.offset.is_multiple_of(alignment) {
        return Err(Error::DexBounds);
    }
    let byte_size = section
        .size
        .checked_mul(item_size)
        .ok_or(Error::DexBounds)?;
    checked_range(bytes, section.offset, byte_size, 1)?;
    Ok(())
}

fn validate_map(bytes: &[u8], header: Header) -> Result<(), Error> {
    let count = read_u32(bytes, usize_from(header.map_off)?)?;
    if count == 0 || count > MAX_MAP_ITEMS {
        return Err(Error::DexMap);
    }
    let items_bytes = count.checked_mul(12).ok_or(Error::DexMap)?;
    checked_range(bytes, header.map_off + 4, items_bytes, 1)?;
    let mut previous_offset = 0u32;
    let mut index = 0u32;
    while index < count {
        let base = header
            .map_off
            .checked_add(4)
            .and_then(|value| value.checked_add(index * 12))
            .ok_or(Error::DexMap)?;
        let kind = read_u16(bytes, usize_from(base)?)?;
        if read_u16(bytes, usize_from(base + 2)?)? != 0 {
            return Err(Error::DexMap);
        }
        let size = read_u32(bytes, usize_from(base + 4)?)?;
        let offset = read_u32(bytes, usize_from(base + 8)?)?;
        if size == 0 || (index != 0 && offset <= previous_offset) {
            return Err(Error::DexMap);
        }
        previous_offset = offset;
        validate_map_item(bytes, header, kind, size, offset)?;

        let mut earlier = 0u32;
        while earlier < index {
            let earlier_base = header.map_off + 4 + earlier * 12;
            if read_u16(bytes, usize_from(earlier_base)?)? == kind {
                return Err(Error::DexMap);
            }
            earlier += 1;
        }
        index += 1;
    }
    require_map_item(bytes, header, TYPE_HEADER_ITEM, 1, 0)?;
    require_map_section(bytes, header, TYPE_STRING_ID_ITEM, header.string_ids)?;
    require_map_section(bytes, header, TYPE_TYPE_ID_ITEM, header.type_ids)?;
    require_map_section(bytes, header, TYPE_PROTO_ID_ITEM, header.proto_ids)?;
    require_map_section(bytes, header, TYPE_FIELD_ID_ITEM, header.field_ids)?;
    require_map_section(bytes, header, TYPE_METHOD_ID_ITEM, header.method_ids)?;
    require_map_section(bytes, header, TYPE_CLASS_DEF_ITEM, header.class_defs)?;
    require_map_item(bytes, header, TYPE_MAP_LIST, 1, header.map_off)?;
    Ok(())
}

fn validate_map_item(
    bytes: &[u8],
    header: Header,
    kind: u16,
    size: u32,
    offset: u32,
) -> Result<(), Error> {
    let fixed_width = match kind {
        TYPE_HEADER_ITEM => Some(HEADER_SIZE as u32),
        TYPE_STRING_ID_ITEM | TYPE_TYPE_ID_ITEM | TYPE_CALL_SITE_ID_ITEM => Some(4),
        TYPE_PROTO_ID_ITEM => Some(12),
        TYPE_FIELD_ID_ITEM | TYPE_METHOD_ID_ITEM | TYPE_METHOD_HANDLE_ITEM => Some(8),
        TYPE_CLASS_DEF_ITEM => Some(32),
        TYPE_MAP_LIST => {
            if size != 1 || offset != header.map_off {
                return Err(Error::DexMap);
            }
            None
        }
        TYPE_TYPE_LIST
        | TYPE_ANNOTATION_SET_REF_LIST
        | TYPE_ANNOTATION_SET_ITEM
        | TYPE_CLASS_DATA_ITEM
        | TYPE_CODE_ITEM
        | TYPE_STRING_DATA_ITEM
        | TYPE_DEBUG_INFO_ITEM
        | TYPE_ANNOTATION_ITEM
        | TYPE_ENCODED_ARRAY_ITEM
        | TYPE_ANNOTATIONS_DIRECTORY_ITEM
        | TYPE_HIDDENAPI_CLASS_DATA_ITEM => None,
        _ => return Err(Error::DexMap),
    };
    if kind == TYPE_HEADER_ITEM {
        if size != 1 || offset != 0 {
            return Err(Error::DexMap);
        }
    } else if offset < header.data.offset && kind >= TYPE_MAP_LIST {
        return Err(Error::DexMap);
    }
    if let Some(width) = fixed_width {
        let bytes_len = size.checked_mul(width).ok_or(Error::DexMap)?;
        checked_range(bytes, offset, bytes_len, 1).map_err(|_| Error::DexMap)?;
    } else if offset >= header.file_size {
        return Err(Error::DexMap);
    }
    Ok(())
}

fn require_map_section(
    bytes: &[u8],
    header: Header,
    kind: u16,
    section: Section,
) -> Result<(), Error> {
    if section.size == 0 {
        if find_map_item(bytes, header, kind)?.is_some() {
            return Err(Error::DexMap);
        }
        return Ok(());
    }
    require_map_item(bytes, header, kind, section.size, section.offset)
}

fn require_map_item(
    bytes: &[u8],
    header: Header,
    kind: u16,
    expected_size: u32,
    expected_offset: u32,
) -> Result<(), Error> {
    match find_map_item(bytes, header, kind)? {
        Some((size, offset)) if size == expected_size && offset == expected_offset => Ok(()),
        _ => Err(Error::DexMap),
    }
}

fn find_map_item(bytes: &[u8], header: Header, kind: u16) -> Result<Option<(u32, u32)>, Error> {
    let count = read_u32(bytes, usize_from(header.map_off)?)?;
    let mut index = 0u32;
    while index < count {
        let base = header.map_off + 4 + index * 12;
        if read_u16(bytes, usize_from(base)?)? == kind {
            return Ok(Some((
                read_u32(bytes, usize_from(base + 4)?)?,
                read_u32(bytes, usize_from(base + 8)?)?,
            )));
        }
        index += 1;
    }
    Ok(None)
}

fn validate_tables(bytes: &[u8], header: Header) -> Result<(), Error> {
    let mut index = 0u32;
    while index < header.string_ids.size {
        dex_string(bytes, header, index)?;
        index += 1;
    }
    index = 0;
    while index < header.type_ids.size {
        let descriptor = read_table_u32(bytes, header.type_ids, 4, index, 0)?;
        if descriptor >= header.string_ids.size {
            return Err(Error::DexBounds);
        }
        index += 1;
    }
    index = 0;
    while index < header.proto_ids.size {
        let shorty = read_table_u32(bytes, header.proto_ids, 12, index, 0)?;
        let return_type = read_table_u32(bytes, header.proto_ids, 12, index, 4)?;
        let parameters = read_table_u32(bytes, header.proto_ids, 12, index, 8)?;
        if shorty >= header.string_ids.size || return_type >= header.type_ids.size {
            return Err(Error::DexBounds);
        }
        dex_string(bytes, header, shorty)?;
        if parameters != 0 {
            validate_type_list(bytes, header, parameters)?;
        }
        index += 1;
    }
    index = 0;
    while index < header.field_ids.size {
        let class_index = u32::from(read_table_u16(bytes, header.field_ids, 8, index, 0)?);
        let type_index = u32::from(read_table_u16(bytes, header.field_ids, 8, index, 2)?);
        let name_index = read_table_u32(bytes, header.field_ids, 8, index, 4)?;
        if class_index >= header.type_ids.size
            || type_index >= header.type_ids.size
            || name_index >= header.string_ids.size
        {
            return Err(Error::DexBounds);
        }
        index += 1;
    }
    index = 0;
    while index < header.method_ids.size {
        let class_index = u32::from(read_table_u16(bytes, header.method_ids, 8, index, 0)?);
        let proto_index = u32::from(read_table_u16(bytes, header.method_ids, 8, index, 2)?);
        let name_index = read_table_u32(bytes, header.method_ids, 8, index, 4)?;
        if class_index >= header.type_ids.size
            || proto_index >= header.proto_ids.size
            || name_index >= header.string_ids.size
        {
            return Err(Error::DexBounds);
        }
        index += 1;
    }
    index = 0;
    while index < header.class_defs.size {
        let base = table_offset(header.class_defs, 32, index)?;
        let class_index = read_u32(bytes, base)?;
        let superclass = read_u32(bytes, base + 8)?;
        let interfaces = read_u32(bytes, base + 12)?;
        let source_file = read_u32(bytes, base + 16)?;
        let annotations = read_u32(bytes, base + 20)?;
        let class_data = read_u32(bytes, base + 24)?;
        let static_values = read_u32(bytes, base + 28)?;
        if class_index >= header.type_ids.size
            || (superclass != NO_INDEX && superclass >= header.type_ids.size)
            || (source_file != NO_INDEX && source_file >= header.string_ids.size)
        {
            return Err(Error::DexBounds);
        }
        if interfaces != 0 {
            validate_type_list(bytes, header, interfaces)?;
        }
        for offset in [annotations, class_data, static_values] {
            if offset != 0 && (offset < header.data.offset || offset >= header.file_size) {
                return Err(Error::DexBounds);
            }
        }
        index += 1;
    }
    Ok(())
}

fn validate_type_list(bytes: &[u8], header: Header, offset: u32) -> Result<(), Error> {
    if offset < header.data.offset || !offset.is_multiple_of(4) {
        return Err(Error::DexBounds);
    }
    let count = read_u32(bytes, usize_from(offset)?)?;
    if count > MAX_TYPES {
        return Err(Error::DexBounds);
    }
    checked_range(
        bytes,
        offset + 4,
        count.checked_mul(2).ok_or(Error::DexBounds)?,
        1,
    )?;
    let mut index = 0u32;
    while index < count {
        if u32::from(read_u16(bytes, usize_from(offset + 4 + index * 2)?)?) >= header.type_ids.size
        {
            return Err(Error::DexBounds);
        }
        index += 1;
    }
    Ok(())
}

fn locate_entries(bytes: &[u8], header: Header) -> Result<(Method, Method), Error> {
    let mut class_data_offset = None;
    let mut class_index = 0u32;
    let mut index = 0u32;
    while index < header.class_defs.size {
        let base = table_offset(header.class_defs, 32, index)?;
        let candidate = read_u32(bytes, base)?;
        if type_descriptor(bytes, header, candidate)? == ENTRY_CLASS {
            if class_data_offset.is_some() {
                return Err(Error::DexClassDuplicate);
            }
            let offset = read_u32(bytes, base + 24)?;
            if offset == 0 {
                return Err(Error::DexMethodMissing);
            }
            class_data_offset = Some(offset);
            class_index = candidate;
        }
        index += 1;
    }
    let class_data_offset = class_data_offset.ok_or(Error::DexClassMissing)?;
    scan_class_data(bytes, header, class_data_offset, class_index)
}

fn scan_class_data(
    bytes: &[u8],
    header: Header,
    class_data_offset: u32,
    class_index: u32,
) -> Result<(Method, Method), Error> {
    let mut cursor = usize_from(class_data_offset)?;
    let static_fields = read_uleb128(bytes, &mut cursor)?;
    let instance_fields = read_uleb128(bytes, &mut cursor)?;
    let direct_methods = read_uleb128(bytes, &mut cursor)?;
    let virtual_methods = read_uleb128(bytes, &mut cursor)?;
    for count in [
        static_fields,
        instance_fields,
        direct_methods,
        virtual_methods,
    ] {
        if count > MAX_CLASS_MEMBERS {
            return Err(Error::DexCode);
        }
    }
    skip_encoded_fields(bytes, &mut cursor, static_fields, header.field_ids.size)?;
    skip_encoded_fields(bytes, &mut cursor, instance_fields, header.field_ids.size)?;

    let mut boot = None;
    let mut on_tap = None;
    scan_encoded_methods(
        bytes,
        header,
        &mut cursor,
        direct_methods,
        class_index,
        &mut boot,
        &mut on_tap,
    )?;
    scan_encoded_methods(
        bytes,
        header,
        &mut cursor,
        virtual_methods,
        class_index,
        &mut boot,
        &mut on_tap,
    )?;
    Ok((
        boot.ok_or(Error::DexMethodMissing)?,
        on_tap.ok_or(Error::DexMethodMissing)?,
    ))
}

fn skip_encoded_fields(
    bytes: &[u8],
    cursor: &mut usize,
    count: u32,
    field_count: u32,
) -> Result<(), Error> {
    let mut accumulated = 0u32;
    let mut index = 0u32;
    while index < count {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= field_count {
            return Err(Error::DexBounds);
        }
        read_uleb128(bytes, cursor)?;
        index += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn scan_encoded_methods(
    bytes: &[u8],
    header: Header,
    cursor: &mut usize,
    count: u32,
    expected_class: u32,
    boot: &mut Option<Method>,
    on_tap: &mut Option<Method>,
) -> Result<(), Error> {
    let mut accumulated = 0u32;
    let mut index = 0u32;
    while index < count {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        let access_flags = read_uleb128(bytes, cursor)?;
        let code_offset = read_uleb128(bytes, cursor)?;
        let method_class = u32::from(read_table_u16(bytes, header.method_ids, 8, accumulated, 0)?);
        if method_class != expected_class {
            return Err(Error::DexBounds);
        }
        let name_index = read_table_u32(bytes, header.method_ids, 8, accumulated, 4)?;
        let name = dex_string(bytes, header, name_index)?;
        if name == BOOT_METHOD || name == ON_TAP_METHOD {
            let is_boot = name == BOOT_METHOD;
            validate_entry_proto(bytes, header, accumulated, is_boot)?;
            if access_flags & (ACC_NATIVE | ACC_ABSTRACT) != 0 {
                return Err(Error::DexNativeOrAbstract);
            }
            if access_flags & (ACC_PUBLIC | ACC_STATIC) != (ACC_PUBLIC | ACC_STATIC) {
                return Err(Error::DexMethodFlags);
            }
            let parameters = if is_boot { 0 } else { 1 };
            let code = validate_code(bytes, header, code_offset, parameters, 0)?;
            let method = Method {
                info: MethodInfo {
                    method_index: accumulated,
                    code_offset,
                    registers: code.registers,
                    parameters: code.parameters,
                    instruction_units: code.units,
                },
            };
            if is_boot {
                if boot.replace(method).is_some() {
                    return Err(Error::DexMethodDuplicate);
                }
            } else if on_tap.replace(method).is_some() {
                return Err(Error::DexMethodDuplicate);
            }
        }
        index += 1;
    }
    Ok(())
}

fn validate_entry_proto(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    is_boot: bool,
) -> Result<(), Error> {
    let proto_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        2,
    )?);
    let shorty_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 0)?;
    let return_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 4)?;
    let parameters_offset = read_table_u32(bytes, header.proto_ids, 12, proto_index, 8)?;
    if type_descriptor(bytes, header, return_index)? != b"I" {
        return Err(Error::DexPrototype);
    }
    if is_boot {
        if dex_string(bytes, header, shorty_index)? != b"I" || parameters_offset != 0 {
            return Err(Error::DexPrototype);
        }
    } else {
        if dex_string(bytes, header, shorty_index)? != b"II" || parameters_offset == 0 {
            return Err(Error::DexPrototype);
        }
        let parameter_count = read_u32(bytes, usize_from(parameters_offset)?)?;
        if parameter_count != 1
            || type_descriptor(
                bytes,
                header,
                u32::from(read_u16(bytes, usize_from(parameters_offset + 4)?)?),
            )? != b"I"
        {
            return Err(Error::DexPrototype);
        }
    }
    Ok(())
}

fn locate_activity(
    bytes: &[u8],
    header: Header,
    activity_descriptor: &[u8],
) -> Result<ActivityClass, Error> {
    let mut found = None;
    let mut index = 0u32;
    while index < header.class_defs.size {
        let base = table_offset(header.class_defs, 32, index)?;
        let class_type = read_u32(bytes, base)?;
        if type_descriptor(bytes, header, class_type)? == activity_descriptor {
            if found.is_some() {
                return Err(Error::DexActivityClassDuplicate);
            }
            let access_flags = read_u32(bytes, base + 4)?;
            let superclass = read_u32(bytes, base + 8)?;
            let interfaces = read_u32(bytes, base + 12)?;
            let class_data = read_u32(bytes, base + 24)?;
            if access_flags != (ACC_PUBLIC | ACC_FINAL)
                || superclass == NO_INDEX
                || type_descriptor(bytes, header, superclass)? != ACTIVITY_CLASS
                || class_data == 0
            {
                return Err(Error::DexActivityClassInvalid);
            }
            let interactive = if interfaces == 0 {
                false
            } else {
                validate_on_click_listener_interface(bytes, header, interfaces)?
            };
            found = Some(scan_activity_class(
                bytes,
                header,
                class_data,
                class_type,
                interactive,
            )?);
        }
        index += 1;
    }
    found.ok_or(Error::DexActivityClassMissing)
}

fn scan_activity_class(
    bytes: &[u8],
    header: Header,
    class_data_offset: u32,
    class_type: u32,
    interactive: bool,
) -> Result<ActivityClass, Error> {
    let mut cursor = usize_from(class_data_offset)?;
    let static_fields = read_uleb128(bytes, &mut cursor)?;
    let instance_fields = read_uleb128(bytes, &mut cursor)?;
    let direct_methods = read_uleb128(bytes, &mut cursor)?;
    let virtual_methods = read_uleb128(bytes, &mut cursor)?;
    for count in [
        static_fields,
        instance_fields,
        direct_methods,
        virtual_methods,
    ] {
        if count > MAX_CLASS_MEMBERS {
            return Err(Error::DexCode);
        }
    }
    let activity_field = scan_activity_fields(
        bytes,
        header,
        &mut cursor,
        class_type,
        interactive,
        static_fields,
        instance_fields,
    )?;

    let mut methods = ActivityMethods {
        constructor: None,
        on_create: None,
        on_click: None,
    };
    scan_activity_methods(
        bytes,
        header,
        &mut cursor,
        direct_methods,
        class_type,
        false,
        &mut methods,
    )?;
    scan_activity_methods(
        bytes,
        header,
        &mut cursor,
        virtual_methods,
        class_type,
        true,
        &mut methods,
    )?;
    Ok(ActivityClass {
        constructor: methods.constructor.ok_or(Error::DexMethodMissing)?,
        on_create: methods.on_create.ok_or(Error::DexActivityMethodMissing)?,
        on_click: match (interactive, methods.on_click) {
            (true, Some(method)) => Some(method),
            (true, None) => return Err(Error::DexActivityMethodMissing),
            (false, None) => None,
            (false, Some(_)) => return Err(Error::DexActivityClassInvalid),
        },
        activity_field,
    })
}

fn scan_activity_fields(
    bytes: &[u8],
    header: Header,
    cursor: &mut usize,
    class_type: u32,
    interactive: bool,
    static_fields: u32,
    instance_fields: u32,
) -> Result<Option<ActivityFieldDeclaration>, Error> {
    if static_fields != 0 {
        return Err(Error::DexActivityClassInvalid);
    }
    #[cfg(not(feature = "androidbox-activity-fields10"))]
    {
        let _ = (bytes, header, cursor, class_type, interactive);
        if instance_fields != 0 {
            return Err(Error::DexActivityClassInvalid);
        }
        Ok(None)
    }
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    {
        if instance_fields == 0 {
            return Ok(None);
        }
        if !interactive || instance_fields != 1 {
            return Err(Error::DexActivityClassInvalid);
        }
        let field_index = read_uleb128(bytes, cursor)?;
        if field_index >= header.field_ids.size {
            return Err(Error::DexBounds);
        }
        let access_flags = read_uleb128(bytes, cursor)?;
        let owner = u32::from(read_table_u16(bytes, header.field_ids, 8, field_index, 0)?);
        let field_type = u32::from(read_table_u16(bytes, header.field_ids, 8, field_index, 2)?);
        if access_flags != ACC_PRIVATE
            || owner != class_type
            || type_descriptor(bytes, header, field_type)? != TEXT_VIEW_CLASS
        {
            return Err(Error::DexActivityClassInvalid);
        }
        Ok(Some(ActivityFieldDeclaration {
            text_view_field_index: field_index,
            int_field_index: None,
        }))
    }
    #[cfg(feature = "androidbox-activity-state11")]
    {
        if instance_fields == 0 {
            return Ok(None);
        }
        if !interactive || !(1..=2).contains(&instance_fields) {
            return Err(Error::DexActivityClassInvalid);
        }
        let mut accumulated = 0u32;
        let mut text_view_field_index = None;
        let mut int_field_index = None;
        for _ in 0..instance_fields {
            accumulated = accumulated
                .checked_add(read_uleb128(bytes, cursor)?)
                .ok_or(Error::DexBounds)?;
            if accumulated >= header.field_ids.size {
                return Err(Error::DexBounds);
            }
            let access_flags = read_uleb128(bytes, cursor)?;
            let owner = u32::from(read_table_u16(bytes, header.field_ids, 8, accumulated, 0)?);
            let field_type = u32::from(read_table_u16(bytes, header.field_ids, 8, accumulated, 2)?);
            if access_flags != ACC_PRIVATE || owner != class_type {
                return Err(Error::DexActivityClassInvalid);
            }
            match type_descriptor(bytes, header, field_type)? {
                TEXT_VIEW_CLASS if text_view_field_index.replace(accumulated).is_none() => {}
                INT_CLASS if int_field_index.replace(accumulated).is_none() => {}
                _ => return Err(Error::DexActivityClassInvalid),
            }
        }
        let text_view_field_index = text_view_field_index.ok_or(Error::DexActivityClassInvalid)?;
        if (instance_fields == 1) != int_field_index.is_none() {
            return Err(Error::DexActivityClassInvalid);
        }
        Ok(Some(ActivityFieldDeclaration {
            text_view_field_index,
            int_field_index,
        }))
    }
}

fn validate_on_click_listener_interface(
    bytes: &[u8],
    header: Header,
    interfaces_offset: u32,
) -> Result<bool, Error> {
    if interfaces_offset < header.data.offset || !interfaces_offset.is_multiple_of(4) {
        return Err(Error::DexActivityClassInvalid);
    }
    checked_range(bytes, interfaces_offset, 8, 4)?;
    let offset = usize_from(interfaces_offset)?;
    if read_u32(bytes, offset)? != 1 || read_u16(bytes, offset + 6)? != 0 {
        return Err(Error::DexActivityClassInvalid);
    }
    let interface_type = u32::from(read_u16(bytes, offset + 4)?);
    if interface_type >= header.type_ids.size
        || type_descriptor(bytes, header, interface_type)? != ON_CLICK_LISTENER_CLASS
    {
        return Err(Error::DexActivityClassInvalid);
    }
    Ok(true)
}

fn scan_activity_methods(
    bytes: &[u8],
    header: Header,
    cursor: &mut usize,
    count: u32,
    expected_class: u32,
    is_virtual: bool,
    methods: &mut ActivityMethods,
) -> Result<(), Error> {
    let mut accumulated = 0u32;
    let mut index = 0u32;
    while index < count {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        let access_flags = read_uleb128(bytes, cursor)?;
        let code_offset = read_uleb128(bytes, cursor)?;
        let method_class = u32::from(read_table_u16(bytes, header.method_ids, 8, accumulated, 0)?);
        if method_class != expected_class {
            return Err(Error::DexBounds);
        }
        let name_index = read_table_u32(bytes, header.method_ids, 8, accumulated, 4)?;
        let name = dex_string(bytes, header, name_index)?;
        if name == INIT_METHOD {
            if !constructor_proto_matches(bytes, header, accumulated)? {
                return Err(Error::DexPrototype);
            }
            if is_virtual {
                return Err(Error::DexActivityClassInvalid);
            }
            if access_flags != (ACC_PUBLIC | ACC_CONSTRUCTOR) {
                return Err(Error::DexMethodFlags);
            }
            if methods.constructor.is_some() {
                return Err(Error::DexMethodDuplicate);
            }
            let code = validate_code(bytes, header, code_offset, 1, 1)?;
            let method = ActivityConstructor {
                info: ActivityConstructorInfo {
                    method_index: accumulated,
                    code_offset,
                    registers: code.registers,
                    parameters: code.parameters,
                    outgoing: code.outgoing,
                    instruction_units: code.units,
                },
            };
            methods.constructor = Some(method);
        } else if name == ON_CREATE_METHOD {
            if !activity_proto_matches(bytes, header, accumulated)? {
                return Err(Error::DexActivityPrototype);
            }
            if !is_virtual {
                return Err(Error::DexActivityClassInvalid);
            }
            if access_flags != ACC_PROTECTED {
                return Err(Error::DexMethodFlags);
            }
            let code = validate_code(bytes, header, code_offset, 2, 2)?;
            let method = ActivityMethod {
                info: ActivityMethodInfo {
                    method_index: accumulated,
                    code_offset,
                    registers: code.registers,
                    parameters: code.parameters,
                    outgoing: code.outgoing,
                    instruction_units: code.units,
                },
                class_type_index: expected_class,
            };
            if methods.on_create.replace(method).is_some() {
                return Err(Error::DexActivityMethodDuplicate);
            }
        } else if name == ON_CLICK_METHOD {
            if !method_proto_matches(bytes, header, accumulated, VOID_CLASS, VIEW_CLASS)? {
                return Err(Error::DexActivityPrototype);
            }
            if !is_virtual {
                return Err(Error::DexActivityClassInvalid);
            }
            if access_flags != ACC_PUBLIC {
                return Err(Error::DexMethodFlags);
            }
            let code = validate_code(bytes, header, code_offset, 2, 2)?;
            let method = ActivityMethod {
                info: ActivityMethodInfo {
                    method_index: accumulated,
                    code_offset,
                    registers: code.registers,
                    parameters: code.parameters,
                    outgoing: code.outgoing,
                    instruction_units: code.units,
                },
                class_type_index: expected_class,
            };
            if methods.on_click.replace(method).is_some() {
                return Err(Error::DexActivityMethodDuplicate);
            }
        }
        index += 1;
    }
    Ok(())
}

fn constructor_proto_matches(
    bytes: &[u8],
    header: Header,
    method_index: u32,
) -> Result<bool, Error> {
    let proto_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        2,
    )?);
    let shorty_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 0)?;
    let return_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 4)?;
    let parameters_offset = read_table_u32(bytes, header.proto_ids, 12, proto_index, 8)?;
    Ok(dex_string(bytes, header, shorty_index)? == b"V"
        && type_descriptor(bytes, header, return_index)? == VOID_CLASS
        && parameters_offset == 0)
}

fn activity_proto_matches(bytes: &[u8], header: Header, method_index: u32) -> Result<bool, Error> {
    let proto_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        2,
    )?);
    let shorty_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 0)?;
    let return_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 4)?;
    let parameters_offset = read_table_u32(bytes, header.proto_ids, 12, proto_index, 8)?;
    if dex_string(bytes, header, shorty_index)? != b"VL"
        || type_descriptor(bytes, header, return_index)? != VOID_CLASS
        || parameters_offset == 0
    {
        return Ok(false);
    }
    let parameter_count = read_u32(bytes, usize_from(parameters_offset)?)?;
    if parameter_count != 1 {
        return Ok(false);
    }
    Ok(type_descriptor(
        bytes,
        header,
        u32::from(read_u16(bytes, usize_from(parameters_offset + 4)?)?),
    )? == BUNDLE_CLASS)
}

fn validate_code(
    bytes: &[u8],
    header: Header,
    code_offset: u32,
    expected_parameters: u16,
    expected_outgoing: u16,
) -> Result<Code, Error> {
    if code_offset == 0 || code_offset < header.data.offset || !code_offset.is_multiple_of(4) {
        return Err(Error::DexCode);
    }
    checked_range(bytes, code_offset, 16, 1)?;
    let base = usize_from(code_offset)?;
    let registers = read_u16(bytes, base)?;
    let parameters = read_u16(bytes, base + 2)?;
    let outgoing = read_u16(bytes, base + 4)?;
    let tries = read_u16(bytes, base + 6)?;
    let debug_info = read_u32(bytes, base + 8)?;
    let units = read_u32(bytes, base + 12)?;
    if registers == 0
        || registers > MAX_REGISTERS
        || parameters != expected_parameters
        || registers < parameters
    {
        return Err(Error::DexRegisterLimit);
    }
    if outgoing != expected_outgoing {
        return Err(Error::DexCode);
    }
    if tries != 0 {
        return Err(Error::DexExceptionsUnsupported);
    }
    if debug_info != 0 {
        return Err(Error::DexCode);
    }
    if units == 0 || units > MAX_INSTRUCTION_UNITS {
        return Err(Error::DexInstructionLimit);
    }
    let units_offset = base.checked_add(16).ok_or(Error::DexBounds)?;
    let byte_count = units.checked_mul(2).ok_or(Error::DexBounds)?;
    checked_range(bytes, u32_from(units_offset)?, byte_count, 1)?;
    Ok(Code {
        registers,
        parameters,
        outgoing,
        units_offset,
        units,
    })
}

fn execute(bytes: &[u8], method: Method, input: Option<i32>) -> Result<(i32, u16), Error> {
    let code = validate_code(
        bytes,
        Header {
            file_size: bytes.len() as u32,
            map_off: 0,
            string_ids: Section { size: 0, offset: 0 },
            type_ids: Section { size: 0, offset: 0 },
            proto_ids: Section { size: 0, offset: 0 },
            field_ids: Section { size: 0, offset: 0 },
            method_ids: Section { size: 0, offset: 0 },
            class_defs: Section { size: 0, offset: 0 },
            data: Section {
                size: bytes.len() as u32 - HEADER_SIZE as u32,
                offset: HEADER_SIZE as u32,
            },
        },
        method.info.code_offset,
        method.info.parameters,
        0,
    )?;
    let mut registers = [0i32; MAX_REGISTERS as usize];
    if let Some(value) = input {
        let parameter = usize::from(code.registers - code.parameters);
        registers[parameter] = value;
    }
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        if steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        steps += 1;
        match opcode {
            0x01 => {
                let destination = usize::from((first >> 8) & 0x0f);
                let source = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, source)?;
                registers[destination] = registers[source];
                pc += 1;
            }
            0x0f => {
                let register = usize::from(first >> 8);
                ensure_register(code.registers, register)?;
                pc += 1;
                if pc != code.units {
                    return Err(Error::DexCode);
                }
                return Ok((registers[register], steps));
            }
            0x12 => {
                let destination = usize::from((first >> 8) & 0x0f);
                ensure_register(code.registers, destination)?;
                registers[destination] = i32::from((first as i16) >> 12);
                pc += 1;
            }
            0x13 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] = i32::from(instruction_unit(bytes, code, pc + 1)? as i16);
                pc += 2;
            }
            0x14 => {
                ensure_remaining(code, pc, 3)?;
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                let low = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let high = u32::from(instruction_unit(bytes, code, pc + 2)?);
                registers[destination] = (low | (high << 16)) as i32;
                pc += 3;
            }
            0xd8 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from(first >> 8);
                let operands = instruction_unit(bytes, code, pc + 1)?;
                let source = usize::from(operands as u8);
                let literal = i32::from((operands >> 8) as u8 as i8);
                ensure_registers(code.registers, destination, source)?;
                registers[destination] = registers[source].wrapping_add(literal);
                pc += 2;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

fn execute_activity_lifecycle(
    bytes: &[u8],
    header: Header,
    activity: ActivityClass,
) -> Result<(ActivityContent, u16, u16), Error> {
    let constructor_instruction_count =
        execute_activity_constructor(bytes, header, activity.constructor)?;
    let (content, on_create_instruction_count) =
        execute_on_create(bytes, header, activity.on_create)?;
    Ok((
        content,
        constructor_instruction_count,
        on_create_instruction_count,
    ))
}

fn execute_activity_constructor(
    bytes: &[u8],
    header: Header,
    constructor: ActivityConstructor,
) -> Result<u16, Error> {
    let code = validate_code(
        bytes,
        header,
        constructor.info.code_offset,
        constructor.info.parameters,
        constructor.info.outgoing,
    )?;
    let activity_register = usize::from(code.registers - code.parameters);
    let mut super_constructed = false;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        if steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        steps += 1;
        match opcode {
            0x0e => {
                if first != 0x000e || !super_constructed {
                    return Err(Error::DexFrameworkState);
                }
                pc += 1;
                if pc != code.units {
                    return Err(Error::DexCode);
                }
                return Ok(steps);
            }
            0x70 => {
                if super_constructed {
                    return Err(Error::DexFrameworkState);
                }
                let invocation = decode_invoke(bytes, code, pc)?;
                if invocation.count != 1 || invocation.registers[0] != activity_register {
                    return Err(Error::DexFrameworkState);
                }
                resolve_activity_super_constructor(bytes, header, invocation.method_index)?;
                super_constructed = true;
                pc += 3;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

fn discover_interactive_layout_resource(
    bytes: &[u8],
    header: Header,
    method: ActivityMethod,
) -> Result<u32, Error> {
    let code = validate_code(
        bytes,
        header,
        method.info.code_offset,
        method.info.parameters,
        method.info.outgoing,
    )?;
    let parameter_start = usize::from(code.registers - code.parameters);
    let first = instruction_unit(bytes, code, 0)?;
    if first as u8 != 0x6f {
        return Err(Error::DexFrameworkState);
    }
    let super_call = decode_invoke(bytes, code, 0)?;
    if super_call.count != 2
        || super_call.registers[0] != parameter_start
        || super_call.registers[1] != parameter_start + 1
        || !matches!(
            resolve_interactive_framework_method(bytes, header, super_call.method_index, method,)?,
            InteractiveFrameworkMethod::ActivityOnCreate
        )
    {
        return Err(Error::DexFrameworkState);
    }
    let const_pc = 3;
    let (destination, layout_resource_id, width) = decode_resource_const(bytes, code, const_pc)?;
    if destination == parameter_start {
        return Err(Error::DexFrameworkState);
    }
    let invoke_pc = const_pc.checked_add(width).ok_or(Error::DexCode)?;
    let invoke_unit = instruction_unit(bytes, code, invoke_pc)?;
    if invoke_unit as u8 != 0x6e {
        return Err(Error::DexFrameworkState);
    }
    let set_content = decode_invoke(bytes, code, invoke_pc)?;
    if set_content.count != 2
        || set_content.registers[0] != parameter_start
        || set_content.registers[1] != destination
        || !matches!(
            resolve_interactive_framework_method(bytes, header, set_content.method_index, method,)?,
            InteractiveFrameworkMethod::ActivitySetContentViewResource
        )
    {
        return Err(Error::DexFrameworkState);
    }
    Ok(layout_resource_id)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InteractiveRegister {
    Empty,
    Null,
    Activity,
    Int(u32),
    ResourceId(u32),
    ViewRef(u8),
    #[cfg(feature = "androidbox-string-text12")]
    StringIndex(u32),
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderUninitialized,
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderPrefix(u32),
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderInt(u32, u8),
    #[cfg(feature = "androidbox-string-builder13")]
    BuiltString(u32, u8),
}

#[derive(Clone, Copy)]
enum InteractiveFrameworkMethod {
    ActivityOnCreate,
    ActivitySetContentViewResource,
    ActivityFindViewById,
    ViewGetId,
    ViewSetOnClickListener,
    TextViewSetTextResource,
    #[cfg(feature = "androidbox-string-text12")]
    TextViewSetTextCharSequence,
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderInitString,
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderAppendInt,
    #[cfg(feature = "androidbox-string-builder13")]
    StringBuilderToString,
}

#[derive(Clone, Copy)]
struct InteractiveListeners {
    indices: [u8; MAX_INTERACTIVE_CALLBACKS],
    len: u8,
}

impl InteractiveListeners {
    const fn empty() -> Self {
        Self {
            indices: [0; MAX_INTERACTIVE_CALLBACKS],
            len: 0,
        }
    }

    fn insert(&mut self, index: u8) -> Result<(), Error> {
        if self.contains(index) || usize::from(self.len) >= MAX_INTERACTIVE_CALLBACKS {
            return Err(Error::DexFrameworkState);
        }
        self.indices[usize::from(self.len)] = index;
        self.len = self.len.checked_add(1).ok_or(Error::DexFrameworkState)?;
        Ok(())
    }

    fn contains(self, index: u8) -> bool {
        self.indices[..usize::from(self.len)].contains(&index)
    }

    fn first(self) -> Result<u8, Error> {
        self.indices
            .first()
            .copied()
            .filter(|_| self.len != 0)
            .ok_or(Error::DexFrameworkState)
    }

    fn as_slice(&self) -> &[u8] {
        &self.indices[..usize::from(self.len)]
    }
}

#[derive(Clone, Copy)]
struct InteractiveOnCreate {
    layout_resource_id: u32,
    listeners: InteractiveListeners,
    activity_field: Option<ActivityFieldValue>,
    instruction_count: u16,
}

fn execute_interactive_on_create(
    bytes: &[u8],
    header: Header,
    method: ActivityMethod,
    activity_field: Option<ActivityFieldDeclaration>,
    scene: &ActivityScene,
) -> Result<InteractiveOnCreate, Error> {
    let code = validate_code(
        bytes,
        header,
        method.info.code_offset,
        method.info.parameters,
        method.info.outgoing,
    )?;
    let mut registers = [InteractiveRegister::Empty; MAX_REGISTERS as usize];
    let parameter_start = usize::from(code.registers - code.parameters);
    registers[parameter_start] = InteractiveRegister::Activity;
    registers[parameter_start + 1] = InteractiveRegister::Null;
    let mut super_created = false;
    let mut layout_resource_id = None;
    let mut listeners = InteractiveListeners::empty();
    let mut pending_result = None;
    #[cfg(feature = "androidbox-activity-fields10")]
    let mut checked_field_candidate = None;
    let mut activity_text_field_value: Option<(u32, u8)> = None;
    let mut activity_int_field_value: Option<(u32, i32)> = None;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        if steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        if pending_result.is_some() && opcode != 0x0c {
            return Err(Error::DexFrameworkState);
        }
        steps += 1;
        match opcode {
            0x0c => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = pending_result.take().ok_or(Error::DexFrameworkState)?;
                pc += 1;
            }
            0x0e => {
                if first != 0x000e || pending_result.is_some() || !super_created {
                    return Err(Error::DexFrameworkState);
                }
                let layout_resource_id = layout_resource_id.ok_or(Error::DexFrameworkState)?;
                listeners.first()?;
                let activity_field_value = match activity_field {
                    None if activity_text_field_value.is_none()
                        && activity_int_field_value.is_none() =>
                    {
                        None
                    }
                    Some(declaration) => {
                        let (text_view_field_index, text_view_index) =
                            activity_text_field_value.ok_or(Error::DexFrameworkState)?;
                        if text_view_field_index != declaration.text_view_field_index {
                            return Err(Error::DexFrameworkState);
                        }
                        let int_value =
                            match (declaration.int_field_index, activity_int_field_value) {
                                (None, None) => 0,
                                (Some(expected), Some((actual, value)))
                                    if expected == actual && value == 0 =>
                                {
                                    value
                                }
                                _ => return Err(Error::DexFrameworkState),
                            };
                        Some(ActivityFieldValue {
                            text_view_field_index,
                            text_view_index,
                            int_field_index: declaration.int_field_index,
                            int_value,
                        })
                    }
                    _ => return Err(Error::DexFrameworkState),
                };
                pc += 1;
                if pc != code.units {
                    return Err(Error::DexCode);
                }
                return Ok(InteractiveOnCreate {
                    layout_resource_id,
                    listeners,
                    activity_field: activity_field_value,
                    instruction_count: steps,
                });
            }
            0x14 | 0x15 => {
                let (destination, resource_id, width) = decode_resource_const(bytes, code, pc)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = InteractiveRegister::ResourceId(resource_id);
                pc = pc.checked_add(width).ok_or(Error::DexCode)?;
            }
            #[cfg(feature = "androidbox-activity-state11")]
            0x12 => {
                let destination = usize::from((first >> 8) & 0x0f);
                ensure_register(code.registers, destination)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = InteractiveRegister::Int(u32::from_ne_bytes(
                    i32::from((first as i16) >> 12).to_ne_bytes(),
                ));
                pc += 1;
            }
            #[cfg(feature = "androidbox-activity-fields10")]
            0x1f => {
                ensure_remaining(code, pc, 2)?;
                let register = usize::from(first >> 8);
                ensure_register(code.registers, register)?;
                let type_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let view_index = match registers[register] {
                    InteractiveRegister::ViewRef(index) => index,
                    _ => return Err(Error::DexFrameworkState),
                };
                let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                if activity_field.is_none()
                    || type_descriptor(bytes, header, type_index)? != TEXT_VIEW_CLASS
                    || view.kind != ViewKind::TextView
                    || view.id == 0
                    || checked_field_candidate
                        .replace((register, view_index))
                        .is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                pc += 2;
            }
            #[cfg(feature = "androidbox-activity-fields10")]
            0x5b => {
                ensure_remaining(code, pc, 2)?;
                let value_register = usize::from((first >> 8) & 0x0f);
                let receiver_register = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, value_register, receiver_register)?;
                let field_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let view_index = match registers[value_register] {
                    InteractiveRegister::ViewRef(index) => index,
                    _ => return Err(Error::DexFrameworkState),
                };
                let declaration = activity_field.ok_or(Error::DexFrameworkState)?;
                if registers[receiver_register] != InteractiveRegister::Activity
                    || declaration.text_view_field_index != field_index
                    || checked_field_candidate != Some((value_register, view_index))
                    || activity_text_field_value
                        .replace((field_index, view_index))
                        .is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                pc += 2;
            }
            #[cfg(feature = "androidbox-activity-state11")]
            0x59 => {
                ensure_remaining(code, pc, 2)?;
                let value_register = usize::from((first >> 8) & 0x0f);
                let receiver_register = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, value_register, receiver_register)?;
                let field_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let value = match registers[value_register] {
                    InteractiveRegister::Int(value) => i32::from_ne_bytes(value.to_ne_bytes()),
                    _ => return Err(Error::DexFrameworkState),
                };
                let declaration = activity_field.ok_or(Error::DexFrameworkState)?;
                if registers[receiver_register] != InteractiveRegister::Activity
                    || declaration.int_field_index != Some(field_index)
                    || value != 0
                    || activity_int_field_value
                        .replace((field_index, value))
                        .is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                pc += 2;
            }
            0x6e | 0x6f => {
                let invocation = decode_invoke(bytes, code, pc)?;
                let framework = resolve_interactive_framework_method(
                    bytes,
                    header,
                    invocation.method_index,
                    method,
                )?;
                if invocation.count != 2 {
                    return Err(Error::DexFrameworkState);
                }
                let receiver = registers[invocation.registers[0]];
                let argument = registers[invocation.registers[1]];
                match framework {
                    InteractiveFrameworkMethod::ActivityOnCreate => {
                        if opcode != 0x6f
                            || super_created
                            || layout_resource_id.is_some()
                            || receiver != InteractiveRegister::Activity
                            || argument != InteractiveRegister::Null
                        {
                            return Err(Error::DexFrameworkState);
                        }
                        super_created = true;
                    }
                    InteractiveFrameworkMethod::ActivitySetContentViewResource => {
                        let resource_id = match argument {
                            InteractiveRegister::ResourceId(id) => id,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        if opcode != 0x6e
                            || !super_created
                            || layout_resource_id.replace(resource_id).is_some()
                            || receiver != InteractiveRegister::Activity
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    InteractiveFrameworkMethod::ActivityFindViewById => {
                        let id = match argument {
                            InteractiveRegister::ResourceId(id) => id,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let (view_index, _) =
                            scene.find_by_id(id).ok_or(Error::DexFrameworkState)?;
                        if opcode != 0x6e
                            || layout_resource_id.is_none()
                            || receiver != InteractiveRegister::Activity
                            || pending_result
                                .replace(InteractiveRegister::ViewRef(view_index))
                                .is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    InteractiveFrameworkMethod::ViewSetOnClickListener => {
                        let view_index = match receiver {
                            InteractiveRegister::ViewRef(index) => index,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                        if opcode != 0x6e
                            || view.kind != ViewKind::Button
                            || view.id == 0
                            || argument != InteractiveRegister::Activity
                        {
                            return Err(Error::DexFrameworkState);
                        }
                        listeners.insert(view_index)?;
                    }
                    InteractiveFrameworkMethod::ViewGetId
                    | InteractiveFrameworkMethod::TextViewSetTextResource => {
                        return Err(Error::DexFrameworkState);
                    }
                    #[cfg(feature = "androidbox-string-text12")]
                    InteractiveFrameworkMethod::TextViewSetTextCharSequence => {
                        return Err(Error::DexFrameworkState);
                    }
                    #[cfg(feature = "androidbox-string-builder13")]
                    InteractiveFrameworkMethod::StringBuilderInitString
                    | InteractiveFrameworkMethod::StringBuilderAppendInt
                    | InteractiveFrameworkMethod::StringBuilderToString => {
                        return Err(Error::DexFrameworkState);
                    }
                }
                pc += 3;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

#[derive(Clone, Copy)]
struct CallbackMutation {
    target_view_index: u8,
    target_view_id: u32,
    text: InteractiveText,
    activity_field: Option<ActivityFieldValue>,
    instruction_count: u16,
    #[cfg(feature = "androidbox-dex-methods8")]
    app_defined_call_count: u8,
    #[cfg(feature = "androidbox-dex-instance9")]
    app_defined_instance_call_count: u8,
    #[cfg(feature = "androidbox-activity-fields10")]
    activity_field_read_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    activity_field_write_count: u8,
    #[cfg(feature = "androidbox-activity-state11")]
    activity_int_state_value: u8,
}

fn execute_on_click(
    bytes: &[u8],
    header: Header,
    method: ActivityMethod,
    scene: &ActivityScene,
    listeners: InteractiveListeners,
    retained_activity_field: Option<ActivityFieldValue>,
    clicked_view_index: u8,
) -> Result<CallbackMutation, Error> {
    let code = validate_code(
        bytes,
        header,
        method.info.code_offset,
        method.info.parameters,
        method.info.outgoing,
    )?;
    validate_interactive_callback_control_flow(bytes, code)?;
    let clicked_view = scene
        .node(clicked_view_index)
        .ok_or(Error::DexFrameworkState)?;
    if clicked_view.kind != ViewKind::Button
        || clicked_view.id == 0
        || !listeners.contains(clicked_view_index)
    {
        return Err(Error::DexFrameworkState);
    }
    let mut registers = [InteractiveRegister::Empty; MAX_REGISTERS as usize];
    let parameter_start = usize::from(code.registers - code.parameters);
    registers[parameter_start] = InteractiveRegister::Activity;
    registers[parameter_start + 1] = InteractiveRegister::ViewRef(clicked_view_index);
    let mut pending_result = None;
    let mut found_view = None;
    let mut checked_text_view = None;
    let mut mutation: Option<(u8, u32, InteractiveText)> = None;
    #[cfg(feature = "androidbox-dex-methods8")]
    let mut app_defined_call_count = 0u8;
    #[cfg(feature = "androidbox-dex-instance9")]
    let mut app_defined_instance_call_count = 0u8;
    #[cfg(feature = "androidbox-activity-fields10")]
    let mut activity_field_read_count = 0u8;
    #[cfg(feature = "androidbox-activity-fields10")]
    let mut text_field_read = false;
    #[cfg(feature = "androidbox-activity-state11")]
    let mut int_field_read = false;
    #[cfg(feature = "androidbox-activity-state11")]
    let mut activity_field_write_count = 0u8;
    #[cfg(feature = "androidbox-activity-state11")]
    let mut incremented_int = None;
    #[cfg(feature = "androidbox-string-builder13")]
    let mut string_builder_stage = 0u8;
    let mut next_activity_field = retained_activity_field;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        if steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        if pending_result.is_some() && !matches!(opcode, 0x0a | 0x0c) {
            return Err(Error::DexFrameworkState);
        }
        steps += 1;
        match opcode {
            0x0a | 0x0c => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                let result = pending_result.take().ok_or(Error::DexFrameworkState)?;
                let type_matches = match opcode {
                    0x0a => match result {
                        InteractiveRegister::Int(_) => true,
                        #[cfg(feature = "androidbox-dex-methods8")]
                        InteractiveRegister::ResourceId(_) => true,
                        _ => false,
                    },
                    0x0c => {
                        matches!(result, InteractiveRegister::ViewRef(_)) || {
                            #[cfg(feature = "androidbox-string-builder13")]
                            {
                                matches!(result, InteractiveRegister::BuiltString(_, _))
                            }
                            #[cfg(not(feature = "androidbox-string-builder13"))]
                            {
                                false
                            }
                        }
                    }
                    _ => false,
                };
                if !type_matches {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = result;
                pc += 1;
            }
            0x0e => {
                if first != 0x000e || pending_result.is_some() {
                    return Err(Error::DexFrameworkState);
                }
                #[cfg(feature = "androidbox-activity-fields10")]
                match retained_activity_field {
                    None if activity_field_read_count == 0 && !text_field_read => {}
                    Some(field) if text_field_read => {
                        #[cfg(not(feature = "androidbox-activity-state11"))]
                        if field.int_field_index.is_some() || activity_field_read_count != 1 {
                            return Err(Error::DexFrameworkState);
                        }
                        #[cfg(feature = "androidbox-activity-state11")]
                        match field.int_field_index {
                            None if activity_field_read_count == 1
                                && !int_field_read
                                && activity_field_write_count == 0 => {}
                            Some(_)
                                if activity_field_read_count == 2
                                    && int_field_read
                                    && activity_field_write_count == 1 => {}
                            _ => return Err(Error::DexFrameworkState),
                        }
                    }
                    _ => return Err(Error::DexFrameworkState),
                }
                let (target_view_index, target_view_id, text) =
                    mutation.ok_or(Error::DexFrameworkState)?;
                #[cfg(feature = "androidbox-string-builder13")]
                if (text.is_dynamic() && string_builder_stage != 4)
                    || (!text.is_dynamic() && string_builder_stage != 0)
                {
                    return Err(Error::DexFrameworkState);
                }
                #[cfg(feature = "androidbox-activity-state11")]
                let activity_int_state_value = match next_activity_field {
                    Some(field) if field.int_field_index.is_some() => {
                        u8::try_from(field.int_value).map_err(|_| Error::DexInstructionLimit)?
                    }
                    _ => 0,
                };
                return Ok(CallbackMutation {
                    target_view_index,
                    target_view_id,
                    text,
                    activity_field: next_activity_field,
                    instruction_count: steps,
                    #[cfg(feature = "androidbox-dex-methods8")]
                    app_defined_call_count,
                    #[cfg(feature = "androidbox-dex-instance9")]
                    app_defined_instance_call_count,
                    #[cfg(feature = "androidbox-activity-fields10")]
                    activity_field_read_count,
                    #[cfg(feature = "androidbox-activity-state11")]
                    activity_field_write_count,
                    #[cfg(feature = "androidbox-activity-state11")]
                    activity_int_state_value,
                });
            }
            0x14 | 0x15 => {
                let (destination, resource_id, width) = decode_resource_const(bytes, code, pc)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = InteractiveRegister::ResourceId(resource_id);
                pc = pc.checked_add(width).ok_or(Error::DexCode)?;
            }
            #[cfg(feature = "androidbox-string-text12")]
            0x12 => {
                let destination = usize::from((first >> 8) & 0x0f);
                ensure_register(code.registers, destination)?;
                let literal = i32::from((first as i16) >> 12);
                if literal != 1 || registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] =
                    InteractiveRegister::Int(u32::from_ne_bytes(literal.to_ne_bytes()));
                pc += 1;
            }
            #[cfg(feature = "androidbox-string-text12")]
            0x1a => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if registers[destination] == InteractiveRegister::Activity {
                    return Err(Error::DexFrameworkState);
                }
                let string_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                // Validate the bounded ASCII string now, but retain only its
                // DEX index in the register file. Copying a 256-byte value
                // into each of eight registers would needlessly grow the
                // protected EL0 callback stack.
                let _ = dex_string(bytes, header, string_index)?;
                registers[destination] = InteractiveRegister::StringIndex(string_index);
                pc += 2;
            }
            #[cfg(feature = "androidbox-string-builder13")]
            0x22 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                let type_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                if type_descriptor(bytes, header, type_index)? != STRING_BUILDER_CLASS
                    || registers[destination] != InteractiveRegister::Empty
                    || string_builder_stage != 0
                {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = InteractiveRegister::StringBuilderUninitialized;
                string_builder_stage = 1;
                pc += 2;
            }
            0x1f => {
                ensure_remaining(code, pc, 2)?;
                let register = usize::from(first >> 8);
                ensure_register(code.registers, register)?;
                let type_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let view_index = match registers[register] {
                    InteractiveRegister::ViewRef(index) => index,
                    _ => return Err(Error::DexFrameworkState),
                };
                if type_descriptor(bytes, header, type_index)? != TEXT_VIEW_CLASS
                    || scene.node(view_index).ok_or(Error::DexFrameworkState)?.kind
                        != ViewKind::TextView
                    || found_view != Some(view_index)
                    || checked_text_view.replace(view_index).is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                pc += 2;
            }
            0x32 | 0x33 => {
                pc = evaluate_interactive_callback_branch(
                    bytes, code, pc, opcode, &registers, scene, listeners,
                )?;
            }
            0x28 => {
                pc = interactive_callback_goto_target(bytes, code, pc)?;
            }
            #[cfg(feature = "androidbox-activity-fields10")]
            0x54 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from((first >> 8) & 0x0f);
                let receiver = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, receiver)?;
                let field_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let field = retained_activity_field.ok_or(Error::DexFrameworkState)?;
                let view = scene
                    .node(field.text_view_index)
                    .ok_or(Error::DexFrameworkState)?;
                if registers[receiver] != InteractiveRegister::Activity
                    || field.text_view_field_index != field_index
                    || view.kind != ViewKind::TextView
                    || view.id == 0
                    || text_field_read
                    || registers[destination] == InteractiveRegister::Activity
                {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = InteractiveRegister::ViewRef(field.text_view_index);
                checked_text_view = Some(field.text_view_index);
                text_field_read = true;
                activity_field_read_count = activity_field_read_count
                    .checked_add(1)
                    .ok_or(Error::DexFrameworkState)?;
                pc += 2;
            }
            #[cfg(feature = "androidbox-activity-state11")]
            0x52 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from((first >> 8) & 0x0f);
                let receiver = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, receiver)?;
                let field_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let field = retained_activity_field.ok_or(Error::DexFrameworkState)?;
                if registers[receiver] != InteractiveRegister::Activity
                    || field.int_field_index != Some(field_index)
                    || int_field_read
                    || field.int_value < 0
                    || field.int_value >= i32::from(u8::MAX)
                    || registers[destination] == InteractiveRegister::Activity
                {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] =
                    InteractiveRegister::Int(u32::from_ne_bytes(field.int_value.to_ne_bytes()));
                int_field_read = true;
                activity_field_read_count = activity_field_read_count
                    .checked_add(1)
                    .ok_or(Error::DexFrameworkState)?;
                pc += 2;
            }
            #[cfg(feature = "androidbox-activity-state11")]
            0x59 => {
                ensure_remaining(code, pc, 2)?;
                let value_register = usize::from((first >> 8) & 0x0f);
                let receiver = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, value_register, receiver)?;
                let field_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let value = match registers[value_register] {
                    InteractiveRegister::Int(value) => i32::from_ne_bytes(value.to_ne_bytes()),
                    _ => return Err(Error::DexFrameworkState),
                };
                let field = next_activity_field.ok_or(Error::DexFrameworkState)?;
                if registers[receiver] != InteractiveRegister::Activity
                    || field.int_field_index != Some(field_index)
                    || incremented_int != Some(value)
                    || activity_field_write_count != 0
                {
                    return Err(Error::DexFrameworkState);
                }
                next_activity_field = Some(ActivityFieldValue {
                    int_value: value,
                    ..field
                });
                activity_field_write_count = 1;
                pc += 2;
            }
            0x6e => {
                let invocation = decode_invoke(bytes, code, pc)?;
                let framework = resolve_interactive_framework_method(
                    bytes,
                    header,
                    invocation.method_index,
                    method,
                )?;
                match framework {
                    InteractiveFrameworkMethod::ViewGetId => {
                        if invocation.count != 1 {
                            return Err(Error::DexFrameworkState);
                        }
                        let view_index = match registers[invocation.registers[0]] {
                            InteractiveRegister::ViewRef(index) => index,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                        if view_index != clicked_view_index
                            || view.kind != ViewKind::Button
                            || view.id == 0
                            || !listeners.contains(view_index)
                            || pending_result
                                .replace(InteractiveRegister::Int(view.id))
                                .is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    InteractiveFrameworkMethod::ActivityFindViewById => {
                        if invocation.count != 2 {
                            return Err(Error::DexFrameworkState);
                        }
                        let receiver = registers[invocation.registers[0]];
                        let argument = registers[invocation.registers[1]];
                        let id = match argument {
                            InteractiveRegister::ResourceId(id) => id,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let (view_index, _) =
                            scene.find_by_id(id).ok_or(Error::DexFrameworkState)?;
                        if receiver != InteractiveRegister::Activity
                            || found_view.replace(view_index).is_some()
                            || pending_result
                                .replace(InteractiveRegister::ViewRef(view_index))
                                .is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    InteractiveFrameworkMethod::TextViewSetTextResource => {
                        if invocation.count != 2 {
                            return Err(Error::DexFrameworkState);
                        }
                        let receiver = registers[invocation.registers[0]];
                        let argument = registers[invocation.registers[1]];
                        let view_index = match receiver {
                            InteractiveRegister::ViewRef(index) => index,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                        let text_resource_id = match argument {
                            InteractiveRegister::ResourceId(id) => id,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        if view.kind != ViewKind::TextView
                            || view.id == 0
                            || checked_text_view != Some(view_index)
                            || mutation
                                .replace((
                                    view_index,
                                    view.id,
                                    InteractiveText::resource(text_resource_id),
                                ))
                                .is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    #[cfg(feature = "androidbox-string-text12")]
                    InteractiveFrameworkMethod::TextViewSetTextCharSequence => {
                        if invocation.count != 2 {
                            return Err(Error::DexFrameworkState);
                        }
                        let receiver = registers[invocation.registers[0]];
                        let argument = registers[invocation.registers[1]];
                        let view_index = match receiver {
                            InteractiveRegister::ViewRef(index) => index,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                        let text = match argument {
                            InteractiveRegister::StringIndex(string_index) => {
                                InteractiveText::direct(ResourceString::from_dex_ascii(
                                    dex_string(bytes, header, string_index)?,
                                )?)
                            }
                            #[cfg(feature = "androidbox-string-builder13")]
                            InteractiveRegister::BuiltString(string_index, value) => {
                                if string_builder_stage != 4 {
                                    return Err(Error::DexFrameworkState);
                                }
                                InteractiveText::dynamic(
                                    ResourceString::from_dex_ascii_with_u8_suffix(
                                        dex_string(bytes, header, string_index)?,
                                        value,
                                    )?,
                                )
                            }
                            _ => return Err(Error::DexFrameworkState),
                        };
                        if view.kind != ViewKind::TextView
                            || view.id == 0
                            || checked_text_view != Some(view_index)
                            || mutation.replace((view_index, view.id, text)).is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                    }
                    #[cfg(feature = "androidbox-string-builder13")]
                    InteractiveFrameworkMethod::StringBuilderAppendInt => {
                        if invocation.count != 2 || string_builder_stage != 2 {
                            return Err(Error::DexFrameworkState);
                        }
                        let receiver_register = invocation.registers[0];
                        let (string_index, value) = match (
                            registers[receiver_register],
                            registers[invocation.registers[1]],
                        ) {
                            (
                                InteractiveRegister::StringBuilderPrefix(string_index),
                                InteractiveRegister::Int(value),
                            ) => (string_index, value),
                            _ => return Err(Error::DexFrameworkState),
                        };
                        let value = u8::try_from(value).map_err(|_| Error::DexFrameworkState)?;
                        if value == 0
                            || incremented_int != Some(i32::from(value))
                            || pending_result.is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                        registers[receiver_register] =
                            InteractiveRegister::StringBuilderInt(string_index, value);
                        string_builder_stage = 3;
                    }
                    #[cfg(feature = "androidbox-string-builder13")]
                    InteractiveFrameworkMethod::StringBuilderToString => {
                        if invocation.count != 1 || string_builder_stage != 3 {
                            return Err(Error::DexFrameworkState);
                        }
                        let (string_index, value) = match registers[invocation.registers[0]] {
                            InteractiveRegister::StringBuilderInt(string_index, value) => {
                                (string_index, value)
                            }
                            _ => return Err(Error::DexFrameworkState),
                        };
                        if pending_result
                            .replace(InteractiveRegister::BuiltString(string_index, value))
                            .is_some()
                        {
                            return Err(Error::DexFrameworkState);
                        }
                        string_builder_stage = 4;
                    }
                    #[cfg(feature = "androidbox-string-builder13")]
                    InteractiveFrameworkMethod::StringBuilderInitString => {
                        return Err(Error::DexFrameworkState);
                    }
                    _ => return Err(Error::DexFrameworkState),
                }
                pc += 3;
            }
            #[cfg(feature = "androidbox-dex-instance9")]
            0x70 => {
                let invocation = decode_invoke(bytes, code, pc)?;
                #[cfg(feature = "androidbox-string-builder13")]
                match resolve_interactive_framework_method(
                    bytes,
                    header,
                    invocation.method_index,
                    method,
                ) {
                    Ok(InteractiveFrameworkMethod::StringBuilderInitString) => {
                        if invocation.count != 2 || string_builder_stage != 1 {
                            return Err(Error::DexFrameworkState);
                        }
                        let receiver_register = invocation.registers[0];
                        let string_index = match registers[invocation.registers[1]] {
                            InteractiveRegister::StringIndex(index) => index,
                            _ => return Err(Error::DexFrameworkState),
                        };
                        if registers[receiver_register]
                            != InteractiveRegister::StringBuilderUninitialized
                        {
                            return Err(Error::DexFrameworkState);
                        }
                        registers[receiver_register] =
                            InteractiveRegister::StringBuilderPrefix(string_index);
                        string_builder_stage = 2;
                        pc += 3;
                        continue;
                    }
                    Ok(_) => return Err(Error::DexFrameworkState),
                    Err(Error::DexFrameworkUnknownMethod) => {}
                    Err(error) => return Err(error),
                }
                if invocation.count != 2
                    || app_defined_call_count != 0
                    || app_defined_instance_call_count != 0
                    || registers[invocation.registers[0]] != InteractiveRegister::Activity
                {
                    return Err(Error::DexFrameworkState);
                }
                let argument_view_index = match registers[invocation.registers[1]] {
                    InteractiveRegister::ViewRef(index) => index,
                    _ => return Err(Error::DexFrameworkState),
                };
                if argument_view_index != clicked_view_index {
                    return Err(Error::DexFrameworkState);
                }
                let helper = resolve_instance_callback_helper(
                    bytes,
                    header,
                    invocation.method_index,
                    method.class_type_index,
                )?;
                let (result, helper_steps) = execute_instance_callback_helper(
                    bytes,
                    header,
                    helper,
                    scene,
                    clicked_view_index,
                )?;
                if result == 0
                    || pending_result
                        .replace(InteractiveRegister::ResourceId(result))
                        .is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                steps = steps
                    .checked_add(helper_steps)
                    .filter(|total| *total <= MAX_EXECUTED_INSTRUCTIONS)
                    .ok_or(Error::DexInstructionLimit)?;
                app_defined_call_count = 1;
                app_defined_instance_call_count = 1;
                pc += 3;
            }
            #[cfg(feature = "androidbox-activity-state11")]
            0xd8 => {
                ensure_remaining(code, pc, 2)?;
                let destination = usize::from(first >> 8);
                let operands = instruction_unit(bytes, code, pc + 1)?;
                let source = usize::from(operands as u8);
                let literal = i32::from((operands >> 8) as u8 as i8);
                ensure_registers(code.registers, destination, source)?;
                let source_value = match registers[source] {
                    InteractiveRegister::Int(value) => i32::from_ne_bytes(value.to_ne_bytes()),
                    _ => return Err(Error::DexFrameworkState),
                };
                let field = retained_activity_field.ok_or(Error::DexFrameworkState)?;
                let value = source_value
                    .checked_add(literal)
                    .ok_or(Error::DexInstructionLimit)?;
                if literal != 1
                    || field.int_field_index.is_none()
                    || source_value != field.int_value
                    || incremented_int.replace(value).is_some()
                    || registers[destination] == InteractiveRegister::Activity
                {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] =
                    InteractiveRegister::Int(u32::from_ne_bytes(value.to_ne_bytes()));
                pc += 2;
            }
            #[cfg(feature = "androidbox-string-text12")]
            0xb0 => {
                let destination = usize::from((first >> 8) & 0x0f);
                let source = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, source)?;
                let destination_value = match registers[destination] {
                    InteractiveRegister::Int(value) => i32::from_ne_bytes(value.to_ne_bytes()),
                    _ => return Err(Error::DexFrameworkState),
                };
                let source_value = match registers[source] {
                    InteractiveRegister::Int(value) => i32::from_ne_bytes(value.to_ne_bytes()),
                    _ => return Err(Error::DexFrameworkState),
                };
                let field = retained_activity_field.ok_or(Error::DexFrameworkState)?;
                let value = destination_value
                    .checked_add(source_value)
                    .ok_or(Error::DexInstructionLimit)?;
                if source_value != 1
                    || field.int_field_index.is_none()
                    || destination_value != field.int_value
                    || incremented_int.replace(value).is_some()
                    || registers[destination] == InteractiveRegister::Activity
                {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] =
                    InteractiveRegister::Int(u32::from_ne_bytes(value.to_ne_bytes()));
                pc += 1;
            }
            #[cfg(feature = "androidbox-dex-methods8")]
            0x71 => {
                let invocation = decode_invoke(bytes, code, pc)?;
                if invocation.count != 1 || app_defined_call_count != 0 {
                    return Err(Error::DexFrameworkState);
                }
                let argument = match registers[invocation.registers[0]] {
                    InteractiveRegister::Int(value) => value,
                    _ => return Err(Error::DexFrameworkState),
                };
                let helper = resolve_callback_helper(
                    bytes,
                    header,
                    invocation.method_index,
                    method.class_type_index,
                )?;
                let (result, helper_steps) = execute_callback_helper(
                    bytes,
                    helper,
                    i32::from_ne_bytes(argument.to_ne_bytes()),
                )?;
                if result == 0
                    || pending_result
                        .replace(InteractiveRegister::ResourceId(result))
                        .is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                steps = steps
                    .checked_add(helper_steps)
                    .filter(|total| *total <= MAX_EXECUTED_INSTRUCTIONS)
                    .ok_or(Error::DexInstructionLimit)?;
                app_defined_call_count = 1;
                pc += 3;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

fn validate_interactive_callback_control_flow(bytes: &[u8], code: Code) -> Result<(), Error> {
    let mut instruction_starts = 0u128;
    let mut pc = 0u32;
    while pc < code.units {
        instruction_starts |= 1u128.checked_shl(pc).ok_or(Error::DexInstructionLimit)?;
        pc = pc
            .checked_add(interactive_callback_instruction_width(bytes, code, pc)?)
            .ok_or(Error::DexCode)?;
    }
    if pc != code.units {
        return Err(Error::DexCode);
    }

    // Prove that every encoded instruction belongs to the entry-point CFG.
    // d8 may fold two setText arms into one shared tail using a backward
    // `goto`; the fixed graph below accepts that shape only when the complete
    // CFG remains acyclic.
    let mut reachable = 0u128;
    let mut queued = 0u128;
    let mut queue = [0u8; MAX_INSTRUCTION_UNITS as usize];
    let mut head = 0usize;
    let mut tail = 1usize;
    queue[0] = 0;
    queued |= 1;
    let mut returns = 0u8;
    while head < tail {
        let current = u32::from(queue[head]);
        head += 1;
        let bit = 1u128
            .checked_shl(current)
            .ok_or(Error::DexInstructionLimit)?;
        reachable |= bit;
        let opcode = instruction_unit(bytes, code, current)? as u8;
        if opcode == 0x0e {
            returns = returns.checked_add(1).ok_or(Error::DexCode)?;
            continue;
        }
        let width = interactive_callback_instruction_width(bytes, code, current)?;
        let fallthrough = current.checked_add(width).ok_or(Error::DexCode)?;
        if opcode == 0x28 {
            let target = interactive_callback_goto_target(bytes, code, current)?;
            if instruction_starts
                & 1u128
                    .checked_shl(target)
                    .ok_or(Error::DexInstructionLimit)?
                == 0
            {
                return Err(Error::DexCode);
            }
            enqueue_callback_pc(target, &mut queue, &mut tail, &mut queued)?;
            continue;
        }
        if fallthrough >= code.units
            || instruction_starts
                & 1u128
                    .checked_shl(fallthrough)
                    .ok_or(Error::DexInstructionLimit)?
                == 0
        {
            return Err(Error::DexNoReturn);
        }
        enqueue_callback_pc(fallthrough, &mut queue, &mut tail, &mut queued)?;
        if matches!(opcode, 0x32 | 0x33) {
            let target = interactive_callback_branch_target(bytes, code, current)?;
            if instruction_starts
                & 1u128
                    .checked_shl(target)
                    .ok_or(Error::DexInstructionLimit)?
                == 0
            {
                return Err(Error::DexCode);
            }
            enqueue_callback_pc(target, &mut queue, &mut tail, &mut queued)?;
        }
    }
    if returns == 0 || reachable != instruction_starts {
        return Err(Error::DexNoReturn);
    }
    validate_interactive_callback_cfg_is_acyclic(bytes, code, instruction_starts)?;
    Ok(())
}

fn validate_interactive_callback_cfg_is_acyclic(
    bytes: &[u8],
    code: Code,
    instruction_starts: u128,
) -> Result<(), Error> {
    let mut indegree = [0u8; MAX_INSTRUCTION_UNITS as usize];
    let mut pc = 0u32;
    while pc < code.units {
        for successor in interactive_callback_successors(bytes, code, pc)?
            .into_iter()
            .flatten()
        {
            let slot = indegree
                .get_mut(usize::try_from(successor).map_err(|_| Error::DexInstructionLimit)?)
                .ok_or(Error::DexInstructionLimit)?;
            *slot = slot.checked_add(1).ok_or(Error::DexCode)?;
        }
        pc = pc
            .checked_add(interactive_callback_instruction_width(bytes, code, pc)?)
            .ok_or(Error::DexCode)?;
    }

    let mut queue = [0u8; MAX_INSTRUCTION_UNITS as usize];
    let mut head = 0usize;
    let mut tail = 0usize;
    pc = 0;
    while pc < code.units {
        if indegree[usize::try_from(pc).map_err(|_| Error::DexInstructionLimit)?] == 0 {
            queue[tail] = u8::try_from(pc).map_err(|_| Error::DexInstructionLimit)?;
            tail = tail.checked_add(1).ok_or(Error::DexInstructionLimit)?;
        }
        pc = pc
            .checked_add(interactive_callback_instruction_width(bytes, code, pc)?)
            .ok_or(Error::DexCode)?;
    }

    let mut processed = 0u32;
    while head < tail {
        let current = u32::from(queue[head]);
        head += 1;
        processed = processed.checked_add(1).ok_or(Error::DexCode)?;
        for successor in interactive_callback_successors(bytes, code, current)?
            .into_iter()
            .flatten()
        {
            let slot = indegree
                .get_mut(usize::try_from(successor).map_err(|_| Error::DexInstructionLimit)?)
                .ok_or(Error::DexInstructionLimit)?;
            *slot = slot.checked_sub(1).ok_or(Error::DexCode)?;
            if *slot == 0 {
                let queued = queue.get_mut(tail).ok_or(Error::DexInstructionLimit)?;
                *queued = u8::try_from(successor).map_err(|_| Error::DexInstructionLimit)?;
                tail = tail.checked_add(1).ok_or(Error::DexInstructionLimit)?;
            }
        }
    }
    if processed != instruction_starts.count_ones() {
        return Err(Error::DexCode);
    }
    Ok(())
}

fn interactive_callback_successors(
    bytes: &[u8],
    code: Code,
    pc: u32,
) -> Result<[Option<u32>; 2], Error> {
    let opcode = instruction_unit(bytes, code, pc)? as u8;
    if opcode == 0x0e {
        return Ok([None, None]);
    }
    if opcode == 0x28 {
        return Ok([
            Some(interactive_callback_goto_target(bytes, code, pc)?),
            None,
        ]);
    }
    let fallthrough = pc
        .checked_add(interactive_callback_instruction_width(bytes, code, pc)?)
        .ok_or(Error::DexCode)?;
    if fallthrough >= code.units {
        return Err(Error::DexNoReturn);
    }
    let branch = if matches!(opcode, 0x32 | 0x33) {
        Some(interactive_callback_branch_target(bytes, code, pc)?)
    } else {
        None
    };
    Ok([Some(fallthrough), branch])
}

fn enqueue_callback_pc(
    pc: u32,
    queue: &mut [u8; MAX_INSTRUCTION_UNITS as usize],
    tail: &mut usize,
    queued: &mut u128,
) -> Result<(), Error> {
    let bit = 1u128.checked_shl(pc).ok_or(Error::DexInstructionLimit)?;
    if *queued & bit != 0 {
        return Ok(());
    }
    let slot = queue.get_mut(*tail).ok_or(Error::DexInstructionLimit)?;
    *slot = u8::try_from(pc).map_err(|_| Error::DexInstructionLimit)?;
    *tail = tail.checked_add(1).ok_or(Error::DexInstructionLimit)?;
    *queued |= bit;
    Ok(())
}

fn interactive_callback_instruction_width(bytes: &[u8], code: Code, pc: u32) -> Result<u32, Error> {
    let opcode = instruction_unit(bytes, code, pc)? as u8;
    let width = match opcode {
        0x0a | 0x0c | 0x0e | 0x28 => 1,
        #[cfg(feature = "androidbox-string-text12")]
        0x12 | 0xb0 => 1,
        0x14 | 0x6e => 3,
        #[cfg(feature = "androidbox-dex-instance9")]
        0x70 => 3,
        #[cfg(feature = "androidbox-dex-methods8")]
        0x71 => 3,
        0x15 | 0x1f | 0x32 | 0x33 => 2,
        #[cfg(feature = "androidbox-string-text12")]
        0x1a => 2,
        #[cfg(feature = "androidbox-string-builder13")]
        0x22 => 2,
        #[cfg(feature = "androidbox-activity-fields10")]
        0x54 => 2,
        #[cfg(feature = "androidbox-activity-state11")]
        0x52 | 0x59 | 0xd8 => 2,
        _ => return Err(Error::DexUnknownOpcode(opcode)),
    };
    ensure_remaining(code, pc, width)?;
    Ok(width)
}

fn interactive_callback_branch_target(bytes: &[u8], code: Code, pc: u32) -> Result<u32, Error> {
    ensure_remaining(code, pc, 2)?;
    let offset = i32::from(instruction_unit(bytes, code, pc + 1)? as i16);
    if offset <= 0 {
        return Err(Error::DexCode);
    }
    let target = pc
        .checked_add(u32::try_from(offset).map_err(|_| Error::DexCode)?)
        .ok_or(Error::DexCode)?;
    if target >= code.units {
        return Err(Error::DexCode);
    }
    Ok(target)
}

fn interactive_callback_goto_target(bytes: &[u8], code: Code, pc: u32) -> Result<u32, Error> {
    let first = instruction_unit(bytes, code, pc)?;
    if first as u8 != 0x28 {
        return Err(Error::DexUnknownOpcode(first as u8));
    }
    let offset = i32::from((first >> 8) as u8 as i8);
    if offset == 0 {
        return Err(Error::DexCode);
    }
    let target = i64::from(pc)
        .checked_add(i64::from(offset))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(Error::DexCode)?;
    if target >= code.units {
        return Err(Error::DexCode);
    }
    Ok(target)
}

fn evaluate_interactive_callback_branch(
    bytes: &[u8],
    code: Code,
    pc: u32,
    opcode: u8,
    registers: &[InteractiveRegister; MAX_REGISTERS as usize],
    scene: &ActivityScene,
    listeners: InteractiveListeners,
) -> Result<u32, Error> {
    let first = instruction_unit(bytes, code, pc)?;
    let left_index = usize::from((first >> 8) & 0x0f);
    let right_index = usize::from((first >> 12) & 0x0f);
    ensure_registers(code.registers, left_index, right_index)?;
    #[cfg(feature = "androidbox-string-text12")]
    if let (InteractiveRegister::Int(left), InteractiveRegister::Int(right)) =
        (registers[left_index], registers[right_index])
    {
        let equal = left == right;
        let take = match opcode {
            0x32 => equal,
            0x33 => !equal,
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        };
        return if take {
            interactive_callback_branch_target(bytes, code, pc)
        } else {
            pc.checked_add(2).ok_or(Error::DexCode)
        };
    }
    let (clicked_id, candidate_id) = match (registers[left_index], registers[right_index]) {
        (InteractiveRegister::Int(clicked), InteractiveRegister::ResourceId(candidate))
        | (InteractiveRegister::ResourceId(candidate), InteractiveRegister::Int(clicked)) => {
            (clicked, candidate)
        }
        _ => return Err(Error::DexFrameworkState),
    };
    let (candidate_index, candidate) = scene
        .find_by_id(candidate_id)
        .ok_or(Error::DexFrameworkState)?;
    if candidate.kind != ViewKind::Button
        || candidate.id == 0
        || !listeners.contains(candidate_index)
    {
        return Err(Error::DexFrameworkState);
    }
    let equal = clicked_id == candidate_id;
    let take = match opcode {
        0x32 => equal,
        0x33 => !equal,
        _ => return Err(Error::DexUnknownOpcode(opcode)),
    };
    if take {
        interactive_callback_branch_target(bytes, code, pc)
    } else {
        pc.checked_add(2).ok_or(Error::DexCode)
    }
}

#[cfg(feature = "androidbox-dex-methods8")]
fn resolve_callback_helper(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    activity_class: u32,
) -> Result<Code, Error> {
    if method_index >= header.method_ids.size
        || u32::from(read_table_u16(
            bytes,
            header.method_ids,
            8,
            method_index,
            0,
        )?) != activity_class
        || !method_proto_matches(bytes, header, method_index, INT_CLASS, INT_CLASS)?
    {
        return Err(Error::DexFrameworkUnknownMethod);
    }

    let mut class_data_offset = None;
    for index in 0..header.class_defs.size {
        let base = table_offset(header.class_defs, 32, index)?;
        if read_u32(bytes, base)? == activity_class {
            if class_data_offset.is_some() {
                return Err(Error::DexActivityClassDuplicate);
            }
            let offset = read_u32(bytes, base + 24)?;
            if offset == 0 {
                return Err(Error::DexActivityClassInvalid);
            }
            class_data_offset = Some(offset);
        }
    }
    let mut cursor = usize_from(class_data_offset.ok_or(Error::DexActivityClassMissing)?)?;
    let static_fields = read_uleb128(bytes, &mut cursor)?;
    let instance_fields = read_uleb128(bytes, &mut cursor)?;
    let direct_methods = read_uleb128(bytes, &mut cursor)?;
    let virtual_methods = read_uleb128(bytes, &mut cursor)?;
    for count in [
        static_fields,
        instance_fields,
        direct_methods,
        virtual_methods,
    ] {
        if count > MAX_CLASS_MEMBERS {
            return Err(Error::DexCode);
        }
    }
    skip_encoded_fields(bytes, &mut cursor, static_fields, header.field_ids.size)?;
    skip_encoded_fields(bytes, &mut cursor, instance_fields, header.field_ids.size)?;

    let mut found = None;
    let mut accumulated = 0u32;
    for _ in 0..direct_methods {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, &mut cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        let access_flags = read_uleb128(bytes, &mut cursor)?;
        let code_offset = read_uleb128(bytes, &mut cursor)?;
        if accumulated == method_index {
            if access_flags != (ACC_PRIVATE | ACC_STATIC) || code_offset == 0 {
                return Err(Error::DexMethodFlags);
            }
            if found.replace(code_offset).is_some() {
                return Err(Error::DexMethodDuplicate);
            }
        }
    }
    accumulated = 0;
    for _ in 0..virtual_methods {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, &mut cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        read_uleb128(bytes, &mut cursor)?;
        read_uleb128(bytes, &mut cursor)?;
        if accumulated == method_index {
            return Err(Error::DexMethodFlags);
        }
    }

    let code = validate_code(bytes, header, found.ok_or(Error::DexMethodMissing)?, 1, 0)?;
    validate_callback_helper_control_flow(bytes, code)?;
    Ok(code)
}

#[cfg(feature = "androidbox-dex-methods8")]
fn execute_callback_helper(bytes: &[u8], code: Code, input: i32) -> Result<(u32, u16), Error> {
    validate_callback_helper_control_flow(bytes, code)?;
    let mut registers = [0i32; MAX_REGISTERS as usize];
    registers[usize::from(code.registers - code.parameters)] = input;
    let mut visited = 0u128;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        let bit = 1u128.checked_shl(pc).ok_or(Error::DexInstructionLimit)?;
        if visited & bit != 0 || steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        visited |= bit;
        steps += 1;
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        match opcode {
            0x01 => {
                let destination = usize::from((first >> 8) & 0x0f);
                let source = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, source)?;
                registers[destination] = registers[source];
                pc += 1;
            }
            0x0f => {
                let register = usize::from(first >> 8);
                ensure_register(code.registers, register)?;
                return Ok((
                    u32::try_from(registers[register]).map_err(|_| Error::DexFrameworkState)?,
                    steps,
                ));
            }
            0x12 => {
                let destination = usize::from((first >> 8) & 0x0f);
                ensure_register(code.registers, destination)?;
                registers[destination] = i32::from((first as i16) >> 12);
                pc += 1;
            }
            0x13 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] = i32::from(instruction_unit(bytes, code, pc + 1)? as i16);
                pc += 2;
            }
            0x14 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                let low = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let high = u32::from(instruction_unit(bytes, code, pc + 2)?);
                registers[destination] = (low | (high << 16)) as i32;
                pc += 3;
            }
            0x15 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] =
                    i32::from(instruction_unit(bytes, code, pc + 1)? as i16) << 16;
                pc += 2;
            }
            0x28 => {
                pc = interactive_callback_goto_target(bytes, code, pc)?;
            }
            0x32 | 0x33 => {
                let left = usize::from((first >> 8) & 0x0f);
                let right = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, left, right)?;
                let equal = registers[left] == registers[right];
                let take = if opcode == 0x32 { equal } else { !equal };
                pc = if take {
                    interactive_callback_branch_target(bytes, code, pc)?
                } else {
                    pc.checked_add(2).ok_or(Error::DexCode)?
                };
            }
            0xd8 => {
                let destination = usize::from(first >> 8);
                let operands = instruction_unit(bytes, code, pc + 1)?;
                let source = usize::from(operands as u8);
                let literal = i32::from((operands >> 8) as u8 as i8);
                ensure_registers(code.registers, destination, source)?;
                registers[destination] = registers[source].wrapping_add(literal);
                pc += 2;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

#[cfg(feature = "androidbox-dex-instance9")]
fn resolve_instance_callback_helper(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    activity_class: u32,
) -> Result<InstanceCallbackHelper, Error> {
    if method_index >= header.method_ids.size
        || u32::from(read_table_u16(
            bytes,
            header.method_ids,
            8,
            method_index,
            0,
        )?) != activity_class
        || !method_proto_matches(bytes, header, method_index, INT_CLASS, VIEW_CLASS)?
    {
        return Err(Error::DexFrameworkUnknownMethod);
    }

    let mut class_data_offset = None;
    for index in 0..header.class_defs.size {
        let base = table_offset(header.class_defs, 32, index)?;
        if read_u32(bytes, base)? == activity_class {
            if class_data_offset.is_some() {
                return Err(Error::DexActivityClassDuplicate);
            }
            let offset = read_u32(bytes, base + 24)?;
            if offset == 0 {
                return Err(Error::DexActivityClassInvalid);
            }
            class_data_offset = Some(offset);
        }
    }
    let mut cursor = usize_from(class_data_offset.ok_or(Error::DexActivityClassMissing)?)?;
    let static_fields = read_uleb128(bytes, &mut cursor)?;
    let instance_fields = read_uleb128(bytes, &mut cursor)?;
    let direct_methods = read_uleb128(bytes, &mut cursor)?;
    let virtual_methods = read_uleb128(bytes, &mut cursor)?;
    for count in [
        static_fields,
        instance_fields,
        direct_methods,
        virtual_methods,
    ] {
        if count > MAX_CLASS_MEMBERS {
            return Err(Error::DexCode);
        }
    }
    skip_encoded_fields(bytes, &mut cursor, static_fields, header.field_ids.size)?;
    skip_encoded_fields(bytes, &mut cursor, instance_fields, header.field_ids.size)?;

    let mut found = None;
    let mut accumulated = 0u32;
    for _ in 0..direct_methods {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, &mut cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        let access_flags = read_uleb128(bytes, &mut cursor)?;
        let code_offset = read_uleb128(bytes, &mut cursor)?;
        if accumulated == method_index {
            if access_flags != ACC_PRIVATE || code_offset == 0 {
                return Err(Error::DexMethodFlags);
            }
            if found.replace(code_offset).is_some() {
                return Err(Error::DexMethodDuplicate);
            }
        }
    }
    accumulated = 0;
    for _ in 0..virtual_methods {
        accumulated = accumulated
            .checked_add(read_uleb128(bytes, &mut cursor)?)
            .ok_or(Error::DexBounds)?;
        if accumulated >= header.method_ids.size {
            return Err(Error::DexBounds);
        }
        read_uleb128(bytes, &mut cursor)?;
        read_uleb128(bytes, &mut cursor)?;
        if accumulated == method_index {
            return Err(Error::DexMethodFlags);
        }
    }

    let code = validate_code(bytes, header, found.ok_or(Error::DexMethodMissing)?, 2, 1)?;
    validate_instance_callback_helper_control_flow(bytes, code)?;
    Ok(InstanceCallbackHelper {
        method: ActivityMethod {
            info: ActivityMethodInfo {
                method_index,
                code_offset: found.ok_or(Error::DexMethodMissing)?,
                registers: code.registers,
                parameters: code.parameters,
                outgoing: code.outgoing,
                instruction_units: code.units,
            },
            class_type_index: activity_class,
        },
        code,
    })
}

#[cfg(feature = "androidbox-dex-instance9")]
#[derive(Clone, Copy, Eq, PartialEq)]
enum InstanceHelperRegister {
    Empty,
    Activity,
    ViewRef(u8),
    Scalar(u32),
}

#[cfg(feature = "androidbox-dex-instance9")]
fn execute_instance_callback_helper(
    bytes: &[u8],
    header: Header,
    helper: InstanceCallbackHelper,
    scene: &ActivityScene,
    clicked_view_index: u8,
) -> Result<(u32, u16), Error> {
    validate_instance_callback_helper_control_flow(bytes, helper.code)?;
    let code = helper.code;
    let clicked_view = scene
        .node(clicked_view_index)
        .ok_or(Error::DexFrameworkState)?;
    if clicked_view.kind != ViewKind::Button || clicked_view.id == 0 {
        return Err(Error::DexFrameworkState);
    }
    let mut registers = [InstanceHelperRegister::Empty; MAX_REGISTERS as usize];
    let parameter_start = usize::from(code.registers - code.parameters);
    registers[parameter_start] = InstanceHelperRegister::Activity;
    registers[parameter_start + 1] = InstanceHelperRegister::ViewRef(clicked_view_index);
    let mut pending_result = None;
    let mut framework_call_count = 0u8;
    let mut visited = 0u128;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        let bit = 1u128.checked_shl(pc).ok_or(Error::DexInstructionLimit)?;
        if visited & bit != 0 || steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        visited |= bit;
        steps += 1;
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        if pending_result.is_some() && opcode != 0x0a {
            return Err(Error::DexFrameworkState);
        }
        match opcode {
            0x01 => {
                let destination = usize::from((first >> 8) & 0x0f);
                let source = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, destination, source)?;
                if registers[source] == InstanceHelperRegister::Empty {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = registers[source];
                pc += 1;
            }
            0x0a => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] = InstanceHelperRegister::Scalar(
                    pending_result.take().ok_or(Error::DexFrameworkState)?,
                );
                pc += 1;
            }
            0x0f => {
                let register = usize::from(first >> 8);
                ensure_register(code.registers, register)?;
                let result = match registers[register] {
                    InstanceHelperRegister::Scalar(value) if value != 0 => value,
                    _ => return Err(Error::DexFrameworkState),
                };
                return Ok((result, steps));
            }
            0x12 => {
                let destination = usize::from((first >> 8) & 0x0f);
                ensure_register(code.registers, destination)?;
                registers[destination] = InstanceHelperRegister::Scalar(u32::from_ne_bytes(
                    i32::from((first as i16) >> 12).to_ne_bytes(),
                ));
                pc += 1;
            }
            0x13 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] = InstanceHelperRegister::Scalar(u32::from_ne_bytes(
                    i32::from(instruction_unit(bytes, code, pc + 1)? as i16).to_ne_bytes(),
                ));
                pc += 2;
            }
            0x14 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                let low = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let high = u32::from(instruction_unit(bytes, code, pc + 2)?);
                registers[destination] = InstanceHelperRegister::Scalar(low | (high << 16));
                pc += 3;
            }
            0x15 => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                registers[destination] = InstanceHelperRegister::Scalar(
                    u32::from(instruction_unit(bytes, code, pc + 1)?) << 16,
                );
                pc += 2;
            }
            0x28 => {
                pc = interactive_callback_goto_target(bytes, code, pc)?;
            }
            0x32 | 0x33 => {
                let left = usize::from((first >> 8) & 0x0f);
                let right = usize::from((first >> 12) & 0x0f);
                ensure_registers(code.registers, left, right)?;
                let (left, right) = match (registers[left], registers[right]) {
                    (
                        InstanceHelperRegister::Scalar(left),
                        InstanceHelperRegister::Scalar(right),
                    ) => (left, right),
                    _ => return Err(Error::DexFrameworkState),
                };
                let equal = left == right;
                let take = if opcode == 0x32 { equal } else { !equal };
                pc = if take {
                    interactive_callback_branch_target(bytes, code, pc)?
                } else {
                    pc.checked_add(2).ok_or(Error::DexCode)?
                };
            }
            0x6e => {
                let invocation = decode_invoke(bytes, code, pc)?;
                if invocation.count != 1
                    || framework_call_count != 0
                    || !matches!(
                        resolve_interactive_framework_method(
                            bytes,
                            header,
                            invocation.method_index,
                            helper.method,
                        )?,
                        InteractiveFrameworkMethod::ViewGetId
                    )
                {
                    return Err(Error::DexFrameworkState);
                }
                let view_index = match registers[invocation.registers[0]] {
                    InstanceHelperRegister::ViewRef(index) => index,
                    _ => return Err(Error::DexFrameworkState),
                };
                let view = scene.node(view_index).ok_or(Error::DexFrameworkState)?;
                if view_index != clicked_view_index
                    || view.kind != ViewKind::Button
                    || view.id == 0
                    || pending_result.replace(view.id).is_some()
                {
                    return Err(Error::DexFrameworkState);
                }
                framework_call_count = 1;
                pc += 3;
            }
            0xd8 => {
                let destination = usize::from(first >> 8);
                let operands = instruction_unit(bytes, code, pc + 1)?;
                let source = usize::from(operands as u8);
                let literal = i32::from((operands >> 8) as u8 as i8);
                ensure_registers(code.registers, destination, source)?;
                let source = match registers[source] {
                    InstanceHelperRegister::Scalar(value) => value,
                    _ => return Err(Error::DexFrameworkState),
                };
                registers[destination] =
                    InstanceHelperRegister::Scalar(source.wrapping_add_signed(literal));
                pc += 2;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

#[cfg(feature = "androidbox-dex-methods8")]
fn validate_callback_helper_control_flow(bytes: &[u8], code: Code) -> Result<(), Error> {
    validate_callback_helper_control_flow_profile(bytes, code, CallbackHelperProfile::StaticInt)
}

#[cfg(feature = "androidbox-dex-instance9")]
fn validate_instance_callback_helper_control_flow(bytes: &[u8], code: Code) -> Result<(), Error> {
    validate_callback_helper_control_flow_profile(bytes, code, CallbackHelperProfile::InstanceView)
}

#[cfg(feature = "androidbox-dex-methods8")]
#[derive(Clone, Copy, Eq, PartialEq)]
enum CallbackHelperProfile {
    StaticInt,
    #[cfg(feature = "androidbox-dex-instance9")]
    InstanceView,
}

#[cfg(feature = "androidbox-dex-methods8")]
fn validate_callback_helper_control_flow_profile(
    bytes: &[u8],
    code: Code,
    profile: CallbackHelperProfile,
) -> Result<(), Error> {
    let mut instruction_starts = 0u128;
    let mut pc = 0u32;
    while pc < code.units {
        instruction_starts |= 1u128.checked_shl(pc).ok_or(Error::DexInstructionLimit)?;
        pc = pc
            .checked_add(callback_helper_instruction_width(bytes, code, pc, profile)?)
            .ok_or(Error::DexCode)?;
    }
    if pc != code.units {
        return Err(Error::DexCode);
    }

    let mut reachable = 0u128;
    let mut queued = 1u128;
    let mut queue = [0u8; MAX_INSTRUCTION_UNITS as usize];
    let mut head = 0usize;
    let mut tail = 1usize;
    let mut returns = 0u8;
    while head < tail {
        let current = u32::from(queue[head]);
        head += 1;
        reachable |= 1u128
            .checked_shl(current)
            .ok_or(Error::DexInstructionLimit)?;
        let successors = callback_helper_successors(bytes, code, current, profile)?;
        if successors == [None, None] {
            returns = returns.checked_add(1).ok_or(Error::DexCode)?;
        }
        for successor in successors.into_iter().flatten() {
            let bit = 1u128
                .checked_shl(successor)
                .ok_or(Error::DexInstructionLimit)?;
            if instruction_starts & bit == 0 {
                return Err(Error::DexCode);
            }
            enqueue_callback_pc(successor, &mut queue, &mut tail, &mut queued)?;
        }
    }
    if returns == 0 || reachable != instruction_starts {
        return Err(Error::DexNoReturn);
    }

    let mut indegree = [0u8; MAX_INSTRUCTION_UNITS as usize];
    pc = 0;
    while pc < code.units {
        for successor in callback_helper_successors(bytes, code, pc, profile)?
            .into_iter()
            .flatten()
        {
            let slot = indegree
                .get_mut(usize::try_from(successor).map_err(|_| Error::DexInstructionLimit)?)
                .ok_or(Error::DexInstructionLimit)?;
            *slot = slot.checked_add(1).ok_or(Error::DexCode)?;
        }
        pc += callback_helper_instruction_width(bytes, code, pc, profile)?;
    }
    head = 0;
    tail = 0;
    pc = 0;
    while pc < code.units {
        if indegree[usize::try_from(pc).map_err(|_| Error::DexInstructionLimit)?] == 0 {
            queue[tail] = u8::try_from(pc).map_err(|_| Error::DexInstructionLimit)?;
            tail += 1;
        }
        pc += callback_helper_instruction_width(bytes, code, pc, profile)?;
    }
    let mut processed = 0u32;
    while head < tail {
        let current = u32::from(queue[head]);
        head += 1;
        processed += 1;
        for successor in callback_helper_successors(bytes, code, current, profile)?
            .into_iter()
            .flatten()
        {
            let slot = indegree
                .get_mut(usize::try_from(successor).map_err(|_| Error::DexInstructionLimit)?)
                .ok_or(Error::DexInstructionLimit)?;
            *slot = slot.checked_sub(1).ok_or(Error::DexCode)?;
            if *slot == 0 {
                queue[tail] = u8::try_from(successor).map_err(|_| Error::DexInstructionLimit)?;
                tail += 1;
            }
        }
    }
    if processed != instruction_starts.count_ones() {
        return Err(Error::DexCode);
    }
    Ok(())
}

#[cfg(feature = "androidbox-dex-methods8")]
fn callback_helper_instruction_width(
    bytes: &[u8],
    code: Code,
    pc: u32,
    profile: CallbackHelperProfile,
) -> Result<u32, Error> {
    let opcode = instruction_unit(bytes, code, pc)? as u8;
    let width = match opcode {
        0x01 | 0x0f | 0x12 | 0x28 => 1,
        0x13 | 0x15 | 0x32 | 0x33 | 0xd8 => 2,
        0x14 => 3,
        #[cfg(feature = "androidbox-dex-instance9")]
        0x0a if profile == CallbackHelperProfile::InstanceView => 1,
        #[cfg(feature = "androidbox-dex-instance9")]
        0x6e if profile == CallbackHelperProfile::InstanceView => 3,
        _ => return Err(Error::DexUnknownOpcode(opcode)),
    };
    ensure_remaining(code, pc, width)?;
    Ok(width)
}

#[cfg(feature = "androidbox-dex-methods8")]
fn callback_helper_successors(
    bytes: &[u8],
    code: Code,
    pc: u32,
    profile: CallbackHelperProfile,
) -> Result<[Option<u32>; 2], Error> {
    let opcode = instruction_unit(bytes, code, pc)? as u8;
    if opcode == 0x0f {
        return Ok([None, None]);
    }
    if opcode == 0x28 {
        return Ok([
            Some(interactive_callback_goto_target(bytes, code, pc)?),
            None,
        ]);
    }
    let fallthrough = pc
        .checked_add(callback_helper_instruction_width(bytes, code, pc, profile)?)
        .ok_or(Error::DexCode)?;
    if fallthrough >= code.units {
        return Err(Error::DexNoReturn);
    }
    let branch = if matches!(opcode, 0x32 | 0x33) {
        Some(interactive_callback_branch_target(bytes, code, pc)?)
    } else {
        None
    };
    Ok([Some(fallthrough), branch])
}

fn decode_resource_const(bytes: &[u8], code: Code, pc: u32) -> Result<(usize, u32, u32), Error> {
    let first = instruction_unit(bytes, code, pc)?;
    let opcode = first as u8;
    let destination = usize::from(first >> 8);
    ensure_register(code.registers, destination)?;
    let (resource_id, width) = match opcode {
        0x14 => {
            ensure_remaining(code, pc, 3)?;
            let low = u32::from(instruction_unit(bytes, code, pc + 1)?);
            let high = u32::from(instruction_unit(bytes, code, pc + 2)?);
            (low | (high << 16), 3)
        }
        0x15 => {
            ensure_remaining(code, pc, 2)?;
            (u32::from(instruction_unit(bytes, code, pc + 1)?) << 16, 2)
        }
        _ => return Err(Error::DexUnknownOpcode(opcode)),
    };
    if resource_id == 0 {
        return Err(Error::DexFrameworkState);
    }
    Ok((destination, resource_id, width))
}

fn resolve_interactive_framework_method(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    activity: ActivityMethod,
) -> Result<InteractiveFrameworkMethod, Error> {
    if method_index >= header.method_ids.size {
        return Err(Error::DexBounds);
    }
    let class_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        0,
    )?);
    let name_index = read_table_u32(bytes, header.method_ids, 8, method_index, 4)?;
    let class = type_descriptor(bytes, header, class_index)?;
    let name = dex_string(bytes, header, name_index)?;
    if class == ACTIVITY_CLASS
        && name == ON_CREATE_METHOD
        && method_proto_matches(bytes, header, method_index, VOID_CLASS, BUNDLE_CLASS)?
    {
        return Ok(InteractiveFrameworkMethod::ActivityOnCreate);
    }
    if class_index == activity.class_type_index {
        if name == SET_CONTENT_VIEW_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, INT_CLASS)?
        {
            return Ok(InteractiveFrameworkMethod::ActivitySetContentViewResource);
        }
        if name == FIND_VIEW_BY_ID_METHOD
            && method_proto_matches(bytes, header, method_index, VIEW_CLASS, INT_CLASS)?
        {
            return Ok(InteractiveFrameworkMethod::ActivityFindViewById);
        }
    }
    if class == VIEW_CLASS
        && name == SET_ON_CLICK_LISTENER_METHOD
        && method_proto_matches(
            bytes,
            header,
            method_index,
            VOID_CLASS,
            ON_CLICK_LISTENER_CLASS,
        )?
    {
        return Ok(InteractiveFrameworkMethod::ViewSetOnClickListener);
    }
    if class == VIEW_CLASS
        && name == GET_ID_METHOD
        && method_no_arg_proto_matches(bytes, header, method_index, INT_CLASS)?
    {
        return Ok(InteractiveFrameworkMethod::ViewGetId);
    }
    if class == TEXT_VIEW_CLASS
        && name == SET_TEXT_METHOD
        && method_proto_matches(bytes, header, method_index, VOID_CLASS, INT_CLASS)?
    {
        return Ok(InteractiveFrameworkMethod::TextViewSetTextResource);
    }
    #[cfg(feature = "androidbox-string-text12")]
    if class == TEXT_VIEW_CLASS
        && name == SET_TEXT_METHOD
        && method_proto_matches(bytes, header, method_index, VOID_CLASS, CHAR_SEQUENCE_CLASS)?
    {
        return Ok(InteractiveFrameworkMethod::TextViewSetTextCharSequence);
    }
    #[cfg(feature = "androidbox-string-builder13")]
    if class == STRING_BUILDER_CLASS {
        if name == INIT_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, STRING_CLASS)?
        {
            return Ok(InteractiveFrameworkMethod::StringBuilderInitString);
        }
        if name == APPEND_METHOD
            && method_proto_matches(bytes, header, method_index, STRING_BUILDER_CLASS, INT_CLASS)?
        {
            return Ok(InteractiveFrameworkMethod::StringBuilderAppendInt);
        }
        if name == TO_STRING_METHOD
            && method_no_arg_proto_matches(bytes, header, method_index, STRING_CLASS)?
        {
            return Ok(InteractiveFrameworkMethod::StringBuilderToString);
        }
    }
    Err(Error::DexFrameworkUnknownMethod)
}

fn method_no_arg_proto_matches(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    expected_return: &[u8],
) -> Result<bool, Error> {
    let proto_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        2,
    )?);
    let shorty_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 0)?;
    let return_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 4)?;
    let parameters_offset = read_table_u32(bytes, header.proto_ids, 12, proto_index, 8)?;
    Ok(parameters_offset == 0
        && dex_string(bytes, header, shorty_index)? == [shorty_descriptor(expected_return)?]
        && type_descriptor(bytes, header, return_index)? == expected_return)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ActivityRegister {
    Empty,
    Null,
    Activity,
    TextViewUninitialized,
    TextView,
    String(u32),
    ResourceId(u32),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ActivityStage {
    Start,
    SuperCreated,
    ViewAllocated,
    ViewConstructed,
    StringLoaded,
    TextSet,
    LayoutLoaded,
    ContentSet,
}

#[derive(Clone, Copy)]
enum FrameworkMethod {
    ActivityOnCreate,
    TextViewInit,
    TextViewSetText,
    ActivitySetContentViewView,
    ActivitySetContentViewResource,
}

#[derive(Clone, Copy)]
struct Invoke {
    method_index: u32,
    count: usize,
    registers: [usize; 5],
}

fn execute_on_create(
    bytes: &[u8],
    header: Header,
    method: ActivityMethod,
) -> Result<(ActivityContent, u16), Error> {
    let code = validate_code(
        bytes,
        header,
        method.info.code_offset,
        method.info.parameters,
        method.info.outgoing,
    )?;
    let mut registers = [ActivityRegister::Empty; MAX_REGISTERS as usize];
    let parameter_start = usize::from(code.registers - code.parameters);
    registers[parameter_start] = ActivityRegister::Activity;
    registers[parameter_start + 1] = ActivityRegister::Null;

    let mut stage = ActivityStage::Start;
    let mut content = None;
    let mut pending_result = None;
    let mut pc = 0u32;
    let mut steps = 0u16;
    while pc < code.units {
        if steps >= MAX_EXECUTED_INSTRUCTIONS {
            return Err(Error::DexInstructionLimit);
        }
        let first = instruction_unit(bytes, code, pc)?;
        let opcode = first as u8;
        if pending_result.is_some() && !matches!(opcode, 0x0a | 0x0c) {
            return Err(Error::DexFrameworkState);
        }
        steps += 1;
        match opcode {
            0x0a | 0x0c => {
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                let result = pending_result.take().ok_or(Error::DexFrameworkState)?;
                let type_matches = match opcode {
                    0x0a => false,
                    0x0c => matches!(
                        result,
                        ActivityRegister::Null
                            | ActivityRegister::Activity
                            | ActivityRegister::TextView
                            | ActivityRegister::String(_)
                            | ActivityRegister::ResourceId(_)
                    ),
                    _ => false,
                };
                if !type_matches {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = result;
                pc += 1;
            }
            0x0e => {
                if first != 0x000e || pending_result.is_some() || stage != ActivityStage::ContentSet
                {
                    return Err(Error::DexFrameworkState);
                }
                pc += 1;
                if pc != code.units {
                    return Err(Error::DexCode);
                }
                return Ok((content.ok_or(Error::DexFrameworkState)?, steps));
            }
            0x1a => {
                ensure_remaining(code, pc, 2)?;
                if stage != ActivityStage::ViewConstructed {
                    return Err(Error::DexFrameworkState);
                }
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if registers[destination] != ActivityRegister::Empty {
                    return Err(Error::DexFrameworkState);
                }
                let string_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let value = dex_string(bytes, header, string_index)?;
                let text =
                    ViewText::from_ascii(value, Error::DexViewTextTooLong, Error::DexString)?;
                content = Some(ActivityContent::InlineText(text));
                registers[destination] = ActivityRegister::String(string_index);
                stage = ActivityStage::StringLoaded;
                pc += 2;
            }
            0x15 => {
                ensure_remaining(code, pc, 2)?;
                if stage != ActivityStage::SuperCreated {
                    return Err(Error::DexFrameworkState);
                }
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if !matches!(
                    registers[destination],
                    ActivityRegister::Empty | ActivityRegister::Null
                ) {
                    return Err(Error::DexFrameworkState);
                }
                let high = u32::from(instruction_unit(bytes, code, pc + 1)?);
                let resource_id = high << 16;
                if resource_id == 0 {
                    return Err(Error::DexFrameworkState);
                }
                registers[destination] = ActivityRegister::ResourceId(resource_id);
                content = Some(ActivityContent::LayoutResource(resource_id));
                stage = ActivityStage::LayoutLoaded;
                pc += 2;
            }
            0x22 => {
                ensure_remaining(code, pc, 2)?;
                if stage != ActivityStage::SuperCreated {
                    return Err(Error::DexFrameworkState);
                }
                let destination = usize::from(first >> 8);
                ensure_register(code.registers, destination)?;
                if !matches!(
                    registers[destination],
                    ActivityRegister::Empty | ActivityRegister::Null
                ) {
                    return Err(Error::DexFrameworkState);
                }
                let type_index = u32::from(instruction_unit(bytes, code, pc + 1)?);
                if type_descriptor(bytes, header, type_index)? != TEXT_VIEW_CLASS {
                    return Err(Error::DexFrameworkUnknownClass);
                }
                registers[destination] = ActivityRegister::TextViewUninitialized;
                stage = ActivityStage::ViewAllocated;
                pc += 2;
            }
            0x6e..=0x70 => {
                let invocation = decode_invoke(bytes, code, pc)?;
                let framework =
                    resolve_framework_method(bytes, header, invocation.method_index, method)?;
                pending_result = execute_framework_call(
                    opcode,
                    framework,
                    invocation,
                    &mut registers,
                    &mut stage,
                )?;
                pc += 3;
            }
            _ => return Err(Error::DexUnknownOpcode(opcode)),
        }
    }
    Err(Error::DexNoReturn)
}

fn decode_invoke(bytes: &[u8], code: Code, pc: u32) -> Result<Invoke, Error> {
    ensure_remaining(code, pc, 3)?;
    let first = instruction_unit(bytes, code, pc)?;
    let count = usize::from((first >> 12) & 0x0f);
    if count > 5 {
        return Err(Error::DexCode);
    }
    let fifth = usize::from((first >> 8) & 0x0f);
    let packed = instruction_unit(bytes, code, pc + 2)?;
    let registers = [
        usize::from(packed & 0x0f),
        usize::from((packed >> 4) & 0x0f),
        usize::from((packed >> 8) & 0x0f),
        usize::from((packed >> 12) & 0x0f),
        fifth,
    ];
    let mut index = 0usize;
    while index < count {
        ensure_register(code.registers, registers[index])?;
        index += 1;
    }
    while index < registers.len() {
        if registers[index] != 0 {
            return Err(Error::DexCode);
        }
        index += 1;
    }
    Ok(Invoke {
        method_index: u32::from(instruction_unit(bytes, code, pc + 1)?),
        count,
        registers,
    })
}

fn resolve_activity_super_constructor(
    bytes: &[u8],
    header: Header,
    method_index: u32,
) -> Result<(), Error> {
    if method_index >= header.method_ids.size {
        return Err(Error::DexBounds);
    }
    let class_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        0,
    )?);
    let name_index = read_table_u32(bytes, header.method_ids, 8, method_index, 4)?;
    let class = type_descriptor(bytes, header, class_index)?;
    let name = dex_string(bytes, header, name_index)?;
    if class != ACTIVITY_CLASS {
        return Err(Error::DexFrameworkUnknownClass);
    }
    if name != INIT_METHOD || !constructor_proto_matches(bytes, header, method_index)? {
        return Err(Error::DexFrameworkUnknownMethod);
    }
    Ok(())
}

fn resolve_framework_method(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    activity: ActivityMethod,
) -> Result<FrameworkMethod, Error> {
    if method_index >= header.method_ids.size {
        return Err(Error::DexBounds);
    }
    let class_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        0,
    )?);
    let name_index = read_table_u32(bytes, header.method_ids, 8, method_index, 4)?;
    let class = type_descriptor(bytes, header, class_index)?;
    let name = dex_string(bytes, header, name_index)?;

    if class == ACTIVITY_CLASS {
        if name == ON_CREATE_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, BUNDLE_CLASS)?
        {
            return Ok(FrameworkMethod::ActivityOnCreate);
        }
        return Err(Error::DexFrameworkUnknownMethod);
    }
    if class == TEXT_VIEW_CLASS {
        if name == INIT_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, CONTEXT_CLASS)?
        {
            return Ok(FrameworkMethod::TextViewInit);
        }
        if name == SET_TEXT_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, CHAR_SEQUENCE_CLASS)?
        {
            return Ok(FrameworkMethod::TextViewSetText);
        }
        return Err(Error::DexFrameworkUnknownMethod);
    }
    if class_index == activity.class_type_index {
        if name == SET_CONTENT_VIEW_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, VIEW_CLASS)?
        {
            return Ok(FrameworkMethod::ActivitySetContentViewView);
        }
        if name == SET_CONTENT_VIEW_METHOD
            && method_proto_matches(bytes, header, method_index, VOID_CLASS, INT_CLASS)?
        {
            return Ok(FrameworkMethod::ActivitySetContentViewResource);
        }
        return Err(Error::DexFrameworkUnknownMethod);
    }
    Err(Error::DexFrameworkUnknownClass)
}

fn method_proto_matches(
    bytes: &[u8],
    header: Header,
    method_index: u32,
    expected_return: &[u8],
    expected_parameter: &[u8],
) -> Result<bool, Error> {
    let proto_index = u32::from(read_table_u16(
        bytes,
        header.method_ids,
        8,
        method_index,
        2,
    )?);
    let shorty_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 0)?;
    let return_index = read_table_u32(bytes, header.proto_ids, 12, proto_index, 4)?;
    let parameters_offset = read_table_u32(bytes, header.proto_ids, 12, proto_index, 8)?;
    let expected_shorty = [
        shorty_descriptor(expected_return)?,
        shorty_descriptor(expected_parameter)?,
    ];
    if dex_string(bytes, header, shorty_index)? != expected_shorty
        || type_descriptor(bytes, header, return_index)? != expected_return
        || parameters_offset == 0
        || read_u32(bytes, usize_from(parameters_offset)?)? != 1
    {
        return Ok(false);
    }
    Ok(type_descriptor(
        bytes,
        header,
        u32::from(read_u16(bytes, usize_from(parameters_offset + 4)?)?),
    )? == expected_parameter)
}

fn shorty_descriptor(descriptor: &[u8]) -> Result<u8, Error> {
    match descriptor.first().copied() {
        Some(b'L' | b'[') => Ok(b'L'),
        Some(value @ (b'V' | b'Z' | b'B' | b'S' | b'C' | b'I' | b'J' | b'F' | b'D')) => Ok(value),
        _ => Err(Error::DexPrototype),
    }
}

fn execute_framework_call(
    opcode: u8,
    method: FrameworkMethod,
    invocation: Invoke,
    registers: &mut [ActivityRegister; MAX_REGISTERS as usize],
    stage: &mut ActivityStage,
) -> Result<Option<ActivityRegister>, Error> {
    if invocation.count != 2 {
        return Err(Error::DexFrameworkState);
    }
    let first = invocation.registers[0];
    let second = invocation.registers[1];
    match method {
        FrameworkMethod::ActivityOnCreate => {
            if opcode != 0x6f
                || *stage != ActivityStage::Start
                || registers[first] != ActivityRegister::Activity
                || registers[second] != ActivityRegister::Null
            {
                return Err(Error::DexFrameworkState);
            }
            *stage = ActivityStage::SuperCreated;
        }
        FrameworkMethod::TextViewInit => {
            if opcode != 0x70
                || *stage != ActivityStage::ViewAllocated
                || registers[first] != ActivityRegister::TextViewUninitialized
                || registers[second] != ActivityRegister::Activity
            {
                return Err(Error::DexFrameworkState);
            }
            registers[first] = ActivityRegister::TextView;
            *stage = ActivityStage::ViewConstructed;
        }
        FrameworkMethod::TextViewSetText => {
            if opcode != 0x6e
                || *stage != ActivityStage::StringLoaded
                || registers[first] != ActivityRegister::TextView
                || !matches!(registers[second], ActivityRegister::String(_))
            {
                return Err(Error::DexFrameworkState);
            }
            *stage = ActivityStage::TextSet;
        }
        FrameworkMethod::ActivitySetContentViewView => {
            if opcode != 0x6e
                || *stage != ActivityStage::TextSet
                || registers[first] != ActivityRegister::Activity
                || registers[second] != ActivityRegister::TextView
            {
                return Err(Error::DexFrameworkState);
            }
            *stage = ActivityStage::ContentSet;
        }
        FrameworkMethod::ActivitySetContentViewResource => {
            if opcode != 0x6e
                || *stage != ActivityStage::LayoutLoaded
                || registers[first] != ActivityRegister::Activity
                || !matches!(registers[second], ActivityRegister::ResourceId(_))
            {
                return Err(Error::DexFrameworkState);
            }
            *stage = ActivityStage::ContentSet;
        }
    }
    Ok(None)
}

fn ensure_remaining(code: Code, pc: u32, width: u32) -> Result<(), Error> {
    if pc.checked_add(width).ok_or(Error::DexCode)? <= code.units {
        Ok(())
    } else {
        Err(Error::DexCode)
    }
}

fn ensure_register(registers: u16, index: usize) -> Result<(), Error> {
    if index < usize::from(registers) {
        Ok(())
    } else {
        Err(Error::DexCode)
    }
}

fn ensure_registers(registers: u16, first: usize, second: usize) -> Result<(), Error> {
    ensure_register(registers, first)?;
    ensure_register(registers, second)
}

fn instruction_unit(bytes: &[u8], code: Code, pc: u32) -> Result<u16, Error> {
    if pc >= code.units {
        return Err(Error::DexCode);
    }
    let offset = code
        .units_offset
        .checked_add(usize_from(pc)?.checked_mul(2).ok_or(Error::DexBounds)?)
        .ok_or(Error::DexBounds)?;
    read_u16(bytes, offset)
}

fn type_descriptor(bytes: &[u8], header: Header, type_index: u32) -> Result<&[u8], Error> {
    if type_index >= header.type_ids.size {
        return Err(Error::DexBounds);
    }
    let string_index = read_table_u32(bytes, header.type_ids, 4, type_index, 0)?;
    dex_string(bytes, header, string_index)
}

fn dex_string(bytes: &[u8], header: Header, index: u32) -> Result<&[u8], Error> {
    if index >= header.string_ids.size {
        return Err(Error::DexBounds);
    }
    let offset = read_table_u32(bytes, header.string_ids, 4, index, 0)?;
    if offset < header.data.offset || offset >= header.file_size {
        return Err(Error::DexString);
    }
    let mut cursor = usize_from(offset)?;
    let utf16_length = read_uleb128(bytes, &mut cursor)?;
    if utf16_length == 0 || utf16_length as usize > MAX_STRING_BYTES {
        return Err(Error::DexString);
    }
    let start = cursor;
    let mut length = 0usize;
    loop {
        let byte = *bytes.get(cursor).ok_or(Error::DexString)?;
        if byte == 0 {
            break;
        }
        if !byte.is_ascii() {
            return Err(Error::DexString);
        }
        cursor += 1;
        length += 1;
        if length > MAX_STRING_BYTES {
            return Err(Error::DexString);
        }
    }
    if length != utf16_length as usize {
        return Err(Error::DexString);
    }
    Ok(&bytes[start..cursor])
}

fn read_uleb128(bytes: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let start = *cursor;
    let mut result = 0u32;
    let mut shift = 0u32;
    let mut count = 0u32;
    loop {
        if count == 5 {
            return Err(Error::DexBounds);
        }
        let byte = *bytes.get(*cursor).ok_or(Error::DexBounds)?;
        *cursor = cursor.checked_add(1).ok_or(Error::DexBounds)?;
        if count == 4 && byte & 0xf0 != 0 {
            return Err(Error::DexBounds);
        }
        result |= u32::from(byte & 0x7f) << shift;
        count += 1;
        if byte & 0x80 == 0 {
            if count > 1 && byte == 0 {
                return Err(Error::DexBounds);
            }
            return Ok(result);
        }
        shift += 7;
        if *cursor <= start {
            return Err(Error::DexBounds);
        }
    }
}

fn read_table_u16(
    bytes: &[u8],
    section: Section,
    item_size: u32,
    index: u32,
    field_offset: u32,
) -> Result<u16, Error> {
    read_u16(
        bytes,
        table_offset(section, item_size, index)?
            .checked_add(usize_from(field_offset)?)
            .ok_or(Error::DexBounds)?,
    )
}

fn read_table_u32(
    bytes: &[u8],
    section: Section,
    item_size: u32,
    index: u32,
    field_offset: u32,
) -> Result<u32, Error> {
    read_u32(
        bytes,
        table_offset(section, item_size, index)?
            .checked_add(usize_from(field_offset)?)
            .ok_or(Error::DexBounds)?,
    )
}

fn table_offset(section: Section, item_size: u32, index: u32) -> Result<usize, Error> {
    if index >= section.size {
        return Err(Error::DexBounds);
    }
    let offset = section
        .offset
        .checked_add(index.checked_mul(item_size).ok_or(Error::DexBounds)?)
        .ok_or(Error::DexBounds)?;
    usize_from(offset)
}

fn checked_range(
    bytes: &[u8],
    offset: u32,
    count: u32,
    width: u32,
) -> Result<core::ops::Range<usize>, Error> {
    let byte_count = count.checked_mul(width).ok_or(Error::DexBounds)?;
    let end = offset.checked_add(byte_count).ok_or(Error::DexBounds)?;
    if end > bytes.len() as u32 {
        return Err(Error::DexBounds);
    }
    Ok(usize_from(offset)?..usize_from(end)?)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    let value = bytes
        .get(offset..offset.checked_add(2).ok_or(Error::DexBounds)?)
        .ok_or(Error::DexBounds)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let value = bytes
        .get(offset..offset.checked_add(4).ok_or(Error::DexBounds)?)
        .ok_or(Error::DexBounds)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn usize_from(value: u32) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::DexBounds)
}

fn u32_from(value: usize) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| Error::DexBounds)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    #[cfg(feature = "androidbox-dex-instance9")]
    use super::validate_instance_callback_helper_control_flow;
    use super::{
        Code, Error, InteractiveListeners, MAX_INTERACTIVE_CALLBACKS,
        validate_interactive_callback_control_flow,
    };
    #[cfg(feature = "androidbox-dex-methods8")]
    use super::{execute_callback_helper, validate_callback_helper_control_flow};

    #[cfg(feature = "androidbox-multiaction3")]
    const MULTIACTION_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-multiaction-demo/androidbox-multiaction-demo.apk"
    );

    fn callback_code(units: &[u16]) -> (Vec<u8>, Code) {
        let mut bytes = Vec::with_capacity(units.len() * 2);
        for unit in units {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        (
            bytes,
            Code {
                registers: 2,
                parameters: 2,
                outgoing: 2,
                units_offset: 0,
                units: u32::try_from(units.len()).unwrap(),
            },
        )
    }

    #[cfg(feature = "androidbox-dex-methods8")]
    fn helper_code(units: &[u16]) -> (Vec<u8>, Code) {
        let (bytes, mut code) = callback_code(units);
        code.parameters = 1;
        code.outgoing = 0;
        (bytes, code)
    }

    #[cfg(feature = "androidbox-dex-instance9")]
    fn instance_helper_code(units: &[u16]) -> (Vec<u8>, Code) {
        let (bytes, mut code) = callback_code(units);
        code.registers = 3;
        code.parameters = 2;
        code.outgoing = 1;
        (bytes, code)
    }

    #[test]
    fn interactive_listener_set_is_unique_and_fixed_capacity() {
        let mut listeners = InteractiveListeners::empty();
        for index in 0..MAX_INTERACTIVE_CALLBACKS {
            listeners.insert(u8::try_from(index + 1).unwrap()).unwrap();
        }
        assert_eq!(listeners.as_slice().len(), MAX_INTERACTIVE_CALLBACKS);
        assert!(listeners.contains(1));
        assert_eq!(listeners.first(), Ok(1));
        assert_eq!(listeners.insert(1), Err(Error::DexFrameworkState));
        assert_eq!(
            listeners.insert(u8::try_from(MAX_INTERACTIVE_CALLBACKS + 1).unwrap()),
            Err(Error::DexFrameworkState)
        );
    }

    #[test]
    fn callback_cfg_accepts_two_forward_branch_returns() {
        // if-ne v0, v1, +3; return-void; return-void
        let (bytes, code) = callback_code(&[0x1033, 0x0003, 0x000e, 0x000e]);
        assert_eq!(
            validate_interactive_callback_control_flow(&bytes, code),
            Ok(())
        );
    }

    #[test]
    fn callback_cfg_rejects_interior_backward_and_unreachable_targets() {
        // Target 3 lands in the middle of the const at 2.
        let (interior, interior_code) = callback_code(&[0x1033, 0x0003, 0x0014, 0, 0, 0x000e]);
        assert_eq!(
            validate_interactive_callback_control_flow(&interior, interior_code),
            Err(Error::DexCode)
        );

        let (backward, backward_code) = callback_code(&[0x0014, 0, 0, 0x1033, 0xffff, 0x000e]);
        assert_eq!(
            validate_interactive_callback_control_flow(&backward, backward_code),
            Err(Error::DexCode)
        );

        let (unreachable, unreachable_code) = callback_code(&[0x000e, 0x000e]);
        assert_eq!(
            validate_interactive_callback_control_flow(&unreachable, unreachable_code),
            Err(Error::DexNoReturn)
        );
    }

    #[test]
    fn callback_cfg_requires_a_bounded_returning_path() {
        let (missing_return, missing_return_code) = callback_code(&[0x0014, 0, 0]);
        assert_eq!(
            validate_interactive_callback_control_flow(&missing_return, missing_return_code),
            Err(Error::DexNoReturn)
        );

        let (zero_branch, zero_branch_code) = callback_code(&[0x1032, 0x0000, 0x000e]);
        assert_eq!(
            validate_interactive_callback_control_flow(&zero_branch, zero_branch_code),
            Err(Error::DexCode)
        );
    }

    #[cfg(feature = "androidbox-dex-methods8")]
    #[test]
    fn app_defined_int_helper_executes_both_acyclic_resource_branches() {
        // const/high16 v0, #0x7f02; if-ne v1, v0, +6;
        // const v1, #0x7f040003; return v1;
        // const v1, #0x7f040005; return v1.
        let (bytes, code) = helper_code(&[
            0x0015, 0x7f02, 0x0133, 0x0006, 0x0114, 0x0003, 0x7f04, 0x010f, 0x0114, 0x0005, 0x7f04,
            0x010f,
        ]);
        assert_eq!(validate_callback_helper_control_flow(&bytes, code), Ok(()));
        assert_eq!(
            execute_callback_helper(&bytes, code, 0x7f02_0000),
            Ok((0x7f04_0003, 4))
        );
        assert_eq!(
            execute_callback_helper(&bytes, code, 0x7f02_0001),
            Ok((0x7f04_0005, 4))
        );

        let (looping, looping_code) = helper_code(&[0x0028]);
        assert_eq!(
            validate_callback_helper_control_flow(&looping, looping_code),
            Err(Error::DexCode)
        );
    }

    #[cfg(feature = "androidbox-dex-instance9")]
    #[test]
    fn app_defined_instance_helper_cfg_accepts_view_result_and_resource_branches() {
        // invoke-virtual {v2}, View.getId; move-result v2;
        // const/high16 v0, #0x7f02; if-ne v2, v0, +6;
        // const v2, #0x7f040004; return v2;
        // const v2, #0x7f040006; return v2.
        let (bytes, code) = instance_helper_code(&[
            0x106e, 0x0002, 0x0002, 0x020a, 0x0015, 0x7f02, 0x0233, 0x0006, 0x0214, 0x0004, 0x7f04,
            0x020f, 0x0214, 0x0006, 0x7f04, 0x020f,
        ]);
        assert_eq!(
            validate_instance_callback_helper_control_flow(&bytes, code),
            Ok(())
        );

        let (unknown, unknown_code) = instance_helper_code(&[0x0072, 0, 0, 0x000f]);
        assert_eq!(
            validate_instance_callback_helper_control_flow(&unknown, unknown_code),
            Err(Error::DexUnknownOpcode(0x72))
        );
    }

    #[cfg(feature = "androidbox-multiaction3")]
    #[test]
    fn real_d8_multiaction_callback_selects_two_distinct_resources() {
        let image = crate::AndroidBox::load(MULTIACTION_APK).expect("multi-action image");
        let mut session = image
            .launch_activity_session()
            .expect("multi-action session");
        assert_eq!(session.listener_button_id(), 0x7f01_0000);

        let approved = session
            .dispatch_click(0x7f01_0000)
            .expect("approved callback");
        assert_eq!(approved.changed_view_id, 0x7f01_0002);
        assert_eq!(approved.text_resource_id, 0x7f03_0003);
        assert_eq!(approved.revision, 1);
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .unwrap()
                .1
                .text
                .as_bytes(),
            b"Decision: approved"
        );

        let rejected = session
            .dispatch_click(0x7f01_0001)
            .expect("rejected callback");
        assert_eq!(rejected.changed_view_id, 0x7f01_0002);
        assert_eq!(rejected.text_resource_id, 0x7f03_0005);
        assert_eq!(rejected.revision, 2);
        assert_eq!(
            session
                .scene()
                .find_by_id(0x7f01_0002)
                .unwrap()
                .1
                .text
                .as_bytes(),
            b"Decision: rejected"
        );
    }
}
