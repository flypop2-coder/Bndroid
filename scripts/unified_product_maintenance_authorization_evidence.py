#!/usr/bin/env python3
"""Strict M77 signed-maintenance and durable anti-replay evidence parser."""

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


PHASES = ("sequence1", "sequence2")
NEGATIVE_REASONS = ("signature", "binding", "replay")
ROOT_SHA256 = "683eafa41277e75ae8cd1332490839a9cab204c8e8b808e2ebdf77f37259b005"
MANIFEST_SHA256 = "5a8ccd0ae601dad43356782ed0cc803fc3fe1232d3f42be981a71d94ca33e988"
PRODUCT_KEY_POLICY_SHA256 = (
    "30676f357683b6c8d2c5d67755e48beb93bdfe8864eccc67c716a1785e935e01"
)
DEVICE_BINDING_SHA256 = (
    "04ed2c7c657ebe5147d3da7103ea7a7e0d4d9bbe808915898967259e1b04a555"
)
MAINTENANCE_POLICY_SHA256 = (
    "d51ffa50e8ae40f278b5243f9d86a985b6d2835be59ff4c262d6c6436efe8200"
)
AUTHORIZATION_SHA256 = {
    "sequence1": "c324d66957b1b15321ed95ab220d355b0b9bb32c1d1d840dc59360a8146c3c4a",
    "sequence2": "cf910d5a62f53b1d8ee37af5745bf41f6986b101e54f49980f10558275e7db54",
}
AUTHORIZATION_ID = {
    "sequence1": "39429b7c72cd1cb86035a4fd0c040e9cd4aa80ee5e3b1dc8c84f75cb5f6eb54e",
    "sequence2": "e518725e0393dbd2115e7807e50949eb2feeb788dee21b1470142a9d4b8293a5",
}
CHAIN_SHA256 = {
    "sequence1": "87c402f7a09f63ffc64f5bfeed9fed182bfcad77f2cfe935cd7179180ca95c91",
    "sequence2": "14f45f4f2a243821a6c84c5afc73f32223d1f9ae2f64a02060effc391b6c119f",
}
FULL_ARTIFACT_SHA256 = {
    "sequence1": "39aa2a518984e0d57d166ce9df502d338ab6a2ba9048416da1f8db5b0e8f2799",
    "sequence2": "2291925f70eebc9c45e08af972d1b16c763367aa2d10bef6d63a8fb116fe86ad",
}
ROOT_MODULUS = bytes.fromhex(
    "d444712ced368992a4221470281839d6aa7b76c49d60a4463b47be23f9228694"
    "14b94649aef5fce59b03ac1401d07b73952a2868a1ee2a58b27cbd22317c5ea"
    "248e7dc0376695958fad47b3f7ea6685288a3693e211c418894206c02ac0ff6b"
    "be00190e59495510972229d4227840d8362ca12ff09f8a30460fdb0c94ca552"
    "8c707cf3540471fd5885f6055d376879bad30b5f9e06c32f28eafe43f9f2498"
    "ae32c90c86402fb65cf3ef7b81b2bcdccdc6d1c0a251eee1d6bef82b4da019"
    "4906af334f3f58795ebe56e4aa94079aa16e904a7cdbbcd2944042a19cb8808"
    "e62b38578477259f39b6da383fd3a2af61c1dc47e4586699eb253480e9bc1e2"
    "e86bf83"
)
RSA_EXPONENT = 65537
SHA256_DIGEST_INFO_PREFIX = bytes.fromhex("3031300d060960864801650304020105000420")

UI_BOOT_MARKER = (
    "BOOT_OK: M77 unified real UI is interactive; signed maintenance-session "
    "event supervision is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M77 signed maintenance session, durable anti-replay audit, "
    "persistent key policy, event supervision, AppData, and PSCI shutdown armed"
)
AUDIT_PREFIX = "MAINTENANCE_AUDIT_OK"
FINAL_PREFIX = "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK"
FAILURE_PATTERN = re.compile(
    m73.FAILURE_PATTERN.pattern + r"|MAINTENANCE_AUTHORIZATION_REJECTED",
    m73.FAILURE_PATTERN.flags,
)

AUDIT_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "sequence",
    "operations",
    "max_uses",
    "provisioned_before",
    "previous_sequence",
    "committed_sequence",
    "previous_accepted_count",
    "committed_accepted_count",
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
    "authorization_sha256",
    "authorization_id",
    "chain_sha256",
    "signature_valid",
    "binding_valid",
    "audit_mutations",
    "write_flush_readback",
    "manifest_published",
    "init_ready",
    "trusted_monotonic_backend",
    "rpmb_claim",
    "efuse_claim",
    "hsm_claim",
    "host_rollback_resistance",
    "hardware_powercut_claim",
    "emulator_only",
)
AUDIT_FIXED = {
    "format": "1",
    "state": "BNDRMAU1",
    "state_version": "1",
    "authority": "kernel-pre-el0-qemu-disk",
    "slots": "2",
    "relative_lbas": "5/6",
    "operations": "1",
    "max_uses": "2",
    "reads": "4",
    "writes": "1",
    "flushes": "1",
    "signature_valid": "1",
    "binding_valid": "1",
    "audit_mutations": "1",
    "write_flush_readback": "1",
    "manifest_published": "0",
    "init_ready": "0",
    "trusted_monotonic_backend": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "hsm_claim": "0",
    "host_rollback_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
}
PHASE_AUDIT = {
    "sequence1": {
        "sequence": "1",
        "provisioned_before": "0",
        "previous_sequence": "0",
        "committed_sequence": "1",
        "previous_accepted_count": "0",
        "committed_accepted_count": "1",
        "initial_generation": "0",
        "committed_generation": "1",
        "initial_slot": "255",
        "committed_slot": "0",
        "initial_valid_slots": "0",
        "initial_rejected_slots": "0",
        "committed_valid_slots": "1",
        "committed_rejected_slots": "1",
    },
    "sequence2": {
        "sequence": "2",
        "provisioned_before": "1",
        "previous_sequence": "1",
        "committed_sequence": "2",
        "previous_accepted_count": "1",
        "committed_accepted_count": "2",
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

FINAL_KEYS = (
    "format",
    "abi",
    "artifact",
    "artifact_bytes",
    "signed_bytes",
    "algorithm",
    "root_key_id",
    "root_sha256",
    "manifest_generation",
    "manifest_sha256",
    "active_key_epoch",
    "product_key_policy_sha256",
    "device_binding_sha256",
    "maintenance_policy_sha256",
    "sequence",
    "operations",
    "operation_storage_rotation",
    "max_uses",
    "authorization_sha256",
    "authorization_id",
    "audit_state",
    "audit_state_version",
    "audit_slots",
    "audit_relative_lbas",
    "provisioned_before",
    "previous_sequence",
    "committed_sequence",
    "previous_accepted_count",
    "committed_accepted_count",
    "initial_generation",
    "committed_generation",
    "initial_slot",
    "committed_slot",
    "initial_valid_slots",
    "initial_rejected_slots",
    "committed_valid_slots",
    "committed_rejected_slots",
    "chain_sha256",
    "reads",
    "writes",
    "flushes",
    "authorization_attempts",
    "authorization_successes",
    "signature_rejections",
    "binding_rejections",
    "audit_attempts",
    "audit_successes",
    "audit_replay_rejections",
    "audit_failures",
    "session_scope",
    "session_open_calls",
    "session_open_successes",
    "session_argument_rejections",
    "session_permission_rejections",
    "session_state_rejections",
    "session_opened",
    "mutating_report_gate_denials",
    "ui_convergence_query_read_only",
    "report_gate_before_trace_mutation",
    "exact_sequence",
    "signed_before_audit",
    "write_flush_readback",
    "old_slot_preserved",
    "external_artifact",
    "separate_manifest_and_maintenance_roots",
    "offline_split_signing_supported",
    "private_key_in_repository",
    "trusted_monotonic_backend",
    "production_key_claim",
    "hsm_claim",
    "rpmb_claim",
    "efuse_claim",
    "erase_resistance",
    "tamper_resistance",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
FINAL_FIXED = {
    "format": "1",
    "abi": "38",
    "artifact": "BMA1",
    "artifact_bytes": "512",
    "signed_bytes": "256",
    "algorithm": "RSA2048-PKCS1-v1_5-SHA256",
    "root_key_id": "1",
    "root_sha256": ROOT_SHA256,
    "manifest_generation": "5",
    "manifest_sha256": MANIFEST_SHA256,
    "active_key_epoch": "4",
    "product_key_policy_sha256": PRODUCT_KEY_POLICY_SHA256,
    "device_binding_sha256": DEVICE_BINDING_SHA256,
    "maintenance_policy_sha256": MAINTENANCE_POLICY_SHA256,
    "operations": "1",
    "operation_storage_rotation": "1",
    "max_uses": "2",
    "audit_state": "BNDRMAU1",
    "audit_state_version": "1",
    "audit_slots": "2",
    "audit_relative_lbas": "5/6",
    "reads": "4",
    "writes": "1",
    "flushes": "1",
    "authorization_attempts": "1",
    "authorization_successes": "1",
    "signature_rejections": "0",
    "binding_rejections": "0",
    "audit_attempts": "1",
    "audit_successes": "1",
    "audit_replay_rejections": "0",
    "audit_failures": "0",
    "session_scope": "boot-local-init-only",
    "session_open_calls": "3",
    "session_open_successes": "1",
    "session_argument_rejections": "1",
    "session_permission_rejections": "0",
    "session_state_rejections": "1",
    "session_opened": "1",
    "mutating_report_gate_denials": "1",
    "ui_convergence_query_read_only": "1",
    "report_gate_before_trace_mutation": "1",
    "exact_sequence": "1",
    "signed_before_audit": "1",
    "write_flush_readback": "1",
    "old_slot_preserved": "1",
    "external_artifact": "1",
    "separate_manifest_and_maintenance_roots": "1",
    "offline_split_signing_supported": "1",
    "private_key_in_repository": "0",
    "trusted_monotonic_backend": "0",
    "production_key_claim": "0",
    "hsm_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "erase_resistance": "0",
    "tamper_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

DATA_RECORD_MAGIC = b"BNDRREC1"
AUDIT_MAGIC = b"BNDRMAU1"
RECORD_COOKIE = 0x434F_4D4D_4954_2131
AUDIT_WITNESS = 0xB24D_77DA_7A5E_C001
AUDIT_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 5,
    fixture.DATA_PARTITION_FIRST_LBA + 6,
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


def slash_digest(digest: str) -> str:
    return "/".join(digest[index : index + 16] for index in range(0, 64, 16))


def phase_expected(phase: str) -> tuple[dict[str, str], dict[str, str]]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M77 phase: {phase}")
    audit = {
        **AUDIT_FIXED,
        **PHASE_AUDIT[phase],
        "authorization_sha256": slash_digest(AUTHORIZATION_SHA256[phase]),
        "authorization_id": slash_digest(AUTHORIZATION_ID[phase]),
        "chain_sha256": slash_digest(CHAIN_SHA256[phase]),
    }
    final = {
        **FINAL_FIXED,
        **PHASE_AUDIT[phase],
        "authorization_sha256": AUTHORIZATION_SHA256[phase],
        "authorization_id": AUTHORIZATION_ID[phase],
        "chain_sha256": CHAIN_SHA256[phase],
    }
    return audit, final


def validate_maintenance_markers(
    lines: list[str], phase: str
) -> tuple[dict[str, str], dict[str, str]]:
    expected_audit, expected_final = phase_expected(phase)
    audit = parse_marker(lines, AUDIT_PREFIX, AUDIT_KEYS)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    require(audit, expected_audit, AUDIT_PREFIX)
    require(final, expected_final, FINAL_PREFIX)
    for key in PHASE_AUDIT[phase]:
        if final[key] != audit[key]:
            raise EvidenceError(f"M77 audit/final field {key} diverged")
    for key in ("authorization_sha256", "authorization_id", "chain_sha256"):
        if final[key] != audit[key].replace("/", ""):
            raise EvidenceError(f"M77 audit/final digest {key} diverged")
    return audit, final


def normalize_to_m76(text: str) -> str:
    lines = [
        line
        for line in text.replace("\r", "").splitlines()
        if not line.startswith(AUDIT_PREFIX + " ")
        and not line.startswith(FINAL_PREFIX + " ")
    ]
    normalized = "\n".join(lines) + "\n"
    normalized = normalized.replace(UI_BOOT_MARKER, m76.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(SHUTDOWN_BOOT_MARKER, m76.SHUTDOWN_BOOT_MARKER, 1)
    return normalized.replace("abi=38", "abi=37")


def validate_positive(text: str, phase: str) -> dict[str, int]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M77 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(boots) != 2 or set(boots) != {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}:
        raise EvidenceError("M77 BOOT_OK marker set changed")
    validate_maintenance_markers(lines, phase)
    try:
        metrics = m76.validate_positive(normalize_to_m76(normalized), "steady")
    except ValueError as error:
        raise EvidenceError(str(error)) from error
    policy_index = lines.index(m73.unique_line(lines, m76.POLICY_PREFIX))
    audit_index = lines.index(m73.unique_line(lines, AUDIT_PREFIX))
    user_map_index = lines.index(m73.unique_line(lines, "USER_MAP_OK"))
    event_index = lines.index(m73.unique_line(lines, m73.EVENT_PREFIX))
    key_final_index = lines.index(m73.unique_line(lines, m76.FINAL_PREFIX))
    final_index = lines.index(m73.unique_line(lines, FINAL_PREFIX))
    shutdown_index = lines.index(m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK"))
    if not (
        policy_index < audit_index < user_map_index
        and event_index < key_final_index < final_index < shutdown_index
    ):
        raise EvidenceError("M77 verification, audit, publication, or shutdown order changed")
    return metrics


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={m73.FINAL_SCREEN_SHA256} "
        "maintenance_rotations=2 cancel_pending=4"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M77 {phase} host QMP self-exit evidence changed")


def negative_rejection(reason: str) -> str:
    if reason == "signature":
        return (
            "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=signature "
            "signature_valid=0 binding_valid=0 audit_attempted=0 "
            "audit_mutations=0 manifest_published=0 init_ready=0"
        )
    if reason == "binding":
        return (
            "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=binding "
            "signature_valid=1 binding_valid=0 audit_attempted=0 "
            "audit_mutations=0 manifest_published=0 init_ready=0"
        )
    if reason == "replay":
        return (
            "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=replay "
            "sequence=2 minimum=3 signature_valid=1 binding_valid=1 "
            "audit_attempted=1 audit_mutations=0 manifest_published=0 init_ready=0"
        )
    raise EvidenceError(f"unknown M77 rejection reason: {reason}")


def negative_driver(reason: str) -> str:
    signature_valid = int(reason != "signature")
    binding_valid = int(reason == "replay")
    audit_attempted = int(reason == "replay")
    return (
        "MAINTENANCE_AUTHORIZATION_NEGATIVE_BOOT_OK "
        f"reason={reason} pre_el0=1 signature_valid={signature_valid} "
        f"binding_valid={binding_valid} audit_attempted={audit_attempted} "
        "audit_slots_mutated=0 manifest_published=0 init_ready=0 "
        "qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
    )


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in NEGATIVE_REASONS:
        raise EvidenceError(f"unknown M77 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = negative_rejection(reason)
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M77 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("boot error: maintenance authorization preparation failed:")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M77 {reason} boot diagnostic changed")
    policy = m76.parse_marker(lines, m76.POLICY_PREFIX, m76.POLICY_KEYS)
    m76.require(policy, m76.POLICY_FIXED, m76.POLICY_PREFIX)
    m76.require(policy, m76.PHASE_POLICY["steady"], m76.POLICY_PREFIX)
    forbidden = (
        AUDIT_PREFIX + " ",
        "USER_MAP_OK ",
        "ELF_LOAD_OK ",
        "UNIFIED_PRODUCT_UI_OK ",
        "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        "BOOT_OK: M77 ",
    )
    if any(any(line.startswith(prefix) for prefix in forbidden) for line in lines):
        raise EvidenceError(f"M77 {reason} boot crossed the pre-EL0 boundary")
    if not (
        lines.index(m73.unique_line(lines, m76.POLICY_PREFIX))
        < lines.index(rejection)
        < lines.index(diagnostics[0])
    ):
        raise EvidenceError(f"M77 {reason} rejection/diagnostic order changed")
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [negative_driver(reason)]:
        raise EvidenceError(f"M77 {reason} negative host evidence changed")


def decode_hex(path: Path, expected_bytes: int | None = None) -> bytes:
    digits = "".join(path.read_text(encoding="ascii").split())
    if (
        not digits
        or len(digits) % 2
        or digits != digits.lower()
        or re.fullmatch(r"[0-9a-f]+", digits) is None
    ):
        raise EvidenceError(f"{path} is not canonical lowercase hexadecimal")
    payload = bytes.fromhex(digits)
    if expected_bytes is not None and len(payload) != expected_bytes:
        raise EvidenceError(f"{path} has {len(payload)} bytes, expected {expected_bytes}")
    return payload


def rsa_signature_valid(artifact: bytes) -> bool:
    signature_value = int.from_bytes(artifact[256:], "big")
    encoded = pow(
        signature_value, RSA_EXPONENT, int.from_bytes(ROOT_MODULUS, "big")
    ).to_bytes(256, "big")
    digest = hashlib.sha256(artifact[:256]).digest()
    expected = (
        bytes((0, 1))
        + bytes((0xFF,)) * 202
        + bytes((0,))
        + SHA256_DIGEST_INFO_PREFIX
        + digest
    )
    return encoded == expected


def computed_authorization_id(artifact: bytes) -> bytes:
    material = bytearray(132)
    material[:8] = b"BMA1-ID\0"
    material[8:16] = artifact[24:32]
    material[16:24] = artifact[32:40]
    material[24:28] = artifact[40:44]
    material[28:32] = artifact[44:48]
    material[32:36] = artifact[48:52]
    material[36:68] = artifact[64:96]
    material[68:100] = artifact[128:160]
    material[100:132] = artifact[160:192]
    return hashlib.sha256(material).digest()


def inspect_artifact(artifact: bytes) -> dict[str, object]:
    if len(artifact) != 512:
        raise EvidenceError("M77 BMA1 artifact length changed")
    if (
        artifact[:4] != b"BMA1"
        or artifact[4:8] != bytes((1, 1, 1, 0))
        or struct.unpack_from("<I", artifact, 8)[0] != 512
        or struct.unpack_from("<I", artifact, 12)[0] != 256
        or struct.unpack_from("<I", artifact, 16)[0] != 64
        or struct.unpack_from("<I", artifact, 20)[0] != 192
        or artifact[56:64] != bytes(8)
        or artifact[224:256] != bytes(32)
    ):
        raise EvidenceError("M77 BMA1 strict envelope changed")
    sequence = struct.unpack_from("<Q", artifact, 24)[0]
    operations = struct.unpack_from("<Q", artifact, 32)[0]
    max_uses = struct.unpack_from("<I", artifact, 40)[0]
    manifest_generation = struct.unpack_from("<I", artifact, 44)[0]
    active_key_epoch = struct.unpack_from("<I", artifact, 48)[0]
    audit_version = struct.unpack_from("<I", artifact, 52)[0]
    if (
        sequence == 0
        or operations != 1
        or max_uses != 2
        or manifest_generation != 5
        or active_key_epoch != 4
        or audit_version != 1
    ):
        raise EvidenceError("M77 BMA1 signed policy fields changed")
    return {
        "sequence": sequence,
        "manifest_sha256": artifact[64:96].hex(),
        "product_key_policy_sha256": artifact[96:128].hex(),
        "device_binding_sha256": artifact[128:160].hex(),
        "maintenance_policy_sha256": artifact[160:192].hex(),
        "authorization_id": artifact[192:224].hex(),
        "authorization_id_valid": artifact[192:224]
        == computed_authorization_id(artifact),
        "signed_sha256": hashlib.sha256(artifact[:256]).hexdigest(),
        "full_sha256": hashlib.sha256(artifact).hexdigest(),
        "signature_valid": rsa_signature_valid(artifact),
        "signed": artifact[:256],
        "signature": artifact[256:],
    }


def verify_artifacts(
    sequence1_path: Path,
    sequence2_path: Path,
    bad_signature_path: Path,
    wrong_binding_path: Path,
    request1_path: Path,
    signature1_path: Path,
    request2_path: Path,
    signature2_path: Path,
) -> str:
    if len(ROOT_MODULUS) != 256 or hashlib.sha256(ROOT_MODULUS).hexdigest() != ROOT_SHA256:
        raise EvidenceError("M77 maintenance trust anchor changed")
    sequence1 = inspect_artifact(decode_hex(sequence1_path, 512))
    sequence2 = inspect_artifact(decode_hex(sequence2_path, 512))
    bad_signature = inspect_artifact(decode_hex(bad_signature_path, 512))
    wrong_binding = inspect_artifact(decode_hex(wrong_binding_path, 512))
    for phase, artifact, sequence in (
        ("sequence1", sequence1, 1),
        ("sequence2", sequence2, 2),
    ):
        if (
            artifact["sequence"] != sequence
            or artifact["manifest_sha256"] != MANIFEST_SHA256
            or artifact["product_key_policy_sha256"] != PRODUCT_KEY_POLICY_SHA256
            or artifact["device_binding_sha256"] != DEVICE_BINDING_SHA256
            or artifact["maintenance_policy_sha256"] != MAINTENANCE_POLICY_SHA256
            or artifact["authorization_id"] != AUTHORIZATION_ID[phase]
            or artifact["authorization_id_valid"] is not True
            or artifact["signed_sha256"] != AUTHORIZATION_SHA256[phase]
            or artifact["full_sha256"] != FULL_ARTIFACT_SHA256[phase]
            or artifact["signature_valid"] is not True
        ):
            raise EvidenceError(f"M77 {phase} artifact contract changed")
    if (
        bad_signature["sequence"] != 2
        or bad_signature["signed"] != sequence2["signed"]
        or bad_signature["signature_valid"] is not False
    ):
        raise EvidenceError("M77 bad-signature fixture changed its signed region")
    if (
        wrong_binding["sequence"] != 3
        or wrong_binding["signature_valid"] is not True
        or wrong_binding["authorization_id_valid"] is not False
        or wrong_binding["manifest_sha256"]
        != ("da" + MANIFEST_SHA256[2:])
        or wrong_binding["product_key_policy_sha256"] != PRODUCT_KEY_POLICY_SHA256
        or wrong_binding["device_binding_sha256"] != DEVICE_BINDING_SHA256
        or wrong_binding["maintenance_policy_sha256"] != MAINTENANCE_POLICY_SHA256
    ):
        raise EvidenceError("M77 signed wrong-binding fixture changed")
    request1 = decode_hex(request1_path, 256)
    signature1 = decode_hex(signature1_path, 256)
    request2 = decode_hex(request2_path, 256)
    signature2 = decode_hex(signature2_path, 256)
    if (
        request1 != sequence1["signed"]
        or signature1 != sequence1["signature"]
        or request2 != sequence2["signed"]
        or signature2 != sequence2["signature"]
    ):
        raise EvidenceError("M77 offline request/signature split changed")
    return (
        "MAINTENANCE_AUTHORIZATION_ARTIFACTS_OK format=1 root_key_id=1 "
        f"root_sha256={ROOT_SHA256} manifest_sha256={MANIFEST_SHA256} "
        f"product_key_policy_sha256={PRODUCT_KEY_POLICY_SHA256} "
        f"device_binding_sha256={DEVICE_BINDING_SHA256} "
        f"maintenance_policy_sha256={MAINTENANCE_POLICY_SHA256} "
        "sequence1=1 sequence1_signature_valid=1 sequence2=2 "
        "sequence2_signature_valid=1 exact_sequence=1 max_uses=2 "
        "operations=1 bad_signature_valid=0 wrong_binding_signature_valid=1 "
        "wrong_binding_rejected_before_audit=1 offline_requests=2 "
        "request_bytes=256 detached_signature_bytes=256 offline_roundtrip=1 "
        "private_key_in_repository=0 separate_manifest_and_maintenance_roots=1 "
        "production_key_claim=0 trusted_monotonic_backend=0"
    )


def read_sector(path: Path, lba: int) -> bytes:
    return m76.read_sector(path, lba)


def decode_audit_record(sector: bytes) -> dict[str, object]:
    if (
        sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 320
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M77 audit record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M77 audit record commit proof changed")
    payload = sector[80:400]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[400:508] != bytes(108)
        or payload[:8] != AUDIT_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or payload[12:16] != bytes(4)
        or payload[44:48] != bytes(4)
        or payload[312:320] != bytes(8)
    ):
        raise EvidenceError("M77 audit state framing changed")
    sequence = struct.unpack_from("<Q", payload, 16)[0]
    accepted_count = struct.unpack_from("<Q", payload, 24)[0]
    operations = struct.unpack_from("<Q", payload, 32)[0]
    max_uses = struct.unpack_from("<I", payload, 40)[0]
    authorization_sha256 = payload[48:80]
    authorization_id = payload[80:112]
    manifest_sha256 = payload[112:144]
    policy_sha256 = payload[144:176]
    root_sha256 = payload[176:208]
    device_binding_sha256 = payload[208:240]
    previous_chain_sha256 = payload[240:272]
    chain_sha256 = payload[272:304]
    witness = struct.unpack_from("<Q", payload, 304)[0]
    chain_material = (
        AUDIT_MAGIC
        + previous_chain_sha256
        + authorization_sha256
        + authorization_id
        + struct.pack("<Q", sequence)
        + struct.pack("<Q", operations)
        + struct.pack("<I", max_uses)
        + manifest_sha256
    )
    if (
        sequence == 0
        or accepted_count != sequence
        or operations != 1
        or max_uses != 2
        or manifest_sha256.hex() != MANIFEST_SHA256
        or policy_sha256.hex() != MAINTENANCE_POLICY_SHA256
        or root_sha256.hex() != ROOT_SHA256
        or device_binding_sha256.hex() != DEVICE_BINDING_SHA256
        or hashlib.sha256(chain_material).digest() != chain_sha256
        or fixture.fnv1a64(payload[16:304]) ^ AUDIT_WITNESS != witness
    ):
        raise EvidenceError("M77 durable audit binding or hash chain changed")
    return {
        "generation": generation,
        "sequence": sequence,
        "accepted_count": accepted_count,
        "authorization_sha256": authorization_sha256.hex(),
        "authorization_id": authorization_id.hex(),
        "previous_chain_sha256": previous_chain_sha256.hex(),
        "chain_sha256": chain_sha256.hex(),
    }


def verify_reboot(
    base_disk: Path,
    sequence1_disk: Path,
    sequence2_disk: Path,
    signature_disk: Path,
    binding_disk: Path,
    replay_disk: Path,
    sequence1_log: Path,
    sequence1_driver: Path,
    sequence2_log: Path,
    sequence2_driver: Path,
    signature_log: Path,
    signature_driver: Path,
    binding_log: Path,
    binding_driver: Path,
    replay_log: Path,
    replay_driver: Path,
) -> str:
    metrics = []
    for phase, log_path, driver_path in (
        ("sequence1", sequence1_log, sequence1_driver),
        ("sequence2", sequence2_log, sequence2_driver),
    ):
        metrics.append(
            validate_positive(
                log_path.read_text(encoding="utf-8", errors="replace"), phase
            )
        )
        validate_driver(
            driver_path.read_text(encoding="utf-8", errors="replace"), phase
        )
    for reason, log_path, driver_path in (
        ("signature", signature_log, signature_driver),
        ("binding", binding_log, binding_driver),
        ("replay", replay_log, replay_driver),
    ):
        validate_negative(
            log_path.read_text(encoding="utf-8", errors="replace"),
            driver_path.read_text(encoding="utf-8", errors="replace"),
            reason,
        )

    base_audit = tuple(read_sector(base_disk, lba) for lba in AUDIT_LBAS)
    sequence1_audit = tuple(read_sector(sequence1_disk, lba) for lba in AUDIT_LBAS)
    sequence2_audit = tuple(read_sector(sequence2_disk, lba) for lba in AUDIT_LBAS)
    if any(slot != bytes(fixture.SECTOR_BYTES) for slot in base_audit):
        raise EvidenceError("M77 base audit slots were not pristine")
    first = decode_audit_record(sequence1_audit[0])
    if (
        first["generation"] != 1
        or first["sequence"] != 1
        or first["accepted_count"] != 1
        or first["authorization_sha256"] != AUTHORIZATION_SHA256["sequence1"]
        or first["authorization_id"] != AUTHORIZATION_ID["sequence1"]
        or first["previous_chain_sha256"] != bytes(32).hex()
        or first["chain_sha256"] != CHAIN_SHA256["sequence1"]
        or sequence1_audit[1] != bytes(fixture.SECTOR_BYTES)
    ):
        raise EvidenceError("M77 sequence-one audit commit changed")
    first_after = decode_audit_record(sequence2_audit[0])
    second = decode_audit_record(sequence2_audit[1])
    if (
        sequence2_audit[0] != sequence1_audit[0]
        or first_after != first
        or second["generation"] != 2
        or second["sequence"] != 2
        or second["accepted_count"] != 2
        or second["authorization_sha256"] != AUTHORIZATION_SHA256["sequence2"]
        or second["authorization_id"] != AUTHORIZATION_ID["sequence2"]
        or second["previous_chain_sha256"] != CHAIN_SHA256["sequence1"]
        or second["chain_sha256"] != CHAIN_SHA256["sequence2"]
    ):
        raise EvidenceError("M77 sequence-two audit chain changed")
    for rejected_disk in (signature_disk, binding_disk, replay_disk):
        slots = tuple(read_sector(rejected_disk, lba) for lba in AUDIT_LBAS)
        if slots != sequence2_audit:
            raise EvidenceError("M77 rejected boot mutated an audit slot")

    base_policy = tuple(read_sector(base_disk, lba) for lba in m76.POLICY_LBAS)
    decoded_policy = tuple(m76.decode_policy_record(slot) for slot in base_policy)
    if [item["generation"] for item in decoded_policy] != [3, 4]:
        raise EvidenceError("M77 did not begin from the replicated key4 policy")
    for disk in (
        sequence1_disk,
        sequence2_disk,
        signature_disk,
        binding_disk,
        replay_disk,
    ):
        if tuple(read_sector(disk, lba) for lba in m76.POLICY_LBAS) != base_policy:
            raise EvidenceError("M77 boot mutated the inherited key policy")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_REBOOT_OK format=1 abi=38 "
        "maintenance_sessions=2 signed_authorizations=5 signature_valid=4 "
        "signature_rejections=1 binding_rejections=1 replay_rejections=1 "
        "sequence1=1 sequence2=2 committed_sequence=2 accepted_count=2 "
        "audit_reads=8 audit_writes=2 audit_flushes=2 slots=2 "
        "relative_lbas=5/6 slot_generations=1/2 crc_verified=2 "
        "witness_verified=2 hash_chain_verified=2 old_slot_preserved=1 "
        "exact_sequence=1 rejected_boot_slot_mutations=0 "
        "fail_closed_pre_el0_boots=3 manifests_published_after_rejection=0 "
        "session_open_calls=6 session_open_successes=2 "
        "session_argument_rejections=2 session_state_rejections=2 "
        "mutating_report_gate_denials=2 clean_rotations=4 "
        f"ui_query_calls={sum(item['ui_query_calls'] for item in metrics)} "
        f"ui_query_waits={sum(item['ui_query_waits'] for item in metrics)} "
        "ui_query_successes=2 qemu_psci_self_exits=2 semihosting_uses=0 "
        "qemu_disk_audit=1 trusted_monotonic_backend=0 fixture_root=1 "
        "production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0 "
        "erase_resistance=0 tamper_resistance=0 hardware_powercut_claim=0 "
        "emulator_only=1 real_phone_claim=0"
    )


def marker(prefix: str, keys: tuple[str, ...], fields: dict[str, str]) -> str:
    return prefix + " " + " ".join(f"{key}={fields[key]}" for key in keys)


def self_test() -> str:
    root = Path(__file__).resolve().parent.parent
    artifact_marker = verify_artifacts(
        root / "boot/maintenance-authorization-sequence1.bma1.hex",
        root / "boot/maintenance-authorization-sequence2.bma1.hex",
        root
        / "boot/test-fixtures/maintenance-authorization-sequence2-bad-signature.bma1.hex",
        root
        / "boot/test-fixtures/maintenance-authorization-sequence3-wrong-binding.bma1.hex",
        root / "boot/maintenance/maintenance-authorization-sequence1.request.hex",
        root / "boot/maintenance/maintenance-authorization-sequence1.signature.hex",
        root / "boot/maintenance/maintenance-authorization-sequence2.request.hex",
        root / "boot/maintenance/maintenance-authorization-sequence2.signature.hex",
    )
    if not artifact_marker.startswith("MAINTENANCE_AUTHORIZATION_ARTIFACTS_OK "):
        raise EvidenceError("M77 artifact self-test marker changed")
    positive_rejections = 0
    for phase in PHASES:
        audit, final = phase_expected(phase)
        lines = [
            marker(AUDIT_PREFIX, AUDIT_KEYS, audit),
            marker(FINAL_PREFIX, FINAL_KEYS, final),
        ]
        validate_maintenance_markers(lines, phase)
        mutated = lines.copy()
        mutated[0] = mutated[0].replace(
            "trusted_monotonic_backend=0", "trusted_monotonic_backend=1"
        )
        try:
            validate_maintenance_markers(mutated, phase)
        except EvidenceError:
            positive_rejections += 1
        else:
            raise EvidenceError("M77 marker parser accepted a false hardware claim")
    negative_rejections = 0
    policy = m76.policy_marker("steady")
    for reason in NEGATIVE_REASONS:
        diagnostic = (
            "boot error: maintenance authorization preparation failed: "
            "synthetic self-test diagnostic"
        )
        text = "\n".join((policy, negative_rejection(reason), diagnostic)) + "\n"
        driver = negative_driver(reason) + "\n"
        validate_negative(text, driver, reason)
        try:
            validate_negative(text, driver.replace("pre_el0=1", "pre_el0=0"), reason)
        except EvidenceError:
            negative_rejections += 1
        else:
            raise EvidenceError("M77 negative parser accepted a mutated host claim")
    return (
        "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive_rejections={positive_rejections} "
        f"negative_rejections={negative_rejections} artifact_profiles=4"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")

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

    positive = subparsers.add_parser("parse")
    positive.add_argument("phase", choices=PHASES)
    positive.add_argument("log", type=Path)

    negative = subparsers.add_parser("parse-negative")
    negative.add_argument("reason", choices=NEGATIVE_REASONS)
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)

    reboot = subparsers.add_parser("reboot")
    for name in (
        "base_disk",
        "sequence1_disk",
        "sequence2_disk",
        "signature_disk",
        "binding_disk",
        "replay_disk",
        "sequence1_log",
        "sequence1_driver",
        "sequence2_log",
        "sequence2_driver",
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
        elif args.command == "artifacts":
            print(
                verify_artifacts(
                    args.sequence1,
                    args.sequence2,
                    args.bad_signature,
                    args.wrong_binding,
                    args.request1,
                    args.signature1,
                    args.request2,
                    args.signature2,
                )
            )
        elif args.command == "parse":
            validate_positive(
                args.log.read_text(encoding="utf-8", errors="replace"), args.phase
            )
            print(
                f"UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_EVIDENCE_OK phase={args.phase}"
            )
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason}"
            )
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.base_disk,
                    args.sequence1_disk,
                    args.sequence2_disk,
                    args.signature_disk,
                    args.binding_disk,
                    args.replay_disk,
                    args.sequence1_log,
                    args.sequence1_driver,
                    args.sequence2_log,
                    args.sequence2_driver,
                    args.signature_log,
                    args.signature_driver,
                    args.binding_log,
                    args.binding_driver,
                    args.replay_log,
                    args.replay_driver,
                )
            )
        else:
            raise AssertionError(args.command)
    except (EvidenceError, OSError) as error:
        print(f"evidence error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
