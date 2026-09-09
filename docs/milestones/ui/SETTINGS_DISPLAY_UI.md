# Settings, Display, and Accessibility

Recorded work began August 23, 2026, with later buffering, dynamic-color, and accessibility regressions. This page summarizes the [complete original evidence](../../archive/originals/SETTINGS_DISPLAY_UI.txt).

## User-visible behavior

The local QEMU scanout is 720x1600 (20:9), with a 360x800 design grid rendered at exactly 2x scale. This design scale is not a physical-panel DPI claim.

Settings groups Personalization, Connections, This phone, and System while retaining the Apps and About phone hit targets. Display & appearance is a real subpage with session-local dark mode, accent selection, and five software-dimming levels. Back returns to Settings.

Unavailable capabilities have explicit states: networking shows `Unavailable in QEMU`, peripherals show `No device transport`, and sound shows `Not implemented`. Display states `Software surface only`; it does not control physical backlight, HDR, refresh rate, or a real panel.

| Action | Physical half-open rectangle (x/y/width/height) |
| --- | --- |
| Settings to Display | `32/408/656/132` |
| Display dark mode | `32/408/656/132` |
| Display accent | `32/540/656/132` |
| Display dimming | `336/744/336/112` |
| Settings Apps | `48/1284/640/132` |
| Settings About | `48/1416/640/132` |

## Guarded-stack repair

The original QEMU Settings interaction exposed an EL0 guard-page fault not caught by host raster tests. The monolithic release AArch64 renderer allocated approximately 8,352 stack bytes before its call chain, exceeding the ordinary App's 16 KiB stack.

Sequential `#[inline(never)]` page components and borrowed read-only MobileModel state reduced the main render_settings frame to 128 bytes and the largest page component to 2,416 bytes. QEMU then opened both pages without user faults. The Settings fix retained the original stack and guards.

A separate bounded DEX0 deep-parser regression later increased the applicable non-storage, non-interactive DEX0/install stacks from four to eight pages, preserving both guards. Pure mobile UI retained its four-page stack. AndroidBox rejection-reason mapping was also completed without expanding accepted APK formats.

## Shared appearance

Dynamic color v2 derives dark/light wallpaper, backgrounds, surfaces, raised surfaces, panels, outlines, accent containers, and on-accent ink from Ocean/Violet selection. Settings, Display, the home date card, Dock, Calculator, and projected Android controls reuse semantic spacing, radii, elevation, typography, and bounded shadows.

Display opens a real Accessibility page with two semantic text sizes and explicit high contrast. SurfaceServer owns one UiAppearance session state and distributes it to Launcher, Shell App, and projected Android Activity. APK text is measured at its final size and ellipsized within fixed layout bounds. Contrast gates cover accent/icon, primary, and secondary text. Appearance changes do not grant service/device authority or alter authorized hit/damage geometry.

## Buffering regression

Mobile and bounded DEX0 now use two mapped Launcher/App slots with producer RW and SurfaceServer RO rights. BufferPresent v6 carries exact slot selection, Present/Discard, and up to two canonical non-overlapping regions in 64 bytes, retaining v5 decode compatibility. Depth-two asynchronous scheduling has recorded dual queueing and in-flight publication.

First frames, focus-generation changes, Discard, and unsupported state changes require Full. Supported press/clock/Phone/Calculator updates use shared damage planning, regional raster/copy/composition, and separate written/presented baselines. Settings page and theme changes still fall back to Full when required.

The shared software frame clock supplies nominal 50 Hz grants from a divided 100 Hz logical timer. It is not hardware VSync or measured FPS. See [the graphics transaction report](MOBILE_MAPPED_BUFFERQUEUE.md).

## Evidence and limits

The original Settings gate recorded `MOBILE_UI_PREVIEW_OK`, a 720x1600 scanout, zero user faults after the stack fix, 269 UI tests, clean UI Clippy, and successful AArch64 check/release builds for the recorded preview. Later reports record their own counts and profiles; these are historical results, not synchronization-time test results.

Original screenshots, serial logs, hashes, and regression artifacts are retained by path in the source record under ignored `target/mobile-ui/`. They demonstrate local geometry, state, input, isolation, and pixels, not GPU composition, power, 60/120 Hz rendering, or real touch latency.

Remaining work includes actual sound/network/peripheral/settings backends, persistent preference policy, Unicode shaping, locale/RTL, broader font scaling, screen-reader semantics, and real-device validation. See [TODO.md](../../../TODO.md).
