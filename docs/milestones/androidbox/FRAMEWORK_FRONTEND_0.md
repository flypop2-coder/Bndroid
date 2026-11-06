# Framework Frontend-0: upstream app_process and boot properties

Date: November 6, 2026. Native ABI remains 70. A1 and general Android application acceptance remain incomplete.

The later [ART Memfd-0](ART_MEMFD_0.md) update supersedes the code-cache blocker
below and records the current dynamic-JNI/Binder failures. The observations in
this document retain the original frontend milestone's evidence.

## Observed result

The separate Framework profile loads the unmodified local Android 16 `app_process64`, its platform dependencies and `libart.so` through the source linker namespaces. AndroidRuntime reaches ART argument parsing and its own help/exit hooks in two process generations. Both exit zero and release all private pages, page tables, file descriptions and thread resources. The frontend gate authenticates the Android log records against the actual exiting process IDs; ordinary `dalvikvm` stdout cannot satisfy it.

Final evidence `target/art-runtime/check.I7Nqtp` reports `framework-help-passed`, 19,487 Linux calls, a 26,816-page sparse peak and 62,118 reclaimed pages, with zero remaining sparse pages/tables/files/waiters. Its temporary disk is unchanged. This verifies the native startup/frontend path, not a completed Java VM, Framework native registration, ActivityThread or an Activity launch.

The distinct managed `framework` attempt supplies an owned D8 program that must perform native Parcel/Bundle serialization and a delayed Handler/Looper callback. It does not currently pass. Evidence `target/art-runtime/check.cD5XlQ` gets beyond boot-DEX loading and reaches code-cache initialization, where a 32 MiB `mprotect(PROT_READ | PROT_WRITE | PROT_EXEC)` request is rejected with `EACCES`. The abort reports `Checked call failed for Cache +W: Permission denied`. At that request, diagnostics show 101,372 resident sparse pages and 293 private page-table pages. The failed child is reaped with zero residual pages/tables/files/threads; the temporary disk is unchanged. No Framework application result occurs, and `full_android_apps` remains false.

AndroidRuntime adds profiling options even when the fixture requests interpreter execution and disables JIT compilation. Compatible code-cache backing/publication is still required; this update does not relax W^X or replace the upstream runtime. Binder, ServiceManager, Framework services, ActivityThread, application graphics and broader app acceptance remain further work.

## Input preparation and resource bounds

The local source remains the frozen image in [ADR 0001](../../decisions/0001-android-runtime-baseline.md). The Framework extension records 347 files / 265,049,386 bytes, 280 ELF dependency entries and all 46 source boot-classpath jars. This includes `framework-res.apk` and timezone data. All 66 earlier ART input records, including their resolved-source hashes, are unchanged. The source global/ART linker namespace configurations remain intact. Google applications, Play services and vendor APKs are not acquired or imported into Git.

The extractor now supports verified progress checkpoints. Resume requires the same source package properties/fingerprint, unchanged core files and metadata, and matching per-file checkpoint hashes. Names such as `android.hardware.camera.device@3.2.so` are valid; path escapes and out-of-scope roots are rejected. The 512 MiB acquisition cap remains enforced. A completed receipt is published atomically; incomplete inputs cannot satisfy the Framework root builder. Each acquisition owns a fresh offline temporary AVD and terminates only that process.

The verified root contains 357 archive entries / 264,181,760 bytes; the kernel adds seven interpreter/device/proc entries. Framework mode explicitly budgets 1 GiB RAM, a 320 MiB immutable archive / 512 catalog slots, 512 descriptors, 4,096 mapping regions, 512 MiB sparse backing and 512 page-table pages. Kernel heap remains 8 MiB. These larger bounds follow observed failures at 64 descriptors, 1,024/2,048 regions and 256 MiB backing; native, mobile and ART-only profiles retain their prior limits. Both source boot-classpath variables fit the checked 4 KiB startup page.

The kernel image now carries the standard 64-byte AArch64 Image header and advertises its complete in-memory size, including BSS. This prevents QEMU from placing the device tree in the larger zero-initialized runtime root. Descriptor tables use fallible heap storage instead of embedding a large array in process-construction stack frames. `RLIMIT_NOFILE` reports the actual selected capacity.

## Fixed relocation and immutable properties

Private-anonymous `mremap(MREMAP_FIXED | MREMAP_MAYMOVE)` preflights source/destination ranges, flags, overlap, layout and resource allocation. It prepares destination contents and rights before removing either mapping, replaces an existing destination, removes the old source and preserves neighboring mappings. Shrink, equal-size and growth are supported; growth bytes are zero. File/shared source remapping and `MREMAP_DONTUNMAP` remain unsupported. Invalid or exhausted requests leave both mappings intact.

This operation is needed by the original linker's CFI shadow updates, which prepare data separately and publish read-only pages at the shadow's fixed address. The owned memory gate verifies equal-size/shrink/growth relocation, source removal, destination replacement, preserved read-only access, zero growth, overlap/alignment/kernel-address rejection and allocation-failure atomicity. A direct EL0 store to the relocated read-only page causes the expected `SIGSEGV/SEGV_ACCERR`, returns through the signal frame and leaves its contents intact. Both process generations pass with full cleanup and both signal frames returned (`check.P0oA2o`).

The bootstrap root also contains six declared public, immutable properties: arm64 ABI lists, disabled native bridge, and Bndroid product/manufacturer names. An owned encoder emits the binary format that the unmodified Bionic pre-split reader supports. This is not a writable property service or a private property namespace. The `properties` gate (`check.Dc708z`) validates real Bionic lookup, read callbacks, enumeration, two actual timed property waits, and rejected writable opens/shared mappings/protection changes. Both process generations exit zero and release all resources.

## Reproduction and validation

See the [fixture guide](../../../fixtures/art-runtime/README.md) for local acquisition, resumability and separate `framework-help`, `framework`, `properties`, `memory` and managed-APK commands. Framework timeouts are bounded at 300 guest seconds / 360 host seconds for two cold library loads and cleanup; earlier ART bounds remain 90/120 seconds. Every gate retains its own input receipts, logs, temporary disk and summary under ignored `target/`.

Passed: 1,127 default workspace host tests, 393 Framework kernel-library tests, and 18 extraction/property/UART/evidence-parser tests. Host and AArch64 Framework Clippy, including the syscall-tracing build, pass with warnings denied. Native boot and mobile UI (`target/mobile-ui/check-overview.e1HRsr`) regressions pass. Dynamic Bionic (`check.qIdbxQ`), all four pthread lifecycle modes (`check.DMvQAh`, `check.2HvUyj`, `check.oyMyIS`, `check.b5irLk`) and EOF containment (`check.0icY9B`) pass under `target/bionic-dynamic/`. Runtime binaries and generated evidence remain local.

The existing APK/GC/JNI gate also passes (`target/art-runtime/check.x0g68T`): two zero-status processes, all 20 threads reaped, 61,006 Linux calls, 84,155 discarded pages, 36,000 demand-zero pages, and zero remaining sparse pages/tables/files/waiters. This retains the earlier packaged managed-runtime behavior while the larger Framework path remains incomplete.

## ABI references

- [AArch64 Image boot protocol](https://docs.kernel.org/arch/arm64/booting.html).
- [Android app_process entry](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/cmds/app_process/app_main.cpp) and [AndroidRuntime startup/options](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/core/jni/AndroidRuntime.cpp).
- [Bionic CFI shadow updates](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/linker/linker_cfi.cpp).
- [Bionic property-area mapping and lookup](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/system_properties/prop_area.cpp), [area layout](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/system_properties/include/system_properties/prop_area.h), [property layout](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/system_properties/include/system_properties/prop_info.h), and [pre-split reader](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/system_properties/include/system_properties/contexts_pre_split.h).
