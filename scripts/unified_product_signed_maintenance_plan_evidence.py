#!/usr/bin/env python3
"""Strict M81 signed maintenance-plan runtime and disk evidence."""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
from pathlib import Path

import build_storage_image as fixture
import unified_product_event_supervision_evidence as m73
import unified_product_maintenance_authorization_evidence as m77
import unified_product_maintenance_plan_evidence as m80
import unified_product_maintenance_step_evidence as m79


PHASES = ("normal", "recover-binding")
PROGRAM_PREFIX = "SIGNED_MAINTENANCE_PLAN_OK"
OPERATION_PREFIX = "SIGNED_MAINTENANCE_OPERATION_BOUND"
TERMINAL_PREFIX = "SIGNED_MAINTENANCE_PLAN_TERMINAL_OK"
FINAL_PREFIX = "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK"
UI_BOOT_MARKER = (
    "BOOT_OK: M81 unified real UI is interactive; signed descriptor-bound "
    "maintenance-plan reconciliation is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M81 signed descriptor-bound maintenance plan, durable program "
    "binding, AppData, and PSCI shutdown armed"
)

PROGRAM_MAGIC = b"BNDRMPB1"
PROGRAM_WITNESS = 0xB24D_81DA_7A5E_C001
RECORD_MAGIC = b"BNDRREC1"
RECORD_COOKIE = 0x434F_4D4D_4954_2131
PROGRAM_BYTES = 336
PROGRAM_LBAS = (
    fixture.DATA_PARTITION_FIRST_LBA + 13,
    fixture.DATA_PARTITION_FIRST_LBA + 14,
)
ROOT_SHA256 = bytes.fromhex(
    "30f139bf41df6593a0db7c80be1b806f"
    "56954977c92276765e4156685b10a24e"
)
EXPECTED_DESCRIPTORS = (
    (1, 1, 1, 0x4D810001),
    (1, 0, 2, 0x4D810002),
    (2, 0, 2, 0x4D810003),
)


class EvidenceError(ValueError):
    pass


def slash(value: bytes) -> str:
    encoded = value.hex()
    return "/".join(encoded[index : index + 16] for index in range(0, 64, 16))


def read_fixture(path: Path, expected_bytes: int) -> bytes:
    try:
        digits = "".join(path.read_text(encoding="ascii").split())
    except (OSError, UnicodeError) as error:
        raise EvidenceError(f"cannot read fixture {path}") from error
    if (
        len(digits) != expected_bytes * 2
        or digits != digits.lower()
        or re.fullmatch(r"[0-9a-f]+", digits) is None
    ):
        raise EvidenceError(f"fixture {path} is not canonical lowercase hexadecimal")
    return bytes.fromhex(digits)


def artifact_material() -> dict[str, bytes | int]:
    root = Path(__file__).resolve().parent.parent
    request = read_fixture(
        root / "boot/maintenance/maintenance-plan-sequence2.request.hex", 256
    )
    artifact = read_fixture(root / "boot/maintenance-plan-sequence2.bmp1.hex", 512)
    if artifact[:256] != request:
        raise EvidenceError("BMP1 request is not the exact artifact signed region")
    if (
        request[:4] != b"BMP1"
        or request[4:8] != bytes((1, 1, 1, 0))
        or struct.unpack_from("<IIII", request, 8) != (512, 256, 64, 192)
        or struct.unpack_from("<QIIIIQ", request, 24) != (2, 3, 9, 1, 16, 2)
        or request[56:64] != bytes(8)
        or request[240:256] != bytes(16)
    ):
        raise EvidenceError("BMP1 fixture envelope changed")
    descriptors = tuple(
        struct.unpack_from("<IIII", request, 192 + index * 16)
        for index in range(3)
    )
    expected = tuple(
        (kind, 1 | (2 if cancellable else 0), argument, token)
        for kind, cancellable, argument, token in EXPECTED_DESCRIPTORS
    )
    if descriptors != expected:
        raise EvidenceError("BMP1 bounded descriptor table changed")
    return {
        "request": request,
        "program_sha256": hashlib.sha256(request).digest(),
        "descriptor_sha256": hashlib.sha256(request[192:256]).digest(),
        "authorization_sha256": request[64:96],
        "authorization_id": request[96:128],
        "device_binding_sha256": request[128:160],
        "plan_id_sha256": request[160:192],
    }


def program_chain() -> bytes:
    material = artifact_material()
    chain_material = (
        PROGRAM_MAGIC
        + bytes(32)
        + struct.pack("<QIIII", 2, 3, 9, 1, 1)
        + bytes(material["authorization_sha256"])
        + bytes(material["authorization_id"])
        + bytes(material["program_sha256"])
        + bytes(material["plan_id_sha256"])
        + bytes(material["descriptor_sha256"])
        + ROOT_SHA256
        + bytes(material["device_binding_sha256"])
    )
    if len(chain_material) != 288:
        raise AssertionError("M81 program chain material changed size")
    return hashlib.sha256(chain_material).digest()


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
    for key, value in expected.items():
        if fields.get(key) != value:
            raise EvidenceError(
                f"{prefix} {key}={fields.get(key)!r}, expected {value!r}"
            )


def runtime_digest(lines: list[str], plan_chain: bytes, binding_chain: bytes) -> bytes:
    event = parse_loose(lines, m73.EVENT_PREFIX)
    material = bytearray(288)
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
    material[224:256] = binding_chain
    material[256:288] = bytes(artifact_material()["program_sha256"])
    return hashlib.sha256(material).digest()


def validate_positive(text: str, phase: str) -> None:
    if phase not in PHASES:
        raise EvidenceError(f"unsupported M81 phase {phase}")
    lines = text.replace("\r", "").splitlines()
    if lines.count(UI_BOOT_MARKER) != 1 or lines.count(SHUTDOWN_BOOT_MARKER) != 1:
        raise EvidenceError("M81 exact boot markers changed")
    material = artifact_material()
    chain = program_chain()
    plan_chain = m80.expected_chains()[9]

    program = parse_loose(lines, PROGRAM_PREFIX)
    replayed = phase == "recover-binding"
    require(
        program,
        {
            "format": "1",
            "artifact": "BMP1",
            "state": "BNDRMPB1",
            "state_version": "1",
            "slots": "2",
            "relative_lbas": "13/14",
            "authorization_sequence": "2",
            "operation_count": "3",
            "transition_count": "9",
            "program_version": "1",
            "plan_generation": "2",
            "preapply_cancel_mask": "1",
            "signature_valid": "1",
            "binding_valid": "1",
            "program_valid": "1",
            "plan_preflight_reads": "10",
            "program_provisioned_before": str(int(replayed)),
            "program_exact_replay_before": str(int(replayed)),
            "predecessor_complete": "1",
            "initial_generation": "1" if replayed else "0",
            "committed_generation": "1",
            "initial_slot": "0" if replayed else "255",
            "committed_slot": "0",
            "initial_valid_slots": "1" if replayed else "0",
            "initial_rejected_slots": "1" if replayed else "0",
            "committed_valid_slots": "1",
            "committed_rejected_slots": "1",
            "reads": "10" if replayed else "14",
            "writes": "0" if replayed else "1",
            "flushes": "0" if replayed else "1",
            "program_sha256": slash(bytes(material["program_sha256"])),
            "descriptor_sha256": slash(bytes(material["descriptor_sha256"])),
            "chain_sha256": slash(chain),
            "old_slot_preserved": "1",
            "preflight_before_program_mutation": "1",
            "program_bound_before_audit_mutation": "1",
            "descriptor_driven": "1",
            "arbitrary_program_claim": "0",
            "external_effect_exactly_once_claim": "0",
            "hardware_powercut_claim": "0",
            "emulator_only": "1",
            "real_phone_claim": "0",
        },
        PROGRAM_PREFIX,
    )

    operations = [
        parse_loose([line], OPERATION_PREFIX)
        for line in lines
        if line.startswith(OPERATION_PREFIX + " ")
    ]
    expected_phases = tuple(
        (operation, phase_value)
        for operation in (1, 2, 3)
        for phase_value in (1, 2, 3)
    )
    if len(operations) != len(expected_phases):
        raise EvidenceError("M81 did not bind exactly nine descriptor phases")
    for fields, (ordinal, phase_value) in zip(operations, expected_phases):
        kind, cancellable, argument, token = EXPECTED_DESCRIPTORS[ordinal - 1]
        require(
            fields,
            {
                "format": "1",
                "artifact": "BMP1",
                "state": "BNDRMPB1",
                "operation": str(ordinal),
                "phase": str(phase_value),
                "kind": str(kind),
                "argument": str(argument),
                "token": str(token),
                "idempotent": "1",
                "preapply_cancellable": str(cancellable),
                "exact_program": "1",
                "descriptor_driven": "1",
                "arbitrary_program_claim": "0",
                "external_effect_exactly_once_claim": "0",
                "emulator_only": "1",
                "real_phone_claim": "0",
            },
            OPERATION_PREFIX,
        )

    terminal = parse_loose(lines, TERMINAL_PREFIX)
    require(
        terminal,
        {
            "format": "1",
            "artifact": "BMP1",
            "state": "BNDRMPB1",
            "state_version": "1",
            "relative_lbas": "13/14",
            "sequence": "2",
            "operation_count": "3",
            "transition_count": "9",
            "program_version": "1",
            "generation": "1",
            "slot": "0",
            "valid_slots": "1",
            "rejected_slots": "1",
            "reads": "10",
            "writes": "0",
            "flushes": "0",
            "program_sha256": slash(bytes(material["program_sha256"])),
            "descriptor_sha256": slash(bytes(material["descriptor_sha256"])),
            "program_chain_sha256": slash(chain),
            "plan_chain_sha256": slash(plan_chain),
            "exact_signed_program": "1",
            "terminal_m80_plan_bound": "1",
            "hardware_powercut_claim": "0",
            "emulator_only": "1",
            "real_phone_claim": "0",
        },
        TERMINAL_PREFIX,
    )

    final = parse_loose(lines, FINAL_PREFIX)
    expected_runtime = runtime_digest(lines, plan_chain, chain)
    require(
        final,
        {
            "format": "1",
            "abi": "42",
            "artifact": "BMP1",
            "artifact_bytes": "512",
            "signed_bytes": "256",
            "root_key_id": "1",
            "root_sha256": ROOT_SHA256.hex(),
            "authorization_sequence": "2",
            "operation_count": "3",
            "transition_count": "9",
            "program_version": "1",
            "plan_generation": "2",
            "preapply_cancel_mask": "1",
            "descriptor_capacity": "4",
            "descriptor_size": "16",
            "program_state": "BNDRMPB1",
            "program_state_version": "1",
            "program_relative_lbas": "13/14",
            "program_provisioned_before": str(int(replayed)),
            "program_replayed": str(int(replayed)),
            "predecessor_complete": "1",
            "program_initial_generation": "1" if replayed else "0",
            "program_committed_generation": "1",
            "program_initial_slot": "0" if replayed else "255",
            "program_committed_slot": "0",
            "program_initial_valid_slots": "1" if replayed else "0",
            "program_initial_rejected_slots": "1" if replayed else "0",
            "program_committed_valid_slots": "1",
            "program_committed_rejected_slots": "1",
            "program_reads": "10" if replayed else "14",
            "program_writes": "0" if replayed else "1",
            "program_flushes": "0" if replayed else "1",
            "terminal_generation": "1",
            "terminal_slot": "0",
            "terminal_valid_slots": "1",
            "terminal_rejected_slots": "1",
            "terminal_reads": "10",
            "descriptor_bound_phases": "9",
            "program_sha256": bytes(material["program_sha256"]).hex(),
            "descriptor_sha256": bytes(material["descriptor_sha256"]).hex(),
            "plan_id_sha256": bytes(material["plan_id_sha256"]).hex(),
            "program_chain_sha256": chain.hex(),
            "plan_chain_sha256": plan_chain.hex(),
            "runtime_evidence_sha256": expected_runtime.hex(),
            "signed_before_semantic_binding": "1",
            "verified_before_program_mutation": "1",
            "program_bound_before_audit_mutation": "1",
            "exact_artifact_replay_read_only": str(int(replayed)),
            "bounded_descriptor_execution": "1",
            "arbitrary_program_claim": "0",
            "external_effect_exactly_once_claim": "0",
            "offline_split_signing_supported": "1",
            "private_key_in_repository": "0",
            "production_key_claim": "0",
            "trusted_monotonic_backend": "0",
            "hardware_powercut_claim": "0",
            "emulator_only": "1",
            "real_phone_claim": "0",
        },
        FINAL_PREFIX,
    )


def read_sector(path: Path, lba: int) -> bytes:
    with path.open("rb") as stream:
        stream.seek(lba * fixture.SECTOR_BYTES)
        sector = stream.read(fixture.SECTOR_BYTES)
    if len(sector) != fixture.SECTOR_BYTES:
        raise EvidenceError(f"disk ended before sector {lba}")
    return sector


def program_slot_digest(path: Path) -> str:
    slots = b"".join(read_sector(path, lba) for lba in PROGRAM_LBAS)
    return hashlib.sha256(slots).hexdigest()


def assert_empty_program(path: Path) -> str:
    slots = b"".join(read_sector(path, lba) for lba in PROGRAM_LBAS)
    if slots != bytes(fixture.SECTOR_BYTES * len(PROGRAM_LBAS)):
        raise EvidenceError("M81 program slots changed before durable admission")
    return (
        "SIGNED_MAINTENANCE_PLAN_EMPTY_DISK_OK slots=2 relative_lbas=13/14 "
        f"sha256={hashlib.sha256(slots).hexdigest()}"
    )


def decode_program_record(sector: bytes) -> dict[str, bytes | int]:
    material = artifact_material()
    if (
        len(sector) != fixture.SECTOR_BYTES
        or sector[:8] != RECORD_MAGIC
        or struct.unpack_from("<I", sector, 8)[0] != 1
        or struct.unpack_from("<I", sector, 12)[0] != 80
        or struct.unpack_from("<I", sector, 24)[0] != PROGRAM_BYTES
        or struct.unpack_from("<I", sector, 28)[0] != 1
        or sector[36:40] != bytes(4)
        or sector[64:80] != fixture.DATA_FORMAT_EPOCH
        or sector[416:508] != bytes(92)
    ):
        raise EvidenceError("M81 program record envelope changed")
    generation = struct.unpack_from("<Q", sector, 16)[0]
    payload = sector[80:416]
    if (
        generation == 0
        or struct.unpack_from("<Q", sector, 48)[0]
        != (~generation & 0xFFFF_FFFF_FFFF_FFFF)
        or struct.unpack_from("<Q", sector, 56)[0] != RECORD_COOKIE
        or fixture.crc32(sector[:508]) != struct.unpack_from("<I", sector, 508)[0]
        or fixture.crc32(payload) != struct.unpack_from("<I", sector, 32)[0]
        or fixture.fnv1a64(payload) != struct.unpack_from("<Q", sector, 40)[0]
        or payload[:8] != PROGRAM_MAGIC
        or struct.unpack_from("<I", payload, 8)[0] != 1
        or payload[12:16] != bytes(4)
    ):
        raise EvidenceError("M81 program record commit proof changed")
    fields: dict[str, bytes | int] = {
        "generation": generation,
        "sequence": struct.unpack_from("<Q", payload, 16)[0],
        "operation_count": struct.unpack_from("<I", payload, 24)[0],
        "transition_count": struct.unpack_from("<I", payload, 28)[0],
        "program_version": struct.unpack_from("<I", payload, 32)[0],
        "cancel_mask": struct.unpack_from("<I", payload, 36)[0],
        "authorization_sha256": payload[40:72],
        "authorization_id": payload[72:104],
        "program_sha256": payload[104:136],
        "plan_id_sha256": payload[136:168],
        "descriptor_sha256": payload[168:200],
        "root_sha256": payload[200:232],
        "device_binding_sha256": payload[232:264],
        "previous_chain_sha256": payload[264:296],
        "chain_sha256": payload[296:328],
    }
    if (
        fields["sequence"] != 2
        or fields["operation_count"] != 3
        or fields["transition_count"] != 9
        or fields["program_version"] != 1
        or fields["cancel_mask"] != 1
        or fields["authorization_sha256"] != material["authorization_sha256"]
        or fields["authorization_id"] != material["authorization_id"]
        or fields["program_sha256"] != material["program_sha256"]
        or fields["plan_id_sha256"] != material["plan_id_sha256"]
        or fields["descriptor_sha256"] != material["descriptor_sha256"]
        or fields["root_sha256"] != ROOT_SHA256
        or fields["device_binding_sha256"] != material["device_binding_sha256"]
        or fields["previous_chain_sha256"] != bytes(32)
        or fields["chain_sha256"] != program_chain()
        or struct.unpack_from("<Q", payload, 328)[0]
        != fixture.fnv1a64(payload[16:328]) ^ PROGRAM_WITNESS
    ):
        raise EvidenceError("M81 durable program identity or chain changed")
    return fields


def select_program(path: Path) -> dict[str, object]:
    valid: list[tuple[int, int, dict[str, bytes | int]]] = []
    rejected = 0
    for slot, lba in enumerate(PROGRAM_LBAS):
        sector = read_sector(path, lba)
        if sector == bytes(fixture.SECTOR_BYTES):
            rejected += 1
            continue
        try:
            state = decode_program_record(sector)
        except EvidenceError:
            rejected += 1
            continue
        valid.append((int(state["generation"]), slot, state))
    if not valid:
        raise EvidenceError("M81 disk contains no valid signed program binding")
    valid.sort(key=lambda item: item[0])
    if len(valid) == 2 and valid[0][0] == valid[1][0]:
        raise EvidenceError("M81 program slots have an ambiguous generation")
    generation, slot, state = valid[-1]
    return {
        "generation": generation,
        "slot": slot,
        "valid_slots": len(valid),
        "rejected_slots": rejected,
        "state": state,
    }


def corrupt_selected(path: Path) -> str:
    selected = select_program(path)
    lba = PROGRAM_LBAS[int(selected["slot"])]
    offset = lba * fixture.SECTOR_BYTES + 184
    with path.open("r+b") as stream:
        stream.seek(offset)
        original = stream.read(1)
        if len(original) != 1:
            raise EvidenceError("M81 selected program byte is unavailable")
        stream.seek(offset)
        stream.write(bytes((original[0] ^ 1,)))
        stream.flush()
    try:
        select_program(path)
    except EvidenceError:
        pass
    else:
        raise EvidenceError("M81 corrupt-only program ledger remained selectable")
    return (
        "SIGNED_MAINTENANCE_PLAN_CORRUPT_OK selected_slot="
        f"{selected['slot']} valid_before={selected['valid_slots']} "
        "valid_after=0 fail_closed=1"
    )


def build_self_test_sector() -> bytes:
    material = artifact_material()
    payload = bytearray(PROGRAM_BYTES)
    payload[:8] = PROGRAM_MAGIC
    struct.pack_into("<I", payload, 8, 1)
    struct.pack_into("<QIIII", payload, 16, 2, 3, 9, 1, 1)
    payload[40:72] = bytes(material["authorization_sha256"])
    payload[72:104] = bytes(material["authorization_id"])
    payload[104:136] = bytes(material["program_sha256"])
    payload[136:168] = bytes(material["plan_id_sha256"])
    payload[168:200] = bytes(material["descriptor_sha256"])
    payload[200:232] = ROOT_SHA256
    payload[232:264] = bytes(material["device_binding_sha256"])
    payload[296:328] = program_chain()
    struct.pack_into(
        "<Q", payload, 328, fixture.fnv1a64(payload[16:328]) ^ PROGRAM_WITNESS
    )
    sector = bytearray(fixture.SECTOR_BYTES)
    sector[:8] = RECORD_MAGIC
    struct.pack_into("<I", sector, 8, 1)
    struct.pack_into("<I", sector, 12, 80)
    struct.pack_into("<Q", sector, 16, 1)
    struct.pack_into("<I", sector, 24, PROGRAM_BYTES)
    struct.pack_into("<I", sector, 28, 1)
    struct.pack_into("<I", sector, 32, fixture.crc32(payload))
    struct.pack_into("<Q", sector, 40, fixture.fnv1a64(payload))
    struct.pack_into("<Q", sector, 48, ~1 & 0xFFFF_FFFF_FFFF_FFFF)
    struct.pack_into("<Q", sector, 56, RECORD_COOKIE)
    sector[64:80] = fixture.DATA_FORMAT_EPOCH
    sector[80:416] = payload
    struct.pack_into("<I", sector, 508, fixture.crc32(sector[:508]))
    return bytes(sector)


def self_test() -> str:
    decoded = decode_program_record(build_self_test_sector())
    if decoded["chain_sha256"] != program_chain():
        raise EvidenceError("M81 self-test chain changed")
    corrupt = bytearray(build_self_test_sector())
    corrupt[200] ^= 1
    try:
        decode_program_record(bytes(corrupt))
    except EvidenceError:
        pass
    else:
        raise EvidenceError("M81 self-test accepted record corruption")
    return (
        "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_PARSER_SELF_TEST_OK "
        "format=1 abi=42 artifact=BMP1 signed_bytes=256 signature_bytes=256 "
        "program_record=BNDRMPB1 bytes=336 descriptors=3 transitions=9 "
        "crc_rejection=1 witness_rejection=1 hash_chain_verified=1"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    positive = subparsers.add_parser("parse")
    positive.add_argument("phase", choices=PHASES)
    positive.add_argument("log", type=Path)
    inspect = subparsers.add_parser("inspect-disk")
    inspect.add_argument("disk", type=Path)
    empty = subparsers.add_parser("assert-empty")
    empty.add_argument("disk", type=Path)
    digest = subparsers.add_parser("program-digest")
    digest.add_argument("disk", type=Path)
    corrupt = subparsers.add_parser("corrupt-selected")
    corrupt.add_argument("disk", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-test":
        print(self_test())
    elif args.command == "parse":
        validate_positive(
            args.log.read_text(encoding="utf-8", errors="replace"), args.phase
        )
        print(
            "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_EVIDENCE_OK "
            f"phase={args.phase} abi=42 descriptors=3 descriptor_phases=9 "
            "qemu_psci_self_exit=1"
        )
    elif args.command == "inspect-disk":
        selected = select_program(args.disk)
        print(
            "SIGNED_MAINTENANCE_PLAN_DISK_OK "
            f"generation={selected['generation']} slot={selected['slot']} "
            f"valid_slots={selected['valid_slots']} "
            f"rejected_slots={selected['rejected_slots']} "
            f"chain_sha256={program_chain().hex()}"
        )
    elif args.command == "assert-empty":
        print(assert_empty_program(args.disk))
    elif args.command == "program-digest":
        print(program_slot_digest(args.disk))
    elif args.command == "corrupt-selected":
        print(corrupt_selected(args.disk))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
