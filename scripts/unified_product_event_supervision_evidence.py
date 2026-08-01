#!/usr/bin/env python3
"""Strict M73 event-supervision, maintenance, PSCI, and reboot evidence parser."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import unified_product_manifest_supervision_evidence as m72


FINAL_SCREEN_SHA256 = m72.FINAL_SCREEN_SHA256
PHASES = m72.PHASES
UI_MARKER = m72.UI_MARKER.replace("abi=33", "abi=34", 1)
UI_BOOT_MARKER = (
    "BOOT_OK: M73 unified real UI is interactive; event supervision and "
    "external maintenance controls are starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M73 event-driven five-service supervision, two clean storage "
    "rotations, in-flight drain, AppData, and PSCI shutdown armed"
)
DISCOVERY_MARKER = (
    "M73_PSCI_DISCOVERY_OK format=1 abi=34 node=/psci "
    "compatible=arm,psci-1.0 method=hvc psci_version=1.1 "
    "version_raw=0x00010001 version_probe=PSCI_VERSION "
    "version_function_id=0x84000000 system_off_function_id=0x84000008 "
    "fdt_validated=1 installed_once=1 semihosting=0 emulator_only=1 "
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0"
)
PSCI_SHUTDOWN_MARKER = m72.PSCI_SHUTDOWN_MARKER.replace(
    "abi=33", "abi=34", 1
)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M73_(?:FRAME|STACK|EXIT|SYSCALL|F5|UNFOCUSED)_TRACE(?: |$)|"
    r"qemu-semihosting",
    re.IGNORECASE | re.MULTILINE,
)

EVENT_PREFIX = "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK"
EVENT_KEYS = (
    "format",
    "abi",
    "authority",
    "manifest",
    "manifest_generation",
    "manifest_bytes",
    "manifest_fingerprint",
    "manifest_open_calls",
    "manifest_open_successes",
    "manifest_argument_rejections",
    "manifest_permission_denials",
    "services",
    "service_order",
    "dependency_edges",
    "hard_edges",
    "soft_edges",
    "protocol",
    "ui_query_calls",
    "ui_query_waits",
    "ui_query_successes",
    "ui_query_read_only",
    "event_driven",
    "multi_channel_wait",
    "continuous_until_power",
    "active_batches",
    "report_calls",
    "report_successes",
    "report_argument_rejections",
    "report_permission_denials",
    "report_state_rejections",
    "rotations_started",
    "rotations_completed",
    "clean_rotations",
    "old_pid_1",
    "new_pid_1",
    "old_pid_2",
    "new_pid_2",
    "same_slot",
    "generation_steps",
    "process_exit_faults",
    "process_terminate_calls",
    "process_terminate_successes",
    "terminated_exited",
    "terminated_killed",
    "backoff_waits",
    "backoff_ms",
    "healthy_stability_rounds",
    "restart_budget",
    "restart_budget_rearms",
    "dependent_blocks",
    "dependent_resumes",
    "cancel_batch",
    "cancel_pending",
    "stop_batch",
    "stop_pending",
    "drained_pending",
    "storage_epochs",
    "next_epoch",
    "releases",
    "injected_health_faults",
    "injected_process_kills",
    "proof_profile_rotations",
    "arbitrary_service_set_claim",
    "emulator_only",
    "general_runtime",
    "real_phone_claim",
)
DYNAMIC_EVENT_KEYS = {
    "ui_query_calls",
    "ui_query_waits",
    "active_batches",
    "cancel_batch",
    "stop_batch",
}
EVENT_EXPECTED = {
    "format": "1",
    "abi": "34",
    "authority": (
        "kernel-owned-bmf1-plus-init-event-loop-plus-kernel-authenticated-report"
    ),
    "manifest": "BMF1",
    "manifest_generation": "1",
    "manifest_bytes": "224",
    "manifest_fingerprint": "0xd69d11fdbc0fc7a9",
    "manifest_open_calls": "2",
    "manifest_open_successes": "1",
    "manifest_argument_rejections": "1",
    "manifest_permission_denials": "0",
    "services": "5",
    "service_order": (
        "App+ServiceManager+StorageServer+InputServer+SurfaceServer"
    ),
    "dependency_edges": "4",
    "hard_edges": "3",
    "soft_edges": "1",
    "protocol": "BSH1",
    "ui_query_successes": "1",
    "ui_query_read_only": "1",
    "event_driven": "1",
    "multi_channel_wait": "1",
    "continuous_until_power": "1",
    "report_calls": "9",
    "report_successes": "8",
    "report_argument_rejections": "1",
    "report_permission_denials": "0",
    "report_state_rejections": "0",
    "rotations_started": "2",
    "rotations_completed": "2",
    "clean_rotations": "2",
    "old_pid_1": "4294967306",
    "new_pid_1": "8589934602",
    "old_pid_2": "8589934602",
    "new_pid_2": "12884901898",
    "same_slot": "1",
    "generation_steps": "2",
    "process_exit_faults": "2",
    "process_terminate_calls": "0",
    "process_terminate_successes": "0",
    "terminated_exited": "12",
    "terminated_killed": "0",
    "backoff_waits": "2",
    "backoff_ms": "30",
    "healthy_stability_rounds": "2",
    "restart_budget": "1",
    "restart_budget_rearms": "2",
    "dependent_blocks": "2",
    "dependent_resumes": "2",
    "cancel_pending": "4",
    "stop_pending": "4",
    "drained_pending": "0",
    "storage_epochs": "3",
    "next_epoch": "4",
    "releases": "3",
    "injected_health_faults": "0",
    "injected_process_kills": "0",
    "proof_profile_rotations": "2",
    "arbitrary_service_set_claim": "0",
    "emulator_only": "1",
    "general_runtime": "0",
    "real_phone_claim": "0",
}


class EvidenceError(ValueError):
    pass


def parse_fields(
    line: str, prefix: str, expected_keys: tuple[str, ...] | None = None
) -> dict[str, str]:
    if not line.startswith(prefix + " "):
        raise EvidenceError(f"{prefix} marker prefix changed")
    keys: list[str] = []
    fields: dict[str, str] = {}
    for token in line[len(prefix) + 1 :].split():
        if token.count("=") != 1:
            raise EvidenceError(f"{prefix} contains a malformed field")
        key, value = token.split("=", 1)
        if not key or not value or key in fields:
            raise EvidenceError(f"{prefix} contains a duplicate or empty field")
        keys.append(key)
        fields[key] = value
    if expected_keys is not None and tuple(keys) != expected_keys:
        raise EvidenceError(f"{prefix} field order or schema changed")
    return fields


def decimal(fields: dict[str, str], key: str, prefix: str) -> int:
    value = fields[key]
    if not value.isdecimal():
        raise EvidenceError(f"{prefix} {key} is not an unsigned decimal")
    return int(value)


def unique_line(lines: list[str], prefix: str) -> str:
    matches = [line for line in lines if line == prefix or line.startswith(prefix + " ")]
    if len(matches) != 1:
        raise EvidenceError(
            f"{prefix} marker count is {len(matches)}, expected exactly one"
        )
    return matches[0]


def require_fixed(
    fields: dict[str, str],
    expected: dict[str, str],
    prefix: str,
) -> None:
    for key, value in expected.items():
        if fields.get(key) != value:
            raise EvidenceError(f"{prefix} field {key} changed")


def event_marker(
    *,
    ui_query_calls: int = 2,
    ui_query_waits: int = 1,
    active_batches: int = 3,
    cancel_batch: int = 5,
) -> str:
    values = dict(EVENT_EXPECTED)
    values.update(
        {
            "ui_query_calls": str(ui_query_calls),
            "ui_query_waits": str(ui_query_waits),
            "active_batches": str(active_batches),
            "cancel_batch": str(cancel_batch),
            "stop_batch": str(cancel_batch),
        }
    )
    return EVENT_PREFIX + " " + " ".join(
        f"{key}={values[key]}" for key in EVENT_KEYS
    )


def active_marker(batches: int = 3) -> str:
    return (
        "M73_EVENT_SUPERVISOR_ACTIVE_OK format=1 abi=34 "
        f"batches={batches} services=5 report_calls=2 report_successes=1 "
        "argument_rejections=1 event_driven=1 emulator_only=1 "
        "real_phone_claim=0"
    )


def rotation_marker(ordinal: int) -> str:
    if ordinal == 1:
        old_pid, new_pid = "4294967306", "8589934602"
    elif ordinal == 2:
        old_pid, new_pid = "8589934602", "12884901898"
    else:
        raise AssertionError("M73 has exactly two proof-profile rotations")
    return (
        "M73_STORAGE_ROTATION_OK format=1 abi=34 "
        f"ordinal={ordinal} old_pid={old_pid} new_pid={new_pid} "
        "same_slot=1 generation_step=1 clean_exit=1 process_terminate=0"
    )


def cancel_marker(batch: int = 5) -> str:
    return (
        "M73_CANCEL_WINDOW_OK format=1 abi=34 "
        f"batch={batch} pending=4 rotations=2 external_power=awaiting "
        "drain_after_stop=1"
    )


def shutdown_marker(phase: str) -> str:
    return (
        m72.shutdown_marker(phase)
        .replace("abi=33", "abi=34", 1)
        .replace("children_created=12", "children_created=13", 1)
        .replace("children_exited=11", "children_exited=12", 1)
        .replace("children_reaped=11", "children_reaped=12", 1)
    )


def validate_shutdown(line: str, phase: str) -> None:
    expected = parse_fields(shutdown_marker(phase), "UNIFIED_PRODUCT_SHUTDOWN_OK")
    observed = parse_fields(
        line,
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
        tuple(expected),
    )
    expected.pop("storage_server_submissions")
    require_fixed(observed, expected, "UNIFIED_PRODUCT_SHUTDOWN_OK")
    if decimal(
        observed,
        "storage_server_submissions",
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
    ) == 0:
        raise EvidenceError("M73 shutdown reported no storage submissions")


def validate_event(
    line: str,
    active_batches: int,
    cancel_batch: int,
) -> dict[str, int]:
    fields = parse_fields(line, EVENT_PREFIX, EVENT_KEYS)
    require_fixed(fields, EVENT_EXPECTED, EVENT_PREFIX)
    calls = decimal(fields, "ui_query_calls", EVENT_PREFIX)
    waits = decimal(fields, "ui_query_waits", EVENT_PREFIX)
    successes = decimal(fields, "ui_query_successes", EVENT_PREFIX)
    batches = decimal(fields, "active_batches", EVENT_PREFIX)
    reported_cancel = decimal(fields, "cancel_batch", EVENT_PREFIX)
    stopped = decimal(fields, "stop_batch", EVENT_PREFIX)
    if calls < 2 or waits < 1 or calls != waits + successes:
        raise EvidenceError("M73 UI convergence query accounting changed")
    if batches < 3 or batches != active_batches:
        raise EvidenceError("M73 active event-batch accounting changed")
    if reported_cancel != cancel_batch or stopped != cancel_batch:
        raise EvidenceError("M73 cancel/stop batch accounting changed")
    if cancel_batch <= active_batches:
        raise EvidenceError("M73 cancel window did not follow the active batch boundary")
    return {
        "ui_query_calls": calls,
        "ui_query_waits": waits,
        "active_batches": batches,
    }


def validate_text(text: str, phase: str) -> dict[str, int]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M73 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M73 BOOT_OK marker set changed")

    discovery = unique_line(lines, "M73_PSCI_DISCOVERY_OK")
    if discovery != DISCOVERY_MARKER:
        raise EvidenceError("M73 PSCI discovery marker changed")
    ui = unique_line(lines, "UNIFIED_PRODUCT_UI_OK")
    if ui != UI_MARKER:
        raise EvidenceError("M73 interactive UI marker changed")

    active = unique_line(lines, "M73_EVENT_SUPERVISOR_ACTIVE_OK")
    active_fields = parse_fields(
        active,
        "M73_EVENT_SUPERVISOR_ACTIVE_OK",
        (
            "format",
            "abi",
            "batches",
            "services",
            "report_calls",
            "report_successes",
            "argument_rejections",
            "event_driven",
            "emulator_only",
            "real_phone_claim",
        ),
    )
    require_fixed(
        active_fields,
        {
            "format": "1",
            "abi": "34",
            "services": "5",
            "report_calls": "2",
            "report_successes": "1",
            "argument_rejections": "1",
            "event_driven": "1",
            "emulator_only": "1",
            "real_phone_claim": "0",
        },
        "M73_EVENT_SUPERVISOR_ACTIVE_OK",
    )
    active_batches = decimal(
        active_fields, "batches", "M73_EVENT_SUPERVISOR_ACTIVE_OK"
    )
    if active_batches < 3:
        raise EvidenceError("M73 supervisor activated before three event batches")

    rotation_lines = [
        line for line in lines if line.startswith("M73_STORAGE_ROTATION_OK ")
    ]
    if len(rotation_lines) != 2:
        raise EvidenceError("M73 storage rotation marker count changed")
    if rotation_lines != [rotation_marker(1), rotation_marker(2)]:
        raise EvidenceError("M73 clean storage rotation chain changed")
    rotation_one = rotation_lines[0]
    rotation_two = rotation_lines[1]

    cancel = unique_line(lines, "M73_CANCEL_WINDOW_OK")
    cancel_fields = parse_fields(
        cancel,
        "M73_CANCEL_WINDOW_OK",
        (
            "format",
            "abi",
            "batch",
            "pending",
            "rotations",
            "external_power",
            "drain_after_stop",
        ),
    )
    require_fixed(
        cancel_fields,
        {
            "format": "1",
            "abi": "34",
            "pending": "4",
            "rotations": "2",
            "external_power": "awaiting",
            "drain_after_stop": "1",
        },
        "M73_CANCEL_WINDOW_OK",
    )
    cancel_batch = decimal(cancel_fields, "batch", "M73_CANCEL_WINDOW_OK")

    event = unique_line(lines, EVENT_PREFIX)
    metrics = validate_event(event, active_batches, cancel_batch)
    shutdown = unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK")
    validate_shutdown(shutdown, phase)
    psci = unique_line(lines, "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK")
    if psci != PSCI_SHUTDOWN_MARKER:
        raise EvidenceError("M73 PSCI shutdown marker changed")

    required_order = [
        lines.index(discovery),
        lines.index(ui),
        lines.index(UI_BOOT_MARKER),
        lines.index(active),
        lines.index(rotation_one),
        lines.index(rotation_two),
        lines.index(cancel),
        lines.index(event),
        lines.index(shutdown),
        lines.index(psci),
        lines.index(SHUTDOWN_BOOT_MARKER),
    ]
    if required_order != sorted(required_order) or len(set(required_order)) != len(
        required_order
    ):
        raise EvidenceError("M73 discovery/UI/event/rotation/drain/PSCI order changed")

    base = m72.m71.m70.m69.m68
    if base.exact_line(lines, "DATA_PERSIST_OK") != base.m67.data_marker(phase):
        raise EvidenceError("M73 DATA_PERSIST_OK marker changed")
    if (
        base.exact_line(lines, "STORAGE_DEVICE_HEALTH_OK")
        != base.m67.health_marker(phase)
    ):
        raise EvidenceError("M73 STORAGE_DEVICE_HEALTH_OK marker changed")
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
        base.require_exact(lines, checkpoint)
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
    return metrics


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={FINAL_SCREEN_SHA256} "
        "maintenance_rotations=2 cancel_pending=4"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M73 {phase} host QMP self-exit evidence changed")


def synthetic_log(phase: str) -> str:
    base = m72.synthetic_log(phase)
    replacement = "\n".join(
        (
            active_marker(),
            rotation_marker(1),
            rotation_marker(2),
            cancel_marker(),
            event_marker(),
        )
    )
    for previous, current in (
        (m72.DISCOVERY_MARKER, DISCOVERY_MARKER),
        (m72.UI_MARKER, UI_MARKER),
        (m72.UI_BOOT_MARKER, UI_BOOT_MARKER),
        (m72.MANIFEST_MARKER, replacement),
        (m72.shutdown_marker(phase), shutdown_marker(phase)),
        (m72.PSCI_SHUTDOWN_MARKER, PSCI_SHUTDOWN_MARKER),
        (m72.SHUTDOWN_BOOT_MARKER, SHUTDOWN_BOOT_MARKER),
    ):
        base = base.replace(previous, current, 1)
    return base


def parser_self_test() -> None:
    for phase in PHASES:
        validate_text(synthetic_log(phase), phase)
        validate_driver(driver_marker(phase) + "\n", phase)

    base = synthetic_log("first")
    reordered = base.replace(
        rotation_marker(1) + "\n" + rotation_marker(2),
        rotation_marker(2) + "\n" + rotation_marker(1),
        1,
    )
    mutations = (
        base.replace("abi=34", "abi=33", 1),
        base.replace("ui_query_calls=2", "ui_query_calls=1", 1),
        base.replace("ui_query_waits=1", "ui_query_waits=0", 1),
        base.replace("ui_query_read_only=1", "ui_query_read_only=0", 1),
        base.replace("event_driven=1", "event_driven=0", 1),
        base.replace("multi_channel_wait=1", "multi_channel_wait=0", 1),
        base.replace("continuous_until_power=1", "continuous_until_power=0", 1),
        base.replace("report_successes=8", "report_successes=7", 1),
        base.replace("rotations_completed=2", "rotations_completed=1", 1),
        base.replace("old_pid_2=8589934602", "old_pid_2=4294967306", 1),
        base.replace("process_terminate_calls=0", "process_terminate_calls=1", 1),
        base.replace("restart_budget_rearms=2", "restart_budget_rearms=1", 1),
        base.replace("cancel_pending=4", "cancel_pending=3", 1),
        base.replace("stop_batch=5", "stop_batch=4", 1),
        base.replace("drained_pending=0", "drained_pending=1", 1),
        base.replace("storage_epochs=3", "storage_epochs=2", 1),
        base.replace("injected_process_kills=0", "injected_process_kills=1", 1),
        base.replace(DISCOVERY_MARKER + "\n", "", 1),
        base.replace(active_marker() + "\n", "", 1),
        base.replace(rotation_marker(1) + "\n", "", 1),
        base.replace(cancel_marker() + "\n", "", 1),
        base.replace(event_marker() + "\n", "", 1),
        base.replace(PSCI_SHUTDOWN_MARKER + "\n", "", 1),
        base + event_marker() + "\n",
        reordered,
        base + "M73_SYSCALL_TRACE pid=1 syscall=56\n",
    )
    rejected = 0
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except (
            EvidenceError,
            m72.EvidenceError,
            m72.m71.EvidenceError,
            m72.m71.m70.EvidenceError,
            m72.m71.m70.m69.EvidenceError,
            m72.m71.m70.m69.m68.EvidenceError,
        ):
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M73 parser negative self-test accepted a mutation")

    bad_drivers = (
        driver_marker("first").replace("qemu_self_exit=1", "qemu_self_exit=0"),
        driver_marker("first").replace(
            "maintenance_rotations=2", "maintenance_rotations=1"
        ),
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
        raise EvidenceError("M73 host-exit parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK "
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
    first = validate_text(
        first_serial_path.read_text(encoding="utf-8", errors="replace"),
        "first",
    )
    second = validate_text(
        second_serial_path.read_text(encoding="utf-8", errors="replace"),
        "second",
    )
    validate_driver(
        first_driver_path.read_text(encoding="utf-8", errors="replace"),
        "first",
    )
    validate_driver(
        second_driver_path.read_text(encoding="utf-8", errors="replace"),
        "second",
    )
    marker = m72.m71.m70.m69.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared M69 disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_REBOOT_OK",
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
    event = (
        "product_shutdowns=2 event_supervision_sessions=2 manifests_decoded=2 "
        "manifest_open_calls=4 manifest_open_successes=2 "
        "manifest_argument_rejections=2 manifest_permission_denials=0 "
        "supervised_services=10 liveness_dependency_edges=8 "
        f"ui_query_calls={first['ui_query_calls'] + second['ui_query_calls']} "
        f"ui_query_waits={first['ui_query_waits'] + second['ui_query_waits']} "
        "ui_query_successes=2 "
        f"event_batches={first['active_batches'] + second['active_batches']} "
        "report_calls=18 report_successes=16 report_argument_rejections=2 "
        "report_permission_denials=0 report_state_rejections=0 "
        "clean_rotations=4 process_exit_faults=4 process_terminate_calls=0 "
        "terminated_exited=24 terminated_killed=0 backoff_waits=4 "
        "restart_budget_rearms=4 dependent_blocks=4 dependent_resumes=4 "
        "cancel_windows=2 cancelled_pending=8 drained_pending=0 "
        "storage_epochs=6 psci_discoveries=2 psci_version_probes=2 "
        "psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 "
        "psci_system_off_requests=2 qemu_psci_self_exits=2 "
        "semihosting_uses=0 "
    )
    if historical not in marker:
        raise EvidenceError("shared M69 aggregate fields changed")
    marker = marker.replace(historical, event, 1)
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
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.phase,
            )
            print(
                "UNIFIED_PRODUCT_EVENT_SUPERVISION_EVIDENCE_OK "
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
        m72.EvidenceError,
        m72.m71.EvidenceError,
        m72.m71.m70.EvidenceError,
        m72.m71.m70.m69.EvidenceError,
        m72.m71.m70.m69.m68.EvidenceError,
        m72.m71.m70.m69.m68.m67.EvidenceError,
        OSError,
    ) as error:
        print(
            f"unified_product_event_supervision_evidence.py: {error}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
