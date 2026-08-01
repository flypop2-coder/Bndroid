#!/usr/bin/env python3
"""Stop one M80 boot at a deterministic durable plan boundary."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

import unified_product_qmp as product


MODES = ("cut-prepared", "cut-applying", "cut-effect", "cancel-prepared")
PHASE_PREFIX = "MAINTENANCE_PLAN_PHASE_OK "
STEP_PREFIX = "MAINTENANCE_STEP_COMMIT_OK "
PAUSE_PREFIX = "MAINTENANCE_PLAN_TEST_PAUSE "


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--serial", type=Path, required=True)
    parser.add_argument("--qmp", type=Path, required=True)
    parser.add_argument("--mode", choices=MODES, required=True)
    parser.add_argument("--cpu", choices=("cortex-a72", "max"), default="cortex-a72")
    parser.add_argument("--timeout", type=int, default=120)
    return parser.parse_args()


def stop_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def matching_lines(log: str, prefix: str) -> list[str]:
    return [line for line in log.splitlines() if line.startswith(prefix)]


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
        product.replay_interaction(qmp, process, args.serial, args.timeout)
        if args.mode == "cut-effect":
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
        product.wait_for_prefix(
            process,
            args.serial,
            PAUSE_PREFIX.strip(),
            1,
            args.timeout,
        )
        qmp.command({"execute": "stop"})
        time.sleep(0.02)
        log = product.read_log(args.serial)
        product.reject_failure(log)

        phases = matching_lines(log, PHASE_PREFIX)
        steps = matching_lines(log, STEP_PREFIX)
        expected_phases = 2 if args.mode != "cut-prepared" else 1
        expected_steps = 1 if args.mode == "cut-effect" else 0
        if len(phases) != expected_phases or len(steps) != expected_steps:
            raise product.RuntimeFailure(
                f"M80 {args.mode} observed phases={len(phases)} steps={len(steps)}, "
                f"expected {expected_phases}/{expected_steps}"
            )
        if args.mode == "cancel-prepared":
            if len(matching_lines(log, "MAINTENANCE_PLAN_CANCEL_OK ")) != 1:
                raise product.RuntimeFailure("M80 cancellation marker was not exact")
            if "requested_phase=4 " not in phases[-1]:
                raise product.RuntimeFailure("M80 cancellation did not select COMPENSATED")
        forbidden = (
            "MAINTENANCE_PLAN_TERMINAL_OK ",
            "MAINTENANCE_EXECUTION_COMMIT_OK ",
            "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
            "BOOT_OK: M80 durable ",
        )
        if any(marker in log for marker in forbidden):
            raise product.RuntimeFailure(
                f"M80 {args.mode} crossed its terminal boundary"
            )
        stop_process(process)
        print(
            "MAINTENANCE_PLAN_INTERRUPT_BOOT_OK "
            f"mode={args.mode} durable_plan_transitions={len(phases)} "
            f"durable_step_effects={len(steps)} terminal_plan=0 "
            "aggregate_completion=0 host_stop_after_marker=1 qemu_terminated_by_host=1 "
            "powercut_claim=0 hardware_powercut_claim=0 emulator_only=1 "
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
