#!/usr/bin/env python3
"""Strict M72 manifest-supervision, PSCI, host-exit, and reboot evidence parser."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import unified_product_continuous_supervision_evidence as m71


FINAL_SCREEN_SHA256 = m71.FINAL_SCREEN_SHA256
PHASES = m71.PHASES
UI_MARKER = m71.UI_MARKER.replace("abi=32", "abi=33", 1)
UI_BOOT_MARKER = (
    "BOOT_OK: M72 unified real UI is interactive and awaiting authenticated power key"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M72 kernel-manifest service discovery, five-service supervision, "
    "AppData, and PSCI shutdown armed"
)
MANIFEST_MARKER = (
    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK format=1 abi=33 "
    "authority=kernel-owned-immutable-bmf1-vmo-plus-init-transactional-binding-"
    "plus-kernel-authenticated-pid manifest=BMF1 manifest_generation=1 "
    "manifest_bytes=224 manifest_fingerprint=0xd69d11fdbc0fc7a9 "
    "manifest_open_calls=2 manifest_open_successes=1 "
    "manifest_argument_rejections=1 manifest_permission_denials=0 "
    "parser_transactional=1 bounded_service_capacity=5 "
    "bounded_dependency_capacity=10 services=5 "
    "service_order=App+ServiceManager+StorageServer+InputServer+SurfaceServer "
    "legacy_literal_order_independent=1 resident_bindings=4 manifest_spawns=1 "
    "service_set=ServiceManager+SurfaceServer+InputServer+StorageServer+App "
    "protocol=BSH1 dependency_edges=4 hard_edges=3 soft_edges=1 probes=107 "
    "healthy=104 missed=3 cadence_waits=21 cadence_ms=40 periodic_rounds=21 "
    "fully_healthy_rounds=20 healthy_soak_rounds=16 batch_rounds=18 "
    "batched_probes=90 health_timeouts=3 health_timeout_ms=100 "
    "concurrent_miss_windows=1 concurrent_miss_services=2 "
    "missed_probe_tolerance=1 transient_miss_recoveries=2 escalated_faults=1 "
    "backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 "
    "old_pid=4294967306 replacement_pid=8589934602 app_pid=4294967305 "
    "same_slot=1 generation_step=1 process_terminate_calls=1 "
    "process_terminate_successes=1 terminated_killed=1 storage_epochs=2 "
    "next_epoch=3 releases=2 fault_transitions=2 recovery_transitions=2 "
    "tolerated_miss_dependency_transitions=0 dependent_blocks=1 "
    "dependent_resumes=1 resident_health_sequences=21 app_health_sequences=21 "
    "replacement_health_sequences=20 replacement_mounted=1 "
    "replacement_healthy=1 elapsed_supervision_ms=1070 bounded=1 "
    "arbitrary_service_set_claim=0 injected_storage_hang=1 "
    "injected_transient_misses=1 emulator_only=1 general_runtime=0 "
    "real_phone_claim=0"
)
DISCOVERY_MARKER = (
    "M72_PSCI_DISCOVERY_OK format=1 abi=33 node=/psci "
    "compatible=arm,psci-1.0 method=hvc psci_version=1.1 "
    "version_raw=0x00010001 version_probe=PSCI_VERSION "
    "version_function_id=0x84000000 system_off_function_id=0x84000008 "
    "fdt_validated=1 installed_once=1 semihosting=0 emulator_only=1 "
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0"
)
PSCI_SHUTDOWN_MARKER = m71.PSCI_SHUTDOWN_MARKER.replace("abi=32", "abi=33", 1)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M72_(?:FRAME|STACK|EXIT|SYSCALL)_TRACE(?: |$)|qemu-semihosting",
    re.IGNORECASE | re.MULTILINE,
)


class EvidenceError(ValueError):
    pass


def shutdown_marker(phase: str) -> str:
    return m71.shutdown_marker(phase).replace("abi=32", "abi=33", 1)


def validate_text(text: str, phase: str) -> None:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M72 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M72 BOOT_OK marker set changed")

    exact_markers = (
        ("M72_PSCI_DISCOVERY_OK", DISCOVERY_MARKER),
        ("UNIFIED_PRODUCT_UI_OK", UI_MARKER),
        ("UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK", MANIFEST_MARKER),
        ("UNIFIED_PRODUCT_SHUTDOWN_OK", shutdown_marker(phase)),
        ("UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK", PSCI_SHUTDOWN_MARKER),
    )
    positions: list[int] = []
    for prefix, expected in exact_markers:
        observed = m71.m70.m69.m68.exact_line(lines, prefix)
        if observed != expected:
            raise EvidenceError(f"M72 {prefix} marker changed")
        positions.append(lines.index(observed))
    m71.m70.m69.m68.require_exact(lines, UI_BOOT_MARKER)
    m71.m70.m69.m68.require_exact(lines, SHUTDOWN_BOOT_MARKER)
    expected_order = [
        positions[0],
        positions[1],
        lines.index(UI_BOOT_MARKER),
        positions[2],
        positions[3],
        positions[4],
        lines.index(SHUTDOWN_BOOT_MARKER),
    ]
    if expected_order != sorted(expected_order) or len(set(expected_order)) != len(
        expected_order
    ):
        raise EvidenceError("M72 discovery/UI/manifest/seal/PSCI marker order changed")

    if (
        m71.m70.m69.m68.exact_line(lines, "DATA_PERSIST_OK")
        != m71.m70.m69.m68.m67.data_marker(phase)
    ):
        raise EvidenceError("M72 DATA_PERSIST_OK marker changed")
    if (
        m71.m70.m69.m68.exact_line(lines, "STORAGE_DEVICE_HEALTH_OK")
        != m71.m70.m69.m68.m67.health_marker(phase)
    ):
        raise EvidenceError("M72 STORAGE_DEVICE_HEALTH_OK marker changed")

    for checkpoint in (
        "WINDOW_INPUT_PHASE1_READY",
        "WINDOW_INPUT_PHASE2_READY",
        "PERSISTENT_WINDOW_INPUT_READY",
        "TEXT_POINTER_FOCUS_READY",
        "TEXT_INPUT_READY",
        "TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0",
        "SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6",
        "SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible",
        "SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent",
    ):
        m71.m70.m69.m68.require_exact(lines, checkpoint)
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
        raise EvidenceError(f"M72 {phase} host QMP self-exit evidence changed")


def synthetic_log(phase: str) -> str:
    base = m71.synthetic_log(phase)
    for previous, current in (
        (m71.DISCOVERY_MARKER, DISCOVERY_MARKER),
        (m71.UI_MARKER, UI_MARKER),
        (m71.UI_BOOT_MARKER, UI_BOOT_MARKER),
        (m71.CONTINUOUS_MARKER, MANIFEST_MARKER),
        (m71.shutdown_marker(phase), shutdown_marker(phase)),
        (m71.PSCI_SHUTDOWN_MARKER, PSCI_SHUTDOWN_MARKER),
        (m71.SHUTDOWN_BOOT_MARKER, SHUTDOWN_BOOT_MARKER),
    ):
        base = base.replace(previous, current, 1)
    return base


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
        base.replace("abi=33", "abi=32", 1),
        base.replace("manifest=BMF1", "manifest=BMF0", 1),
        base.replace("manifest_generation=1", "manifest_generation=0", 1),
        base.replace("manifest_bytes=224", "manifest_bytes=223", 1),
        base.replace("manifest_fingerprint=0xd69d11fdbc0fc7a9", "manifest_fingerprint=0x0", 1),
        base.replace("manifest_open_calls=2", "manifest_open_calls=1", 1),
        base.replace("manifest_open_successes=1", "manifest_open_successes=0", 1),
        base.replace("manifest_argument_rejections=1", "manifest_argument_rejections=0", 1),
        base.replace("parser_transactional=1", "parser_transactional=0", 1),
        base.replace("bounded_dependency_capacity=10", "bounded_dependency_capacity=4", 1),
        base.replace(
            "service_order=App+ServiceManager+StorageServer+InputServer+SurfaceServer",
            "service_order=ServiceManager+SurfaceServer+InputServer+StorageServer+App",
            1,
        ),
        base.replace("legacy_literal_order_independent=1", "legacy_literal_order_independent=0", 1),
        base.replace("resident_bindings=4", "resident_bindings=3", 1),
        base.replace("manifest_spawns=1", "manifest_spawns=0", 1),
        base.replace("dependency_edges=4", "dependency_edges=3", 1),
        base.replace("probes=107", "probes=106", 1),
        base.replace("healthy=104", "healthy=103", 1),
        base.replace("concurrent_miss_services=2", "concurrent_miss_services=1", 1),
        base.replace("transient_miss_recoveries=2", "transient_miss_recoveries=1", 1),
        base.replace("arbitrary_service_set_claim=0", "arbitrary_service_set_claim=1", 1),
        base.replace(DISCOVERY_MARKER + "\n", "", 1),
        base.replace(MANIFEST_MARKER + "\n", "", 1),
        base.replace(PSCI_SHUTDOWN_MARKER + "\n", "", 1),
        base + MANIFEST_MARKER + "\n",
        reordered,
        base + "M72_SYSCALL_TRACE pid=1 syscall=55\n",
    )
    rejected = 0
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except (
            EvidenceError,
            m71.EvidenceError,
            m71.m70.EvidenceError,
            m71.m70.m69.EvidenceError,
            m71.m70.m69.m68.EvidenceError,
        ):
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M72 parser negative self-test accepted a mutation")

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
        raise EvidenceError("M72 host-exit parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK "
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
    marker = m71.m70.m69.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared M69 disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
        "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_REBOOT_OK",
        1,
    )
    historical = (
        "product_shutdowns=2 multiservice_liveness_recoveries=2 "
        "supervised_services=4 liveness_dependency_edges=2 health_probes=16 "
        "healthy=14 withheld=2 health_timeouts=2 cadence_waits=6 "
        "dependency_fault_transitions=4 dependency_recovery_transitions=4 "
        "dependent_blocks=2 dependent_resumes=2 killed_servers=2 "
        "replacements=2 replacement_healthy=2 storage_epochs=4 "
    )
    manifest = (
        "product_shutdowns=2 manifest_supervision_recoveries=2 manifests_decoded=2 "
        "manifest_open_calls=4 manifest_open_successes=2 "
        "manifest_argument_rejections=2 manifest_permission_denials=0 "
        "manifest_services=10 resident_bindings=8 manifest_spawns=2 "
        "supervised_services=10 liveness_dependency_edges=8 health_probes=214 "
        "healthy=208 missed=6 health_timeouts=6 cadence_waits=42 "
        "periodic_rounds=42 fully_healthy_rounds=40 healthy_soak_rounds=32 "
        "batch_rounds=36 batched_probes=180 concurrent_miss_windows=2 "
        "concurrent_miss_services=4 transient_miss_recoveries=4 "
        "escalated_faults=2 dependency_fault_transitions=4 "
        "dependency_recovery_transitions=4 dependent_blocks=2 "
        "dependent_resumes=2 killed_servers=2 replacements=2 "
        "replacement_healthy=2 storage_epochs=4 psci_discoveries=2 "
        "psci_version_probes=2 psci_version_1_1=2 fdt_psci_nodes=2 "
        "hvc_conduits=2 psci_system_off_requests=2 qemu_psci_self_exits=2 "
        "semihosting_uses=0 "
    )
    if historical not in marker:
        raise EvidenceError("shared M69 aggregate fields changed")
    marker = marker.replace(historical, manifest, 1)
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
            print(
                "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_EVIDENCE_OK "
                f"phase={args.phase}"
            )
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
        m71.EvidenceError,
        m71.m70.EvidenceError,
        m71.m70.m69.EvidenceError,
        m71.m70.m69.m68.EvidenceError,
        m71.m70.m69.m68.m67.EvidenceError,
        OSError,
    ) as error:
        print(
            f"unified_product_manifest_supervision_evidence.py: {error}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
