#!/usr/bin/env python3
"""Stop one M78 boot after durable admission but before execution completion."""

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
    parser.add_argument("--sequence", type=int, required=True)
    parser.add_argument("--cpu", choices=("cortex-a72", "max"), default="cortex-a72")
    parser.add_argument("--timeout", type=int, default=120)
    return parser.parse_args()


def wait_for_admission(
    process: subprocess.Popen[bytes],
    serial: Path,
    sequence: int,
    timeout: int,
) -> str:
    prefix = "MAINTENANCE_EXECUTION_ADMISSION_OK "
    expected = f"sequence={sequence} resumed=0 "
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        log = product.read_log(serial)
        admissions = [
            line for line in log.splitlines() if line.startswith(prefix)
        ]
        if len(admissions) == 1 and expected in admissions[0]:
            return log
        if len(admissions) > 1:
            raise product.RuntimeFailure(
                f"M78 admission marker count was {len(admissions)}, expected one"
            )
        unexpected = [
            line
            for line in log.splitlines()
            if product.FAILURE_PATTERN.search(line)
        ]
        if unexpected:
            raise product.RuntimeFailure(
                f"unexpected guest failure before M78 interruption: {unexpected[-1]}"
            )
        if process.poll() is not None:
            raise product.RuntimeFailure(
                f"QEMU exited with status {process.returncode} before M78 admission"
            )
        time.sleep(0.02)
    raise product.RuntimeFailure("timed out waiting for M78 durable admission")


def stop_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def main() -> int:
    args = parse_args()
    if args.timeout < 1 or args.timeout > 600:
        raise SystemExit("--timeout must be from 1 through 600")
    if args.sequence < 1:
        raise SystemExit("--sequence must be positive")
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
        log = wait_for_admission(process, args.serial, args.sequence, args.timeout)
        for forbidden in (
            "MAINTENANCE_EXECUTION_COMMIT_OK ",
            "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        ):
            if forbidden in log:
                raise product.RuntimeFailure(
                    f"M78 interruption crossed the completion boundary: {forbidden}"
                )
        stop_process(process)
        print(
            "MAINTENANCE_EXECUTION_INTERRUPT_BOOT_OK "
            f"sequence={args.sequence} admission_pre_el0=1 audit_committed=1 "
            "execution_completion=0 host_interrupted_before_completion=1 "
            "qemu_terminated_by_host=1 powercut_claim=0 emulator_only=1 "
            "real_phone_claim=0"
        )
        return 0
    except Exception:
        sys.stderr.write(product.read_log(args.serial)[-30_000:])
        raise
    finally:
        if qmp is not None:
            qmp.close()
        stop_process(process)


if __name__ == "__main__":
    raise SystemExit(main())
