#!/usr/bin/env python3
"""Strict M79 fixed-step maintenance journal and cut-point recovery evidence."""

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


PHASES = (
    "normal",
    "recover-step1",
    "recover-step2",
    "recover-step3",
    "recover-corrupt",
)
NEGATIVE_REASONS = ("signature", "binding", "completed-replay")
UI_BOOT_MARKER = (
    "BOOT_OK: M79 unified real UI is interactive; durable maintenance-step "
    "reconciliation is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M79 durable three-step maintenance journal, idempotent cut-point "
    "recovery, AppData, and PSCI shutdown armed"
)
STEP_ADMISSION_PREFIX = "MAINTENANCE_STEP_ADMISSION_OK"
STEP_COMMIT_PREFIX = "MAINTENANCE_STEP_COMMIT_OK"
STEP_TERMINAL_PREFIX = "MAINTENANCE_STEP_TERMINAL_OK"
FINAL_PREFIX = "UNIFIED_PRODUCT_MAINTENANCE_STEP_OK"

STEP_ADMISSION_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "audit_sequence_before",
    "execution_sequence_before",
    "step_provisioned_before",
    "step_legacy_anchor",
    "resumes_current_sequence",
    "step_sequence_before",
    "step_base_sequence_before",
    "step_entry_count_before",
    "step_ordinal_before",
    "step_initial_generation",
    "step_initial_slot",
    "step_initial_valid_slots",
    "step_initial_rejected_slots",
    "reads",
    "writes",
    "flushes",
    "exact_authorization",
    "predecessor_terminal_or_legacy",
    "preflight_before_audit_mutation",
    "replay_safe_from_boot_start",
    "arbitrary_resume_claim",
    "exactly_once_claim",
    "trusted_monotonic_backend",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
STEP_ADMISSION_FIXED = {
    "format": "1",
    "state": "BNDRMST1",
    "state_version": "1",
    "authority": "kernel-pre-el0-three-ledger-preflight",
    "slots": "2",
    "relative_lbas": "9/10",
    "sequence": "2",
    "execution_sequence_before": "1",
    "reads": "6",
    "writes": "0",
    "flushes": "0",
    "exact_authorization": "1",
    "predecessor_terminal_or_legacy": "1",
    "preflight_before_audit_mutation": "1",
    "replay_safe_from_boot_start": "1",
    "arbitrary_resume_claim": "0",
    "exactly_once_claim": "0",
    "trusted_monotonic_backend": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

STEP_COMMIT_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "requested_step",
    "selected_step",
    "observed_rotations",
    "drain_validated",
    "replayed",
    "replay_safe_from_boot_start",
    "provisioned_before",
    "base_sequence",
    "previous_sequence",
    "committed_sequence",
    "previous_entry_count",
    "committed_entry_count",
    "previous_step",
    "committed_step",
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
    "effect_sha256",
    "chain_sha256",
    "exact_authorization",
    "deterministic_effect",
    "no_step_gap",
    "old_slot_preserved",
    "write_flush_readback",
    "storage_terminal_preserved",
    "host_cut_window_ticks",
    "idempotent_reconciliation_claim",
    "external_effect_exactly_once_claim",
    "arbitrary_resume_claim",
    "trusted_monotonic_backend",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
STEP_COMMIT_FIXED = {
    "format": "1",
    "state": "BNDRMST1",
    "state_version": "1",
    "authority": "kernel-observed-fixed-program-plus-qemu-disk",
    "slots": "2",
    "relative_lbas": "9/10",
    "sequence": "2",
    "replay_safe_from_boot_start": "1",
    "base_sequence": "2",
    "committed_sequence": "2",
    "exact_authorization": "1",
    "deterministic_effect": "1",
    "no_step_gap": "1",
    "old_slot_preserved": "1",
    "storage_terminal_preserved": "1",
    "host_cut_window_ticks": "50",
    "idempotent_reconciliation_claim": "1",
    "external_effect_exactly_once_claim": "0",
    "arbitrary_resume_claim": "0",
    "trusted_monotonic_backend": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

STEP_TERMINAL_KEYS = (
    "format",
    "state",
    "state_version",
    "relative_lbas",
    "sequence",
    "base_sequence",
    "entry_count",
    "step",
    "generation",
    "slot",
    "valid_slots",
    "rejected_slots",
    "reads",
    "writes",
    "flushes",
    "rotations_completed",
    "chain_sha256",
    "exact_authorization",
    "terminal_drain",
    "selected_after_read",
    "aggregate_completion_next",
    "storage_terminal_preserved",
    "arbitrary_resume_claim",
    "exactly_once_claim",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
STEP_TERMINAL_FIXED = {
    "format": "1",
    "state": "BNDRMST1",
    "state_version": "1",
    "relative_lbas": "9/10",
    "sequence": "2",
    "base_sequence": "2",
    "entry_count": "3",
    "step": "3",
    "generation": "3",
    "slot": "0",
    "valid_slots": "2",
    "rejected_slots": "0",
    "reads": "4",
    "writes": "0",
    "flushes": "0",
    "rotations_completed": "2",
    "exact_authorization": "1",
    "terminal_drain": "1",
    "selected_after_read": "1",
    "aggregate_completion_next": "1",
    "storage_terminal_preserved": "1",
    "arbitrary_resume_claim": "0",
    "exactly_once_claim": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

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
    "step_state",
    "step_state_version",
    "step_relative_lbas",
    "sequence",
    "operations",
    "operation_storage_rotation",
    "max_uses",
    "program_steps",
    "admission_resumed",
    "completed_sequence_before",
    "audit_preflight_reads",
    "audit_reads",
    "audit_writes",
    "audit_flushes",
    "step_preflight_reads",
    "step_provisioned_before",
    "step_legacy_anchor",
    "step_sequence_before",
    "step_base_sequence_before",
    "step_entry_count_before",
    "step_ordinal_before",
    "step_initial_generation",
    "step_initial_slot",
    "step_initial_valid_slots",
    "step_initial_rejected_slots",
    "step1_replayed",
    "step1_selected",
    "step1_entry_count",
    "step1_generation",
    "step2_replayed",
    "step2_selected",
    "step2_entry_count",
    "step2_generation",
    "step3_replayed",
    "step3_selected",
    "step3_entry_count",
    "step3_generation",
    "terminal_sequence",
    "terminal_base_sequence",
    "terminal_entry_count",
    "terminal_generation",
    "terminal_slot",
    "terminal_valid_slots",
    "terminal_rejected_slots",
    "terminal_reads",
    "execution_provisioned_before",
    "previous_execution_sequence",
    "committed_execution_sequence",
    "previous_completed_count",
    "committed_completed_count",
    "execution_initial_generation",
    "execution_committed_generation",
    "execution_initial_slot",
    "execution_committed_slot",
    "execution_reads",
    "execution_writes",
    "execution_flushes",
    "uses_consumed",
    "rotations_completed",
    "appdata_generation",
    "runtime_evidence_sha256",
    "terminal_chain_sha256",
    "audit_chain_sha256",
    "execution_chain_sha256",
    "exact_authorization",
    "deterministic_effects",
    "no_step_gap",
    "terminal_bound_into_aggregate",
    "aggregate_after_terminal_read",
    "completion_before_device_health_close",
    "idempotent_reconciliation_claim",
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
FINAL_FIXED = {
    "format": "1",
    "abi": "40",
    "artifact": "BMA1",
    "audit_state": "BNDRMAU1",
    "audit_state_version": "1",
    "audit_relative_lbas": "5/6",
    "execution_state": "BNDRMEX1",
    "execution_state_version": "1",
    "execution_relative_lbas": "7/8",
    "step_state": "BNDRMST1",
    "step_state_version": "1",
    "step_relative_lbas": "9/10",
    "sequence": "2",
    "operations": "1",
    "operation_storage_rotation": "1",
    "max_uses": "2",
    "program_steps": "3",
    "completed_sequence_before": "1",
    "audit_preflight_reads": "4",
    "step_preflight_reads": "6",
    "terminal_sequence": "2",
    "terminal_base_sequence": "2",
    "terminal_entry_count": "3",
    "terminal_generation": "3",
    "terminal_slot": "0",
    "terminal_valid_slots": "2",
    "terminal_rejected_slots": "0",
    "terminal_reads": "4",
    "execution_provisioned_before": "1",
    "previous_execution_sequence": "1",
    "committed_execution_sequence": "2",
    "previous_completed_count": "1",
    "committed_completed_count": "2",
    "execution_initial_generation": "1",
    "execution_committed_generation": "2",
    "execution_initial_slot": "0",
    "execution_committed_slot": "1",
    "execution_reads": "6",
    "execution_writes": "1",
    "execution_flushes": "1",
    "uses_consumed": "2",
    "rotations_completed": "2",
    "exact_authorization": "1",
    "deterministic_effects": "1",
    "no_step_gap": "1",
    "terminal_bound_into_aggregate": "1",
    "aggregate_after_terminal_read": "1",
    "completion_before_device_health_close": "1",
    "idempotent_reconciliation_claim": "1",
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

PHASE_INITIAL_STEP = {
    "normal": (0, 0, 0, 255),
    "recover-step1": (1, 1, 1, 0),
    "recover-step2": (2, 2, 0, 1),
    "recover-step3": (3, 2, 0, 0),
    "recover-corrupt": (1, 1, 1, 0),
}

DATA_RECORD_MAGIC = b"BNDRREC1"
STEP_MAGIC = b"BNDRMST1"
STEP_WITNESS = 0xB24D_79DA_7A5E_C001
RECORD_COOKIE = 0x434F_4D4D_4954_2131
STEP_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 9,
    fixture.DATA_PARTITION_FIRST_LBA + 10,
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


def canonical_digest(value: str, prefix: str, key: str, *, slashed: bool) -> str:
    pattern = r"[0-9a-f]{16}(?:/[0-9a-f]{16}){3}" if slashed else r"[0-9a-f]{64}"
    if re.fullmatch(pattern, value) is None:
        raise EvidenceError(f"{prefix} {key} is not a canonical SHA-256 digest")
    return value.replace("/", "")


def slash_digest(value: bytes | str) -> str:
    raw = value.hex() if isinstance(value, bytes) else value
    return "/".join(raw[index : index + 16] for index in range(0, 64, 16))


def authorization_material() -> dict[str, bytes]:
    return {
        "authorization_sha256": bytes.fromhex(m77.AUTHORIZATION_SHA256["sequence2"]),
        "authorization_id": bytes.fromhex(m77.AUTHORIZATION_ID["sequence2"]),
        "manifest_sha256": bytes.fromhex(m77.MANIFEST_SHA256),
        "policy_sha256": bytes.fromhex(m77.MAINTENANCE_POLICY_SHA256),
        "root_sha256": bytes.fromhex(m77.ROOT_SHA256),
        "device_binding_sha256": bytes.fromhex(m77.DEVICE_BINDING_SHA256),
    }


def expected_effect(step: int) -> bytes:
    if step not in (1, 2, 3):
        raise EvidenceError(f"invalid fixed maintenance step: {step}")
    observed = 1 if step == 1 else 2
    flags = 1 if step != 3 else 3
    auth = authorization_material()
    material = (
        STEP_MAGIC
        + auth["authorization_sha256"]
        + auth["authorization_id"]
        + struct.pack("<Q", 2)
        + struct.pack("<Q", 1)
        + struct.pack("<I", 2)
        + struct.pack("<I", step)
        + struct.pack("<I", observed)
        + struct.pack("<I", flags)
        + auth["manifest_sha256"]
        + auth["policy_sha256"]
        + auth["root_sha256"]
        + auth["device_binding_sha256"]
    )
    if len(material) != 232:
        raise AssertionError("M79 effect material length changed")
    return hashlib.sha256(material).digest()


def expected_step_chains() -> dict[int, bytes]:
    auth = authorization_material()
    previous = bytes(32)
    chains: dict[int, bytes] = {}
    for step in (1, 2, 3):
        effect = expected_effect(step)
        material = (
            STEP_MAGIC
            + previous
            + effect
            + auth["authorization_sha256"]
            + auth["authorization_id"]
            + struct.pack("<Q", 2)
            + struct.pack("<Q", 2)
            + struct.pack("<Q", step)
            + struct.pack("<Q", 1)
            + struct.pack("<I", 2)
            + struct.pack("<I", step)
            + struct.pack("<I", 1 if step == 1 else 2)
            + struct.pack("<I", 1 if step != 3 else 3)
            + auth["manifest_sha256"]
            + auth["policy_sha256"]
            + auth["root_sha256"]
            + auth["device_binding_sha256"]
        )
        if len(material) != 312:
            raise AssertionError("M79 chain material length changed")
        previous = hashlib.sha256(material).digest()
        chains[step] = previous
    return chains


def decode_step_record(sector: bytes) -> dict[str, object]:
    if (
        len(sector) != fixture.SECTOR_BYTES
        or sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 376
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M79 step record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M79 step record commit proof changed")
    payload = sector[80:456]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[456:508] != bytes(52)
        or payload[:8] != STEP_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or payload[12:16] != bytes(4)
        or payload[360:376] != bytes(16)
    ):
        raise EvidenceError("M79 step state framing changed")
    state: dict[str, object] = {
        "generation": generation,
        "sequence": struct.unpack_from("<Q", payload, 16)[0],
        "base_sequence": struct.unpack_from("<Q", payload, 24)[0],
        "entry_count": struct.unpack_from("<Q", payload, 32)[0],
        "operations": struct.unpack_from("<Q", payload, 40)[0],
        "max_uses": struct.unpack_from("<I", payload, 48)[0],
        "step_ordinal": struct.unpack_from("<I", payload, 52)[0],
        "observed_rotations": struct.unpack_from("<I", payload, 56)[0],
        "flags": struct.unpack_from("<I", payload, 60)[0],
        "authorization_sha256": payload[64:96],
        "authorization_id": payload[96:128],
        "manifest_sha256": payload[128:160],
        "policy_sha256": payload[160:192],
        "root_sha256": payload[192:224],
        "device_binding_sha256": payload[224:256],
        "effect_sha256": payload[256:288],
        "previous_chain_sha256": payload[288:320],
        "chain_sha256": payload[320:352],
    }
    step = int(state["step_ordinal"])
    chains = expected_step_chains()
    auth = authorization_material()
    witness = struct.unpack_from("<Q", payload, 352)[0]
    if (
        state["sequence"] != 2
        or state["base_sequence"] != 2
        or state["entry_count"] != step
        or state["operations"] != 1
        or state["max_uses"] != 2
        or step not in (1, 2, 3)
        or state["observed_rotations"] != (1 if step == 1 else 2)
        or state["flags"] != (1 if step != 3 else 3)
        or state["authorization_sha256"] != auth["authorization_sha256"]
        or state["authorization_id"] != auth["authorization_id"]
        or state["manifest_sha256"] != auth["manifest_sha256"]
        or state["policy_sha256"] != auth["policy_sha256"]
        or state["root_sha256"] != auth["root_sha256"]
        or state["device_binding_sha256"] != auth["device_binding_sha256"]
        or state["effect_sha256"] != expected_effect(step)
        or state["previous_chain_sha256"]
        != (bytes(32) if step == 1 else chains[step - 1])
        or state["chain_sha256"] != chains[step]
        or fixture.fnv1a64(payload[16:352]) ^ STEP_WITNESS != witness
        or generation != step
    ):
        raise EvidenceError("M79 durable step binding, ordinal, or hash chain changed")
    return {
        key: value.hex() if isinstance(value, bytes) else value
        for key, value in state.items()
    }


def read_sector(path: Path, lba: int) -> bytes:
    return m78.read_sector(path, lba)


def select_step(path: Path) -> dict[str, object]:
    valid: list[tuple[int, int, dict[str, object], bytes]] = []
    rejected = 0
    for slot, lba in enumerate(STEP_LBAS):
        sector = read_sector(path, lba)
        if sector == bytes(fixture.SECTOR_BYTES):
            rejected += 1
            continue
        try:
            state = decode_step_record(sector)
        except EvidenceError:
            rejected += 1
            continue
        valid.append((int(state["generation"]), slot, state, sector))
    if not valid:
        raise EvidenceError("M79 disk contains no valid maintenance step record")
    valid.sort(key=lambda item: item[0])
    if len(valid) == 2 and valid[0][0] == valid[1][0]:
        raise EvidenceError("M79 step slots have an ambiguous generation")
    generation, slot, state, sector = valid[-1]
    return {
        "generation": generation,
        "slot": slot,
        "state": state,
        "sector": sector,
        "valid_slots": len(valid),
        "rejected_slots": rejected,
    }


def normalize_to_m76(text: str) -> str:
    removable = (
        m78.AUDIT_PREFIX,
        m78.ADMISSION_PREFIX,
        STEP_ADMISSION_PREFIX,
        STEP_COMMIT_PREFIX,
        STEP_TERMINAL_PREFIX,
        m78.COMMIT_PREFIX,
        m78.AUTHORIZATION_PREFIX,
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
    return normalized.replace("abi=40", "abi=37")


def expected_audit(phase: str) -> dict[str, str]:
    dynamic = (
        m77.PHASE_AUDIT["sequence2"]
        if phase == "normal"
        else m78.PHASE_AUDIT["resume"]
    )
    return {
        **m77.AUDIT_FIXED,
        **dynamic,
        "sequence": "2",
        "authorization_sha256": m77.slash_digest(
            m77.AUTHORIZATION_SHA256["sequence2"]
        ),
        "authorization_id": m77.slash_digest(m77.AUTHORIZATION_ID["sequence2"]),
        "chain_sha256": m77.slash_digest(m77.CHAIN_SHA256["sequence2"]),
    }


def expected_execution_admission(phase: str) -> dict[str, str]:
    expected = {
        **m78.ADMISSION_FIXED,
        **m78.PHASE_ADMISSION["resume"],
    }
    if phase == "normal":
        expected.update(
            {
                "resumed": "0",
                "audit_reads": "4",
                "audit_writes": "1",
                "audit_flushes": "1",
                "audit_write_flush_readback": "1",
            }
        )
    return expected


def expected_step_admission(phase: str) -> dict[str, str]:
    selected, valid, rejected, slot = PHASE_INITIAL_STEP[phase]
    if phase == "normal":
        return {
            **STEP_ADMISSION_FIXED,
            "audit_sequence_before": "1",
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
        **STEP_ADMISSION_FIXED,
        "audit_sequence_before": "2",
        "step_provisioned_before": "1",
        "step_legacy_anchor": "0",
        "resumes_current_sequence": "1",
        "step_sequence_before": "2",
        "step_base_sequence_before": "2",
        "step_entry_count_before": str(selected),
        "step_ordinal_before": str(selected),
        "step_initial_generation": str(selected),
        "step_initial_slot": str(slot),
        "step_initial_valid_slots": str(valid),
        "step_initial_rejected_slots": str(rejected),
    }


def parse_step_markers(
    lines: list[str],
    phase: str,
    *,
    count: int = 3,
) -> list[dict[str, str]]:
    marker_lines = [
        line for line in lines if line.startswith(STEP_COMMIT_PREFIX + " ")
    ]
    if len(marker_lines) != count:
        raise EvidenceError(
            f"M79 step marker count was {len(marker_lines)}, expected {count}"
        )
    parsed = [
        m73.parse_fields(line, STEP_COMMIT_PREFIX, STEP_COMMIT_KEYS)
        for line in marker_lines
    ]
    current, valid, rejected, slot = PHASE_INITIAL_STEP[phase]
    chains = expected_step_chains()
    for requested, fields in enumerate(parsed, start=1):
        require(fields, STEP_COMMIT_FIXED, STEP_COMMIT_PREFIX)
        replayed = requested <= current
        provisioned = current != 0
        if replayed:
            selected = current
            expected = {
                "requested_step": str(requested),
                "selected_step": str(selected),
                "observed_rotations": str(1 if requested == 1 else 2),
                "drain_validated": str(int(requested == 3)),
                "replayed": "1",
                "provisioned_before": "1",
                "previous_sequence": "2",
                "previous_entry_count": str(selected),
                "committed_entry_count": str(selected),
                "previous_step": str(selected),
                "committed_step": str(selected),
                "initial_generation": str(selected),
                "committed_generation": str(selected),
                "initial_slot": str(slot),
                "committed_slot": str(slot),
                "initial_valid_slots": str(valid),
                "initial_rejected_slots": str(rejected),
                "committed_valid_slots": str(valid),
                "committed_rejected_slots": str(rejected),
                "reads": "6",
                "writes": "0",
                "flushes": "0",
                "write_flush_readback": "0",
            }
        else:
            if requested != current + 1:
                raise EvidenceError("M79 host phase would require a skipped durable step")
            selected = requested
            committed_slot = 0 if current == 0 else slot ^ 1
            committed_valid = 1 if current == 0 else 2
            committed_rejected = 1 if current == 0 else 0
            expected = {
                "requested_step": str(requested),
                "selected_step": str(selected),
                "observed_rotations": str(1 if requested == 1 else 2),
                "drain_validated": str(int(requested == 3)),
                "replayed": "0",
                "provisioned_before": str(int(provisioned)),
                "previous_sequence": "0" if current == 0 else "2",
                "previous_entry_count": str(current),
                "committed_entry_count": str(selected),
                "previous_step": str(current),
                "committed_step": str(selected),
                "initial_generation": str(current),
                "committed_generation": str(selected),
                "initial_slot": "255" if current == 0 else str(slot),
                "committed_slot": str(committed_slot),
                "initial_valid_slots": str(valid),
                "initial_rejected_slots": str(rejected),
                "committed_valid_slots": str(committed_valid),
                "committed_rejected_slots": str(committed_rejected),
                "reads": "8",
                "writes": "1",
                "flushes": "1",
                "write_flush_readback": "1",
            }
            current = selected
            valid = committed_valid
            rejected = committed_rejected
            slot = committed_slot
        require(fields, expected, STEP_COMMIT_PREFIX)
        effect = canonical_digest(
            fields["effect_sha256"],
            STEP_COMMIT_PREFIX,
            "effect_sha256",
            slashed=True,
        )
        chain = canonical_digest(
            fields["chain_sha256"],
            STEP_COMMIT_PREFIX,
            "chain_sha256",
            slashed=True,
        )
        if effect != expected_effect(requested).hex() or chain != chains[selected].hex():
            raise EvidenceError("M79 step marker effect or selected chain diverged")
    return parsed


def validate_authorization(
    lines: list[str],
    phase: str,
    audit: dict[str, str],
    admission: dict[str, str],
) -> dict[str, str]:
    authorization = parse_marker(
        lines, m78.AUTHORIZATION_PREFIX, m78.AUTHORIZATION_KEYS
    )
    audit_phase = (
        m77.PHASE_AUDIT["sequence2"]
        if phase == "normal"
        else m78.PHASE_AUDIT["resume"]
    )
    expected = {
        **m77.FINAL_FIXED,
        **{key: value for key, value in audit_phase.items() if key in authorization},
        "abi": "40",
        "sequence": "2",
        "authorization_sha256": m77.AUTHORIZATION_SHA256["sequence2"],
        "authorization_id": m77.AUTHORIZATION_ID["sequence2"],
        "chain_sha256": m77.CHAIN_SHA256["sequence2"],
        "admission_resumed": admission["resumed"],
        "completed_sequence_before": "1",
        "preflight_reads": "4",
        "reads": audit["reads"],
        "writes": audit["writes"],
        "flushes": audit["flushes"],
        "write_flush_readback": str(int(phase == "normal")),
    }
    require(authorization, expected, m78.AUTHORIZATION_PREFIX)
    return authorization


def validate_positive(text: str, phase: str) -> dict[str, object]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M79 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = m73.FAILURE_PATTERN.search(normalized)
    if failure is not None or "MAINTENANCE_AUTHORIZATION_REJECTED " in normalized:
        raise EvidenceError("M79 positive boot contains failure evidence")
    lines = normalized.splitlines()
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(boots) != 2 or set(boots) != {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}:
        raise EvidenceError("M79 BOOT_OK marker set changed")

    audit = parse_marker(lines, m78.AUDIT_PREFIX, m77.AUDIT_KEYS)
    execution_admission = parse_marker(
        lines, m78.ADMISSION_PREFIX, m78.ADMISSION_KEYS
    )
    step_admission = parse_marker(
        lines, STEP_ADMISSION_PREFIX, STEP_ADMISSION_KEYS
    )
    steps = parse_step_markers(lines, phase)
    terminal = parse_marker(lines, STEP_TERMINAL_PREFIX, STEP_TERMINAL_KEYS)
    execution = parse_marker(lines, m78.COMMIT_PREFIX, m78.COMMIT_KEYS)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    require(audit, expected_audit(phase), m78.AUDIT_PREFIX)
    require(
        execution_admission,
        expected_execution_admission(phase),
        m78.ADMISSION_PREFIX,
    )
    require(
        step_admission,
        expected_step_admission(phase),
        STEP_ADMISSION_PREFIX,
    )
    require(terminal, STEP_TERMINAL_FIXED, STEP_TERMINAL_PREFIX)
    require(execution, m78.COMMIT_FIXED, m78.COMMIT_PREFIX)
    require(execution, m78.PHASE_EXECUTION["resume"], m78.COMMIT_PREFIX)
    require(final, FINAL_FIXED, FINAL_PREFIX)
    authorization = validate_authorization(lines, phase, audit, execution_admission)

    chains = expected_step_chains()
    terminal_chain = canonical_digest(
        terminal["chain_sha256"],
        STEP_TERMINAL_PREFIX,
        "chain_sha256",
        slashed=True,
    )
    runtime_digest = canonical_digest(
        execution["runtime_evidence_sha256"],
        m78.COMMIT_PREFIX,
        "runtime_evidence_sha256",
        slashed=True,
    )
    execution_chain = canonical_digest(
        execution["chain_sha256"],
        m78.COMMIT_PREFIX,
        "chain_sha256",
        slashed=True,
    )
    for key in (
        "runtime_evidence_sha256",
        "terminal_chain_sha256",
        "audit_chain_sha256",
        "execution_chain_sha256",
    ):
        canonical_digest(final[key], FINAL_PREFIX, key, slashed=False)
    if (
        terminal_chain != chains[3].hex()
        or terminal_chain != final["terminal_chain_sha256"]
        or runtime_digest != final["runtime_evidence_sha256"]
        or execution_chain != final["execution_chain_sha256"]
        or final["audit_chain_sha256"] != m77.CHAIN_SHA256["sequence2"]
        or authorization["chain_sha256"] != final["audit_chain_sha256"]
        or execution["appdata_generation"] != final["appdata_generation"]
    ):
        raise EvidenceError("M79 audit, step, runtime, or aggregate chain diverged")

    initial, _, _, _ = PHASE_INITIAL_STEP[phase]
    for requested, fields in enumerate(steps, start=1):
        final_expected = {
            f"step{requested}_replayed": fields["replayed"],
            f"step{requested}_selected": fields["selected_step"],
            f"step{requested}_entry_count": fields["committed_entry_count"],
            f"step{requested}_generation": fields["committed_generation"],
        }
        require(final, final_expected, FINAL_PREFIX)
    require(
        final,
        {
            "admission_resumed": execution_admission["resumed"],
            "audit_reads": execution_admission["audit_reads"],
            "audit_writes": execution_admission["audit_writes"],
            "audit_flushes": execution_admission["audit_flushes"],
            "step_provisioned_before": step_admission["step_provisioned_before"],
            "step_legacy_anchor": step_admission["step_legacy_anchor"],
            "step_sequence_before": step_admission["step_sequence_before"],
            "step_base_sequence_before": step_admission[
                "step_base_sequence_before"
            ],
            "step_entry_count_before": str(initial),
            "step_ordinal_before": str(initial),
            "step_initial_generation": step_admission["step_initial_generation"],
            "step_initial_slot": step_admission["step_initial_slot"],
            "step_initial_valid_slots": step_admission[
                "step_initial_valid_slots"
            ],
            "step_initial_rejected_slots": step_admission[
                "step_initial_rejected_slots"
            ],
            "appdata_generation": execution["appdata_generation"],
        },
        FINAL_PREFIX,
    )
    if int(execution["appdata_generation"]) < 1:
        raise EvidenceError("M79 app-data generation is not positive")

    try:
        metrics = m76.validate_positive(normalize_to_m76(normalized), "steady")
    except ValueError as error:
        raise EvidenceError(str(error)) from error
    event = m73.parse_fields(
        m73.unique_line(lines, m73.EVENT_PREFIX),
        m73.EVENT_PREFIX,
        m73.EVENT_KEYS,
    )
    shutdown = m73.parse_fields(
        m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK"),
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
    )
    runtime_material = bytearray(192)
    runtime_material[:8] = b"BNDRMEX1"
    struct.pack_into("<Q", runtime_material, 8, 2)
    struct.pack_into("<Q", runtime_material, 16, 1)
    struct.pack_into("<I", runtime_material, 24, 2)
    struct.pack_into("<I", runtime_material, 28, 2)
    struct.pack_into("<Q", runtime_material, 32, int(execution["appdata_generation"]))
    struct.pack_into("<Q", runtime_material, 40, int(event["report_calls"]))
    struct.pack_into("<Q", runtime_material, 48, int(event["report_successes"]))
    struct.pack_into("<Q", runtime_material, 56, int(event["rotations_started"]))
    struct.pack_into("<Q", runtime_material, 64, int(event["rotations_completed"]))
    struct.pack_into("<Q", runtime_material, 72, int(event["stop_batch"]))
    struct.pack_into("<Q", runtime_material, 80, int(event["stop_pending"]))
    struct.pack_into("<Q", runtime_material, 88, int(shutdown["registered_mask"], 0))
    struct.pack_into("<Q", runtime_material, 96, int(shutdown["quiesced_mask"], 0))
    struct.pack_into("<Q", runtime_material, 104, 5)
    struct.pack_into("<Q", runtime_material, 112, int(event["next_epoch"]))
    struct.pack_into("<Q", runtime_material, 120, int(event["releases"]))
    runtime_material[128:160] = bytes.fromhex(m77.CHAIN_SHA256["sequence2"])
    runtime_material[160:192] = chains[3]
    if hashlib.sha256(runtime_material).hexdigest() != runtime_digest:
        raise EvidenceError("M79 aggregate runtime digest did not bind the terminal step")

    order = [
        m76.POLICY_PREFIX,
        m78.AUDIT_PREFIX,
        m78.ADMISSION_PREFIX,
        STEP_ADMISSION_PREFIX,
        "USER_MAP_OK",
        STEP_TERMINAL_PREFIX,
        m78.COMMIT_PREFIX,
        m73.EVENT_PREFIX,
        m76.FINAL_PREFIX,
        m78.AUTHORIZATION_PREFIX,
        FINAL_PREFIX,
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
    ]
    indices = [lines.index(m73.unique_line(lines, prefix)) for prefix in order]
    if indices != sorted(indices) or len(indices) != len(set(indices)):
        raise EvidenceError("M79 preflight, terminal, aggregate, or shutdown order changed")
    step_indices = [
        lines.index(line)
        for line in lines
        if line.startswith(STEP_COMMIT_PREFIX + " ")
    ]
    if (
        step_indices != sorted(step_indices)
        or step_indices[0] <= indices[3]
        or step_indices[-1] >= indices[5]
    ):
        raise EvidenceError("M79 fixed-step commit order changed")
    return {
        "metrics": metrics,
        "audit": audit,
        "execution_admission": execution_admission,
        "step_admission": step_admission,
        "steps": steps,
        "terminal": terminal,
        "execution": execution,
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
        raise EvidenceError(f"M79 {phase} QMP self-exit evidence changed")


def validate_interrupt(text: str, driver: str, step: int) -> None:
    if step not in (1, 2, 3):
        raise EvidenceError("M79 interrupt step is outside the fixed program")
    normalized = text.replace("\r", "")
    lines = normalized.splitlines()
    product_failure = m73.FAILURE_PATTERN.search(normalized)
    if product_failure is not None:
        raise EvidenceError("M79 interrupted boot contains failure evidence")
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if boots != [UI_BOOT_MARKER]:
        raise EvidenceError("M79 interrupted boot crossed the final BOOT_OK boundary")
    audit = parse_marker(lines, m78.AUDIT_PREFIX, m77.AUDIT_KEYS)
    admission = parse_marker(lines, m78.ADMISSION_PREFIX, m78.ADMISSION_KEYS)
    step_admission = parse_marker(
        lines, STEP_ADMISSION_PREFIX, STEP_ADMISSION_KEYS
    )
    require(audit, expected_audit("normal"), m78.AUDIT_PREFIX)
    require(admission, expected_execution_admission("normal"), m78.ADMISSION_PREFIX)
    require(
        step_admission,
        expected_step_admission("normal"),
        STEP_ADMISSION_PREFIX,
    )
    steps = parse_step_markers(lines, "normal", count=step)
    if [int(fields["requested_step"]) for fields in steps] != list(
        range(1, step + 1)
    ):
        raise EvidenceError("M79 interrupted step prefix changed")
    for forbidden in (
        STEP_TERMINAL_PREFIX,
        m78.COMMIT_PREFIX,
        FINAL_PREFIX,
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
        SHUTDOWN_BOOT_MARKER,
        "MAINTENANCE_AUTHORIZATION_REJECTED",
    ):
        if any(line == forbidden or line.startswith(forbidden + " ") for line in lines):
            raise EvidenceError(f"M79 interruption crossed boundary: {forbidden}")
    expected_driver = (
        "MAINTENANCE_STEP_INTERRUPT_BOOT_OK "
        f"sequence=2 cut_after_step={step} durable_steps={step} "
        "later_step_observed=0 terminal_read=0 aggregate_completion=0 "
        "host_stop_after_marker=1 qemu_terminated_by_host=1 powercut_claim=0 "
        "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
    )
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [expected_driver]:
        raise EvidenceError(f"M79 step {step} interruption host evidence changed")


def negative_rejection(reason: str) -> str:
    if reason in ("signature", "binding"):
        return m77.negative_rejection(reason)
    if reason == "completed-replay":
        return (
            "MAINTENANCE_AUTHORIZATION_REJECTED format=2 "
            "reason=completed-replay sequence=2 minimum=3 signature_valid=1 "
            "binding_valid=1 audit_attempted=1 audit_mutations=0 "
            "execution_mutations=0 step_mutations=0 manifest_published=0 "
            "init_ready=0"
        )
    raise EvidenceError(f"unknown M79 rejection reason: {reason}")


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
        f"audit_slots_mutated=0{execution} step_slots_mutated=0 "
        "manifest_published=0 init_ready=0 qemu_terminated_by_host=1 "
        "emulator_only=1 real_phone_claim=0"
    )


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in NEGATIVE_REASONS:
        raise EvidenceError(f"unknown M79 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = negative_rejection(reason)
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M79 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("boot error: maintenance authorization preparation failed:")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M79 {reason} boot diagnostic changed")
    policy = m76.parse_marker(lines, m76.POLICY_PREFIX, m76.POLICY_KEYS)
    m76.require(policy, m76.POLICY_FIXED, m76.POLICY_PREFIX)
    m76.require(policy, m76.PHASE_POLICY["steady"], m76.POLICY_PREFIX)
    for forbidden in (
        m78.AUDIT_PREFIX,
        m78.ADMISSION_PREFIX,
        STEP_ADMISSION_PREFIX,
        STEP_COMMIT_PREFIX,
        STEP_TERMINAL_PREFIX,
        m78.COMMIT_PREFIX,
        FINAL_PREFIX,
        "USER_MAP_OK",
        "ELF_LOAD_OK",
        "UNIFIED_PRODUCT_UI_OK",
        "UNIFIED_PRODUCT_SHUTDOWN_OK",
        "BOOT_OK: M79",
    ):
        if any(line == forbidden or line.startswith(forbidden + " ") for line in lines):
            raise EvidenceError(f"M79 {reason} boot crossed the pre-EL0 boundary")
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [negative_driver(reason)]:
        raise EvidenceError(f"M79 {reason} negative host evidence changed")


def validate_disk_step(path: Path, expected_step: int) -> dict[str, object]:
    selected = select_step(path)
    state = selected["state"]
    if (
        state["step_ordinal"] != expected_step
        or selected["generation"] != expected_step
        or selected["slot"] != (0 if expected_step % 2 == 1 else 1)
    ):
        raise EvidenceError(
            f"M79 {path.name} selected step does not equal {expected_step}"
        )
    return selected


def unchanged_slots(left: Path, right: Path, lbas: tuple[int, int]) -> bool:
    return tuple(read_sector(left, lba) for lba in lbas) == tuple(
        read_sector(right, lba) for lba in lbas
    )


def verify_reboot(args: argparse.Namespace) -> str:
    positive_specs = (
        ("normal", args.normal_log, args.normal_driver),
        ("recover-step1", args.recover1_log, args.recover1_driver),
        ("recover-step2", args.recover2_log, args.recover2_driver),
        ("recover-step3", args.recover3_log, args.recover3_driver),
        ("recover-corrupt", args.recover_corrupt_log, args.recover_corrupt_driver),
    )
    positive = []
    for phase, log_path, driver_path in positive_specs:
        positive.append(
            validate_positive(
                log_path.read_text(encoding="utf-8", errors="replace"), phase
            )
        )
        validate_driver(
            driver_path.read_text(encoding="utf-8", errors="replace"), phase
        )
    for step, log_path, driver_path in (
        (1, args.cut1_log, args.cut1_driver),
        (2, args.cut2_log, args.cut2_driver),
        (3, args.cut3_log, args.cut3_driver),
    ):
        validate_interrupt(
            log_path.read_text(encoding="utf-8", errors="replace"),
            driver_path.read_text(encoding="utf-8", errors="replace"),
            step,
        )
    for reason, log_path, driver_path in (
        ("signature", args.signature_log, args.signature_driver),
        ("binding", args.binding_log, args.binding_driver),
        ("completed-replay", args.replay_log, args.replay_driver),
    ):
        validate_negative(
            log_path.read_text(encoding="utf-8", errors="replace"),
            driver_path.read_text(encoding="utf-8", errors="replace"),
            reason,
        )

    base_steps = tuple(read_sector(args.base_disk, lba) for lba in STEP_LBAS)
    if base_steps != (bytes(fixture.SECTOR_BYTES), bytes(fixture.SECTOR_BYTES)):
        raise EvidenceError("M79 migration base unexpectedly contains a step ledger")
    base_audit = tuple(read_sector(args.base_disk, lba) for lba in m77.AUDIT_LBAS)
    base_execution = tuple(
        read_sector(args.base_disk, lba) for lba in m78.EXECUTION_LBAS
    )
    audit1 = m77.decode_audit_record(base_audit[0])
    execution1 = m78.decode_execution_record(base_execution[0])
    if (
        base_audit[1] != bytes(fixture.SECTOR_BYTES)
        or base_execution[1] != bytes(fixture.SECTOR_BYTES)
        or audit1["sequence"] != 1
        or execution1["sequence"] != 1
    ):
        raise EvidenceError("M79 migration base is not exact M78 sequence one")

    cut_disks = (args.cut1_disk, args.cut2_disk, args.cut3_disk)
    for expected_step, disk in enumerate(cut_disks, start=1):
        selected = validate_disk_step(disk, expected_step)
        if (
            not unchanged_slots(args.base_disk, disk, m78.EXECUTION_LBAS)
            or selected["valid_slots"] != (1 if expected_step == 1 else 2)
            or selected["rejected_slots"] != (1 if expected_step == 1 else 0)
        ):
            raise EvidenceError(f"M79 cut after step {expected_step} disk state changed")
        audit_slots = tuple(read_sector(disk, lba) for lba in m77.AUDIT_LBAS)
        audit2 = m77.decode_audit_record(audit_slots[1])
        if audit_slots[0] != base_audit[0] or audit2["sequence"] != 2:
            raise EvidenceError(f"M79 cut after step {expected_step} audit changed")

    chains = expected_step_chains()
    final_disks = (
        args.normal_disk,
        args.recover1_disk,
        args.recover2_disk,
        args.recover3_disk,
        args.recover_corrupt_disk,
    )
    final_execution_chains: list[str] = []
    for disk, evidence in zip(final_disks, positive, strict=True):
        selected = validate_disk_step(disk, 3)
        state = selected["state"]
        execution_slots = tuple(
            read_sector(disk, lba) for lba in m78.EXECUTION_LBAS
        )
        execution2 = m78.decode_execution_record(execution_slots[1])
        audit_slots = tuple(read_sector(disk, lba) for lba in m77.AUDIT_LBAS)
        if (
            state["chain_sha256"] != chains[3].hex()
            or selected["valid_slots"] != 2
            or selected["rejected_slots"] != 0
            or audit_slots[0] != base_audit[0]
            or m77.decode_audit_record(audit_slots[1])["sequence"] != 2
            or execution_slots[0] != base_execution[0]
            or execution2["sequence"] != 2
            or execution2["previous_chain_sha256"]
            != execution1["chain_sha256"]
            or execution2["runtime_evidence_sha256"]
            != evidence["final"]["runtime_evidence_sha256"]
            or execution2["chain_sha256"]
            != evidence["final"]["execution_chain_sha256"]
        ):
            raise EvidenceError(f"M79 final disk {disk.name} diverged")
        final_execution_chains.append(str(execution2["chain_sha256"]))

    corrupt_selected = validate_disk_step(args.corrupt_disk, 1)
    if (
        corrupt_selected["valid_slots"] != 1
        or corrupt_selected["rejected_slots"] != 1
        or not unchanged_slots(args.cut2_disk, args.corrupt_disk, m77.AUDIT_LBAS)
        or not unchanged_slots(args.cut2_disk, args.corrupt_disk, m78.EXECUTION_LBAS)
    ):
        raise EvidenceError("M79 newest-slot corruption did not expose the prior valid head")

    for rejected_disk in (args.signature_disk, args.binding_disk):
        if (
            not unchanged_slots(args.base_disk, rejected_disk, m77.AUDIT_LBAS)
            or not unchanged_slots(
                args.base_disk, rejected_disk, m78.EXECUTION_LBAS
            )
            or not unchanged_slots(args.base_disk, rejected_disk, STEP_LBAS)
        ):
            raise EvidenceError("M79 signature or binding rejection mutated a ledger")
    if (
        not unchanged_slots(args.normal_disk, args.replay_disk, m77.AUDIT_LBAS)
        or not unchanged_slots(
            args.normal_disk, args.replay_disk, m78.EXECUTION_LBAS
        )
        or not unchanged_slots(args.normal_disk, args.replay_disk, STEP_LBAS)
    ):
        raise EvidenceError("M79 completed replay mutated a maintenance ledger")

    base_policy = tuple(read_sector(args.base_disk, lba) for lba in m76.POLICY_LBAS)
    for disk in (
        *cut_disks,
        *final_disks,
        args.corrupt_disk,
        args.signature_disk,
        args.binding_disk,
        args.replay_disk,
    ):
        if tuple(read_sector(disk, lba) for lba in m76.POLICY_LBAS) != base_policy:
            raise EvidenceError("M79 boot mutated the inherited key policy")

    replayed_steps = sum(
        int(step["replayed"])
        for evidence in positive
        for step in evidence["steps"]
    )
    metrics = [evidence["metrics"] for evidence in positive]
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_STEP_REBOOT_OK format=1 abi=40 "
        "migration_anchor=m78-sequence1 positive_self_exits=5 interrupted_boots=3 "
        "fail_closed_pre_el0_boots=3 audit_sequence=2 execution_sequence=2 "
        "step_sequence=2 step_base_sequence=2 step_entry_count=3 "
        "step_slot_generations=3/2 cut_states=step1/step2/step3 "
        "cut_host_stop_after_marker=1 terminal_reads=5 aggregate_completions=5 "
        f"idempotent_step_replays={replayed_steps} corruption_fallbacks=1 "
        "corrupt_newest_valid_slots=1 corrupt_newest_rejected_slots=1 "
        "corruption_repaired=1 step_crc_verified=1 step_witness_verified=1 "
        "step_effects_verified=3 step_hash_chain_verified=3 "
        "terminal_chain_bound_into_aggregate=1 rejected_boot_slot_mutations=0 "
        "completed_replay_step_mutations=0 exact_authorization=1 "
        f"ui_query_calls={sum(item['ui_query_calls'] for item in metrics)} "
        f"ui_query_waits={sum(item['ui_query_waits'] for item in metrics)} "
        "ui_query_successes=5 qemu_psci_self_exits=5 host_terminated_boots=6 "
        "semihosting_uses=0 qemu_disk_step_journal=1 "
        "idempotent_reconciliation_claim=1 external_effect_exactly_once_claim=0 "
        "arbitrary_resume_claim=0 trusted_monotonic_backend=0 fixture_root=1 "
        "production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0 "
        "erase_resistance=0 tamper_resistance=0 host_rollback_resistance=0 "
        "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
    )


def build_self_test_sector(step: int = 3) -> bytes:
    auth = authorization_material()
    chains = expected_step_chains()
    payload = bytearray(376)
    payload[:8] = STEP_MAGIC
    struct.pack_into("<I", payload, 8, 1)
    struct.pack_into("<Q", payload, 16, 2)
    struct.pack_into("<Q", payload, 24, 2)
    struct.pack_into("<Q", payload, 32, step)
    struct.pack_into("<Q", payload, 40, 1)
    struct.pack_into("<I", payload, 48, 2)
    struct.pack_into("<I", payload, 52, step)
    struct.pack_into("<I", payload, 56, 1 if step == 1 else 2)
    struct.pack_into("<I", payload, 60, 1 if step != 3 else 3)
    payload[64:96] = auth["authorization_sha256"]
    payload[96:128] = auth["authorization_id"]
    payload[128:160] = auth["manifest_sha256"]
    payload[160:192] = auth["policy_sha256"]
    payload[192:224] = auth["root_sha256"]
    payload[224:256] = auth["device_binding_sha256"]
    payload[256:288] = expected_effect(step)
    payload[288:320] = bytes(32) if step == 1 else chains[step - 1]
    payload[320:352] = chains[step]
    struct.pack_into(
        "<Q",
        payload,
        352,
        fixture.fnv1a64(payload[16:352]) ^ STEP_WITNESS,
    )
    sector = bytearray(fixture.SECTOR_BYTES)
    sector[:8] = DATA_RECORD_MAGIC
    struct.pack_into("<I", sector, 8, 1)
    struct.pack_into("<I", sector, 12, 80)
    struct.pack_into("<Q", sector, 16, step)
    struct.pack_into("<I", sector, 24, 376)
    struct.pack_into("<I", sector, 28, 1)
    struct.pack_into("<I", sector, 32, fixture.crc32(payload))
    struct.pack_into("<Q", sector, 40, fixture.fnv1a64(payload))
    struct.pack_into("<Q", sector, 48, ~step & 0xFFFF_FFFF_FFFF_FFFF)
    struct.pack_into("<Q", sector, 56, RECORD_COOKIE)
    sector[64:80] = fixture.DATA_FORMAT_EPOCH
    sector[80:456] = payload
    struct.pack_into("<I", sector, 508, fixture.crc32(sector[:508]))
    return bytes(sector)


def self_test() -> str:
    for step in (1, 2, 3):
        decoded = decode_step_record(build_self_test_sector(step))
        if (
            decoded["step_ordinal"] != step
            or decoded["entry_count"] != step
            or decoded["generation"] != step
        ):
            raise EvidenceError("M79 self-test step decode changed")
    corrupt = bytearray(build_self_test_sector())
    corrupt[336] ^= 1
    try:
        decode_step_record(bytes(corrupt))
    except EvidenceError:
        pass
    else:
        raise EvidenceError("M79 self-test accepted a corrupt step record")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_STEP_PARSER_SELF_TEST_OK format=1 abi=40 "
        "step_record=BNDRMST1 bytes=376 fixed_steps=3 crc_rejection=1 "
        "witness_rejection=1 deterministic_effects=3 hash_chain_verified=3"
    )


def corrupt_newest_step(path: Path) -> str:
    selected = select_step(path)
    slot = int(selected["slot"])
    generation = int(selected["generation"])
    offset = STEP_LBAS[slot] * fixture.SECTOR_BYTES + 336
    with path.open("r+b") as disk:
        disk.seek(offset)
        original = disk.read(1)
        if len(original) != 1:
            raise EvidenceError("M79 corruption offset is outside the disk")
        disk.seek(offset)
        disk.write(bytes((original[0] ^ 1,)))
        disk.flush()
    fallback = select_step(path)
    if (
        generation != 2
        or fallback["generation"] != 1
        or fallback["valid_slots"] != 1
        or fallback["rejected_slots"] != 1
    ):
        raise EvidenceError("M79 newest-slot corruption did not select step one")
    return (
        "MAINTENANCE_STEP_CORRUPTION_OK corrupted_generation=2 "
        f"corrupted_slot={slot} fallback_generation=1 "
        f"fallback_slot={fallback['slot']} valid_slots=1 rejected_slots=1 "
        "audit_mutations=0 execution_mutations=0 host_fault_injection=1 "
        "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    positive = subparsers.add_parser("parse")
    positive.add_argument("phase", choices=PHASES)
    positive.add_argument("log", type=Path)
    interrupted = subparsers.add_parser("parse-interrupt")
    interrupted.add_argument("step", type=int, choices=(1, 2, 3))
    interrupted.add_argument("log", type=Path)
    interrupted.add_argument("driver", type=Path)
    negative = subparsers.add_parser("parse-negative")
    negative.add_argument("reason", choices=NEGATIVE_REASONS)
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)
    corrupt = subparsers.add_parser("corrupt-newest-step")
    corrupt.add_argument("disk", type=Path)
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
        "normal_disk",
        "cut1_disk",
        "recover1_disk",
        "cut2_disk",
        "recover2_disk",
        "cut3_disk",
        "recover3_disk",
        "corrupt_disk",
        "recover_corrupt_disk",
        "signature_disk",
        "binding_disk",
        "replay_disk",
        "normal_log",
        "normal_driver",
        "cut1_log",
        "cut1_driver",
        "recover1_log",
        "recover1_driver",
        "cut2_log",
        "cut2_driver",
        "recover2_log",
        "recover2_driver",
        "cut3_log",
        "cut3_driver",
        "recover3_log",
        "recover3_driver",
        "recover_corrupt_log",
        "recover_corrupt_driver",
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
                "UNIFIED_PRODUCT_MAINTENANCE_STEP_EVIDENCE_OK "
                f"phase={args.phase} abi=40 fixed_steps=3 qemu_psci_self_exit=1"
            )
        elif args.command == "parse-interrupt":
            validate_interrupt(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.step,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_STEP_INTERRUPT_EVIDENCE_OK "
                f"cut_after_step={args.step} host_terminated=1"
            )
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_STEP_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason} pre_el0=1 slot_mutations=0"
            )
        elif args.command == "corrupt-newest-step":
            print(corrupt_newest_step(args.disk))
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
                    "MAINTENANCE_STEP_AUTHORIZATION_ARTIFACTS_OK",
                    1,
                )
            )
        elif args.command == "reboot":
            print(verify_reboot(args))
        else:
            raise EvidenceError("unknown M79 parser command")
    except (EvidenceError, m73.EvidenceError, ValueError) as error:
        print(f"M79 evidence error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
