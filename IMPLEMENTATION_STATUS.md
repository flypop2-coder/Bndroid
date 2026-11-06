# Implementation status

This page records implemented behavior and validation, with a September 9, 2026 platform baseline and dated updates below. Historical test counts belong to the indicated development runs; they are not a claim that every historical gate was rerun during publication. The [complete original status log](docs/archive/originals/IMPLEMENTATION_STATUS.txt) preserves detailed evidence and earlier milestones. Future tasks belong only in [TODO.md](TODO.md).

## Binder file transfer and service event-loop update — November 6, 2026

Binder now captures actual open descriptions, installs whole FD batches in the
receiver with CLOEXEC, shares offsets/status flags and rolls back failed receives.
Node/caller consent gates request/reply FD transfer. Full descriptor tables fail
without partial installation; a failed request resumes the receiving thread's
blocking loop. Memory-file resize invalidates all affected process mappings before
freeing frames, and write seals inspect mappings in every process.

Binder epoll readiness is computed for the original waiting process/thread from
non-consuming snapshots. External notifications propagate through nested epoll,
and last-close cleanup removes watches without retaining Binder devices. The
snapshot boundary avoids Binder/event-lock inversion during transferred-file
cleanup. SG/FD-array transport, Binder-device transfer, ServiceManager, ActivityThread
and per-app policy remain open. Full Android application compatibility is still
unproven; A1–A6 remain incomplete.

The final native gate (`check.ixHTrO`) verifies real file sharing, cross-process EOF
faults and regrowth, seal enforcement, both receiver-table exhaustion directions,
invalid copies, creator exit and nested Binder epoll. Eleven FD submissions include
five delivered descriptors and two atomic installation failures, with zero
remaining resources. Host validation passes 1,165 workspace library tests, 434
Framework library tests and 34 Python checks. Memfd, managed APK/GC/JNI and
eventfd/epoll regressions pass with unchanged temporary disks. The final Framework
repeat (`check.mY1RXD`) completes both normal startup/shutdown generations and
reclaims all 16 workers; an earlier second-generation deadline remains recorded
as a failure. Host/AArch64 Clippy passes with warnings denied. Evidence and
regression diagnostics are recorded in
[Binder Files-2](docs/milestones/androidbox/BINDER_FILES_2.md).

## Binder objects and nested callbacks update — November 6, 2026

Binder now translates strong/weak objects into connection-scoped handles, restores
local objects on round trips and manages explicit reference handshakes separately
from receive-buffer holds. Retain acknowledgments precede sender completion;
failed copies preserve reference/death notifications. One-way work is ordered per
object. Synchronous callbacks return to the original waiting thread, with bounded
call stacks, authenticated replies and descendant failure cleanup.

The combined native gate exercises two nested callbacks, seven translated
objects, ten reference events, independent one-way queues, malformed transfers,
three deaths and complete process/resource cleanup (`check.x1QDbZ`). Full Framework
passes both normal startup/shutdown generations (`check.b7VdzY`), and managed
APK/GC/JNI plus eventfd/epoll regressions pass with unchanged disks. Host validation covers 1,158
workspace tests, 427 Framework library tests and 34 Python checks; host/AArch64
Clippy passes with warnings denied. Full evidence,
failed diagnostics, regression results and current limits are recorded in
[Binder Objects-1](docs/milestones/androidbox/BINDER_OBJECTS_1.md).

FD/SG ownership, real ServiceManager and ActivityThread startup, per-app policy
and Android graphics/service bridges remain open. This does not complete A1–A6
or establish general Android application compatibility. Acquired binaries and
runtime evidence remain local/ignored; the demo uses temporary disk storage.

## Framework Looper and normal shutdown update — November 6, 2026

The full Framework command-line gate now passes with unmodified Android 16 app_process/AndroidRuntime/ART. Two cold launches complete Parcel/Bundle round trips, cross-thread Looper wakeups and delayed callbacks, return normally from Java, destroy the VM and exit zero. All 16 workers, event objects, Binder connections, pages, page tables and file descriptions are reclaimed. Evidence `.bndroid/art-runtime/check.qpDz2A` records 163,452 Linux calls, a 102,855-page peak, 335,965 reclaimed pages and an unchanged temporary disk. `framework_parcel_looper_verified=true`; `full_android_apps=false` and A1–A6 remain incomplete.

The kernel now implements actual eventfd counters and epoll readiness, blocking, deadlines, nested/edge/one-shot watches, descriptor lifetime, vectored I/O and signal-mask/restart behavior. Independent Binder/hwbinder/vndbinder domains and descriptor-close flush returns let the original Binder pool detach during VM shutdown. The Framework proc-map text limit now follows its larger VMA profile: 512 KiB, with the existing heap and physical limits retained. Diagnostic evidence showed the old 128 KiB bound failing despite 4.5 MB free heap. See [Framework Looper-0](docs/milestones/androidbox/FRAMEWORK_LOOPER_0.md) for implementation limits, failure history and validation.

Host validation passes 1,148 workspace tests, 417 Framework library tests and 34 Python checks; host-library and AArch64 Framework lint gates pass with warnings denied. Native event/Binder tests, boot/mobile UI, dynamic Bionic/thread/EOF and ART regressions are recorded in the milestone. Runtime inputs remain unchanged and local. ActivityThread, Binder objects/FDs, ServiceManager, service bridges and application graphics remain open.

## Initial Binder transport update — November 6, 2026

Two concurrent Bionic processes now communicate through the custom kernel's LP64 Binder adapter: context-manager byte transactions/replies, ordered one-way messages, authenticated process identity and death notifications. Read-only/non-executable receive pools, retained open-file ownership across descriptor reuse, interrupted reads, failed user copies, peer exit and complete cleanup have real QEMU evidence. The final native gate (`.bndroid/art-runtime/check.OfR9ow`) reports eight submitted transactions, three replies, three one-way messages, five expected failures, one death notification and zero live Binder resources. Its temporary disk is unchanged.

The original Framework starts its Binder pool and reaches the owned Java entry's actual Parcel/Bundle round trip. Looper creation still fails because eventfd is missing; no Looper, ActivityThread, ServiceManager or full-app acceptance is claimed. Binder object/FD transfer and capability policy remain open, so A1–A6 are still incomplete. A boot-stack overflow exposed by the larger root catalog was fixed by parsing directly into static storage, without increasing stack or memory limits. See [Binder Transport-0](docs/milestones/androidbox/BINDER_TRANSPORT_0.md) for scope, commands, evidence and the current failure.

Validation passes 1,139 workspace host tests, 407 Framework library tests, 33 Python checks, host-library and AArch64 lint gates, native boot/mobile UI, dynamic Bionic/thread/EOF regressions, shared code-cache, properties, memory, managed APK/GC/JNI, all signal gates and Framework frontend. Runtime inputs remain unchanged and local; the temporary-disk interactive demo remains available.

## Framework dynamic JNI update — November 6, 2026

TrafficStats's runtime JNI dependency is now included with its complete ELF dependency graph. The original Framework loads it through the unchanged Tethering namespace and reaches `RuntimeInit.nativeFinishInit`; startup then fails directly at the absent `/dev/binder`. The failed process and all six workers are completely reclaimed. Parcel/Looper, ActivityThread and A1–A6 remain incomplete. The local input set contains 349 files / 265,085,778 bytes. See [Framework JNI-0](docs/milestones/androidbox/FRAMEWORK_JNI_0.md) for current evidence and limits.

An erroneous Cargo cleanup removed the old local `target/` inputs/evidence during this work. Source, Git and SDK files were unaffected; inputs were re-extracted and current regressions rerun. New ART/Bionic inputs and evidence now live under ignored `.bndroid/`, independent of Cargo cleanup. Historical `target/` paths in earlier updates are no longer available locally. The interactive temporary-disk demo has been restored. Fresh validation passes shared code-cache, boot-property, memory, APK/GC/JNI, Framework frontend, dynamic Bionic/thread/EOF, native boot/mobile UI, 1,128 workspace host tests, 395 Framework library tests, 22 Python checks and host/AArch64 library lint gates.

## Shared ART code-cache backing update — November 6, 2026

Real memfd objects now supply shared physical mappings, truncation/EOF faults, seals, descriptor I/O and lifetime across close/reuse. ART's 64 MiB code-cache layout uses separate RW/NX and R/X views; single RWX mappings remain forbidden. The original physical/heap bounds remain, with compact page ownership and corrected reservation for fixed replacements. [ART Memfd-0](docs/milestones/androidbox/ART_MEMFD_0.md) records the behavior and [ADR 0002](docs/decisions/0002-linux-abi-release-and-code-cache.md) explains the partial Linux ABI release value and protection model.

The native memfd gate (`check.Zb1UFr`) passes in two generations with generated-code execution, six expected signal faults returned, a 17,335-page physical peak and complete cleanup. Properties/boot-clock (`check.Pxb0Zz`), memory/relocation (`check.x2KT7Q`) and APK/GC/JNI (`check.8rZwR4`) pass. Validation includes 1,128 workspace host tests, 395 Framework library tests, 20 Python checks, host-library and AArch64 Clippy, native boot/mobile and Bionic/thread/EOF regressions. Runtime inputs and all generated evidence stay local.

The unmodified Framework now passes code-cache initialization and reaches Java RuntimeInit. Public version properties and the source's 696-byte netbpfload version metadata bring the local input set to 348 files / 265,050,082 bytes; previous inputs are unchanged. The full gate (`check.fDK3by`) still fails: TrafficStats requests the missing dynamic JNI dependency `libframework-connectivity-tiramisu-jni.so`, then its exception path encounters the absent Binder driver. The failed process and all six workers are reclaimed with zero remaining resources. Parcel/Looper, ActivityThread, general APK support and A1–A6 remain incomplete. Earlier failure descriptions below are historical.

## Framework frontend and bootstrap update — November 6, 2026

The original Android 16 `app_process64` now loads its platform libraries, enters AndroidRuntime and executes ART's help/exit hooks in two supervised process generations. The verified source set contains 347 files / 265,049,386 bytes and all 46 boot jars, with unchanged hashes for the 66 earlier ART inputs. Acquisition supports checked resume and atomic receipts. Runtime inputs and notices remain local; only Bndroid adaptation code, owned probes and documentation enter Git. See [Framework Frontend-0](docs/milestones/androidbox/FRAMEWORK_FRONTEND_0.md).

This required a separate Framework resource profile, a proper AArch64 Image header to keep the DTB outside the larger BSS, fallible heap-backed descriptor tables, atomic fixed-address private-anonymous relocation for the linker's CFI shadow, and a public immutable boot-property area read by unmodified Bionic. The memory/property gates verify permissions, failed-operation atomicity, real timed property waiting and complete resource cleanup. The native/mobile and ART-only resource limits remain unchanged.

Frontend evidence `target/art-runtime/check.I7Nqtp` passes with 19,487 Linux calls, two zero-status exits, a 26,816-page peak, 62,118 reclaimed pages and no remaining sparse pages/tables/files/threads. The disk is unchanged. The separate managed Framework gate (`check.cD5XlQ`) gets past boot-DEX loading but still fails at code-cache initialization: its request for writable/executable memory is rejected. Its failed child is completely reaped. Framework Parcel/Looper, ActivityThread and general app acceptance are not claimed; `full_android_apps` remains false.

Passed: 1,127 default workspace host tests, 393 Framework library tests, 18 input/property/UART/proof tests, host/AArch64 Framework Clippy with tracing on/off, fixed-remap memory with real EL0 read-only faults (`check.P0oA2o`), boot properties (`check.Dc708z`), default boot, mobile UI (`target/mobile-ui/check-overview.e1HRsr`), dynamic Bionic, all four pthread lifecycle modes and EOF fault containment. The existing APK/GC/JNI gate (`check.x0g68T`) also passes with all 20 threads reaped and no residual process resources. The milestone contains the individual native evidence paths.

## Packaged ART, GC and JNI update — November 6, 2026

The `managed` ART gate now executes DEX and an aligned arm64 JNI library directly from a signed, owned APK. Both command-line launches validate Java workers/monitors/interruption, 64 MiB of small-array payload turnover and live reference graphs under concurrent-copying GC, weak-reference clearing/queue delivery, JNI array mutation/native exceptions, and a real pthread attaching to the VM and calling Java. Android's classloader namespace still rejects the existing non-public system-library copy. No loose managed DEX or extracted application library is present on the positive path. Package installation, ActivityThread, app graphics and Framework services remain unimplemented in this ART profile.

The kernel supplies actual private-anonymous `MADV_DONTNEED`: release translations/frames, retain rights, then demand-zero permitted user faults and exactly validated kernel-copy faults. File/shared discard and general lazy mmap remain unsupported. `gettimeofday` now supplies Java's wall time; process/thread CPU clocks charge real scheduled execution and keep retired-worker accounting separate from the leader's own CPU time. The previous real GC crash no longer occurs under the expanded workload. See [ART Managed-0](docs/milestones/androidbox/ART_MANAGED_0.md).

Final APK evidence `target/art-runtime/check.5wW9qO` reports both process exits, all 20 threads reaped, 61,198 Linux calls, 84,563 discarded pages, 36,407 demand-zero pages, a 51,532-page peak, and zero remaining sparse pages/tables/files/waiters. Temporary disk contents are unchanged. `managed_runtime_verified` and `apk_dex_jni_verified` are true; `full_android_apps` remains false. The SDK tools verify the compiled manifest, APK v2 test signature and 16 KiB library-entry alignment; the guest still has 4 KiB pages. Host checks also verify payload hashes, absence of loose positive-path inputs, and rejection of a duplicate/tampered APK before rootfs publication.

Validation passed: 1,127 default workspace host tests, 393 ART kernel-library tests and 13 ART/UART proof-parser tests. Host and AArch64 ART Clippy, including the tracing build, pass with warnings denied. The extended native memory/CPU gate (`check.PRRWnV`) proves RSS reduction, zero refill, usercopy recovery, guards, atomic failed discard and CPU accounting across sleep/reaping. The signal gate (`check.Oc8mE6`) includes a real denied write to a discarded read-only page; all five signal-containment cases pass (`check.eMcwxs`, `check.Fz4gXK`, `check.eSWlwx`, `check.rHbdS8`, `check.ZjK8it`). Basic Java (`check.NeevA6`), dynamic Bionic (`target/bionic-dynamic/check.VILQZe`), EOF containment (`check.b1ooMD`), default kernel boot and mobile UI (`target/mobile-ui/check-overview.THlsRo`) pass. Logs and generated APK/runtime artifacts remain local under ignored `target/`.

The next integration work remains upstream Framework/ActivityThread dependencies and Binder/service bridges, followed by installation/data paths and broader API/security/performance acceptance. Missing timezone/proc-cmdline/tracing data and optional directory/affinity/niceness calls remain visible in runtime summaries. A1 and the full JNI/native-APK gate stay incomplete.

## First managed ART execution and signals update — November 6, 2026

The default ART gate now passes. Two cold launches of the unmodified Android 16 runtime execute the unchanged standard DEX fixture's collections, strings, arithmetic and caught exception, then exit normally. Both process generations and all ten runtime threads are reaped; sparse pages/tables, file descriptions and waiters return to zero. `target/art-runtime/check.X9Dk4V` records 45,988 Linux calls, 64,085 peak sparse pages, 183,086 reclaimed pages, `managed_java_verified=true` and an unchanged temporary disk. The program is a command-line DEX fixture; `full_android_apps=false` and A1/ActivityThread/Framework acceptance remain incomplete.

The ART profile now has real AArch64 signal actions, alternate stacks, queued/synchronous delivery, validated sigreturn, interrupted/restarted futex waits and signal waits. A dedicated SIGKILL slot preserves group termination when the realtime queue is full. Futex requeue preserves waiter ownership, deadlines and bitsets. Realtime reads and absolute waits share a QEMU PL031/counter anchor. ART's region bound is 1,024 while its 256 MiB sparse-page bound remains; the guest is close to that page limit. Logging buffers were separated to retain the existing kernel-stack headroom checks. ART's two-launch deadline is 90 guest seconds, and syscall tracing is now opt-in. See [ART Signals-0](docs/milestones/androidbox/ART_SIGNALS_0.md) for semantics and limits.

Validation passed: 1,126 default workspace host tests, 392 ART kernel tests, eight ART proof-parser checks plus four UART checks, and host/AArch64 ART Clippy including tracing. The signal gate (`check.fNhwQ7`) delivered and returned 30 handlers, interrupted eight waits and reaped all 24 native test threads. Five expected-failure gates pass for corrupt PSTATE (`check.TnENCU`), kernel PC (`check.j8F1gm`), malformed FPSIMD (`check.CWH77e`), protected alternate stack (`check.FAkJPc`) and group SIGKILL with a full queue (`check.huEHr0`); every case proves complete fault cleanup.

ART memory (`check.TeE6fc`) and frontend (`check.mZ21I8`) regressions pass. Dynamic Bionic normal/leader-exit/group-exit/worker-fault runs (`check.pjxdIN`, `check.A6IW2U`, `check.rwlObN`, `check.QgWQr0`) and EOF containment (`check.DUOb6J`) pass. During this update, static Bionic (`target/bionic/check.DzV0QZ`), raw Linux ABI (`target/linux-abi/check.W8zN6I`), native boot and mobile UI (`target/mobile-ui/check-overview.EYXGnB`) also passed. Generated inputs, disks and logs remain ignored/local.

Missing timezone/proc-cmdline/tracing data and optional scheduler/gettimeofday calls are still reported in the Java summary. General writable runtime storage, discard/demand paging, broader GC/JNI/Java-thread stress, Binder, Framework and APK execution remain open. Earlier blocker descriptions below are historical and are superseded by this update.

## ART process and native initialization update — November 6, 2026

ART now reads its core jars, obtains real process/stack metadata and loads ICU, Java core and OpenJDK native libraries through the source Android linker's namespaces. The kernel supplies bounded `/proc/self/stat` and `/proc/self/maps` snapshots, actual CPU/memory accounting, stack guards, large file reads, read-only shared file mappings, access-pattern advice, private anonymous remapping and a bounded Android log endpoint. Extraction includes symlink-backed APEX dependencies and verifies configuration hashes. See [ART Process-0](docs/milestones/androidbox/ART_PROCESS_0.md).

Managed initialization still fails after `rt_sigaction(34, ...)` returns `ENOSYS`, with `java.io.IOException: Function not implemented`. The recorded child cleanup leaves no sparse pages, tables, file descriptions or threads. The supplied Java result has not occurred; A1, ActivityThread, Framework and general APK acceptance remain incomplete.

Validation passed: 1,121 default workspace host tests, 387 ART kernel tests, six ART proof-parser tests plus four UART tests, host/AArch64 ART Clippy, extended ART memory and frontend gates, raw/static/dynamic Bionic regression gates, all four pthread lifecycle modes, EOF containment, native boot and mobile UI. Local evidence includes `target/art-runtime/check.EXkPn7` (memory), `check.tV60g7` (help), and `check.N55rbZ` (managed failure). The APK example's required feature declaration was also corrected and checked.

## ART frontend and memory update — November 6, 2026

The separate `art-runtime` profile loads the installed Android 16 `dalvikvm64`, `libart.so` and required shared libraries without modifying them. ART's own help/parser completes in two distinct process generations, both exiting zero with no remaining sparse pages/tables, file descriptions or thread resources. An owned native gate also validates 64 MiB below 4 GiB, protection and hole handling, shared anonymous memory across a pthread, rejected kernel/cross-arena mappings, allocation bounds and complete reaping. The runtime source and local-only distribution boundary are frozen in [ADR 0001](docs/decisions/0001-android-runtime-baseline.md).

The real Java/DEX startup reaches ART initialization and aborts because `/proc/self/stat` is missing. This remains a failing gate with a recorded abort message and zero-resource cleanup; no Java, ActivityThread, Framework or general APK success is claimed. See [ART Frontend-0](docs/milestones/androidbox/ART_FRONTEND_0.md) and the [fixture guide](fixtures/art-runtime/README.md) for separate frontend, memory and managed modes.

Validation passed: 1,117 default workspace host tests, all 383 kernel-library tests under the ART feature (including seven VM model tests), five ART evidence-parser tests, four existing UART parser tests, ART library and complete AArch64 kernel Clippy with warnings denied, the raw Linux/static Bionic gates, all four dynamic pthread lifecycle gates, EOF fault containment, native boot, and the mobile UI gate. An altered runtime input is rejected before archive creation. The positive frontend evidence is `target/art-runtime/check.GxjRwC`; the final memory evidence is `check.vYJPbH`; the managed failure and cleanup are `check.lVxIsp`. These generated inputs/logs remain local and ignored by Git.

## Bionic pthread update — November 6, 2026

The existing dynamic profile now creates real Bionic pthreads sharing a process address space and file table, with separate kernel stacks, TIDs, TLS, signal masks, and clear-TID state. Futex waits/wakes/bitsets, monotonic deadlines, sleeping, yielding, join, and detach execute through the scheduler. Native process resources remain alive until the entire thread group stops.

Four QEMU gates passed: normal synchronization with 24 total created/reaped threads, main-thread-first exit, worker-initiated group exit, and a worker fault while siblings wait. All gates verify zero surviving thread stacks/waiters and full shared-resource cleanup. The 1,117 default host tests, synchronization helper tests, dynamic-profile Clippy, previous dynamic/static/raw Linux gates, EOF containment, native boot/fault tests, and mobile UI gate passed. See [Bionic Threads-0](docs/milestones/androidbox/BIONIC_THREADS_0.md) for scope and commands.

Signal-handler delivery, advanced futex modes, writable Linux filesystems, JNI/ART, Binder, Framework, and general Android applications remain unfinished. The single-core dynamic profile has a bounded global thread pool; this is not a full pthread/Linux or Android compatibility claim.

## Android dynamic linker update — November 6, 2026

The `bionic-dynamic-runtime` profile executes an ordinary NDK PIE through unmodified Android 16 `linker64`, shared libc/libm/libdl, and a separately built plugin. Both supervised process generations verify `dlopen`/symbols/unload, ELF TLS across timer preemption, constructors/destructors, private file mapping, and POSIX descriptor operations. The immutable runtime root comes from explicitly supplied local SDK files; a tested extraction helper uses an isolated temporary AVD. No upstream binaries enter Git.

The positive QEMU gate recorded 1,014 Linux calls, four bad-address checks, two zero-status exits, and zero remaining sparse page/table owners or file descriptions. An intentional read beyond file EOF is contained and also releases every child resource. The default 1,114 workspace host tests, predecessor static Bionic/raw Linux gates, native kernel boot and EL0 fault containment, mobile UI gate, and dynamic-profile Clippy with warnings denied passed. Nineteen Linux-feature helper tests and four UART parser tests passed; missing/corrupt root archives reject boot before userspace. See [Bionic Dynamic-0](docs/milestones/androidbox/BIONIC_DYNAMIC_0.md) for reproducible commands, frozen input hashes, detailed limits, and remaining syscall gaps.

Native dynamic linking works in this separate profile; thread support is described in the later update above. Signal delivery, writable Linux filesystems, JNI/APK native libraries, ART, Binder, Framework, and general Android applications still require implementation.

## Real static Bionic update — November 6, 2026

The `bionic-runtime` profile now boots an ordinary C program using NDK r27b/API 35 CRT and Bionic libraries without modifying them. Two process generations verify constructors, ELF TLS/errno across preemption, Scudo allocation through 2 MiB, stdio, atexit, and destructors. The kernel adds validated static metadata, saved thread pointers, private sparse anonymous memory, per-process standard streams and tagged-pointer control, and consumed-once host entropy. All anonymous page/table owners are reclaimed after exit.

The gate passes with 220 Linux calls, 12 bad-address results, and explicit `ENOSYS` for missing optional startup interfaces. The 1,104 default workspace tests, original native-entry gate, default kernel boot, mobile UI acceptance gate, four UART proof-parser checks, and Bionic Clippy check passed. Missing entropy rejects boot. See [Bionic Static-0](docs/milestones/androidbox/BIONIC_STATIC_0.md) for exact limits, syscall gaps, reproduction, and local artifact provenance.

This static profile provides real native Bionic execution. Dynamic linking and bounded runtime files are covered by the separate update above; threads/signals, a general writable filesystem ABI, JNI, ART, Binder, Framework, and general APK execution remain open.

## Native arm64 entry update — November 6, 2026

The isolated ABI 70 `linux-abi-runtime` profile loads and executes an NDK-compiled static arm64 ELF in a separate EL0 address space. It supplies a Linux `argc`/`argv`/`envp`/auxv stack, validates mapped program headers, routes Linux syscall numbers by kernel-owned process identity, and implements a small console/time/identity/exit subset. Two successive native processes verify fresh data/BSS, normal exit/reaping, and zero inherited native handles. Live-process accounting includes the new image type, preventing a scheduling-dependent snapshot panic.

The QEMU gate validates 52 Linux calls, eight unsupported calls, two rejected private SVCs, ten invalid-address results, bounded short writes, and an unchanged temporary-disk base. The default workspace's 1,096 host tests, 36 Linux-feature ABI tests, four Linux stack/time tests, default kernel boot, and mobile UI QEMU gate passed. See [native entry scope and reproduction](docs/milestones/androidbox/LINUX_ARM64_ENTRY.md).

The probe supplies its own startup code and does not link Bionic. Dynamic linking, TLS, ART, JNI, Binder, and general APK execution remain unimplemented in this profile; this is a foundation for their integration rather than an Android compatibility milestone completion.

## Interactive demo update — November 6, 2026

`scripts/run-mobile-ui.sh` provides an offline interactive demo, `--tour` for a guided walkthrough that leaves Home open, and `--headless` for a finite smoke check. It uses a temporary disk overlay and the checked-in Resources-1 APK without requiring an SDK. Build outputs are isolated from the pure-mobile gate; each session retains separate logs, screenshots, and a success summary. Startup and QMP failures are bounded and clean up only the runner's child process.

The guided QEMU run verified Calculator addition through a pixel comparison with the independently entered expected value, divide-by-zero recovery, shared theme/accent/accessibility changes, Activity/resource loading, a DEX callback, and return to Home. The AndroidBox result card now leaves room below its second text row instead of drawing its border through the result. The default workspace host suite also passes after correcting two compositor regression tests that incorrectly assumed mobile dimensions while running with the default 320x480 scanout. These tests continue to validate disjoint damage and cursor composition in both screen profiles.

See [the demo guide](docs/DEMO.md) for commands, controls, evidence, and limits. This update retains session-local settings, temporary storage, and bounded Android compatibility.

Validation for this update: 1,089 default-workspace host tests, 264 mobile UI host tests, 16 mobile compositor tests, 10 demo/QMP host checks, the mobile UI runtime gate, the AndroidBox Resources-1 QEMU gate, and the headless tour passed. Negative runs verified early QEMU exit and bounded startup timeout. The temporary disk's base image still matched the deterministic fixture after the tour. Evidence remains under ignored `target/`, including `usability-*.log` and each gate's session directory.

## Platform baseline

Bndroid has a custom Rust AArch64 kernel for QEMU virt, capability-oriented handles and objects, memory protection, channels/events, supervised processes, storage and package recovery paths, graphics, input, and application-data services. It remains a non-Linux research prototype with bounded Android compatibility.

## Mobile UI and appearance

- 720x1600, 20:9 scanout with a 360x800 design grid at 2x raster scale.
- Grouped Settings, a functional Display subpage, dark/accent/software-dimming controls, and explicit unavailable backend states.
- Shared Ocean/Violet dynamic color, semantic spacing/radii/elevation, bounded shadows, and contrast checks across Shell and projected Android UI.
- An Accessibility subpage with two semantic text sizes and high contrast, synchronized through SurfaceServer-owned UiAppearance state. Dynamic text respects fixed layout bounds through measurement and ellipsis.
- Settings renderer decomposition avoids its original EL0 stack-guard overflow; pure mobile UI retains its guarded four-page stack.

See [Settings/Display evidence](docs/milestones/ui/SETTINGS_DISPLAY_UI.md).

## Session-owned interactions

Overview supports one recent identity, upward dragging, rebound, and authorized dismissal: 32-pixel gesture start, 8-pixel quantization, and a 224-pixel removal threshold. Compatible icons bind to validated session/package-generation/APK-digest state. The top 120 pixels retain Quick Settings gesture priority, with pixel-exact restoration on close. These paths do not implement process killing, background tasks, thumbnails, or persistent history.

The lock-screen boot notification shares the shade's session and swipe controller: 8-pixel quantization, exact rebound below 160 pixels, and SurfaceServer-authorized dismissal at 160 pixels. Tapping does not navigate or unlock. There is no general notification posting, push, background delivery, sound, vibration, or persistence service.

SurfaceServer owns PL031 minute-clock refresh. It waits to the next minute boundary, rereads the clock, and increments ClockChanged only when the visible minute changes. Recorded QEMU evidence captured 09:41 to 09:42 with matching Launcher/App revisions and 122,336 pixels of damage across two clock regions. Clients have no RTC authority; timezone/DST, NTP, secure time, and physical/battery-backed RTC support are not established.

## Graphics transactions

Mobile and bounded DEX0 profiles provide two mapped backings per Launcher/App producer, with RW producer and RO SurfaceServer mappings. A 64-byte BufferPresent v6 transaction carries exact slots, Present/Discard, and up to two canonical non-overlapping damage regions, retaining v5 decoding. Page transitions use depth-two asynchronous one-ahead scheduling.

Copied AndroidBox layers share region validation and compositor publication. Compact regional producer rendering, separate written/presented baselines, and cached system chrome avoid full-layer transfers for supported changes. Dynamic text/button updates use at most two actual control rectangles; structure, identity, geometry, overflow, or region-cap violations require Full. Ownership, generation, and source XRGB validation remain enforced.

The recorded September 9 ABI 69 callbacks copied 322,048 and 310,528 bytes, with zero system-chrome writes, versus the 4,608,000-byte full-copy baseline. Kernel copy counters and the verifier distinguish byte volume from call count and reject forged or missing evidence. See [the complete graphics summary](docs/milestones/ui/MOBILE_MAPPED_BUFFERQUEUE.md).

The shared software frame clock supplies nominal 50 Hz grants from a 100 Hz divided timer. No measured FPS, hardware VSync, GPU throughput, or input-latency claim follows from that rate.

## AndroidBox

ABI 69 supports controlled SDK/D8/AAPT2 fixtures with fixed-and-weighted horizontal Buttons, exact positive integer dp dimensions, per-side spacing, dynamic StringBuilder text, retained Activity fields/state, verified icons, and bounded manifest/package catalogs. APK identity and admission remain strict through worker execution, scene transfer, and rendering.

[Layout Mixed-19](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_MIXED_19.md) records two real APKs, independent worker recovery, callbacks, icons, and source-free disk recovery. This is A0 in the roadmap, not general Android application compatibility.

## Validation entry points

Run host tests with the host target explicitly because `.cargo/config.toml` defaults to `aarch64-unknown-none`:

```sh
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --offline --target "$HOST_TARGET" --lib -p bndr-abi -p bndr-androidbox -p bndr-ui
cargo test --offline --target "$HOST_TARGET" --lib -p bndr-ui --features androidbox-layout-mixed19
./scripts/check-mobile-ui-runtime.sh
./scripts/check-androidbox-layout-mixed19.sh
```

The last gate requires the local SDK/JDK fixture prerequisites. Build and test logs stay under ignored `target/`. Full ART/Bionic/Binder/JNI/Framework integration, hardware drivers, product security services, and physical-phone readiness remain future work.
