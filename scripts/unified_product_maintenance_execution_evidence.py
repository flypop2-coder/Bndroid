#!/usr/bin/env python3
"""Strict M78 maintenance execution, interruption, and disk-ledger evidence."""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import sys
from pathlib import Path

import build_storage_image as fixture
import unified_product_event_supervision_evidence as m73
import unified_product_key_rotation_evidence as m76
import unified_product_maintenance_authorization_evidence as m77


PHASES = ("sequence1", "resume")
NEGATIVE_REASONS = ("signature", "binding", "completed-replay")
UI_BOOT_MARKER = (
    "BOOT_OK: M78 unified real UI is interactive; recoverable maintenance "
    "execution is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M78 durable maintenance execution completion, exact "
    "interrupted-session recovery, AppData, and PSCI shutdown armed"
)
AUDIT_PREFIX = "MAINTENANCE_AUDIT_OK"
ADMISSION_PREFIX = "MAINTENANCE_EXECUTION_ADMISSION_OK"
COMMIT_PREFIX = "MAINTENANCE_EXECUTION_COMMIT_OK"
AUTHORIZATION_PREFIX = "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK"
FINAL_PREFIX = "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_OK"

ADMISSION_KEYS = (
    "format",
    "audit_state",
    "execution_state",
    "audit_relative_lbas",
    "execution_relative_lbas",
    "sequence",
    "resumed",
    "completed_sequence_before",
    "preflight_reads",
    "audit_reads",
    "audit_writes",
    "audit_flushes",
    "execution_initial_generation",
    "execution_initial_slot",
    "execution_initial_valid_slots",
    "execution_initial_rejected_slots",
    "exact_binding",
    "predecessor_complete",
    "completed_replay",
    "audit_write_flush_readback",
    "signature_valid",
    "binding_valid",
    "manifest_published",
    "init_ready",
    "exactly_once_claim",
    "trusted_monotonic_backend",
    "rpmb_claim",
    "efuse_claim",
    "hsm_claim",
    "host_rollback_resistance",
    "hardware_powercut_claim",
    "emulator_only",
)
ADMISSION_FIXED = {
    "format": "1",
    "audit_state": "BNDRMAU1",
    "execution_state": "BNDRMEX1",
    "audit_relative_lbas": "5/6",
    "execution_relative_lbas": "7/8",
    "preflight_reads": "4",
    "exact_binding": "1",
    "predecessor_complete": "1",
    "completed_replay": "0",
    "signature_valid": "1",
    "binding_valid": "1",
    "manifest_published": "0",
    "init_ready": "0",
    "exactly_once_claim": "0",
    "trusted_monotonic_backend": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "hsm_claim": "0",
    "host_rollback_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
}

COMMIT_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "operations",
    "max_uses",
    "uses_consumed",
    "rotations_completed",
    "stop_drained",
    "clean_shutdown_validated",
    "appdata_generation",
    "provisioned_before",
    "previous_sequence",
    "committed_sequence",
    "previous_completed_count",
    "committed_completed_count",
    "initial_generation",
    "committed_generation",
    "initial_slot",
    "committed_slot",
    "initial_valid_slots",
    "initial_rejected_slots",
    "committed_valid_slots",
    "committed_rejected_slots",
    "reads",
    "writes",
    "flushes",
    "runtime_evidence_sha256",
    "chain_sha256",
    "audit_chain_bound",
    "old_slot_preserved",
    "write_flush_readback",
    "resident_state_preserved",
    "exactly_once_claim",
    "trusted_monotonic_backend",
    "rpmb_claim",
    "efuse_claim",
    "hsm_claim",
    "host_rollback_resistance",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
COMMIT_FIXED = {
    "format": "1",
    "state": "BNDRMEX1",
    "state_version": "1",
    "authority": "kernel-validated-resident-runtime-plus-qemu-disk",
    "slots": "2",
    "relative_lbas": "7/8",
    "operations": "1",
    "max_uses": "2",
    "uses_consumed": "2",
    "rotations_completed": "2",
    "stop_drained": "1",
    "clean_shutdown_validated": "1",
    "reads": "6",
    "writes": "1",
    "flushes": "1",
    "audit_chain_bound": "1",
    "old_slot_preserved": "1",
    "write_flush_readback": "1",
    "resident_state_preserved": "1",
    "exactly_once_claim": "0",
    "trusted_monotonic_backend": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "hsm_claim": "0",
    "host_rollback_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

AUTHORIZATION_KEYS = (
    m77.FINAL_KEYS[:18]
    + ("admission_resumed", "completed_sequence_before", "preflight_reads")
    + m77.FINAL_KEYS[18:]
)

FINAL_KEYS = (
    "format",
    "abi",
    "artifact",
    "audit_state",
    "audit_state_version",
    "audit_relative_lbas",
    "execution_state",
    "execution_state_version",
    "execution_relative_lbas",
    "sequence",
    "operations",
    "operation_storage_rotation",
    "max_uses",
    "admission_resumed",
    "completed_sequence_before",
    "audit_preflight_reads",
    "audit_reads",
    "audit_writes",
    "audit_flushes",
    "exact_unfinished_head",
    "execution_provisioned_before",
    "previous_execution_sequence",
    "committed_execution_sequence",
    "previous_completed_count",
    "committed_completed_count",
    "execution_initial_generation",
    "execution_committed_generation",
    "execution_initial_slot",
    "execution_committed_slot",
    "execution_initial_valid_slots",
    "execution_initial_rejected_slots",
    "execution_committed_valid_slots",
    "execution_committed_rejected_slots",
    "execution_reads",
    "execution_writes",
    "execution_flushes",
    "uses_consumed",
    "rotations_completed",
    "stop_drained",
    "clean_shutdown_validated",
    "appdata_generation",
    "runtime_evidence_sha256",
    "audit_chain_sha256",
    "execution_chain_sha256",
    "completed_replay_rejected_by_admission",
    "incomplete_resume_exact_binding",
    "predecessor_completion_required",
    "completion_after_runtime_validation",
    "completion_before_device_health_close",
    "audit_unchanged_on_resume",
    "execution_advanced_only_after_completion",
    "write_flush_readback",
    "old_slot_preserved",
    "resident_state_preserved",
    "shutdown_sealed",
    "storage_admission_closed",
    "irq_armed",
    "local_irq_masked",
    "emulator_backend",
    "exactly_once_claim",
    "arbitrary_resume_claim",
    "trusted_monotonic_backend",
    "production_key_claim",
    "hsm_claim",
    "rpmb_claim",
    "efuse_claim",
    "erase_resistance",
    "tamper_resistance",
    "host_rollback_resistance",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
FINAL_FIXED = {
    "format": "1",
    "abi": "39",
    "artifact": "BMA1",
    "audit_state": "BNDRMAU1",
    "audit_state_version": "1",
    "audit_relative_lbas": "5/6",
    "execution_state": "BNDRMEX1",
    "execution_state_version": "1",
    "execution_relative_lbas": "7/8",
    "operations": "1",
    "operation_storage_rotation": "1",
    "max_uses": "2",
    "audit_preflight_reads": "4",
    "exact_unfinished_head": "1",
    "execution_reads": "6",
    "execution_writes": "1",
    "execution_flushes": "1",
    "uses_consumed": "2",
    "rotations_completed": "2",
    "stop_drained": "1",
    "clean_shutdown_validated": "1",
    "completed_replay_rejected_by_admission": "1",
    "incomplete_resume_exact_binding": "1",
    "predecessor_completion_required": "1",
    "completion_after_runtime_validation": "1",
    "completion_before_device_health_close": "1",
    "execution_advanced_only_after_completion": "1",
    "write_flush_readback": "1",
    "old_slot_preserved": "1",
    "resident_state_preserved": "1",
    "shutdown_sealed": "1",
    "storage_admission_closed": "1",
    "irq_armed": "0",
    "local_irq_masked": "1",
    "emulator_backend": "qemu-psci",
    "exactly_once_claim": "0",
    "arbitrary_resume_claim": "0",
    "trusted_monotonic_backend": "0",
    "production_key_claim": "0",
    "hsm_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "erase_resistance": "0",
    "tamper_resistance": "0",
    "host_rollback_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

PHASE_AUDIT = {
    "sequence1": {
        **m77.PHASE_AUDIT["sequence1"],
        "reads": "4",
        "writes": "1",
        "flushes": "1",
        "audit_mutations": "1",
        "write_flush_readback": "1",
    },
    "resume": {
        "sequence": "2",
        "provisioned_before": "1",
        "previous_sequence": "2",
        "committed_sequence": "2",
        "previous_accepted_count": "2",
        "committed_accepted_count": "2",
        "initial_generation": "2",
        "committed_generation": "2",
        "initial_slot": "1",
        "committed_slot": "1",
        "initial_valid_slots": "2",
        "initial_rejected_slots": "0",
        "committed_valid_slots": "2",
        "committed_rejected_slots": "0",
        "reads": "0",
        "writes": "0",
        "flushes": "0",
        "audit_mutations": "0",
        "write_flush_readback": "0",
    },
}
PHASE_ADMISSION = {
    "sequence1": {
        "sequence": "1",
        "resumed": "0",
        "completed_sequence_before": "0",
        "audit_reads": "4",
        "audit_writes": "1",
        "audit_flushes": "1",
        "execution_initial_generation": "0",
        "execution_initial_slot": "255",
        "execution_initial_valid_slots": "0",
        "execution_initial_rejected_slots": "0",
        "audit_write_flush_readback": "1",
    },
    "resume": {
        "sequence": "2",
        "resumed": "1",
        "completed_sequence_before": "1",
        "audit_reads": "0",
        "audit_writes": "0",
        "audit_flushes": "0",
        "execution_initial_generation": "1",
        "execution_initial_slot": "0",
        "execution_initial_valid_slots": "1",
        "execution_initial_rejected_slots": "1",
        "audit_write_flush_readback": "0",
    },
}
PHASE_EXECUTION = {
    "sequence1": {
        "sequence": "1",
        "provisioned_before": "0",
        "previous_sequence": "0",
        "committed_sequence": "1",
        "previous_completed_count": "0",
        "committed_completed_count": "1",
        "initial_generation": "0",
        "committed_generation": "1",
        "initial_slot": "255",
        "committed_slot": "0",
        "initial_valid_slots": "0",
        "initial_rejected_slots": "0",
        "committed_valid_slots": "1",
        "committed_rejected_slots": "1",
    },
    "resume": {
        "sequence": "2",
        "provisioned_before": "1",
        "previous_sequence": "1",
        "committed_sequence": "2",
        "previous_completed_count": "1",
        "committed_completed_count": "2",
        "initial_generation": "1",
        "committed_generation": "2",
        "initial_slot": "0",
        "committed_slot": "1",
        "initial_valid_slots": "1",
        "initial_rejected_slots": "1",
        "committed_valid_slots": "2",
        "committed_rejected_slots": "0",
    },
}

DATA_RECORD_MAGIC = b"BNDRREC1"
EXECUTION_MAGIC = b"BNDRMEX1"
EXECUTION_WITNESS = 0xB24D_78DA_7A5E_C001
RECORD_COOKIE = 0x434F_4D4D_4954_2131
EXECUTION_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 7,
    fixture.DATA_PARTITION_FIRST_LBA + 8,
)


class EvidenceError(ValueError):
    pass


def parse_marker(
    lines: list[str], prefix: str, keys: tuple[str, ...]
) -> dict[str, str]:
    try:
        return m73.parse_fields(m73.unique_line(lines, prefix), prefix, keys)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def require(fields: dict[str, str], expected: dict[str, str], prefix: str) -> None:
    try:
        m73.require_fixed(fields, expected, prefix)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def digest(value: str, prefix: str, key: str, *, slashed: bool) -> str:
    pattern = r"[0-9a-f]{16}(?:/[0-9a-f]{16}){3}" if slashed else r"[0-9a-f]{64}"
    if re.fullmatch(pattern, value) is None:
        raise EvidenceError(f"{prefix} {key} is not a canonical SHA-256 digest")
    return value.replace("/", "")


def authorization_phase(phase: str) -> str:
    return "sequence1" if phase == "sequence1" else "sequence2"


def normalize_to_m76(text: str) -> str:
    removable = (
        AUDIT_PREFIX,
        ADMISSION_PREFIX,
        COMMIT_PREFIX,
        AUTHORIZATION_PREFIX,
        FINAL_PREFIX,
    )
    lines = [
        line
        for line in text.replace("\r", "").splitlines()
        if not any(line.startswith(prefix + " ") for prefix in removable)
    ]
    normalized = "\n".join(lines) + "\n"
    normalized = normalized.replace(UI_BOOT_MARKER, m76.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(SHUTDOWN_BOOT_MARKER, m76.SHUTDOWN_BOOT_MARKER, 1)
    return normalized.replace("abi=39", "abi=37")


def validate_positive(text: str, phase: str) -> dict[str, object]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M78 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = m73.FAILURE_PATTERN.search(normalized)
    if failure is not None or "MAINTENANCE_AUTHORIZATION_REJECTED " in normalized:
        raise EvidenceError("M78 positive boot contains failure evidence")
    lines = normalized.splitlines()
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(boots) != 2 or set(boots) != {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}:
        raise EvidenceError("M78 BOOT_OK marker set changed")

    audit = parse_marker(lines, AUDIT_PREFIX, m77.AUDIT_KEYS)
    admission = parse_marker(lines, ADMISSION_PREFIX, ADMISSION_KEYS)
    commit = parse_marker(lines, COMMIT_PREFIX, COMMIT_KEYS)
    authorization = parse_marker(lines, AUTHORIZATION_PREFIX, AUTHORIZATION_KEYS)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    auth_phase = authorization_phase(phase)

    audit_fixed = {
        **m77.AUDIT_FIXED,
        **PHASE_AUDIT[phase],
        "sequence": str(1 if phase == "sequence1" else 2),
        "authorization_sha256": m77.slash_digest(
            m77.AUTHORIZATION_SHA256[auth_phase]
        ),
        "authorization_id": m77.slash_digest(m77.AUTHORIZATION_ID[auth_phase]),
        "chain_sha256": m77.slash_digest(m77.CHAIN_SHA256[auth_phase]),
    }
    audit_fixed["reads"] = PHASE_AUDIT[phase]["reads"]
    audit_fixed["writes"] = PHASE_AUDIT[phase]["writes"]
    audit_fixed["flushes"] = PHASE_AUDIT[phase]["flushes"]
    audit_fixed["audit_mutations"] = PHASE_AUDIT[phase]["audit_mutations"]
    audit_fixed["write_flush_readback"] = PHASE_AUDIT[phase][
        "write_flush_readback"
    ]
    require(audit, audit_fixed, AUDIT_PREFIX)
    require(admission, ADMISSION_FIXED, ADMISSION_PREFIX)
    require(admission, PHASE_ADMISSION[phase], ADMISSION_PREFIX)
    require(commit, COMMIT_FIXED, COMMIT_PREFIX)
    require(commit, PHASE_EXECUTION[phase], COMMIT_PREFIX)
    require(final, FINAL_FIXED, FINAL_PREFIX)

    authorization_fixed = {
        **m77.FINAL_FIXED,
        **{
            key: value
            for key, value in PHASE_AUDIT[phase].items()
            if key in AUTHORIZATION_KEYS
        },
        "abi": "39",
        "sequence": audit["sequence"],
        "authorization_sha256": m77.AUTHORIZATION_SHA256[auth_phase],
        "authorization_id": m77.AUTHORIZATION_ID[auth_phase],
        "chain_sha256": m77.CHAIN_SHA256[auth_phase],
        "admission_resumed": PHASE_ADMISSION[phase]["resumed"],
        "completed_sequence_before": PHASE_ADMISSION[phase][
            "completed_sequence_before"
        ],
        "preflight_reads": "4",
    }
    authorization_fixed["reads"] = PHASE_AUDIT[phase]["reads"]
    authorization_fixed["writes"] = PHASE_AUDIT[phase]["writes"]
    authorization_fixed["flushes"] = PHASE_AUDIT[phase]["flushes"]
    authorization_fixed["write_flush_readback"] = PHASE_AUDIT[phase][
        "write_flush_readback"
    ]
    require(authorization, authorization_fixed, AUTHORIZATION_PREFIX)

    final_phase = {
        "sequence": commit["sequence"],
        "admission_resumed": admission["resumed"],
        "completed_sequence_before": admission["completed_sequence_before"],
        "audit_reads": admission["audit_reads"],
        "audit_writes": admission["audit_writes"],
        "audit_flushes": admission["audit_flushes"],
        "execution_provisioned_before": commit["provisioned_before"],
        "previous_execution_sequence": commit["previous_sequence"],
        "committed_execution_sequence": commit["committed_sequence"],
        "previous_completed_count": commit["previous_completed_count"],
        "committed_completed_count": commit["committed_completed_count"],
        "execution_initial_generation": commit["initial_generation"],
        "execution_committed_generation": commit["committed_generation"],
        "execution_initial_slot": commit["initial_slot"],
        "execution_committed_slot": commit["committed_slot"],
        "execution_initial_valid_slots": commit["initial_valid_slots"],
        "execution_initial_rejected_slots": commit["initial_rejected_slots"],
        "execution_committed_valid_slots": commit["committed_valid_slots"],
        "execution_committed_rejected_slots": commit["committed_rejected_slots"],
        "audit_unchanged_on_resume": admission["resumed"],
    }
    require(final, final_phase, FINAL_PREFIX)

    commit_runtime = digest(
        commit["runtime_evidence_sha256"],
        COMMIT_PREFIX,
        "runtime_evidence_sha256",
        slashed=True,
    )
    commit_chain = digest(
        commit["chain_sha256"], COMMIT_PREFIX, "chain_sha256", slashed=True
    )
    final_runtime = digest(
        final["runtime_evidence_sha256"],
        FINAL_PREFIX,
        "runtime_evidence_sha256",
        slashed=False,
    )
    final_audit = digest(
        final["audit_chain_sha256"],
        FINAL_PREFIX,
        "audit_chain_sha256",
        slashed=False,
    )
    final_chain = digest(
        final["execution_chain_sha256"],
        FINAL_PREFIX,
        "execution_chain_sha256",
        slashed=False,
    )
    if (
        commit_runtime != final_runtime
        or commit_chain != final_chain
        or final_audit != m77.CHAIN_SHA256[auth_phase]
        or authorization["chain_sha256"] != final_audit
        or commit["appdata_generation"] != final["appdata_generation"]
    ):
        raise EvidenceError("M78 audit, runtime, execution, or app-data binding diverged")
    if int(commit["appdata_generation"]) < 1:
        raise EvidenceError("M78 app-data generation is not positive")

    try:
        metrics = m76.validate_positive(normalize_to_m76(normalized), "steady")
    except ValueError as error:
        raise EvidenceError(str(error)) from error
    order = [
        m76.POLICY_PREFIX,
        AUDIT_PREFIX,
        ADMISSION_PREFIX,
        "USER_MAP_OK",
        COMMIT_PREFIX,
        m73.EVENT_PREFIX,
        m76.FINAL_PREFIX,
        AUTHORIZATION_PREFIX,
        FINAL_PREFIX,
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
    ]
    indices = [lines.index(m73.unique_line(lines, prefix)) for prefix in order]
    if indices != sorted(indices) or len(set(indices)) != len(indices):
        raise EvidenceError("M78 admission, completion, and shutdown order changed")
    return {
        "metrics": metrics,
        "audit": audit,
        "admission": admission,
        "commit": commit,
        "authorization": authorization,
        "final": final,
    }


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={m73.FINAL_SCREEN_SHA256} "
        "maintenance_rotations=2 cancel_pending=4"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M78 {phase} QMP self-exit evidence changed")


def validate_interrupt(text: str, driver: str) -> None:
    lines = text.replace("\r", "").splitlines()
    policy = m76.parse_marker(lines, m76.POLICY_PREFIX, m76.POLICY_KEYS)
    m76.require(policy, m76.POLICY_FIXED, m76.POLICY_PREFIX)
    m76.require(policy, m76.PHASE_POLICY["steady"], m76.POLICY_PREFIX)
    audit = parse_marker(lines, AUDIT_PREFIX, m77.AUDIT_KEYS)
    expected_audit = {
        **m77.AUDIT_FIXED,
        **m77.PHASE_AUDIT["sequence2"],
        "authorization_sha256": m77.slash_digest(
            m77.AUTHORIZATION_SHA256["sequence2"]
        ),
        "authorization_id": m77.slash_digest(m77.AUTHORIZATION_ID["sequence2"]),
        "chain_sha256": m77.slash_digest(m77.CHAIN_SHA256["sequence2"]),
    }
    require(audit, expected_audit, AUDIT_PREFIX)
    admission = parse_marker(lines, ADMISSION_PREFIX, ADMISSION_KEYS)
    require(admission, ADMISSION_FIXED, ADMISSION_PREFIX)
    expected_admission = {
        **PHASE_ADMISSION["resume"],
        "resumed": "0",
        "audit_reads": "4",
        "audit_writes": "1",
        "audit_flushes": "1",
        "audit_write_flush_readback": "1",
    }
    require(admission, expected_admission, ADMISSION_PREFIX)
    for forbidden in (
        COMMIT_PREFIX + " ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        "BOOT_OK: M78 ",
        "MAINTENANCE_AUTHORIZATION_REJECTED ",
    ):
        if any(line.startswith(forbidden) for line in lines):
            raise EvidenceError(f"M78 interrupted boot crossed boundary: {forbidden}")
    if not (
        lines.index(m73.unique_line(lines, m76.POLICY_PREFIX))
        < lines.index(m73.unique_line(lines, AUDIT_PREFIX))
        < lines.index(m73.unique_line(lines, ADMISSION_PREFIX))
    ):
        raise EvidenceError("M78 interrupted admission order changed")
    expected_driver = (
        "MAINTENANCE_EXECUTION_INTERRUPT_BOOT_OK sequence=2 "
        "admission_pre_el0=1 audit_committed=1 execution_completion=0 "
        "host_interrupted_before_completion=1 qemu_terminated_by_host=1 "
        "powercut_claim=0 emulator_only=1 real_phone_claim=0"
    )
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [expected_driver]:
        raise EvidenceError("M78 interruption host evidence changed")


def negative_rejection(reason: str) -> str:
    if reason == "signature":
        return m77.negative_rejection("signature")
    if reason == "binding":
        return m77.negative_rejection("binding")
    if reason == "completed-replay":
        return (
            "MAINTENANCE_AUTHORIZATION_REJECTED format=2 "
            "reason=completed-replay sequence=2 minimum=3 signature_valid=1 "
            "binding_valid=1 audit_attempted=1 audit_mutations=0 "
            "execution_mutations=0 manifest_published=0 init_ready=0"
        )
    raise EvidenceError(f"unknown M78 rejection reason: {reason}")


def negative_driver(reason: str) -> str:
    signature_valid = int(reason != "signature")
    binding_valid = int(reason == "completed-replay")
    audit_attempted = int(reason == "completed-replay")
    execution = (
        " execution_slots_mutated=0" if reason == "completed-replay" else ""
    )
    return (
        "MAINTENANCE_AUTHORIZATION_NEGATIVE_BOOT_OK "
        f"reason={reason} pre_el0=1 signature_valid={signature_valid} "
        f"binding_valid={binding_valid} audit_attempted={audit_attempted} "
        f"audit_slots_mutated=0{execution} manifest_published=0 init_ready=0 "
        "qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
    )


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in NEGATIVE_REASONS:
        raise EvidenceError(f"unknown M78 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = negative_rejection(reason)
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M78 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("boot error: maintenance authorization preparation failed:")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M78 {reason} boot diagnostic changed")
    policy = m76.parse_marker(lines, m76.POLICY_PREFIX, m76.POLICY_KEYS)
    m76.require(policy, m76.POLICY_FIXED, m76.POLICY_PREFIX)
    m76.require(policy, m76.PHASE_POLICY["steady"], m76.POLICY_PREFIX)
    for forbidden in (
        AUDIT_PREFIX + " ",
        ADMISSION_PREFIX + " ",
        COMMIT_PREFIX + " ",
        "USER_MAP_OK ",
        "ELF_LOAD_OK ",
        "UNIFIED_PRODUCT_UI_OK ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        "BOOT_OK: M78 ",
    ):
        if any(line.startswith(forbidden) for line in lines):
            raise EvidenceError(f"M78 {reason} boot crossed the pre-EL0 boundary")
    if not (
        lines.index(m73.unique_line(lines, m76.POLICY_PREFIX))
        < lines.index(rejection)
        < lines.index(diagnostics[0])
    ):
        raise EvidenceError(f"M78 {reason} rejection order changed")
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [negative_driver(reason)]:
        raise EvidenceError(f"M78 {reason} negative host evidence changed")


def read_sector(path: Path, lba: int) -> bytes:
    return m76.read_sector(path, lba)


def decode_execution_record(sector: bytes) -> dict[str, object]:
    if (
        len(sector) != fixture.SECTOR_BYTES
        or sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 368
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M78 execution record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M78 execution record commit proof changed")
    payload = sector[80:448]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[448:508] != bytes(60)
        or payload[:8] != EXECUTION_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or payload[12:16] != bytes(4)
        or payload[360:368] != bytes(8)
    ):
        raise EvidenceError("M78 execution state framing changed")
    state = {
        "generation": generation,
        "sequence": struct.unpack_from("<Q", payload, 16)[0],
        "completed_count": struct.unpack_from("<Q", payload, 24)[0],
        "operations": struct.unpack_from("<Q", payload, 32)[0],
        "max_uses": struct.unpack_from("<I", payload, 40)[0],
        "uses_consumed": struct.unpack_from("<I", payload, 44)[0],
        "rotations_completed": struct.unpack_from("<I", payload, 48)[0],
        "flags": struct.unpack_from("<I", payload, 52)[0],
        "appdata_generation": struct.unpack_from("<Q", payload, 56)[0],
        "authorization_sha256": payload[64:96],
        "authorization_id": payload[96:128],
        "manifest_sha256": payload[128:160],
        "policy_sha256": payload[160:192],
        "root_sha256": payload[192:224],
        "device_binding_sha256": payload[224:256],
        "previous_chain_sha256": payload[256:288],
        "chain_sha256": payload[288:320],
        "runtime_evidence_sha256": payload[320:352],
    }
    sequence = int(state["sequence"])
    phase = "sequence1" if sequence == 1 else "sequence2" if sequence == 2 else ""
    material = (
        EXECUTION_MAGIC
        + state["previous_chain_sha256"]
        + state["authorization_sha256"]
        + state["authorization_id"]
        + struct.pack("<Q", sequence)
        + struct.pack("<Q", int(state["operations"]))
        + struct.pack("<I", int(state["max_uses"]))
        + struct.pack("<I", int(state["uses_consumed"]))
        + struct.pack("<I", int(state["rotations_completed"]))
        + struct.pack("<I", int(state["flags"]))
        + struct.pack("<Q", int(state["appdata_generation"]))
        + state["runtime_evidence_sha256"]
        + state["manifest_sha256"]
        + state["policy_sha256"]
        + state["root_sha256"]
        + state["device_binding_sha256"]
    )
    witness = struct.unpack_from("<Q", payload, 352)[0]
    if (
        not phase
        or state["completed_count"] != sequence
        or state["operations"] != 1
        or state["max_uses"] != 2
        or state["uses_consumed"] != 2
        or state["rotations_completed"] != 2
        or state["flags"] != 3
        or state["appdata_generation"] == 0
        or state["authorization_sha256"].hex() != m77.AUTHORIZATION_SHA256[phase]
        or state["authorization_id"].hex() != m77.AUTHORIZATION_ID[phase]
        or state["manifest_sha256"].hex() != m77.MANIFEST_SHA256
        or state["policy_sha256"].hex() != m77.MAINTENANCE_POLICY_SHA256
        or state["root_sha256"].hex() != m77.ROOT_SHA256
        or state["device_binding_sha256"].hex() != m77.DEVICE_BINDING_SHA256
        or state["runtime_evidence_sha256"] == bytes(32)
        or hashlib.sha256(material).digest() != state["chain_sha256"]
        or fixture.fnv1a64(payload[16:352]) ^ EXECUTION_WITNESS != witness
    ):
        raise EvidenceError("M78 durable execution binding or hash chain changed")
    return {
        key: value.hex() if isinstance(value, bytes) else value
        for key, value in state.items()
    }


def verify_reboot(
    base_disk: Path,
    sequence1_disk: Path,
    interrupt_disk: Path,
    resume_disk: Path,
    signature_disk: Path,
    binding_disk: Path,
    replay_disk: Path,
    sequence1_log: Path,
    sequence1_driver: Path,
    interrupt_log: Path,
    interrupt_driver: Path,
    resume_log: Path,
    resume_driver: Path,
    signature_log: Path,
    signature_driver: Path,
    binding_log: Path,
    binding_driver: Path,
    replay_log: Path,
    replay_driver: Path,
) -> str:
    sequence1 = validate_positive(
        sequence1_log.read_text(encoding="utf-8", errors="replace"),
        "sequence1",
    )
    validate_driver(
        sequence1_driver.read_text(encoding="utf-8", errors="replace"),
        "sequence1",
    )
    validate_interrupt(
        interrupt_log.read_text(encoding="utf-8", errors="replace"),
        interrupt_driver.read_text(encoding="utf-8", errors="replace"),
    )
    resume = validate_positive(
        resume_log.read_text(encoding="utf-8", errors="replace"), "resume"
    )
    validate_driver(
        resume_driver.read_text(encoding="utf-8", errors="replace"), "resume"
    )
    for reason, log_path, driver_path in (
        ("signature", signature_log, signature_driver),
        ("binding", binding_log, binding_driver),
        ("completed-replay", replay_log, replay_driver),
    ):
        validate_negative(
            log_path.read_text(encoding="utf-8", errors="replace"),
            driver_path.read_text(encoding="utf-8", errors="replace"),
            reason,
        )

    base_audit = tuple(read_sector(base_disk, lba) for lba in m77.AUDIT_LBAS)
    base_execution = tuple(read_sector(base_disk, lba) for lba in EXECUTION_LBAS)
    if any(slot != bytes(fixture.SECTOR_BYTES) for slot in base_audit + base_execution):
        raise EvidenceError("M78 base maintenance ledgers were not pristine")

    sequence1_audit = tuple(
        read_sector(sequence1_disk, lba) for lba in m77.AUDIT_LBAS
    )
    sequence1_execution = tuple(
        read_sector(sequence1_disk, lba) for lba in EXECUTION_LBAS
    )
    audit1 = m77.decode_audit_record(sequence1_audit[0])
    execution1 = decode_execution_record(sequence1_execution[0])
    if (
        sequence1_audit[1] != bytes(fixture.SECTOR_BYTES)
        or sequence1_execution[1] != bytes(fixture.SECTOR_BYTES)
        or audit1["sequence"] != 1
        or execution1["generation"] != 1
        or execution1["sequence"] != 1
        or execution1["previous_chain_sha256"] != bytes(32).hex()
        or execution1["runtime_evidence_sha256"]
        != sequence1["final"]["runtime_evidence_sha256"]
        or execution1["chain_sha256"]
        != sequence1["final"]["execution_chain_sha256"]
    ):
        raise EvidenceError("M78 sequence-one dual-ledger commit changed")

    interrupt_audit = tuple(
        read_sector(interrupt_disk, lba) for lba in m77.AUDIT_LBAS
    )
    interrupt_execution = tuple(
        read_sector(interrupt_disk, lba) for lba in EXECUTION_LBAS
    )
    audit2 = m77.decode_audit_record(interrupt_audit[1])
    if (
        interrupt_audit[0] != sequence1_audit[0]
        or audit2["generation"] != 2
        or audit2["sequence"] != 2
        or audit2["previous_chain_sha256"] != m77.CHAIN_SHA256["sequence1"]
        or interrupt_execution != sequence1_execution
    ):
        raise EvidenceError("M78 interruption did not leave audit2/execution1")

    resume_audit = tuple(read_sector(resume_disk, lba) for lba in m77.AUDIT_LBAS)
    resume_execution = tuple(
        read_sector(resume_disk, lba) for lba in EXECUTION_LBAS
    )
    execution1_after = decode_execution_record(resume_execution[0])
    execution2 = decode_execution_record(resume_execution[1])
    if (
        resume_audit != interrupt_audit
        or resume_execution[0] != sequence1_execution[0]
        or execution1_after != execution1
        or execution2["generation"] != 2
        or execution2["sequence"] != 2
        or execution2["previous_chain_sha256"] != execution1["chain_sha256"]
        or execution2["runtime_evidence_sha256"]
        != resume["final"]["runtime_evidence_sha256"]
        or execution2["chain_sha256"]
        != resume["final"]["execution_chain_sha256"]
    ):
        raise EvidenceError("M78 exact resume did not preserve audit2 and commit execution2")

    for rejected_disk in (signature_disk, binding_disk, replay_disk):
        if tuple(read_sector(rejected_disk, lba) for lba in m77.AUDIT_LBAS) != resume_audit:
            raise EvidenceError("M78 rejected boot mutated an audit slot")
        if tuple(read_sector(rejected_disk, lba) for lba in EXECUTION_LBAS) != resume_execution:
            raise EvidenceError("M78 rejected boot mutated an execution slot")

    base_policy = tuple(read_sector(base_disk, lba) for lba in m76.POLICY_LBAS)
    for disk in (
        sequence1_disk,
        interrupt_disk,
        resume_disk,
        signature_disk,
        binding_disk,
        replay_disk,
    ):
        if tuple(read_sector(disk, lba) for lba in m76.POLICY_LBAS) != base_policy:
            raise EvidenceError("M78 boot mutated the inherited key policy")
    metrics = (
        sequence1["metrics"],
        resume["metrics"],
    )
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_REBOOT_OK format=1 abi=39 "
        "signed_authorizations=6 signature_valid=5 signature_rejections=1 "
        "binding_rejections=1 completed_replay_rejections=1 "
        "positive_self_exits=2 interrupted_boots=1 fail_closed_pre_el0_boots=3 "
        "audit_sequence=2 execution_sequence=2 audit_slot_generations=1/2 "
        "execution_slot_generations=1/2 audit_crc_verified=2 "
        "execution_crc_verified=2 audit_witness_verified=2 "
        "execution_witness_verified=2 audit_hash_chain_verified=2 "
        "execution_hash_chain_verified=2 interruption_state=audit2/execution1 "
        "resume_state=audit2/execution2 audit_unchanged_on_resume=1 "
        "execution_unchanged_before_completion=1 completed_replay_slot_mutations=0 "
        "rejected_boot_slot_mutations=0 old_slots_preserved=1 exact_binding_resume=1 "
        "predecessor_completion_required=1 uses_consumed=4 clean_rotations=4 "
        f"ui_query_calls={sum(item['ui_query_calls'] for item in metrics)} "
        f"ui_query_waits={sum(item['ui_query_waits'] for item in metrics)} "
        "ui_query_successes=2 qemu_psci_self_exits=2 host_terminated_boots=4 "
        "semihosting_uses=0 qemu_disk_audit=1 qemu_disk_execution=1 "
        "exactly_once_claim=0 arbitrary_resume_claim=0 trusted_monotonic_backend=0 "
        "fixture_root=1 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
        "efuse_claim=0 erase_resistance=0 tamper_resistance=0 "
        "host_rollback_resistance=0 hardware_powercut_claim=0 emulator_only=1 "
        "real_phone_claim=0"
    )


def build_self_test_sector() -> bytes:
    sequence = 1
    runtime = bytes((0x42,)) * 32
    previous = bytes(32)
    authorization = bytes.fromhex(m77.AUTHORIZATION_SHA256["sequence1"])
    authorization_id = bytes.fromhex(m77.AUTHORIZATION_ID["sequence1"])
    manifest = bytes.fromhex(m77.MANIFEST_SHA256)
    policy = bytes.fromhex(m77.MAINTENANCE_POLICY_SHA256)
    root = bytes.fromhex(m77.ROOT_SHA256)
    device = bytes.fromhex(m77.DEVICE_BINDING_SHA256)
    material = (
        EXECUTION_MAGIC
        + previous
        + authorization
        + authorization_id
        + struct.pack("<Q", sequence)
        + struct.pack("<Q", 1)
        + struct.pack("<I", 2)
        + struct.pack("<I", 2)
        + struct.pack("<I", 2)
        + struct.pack("<I", 3)
        + struct.pack("<Q", 6)
        + runtime
        + manifest
        + policy
        + root
        + device
    )
    chain = hashlib.sha256(material).digest()
    payload = bytearray(368)
    payload[:8] = EXECUTION_MAGIC
    struct.pack_into("<I", payload, 8, 1)
    struct.pack_into("<Q", payload, 16, 1)
    struct.pack_into("<Q", payload, 24, 1)
    struct.pack_into("<Q", payload, 32, 1)
    struct.pack_into("<I", payload, 40, 2)
    struct.pack_into("<I", payload, 44, 2)
    struct.pack_into("<I", payload, 48, 2)
    struct.pack_into("<I", payload, 52, 3)
    struct.pack_into("<Q", payload, 56, 6)
    payload[64:96] = authorization
    payload[96:128] = authorization_id
    payload[128:160] = manifest
    payload[160:192] = policy
    payload[192:224] = root
    payload[224:256] = device
    payload[256:288] = previous
    payload[288:320] = chain
    payload[320:352] = runtime
    struct.pack_into(
        "<Q",
        payload,
        352,
        fixture.fnv1a64(payload[16:352]) ^ EXECUTION_WITNESS,
    )
    sector = bytearray(512)
    sector[:8] = DATA_RECORD_MAGIC
    struct.pack_into("<I", sector, 8, 1)
    struct.pack_into("<I", sector, 12, 80)
    struct.pack_into("<Q", sector, 16, 1)
    struct.pack_into("<I", sector, 24, 368)
    struct.pack_into("<I", sector, 28, 1)
    struct.pack_into("<I", sector, 32, fixture.crc32(payload))
    struct.pack_into("<Q", sector, 40, fixture.fnv1a64(payload))
    struct.pack_into("<Q", sector, 48, 0xFFFF_FFFF_FFFF_FFFE)
    struct.pack_into("<Q", sector, 56, RECORD_COOKIE)
    sector[64:80] = fixture.DATA_FORMAT_EPOCH
    sector[80:448] = payload
    struct.pack_into("<I", sector, 508, fixture.crc32(sector[:508]))
    return bytes(sector)


def self_test() -> str:
    sector = build_self_test_sector()
    decoded = decode_execution_record(sector)
    if (
        decoded["sequence"] != 1
        or decoded["completed_count"] != 1
        or decoded["generation"] != 1
    ):
        raise EvidenceError("M78 self-test execution decode changed")
    corrupt = bytearray(sector)
    corrupt[320] ^= 1
    try:
        decode_execution_record(bytes(corrupt))
    except EvidenceError:
        pass
    else:
        raise EvidenceError("M78 self-test accepted a corrupt execution record")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_PARSER_SELF_TEST_OK "
        "format=1 abi=39 execution_record=BNDRMEX1 bytes=368 "
        "crc_rejection=1 witness_rejection=1 hash_chain_verified=1"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    positive = subparsers.add_parser("parse")
    positive.add_argument("phase", choices=PHASES)
    positive.add_argument("log", type=Path)
    interrupted = subparsers.add_parser("parse-interrupt")
    interrupted.add_argument("log", type=Path)
    interrupted.add_argument("driver", type=Path)
    negative = subparsers.add_parser("parse-negative")
    negative.add_argument("reason", choices=NEGATIVE_REASONS)
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)
    artifacts = subparsers.add_parser("artifacts")
    for name in (
        "sequence1",
        "sequence2",
        "bad_signature",
        "wrong_binding",
        "request1",
        "signature1",
        "request2",
        "signature2",
    ):
        artifacts.add_argument(name, type=Path)
    reboot = subparsers.add_parser("reboot")
    for name in (
        "base_disk",
        "sequence1_disk",
        "interrupt_disk",
        "resume_disk",
        "signature_disk",
        "binding_disk",
        "replay_disk",
        "sequence1_log",
        "sequence1_driver",
        "interrupt_log",
        "interrupt_driver",
        "resume_log",
        "resume_driver",
        "signature_log",
        "signature_driver",
        "binding_log",
        "binding_driver",
        "replay_log",
        "replay_driver",
    ):
        reboot.add_argument(name, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "self-test":
            print(self_test())
        elif args.command == "parse":
            validate_positive(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.phase,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_EVIDENCE_OK "
                f"phase={args.phase} abi=39 qemu_psci_self_exit=1"
            )
        elif args.command == "parse-interrupt":
            validate_interrupt(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_INTERRUPT_EVIDENCE_OK "
                "sequence=2 state=audit2/execution1 host_terminated=1"
            )
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason} pre_el0=1 slot_mutations=0"
            )
        elif args.command == "artifacts":
            print(
                m77.verify_artifacts(
                    args.sequence1,
                    args.sequence2,
                    args.bad_signature,
                    args.wrong_binding,
                    args.request1,
                    args.signature1,
                    args.request2,
                    args.signature2,
                ).replace(
                    "MAINTENANCE_AUTHORIZATION_ARTIFACTS_OK",
                    "MAINTENANCE_EXECUTION_AUTHORIZATION_ARTIFACTS_OK",
                    1,
                )
            )
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.base_disk,
                    args.sequence1_disk,
                    args.interrupt_disk,
                    args.resume_disk,
                    args.signature_disk,
                    args.binding_disk,
                    args.replay_disk,
                    args.sequence1_log,
                    args.sequence1_driver,
                    args.interrupt_log,
                    args.interrupt_driver,
                    args.resume_log,
                    args.resume_driver,
                    args.signature_log,
                    args.signature_driver,
                    args.binding_log,
                    args.binding_driver,
                    args.replay_log,
                    args.replay_driver,
                )
            )
        else:
            raise EvidenceError("unknown M78 parser command")
    except (EvidenceError, m73.EvidenceError, ValueError) as error:
        print(f"M78 evidence error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
