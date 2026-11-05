#!/usr/bin/env python3
"""Supervise the local QEMU demo and optionally exercise it through real input."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

from mobile_ui_qmp import Qmp, drag, move, screenshot, tap

ROOT = Path(__file__).resolve().parent.parent
FAILURE = re.compile(
    r"fatal exception:|kernel panic:|panicked at|boot error:|"
    r"MOBILE_UI_PREVIEW_(?:TIMEOUT|FAULT|RUNTIME_FAIL)|"
    r"MOBILE_UI_(?:CHILD_DIAG|USER_FAULT)|"
    r"(?:FRAMEBUFFER|INPUT|UI|USER)_FAIL:|EL0_FAIL",
    re.IGNORECASE,
)
COMMIT = re.compile(r"^USER_SURFACE_BUFFER_COMMIT_OK .*", re.MULTILINE)


def validate_socket(path: str) -> Path:
    socket_path = Path(path).absolute()
    directory = (ROOT / "target/mobile-ui").resolve()
    if not socket_path.resolve().is_relative_to(directory) or socket_path.suffix != ".sock":
        raise ValueError("QMP socket must be a .sock path inside target/mobile-ui")
    if any(character in str(socket_path) for character in (",", "\n", "\r")):
        raise ValueError("QMP socket paths cannot contain commas or newlines")
    # macOS sockaddr_un has a 104-byte sun_path, including the trailing NUL.
    if len(os.fsencode(socket_path)) >= 104:
        raise ValueError("QMP socket path is too long (maximum 103 bytes)")
    if os.path.lexists(socket_path):
        raise ValueError(f"QMP socket already exists; refusing to replace it: {socket_path}")
    if not socket_path.parent.is_dir():
        raise ValueError(f"QMP socket directory does not exist: {socket_path.parent}")
    return socket_path


def ppm_pixels(path: Path) -> bytes:
    parts = path.read_bytes().split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise RuntimeError(f"Unexpected screenshot geometry: {path}")
    if len(parts[3]) != 720 * 1600 * 3:
        raise RuntimeError(f"Truncated screenshot: {path}")
    return parts[3]


def calculator_display(pixels: bytes) -> bytes:
    # Only the numeric display: excludes the clock, keys and pointer.
    return b"".join(pixels[(y * 720 + 32) * 3:(y * 720 + 688) * 3] for y in range(192, 360))


class Demo:
    def __init__(self, process: subprocess.Popen, artifacts: Path, timeout: float) -> None:
        self.process = process
        self.artifacts = artifacts
        self.timeout = timeout
        self.qmp: Qmp | None = None
        self.snapshots: dict[str, str] = {}

    def log(self, require_running: bool = True) -> str:
        text = (self.artifacts / "serial.log").read_text(errors="replace").replace("\r", "")
        failure = FAILURE.search(text)
        if failure:
            raise RuntimeError(f"Guest failure: {text[failure.start():].splitlines()[0]}")
        if require_running and self.process.poll() is not None:
            raise RuntimeError(f"QEMU exited unexpectedly (status {self.process.returncode})")
        return text

    def wait_for(self, pattern: str, description: str, start: int = 0) -> None:
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            if re.search(pattern, self.log()[start:], re.MULTILINE):
                return
            time.sleep(0.05)
        raise TimeoutError(f"Timed out waiting for {description}")

    def settle(self, before: int, minimum_commits: int = 1) -> None:
        """Require new committed pixels and let the bounded transition finish."""
        deadline = time.monotonic() + self.timeout
        last_count = before
        changed_at = time.monotonic()
        while time.monotonic() < deadline:
            count = len(COMMIT.findall(self.log()))
            if count != last_count:
                last_count, changed_at = count, time.monotonic()
            if count >= before + minimum_commits and time.monotonic() - changed_at >= 0.4:
                return
            time.sleep(0.05)
        raise TimeoutError("The UI did not commit and settle after input")

    def touch(self, x: int, y: int, minimum_commits: int = 1) -> None:
        before = len(COMMIT.findall(self.log()))
        tap(self.qmp, x, y)
        self.settle(before, minimum_commits)

    def swipe(self, x: int, y: int, end_x: int, end_y: int) -> None:
        before = len(COMMIT.findall(self.log()))
        drag(self.qmp, x, y, end_x, end_y)
        self.settle(before)

    def capture(self, name: str, androidbox_ready: bool = False) -> bytes:
        move(self.qmp, 719, 1599)
        path = self.artifacts / f"{name}.ppm"
        deadline = time.monotonic() + self.timeout
        while True:
            self.log()
            screenshot(self.qmp, str(path))
            pixels = ppm_pixels(path)
            # This unadorned part of the execution button is absent from the
            # intermediate page frames, matching the Resources-1 QEMU gate.
            offset = (1300 * 720 + 80) * 3
            if not androidbox_ready or pixels[offset:offset + 3] == bytes((16, 118, 111)):
                break
            if time.monotonic() >= deadline:
                raise TimeoutError("The AndroidBox page did not finish its transition")
            time.sleep(0.05)
        self.snapshots[path.name] = hashlib.sha256(pixels).hexdigest()
        return pixels

    def appearance(self, x: int, y: int, expected: str) -> None:
        start = len(self.log())
        self.touch(x, y)
        for client in ("launcher", "app"):
            self.wait_for(
                rf"^UI_APPEARANCE_CHANGED_OK .* receiver_image={client} .*{expected}",
                f"{client} appearance synchronization", start,
            )

    def tour(self) -> None:
        print("Tour: unlocking Home", flush=True)
        self.capture("01-lock")
        self.swipe(360, 1200, 360, 800)
        self.wait_for(r"^UI_SYSTEM_UI_CHANGED_OK .* mode=home ", "unlocked Home")
        home = self.capture("02-home")

        print("Tour: Calculator, checking 12 + 7 = 19 and divide-by-zero recovery", flush=True)
        self.touch(445, 1430, minimum_commits=7)
        zero = calculator_display(self.capture("03-calculator-zero"))
        for point in ((108, 1164), (270, 1164), (594, 1164), (108, 764), (594, 1364)):
            self.touch(*point)
        result = calculator_display(self.capture("04-calculator-sum"))
        self.touch(108, 564)  # Clear; independently enter the expected result.
        self.touch(108, 1164)
        self.touch(432, 764)
        literal = calculator_display(self.capture("05-calculator-literal"))
        if result != literal or result == zero:
            raise RuntimeError("Calculator pixels for 12 + 7 do not equal the entered value 19")
        self.touch(594, 564)  # Divide by zero must display an error.
        self.touch(189, 1364)
        self.touch(594, 1364)
        error = calculator_display(self.capture("06-calculator-error"))
        if error in (zero, result):
            raise RuntimeError("Divide by zero did not change the calculator output")
        self.touch(108, 564)
        if calculator_display(self.capture("07-calculator-recovered")) != zero:
            raise RuntimeError("Calculator did not recover after Clear")
        self.touch(360, 1570)

        print("Tour: Settings, shared theme, accent and accessibility", flush=True)
        self.touch(615, 1430)
        self.wait_for(r"^UI_ROUTE_FOCUS_OK .* active_client=app app=settings ", "Settings focus")
        self.capture("08-settings")
        self.touch(360, 474)
        self.capture("09-display-dark")
        self.appearance(360, 474, r"theme=light ")
        self.appearance(360, 606, r"accent=violet ")
        self.capture("10-display-light-violet")
        self.touch(360, 1380)
        self.appearance(360, 474, r"large_text=1 ")
        self.appearance(360, 606, r"high_contrast=1$")
        self.capture("11-accessibility")
        self.appearance(360, 474, r"large_text=0 ")
        self.appearance(360, 606, r"high_contrast=0$")
        self.touch(56, 120)
        self.appearance(360, 474, r"theme=dark ")
        self.appearance(360, 606, r"accent=ocean ")
        self.touch(360, 1570)

        print("Tour: AndroidBox, executing the bundled APK and its DEX callback", flush=True)
        self.swipe(360, 1000, 360, 760)
        self.capture("12-all-apps")
        # Press + release + all five transition frames. APK parsing can take
        # longer than the quiet interval after the release frame, especially
        # with Cocoa display enabled. Never click through that transition.
        self.touch(105, 700, minimum_commits=7)
        self.wait_for(r"^ANDROIDBOX_DEX_EXECUTED_OK .* kind=boot .* result=20260729 ", "DEX boot")
        self.wait_for(r"^ANDROIDBOX_ACTIVITY_EXECUTED_OK .* on_create_method=7 .*", "Activity onCreate")
        self.wait_for(r"^ANDROIDBOX_RESOURCES_RESOLVED_OK .* success_flags=31 ", "APK resources")
        self.capture("13-androidbox", androidbox_ready=True)
        self.touch(360, 1312)
        self.wait_for(r"^ANDROIDBOX_DEX_EXECUTED_OK .* kind=tap .* result=20260736 .* tap_count=1$", "DEX callback")
        self.capture("14-androidbox-callback")
        self.touch(360, 1570)
        returned = self.capture("15-home-ready")
        # Compare the dock and wallpaper below the clock; minute changes must
        # not hide a failed navigation or turn this into a clock timing test.
        dock = slice(1100 * 720 * 3, 1512 * 720 * 3)
        if returned[dock] != home[dock]:
            raise RuntimeError("The tour did not restore the Home dock")
        self.log()
        summary = {
            "status": "passed", "disk": "temporary-overlay", "screen": "720x1600",
            "calculator": "12 + 7 = 19 (pixel comparison with literal 19)",
            "calculator_error_recovery": True, "appearance_synced_clients": 2,
            "androidbox": "bundled Resources-1 APK; DEX boot + onTap",
            "general_android_compatibility": False, "screenshots_sha256": self.snapshots,
        }
        (self.artifacts / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(f"MOBILE_UI_DEMO_OK screenshots={len(self.snapshots)} evidence={self.artifacts}", flush=True)


def interrupted(_signum: int, _frame: object) -> None:
    raise KeyboardInterrupt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--validate-socket")
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--socket")
    parser.add_argument("--tour", action="store_true")
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.validate_socket:
        validate_socket(args.validate_socket)
        return 0
    if not args.artifacts or not args.socket or not args.command or args.command[0] != "--":
        parser.error("use run-mobile-ui.sh to launch the demo")
    timeout = float(os.environ.get("BNDROID_QEMU_TIMEOUT_SECONDS", "60"))
    if not math.isfinite(timeout) or timeout <= 0:
        raise ValueError("BNDROID_QEMU_TIMEOUT_SECONDS must be finite and positive")
    socket_path = validate_socket(args.socket)
    signal.signal(signal.SIGTERM, interrupted)
    process = None
    demo = None
    try:
        with (args.artifacts / "qemu.log").open("wb") as output:
            process = subprocess.Popen(args.command[1:], stdout=output, stderr=subprocess.STDOUT)
        demo = Demo(process, args.artifacts, timeout)
        demo.wait_for(r"^MOBILE_UI_PREVIEW_OK .* width=720 height=1600 ", "mobile boot")
        demo.wait_for(r"^ANDROIDBOX_DEX0_PROFILE_OK ", "the AndroidBox demo profile (rebuild if using --skip-build)")
        demo.qmp = Qmp(str(socket_path))
        print("Demo ready. Swipe up to unlock; click icons; click the bottom bar for Home.", flush=True)
        if args.tour:
            demo.tour()
        # Leave the socket available to the documented QMP helper while the
        # user interacts. The remaining health monitor only reads serial logs.
        demo.qmp.close()
        demo.qmp = None
        if not args.headless:
            print("The demo is interactive. Close the QEMU window or press Ctrl-C to stop.", flush=True)
            while process.poll() is None:
                time.sleep(0.25)
                if process.poll() is None:
                    demo.log(require_running=False)
            if process.returncode != 0:
                raise RuntimeError(f"QEMU exited with status {process.returncode}")
        return 0
    except KeyboardInterrupt:
        return 130
    except (Exception, SystemExit) as error:
        print(f"Demo failed: {error}\nEvidence: {args.artifacts}", file=sys.stderr)
        for name in ("serial.log", "qemu.log"):
            path = args.artifacts / name
            if path.exists():
                print("\n".join(path.read_text(errors="replace").splitlines()[-12:]), file=sys.stderr)
        return 1
    finally:
        if demo and demo.qmp:
            demo.qmp.close()
        if process is not None:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            socket_path.unlink(missing_ok=True)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError) as error:
        sys.exit(str(error))
