#!/usr/bin/env python3
"""Strict M80 durable maintenance-plan and reconciliation evidence."""

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
import unified_product_maintenance_execution_evidence as m78
import unified_product_maintenance_step_evidence as m79


PHASES = (
    "normal",
    "recover-prepared",
    "recover-applying",
    "recover-effect",
    "recover-plan-corrupt",
)
INTERRUPT_MODES = (
    "cut-prepared",
    "cut-applying",
    "cut-effect",
    "cancel-prepared",
)
UI_BOOT_MARKER = (
    "BOOT_OK: M80 unified real UI is interactive; durable maintenance-plan "
    "reconciliation is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M80 durable maintenance plan phases, result-unknown "
    "reconciliation, AppData, and PSCI shutdown armed"
)
PLAN_ADMISSION_PREFIX = "MAINTENANCE_PLAN_ADMISSION_OK"
PLAN_PHASE_PREFIX = "MAINTENANCE_PLAN_PHASE_OK"
PLAN_CANCEL_PREFIX = "MAINTENANCE_PLAN_CANCEL_OK"
PLAN_TERMINAL_PREFIX = "MAINTENANCE_PLAN_TERMINAL_OK"
FINAL_PREFIX = "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK"

PLAN_ADMISSION_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "plan_provisioned_before",
    "plan_legacy_anchor",
    "resumes_current_sequence",
    "plan_sequence_before",
    "plan_base_sequence_before",
    "transition_count_before",
    "operation_before",
    "phase_before",
    "generation",
    "slot",
    "valid_slots",
    "rejected_slots",
    "reads",
    "writes",
    "flushes",
    "result_unknown",
    "effect_observed_unconfirmed",
    "plan_id_sha256",
    "chain_sha256",
    "exact_authorization",
    "operation_instance_bound",
    "idempotency_key_bound",
    "preflight_before_audit_mutation",
    "external_effect_exactly_once_claim",
    "arbitrary_resume_claim",
    "trusted_monotonic_backend",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
PLAN_PHASE_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "requested_operation",
    "requested_phase",
    "selected_operation",
    "selected_phase",
    "replayed",
    "provisioned_before",
    "base_sequence",
    "previous_transition_count",
    "committed_transition_count",
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
    "result_unknown_before",
    "effect_observed_before",
    "plan_id_sha256",
    "operation_instance_id",
    "idempotency_key",
    "effect_sha256",
    "chain_sha256",
    "exact_authorization",
    "phase_ordered",
    "effect_idempotent",
    "old_slot_preserved",
    "write_flush_readback",
    "host_cut_window_ticks",
    "external_effect_exactly_once_claim",
    "arbitrary_resume_claim",
    "trusted_monotonic_backend",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
PLAN_TERMINAL_KEYS = (
    "format",
    "state",
    "state_version",
    "relative_lbas",
    "sequence",
    "base_sequence",
    "transition_count",
    "operation",
    "phase",
    "generation",
    "slot",
    "valid_slots",
    "rejected_slots",
    "reads",
    "writes",
    "flushes",
    "chain_sha256",
    "exact_authorization",
    "terminal_confirmed",
    "step_chain_bound",
    "selected_after_read",
    "aggregate_completion_next",
    "external_effect_exactly_once_claim",
    "arbitrary_resume_claim",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
FINAL_KEYS = (
    "format",
    "abi",
    "artifact",
    "audit_state",
    "audit_relative_lbas",
    "execution_state",
    "execution_relative_lbas",
    "step_state",
    "step_relative_lbas",
    "plan_state",
    "plan_state_version",
    "plan_relative_lbas",
    "sequence",
    "operations",
    "operation_storage_rotation",
    "max_uses",
    "program_operations",
    "normal_phase_transitions",
    "plan_base_sequence",
    "plan_transition_count",
    "plan_generation",
    "plan_slot",
    "plan_valid_slots",
    "plan_rejected_slots",
    "plan_terminal_reads",
    "plan_replayed_transitions",
    "result_unknown_observations",
    "effect_observed_reconciliations",
    "step1_replayed",
    "step2_replayed",
    "step3_replayed",
    "step_terminal_generation",
    "execution_generation",
    "plan_id_sha256",
    "runtime_evidence_sha256",
    "step_chain_sha256",
    "plan_chain_sha256",
    "execution_chain_sha256",
    "prepare_before_apply",
    "applying_before_effect",
    "confirm_after_step_durable",
    "preapply_compensation_supported",
    "exact_authorization",
    "operation_instance_ids",
    "idempotency_keys",
    "terminal_plan_bound_into_aggregate",
    "bounded_resident_reconciliation",
    "external_effect_exactly_once_claim",
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

PLAN_ADMISSION_FIXED = {
    "format": "1",
    "state": "BNDRMPL1",
    "state_version": "1",
    "authority": "kernel-pre-el0-four-ledger-preflight",
    "slots": "2",
    "relative_lbas": "11/12",
    "sequence": "2",
    "reads": "8",
    "writes": "0",
    "flushes": "0",
    "exact_authorization": "1",
    "operation_instance_bound": "1",
    "idempotency_key_bound": "1",
    "preflight_before_audit_mutation": "1",
    "external_effect_exactly_once_claim": "0",
    "arbitrary_resume_claim": "0",
    "trusted_monotonic_backend": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}
PLAN_PHASE_FIXED = {
    "format": "1",
    "state": "BNDRMPL1",
    "state_version": "1",
    "authority": "kernel-phase-machine-plus-qemu-disk",
    "slots": "2",
    "relative_lbas": "11/12",
    "sequence": "2",
    "base_sequence": "2",
    "exact_authorization": "1",
    "phase_ordered": "1",
    "effect_idempotent": "1",
    "old_slot_preserved": "1",
    "host_cut_window_ticks": "50",
    "external_effect_exactly_once_claim": "0",
    "arbitrary_resume_claim": "0",
    "trusted_monotonic_backend": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}
PLAN_TERMINAL_FIXED = {
    "format": "1",
    "state": "BNDRMPL1",
    "state_version": "1",
    "relative_lbas": "11/12",
    "sequence": "2",
    "base_sequence": "2",
    "transition_count": "9",
    "operation": "3",
    "phase": "3",
    "generation": "9",
    "slot": "0",
    "valid_slots": "2",
    "rejected_slots": "0",
    "reads": "8",
    "writes": "0",
    "flushes": "0",
    "exact_authorization": "1",
    "terminal_confirmed": "1",
    "step_chain_bound": "1",
    "selected_after_read": "1",
    "aggregate_completion_next": "1",
    "external_effect_exactly_once_claim": "0",
    "arbitrary_resume_claim": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}
FINAL_FIXED = {
    "format": "1",
    "abi": "41",
    "artifact": "BMA1",
    "audit_state": "BNDRMAU1",
    "audit_relative_lbas": "5/6",
    "execution_state": "BNDRMEX1",
    "execution_relative_lbas": "7/8",
    "step_state": "BNDRMST1",
    "step_relative_lbas": "9/10",
    "plan_state": "BNDRMPL1",
    "plan_state_version": "1",
    "plan_relative_lbas": "11/12",
    "sequence": "2",
    "operations": "1",
    "operation_storage_rotation": "1",
    "max_uses": "2",
    "program_operations": "3",
    "normal_phase_transitions": "9",
    "plan_base_sequence": "2",
    "plan_transition_count": "9",
    "plan_generation": "9",
    "plan_slot": "0",
    "plan_valid_slots": "2",
    "plan_rejected_slots": "0",
    "plan_terminal_reads": "8",
    "step2_replayed": "0",
    "step3_replayed": "0",
    "step_terminal_generation": "3",
    "execution_generation": "2",
    "prepare_before_apply": "1",
    "applying_before_effect": "1",
    "confirm_after_step_durable": "1",
    "preapply_compensation_supported": "1",
    "exact_authorization": "1",
    "operation_instance_ids": "3",
    "idempotency_keys": "3",
    "terminal_plan_bound_into_aggregate": "1",
    "bounded_resident_reconciliation": "1",
    "external_effect_exactly_once_claim": "0",
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

DATA_RECORD_MAGIC = b"BNDRREC1"
PLAN_MAGIC = b"BNDRMPL1"
PLAN_WITNESS = 0xB24D_80DA_7A5E_C001
RECORD_COOKIE = 0x434F_4D4D_4954_2131
PLAN_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 11,
    fixture.DATA_PARTITION_FIRST_LBA + 12,
)
PHASE_FLAGS = {1: 1, 2: 3, 3: 5, 4: 9}
NORMAL_PROGRAM = tuple(
    (operation, phase)
    for operation in (1, 2, 3)
    for phase in (1, 2, 3)
)
INITIAL_HEAD = {
    "normal": (0, 0, 0, 0, 255, 0, 0, 0),
    "recover-prepared": (1, 1, 1, 1, 0, 1, 1, 0),
    "recover-applying": (2, 1, 2, 2, 1, 2, 0, 0),
    "recover-effect": (2, 1, 2, 2, 1, 2, 0, 1),
    "recover-plan-corrupt": (1, 1, 1, 1, 0, 1, 1, 0),
}
PHASE_FINAL_COUNTS = {
    "normal": (0, 0, 3, 0),
    "recover-prepared": (1, 0, 3, 0),
    "recover-applying": (2, 2, 3, 0),
    "recover-effect": (2, 0, 5, 1),
    "recover-plan-corrupt": (1, 0, 3, 0),
}


class EvidenceError(ValueError):
    pass


def slash(value: bytes | str) -> str:
    raw = value.hex() if isinstance(value, bytes) else value
    return "/".join(raw[index : index + 16] for index in range(0, 64, 16))


def canonical_digest(value: str, prefix: str, key: str, *, slashed: bool) -> str:
    pattern = r"[0-9a-f]{16}(?:/[0-9a-f]{16}){3}" if slashed else r"[0-9a-f]{64}"
    if re.fullmatch(pattern, value) is None:
        raise EvidenceError(f"{prefix} {key} is not a canonical SHA-256 digest")
    return value.replace("/", "")


def parse_marker(
    lines: list[str], prefix: str, keys: tuple[str, ...]
) -> dict[str, str]:
    try:
        return m73.parse_fields(m73.unique_line(lines, prefix), prefix, keys)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def parse_many(
    lines: list[str], prefix: str, keys: tuple[str, ...]
) -> list[dict[str, str]]:
    markers = [line for line in lines if line.startswith(prefix + " ")]
    parsed = []
    for line in markers:
        try:
            parsed.append(m73.parse_fields(line, prefix, keys))
        except m73.EvidenceError as error:
            raise EvidenceError(str(error)) from error
    return parsed


def parse_loose(lines: list[str], prefix: str) -> dict[str, str]:
    try:
        line = m73.unique_line(lines, prefix)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error
    fields: dict[str, str] = {}
    for token in line.split()[1:]:
        if token.count("=") != 1:
            raise EvidenceError(f"{prefix} contains a non-field token")
        key, value = token.split("=", 1)
        if not key or not value or key in fields:
            raise EvidenceError(f"{prefix} contains an invalid or duplicate field")
        fields[key] = value
    return fields


def require(fields: dict[str, str], expected: dict[str, str], prefix: str) -> None:
    try:
        m73.require_fixed(fields, expected, prefix)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def authorization_material() -> dict[str, bytes]:
    return m79.authorization_material()


def plan_id() -> bytes:
    auth = authorization_material()
    material = (
        PLAN_MAGIC
        + struct.pack("<Q", 2)
        + struct.pack("<Q", 1)
        + struct.pack("<I", 2)
        + auth["authorization_sha256"]
        + auth["authorization_id"]
        + auth["manifest_sha256"]
        + auth["policy_sha256"]
        + auth["root_sha256"]
        + auth["device_binding_sha256"]
    )
    if len(material) != 220:
        raise AssertionError("M80 plan-id material length changed")
    return hashlib.sha256(material).digest()


def transition_binding(operation: int, phase: int) -> dict[str, object]:
    if operation not in (1, 2, 3) or phase not in PHASE_FLAGS:
        raise EvidenceError("invalid M80 transition binding")
    effect = m79.expected_effect(operation)
    identifier = plan_id()
    operation_instance = hashlib.sha256(
        b"M80-OPI1"
        + identifier
        + struct.pack("<I", operation)
        + effect
    ).digest()
    idempotency_key = hashlib.sha256(
        b"M80-IDK1" + identifier + operation_instance + effect
    ).digest()
    return {
        "sequence": 2,
        "operations": 1,
        "max_uses": 2,
        "operation": operation,
        "phase": phase,
        "flags": PHASE_FLAGS[phase],
        **authorization_material(),
        "effect_sha256": effect,
        "operation_instance_id": operation_instance,
        "idempotency_key": idempotency_key,
    }


def chain_digest(
    previous: bytes, transition_count: int, operation: int, phase: int
) -> bytes:
    binding = transition_binding(operation, phase)
    material = (
        PLAN_MAGIC
        + previous
        + struct.pack("<Q", 2)
        + struct.pack("<Q", 2)
        + struct.pack("<Q", transition_count)
        + struct.pack("<Q", 1)
        + struct.pack("<I", 2)
        + struct.pack("<I", operation)
        + struct.pack("<I", phase)
        + struct.pack("<I", int(binding["flags"]))
        + binding["authorization_sha256"]
        + binding["authorization_id"]
        + binding["manifest_sha256"]
        + binding["policy_sha256"]
        + binding["root_sha256"]
        + binding["device_binding_sha256"]
        + binding["effect_sha256"]
        + binding["operation_instance_id"]
        + binding["idempotency_key"]
    )
    if len(material) != 376:
        raise AssertionError("M80 plan-chain material length changed")
    return hashlib.sha256(material).digest()


def expected_chains(*, compensated: bool = False) -> dict[int, bytes]:
    chains: dict[int, bytes] = {}
    previous = bytes(32)
    program = ((1, 1), (1, 4)) if compensated else NORMAL_PROGRAM
    for count, (operation, phase) in enumerate(program, start=1):
        previous = chain_digest(previous, count, operation, phase)
        chains[count] = previous
    return chains


def read_sector(path: Path, lba: int) -> bytes:
    return m78.read_sector(path, lba)


def decode_plan_record(sector: bytes) -> dict[str, object]:
    if len(sector) != fixture.SECTOR_BYTES:
        raise EvidenceError("M80 plan sector size changed")
    if (
        sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 424
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M80 plan record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M80 plan record commit proof changed")
    payload = sector[80:504]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[504:508] != bytes(4)
        or payload[:8] != PLAN_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or payload[12:16] != bytes(4)
    ):
        raise EvidenceError("M80 plan state framing changed")
    state: dict[str, object] = {
        "generation": generation,
        "sequence": struct.unpack_from("<Q", payload, 16)[0],
        "base_sequence": struct.unpack_from("<Q", payload, 24)[0],
        "transition_count": struct.unpack_from("<Q", payload, 32)[0],
        "operations": struct.unpack_from("<Q", payload, 40)[0],
        "max_uses": struct.unpack_from("<I", payload, 48)[0],
        "operation": struct.unpack_from("<I", payload, 52)[0],
        "phase": struct.unpack_from("<I", payload, 56)[0],
        "flags": struct.unpack_from("<I", payload, 60)[0],
        "authorization_sha256": payload[64:96],
        "authorization_id": payload[96:128],
        "manifest_sha256": payload[128:160],
        "policy_sha256": payload[160:192],
        "root_sha256": payload[192:224],
        "device_binding_sha256": payload[224:256],
        "effect_sha256": payload[256:288],
        "operation_instance_id": payload[288:320],
        "idempotency_key": payload[320:352],
        "previous_chain_sha256": payload[352:384],
        "chain_sha256": payload[384:416],
    }
    operation = int(state["operation"])
    phase = int(state["phase"])
    count = int(state["transition_count"])
    binding = transition_binding(operation, phase)
    expected_count = (operation - 1) * 3 + (2 if phase == 4 else phase)
    compensated = phase == 4
    chains = expected_chains(compensated=compensated)
    if (
        state["sequence"] != 2
        or state["base_sequence"] != 2
        or state["operations"] != 1
        or state["max_uses"] != 2
        or count != expected_count
        or generation != count
        or state["flags"] != binding["flags"]
        or any(state[key] != binding[key] for key in authorization_material())
        or state["effect_sha256"] != binding["effect_sha256"]
        or state["operation_instance_id"] != binding["operation_instance_id"]
        or state["idempotency_key"] != binding["idempotency_key"]
        or state["previous_chain_sha256"]
        != (bytes(32) if count == 1 else chains[count - 1])
        or state["chain_sha256"] != chains[count]
        or struct.unpack_from("<Q", payload, 416)[0]
        != fixture.fnv1a64(payload[16:416]) ^ PLAN_WITNESS
    ):
        raise EvidenceError("M80 durable plan identity, order, or hash chain changed")
    return {
        key: value.hex() if isinstance(value, bytes) else value
        for key, value in state.items()
    }


def select_plan(path: Path) -> dict[str, object]:
    valid: list[tuple[int, int, dict[str, object], bytes]] = []
    rejected = 0
    for slot, lba in enumerate(PLAN_LBAS):
        sector = read_sector(path, lba)
        if sector == bytes(fixture.SECTOR_BYTES):
            rejected += 1
            continue
        try:
            state = decode_plan_record(sector)
        except EvidenceError:
            rejected += 1
            continue
        valid.append((int(state["generation"]), slot, state, sector))
    if not valid:
        raise EvidenceError("M80 disk contains no valid maintenance plan record")
    valid.sort(key=lambda item: item[0])
    if len(valid) == 2 and valid[0][0] == valid[1][0]:
        raise EvidenceError("M80 plan slots have an ambiguous generation")
    generation, slot, state, sector = valid[-1]
    return {
        "generation": generation,
        "slot": slot,
        "state": state,
        "sector": sector,
        "valid_slots": len(valid),
        "rejected_slots": rejected,
    }


def plan_slots_zero(path: Path) -> bool:
    return all(
        read_sector(path, lba) == bytes(fixture.SECTOR_BYTES) for lba in PLAN_LBAS
    )


def expected_admission(phase: str) -> dict[str, str]:
    count, operation, plan_phase, generation, slot, valid, rejected, step = (
        INITIAL_HEAD[phase]
    )
    absent = count == 0
    result_unknown = int(plan_phase == 2 and step == operation - 1)
    effect_observed = int(plan_phase == 2 and step == operation)
    chain = bytes(32) if absent else expected_chains()[count]
    return {
        **PLAN_ADMISSION_FIXED,
        "plan_provisioned_before": str(int(not absent)),
        "plan_legacy_anchor": str(int(absent)),
        "resumes_current_sequence": str(int(not absent)),
        "plan_sequence_before": "0" if absent else "2",
        "plan_base_sequence_before": "0" if absent else "2",
        "transition_count_before": str(count),
        "operation_before": str(operation),
        "phase_before": str(plan_phase),
        "generation": str(generation),
        "slot": str(slot),
        "valid_slots": str(valid),
        "rejected_slots": str(rejected),
        "result_unknown": str(result_unknown),
        "effect_observed_unconfirmed": str(effect_observed),
        "plan_id_sha256": slash(plan_id()),
        "chain_sha256": slash(chain),
    }


def expected_step_admission(phase: str) -> dict[str, str]:
    step = INITIAL_HEAD[phase][-1]
    if step == 0:
        return {
            **m79.STEP_ADMISSION_FIXED,
            "audit_sequence_before": "1" if phase == "normal" else "2",
            "step_provisioned_before": "0",
            "step_legacy_anchor": "1",
            "resumes_current_sequence": "0",
            "step_sequence_before": "0",
            "step_base_sequence_before": "0",
            "step_entry_count_before": "0",
            "step_ordinal_before": "0",
            "step_initial_generation": "0",
            "step_initial_slot": "255",
            "step_initial_valid_slots": "0",
            "step_initial_rejected_slots": "0",
        }
    return {
        **m79.STEP_ADMISSION_FIXED,
        "audit_sequence_before": "2",
        "step_provisioned_before": "1",
        "step_legacy_anchor": "0",
        "resumes_current_sequence": "1",
        "step_sequence_before": "2",
        "step_base_sequence_before": "2",
        "step_entry_count_before": str(step),
        "step_ordinal_before": str(step),
        "step_initial_generation": str(step),
        "step_initial_slot": str((step - 1) % 2),
        "step_initial_valid_slots": "1" if step == 1 else "2",
        "step_initial_rejected_slots": "1" if step == 1 else "0",
    }


def reconciliation(head_operation: int, head_phase: int, step: int) -> tuple[int, int]:
    if head_phase != 2:
        return (0, 0)
    if step == head_operation - 1:
        return (1, 0)
    if step == head_operation:
        return (0, 1)
    raise EvidenceError("M80 simulated step and APPLYING head diverged")


def validate_plan_phases(
    lines: list[str], phase: str
) -> list[dict[str, str]]:
    markers = parse_many(lines, PLAN_PHASE_PREFIX, PLAN_PHASE_KEYS)
    if len(markers) != 9:
        raise EvidenceError("M80 positive boot did not emit exactly nine plan phases")
    count, head_operation, head_phase, generation, slot, valid, rejected, step = (
        INITIAL_HEAD[phase]
    )
    chains = expected_chains()
    for marker, (operation, requested_phase) in zip(markers, NORMAL_PROGRAM):
        require(marker, PLAN_PHASE_FIXED, PLAN_PHASE_PREFIX)
        if requested_phase == 3:
            step = operation
        unknown, observed = reconciliation(head_operation, head_phase, step)
        requested_count = (operation - 1) * 3 + requested_phase
        replayed = requested_count <= count
        initial = (count, generation, slot, valid, rejected)
        if replayed:
            committed_operation = head_operation
            committed_phase = head_phase
            writes = 0
            reads = 10
            committed_slot = slot
            committed_valid = valid
            committed_rejected = rejected
            write_proof = 0
        else:
            if requested_count != count + 1:
                raise EvidenceError("M80 plan phase skipped a transition")
            count = requested_count
            generation += 1
            slot = 0 if slot == 255 else slot ^ 1
            valid = 1 if initial[3] == 0 else 2
            rejected = 1 if initial[3] == 0 else 0
            head_operation = operation
            head_phase = requested_phase
            committed_operation = operation
            committed_phase = requested_phase
            writes = 1
            reads = 12
            committed_slot = slot
            committed_valid = valid
            committed_rejected = rejected
            write_proof = 1
        binding = transition_binding(operation, requested_phase)
        expected = {
            "requested_operation": str(operation),
            "requested_phase": str(requested_phase),
            "selected_operation": str(committed_operation),
            "selected_phase": str(committed_phase),
            "replayed": str(int(replayed)),
            "provisioned_before": str(int(initial[0] != 0)),
            "previous_transition_count": str(initial[0]),
            "committed_transition_count": str(count),
            "initial_generation": str(initial[1]),
            "committed_generation": str(generation),
            "initial_slot": str(initial[2]),
            "committed_slot": str(committed_slot),
            "initial_valid_slots": str(initial[3]),
            "initial_rejected_slots": str(initial[4]),
            "committed_valid_slots": str(committed_valid),
            "committed_rejected_slots": str(committed_rejected),
            "reads": str(reads),
            "writes": str(writes),
            "flushes": str(writes),
            "result_unknown_before": str(unknown),
            "effect_observed_before": str(observed),
            "plan_id_sha256": slash(plan_id()),
            "operation_instance_id": slash(binding["operation_instance_id"]),
            "idempotency_key": slash(binding["idempotency_key"]),
            "effect_sha256": slash(binding["effect_sha256"]),
            "chain_sha256": slash(chains[count]),
            "write_flush_readback": str(write_proof),
        }
        require(marker, expected, PLAN_PHASE_PREFIX)
    return markers


def validate_steps(lines: list[str], phase: str) -> list[dict[str, str]]:
    markers = parse_many(lines, m79.STEP_COMMIT_PREFIX, m79.STEP_COMMIT_KEYS)
    if len(markers) != 3:
        raise EvidenceError("M80 positive boot did not emit exactly three M79 steps")
    current = INITIAL_HEAD[phase][-1]
    valid = 0 if current == 0 else (1 if current == 1 else 2)
    rejected = 0 if current == 0 else (1 if current == 1 else 0)
    slot = 255 if current == 0 else (current - 1) % 2
    chains = m79.expected_step_chains()
    for requested, marker in enumerate(markers, start=1):
        require(marker, m79.STEP_COMMIT_FIXED, m79.STEP_COMMIT_PREFIX)
        replayed = requested <= current
        before = current
        if replayed:
            reads, writes = 6, 0
            committed_slot = slot
            committed_valid, committed_rejected = valid, rejected
        else:
            if requested != current + 1:
                raise EvidenceError("M80 M79 sub-journal skipped a step")
            current = requested
            reads, writes = 8, 1
            committed_slot = 0 if slot == 255 else slot ^ 1
            committed_valid = 1 if valid == 0 else 2
            committed_rejected = 1 if valid == 0 else 0
        expected = {
            "requested_step": str(requested),
            "selected_step": str(current),
            "observed_rotations": str(1 if requested == 1 else 2),
            "drain_validated": str(int(requested == 3)),
            "replayed": str(int(replayed)),
            "provisioned_before": str(int(before != 0)),
            "previous_sequence": "0" if before == 0 else "2",
            "previous_entry_count": str(before),
            "committed_entry_count": str(current),
            "previous_step": str(before),
            "committed_step": str(current),
            "initial_generation": str(before),
            "committed_generation": str(current),
            "initial_slot": str(slot),
            "committed_slot": str(committed_slot),
            "initial_valid_slots": str(valid),
            "initial_rejected_slots": str(rejected),
            "committed_valid_slots": str(committed_valid),
            "committed_rejected_slots": str(committed_rejected),
            "reads": str(reads),
            "writes": str(writes),
            "flushes": str(writes),
            "effect_sha256": slash(m79.expected_effect(requested)),
            "chain_sha256": slash(chains[current]),
            "write_flush_readback": str(writes),
        }
        require(marker, expected, m79.STEP_COMMIT_PREFIX)
        slot, valid, rejected = (
            committed_slot,
            committed_valid,
            committed_rejected,
        )
    return markers


def runtime_digest(lines: list[str], plan_chain: bytes) -> bytes:
    event = parse_loose(lines, m73.EVENT_PREFIX)
    material = bytearray(224)
    material[:8] = b"BNDRMEX1"
    struct.pack_into("<Q", material, 8, 2)
    struct.pack_into("<Q", material, 16, 1)
    struct.pack_into("<I", material, 24, 2)
    struct.pack_into("<I", material, 28, 2)
    struct.pack_into("<Q", material, 32, 6)
    struct.pack_into("<Q", material, 40, int(event["report_calls"]))
    struct.pack_into("<Q", material, 48, int(event["report_successes"]))
    struct.pack_into("<Q", material, 56, int(event["rotations_started"]))
    struct.pack_into("<Q", material, 64, int(event["rotations_completed"]))
    struct.pack_into("<Q", material, 72, int(event["stop_batch"]))
    struct.pack_into("<Q", material, 80, int(event["stop_pending"]))
    struct.pack_into("<Q", material, 88, 0xFF)
    struct.pack_into("<Q", material, 96, 0xFF)
    struct.pack_into("<Q", material, 104, 5)
    struct.pack_into("<Q", material, 112, int(event["next_epoch"]))
    struct.pack_into("<Q", material, 120, int(event["releases"]))
    material[128:160] = bytes.fromhex(m77.CHAIN_SHA256["sequence2"])
    material[160:192] = m79.expected_step_chains()[3]
    material[192:224] = plan_chain
    return hashlib.sha256(material).digest()


def normalize_to_m76(text: str) -> str:
    prefixes = (
        PLAN_ADMISSION_PREFIX,
        PLAN_PHASE_PREFIX,
        PLAN_TERMINAL_PREFIX,
        FINAL_PREFIX,
    )
    lines = [
        line
        for line in text.splitlines()
        if not any(line.startswith(prefix + " ") for prefix in prefixes)
    ]
    normalized = "\n".join(lines) + "\n"
    normalized = normalized.replace(UI_BOOT_MARKER, m79.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(
        SHUTDOWN_BOOT_MARKER, m79.SHUTDOWN_BOOT_MARKER, 1
    )
    normalized = normalized.replace("abi=41", "abi=40")
    return m79.normalize_to_m76(normalized)


def validate_positive(text: str, phase: str) -> dict[str, object]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M80 phase: {phase}")
    normalized = text.replace("\r", "")
    if (
        m73.FAILURE_PATTERN.search(normalized) is not None
        or "MAINTENANCE_AUTHORIZATION_REJECTED " in normalized
    ):
        raise EvidenceError("M80 positive boot contains failure evidence")
    lines = normalized.splitlines()
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(boots) != 2 or set(boots) != {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}:
        raise EvidenceError("M80 BOOT_OK marker set changed")

    admission = parse_marker(lines, PLAN_ADMISSION_PREFIX, PLAN_ADMISSION_KEYS)
    require(admission, expected_admission(phase), PLAN_ADMISSION_PREFIX)
    audit_phase = "normal" if phase == "normal" else "recover-step1"
    audit = parse_marker(lines, m78.AUDIT_PREFIX, m77.AUDIT_KEYS)
    require(audit, m79.expected_audit(audit_phase), m78.AUDIT_PREFIX)
    execution_admission = parse_marker(
        lines, m78.ADMISSION_PREFIX, m78.ADMISSION_KEYS
    )
    require(
        execution_admission,
        m79.expected_execution_admission(audit_phase),
        m78.ADMISSION_PREFIX,
    )
    step_admission = parse_marker(
        lines, m79.STEP_ADMISSION_PREFIX, m79.STEP_ADMISSION_KEYS
    )
    require(
        step_admission,
        expected_step_admission(phase),
        m79.STEP_ADMISSION_PREFIX,
    )
    plan_phases = validate_plan_phases(lines, phase)
    steps = validate_steps(lines, phase)

    step_terminal = parse_marker(
        lines, m79.STEP_TERMINAL_PREFIX, m79.STEP_TERMINAL_KEYS
    )
    require(step_terminal, m79.STEP_TERMINAL_FIXED, m79.STEP_TERMINAL_PREFIX)
    plan_terminal = parse_marker(lines, PLAN_TERMINAL_PREFIX, PLAN_TERMINAL_KEYS)
    require(plan_terminal, PLAN_TERMINAL_FIXED, PLAN_TERMINAL_PREFIX)
    terminal_chain = expected_chains()[9]
    if canonical_digest(
        plan_terminal["chain_sha256"],
        PLAN_TERMINAL_PREFIX,
        "chain_sha256",
        slashed=True,
    ) != terminal_chain.hex():
        raise EvidenceError("M80 terminal plan chain changed")

    execution = parse_marker(lines, m78.COMMIT_PREFIX, m78.COMMIT_KEYS)
    require(execution, m78.COMMIT_FIXED, m78.COMMIT_PREFIX)
    require(execution, m78.PHASE_EXECUTION["resume"], m78.COMMIT_PREFIX)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    require(final, FINAL_FIXED, FINAL_PREFIX)
    replayed, unknown, observed, step1_replayed = PHASE_FINAL_COUNTS[phase]
    require(
        final,
        {
            "plan_replayed_transitions": str(replayed),
            "result_unknown_observations": str(unknown),
            "effect_observed_reconciliations": str(observed),
            "step1_replayed": str(step1_replayed),
            "plan_id_sha256": plan_id().hex(),
            "step_chain_sha256": m79.expected_step_chains()[3].hex(),
            "plan_chain_sha256": terminal_chain.hex(),
        },
        FINAL_PREFIX,
    )
    expected_runtime = runtime_digest(lines, terminal_chain)
    commit_runtime = canonical_digest(
        execution["runtime_evidence_sha256"],
        m78.COMMIT_PREFIX,
        "runtime_evidence_sha256",
        slashed=True,
    )
    final_runtime = canonical_digest(
        final["runtime_evidence_sha256"],
        FINAL_PREFIX,
        "runtime_evidence_sha256",
        slashed=False,
    )
    commit_chain = canonical_digest(
        execution["chain_sha256"],
        m78.COMMIT_PREFIX,
        "chain_sha256",
        slashed=True,
    )
    final_execution_chain = canonical_digest(
        final["execution_chain_sha256"],
        FINAL_PREFIX,
        "execution_chain_sha256",
        slashed=False,
    )
    if (
        commit_runtime != expected_runtime.hex()
        or final_runtime != expected_runtime.hex()
        or commit_chain != final_execution_chain
    ):
        raise EvidenceError("M80 aggregate runtime or execution-chain binding changed")

    try:
        m76.validate_positive(normalize_to_m76(normalized), "steady")
    except ValueError as error:
        raise EvidenceError(str(error)) from error
    order = (
        m78.AUDIT_PREFIX,
        m79.STEP_ADMISSION_PREFIX,
        PLAN_ADMISSION_PREFIX,
        PLAN_PHASE_PREFIX,
        m79.STEP_TERMINAL_PREFIX,
        PLAN_TERMINAL_PREFIX,
        m78.COMMIT_PREFIX,
        FINAL_PREFIX,
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
    )
    indices = [
        next(
            index
            for index, line in enumerate(lines)
            if line == prefix or line.startswith(prefix + " ")
        )
        for prefix in order
    ]
    if indices != sorted(indices) or len(set(indices)) != len(indices):
        raise EvidenceError("M80 admission, phase, terminal, aggregate, or shutdown order changed")
    return {
        "admission": admission,
        "plan_phases": plan_phases,
        "steps": steps,
        "terminal": plan_terminal,
        "execution": execution,
        "final": final,
    }


def validate_interrupt(text: str, driver: str, mode: str) -> None:
    if mode not in INTERRUPT_MODES:
        raise EvidenceError(f"unknown M80 interrupt mode: {mode}")
    normalized = text.replace("\r", "")
    if m73.FAILURE_PATTERN.search(normalized) is not None:
        raise EvidenceError("M80 interrupted boot contains failure evidence")
    lines = normalized.splitlines()
    phases = parse_many(lines, PLAN_PHASE_PREFIX, PLAN_PHASE_KEYS)
    expected_count = 1 if mode == "cut-prepared" else 2
    if len(phases) != expected_count:
        raise EvidenceError("M80 interrupted plan transition count changed")
    program = ((1, 1), (1, 4 if mode == "cancel-prepared" else 2))
    for index, marker in enumerate(phases):
        operation, phase = program[index]
        require(marker, PLAN_PHASE_FIXED, PLAN_PHASE_PREFIX)
        require(
            marker,
            {
                "requested_operation": str(operation),
                "requested_phase": str(phase),
                "selected_operation": str(operation),
                "selected_phase": str(phase),
                "replayed": "0",
                "previous_transition_count": str(index),
                "committed_transition_count": str(index + 1),
                "initial_generation": str(index),
                "committed_generation": str(index + 1),
                "writes": "1",
                "flushes": "1",
                "write_flush_readback": "1",
            },
            PLAN_PHASE_PREFIX,
        )
    steps = [
        line for line in lines if line.startswith(m79.STEP_COMMIT_PREFIX + " ")
    ]
    if len(steps) != int(mode == "cut-effect"):
        raise EvidenceError("M80 interrupted durable step count changed")
    if mode == "cut-effect":
        parsed = m73.parse_fields(
            steps[0], m79.STEP_COMMIT_PREFIX, m79.STEP_COMMIT_KEYS
        )
        require(
            parsed,
            {
                **m79.STEP_COMMIT_FIXED,
                "requested_step": "1",
                "selected_step": "1",
                "replayed": "0",
                "committed_generation": "1",
            },
            m79.STEP_COMMIT_PREFIX,
        )
    cancel_lines = [
        line for line in lines if line.startswith(PLAN_CANCEL_PREFIX + " ")
    ]
    if len(cancel_lines) != int(mode == "cancel-prepared"):
        raise EvidenceError("M80 cancellation marker count changed")
    forbidden = (
        PLAN_TERMINAL_PREFIX + " ",
        m78.COMMIT_PREFIX + " ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        SHUTDOWN_BOOT_MARKER,
    )
    if any(token in normalized for token in forbidden):
        raise EvidenceError("M80 interrupted boot crossed its terminal boundary")
    expected_driver = (
        "MAINTENANCE_PLAN_INTERRUPT_BOOT_OK "
        f"mode={mode} durable_plan_transitions={expected_count} "
        f"durable_step_effects={int(mode == 'cut-effect')} terminal_plan=0 "
        "aggregate_completion=0 host_stop_after_marker=1 "
        "qemu_terminated_by_host=1 powercut_claim=0 hardware_powercut_claim=0 "
        "emulator_only=1 real_phone_claim=0"
    )
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [expected_driver]:
        raise EvidenceError("M80 interrupt host evidence changed")


def corrupt_newest_plan(path: Path) -> str:
    selected = select_plan(path)
    state = selected["state"]
    slot = int(selected["slot"])
    generation = int(selected["generation"])
    if (
        generation != 2
        or state["operation"] != 1
        or state["phase"] != 2
        or selected["valid_slots"] != 2
    ):
        raise EvidenceError("M80 corruption source is not the APPLYING head")
    offset = PLAN_LBAS[slot] * fixture.SECTOR_BYTES + 80 + 384
    with path.open("r+b") as disk:
        disk.seek(offset)
        original = disk.read(1)
        if len(original) != 1:
            raise EvidenceError("M80 corruption offset is outside the disk")
        disk.seek(offset)
        disk.write(bytes((original[0] ^ 1,)))
        disk.flush()
    fallback = select_plan(path)
    if (
        fallback["generation"] != 1
        or fallback["state"]["operation"] != 1
        or fallback["state"]["phase"] != 1
        or fallback["valid_slots"] != 1
        or fallback["rejected_slots"] != 1
    ):
        raise EvidenceError("M80 newest-slot corruption did not select PREPARED")
    return (
        "MAINTENANCE_PLAN_CORRUPTION_OK corrupted_generation=2 "
        f"corrupted_slot={slot} fallback_generation=1 "
        f"fallback_slot={fallback['slot']} valid_slots=1 rejected_slots=1 "
        "audit_mutations=0 execution_mutations=0 step_mutations=0 "
        "host_fault_injection=1 hardware_powercut_claim=0 emulator_only=1 "
        "real_phone_claim=0"
    )


def build_self_test_sector(operation: int = 3, phase: int = 3) -> bytes:
    count = (operation - 1) * 3 + (2 if phase == 4 else phase)
    compensated = phase == 4
    chains = expected_chains(compensated=compensated)
    binding = transition_binding(operation, phase)
    payload = bytearray(424)
    payload[:8] = PLAN_MAGIC
    struct.pack_into("<I", payload, 8, 1)
    struct.pack_into("<Q", payload, 16, 2)
    struct.pack_into("<Q", payload, 24, 2)
    struct.pack_into("<Q", payload, 32, count)
    struct.pack_into("<Q", payload, 40, 1)
    struct.pack_into("<I", payload, 48, 2)
    struct.pack_into("<I", payload, 52, operation)
    struct.pack_into("<I", payload, 56, phase)
    struct.pack_into("<I", payload, 60, int(binding["flags"]))
    payload[64:96] = binding["authorization_sha256"]
    payload[96:128] = binding["authorization_id"]
    payload[128:160] = binding["manifest_sha256"]
    payload[160:192] = binding["policy_sha256"]
    payload[192:224] = binding["root_sha256"]
    payload[224:256] = binding["device_binding_sha256"]
    payload[256:288] = binding["effect_sha256"]
    payload[288:320] = binding["operation_instance_id"]
    payload[320:352] = binding["idempotency_key"]
    payload[352:384] = bytes(32) if count == 1 else chains[count - 1]
    payload[384:416] = chains[count]
    struct.pack_into(
        "<Q", payload, 416, fixture.fnv1a64(payload[16:416]) ^ PLAN_WITNESS
    )
    sector = bytearray(fixture.SECTOR_BYTES)
    sector[:8] = DATA_RECORD_MAGIC
    struct.pack_into("<I", sector, 8, 1)
    struct.pack_into("<I", sector, 12, 80)
    struct.pack_into("<Q", sector, 16, count)
    struct.pack_into("<I", sector, 24, 424)
    struct.pack_into("<I", sector, 28, 1)
    struct.pack_into("<I", sector, 32, fixture.crc32(payload))
    struct.pack_into("<Q", sector, 40, fixture.fnv1a64(payload))
    struct.pack_into("<Q", sector, 48, ~count & 0xFFFF_FFFF_FFFF_FFFF)
    struct.pack_into("<Q", sector, 56, RECORD_COOKIE)
    sector[64:80] = fixture.DATA_FORMAT_EPOCH
    sector[80:504] = payload
    struct.pack_into("<I", sector, 508, fixture.crc32(sector[:508]))
    return bytes(sector)


def self_test() -> str:
    for operation, phase in NORMAL_PROGRAM:
        decoded = decode_plan_record(build_self_test_sector(operation, phase))
        if decoded["operation"] != operation or decoded["phase"] != phase:
            raise EvidenceError("M80 self-test transition decode changed")
    compensated = decode_plan_record(build_self_test_sector(1, 4))
    if compensated["transition_count"] != 2:
        raise EvidenceError("M80 self-test compensation transition changed")
    corrupt = bytearray(build_self_test_sector())
    corrupt[464] ^= 1
    try:
        decode_plan_record(bytes(corrupt))
    except EvidenceError:
        pass
    else:
        raise EvidenceError("M80 self-test accepted a corrupt plan record")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_PLAN_PARSER_SELF_TEST_OK format=1 abi=41 "
        "plan_record=BNDRMPL1 bytes=424 program_operations=3 transitions=9 "
        "phase_states=4 crc_rejection=1 witness_rejection=1 "
        "operation_instance_ids=3 idempotency_keys=3 hash_chain_verified=9"
    )


def verify_reboot(args: argparse.Namespace) -> str:
    if not plan_slots_zero(args.base_disk):
        raise EvidenceError("M80 base disk unexpectedly contains a plan record")
    expected_heads = {
        args.cut_prepared_disk: (1, 1),
        args.cut_applying_disk: (2, 2),
        args.cut_effect_disk: (2, 2),
        args.cancel_disk: (2, 4),
        args.corrupt_disk: (1, 1),
    }
    for disk, (generation, phase) in expected_heads.items():
        selected = select_plan(disk)
        if (
            selected["generation"] != generation
            or selected["state"]["phase"] != phase
        ):
            raise EvidenceError(f"M80 interrupted disk head changed: {disk}")
    terminal_disks = (
        args.normal_disk,
        args.recover_prepared_disk,
        args.recover_applying_disk,
        args.recover_effect_disk,
        args.recover_corrupt_disk,
    )
    normal_slots = tuple(read_sector(args.normal_disk, lba) for lba in PLAN_LBAS)
    for disk in terminal_disks:
        selected = select_plan(disk)
        if (
            selected["generation"] != 9
            or selected["state"]["operation"] != 3
            or selected["state"]["phase"] != 3
            or selected["state"]["chain_sha256"] != expected_chains()[9].hex()
            or tuple(read_sector(disk, lba) for lba in PLAN_LBAS) != normal_slots
        ):
            raise EvidenceError(f"M80 terminal plan disk did not converge: {disk}")
    if not all(
        read_sector(args.cut_effect_disk, lba) != bytes(fixture.SECTOR_BYTES)
        for lba in m79.STEP_LBAS[:1]
    ):
        raise EvidenceError("M80 effect cut did not preserve its M79 step")
    if select_plan(args.corrupt_disk)["rejected_slots"] != 1:
        raise EvidenceError("M80 corrupt disk did not preserve one rejected slot")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_PLAN_REBOOT_OK base_plan_slots=0 "
        "normal_transitions=9 prepared_cut=1 applying_cut=2 effect_cut=2 "
        "compensated_cut=2 corruption_fallback_generation=1 "
        "terminal_converged_disks=5 terminal_plan_chain_bound=1 "
        "external_effect_exactly_once_claim=0 hardware_powercut_claim=0 "
        "emulator_only=1 real_phone_claim=0"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    positive = subparsers.add_parser("parse")
    positive.add_argument("phase", choices=PHASES)
    positive.add_argument("log", type=Path)
    interrupted = subparsers.add_parser("parse-interrupt")
    interrupted.add_argument("mode", choices=INTERRUPT_MODES)
    interrupted.add_argument("log", type=Path)
    interrupted.add_argument("driver", type=Path)
    corrupt = subparsers.add_parser("corrupt-newest-plan")
    corrupt.add_argument("disk", type=Path)
    reboot = subparsers.add_parser("reboot")
    for name in (
        "base_disk",
        "normal_disk",
        "cut_prepared_disk",
        "recover_prepared_disk",
        "cut_applying_disk",
        "recover_applying_disk",
        "cut_effect_disk",
        "recover_effect_disk",
        "corrupt_disk",
        "recover_corrupt_disk",
        "cancel_disk",
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
                "UNIFIED_PRODUCT_MAINTENANCE_PLAN_EVIDENCE_OK "
                f"phase={args.phase} abi=41 transitions=9 "
                "qemu_psci_self_exit=1"
            )
        elif args.command == "parse-interrupt":
            validate_interrupt(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.mode,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_PLAN_INTERRUPT_EVIDENCE_OK "
                f"mode={args.mode} host_terminated=1 terminal_plan=0"
            )
        elif args.command == "corrupt-newest-plan":
            print(corrupt_newest_plan(args.disk))
        elif args.command == "reboot":
            print(verify_reboot(args))
        else:
            raise EvidenceError("unknown M80 parser command")
    except (EvidenceError, OSError, ValueError) as error:
        print(f"M80 evidence error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
