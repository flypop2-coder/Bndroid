#!/usr/bin/env python3
"""Strict M69 two-service liveness serial and two-boot disk evidence parser."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import unified_product_liveness_evidence as m68


FINAL_SCREEN_SHA256 = m68.FINAL_SCREEN_SHA256
PHASES = m68.PHASES
UI_MARKER = (
    "UNIFIED_PRODUCT_UI_OK format=1 abi=30 profile=m45-real-ui "
    "input_server_pid=4294967302 input_session=1 windows=2 "
    "physical=pointer47+key12 graphics=21/21/21/21 text=complete "
    "soft_keyboard=complete processes=10/1/1/9 process_capacity=10/9 "
    "handles=36 endpoints=30 pairs=15 input_pairs=2 waits=9/2/7 "
    "ui_converged=1 init_ready=0 storage_spawned=0 power_key_code=116 "
    "power_request=awaiting emulator_only=1 hardware_poweroff_claim=0 "
    "real_phone_claim=0"
)
UI_BOOT_MARKER = (
    "BOOT_OK: M69 unified real UI is interactive and awaiting authenticated power key"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M69 unified two-service dependency liveness, AppData, bounded "
    "shutdown, and QEMU exit armed"
)
LIVENESS_MARKER = (
    "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK format=1 abi=30 "
    "authority=init-bsh1-plus-kernel-authenticated-pid services=2 "
    "service_set=StorageServer+App protocol=BSH1 dependency=StorageServer->App "
    "dependency_kind=hard probes=8 healthy=7 withheld=1 cadence_waits=3 "
    "cadence_ms=40 periodic_rounds=3 health_timeouts=1 health_timeout_ms=100 "
    "backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 "
    "old_pid=4294967306 replacement_pid=8589934602 app_pid=4294967305 "
    "same_slot=1 generation_step=1 process_terminate_calls=1 "
    "process_terminate_successes=1 terminated_killed=1 storage_epochs=2 "
    "next_epoch=3 releases=2 fault_transitions=2 recovery_transitions=2 "
    "dependent_blocks=1 dependent_resumes=1 app_health_sequences=3 "
    "replacement_health_sequences=2 replacement_mounted=1 "
    "replacement_healthy=1 bounded=1 single_dependency=1 injected_hang=1 "
    "emulator_only=1 general_runtime=0 real_phone_claim=0"
)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M69_(?:FRAME|STACK|EXIT|SYSCALL)_TRACE(?: |$)",
    re.IGNORECASE | re.MULTILINE,
)


class EvidenceError(ValueError):
    pass


def shutdown_marker(phase: str) -> str:
    return m68.shutdown_marker(phase).replace("abi=29", "abi=30", 1)


def validate_text(text: str, phase: str) -> None:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M69 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M69 BOOT_OK marker set changed")

    if m68.exact_line(lines, "DATA_PERSIST_OK") != m68.m67.data_marker(phase):
        raise EvidenceError("M69 DATA_PERSIST_OK marker changed")
    if (
        m68.exact_line(lines, "STORAGE_DEVICE_HEALTH_OK")
        != m68.m67.health_marker(phase)
    ):
        raise EvidenceError("M69 STORAGE_DEVICE_HEALTH_OK marker changed")
    if m68.exact_line(lines, "UNIFIED_PRODUCT_UI_OK") != UI_MARKER:
        raise EvidenceError("M69 interactive UI marker changed")
    if (
        m68.exact_line(lines, "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK")
        != LIVENESS_MARKER
    ):
        raise EvidenceError("M69 two-service liveness marker changed")
    if (
        m68.exact_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK")
        != shutdown_marker(phase)
    ):
        raise EvidenceError("M69 shutdown marker changed")
    m68.require_exact(lines, UI_BOOT_MARKER)
    m68.require_exact(lines, SHUTDOWN_BOOT_MARKER)

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
        m68.require_exact(lines, checkpoint)
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


def synthetic_log(phase: str) -> str:
    lines = [
        m68.m67.data_marker(phase),
        m68.m67.health_marker(phase),
        "WINDOW_INPUT_PHASE1_READY",
        "WINDOW_INPUT_PHASE2_READY",
        "PERSISTENT_WINDOW_INPUT_READY",
        "TEXT_POINTER_FOCUS_READY",
        "TEXT_INPUT_READY",
    ]
    lines.extend(f"TEXT_FIELD_RENDER_OK synthetic={index}" for index in range(1, 6))
    lines.extend(
        (
            "TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0",
            "TEXT_INPUT_OK synthetic=1",
            "SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6",
            "SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible",
        )
    )
    lines.extend(f"SOFT_TEXT_RENDER_OK synthetic={index}" for index in range(1, 6))
    lines.extend(
        (
            "SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent",
            UI_MARKER,
            UI_BOOT_MARKER,
            LIVENESS_MARKER,
            shutdown_marker(phase),
            SHUTDOWN_BOOT_MARKER,
        )
    )
    return "\n".join(lines) + "\n"


def parser_self_test() -> None:
    for phase in PHASES:
        validate_text(synthetic_log(phase), phase)

    base = synthetic_log("first")
    mutations = (
        base.replace("abi=30", "abi=29", 1),
        base.replace("services=2", "services=1", 1),
        base.replace("probes=8", "probes=7", 1),
        base.replace("healthy=7", "healthy=6", 1),
        base.replace("dependent_blocks=1", "dependent_blocks=0", 1),
        base.replace("dependent_resumes=1", "dependent_resumes=0", 1),
        base.replace("app_health_sequences=3", "app_health_sequences=2", 1),
        base.replace(
            "replacement_pid=8589934602", "replacement_pid=4294967306", 1
        ),
        base.replace(LIVENESS_MARKER + "\n", "", 1),
        base + "M69_SYSCALL_TRACE pid=1 syscall=1\n",
    )
    rejected = 0
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except (EvidenceError, m68.EvidenceError):
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M69 parser negative self-test accepted a mutation")
    print(
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES)} negative={rejected}"
    )


def verify_disk(pristine_path: Path, runtime_path: Path) -> str:
    marker = m68.m67.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_REBOOT_OK",
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
        1,
    )
    return marker.replace(
        "product_shutdowns=2 ",
        "product_shutdowns=2 multiservice_liveness_recoveries=2 "
        "supervised_services=4 liveness_dependency_edges=2 health_probes=16 "
        "healthy=14 withheld=2 health_timeouts=2 cadence_waits=6 "
        "dependency_fault_transitions=4 dependency_recovery_transitions=4 "
        "dependent_blocks=2 dependent_resumes=2 killed_servers=2 "
        "replacements=2 replacement_healthy=2 storage_epochs=4 ",
        1,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    parse = subparsers.add_parser("parse")
    parse.add_argument("phase", choices=tuple(PHASES))
    parse.add_argument("log", type=Path)
    disk = subparsers.add_parser("disk")
    disk.add_argument("pristine", type=Path)
    disk.add_argument("runtime", type=Path)
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
                "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_EVIDENCE_OK "
                f"phase={args.phase}"
            )
        elif args.command == "disk":
            print(verify_disk(args.pristine, args.runtime))
        else:
            raise AssertionError("unreachable command")
    except (EvidenceError, m68.EvidenceError, m68.m67.EvidenceError, OSError) as error:
        print(
            f"unified_product_multiservice_liveness_evidence.py: {error}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
