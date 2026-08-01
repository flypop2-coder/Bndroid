#!/usr/bin/env python3
"""Boot one M75 artifact until the pre-EL0 rollback boundary rejects it."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

import unified_product_qmp as product


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--serial", type=Path, required=True)
    parser.add_argument("--qmp", type=Path, required=True)
    parser.add_argument("--reason", choices=("signature", "rollback"), required=True)
    parser.add_argument("--cpu", choices=("cortex-a72", "max"), default="cortex-a72")
    parser.add_argument("--timeout", type=int, default=120)
    return parser.parse_args()


def wait_for_rejection(
    process: subprocess.Popen[bytes],
    serial: Path,
    reason: str,
    timeout: int,
) -> str:
    marker = f"PERSISTENT_ROLLBACK_REJECTED format=1 reason={reason} "
    diagnostic = "boot error: persistent manifest preparation failed:"
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        log = product.read_log(serial)
        if marker in log and diagnostic in log:
            return log
        unexpected = [
            line
            for line in log.splitlines()
            if product.FAILURE_PATTERN.search(line)
            and not line.startswith(diagnostic)
        ]
        if unexpected:
            raise product.RuntimeFailure(
                f"unexpected guest failure during M75 negative boot: {unexpected[-1]}"
            )
        if process.poll() is not None:
            raise product.RuntimeFailure(
                f"QEMU exited with status {process.returncode} before rejection evidence"
            )
        time.sleep(0.03)
    raise product.RuntimeFailure(f"timed out waiting for {reason} rejection")


def main() -> int:
    args = parse_args()
    if args.timeout < 1 or args.timeout > 600:
        raise SystemExit("--timeout must be from 1 through 600")
    for path in (args.kernel, args.disk):
        if not path.is_file():
            raise SystemExit(f"missing input: {path}")
    for path in (args.serial, args.qmp):
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
    qmp: product.Qmp | None = None
    try:
        qmp = product.Qmp(args.qmp, process, args.timeout)
        log = wait_for_rejection(process, args.serial, args.reason, args.timeout)
        forbidden = [
            "PERSISTENT_ROLLBACK_LEDGER_OK ",
            "USER_MAP_OK ",
            "ELF_LOAD_OK ",
            "UNIFIED_PRODUCT_UI_OK ",
            "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
            "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
            "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
            "BOOT_OK: M75 ",
        ]
        for marker in forbidden:
            if marker in log:
                raise product.RuntimeFailure(
                    f"M75 negative boot crossed fail-closed boundary: {marker}"
                )
        rejection = [
            line
            for line in log.splitlines()
            if line.startswith(
                f"PERSISTENT_ROLLBACK_REJECTED format=1 reason={args.reason} "
            )
        ]
        if len(rejection) != 1:
            raise product.RuntimeFailure(
                f"{args.reason} rejection marker count was {len(rejection)}"
            )
        signature_valid = int(args.reason == "rollback")
        print(
            "PERSISTENT_ROLLBACK_NEGATIVE_BOOT_OK "
            f"reason={args.reason} pre_el0=1 signature_valid={signature_valid} "
            "manifest_published=0 init_ready=0 qemu_terminated_by_host=1 "
            "emulator_only=1 real_phone_claim=0"
        )
        return 0
    except Exception:
        sys.stderr.write(product.read_log(args.serial)[-30_000:])
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
