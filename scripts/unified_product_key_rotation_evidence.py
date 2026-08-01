#!/usr/bin/env python3
"""Strict M76 key-rotation, revocation, and multi-boot evidence parser."""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import sys
from pathlib import Path

import build_storage_image as fixture
import unified_product_event_supervision_evidence as m73
import unified_product_persistent_rollback_evidence as m75


PHASES = ("transition", "activate", "repair", "steady")
POLICY_SHA256 = "30676f357683b6c8d2c5d67755e48beb93bdfe8864eccc67c716a1785e935e01"
KEY3_SHA256 = "c36a7d09c6eabeef6d307827ca0d66a91ffb50396904f7a1f677ae41720c3c9e"
KEY4_SHA256 = "36b7c88dc40ba32c77729d04ac7ceb90a21691d39abe94b17b12b1242f33c52f"
TRANSITION_SIGNED_SHA256 = (
    "38dd4e4df4be7827606dbd4da9fe527fdef0f2c536cb871141fddfde764e5ea3"
)
ACTIVE_SIGNED_SHA256 = (
    "5a8ccd0ae601dad43356782ed0cc803fc3fe1232d3f42be981a71d94ca33e988"
)
RETIRED_SIGNED_SHA256 = (
    "63afbf2e750780e6b6e401f352978b22ee1bedb2b7c1e346428ae67c785fd62d"
)
KEY3_MODULUS = bytes.fromhex(
    "caa12eeee9786123c8bde4fdab27a7200e328d5c709da83d0c953d2055247791"
    "05ad5c1cefb5fe06606c5104e58d8b561feda5ad22cdc38ce6fdbae2ec533e0f"
    "1a6380119340f93e72c10adfa1778de320e27339914e5a7cb4fea3e2ae82544c"
    "66cc96e374e60ecb40790449b1c4be9dc16038520ed2c7433728e4477fb9d1c8"
    "5b059998c3c3c85531454733046bba62126d4d6c0ef5b1285ef7e49f36d9719"
    "f0107b504875c9649c443df475d00149cb96c4060fdd63ad7257148df1a7ddc1"
    "9b11ff614cfc58a2b97e17f32f02c27ac5cf7996df64aab57926edd75ac5459"
    "bfd6949f12846085cd38db7614bf19012915aef5f1332c8f86b0886bae0d8458cb"
)
KEY4_MODULUS = bytes.fromhex(
    "bd77fae2b4818869cd6eb329d58550cccb7fae97a1267b26e194ca8cae5cb7f9"
    "0f1718f95392f7cd328ed6dcca18e4fccf4ad5e437a34c0fc6fd6bbdad73ccb9"
    "537cca48af9a474d25d6895d2254785a59deef5a4d0140bb70160865e2bcc1c4"
    "8cfff78de009bee16cf5f75be4450155c7a3446a128406e66e07cdabbbbf0e6a"
    "bac9ea26423384b99343ad38c7e5b709ed977aefc87e201badd68f6bad19e7c1"
    "253b996412caaec22e8edf0e4c49d343bf35c7ad263766bea9cab0c1a53687a"
    "66e8443b556c4cd98a3e38032bf60ddabeb42bd4cb9e11dece25a7c6e1f343d"
    "9373202a9f3b3b10f938088be94af9ff34d4e303a083970b6e2241c1d38bdd5907"
)
RSA_EXPONENT = 65537
SHA256_DIGEST_INFO_PREFIX = bytes.fromhex(
    "3031300d060960864801650304020105000420"
)

AUTHORITY = (
    "external-bms1-plus-kernel-pinned-ordered-keyring-plus-qemu-disk-key-"
    "epoch-and-rollback-floor-plus-init-event-loop-plus-kernel-authenticated-report"
)
UI_BOOT_MARKER = (
    "BOOT_OK: M76 unified real UI is interactive; persistent key-policy event "
    "supervision is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M76 persistent key rotation and revocation, offline-signing "
    "artifact, event supervision, AppData, and PSCI shutdown armed"
)
UI_MARKER = m73.UI_MARKER.replace("abi=34", "abi=37", 1)
DISCOVERY_MARKER = m73.DISCOVERY_MARKER.replace("abi=34", "abi=37", 1)
PSCI_SHUTDOWN_MARKER = m73.PSCI_SHUTDOWN_MARKER.replace("abi=34", "abi=37", 1)
FAILURE_PATTERN = re.compile(
    m73.FAILURE_PATTERN.pattern + r"|KEY_ROTATION_REJECTED",
    m73.FAILURE_PATTERN.flags,
)

POLICY_PREFIX = "KEY_ROTATION_POLICY_OK"
POLICY_KEYS = (
    "format",
    "state",
    "state_version",
    "authority",
    "keyring_keys",
    "policy_sha256",
    "previous_key_id",
    "previous_key_epoch",
    "active_key_id",
    "active_key_epoch",
    "legacy_migrated",
    "key_transition",
    "bootstrap_floor",
    "persisted_floor_before",
    "effective_floor",
    "artifact_index",
    "committed_floor",
    "initial_generation",
    "committed_generation",
    "initial_slot",
    "committed_slot",
    "initial_valid_slots",
    "initial_rejected_slots",
    "initial_active_policy_slots",
    "committed_active_policy_slots",
    "floor_advanced",
    "record_written",
    "redundancy_repaired",
    "reads",
    "writes",
    "flushes",
    "manifest_published",
    "init_ready",
    "staged_upgrade_required",
    "fixture_keys",
    "production_key_claim",
    "hsm_claim",
    "rpmb_claim",
    "efuse_claim",
    "host_rollback_resistance",
    "hardware_powercut_claim",
    "emulator_only",
)
POLICY_FIXED = {
    "format": "1",
    "state": "BNDRKEY1",
    "state_version": "1",
    "authority": "kernel-pre-el0-qemu-disk",
    "keyring_keys": "3",
    "policy_sha256": POLICY_SHA256,
    "bootstrap_floor": "2",
    "manifest_published": "0",
    "init_ready": "0",
    "staged_upgrade_required": "1",
    "fixture_keys": "1",
    "production_key_claim": "0",
    "hsm_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "host_rollback_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
}
PHASE_POLICY = {
    "transition": {
        "previous_key_id": "2",
        "previous_key_epoch": "2",
        "active_key_id": "3",
        "active_key_epoch": "3",
        "legacy_migrated": "1",
        "key_transition": "1",
        "persisted_floor_before": "3",
        "effective_floor": "3",
        "artifact_index": "4",
        "committed_floor": "4",
        "initial_generation": "1",
        "committed_generation": "2",
        "initial_slot": "0",
        "committed_slot": "1",
        "initial_valid_slots": "1",
        "initial_rejected_slots": "1",
        "initial_active_policy_slots": "0",
        "committed_active_policy_slots": "1",
        "floor_advanced": "1",
        "record_written": "1",
        "redundancy_repaired": "0",
        "reads": "4",
        "writes": "1",
        "flushes": "1",
    },
    "activate": {
        "previous_key_id": "3",
        "previous_key_epoch": "3",
        "active_key_id": "4",
        "active_key_epoch": "4",
        "legacy_migrated": "0",
        "key_transition": "1",
        "persisted_floor_before": "4",
        "effective_floor": "4",
        "artifact_index": "5",
        "committed_floor": "5",
        "initial_generation": "2",
        "committed_generation": "3",
        "initial_slot": "1",
        "committed_slot": "0",
        "initial_valid_slots": "2",
        "initial_rejected_slots": "0",
        "initial_active_policy_slots": "0",
        "committed_active_policy_slots": "1",
        "floor_advanced": "1",
        "record_written": "1",
        "redundancy_repaired": "0",
        "reads": "4",
        "writes": "1",
        "flushes": "1",
    },
    "repair": {
        "previous_key_id": "4",
        "previous_key_epoch": "4",
        "active_key_id": "4",
        "active_key_epoch": "4",
        "legacy_migrated": "0",
        "key_transition": "0",
        "persisted_floor_before": "5",
        "effective_floor": "5",
        "artifact_index": "5",
        "committed_floor": "5",
        "initial_generation": "3",
        "committed_generation": "4",
        "initial_slot": "0",
        "committed_slot": "1",
        "initial_valid_slots": "2",
        "initial_rejected_slots": "0",
        "initial_active_policy_slots": "1",
        "committed_active_policy_slots": "2",
        "floor_advanced": "0",
        "record_written": "1",
        "redundancy_repaired": "1",
        "reads": "4",
        "writes": "1",
        "flushes": "1",
    },
    "steady": {
        "previous_key_id": "4",
        "previous_key_epoch": "4",
        "active_key_id": "4",
        "active_key_epoch": "4",
        "legacy_migrated": "0",
        "key_transition": "0",
        "persisted_floor_before": "5",
        "effective_floor": "5",
        "artifact_index": "5",
        "committed_floor": "5",
        "initial_generation": "4",
        "committed_generation": "4",
        "initial_slot": "1",
        "committed_slot": "1",
        "initial_valid_slots": "2",
        "initial_rejected_slots": "0",
        "initial_active_policy_slots": "2",
        "committed_active_policy_slots": "2",
        "floor_advanced": "0",
        "record_written": "0",
        "redundancy_repaired": "0",
        "reads": "2",
        "writes": "0",
        "flushes": "0",
    },
}

FINAL_PREFIX = "UNIFIED_PRODUCT_KEY_ROTATION_OK"
FINAL_KEYS = (
    "format",
    "abi",
    "artifact",
    "artifact_bytes",
    "signed_bytes",
    "payload",
    "payload_bytes",
    "manifest_generation",
    "algorithm",
    "keyring_keys",
    "key_id",
    "key_epoch",
    "previous_key_id",
    "previous_key_epoch",
    "policy_sha256",
    "rollback_index",
    "bootstrap_floor",
    "persisted_floor_before",
    "effective_floor",
    "committed_floor",
    "legacy_migrated",
    "key_transition",
    "provisioned_before",
    "slots",
    "relative_lbas",
    "initial_generation",
    "committed_generation",
    "initial_slot",
    "committed_slot",
    "initial_valid_slots",
    "initial_rejected_slots",
    "initial_active_policy_slots",
    "committed_active_policy_slots",
    "floor_advanced",
    "record_written",
    "redundancy_repaired",
    "reads",
    "writes",
    "flushes",
    "ledger_attempts",
    "ledger_successes",
    "ledger_rollback_rejections",
    "ledger_failures",
    "retired_key_rejections",
    "verification_attempts",
    "verification_successes",
    "signature_successes",
    "signature_rejections",
    "rollback_rejections",
    "format_rejections",
    "signed_sha256",
    "external_artifact",
    "kernel_pinned_fixture_keyring",
    "staged_upgrade_required",
    "double_slot_crc",
    "write_flush_readback",
    "old_slot_preserved",
    "manifest_published_after_policy_commit",
    "offline_split_signing_supported",
    "private_key_in_repository",
    "production_key_claim",
    "hsm_claim",
    "rpmb_claim",
    "efuse_claim",
    "host_rollback_resistance",
    "erase_resistance",
    "tamper_resistance",
    "hardware_powercut_claim",
    "emulator_only",
    "real_phone_claim",
)
FINAL_FIXED = {
    "format": "1",
    "abi": "37",
    "artifact": "BMS1",
    "artifact_bytes": "512",
    "signed_bytes": "256",
    "payload": "BMF1",
    "payload_bytes": "224",
    "algorithm": "RSA2048-PKCS1-v1_5-SHA256",
    "keyring_keys": "3",
    "policy_sha256": POLICY_SHA256,
    "bootstrap_floor": "2",
    "provisioned_before": "1",
    "slots": "2",
    "relative_lbas": "3/4",
    "ledger_attempts": "1",
    "ledger_successes": "1",
    "ledger_rollback_rejections": "0",
    "ledger_failures": "0",
    "retired_key_rejections": "0",
    "verification_attempts": "1",
    "verification_successes": "1",
    "signature_successes": "1",
    "signature_rejections": "0",
    "rollback_rejections": "0",
    "format_rejections": "0",
    "external_artifact": "1",
    "kernel_pinned_fixture_keyring": "1",
    "staged_upgrade_required": "1",
    "double_slot_crc": "1",
    "write_flush_readback": "1",
    "old_slot_preserved": "1",
    "manifest_published_after_policy_commit": "1",
    "offline_split_signing_supported": "1",
    "private_key_in_repository": "0",
    "production_key_claim": "0",
    "hsm_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "host_rollback_resistance": "0",
    "erase_resistance": "0",
    "tamper_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}
PHASE_FINAL = {
    phase: {
        **{
            "manifest_generation": values["artifact_index"],
            "key_id": values["active_key_id"],
            "key_epoch": values["active_key_epoch"],
            "rollback_index": values["artifact_index"],
            "signed_sha256": (
                TRANSITION_SIGNED_SHA256
                if phase == "transition"
                else ACTIVE_SIGNED_SHA256
            ),
        },
        **{
            key: value
            for key, value in values.items()
            if key not in {"active_key_id", "active_key_epoch", "artifact_index"}
        },
    }
    for phase, values in PHASE_POLICY.items()
}

SIGNATURE_REJECTION = (
    "KEY_ROTATION_REJECTED format=1 reason=signature key_id=4 "
    "signature_valid=0 manifest_published=0 init_ready=0"
)
RETIRED_REJECTION = (
    "KEY_ROTATION_REJECTED format=1 reason=retired-key key_id=3 key_epoch=3 "
    "active_epoch=4 artifact_index=6 signature_valid=1 "
    "manifest_published=0 init_ready=0"
)
NEGATIVE_DRIVER = (
    "KEY_ROTATION_NEGATIVE_BOOT_OK reason={reason} pre_el0=1 "
    "signature_valid={signature_valid} manifest_published=0 init_ready=0 "
    "policy_slots_mutated=0 qemu_terminated_by_host=1 emulator_only=1 "
    "real_phone_claim=0"
)

DATA_RECORD_MAGIC = b"BNDRREC1"
POLICY_MAGIC = b"BNDRKEY1"
RECORD_COOKIE = 0x434F_4D4D_4954_2131
POLICY_WITNESS = 0xB24D_76DA_7A5E_C001
POLICY_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 3,
    fixture.DATA_PARTITION_FIRST_LBA + 4,
)


class EvidenceError(ValueError):
    pass


def parse_marker(lines: list[str], prefix: str, keys: tuple[str, ...]) -> dict[str, str]:
    try:
        return m73.parse_fields(m73.unique_line(lines, prefix), prefix, keys)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def require(fields: dict[str, str], expected: dict[str, str], prefix: str) -> None:
    try:
        m73.require_fixed(fields, expected, prefix)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error


def validate_policy_markers(
    lines: list[str], phase: str
) -> tuple[dict[str, str], dict[str, str]]:
    policy = parse_marker(lines, POLICY_PREFIX, POLICY_KEYS)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    require(policy, POLICY_FIXED, POLICY_PREFIX)
    require(policy, PHASE_POLICY[phase], POLICY_PREFIX)
    require(final, FINAL_FIXED, FINAL_PREFIX)
    require(final, PHASE_FINAL[phase], FINAL_PREFIX)
    mapping = {
        "active_key_id": "key_id",
        "active_key_epoch": "key_epoch",
        "artifact_index": "rollback_index",
    }
    for key in PHASE_POLICY[phase]:
        final_key = mapping.get(key, key)
        if final.get(final_key) != policy[key]:
            raise EvidenceError(f"M76 policy/final field {key} diverged")
    if lines.index(m73.unique_line(lines, POLICY_PREFIX)) >= lines.index(
        m73.unique_line(lines, "USER_MAP_OK")
    ):
        raise EvidenceError("M76 key policy was not committed before EL0 mapping")
    if not (
        lines.index(m73.unique_line(lines, m73.EVENT_PREFIX))
        < lines.index(m73.unique_line(lines, FINAL_PREFIX))
        < lines.index(m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK"))
    ):
        raise EvidenceError("M76 event/key-policy/shutdown marker order changed")
    return policy, final


def normalize_to_m73(text: str, generation: int) -> str:
    lines = [
        line
        for line in text.replace("\r", "").splitlines()
        if not line.startswith(POLICY_PREFIX + " ")
        and not line.startswith(FINAL_PREFIX + " ")
    ]
    normalized = "\n".join(lines) + "\n"
    normalized = normalized.replace(UI_BOOT_MARKER, m73.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(
        SHUTDOWN_BOOT_MARKER, m73.SHUTDOWN_BOOT_MARKER, 1
    )
    normalized = normalized.replace(AUTHORITY, m75.OLD_AUTHORITY, 1)
    normalized = normalized.replace(
        f"manifest_generation={generation}", "manifest_generation=1", 1
    )
    return normalized.replace("abi=37", "abi=34")


def validate_steady_runtime(lines: list[str], generation: int) -> dict[str, int]:
    if m73.unique_line(lines, "M73_PSCI_DISCOVERY_OK") != DISCOVERY_MARKER:
        raise EvidenceError("M76 steady PSCI discovery marker changed")
    if m73.unique_line(lines, "UNIFIED_PRODUCT_UI_OK") != UI_MARKER:
        raise EvidenceError("M76 steady UI marker changed")
    if (
        m73.unique_line(lines, "M73_EVENT_SUPERVISOR_ACTIVE_OK")
        != m73.active_marker().replace("abi=34", "abi=37", 1)
    ):
        raise EvidenceError("M76 steady event activation marker changed")
    for ordinal in (1, 2):
        expected = m73.rotation_marker(ordinal).replace("abi=34", "abi=37", 1)
        if expected not in lines:
            raise EvidenceError(f"M76 steady rotation {ordinal} marker changed")
    cancel = m73.parse_fields(
        m73.unique_line(lines, "M73_CANCEL_WINDOW_OK"),
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
    m73.require_fixed(
        cancel,
        {
            "format": "1",
            "abi": "37",
            "pending": "4",
            "rotations": "2",
            "external_power": "awaiting",
            "drain_after_stop": "1",
        },
        "M73_CANCEL_WINDOW_OK",
    )
    cancel_batch = m73.decimal(cancel, "batch", "M73_CANCEL_WINDOW_OK")
    event = m73.unique_line(lines, m73.EVENT_PREFIX)
    normalized_event = (
        event.replace("abi=37", "abi=34", 1)
        .replace(AUTHORITY, m75.OLD_AUTHORITY, 1)
        .replace(f"manifest_generation={generation}", "manifest_generation=1", 1)
    )
    try:
        metrics = m73.validate_event(normalized_event, 3, cancel_batch)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error
    shutdown = m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK")
    for required in (
        "format=1",
        "abi=37",
        "bounded_shutdown=1",
        "current_boot_open=0",
        "admission_closed=1",
        "post_close_storage_mutations=0",
        "emulator_backend=qemu-psci",
        "hardware_poweroff_claim=0",
        "real_phone_claim=0",
    ):
        if required not in shutdown.split():
            raise EvidenceError(f"M76 steady shutdown field changed: {required}")
    if (
        m73.unique_line(lines, "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK")
        != PSCI_SHUTDOWN_MARKER
    ):
        raise EvidenceError("M76 steady PSCI shutdown marker changed")
    return metrics


def validate_positive(text: str, phase: str) -> dict[str, int]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M76 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    if "abi=36" in normalized or "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK " in normalized:
        raise EvidenceError("M76 retained an M75 runtime marker")
    lines = normalized.splitlines()
    boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(boots) != 2 or set(boots) != {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}:
        raise EvidenceError("M76 BOOT_OK marker set changed")
    validate_policy_markers(lines, phase)
    generation = int(PHASE_POLICY[phase]["artifact_index"])
    if phase == "transition":
        try:
            return m73.validate_text(normalize_to_m73(normalized, generation), "second")
        except ValueError as error:
            raise EvidenceError(str(error)) from error
    return validate_steady_runtime(lines, generation)


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={m73.FINAL_SCREEN_SHA256} "
        "maintenance_rotations=2 cancel_pending=4"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M76 {phase} host QMP self-exit evidence changed")


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in {"signature", "retired-key"}:
        raise EvidenceError(f"unknown M76 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = SIGNATURE_REJECTION if reason == "signature" else RETIRED_REJECTION
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M76 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("boot error: manifest key-rotation preparation failed:")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M76 {reason} boot diagnostic changed")
    forbidden = (
        POLICY_PREFIX + " ",
        "USER_MAP_OK ",
        "ELF_LOAD_OK ",
        "UNIFIED_PRODUCT_UI_OK ",
        "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        "BOOT_OK: M76 ",
    )
    if any(any(line.startswith(prefix) for prefix in forbidden) for line in lines):
        raise EvidenceError(f"M76 {reason} boot crossed the pre-EL0 boundary")
    if lines.index(rejection) >= lines.index(diagnostics[0]):
        raise EvidenceError(f"M76 {reason} rejection/diagnostic order changed")
    expected_driver = NEGATIVE_DRIVER.format(
        reason=reason, signature_valid=int(reason == "retired-key")
    )
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [expected_driver]:
        raise EvidenceError(f"M76 {reason} negative host evidence changed")


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


def inspect_artifact(artifact: bytes, modulus: bytes) -> dict[str, object]:
    if len(artifact) != 512:
        raise EvidenceError("M76 BMS1 artifact length changed")
    key_id = artifact[6]
    if (
        artifact[:4] != b"BMS1"
        or artifact[4:6] != bytes((1, 1))
        or artifact[7] != 0
        or int.from_bytes(artifact[8:12], "little") != 512
        or int.from_bytes(artifact[12:16], "little") != 256
        or int.from_bytes(artifact[16:20], "little") != 32
        or int.from_bytes(artifact[20:24], "little") != 224
        or artifact[28:32] != bytes(4)
    ):
        raise EvidenceError("M76 BMS1 strict envelope changed")
    signed = artifact[:256]
    payload = artifact[32:256]
    rollback_index = int.from_bytes(artifact[24:28], "little")
    if (
        payload[:4] != b"BMF1"
        or payload[4:8] != bytes((1, 32, 32, 8))
        or int.from_bytes(payload[8:12], "little") != 224
        or payload[16:18] != bytes((5, 4))
        or payload[18:24] != bytes(6)
    ):
        raise EvidenceError("M76 signed BMF1 payload header changed")
    generation = int.from_bytes(payload[12:16], "little")
    if generation != rollback_index:
        raise EvidenceError("M76 rollback index no longer matches BMF1 generation")
    signature_value = int.from_bytes(artifact[256:], "big")
    encoded = pow(signature_value, RSA_EXPONENT, int.from_bytes(modulus, "big")).to_bytes(
        256, "big"
    )
    digest = hashlib.sha256(signed).digest()
    expected = (
        bytes((0, 1))
        + bytes((0xFF,)) * 202
        + bytes((0,))
        + SHA256_DIGEST_INFO_PREFIX
        + digest
    )
    return {
        "generation": generation,
        "rollback_index": rollback_index,
        "key_id": key_id,
        "signed_sha256": digest.hex(),
        "signature_valid": encoded == expected,
        "signed": signed,
        "signature": artifact[256:],
    }


def verify_artifacts(
    transition_path: Path,
    active_path: Path,
    retired_path: Path,
    bad_path: Path,
    request_path: Path,
    signature_path: Path,
) -> str:
    if hashlib.sha256(KEY3_MODULUS).hexdigest() != KEY3_SHA256:
        raise EvidenceError("M76 key3 trust anchor changed")
    if hashlib.sha256(KEY4_MODULUS).hexdigest() != KEY4_SHA256:
        raise EvidenceError("M76 key4 trust anchor changed")
    material = (
        bytes((2,))
        + hashlib.sha256(m75.RSA_MODULUS).digest()
        + bytes((3,))
        + hashlib.sha256(KEY3_MODULUS).digest()
        + bytes((4,))
        + hashlib.sha256(KEY4_MODULUS).digest()
    )
    if hashlib.sha256(material).hexdigest() != POLICY_SHA256:
        raise EvidenceError("M76 ordered keyring policy digest changed")

    transition = inspect_artifact(decode_hex(transition_path, 512), KEY3_MODULUS)
    active = inspect_artifact(decode_hex(active_path, 512), KEY4_MODULUS)
    retired = inspect_artifact(decode_hex(retired_path, 512), KEY3_MODULUS)
    bad = inspect_artifact(decode_hex(bad_path, 512), KEY4_MODULUS)
    expected = (
        (transition, 4, 3, TRANSITION_SIGNED_SHA256, True),
        (active, 5, 4, ACTIVE_SIGNED_SHA256, True),
        (retired, 6, 3, RETIRED_SIGNED_SHA256, True),
        (bad, 5, 4, ACTIVE_SIGNED_SHA256, False),
    )
    for artifact, generation, key_id, digest, signature_valid in expected:
        if (
            artifact["generation"] != generation
            or artifact["rollback_index"] != generation
            or artifact["key_id"] != key_id
            or artifact["signed_sha256"] != digest
            or artifact["signature_valid"] is not signature_valid
        ):
            raise EvidenceError("M76 signed artifact contract changed")
    if bad["signed"] != active["signed"]:
        raise EvidenceError("M76 bad-signature fixture mutated its signed region")
    request = decode_hex(request_path, 256)
    signature = decode_hex(signature_path, 256)
    if request != active["signed"] or signature != active["signature"]:
        raise EvidenceError("M76 offline request/signature split changed")
    return (
        "KEY_ROTATION_ARTIFACTS_OK format=1 keyring_keys=3 "
        f"policy_sha256={POLICY_SHA256} key2_id=2 key3_id=3 key4_id=4 "
        "transition_generation=4 transition_signature_valid=1 "
        "active_generation=5 active_signature_valid=1 "
        "retired_generation=6 retired_signature_valid=1 "
        "retired_rejected_by_persistent_epoch=1 bad_signature_valid=0 "
        "signed_region_mutations=0 offline_request_bytes=256 "
        "detached_signature_bytes=256 offline_roundtrip=1 "
        "private_key_in_repository=0 production_key_claim=0 "
        "hardware_rollback_claim=0"
    )


def read_sector(path: Path, lba: int) -> bytes:
    with path.open("rb") as stream:
        stream.seek(lba * fixture.SECTOR_BYTES)
        sector = stream.read(fixture.SECTOR_BYTES)
    if len(sector) != fixture.SECTOR_BYTES:
        raise EvidenceError(f"{path} is too short for LBA {lba}")
    return sector


def decode_policy_record(sector: bytes) -> dict[str, object]:
    if (
        sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 96
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M76 key-policy record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M76 key-policy record commit proof changed")
    payload = sector[80:176]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[176:508] != bytes(332)
        or payload[:8] != POLICY_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or struct.unpack_from("<I", payload, 12)[0] != 5
        or struct.unpack_from("<I", payload, 16)[0] != 4
        or struct.unpack_from("<I", payload, 20)[0] != 4
        or payload[24:56].hex() != KEY4_SHA256
        or payload[56:88].hex() != POLICY_SHA256
        or struct.unpack_from("<Q", payload, 88)[0]
        != fixture.fnv1a64(payload[12:88]) ^ POLICY_WITNESS
    ):
        raise EvidenceError("M76 persistent key-policy binding changed")
    return {"generation": generation, "floor": 5, "key_id": 4, "key_epoch": 4}


def verify_reboot(
    pristine: Path,
    seeded: Path,
    runtime: Path,
    signature_disk: Path,
    retired_disk: Path,
    seed_serial: Path,
    seed_driver: Path,
    positive_logs: tuple[Path, Path, Path, Path],
    positive_drivers: tuple[Path, Path, Path, Path],
    signature_log: Path,
    signature_driver: Path,
    retired_log: Path,
    retired_driver: Path,
) -> str:
    seed_metrics = m75.validate_positive(
        seed_serial.read_text(encoding="utf-8", errors="replace"), "first"
    )
    m75.validate_driver(
        seed_driver.read_text(encoding="utf-8", errors="replace"), "first"
    )
    metrics = [seed_metrics]
    for phase, log_path, driver_path in zip(
        PHASES, positive_logs, positive_drivers, strict=True
    ):
        metrics.append(
            validate_positive(
                log_path.read_text(encoding="utf-8", errors="replace"), phase
            )
        )
        validate_driver(driver_path.read_text(encoding="utf-8", errors="replace"), phase)
    validate_negative(
        signature_log.read_text(encoding="utf-8", errors="replace"),
        signature_driver.read_text(encoding="utf-8", errors="replace"),
        "signature",
    )
    validate_negative(
        retired_log.read_text(encoding="utf-8", errors="replace"),
        retired_driver.read_text(encoding="utf-8", errors="replace"),
        "retired-key",
    )

    pristine_slots = tuple(read_sector(pristine, lba) for lba in POLICY_LBAS)
    seeded_slots = tuple(read_sector(seeded, lba) for lba in POLICY_LBAS)
    runtime_slots = tuple(read_sector(runtime, lba) for lba in POLICY_LBAS)
    if any(slot != bytes(fixture.SECTOR_BYTES) for slot in pristine_slots):
        raise EvidenceError("M76 pristine policy slots were not zero")
    seed = m75.decode_rollback_record(seeded_slots[0])
    if seed["generation"] != 1 or seeded_slots[1] != bytes(fixture.SECTOR_BYTES):
        raise EvidenceError("M76 M75 seed state changed")
    decoded = tuple(decode_policy_record(slot) for slot in runtime_slots)
    if [record["generation"] for record in decoded] != [3, 4]:
        raise EvidenceError("M76 final double-slot generations changed")
    for negative_disk in (signature_disk, retired_disk):
        slots = tuple(read_sector(negative_disk, lba) for lba in POLICY_LBAS)
        if slots != runtime_slots:
            raise EvidenceError("M76 rejected boot mutated a key-policy slot")
    return (
        "UNIFIED_PRODUCT_KEY_ROTATION_REBOOT_OK format=1 abi=37 "
        "product_shutdowns=5 verified_manifest_sessions=5 "
        "artifact_verifications=7 signature_successes=6 signature_rejections=1 "
        "key_transitions=2 retired_key_rejections=1 "
        "manifest_generation=5 artifact_index=5 committed_floor=5 "
        "legacy_seed_key_id=2 transition_key_id=3 active_key_id=4 "
        "legacy_migration=1 transition_floor_advance=1 "
        "active_floor_advance=1 redundancy_repair=1 steady_read_only=1 "
        "policy_reads=14 policy_writes=3 policy_flushes=3 "
        "slots=2 slot_generations=3/4 active_policy_slots=2 crc_verified=2 "
        "binding_verified=2 old_slot_preserved=1 "
        "rejected_boot_slot_mutations=0 negative_signature_boots=1 "
        "negative_retired_key_boots=1 fail_closed_pre_el0_boots=2 "
        "manifests_published_after_rejection=0 offline_split_signing=1 "
        "event_supervision_sessions=5 clean_rotations=10 "
        f"ui_query_calls={sum(item['ui_query_calls'] for item in metrics)} "
        f"ui_query_waits={sum(item['ui_query_waits'] for item in metrics)} "
        "ui_query_successes=5 psci_system_off_requests=5 "
        "qemu_psci_self_exits=5 semihosting_uses=0 qemu_disk_key_policy=1 "
        "fixture_keys=1 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
        "efuse_claim=0 host_rollback_resistance=0 erase_resistance=0 "
        "tamper_resistance=0 hardware_powercut_claim=0 emulator_only=1 "
        "real_phone_claim=0"
    )


def policy_marker(phase: str) -> str:
    fields = {**POLICY_FIXED, **PHASE_POLICY[phase]}
    return POLICY_PREFIX + " " + " ".join(f"{key}={fields[key]}" for key in POLICY_KEYS)


def final_marker(phase: str) -> str:
    fields = {**FINAL_FIXED, **PHASE_FINAL[phase]}
    return FINAL_PREFIX + " " + " ".join(f"{key}={fields[key]}" for key in FINAL_KEYS)


def synthetic_positive(phase: str) -> str:
    generation = int(PHASE_POLICY[phase]["artifact_index"])
    base = m73.synthetic_log("second")
    base = base.replace("abi=34", "abi=37")
    base = base.replace(m73.UI_BOOT_MARKER, UI_BOOT_MARKER, 1)
    base = base.replace(m73.SHUTDOWN_BOOT_MARKER, SHUTDOWN_BOOT_MARKER, 1)
    base = base.replace(m75.OLD_AUTHORITY, AUTHORITY, 1)
    base = base.replace("manifest_generation=1", f"manifest_generation={generation}", 1)
    base = base.replace(
        "WINDOW_INPUT_PHASE1_READY",
        policy_marker(phase)
        + "\nUSER_MAP_OK synthetic=1\nWINDOW_INPUT_PHASE1_READY",
        1,
    )
    event = next(
        line for line in base.splitlines() if line.startswith(m73.EVENT_PREFIX + " ")
    )
    return base.replace(event + "\n", event + "\n" + final_marker(phase) + "\n", 1)


def synthetic_negative(reason: str) -> tuple[str, str]:
    rejection = SIGNATURE_REJECTION if reason == "signature" else RETIRED_REJECTION
    log = rejection + "\nboot error: manifest key-rotation preparation failed: rejected\n"
    driver = NEGATIVE_DRIVER.format(
        reason=reason, signature_valid=int(reason == "retired-key")
    )
    return log, driver + "\n"


def parser_self_test() -> None:
    for phase in PHASES:
        validate_positive(synthetic_positive(phase), phase)
        validate_driver(driver_marker(phase) + "\n", phase)
    dynamic_steady = synthetic_positive("steady").replace(
        m73.cancel_marker().replace("abi=34", "abi=37", 1),
        m73.cancel_marker(6).replace("abi=34", "abi=37", 1),
        1,
    ).replace(
        "cancel_batch=5 cancel_pending=4 stop_batch=5 stop_pending=4",
        "cancel_batch=6 cancel_pending=4 stop_batch=6 stop_pending=4",
        1,
    )
    validate_positive(dynamic_steady, "steady")
    try:
        validate_positive(
            dynamic_steady.replace("stop_batch=6", "stop_batch=5", 1),
            "steady",
        )
    except ValueError:
        pass
    else:
        raise EvidenceError("M76 steady parser accepted inconsistent dynamic batches")
    for reason in ("signature", "retired-key"):
        log, driver = synthetic_negative(reason)
        validate_negative(log, driver, reason)
    base = synthetic_positive("transition")
    mutations = (
        base.replace("abi=37", "abi=36", 1),
        base.replace("active_key_id=3", "active_key_id=4", 1),
        base.replace("policy_sha256=" + POLICY_SHA256, "policy_sha256=" + "0" * 64, 1),
        base.replace("record_written=1", "record_written=0", 1),
        base.replace("legacy_migrated=1", "legacy_migrated=0", 1),
        base.replace("host_rollback_resistance=0", "host_rollback_resistance=1", 1),
        base.replace(policy_marker("transition") + "\n", "", 1),
        base + final_marker("transition") + "\n",
    )
    rejected = 0
    for mutation in mutations:
        try:
            validate_positive(mutation, "transition")
        except ValueError:
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M76 positive parser accepted a mutation")
    negative_rejected = 0
    for reason in ("signature", "retired-key"):
        log, driver = synthetic_negative(reason)
        rejection = SIGNATURE_REJECTION if reason == "signature" else RETIRED_REJECTION
        for mutated_log, mutated_driver in (
            (log.replace(rejection + "\n", "", 1), driver),
            (log + "USER_MAP_OK x=1\n", driver),
            (log, driver.replace("pre_el0=1", "pre_el0=0")),
        ):
            try:
                validate_negative(mutated_log, mutated_driver, reason)
            except ValueError:
                negative_rejected += 1
    if negative_rejected != 6:
        raise EvidenceError("M76 negative parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_KEY_ROTATION_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES) * 2 + 3} serial_negative={rejected + 1} "
        f"fail_closed_negative={negative_rejected}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    parse = subparsers.add_parser("parse")
    parse.add_argument("phase", choices=PHASES)
    parse.add_argument("log", type=Path)
    negative = subparsers.add_parser("parse-negative")
    negative.add_argument("reason", choices=("signature", "retired-key"))
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)
    artifacts = subparsers.add_parser("artifacts")
    for name in ("transition", "active", "retired", "bad", "request", "signature"):
        artifacts.add_argument(name, type=Path)
    reboot = subparsers.add_parser("reboot")
    for name in (
        "pristine",
        "seeded",
        "runtime",
        "signature_disk",
        "retired_disk",
        "seed_serial",
        "seed_driver",
        "transition_serial",
        "activate_serial",
        "repair_serial",
        "steady_serial",
        "transition_driver",
        "activate_driver",
        "repair_driver",
        "steady_driver",
        "signature_serial",
        "signature_driver",
        "retired_serial",
        "retired_driver",
    ):
        reboot.add_argument(name, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "self-test":
            parser_self_test()
        elif args.command == "parse":
            validate_positive(
                args.log.read_text(encoding="utf-8", errors="replace"), args.phase
            )
            print(f"UNIFIED_PRODUCT_KEY_ROTATION_EVIDENCE_OK phase={args.phase}")
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_KEY_ROTATION_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason}"
            )
        elif args.command == "artifacts":
            print(
                verify_artifacts(
                    args.transition,
                    args.active,
                    args.retired,
                    args.bad,
                    args.request,
                    args.signature,
                )
            )
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.pristine,
                    args.seeded,
                    args.runtime,
                    args.signature_disk,
                    args.retired_disk,
                    args.seed_serial,
                    args.seed_driver,
                    (
                        args.transition_serial,
                        args.activate_serial,
                        args.repair_serial,
                        args.steady_serial,
                    ),
                    (
                        args.transition_driver,
                        args.activate_driver,
                        args.repair_driver,
                        args.steady_driver,
                    ),
                    args.signature_serial,
                    args.signature_driver,
                    args.retired_serial,
                    args.retired_driver,
                )
            )
        else:
            raise AssertionError("unreachable command")
    except (ValueError, OSError) as error:
        print(f"unified_product_key_rotation_evidence.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
