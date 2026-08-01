#!/usr/bin/env python3
"""Strict M67 serial-evidence parser and two-boot disk-boundary verifier."""

from __future__ import annotations

import argparse
import re
import struct
import sys
from pathlib import Path


UI_MARKER = (
    "UNIFIED_PRODUCT_UI_OK format=1 abi=28 profile=m45-real-ui "
    "input_server_pid=4294967302 input_session=1 windows=2 "
    "physical=pointer47+key12 graphics=21/21/21/21 text=complete "
    "soft_keyboard=complete processes=10/1/1/9 process_capacity=10/9 "
    "handles=36 endpoints=30 pairs=15 input_pairs=2 waits=9/2/7 "
    "ui_converged=1 init_ready=0 storage_spawned=0 power_key_code=116 "
    "power_request=awaiting emulator_only=1 hardware_poweroff_claim=0 "
    "real_phone_claim=0"
)
UI_BOOT_MARKER = (
    "BOOT_OK: M67 unified real UI is interactive and awaiting authenticated power key"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M67 interactive product, AppData, bounded shutdown, and QEMU exit armed"
)
FINAL_SCREEN_SHA256 = (
    "1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
)
FAILURE_PATTERN = re.compile(
    r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
    r"USER_FAIL|EL0_FAIL|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAILED)(?::| |$)|"
    r"^M67_(?:FRAME|STACK|EXIT|SYSCALL)_TRACE(?: |$)",
    re.IGNORECASE | re.MULTILINE,
)

PHASES = {
    "first": {
        "legacy_upgrade": 1,
        "persisted_contract": 0,
        "data_initial": 0,
        "data_committed": 1,
        "valid_slots": 1,
        "rejected_slots": 1,
        "appdata_generation": 5,
        "open_generation": 1,
        "closed_generation": 2,
        "storage_server_submissions": 1878,
    },
    "second": {
        "legacy_upgrade": 0,
        "persisted_contract": 1,
        "data_initial": 2,
        "data_committed": 3,
        "valid_slots": 2,
        "rejected_slots": 0,
        "appdata_generation": 6,
        "open_generation": 3,
        "closed_generation": 4,
        "storage_server_submissions": 660,
    },
}


class EvidenceError(ValueError):
    pass


def data_marker(phase: str) -> str:
    values = PHASES[phase]
    return (
        "DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 "
        f"slots=2 initial_generation={values['data_initial']} "
        f"committed_generation={values['data_committed']} initial_slot=0 "
        f"committed_slot=1 valid_slots={values['valid_slots']} "
        f"rejected_slots={values['rejected_slots']} reads=5 writes=1 "
        "flushes=1 write_completion=1 flush_completion=1 readback_verified=1 "
        "old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 "
        "rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 "
        "crash_consistency=0 qemu_reboot_proof=0"
    )


def health_marker(phase: str) -> str:
    values = PHASES[phase]
    return (
        "STORAGE_DEVICE_HEALTH_OK format=1 state_version=1 "
        "authority=kernel-boot-probe record_role=unclosed-boot-hint "
        f"legacy_upgrade={values['legacy_upgrade']} "
        f"persisted_contract={values['persisted_contract']} "
        "prior_boot_open=0 contract_changed=0 reprobe_required=0 "
        "reprobe_verified=1 current_boot_open=1 offline_persisted=0 "
        "offline_from_record=0 el0_controls=0 capacity_sectors=16384 "
        "selected_features=0x0000000100000200 "
        "transport_base=0x000000000a003e00 irq=79 sector_bytes=512 "
        "queue_size=8 device_read_only=0 flush_supported=1 "
        f"initial_generation={values['data_initial']} "
        f"committed_generation={values['data_committed']} initial_slot=0 "
        "committed_slot=1 reads=5 writes=1 flushes=1 readback_verified=1 "
        "qemu_reboot_proof=0 hardware_identity_claim=0 hotplug_claim=0 "
        "powercut_claim=0 tamper_resistance_claim=0 general_runtime=0"
    )


def shutdown_marker(phase: str) -> str:
    values = PHASES[phase]
    return (
        "UNIFIED_PRODUCT_SHUTDOWN_OK format=1 abi=28 "
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
        "quiesce_calls=9 order_rejections=1 children_created=11 "
        "children_exited=10 children_reaped=10 live_processes=1 "
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
        raise EvidenceError(f"unknown M67 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    lines = normalized.splitlines()

    expected_boots = {UI_BOOT_MARKER, SHUTDOWN_BOOT_MARKER}
    observed_boots = [line for line in lines if line.startswith("BOOT_OK:")]
    if len(observed_boots) != 2 or set(observed_boots) != expected_boots:
        raise EvidenceError("M67 BOOT_OK marker set changed")

    if exact_line(lines, "DATA_PERSIST_OK") != data_marker(phase):
        raise EvidenceError("M67 DATA_PERSIST_OK marker changed")
    if exact_line(lines, "STORAGE_DEVICE_HEALTH_OK") != health_marker(phase):
        raise EvidenceError("M67 STORAGE_DEVICE_HEALTH_OK marker changed")
    if exact_line(lines, "UNIFIED_PRODUCT_UI_OK") != UI_MARKER:
        raise EvidenceError("M67 interactive UI marker changed")
    if exact_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK") != shutdown_marker(phase):
        raise EvidenceError("M67 shutdown marker changed")
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
        data_marker(phase),
        health_marker(phase),
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
            shutdown_marker(phase),
            SHUTDOWN_BOOT_MARKER,
        )
    )
    return "\n".join(lines) + "\n"


def parser_self_test() -> None:
    for phase in PHASES:
        validate_text(synthetic_log(phase), phase)

    base = synthetic_log("first")
    rejected = 0
    mutations = (
        base.replace("abi=28", "abi=29", 1),
        base + shutdown_marker("first") + "\n",
        base.replace(UI_BOOT_MARKER, "BOOT_OK: M66 stale parent marker", 1),
        base.replace("WINDOW_INPUT_PHASE2_READY\n", "", 1),
        base.replace("real_phone_claim=0", "real_phone_claim=1", 1),
        base + "M67_SYSCALL_TRACE pid=1 syscall=1\n",
    )
    for mutated in mutations:
        try:
            validate_text(mutated, "first")
        except EvidenceError:
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M67 parser negative self-test accepted a mutation")
    print(
        "UNIFIED_PRODUCT_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES)} negative={rejected}"
    )


def fnv1a64(data: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x00000100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def verify_disk(pristine_path: Path, runtime_path: Path) -> str:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import build_storage_image as fixture

    before = pristine_path.read_bytes()
    after = runtime_path.read_bytes()
    sector_bytes = fixture.SECTOR_BYTES
    if len(before) != 8 * 1024 * 1024 or len(after) != len(before):
        raise EvidenceError("M67 runtime disk lost its exact writable 8 MiB geometry")

    data_start = fixture.DATA_PARTITION_FIRST_LBA * sector_bytes
    data_end = (fixture.DATA_PARTITION_LAST_LBA + 1) * sector_bytes
    appdata_start = fixture.APPDATA_PARTITION_FIRST_LBA * sector_bytes
    appdata_end = (fixture.APPDATA_PARTITION_LAST_LBA + 1) * sector_bytes
    if before[:data_start] != after[:data_start] or before[appdata_end:] != after[appdata_end:]:
        raise EvidenceError("M67 changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
    if before[appdata_start:appdata_end] == after[appdata_start:appdata_end]:
        raise EvidenceError("M67 did not persist its bounded AppData workload")

    def sector(image: bytes, lba: int) -> bytes:
        start = lba * sector_bytes
        return image[start : start + sector_bytes]

    def decode_health_record(raw: bytes, generation: int, boot_open: bool) -> None:
        if raw[:8] != fixture.DATA_RECORD_MAGIC:
            raise EvidenceError("M67 health record magic changed")
        if struct.unpack_from("<I", raw, 8)[0] != fixture.DATA_FORMAT_VERSION:
            raise EvidenceError("M67 health record version changed")
        if struct.unpack_from("<I", raw, 12)[0] != fixture.DATA_RECORD_HEADER_BYTES:
            raise EvidenceError("M67 health record header size changed")
        if struct.unpack_from("<Q", raw, 16)[0] != generation:
            raise EvidenceError("M67 health generation changed")
        if struct.unpack_from("<I", raw, 24)[0] != 80:
            raise EvidenceError("M67 health payload size changed")
        if struct.unpack_from("<I", raw, 28)[0] != fixture.DATA_RECORD_COMMITTED:
            raise EvidenceError("M67 health record is not committed")
        if struct.unpack_from("<Q", raw, 48)[0] != (
            (~generation) & 0xFFFFFFFFFFFFFFFF
        ):
            raise EvidenceError("M67 health generation witness changed")
        if struct.unpack_from("<Q", raw, 56)[0] != fixture.DATA_RECORD_COMMIT_COOKIE:
            raise EvidenceError("M67 health commit cookie changed")
        if raw[64:80] != fixture.DATA_FORMAT_EPOCH:
            raise EvidenceError("M67 health format epoch changed")
        payload = raw[80:160]
        if fixture.crc32(payload) != struct.unpack_from("<I", raw, 32)[0]:
            raise EvidenceError("M67 health payload CRC is invalid")
        if fnv1a64(payload) != struct.unpack_from("<Q", raw, 40)[0]:
            raise EvidenceError("M67 health payload digest is invalid")
        if any(raw[160 : fixture.DATA_RECORD_CRC_OFFSET]):
            raise EvidenceError("M67 health record padding changed")
        if fixture.crc32(raw[: fixture.DATA_RECORD_CRC_OFFSET]) != struct.unpack_from(
            "<I", raw, fixture.DATA_RECORD_CRC_OFFSET
        )[0]:
            raise EvidenceError("M67 health record CRC is invalid")
        if payload[:8] != b"BNDRHLT1":
            raise EvidenceError("M67 health payload magic changed")
        if struct.unpack_from("<I", payload, 8)[0] != 1:
            raise EvidenceError("M67 health payload version changed")
        if struct.unpack_from("<I", payload, 12)[0] != int(boot_open):
            raise EvidenceError("M67 persisted boot-open state changed")
        if struct.unpack_from("<Q", payload, 16)[0] != generation:
            raise EvidenceError("M67 health boot count changed")
        expected_contract = (
            16_384,
            0x0000000100000200,
            0x000000000A003E00,
            79,
            512,
            8,
            2,
        )
        observed_contract = (
            struct.unpack_from("<Q", payload, 24)[0],
            struct.unpack_from("<Q", payload, 32)[0],
            struct.unpack_from("<Q", payload, 40)[0],
            struct.unpack_from("<I", payload, 48)[0],
            struct.unpack_from("<I", payload, 52)[0],
            struct.unpack_from("<I", payload, 56)[0],
            struct.unpack_from("<I", payload, 60)[0],
        )
        if observed_contract != expected_contract:
            raise EvidenceError("M67 persisted device contract changed")
        contract_digest = struct.unpack_from("<Q", payload, 64)[0]
        if contract_digest != fnv1a64(payload[24:64]):
            raise EvidenceError("M67 health contract digest is invalid")
        witness = generation ^ contract_digest ^ 0xB24D63DA7A5EC001
        if struct.unpack_from("<Q", payload, 72)[0] != witness:
            raise EvidenceError("M67 health state witness is invalid")

    if sector(after, fixture.DATA_PARTITION_FIRST_LBA) != sector(
        before, fixture.DATA_PARTITION_FIRST_LBA
    ):
        raise EvidenceError("M67 immutable data superblock changed")
    slot_zero_lba = (
        fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[0]
    )
    slot_one_lba = (
        fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
    )
    decode_health_record(sector(after, slot_zero_lba), 4, False)
    decode_health_record(sector(after, slot_one_lba), 3, True)
    unused_start = (fixture.DATA_PARTITION_FIRST_LBA + 3) * sector_bytes
    if after[unused_start:data_end] != before[unused_start:data_end] or any(
        after[unused_start:data_end]
    ):
        raise EvidenceError("M67 changed an unused BNDROID_DATA sector")

    changed_data_bytes = sum(
        left != right
        for left, right in zip(before[data_start:data_end], after[data_start:data_end])
    )
    changed_appdata_bytes = sum(
        left != right
        for left, right in zip(
            before[appdata_start:appdata_end], after[appdata_start:appdata_end]
        )
    )
    return (
        "UNIFIED_PRODUCT_REBOOT_OK boots=2 qemu_self_exits=2 "
        "authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 "
        f"ui_sha256={FINAL_SCREEN_SHA256} product_shutdowns=2 resident_nodes=8 "
        "dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 "
        "order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 "
        "storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 "
        "commits=2 spawn_rejections=2 connect_rejections=4 "
        "final_appdata_generation=6 final_health_generation=4 "
        "final_health_slot=0 prior_health_generation=3 prior_health_slot=1 "
        "final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 "
        "outside_data_appdata_unchanged=1 unused_data_unchanged=1 "
        f"appdata_changed=1 changed_data_bytes={changed_data_bytes} "
        f"changed_appdata_bytes={changed_appdata_bytes} emulator_only=1 "
        "hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 "
        "general_runtime=0 real_phone_claim=0"
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
            print(f"UNIFIED_PRODUCT_EVIDENCE_OK phase={args.phase}")
        elif args.command == "disk":
            print(verify_disk(args.pristine, args.runtime))
        else:
            raise AssertionError("unreachable command")
    except (EvidenceError, OSError) as error:
        print(f"unified_product_evidence.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
