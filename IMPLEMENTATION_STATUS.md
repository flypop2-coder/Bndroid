# Implementation status

This page summarizes implemented behavior recorded through September 9, 2026. Historical test counts below belong to the indicated development runs; they are not a claim that every historical gate was rerun during publication. The [complete original status log](docs/archive/originals/IMPLEMENTATION_STATUS.txt) preserves detailed evidence and earlier milestones. Future tasks belong only in [TODO.md](TODO.md).

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
