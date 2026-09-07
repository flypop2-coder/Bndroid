# AndroidBox APK Update-0 contract

Status: completed and evidenced for one exact repository-owned
Resources-1 package transition on local QEMU. The completed scope is
**Update-0**, not a general Android updater, package manager, or Android
compatibility claim.

This contract extends `ANDROIDBOX_APK_INSTALL_0.md`. Install-0 first commits
version 2 as generation 1; Update-0 then admits one same-package,
same-certificate, strictly higher-version APK and atomically publishes it as
generation 2.

The storage/kernel path can subsequently accept the boot-time Uninstall-0
request defined in `ANDROIDBOX_APK_UNINSTALL_0.md`. Its separate 12-boot gate
now proves generation `1→2→removed 3→reinstalled 4`; the completed Update-0
gate below remains an independent six-boot evidence set.

## Exact base and update artifacts

Both APKs contain package `org.bndroid.demo`, launcher Activity
`Lorg/bndroid/demo/MainActivity;`, exactly four STORED ZIP entries, and one APK
Signature Scheme v2 signer. Both are 12566 bytes.

| Field | Install base | Update candidate |
|---|---|---|
| fixture | `fixtures/androidbox-resource-demo/androidbox-resource-demo.apk` | `fixtures/androidbox-resource-update-demo/androidbox-resource-update-demo.apk` |
| `versionCode` / `versionName` | `2` / `2.0` | `3` / `3.0` |
| title | `AndroidBox Resources-1 Demo` | `AndroidBox Resources-1 Demo v3` |
| TextView text | `AndroidBox resource-backed view` | `AndroidBox updated resource view` |
| APK SHA-256 | `2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7` | `3860fcd80eed1a4b284514316bc35169dda60bb1522e85abdea5aa33b8355708` |
| `resources.arsc` CRC-32 | `0x9f67c7b4` (`2674378676`) | `0xba0ad990` (`3121273232`) |
| binary layout CRC-32 | `0x36ce95c4` (`919508420`) | `0x36ce95c4` (`919508420`) |
| layout resource ID | `0x7f020000` | `0x7f020000` |
| text resource ID | `0x7f030000` | `0x7f030000` |

The shared signer certificate SHA-256 is:

```text
e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
```

The admitted signature algorithm is APK v2 `0x0103`, RSA-2048 with exponent
65537, PKCS#1 v1.5, and SHA-256. v1, v3, v3.1, v4, SourceStamp, multiple
signers, signer lineage, and signer rotation are outside Update-0.

The certificate and private key under
`fixtures/androidbox-resource-demo/` are deliberately repository-public,
test-only reproducibility material. The update fixture references those files;
it does not copy signing material into its own directory. This signer is not a
production publisher identity, trust anchor, HSM-custody claim, or secure key
provisioning claim.

## Admission and monotonic update policy

All source parsing, Resources-1 execution, manifest validation, and APK v2
verification complete before the first package-store write.

Given one already committed package, Update-0 permits a mutation only when:

1. the candidate package string exactly equals the installed package;
2. the candidate signer certificate SHA-256 exactly equals the installed
   signer certificate SHA-256;
3. the candidate `versionCode` is strictly greater than the installed
   `versionCode`;
4. the candidate APK digest matches its admitted metadata; and
5. the unique exported `MAIN`/`LAUNCHER` Activity still passes the bounded
   Resources-1 profile.

An exact replay of the already committed transaction ID, metadata, APK length,
and bytes is idempotent and write-free. Reusing a transaction ID with different
metadata or content is a conflict. A different package, different signer,
equal or lower version, invalid signature, invalid manifest, or incompatible
Resources-1 source is rejected before update mutation.

`versionName` is APK metadata verified for these fixtures but is not an
Update-0 policy input. Monotonicity is enforced on the integer `versionCode`;
Update-0 does not implement semantic-version ordering, downgrade
authorization, signer rotation, lineage, staged rollout, channels, or rollback
packages.

## Double-slot atomic publication

The one-package store has two registry slots and two fixed-capacity APK blob
slots. If generation `g` is active in slot `s`, a candidate is written to
inactive slot `1-s` in this order:

1. Write the complete candidate blob and its header to the inactive blob slot.
2. Flush the blob.
3. Read back and validate the complete blob header, payload, APK length,
   metadata, and SHA-256.
4. Write a committed registry record to the inactive registry slot. The record
   binds the new generation, inactive blob slot, transaction ID, package,
   version, Activity, Resources-1 profile, APK SHA-256, and signer certificate
   SHA-256.
5. Flush the registry.
6. Read back the registry/blob pair and publish success only if the complete
   pair validates together.

The old registry/blob pair is not overwritten before the candidate pair is
durable. A new blob without a committed matching registry is an invisible
orphan. Recovery never combines a registry from one generation with a blob
from another. Before registry publication the observable result is the old
generation; after complete publication it is the new generation.

An error at the final commit/ack boundary is `OutcomeUnknown`. The caller must
recover and inspect the two slots; it must not blindly repeat a mutation.
Exact-transaction retry becomes a zero-write replay if the new generation was
already committed.

The evidenced transition is generation 1 / slot 0 / version 2 to generation 2
/ slot 1 / version 3.

## Runtime decision markers

The canonical decision line begins with `APK_PACKAGE_STORE_OK format=2`. Its
`operation` field is closed to these successful Update-0 paths:

- `operation=install`: fresh store to generation 1;
- `operation=update`: same package/signer and increasing version to generation
  2;
- `operation=source-replay`: exact committed source, no mutation;
- `operation=recovery`: no source, recover the newest complete pair without
  mutation.

The four positive boots in the completed gate emitted these exact decision
facts:

```text
operation=install previous_generation=0 previous_version_code=0 generation=1 slot=0 version_code=2 reads=1032 writes=130 flushes=3
operation=update previous_generation=1 previous_version_code=2 generation=2 slot=1 version_code=3 reads=905 writes=129 flushes=2
operation=source-replay previous_generation=2 previous_version_code=3 generation=2 slot=1 version_code=3 reads=904 writes=0 flushes=0
operation=recovery previous_generation=2 previous_version_code=3 generation=2 slot=1 version_code=3 reads=645 writes=0 flushes=0
```

The existing opt-in feature/profile name remains
`androidbox-apk-install0`; its exact profile marker now explicitly carries:

```text
update0=same-package-same-signer-monotonic-version atomic_old-new-switch=1 exact-source-replay=idempotent-zero-write rollback-source=reject-before-write
```

Rejected rollback and signature-tamper boots emit no
`APK_PACKAGE_STORE_OK`, `ANDROIDBOX_INSTALLED_ACTIVITY_OK`, or profile success
marker. Their exact terminal reasons are:

```text
STORAGE_FAIL reason=package_manager_invalid
boot error: storage validation failed: package update version must strictly increase
boot error: storage validation failed: APK v2 signature admission failed
```

## Completed six-boot local-QEMU evidence

The offline gate is `scripts/check-androidbox-apk-update0.sh`. Its retained
evidence directory is:

```text
target/androidbox-apk-update0/check.yu1Xpq/
```

It builds one unchanged release kernel/userspace image, accepts only two
explicit absolute APK paths, does not scan for APKs, and launches every QEMU
boot with exactly one `-nic none`.

The six boots are:

1. **base-install** — supplies signed version 2 through read-only `fw_cfg`,
   formats the package store, commits generation 1 / slot 0, fully reads the
   APK back, re-verifies it, and launches
   `AndroidBox resource-backed view`.
2. **update** — supplies signed version 3 to the generation-1 disk, commits
   generation 2 / slot 1, fully reads the updated APK back, re-verifies it, and
   launches `AndroidBox updated resource view`.
3. **replay** — supplies the exact committed version-3 source again; generation
   remains 2 and package I/O is `writes=0 flushes=0`.
4. **recovery** — supplies no APK source; generation 2 is recovered and
   launched with `writes=0 flushes=0`.
5. **rollback-negative** — supplies old version 2 to a copy of the
   generation-2 disk; monotonic-version admission rejects it and the complete
   disk is unchanged.
6. **tamper-negative** — supplies a one-byte content-tampered version-3 APK to
   a saved generation-1 disk; APK v2 admission rejects it and the complete disk
   is unchanged.

The gate's terminal success marker binds the two content identities and the
completed state:

```text
ANDROIDBOX_APK_UPDATE0_QEMU_OK artifact_dir=/Users/apple/Desktop/Bndroid/target/androidbox-apk-update0/check.yu1Xpq base_apk_sha256=2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7 update_apk_sha256=3860fcd80eed1a4b284514316bc35169dda60bb1522e85abdea5aa33b8355708 signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf base_generation=1 update_generation=2 update_slot=1 replay_writes=0 replay_flushes=0 recovery_writes=0 recovery_flushes=0 rollback_disk_unchanged=1 tamper_disk_unchanged=1
```

Every successful Activity marker is sourced from package-store readback.
Version 3 produces:

```text
title=AndroidBox Resources-1 Demo v3
text=AndroidBox updated resource view
constructor=1
constructor_instructions=2
on_create=1
on_create_instructions=4
resources_arsc_crc=3121273232
layout_xml_crc=919508420
layout_resource_id=2130837504
string_resource_id=2130903040
```

## Complete-disk hashes and bounded mutation

All disk artifacts are exactly 16 MiB:

| State | Whole-disk SHA-256 |
|---|---|
| deterministic empty package store | `b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801` |
| generation 1 / version 2 | `37b2afa4a85153ef84f2329648eb3e364f8629c78cca17eb3a45358f48bb9965` |
| generation 2 / version 3 | `d3166bc25b95d28d3a18ee612fe03be96d342d2b37ac620947fe71828abba1de` |

Exact-source replay and source-free recovery both preserve the generation-2
hash. Rejected rollback preserves the generation-2 hash. Rejected tampered
version 3 preserves the saved generation-1 hash.

Whole-image comparison records:

- base install: 4781 changed bytes, offsets `8388608..8409873`;
- update: 4742 changed bytes, offsets `8389632..8475409`;
- both transactions: `outside_changed_bytes=0`;
- allowed partition: `BNDROID_PACKAGES`, LBA `16384..16895` inclusive.

This proves bounded mutation in these completed QEMU transactions. It does not
prove resistance to host disk rollback, erasure, malicious image replacement,
controller-cache loss, or physical-media failure.

## Host fault injection and its boundary

`bndr-package-store` currently has 43 host unit tests. The suite
includes deterministic in-memory block-device fault injection for every update
write failure, every torn update-write prefix, and every flush failure. It also
covers pre-commit orphan invisibility, post-commit acknowledgement loss,
`OutcomeUnknown`, exact retry, zero-write stable recovery, same-package/signer/
increasing-version policy, slot alternation, stale-handle rejection, newest
registry/blob corruption fallback, epoch/CRC enforcement, nonzero padding,
generation exhaustion, and volume bounds.

The expanded suite also covers the `Removed` state, identical
same-generation tombstone mirroring, uninstall write/torn-write/flush/readback
failures, exact zero-write replay, one-copy repair, non-revival after one
tombstone is cleared or corrupt, conflicting-tombstone rejection, and
reinstall identity/signer/version/digest policy.

Those 43 are host tests, not 43 QEMU boots. Their block device and failures are
software models; they are not local-QEMU host-cut evidence and not physical
power-loss evidence.

The six-boot QEMU gate proves complete transactions, replay, recovery, and two
pre-mutation rejections. It does not inject a process kill between blob and
registry publication or after registry durability but before acknowledgement.
Neither evidence set proves PMIC behavior, volatile controller caches, real
flash ordering, FTL behavior, torn physical sectors, RPMB/eFuse monotonicity,
hardware rollback resistance, or recovery after a physical power cut.

## ABI 44 boot catalog and read-only relaunch

The isolated profile uses ABI 44. Syscall 59 remains the boot-catalog read and
publishes one canonical 640-byte `BNDAPS01` installed-package snapshot to
Launcher and App. The generation-2 boots report:

```text
installed=1 generation=2 version_code=3 authority_granted=0 apk_bytes_exposed=0
```

EL0 receives no APK bytes, package-store handle, write capability, or install/
update authority.

ABI 44 also provides Launcher-only syscall 60. Its canonical 640-byte
`BNDARQ01` request binds a nonzero monotonic sequence to the exact generation,
version, length, Resources-1 profile, APK/signer digests, package, and Activity.
Launcher submits and exact-retries the same wire while an IRQ-enabled service
freshly recovers the registry, calls `read_blob`, rechecks APK SHA-256 and APK
v2 signer, reparses Manifest/Resources-1, and executes the two-instruction
constructor followed by the four-instruction `onCreate`. Only a matching
success returns a fresh `BNDAPS01` and permits navigation. This path has no
write or mutation authority.

The two-sequence QEMU proof currently belongs to the separately Mac-built
generation-1 APK and is documented in `ANDROIDBOX_LOCAL_APK.md`. It reports
`reads=389 writes=0 flushes=0` for both sequence 1 and sequence 2 and preserves
the whole-disk SHA-256. It must not be generalized into evidence that every
Update-0 candidate runs.

Earlier local-QEMU visual evidence uses the exact generation-2 disk hash above
and a source-free recovery:

```text
target/androidbox-apk-update0-live-2/screenshots/apps-v3.png
target/androidbox-apk-update0-live-2/screenshots/activity-v3.png
```

The 720x1600 Settings **Apps** page visibly reports title
`AndroidBox Resources-1 Demo v3`, package `org.bndroid.demo`, Version `3`, APK
size `12566 bytes`, Generation `2`, `APK v2 verified`, Resources-1, APK digest
prefix `3860fcd8`, signer prefix `e7412e1c`, Activity identity, and updated
TextView text. Opening the installed entry shows
`AndroidBox updated resource view` as output from durable APK readback and
again reports Version 3 / Generation 2. That interaction produced 43 complete
720x1600 surface commits without a UI fault marker. These retained screenshots
predate the ABI-44 two-sequence relaunch proof and are not cited as syscall-60
evidence.

The catalog and relaunch paths are read-only. There is still no runtime
install, update, rollback, uninstall, permission, or general PackageManager
UI/API. Uninstall-0 accepts only an offline trusted-host `fw_cfg` request
before EL0; it does not add a mutation syscall or Settings action. Successful
relaunch is an EL1 pure-data Resources-1 execution, not ART and not a
standalone Android application process.

## Android compatibility and product boundary

Update-0 proves only one repository-owned Resources-1 package changing from
the exact version-2 APK to the exact version-3 APK under the same public test
signer. It does not prove:

- arbitrary APK installation or update;
- multiple packages, quotas, stable application identities, per-app package
  data migration, runtime uninstall, or a managed-data retention/deletion
  policy;
- production publisher policy, signer rotation, signing lineage, app-store,
  download, USB, or user-selected sideload flows;
- ART, Dalvik, ActivityThread, general Android Framework, ResourceManager,
  arbitrary Activities, Views, layouts, qualifiers, services, broadcasts, or
  content providers;
- Binder, Bionic, JNI, native libraries, Linux ABI compatibility, Android
  permissions, or Android sandbox integration;
- Android networking, background execution, notifications, audio, camera,
  telephony, cellular, Wi-Fi, Bluetooth, sensors, or location;
- OTA/system update, verified boot, rollback protection, production secure
  storage, physical power-loss safety, or hardware-backed monotonic state;
- a physical display, DPI, refresh rate, touch panel, BSP, device drivers,
  phone hardware, a real-device port, or a usable real phone.

The exact boundary remains:

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
standalone_android_process=0 compatible_activity_session=1
compatible_activity_capacity=1 background_execution=0 activity_pixels_in_overview=0
network=disabled emulator_only=1 real_phone_claim=0
```

The follow-on Uninstall-0 does not erase APK blob sectors and has only
`NoManagedPackageData`; it must not be described as secure deletion or Android
application-data cleanup. Its `BNDPRM01` tombstone is not understood by older
kernels, so an Uninstall-0 disk has no downgrade-safe claim.

## Follow-on order

1. Completed Install-0: one signed Resources-1 package to generation 1.
2. Completed Update-0: exact same-package/signer version 2 to version 3,
   atomic generation 1 to generation 2.
3. Completed boot-time Uninstall-0/reinstall gate with identical
   same-generation tombstones, logical APK revocation, no managed package
   data, and generation `1→2→removed 3→reinstalled 4`.
4. Completed ABI-44 generation-bound, read-only durable relaunch for the
   separately Mac-built Resources-1 package.
5. Completed separately: `CompatibleActivitySession-0`, with a boot-local,
   capacity-one identity, fresh verification on recent activation, Home
   without background execution, identity-only Overview, and Back finish. It
   does not create a separate Android app process.
6. General multi-package install/update/uninstall, quotas, stable identities,
   per-app storage isolation, signer rotation, and user-facing PackageManager.
7. ART/ActivityThread and broader Android framework compatibility.
