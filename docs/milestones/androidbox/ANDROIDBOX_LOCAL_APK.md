# AndroidBox manifest-component Resources-1 contract

Status: completed and evidenced for one Android APK created locally on the Mac,
then installed, recovered, and launched twice by an offline local-QEMU gate.
The completed `CompatibleActivitySession-0` path retains one capacity-one,
boot-local compatible identity across Home and Overview, freshly re-verifies
the durable APK before recent activation, and clears that identity on Back.
Its DEX deliberately contains no repository-specific DEX-0 probe class, so
this proves exact Manifest-to-Activity component binding, the bounded
`<init>()V`-then-`onCreate` ActivityLifecycle-1 path, and generation-bound
fresh durable relaunch for the Resources-1 subset. It does not establish
arbitrary APK support, ART, a standalone Android application process, or
general Android compatibility.

## Locally built artifact

The fixture is:

```text
fixtures/androidbox-mac-demo/androidbox-mac-demo.apk
```

Its exact admitted identity is:

| Field | Value |
|---|---|
| byte length | `12566` |
| APK SHA-256 | `a604d16298e939d728aa805a636bb119c906bc1b9c224949da445954a4f8363e` |
| package | `org.bndroid.macdemo` |
| launcher Activity | `Lorg/bndroid/macdemo/MainActivity;` |
| `versionCode` / `versionName` | `1` / `1.0` |
| title | `Mac-built Android app` |
| TextView text | `Hello from a Mac-built APK` |
| signature | APK Signature Scheme v2 only, one signer |
| signer-certificate SHA-256 | `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf` |
| compatibility profile | `Resources-1` |

`fixtures/androidbox-mac-demo/build.sh` uses the already installed local
Android SDK and builds offline. It compiles the manifest, Java, resources, and
binary layout, creates exactly four STORED/aligned APK entries, signs the same
unsigned artifact twice, and requires deterministic v2-only output. The
fixture references the repository-public test-only key under
`fixtures/androidbox-resource-demo`; no signing material is copied into the
Mac fixture directory. That key is not a production publisher identity,
product trust anchor, provisioning mechanism, or HSM-custody claim.

The pinned artifact hash is also stored in:

```text
fixtures/androidbox-mac-demo/androidbox-mac-demo.apk.sha256
```

## Manifest-driven component binding

The manifest and launcher identity are genuinely
`org.bndroid.macdemo` / `Lorg/bndroid/macdemo/MainActivity;`. The generated
`classes.dex` deliberately does **not** contain:

```text
Lorg/bndroid/demo/Main;
boot()
onTap(int)
```

AndroidBox parses the binary manifest, resolves the unique exported
`MAIN`/`LAUNCHER` component, converts it to an exact DEX descriptor, binds that
class, rejects fields/interfaces, requires one exact public no-argument
constructor, and validates its Activity superclass and `onCreate(Bundle)`
method. The pure-data lifecycle interpreter then executes the constructor's
two allowed instructions—`invoke-direct Activity.<init>()V` and
`return-void`—before the admitted `onCreate` subset. The historical DEX-0
diagnostics remain available only to the old embedded demo; missing or invalid
diagnostics no longer participate in APK component admission.

The Activity evidence still uses the narrow Resources-1 lifecycle and layout:
one exported `MAIN`/`LAUNCHER` Activity, real
`onCreate(Bundle)→setContentView(int)`, one compiled root `TextView`, and one
default-configuration string resource. There is no ART, ActivityThread,
Binder, Bionic, JNI, native-library loading, arbitrary View inflation, resource
qualifier selection, Android permission bridge, service runtime, or general
APK execution.

## Generic offline local-APK gate

The gate is:

```text
scripts/check-androidbox-local-apk.sh
```

It accepts one explicit absolute APK path, scans no directory for APKs, uses
the local official `apksigner`, `aapt2`, and `dexdump` tools as independent host checks,
pins `TMPDIR` inside each run's target evidence tree, and runs QEMU with
networking disabled:

```bash
./scripts/check-androidbox-local-apk.sh \
  --apk "$PWD/fixtures/androidbox-mac-demo/androidbox-mac-demo.apk"
```

“Generic” here means the checker derives the bounded package, Activity,
version, title, TextView, resource IDs/CRCs, APK digest, and signer digest from
the explicitly supplied file instead of hard-coding the repository demo
identity. It also requires exactly one manifest-selected Activity descriptor
in DEX and rejects the retired fixed probe class. Admission remains restricted
to the exact v2-only, one-signer, four-entry Resources-1 shape. It is not a
general Android APK gate.

The current completed evidence directory is:

```text
target/androidbox-local-apk/check.4rO29p/
```

The gate performs two boots over one persistent 16 MiB disk:

1. **install** — the explicit 12566-byte Mac APK becomes generation 1 / slot 0
   with `reads=1032 writes=130 flushes=3`; only the Activity result from full
   package-store readback is publishable to the UI. A separate source
   execution occurs before mutation as a fail-closed admission dry-run.
2. **recovery and compatible-session relaunch** — no APK source is supplied;
   generation 1 is recovered with `reads=389 writes=0 flushes=0`. The QEMU
   touch path launches once from All apps, goes Home, opens the identity-only
   Overview card, and activates that recent once. Request sequences 1 and 2
   each start a fresh read-only service pass with
   `reads=389 writes=0 flushes=0`. Back then finishes the session and the
   following Overview is empty and inert.

The complete recovery disk is byte-for-byte unchanged before and after both
icon launches. Both boots emit byte-identical
`ANDROIDBOX_INSTALLED_ACTIVITY_OK` evidence:

```text
title=Mac-built Android app
text=Hello from a Mac-built APK
constructor=1
constructor_method=3
constructor_code_offset=588
constructor_instructions=2
on_create=1
on_create_method=4
on_create_code_offset=612
on_create_instructions=4
resources_arsc_crc=350773751
layout_xml_crc=919508420
layout_resource_id=2130837504
string_resource_id=2130903040
```

The retained whole-disk SHA-256 changes from the deterministic empty image
`b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801`
to generation 1
`6becd521ff9377e34c18cc6eaec692c361a3d371b2e8fbae1960489cbc66eaf1`;
the source-free recovery preserves the latter hash.

Key fields retained from the terminal marker are:

```text
ANDROIDBOX_LOCAL_APK_QEMU_OK artifact_dir=/Users/apple/Desktop/Bndroid/target/androidbox-local-apk/check.4rO29p ... durable_relaunches=2 relaunch_sequence1_reads=389 relaunch_sequence2_reads=389 relaunch_writes=0 relaunch_flushes=0 recovery_disk_unchanged=1 compatible_activity_session=1 compatible_activity_generation=1 compatible_activity_home_recent=1 compatible_activity_overview_recent=1 compatible_activity_recent_relaunch=1 compatible_activity_finish_cleared_recent=1 overview_activity_pixels=0 overview_thumbnail=0 overview_live_preview=0 overview_background_execution=0 empty_overview_tap_inert=1 activity_sequence1_ppm_sha256=e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84 activity_sequence2_ppm_sha256=e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84 compatible_overview_ppm_sha256=ff4012ba8d990bb6defc2944fc44021679ce5da72835d456a5446589f610e500 empty_overview_ppm_sha256=19363d900d0a489676b1ca24ada784a167609a626fd899a3b263d3ccd3bcef44 network=disabled general_apk_claim=0 android_compatibility_claim=0
```

This is a two-boot local-QEMU transaction result. It is not physical-power
evidence, a real-device result, or proof that applications outside Resources-1
work.

## ABI 45 EL0Runtime-0 child path

`androidbox-el0-runtime0` is an opt-in child profile. It does not replace or
renumber the completed ABI 44 local-APK gate below. The parent keeps syscall
59/60 and `BUE1` v6; the child selects ABI 45, retains `BUC1` v8, uses
`BUE1` v7 for authenticated `App/None` compatible focus, and adds syscall 61
`AndroidPackageImageClaim`.

In the child, Launcher still submits the generation-bound syscall 60 relaunch,
now with the explicit nonzero compatible-session ID. The IRQ-enabled monitor
freshly recovers and reads the package blob, performs the bounded admission
checks, and publishes one immutable VMO grant addressed to the resident App.
Launcher receives only the canonical 640-byte package snapshot: it receives no
APK bytes, image handle, package-store handle, storage authority, or Activity
pixels, and it does not execute or render the Activity.

After SurfaceServer publishes the accepted Foreground state and
`active_client=App, app=None`, App supplies an exact canonical 128-byte
`BNDACM01` claim to syscall 61. The claim binds compatible session, package
generation, APK length, APK SHA-256, signer SHA-256, and Resources-1 profile.
It is one-shot and App-only. The returned VMO handle has exactly `READ`;
duplicate, transfer, map, write, execute, wait, and storage authority are all
absent. App reads it in ABI-bounded chunks, independently rechecks the APK
digest, APK-v2 signer, Manifest package/Activity/version, and Resources-1
result, executes the admitted constructor/`onCreate` subset, wipes its private
APK buffer, and submits the Activity raster through its own graphics buffer.

The kernel monitor still invokes the existing bounded AndroidBox interpreter
for admission/profile validation before the immutable VMO is published.
EL0Runtime-0 moves foreground execution verification and visible raster
ownership to App EL0; it is not evidence that every APK parser/interpreter step
has left EL1.

The completed offline child gate is:

```text
scripts/check-androidbox-el0-runtime0.sh
target/androidbox-el0-runtime0/check.R6tuKT/
```

Its `ANDROIDBOX_EL0_RUNTIME0_QEMU_OK` terminal record and serial log prove ABI
45, two Launcher collections with
`apk_bytes_exposed_to_launcher=0 authority_granted=app-read-only-vmo`, two
`ANDROID_PACKAGE_IMAGE_CLAIM_OK` lines with `owner=<App>` and `rights=READ`,
App-owned Activity presents, Launcher-owned Home/Overview, Back clearing the
recent, an unchanged whole-package disk, and no panic.
`activity-sequence1.ppm` and `activity-sequence2.ppm` are byte-identical with
SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`.
This child terminal marker is separate from, and does not replace, the
completed ABI 44 `ANDROIDBOX_LOCAL_APK_QEMU_OK` gate.

The process move changes ownership, not compatibility scope. Resources-1
remains one static compiled root `TextView` whose `android:text` resolves one
default-configuration string under one exported `MAIN`/`LAUNCHER` Activity and
one admitted constructor→`onCreate` shape. It adds no ART, Dalvik,
ActivityThread, Android Framework, Binder, Bionic, JNI, native libraries,
arbitrary View inflation, resource qualifier/alias handling, Android
permissions/services/networking, general APK execution, phone hardware, or
real-device proof.

## ABI 46 InteractiveActivity-1 completed offline gate

The separate opt-in `androidbox-interactive0` child advances only its own
profile to ABI 46. ABI 45 remains unchanged. Its exact offline Mac fixture is:

| Field | Value |
|---|---|
| path | `fixtures/androidbox-interactive-demo/androidbox-interactive-demo.apk` |
| byte length | `12566` |
| APK SHA-256 | `0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee` |
| package | `org.bndroid.interactive` |
| launcher Activity | `Lorg/bndroid/interactive/MainActivity;` |
| title | `Interactive Android app` |
| initial TextView | `Ready for a real APK click` |
| Button | `Update text` |
| clicked TextView | `Button callback executed` |
| compatibility profile | strict `InteractiveActivity-1` child of `Resources-1` |

The APK is deterministic and built offline with the installed Android SDK.
Its compiled layout uses the standard Android
`LinearLayout`/`TextView`/`Button` classes. The Activity implements
`View.OnClickListener`; `onCreate` binds the Button with
`setOnClickListener`, and the retained `onClick(View)` method executes
`findViewById`, `check-cast TextView`, and `TextView.setText(int)`.

In this profile the authenticated App EL0 generation claims the immutable APK
through syscall 61, re-verifies it, and retains one bounded
`ActivitySession` for the foreground compatible session. Button release is
dispatched to that retained session in App EL0, changes the exact TextView
resource from `Ready for a real APK click` to
`Button callback executed`, increments the session revision, and produces a
new App-owned content frame. Leaving the compatible foreground closes the
retained session; this is still a capacity-bounded interpreter, not ART.

The physical preview raster remains 720x1600, but ABI 46 gives the client only
the content viewport `(x=0,y=64,width=720,height=1448)`. SurfaceServer alone
renders rows `0..64` and `1512..1600`, corresponding to the top 64-pixel
status area and bottom 88-pixel navigation area. Syscall 62,
`SurfacePresentBufferLayers`, validates two distinct generation-bound source
buffers in full and commits their content-plus-system-chrome composition
atomically. Client pixels in the chrome rows are ignored, and system-chrome
input is consumed by SurfaceServer.

The initial QEMU run reached the lower unmapped guard of the old 32 KiB EL0
stack while constructing retained APK/session state. The ABI 46 child now uses
a 64 KiB user stack plus one unmapped guard page at each end. This is a
feature-scoped correction; it neither changes ABI 45 nor weakens the two-guard
mapping invariant.

The completed automated checker and retained evidence tree are:

```text
scripts/check-androidbox-interactive0.sh
target/androidbox-interactive0/check.4sMhMR/
```

The checker completes exactly two network-disabled offline QEMU boots. The
install boot supplies the explicit APK through `fw_cfg`; the second boot has
no APK source and recovers generation 1 from the same package disk. Its
terminal result is `ANDROIDBOX_INTERACTIVE0_QEMU_OK`.

The ABI 46 runtime proves exactly three graphics-buffer identities and six
handles with the roles SurfaceServer, Launcher, and App. Each identity has
one producer and one server handle. The completed interaction records 13 App
layered commits and 16 Launcher layered commits. The real Button receives
three input samples and produces two callback frames. Its clicked raster
changes 4138 content pixels, including 4020 text-label pixels, while the top
64 and bottom 88 chrome regions remain byte-for-byte identical.

The initial and relaunched Activity PPMs are byte-for-byte identical with
SHA-256
`69e6e61e005c2762c6cf4f53168b8929783df402075d53d3ecdebc448efd8209`.
The clicked PPM SHA-256 is
`941bac90d2e38de106420c8c1ed2797dd0670e2b1e83fb8114c3d90fed8f1baf`.
Top, Home, and bottom-corner probes all prove `samples_to_app=0`; the corner
probe leaves both the claim count and raster unchanged. The source-free
recovery disk remains byte-for-byte unchanged with SHA-256
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`.
Both boots keep networking disabled and finish without panic or user fault.

The associated completed host suites contain 243 UI tests, 82 AndroidBox
tests, 13 init runtime tests, and 377 kernel tests.

The exact negative boundary is:

```text
art=0 binder=0 bionic=0 jni=0 network=0 general_apk=0
real_phone=0 general_android_compatibility=0
standalone_android_process=0 independent_android_app_process=0
```

InteractiveActivity-1 does not make Bndroid a real phone or a general Android
runtime. This ABI 46 parent intentionally hosts the retained session inside
the existing App EL0 process. The separate ABI 47 child documented below
introduces the dedicated `AndroidApp` image without retroactively changing
this ABI 46 ownership or evidence.

## ABI 47 Process-0 completed offline gate

The opt-in `androidbox-process0` child advances only its own profile to ABI
47; ABI 44/45/46 remain unchanged. It uses the same deterministic
InteractiveActivity-1 APK: 12566 bytes, APK SHA-256
`0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee`,
and test-only signer-certificate SHA-256
`e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.
Process-0 changes execution ownership, not compatibility admission.

ABI 47 adds a separately built `AndroidApp` EL0 image (image ID 10).
AndroidApp owns the immutable APK readback, independent digest/APK-v2
signature/signer and Manifest verification, bounded interpreter, and retained
`ActivitySession`. App (image ID 7) remains the trusted UI host: it owns the
Surface endpoint, receives input, validates the bounded scene response, and
produces Activity pixels. AndroidApp receives no Surface, graphics, input,
storage, or system-channel handle.

Init creates one private App/AndroidApp Channel pair. The AndroidApp endpoint
has exactly `READ|WRITE|WAIT` (`0x00000103`) and cannot be duplicated or
transferred; at steady state it is the worker's only handle. The trusted App
endpoint retains `CHANNEL_DEFAULT`. Syscall 59 exposes only canonical metadata
to Launcher, App, and AndroidApp. Launcher-only syscall 60 performs the fresh
durable readback and publishes a one-shot grant bound to the exact Launcher
and AndroidApp generations, compatible session, package generation, APK
length, APK digest, and signer digest. Only AndroidApp can use syscall 61 to
claim the image. The returned VMO handle has exactly `READ`, with no
duplicate, transfer, map, write, execute, wait, or storage authority. Launcher
and App receive no APK bytes.

The kernel monitor still runs the existing bounded admission/profile pass
before publishing the immutable VMO. Process-0 therefore does not establish
that all APK parsing or interpretation has left EL1.

App and AndroidApp exchange canonical fixed-64-byte `BNDAPC01` messages over
the private kernel-sender-stamped Channel. The bounded lifecycle is
`Ready → Open/Opened/chunks → Click/Updated/chunk → Close/Closed`.
Request IDs are strictly ordered `0/1/2/3`, the sender PID is authenticated,
and session, package generation, view IDs, and revision are checked. There is
at most one outstanding request; each Channel direction has capacity 8, and
the maximum admitted Open response fits exactly within that capacity.
AndroidApp receives no direct touch input.

The completed checker and retained evidence are:

```text
scripts/check-androidbox-process0.sh
target/androidbox-process0/check.sFpefj/
ANDROIDBOX_PROCESS0_QEMU_OK
```

It performs exactly two offline, network-disabled QEMU boots. The first
supplies the APK through explicit `fw_cfg` and installs generation 1 with
`reads=1032 writes=130 flushes=3`. The second supplies no APK source, recovers
the package with `reads=389 writes=0 flushes=0`, opens it through AndroidApp,
dispatches one real Button callback, and closes the worker session.

The App ELF is 427528 bytes with SHA-256
`d133b012ff4939eeee783aec05a2c501895425be10b95276d6925a587fec8184`.
The AndroidApp ELF is 157856 bytes with SHA-256
`478eb4e647e7c1fd4a0e7d49dad6d771e1b0144e97079174a1cc96fa97e3d9b0`.
`ANDROID_APP_PROCESS_OK` records distinct App/AndroidApp PIDs
`4294967304/4294967305`, ASIDs `8/9`, translation roots
`0x0000000041b2e000/0x0000000041b73000`, and catalog digests
`0xb7b6c173e5230626/0x52ccab253f643bae`. The private graph contains exactly
two endpoints and one pair. AndroidApp is live with one Channel handle, no
unexpected handle, empty queues at convergence, and no Surface, graphics,
input, or storage handle.

The canonical `ANDROID_APP_RPC_OK` transcript records Ready/Open/Opened,
two label chunks of `24/2` bytes, one 11-byte Button chunk, Click/Updated,
one 24-byte update chunk, and Close/Closed. It finishes at revision 1 with
zero errors, `max_outstanding=1`, `queue_capacity=8`, and empty queues. App
produces 7 layered commits, Launcher 11, and AndroidApp 0. Three App-routed
Button samples produce two callback frames; no input sample reaches
AndroidApp.

The pre-click and updated 720x1600 guest PPM SHA-256 values are respectively:

```text
69e6e61e005c2762c6cf4f53168b8929783df402075d53d3ecdebc448efd8209
941bac90d2e38de106420c8c1ed2797dd0670e2b1e83fb8114c3d90fed8f1baf
```

The callback changes 4138 content pixels, including 4020 text-label pixels,
while the top 64 and bottom 88 SurfaceServer-owned chrome rows remain
byte-identical. The deterministic empty package disk SHA-256 is
`b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801`.
After installation, before recovery, and after the complete source-free RPC
lifecycle it is identically
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`.

The exact boundary is:

```text
abi47_execution_owner=android-app-el0-read-only-vmo
abi47_ui_owner=trusted-app-el0
abi47_launcher_apk_bytes=0 abi47_app_apk_bytes=0
abi47_android_app_apk_bytes=1
abi47_android_app_surface=0 abi47_android_app_graphics=0
abi47_android_app_input=0 abi47_android_app_storage=0
abi47_rpc=BNDAPC01 abi47_rpc_max_outstanding=1
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 package_manager_api=0 general_apk_claim=0
general_android_compatibility=0 standalone_general_android_runtime=0
background_execution=0 crash_recovery_claim=0 network=disabled
emulator_only=1 real_phone_claim=0
```

The profile marker's `standalone_android_process=1` means only that
AndroidApp is a distinct Bndroid process and address space. It is not ART,
ActivityThread, Android Framework, Binder, Bionic, JNI, a general Android
application runtime, arbitrary APK compatibility, or a real phone. The
successful no-panic gate is not AndroidApp crash-restart, rebind,
peer-close-recovery, or availability-isolation evidence. The 720x1600 value is
a QEMU guest raster, not a physical panel, DPI, refresh-rate, or real-device
claim.

## ABI 44 durable icon relaunch

Syscall 59 remains the boot-catalog read: it returns one canonical 640-byte
`BNDAPS01` installed-package snapshot without exposing APK bytes, a
package-store handle, or storage authority. ABI 44 adds Launcher-only syscall
60 for relaunch. Its input is one canonical 640-byte `BNDARQ01` request bound
to a nonzero request sequence and the exact expected generation, version,
length, Resources-1 profile, APK SHA-256, signer-certificate SHA-256, package,
and Activity identity.

The syscall path copies and validates the request while IRQs are masked, queues
one exact request, and returns `ShouldWait`. Launcher waits and retries the
byte-identical request. An IRQ-enabled package service then freshly performs:

1. package-store `recover`;
2. exact generation/identity matching against both the request and boot
   catalog;
3. `read_blob` into kernel-owned bounded scratch space;
4. APK SHA-256 and APK Signature Scheme v2 signer verification;
5. binary Manifest component and Resources-1 verification; and
6. the two-instruction public constructor followed by the four-instruction
   `onCreate`.

Only a successful exact retry consumes the result. Success overwrites the same
exchange buffer with a fresh canonical `BNDAPS01`; a stale generation,
identity mismatch, malformed/replayed sequence, verification failure, or
owner change fails closed and does not navigate to old content. The runtime
path is structurally read-only: each of the two evidenced launches reports
`reads=389 writes=0 flushes=0`.

The local-QEMU interaction evidence launches the exact installed
`org.bndroid.macdemo` package with request sequences 1 and 2. Both passes
reproduce `Mac-built Android app` / `Hello from a Mac-built APK`, constructor
instruction count 2, and `onCreate` instruction count 4. The whole-disk SHA-256
is unchanged across both launches.

The automated gate retains both exact 720x1600 Activity rasters:

```text
target/androidbox-local-apk/check.4rO29p/recovery.relaunch-sequence1.activity.ppm
target/androidbox-local-apk/check.4rO29p/recovery.relaunch-sequence2.activity.ppm
```

Both have SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`,
so the two separately verified launches converge to the same raster. This is a
bounded output identity, not a frame-rate, animation, physical-display, or
general Android-rendering claim.

## Completed CompatibleActivitySession-0

System UI retains at most one boot-local compatible identity:
`(session_id, package_generation)`. It is not a process handle, persistent
Android task, storage capability, or execution grant. The completed lifecycle
is:

1. All apps asks SurfaceServer to reserve the identity before verification.
2. Only an accepted reservation permits the Launcher to run a fresh syscall
   60 durable readback and Resources-1 verification.
3. Success commits the reserved identity to Foreground; failure explicitly
   aborts the reservation. A conflicting reservation or active navigation
   capture is acknowledged without granting launch authority.
4. Home retains only that capacity-one identity and performs no background
   execution.
5. Overview renders only system-owned identity copy. It stores and reads no
   Activity pixels, thumbnail, screenshot, or live preview.
6. Selecting the compatible recent reserves the same generation again and
   performs another fresh syscall 60 pass before Foreground.
7. Back finishes the session, clears the recent identity, and leaves the
   following Overview empty; tapping that empty card is inert.

The control order is `Reserve → fresh verification → commit or abort`.
The control wire is canonical `BUC1` v8. In the completed ABI 44 parent gate,
the server publishes canonical `BUE1` v6 request completions; the ABI 45 child
uses `BUE1` v7 to additionally carry compatible `App/None` focus. Both retain
explicit `accepted`, `conflict`, or `capture-busy` status; absence of an
accepted completion cannot be interpreted as authority. Reserve starts only
from a known released input state. While a
reservation is pending, input samples are quarantined; commit or abort drains
at most 64 queued samples, and a pressed tail or the drain limit keeps the
barrier active through and including the matching release. The `BUP1` v2
present wire binds each mobile raster to the
nonzero `system_ui_revision` under which it was produced. SurfaceServer rejects
a stale revision before accepting its raster, focus transition, syscall
effect, or presentation counters; the first transferred buffer remains
available only for a same-frame retry under the current revision.

The retained visual evidence is:

```text
target/androidbox-local-apk/check.4rO29p/recovery.relaunch-sequence1.activity.ppm
  sha256=e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84
target/androidbox-local-apk/check.4rO29p/recovery.compatible-overview.ppm
  sha256=ff4012ba8d990bb6defc2944fc44021679ce5da72835d456a5446589f610e500
target/androidbox-local-apk/check.4rO29p/recovery.empty-overview-before-tap.ppm
  sha256=19363d900d0a489676b1ca24ada784a167609a626fd899a3b263d3ccd3bcef44
```

The second Activity raster is byte-identical to the first. The empty Overview
after the inert tap is byte-identical to the empty Overview before it. These
facts prove only the bounded session and raster contract above.

## Manual Cocoa-window preview

A separate interactive Cocoa-window session manually displayed the recovered
Mac APK at 720x1600. These PNGs are useful visual previews:

```text
target/androidbox-mac-session-preview-v2/mac-built-apk-session.png
target/androidbox-mac-demo-live-final/screenshots/settings-final.png
target/androidbox-mac-demo-live-final/screenshots/settings-apps.png
target/androidbox-mac-demo-live-final/screenshots/all-apps.png
target/androidbox-mac-demo-live-final/screenshots/mac-apk-activity-final.png
```

The first path is the retained ABI-44 source-free
`Bndroid Mac APK Session Final` window after syscall 60 succeeded. These
separate manual PNGs are useful only as visual previews. The durable
relaunch assertion comes from the automated PPMs and serial markers above, not
from this Cocoa session. None of the images is evidence of ART, a separate
Android process, a physical display, a real phone, or a general Android
runtime.

## Exact compatibility boundary

```text
base_framework_subset=Resources-1
interactive_child_subset=InteractiveActivity-1
launcher_resolution=manifest-main-launcher
activity_class_binding=exact-dex-descriptor
fixed_dex_probe_required=0 fixed_dex_probe_present=0
activity_lifecycle=constructor-then-onCreate constructor_required=1
constructor_instructions=2
interactive_callback=bounded-onClick-view text_update=bounded-setText-resource
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
abi44_activity_owner=kernel-el1-pure-data+launcher-raster
abi45_activity_owner=app-el0-read-only-vmo+app-raster
abi45_launcher_apk_bytes=0 abi45_launcher_activity_pixels=0
abi45_app_claim_rights=read-only abi45_claim_transfer=0
abi46_activity_owner=app-el0-retained-interactive-session+app-raster
abi46_independent_android_app_process=0
abi47_execution_owner=android-app-el0-read-only-vmo
abi47_ui_owner=trusted-app-el0
abi47_launcher_apk_bytes=0 abi47_app_apk_bytes=0
abi47_android_app_apk_bytes=1
abi47_android_app_surface=0 abi47_android_app_graphics=0
abi47_android_app_input=0 abi47_android_app_storage=0
abi47_rpc=BNDAPC01 abi47_rpc_max_outstanding=1
standalone_general_android_runtime=0 compatible_activity_session=1
compatible_activity_capacity=1 compatible_activity_persistence=boot-local
compatible_activity_background_execution=0 overview_activity_pixels=0
overview_thumbnail=0 overview_live_preview=0
crash_recovery_claim=0 network=disabled emulator_only=1 real_phone_claim=0
```

`CompatibleActivitySession-0`, EL0Runtime-0, InteractiveActivity-1, and
Process-0 are bounded to the identities and exact interpreter paths documented
above. ABI 45/46 use the existing resident App EL0 process for execution;
ABI 47 moves execution into a distinct AndroidApp EL0 process while keeping
App as the trusted input/raster host. That distinct process is not ART or a
general standalone Android runtime. None of these profiles adds ART/Dalvik,
ActivityThread, Binder, Bionic, JNI, native libraries, Android permissions,
background execution, general APK support, complete Android lifecycle
compatibility, AndroidApp crash recovery, phone hardware, or a real-phone
result.
