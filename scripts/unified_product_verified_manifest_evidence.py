#!/usr/bin/env python3
"""Strict M74 signed-manifest, rollback, event, and reboot evidence parser."""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

import unified_product_event_supervision_evidence as m73


PHASES = m73.PHASES
FINAL_SCREEN_SHA256 = m73.FINAL_SCREEN_SHA256
UI_MARKER = m73.UI_MARKER.replace("abi=34", "abi=35", 1)
UI_BOOT_MARKER = (
    "BOOT_OK: M74 unified real UI is interactive; verified-manifest event "
    "supervision is starting"
)
SHUTDOWN_BOOT_MARKER = (
    "BOOT_OK: M74 verified external manifest, rollback floor, event "
    "supervision, AppData, and PSCI shutdown armed"
)
OLD_AUTHORITY = (
    "kernel-owned-bmf1-plus-init-event-loop-plus-kernel-authenticated-report"
)
AUTHORITY = (
    "external-bms1-plus-kernel-pinned-rsa2048-plus-static-rollback-floor-plus-"
    "init-event-loop-plus-kernel-authenticated-report"
)
SIGNED_SHA256 = "99c3865eb64c5314c96ee87318924c9dd29a4bf55fdb571499d36db9cb2f7bc3"
TRUST_ANCHOR_SHA256 = (
    "6249c4c758ecd9117cd3799d705a371fac64a210fa464c0e64007fccc30c7d6c"
)
RSA_MODULUS_HEX = (
    "ac4b9f4890228584eeee8333c84221db1a3273366c4b6e9783f7c279a77d804d"
    "76555f4a57f0fe69974da419d99fccaf1f72ab4aae797464914a0601dea42c83"
    "b33d4ffe020880a83b2418e0795380654b8a9567310f1531ba78c0cbbeae93e3"
    "67d3825ebc88d63f593d89fb69b819ee0eac23273ba14054c5114cb87d5c3d3"
    "3529ec7b331e1ab401617a8be9966721ffdaf1486b384b68b080c84f5038e76"
    "e190fdba1ee136803157756021a9b791e4ae27938d507b09b774e63dc5fb8bc"
    "13a2ba9dc0913977490c84170c004ec2a94ba62e49974154f41d08284bbc739"
    "9fe4aefab4baab7349a8a6e2c696ff0d1e3729dd2f815f6ba4b30d59104820"
    "f5286b"
)
RSA_MODULUS = bytes.fromhex(RSA_MODULUS_HEX)
RSA_EXPONENT = 65537
SHA256_DIGEST_INFO_PREFIX = bytes.fromhex(
    "3031300d060960864801650304020105000420"
)
FAILURE_PATTERN = re.compile(
    m73.FAILURE_PATTERN.pattern + r"|VERIFIED_MANIFEST_REJECTED",
    m73.FAILURE_PATTERN.flags,
)

VERIFIED_PREFIX = "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK"
VERIFIED_KEYS = (
    "format",
    "abi",
    "artifact",
    "artifact_bytes",
    "signed_bytes",
    "payload",
    "payload_bytes",
    "algorithm",
    "key_id",
    "rollback_index",
    "rollback_floor",
    "verification_attempts",
    "verification_successes",
    "signature_successes",
    "signature_rejections",
    "rollback_rejections",
    "format_rejections",
    "signed_sha256",
    "trust_anchor_sha256",
    "external_artifact",
    "kernel_pinned_key",
    "static_rollback_floor",
    "manifest_published_after_verification",
    "private_key_in_repository",
    "production_key_claim",
    "rpmb_claim",
    "efuse_claim",
    "hardware_rollback_claim",
    "emulator_only",
    "real_phone_claim",
)
VERIFIED_EXPECTED = {
    "format": "1",
    "abi": "35",
    "artifact": "BMS1",
    "artifact_bytes": "512",
    "signed_bytes": "256",
    "payload": "BMF1",
    "payload_bytes": "224",
    "algorithm": "RSA2048-PKCS1-v1_5-SHA256",
    "key_id": "1",
    "rollback_index": "2",
    "rollback_floor": "2",
    "verification_attempts": "1",
    "verification_successes": "1",
    "signature_successes": "1",
    "signature_rejections": "0",
    "rollback_rejections": "0",
    "format_rejections": "0",
    "signed_sha256": SIGNED_SHA256,
    "trust_anchor_sha256": TRUST_ANCHOR_SHA256,
    "external_artifact": "1",
    "kernel_pinned_key": "1",
    "static_rollback_floor": "1",
    "manifest_published_after_verification": "1",
    "private_key_in_repository": "0",
    "production_key_claim": "0",
    "rpmb_claim": "0",
    "efuse_claim": "0",
    "hardware_rollback_claim": "0",
    "emulator_only": "1",
    "real_phone_claim": "0",
}

SIGNATURE_REJECTION = (
    "VERIFIED_MANIFEST_REJECTED format=1 reason=signature key_id=1 "
    "rollback_floor=2 init_ready=0 manifest_published=0"
)
ROLLBACK_REJECTION = (
    "VERIFIED_MANIFEST_REJECTED format=1 reason=rollback key_id=1 "
    "rollback_index=1 rollback_floor=2 signature_valid=1 init_ready=0 "
    "manifest_published=0"
)
NEGATIVE_DRIVER = (
    "VERIFIED_MANIFEST_NEGATIVE_BOOT_OK reason={reason} failure=0x7201 "
    "ui_converged=1 manifest_published=0 init_ready=0 "
    "qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
)


class EvidenceError(ValueError):
    pass


def verified_marker() -> str:
    return VERIFIED_PREFIX + " " + " ".join(
        f"{key}={VERIFIED_EXPECTED[key]}" for key in VERIFIED_KEYS
    )


def normalize_positive(text: str) -> str:
    normalized = text.replace("\r", "")
    marker = verified_marker()
    if normalized.splitlines().count(marker) != 1:
        raise EvidenceError("M74 verified-manifest marker count changed")
    normalized = normalized.replace(marker + "\n", "", 1)
    normalized = normalized.replace(UI_BOOT_MARKER, m73.UI_BOOT_MARKER, 1)
    normalized = normalized.replace(SHUTDOWN_BOOT_MARKER, m73.SHUTDOWN_BOOT_MARKER, 1)
    normalized = normalized.replace(AUTHORITY, OLD_AUTHORITY, 1)
    normalized = normalized.replace("manifest_generation=2", "manifest_generation=1", 1)
    normalized = normalized.replace("abi=35", "abi=34")
    return normalized


def validate_positive(text: str, phase: str) -> dict[str, int]:
    if phase not in PHASES:
        raise EvidenceError(f"unknown M74 phase: {phase}")
    normalized = text.replace("\r", "")
    failure = FAILURE_PATTERN.search(normalized)
    if failure is not None:
        raise EvidenceError(f"failure evidence is present: {failure.group(0)!r}")
    if "abi=34" in normalized:
        raise EvidenceError("M74 retained an ABI-v34 marker")
    lines = normalized.splitlines()

    verification = m73.unique_line(lines, VERIFIED_PREFIX)
    fields = m73.parse_fields(verification, VERIFIED_PREFIX, VERIFIED_KEYS)
    m73.require_fixed(fields, VERIFIED_EXPECTED, VERIFIED_PREFIX)
    event = m73.unique_line(lines, m73.EVENT_PREFIX)
    event_fields = m73.parse_fields(event, m73.EVENT_PREFIX, m73.EVENT_KEYS)
    if (
        event_fields["abi"] != "35"
        or event_fields["authority"] != AUTHORITY
        or event_fields["manifest_generation"] != "2"
    ):
        raise EvidenceError("M74 verified BMF1 event authority changed")
    if lines.count(UI_BOOT_MARKER) != 1 or lines.count(SHUTDOWN_BOOT_MARKER) != 1:
        raise EvidenceError("M74 BOOT_OK marker set changed")
    if not (
        lines.index(event)
        < lines.index(verification)
        < lines.index(m73.unique_line(lines, "UNIFIED_PRODUCT_SHUTDOWN_OK"))
    ):
        raise EvidenceError("M74 verification/event/shutdown order changed")

    return m73.validate_text(normalize_positive(normalized), phase)


def validate_driver(text: str, phase: str) -> None:
    try:
        m73.validate_driver(text, phase)
    except ValueError as error:
        raise EvidenceError(str(error)) from error


def validate_negative(text: str, driver: str, reason: str) -> None:
    if reason not in {"signature", "rollback"}:
        raise EvidenceError(f"unknown M74 rejection reason: {reason}")
    lines = text.replace("\r", "").splitlines()
    rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
    if lines.count(UI_MARKER) != 1:
        raise EvidenceError(f"M74 {reason} negative UI marker changed")
    if lines.count(UI_BOOT_MARKER) != 1:
        raise EvidenceError(f"M74 {reason} negative UI boot marker changed")
    if lines.count(rejection) != 1:
        raise EvidenceError(f"M74 {reason} rejection marker changed")
    diagnostics = [
        line
        for line in lines
        if line.startswith("M66_PLATFORM_SHUTDOWN_DIAG failure=29185 ")
    ]
    if len(diagnostics) != 1:
        raise EvidenceError(f"M74 {reason} init-failure diagnostic changed")
    forbidden = (
        "M73_EVENT_SUPERVISOR_ACTIVE_OK ",
        "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK ",
        "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK ",
        "UNIFIED_PRODUCT_SHUTDOWN_OK ",
        SHUTDOWN_BOOT_MARKER,
    )
    if any(any(line.startswith(prefix) for prefix in forbidden) for line in lines):
        raise EvidenceError(f"M74 {reason} negative boot crossed publication boundary")
    order = [
        lines.index(UI_MARKER),
        lines.index(UI_BOOT_MARKER),
        lines.index(rejection),
        lines.index(diagnostics[0]),
    ]
    if order != sorted(order):
        raise EvidenceError(f"M74 {reason} negative marker order changed")
    driver_lines = [line for line in driver.replace("\r", "").splitlines() if line]
    expected_driver = NEGATIVE_DRIVER.format(reason=reason)
    if driver_lines != [expected_driver]:
        raise EvidenceError(f"M74 {reason} negative host evidence changed")


def decode_hex_artifact(path: Path) -> bytes:
    encoded = path.read_text(encoding="ascii")
    digits = "".join(encoded.split())
    if not digits or len(digits) % 2 or re.fullmatch(r"[0-9a-f]+", digits) is None:
        raise EvidenceError(f"{path} is not canonical lowercase hexadecimal")
    return bytes.fromhex(digits)


def inspect_artifact(artifact: bytes) -> dict[str, object]:
    if len(artifact) != 512:
        raise EvidenceError("BMS1 artifact length changed")
    if (
        artifact[:4] != b"BMS1"
        or artifact[4:8] != bytes((1, 1, 1, 0))
        or int.from_bytes(artifact[8:12], "little") != 512
        or int.from_bytes(artifact[12:16], "little") != 256
        or int.from_bytes(artifact[16:20], "little") != 32
        or int.from_bytes(artifact[20:24], "little") != 224
        or artifact[28:32] != bytes(4)
    ):
        raise EvidenceError("BMS1 strict envelope changed")
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
        raise EvidenceError("signed BMF1 payload header changed")
    generation = int.from_bytes(payload[12:16], "little")
    if generation != rollback_index:
        raise EvidenceError("BMS1 rollback index no longer matches BMF1 generation")
    signature_value = int.from_bytes(artifact[256:], "big")
    modulus = int.from_bytes(RSA_MODULUS, "big")
    encoded = pow(signature_value, RSA_EXPONENT, modulus).to_bytes(256, "big")
    digest = hashlib.sha256(signed).digest()
    padding = bytes((0, 1)) + bytes((0xFF,)) * 202 + bytes((0,))
    expected = padding + SHA256_DIGEST_INFO_PREFIX + digest
    return {
        "generation": generation,
        "rollback_index": rollback_index,
        "signed_sha256": digest.hex(),
        "signature_valid": encoded == expected,
        "payload": payload,
        "signed": signed,
    }


def verify_artifacts(product: Path, rollback: Path, bad_signature: Path) -> str:
    if hashlib.sha256(RSA_MODULUS).hexdigest() != TRUST_ANCHOR_SHA256:
        raise EvidenceError("M74 parser trust anchor changed")
    current = inspect_artifact(decode_hex_artifact(product))
    old = inspect_artifact(decode_hex_artifact(rollback))
    bad = inspect_artifact(decode_hex_artifact(bad_signature))
    if (
        current["generation"] != 2
        or current["rollback_index"] != 2
        or current["signed_sha256"] != SIGNED_SHA256
        or current["signature_valid"] is not True
    ):
        raise EvidenceError("M74 product artifact verification changed")
    if (
        old["generation"] != 1
        or old["rollback_index"] != 1
        or old["signature_valid"] is not True
    ):
        raise EvidenceError("M74 signed rollback fixture changed")
    if (
        bad["generation"] != 2
        or bad["rollback_index"] != 2
        or bad["signed"] != current["signed"]
        or bad["signature_valid"] is not False
    ):
        raise EvidenceError("M74 bad-signature fixture changed")
    return (
        "VERIFIED_MANIFEST_ARTIFACTS_OK format=1 product_signature_valid=1 "
        "product_generation=2 rollback_index=2 rollback_floor=2 "
        "old_signature_valid=1 old_generation=1 old_rejected_by_floor=1 "
        "bad_signature_valid=0 signed_region_mutations=0 algorithm=RSA2048-"
        "PKCS1-v1_5-SHA256 private_key_in_repository=0 production_key_claim=0 "
        "hardware_rollback_claim=0"
    )


def synthetic_positive(phase: str) -> str:
    base = m73.synthetic_log(phase).replace("abi=34", "abi=35")
    base = base.replace(m73.UI_BOOT_MARKER, UI_BOOT_MARKER, 1)
    base = base.replace(m73.SHUTDOWN_BOOT_MARKER, SHUTDOWN_BOOT_MARKER, 1)
    base = base.replace(OLD_AUTHORITY, AUTHORITY, 1)
    base = base.replace("manifest_generation=1", "manifest_generation=2", 1)
    event = next(
        line
        for line in base.splitlines()
        if line.startswith(m73.EVENT_PREFIX + " ")
    )
    return base.replace(event + "\n", event + "\n" + verified_marker() + "\n", 1)


def synthetic_negative(reason: str) -> tuple[str, str]:
    rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
    log = "\n".join(
        (
            UI_MARKER,
            UI_BOOT_MARKER,
            rejection,
            "M66_PLATFORM_SHUTDOWN_DIAG failure=29185 fault=0 reason=0 "
            "shutdown=Open/0/0/0/0 services=0/0/0/0/0x0/0x0 "
            "process=10/1/1/9 capacity=9/0 heap_free=1 "
            "broker=0/0/0/0 io=0/0/0/0/0",
            "",
        )
    )
    return log, NEGATIVE_DRIVER.format(reason=reason) + "\n"


def parser_self_test() -> None:
    for phase in PHASES:
        validate_positive(synthetic_positive(phase), phase)
        validate_driver(m73.driver_marker(phase) + "\n", phase)
    for reason in ("signature", "rollback"):
        log, driver = synthetic_negative(reason)
        validate_negative(log, driver, reason)

    base = synthetic_positive("first")
    mutations = (
        base.replace("abi=35", "abi=34", 1),
        base.replace("artifact=BMS1", "artifact=BMS0", 1),
        base.replace("manifest_generation=2", "manifest_generation=1", 1),
        base.replace("rollback_index=2", "rollback_index=1", 1),
        base.replace("rollback_floor=2", "rollback_floor=1", 1),
        base.replace("signature_successes=1", "signature_successes=0", 1),
        base.replace("signature_rejections=0", "signature_rejections=1", 1),
        base.replace("signed_sha256=" + SIGNED_SHA256, "signed_sha256=0", 1),
        base.replace(
            "trust_anchor_sha256=" + TRUST_ANCHOR_SHA256,
            "trust_anchor_sha256=0",
            1,
        ),
        base.replace("manifest_published_after_verification=1", "manifest_published_after_verification=0", 1),
        base.replace("production_key_claim=0", "production_key_claim=1", 1),
        base.replace("hardware_rollback_claim=0", "hardware_rollback_claim=1", 1),
        base.replace(verified_marker() + "\n", "", 1),
        base + verified_marker() + "\n",
        base.replace(
            m73.EVENT_PREFIX,
            verified_marker() + "\n" + m73.EVENT_PREFIX,
            1,
        ),
    )
    rejected = 0
    for mutation in mutations:
        try:
            validate_positive(mutation, "first")
        except ValueError:
            rejected += 1
    if rejected != len(mutations):
        raise EvidenceError("M74 positive parser accepted a mutation")

    negative_rejected = 0
    for reason in ("signature", "rollback"):
        log, driver = synthetic_negative(reason)
        rejection = SIGNATURE_REJECTION if reason == "signature" else ROLLBACK_REJECTION
        for mutated_log, mutated_driver in (
            (log.replace(rejection + "\n", "", 1), driver),
            (
                log.replace("manifest_published=0", "manifest_published=1", 1),
                driver,
            ),
            (
                log + "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK format=1\n",
                driver,
            ),
            (
                log,
                driver.replace("qemu_terminated_by_host=1", "qemu_terminated_by_host=0"),
            ),
        ):
            try:
                validate_negative(mutated_log, mutated_driver, reason)
            except ValueError:
                negative_rejected += 1
    if negative_rejected != 8:
        raise EvidenceError("M74 negative parser accepted a mutation")
    print(
        "UNIFIED_PRODUCT_VERIFIED_MANIFEST_EVIDENCE_PARSER_SELF_TEST_OK "
        f"positive={len(PHASES) * 2 + 2} serial_negative={rejected} "
        f"fail_closed_negative={negative_rejected}"
    )


def verify_reboot(
    pristine_path: Path,
    runtime_path: Path,
    first_serial_path: Path,
    second_serial_path: Path,
    first_driver_path: Path,
    second_driver_path: Path,
    signature_serial_path: Path,
    rollback_serial_path: Path,
    signature_driver_path: Path,
    rollback_driver_path: Path,
) -> str:
    first = validate_positive(
        first_serial_path.read_text(encoding="utf-8", errors="replace"),
        "first",
    )
    second = validate_positive(
        second_serial_path.read_text(encoding="utf-8", errors="replace"),
        "second",
    )
    validate_driver(
        first_driver_path.read_text(encoding="utf-8", errors="replace"), "first"
    )
    validate_driver(
        second_driver_path.read_text(encoding="utf-8", errors="replace"), "second"
    )
    validate_negative(
        signature_serial_path.read_text(encoding="utf-8", errors="replace"),
        signature_driver_path.read_text(encoding="utf-8", errors="replace"),
        "signature",
    )
    validate_negative(
        rollback_serial_path.read_text(encoding="utf-8", errors="replace"),
        rollback_driver_path.read_text(encoding="utf-8", errors="replace"),
        "rollback",
    )
    marker = m73.m72.m71.m70.m69.verify_disk(pristine_path, runtime_path)
    prefix = "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK "
    if not marker.startswith(prefix):
        raise EvidenceError("shared M69 disk verifier returned an unexpected marker")
    marker = marker.replace(
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
        "UNIFIED_PRODUCT_VERIFIED_MANIFEST_REBOOT_OK",
        1,
    )
    historical = (
        "product_shutdowns=2 multiservice_liveness_recoveries=2 "
        "supervised_services=4 liveness_dependency_edges=2 health_probes=16 "
        "healthy=14 withheld=2 health_timeouts=2 cadence_waits=6 "
        "dependency_fault_transitions=4 dependency_recovery_transitions=4 "
        "dependent_blocks=2 dependent_resumes=2 killed_servers=2 "
        "replacements=2 replacement_healthy=2 storage_epochs=4 "
    )
    verified = (
        "product_shutdowns=2 verified_manifest_sessions=2 artifact_verifications=2 "
        "signature_successes=2 signature_rejections=0 rollback_rejections=0 "
        "manifest_generation=2 rollback_index=2 rollback_floor=2 "
        "signed_sha256_consistent=1 trust_anchor_consistent=1 "
        "negative_signature_boots=1 negative_rollback_boots=1 "
        "fail_closed_boots=2 manifests_published_after_rejection=0 "
        "event_supervision_sessions=2 manifests_decoded=2 "
        "manifest_open_calls=4 manifest_open_successes=2 "
        "manifest_argument_rejections=2 manifest_permission_denials=0 "
        "supervised_services=10 liveness_dependency_edges=8 "
        f"ui_query_calls={first['ui_query_calls'] + second['ui_query_calls']} "
        f"ui_query_waits={first['ui_query_waits'] + second['ui_query_waits']} "
        "ui_query_successes=2 "
        f"event_batches={first['active_batches'] + second['active_batches']} "
        "report_calls=18 report_successes=16 report_argument_rejections=2 "
        "report_permission_denials=0 report_state_rejections=0 "
        "clean_rotations=4 process_exit_faults=4 process_terminate_calls=0 "
        "terminated_exited=24 terminated_killed=0 backoff_waits=4 "
        "restart_budget_rearms=4 dependent_blocks=4 dependent_resumes=4 "
        "cancel_windows=2 cancelled_pending=8 drained_pending=0 "
        "storage_epochs=6 psci_discoveries=2 psci_version_probes=2 "
        "psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 "
        "psci_system_off_requests=2 qemu_psci_self_exits=2 "
        "semihosting_uses=0 "
    )
    if historical not in marker:
        raise EvidenceError("shared M69 aggregate fields changed")
    marker = marker.replace(historical, verified, 1)
    return marker.replace(
        "hardware_poweroff_claim=0 psci_claim=0",
        "hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1 "
        "production_key_claim=0 hardware_rollback_claim=0",
        1,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    parse = subparsers.add_parser("parse")
    parse.add_argument("phase", choices=tuple(PHASES))
    parse.add_argument("log", type=Path)
    negative = subparsers.add_parser("parse-negative")
    negative.add_argument("reason", choices=("signature", "rollback"))
    negative.add_argument("log", type=Path)
    negative.add_argument("driver", type=Path)
    artifacts = subparsers.add_parser("artifacts")
    artifacts.add_argument("product", type=Path)
    artifacts.add_argument("rollback", type=Path)
    artifacts.add_argument("bad_signature", type=Path)
    reboot = subparsers.add_parser("reboot")
    for name in (
        "pristine",
        "runtime",
        "first_serial",
        "second_serial",
        "first_driver",
        "second_driver",
        "signature_serial",
        "rollback_serial",
        "signature_driver",
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
                "UNIFIED_PRODUCT_VERIFIED_MANIFEST_EVIDENCE_OK "
                f"phase={args.phase}"
            )
        elif args.command == "parse-negative":
            validate_negative(
                args.log.read_text(encoding="utf-8", errors="replace"),
                args.driver.read_text(encoding="utf-8", errors="replace"),
                args.reason,
            )
            print(
                "UNIFIED_PRODUCT_VERIFIED_MANIFEST_NEGATIVE_EVIDENCE_OK "
                f"reason={args.reason}"
            )
        elif args.command == "artifacts":
            print(
                verify_artifacts(
                    args.product, args.rollback, args.bad_signature
                )
            )
        elif args.command == "reboot":
            print(
                verify_reboot(
                    args.pristine,
                    args.runtime,
                    args.first_serial,
                    args.second_serial,
                    args.first_driver,
                    args.second_driver,
                    args.signature_serial,
                    args.rollback_serial,
                    args.signature_driver,
                    args.rollback_driver,
                )
            )
        else:
            raise AssertionError("unreachable command")
    except (ValueError, OSError) as error:
        print(
            f"unified_product_verified_manifest_evidence.py: {error}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
