#!/usr/bin/env python3
"""Strict M70 FDT/PSCI serial, host-exit, and two-boot disk evidence parser."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import unified_product_multiservice_liveness_evidence as m69


FINAL_SCREEN_SHA256 = m69.FINAL_SCREEN_SHA256
PHASES = m69.PHASES
UI_MARKER = m69.UI_MARKER.replace("abi=30", "abi=31", 1)
UI_BOOT_MARKER = (
    "BOOT_OK: M70 unified real UI is interactive and awaiting authenticated power key"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M70 FDT-validated QEMU PSCI SYSTEM_OFF, two-service liveness, "
    "AppData, and bounded shutdown armed"
)
LIVENESS_MARKER = m69.LIVENESS_MARKER.replace("abi=30", "abi=31", 1)
DISCOVERY_MARKER = (
    "M70_PSCI_DISCOVERY_OK format=1 abi=31 node=/psci "
    "compatible=arm,psci-1.0 method=hvc psci_version=1.1 "
    "version_raw=0x00010001 version_probe=PSCI_VERSION "
    "version_function_id=0x84000000 system_off_function_id=0x84000008 "
    "fdt_validated=1 installed_once=1 semihosting=0 emulator_only=1 "
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0"
)
PSCI_SHUTDOWN_MARKER = (
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK format=1 abi=31 "
    "authority=fdt-plus-psci-version-plus-kernel-seal node=/psci "
    "compatible=arm,psci-1.0 method=hvc psci_version=1.1 "
    "version_raw=0x00010001 version_probe=PSCI_VERSION "
    "version_function_id=0x84000000 system_off_function_id=0x84000008 "
    "backend=qemu-psci system_off_requested=1 host_self_exit_verified=0 "
    "semihosting=0 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 "
    "real_phone_claim=0"
)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M70_(?:FRAME|STACK|EXIT|SYSCALL)_TRACE(?: |$)|qemu-semihosting",
    re.IGNORECASE | re.MULTILINE,
)


class EvidenceError(ValueError):
    pass


def shutdown_marker(phase: str) -> str:
    return (
        m69.shutdown_marker(phase)
        .replace("abi=30", "abi=31", 1)
        .replace(
            "emulator_backend=qemu-semihosting",
            "emulator_backend=qemu-psci",
            1,
        )
        .replace("psci_claim=0", "psci_claim=1", 1)
    )


def validate_text(text: str, phase: str) -> None:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M70 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M70 BOOT_OK marker set changed")

    exact_markers = (
        ("M70_PSCI_DISCOVERY_OK", DISCOVERY_MARKER),
        ("UNIFIED_PRODUCT_UI_OK", UI_MARKER),
        ("UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK", LIVENESS_MARKER),
        ("UNIFIED_PRODUCT_SHUTDOWN_OK", shutdown_marker(phase)),
        ("UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK", PSCI_SHUTDOWN_MARKER),
    )
    positions: list[int] = []
    for prefix, expected in exact_markers:
        observed = m69.m68.exact_line(lines, prefix)
        if observed != expected:
            raise EvidenceError(f"M70 {prefix} marker changed")
        positions.append(lines.index(observed))
    m69.m68.require_exact(lines, UI_BOOT_MARKER)
    m69.m68.require_exact(lines, SHUTDOWN_BOOT_MARKER)
    ui_boot_position = lines.index(UI_BOOT_MARKER)
    shutdown_boot_position = lines.index(SHUTDOWN_BOOT_MARKER)
    expected_order = [
        positions[0],
        positions[1],
        ui_boot_position,
        positions[2],
        positions[3],
        positions[4],
        shutdown_boot_position,
    ]
    if expected_order != sorted(expected_order) or len(set(expected_order)) != len(
        expected_order
    ):
        raise EvidenceError("M70 discovery/UI/liveness/seal/PSCI marker order changed")

    if (
        m69.m68.exact_line(lines, "DATA_PERSIST_OK")
        != m69.m68.m67.data_marker(phase)
    ):
        raise EvidenceError("M70 DATA_PERSIST_OK marker changed")
    if (
        m69.m68.exact_line(lines, "STORAGE_DEVICE_HEALTH_OK")
        != m69.m68.m67.health_marker(phase)
    ):
        raise EvidenceError("M70 STORAGE_DEVICE_HEALTH_OK marker changed")

    exact_checkpoints = (
        "WINDOW_INPUT_PHASE1_READY",
        "WINDOW_INPUT_PHASE2_READY",
        "PERSISTENT_WINDOW_INPUT_READY",
        "TEXT_POINTER_FOCUS_READY",
        "TEXT_INPUT_READY",
        "TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0",
        "SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6",
        "SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible",
        "SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent",
    )
    for checkpoint in exact_checkpoints:
        m69.m68.require_exact(lines, checkpoint)
    for prefix, expected_count in (
        ("TEXT_FIELD_RENDER_OK", 5),
        ("SOFT_TEXT_RENDER_OK", 5),
        ("TEXT_INPUT_OK", 1),
    ):
        observed = sum(line.startswith(prefix + " ") for line in lines)
        if observed != expected_count:
            raise EvidenceError(
                f"{prefix} count is {observed}, expected {expected_count}"
            )


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={FINAL_SCREEN_SHA256}"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M70 {phase} host QMP self-exit evidence changed")


def synthetic_log(phase: str) -> str:
    base = m69.synthetic_log(phase)
    base = base.replace(m69.UI_MARKER, UI_MARKER, 1)
    base = base.replace(m69.UI_BOOT_MARKER, UI_BOOT_MARKER, 1)
    base = base.replace(m69.LIVENESS_MARKER, LIVENESS_MARKER, 1)
    base = base.replace(m69.shutdown_marker(phase), shutdown_marker(phase), 1)
    base = base.replace(
        m69.SHUTDOWN_BOOT_MARKER,
        PSCI_SHUTDOWN_MARKER + "\n" + SHUTDOWN_BOOT_MARKER,
        1,
    )
    return DISCOVERY_MARKER + "\n" + base


def parser_self_test() -> None:
    for phase in PHASES:
        validate_text(synthetic_log(phase), phase)
        validate_driver(driver_marker(phase) + "\n", phase)

    base = synthetic_log("first")
    reordered = base.replace(
        shutdown_marker("first") + "\n" + PSCI_SHUTDOWN_MARKER,
        PSCI_SHUTDOWN_MARKER + "\n" + shutdown_marker("first"),
        1,
    )
    mutations = (
        base.replace("abi=31", "abi=30", 1),
        base.replace("compatible=arm,psci-1.0", "compatible=arm,psci-0.2", 1),
        base.replace("method=hvc", "method=smc", 1),
        base.replace("psci_version=1.1", "psci_version=1.0", 1),
        base.replace("version_raw=0x00010001", "version_raw=0x00010000", 1),
        base.replace("system_off_function_id=0x84000008", "system_off_function_id=0x0", 1),
        base.replace("semihosting=0", "semihosting=1", 1),
        base.replace("psci_claim=1", "psci_claim=0", 1),
        base.replace(DISCOVERY_MARKER + "\n", "", 1),
        base.replace(PSCI_SHUTDOWN_MARKER + "\n", "", 1),
        base + PSCI_SHUTDOWN_MARKER + "\n",
        reordered,
        base + "M70_SYSCALL_TRACE pid=1 syscall=1\n",
    )
    rejected = 0
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except (EvidenceError, m69.EvidenceError, m69.m68.EvidenceError):
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M70 parser negative self-test accepted a mutation")

    bad_drivers = (
        driver_marker("first").replace("qemu_self_exit=1", "qemu_self_exit=0"),
        driver_marker("first") + "\n" + driver_marker("first"),
        "",
    )
    driver_rejected = 0
    for mutated in bad_drivers:
        try:
            validate_driver(mutated, "first")
        except EvidenceError:
            driver_rejected += 1
    if driver_rejected != len(bad_drivers):
        raise EvidenceError("M70 host-exit parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_PSCI_SHUTDOWN_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES) * 2} serial_negative={rejected} "
        f"host_negative={driver_rejected}"
    )


def verify_reboot(
    pristine_path: Path,
    runtime_path: Path,
    first_serial_path: Path,
    second_serial_path: Path,
    first_driver_path: Path,
    second_driver_path: Path,
) -> str:
    validate_text(first_serial_path.read_text(encoding="utf-8", errors="replace"), "first")
    validate_text(
        second_serial_path.read_text(encoding="utf-8", errors="replace"), "second"
    )
    validate_driver(
        first_driver_path.read_text(encoding="utf-8", errors="replace"), "first"
    )
    validate_driver(
        second_driver_path.read_text(encoding="utf-8", errors="replace"), "second"
    )
    marker = m69.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared M69 disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
        "UNIFIED_PRODUCT_PSCI_SHUTDOWN_REBOOT_OK",
        1,
    )
    marker = marker.replace(
        "product_shutdowns=2 ",
        "product_shutdowns=2 psci_discoveries=2 psci_version_probes=2 "
        "psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 "
        "psci_system_off_requests=2 qemu_psci_self_exits=2 "
        "semihosting_uses=0 ",
        1,
    )
    return marker.replace(
        "hardware_poweroff_claim=0 psci_claim=0",
        "hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1",
        1,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    parse = subparsers.add_parser("parse")
    parse.add_argument("phase", choices=tuple(PHASES))
    parse.add_argument("log", type=Path)
    reboot = subparsers.add_parser("reboot")
    reboot.add_argument("pristine", type=Path)
    reboot.add_argument("runtime", type=Path)
    reboot.add_argument("first_serial", type=Path)
    reboot.add_argument("second_serial", type=Path)
    reboot.add_argument("first_driver", type=Path)
    reboot.add_argument("second_driver", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "self-test":
            parser_self_test()
        elif args.command == "parse":
            validate_text(
                args.log.read_text(encoding="utf-8", errors="replace"), args.phase
            )
            print(f"UNIFIED_PRODUCT_PSCI_SHUTDOWN_EVIDENCE_OK phase={args.phase}")
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.pristine,
                    args.runtime,
                    args.first_serial,
                    args.second_serial,
                    args.first_driver,
                    args.second_driver,
                )
            )
        else:
            raise AssertionError("unreachable command")
    except (
        EvidenceError,
        m69.EvidenceError,
        m69.m68.EvidenceError,
        m69.m68.m67.EvidenceError,
        OSError,
    ) as error:
        print(f"unified_product_psci_shutdown_evidence.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
