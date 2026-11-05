# ART Process-0: process metadata, runtime files and native initialization

Date: November 6, 2026. ABI 70 remains an isolated integration profile. A1 and general APK acceptance remain incomplete.

## Observed behavior

The unmodified Android 16 ART runtime now reads its core Java jars, initializes the main-thread stack and heap, and loads `libicu_jni.so`, `libjavacore.so` and `libopenjdk.so` through Android's actual exported linker namespaces. The previous `/proc/self/stat` failure is resolved. This advances [Frontend-0](ART_FRONTEND_0.md); it does not yet complete `JNI_CreateJavaVM` or the supplied Java program.

The current Java attempt fails with `No pending exception expected: java.io.IOException: Function not implemented`. The trace records `rt_sigaction(34, ...) = -ENOSYS` during core native initialization. This is consistent with [upstream NativeThread initialization](https://android.googlesource.com/platform/libcore/+/0dac6dd2e30993ba45f77a20982928dae33200c8/ojluni/src/main/native/NativeThread.c), which installs that signal handler and raises an IOException on failure. No handler registration is reported successful without delivery/return semantics. The failed child is completely reaped; the supervisor reports failure and keeps `managed_java_verified=false` and `full_android_apps=false`.

## Kernel behavior

- `/proc/self/stat` and `/proc/self/maps` are bounded, process-owned snapshots. Their PID/parent, thread count, initial stack pointer, memory usage, mappings, access rights and file offsets come from actual scheduler/VM state. User/kernel CPU time is charged at exception entry/return, with nested usercopy faults and thread reaping accounted for. Snapshot descriptors retain their own bytes, share offsets through dup, and release storage on final close/reaping.
- The proc view implements these two files, not a general procfs. Unimplemented Linux-specific stat fields are zero; command-line, per-thread directories, status, memory, scheduling statistics and foreign-process access remain absent. Rewinding an open descriptor retains its original snapshot; reopening obtains current state.
- Initial writable stack pages can become real inaccessible guards and later return to their original rights. User copies reject guard pages. Initial code/rodata cannot gain write access, and user mappings cannot select kernel tables.
- Anonymous private `mremap` supports shrinking, in-place growth and `MREMAP_MAYMOVE` for a uniformly protected source range. Moving prepares all metadata, frames and copies before publishing changes. Contents/rights survive; new tail bytes are zero; failure leaves the old mapping intact. Fixed destinations, alias-preserving moves, shared/file remaps and process-style fork remain unsupported. Moving currently copies physical pages and can temporarily require both allocations.
- Read-only shared file mappings are supported for immutable catalog bytes. Every view sees the same frozen data; write mappings and later writable protections fail. This is not a writable shared page cache. File identity and offsets survive mapping splits. Large reads validate the complete destination and copy in bounded chunks; they no longer truncate ZIP directory reads at the native IPC copy limit.
- `MADV_NORMAL`, `MADV_RANDOM` and `MADV_SEQUENTIAL` validate the complete mapping without discarding contents. The profile eagerly populates pages and has no speculative read-ahead/eviction policy to change. Discard and demand paging remain separate work.
- A bounded write-only Unix datagram endpoint at `/dev/socket/logdw` accepts Android log packets and attributes text to the actual calling process. It supports descriptor sharing, close-on-exec and atomic bounded writev gathering, and sanitizes line breaks. Binary packets currently produce metadata only. General Unix IPC, log readers, persistent logging and the Android logd service are not implemented.

All changes preserve the existing per-process resource limits, checked user copies, W^X enforcement and generation-qualified cleanup. CPU accounting and the additional ART mechanisms are feature gated; native ABI 23 remains unchanged.

## Runtime inputs and namespaces

The source remains the [frozen local SDK image](../../decisions/0001-android-runtime-baseline.md). Extraction now inventories both regular files and symlink names, resolves their real source paths and records hashes. This captures APEX-local ICU dependencies that the previous inventory incorrectly resolved through global search paths. The updated set contains 66 named inputs / 74,543,749 bytes; all previously captured binary hashes remain identical. Some names refer to identical source data, so this count includes aliases.

The builder validates and bundles the source's global and ART-specific linker configuration, removes the diagnostic `LD_LIBRARY_PATH`, and supplies canonical Bionic system aliases. The Bndroid public native-library list advertises only its supplied libc/libdl/libm/liblog/libz subset. These files enable the upstream linker itself to enforce namespace search/link rules. They do not implement Android's package/service/property ecosystem or install every APEX named by the source policy. APEX identity metadata remains provenance only.

Old receipts without configuration hashes must be refreshed into a new extraction directory. Modified binaries or configurations are rejected before packaging. Source binaries and generated artifacts remain local and ignored; no SDK files are modified or redistributed through Git.

## Validation

The extended `memory` gate checks real `pthread_getattr_np`, proc field positions and CPU progress, stack guards, mapping rights, immutable snapshots and dup offsets, a 65,557-byte ZIP read, shared read-only mappings/advice, mremap movement/shrink/failure preservation, and authenticated log transport. It also retains the previous complete 64 MiB low-address/pthread memory checks. Two independent process generations must complete and leave zero sparse pages/tables/files/threads.

Recorded evidence: `target/art-runtime/check.EXkPn7` (memory), `check.tV60g7` (ART help), and `check.N55rbZ` (managed failure and cleanup). The evidence parser requires complete, unprefixed result lines; log-message text cannot impersonate a Java result. Native log records must identify both of the actual exited process generations. Android error logs are retained separately in new summary files.

Validation passed: 1,121 default workspace host tests, 387 ART kernel-library tests, six ART proof-parser tests and four existing UART tests, host/AArch64 ART Clippy, static Bionic, raw Linux ABI, all four dynamic pthread modes, EOF containment, default kernel boot and the mobile UI gate. The APK probe example now declares its required feature so ordinary workspace tests build successfully; it also compiles with that minimum feature enabled.

See the [fixture guide](../../../fixtures/art-runtime/README.md) for reproduction. Next requirements include actual signal actions, alternate stacks, delivery and safe return, followed by the remaining memory, runtime storage, property/time data and managed execution gates. ActivityThread, Binder, Framework and general APK support remain open.
