#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 openssl cargo rustc cmp mkdir; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M76 key-rotation contracts." >&2
    exit 1
  fi
done

for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import ast
import hashlib
import re
import sys
import tomllib
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M76 contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(
    relative: str, needle: str, label: str, count: int | None = None
) -> None:
    observed = read(relative).count(needle)
    if observed == 0:
        raise SystemExit(f"{relative}: missing {label}: {needle!r}")
    if count is not None and observed != count:
        raise SystemExit(
            f"{relative}: {label} count is {observed}, expected {count}: {needle!r}"
        )


def features(relative: str) -> dict[str, list[str]]:
    with (root / relative).open("rb") as stream:
        return tomllib.load(stream)["features"]


feature = "unified-product-key-rotation-runtime"
parent = "unified-product-persistent-rollback-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M76 feature must extend exactly the M75 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M76 feature closure changed")

abi = "crates/bndr-abi/src/lib.rs"
for needle in (
    'feature = "unified-product-key-rotation-runtime",',
    'not(feature = "unified-product-maintenance-authorization-runtime")',
    "pub const ABI_VERSION: u64 = 37;",
    "assert_eq!(ABI_VERSION, 37);",
    "assert_eq!(SyscallNumber::from_raw(57), None);",
    "ServiceManifestOpen = 55,",
    "ServiceSupervisorReport = 56,",
):
    require(abi, needle, "ABI v37 no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 37] = [(); ABI_VERSION as usize];",
    "userspace ABI v37 compile-time assertion",
    count=1,
)

build_script = "scripts/build-kernel.sh"
for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-key-rotation-runtime"',
    "KERNEL_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-key-rotation-runtime"',
    "USER_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-key-rotation-runtime"',
    "userspace key-rotation profile requires the matching kernel profile.",
):
    require(build_script, needle, "matched M76 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME",
    'feature=\\"unified-product-key-rotation-runtime\\"',
    "product-service-manifest-v5-key4.bms1.hex",
    "BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX",
):
    require("kernel/build.rs", needle, "ABI37 and generation-five build embedding")

manifest = "crates/bndr-sm/src/manifest.rs"
for needle in (
    "pub const KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION: u32 = 4;",
    "pub const KEY_ROTATION_SERVICE_MANIFEST_GENERATION: u32 = 5;",
    "pub const KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION: u32 = 6;",
    "KEY_ROTATION_TRANSITION_PRODUCT_SERVICE_MANIFEST_FINGERPRINT",
    "KEY_ROTATION_PRODUCT_SERVICE_MANIFEST_FINGERPRINT",
    "KEY_ROTATION_RETIRED_KEY_FIXTURE_MANIFEST_FINGERPRINT",
):
    require(manifest, needle, "three exact M76 BMF1 generations")

verifier = "crates/bndr-sm/src/verified_manifest.rs"
for needle in (
    "pub const KEY_ROTATION_TRANSITION_KEY_ID: u8 = 3;",
    "pub const KEY_ROTATION_ACTIVE_KEY_ID: u8 = 4;",
    "pub const KEY_ROTATION_TRANSITION_KEY_EPOCH: u32 = 3;",
    "pub const KEY_ROTATION_ACTIVE_KEY_EPOCH: u32 = 4;",
    "pub const KEY_ROTATION_POLICY_SHA256: [u8; 32]",
    "pub const KEY_ROTATION_MANIFEST_KEYRING: [Rsa2048PublicKey; 3]",
    "pub fn verify_signed_service_manifest_with_keyring(",
    "pub fn rsa2048_keyring_sha256(",
    "DuplicateTrustAnchor(u8)",
    "UnknownKeyringKey { actual: u8 }",
    "rotation_keyring_selects_both_successor_keys_and_has_a_stable_policy_digest",
    "rotation_keyring_rejects_bad_signatures_unknown_ids_and_invalid_policy_sets",
    "0x30, 0x67, 0x6f, 0x35",
):
    require(verifier, needle, "bounded ordered RSA keyring")

persist = "kernel/src/persist.rs"
for needle in (
    'pub const MANIFEST_KEY_POLICY_STATE_MAGIC: [u8; 8] = *b"BNDRKEY1";',
    "pub const MANIFEST_KEY_POLICY_STATE_VERSION: u32 = 1;",
    "pub const MANIFEST_KEY_POLICY_STATE_BYTES: usize = 96;",
    "pub struct ManifestKeyPolicyBinding",
    "pub struct ManifestKeyPolicyState",
    "pub enum ManifestKeyRotationError",
    "pub struct ManifestKeyRotationEvidence",
    "pub fn enforce_and_rotate_manifest_key_policy",
    "ExistingManifestKeyState::Legacy",
    "ExistingManifestKeyState::Policy",
    "ManifestKeyRotationError::RetiredKey",
    "ManifestKeyRotationError::SkippedKeyEpoch",
    "ManifestKeyRotationError::TransitionWithoutFloorAdvance",
    "manifest_policy_slot_count",
    "io.write_sector(committed_lba, &committed_sector)",
    "io.flush()",
    "let mut verified_slots",
    "verified_slots[preserved_slot] != initial_slots[preserved_slot]",
    "manifest_key_policy_migrates_two_epochs_repairs_then_becomes_read_only",
    "retired_or_skipped_manifest_key_epochs_fail_without_mutation",
):
    require(persist, needle, "persistent staged key-policy transaction")
transaction = read(persist).split(
    "pub fn enforce_and_rotate_manifest_key_policy", 1
)[1].split("pub fn advance_boot_state", 1)[0]
if not (
    transaction.index("if artifact_binding.key_epoch < previous_key_epoch")
    < transaction.index("let effective_floor")
    < transaction.index("io.write_sector(committed_lba, &committed_sector)")
    < transaction.index("io.flush()")
    < transaction.index("let mut verified_slots")
    < transaction.index("verified_slots[preserved_slot] != initial_slots[preserved_slot]")
):
    raise SystemExit("M76 transaction lost epoch/floor/write/flush/readback order")

adapter = "kernel/src/storage_persist.rs"
for needle in (
    "ManifestKeyRotationTransaction(ManifestKeyRotationError)",
    "pub fn enforce_verified_manifest_key_rotation(",
    "partition_first_lba != DATA_PARTITION_FIRST_LBA",
    "storage::durability_contract()",
    "enforce_and_rotate_manifest_key_policy(",
):
    require(adapter, needle, "IRQ-backed M76 persistence adapter")

syscall = "kernel/src/syscall.rs"
for needle in (
    "struct KeyRotationManifestPreparation",
    "KEY_ROTATION_MANIFEST_PREPARATION",
    "pub fn prepare_key_rotation_verified_manifest(",
    "verify_signed_service_manifest_with_keyring(",
    "rsa2048_keyring_sha256(&KEY_ROTATION_MANIFEST_KEYRING)",
    "enforce_verified_manifest_key_rotation(",
    "KEY_ROTATION_REJECTED format=1 reason=signature",
    "KEY_ROTATION_REJECTED format=1 reason=retired-key",
    "KEY_ROTATION_REJECTED format=1 reason=skipped-key-epoch",
    "KEY_ROTATION_POLICY_OK format=1 state=BNDRKEY1",
    "KEY_ROTATION_MANIFEST_PREPARATION.publish(evidence)",
    "key-rotation BMS1 requested before durable preparation",
    "Vmo::try_from_slice(manifest_bytes)",
):
    require(syscall, needle, "verify-policy-persist-before-publish path")
prepare = read(syscall).split(
    "pub fn prepare_key_rotation_verified_manifest(", 1
)[1].split("pub fn key_rotation_manifest_snapshot", 1)[0]
if not (
    prepare.index("verify_signed_service_manifest_with_keyring(")
    < prepare.index("enforce_verified_manifest_key_rotation(")
    < prepare.index("KEY_ROTATION_MANIFEST_PREPARATION.publish(evidence)")
):
    raise SystemExit("M76 preparation lost signature/policy/publication order")
open_body = read(syscall).split("fn service_manifest_open(", 1)[1].split(
    "fn store_verified_manifest_digest", 1
)[0]
if not (
    open_body.index("KEY_ROTATION_MANIFEST_PREPARATION")
    < open_body.index("Vmo::try_from_slice(manifest_bytes)")
):
    raise SystemExit("M76 manifest VMO can precede durable key-policy preparation")
if "enable_irq()" in open_body:
    raise SystemExit("M76 SVC path illegally enables IRQs for synchronous block I/O")

main = "kernel/src/main.rs"
for needle in (
    "syscall::prepare_key_rotation_verified_manifest(interrupt_info.timer_frequency_hz)",
    "UNIFIED_PRODUCT_KEY_ROTATION_OK format=1 abi={}",
    "keyring_keys=3 key_id={} key_epoch={}",
    "manifest_published_after_policy_commit=1",
    "offline_split_signing_supported=1",
    "private_key_in_repository=0 production_key_claim=0 hsm_claim=0",
    "host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0",
    "rpmb_claim=0 efuse_claim=0",
    "BOOT_OK: M76 unified real UI is interactive; persistent key-policy event supervision is starting",
    "BOOT_OK: M76 persistent key rotation and revocation, offline-signing artifact, event supervision, AppData, and PSCI shutdown armed",
    "real_phone_claim=0",
):
    require(main, needle, "sealed M76 kernel evidence")
main_source = read(main)
if not (
    main_source.index("process::init();")
    < main_source.index(
        "syscall::prepare_key_rotation_verified_manifest(interrupt_info.timer_frequency_hz)"
    )
    < main_source.index("let user_info = userboot::start()")
):
    raise SystemExit("M76 key-policy preparation is not between process reset and EL0 start")

product = "user/init/src/product_runtime.rs"
for needle in (
    "KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION",
    "KEY_ROTATION_SERVICE_MANIFEST_GENERATION",
    "KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION",
    "let generation_matches = matches!(",
):
    require(product, needle, "init M76 transactional generation validation")

offline = "scripts/offline_service_manifest_signing.py"
for needle in (
    'subparsers.add_parser("prepare")',
    'subparsers.add_parser("assemble")',
    '"--expected-modulus-sha256"',
    '"--public-key"',
    "verify_signature(region, signature, args.public_key, args.openssl)",
    "BMS1_OFFLINE_REQUEST_OK",
    "BMS1_OFFLINE_ASSEMBLY_OK",
    "contains_private_key=0",
    "private_key_consumed=0",
):
    require(offline, needle, "split offline-signing boundary")
offline_source = read(offline)
for forbidden in ('"--private-key"', '"-sign"', "genpkey"):
    if forbidden in offline_source:
        raise SystemExit(f"M76 offline assembly gained a signing primitive: {forbidden}")

evidence = "scripts/unified_product_key_rotation_evidence.py"
for needle in (
    "UNIFIED_PRODUCT_KEY_ROTATION_EVIDENCE_PARSER_SELF_TEST_OK",
    "KEY_ROTATION_ARTIFACTS_OK",
    "UNIFIED_PRODUCT_KEY_ROTATION_REBOOT_OK",
    "decode_policy_record",
    "retired_rejected_by_persistent_epoch=1",
    "rejected_boot_slot_mutations=0",
    "steady_read_only=1",
    "host_rollback_resistance=0",
):
    require(evidence, needle, "strict M76 artifact/log/disk parser")

negative = "scripts/key_rotation_negative_qemu.py"
for needle in (
    '"-nic",',
    '"none",',
    "KEY_ROTATION_REJECTED format=1 reason={reason}",
    "boot error: manifest key-rotation preparation failed:",
    '"USER_MAP_OK "',
    "policy_slots_mutated=0",
    "pre_el0=1",
):
    require(negative, needle, "pre-EL0 M76 negative QEMU driver")

qmp = "scripts/unified_product_qmp.py"
for needle in (
    '"M76": (',
    '"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"',
    'if args.milestone in {"M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}:',
    '"KEY_ROTATION_POLICY_OK "',
    '"UNIFIED_PRODUCT_KEY_ROTATION_OK "',
    '"transition"',
    '"activate"',
    '"repair"',
):
    require(qmp, needle, "M76 positive QMP maintenance path")

runtime = "scripts/check-unified-product-key-rotation-runtime.sh"
for needle in (
    "unified_product_key_rotation_evidence.py\" self-test",
    "offline_service_manifest_signing.py\" prepare",
    "offline_service_manifest_signing.py\" assemble",
    "run_positive first M75",
    "run_positive transition M76",
    "run_positive activate M76",
    "run_positive repair M76",
    "run_positive steady M76",
    "run_negative signature",
    "run_negative retired-key",
    "cp \"$RUNTIME_IMAGE\" \"$SIGNATURE_IMAGE\"",
    "cp \"$RUNTIME_IMAGE\" \"$RETIRED_IMAGE\"",
):
    require(runtime, needle, "five-positive/two-negative M76 runtime gate")

artifacts = {
    "boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex": 512,
    "boot/product-service-manifest-v5-key4.bms1.hex": 512,
    "boot/test-fixtures/product-service-manifest-v6-retired-key3.bms1.hex": 512,
    "boot/test-fixtures/product-service-manifest-v5-key4-bad-signature.bms1.hex": 512,
    "boot/offline/product-service-manifest-v5-key4.request.hex": 256,
    "boot/offline/product-service-manifest-v5-key4.signature.hex": 256,
}
decoded = {}
for relative, expected_bytes in artifacts.items():
    digits = "".join(read(relative).split())
    if (
        len(digits) != expected_bytes * 2
        or digits != digits.lower()
        or re.fullmatch(r"[0-9a-f]+", digits) is None
    ):
        raise SystemExit(f"{relative}: expected canonical {expected_bytes}-byte hex")
    decoded[relative] = bytes.fromhex(digits)
active = decoded["boot/product-service-manifest-v5-key4.bms1.hex"]
if (
    decoded["boot/offline/product-service-manifest-v5-key4.request.hex"]
    + decoded["boot/offline/product-service-manifest-v5-key4.signature.hex"]
    != active
):
    raise SystemExit("M76 offline request/signature do not reassemble the active artifact")
if hashlib.sha256(active[:256]).hexdigest() != (
    "5a8ccd0ae601dad43356782ed0cc803fc3fe1232d3f42be981a71d94ca33e988"
):
    raise SystemExit("M76 active signed-region digest changed")

for public_key in (
    "boot/trust/fixture-key3-public.pem",
    "boot/trust/fixture-key4-public.pem",
):
    source = read(public_key)
    if (
        source.count("-----BEGIN PUBLIC KEY-----") != 1
        or source.count("-----END PUBLIC KEY-----") != 1
    ):
        raise SystemExit(f"{public_key}: expected one SubjectPublicKeyInfo PEM")

for path in root.rglob("*"):
    if not path.is_file() or "target" in path.relative_to(root).parts:
        continue
    if path.suffix.lower() in {".key", ".p12", ".pfx"}:
        raise SystemExit(f"M76 repository contains a private-key-shaped file: {path}")
    try:
        source = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    private_markers = (
        "-----BEGIN " + "PRIVATE KEY-----",
        "-----BEGIN RSA " + "PRIVATE KEY-----",
        "-----BEGIN EC " + "PRIVATE KEY-----",
    )
    if any(marker in source for marker in private_markers):
        raise SystemExit(f"M76 repository contains private key material: {path}")

for relative in (runtime, evidence, qmp, negative, offline):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M76 evidence path gained network access")

# Every Python QEMU argv must contain exactly one disabled NIC.
for relative in (qmp, negative):
    tree = ast.parse(read(relative))
    launches = [
        node.args[0].elts
        for node in ast.walk(tree)
        if isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and node.func.attr == "Popen"
        and node.args
        and isinstance(node.args[0], ast.List)
    ]
    if len(launches) != 1:
        raise SystemExit(f"{relative}: expected exactly one QEMU Popen argv")
    argv = [
        element.value
        if isinstance(element, ast.Constant) and isinstance(element.value, str)
        else None
        for element in launches[0]
    ]
    nic = [index for index, value in enumerate(argv) if value == "-nic"]
    if len(nic) != 1 or argv[nic[0] + 1] != "none":
        raise SystemExit(f"{relative}: QEMU argv lacks exactly one '-nic none'")
    if any(value in {"-net", "-netdev"} for value in argv):
        raise SystemExit(f"{relative}: QEMU argv added a network backend")

# Audit every literal shell QEMU launch, including historical gates.
launch_pattern = re.compile(r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$")
nic_pattern = re.compile(r"^\s*-nic(?:\s|$)")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
launches = 0
for path in sorted((root / "scripts").glob("*.sh")):
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        if not launch_pattern.match(lines[index]):
            index += 1
            continue
        block = [lines[index]]
        while block[-1].rstrip().endswith("\\"):
            index += 1
            if index >= len(lines):
                raise SystemExit(f"unterminated QEMU launch: {path.relative_to(root)}")
            block.append(lines[index])
        launches += 1
        if sum(bool(nic_pattern.match(line)) for line in block) != 1:
            raise SystemExit(f"QEMU launch has multiple/no NIC options: {path}")
        if sum(bool(nic_none_pattern.match(line)) for line in block) != 1:
            raise SystemExit(f"QEMU launch lacks exact '-nic none': {path}")
        index += 1
if launches == 0:
    raise SystemExit("M76 static contract found no literal shell QEMU launches")

for needle in (
    '"$SCRIPT_DIR/check-unified-product-key-rotation-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-key-rotation-runtime.sh"',
    "unified_product_key_rotation_static=1",
    "unified_product_key_rotation_reboot=1",
    "key_rotation_negative_signature_boots=1",
    "key_rotation_negative_retired_key_boots=1",
    "key_rotation_steady_read_only=1",
    "offline_split_signing=1",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require("scripts/test.sh", needle, "full-suite M76 integration", count=1)

print(
    "UNIFIED_PRODUCT_KEY_ROTATION_SOURCE_OK abi=37 syscalls=0-56 "
    "artifact=BMS1 bytes=512 algorithm=RSA2048-PKCS1-v1_5-SHA256 "
    "keyring_keys=3 key_ids=2/3/4 key_epochs=2/3/4 "
    f"policy_sha256=30676f357683b6c8d2c5d67755e48beb93bdfe8864eccc67c716a1785e935e01 "
    "committed_floor=5 slots=2 positive_boots=5 negative_boots=2 "
    "key_transitions=2 redundancy_repair=1 steady_read_only=1 "
    "retired_key_rejection=1 rejected_slot_mutations=0 offline_split_signing=1 "
    f"qemu_launches={launches} nic_none={launches} network=0 semihosting=0 "
    "fixture_keys=1 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
    "efuse_claim=0 host_rollback_resistance=0 erase_resistance=0 "
    "tamper_resistance=0 hardware_powercut_claim=0 emulator_only=1 "
    "general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/offline_service_manifest_signing.py" \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/key_rotation_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_key_rotation_evidence.py"
python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" self-test
python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" artifacts \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex" \
  "$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v6-retired-key3.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v5-key4-bad-signature.bms1.hex" \
  "$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.request.hex" \
  "$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.signature.hex"

mkdir -p "$WORKSPACE_ROOT/target/m76-offline-static"
python3 "$SCRIPT_DIR/offline_service_manifest_signing.py" prepare \
  --generation 5 \
  --rollback-index 5 \
  --key-id 4 \
  --request "$WORKSPACE_ROOT/target/m76-offline-static/generated.request.hex"
cmp \
  "$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.request.hex" \
  "$WORKSPACE_ROOT/target/m76-offline-static/generated.request.hex"
python3 "$SCRIPT_DIR/offline_service_manifest_signing.py" assemble \
  --request "$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.request.hex" \
  --signature "$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.signature.hex" \
  --public-key "$WORKSPACE_ROOT/boot/trust/fixture-key4-public.pem" \
  --key-id 4 \
  --expected-modulus-sha256 \
    36b7c88dc40ba32c77729d04ac7ceb90a21691d39abe94b17b12b1242f33c52f \
  --output "$WORKSPACE_ROOT/target/m76-offline-static/assembled.bms1.hex"
cmp \
  "$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex" \
  "$WORKSPACE_ROOT/target/m76-offline-static/assembled.bms1.hex"
openssl pkey -pubin -in "$WORKSPACE_ROOT/boot/trust/fixture-key3-public.pem" -noout
openssl pkey -pubin -in "$WORKSPACE_ROOT/boot/trust/fixture-key4-public.pem" -noout
