# Running the mobile demo

The demo is an interactive QEMU system running the Bndroid AArch64 kernel and userspace. It includes a lock screen, Home, All apps, Calculator, Settings, and the bundled AndroidBox Resources-1 APK. The disk uses a writable temporary overlay; closing QEMU discards its changes. Appearance and application state are also session-local.

## Prerequisites

- Rust 1.91.0 with the components and `aarch64-unknown-none` target in `rust-toolchain.toml`.
- `qemu-system-aarch64`, including Cocoa on macOS or GTK on Linux for a window.
- Python 3.10 or newer and the usual Unix shell utilities.

Dependencies must already be installed. Builds run offline, and the included demo APK does not need SDK/JDK tools or a download. Run commands from the repository root.

## Start

```sh
./scripts/run-mobile-ui.sh
```

The script builds a release kernel, starts QEMU, checks guest readiness, and reports its log directory. On macOS the phone window scales to fit the display. Close the window or use Ctrl-C in the launching terminal to exit. The runner stops only the QEMU process it started.

```sh
# Automatically demonstrate the UI, then leave Home ready for interaction.
./scripts/run-mobile-ui.sh --tour

# Run the same checks without a window and exit with a success/failure status.
./scripts/run-mobile-ui.sh --headless

# Reuse the demo build after a successful initial run.
./scripts/run-mobile-ui.sh --skip-build --tour
```

`--help` prints usage without building or launching anything. The demo build lives in `target/mobile-ui-demo-build`, separate from the pure-mobile acceptance profile, so running that gate cannot silently replace the demo's AndroidBox-enabled kernel.

## Controls

Use the mouse as a single touch: hold the left button and drag for a swipe, and click for a tap. Allow a page transition to settle before the next gesture.

| Action | Control |
| --- | --- |
| Unlock | Swipe upward through the lower half of Lock |
| Open an app | Click a Home dock or All apps icon |
| Open All apps | Swipe upward through the middle of Home |
| Return Home | Click the bottom navigation bar |
| Open Overview | Drag upward a short distance from the bottom bar |
| Quick Settings | Drag downward from the top edge |
| Display preferences | Settings → Display |
| Text size / high contrast | Settings → Display → Accessibility |
| AndroidBox | Home → All apps → AndroidBox; click its execution button |

Calculator supports bounded decimal arithmetic, signs, percentages, backspace, and clear-to-recover errors. Phone and Messages remain local previews; they do not place calls or deliver messages. AndroidBox executes only the included bounded APK/DEX/resource subset. This demo does not establish general APK compatibility or physical-phone support.

## Repeatable evidence

Every run gets a separate `target/mobile-ui/demo.*/` directory containing `serial.log` and `qemu.log`, plus build logs when building. A successful tour adds 15 PPM screenshots and `summary.json` with screenshot hashes. The final `MOBILE_UI_DEMO_OK` marker is emitted only after all checks pass.

The tour checks real QMP touch input, committed guest frames, and guest trace evidence:

- Unlock and navigation through Home and All apps.
- Calculator `12 + 7 = 19`, comparing the numeric display against independently entering `19`; division by zero and clear recovery.
- Dark/light and Ocean/Violet changes, large text, and high contrast reaching both Launcher and App; original appearance restored before the end.
- The bundled APK's manifest-selected Activity `onCreate`, compiled resource resolution, and DEX `onTap` result.
- A final return to Home with no guest failure marker.

These are smoke checks. The existing mobile and AndroidBox gates retain their deeper geometry, damage, identity, and lifecycle assertions:

```sh
python3 scripts/test_mobile_ui_demo.py
./scripts/check-mobile-ui-runtime.sh
./scripts/check-androidbox-dex0.sh  # Requires the local SDK aapt2 oracle.
```

## Troubleshooting

Startup, frame waits, and QMP calls have deadlines. Failures exit nonzero, stop the owned QEMU child, and retain logs with the last diagnostic lines printed in the terminal. A stalled QMP connection, mismatched response, or malformed reply cannot count as success.

- If a dependency is missing, install the listed prerequisites; no installer runs automatically.
- If a reused image has the wrong profile, rerun without `--skip-build`.
- For a display backend error, use `--headless`, or select a locally supported backend with `BNDROID_QEMU_DISPLAY`.
- For a slower host, set a larger positive `BNDROID_QEMU_TIMEOUT_SECONDS` (default 60 seconds per guest wait).
- `BNDROID_MOBILE_TARGET_DIR` selects a build directory. `BNDROID_QMP_SOCKET` can select an unused `.sock` path inside `target/mobile-ui`; existing paths are refused instead of unlinked, and Unix socket length limits are checked.

For example:

```sh
BNDROID_QEMU_TIMEOUT_SECONDS=120 ./scripts/run-mobile-ui.sh --headless
```
