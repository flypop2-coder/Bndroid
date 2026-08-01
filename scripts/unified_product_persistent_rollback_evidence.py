#!/usr/bin/env python3
"""Strict M75 persistent-manifest rollback and multi-boot evidence parser."""

from __future__ import annotations

import argparse
import binascii
import hashlib
import re
import struct
import sys
from pathlib import Path

import build_storage_image as fixture
import unified_product_event_supervision_evidence as m73


PHASES = ("first", "second", "steady")
SIGNED_SHA256 = "76970b66ada0eab016c7d04af345c0b0a99d0bd9c9dc4e50e6df596cb795143a"
OLD_SIGNED_SHA256 = (
    "30701a3a6281872c489a9569b77a32dcebafa4c64dc8f19ebb5dc0c0b3d8d3df"
)
TRUST_ANCHOR_SHA256 = (
    "a050397ce65d2a9f46bb65d9220b56c02984b10dcecd39c530282190ca676c37"
)
RSA_MODULUS_HEX = (
    "d7cc4e2922920a3feafda46ac10a2bcc23bdb7efebcbba4ae5461a1da11a90c4"
    "f40c47515dba303da54070838738fbd0d1d992c29fddb00addb729b52e793af5"
    "d9d542344c93f01c2d574634b58f560cd517cc020fb0d4207c3f761cc7cc7604"
    "c30b9797e44f3c1ce2182134b4b55ff751b71c895f068addedc14a8473d45c2e"
    "f53a068d87d3aa33b004ff4584b2fc48755cbdb3d1dface05c3d2f7fbd6d8e8"
    "c64bbc7caec1123bbb520a901560d489972913ad19adc7bec863958814d70bb51"
    "9c9bac7af42fa7746cf5ae3b66166fdbbfdeb11eee7b748155ac915863bfb967"
    "032bad3b08bedce2b09ae317cacbb69d1284278cc1855be37244aabf6c7df697"
)
RSA_MODULUS = bytes.fromhex(RSA_MODULUS_HEX)
RSA_EXPONENT = 65537
SHA256_DIGEST_INFO_PREFIX = bytes.fromhex(
    "3031300d060960864801650304020105000420"
)

AUTHORITY = (
    "external-bms1-plus-kernel-pinned-rsa2048-plus-qemu-disk-double-slot-"
    "rollback-floor-plus-init-event-loop-plus-kernel-authenticated-report"
)
OLD_AUTHORITY = (
    "kernel-owned-bmf1-plus-init-event-loop-plus-kernel-authenticated-report"
)
UI_BOOT_MARKER = (
    "BOOT_OK: M75 unified real UI is interactive; persistent-rollback event "
    "supervision is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M75 persistent rollback ledger, verified external manifest, "
    "event supervision, AppData, and PSCI shutdown armed"
)
UI_MARKER = m73.UI_MARKER.replace("abi=34", "abi=36", 1)
DISCOVERY_MARKER = m73.DISCOVERY_MARKER.replace("abi=34", "abi=36", 1)
PSCI_SHUTDOWN_MARKER = m73.PSCI_SHUTDOWN_MARKER.replace("abi=34", "abi=36", 1)
FAILURE_PATTERN = re.compile(
    m73.FAILURE_PATTERN.pattern + r"|PERSISTENT_ROLLBACK_REJECTED",
    m73.FAILURE_PATTERN.flags,
)

LEDGER_PREFIX = "PERSISTENT_ROLLBACK_LEDGER_OK"
LEDGER_KEYS = (
    "format",
    "state_version",
    "authority",
    "slots",
    "relative_lbas",
    "key_id",
    "bootstrap_floor",
    "provisioned_before",
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
    "committed_valid_slots",
    "committed_rejected_slots",
    "floor_advanced",
    "record_written",
    "redundancy_repaired",
    "reads",
    "writes",
    "flushes",
    "write_flush_readback",
    "manifest_published",
    "init_ready",
    "host_rollback_resistance",
    "erase_resistance",
    "rpmb",
    "efuse",
    "hardware_powercut_claim",
    "emulator_only",
)
LEDGER_FIXED = {
    "format": "1",
    "state_version": "1",
    "authority": "kernel-pre-el0-qemu-disk",
    "slots": "2",
    "relative_lbas": "3/4",
    "key_id": "2",
    "bootstrap_floor": "2",
    "artifact_index": "3",
    "committed_floor": "3",
    "write_flush_readback": "1",
    "manifest_published": "0",
    "init_ready": "0",
    "host_rollback_resistance": "0",
    "erase_resistance": "0",
    "rpmb": "0",
    "efuse": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
}
PHASE_LEDGER = {
    "first": {
        "provisioned_before": "0",
        "persisted_floor_before": "0",
        "effective_floor": "2",
        "initial_generation": "0",
        "committed_generation": "1",
        "initial_slot": "255",
        "committed_slot": "0",
        "initial_valid_slots": "0",
        "initial_rejected_slots": "0",
        "committed_valid_slots": "1",
        "committed_rejected_slots": "1",
        "floor_advanced": "1",
        "record_written": "1",
        "redundancy_repaired": "0",
        "reads": "4",
        "writes": "1",
        "flushes": "1",
    },
    "second": {
        "provisioned_before": "1",
        "persisted_floor_before": "3",
        "effective_floor": "3",
        "initial_generation": "1",
        "committed_generation": "2",
        "initial_slot": "0",
        "committed_slot": "1",
        "initial_valid_slots": "1",
        "initial_rejected_slots": "1",
        "committed_valid_slots": "2",
        "committed_rejected_slots": "0",
        "floor_advanced": "0",
        "record_written": "1",
        "redundancy_repaired": "1",
        "reads": "4",
        "writes": "1",
        "flushes": "1",
    },
    "steady": {
        "provisioned_before": "1",
        "persisted_floor_before": "3",
        "effective_floor": "3",
        "initial_generation": "2",
        "committed_generation": "2",
        "initial_slot": "1",
        "committed_slot": "1",
        "initial_valid_slots": "2",
        "initial_rejected_slots": "0",
        "committed_valid_slots": "2",
        "committed_rejected_slots": "0",
        "floor_advanced": "0",
        "record_written": "0",
        "redundancy_repaired": "0",
        "reads": "2",
        "writes": "0",
        "flushes": "0",
    },
}

FINAL_PREFIX = "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK"
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
    "key_id",
    "rollback_index",
    "bootstrap_floor",
    "persisted_floor_before",
    "effective_floor",
    "committed_floor",
    "provisioned_before",
    "slots",
    "relative_lbas",
    "initial_generation",
    "committed_generation",
    "initial_slot",
    "committed_slot",
    "initial_valid_slots",
    "initial_rejected_slots",
    "committed_valid_slots",
    "committed_rejected_slots",
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
    "verification_attempts",
    "verification_successes",
    "signature_successes",
    "signature_rejections",
    "rollback_rejections",
    "format_rejections",
    "signed_sha256",
    "trust_anchor_sha256",
    "external_artifact",
    "kernel_pinned_fixture_key",
    "double_slot_crc",
    "write_flush_readback",
    "old_slot_preserved",
    "manifest_published_after_persistent_commit",
    "private_key_in_repository",
    "production_key_claim",
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
    "abi": "36",
    "artifact": "BMS1",
    "artifact_bytes": "512",
    "signed_bytes": "256",
    "payload": "BMF1",
    "payload_bytes": "224",
    "manifest_generation": "3",
    "algorithm": "RSA2048-PKCS1-v1_5-SHA256",
    "key_id": "2",
    "rollback_index": "3",
    "bootstrap_floor": "2",
    "committed_floor": "3",
    "slots": "2",
    "relative_lbas": "3/4",
    "ledger_attempts": "1",
    "ledger_successes": "1",
    "ledger_rollback_rejections": "0",
    "ledger_failures": "0",
    "verification_attempts": "1",
    "verification_successes": "1",
    "signature_successes": "1",
    "signature_rejections": "0",
    "rollback_rejections": "0",
    "format_rejections": "0",
    "signed_sha256": SIGNED_SHA256,
    "trust_anchor_sha256": TRUST_ANCHOR_SHA256,
    "external_artifact": "1",
    "kernel_pinned_fixture_key": "1",
    "double_slot_crc": "1",
    "write_flush_readback": "1",
    "old_slot_preserved": "1",
    "manifest_published_after_persistent_commit": "1",
    "private_key_in_repository": "0",
    "production_key_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "host_rollback_resistance": "0",
    "erase_resistance": "0",
    "tamper_resistance": "0",
    "hardware_powercut_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

SIGNATURE_REJECTION = (
    "PERSISTENT_ROLLBACK_REJECTED format=1 reason=signature key_id=2 "
    "bootstrap_floor=2 signature_valid=0 manifest_published=0 init_ready=0"
)
ROLLBACK_REJECTION = (
    "PERSISTENT_ROLLBACK_REJECTED format=1 reason=rollback key_id=2 "
    "artifact_index=2 persisted_floor=3 signature_valid=1 "
    "manifest_published=0 init_ready=0"
)
NEGATIVE_DRIVER = (
    "PERSISTENT_ROLLBACK_NEGATIVE_BOOT_OK reason={reason} pre_el0=1 "
    "signature_valid={signature_valid} manifest_published=0 init_ready=0 "
    "qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
)

DATA_RECORD_MAGIC = b"BNDRREC1"
ROLLBACK_MAGIC = b"BNDRRBK1"
RECORD_COOKIE = 0x434F_4D4D_4954_2131
ROLLBACK_WITNESS = 0xB24D_75DA_7A5E_C001
ROLLBACK_LBAS = (fixture.DATA_PARTITION_FIRST_LBA + 3, fixture.DATA_PARTITION_FIRST_LBA + 4)


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


def validate_rollback_markers(
    lines: list[str], phase: str
) -> tuple[dict[str, str], dict[str, str]]:
    ledger = parse_marker(lines, LEDGER_PREFIX, LEDGER_KEYS)
    final = parse_marker(lines, FINAL_PREFIX, FINAL_KEYS)
    require(ledger, LEDGER_FIXED, LEDGER_PREFIX)
    require(ledger, PHASE_LEDGER[phase], LEDGER_PREFIX)
    require(final, FINAL_FIXED, FINAL_PREFIX)
    phase_final = {
        key: value
        for key, value in PHASE_LEDGER[phase].items()
        if key not in {"artifact_index"}
    }
    require(final, phase_final, FINAL_PREFIX)
    for key in (
        "provisioned_before",
        "persisted_floor_before",
        "effective_floor",
        "committed_floor",
        "initial_generation",
        "committed_generation",
        "initial_slot",
        "committed_slot",
        "initial_valid_slots",
        "initial_rejected_slots",
        "committed_valid_slots",
        "committed_rejected_slots",
        "floor_advanced",
        "record_written",
        "redundancy_repaired",
        "reads",
        "writes",
        "flushes",
    ):
        if ledger[key] != final[key]:
            raise EvidenceError(f"M75 ledger/final field {key} diverged")
    if lines.index(m73.unique_line(lines, LEDGER_PREFIX)) >= lines.index(
        m73.unique_line(lines, "USER_MAP_OK")
    ):
        raise EvidenceError("M75 persistent ledger was not prepared before EL0 mapping")
    if not (
        lines.index(m73.unique_line(lines, m73.EVENT_PREFIX))
        < lines.index(m73.unique_line(lines, FINAL_PREFIX))
        < lines.index(m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK"))
    ):
        raise EvidenceError("M75 event/rollback/shutdown marker order changed")
    return ledger, final


def normalize_to_m73(text: str) -> str:
    lines = [
        line
        for line in text.replace("\r", "").splitlines()
        if not line.startswith(LEDGER_PREFIX + " ")
        and not line.startswith(FINAL_PREFIX + " ")
    ]
    normalized = "\n".join(lines) + "\n"
    normalized = normalized.replace(UI_BOOT_MARKER, m73.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(
        SHUTDOWN_BOOT_MARKER, m73.SHUTDOWN_BOOT_MARKER, 1
    )
    normalized = normalized.replace(AUTHORITY, OLD_AUTHORITY, 1)
    normalized = normalized.replace("manifest_generation=3", "manifest_generation=1", 1)
    normalized = normalized.replace("abi=36", "abi=34")
    return normalized


def validate_steady_runtime(lines: list[str]) -> dict[str, int]:
    if m73.unique_line(lines, "M73_PSCI_DISCOVERY_OK") != DISCOVERY_MARKER:
        raise EvidenceError("M75 steady PSCI discovery marker changed")
    if m73.unique_line(lines, "UNIFIED_PRODUCT_UI_OK") != UI_MARKER:
        raise EvidenceError("M75 steady UI marker changed")
    if (
        m73.unique_line(lines, "M73_EVENT_SUPERVISOR_ACTIVE_OK")
        != m73.active_marker().replace("abi=34", "abi=36", 1)
    ):
        raise EvidenceError("M75 steady event activation marker changed")
    for ordinal in (1, 2):
        expected = m73.rotation_marker(ordinal).replace("abi=34", "abi=36", 1)
        if expected not in lines:
            raise EvidenceError(f"M75 steady rotation {ordinal} marker changed")
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
            "abi": "36",
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
        event.replace("abi=36", "abi=34", 1)
        .replace(AUTHORITY, OLD_AUTHORITY, 1)
        .replace("manifest_generation=3", "manifest_generation=1", 1)
    )
    try:
        metrics = m73.validate_event(normalized_event, 3, cancel_batch)
    except m73.EvidenceError as error:
        raise EvidenceError(str(error)) from error
    shutdown = m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK")
    for required in (
        "format=1",
        "abi=36",
        "bounded_shutdown=1",
        "current_boot_open=0",
        "admission_closed=1",
        "post_close_storage_mutations=0",
        "emulator_backend=qemu-psci",
        "hardware_poweroff_claim=0",
        "real_phone_claim=0",
    ):
        if required not in shutdown.split():
            raise EvidenceError(f"M75 steady shutdown field changed: {required}")
    if m73.unique_line(lines, "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK") != PSCI_SHUTDOWN_MARKER:
        raise EvidenceError("M75 steady PSCI shutdown marker changed")
    return metrics


def validate_positive(text: str, phase: str) -> dict[str, int]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M75 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    if "abi=35" in normalized or "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK " in normalized:
        raise EvidenceError("M75 retained an M74 runtime marker")
    lines = normalized.splitlines()
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != {
        UI_BOOT_MARKER,
        SHUTDOWN_BOOT_MARKER,
    }:
        raise EvidenceError("M75 BOOT_OK marker set changed")
    validate_rollback_markers(lines, phase)
    if phase in {"first", "second"}:
        try:
            return m73.validate_text(normalize_to_m73(normalized), phase)
        except ValueError as error:
            raise EvidenceError(str(error)) from error
    return validate_steady_runtime(lines)


def driver_marker(phase: str) -> str:
    return (
        f"UNIFIED_PRODUCT_QMP_BOOT_OK phase={phase} qemu_self_exit=1 "
        f"power_key=116 ui_sha256={m73.FINAL_SCREEN_SHA256} "
        "maintenance_rotations=2 cancel_pending=4"
    )


def validate_driver(text: str, phase: str) -> None:
    lines = [line for line in text.replace("\r", "").splitlines() if line]
    if lines != [driver_marker(phase)]:
        raise EvidenceError(f"M75 {phase} host QMP self-exit evidence changed")


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in {"signature", "rollback"}:
        raise EvidenceError(f"unknown M75 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M75 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("boot error: persistent manifest preparation failed:")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M75 {reason} boot diagnostic changed")
    forbidden = (
        LEDGER_PREFIX + " ",
        "USER_MAP_OK ",
        "ELF_LOAD_OK ",
        "UNIFIED_PRODUCT_UI_OK ",
        "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
        FINAL_PREFIX + " ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        "BOOT_OK: M75 ",
    )
    if any(any(line.startswith(prefix) for prefix in forbidden) for line in lines):
        raise EvidenceError(f"M75 {reason} negative boot crossed the pre-EL0 boundary")
    if lines.index(rejection) >= lines.index(diagnostics[0]):
        raise EvidenceError(f"M75 {reason} rejection/diagnostic order changed")
    expected_driver = NEGATIVE_DRIVER.format(
        reason=reason, signature_valid=int(reason == "rollback")
    )
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    if driver_lines != [expected_driver]:
        raise EvidenceError(f"M75 {reason} negative host evidence changed")


def decode_hex_artifact(path: Path) -> bytes:
    digits = "".join(path.read_text(encoding="ascii").split())
    if not digits or len(digits) % 2 or re.fullmatch(r"[0-9a-f]+", digits) is None:
        raise EvidenceError(f"{path} is not canonical lowercase hexadecimal")
    return bytes.fromhex(digits)


def inspect_artifact(artifact: bytes) -> dict[str, object]:
    if len(artifact) != 512:
        raise EvidenceError("M75 BMS1 artifact length changed")
    if (
        artifact[:4] != b"BMS1"
        or artifact[4:8] != bytes((1, 1, 2, 0))
        or int.from_bytes(artifact[8:12], "little") != 512
        or int.from_bytes(artifact[12:16], "little") != 256
        or int.from_bytes(artifact[16:20], "little") != 32
        or int.from_bytes(artifact[20:24], "little") != 224
        or artifact[28:32] != bytes(4)
    ):
        raise EvidenceError("M75 BMS1 strict envelope changed")
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
        raise EvidenceError("M75 signed BMF1 payload header changed")
    generation = int.from_bytes(payload[12:16], "little")
    if generation != rollback_index:
        raise EvidenceError("M75 rollback index no longer matches BMF1 generation")
    signature_value = int.from_bytes(artifact[256:], "big")
    modulus = int.from_bytes(RSA_MODULUS, "big")
    encoded = pow(signature_value, RSA_EXPONENT, modulus).to_bytes(256, "big")
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
        "signed_sha256": digest.hex(),
        "signature_valid": encoded == expected,
        "signed": signed,
    }


def verify_artifacts(current_path: Path, old_path: Path, bad_path: Path) -> str:
    if hashlib.sha256(RSA_MODULUS).hexdigest() != TRUST_ANCHOR_SHA256:
        raise EvidenceError("M75 parser trust anchor changed")
    current = inspect_artifact(decode_hex_artifact(current_path))
    old = inspect_artifact(decode_hex_artifact(old_path))
    bad = inspect_artifact(decode_hex_artifact(bad_path))
    if (
        current["generation"] != 3
        or current["rollback_index"] != 3
        or current["signed_sha256"] != SIGNED_SHA256
        or current["signature_valid"] is not True
    ):
        raise EvidenceError("M75 current artifact verification changed")
    if (
        old["generation"] != 2
        or old["rollback_index"] != 2
        or old["signed_sha256"] != OLD_SIGNED_SHA256
        or old["signature_valid"] is not True
    ):
        raise EvidenceError("M75 valid-old artifact verification changed")
    if (
        bad["generation"] != 3
        or bad["rollback_index"] != 3
        or bad["signed"] != current["signed"]
        or bad["signature_valid"] is not False
    ):
        raise EvidenceError("M75 bad-signature fixture changed")
    return (
        "PERSISTENT_ROLLBACK_ARTIFACTS_OK format=1 current_signature_valid=1 "
        "current_generation=3 rollback_index=3 bootstrap_floor=2 "
        "old_signature_valid=1 old_generation=2 old_rejected_after_floor3=1 "
        "bad_signature_valid=0 signed_region_mutations=0 key_id=2 "
        "algorithm=RSA2048-PKCS1-v1_5-SHA256 fixture_key=1 "
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


def decode_rollback_record(sector: bytes) -> dict[str, object]:
    if (
        sector[:8] != DATA_RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != 64
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
    ):
        raise EvidenceError("M75 persistent rollback record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
    ):
        raise EvidenceError("M75 persistent rollback record commit proof changed")
    payload = sector[80:144]
    if (
        fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or sector[144:508] != bytes(364)
        or payload[:8] != ROLLBACK_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or struct.unpack_from("<I", payload, 12)[0] != 3
        or struct.unpack_from("<I", payload, 16)[0] != 2
        or payload[20:24] != bytes(4)
        or payload[24:56].hex() != TRUST_ANCHOR_SHA256
        or struct.unpack_from("<Q", payload, 56)[0]
        != fixture.fnv1a64(payload[12:56]) ^ ROLLBACK_WITNESS
    ):
        raise EvidenceError("M75 persistent rollback state binding changed")
    return {"generation": generation, "floor": 3, "key_id": 2}


def verify_reboot(
    pristine: Path,
    runtime: Path,
    signature_disk: Path,
    rollback_disk: Path,
    positive_logs: tuple[Path, Path, Path],
    positive_drivers: tuple[Path, Path, Path],
    signature_log: Path,
    signature_driver: Path,
    rollback_log: Path,
    rollback_driver: Path,
) -> str:
    metrics = []
    for phase, log_path, driver_path in zip(
        PHASES, positive_logs, positive_drivers, strict=True
    ):
        metrics.append(
            validate_positive(
                log_path.read_text(encoding="utf-8", errors="replace"), phase
            )
        )
        validate_driver(
            driver_path.read_text(encoding="utf-8", errors="replace"), phase
        )
    validate_negative(
        signature_log.read_text(encoding="utf-8", errors="replace"),
        signature_driver.read_text(encoding="utf-8", errors="replace"),
        "signature",
    )
    validate_negative(
        rollback_log.read_text(encoding="utf-8", errors="replace"),
        rollback_driver.read_text(encoding="utf-8", errors="replace"),
        "rollback",
    )
    pristine_slots = tuple(read_sector(pristine, lba) for lba in ROLLBACK_LBAS)
    runtime_slots = tuple(read_sector(runtime, lba) for lba in ROLLBACK_LBAS)
    if any(slot != bytes(fixture.SECTOR_BYTES) for slot in pristine_slots):
        raise EvidenceError("M75 pristine rollback slots were not zero")
    decoded = tuple(decode_rollback_record(slot) for slot in runtime_slots)
    if [record["generation"] for record in decoded] != [1, 2]:
        raise EvidenceError("M75 final double-slot generations changed")
    for negative_disk in (signature_disk, rollback_disk):
        negative_slots = tuple(read_sector(negative_disk, lba) for lba in ROLLBACK_LBAS)
        if negative_slots != runtime_slots:
            raise EvidenceError("M75 rejected boot mutated a rollback slot")
    return (
        "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_REBOOT_OK format=1 abi=36 "
        "product_shutdowns=3 verified_manifest_sessions=3 "
        "artifact_verifications=5 signature_successes=4 "
        "signature_rejections=1 persistent_rollback_rejections=1 "
        "manifest_generation=3 artifact_index=3 bootstrap_floor=2 "
        "committed_floor=3 first_boot_advances=1 second_boot_repairs=1 "
        "steady_boot_read_only=1 ledger_reads=10 ledger_writes=2 "
        "ledger_flushes=2 slots=2 slot_generations=1/2 valid_slots=2 "
        "rejected_slots=0 crc_verified=2 binding_verified=2 "
        "old_slot_preserved=1 rejected_boot_slot_mutations=0 "
        "negative_signature_boots=1 negative_rollback_boots=1 "
        "fail_closed_pre_el0_boots=2 manifests_published_after_rejection=0 "
        "event_supervision_sessions=3 clean_rotations=6 "
        f"ui_query_calls={sum(item['ui_query_calls'] for item in metrics)} "
        f"ui_query_waits={sum(item['ui_query_waits'] for item in metrics)} "
        "ui_query_successes=3 psci_system_off_requests=3 "
        "qemu_psci_self_exits=3 semihosting_uses=0 qemu_disk_ledger=1 "
        "fixture_key=1 production_key_claim=0 rpmb_claim=0 efuse_claim=0 "
        "host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0 "
        "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
    )


def ledger_marker(phase: str) -> str:
    fields = {**LEDGER_FIXED, **PHASE_LEDGER[phase]}
    return LEDGER_PREFIX + " " + " ".join(f"{key}={fields[key]}" for key in LEDGER_KEYS)


def final_marker(phase: str) -> str:
    fields = {
        **FINAL_FIXED,
        **{
            key: value
            for key, value in PHASE_LEDGER[phase].items()
            if key != "artifact_index"
        },
    }
    return FINAL_PREFIX + " " + " ".join(f"{key}={fields[key]}" for key in FINAL_KEYS)


def synthetic_positive(phase: str) -> str:
    base_phase = phase if phase in m73.PHASES else "second"
    base = m73.synthetic_log(base_phase)
    base = base.replace("abi=34", "abi=36")
    base = base.replace(m73.UI_BOOT_MARKER, UI_BOOT_MARKER, 1)
    base = base.replace(m73.SHUTDOWN_BOOT_MARKER, SHUTDOWN_BOOT_MARKER, 1)
    base = base.replace(OLD_AUTHORITY, AUTHORITY, 1)
    base = base.replace("manifest_generation=1", "manifest_generation=3", 1)
    base = base.replace(
        "WINDOW_INPUT_PHASE1_READY",
        ledger_marker(phase)
        + "\nUSER_MAP_OK synthetic=1\nWINDOW_INPUT_PHASE1_READY",
        1,
    )
    event = next(
        line for line in base.splitlines() if line.startswith(m73.EVENT_PREFIX + " ")
    )
    return base.replace(event + "\n", event + "\n" + final_marker(phase) + "\n", 1)


def synthetic_negative(reason: str) -> tuple[str, str]:
    rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
    log = rejection + "\nboot error: persistent manifest preparation failed: rejected\n"
    driver = NEGATIVE_DRIVER.format(
        reason=reason, signature_valid=int(reason == "rollback")
    )
    return log, driver + "\n"


def parser_self_test() -> None:
    for phase in PHASES:
        validate_positive(synthetic_positive(phase), phase)
        validate_driver(driver_marker(phase) + "\n", phase)
    dynamic_steady = synthetic_positive("steady").replace(
        m73.cancel_marker().replace("abi=34", "abi=36", 1),
        m73.cancel_marker(6).replace("abi=34", "abi=36", 1),
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
        raise EvidenceError("M75 steady parser accepted inconsistent dynamic batches")
    for reason in ("signature", "rollback"):
        log, driver = synthetic_negative(reason)
        validate_negative(log, driver, reason)
    base = synthetic_positive("first")
    mutations = (
        base.replace("abi=36", "abi=35", 1),
        base.replace("artifact_index=3", "artifact_index=2", 1),
        base.replace("committed_floor=3", "committed_floor=2", 1),
        base.replace("record_written=1", "record_written=0", 1),
        base.replace("manifest_published_after_persistent_commit=1", "manifest_published_after_persistent_commit=0", 1),
        base.replace("host_rollback_resistance=0", "host_rollback_resistance=1", 1),
        base.replace(ledger_marker("first") + "\n", "", 1),
        base + final_marker("first") + "\n",
    )
    rejected = 0
    for mutation in mutations:
        try:
            validate_positive(mutation, "first")
        except ValueError:
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M75 positive parser accepted a mutation")
    negative_rejected = 0
    for reason in ("signature", "rollback"):
        log, driver = synthetic_negative(reason)
        rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
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
        raise EvidenceError("M75 negative parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_EVIDENCE_PARSER_SELF_TEST_OK "
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
    negative.add_argument("reason", choices=("signature", "rollback"))
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)
    artifacts = subparsers.add_parser("artifacts")
    artifacts.add_argument("current", type=Path)
    artifacts.add_argument("old", type=Path)
    artifacts.add_argument("bad_signature", type=Path)
    reboot = subparsers.add_parser("reboot")
    for name in (
        "pristine",
        "runtime",
        "signature_disk",
        "rollback_disk",
        "first_serial",
        "second_serial",
        "steady_serial",
        "first_driver",
        "second_driver",
        "steady_driver",
        "signature_serial",
        "signature_driver",
        "rollback_serial",
        "rollback_driver",
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
            print(
                "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_EVIDENCE_OK "
                f"phase={args.phase}"
            )
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason}"
            )
        elif args.command == "artifacts":
            print(verify_artifacts(args.current, args.old, args.bad_signature))
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.pristine,
                    args.runtime,
                    args.signature_disk,
                    args.rollback_disk,
                    (args.first_serial, args.second_serial, args.steady_serial),
                    (args.first_driver, args.second_driver, args.steady_driver),
                    args.signature_serial,
                    args.signature_driver,
                    args.rollback_serial,
                    args.rollback_driver,
                )
            )
        else:
            raise AssertionError("unreachable command")
    except (ValueError, OSError) as error:
        print(
            f"unified_product_persistent_rollback_evidence.py: {error}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
