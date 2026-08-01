//! Launcher-local host for the deliberately restricted AndroidBox demo.
//!
//! The interpreted bytecode never receives a Bndroid handle and cannot issue
//! syscalls. The trusted Launcher host exposes the two fixed DEX-0 integer
//! entry points plus one Resources-1 `onCreate`/compiled-layout/TextView path
//! from the immutable, repository-owned APK fixture.

use bndr_androidbox::{ActivityLaunch, AndroidBox, Error as AndroidBoxCoreError, Execution};
use bndr_ui::mobile::{
    AndroidBoxActivityError, AndroidBoxActivityIdentity, AndroidBoxError, AndroidBoxTextViewContent,
};
use bndr_ui::{UiAndroidBoxActivityLifecycleKind, UiAndroidBoxExecutionKind, UiClientControl};

static ANDROIDBOX_DEMO_APK: &[u8] =
    include_bytes!("../../../fixtures/androidbox-resource-demo/androidbox-resource-demo.apk");
const ANDROIDBOX_ACTIVITY_DESCRIPTOR: &[u8] = b"Lorg/bndroid/demo/MainActivity;";
const ANDROIDBOX_ACTIVITY_IDENTITY: &str = "org.bndroid.demo.MainActivity";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidBoxViewState {
    pub(super) verified: bool,
    pub(super) result: i32,
    pub(super) instruction_count: u32,
    pub(super) tap_count: u32,
    pub(super) error: AndroidBoxError,
}

impl AndroidBoxViewState {
    const fn idle() -> Self {
        Self {
            verified: false,
            result: 0,
            instruction_count: 0,
            tap_count: 0,
            error: AndroidBoxError::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidBoxRun {
    pub(super) view: AndroidBoxViewState,
    pub(super) report: UiClientControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidBoxActivityViewState {
    pub(super) manifest_verified: bool,
    pub(super) launcher_activity: AndroidBoxActivityIdentity,
    pub(super) on_create_completed: bool,
    pub(super) text_view_content: AndroidBoxTextViewContent,
    pub(super) instruction_count: u32,
    pub(super) resource_table_parsed: bool,
    pub(super) layout_entry_resolved: bool,
    pub(super) binary_xml_parsed: bool,
    pub(super) text_view_verified: bool,
    pub(super) string_reference_resolved: bool,
    pub(super) layout_resource_id: u32,
    pub(super) string_resource_id: u32,
    pub(super) error: AndroidBoxActivityError,
}

impl AndroidBoxActivityViewState {
    const fn idle() -> Self {
        Self {
            manifest_verified: false,
            launcher_activity: AndroidBoxActivityIdentity::empty(),
            on_create_completed: false,
            text_view_content: AndroidBoxTextViewContent::empty(),
            instruction_count: 0,
            resource_table_parsed: false,
            layout_entry_resolved: false,
            binary_xml_parsed: false,
            text_view_verified: false,
            string_reference_resolved: false,
            layout_resource_id: 0,
            string_resource_id: 0,
            error: AndroidBoxActivityError::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidBoxActivityRun {
    pub(super) view: AndroidBoxActivityViewState,
    pub(super) activity_report: UiClientControl,
    pub(super) resource_report: UiClientControl,
}

pub(super) struct AndroidBoxRuntime {
    program: Option<AndroidBox<'static>>,
    last_request_id: u64,
    last_activity_request_id: u32,
    last_resource_request_id: u32,
    last_result: i32,
    tap_count: u16,
    view: AndroidBoxViewState,
    activity_attempted: bool,
    activity_view: AndroidBoxActivityViewState,
}

impl AndroidBoxRuntime {
    pub(super) const fn new() -> Self {
        Self {
            program: None,
            last_request_id: 0,
            last_activity_request_id: 0,
            last_resource_request_id: 0,
            last_result: 0,
            tap_count: 0,
            view: AndroidBoxViewState::idle(),
            activity_attempted: false,
            activity_view: AndroidBoxActivityViewState::idle(),
        }
    }

    pub(super) const fn has_booted(&self) -> bool {
        self.program.is_some()
    }

    pub(super) const fn view(&self) -> AndroidBoxViewState {
        self.view
    }

    pub(super) const fn has_attempted_activity(&self) -> bool {
        self.activity_attempted
    }

    pub(super) const fn activity_view(&self) -> AndroidBoxActivityViewState {
        self.activity_view
    }

    pub(super) fn boot(&mut self) -> Result<AndroidBoxRun, AndroidBoxViewState> {
        if self.program.is_some() {
            return Err(self.view);
        }
        let program = match AndroidBox::load(ANDROIDBOX_DEMO_APK) {
            Ok(program) => program,
            Err(error) => return Err(self.reject(error)),
        };
        let execution = match program.boot() {
            Ok(execution) => execution,
            Err(error) => return Err(self.reject(error)),
        };
        let run = self.admit_execution(execution, UiAndroidBoxExecutionKind::Boot, 0)?;
        self.program = Some(program);
        Ok(run)
    }

    pub(super) fn on_tap(&mut self) -> Result<AndroidBoxRun, AndroidBoxViewState> {
        let Some(program) = self.program else {
            self.view.verified = false;
            self.view.error = AndroidBoxError::RuntimeTrap;
            return Err(self.view);
        };
        let tap_count = match self.tap_count.checked_add(1) {
            Some(tap_count) => tap_count,
            None => {
                self.view.verified = false;
                self.view.error = AndroidBoxError::StepLimit;
                return Err(self.view);
            }
        };
        let execution = match program.on_tap(self.last_result) {
            Ok(execution) => execution,
            Err(error) => return Err(self.reject(error)),
        };
        self.admit_execution(execution, UiAndroidBoxExecutionKind::Tap, tap_count)
    }

    // The complete fixed-capacity view state is intentionally returned on
    // failure so the Launcher can render the exact authoritative error
    // without allocation or a second fallible lookup.
    #[allow(clippy::result_large_err)]
    pub(super) fn launch_activity(
        &mut self,
    ) -> Result<AndroidBoxActivityRun, AndroidBoxActivityViewState> {
        if self.activity_attempted {
            return Err(self.activity_view);
        }
        let Some(program) = self.program else {
            self.activity_view.error = AndroidBoxActivityError::RuntimeTrap;
            return Err(self.activity_view);
        };
        self.activity_attempted = true;
        let launch = match program.launch_activity() {
            Ok(launch) => launch,
            Err(error) => return Err(self.reject_activity(error)),
        };
        self.admit_activity(launch)
    }

    fn admit_execution(
        &mut self,
        execution: Execution,
        kind: UiAndroidBoxExecutionKind,
        tap_count: u16,
    ) -> Result<AndroidBoxRun, AndroidBoxViewState> {
        let request_id = match self.last_request_id.checked_add(1) {
            Some(request_id) => request_id,
            None => {
                self.view.verified = false;
                self.view.error = AndroidBoxError::StepLimit;
                return Err(self.view);
            }
        };
        let instruction_count = match u8::try_from(execution.instruction_count) {
            Ok(count) => count,
            Err(_) => {
                self.view.verified = false;
                self.view.error = AndroidBoxError::StepLimit;
                return Err(self.view);
            }
        };
        let report = match UiClientControl::report_androidbox_execution(
            request_id,
            kind,
            execution.image.classes_dex_crc32,
            execution.image.dex_header_checksum,
            execution.result,
            instruction_count,
            tap_count,
        ) {
            Ok(report) => report,
            Err(_) => {
                self.view.verified = false;
                self.view.error = AndroidBoxError::RuntimeTrap;
                return Err(self.view);
            }
        };
        let view = AndroidBoxViewState {
            verified: true,
            result: execution.result,
            instruction_count: u32::from(execution.instruction_count),
            tap_count: u32::from(tap_count),
            error: AndroidBoxError::None,
        };
        self.last_request_id = request_id;
        self.last_result = execution.result;
        self.tap_count = tap_count;
        self.view = view;
        Ok(AndroidBoxRun { view, report })
    }

    #[allow(clippy::result_large_err)]
    fn admit_activity(
        &mut self,
        launch: ActivityLaunch,
    ) -> Result<AndroidBoxActivityRun, AndroidBoxActivityViewState> {
        if launch.manifest.activity_descriptor.as_bytes() != ANDROIDBOX_ACTIVITY_DESCRIPTOR {
            return Err(self.reject_activity(AndroidBoxCoreError::DexActivityClassInvalid));
        }
        let request_id = match self.last_activity_request_id.checked_add(1) {
            Some(request_id) => request_id,
            None => {
                self.activity_view.error = AndroidBoxActivityError::StepLimit;
                return Err(self.activity_view);
            }
        };
        let resource_request_id = match self.last_resource_request_id.checked_add(1) {
            Some(request_id) => request_id,
            None => {
                self.activity_view.error = AndroidBoxActivityError::StepLimit;
                return Err(self.activity_view);
            }
        };
        let method_index = match u16::try_from(launch.method.method_index) {
            Ok(method_index) => method_index,
            Err(_) => {
                self.activity_view.error = AndroidBoxActivityError::ActivityVerificationFailed;
                return Err(self.activity_view);
            }
        };
        let instruction_count = match u8::try_from(launch.instruction_count) {
            Ok(instruction_count) => instruction_count,
            Err(_) => {
                self.activity_view.error = AndroidBoxActivityError::StepLimit;
                return Err(self.activity_view);
            }
        };
        let launcher_activity =
            match AndroidBoxActivityIdentity::from_ascii(ANDROIDBOX_ACTIVITY_IDENTITY) {
                Some(identity) => identity,
                None => {
                    self.activity_view.error = AndroidBoxActivityError::ActivityVerificationFailed;
                    return Err(self.activity_view);
                }
            };
        let text_view_content =
            match AndroidBoxTextViewContent::from_ascii(launch.view.text.as_str()) {
                Some(content) => content,
                None => {
                    self.activity_view.error = AndroidBoxActivityError::RuntimeTrap;
                    return Err(self.activity_view);
                }
            };
        let Some(resources) = launch.resources else {
            self.activity_view.error = AndroidBoxActivityError::ActivityVerificationFailed;
            return Err(self.activity_view);
        };
        let activity_report = match UiClientControl::report_androidbox_activity_execution(
            request_id,
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            launch.manifest.android_manifest_crc32,
            launch.image.classes_dex_crc32,
            launch.image.dex_header_checksum,
            method_index,
            launch.method.code_offset,
            instruction_count,
            launch.view.text.as_bytes(),
        ) {
            Ok(report) => report,
            Err(_) => {
                self.activity_view.error = AndroidBoxActivityError::ActivityVerificationFailed;
                return Err(self.activity_view);
            }
        };
        let resource_report = match UiClientControl::report_androidbox_resource_execution(
            resource_request_id,
            resources.resources_arsc_crc32,
            resources.layout_xml_crc32,
            resources.layout_resource_id,
            resources.text_resource_id,
            launch.view.text.as_bytes(),
        ) {
            Ok(report) => report,
            Err(_) => {
                self.activity_view.error = AndroidBoxActivityError::ActivityVerificationFailed;
                return Err(self.activity_view);
            }
        };
        let view = AndroidBoxActivityViewState {
            manifest_verified: true,
            launcher_activity,
            on_create_completed: true,
            text_view_content,
            instruction_count: u32::from(launch.instruction_count),
            resource_table_parsed: true,
            layout_entry_resolved: true,
            binary_xml_parsed: true,
            text_view_verified: true,
            string_reference_resolved: true,
            layout_resource_id: resources.layout_resource_id,
            string_resource_id: resources.text_resource_id,
            error: AndroidBoxActivityError::None,
        };
        self.last_activity_request_id = request_id;
        self.last_resource_request_id = resource_request_id;
        self.activity_view = view;
        Ok(AndroidBoxActivityRun {
            view,
            activity_report,
            resource_report,
        })
    }

    fn reject(&mut self, error: AndroidBoxCoreError) -> AndroidBoxViewState {
        self.view.verified = false;
        self.view.instruction_count = 0;
        self.view.error = map_error(error);
        self.activity_view.error = map_activity_error(error);
        self.view
    }

    fn reject_activity(&mut self, error: AndroidBoxCoreError) -> AndroidBoxActivityViewState {
        self.activity_view.on_create_completed = false;
        self.activity_view.text_view_content = AndroidBoxTextViewContent::empty();
        self.activity_view.instruction_count = 0;
        self.activity_view.resource_table_parsed = false;
        self.activity_view.layout_entry_resolved = false;
        self.activity_view.binary_xml_parsed = false;
        self.activity_view.text_view_verified = false;
        self.activity_view.string_reference_resolved = false;
        self.activity_view.layout_resource_id = 0;
        self.activity_view.string_resource_id = 0;
        self.activity_view.error = map_activity_error(error);
        self.activity_view
    }
}

fn map_error(error: AndroidBoxCoreError) -> AndroidBoxError {
    use AndroidBoxCoreError::{
        ApkTooLarge, DexActivityClassDuplicate, DexActivityClassInvalid, DexActivityClassMissing,
        DexActivityMethodDuplicate, DexActivityMethodMissing, DexActivityPrototype, DexBounds,
        DexChecksum, DexClassDuplicate, DexClassMissing, DexCode, DexEndian,
        DexExceptionsUnsupported, DexFrameworkState, DexFrameworkUnknownClass,
        DexFrameworkUnknownMethod, DexHeader, DexInstructionLimit, DexMagic, DexMap,
        DexMethodDuplicate, DexMethodFlags, DexMethodMissing, DexNativeOrAbstract, DexNoReturn,
        DexPrototype, DexRegisterLimit, DexSignature, DexString, DexTooLarge, DexUnknownOpcode,
        DexViewTextTooLong, LayoutAttribute, LayoutEntryName, LayoutHeader, LayoutResourceMap,
        LayoutStringPool, LayoutStructure, LayoutTextViewDuplicate, LayoutTextViewMissing,
        LayoutTooLarge, LayoutUnknownChunk, ManifestActivityDuplicate, ManifestActivityInvalid,
        ManifestActivityMissing, ManifestActivityNotExported, ManifestApplicationDuplicate,
        ManifestApplicationMissing, ManifestAttribute, ManifestHeader, ManifestLauncherInvalid,
        ManifestPackageDuplicate, ManifestPackageInvalid, ManifestPackageMissing,
        ManifestResourceMap, ManifestString, ManifestStringPool, ManifestStructure,
        ManifestTooLarge, ManifestUnknownChunk, ManifestUnknownElement,
        ManifestVersionCodeDuplicate, ManifestVersionCodeInvalid, ManifestVersionCodeMissing,
        ResourceConfigurationAmbiguous, ResourceEntry, ResourceIdInvalid, ResourceNotFound,
        ResourcePackage, ResourceReference, ResourceString, ResourceStringPool,
        ResourceStringTooLong, ResourceTableHeader, ResourceTableTooLarge, ResourceType,
        ResourceValueType, Zip64Unsupported, ZipCentralDirectoryBounds, ZipCentralHeaderInvalid,
        ZipCompressedDex, ZipCompressedManifest, ZipCompressionUnsupported, ZipCrcMismatch,
        ZipDataDescriptorUnsupported, ZipDuplicateDex, ZipDuplicateLayout, ZipDuplicateManifest,
        ZipDuplicateResources, ZipEncrypted, ZipEndRecordInvalid, ZipEndRecordMissing,
        ZipEntryBounds, ZipEntryMismatch, ZipExtraInvalid, ZipLocalHeaderInvalid, ZipMissingDex,
        ZipMissingLayout, ZipMissingManifest, ZipMissingResources, ZipNameTooLong,
        ZipTooManyEntries,
    };

    match error {
        ApkTooLarge
        | ZipEndRecordMissing
        | ZipEndRecordInvalid
        | Zip64Unsupported
        | ZipTooManyEntries
        | ZipCentralDirectoryBounds
        | ZipCentralHeaderInvalid
        | ZipLocalHeaderInvalid
        | ZipEntryBounds
        | ZipNameTooLong
        | ZipExtraInvalid
        | ZipEncrypted
        | ZipDataDescriptorUnsupported
        | ZipCompressionUnsupported
        | ZipCompressedDex
        | ZipEntryMismatch
        | ZipCrcMismatch
        | ZipMissingDex
        | ZipDuplicateDex
        | DexTooLarge => AndroidBoxError::InvalidDex,
        DexMagic | DexHeader | DexEndian | DexChecksum | DexSignature | DexBounds | DexMap
        | DexString | DexClassMissing | DexClassDuplicate | DexMethodMissing
        | DexMethodDuplicate | DexPrototype | DexMethodFlags | DexNativeOrAbstract => {
            AndroidBoxError::VerificationFailed
        }
        DexUnknownOpcode(_) => AndroidBoxError::UnsupportedOpcode,
        DexInstructionLimit => AndroidBoxError::StepLimit,
        DexCode | DexExceptionsUnsupported | DexRegisterLimit | DexNoReturn => {
            AndroidBoxError::RuntimeTrap
        }
        ZipMissingManifest
        | ZipDuplicateManifest
        | ZipCompressedManifest
        | ManifestTooLarge
        | ManifestHeader
        | ManifestStringPool
        | ManifestString
        | ManifestResourceMap
        | ManifestUnknownChunk(_)
        | ManifestStructure
        | ManifestUnknownElement
        | ManifestAttribute
        | ManifestPackageMissing
        | ManifestPackageDuplicate
        | ManifestPackageInvalid
        | ManifestVersionCodeMissing
        | ManifestVersionCodeDuplicate
        | ManifestVersionCodeInvalid
        | ManifestApplicationMissing
        | ManifestApplicationDuplicate
        | ManifestActivityMissing
        | ManifestActivityDuplicate
        | ManifestActivityInvalid
        | ManifestActivityNotExported
        | ManifestLauncherInvalid
        | DexActivityClassMissing
        | DexActivityClassDuplicate
        | DexActivityClassInvalid
        | DexActivityMethodMissing
        | DexActivityMethodDuplicate
        | DexActivityPrototype
        | DexFrameworkUnknownClass
        | DexFrameworkUnknownMethod
        | ZipMissingResources
        | ZipDuplicateResources
        | ZipMissingLayout
        | ZipDuplicateLayout
        | ResourceTableTooLarge
        | ResourceTableHeader
        | ResourceStringPool
        | ResourceString
        | ResourceStringTooLong
        | ResourcePackage
        | ResourceType
        | ResourceEntry
        | ResourceIdInvalid
        | ResourceNotFound
        | ResourceValueType
        | ResourceReference
        | ResourceConfigurationAmbiguous
        | LayoutTooLarge
        | LayoutEntryName
        | LayoutHeader
        | LayoutStringPool
        | LayoutResourceMap
        | LayoutStructure
        | LayoutUnknownChunk(_)
        | LayoutAttribute
        | LayoutTextViewMissing
        | LayoutTextViewDuplicate => AndroidBoxError::VerificationFailed,
        DexFrameworkState | DexViewTextTooLong => AndroidBoxError::RuntimeTrap,
    }
}

fn map_activity_error(error: AndroidBoxCoreError) -> AndroidBoxActivityError {
    use AndroidBoxCoreError::{
        DexActivityClassDuplicate, DexActivityClassInvalid, DexActivityClassMissing,
        DexActivityMethodDuplicate, DexActivityMethodMissing, DexActivityPrototype,
        DexFrameworkState, DexFrameworkUnknownClass, DexFrameworkUnknownMethod,
        DexInstructionLimit, DexUnknownOpcode, DexViewTextTooLong, LayoutAttribute,
        LayoutEntryName, LayoutHeader, LayoutResourceMap, LayoutStringPool, LayoutStructure,
        LayoutTextViewDuplicate, LayoutTextViewMissing, LayoutTooLarge, LayoutUnknownChunk,
        ManifestActivityInvalid, ManifestActivityMissing, ManifestActivityNotExported,
        ManifestLauncherInvalid, ResourceConfigurationAmbiguous, ResourceEntry, ResourceIdInvalid,
        ResourceNotFound, ResourcePackage, ResourceReference, ResourceString, ResourceStringPool,
        ResourceStringTooLong, ResourceTableHeader, ResourceTableTooLarge, ResourceType,
        ResourceValueType, ZipCompressedManifest, ZipDuplicateLayout, ZipDuplicateManifest,
        ZipDuplicateResources, ZipMissingLayout, ZipMissingManifest, ZipMissingResources,
    };

    match error {
        ZipMissingManifest | ZipDuplicateManifest | ZipCompressedManifest => {
            AndroidBoxActivityError::ManifestRejected
        }
        ManifestActivityMissing
        | ManifestActivityInvalid
        | ManifestActivityNotExported
        | ManifestLauncherInvalid => AndroidBoxActivityError::LauncherActivityMissing,
        DexActivityClassMissing
        | DexActivityClassDuplicate
        | DexActivityClassInvalid
        | DexActivityMethodMissing
        | DexActivityMethodDuplicate
        | DexActivityPrototype => AndroidBoxActivityError::ActivityVerificationFailed,
        DexFrameworkUnknownClass | DexFrameworkUnknownMethod | DexUnknownOpcode(_) => {
            AndroidBoxActivityError::UnsupportedFrameworkCall
        }
        DexInstructionLimit => AndroidBoxActivityError::StepLimit,
        DexFrameworkState | DexViewTextTooLong => AndroidBoxActivityError::RuntimeTrap,
        ZipMissingResources
        | ZipDuplicateResources
        | ZipMissingLayout
        | ZipDuplicateLayout
        | ResourceTableTooLarge
        | ResourceTableHeader
        | ResourceStringPool
        | ResourceString
        | ResourceStringTooLong
        | ResourcePackage
        | ResourceType
        | ResourceEntry
        | ResourceIdInvalid
        | ResourceNotFound
        | ResourceValueType
        | ResourceReference
        | ResourceConfigurationAmbiguous
        | LayoutTooLarge
        | LayoutEntryName
        | LayoutHeader
        | LayoutStringPool
        | LayoutResourceMap
        | LayoutStructure
        | LayoutUnknownChunk(_)
        | LayoutAttribute
        | LayoutTextViewMissing
        | LayoutTextViewDuplicate => AndroidBoxActivityError::ActivityVerificationFailed,
        _ => AndroidBoxActivityError::ManifestRejected,
    }
}
