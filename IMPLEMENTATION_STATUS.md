# Implementation status

This page records implemented behavior and validation, with a September 9, 2026 platform baseline and dated updates below. Historical test counts belong to the indicated development runs; they are not a claim that every historical gate was rerun during publication. The [complete original status log](docs/archive/originals/IMPLEMENTATION_STATUS.txt) preserves detailed evidence and earlier milestones. Future tasks belong only in [TODO.md](TODO.md).

## Android dynamic linker update — November 6, 2026

The `bionic-dynamic-runtime` profile executes an ordinary NDK PIE through unmodified Android 16 `linker64`, shared libc/libm/libdl, and a separately built plugin. Both supervised process generations verify `dlopen`/symbols/unload, ELF TLS across timer preemption, constructors/destructors, private file mapping, and POSIX descriptor operations. The immutable runtime root comes from explicitly supplied local SDK files; a tested extraction helper uses an isolated temporary AVD. No upstream binaries enter Git.

The positive QEMU gate recorded 1,014 Linux calls, four bad-address checks, two zero-status exits, and zero remaining sparse page/table owners or file descriptions. An intentional read beyond file EOF is contained and also releases every child resource. The default 1,114 workspace host tests, predecessor static Bionic/raw Linux gates, native kernel boot and EL0 fault containment, mobile UI gate, and dynamic-profile Clippy with warnings denied passed. Nineteen Linux-feature helper tests and four UART parser tests passed; missing/corrupt root archives reject boot before userspace. See [Bionic Dynamic-0](docs/milestones/androidbox/BIONIC_DYNAMIC_0.md) for reproducible commands, frozen input hashes, detailed limits, and remaining syscall gaps.

Native dynamic linking now works in this separate profile. Threads, signal delivery, writable Linux filesystems, JNI/APK native libraries, ART, Binder, Framework, and general Android applications still require implementation.

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
