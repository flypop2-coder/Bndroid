#!/usr/bin/env python3
"""Replay the real UI transcript, request product poweroff, and await self-exit."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import socket
import subprocess
import sys
import time
from pathlib import Path


FINAL_SCREEN_SHA256 = "1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
MILESTONE_BOOT_MARKERS = {
    "M67": (
        "BOOT_OK: M67 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M67 interactive product, AppData, bounded shutdown, and QEMU exit armed",
    ),
    "M68": (
        "BOOT_OK: M68 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M68 unified product liveness recovery, AppData, bounded shutdown, and QEMU exit armed",
    ),
    "M69": (
        "BOOT_OK: M69 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M69 unified two-service dependency liveness, AppData, bounded shutdown, and QEMU exit armed",
    ),
    "M70": (
        "BOOT_OK: M70 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M70 FDT-validated QEMU PSCI SYSTEM_OFF, two-service liveness, AppData, and bounded shutdown armed",
    ),
    "M71": (
        "BOOT_OK: M71 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M71 five-service continuous supervision, concurrent miss recovery, AppData, and PSCI shutdown armed",
    ),
    "M72": (
        "BOOT_OK: M72 unified real UI is interactive and awaiting authenticated power key",
        "BOOT_OK: M72 kernel-manifest service discovery, five-service supervision, AppData, and PSCI shutdown armed",
    ),
    "M73": (
        "BOOT_OK: M73 unified real UI is interactive; event supervision and external maintenance controls are starting",
        "BOOT_OK: M73 event-driven five-service supervision, two clean storage rotations, in-flight drain, AppData, and PSCI shutdown armed",
    ),
    "M74": (
        "BOOT_OK: M74 unified real UI is interactive; verified-manifest event supervision is starting",
        "BOOT_OK: M74 verified external manifest, rollback floor, event supervision, AppData, and PSCI shutdown armed",
    ),
    "M75": (
        "BOOT_OK: M75 unified real UI is interactive; persistent-rollback event supervision is starting",
        "BOOT_OK: M75 persistent rollback ledger, verified external manifest, event supervision, AppData, and PSCI shutdown armed",
    ),
    "M76": (
        "BOOT_OK: M76 unified real UI is interactive; persistent key-policy event supervision is starting",
        "BOOT_OK: M76 persistent key rotation and revocation, offline-signing artifact, event supervision, AppData, and PSCI shutdown armed",
    ),
    "M77": (
        "BOOT_OK: M77 unified real UI is interactive; signed maintenance-session event supervision is starting",
        "BOOT_OK: M77 signed maintenance session, durable anti-replay audit, persistent key policy, event supervision, AppData, and PSCI shutdown armed",
    ),
    "M78": (
        "BOOT_OK: M78 unified real UI is interactive; recoverable maintenance execution is starting",
        "BOOT_OK: M78 durable maintenance execution completion, exact interrupted-session recovery, AppData, and PSCI shutdown armed",
    ),
    "M79": (
        "BOOT_OK: M79 unified real UI is interactive; durable maintenance-step reconciliation is starting",
        "BOOT_OK: M79 durable three-step maintenance journal, idempotent cut-point recovery, AppData, and PSCI shutdown armed",
    ),
    "M80": (
        "BOOT_OK: M80 unified real UI is interactive; durable maintenance-plan reconciliation is starting",
        "BOOT_OK: M80 durable maintenance plan phases, result-unknown reconciliation, AppData, and PSCI shutdown armed",
    ),
    "M81": (
        "BOOT_OK: M81 unified real UI is interactive; signed descriptor-bound maintenance-plan reconciliation is starting",
        "BOOT_OK: M81 signed descriptor-bound maintenance plan, durable program binding, AppData, and PSCI shutdown armed",
    ),
}
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)",
    re.IGNORECASE | re.MULTILINE,
)
POSITIONS = {
    "overlap": (13970, 12587),
    "launcher": (7396, 6568),
    "phone-outside": (2055, 1369),
    "soft-a": (9040, 25722),
    "soft-backspace": (15614, 25722),
    "soft-enter": (23009, 25722),
}


class RuntimeFailure(RuntimeError):
    pass


class Qmp:
    def __init__(self, socket_path: Path, process: subprocess.Popen[bytes], timeout: int):
        self._socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        deadline = time.monotonic() + timeout
        while True:
            try:
                self._socket.connect(str(socket_path))
                break
            except (FileNotFoundError, ConnectionRefusedError) as error:
                if process.poll() is not None:
                    raise RuntimeFailure(
                        f"QEMU exited with status {process.returncode} before QMP was ready"
                    ) from error
                if time.monotonic() >= deadline:
                    raise RuntimeFailure("QMP socket did not become ready") from error
                time.sleep(0.02)
        self._stream = self._socket.makefile("rwb", buffering=0)
        greeting = json.loads(self._stream.readline())
        if "QMP" not in greeting:
            raise RuntimeFailure("invalid QMP greeting")
        self.command({"execute": "qmp_capabilities"})

    def close(self) -> None:
        self._stream.close()
        self._socket.close()

    def command(self, payload: dict[str, object]) -> object:
        self._stream.write(json.dumps(payload).encode("ascii") + b"\n")
        while True:
            line = self._stream.readline()
            if not line:
                raise RuntimeFailure("QMP connection closed")
            response = json.loads(line)
            if "error" in response:
                raise RuntimeFailure(f"QMP error: {response['error']}")
            if "return" in response:
                return response["return"]

    def events(self, events: list[dict[str, object]]) -> None:
        self.command(
            {
                "execute": "input-send-event",
                "arguments": {"events": events},
            }
        )
        time.sleep(0.05)

    def absolute(self, name: str) -> None:
        x, y = POSITIONS[name]
        self.events(
            [
                {"type": "abs", "data": {"axis": "x", "value": x}},
                {"type": "abs", "data": {"axis": "y", "value": y}},
            ]
        )

    def touch(self, down: bool) -> None:
        self.events([{"type": "btn", "data": {"down": down, "button": "touch"}}])

    def click(self, name: str) -> None:
        self.absolute(name)
        self.touch(True)
        self.touch(False)

    def key(self, qcode: str) -> None:
        for down in (True, False):
            self.events(
                [
                    {
                        "type": "key",
                        "data": {
                            "down": down,
                            "key": {"type": "qcode", "data": qcode},
                        },
                    }
                ]
            )

    def screenshot(self, output: Path) -> None:
        self.command(
            {
                "execute": "screendump",
                "arguments": {"filename": str(output), "format": "ppm"},
            }
        )


def read_log(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace").replace("\r", "")
    except FileNotFoundError:
        return ""


def reject_failure(log: str) -> None:
    match = FAILURE_PATTERN.search(log)
    if match is not None:
        raise RuntimeFailure(f"guest emitted failure evidence: {match.group(0)!r}")


def wait_for_exact(
    process: subprocess.Popen[bytes], serial: Path, marker: str, timeout: int
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        log = read_log(serial)
        reject_failure(log)
        count = log.splitlines().count(marker)
        if count == 1:
            return
        if count > 1:
            raise RuntimeFailure(f"duplicate exact marker: {marker}")
        if process.poll() is not None:
            raise RuntimeFailure(
                f"QEMU exited with status {process.returncode} before {marker}"
            )
        time.sleep(0.03)
    raise RuntimeFailure(f"timed out waiting for {marker}")


def wait_for_prefix(
    process: subprocess.Popen[bytes],
    serial: Path,
    prefix: str,
    count: int,
    timeout: int,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        log = read_log(serial)
        reject_failure(log)
        observed = sum(line.startswith(prefix + " ") for line in log.splitlines())
        if observed == count:
            return
        if observed > count:
            raise RuntimeFailure(f"{prefix} count exceeded {count}: {observed}")
        if process.poll() is not None:
            raise RuntimeFailure(
                f"QEMU exited with status {process.returncode} before {prefix}"
            )
        time.sleep(0.03)
    raise RuntimeFailure(f"timed out waiting for {prefix} count {count}")


def validate_screenshot(path: Path, milestone: str) -> str:
    payload = path.read_bytes()
    if not payload.startswith(b"P6\n320 480\n255\n"):
        raise RuntimeFailure(
            f"{milestone} screenshot is not exact 320x480 binary PPM"
        )
    if len(payload) != len(b"P6\n320 480\n255\n") + 320 * 480 * 3:
        raise RuntimeFailure(f"{milestone} screenshot payload length changed")
    digest = hashlib.sha256(payload).hexdigest()
    if digest != FINAL_SCREEN_SHA256:
        raise RuntimeFailure(
            f"{milestone} final UI digest changed: {digest}, expected {FINAL_SCREEN_SHA256}"
        )
    return digest


def replay_interaction(
    qmp: Qmp,
    process: subprocess.Popen[bytes],
    serial: Path,
    timeout: int,
) -> None:
    wait_for_exact(process, serial, "WINDOW_INPUT_PHASE1_READY", timeout)
    qmp.absolute("overlap")
    qmp.touch(True)
    qmp.absolute("launcher")
    qmp.touch(False)

    wait_for_exact(process, serial, "WINDOW_INPUT_PHASE2_READY", timeout)
    qmp.click("overlap")
    wait_for_exact(process, serial, "PERSISTENT_WINDOW_INPUT_READY", timeout)
    qmp.click("phone-outside")
    qmp.absolute("overlap")
    qmp.touch(True)
    qmp.absolute("phone-outside")
    qmp.touch(False)

    wait_for_exact(process, serial, "TEXT_POINTER_FOCUS_READY", timeout)
    qmp.click("overlap")
    wait_for_exact(process, serial, "TEXT_INPUT_READY", timeout)
    for index, qcode in enumerate(("a", "ret", "backspace", "a", "ret"), 1):
        qmp.key(qcode)
        wait_for_prefix(process, serial, "TEXT_FIELD_RENDER_OK", index, timeout)
    qmp.click("launcher")
    wait_for_exact(
        process,
        serial,
        "TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0",
        timeout,
    )
    qmp.key("a")
    wait_for_prefix(process, serial, "TEXT_INPUT_OK", 1, timeout)

    wait_for_exact(
        process,
        serial,
        "SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6",
        timeout,
    )
    qmp.click("overlap")
    wait_for_exact(
        process,
        serial,
        "SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible",
        timeout,
    )
    for index, position in enumerate(
        ("soft-a", "soft-enter", "soft-backspace", "soft-a", "soft-enter"), 1
    ):
        qmp.click(position)
        wait_for_prefix(process, serial, "SOFT_TEXT_RENDER_OK", index, timeout)
    qmp.click("launcher")
    wait_for_exact(
        process,
        serial,
        "SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent",
        timeout,
    )
    qmp.click("soft-a")
    time.sleep(0.20)
    qmp.click("overlap")
    wait_for_prefix(process, serial, "UNIFIED_PRODUCT_UI_OK", 1, timeout)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--serial", type=Path, required=True)
    parser.add_argument("--qmp", type=Path, required=True)
    parser.add_argument("--screenshot", type=Path, required=True)
    parser.add_argument(
        "--phase",
        choices=(
            "first",
            "second",
            "transition",
            "activate",
            "repair",
            "steady",
            "sequence1",
            "sequence2",
            "resume",
            "normal",
            "recover-step1",
            "recover-step2",
            "recover-step3",
            "recover-corrupt",
            "recover-prepared",
            "recover-applying",
            "recover-effect",
            "recover-plan-corrupt",
            "recover-binding",
        ),
        required=True,
    )
    parser.add_argument(
        "--milestone", choices=tuple(MILESTONE_BOOT_MARKERS), default="M67"
    )
    parser.add_argument("--cpu", choices=("cortex-a72", "max"), default="cortex-a72")
    parser.add_argument("--timeout", type=int, default=120)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.timeout < 1 or args.timeout > 600:
        raise SystemExit("--timeout must be from 1 through 600")
    for path in (args.kernel, args.disk):
        if not path.is_file():
            raise SystemExit(f"missing input: {path}")
    for path in (args.serial, args.qmp, args.screenshot):
        path.parent.mkdir(parents=True, exist_ok=True)

    process = subprocess.Popen(
        [
            "qemu-system-aarch64",
            "-machine",
            "virt,gic-version=2,secure=off,virtualization=off",
            "-cpu",
            args.cpu,
            "-smp",
            "1",
            "-m",
            "128M",
            "-display",
            "none",
            "-monitor",
            "none",
            "-nic",
            "none",
            "-serial",
            f"file:{args.serial}",
            "-qmp",
            f"unix:{args.qmp},server=on,wait=off",
            "-no-reboot",
            *(
                ["-semihosting-config", "enable=on,target=native"]
                if args.milestone
                not in {"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}
                else []
            ),
            "-kernel",
            str(args.kernel),
            "-device",
            "ramfb",
            "-global",
            "virtio-mmio.force-legacy=false",
            "-drive",
            (
                f"if=none,file={args.disk},format=raw,readonly=off,snapshot=off,"
                "cache=writeback,id=bndroid-storage"
            ),
            "-device",
            (
                "virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,"
                "indirect_desc=off,config-wce=off,write-cache=on,discard=off,"
                "write-zeroes=off"
            ),
            "-device",
            (
                "virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,"
                "packed=off,queue_reset=off"
            ),
            "-device",
            (
                "virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,"
                "packed=off,queue_reset=off,wheel-axis=on"
            ),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.STDOUT,
    )
    qmp: Qmp | None = None
    try:
        qmp = Qmp(args.qmp, process, args.timeout)
        replay_interaction(qmp, process, args.serial, args.timeout)
        qmp.screenshot(args.screenshot)
        digest = validate_screenshot(args.screenshot, args.milestone)
        if args.milestone in {"M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}:
            wait_for_prefix(
                process,
                args.serial,
                "M73_EVENT_SUPERVISOR_ACTIVE_OK",
                1,
                args.timeout,
            )
            qmp.key("f5")
            wait_for_prefix(
                process,
                args.serial,
                "M73_STORAGE_ROTATION_OK",
                1,
                args.timeout,
            )
            if args.milestone in {"M80", "M81"}:
                wait_for_prefix(
                    process,
                    args.serial,
                    "MAINTENANCE_PLAN_PHASE_OK",
                    5,
                    args.timeout,
                )
            qmp.key("f5")
            wait_for_prefix(
                process,
                args.serial,
                "M73_STORAGE_ROTATION_OK",
                2,
                args.timeout,
            )
            if args.milestone in {"M80", "M81"}:
                wait_for_prefix(
                    process,
                    args.serial,
                    "MAINTENANCE_PLAN_PHASE_OK",
                    8,
                    args.timeout,
                )
            wait_for_prefix(
                process,
                args.serial,
                "M73_CANCEL_WINDOW_OK",
                1,
                args.timeout,
            )
            qmp.key("power")
        else:
            qmp.key("power")

        deadline = time.monotonic() + args.timeout
        while process.poll() is None:
            reject_failure(read_log(args.serial))
            if time.monotonic() >= deadline:
                raise RuntimeFailure("guest did not self-exit after the power key")
            time.sleep(0.05)
        status = process.returncode
        log = read_log(args.serial)
        reject_failure(log)
        if status != 0:
            raise RuntimeFailure(f"QEMU self-exit status was {status}, expected 0")
        required = [
            "UNIFIED_PRODUCT_UI_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
            *MILESTONE_BOOT_MARKERS[args.milestone],
        ]
        if args.milestone == "M68":
            required.append("UNIFIED_PRODUCT_LIVENESS_OK ")
        elif args.milestone == "M69":
            required.append("UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK ")
        elif args.milestone == "M70":
            required.extend(
                (
                    "M70_PSCI_DISCOVERY_OK ",
                    "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M71":
            required.extend(
                (
                    "M71_PSCI_DISCOVERY_OK ",
                    "UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M72":
            required.extend(
                (
                    "M72_PSCI_DISCOVERY_OK ",
                    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M73":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M74":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M75":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "PERSISTENT_ROLLBACK_LEDGER_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M76":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M77":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "MAINTENANCE_AUDIT_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M78":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "MAINTENANCE_AUDIT_OK ",
                    "MAINTENANCE_EXECUTION_ADMISSION_OK ",
                    "MAINTENANCE_EXECUTION_COMMIT_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M79":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "MAINTENANCE_AUDIT_OK ",
                    "MAINTENANCE_EXECUTION_ADMISSION_OK ",
                    "MAINTENANCE_STEP_ADMISSION_OK ",
                    "MAINTENANCE_STEP_TERMINAL_OK ",
                    "MAINTENANCE_EXECUTION_COMMIT_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_STEP_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M80":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "MAINTENANCE_AUDIT_OK ",
                    "MAINTENANCE_EXECUTION_ADMISSION_OK ",
                    "MAINTENANCE_STEP_ADMISSION_OK ",
                    "MAINTENANCE_PLAN_ADMISSION_OK ",
                    "MAINTENANCE_STEP_TERMINAL_OK ",
                    "MAINTENANCE_PLAN_TERMINAL_OK ",
                    "MAINTENANCE_EXECUTION_COMMIT_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        elif args.milestone == "M81":
            required.extend(
                (
                    "M73_PSCI_DISCOVERY_OK ",
                    "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
                    "M73_CANCEL_WINDOW_OK ",
                    "KEY_ROTATION_POLICY_OK ",
                    "MAINTENANCE_AUDIT_OK ",
                    "MAINTENANCE_EXECUTION_ADMISSION_OK ",
                    "MAINTENANCE_STEP_ADMISSION_OK ",
                    "MAINTENANCE_PLAN_ADMISSION_OK ",
                    "SIGNED_MAINTENANCE_PLAN_OK ",
                    "MAINTENANCE_STEP_TERMINAL_OK ",
                    "MAINTENANCE_PLAN_TERMINAL_OK ",
                    "SIGNED_MAINTENANCE_PLAN_TERMINAL_OK ",
                    "MAINTENANCE_EXECUTION_COMMIT_OK ",
                    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
                    "UNIFIED_PRODUCT_KEY_ROTATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK ",
                    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK ",
                    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK ",
                    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK ",
                )
            )
        for marker in required:
            count = sum(
                line == marker or line.startswith(marker)
                for line in log.splitlines()
            )
            if count != 1:
                raise RuntimeFailure(f"final marker count for {marker!r} was {count}")
        if args.milestone in {"M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}:
            rotations = sum(
                line.startswith("M73_STORAGE_ROTATION_OK ")
                for line in log.splitlines()
            )
            if rotations != 2:
                raise RuntimeFailure(
                    f"{args.milestone} storage rotation marker count was {rotations}, expected 2"
                )
            if args.milestone == "M79":
                steps = sum(
                    line.startswith("MAINTENANCE_STEP_COMMIT_OK ")
                    for line in log.splitlines()
                )
                if steps != 3:
                    raise RuntimeFailure(
                        f"M79 maintenance step marker count was {steps}, expected 3"
                    )
            if args.milestone in {"M80", "M81"}:
                steps = sum(
                    line.startswith("MAINTENANCE_STEP_COMMIT_OK ")
                    for line in log.splitlines()
                )
                phases = sum(
                    line.startswith("MAINTENANCE_PLAN_PHASE_OK ")
                    for line in log.splitlines()
                )
                if steps != 3 or phases != 9:
                    raise RuntimeFailure(
                        "M80 maintenance marker counts changed: "
                        f"steps={steps} phases={phases}"
                    )
                if args.milestone == "M81":
                    descriptor_phases = sum(
                        line.startswith("SIGNED_MAINTENANCE_OPERATION_BOUND ")
                        for line in log.splitlines()
                    )
                    if descriptor_phases != 9:
                        raise RuntimeFailure(
                            "M81 signed descriptor phase count changed: "
                            f"{descriptor_phases}"
                        )
            maintenance = " maintenance_rotations=2 cancel_pending=4"
        else:
            maintenance = ""
        print(
            "UNIFIED_PRODUCT_QMP_BOOT_OK "
            f"phase={args.phase} qemu_self_exit=1 power_key=116 "
            f"ui_sha256={digest}{maintenance}"
        )
        return 0
    except Exception:
        sys.stderr.write(read_log(args.serial)[-30_000:])
        raise
    finally:
        if qmp is not None:
            qmp.close()
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    raise SystemExit(main())
