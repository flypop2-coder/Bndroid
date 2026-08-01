#!/usr/bin/env python3
"""Stop one M79 boot immediately after a selected durable fixed-program step."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

import unified_product_qmp as product


STEP_PREFIX = "MAINTENANCE_STEP_COMMIT_OK "


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--serial", type=Path, required=True)
    parser.add_argument("--qmp", type=Path, required=True)
    parser.add_argument("--sequence", type=int, required=True)
    parser.add_argument("--step", type=int, choices=(1, 2, 3), required=True)
    parser.add_argument("--cpu", choices=("cortex-a72", "max"), default="cortex-a72")
    parser.add_argument("--timeout", type=int, default=120)
    return parser.parse_args()


def wait_for_step(
    process: subprocess.Popen[bytes],
    serial: Path,
    sequence: int,
    step: int,
    timeout: int,
) -> str:
    expected = f"sequence={sequence} requested_step={step} "
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        log = product.read_log(serial)
        matches = [
            line
            for line in log.splitlines()
            if line.startswith(STEP_PREFIX) and expected in line
        ]
        if len(matches) == 1:
            return log
        if len(matches) > 1:
            raise product.RuntimeFailure(
                f"M79 step {step} marker count was {len(matches)}, expected one"
            )
        product.reject_failure(log)
        if process.poll() is not None:
            raise product.RuntimeFailure(
                f"QEMU exited with status {process.returncode} before M79 step {step}"
            )
        time.sleep(0.01)
    raise product.RuntimeFailure(f"timed out waiting for M79 durable step {step}")


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
        product.replay_interaction(qmp, process, args.serial, args.timeout)
        product.wait_for_prefix(
            process,
            args.serial,
            "M73_EVENT_SUPERVISOR_ACTIVE_OK",
            1,
            args.timeout,
        )
        qmp.key("f5")
        product.wait_for_prefix(
            process,
            args.serial,
            "M73_STORAGE_ROTATION_OK",
            1,
            args.timeout,
        )
        log = wait_for_step(process, args.serial, args.sequence, 1, args.timeout)
        if args.step >= 2:
            qmp.key("f5")
            product.wait_for_prefix(
                process,
                args.serial,
                "M73_STORAGE_ROTATION_OK",
                2,
                args.timeout,
            )
            log = wait_for_step(process, args.serial, args.sequence, 2, args.timeout)
        if args.step == 3:
            product.wait_for_prefix(
                process,
                args.serial,
                "M73_CANCEL_WINDOW_OK",
                1,
                args.timeout,
            )
            qmp.key("power")
            log = wait_for_step(process, args.serial, args.sequence, 3, args.timeout)

        qmp.command({"execute": "stop"})
        time.sleep(0.02)
        log = product.read_log(args.serial)
        expected_steps = list(range(1, args.step + 1))
        observed_steps = []
        for line in log.splitlines():
            if not line.startswith(STEP_PREFIX):
                continue
            fields = {
                key: value
                for key, value in (
                    token.split("=", 1)
                    for token in line.split()[1:]
                    if "=" in token
                )
            }
            if int(fields.get("sequence", "0")) == args.sequence:
                observed_steps.append(int(fields.get("requested_step", "0")))
        if observed_steps != expected_steps:
            raise product.RuntimeFailure(
                f"M79 interruption observed steps {observed_steps}, expected {expected_steps}"
            )
        for forbidden in (
            "MAINTENANCE_STEP_TERMINAL_OK ",
            "MAINTENANCE_EXECUTION_COMMIT_OK ",
            "UNIFIED_PRODUCT_MAINTENANCE_STEP_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
            "BOOT_OK: M79 durable ",
        ):
            if forbidden in log:
                raise product.RuntimeFailure(
                    f"M79 step {args.step} interruption crossed boundary: {forbidden}"
                )
        stop_process(process)
        print(
            "MAINTENANCE_STEP_INTERRUPT_BOOT_OK "
            f"sequence={args.sequence} cut_after_step={args.step} "
            f"durable_steps={args.step} later_step_observed=0 "
            "terminal_read=0 aggregate_completion=0 host_stop_after_marker=1 "
            "qemu_terminated_by_host=1 powercut_claim=0 hardware_powercut_claim=0 "
            "emulator_only=1 real_phone_claim=0"
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
