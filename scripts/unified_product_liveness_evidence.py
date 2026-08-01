#!/usr/bin/env python3
"""Strict M68 product-liveness serial and two-boot disk evidence parser."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import unified_product_evidence as m67


FINAL_SCREEN_SHA256 = (
    "1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
)
UI_MARKER = (
    "UNIFIED_PRODUCT_UI_OK format=1 abi=29 profile=m45-real-ui "
    "input_server_pid=4294967302 input_session=1 windows=2 "
    "physical=pointer47+key12 graphics=21/21/21/21 text=complete "
    "soft_keyboard=complete processes=10/1/1/9 process_capacity=10/9 "
    "handles=36 endpoints=30 pairs=15 input_pairs=2 waits=9/2/7 "
    "ui_converged=1 init_ready=0 storage_spawned=0 power_key_code=116 "
    "power_request=awaiting emulator_only=1 hardware_poweroff_claim=0 "
    "real_phone_claim=0"
)
UI_BOOT_MARKER = (
    "BOOT_OK: M68 unified real UI is interactive and awaiting authenticated power key"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M68 unified product liveness recovery, AppData, bounded shutdown, "
    "and QEMU exit armed"
)
LIVENESS_MARKER = (
    "UNIFIED_PRODUCT_LIVENESS_OK format=1 abi=29 "
    "authority=init-bsh1-plus-kernel-authenticated-pid service=StorageServer "
    "protocol=BSH1 probes=3 healthy=2 withheld=1 health_timeouts=1 "
    "health_timeout_ms=100 backoff_waits=1 backoff_ms=30 restart_budget=1 "
    "restarts=1 old_pid=4294967306 replacement_pid=8589934602 same_slot=1 "
    "generation_step=1 process_terminate_calls=1 process_terminate_successes=1 "
    "terminated_killed=1 storage_epochs=2 next_epoch=3 releases=2 "
    "replacement_mounted=1 replacement_healthy=1 bounded=1 single_service=1 "
    "injected_hang=1 emulator_only=1 general_runtime=0 real_phone_claim=0"
)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M68_(?:FRAME|STACK|EXIT|SYSCALL)_TRACE(?: |$)",
    re.IGNORECASE | re.MULTILINE,
)

PHASES = {
    "first": {
        "appdata_generation": 5,
        "open_generation": 1,
        "closed_generation": 2,
        "storage_server_submissions": 1953,
    },
    "second": {
        "appdata_generation": 6,
        "open_generation": 3,
        "closed_generation": 4,
        "storage_server_submissions": 735,
    },
}


class EvidenceError(ValueError):
    pass


def shutdown_marker(phase: str) -> str:
    values = PHASES[phase]
    return (
        "UNIFIED_PRODUCT_SHUTDOWN_OK format=1 abi=29 "
        "authority=authenticated-power-key-plus-init-resident-graph-plus-kernel-seal "
        "interactive_ui=1 appdata=1 bounded_shutdown=1 ui_converged=1 "
        "power_key_code=116 "
        f"appdata_generation={values['appdata_generation']} "
        f"open_generation={values['open_generation']} "
        f"closed_generation={values['closed_generation']} "
        "open_slot=1 closed_slot=0 prior_boot_open=1 current_boot_open=0 "
        "contract_matched=1 health_reads=5 health_writes=1 health_flushes=1 "
        "resident_nodes=8 dependency_edges=10 quiesce_waves=3 "
        "registered_mask=0xff quiesced_mask=0xff registration_calls=9 "
        "quiesce_calls=9 order_rejections=1 children_created=12 "
        "children_exited=11 children_reaped=11 live_processes=1 "
        "dynamic_contexts=0 dynamic_stacks=0 prepare_calls=4 prepares=1 "
        "commit_calls=1 commits=1 permission_denied=2 early_rejections=1 "
        "spawn_rejections=1 connect_rejections=2 broker_bound=0 "
        "broker_pending=0 broker_state=0 "
        f"storage_server_submissions={values['storage_server_submissions']} "
        "admission_closed=1 recovery_required=0 irq_armed=0 irq_failed=0 "
        "in_flight=0 requests_terminal=1 post_close_storage_mutations=0 "
        "storage_server_flushed=1 storage_server_readback=1 "
        "storage_server_exited=1 emulator_backend=qemu-semihosting "
        "emulator_exit_requested=1 emulator_exit_verified_by_host=0 "
        "hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 "
        "general_runtime=0 real_phone_claim=0"
    )


def exact_line(lines: list[str], prefix: str) -> str:
    matches = [line for line in lines if line == prefix or line.startswith(prefix + " ")]
    if len(matches) != 1:
        raise EvidenceError(f"{prefix!r} count is {len(matches)}, expected one")
    return matches[0]


def require_exact(lines: list[str], marker: str) -> None:
    if lines.count(marker) != 1:
        raise EvidenceError(f"exact marker count is not one: {marker!r}")


def validate_text(text: str, phase: str) -> None:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M68 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M68 BOOT_OK marker set changed")

    if exact_line(lines, "DATA_PERSIST_OK") != m67.data_marker(phase):
        raise EvidenceError("M68 DATA_PERSIST_OK marker changed")
    if exact_line(lines, "STORAGE_DEVICE_HEALTH_OK") != m67.health_marker(phase):
        raise EvidenceError("M68 STORAGE_DEVICE_HEALTH_OK marker changed")
    if exact_line(lines, "UNIFIED_PRODUCT_UI_OK") != UI_MARKER:
        raise EvidenceError("M68 interactive UI marker changed")
    if exact_line(lines, "UNIFIED_PRODUCT_LIVENESS_OK") != LIVENESS_MARKER:
        raise EvidenceError("M68 liveness marker changed")
    if exact_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK") != shutdown_marker(phase):
        raise EvidenceError("M68 shutdown marker changed")
    require_exact(lines, UI_BOOT_MARKER)
    require_exact(lines, SHUTDOWN_BOOT_MARKER)

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
        require_exact(lines, checkpoint)
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
        m67.data_marker(phase),
        m67.health_marker(phase),
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
        base.replace("abi=29", "abi=28", 1),
        base.replace("probes=3", "probes=2", 1),
        base + LIVENESS_MARKER + "\n",
        base.replace("WINDOW_INPUT_PHASE2_READY\n", "", 1),
        base.replace("real_phone_claim=0", "real_phone_claim=1", 1),
        base + "M68_SYSCALL_TRACE pid=1 syscall=1\n",
        base.replace("replacement_pid=8589934602", "replacement_pid=4294967306", 1),
        base.replace(LIVENESS_MARKER + "\n", "", 1),
    )
    rejected = 0
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except EvidenceError:
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M68 parser negative self-test accepted a mutation")
    print(
        "UNIFIED_PRODUCT_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES)} negative={rejected}"
    )


def verify_disk(pristine_path: Path, runtime_path: Path) -> str:
    marker = m67.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_REBOOT_OK",
        "UNIFIED_PRODUCT_LIVENESS_REBOOT_OK",
        1,
    )
    return marker.replace(
        "product_shutdowns=2 ",
        "product_shutdowns=2 storage_liveness_recoveries=2 health_probes=6 "
        "healthy=4 health_timeouts=2 backoff_waits=2 killed_servers=2 "
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
            validate_text(args.log.read_text(encoding="utf-8", errors="replace"), args.phase)
            print(f"UNIFIED_PRODUCT_LIVENESS_EVIDENCE_OK phase={args.phase}")
        elif args.command == "disk":
            print(verify_disk(args.pristine, args.runtime))
        else:
            raise AssertionError("unreachable command")
    except (EvidenceError, m67.EvidenceError, OSError) as error:
        print(f"unified_product_liveness_evidence.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
