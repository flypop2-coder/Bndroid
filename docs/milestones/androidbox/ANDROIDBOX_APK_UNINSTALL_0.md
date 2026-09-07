# AndroidBox APK Uninstall-0 contract

Status: completed and evidenced for one exact boot-time removal/reinstall
lifecycle on local QEMU. The retained gate moves the repository Resources-1
fixture through generation 1 install, generation 2 update, generation 3
removal, and generation 4 reinstall. This remains a deliberately bounded
Uninstall-0 result, not general package management or Android compatibility.

Uninstall-0 removes one already committed AndroidBox Resources-1 package from
the installed catalog. It is not a runtime Settings action, a general
PackageManager, secure deletion, managed application-data deletion, arbitrary
APK compatibility, or a real-phone claim.

## Authority and invocation boundary

The only Uninstall-0 authority is an explicit, trusted, offline host launch
decision made before EL0 starts:

- QEMU exposes exactly one 256-byte canonical request as the read-only
  `fw_cfg` file `opt/bndroid/package-uninstall`.
- The request and `opt/bndroid/apk` are mutually exclusive. A boot cannot
  install/update and uninstall in the same package-manager pass.
- The package source directory is bounded and scanned once. The selected
  request is copied into kernel-owned staging memory and validated before the
  first package-store mutation.
- The completed acceptance gate ran with networking disabled and exactly one
  `-nic none` per QEMU invocation.
- EL0 receives no request handle, APK bytes, package-store handle, storage
  capability, or mutation authority.

“Trusted host” means the operator that already controls this local QEMU
invocation and its disk. `fw_cfg` is not a remote authentication protocol, an
app-store authorization, USB installer, user confirmation UI, production
policy engine, or protection against a malicious host.

There is deliberately no runtime uninstall syscall, Settings button, Binder
API, Installer API, PackageManager service API, or Android permission flow.
ABI 44 syscall 59 remains read-only and returns the boot-stable 640-byte
`BNDAPS01` catalog snapshot. Launcher-only syscall 60 is also read-only: it
accepts one 640-byte generation-bound `BNDARQ01` relaunch request and can
return a fresh `BNDAPS01` only after durable readback and verification. It
cannot install, update, remove, repair, or otherwise mutate package state.

## Canonical request

The request wire is `BNDUNS01`, version 1, flags 0, exactly 256 bytes, with
little-endian scalar fields and an IEEE CRC-32 over bytes `0..252`. It binds:

- one non-zero operation ID;
- expected registry generation;
- expected `versionCode`;
- expected APK byte length;
- exact printable-ASCII package name;
- expected APK SHA-256;
- expected signer-certificate SHA-256; and
- disposition `NoManagedPackageData`.

The parser rejects a wrong length, CRC, magic, version, flags, zero operation
ID, zero/oversized expected values, empty/oversized/non-printable package,
non-zero package padding or reserved bytes, zero digests, and unknown data
disposition. After recovery, the kernel compares every target field with the
current durable installed-package handle before the first tombstone write.
A stale generation, version, length, package, APK digest, or signer digest
therefore fails closed as a target mismatch.

CRC-32 detects accidental corruption and enforces a canonical wire; it is not
a signature or authorization mechanism. Authority comes only from the trusted
offline host boundary above.

## Durable removal state

The version-1 package-store namespace now has three logical states:

`Empty`, `Installed`, and `Removed`.

Removal does not clear a registry record and does not treat “no valid record”
as success. Instead it writes a sealed `BNDPRM01` tombstone that preserves the
last installed identity:

- uninstall operation ID;
- removal generation and last installed generation;
- package and launcher Activity;
- last `versionCode`, APK length, blob slot, and install transaction ID;
- Resources-1 compatibility profile;
- APK SHA-256 and signer-certificate SHA-256; and
- `NoManagedPackageData`.

For an installed generation `g`, Uninstall-0 computes checked generation
`g + 1` and performs:

1. Write the tombstone to the inactive registry slot.
2. Flush and read back that exact tombstone.
3. Write the logically identical tombstone, with the same removal generation,
   to the other registry slot.
4. Flush and read back the mirror.
5. Return success only after the identical dual-tombstone state is durable.

Before the first complete tombstone, recovery exposes the old installed
package. After the first complete tombstone, recovery exposes `Removed`; a
failure before the mirror cannot revive the older install. Replaying the same
operation repairs a missing mirror, then a complete exact replay performs no
writes or flushes. Two same-generation tombstones are accepted only when they
are logically identical; a conflicting pair is ambiguous corruption and
fails closed.

This is a deterministic software crash-consistency protocol. It is not proof
of physical sector atomicity, controller-cache ordering, flash translation
layer behavior, PMIC power loss, or real-device recovery.

## Logical reachability, blobs, and application data

Uninstall-0 does **not** erase either fixed APK blob slot. Old APK bytes can
remain physically present in `BNDROID_PACKAGES`, but the newer tombstone makes
them logically unreachable through `recover`, `read_blob`, syscall 59, syscall
60, the Launcher catalog, and the installed Activity route. A pre-removal
`InstalledPackage` handle becomes stale.

This distinction is intentional:

```text
installed=0 removed=1 logical_apk_reachable=0 apk_blob_erased=0
data_disposition=no-managed-package-data
```

There is currently no managed per-package mutable application data in this
profile. `NoManagedPackageData` means no such data existed for Uninstall-0 to
delete; it must not be described as “user data erased”, “cache cleared”,
cryptographic erase, secure wipe, or a data-retention choice for a general
Android application.

## Reinstall policy after removal

The tombstone remains an identity and rollback floor for a later reinstall.
The single-package store requires:

- the same package name;
- the same signer-certificate SHA-256; and
- either a higher `versionCode`, or the same `versionCode` with the same APK
  SHA-256.

A lower version, changed same-version content, different package, or different
signer is rejected before mutation. A successful reinstall advances from the
removal generation and uses the blob slot opposite the last installed slot.
If reinstall is interrupted, recovery returns either the durable tombstone or
the complete new install; it must not fall back to the package that preceded
the tombstone.

This is a local single-store rule, not trusted anti-rollback hardware.
The QEMU disk remains host-controlled and can be copied, replaced, erased, or
rolled back.

## Host-test evidence

`bndr-package-store` currently contains 43 host unit tests. In addition to the
Install-0 and Update-0 cases, they cover:

- exact recovery of `Empty`, `Installed`, and `Removed`;
- two registry writes for a fresh uninstall;
- validation failures and exact dual-tombstone replay with zero writes;
- one-copy replay repair followed by zero-write replay;
- every uninstall write failure and four torn prefixes at each write;
- flush, readback, and acknowledgement-loss recovery;
- one cleared or corrupt tombstone without old-package resurrection;
- rejection of conflicting same-generation tombstones;
- reinstall package/signer/version/digest rules;
- failed or corrupt reinstall falling back to `Removed`, not the old install;
  and
- uninstall/reinstall generation exhaustion before mutation.

These 43 are deterministic in-memory block-device tests, not 43 QEMU boots,
host-cut tests, physical-power tests, or real-media tests.

## Completed 12-boot local-QEMU evidence

The offline gate is `scripts/check-androidbox-apk-uninstall0.sh`. It requires
two explicit absolute APK paths, scans no APK directory, pins `TMPDIR` inside
each run's target evidence tree, and retains its completed evidence in:

```text
target/androidbox-apk-uninstall0/check.gWbx8U/
```

It can be rerun with:

```bash
./scripts/check-androidbox-apk-uninstall0.sh \
  --base-apk "$PWD/fixtures/androidbox-resource-demo/androidbox-resource-demo.apk" \
  --update-apk "$PWD/fixtures/androidbox-resource-update-demo/androidbox-resource-update-demo.apk"
```

One unchanged kernel/userspace build completed these 12 boots:

1. **base-install** — v2 becomes generation 1 / slot 0.
2. **update** — v3 becomes generation 2 / slot 1.
3. **uninstall** — the matching `BNDUNS01` request publishes removal
   generation 3.
4. **uninstall-replay** — the exact request is a zero-write replay.
5. **removed-recovery** — no input recovers `Removed` without an Activity.
6. **wrong-target-negative** — a different package target is rejected.
7. **stale-generation-negative** — an otherwise matching stale generation is
   rejected.
8. **bad-crc-negative** — request admission rejects an invalid CRC-32.
9. **apk-request-conflict-negative** — simultaneous APK and uninstall request
   inputs are rejected as ambiguous authority.
10. **rollback-negative** — version 2 cannot replace the removed version-3
    identity.
11. **reinstall** — the exact retained version-3 digest becomes generation 4 /
    slot 0.
12. **reinstall-recovery** — no input recovers generation 4 and reproduces the
    durable Activity.

The exact state and I/O sequence is:

```text
generation 1 install: reads=1032 writes=130 flushes=3
generation 2 update: reads=905 writes=129 flushes=2
generation 3 uninstall: reads=520 writes=2 flushes=2
exact uninstall replay: reads=6 writes=0 flushes=0
removed recovery: reads=3 writes=0 flushes=0
generation 4 reinstall: reads=521 writes=129 flushes=2
generation 4 recovery: reads=389 writes=0 flushes=0
```

Both removal registry sectors are byte-identical `BNDPRM01` records at
generation 3. Comparing generation 2 with generation 3 finds exactly 404
changed bytes, all confined to registry LBAs `16385` and `16386`; both APK
blob regions remain byte-for-byte unchanged. The exact-request replay and
source-free removed recovery preserve the complete generation-3 disk.

Wrong package, stale generation, bad request CRC, simultaneous APK/request,
and post-removal version-2 rollback are each tested on an independent copy of
the generation-3 image. Every negative boot leaves its complete input disk
byte-for-byte unchanged. Reinstall advances generation `3→4`, and its
source-free recovery preserves the disk and emits Activity readback identical
to the reinstall boot.

The retained whole-disk SHA-256 values are:

| State | Whole-disk SHA-256 |
|---|---|
| empty | `b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801` |
| generation 1 / v2 | `37b2afa4a85153ef84f2329648eb3e364f8629c78cca17eb3a45358f48bb9965` |
| generation 2 / v3 | `d3166bc25b95d28d3a18ee612fe03be96d342d2b37ac620947fe71828abba1de` |
| generation 3 / removed | `1418a1eb5f6e1276eea0947ccadde2e9fb040baadb89e44b65d5c8e26c0e5ab0` |
| generation 4 / reinstalled v3 | `4cd653580a774aeb544566a3f4d3bfd5d2e474af677d1443bb5a77988106d67f` |

The terminal marker is:

```text
ANDROIDBOX_APK_UNINSTALL0_QEMU_OK artifact_dir=/Users/apple/Desktop/Bndroid/target/androidbox-apk-uninstall0/check.gWbx8U base_apk_sha256=2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7 update_apk_sha256=3860fcd80eed1a4b284514316bc35169dda60bb1522e85abdea5aa33b8355708 signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf install_generation=1 update_generation=2 removal_generation=3 reinstall_generation=4 tombstone_copies=2 tombstone_bytes_identical=1 uninstall_replay_writes=0 uninstall_replay_flushes=0 removed_recovery_writes=0 removed_recovery_flushes=0 wrong_target_disk_unchanged=1 stale_generation_disk_unchanged=1 bad_crc_disk_unchanged=1 apk_request_conflict_disk_unchanged=1 rollback_disk_unchanged=1 reinstall_activity_persistent=1 network=disabled general_android_compatibility=0
```

This QEMU gate proves the complete bounded transactions above. It does not
inject a host cut between the two tombstone commits; that failure space remains
host-unit-test evidence. Neither evidence set proves controller-cache or
physical-power-loss behavior.

## ABI, UI, and compatibility boundary

The boot-time transaction preserves the package manager's immutable catalog
publication model: one trusted mutation decision occurs before EL0, then the
resulting installed/removed catalog is read-only for the rest of that boot. A
removed snapshot must not expose the old Activity as installed. ABI-44
relaunch does not weaken that rule: the IRQ-enabled service freshly calls
`recover` and `read_blob`, but it has a structurally read-only block interface
and succeeds only when the request, durable generation, and boot catalog all
match.

This design does not support a Settings-triggered runtime mutation. Safe
runtime uninstall would additionally require a narrowly delegated mutation
capability, caller/rights validation, a generation-qualified request ABI,
serialization with package readers, running-Activity termination, APK
reference revocation, managed-data policy, and versioned catalog-change
notification to Launcher and Settings. None of those is claimed by
Uninstall-0.

Tombstones are not downgrade-safe with older Bndroid kernels that do not
understand `BNDPRM01`. Booting an older kernel against an Uninstall-0 disk is
unsupported and has no fail-safe compatibility claim.

Uninstall-0 still does not add ART, Dalvik, ActivityThread, Binder, Bionic,
JNI, native libraries, Android permissions, services, networking, multiple
packages, arbitrary APKs, or general Android compatibility. The exact
boundary remains:

```text
install_ui=0 update_ui=0 uninstall_ui=0 package_manager_api=0
apk_blob_erased=0 old_kernel_downgrade_safe=0
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
standalone_android_process=0 compatible_activity_session=1
compatible_activity_capacity=1 background_execution=0 activity_pixels_in_overview=0
network=disabled emulator_only=1 real_phone_claim=0
```

## Follow-on order

1. Completed: retain the 12-boot offline QEMU Uninstall-0/reinstall gate and
   its exact disk, marker, and Activity evidence.
2. Completed separately: ABI-44 generation-bound read-only relaunch for one
   installed Mac-built Resources-1 package; a removed generation remains
   unreachable.
3. Completed separately: `CompatibleActivitySession-0`, with a boot-local,
   capacity-one identity, fresh verification on recent activation, Home
   without background execution, identity-only Overview, and Back finish. It
   does not claim an independent Android app process.
4. Introduce multiple packages, stable application identities, quotas, and
   isolated per-app mutable data with an explicit retention/deletion policy.
5. Design a capability-checked runtime PackageManager service and Settings
   flow only after process lifecycle, reference revocation, and catalog-change
   publication exist.
6. Continue toward ART/ActivityThread and broader Android Framework
   compatibility.
