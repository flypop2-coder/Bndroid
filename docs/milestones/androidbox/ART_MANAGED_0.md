# ART Managed-0: GC, Java threads and JNI from an APK

Date: November 6, 2026. ABI 70 remains an isolated integration profile. This milestone executes a signed test APK's command-line entry through upstream ART; it does not yet run an Activity or Framework services.

## Observed behavior

`BNDROID_ART_MODE=managed scripts/check-art-runtime.sh` runs two complete generations of `org.bndroid.runtimeprobe`. ART loads its DEX from `base.apk`; Android's linker loads the uncompressed, aligned `lib/arm64-v8a/libbndroidjni.so` directly from that APK. The positive path contains no loose application DEX or extracted application library. JNI `dladdr` verifies the loaded library's APK path, and the Java classpath is checked by the program itself.

Each generation validates three Java workers with synchronized shared updates and latches, join and interruption, 64 MiB of small-array payload turnover within a 16 MiB Java heap, repeated concurrent-copying GC, preserved reference graphs and contents, weak-reference clearing and queue delivery. The graph workload uses 1 KiB arrays so it exercises region evacuation instead of testing only non-moving large-object allocations. JNI validates `JNI_OnLoad`, Java array mutation, a native exception, a global reference, and a real pthread attaching to the VM, invoking Java and detaching. A deliberate attempt to load the existing non-public copy of the library under `/system/lib64` must be refused by the unmodified classloader namespace before loading the application copy succeeds.

The final packaged run is `target/art-runtime/check.5wW9qO`: both processes exit normally, all 20 threads are reaped, and sparse pages/tables, files and waiters return to zero. It records 61,198 Linux calls, 84,563 pages physically discarded, 36,407 demand-zero pages, 51,532 peak sparse pages and 223,483 reclaimed pages. The temporary disk base is unchanged. The summary reports `managed_runtime_verified=true` and `apk_dex_jni_verified=true`, while retaining `full_android_apps=false`.

This advances [ART Signals-0](ART_SIGNALS_0.md). The broader workload initially exposed an incorrect Java wall clock and a GC crash. After implementing the missing clock interface and memory discard semantics, those same workload checks pass; no ART/core-library binary or GC implementation was replaced.

## Memory and time behavior

Private anonymous `MADV_DONTNEED` validates the entire mapped range before mutation, removes its translations, invalidates the ASID and releases physical frames. VMA ownership/rights and empty private page tables remain. A subsequent permitted access allocates a new zero page and retries the original instruction. Read-only pages stay read-only; `PROT_NONE`, unmapped ranges, file EOF and privileged kernel addresses remain inaccessible. An invalid range containing a hole leaves the earlier pages intact.

The same demand path serves kernel copies only after the existing usercopy machinery validates the exact faulting instruction, access direction, active range, translation context and syndrome. Other EL1 faults retain their previous fixup/fail-stop behavior. The existing sparse-page/table limits, W^X rules, stack guards and process-generation cleanup remain enforced. Shared/file-backed discard is explicitly unsupported; initial mmap/mprotect population remains eager. This is demand-zero recovery after anonymous discard, not a complete pager, file cache, swapping system or general overcommit policy.

`gettimeofday` now uses the same RTC/counter anchor as realtime `clock_gettime`, with a real microsecond timeval and the profile's UTC legacy kernel-timezone fields. `System.currentTimeMillis()` advances correctly. Process/thread CPU clocks use actual scheduled user/kernel counter charges, including the current exception's elapsed charge. Reaped workers contribute to a separate process total rather than inflating the leader's own thread clock. CPU time excludes blocked sleep. Foreign/dynamic CPU clock IDs and clock-setting interfaces remain unsupported.

The native memory gate checks an immediate RSS reduction after discarding 4 MiB, zero refill, kernel read/write copies into absent pages, protections, failed-range preservation, clock progression and accounting across worker reaping. Its final receipt is `check.PRRWnV`, with 4,132 discarded and 2,054 demand-zero pages. The signal fixture additionally attempts a real write to a discarded read-only page and requires SIGSEGV/ACCERR before successfully reading a newly zeroed page.

## Package construction and evidence

The owned fixture uses the installed SDK's aapt2, D8, zipalign and apksigner, the existing public test-only signing key, and NDK r27b. It has a compiled Android manifest, package/version/SDK metadata, standard DEX and an arm64 JNI library. Its APK v2 signature is verified by the SDK tool; the test certificate SHA-256 is `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`. Uncompressed library entries are aligned to 16 KiB; this guest still uses 4 KiB pages, so no 16 KiB-device acceptance is claimed. See the Android [alignment contract](https://developer.android.com/tools/zipalign).

The root builder verifies APK payloads match the owned DEX/JNI build, rejects duplicate/corrupt entries, removes the loose DEX, and records APK/DEX/root hashes. Signature, badging, build and runtime logs stay in the gate's ignored artifact directory. SDK/APEX inputs and their original provenance are unchanged. The native library search path points to the application's APK directory; no global linker namespace is relaxed.

The host evidence parser requires two exact application results, every stage, both APK-input results, real paging activity, sufficient created/reaped threads and complete process cleanup. A basic Java or native result cannot pass the managed gate. This test injects the already built APK into the immutable runtime root and calls its `ManagedMain` entry; it does not exercise package installation, on-device signature enforcement, ActivityThread, an Activity lifecycle or app graphics.

## Reproduction and remaining integration

Follow the [ART fixture guide](../../../fixtures/art-runtime/README.md). Managed mode additionally requires the installed Android 36 platform jar and SDK packaging tools. Other ART modes keep their existing prerequisites.

```sh
BNDROID_ART_MODE=managed scripts/check-art-runtime.sh
BNDROID_ART_MODE=memory scripts/check-art-runtime.sh
scripts/check-art-signals.sh
scripts/check-art-runtime.sh
```

All gates use temporary disks and owned finite QEMU processes. Host tests, proof-parser tests, Clippy, signal containment, basic Java, native Bionic and existing UI/boot regression results are recorded in the implementation status.

The next application integration still needs the upstream Framework/ActivityThread dependency set, Binder/service bridges, package installation and per-app data. Writable runtime storage, timezone/properties, directory iteration, scheduling controls, file/shared paging, JIT/AOT, broader API/GC stress and product security/performance acceptance remain open. The current run retains missing proc-cmdline/tracing/timezone warnings and unsupported optional directory/affinity/niceness calls in its summary. A1 and the full JNI/native-APK acceptance gate remain unchecked.
