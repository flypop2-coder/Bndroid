# AndroidBox APK Install-0, Update-0, and Uninstall-0 contract

Status: completed and evidenced on local QEMU for one exact repository-owned
Install-0 fixture and its exact v2-to-v3 Update-0 successor. The bounded
Uninstall-0 lifecycle and same-digest reinstall are now evidenced by a
12-boot gate. A second package built locally on the Mac is independently
installed, recovered, and relaunched twice through its installed icon by the
generic Resources-1 gate. This is not a general APK installer, PackageManager,
standalone Android application process, or Android compatibility claim.

This contract defines a durable package-store commit and update of one bounded
APK profile for Bndroid and points to the separate Uninstall-0 contract. It does not
define general Android compatibility.

The admitted fixture is
`fixtures/androidbox-resource-demo/androidbox-resource-demo.apk`:

- size: `12566` bytes;
- package: `org.bndroid.demo`;
- launcher Activity: `Lorg/bndroid/demo/MainActivity;`;
- `versionCode`: `2`;
- APK SHA-256:
  `2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7`;
- signer certificate SHA-256:
  `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

The admitted Update-0 successor is
`fixtures/androidbox-resource-update-demo/androidbox-resource-update-demo.apk`:

- size: `12566` bytes;
- package: `org.bndroid.demo`;
- launcher Activity: `Lorg/bndroid/demo/MainActivity;`;
- `versionCode`: `3`;
- APK SHA-256:
  `3860fcd80eed1a4b284514316bc35169dda60bb1522e85abdea5aa33b8355708`;
- signer certificate SHA-256:
  `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

Update-0 requires the exact same package identity and signer certificate as
the installed package and a strictly increasing non-zero `versionCode`. The
completed evidence is this v2-to-v3 pair, not arbitrary update admission.

The separately evidenced local-Mac artifact is
`fixtures/androidbox-mac-demo/androidbox-mac-demo.apk`: 12566 bytes, package
`org.bndroid.macdemo`, launcher
`Lorg/bndroid/macdemo/MainActivity;`, `versionCode=1`, title
`Mac-built Android app`, TextView `Hello from a Mac-built APK`, and SHA-256
`a604d16298e939d728aa805a636bb119c906bc1b9c224949da445954a4f8363e`.
It is also APK-v2-only with one signer. Its exact contract is
`ANDROIDBOX_LOCAL_APK.md`.

The fixture's test-only private key and certificate are intentionally public
repository test material:
`fixtures/androidbox-resource-demo/test-only-apk-signing-key.pk8` and
`test-only-apk-signing-cert.pem`. They make the fixture reproducible; they are
not production signing keys, production publisher trust, HSM custody, or a
secure provisioning claim.

## Authority and source

- The Install-0 checker accepts exactly one APK through an explicit absolute
  `--apk` argument. The Update-0 checker accepts exactly two explicit absolute
  files through `--base-apk` and `--update-apk`. Neither checker scans host
  directories, uses a network, or selects a user file implicitly.
- `scripts/check-androidbox-local-apk.sh` is identity-generic within the same
  strict Resources-1 shape: it derives metadata and resource facts from one
  explicit absolute APK path and scans no APK directory.
- QEMU exposes only the source selected for that boot, read-only as
  `opt/bndroid/apk` through `fw_cfg`. Replacing either input APK must not rebuild
  the kernel or userspace images.
- Uninstall-0 instead accepts only one canonical 256-byte `BNDUNS01` request
  selected by a trusted offline host as read-only
  `opt/bndroid/package-uninstall` `fw_cfg` input before EL0 starts. APK and
  uninstall inputs are mutually exclusive. Its completed host-test/QEMU
  evidence boundary is defined in `ANDROIDBOX_APK_UNINSTALL_0.md`.
- The install authority belongs to a trusted Bndroid host component. Interpreted
  Android bytecode receives no storage handle, Bndroid capability, or syscall
  authority.
- Every QEMU boot for this profile uses exactly one `-nic none`.
- This source proves only local QEMU sideloading. It is not USB installation, a
  download manager, an app store, or an Installer UI claim.

## Admission profile

All admission completes before the first package-store mutation:

1. Enforce the existing bounded ZIP, binary Manifest, DEX, and Resources-1
   parser limits.
2. Verify APK Signature Scheme v2 with exactly one signer.
3. Accept only RSA-2048 with exponent 65537, PKCS#1 v1.5, and SHA-256
   (`0x0103`).
4. Verify the signed-data signature, certificate/public-key match, and APK
   chunked content digest.
5. Persist the signer certificate SHA-256 as update identity. This proves
   integrity and signer continuity, not production publisher trust.
6. Admit exactly one manifest package and one exported `MAIN`/`LAUNCHER`
   Activity that passes the current Resources-1 compatibility profile.
7. If a package is already committed, require the same package identity, the
   same signer-certificate SHA-256, and a strictly greater `versionCode` before
   any package-store write.

The Mac APK deliberately contains no fixed
`Lorg/bndroid/demo/Main;`, `boot()`, or `onTap()` diagnostics. Admission now
resolves the unique exported `MAIN`/`LAUNCHER` component from the binary
Manifest and binds that exact descriptor to the DEX Activity class. The
historical diagnostics remain available to old fixtures but no longer decide
component admission.

Multi-signer APKs, v1-only signatures, v3/v4 lineage, unknown algorithms,
native libraries, JNI, Binder, ART, Dalvik, arbitrary layouts, resource
qualifiers, and Android permission bridging remain unsupported.

## Package-store layout

Install-0/Update-0/Uninstall-0 owns one fixed `BNDROID_PACKAGES` GPT partition that does
not overlap `BNDROID_DATA`, `BNDROID_APPDATA`, or `BNDROID_SYS`.

The evidenced image is 16 MiB and the package partition is exactly LBA
`16384..16895` inclusive, 512 sectors of 512 bytes. Whole-image comparison
confirms that the successful Install-0, Update-0, Uninstall-0, reinstall, and
local-APK transactions change no
byte outside that range.

The format is deliberately single-package and stateful:

- one versioned superblock;
- two registry records;
- two fixed-capacity APK blob slots;
- sector-wide checksums and a fixed format epoch;
- a maximum APK size fixed by the blob-slot payload capacity.

Each committed registry generation binds at least:

`transaction_id`, `package`, `version_code`, `launcher_component`,
`compatibility_profile`, `blob_slot`, `blob_generation`, `apk_length`,
`apk_sha256`, `signer_certificate_sha256`, and `registry_generation`.

## Commit and recovery

The only valid publication order is:

1. Write the candidate APK to the inactive blob slot.
2. Flush it, read it back completely, and verify its metadata and SHA-256.
3. Write a committed registry record to the inactive registry slot.
4. Flush and read back that registry record.
5. Publish success only after the record and referenced blob validate together.

The previous committed registry and blob remain unchanged until the new pair is
durable. A blob without a matching committed registry is an invisible orphan.
A registry whose blob is absent, corrupt, or mismatched is invalid.

Recovery chooses the newest complete registry/blob pair. It must never combine
metadata from one generation with bytes from another. Replaying the same
transaction is idempotent. A stable source-free boot performs no package-store
writes.

The first install publishes generation 1 in slot 0. An admitted update uses
checked generation increment and the inactive slot; the completed v2-to-v3
gate therefore switches generation 1/slot 0 to generation 2/slot 1. Exact
source replay returns the already committed handle without a write or flush.
An older version, changed package, changed signer, or reused transaction ID
with different content is rejected before mutation.

Uninstall-0 advances to a `Removed` state by writing one higher-generation
`BNDPRM01` tombstone to the inactive registry, flushing and reading it back,
then mirroring the logically identical tombstone at the same generation to
the other registry. Before the first complete tombstone recovery sees the old
install; afterwards it sees removal, even if the mirror is interrupted.
Exact replay repairs one missing mirror and then becomes write-free. APK blob
sectors are not erased, but no installed-package read path can reach them.

`OutcomeUnknown` is reconciled by rereading the registry; it is never resolved
through a blind mutation retry.

## Completed package-store fault-model evidence

`bndr-package-store` has 43 host unit tests. The Update-0 matrix starts from
generation 2 and reuses the occupied inactive slot for generation 3. It
injects a definite failure at each of the 129 update writes, plus four
representative torn prefixes (`0`, `1`, `37`, and `511` bytes) at each write,
for 516 torn-write cases. Both flush boundaries, durable/not-durable
`OutcomeUnknown`, definite blob-flush failures, commit-flush error, and
post-commit acknowledgement loss are covered. The suite also checks exact
install/update replay with zero writes and flushes, package/signer continuity,
equal/lower-version rejection, generation/slot alternation, stale handles,
orphan invisibility, stable write-free recovery, newest-registry/blob
corruption fallback, non-zero padding rejection, epoch/CRC validation, and
old-or-complete-new outcomes.

The additional Uninstall-0 cases distinguish `Empty`, `Installed`, and
`Removed`; cover both tombstone writes and four torn prefixes per write;
exercise flush/readback/ack loss, one-copy repair, and exact zero-write replay;
prove one cleared or corrupt tombstone cannot revive the old package; reject
conflicting same-generation tombstones; and cover reinstall
package/signer/version/digest policy plus generation exhaustion.

These are deterministic in-memory block-device fault-injection tests of the
package-store transaction protocol. They are not physical power-loss,
controller-cache, storage-media, PMIC, or real-device evidence.

## Completed Install-0 local-QEMU evidence

`scripts/check-androidbox-apk-install0.sh` completed with evidence retained in
`target/androidbox-apk-install0/check.NQ6eql/`. The gate uses one unchanged
kernel/userspace image and exactly one `-nic none` per boot:

1. A content-tampered copy is rejected by APK v2 verification before the first
   package write. The negative disk SHA-256 remains
   `b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801`.
2. Boot 1 receives the valid APK through read-only `fw_cfg`, formats the
   package store, commits generation 1 in slot 0, rereads the durable APK, and
   executes its Resources-1 `onCreate→setContentView(int)` result. Its package
   I/O evidence is `reads=1032 writes=130 flushes=3`.
3. Boot 2 has no APK `fw_cfg` source. It recovers generation 1 and launches
   from package-store readback with `reads=389 writes=0 flushes=0`.
4. Boot 3 is a second no-source recovery and produces the same Activity
   evidence with `reads=389 writes=0 flushes=0`.
5. The package disk SHA-256 after boots 1, 2, and 3 is identically
   `37b2afa4a85153ef84f2329648eb3e364f8629c78cca17eb3a45358f48bb9965`;
   all changed bytes are confined to LBA `16384..16895`.

The QEMU gate therefore proves first installation, two source-free recovery
boots, stable zero-write recovery, durable readback execution, bounded disk
mutation, and tampered-source rejection. It does **not** perform a QEMU
pre-registry cut, post-registry/pre-ack cut, physical power cut, or real-media
corruption campaign. Write/torn-write/flush failure points belong to the
package-store unit-test evidence above and must not be described as QEMU or
physical-power evidence.

## Completed Update-0 local-QEMU evidence

`scripts/check-androidbox-apk-update0.sh` completed with evidence retained in
`target/androidbox-apk-update0/check.yu1Xpq/`. It requires two explicit
absolute paths and runs offline:

```bash
./scripts/check-androidbox-apk-update0.sh \
  --base-apk "$PWD/fixtures/androidbox-resource-demo/androidbox-resource-demo.apk" \
  --update-apk "$PWD/fixtures/androidbox-resource-update-demo/androidbox-resource-update-demo.apk"
```

The gate uses one unchanged kernel/userspace image and exactly one
`-nic none` per QEMU boot. It proves:

1. The base v2 APK installs as generation 1/slot 0 and renders
   `AndroidBox resource-backed view`.
2. The v3 APK has the same package, launcher Activity, and exact signer
   certificate, with strictly increasing `versionCode` 2 to 3. It commits as
   generation 2/slot 1 with
   `reads=905 writes=129 flushes=2` and renders
   `AndroidBox updated resource view` from durable package-store readback.
3. Replaying the exact v3 source is idempotent:
   `reads=904 writes=0 flushes=0`; the disk remains byte-for-byte unchanged.
4. A later boot with no APK source recovers generation 2/slot 1 and the v3
   Activity with `reads=645 writes=0 flushes=0`; the disk again remains
   byte-for-byte unchanged.
5. Supplying the previously valid v2 source after v3 is committed is rejected
   as rollback before mutation. A content-tampered v3 source is rejected by
   APK v2 verification before mutation. Both negative cases leave their input
   disk byte-for-byte unchanged.
6. Whole-image comparison confines both successful install and update changes
   to `BNDROID_PACKAGES` LBA `16384..16895`.

The retained generation-1 and generation-2 disk SHA-256 values are
`37b2afa4a85153ef84f2329648eb3e364f8629c78cca17eb3a45358f48bb9965`
and
`d3166bc25b95d28d3a18ee612fe03be96d342d2b37ac620947fe71828abba1de`
respectively. This gate does not inject a QEMU cut at an update write or flush;
those failure points are covered only by the deterministic host tests above.
It is not physical-power-loss or real-media evidence.

## Completed Uninstall-0/reinstall local-QEMU evidence

`scripts/check-androidbox-apk-uninstall0.sh` completed 12 offline boots with
evidence retained in `target/androidbox-apk-uninstall0/check.gWbx8U/` and
terminal marker `ANDROIDBOX_APK_UNINSTALL0_QEMU_OK`.

The exact lifecycle is generation `1→2→removed 3→reinstalled 4`. Removal uses
`reads=520 writes=2 flushes=2`; its two `BNDPRM01` registry sectors are
byte-identical and exactly 404 disk bytes change, only in LBAs `16385` and
`16386`. Exact uninstall replay is `reads=6 writes=0 flushes=0`, source-free
removed recovery is `reads=3 writes=0 flushes=0`, reinstall is
`reads=521 writes=129 flushes=2`, and final source-free Activity recovery is
`reads=389 writes=0 flushes=0`.

Wrong target, stale generation, bad request CRC, simultaneous APK/request, and
post-removal rollback are independently rejected with the complete disk
unchanged. APK blobs remain byte-for-byte unchanged by removal and logically
unreachable. The exact 12-boot sequence, hashes, marker, and evidence boundary
are in `ANDROIDBOX_APK_UNINSTALL_0.md`.

## Completed locally built APK gate

`scripts/check-androidbox-local-apk.sh` installed the explicit Mac-built APK
and recovered it without a source. Evidence is retained in
`target/androidbox-local-apk/check.4rO29p/`; the terminal marker is
`ANDROIDBOX_LOCAL_APK_QEMU_OK`.

The install is generation 1 / slot 0 with
`reads=1032 writes=130 flushes=3`. Recovery is
`reads=389 writes=0 flushes=0`, preserves the complete disk, and reproduces
byte-identical durable Activity evidence for `Mac-built Android app` /
`Hello from a Mac-built APK`. Host `dexdump` and the guest jointly prove the
Manifest descriptor is present while the retired probe class is absent. This
gate additionally reports the strict two-instruction public no-argument
constructor before the four-instruction `onCreate` on both install and
source-free recovery. It still proves only the strict Resources-1 component
shape. Full artifact and manual preview boundaries are in
`ANDROIDBOX_LOCAL_APK.md`.

On the source-free recovery boot, the QEMU touch path launches from All apps,
returns Home, opens the capacity-one compatible identity in Overview, and
activates that recent once. Generation-bound request sequences 1 and 2 each
cause a fresh package-store recovery, full blob readback, APK/content and signer
verification, Manifest/Resources-1 validation, and constructor-to-`onCreate`
execution. Each runtime pass is `reads=389 writes=0 flushes=0`; the whole-disk
SHA-256 remains unchanged and the displayed Activity text is
`Hello from a Mac-built APK`. Back finishes the compatible session, clears the
recent identity, and leaves an empty inert Overview.

The two byte-identical Activity PPMs have SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`.
The identity-only compatible Overview has SHA-256
`ff4012ba8d990bb6defc2944fc44021679ce5da72835d456a5446589f610e500`;
the empty Overview before and after its inert tap has SHA-256
`19363d900d0a489676b1ca24ada784a167609a626fd899a3b263d3ccd3bcef44`.

## EL0Runtime-0 child profile

`androidbox-el0-runtime0` is a strict opt-in child of this contract. It moves
only foreground compatible-Activity ownership; it does not change package
mutation, signer, resource, lifecycle, or compatibility admission. The parent
`androidbox-apk-install0` profile remains ABI 44 with syscall 59/60 and
`BUE1` v6. The child uses ABI 45, keeps `BUC1` v8, advances server events to
`BUE1` v7 for `active_client=App, app=None`, and adds syscall 61 with canonical
128-byte `BNDACM01`.

The child pipeline is:

1. Launcher reserves/selects the compatible identity and submits syscall 60
   with its nonzero compatible-session ID.
2. The IRQ-enabled package monitor freshly recovers and reads the durable APK,
   applies the bounded checks, and prepares one immutable App-addressed VMO.
   Launcher receives only the metadata snapshot and no APK bytes, VMO handle,
   storage capability, or Activity pixels.
3. After the accepted Foreground SystemUI update, SurfaceServer internally
   focuses App with no `ShellAppId`. App alone may consume the one-shot
   generation/session/digest-bound claim through syscall 61. The returned
   handle has exactly `READ`; duplicate, transfer, mapping, write, execute,
   wait, and package-store authority are absent.
4. App uses bounded `VmoRead`, independently rechecks APK SHA-256, APK-v2
   signer, Manifest identity, and Resources-1, executes the admitted
   constructor/`onCreate` subset, wipes the App-private APK copy, and publishes
   Activity pixels from the App graphics buffer. Launcher neither executes nor
   draws that Activity.

The kernel monitor still uses the existing bounded AndroidBox interpreter for
admission/profile validation before it publishes the immutable VMO. The child
moves foreground execution verification and visible raster ownership to App
EL0; it does not yet move every parser/interpreter step out of EL1.

The offline Mac-built APK is unchanged. The completed child gate
`scripts/check-androidbox-el0-runtime0.sh`, with retained evidence at
`target/androidbox-el0-runtime0/check.R6tuKT/`, records ABI 45, two Launcher
collections with `apk_bytes_exposed_to_launcher=0`, two App-only
`rights=READ` claims, App-owned Activity presents, Launcher-owned
Home/Overview, Back clearing the recent, an unchanged package disk, and no
panic. Its two App-owned Activity PPMs are byte-identical with SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`.
Its `ANDROIDBOX_EL0_RUNTIME0_QEMU_OK` marker is separate from, and does not
retroactively change, the completed ABI 44 automated Install-0/local-APK gates
or their terminal markers.

EL0 ownership is not broader Android compatibility. Resources-1 remains the
static TextView-only pure-data subset: one exported `MAIN`/`LAUNCHER`
Activity, one admitted constructor→`onCreate` shape, one compiled root
`TextView`, and one default-config string. ART, Dalvik, ActivityThread,
general Android Framework, Binder, Bionic, JNI, native libraries, arbitrary
View/layout inflation, resource qualifiers/aliases, permission/service/network
bridges, general APKs, phone hardware, and real-device execution are absent.

## Process-0 child profile

`androidbox-process0` is a strict opt-in ABI 47 child of
`androidbox-interactive0`; it does not alter the ABI 44–46 parent contracts or
their retained gates. It adds a separately linked AndroidApp EL0 image and
moves the immutable package image, bounded InteractiveActivity-1 interpreter,
and retained `ActivitySession` out of App EL0. App remains the trusted
Surface/input/raster host.

Init creates one private App↔AndroidApp Channel before transferring its peer.
In steady state AndroidApp owns exactly that Channel with
`READ|WRITE|WAIT`. It has no Surface, graphics, input, storage, duplicate, or
transfer authority. During an Open, only AndroidApp may consume the one-shot
syscall-61 package grant; the temporary package VMO has exactly `READ` and the
runtime accepts at most one such VMO. Launcher and App receive package
metadata but no APK VMO or bytes.

Canonical fixed-size `BNDAPC01` RPC binds sender PID, request ID, session,
package generation, view ID, and revision. The completed bounded lifecycle is
`Ready(0) → Open(1) → Opened/text chunks → Click(2) → Updated/text chunk →
Close(3) → Closed`; queue capacity is eight and at most one request is
outstanding. AndroidApp interprets the admitted callback and returns pure-data
scene updates. App receives input and performs all rendering, so AndroidApp
never becomes a graphics producer.

`scripts/check-androidbox-process0.sh` performs two offline QEMU boots, each
with exactly one `-nic none`: explicit `fw_cfg` installation and source-free
recovery from the same package disk. Its retained evidence is
`target/androidbox-process0/check.sFpefj/`, and
`ANDROIDBOX_PROCESS0_QEMU_OK` is the terminal marker. The recovery proof
contains distinct App/AndroidApp ELF hashes, PIDs, ASIDs, and page-table roots,
the complete authenticated RPC transcript, the real Button callback, 4138
changed content pixels, byte-identical system chrome, and an unchanged package
disk.

This is a dedicated Bndroid process for one pinned InteractiveActivity-1
profile, not a general Android application runtime. ART, Dalvik,
ActivityThread, Binder, Bionic, JNI, native libraries, general resources and
views, PackageManager/permissions/services, networking, background execution,
AndroidApp crash recovery, phone hardware, and real-device execution remain
absent.

## ABI and UI publication

The parent `androidbox-apk-install0` profile uses ABI 44. Syscall 59 is the
boot-catalog read and copies one canonical 640-byte `BNDAPS01`
installed-package snapshot to EL0. It exposes no APK bytes, handle, storage
capability, or mutation authority.

Launcher-only syscall 60 accepts a canonical 640-byte `BNDARQ01` request. It is
bound to a nonzero monotonic request sequence plus the exact generation,
version, APK length, Resources-1 profile, APK and signer digests, package, and
Activity from the boot catalog. Submission queues one request; Launcher waits
and retries the byte-identical wire. The IRQ-enabled package service freshly
calls package-store recovery and `read_blob`, rechecks APK SHA-256, APK v2
signer, Manifest identity, Resources-1, and the two-instruction constructor
followed by the four-instruction `onCreate`. Success returns a fresh
`BNDAPS01`. Sequence replay/gaps, a stale generation, owner change, identity
mismatch, corruption, or an unsupported component fail closed without
displaying cached Activity content.

Settings has an **Apps** page showing boot-catalog identity, version, APK size,
generation, Resources-1 profile, and digest prefixes. All apps replaces the
local demo label only when that catalog reports an installed generation; its
touch token and syscall request preserve that generation. Navigation to the
Activity occurs only after the fresh response matches the catalog. There is no
UI/API for install, update, uninstall, permissions, or general package
management. Update-0 is driven only by the explicit boot APK source;
Uninstall-0 has only the separate trusted-host boot request and does not add a
runtime mutation syscall.

`CompatibleActivitySession-0` is complete as a capacity-one, boot-local
identity contract. Before either All apps launch or Overview activation,
Launcher sends a canonical `BUC1` v8 reservation request. SurfaceServer
serializes `Reserve → fresh syscall 60 verification → commit or abort` and
returns one canonical completion with explicit `accepted`, `conflict`, or
`capture-busy` status. The ABI 44 parent encodes it as `BUE1` v6; the ABI 45
child uses `BUE1` v7 so the same ordered stream can additionally carry
compatible `App/None` focus. Only `accepted` grants the subsequent bounded
step; the other statuses do not imply state change or launch authority.
Reserve starts only from a known released input state. Pending samples are
quarantined; commit or abort drains at most 64 queued samples, and a pressed
tail or the drain limit keeps consuming through and including the matching
release.

Home retains only `(session_id, package_generation)` and performs no background
execution. Overview renders that system-owned identity but no Activity pixels,
thumbnail, screenshot, or live preview. Selecting the recent performs a new
syscall 60 readback and verification instead of reusing cached Activity
content. Back finishes the session and clears the identity. `BUP1` v2 binds a
mobile present to its nonzero `system_ui_revision`; SurfaceServer rejects a
stale raster before accepting its presentation, focus transition, syscall
effect, or counters, while preserving the first transferred buffer for a
same-frame retry.

## Compatibility boundary

The completed evidence covers the repository v2/v3 pair and the separate
Mac-built package. All are APK-v2-only, RSA-2048/SHA-256, one-signer,
Resources-1 artifacts. The two historical repository fixtures retain optional
DEX-0 diagnostics, while the Mac package proves those diagnostics are not an
admission requirement.
This does not establish arbitrary APK installation, general update
compatibility, or Android compatibility. Uninstall-0 only removes the one
durable demo identity
through a trusted boot-time request; it neither erases APK blobs nor manages
per-package mutable data. ART, ActivityThread, Android Framework, Binder,
Bionic, JNI, native libraries, permission bridging, Android networking,
background services or compatible-Activity background execution, multiple
packages, a general PackageManager, and
update/uninstall UI remain absent. In the ABI 44 parent, relaunch executes in
the bounded EL1 pure-data Resources-1 model and Launcher displays the result.
In the ABI 45 child, foreground re-verification/execution and Activity pixels
belong to the resident App EL0 process through the one-shot read-only VMO;
Launcher still receives no APK bytes or Activity raster. ABI 46 adds the
bounded retained callback session in App. ABI 47 moves that interpreter and
session into a distinct AndroidApp EL0 process while App retains input and
raster ownership. None of these paths is ART or a general Android app runtime.
Older kernels are not
tombstone-aware and have no downgrade-safe claim. The QEMU runs have network
disabled and prove neither phone hardware nor a real-device port.

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
abi44_activity_owner=kernel-el1-pure-data+launcher-raster
abi45_activity_owner=app-el0-read-only-vmo+app-raster
abi45_launcher_apk_bytes=0 abi45_launcher_activity_pixels=0
abi47_interpreter_owner=androidapp-el0 abi47_raster_owner=app-el0
abi47_androidapp_graphics=0 abi47_androidapp_input=0
standalone_general_android_runtime=0 compatible_activity_session=1
compatible_activity_capacity=1 compatible_activity_persistence=boot-local
compatible_activity_background_execution=0 overview_activity_pixels=0
overview_thumbnail=0 overview_live_preview=0 real_phone_claim=0
```

## Follow-on order

1. Completed Install-0: one externally supplied, signed Resources-1 fixture.
2. Completed Update-0: the exact same-package/same-signer v2-to-v3 pair,
   strictly increasing version, and atomic generation 1/slot 0 to generation
   2/slot 1 switch.
3. Completed Uninstall-0/reinstall gate: generation
   `1→2→removed 3→reinstalled 4`, identical dual-registry tombstones, logical
   APK revocation, and `NoManagedPackageData`.
4. Completed local-APK gate: one separately built `org.bndroid.macdemo`
   Resources-1 package with exact Manifest-to-DEX Activity binding and no
   fixed DEX probe class; its pure-data lifecycle is the required
   two-instruction public no-argument constructor followed by `onCreate`.
5. Completed ABI-44 durable icon relaunch: Launcher-only 640-byte `BNDARQ01`,
   exact retry, fresh readback/reverification, `BNDAPS01` success, and two
   write-free Mac-APK launches with sequences 1 and 2.
6. Completed automated ABI-45 EL0Runtime-0 child gate:
   Launcher metadata-only collection, one-shot App-only read VMO claim, App
   re-verification/execution, and App-owned Activity pixels.
7. Completed `CompatibleActivitySession-0`: one boot-local recent identity,
   Home retention without background execution, identity-only Overview, fresh
   syscall 60 recent activation, Back-to-finish, explicit reservation
   completion, and stale-raster rejection through `BUP1` v2.
8. Completed ABI-46 InteractiveActivity-1 gate: retained real Button callback,
   App-owned content, SurfaceServer-owned chrome, and protected input routing.
9. Completed ABI-47 Process-0 gate: distinct AndroidApp EL0 image, private
   authenticated RPC, one-shot read-only package grant, source-free recovery,
   and App-only input/raster ownership.
10. Multiple packages, quotas, stable application identities, signer rotation,
   and per-app storage isolation. ART/ActivityThread and general Android
   Framework compatibility remain separate later work.
