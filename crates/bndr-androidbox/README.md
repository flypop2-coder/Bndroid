# bndr-androidbox

`bndr-androidbox` is the allocation-free `no_std` core for four deliberately
bounded AndroidBox profiles:

- DEX-0 retains two optional fixed pure-static integer diagnostics for the
  historical repository fixture; they are not component admission inputs.
- ActivityLifecycle-1 parses a binary manifest and executes one exact public
  no-argument constructor followed by a fixture-shaped `onCreate` through a
  pure-data Activity/TextView model.
- Resources-1 admits one resource-backed `setContentView(int)` path through a
  strict `resources.arsc` default configuration and one compiled binary
  `TextView` layout.
- InteractiveActivity-1 admits one strict fixed-capacity compiled
  `LinearLayout`/`TextView`/`Button` scene and executes the standard
  `findViewById`, `setOnClickListener`, `onClick(View)`, and
  `TextView.setText(int)` path.

The latest opt-in child `androidbox-layout-mixed19` keeps that bounded scene
but admits a horizontal row whose direct children mix exact-width, zero-weight
Buttons with `0dp`, integer-weight Buttons. No new wire field or authority is
added: ABI69 retains `BNDAPC14` v14 and the v5 24-byte descriptor. Every
horizontal child must be either `Button + Exact + weight0` or
`Button + Zero + weight1..8`; all other shapes fail closed. Its complete
parser, geometry, process and QEMU contract is documented in
`../../ANDROIDBOX_LAYOUT_MIXED_19.md`.

With feature `androidbox-manifest-catalog3`, callers may additionally inspect
an SDK binary Manifest as a fixed-capacity directory of at most 16 components
and 16 requested permissions:

```rust
let catalog =
    bndr_androidbox::inspect_apk_manifest_catalog_with_scratch(apk, &mut envelope_scratch)?;
assert!(!catalog.components().is_empty());

let image = bndr_androidbox::AndroidBox::load_with_manifest_catalog_scratch(
    apk,
    &mut envelope_scratch,
    &mut catalog_scratch,
)?;
```

The catalog recognizes Activity, ActivityAlias, Service, Receiver, Provider,
multiple launchers, ordinary intent filters, alias targets, authorities, and
permission declarations. It executes only the first enabled exported launcher
(or an alias target); all other declarations remain inert metadata and no
permission is granted. Both scratch workspaces must live outside small EL0
stacks and are wiped before and after loading. APK-v2 verification remains an
explicit outer admission step. The exact ABI-54 system integration and QEMU
evidence are documented in `../../ANDROIDBOX_MANIFEST_CATALOG_3.md`.

With feature `androidbox-icon-resources5`, `ManifestInfo` and
`ManifestCatalog` additionally retain the optional application icon resource
reference. `AndroidBox::load_with_manifest_catalog_scratch` resolves that
reference through the same authenticated APK's default `drawable` or `mipmap`
type and decodes one exact stored PNG:

```rust
let image = bndr_androidbox::AndroidBox::load_with_manifest_catalog_scratch(
    apk,
    &mut envelope_scratch,
    &mut catalog_scratch,
)?;
if let Some(icon) = image.launcher_icon() {
    assert_eq!(icon.pixels().len(), 16 * 16);
    assert_ne!(icon.resource_id(), 0);
    assert_ne!(icon.png_crc32(), 0);
}
```

The bounded ABI56 parent accepts only a unique default
`res/drawable/*.png` or `res/mipmap/*.png`, stored in ZIP, at most 16 KiB,
and encoded as non-interlaced 16×16 RGBA8 with exactly `IHDR`, one `IDAT`,
and `IEND`. Whole-entry CRC, per-chunk CRC, zlib input consumption, output
size, filter and canonical transparent pixels are all checked. Its system
integration and QEMU evidence are documented in
`../../ANDROIDBOX_ICON_RESOURCES_5.md`.

The opt-in child feature `androidbox-density-icons7` additionally prefers one
exact mdpi 160-dpi configuration over the default and admits an exact 48×48
RGBA8 PNG at `res/drawable-mdpi[-v4]/*.png` or
`res/mipmap-mdpi[-v4]/*.png`. It alpha-aware downsamples each 3×3 block to
the retained 16×16 wire shape and deterministically quantizes results above
the fixed 16-color UI palette limit. Source dimensions, density, normalization,
and quantization are explicit ABI57 provenance. Other density configurations
may coexist but are not selected. Adaptive icons, vectors, WebP, JPEG,
nearest-density fallback, overlays, runtime configuration changes, and
arbitrary PNG shapes remain unsupported. The ABI57 integration and evidence
are documented in `../../ANDROIDBOX_DENSITY_ICONS_7.md`.

The opt-in child feature `androidbox-dex-methods8` additionally admits one
APK-defined method invocation from the retained `onClick(View)`:

```text
invoke-static {viewId}, SameActivity.privateStaticHelper:(I)I
move-result resourceId
TextView.setText(resourceId)
```

The helper must belong to the selected Activity, have exact `private static`
flags and `(I)I` prototype, and be called no more than once per callback at
depth one. It is bounded to eight registers and 96 instruction units. Every
encoded instruction must be reachable, the complete CFG must be acyclic, and
only integer move/return/const/goto/if-eq/if-ne/add-int-lit8 operations are
accepted. The nonzero return value remains subject to the existing resource
and target-TextView checks. Objects, fields, arrays, exceptions, loops,
recursion, nested or cross-class calls, and arbitrary methods fail closed.
`ActivityUpdate::app_defined_call_count` exposes 0/1 execution provenance to
the outer ABI without giving DEX code a system handle. ABI58 integration and
QEMU evidence are documented in `../../ANDROIDBOX_DEX_METHODS_8.md`.

The opt-in child feature `androidbox-dex-instance9` additionally admits one
real Activity instance call:

```text
invoke-direct {activity, clickedView}, SameActivity.privateHelper:(View)I
move-result resourceId
```

The helper receiver must be the current Activity and its argument must be the
exact clicked View reference. The target is a same-Activity direct method with
exact `private` access and `(Landroid/view/View;)I` prototype. Inside the
helper, one `invoke-virtual View.getId()` plus its immediate `move-result` is
accepted alongside the ABI58 integer/branch opcode subset. The complete helper
CFG remains reachable and acyclic; one callback may make one app call at depth
one, and the callback plus helper stays inside the 96-instruction execution
budget. Fields, allocations, arrays, exceptions, recursion, other framework
calls and forged object references fail closed.
`ActivityUpdate::app_defined_instance_call_count` distinguishes the instance
call from ABI58's static call without granting a handle or syscall. ABI59
integration and evidence are documented in
`../../ANDROIDBOX_DEX_INSTANCE_9.md`.

The opt-in child feature `androidbox-activity-fields10` additionally admits
one retained Activity field with exact same-Activity owner, `private` access,
and `Landroid/widget/TextView;` type:

```text
onCreate: findViewById -> check-cast TextView -> iput-object Activity.field
onClick:  iget-object Activity.field -> TextView.setText(resourceId)
```

The field must be initialized exactly once by `onCreate` from a validated
nonzero TextView in the retained scene. A field-bearing callback must read
that exact field once from the current Activity; the resulting reference is
revalidated against the same retained scene before mutation. No static,
second, primitive, array, cross-class, uninitialized, or callback-written
field is admitted. Fieldless ABI59/58/earlier callbacks remain accepted.
`ActivityUpdate::activity_field_read_count` exposes 0/1 execution provenance
without exposing object addresses or granting a system handle. ABI60
integration and QEMU evidence are documented in
`../../ANDROIDBOX_ACTIVITY_FIELDS_10.md`.

The opt-in child feature `androidbox-activity-state11` additionally admits
one private same-Activity `int` field alongside the retained `TextView`:

```text
onCreate: const/4 0 -> iput Activity.clickCount:I
onClick:  iget Activity.clickCount:I
          add-int/lit8 +1
          iput Activity.clickCount:I
          iget-object Activity.statusView:TextView
```

The integer field must be initialized to zero, read once, incremented by the
exact literal `+1`, and written once to the same current Activity field before
the existing TextView mutation completes. The next callback reads the
committed value from the retained Activity session. Scene, fields, and
revision commit transactionally; failed execution changes none of them.
State callbacks expose exact read/write counts and the bounded committed
`1..=255` value, while ABI60 field-only and older callbacks remain accepted
with zero integer state. No object address or system capability is exposed.
ABI61 integration, stack-lifetime hardening, and QEMU evidence are documented
in `../../ANDROIDBOX_ACTIVITY_STATE_11.md`.

The opt-in child feature `androidbox-string-text12` additionally admits the
real D8 `const/4 1`, `add-int/2addr`, state `if-ne`, `const-string`, and exact
`TextView.setText(CharSequence)` callback shape. DEX registers retain only a
validated 32-bit string index; one bounded string value is copied at the
framework mutation point. Direct text has resource ID zero and separate
execution provenance, while ABI61 resource-backed callbacks remain accepted.
ABI62 integration and QEMU evidence are documented in
`../../ANDROIDBOX_STRING_TEXT_12.md`.

The opt-in child feature `androidbox-string-builder13` additionally admits
the exact real-D8 `new-instance StringBuilder`, `<init>(String)`,
`append(int)`, `toString()`, `move-result-object`, and
`TextView.setText(CharSequence)` chain. Registers retain bounded symbolic
builder/string handles rather than Java addresses. One APK-owned prefix is
combined with the just-committed `1..=255` Activity integer at the mutation
point. ABI63 exposes separate dynamic and direct provenance bits; ABI62
literal-string and earlier callbacks remain accepted. This is one bounded
builder, not a general heap or GC. Integration and QEMU evidence are
documented in `../../ANDROIDBOX_STRING_BUILDER_13.md`.

The earlier `androidbox-demo.apk` remains the Activity-0 regression fixture.
The current `androidbox-resource-demo.apk` must contain exactly four uncompressed
stored entries: binary `AndroidManifest.xml`, `resources.arsc`, compiled binary
`res/layout/activity_main.xml`, and `classes.dex`. It is signed by one
repository-owned test-only RSA-2048 key using APK Signature Scheme v2.
The `androidbox-resource-update-demo.apk` regression fixture retains that exact
archive shape, package, Activity, and signer certificate while advancing to
`versionCode=3`, title `AndroidBox Resources-1 Demo v3`, and resource-backed
text `AndroidBox updated resource view`.

```rust
let signer = bndr_androidbox::verify_apk_v2(apk_bytes)?;
let image = bndr_androidbox::AndroidBox::load(apk_bytes)?;

assert_ne!(signer.certificate_sha256, [0; 32]);
assert_eq!(image.manifest_info().version_code, 2);

// Optional historical DEX-0 diagnostics remain available when present.
let started = image.boot()?;
let tapped = image.on_tap(35)?;

// Resources-1 executes the manifest-selected constructor, then onCreate.
let launched = image.launch_activity()?;
assert_eq!(launched.constructor_instruction_count, 2);
assert_eq!(
    launched.manifest.activity_descriptor.as_bytes(),
    b"Lorg/bndroid/demo/MainActivity;"
);
assert_eq!(
    launched.view.text.as_bytes(),
    b"AndroidBox resource-backed view"
);
let resources = launched.resources.expect("resource-backed Activity");
assert_eq!(resources.layout_resource_id, 0x7f02_0000);
assert_eq!(resources.text_resource_id, 0x7f03_0000);
```

With feature `androidbox-apk-envelope4`, callers may opt into the Envelope-4
container reader. It retains STORED `classes.dex` and `resources.arsc`, while
allowing raw-DEFLATE binary Manifest/layout XML through a caller-owned,
`AndroidBoxEnvelopeScratch`. The 32-KiB limit applies only to its inflated XML
output buffer; the scratch also owns reusable DEFLATE inflater state. The
loader wipes the complete workspace before and after every attempt and caches
the parsed layout, so the returned image does not borrow the workspace. Keep
the scratch in static or other long-lived storage rather than on a small EL0
stack:

```rust
let signer = bndr_androidbox::verify_apk_v2(apk_bytes)?;
let mut scratch = bndr_androidbox::AndroidBoxEnvelopeScratch::new();
let image =
    bndr_androidbox::AndroidBox::load_with_scratch(apk_bytes, &mut scratch)?;
let envelope = image.envelope_info().expect("Envelope-4 evidence");
assert_ne!(signer.certificate_sha256, [0; 32]);
assert!(envelope.manifest_deflated());
```

`load_with_scratch` validates the bounded ZIP envelope but deliberately does
not authenticate it. Production admission must call `verify_apk_v2` first (or
provide an equivalent outer authenticity policy). Deflated entries other than
the selected Manifest/layout are structurally bounded and counted as
unconsumed evidence, but are never inflated or interpreted by this crate.

DEX-0 locates the repository fixture's two fixed pure-static integer
diagnostics when present. A missing or invalid diagnostic fails only an
explicit `boot()`/`on_tap()` request and cannot reject an otherwise valid
manifest-selected Activity.
Each `Execution` exposes the `classes.dex` CRC32, DEX header Adler-32, DEX file
size, method index, code offset, register/parameter/instruction sizes, returned
`i32`, and executed instruction count.

ActivityLifecycle-1:

- parses the bounded binary XML string pool, Android resource map, namespace,
  start/end elements, and typed attributes without allocation;
- requires one canonical package, one nonzero `android:versionCode`, and one
  exported activity whose same
  intent-filter contains both `android.intent.action.MAIN` and
  `android.intent.category.LAUNCHER`;
- normalizes manifest class forms such as `.MainActivity` into a DEX
  descriptor and requires that exact class to extend `android.app.Activity`;
- rejects launcher Activity fields and interfaces, requires one unique direct
  public constructor `<init>()V`, and interprets exactly
  `invoke-direct Activity.<init>()V; return-void`;
- then locates and executes its real
  `onCreate(Landroid/os/Bundle;)V` `code_item`;
- retains the earlier inline-text lifecycle sequence using `invoke-super`,
  `new-instance`, `invoke-direct`, `const-string`, `invoke-virtual`, and
  `return-void`;
- maps only `Activity.<init>`, `Activity.onCreate`, `TextView.<init>`,
  `TextView.setText`, and the admitted forms of `Activity.setContentView` to
  an internal state machine;
- returns an owned, fixed-capacity `ActivityView`. DEX code receives no Bndroid
  handle and performs no syscall.

The locally built `androidbox-mac-demo.apk` is the component-binding
regression: its `classes.dex` contains
`Lorg/bndroid/macdemo/MainActivity;` but deliberately contains no
`Lorg/bndroid/demo/Main;`, `boot()`, or `onTap(int)`. Its successful
Resources-1 launch proves that the manifest component, rather than the old
repository probe, now drives admission. The system-level ABI-44 gate also
launches this exact installed package twice through generation-bound request
sequences 1 and 2. Each pass freshly reads and verifies the durable APK before
calling this crate's constructor-to-`onCreate` interpreter; it reports
`reads=389 writes=0 flushes=0` and leaves the whole disk unchanged.

Resources-1 additionally:

- resolves a manifest `android:label` string reference through the same APK;
- accepts one default-configuration resource table and rejects ambiguity;
- distinguishes the admitted `layout` and `string` type pools;
- resolves layout ID `0x7f020000` to the exact stored binary layout entry;
- requires one root `TextView` as the layout's sole element, verifies its
  `android:text` resource reference, and resolves string ID `0x7f030000`;
- executes the four-instruction
  `Activity.onCreate → const/high16 → setContentView(I) → return-void` path;
- returns CRC32 and resource-ID evidence in `ActivityResourceInfo`.

InteractiveActivity-1 additionally:

- retains one exact public `onClick(View)V` from a final Activity implementing
  exactly `View.OnClickListener`;
- parses at most eight scene nodes and depth three, with only vertical
  `LinearLayout`, `TextView`, and `Button`;
- validates unique compiled IDs, direct default string references, exact
  width/height/orientation attributes, and rejects unknown tags or attributes;
- executes the real d8 `const`, `check-cast`, `findViewById`,
  `setOnClickListener`, and `setText(int)` instructions;
- returns an `ActivitySession` whose `dispatch_click` executes the retained
  callback transactionally and publishes method, step, resource-ID, changed
  view-ID, and monotonic revision evidence.

All profiles fail closed on malformed bounds/checksums, duplicate or missing
identity, unknown XML structure, class, method, opcode, register, lifecycle
order, resource type, reference, configuration, or layout structure.

`verify_apk_v2` is a separate, allocation-free admission primitive and does not
change `AndroidBox::load`. Its strict profile accepts exactly one signer, one
certificate, RSA-2048 exponent 65537, algorithm `0x0103`, matching certificate
and signer SubjectPublicKeyInfo, an authentic signed-data record, and the
standard three-section chunked APK SHA-256 digest. It rejects all signed
attributes, multiple signers/certificates/algorithms, unknown Signing Block
pairs, all `META-INF/` JAR/v1 material, and all nonzero padding. Both standard
three-field v2 signed data and the extra empty field emitted by Android SDK 36
are accepted.

This core crate is not ART or Dalvik and does not claim arbitrary APK
compatibility. It owns no package-store persistence or update policy. The
isolated system-level `androidbox-apk-install0` profile composes
`verify_apk_v2` and `AndroidBox::load` with separate package-store and runtime
components for the strict signed Resources-1 shape. The fixed repository gate
and the separately Mac-built no-probe package both complete that bounded flow,
which is documented in `../../ANDROIDBOX_APK_INSTALL_0.md`. Its separate,
strictly bounded Update-0 policy admits only the same package and v2 signer
certificate with a strictly higher version, then atomically publishes the v3
fixture as package-store generation 2. Identical-source replay and source-free
recovery are write-free; rollback and tampered sources are rejected without
disk mutation. The exact update proof is documented in
`../../ANDROIDBOX_APK_UPDATE_0.md`. The crate itself still owns none of that
persistence or update policy.

At system level, the ABI-44 parent uses syscall 59 to publish the 640-byte
`BNDAPS01` boot catalog. Launcher-only syscall 60 accepts one 640-byte
`BNDARQ01` request bound to the exact installed generation and identity, then
an IRQ-enabled service freshly performs package-store recovery/readback and
calls this core only after APK SHA-256 and APK-v2 signer checks. Success returns
a fresh `BNDAPS01`.

The opt-in ABI-45 `androidbox-el0-runtime0` child additionally lets only the
resident App generation consume one session/generation/digest-bound syscall-61
claim. The handle has exactly `READ`; App reads the immutable APK in bounded
chunks and calls this same core again for foreground re-verification and
execution before submitting Activity pixels. Launcher receives no APK bytes or
Activity raster. The kernel still calls the core for bounded admission/profile
validation before publishing the grant. All syscalls, sequencing, VMO and
storage scratch space, owner checks, and raster routing are outside this crate.

The opt-in ABI-46 `androidbox-interactive0` child composes this crate's
retained `ActivitySession` with the existing App EL0 runtime. Its exact
deterministic offline fixture is
`../../fixtures/androidbox-interactive-demo/androidbox-interactive-demo.apk`
(12566 bytes, SHA-256
`0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee`).
The real Java Activity uses a compiled vertical
`LinearLayout`/`TextView`/`Button`, implements `View.OnClickListener`, binds
the Button in `onCreate`, and mutates the TextView in `onClick(View)`.
The App-retained session dispatches the Button callback and advances the text
from `Ready for a real APK click` to `Button callback executed`.

System integration in that child uses an exact 720x1600 output:
SurfaceServer owns the top 64 and bottom 88 system-chrome pixels, while
Launcher/App owns the 720x1448 content viewport. ABI-46 syscall 62 validates
and atomically composes the independent content and chrome sources. The first
QEMU run exposed a 32 KiB EL0 stack guard fault; the ABI-46 feature now maps a
64 KiB user stack with one unmapped guard page on each side, without changing
ABI 45.

The completed automated checker is
`../../scripts/check-androidbox-interactive0.sh`; its retained evidence is
`../../target/androidbox-interactive0/check.4sMhMR/` and its terminal result is
`ANDROIDBOX_INTERACTIVE0_QEMU_OK`. It completes an APK-bearing install boot
and a source-free recovery boot with networking disabled. The ABI 46 topology
has three graphics-buffer identities and six handles, one producer plus one
server handle for each of SurfaceServer, Launcher, and App.

The gate records 13 App and 16 Launcher layered commits. Three real Button
input samples produce two callback frames and change 4138 content pixels,
including 4020 text-label pixels; the top 64 and bottom 88 chrome regions stay
byte-identical. The initial and relaunched PPM SHA-256 is
`69e6e61e005c2762c6cf4f53168b8929783df402075d53d3ecdebc448efd8209`;
the clicked PPM SHA-256 is
`941bac90d2e38de106420c8c1ed2797dd0670e2b1e83fb8114c3d90fed8f1baf`.
Top, Home, and bottom-corner probes route zero samples to App; the corner
probe changes neither the claim count nor raster. The recovery disk remains
unchanged at SHA-256
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`,
and the gate has no panic or user fault.

The completed host suites contain 243 UI tests, 82 AndroidBox tests, 13 init
runtime tests, and 377 kernel tests.

The opt-in ABI-47 `androidbox-process0` child moves the immutable package VMO,
this crate's bounded interpreter, and the retained `ActivitySession` into a
separately linked `AndroidApp` EL0 image. The existing App EL0 process remains
the trusted input/raster host. Init gives the pair one private Channel;
AndroidApp has no Surface, graphics, input, storage, duplicate, or transfer
authority. During Open it may temporarily hold exactly one additional
read-only package VMO, then returns to the single-Channel steady state.

App and AndroidApp exchange only canonical 64-byte `BNDAPC01` messages. The
completed request sequence is `Ready(0)`, `Open(1)`, `Click(2)`, and
`Close(3)` with authenticated sender PIDs, one outstanding request, and
capacity-eight queues. AndroidApp returns bounded text chunks and revision
evidence; App alone rasterizes the scene and receives touch. Process creation,
grant ownership, RPC transport, raster composition, and reaping are system
integration outside this parser/interpreter crate.

`../../scripts/check-androidbox-process0.sh` completes an offline install boot
and a source-free recovery boot with exactly one `-nic none` on each QEMU
invocation. Retained evidence is in
`../../target/androidbox-process0/check.sFpefj/`; the terminal marker is
`ANDROIDBOX_PROCESS0_QEMU_OK`. The callback changes 4138 content pixels while
the SurfaceServer-owned top and bottom chrome remain byte-identical. Distinct
App/AndroidApp ELF hashes, PIDs, ASIDs, and translation-table roots prove the
process boundary. This is still only InteractiveActivity-1, not ART, Dalvik,
ActivityThread, Binder, Bionic, JNI, native libraries, Android services, a
general APK runtime, crash recovery, or a phone port.

The system-level `CompatibleActivitySession-0` composition is now completed
and evidenced in
`../../target/androidbox-local-apk/check.4rO29p/`. SurfaceServer retains one
capacity-one, boot-local `(session_id, package_generation)` identity. Home may
retain that identity but performs no compatible-Activity background execution.
Overview renders only the identity and reads no Activity pixels, thumbnail,
screenshot, or live preview. Selecting that recent runs a new syscall 60
durable readback and verification before Foreground; Back finishes the session
and clears the identity.

The control order is `Reserve → fresh verification → commit or abort`.
Canonical `BUC1` v8 requests receive canonical completions with explicit
`accepted`, `conflict`, or `capture-busy` status: `BUE1` v6 in the ABI-44
parent and v7 in the ABI-45 child so SurfaceServer can publish authenticated
`App/None` compatible focus. Only `accepted` permits the subsequent bounded
operation. `BUP1` v2 additionally binds a mobile raster to its nonzero
`system_ui_revision`, so SurfaceServer rejects stale raster content before
accepting the associated presentation side effects. None of these protocol
facts belongs to this parser/interpreter crate.

The two Activity rasters are byte-identical at SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`.
The compatible identity-only Overview SHA-256 is
`ff4012ba8d990bb6defc2944fc44021679ce5da72835d456a5446589f610e500`;
the empty Overview before and after its inert tap is
`19363d900d0a489676b1ca24ada784a167609a626fd899a3b263d3ccd3bcef44`.

Neither this core nor Install-0/Update-0 provides a production trust policy,
certificate-chain/time policy, v1/v3/v4 or multi-signer support,
ActivityThread, ResourceManager, resource qualifiers, aliases, complex/sparse
entries, nested or arbitrary View/layout inflation, exceptions, native code,
JNI, Binder, Bionic, Android services, or a general Framework implementation.
The ABI-44 parent runs the interpreter as an EL1 pure-data model. In the
ABI-45 child, the kernel still performs an EL1 admission/profile pass while
the existing App EL0 host repeats foreground verification/execution and owns
the Activity raster. ABI 46 retains that same host and adds the bounded
interactive session. ABI 47 creates a dedicated Bndroid AndroidApp process for
the bounded interpreter/session, but this process boundary is not a claim of
ART/Dalvik, ActivityThread, Binder, Bionic, JNI, native-library execution,
Android permissions, general APK compatibility, Android task management,
background execution, crash recovery, phone hardware, or a real-phone result.

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
abi44_45_46_standalone_android_process=0 abi47_bounded_android_app_process=1
abi47_general_android_runtime=0 compatible_activity_session=1
compatible_activity_capacity=1 compatible_activity_persistence=boot-local
compatible_activity_background_execution=0 overview_activity_pixels=0
overview_thumbnail=0 overview_live_preview=0 real_phone_claim=0
network=0 interactive_general_apk=0 abi47_crash_recovery=0
```

InteractiveActivity-1 remains a bounded interpreter: inside App EL0 for ABI
46, and inside the separate AndroidApp EL0 image for ABI 47. Neither profile
is a real phone or general Android compatibility.

Rebuild the repository-owned fixtures offline with:

```sh
./scripts/build-androidbox-demo.sh
./scripts/build-androidbox-resource-demo.sh
./scripts/build-androidbox-resource-update-demo.sh
```

The offline host suite currently contains 82 unit tests. That count includes a
real v3 fixture regression which calls `verify_apk_v2`, proves the certificate
matches the v2 base fixture, loads the updated manifest through
`AndroidBox::load`, and launches the updated Resources-1 Activity while checking
its four executed instructions, resource IDs, and CRC32 evidence. It also
includes a component-only Mac APK regression that proves Activity launch
succeeds while both legacy diagnostics return `DexClassMissing`.

The fixtures are byte-for-byte evidence for these bounded profiles. Successful
Activity-0 or Resources-1 launches are not evidence that unrelated Android
apps run. The resource fixture's checked-in private key is publicly known and
strictly test-only; it is not a Bndroid product trust anchor and must never be
used for production or distributable applications.
