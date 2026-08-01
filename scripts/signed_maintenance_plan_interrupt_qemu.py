#!/usr/bin/env python3
"""Stop one M81 boot after the signed program is durable but before audit."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

import unified_product_qmp as product


PAUSE_PREFIX = "SIGNED_MAINTENANCE_PLAN_TEST_PAUSE "


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--serial", type=Path, required=True)
    parser.add_argument("--qmp", type=Path, required=True)
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


def parse_fields(line: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for token in line.split()[1:]:
        if token.count("=") != 1:
            raise product.RuntimeFailure("M81 pause contains a non-field token")
        key, value = token.split("=", 1)
        if not key or not value or key in fields:
            raise product.RuntimeFailure("M81 pause contains an invalid field")
        fields[key] = value
    return fields


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
        pauses = [
            line for line in log.splitlines() if line.startswith(PAUSE_PREFIX)
        ]
        if len(pauses) != 1:
            raise product.RuntimeFailure(
                f"M81 signed-program pause marker count was {len(pauses)}"
            )
        expected = {
            "format": "1",
            "mode": "cut-binding",
            "boundary": "program-bound-before-audit",
            "sequence": "2",
            "generation": "1",
            "slot": "0",
            "reads": "14",
            "writes": "1",
            "flushes": "1",
            "program_mutations": "1",
            "audit_attempted": "0",
            "audit_mutations": "0",
            "host_termination_required": "1",
            "qemu_disk_durable": "1",
            "hardware_powercut_claim": "0",
            "emulator_only": "1",
            "real_phone_claim": "0",
        }
        fields = parse_fields(pauses[0])
        if fields != expected:
            raise product.RuntimeFailure(
                f"M81 signed-program pause fields changed: {fields!r}"
            )
        forbidden = (
            "SIGNED_MAINTENANCE_PLAN_OK ",
            "MAINTENANCE_AUDIT_OK ",
            "MAINTENANCE_PLAN_ADMISSION_OK ",
            "USER_MAP_OK ",
            "ELF_LOAD_OK ",
            "UNIFIED_PRODUCT_UI_OK ",
            "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK ",
            "UNIFIED_PRODUCT_SHUTDOWN_OK ",
            "BOOT_OK: M81 ",
        )
        for marker in forbidden:
            if marker in log:
                raise product.RuntimeFailure(
                    f"M81 interrupted boot crossed its boundary: {marker}"
                )
        stop_process(process)
        print(
            "SIGNED_MAINTENANCE_PLAN_INTERRUPT_BOOT_OK "
            "mode=cut-binding sequence=2 generation=1 slot=0 "
            "program_reads=14 program_writes=1 program_flushes=1 "
            "audit_attempted=0 audit_mutations=0 pre_el0=1 "
            "host_stop_after_marker=1 qemu_terminated_by_host=1 "
            "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
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
