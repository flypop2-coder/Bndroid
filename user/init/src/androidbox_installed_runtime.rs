#![cfg_attr(feature = "androidbox-interactive0", allow(dead_code))]

//! EL0-side execution of one freshly granted installed APK image.
//!
//! This module is deliberately capability-free: it accepts only borrowed APK
//! bytes and an authority-free catalog snapshot. The App process performs the
//! syscall-61 claim and bounded VMO reads in `main.rs`, then calls this pure
//! verifier/interpreter before any Activity pixels are submitted.

use core::cell::UnsafeCell;

use bndr_abi::{ANDROID_PACKAGE_APK_MAX_BYTES, VMO_READ_MAX_BYTES};
#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-scene-rpc2")
))]
use bndr_androidbox::ActivityScene;
#[cfg(feature = "androidbox-apk-envelope4")]
use bndr_androidbox::AndroidBoxEnvelopeScratch;
#[cfg(feature = "androidbox-manifest-catalog3")]
use bndr_androidbox::AndroidBoxManifestCatalogScratch;
#[cfg(feature = "androidbox-interactive0")]
use bndr_androidbox::{
    ActivitySession, ActivityUpdate, InteractiveActivityLaunch, MAX_INTERACTIVE_ACTIVITY_CALLBACKS,
    MAX_RESOURCE_STRING_BYTES, ViewKind,
};
use bndr_androidbox::{AndroidBox, verify_apk_v2};
#[cfg(feature = "androidbox-scene-rpc2")]
use bndr_androidbox::{LayoutOrientation, LayoutSize, MAX_ACTIVITY_SCENE_NODES};
use bndr_sm::verified_manifest::sha256;
use bndr_ui::mobile::AndroidInstalledAppStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InstalledActivityExecutionError {
    CatalogInvalid,
    LengthMismatch,
    ImageReadFailed,
    ApkDigestMismatch,
    SignatureRejected,
    SignerMismatch,
    ImageRejected,
    IdentityMismatch,
    ResourcesMissing,
    PublisherContentMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstalledActivityExecution {
    pub(super) status: AndroidInstalledAppStatus,
    pub(super) constructor_instruction_count: u16,
    pub(super) on_create_instruction_count: u16,
    pub(super) resources_arsc_crc32: u32,
    pub(super) layout_xml_crc32: u32,
    pub(super) layout_resource_id: u32,
    pub(super) text_resource_id: u32,
}

struct InstalledApkBuffer(UnsafeCell<[u8; ANDROID_PACKAGE_APK_MAX_BYTES as usize]>);

// SAFETY: the image buffer is private to the single resident App process. Its
// event loop invokes `read_and_execute_installed_activity` synchronously, and
// the function wipes the complete buffer before and after every lease.
unsafe impl Sync for InstalledApkBuffer {}

static INSTALLED_APK_BUFFER: InstalledApkBuffer =
    InstalledApkBuffer(UnsafeCell::new([0; ANDROID_PACKAGE_APK_MAX_BYTES as usize]));

#[cfg(feature = "androidbox-apk-envelope4")]
struct AndroidEnvelopeScratchBuffer(UnsafeCell<AndroidBoxEnvelopeScratch>);

// SAFETY: this workspace is private to one single-threaded EL0 runtime. Every
// call is synchronous, and `load_with_scratch` returns an AndroidBox borrowing
// only the immutable APK bytes while wiping the workspace before return.
#[cfg(feature = "androidbox-apk-envelope4")]
unsafe impl Sync for AndroidEnvelopeScratchBuffer {}

#[cfg(feature = "androidbox-apk-envelope4")]
static ANDROID_ENVELOPE_SCRATCH: AndroidEnvelopeScratchBuffer =
    AndroidEnvelopeScratchBuffer(UnsafeCell::new(AndroidBoxEnvelopeScratch::new()));

#[cfg(feature = "androidbox-manifest-catalog3")]
struct AndroidManifestCatalogScratchBuffer(UnsafeCell<AndroidBoxManifestCatalogScratch>);

// SAFETY: this workspace is private to the same single-threaded AndroidApp
// loop as the envelope scratch and is wiped on every catalog load attempt.
#[cfg(feature = "androidbox-manifest-catalog3")]
unsafe impl Sync for AndroidManifestCatalogScratchBuffer {}

#[cfg(feature = "androidbox-manifest-catalog3")]
static ANDROID_MANIFEST_CATALOG_SCRATCH: AndroidManifestCatalogScratchBuffer =
    AndroidManifestCatalogScratchBuffer(UnsafeCell::new(AndroidBoxManifestCatalogScratch::new()));

#[cfg(feature = "androidbox-interactive0")]
struct InteractiveLeaseState {
    opening: bool,
    session: Option<ActivitySession<'static>>,
    // In InteractiveActivity-1 this is the unique TextView. SceneRPC-2
    // deliberately decouples the catalog's first TextView from the callback
    // mutation target, so this stores the first proven callback target.
    // MultiActionActivity-3 validates each returned target dynamically.
    label_id: u32,
    button_id: u32,
}

#[cfg(feature = "androidbox-interactive0")]
impl InteractiveLeaseState {
    const fn empty() -> Self {
        Self {
            opening: false,
            session: None,
            label_id: 0,
            button_id: 0,
        }
    }
}

#[cfg(feature = "androidbox-interactive0")]
struct InteractiveLeaseCell(UnsafeCell<InteractiveLeaseState>);

// SAFETY: Bndroid's resident App runtime is a single-threaded event loop. All
// access is synchronous through `InstalledInteractiveActivityLease`, which
// enforces one active owner. The stored session borrows only
// `INSTALLED_APK_BUFFER`; that buffer is never mutated while the session is
// present, and close drops the session before wiping the buffer.
#[cfg(feature = "androidbox-interactive0")]
unsafe impl Sync for InteractiveLeaseCell {}

#[cfg(feature = "androidbox-interactive0")]
static INTERACTIVE_LEASE: InteractiveLeaseCell =
    InteractiveLeaseCell(UnsafeCell::new(InteractiveLeaseState::empty()));

#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InstalledInteractiveActivityError {
    AlreadyActive,
    NotActive,
    CatalogInvalid,
    LengthMismatch,
    ImageReadFailed,
    ApkDigestMismatch,
    SignatureRejected,
    SignerMismatch,
    ImageRejected,
    IdentityMismatch,
    SceneRejected,
    TextRejected,
    ClickRejected,
}

#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstalledInteractiveAscii {
    bytes: [u8; MAX_RESOURCE_STRING_BYTES],
    len: u16,
}

#[cfg(feature = "androidbox-interactive0")]
impl InstalledInteractiveAscii {
    pub(super) const fn empty() -> Self {
        Self {
            bytes: [0; MAX_RESOURCE_STRING_BYTES],
            len: 0,
        }
    }

    pub(super) fn from_ascii(value: &[u8]) -> Option<Self> {
        if value.is_empty()
            || value.len() > MAX_RESOURCE_STRING_BYTES
            || !value.iter().all(|byte| byte.is_ascii() && *byte != 0)
        {
            return None;
        }
        let mut result = Self {
            bytes: [0; MAX_RESOURCE_STRING_BYTES],
            len: 0,
        };
        result.bytes[..value.len()].copy_from_slice(value);
        result.len = u16::try_from(value.len()).ok()?;
        Some(result)
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    pub(super) fn as_str(&self) -> &str {
        // ASCII admission above is stronger than UTF-8 validity.
        core::str::from_utf8(self.as_bytes()).expect("interactive ASCII invariant")
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
const INSTALLED_ACTIVITY_SCENE_TEXT_MAX_BYTES: usize = 96;

/// One allocation-free node copied out of the verified Android resource
/// scene. It contains no APK borrow and can therefore cross the AndroidApp
/// process boundary through SceneRPC-2 one node at a time.
#[cfg(feature = "androidbox-scene-rpc2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstalledInteractiveSceneNode {
    pub(super) kind: ViewKind,
    pub(super) parent: Option<u8>,
    pub(super) id: u32,
    pub(super) width: LayoutSize,
    pub(super) height: LayoutSize,
    pub(super) orientation: LayoutOrientation,
    pub(super) layout_weight: u8,
    pub(super) layout_margin_dp: u8,
    pub(super) padding_dp: u8,
    pub(super) layout_margin_left_dp: u8,
    pub(super) layout_margin_top_dp: u8,
    pub(super) layout_margin_right_dp: u8,
    pub(super) layout_margin_bottom_dp: u8,
    pub(super) padding_left_dp: u8,
    pub(super) padding_top_dp: u8,
    pub(super) padding_right_dp: u8,
    pub(super) padding_bottom_dp: u8,
    pub(super) exact_width_dp: u8,
    pub(super) exact_height_dp: u8,
    pub(super) text: InstalledInteractiveAscii,
    pub(super) callback_registered: bool,
}

#[cfg(feature = "androidbox-scene-rpc2")]
impl InstalledInteractiveSceneNode {
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
            text: InstalledInteractiveAscii::empty(),
            callback_registered: false,
        }
    }
}

#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstalledInteractiveActivitySnapshot {
    pub(super) label_id: u32,
    pub(super) button_id: u32,
    pub(super) label_text: InstalledInteractiveAscii,
    pub(super) button_text: InstalledInteractiveAscii,
    pub(super) revision: u32,
    pub(super) execution: InteractiveActivityLaunch,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub(super) scene_nodes: [InstalledInteractiveSceneNode; MAX_ACTIVITY_SCENE_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub(super) scene_node_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub(super) callback_target_id: u32,
}

#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstalledInteractiveActivityUpdate {
    pub(super) label_id: u32,
    pub(super) label_text: InstalledInteractiveAscii,
    pub(super) revision: u32,
    pub(super) execution: ActivityUpdate,
}

/// Owned token for the process-global InteractiveActivity lease.
///
/// The token itself owns no borrowed data. The single session is stored in
/// `INTERACTIVE_LEASE` so it can borrow the static APK buffer across App event
/// turns. `open` is the only lifetime extension point, and `close`/`Drop`
/// always destroy that session before the complete 65,024-byte buffer is
/// wiped.
#[cfg(feature = "androidbox-interactive0")]
pub(super) struct InstalledInteractiveActivityLease {
    active: bool,
}

#[cfg(feature = "androidbox-interactive0")]
impl InstalledInteractiveActivityLease {
    pub(super) const fn new() -> Self {
        Self { active: false }
    }

    pub(super) fn open(
        &mut self,
        expected: AndroidInstalledAppStatus,
        mut read: impl FnMut(usize, &mut [u8]) -> Result<(), ()>,
    ) -> Result<InstalledInteractiveActivitySnapshot, InstalledInteractiveActivityError> {
        if self.active || interactive_lease_active() {
            return Err(InstalledInteractiveActivityError::AlreadyActive);
        }
        // SAFETY: the active check and reservation happen synchronously in the
        // single App event loop. Marking `opening` before invoking the read
        // callback also rejects any accidental reentrant lease attempt.
        unsafe { &mut *INTERACTIVE_LEASE.0.get() }.opening = true;
        // SAFETY: no interactive session is present, the single-threaded App
        // event loop holds the only lease token executing this method, and the
        // callback cannot retain the temporary mutable slices.
        let buffer = unsafe { &mut *INSTALLED_APK_BUFFER.0.get() };
        buffer.fill(0);
        let length = expected.apk_length as usize;
        if length == 0 || length > buffer.len() {
            return Err(abort_interactive_open(
                buffer,
                InstalledInteractiveActivityError::CatalogInvalid,
            ));
        }
        let mut offset = 0usize;
        while offset < length {
            let end = (offset + VMO_READ_MAX_BYTES).min(length);
            if read(offset, &mut buffer[offset..end]).is_err() {
                return Err(abort_interactive_open(
                    buffer,
                    InstalledInteractiveActivityError::ImageReadFailed,
                ));
            }
            offset = end;
        }

        let built = build_interactive_activity_session(&buffer[..length], expected);
        let (session, snapshot) = match built {
            Ok(value) => value,
            Err(error) => {
                return Err(abort_interactive_open(buffer, error));
            }
        };
        // SAFETY: `session` borrows only the prefix of the process-static
        // `INSTALLED_APK_BUFFER`. The session is moved into private singleton
        // state and is never returned, so the extended lifetime cannot escape.
        // Every mutation of that buffer checks that the singleton is empty;
        // `close` and `Drop` first remove/drop this exact session and only then
        // wipe the entire buffer.
        let session: ActivitySession<'static> = unsafe { core::mem::transmute(session) };
        // SAFETY: single App event loop; the preflight active check above
        // guarantees exclusive singleton access for this state transition.
        let state = unsafe { &mut *INTERACTIVE_LEASE.0.get() };
        debug_assert!(state.opening && state.session.is_none());
        state.opening = false;
        #[cfg(not(feature = "androidbox-scene-rpc2"))]
        {
            state.label_id = snapshot.label_id;
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        {
            state.label_id = snapshot.callback_target_id;
        }
        state.button_id = snapshot.button_id;
        state.session = Some(session);
        self.active = true;
        Ok(snapshot)
    }

    pub(super) fn dispatch_button_click(
        &mut self,
        view_id: u32,
    ) -> Result<InstalledInteractiveActivityUpdate, InstalledInteractiveActivityError> {
        if !self.active {
            return Err(InstalledInteractiveActivityError::NotActive);
        }
        // SAFETY: an active owned token is unique in the single App event loop.
        let state = unsafe { &mut *INTERACTIVE_LEASE.0.get() };
        #[cfg(not(feature = "androidbox-multiaction3"))]
        if view_id == 0 || view_id != state.button_id {
            return Err(InstalledInteractiveActivityError::ClickRejected);
        }
        let session = state
            .session
            .as_mut()
            .ok_or(InstalledInteractiveActivityError::NotActive)?;
        #[cfg(feature = "androidbox-multiaction3")]
        if !session.is_listener_button(view_id) {
            return Err(InstalledInteractiveActivityError::ClickRejected);
        }
        let execution = session
            .dispatch_click(view_id)
            .map_err(|_| InstalledInteractiveActivityError::ClickRejected)?;
        #[cfg(not(feature = "androidbox-multiaction3"))]
        let target_matches = execution.changed_view_id == state.label_id;
        #[cfg(feature = "androidbox-multiaction3")]
        let target_matches = session
            .scene()
            .find_by_id(execution.changed_view_id)
            .is_some_and(|(_, node)| node.kind == ViewKind::TextView);
        if !target_matches || execution.revision != session.revision() {
            return Err(InstalledInteractiveActivityError::ClickRejected);
        }
        let (_, label) = session
            .scene()
            .find_by_id(execution.changed_view_id)
            .ok_or(InstalledInteractiveActivityError::SceneRejected)?;
        let label_text = InstalledInteractiveAscii::from_ascii(label.text.as_bytes())
            .ok_or(InstalledInteractiveActivityError::TextRejected)?;
        Ok(InstalledInteractiveActivityUpdate {
            label_id: execution.changed_view_id,
            label_text,
            revision: execution.revision,
            execution,
        })
    }

    pub(super) fn close(&mut self) -> Result<(), InstalledInteractiveActivityError> {
        if !self.active {
            return Err(InstalledInteractiveActivityError::NotActive);
        }
        // SAFETY: this owned token is the unique active accessor in the
        // single-threaded App event loop.
        let state = unsafe { &mut *INTERACTIVE_LEASE.0.get() };
        let had_session = state.session.is_some();
        // Assignment removes the only lifetime-extended ActivitySession before
        // any byte in its backing buffer is mutated.
        state.session = None;
        state.opening = false;
        state.label_id = 0;
        state.button_id = 0;
        self.active = false;
        let result = if had_session {
            Ok(())
        } else {
            Err(InstalledInteractiveActivityError::NotActive)
        };
        // SAFETY: the singleton no longer holds any borrow into the static
        // buffer and the single App event loop is the only accessor.
        unsafe { &mut *INSTALLED_APK_BUFFER.0.get() }.fill(0);
        result
    }
}

#[cfg(feature = "androidbox-interactive0")]
impl Drop for InstalledInteractiveActivityLease {
    fn drop(&mut self) {
        if self.active {
            let _ = self.close();
        }
    }
}

#[cfg(feature = "androidbox-interactive0")]
fn interactive_lease_active() -> bool {
    // SAFETY: every caller runs synchronously in the single resident App event
    // loop; no interrupt or worker accesses this App-private state.
    let state = unsafe { &*INTERACTIVE_LEASE.0.get() };
    state.opening || state.session.is_some()
}

#[cfg(feature = "androidbox-interactive0")]
fn abort_interactive_open(
    buffer: &mut [u8; ANDROID_PACKAGE_APK_MAX_BYTES as usize],
    error: InstalledInteractiveActivityError,
) -> InstalledInteractiveActivityError {
    // SAFETY: only the synchronously executing owner of the `opening`
    // reservation can reach this rollback.
    let state = unsafe { &mut *INTERACTIVE_LEASE.0.get() };
    state.opening = false;
    state.label_id = 0;
    state.button_id = 0;
    debug_assert!(state.session.is_none());
    buffer.fill(0);
    error
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, bndr_androidbox::Error> {
    // SAFETY: the resident App/AndroidApp loop is single-threaded and every
    // caller completes synchronously. Catalog declarations remain inert; only
    // the selected enabled exported launcher is bound to the DEX runtime.
    let envelope_scratch = unsafe { &mut *ANDROID_ENVELOPE_SCRATCH.0.get() };
    let catalog_scratch = unsafe { &mut *ANDROID_MANIFEST_CATALOG_SCRATCH.0.get() };
    AndroidBox::load_with_manifest_catalog_scratch(apk, envelope_scratch, catalog_scratch)
}

#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-manifest-catalog3")
))]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, bndr_androidbox::Error> {
    // SAFETY: the resident App/AndroidApp loop is single-threaded and every
    // caller completes synchronously. The returned image borrows only `apk`;
    // the bounded XML workspace is wiped inside `load_with_scratch`.
    let scratch = unsafe { &mut *ANDROID_ENVELOPE_SCRATCH.0.get() };
    AndroidBox::load_with_scratch(apk, scratch)
}

#[cfg(not(feature = "androidbox-apk-envelope4"))]
fn load_android_box(apk: &[u8]) -> Result<AndroidBox<'_>, bndr_androidbox::Error> {
    AndroidBox::load(apk)
}

#[cfg(feature = "androidbox-interactive0")]
fn build_interactive_activity_session<'a>(
    apk: &'a [u8],
    expected: AndroidInstalledAppStatus,
) -> Result<
    (ActivitySession<'a>, InstalledInteractiveActivitySnapshot),
    InstalledInteractiveActivityError,
> {
    if !expected.installed
        || expected.generation == 0
        || expected.apk_length == 0
        || expected.package.is_empty()
        || expected.activity.is_empty()
        || expected.title.is_empty()
    {
        return Err(InstalledInteractiveActivityError::CatalogInvalid);
    }
    if apk.len() != expected.apk_length as usize {
        return Err(InstalledInteractiveActivityError::LengthMismatch);
    }
    if sha256(apk) != expected.apk_digest_sha256 {
        return Err(InstalledInteractiveActivityError::ApkDigestMismatch);
    }
    let signer =
        verify_apk_v2(apk).map_err(|_| InstalledInteractiveActivityError::SignatureRejected)?;
    if signer.certificate_sha256 != expected.signer_digest_sha256 {
        return Err(InstalledInteractiveActivityError::SignerMismatch);
    }
    let image =
        load_android_box(apk).map_err(|_| InstalledInteractiveActivityError::ImageRejected)?;
    let manifest = image.manifest_info();
    if manifest.package.as_str() != expected.package.as_str()
        || manifest.activity_descriptor.as_str() != expected.activity.as_str()
        || manifest.version_code != expected.version_code
    {
        return Err(InstalledInteractiveActivityError::IdentityMismatch);
    }
    // A separate fresh session proves every admitted callback target and
    // post-click text are representable by this ASCII-only EL0 facade. The
    // retained session remains pristine at revision zero. SceneRPC-2 also uses
    // the first deterministic execution to decouple the first catalog
    // TextView from the TextView actually mutated by the callback. Run that
    // probe in a separate stack frame before constructing the retained
    // session, so the two page-sized fixed-capacity scenes are never live in
    // one AndroidApp EL0 stack frame.
    let probe = probe_interactive_activity(&image)?;
    let session = image
        .launch_activity_session()
        .map_err(|_| InstalledInteractiveActivityError::ImageRejected)?;
    let listener_count = usize::from(probe.listener_count);
    if session.listener_button_count() != probe.listener_count
        || session.listener_button_ids() != &probe.listener_ids[..listener_count]
    {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }
    let snapshot = interactive_snapshot(&session, probe.callback_target_id)?;
    if snapshot.execution.manifest != manifest {
        return Err(InstalledInteractiveActivityError::IdentityMismatch);
    }
    let reconstructed = AndroidInstalledAppStatus::try_new(
        expected.generation,
        manifest.version_code,
        expected.apk_length,
        manifest.package.as_str(),
        manifest.activity_descriptor.as_str(),
        manifest.application_label.as_str(),
        snapshot.label_text.as_str(),
        signer.certificate_sha256,
        expected.apk_digest_sha256,
    )
    .ok_or(InstalledInteractiveActivityError::TextRejected)?;
    if reconstructed != expected {
        return Err(InstalledInteractiveActivityError::IdentityMismatch);
    }

    Ok((session, snapshot))
}

#[cfg(feature = "androidbox-interactive0")]
struct InteractiveActivityProbe {
    callback_target_id: u32,
    listener_ids: [u32; MAX_INTERACTIVE_ACTIVITY_CALLBACKS],
    listener_count: u8,
}

#[cfg(feature = "androidbox-interactive0")]
fn probe_interactive_activity(
    image: &AndroidBox<'_>,
) -> Result<InteractiveActivityProbe, InstalledInteractiveActivityError> {
    let mut probe = image
        .launch_activity_session()
        .map_err(|_| InstalledInteractiveActivityError::ImageRejected)?;
    #[cfg(not(feature = "androidbox-multiaction3"))]
    if probe.listener_button_count() != 1 {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }
    #[cfg(feature = "androidbox-multiaction3")]
    if !(1..=u8::try_from(MAX_INTERACTIVE_ACTIVITY_CALLBACKS).expect("callback capacity fits u8"))
        .contains(&probe.listener_button_count())
    {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }
    let listener_count = probe.listener_button_count();
    let mut listener_ids = [0u32; MAX_INTERACTIVE_ACTIVITY_CALLBACKS];
    listener_ids[..usize::from(listener_count)].copy_from_slice(probe.listener_button_ids());
    let mut callback_target_id = 0u32;
    for listener_id in &listener_ids[..usize::from(listener_count)] {
        let probe_update = probe
            .dispatch_click(*listener_id)
            .map_err(|_| InstalledInteractiveActivityError::ClickRejected)?;
        if callback_target_id == 0 {
            callback_target_id = probe_update.changed_view_id;
        }
        let (_, probe_label) = probe
            .scene()
            .find_by_id(probe_update.changed_view_id)
            .ok_or(InstalledInteractiveActivityError::SceneRejected)?;
        if probe_update.revision != probe.revision()
            || InstalledInteractiveAscii::from_ascii(probe_label.text.as_bytes()).is_none()
        {
            return Err(InstalledInteractiveActivityError::SceneRejected);
        }
    }
    if probe.revision() != u32::from(listener_count) {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }
    Ok(InteractiveActivityProbe {
        callback_target_id,
        listener_ids,
        listener_count,
    })
}

#[cfg(feature = "androidbox-interactive0")]
fn interactive_snapshot(
    session: &ActivitySession<'_>,
    callback_target_id: u32,
) -> Result<InstalledInteractiveActivitySnapshot, InstalledInteractiveActivityError> {
    #[cfg(not(feature = "androidbox-scene-rpc2"))]
    {
        let (label_id, label_text) = unique_scene_text(session.scene(), ViewKind::TextView)?;
        let (button_id, button_text) = unique_scene_text(session.scene(), ViewKind::Button)?;
        if button_id != session.listener_button_id()
            || callback_target_id != label_id
            || session.revision() != 0
        {
            return Err(InstalledInteractiveActivityError::SceneRejected);
        }
        return Ok(InstalledInteractiveActivitySnapshot {
            label_id,
            button_id,
            label_text,
            button_text,
            revision: 0,
            execution: session.launch_info(),
        });
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    {
        interactive_scene_snapshot(session, callback_target_id)
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn interactive_scene_snapshot(
    session: &ActivitySession<'_>,
    callback_target_id: u32,
) -> Result<InstalledInteractiveActivitySnapshot, InstalledInteractiveActivityError> {
    let nodes = session.scene().nodes();
    if nodes.is_empty()
        || nodes.len() > MAX_ACTIVITY_SCENE_NODES
        || session.revision() != 0
        || callback_target_id == 0
    {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }

    let mut scene_nodes = [InstalledInteractiveSceneNode::empty(); MAX_ACTIVITY_SCENE_NODES];
    let mut catalog_label = None;
    let mut listener_button = None;
    let mut listener_button_count = 0u8;
    let mut callback_target_seen = false;
    for (index, node) in nodes.iter().enumerate() {
        let index =
            u8::try_from(index).map_err(|_| InstalledInteractiveActivityError::SceneRejected)?;
        if (index == 0 && node.parent.is_some())
            || (index != 0 && node.parent.is_none_or(|parent| parent >= index))
        {
            return Err(InstalledInteractiveActivityError::SceneRejected);
        }
        #[cfg(feature = "androidbox-layout-weight15")]
        {
            let weighted = node.layout_weight != 0;
            let parent_is_horizontal = node.parent.is_some_and(|parent| {
                nodes[usize::from(parent)].orientation == LayoutOrientation::Horizontal
            });
            if node.layout_weight > 8
                || node.height == LayoutSize::Zero
                || (node.width == LayoutSize::Zero) != weighted
                || (weighted
                    && (node.kind != ViewKind::Button
                        || !{
                            #[cfg(feature = "androidbox-layout-size18")]
                            {
                                matches!(node.height, LayoutSize::WrapContent | LayoutSize::Exact)
                            }
                            #[cfg(not(feature = "androidbox-layout-size18"))]
                            {
                                node.height == LayoutSize::WrapContent
                            }
                        }
                        || !parent_is_horizontal))
            {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
        }
        #[cfg(feature = "androidbox-layout-size18")]
        if (node.width == LayoutSize::Exact) != (node.exact_width_dp != 0)
            || (node.height == LayoutSize::Exact) != (node.exact_height_dp != 0)
        {
            return Err(InstalledInteractiveActivityError::SceneRejected);
        }
        #[cfg(feature = "androidbox-layout-mixed19")]
        if node.parent.is_some_and(|parent| {
            nodes[usize::from(parent)].orientation == LayoutOrientation::Horizontal
        }) {
            let fixed = node.kind == ViewKind::Button
                && node.layout_weight == 0
                && node.width == LayoutSize::Exact;
            let weighted = node.kind == ViewKind::Button
                && node.layout_weight != 0
                && node.width == LayoutSize::Zero;
            if !fixed && !weighted {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
        }
        #[cfg(not(feature = "androidbox-layout-size18"))]
        if node.exact_width_dp != 0 || node.exact_height_dp != 0 {
            return Err(InstalledInteractiveActivityError::SceneRejected);
        }
        #[cfg(feature = "androidbox-layout-spacing16")]
        {
            let margins = [
                node.layout_margin_left_dp,
                node.layout_margin_top_dp,
                node.layout_margin_right_dp,
                node.layout_margin_bottom_dp,
            ];
            let paddings = [
                node.padding_left_dp,
                node.padding_top_dp,
                node.padding_right_dp,
                node.padding_bottom_dp,
            ];
            if node.layout_margin_dp > 16
                || node.padding_dp > 16
                || margins.iter().any(|value| *value > 16)
                || paddings.iter().any(|value| *value > 16)
                || (margins.iter().any(|value| *value != 0) && node.kind != ViewKind::Button)
                || (paddings.iter().any(|value| *value != 0) && node.kind != ViewKind::LinearLayout)
            {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
            #[cfg(not(feature = "androidbox-layout-directional17"))]
            if margins.iter().any(|value| *value != node.layout_margin_dp)
                || paddings.iter().any(|value| *value != node.padding_dp)
            {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
        }

        let text = match node.kind {
            ViewKind::LinearLayout => {
                #[cfg(feature = "androidbox-layout-row14")]
                let orientation_valid = matches!(
                    node.orientation,
                    LayoutOrientation::Vertical | LayoutOrientation::Horizontal
                );
                #[cfg(not(feature = "androidbox-layout-row14"))]
                let orientation_valid = node.orientation == LayoutOrientation::Vertical;
                if !node.text.is_empty() || !orientation_valid {
                    return Err(InstalledInteractiveActivityError::SceneRejected);
                }
                InstalledInteractiveAscii::empty()
            }
            ViewKind::TextView | ViewKind::Button => {
                if node.id == 0
                    || node.text.as_bytes().len() > INSTALLED_ACTIVITY_SCENE_TEXT_MAX_BYTES
                    || (node.kind == ViewKind::Button && node.text.as_bytes().len() > 64)
                {
                    return Err(InstalledInteractiveActivityError::SceneRejected);
                }
                InstalledInteractiveAscii::from_ascii(node.text.as_bytes())
                    .ok_or(InstalledInteractiveActivityError::TextRejected)?
            }
        };
        let callback_registered =
            node.kind == ViewKind::Button && session.is_listener_button(node.id);
        if node.kind == ViewKind::TextView {
            if catalog_label.is_none() {
                catalog_label = Some((node.id, text));
            }
            if node.id == callback_target_id {
                callback_target_seen = true;
            }
        }
        if callback_registered {
            listener_button_count = listener_button_count
                .checked_add(1)
                .ok_or(InstalledInteractiveActivityError::SceneRejected)?;
            if listener_button.is_none() {
                listener_button = Some((node.id, text));
            }
            #[cfg(not(feature = "androidbox-multiaction3"))]
            if listener_button_count != 1 {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
        }
        scene_nodes[usize::from(index)] = InstalledInteractiveSceneNode {
            kind: node.kind,
            parent: node.parent,
            id: node.id,
            width: node.width,
            height: node.height,
            orientation: node.orientation,
            layout_weight: node.layout_weight,
            layout_margin_dp: node.layout_margin_dp,
            padding_dp: node.padding_dp,
            layout_margin_left_dp: node.layout_margin_left_dp,
            layout_margin_top_dp: node.layout_margin_top_dp,
            layout_margin_right_dp: node.layout_margin_right_dp,
            layout_margin_bottom_dp: node.layout_margin_bottom_dp,
            padding_left_dp: node.padding_left_dp,
            padding_top_dp: node.padding_top_dp,
            padding_right_dp: node.padding_right_dp,
            padding_bottom_dp: node.padding_bottom_dp,
            exact_width_dp: node.exact_width_dp,
            exact_height_dp: node.exact_height_dp,
            text,
            callback_registered,
        };
    }

    let (label_id, label_text) =
        catalog_label.ok_or(InstalledInteractiveActivityError::SceneRejected)?;
    let (button_id, button_text) =
        listener_button.ok_or(InstalledInteractiveActivityError::SceneRejected)?;
    if !callback_target_seen
        || listener_button_count != session.listener_button_count()
        || button_id != session.listener_button_id()
    {
        return Err(InstalledInteractiveActivityError::SceneRejected);
    }
    Ok(InstalledInteractiveActivitySnapshot {
        label_id,
        button_id,
        label_text,
        button_text,
        revision: 0,
        execution: session.launch_info(),
        scene_nodes,
        scene_node_count: u8::try_from(nodes.len())
            .map_err(|_| InstalledInteractiveActivityError::SceneRejected)?,
        callback_target_id,
    })
}

#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-scene-rpc2")
))]
fn unique_scene_text(
    scene: &ActivityScene,
    kind: ViewKind,
) -> Result<(u32, InstalledInteractiveAscii), InstalledInteractiveActivityError> {
    let mut found = None;
    for node in scene.nodes() {
        if node.kind == kind {
            if node.id == 0 || found.is_some() {
                return Err(InstalledInteractiveActivityError::SceneRejected);
            }
            let text = InstalledInteractiveAscii::from_ascii(node.text.as_bytes())
                .ok_or(InstalledInteractiveActivityError::TextRejected)?;
            found = Some((node.id, text));
        }
    }
    found.ok_or(InstalledInteractiveActivityError::SceneRejected)
}

#[cfg(all(test, feature = "androidbox-interactive0"))]
fn installed_apk_buffer_is_zero_for_test() -> bool {
    if interactive_lease_active() {
        return false;
    }
    // SAFETY: tests serialize every access with TEST_BUFFER_LEASE and this is
    // called only after the singleton session has been dropped.
    unsafe {
        (&*INSTALLED_APK_BUFFER.0.get())
            .iter()
            .all(|byte| *byte == 0)
    }
}

/// Reads a claimed immutable image in ABI-bounded chunks, executes it, and
/// wipes the App-local copy before returning.
pub(super) fn read_and_execute_installed_activity(
    expected: AndroidInstalledAppStatus,
    mut read: impl FnMut(usize, &mut [u8]) -> Result<(), ()>,
) -> Result<InstalledActivityExecution, InstalledActivityExecutionError> {
    #[cfg(feature = "androidbox-interactive0")]
    if interactive_lease_active() {
        // Preserve the live session's immutable backing bytes. The ordinary
        // one-shot path remains byte-for-byte unchanged whenever no
        // interactive lease is active.
        return Err(InstalledActivityExecutionError::CatalogInvalid);
    }
    // SAFETY: the single-threaded App loop is the only caller and this
    // synchronous function never exposes the buffer beyond the callback.
    let buffer = unsafe { &mut *INSTALLED_APK_BUFFER.0.get() };
    buffer.fill(0);
    let length = expected.apk_length as usize;
    if length == 0 || length > buffer.len() {
        return Err(InstalledActivityExecutionError::CatalogInvalid);
    }
    let mut offset = 0;
    let mut read_result = Ok(());
    while offset < length {
        let end = (offset + VMO_READ_MAX_BYTES).min(length);
        if read(offset, &mut buffer[offset..end]).is_err() {
            read_result = Err(InstalledActivityExecutionError::ImageReadFailed);
            break;
        }
        offset = end;
    }
    let result = match read_result {
        Ok(()) => execute_installed_activity(&buffer[..length], expected),
        Err(error) => Err(error),
    };
    buffer.fill(0);
    result
}

/// Re-verifies and executes one Resources-1 Activity entirely from EL0-owned
/// bytes.
///
/// The returned title and TextView content are reconstructed from the
/// interpreted Activity output. They are then compared with the catalog copy
/// so stale or divergent kernel admission metadata cannot reach the raster.
pub(super) fn execute_installed_activity(
    apk: &[u8],
    expected: AndroidInstalledAppStatus,
) -> Result<InstalledActivityExecution, InstalledActivityExecutionError> {
    if !expected.installed
        || expected.generation == 0
        || expected.apk_length == 0
        || expected.package.is_empty()
        || expected.activity.is_empty()
        || expected.title.is_empty()
    {
        return Err(InstalledActivityExecutionError::CatalogInvalid);
    }
    if apk.len() != expected.apk_length as usize {
        return Err(InstalledActivityExecutionError::LengthMismatch);
    }
    if sha256(apk) != expected.apk_digest_sha256 {
        return Err(InstalledActivityExecutionError::ApkDigestMismatch);
    }
    let signer =
        verify_apk_v2(apk).map_err(|_| InstalledActivityExecutionError::SignatureRejected)?;
    if signer.certificate_sha256 != expected.signer_digest_sha256 {
        return Err(InstalledActivityExecutionError::SignerMismatch);
    }

    let image =
        load_android_box(apk).map_err(|_| InstalledActivityExecutionError::ImageRejected)?;
    let manifest = image.manifest_info();
    if manifest.package.as_str() != expected.package.as_str()
        || manifest.activity_descriptor.as_str() != expected.activity.as_str()
        || manifest.version_code != expected.version_code
    {
        return Err(InstalledActivityExecutionError::IdentityMismatch);
    }
    let launch = image
        .launch_activity()
        .map_err(|_| InstalledActivityExecutionError::ImageRejected)?;
    if launch.manifest != manifest {
        return Err(InstalledActivityExecutionError::IdentityMismatch);
    }
    let resources = launch
        .resources
        .ok_or(InstalledActivityExecutionError::ResourcesMissing)?;
    let status = AndroidInstalledAppStatus::try_new(
        expected.generation,
        launch.manifest.version_code,
        expected.apk_length,
        launch.manifest.package.as_str(),
        launch.manifest.activity_descriptor.as_str(),
        launch.view.title.as_str(),
        launch.view.text.as_str(),
        signer.certificate_sha256,
        expected.apk_digest_sha256,
    )
    .ok_or(InstalledActivityExecutionError::PublisherContentMismatch)?;
    if status != expected {
        return Err(InstalledActivityExecutionError::PublisherContentMismatch);
    }

    Ok(InstalledActivityExecution {
        status,
        constructor_instruction_count: launch.constructor_instruction_count,
        on_create_instruction_count: launch.instruction_count,
        resources_arsc_crc32: resources.resources_arsc_crc32,
        layout_xml_crc32: resources.layout_xml_crc32,
        layout_resource_id: resources.layout_resource_id,
        text_resource_id: resources.text_resource_id,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        InstalledActivityExecutionError, execute_installed_activity,
        read_and_execute_installed_activity,
    };
    #[cfg(feature = "androidbox-interactive0")]
    use super::{
        InstalledInteractiveActivityError, InstalledInteractiveActivityLease,
        installed_apk_buffer_is_zero_for_test,
    };
    use bndr_ui::mobile::AndroidInstalledAppStatus;
    use std::sync::Mutex;

    static TEST_BUFFER_LEASE: Mutex<()> = Mutex::new(());

    const APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-mac-demo/androidbox-mac-demo.apk");
    #[cfg(feature = "androidbox-interactive0")]
    const INTERACTIVE_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-interactive-demo/androidbox-interactive-demo.apk"
    );
    #[cfg(feature = "androidbox-scene-rpc2")]
    const PROFILE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-profile-demo/androidbox-profile-demo.apk");
    #[cfg(feature = "androidbox-multiaction3")]
    const MULTIACTION_APK: &[u8] = include_bytes!(
        "../../../fixtures/androidbox-multiaction-demo/androidbox-multiaction-demo.apk"
    );
    #[cfg(feature = "androidbox-apk-envelope4")]
    const ENVELOPE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-envelope-demo/androidbox-envelope-demo.apk");
    const APK_SHA256: [u8; 32] = [
        0xa6, 0x04, 0xd1, 0x62, 0x98, 0xe9, 0x39, 0xd7, 0x28, 0xaa, 0x80, 0x5a, 0x63, 0x6b, 0xb1,
        0x19, 0xc9, 0x06, 0xbc, 0x1b, 0x9c, 0x22, 0x49, 0x49, 0xda, 0x44, 0x59, 0x54, 0xa4, 0xf8,
        0x36, 0x3e,
    ];
    const SIGNER_SHA256: [u8; 32] = [
        0xe7, 0x41, 0x2e, 0x1c, 0xc0, 0xff, 0xbd, 0x21, 0x00, 0x0e, 0xce, 0x51, 0x76, 0xd0, 0x08,
        0x37, 0xce, 0x27, 0x6e, 0x42, 0xe6, 0xb5, 0x2b, 0xed, 0x99, 0xcb, 0x5e, 0x62, 0x85, 0x7b,
        0x77, 0xbf,
    ];
    #[cfg(feature = "androidbox-interactive0")]
    const INTERACTIVE_APK_SHA256: [u8; 32] = [
        0x0b, 0x6c, 0x76, 0x17, 0xa6, 0xd0, 0x49, 0x1d, 0x3a, 0xc8, 0x1e, 0xa6, 0x5e, 0xb0, 0x4f,
        0x65, 0x1a, 0xd4, 0xaf, 0xe1, 0x47, 0x2e, 0xab, 0x98, 0x7a, 0x9c, 0x36, 0xe4, 0x28, 0x0c,
        0x81, 0xee,
    ];
    #[cfg(feature = "androidbox-scene-rpc2")]
    const PROFILE_APK_SHA256: [u8; 32] = [
        0x53, 0x5d, 0x92, 0xa7, 0xa8, 0xf0, 0xa1, 0x3a, 0x4d, 0x91, 0x38, 0xa5, 0x35, 0x40, 0x40,
        0x33, 0xda, 0x4e, 0x8e, 0x4e, 0xe6, 0x22, 0x1e, 0xa6, 0xfb, 0x35, 0x5d, 0xf0, 0x50, 0x75,
        0x35, 0x07,
    ];
    #[cfg(feature = "androidbox-multiaction3")]
    const MULTIACTION_APK_SHA256: [u8; 32] = [
        0xae, 0xcf, 0x07, 0x49, 0xe2, 0xc0, 0x63, 0xf4, 0x49, 0x45, 0xf6, 0x15, 0x4b, 0x47, 0x97,
        0x14, 0xfa, 0x8e, 0xbe, 0xad, 0x8d, 0xa2, 0x6b, 0x0e, 0xbe, 0xf0, 0x05, 0xd2, 0x73, 0x70,
        0x36, 0xc6,
    ];
    #[cfg(feature = "androidbox-apk-envelope4")]
    const ENVELOPE_APK_SHA256: [u8; 32] = [
        0xa6, 0x55, 0x84, 0x44, 0x1a, 0x52, 0x46, 0x98, 0xbc, 0xae, 0x48, 0x10, 0xe5, 0x58, 0xbc,
        0x6b, 0x94, 0x7e, 0xb8, 0x12, 0x75, 0xfa, 0x5f, 0x0f, 0x5d, 0xf9, 0x14, 0x30, 0x46, 0x47,
        0xe3, 0xc5,
    ];

    fn expected() -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            1,
            1,
            APK.len() as u32,
            "org.bndroid.macdemo",
            "Lorg/bndroid/macdemo/MainActivity;",
            "Mac-built Android app",
            "Hello from a Mac-built APK",
            SIGNER_SHA256,
            APK_SHA256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-interactive0")]
    fn interactive_expected() -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            2,
            1,
            INTERACTIVE_APK.len() as u32,
            "org.bndroid.interactive",
            "Lorg/bndroid/interactive/MainActivity;",
            "Interactive Android app",
            "Ready for a real APK click",
            SIGNER_SHA256,
            INTERACTIVE_APK_SHA256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn profile_expected() -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            3,
            1,
            PROFILE_APK.len() as u32,
            "org.bndroid.profile",
            "Lorg/bndroid/profile/MainActivity;",
            "Profile Android app",
            "Account profile",
            SIGNER_SHA256,
            PROFILE_APK_SHA256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-multiaction3")]
    fn multiaction_expected() -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            4,
            1,
            MULTIACTION_APK.len() as u32,
            "org.bndroid.multiaction",
            "Lorg/bndroid/multiaction/MainActivity;",
            "Multi-action Android app",
            "Review request",
            SIGNER_SHA256,
            MULTIACTION_APK_SHA256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    fn envelope_expected() -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            5,
            1,
            ENVELOPE_APK.len() as u32,
            "org.bndroid.envelope",
            "Lorg/bndroid/envelope/MainActivity;",
            "Envelope Android app",
            "Envelope review",
            SIGNER_SHA256,
            ENVELOPE_APK_SHA256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_fixed_capacity_type_sizes_are_bounded() {
        assert!(core::mem::size_of::<bndr_androidbox::AndroidBox<'static>>() <= 4 * 1024);
        assert!(core::mem::size_of::<bndr_androidbox::ActivitySession<'static>>() <= 4 * 1024);
        assert!(core::mem::size_of::<super::InstalledInteractiveActivitySnapshot>() <= 4 * 1024);
        assert!(
            core::mem::size_of::<(
                bndr_androidbox::ActivitySession<'static>,
                super::InstalledInteractiveActivitySnapshot,
            )>() <= 8 * 1024
        );
        assert!(core::mem::size_of::<super::InteractiveLeaseState>() <= 4 * 1024);
    }

    #[test]
    fn mac_built_apk_is_reverified_executed_and_reconstructed_in_el0_shape() {
        let _lease = TEST_BUFFER_LEASE.lock().unwrap();
        let execution = read_and_execute_installed_activity(expected(), |offset, out| {
            out.copy_from_slice(&APK[offset..offset + out.len()]);
            Ok(())
        })
        .unwrap();
        assert_eq!(execution.status, expected());
        assert_eq!(execution.constructor_instruction_count, 2);
        assert_eq!(execution.on_create_instruction_count, 4);
        assert_eq!(execution.layout_resource_id, 0x7f02_0000);
        assert_eq!(execution.text_resource_id, 0x7f03_0000);
        assert_ne!(execution.resources_arsc_crc32, 0);
        assert_ne!(execution.layout_xml_crc32, 0);
    }

    #[test]
    fn byte_mutation_and_catalog_drift_fail_before_publisher_content_is_returned() {
        let _lease = TEST_BUFFER_LEASE.lock().unwrap();
        let mut mutated = [0_u8; 12_566];
        mutated.copy_from_slice(APK);
        mutated[128] ^= 1;
        assert_eq!(
            execute_installed_activity(&mutated, expected()),
            Err(InstalledActivityExecutionError::ApkDigestMismatch)
        );

        let mut drifted = expected();
        drifted.version_code = 2;
        assert_eq!(
            execute_installed_activity(APK, drifted),
            Err(InstalledActivityExecutionError::IdentityMismatch)
        );

        assert_eq!(
            read_and_execute_installed_activity(expected(), |_, _| Err(())),
            Err(InstalledActivityExecutionError::ImageReadFailed)
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn interactive_el0_lease_retains_session_dispatches_and_wipes_on_close() {
        let _guard = TEST_BUFFER_LEASE.lock().unwrap();
        assert!(installed_apk_buffer_is_zero_for_test());
        let mut lease = InstalledInteractiveActivityLease::new();
        assert_eq!(
            lease.dispatch_button_click(0x7f01_0000),
            Err(InstalledInteractiveActivityError::NotActive)
        );
        assert_eq!(
            lease.close(),
            Err(InstalledInteractiveActivityError::NotActive)
        );

        let mut reentrant = InstalledInteractiveActivityLease::new();
        let mut reentrant_checked = false;
        let initial = lease
            .open(interactive_expected(), |offset, out| {
                if !reentrant_checked {
                    assert_eq!(
                        reentrant.open(interactive_expected(), |_, _| Ok(())),
                        Err(InstalledInteractiveActivityError::AlreadyActive)
                    );
                    reentrant_checked = true;
                }
                out.copy_from_slice(&INTERACTIVE_APK[offset..offset + out.len()]);
                Ok(())
            })
            .expect("open interactive EL0 lease");
        assert!(reentrant_checked);
        assert_eq!(initial.label_id, 0x7f01_0001);
        assert_eq!(initial.button_id, 0x7f01_0000);
        assert_eq!(initial.label_text.as_str(), "Ready for a real APK click");
        assert_eq!(initial.button_text.as_str(), "Update text");
        assert_eq!(initial.revision, 0);
        assert_eq!(initial.execution.constructor_instruction_count, 2);
        assert_eq!(initial.execution.on_create_instruction_count, 8);

        let mut competing = InstalledInteractiveActivityLease::new();
        let mut competing_read = false;
        assert_eq!(
            competing.open(interactive_expected(), |_, _| {
                competing_read = true;
                Ok(())
            }),
            Err(InstalledInteractiveActivityError::AlreadyActive)
        );
        assert!(!competing_read);
        assert_eq!(
            read_and_execute_installed_activity(interactive_expected(), |_, _| Ok(())),
            Err(InstalledActivityExecutionError::CatalogInvalid)
        );

        assert_eq!(
            lease.dispatch_button_click(initial.label_id),
            Err(InstalledInteractiveActivityError::ClickRejected)
        );
        let first = lease
            .dispatch_button_click(initial.button_id)
            .expect("real onClick callback");
        assert_eq!(first.label_id, initial.label_id);
        assert_eq!(first.label_text.as_str(), "Button callback executed");
        assert_eq!(first.revision, 1);
        assert_eq!(first.execution.clicked_view_id, initial.button_id);
        assert_eq!(first.execution.changed_view_id, initial.label_id);
        assert_eq!(first.execution.text_resource_id, 0x7f03_0003);
        assert_eq!(first.execution.instruction_count, 7);

        let repeated = lease
            .dispatch_button_click(initial.button_id)
            .expect("repeat callback");
        assert_eq!(repeated.label_text.as_str(), "Button callback executed");
        assert_eq!(repeated.revision, 2);

        lease.close().expect("close and wipe");
        assert!(installed_apk_buffer_is_zero_for_test());
        assert_eq!(
            lease.dispatch_button_click(initial.button_id),
            Err(InstalledInteractiveActivityError::NotActive)
        );
        assert_eq!(
            lease.close(),
            Err(InstalledInteractiveActivityError::NotActive)
        );
        assert!(installed_apk_buffer_is_zero_for_test());
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn interactive_el0_open_failures_leave_no_session_or_apk_bytes() {
        let _guard = TEST_BUFFER_LEASE.lock().unwrap();
        let mut lease = InstalledInteractiveActivityLease::new();
        assert_eq!(
            lease.open(interactive_expected(), |offset, out| {
                let available = &INTERACTIVE_APK[offset..offset + out.len()];
                out.copy_from_slice(available);
                Err(())
            }),
            Err(InstalledInteractiveActivityError::ImageReadFailed)
        );
        assert!(installed_apk_buffer_is_zero_for_test());
        assert_eq!(
            lease.dispatch_button_click(0x7f01_0000),
            Err(InstalledInteractiveActivityError::NotActive)
        );

        let mut wrong_digest = interactive_expected();
        wrong_digest.apk_digest_sha256[0] ^= 1;
        assert_eq!(
            lease.open(wrong_digest, |offset, out| {
                out.copy_from_slice(&INTERACTIVE_APK[offset..offset + out.len()]);
                Ok(())
            }),
            Err(InstalledInteractiveActivityError::ApkDigestMismatch)
        );
        assert!(installed_apk_buffer_is_zero_for_test());
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    fn scene_rpc2_retains_nested_multitext_scene_and_updates_only_callback_target() {
        use bndr_androidbox::{LayoutOrientation, LayoutSize, ViewKind};

        let _guard = TEST_BUFFER_LEASE.lock().unwrap();
        assert!(installed_apk_buffer_is_zero_for_test());
        let mut lease = InstalledInteractiveActivityLease::new();
        let initial = lease
            .open(profile_expected(), |offset, out| {
                out.copy_from_slice(&PROFILE_APK[offset..offset + out.len()]);
                Ok(())
            })
            .expect("open nested profile Activity");

        assert_eq!(initial.label_id, 0x7f01_0002);
        assert_eq!(initial.label_text.as_str(), "Account profile");
        assert_eq!(initial.button_id, 0x7f01_0000);
        assert_eq!(initial.button_text.as_str(), "Verify profile");
        assert_eq!(initial.callback_target_id, 0x7f01_0001);
        assert_eq!(initial.scene_node_count, 5);
        let nodes = &initial.scene_nodes[..usize::from(initial.scene_node_count)];
        assert_eq!(nodes[0].kind, ViewKind::LinearLayout);
        assert_eq!(nodes[0].parent, None);
        assert_eq!(nodes[0].width, LayoutSize::MatchParent);
        assert_eq!(nodes[0].height, LayoutSize::MatchParent);
        assert_eq!(nodes[0].orientation, LayoutOrientation::Vertical);
        assert_eq!(nodes[1].kind, ViewKind::TextView);
        assert_eq!(nodes[1].parent, Some(0));
        assert_eq!(nodes[1].id, initial.label_id);
        assert_eq!(nodes[1].text.as_str(), "Account profile");
        assert_eq!(nodes[2].kind, ViewKind::LinearLayout);
        assert_eq!(nodes[2].parent, Some(0));
        assert_eq!(nodes[3].kind, ViewKind::TextView);
        assert_eq!(nodes[3].parent, Some(2));
        assert_eq!(nodes[3].id, initial.callback_target_id);
        assert_eq!(nodes[3].text.as_str(), "Profile status: pending");
        assert_eq!(nodes[4].kind, ViewKind::Button);
        assert_eq!(nodes[4].parent, Some(2));
        assert_eq!(nodes[4].id, initial.button_id);
        assert_eq!(nodes[4].text.as_str(), "Verify profile");
        assert!(nodes[4].callback_registered);
        assert!(!nodes[1].callback_registered);
        assert!(!nodes[3].callback_registered);

        let update = lease
            .dispatch_button_click(initial.button_id)
            .expect("dispatch exact listener");
        assert_eq!(update.label_id, initial.callback_target_id);
        assert_eq!(update.label_text.as_str(), "Profile status: verified");
        assert_eq!(update.execution.changed_view_id, initial.callback_target_id);
        assert_eq!(update.revision, 1);
        lease.close().expect("close nested scene");
        assert!(installed_apk_buffer_is_zero_for_test());
    }

    #[cfg(feature = "androidbox-multiaction3")]
    #[test]
    fn multiaction3_retains_two_buttons_and_dispatches_both_dex_branches() {
        use bndr_androidbox::ViewKind;

        let _guard = TEST_BUFFER_LEASE.lock().unwrap();
        assert!(installed_apk_buffer_is_zero_for_test());
        let mut lease = InstalledInteractiveActivityLease::new();
        let initial = lease
            .open(multiaction_expected(), |offset, out| {
                out.copy_from_slice(&MULTIACTION_APK[offset..offset + out.len()]);
                Ok(())
            })
            .expect("open multi-action Activity");

        assert_eq!(initial.label_id, 0x7f01_0003);
        assert_eq!(initial.label_text.as_str(), "Review request");
        assert_eq!(initial.button_id, 0x7f01_0000);
        assert_eq!(initial.button_text.as_str(), "Approve");
        assert_eq!(initial.callback_target_id, 0x7f01_0002);
        assert_eq!(initial.execution.listener_button_count, 2);
        assert_eq!(
            &initial.execution.listener_button_ids[..2],
            &[0x7f01_0000, 0x7f01_0001]
        );
        assert_eq!(initial.scene_node_count, 5);
        let nodes = &initial.scene_nodes[..usize::from(initial.scene_node_count)];
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.kind == ViewKind::Button && node.callback_registered)
                .count(),
            2
        );

        let approved = lease
            .dispatch_button_click(0x7f01_0000)
            .expect("dispatch approve branch");
        assert_eq!(approved.label_id, 0x7f01_0002);
        assert_eq!(approved.label_text.as_str(), "Decision: approved");
        assert_eq!(approved.execution.text_resource_id, 0x7f03_0003);
        assert_eq!(approved.revision, 1);

        let rejected = lease
            .dispatch_button_click(0x7f01_0001)
            .expect("dispatch reject branch");
        assert_eq!(rejected.label_id, 0x7f01_0002);
        assert_eq!(rejected.label_text.as_str(), "Decision: rejected");
        assert_eq!(rejected.execution.text_resource_id, 0x7f03_0005);
        assert_eq!(rejected.revision, 2);

        assert_eq!(
            lease.dispatch_button_click(0x7f01_0002),
            Err(InstalledInteractiveActivityError::ClickRejected)
        );
        lease.close().expect("close multi-action scene");
        assert!(installed_apk_buffer_is_zero_for_test());
    }

    #[cfg(feature = "androidbox-apk-envelope4")]
    #[test]
    fn envelope4_opens_unrepacked_aapt2_apk_and_dispatches_both_branches() {
        use bndr_androidbox::{AndroidBox, AndroidBoxEnvelopeScratch, ViewKind};

        let _guard = TEST_BUFFER_LEASE.lock().unwrap();
        assert!(installed_apk_buffer_is_zero_for_test());
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        let image = AndroidBox::load_with_scratch(ENVELOPE_APK, &mut scratch)
            .expect("parse un-repacked aapt2 APK envelope");
        let envelope = image.envelope_info().expect("Envelope-4 evidence");
        assert!(envelope.manifest_deflated());
        assert!(envelope.layout_deflated());
        assert_eq!(envelope.data_descriptor_count(), 3);
        assert_eq!(envelope.unconsumed_deflated_count(), 1);

        let mut lease = InstalledInteractiveActivityLease::new();
        let initial = lease
            .open(envelope_expected(), |offset, out| {
                out.copy_from_slice(&ENVELOPE_APK[offset..offset + out.len()]);
                Ok(())
            })
            .expect("open un-repacked aapt2 Activity");

        assert_eq!(initial.label_id, 0x7f01_0003);
        assert_eq!(initial.label_text.as_str(), "Envelope review");
        assert_eq!(initial.callback_target_id, 0x7f01_0002);
        assert_eq!(initial.scene_node_count, 5);
        assert_eq!(
            initial.scene_nodes[..usize::from(initial.scene_node_count)]
                .iter()
                .filter(|node| node.kind == ViewKind::Button && node.callback_registered)
                .count(),
            2
        );

        let approved = lease
            .dispatch_button_click(0x7f01_0000)
            .expect("dispatch approve branch");
        assert_eq!(approved.label_text.as_str(), "Decision: approved");
        assert_eq!(approved.revision, 1);

        let rejected = lease
            .dispatch_button_click(0x7f01_0001)
            .expect("dispatch reject branch");
        assert_eq!(rejected.label_text.as_str(), "Decision: rejected");
        assert_eq!(rejected.revision, 2);

        lease.close().expect("close Envelope-4 scene");
        assert!(installed_apk_buffer_is_zero_for_test());
    }
}
