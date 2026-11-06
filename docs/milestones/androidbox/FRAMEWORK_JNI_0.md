# Framework JNI-0: dynamic bootstrap dependencies

Date: November 6, 2026. ABI 70 and Android acceptance levels are unchanged.

## Runtime progress

The unmodified Framework now loads
`/apex/com.android.tethering/lib64/libframework-connectivity-tiramisu-jni.so`
through the source linker's Tethering namespace. TrafficStats requests this
library through `System.loadLibrary`, so the static dependency closure of
`app_process64` alone did not acquire it. Extraction now treats it as a runtime
entry point and inventories its recursive ELF dependencies.

The new library is 35,696 bytes, SHA-256
`986329e0ef95376a5f13bed05751a07ac1a9e5f18ecda34013d6a7eacf7b2ab9`.
Its six direct dependencies—libandroid, liblog, libnativehelper, libc, libm and
libdl—are already in the platform/ART/runtime set. The complete local receipt has
349 files, 265,085,778 bytes, 281 native dependency records and 46 boot jars. The
66-file ART subset is 74,543,749 bytes and matches the Framework receipt exactly.
Input hashes, source identity, namespace policy and the source notice hash were
verified after re-extraction from the same installed SDK image.

Resume now checks the complete runtime JNI graph before declaring a receipt
complete. It walks existing dependency edges to recover absent transitive inputs,
handles cycles, preserves earlier library-candidate records and keeps the original
ART subset unchanged. The root builder rejects incomplete metadata/JNI receipts.
The owned extractor still uses an isolated, read-only, offline source emulator;
inputs, notices and generated APKs remain local and outside Git.

## Actual Framework result

The traced full gate in `.bndroid/art-runtime/check.1kHoFn` reports successful JNI
loading, then reaches `RuntimeInit.nativeFinishInit`. Its native startup calls
libbinder and aborts because `/dev/binder` is absent. This is now the direct
bootstrap failure; it is no longer an exception caused by missing TrafficStats
JNI. No Parcel, Looper or application-main success is recorded.

The result is `guest-failure`, `framework_parcel_looper_verified=false`, and
`full_android_apps=false`. All six workers are reaped, pending signals and
waiters are zero, and pages, page tables and file descriptions return to zero.
The temporary disk remains unchanged. Binder has not been implemented or replaced
with successful no-op ioctls. ActivityThread, service-manager transactions and
general APK acceptance remain open in the roadmap.

## Evidence recovery and storage

During this work, `cargo clean --target aarch64-apple-darwin` unexpectedly removed
the entire local `target/` tree, including acquired inputs, prior run evidence
and the interactive demo's files. This was an operator error. Source files, Git
history and the installed SDK were unaffected. Historical milestone paths under
that old tree are not available locally; their recorded results must not be
represented as retained raw logs.

The runtime was re-extracted from the same SDK image and the current gates were
rerun. New ART/Bionic evidence and runtime inputs now live in ignored `.bndroid/`,
outside Cargo's build tree. Compiler caches remain under `target/`. Existing input
paths remain usable through `BNDROID_ART_RUNTIME_DIR` and
`BNDROID_ANDROID_RUNTIME_DIR`; scripts never move or delete old caches. Native/UI
build and screenshot paths remain under `target/`, so broad cleanup is inappropriate
while a demo or retained evidence uses them.

Fresh evidence, all with unchanged temporary disks:

| Gate | Evidence | Result |
| --- | --- | --- |
| Shared memfd/code execution | `.bndroid/art-runtime/check.gIEI6U` | Two exits; six expected faults returned; peak 17,335 pages; complete cleanup |
| Boot properties/clock | `.bndroid/art-runtime/check.wTdOdz` | All 13 properties; actual timed wait; complete cleanup |
| Memory/relocation | `.bndroid/art-runtime/check.fTypCz` | Real read-only faults, demand-zero and fixed relocation; complete cleanup |
| APK DEX/GC/JNI | `.bndroid/art-runtime/check.zEVlzf` | Two exits; all 20 threads reaped; peak 51,537 pages; complete cleanup |
| Framework frontend | `.bndroid/art-runtime/check.e0Bfa8` | Both authenticated AndroidRuntime/ART help exits; complete cleanup |
| Dynamic Bionic | `.bndroid/bionic-dynamic/check.N7mZ0q` | Two native generations; complete cleanup |
| pthread normal | `.bndroid/bionic-dynamic/check.aMZKLJ` | All 24 workers reaped |
| pthread leader exit | `.bndroid/bionic-dynamic/check.PwYH6Q` | Both generations reclaimed |
| pthread group exit | `.bndroid/bionic-dynamic/check.G12d1g` | Both groups reclaimed |
| pthread worker fault | `.bndroid/bionic-dynamic/check.CGw6gX` | Expected kernel-read fault; complete cleanup |
| File EOF fault | `.bndroid/bionic-dynamic/check.EkcOnO` | Expected beyond-EOF fault; complete cleanup |

The recovered interactive tour is `target/mobile-ui/demo.e5WbNw`: 15 screenshots,
Calculator addition and divide-by-zero recovery, shared settings/accessibility,
the bounded AndroidBox DEX callback and return to Home. It uses a temporary overlay.
This remains the bounded demo described in the [demo guide](../../DEMO.md).

Host validation passes 1,128 workspace tests, 395 Framework library tests and
22 Python input/property/evidence tests. Host-library Clippy and AArch64 Framework
Clippy with tracing both enabled and disabled pass with warnings denied. Python
compilation, shell syntax and diff whitespace checks pass. The native boot smoke
test and mobile UI gate (`target/mobile-ui/check-overview.bwhqVc`) pass. These
results do not claim all-targets test-code Clippy or complete
Android compatibility.

Use the [ART guide](../../../fixtures/art-runtime/README.md) and
[Bionic guide](../../../fixtures/bionic-dynamic/README.md) for current reproduction
commands and cache locations.
