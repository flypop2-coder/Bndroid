# ART Frontend-0: real runtime loading and low-address memory

Date: November 6, 2026. Native ABI remains 70; general Android application acceptance remains incomplete.

## Observed result

Bndroid executes the unmodified local Android 16 `dalvikvm64`, resolves its runtime dependencies, loads `libart.so`, runs its constructors and command-line parser, and prints ART's own help. Two successive process generations exit zero and reclaim all sparse pages, page tables, descriptors and thread resources. This is native ART frontend execution, not a created Java VM or an APK launch.

The separate owned memory gate verifies a 64 MiB allocation below 4 GiB, initial zeroes and writes across every page, RO/RW transitions and rejected kernel copies into RO memory, hole detection, fixed replacement zeroing, shared anonymous memory across a real pthread, kernel-address rejection, and full reaping of intentionally retained low-address pages. One recorded run peaked at 17,390 sparse pages, reclaimed 35,146 across two generations, and ended with zero live pages/tables/files. These counts can vary with entropy.

The real managed startup attempt progresses into ART VM/heap initialization, then aborts with `couldn't open /proc/self/stat: No such file or directory`. Its process is reaped with zero residual pages/tables/files. The Java result marker has not occurred. The supervisor reports this as failure and leaves `managed_java_verified=false` and `full_android_apps=false`.

## Frozen local inputs

[ADR 0001](../../decisions/0001-android-runtime-baseline.md) selects the installed Android 16 API 36.1 / extension 20 revision 3 SDK image and NDK r27b/API 35. Its fingerprint is `google/sdk_gphone64_arm64/emu64a:16/BP41.250916.009.A1/14246511:user/release-keys`; the active ART APEX reports version code `361153120`. The source image's preinstalled ART module path is recorded as `/system/apex/com.google.android.art.apex`. This identifies the installed source; it is not a redistribution/license assertion.

The bounded extractor collects 60 files / 71,621,725 bytes: the real executable and runtime libraries, recursively required ELF dependencies, core-Java jars and ICU data. It retains SHA-256/size/path receipts, dependency edges, APEX metadata, namespace configuration, environment and the source notice. No runtime binaries, SDK changes, AOSP checkout, Google applications, or service APKs enter Git. Dynamic `libartpalette-system.so` is included explicitly. Dependency inventory prefers each library's own directory, Bionic, and platform dependencies before cross-APEX fallbacks; distinct library paths stay distinct in the receipt.

Runtime loading currently uses an explicit diagnostic `LD_LIBRARY_PATH`. Full Android linker-namespace policy is not implemented and is not implied by collecting the source configuration. Boot images are not available in this selected readable input set; managed diagnostics use matching core DEX jars without invoking a compiler.

## Kernel adaptation

Only `art-runtime` widens the immutable root to 96 MiB / 128 entries, VM metadata to 512 regions, sparse backing to 256 MiB and kernel heap to 8 MiB. The measured 68.4 MiB archive fits a bounded 128 MiB non-executable kernel BSS window in the 512 MiB guest. Heap self-validation derives the exact L3 table count from its address span, retaining the old checks for smaller profiles.

The new private low-address arena covers 2–4 GiB (L1 slots 2 and 3). Slots 0/1 keep privileged MMIO/RAM; slots 4–8 keep kernel VM and the initial user image. Unsafe fixed ranges and cross-arena ranges fail. Low non-fixed hints can fall back into the low arena, while zero/high hints still select the existing 32–512 GiB arena. Actual owned pages govern user copies, protection changes, executable return validation and teardown. Shared kernel tables are never chosen by user addresses.

Anonymous shared mappings have zero-filled ownership in the current shared thread-group address space, retain their policy through splits/protection changes, and reject wipe-on-fork advice. Process-style clone/fork and shared file mappings remain unsupported. `msync` validates flags/alignment and complete mapping coverage, including `PROT_NONE` reservations; holes and kernel addresses return `ENOMEM`. Supported private file mappings cannot dirty their immutable source, and shared anonymous pages have no persistent writeback. Unsupported file sharing is not converted into success.

The Linux personality now reports `Linux` / `0.1.0-bndroid` through `uname`, retaining its actual custom-kernel version instead of advertising a newer Linux feature level. ART uses that version to probe optional facilities; `mremap`/`userfaultfd` still return `ENOSYS`. Opt-in syscall tracing can read Bionic's bounded, magic-validated abort-message mapping for its exact process generation, exposing fatal diagnostics when logd is absent without changing guest behavior.

`BNDROID_LINUX_LAUNCH_CONFIG` supplies checked `arg=` and `env=` lines at build time. The first two arguments select the embedded linker and an absolute guest executable path. Entry counts, embedded NULs, and initial stack space are bounded before compilation. The previous dynamic fixture remains the default when no configuration is supplied.

## Reproduction and validation

Follow the [fixture guide](../../../fixtures/art-runtime/README.md). `BNDROID_ART_MODE=help` and `BNDROID_ART_MODE=memory` are positive gates; default Java mode is a diagnostic failure until the VM and owned DEX program really run. Every mode retains independent logs/receipts under `target/art-runtime/check.*/`, uses fresh entropy and an unchanged temporary-disk base, and shuts down its own process. Generic native supervisor output retains `art=0`; the external summary separately records frontend and managed execution proof.

Host VM tests cover both arenas, fixed boundary failures, protection preservation, high-address allocation after low mappings, shared-policy splits, private replacement, invalid sync flags and holes. Default host, prior raw Linux and all four pthread lifecycle gates are regression requirements for this change. Native/mobile and remaining regression results are recorded in the current implementation status after their actual runs.

## Remaining runtime work

Implement real process-information files and accurate stack/accounting metadata, signal actions/alternate stacks/handler delivery and return, discard/demand-paged memory, writable runtime storage and property/namespace semantics. Then advance the unchanged Java gate through class initialization, exceptions, GC and JNI before ActivityThread, Binder, Framework or APK compatibility is claimed. The supplied Java program is ordinary D8 output and does not run through the bounded AndroidBox interpreter.
