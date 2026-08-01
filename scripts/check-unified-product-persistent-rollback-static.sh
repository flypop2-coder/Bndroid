#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M75 persistent rollback contracts." >&2
    exit 1
  fi
done

for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import ast
import re
import sys
import tomllib
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M75 contract source is missing: {relative}")
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


feature = "unified-product-persistent-rollback-runtime"
parent = "unified-product-verified-manifest-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M75 feature must extend exactly the M74 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M75 feature closure changed")

abi = "crates/bndr-abi/src/lib.rs"
for needle in (
    'feature = "unified-product-persistent-rollback-runtime"',
    'not(feature = "unified-product-key-rotation-runtime")',
    "pub const ABI_VERSION: u64 = 36;",
    "assert_eq!(ABI_VERSION, 36);",
    "assert_eq!(SyscallNumber::from_raw(57), None);",
    "ServiceManifestOpen = 55,",
    "ServiceSupervisorReport = 56,",
):
    require(abi, needle, "ABI v36 no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 36] = [(); ABI_VERSION as usize];",
    "userspace ABI v36 compile-time assertion",
    count=1,
)

build_script = "scripts/build-kernel.sh"
for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-persistent-rollback-runtime"',
    "KERNEL_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-persistent-rollback-runtime"',
    "USER_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-persistent-rollback-runtime"',
    "userspace persistent-rollback profile requires the matching kernel profile.",
):
    require(build_script, needle, "matched M75 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME",
    'feature=\\"unified-product-persistent-rollback-runtime\\"',
    "product-service-manifest-v3.bms1.hex",
    "BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX",
):
    require("kernel/build.rs", needle, "ABI36 and generation-three build embedding")

manifest = "crates/bndr-sm/src/manifest.rs"
for needle in (
    "pub const PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION: u32 = 3;",
    "PERSISTENT_ROLLBACK_PRODUCT_SERVICE_MANIFEST_BYTES",
    "encode_product_service_manifest(PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION)",
    "PERSISTENT_ROLLBACK_PRODUCT_SERVICE_MANIFEST_FINGERPRINT",
):
    require(manifest, needle, "generation-three signed BMF1 payload")

verifier = "crates/bndr-sm/src/verified_manifest.rs"
for needle in (
    "pub const PERSISTENT_MANIFEST_TRUSTED_KEY_ID: u8 = 2;",
    "pub const PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR: u32 = 2;",
    "pub const PERSISTENT_MANIFEST_RSA2048_KEY2",
    "PERSISTENT_MANIFEST_RSA2048_MODULUS_HEX",
    "persistent_floor_artifacts_share_the_second_fixture_anchor",
    "PERSISTENT_BAD_SIGNATURE",
    "a050397ce65d2a9f46bb65d9220b56c02984b10dcecd39c530282190ca676c37",
):
    require(verifier, needle, "distinct M75 fixture trust anchor")

persist = "kernel/src/persist.rs"
for needle in (
    'pub const MANIFEST_ROLLBACK_STATE_MAGIC: [u8; 8] = *b"BNDRRBK1";',
    "pub const MANIFEST_ROLLBACK_STATE_VERSION: u32 = 1;",
    "pub const MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [3, 4];",
    "pub struct ManifestRollbackBinding",
    "pub struct ManifestRollbackState",
    "pub enum ManifestRollbackError",
    "pub struct ManifestRollbackEvidence",
    "pub fn enforce_and_advance_manifest_rollback",
    "let effective_floor = bootstrap_floor.max(persisted_floor_before);",
    "if artifact_index < effective_floor",
    "io.write_sector(committed_lba, &committed_sector)",
    "io.flush()",
    "let mut verified_slots",
    "verified_slots[preserved_slot] != initial_slots[preserved_slot]",
    "host rolling the whole disk back or erasing it",
    "manifest_floor_advances_repairs_redundancy_then_becomes_read_only",
    "persistent_manifest_floor_rejects_a_valid_old_index_without_mutation",
    "torn_inactive_manifest_record_preserves_and_repairs_the_old_floor",
    "manifest_floor_fails_closed_on_foreign_binding_and_no_valid_record",
    "manifest_floor_mutation_failures_never_destroy_the_selected_slot",
):
    require(persist, needle, "strict double-slot persistent floor")
transaction = read(persist).split(
    "pub fn enforce_and_advance_manifest_rollback", 1
)[1].split("pub fn advance_boot_state", 1)[0]
if not (
    transaction.index("io.write_sector(committed_lba, &committed_sector)")
    < transaction.index("io.flush()")
    < transaction.index("let mut verified_slots")
    < transaction.index("verified_slots[preserved_slot] != initial_slots[preserved_slot]")
):
    raise SystemExit("M75 persistent transaction lost write/flush/readback/preservation order")

storage = "kernel/src/storage.rs"
for needle in (
    "pub const DATA_PARTITION_FORMAT_EPOCH: [u8; 16]",
    "0x34, 0xf1, 0x5c, 0xd2",
    "write_data_sector_irq",
    "WriteOutsideDataPartition",
):
    require(storage, needle, "fixed BNDROID_DATA epoch and bounded write policy")

adapter = "kernel/src/storage_persist.rs"
for needle in (
    "ManifestRollbackTransaction(ManifestRollbackError)",
    "pub fn enforce_verified_manifest_rollback(",
    "partition_first_lba != DATA_PARTITION_FIRST_LBA",
    "storage::durability_contract()",
    "StoragePersistError::MissingFlush",
    "StoragePersistError::ReadOnlyDevice",
    "enforce_and_advance_manifest_rollback(",
):
    require(adapter, needle, "IRQ-backed M75 persistence adapter")

syscall = "kernel/src/syscall.rs"
for needle in (
    "struct PersistentManifestPreparation",
    "PERSISTENT_MANIFEST_PREPARATION",
    "pub fn prepare_persistent_verified_manifest(",
    "PERSISTENT_MANIFEST_RSA2048_KEY2",
    "sha256(PERSISTENT_MANIFEST_RSA2048_KEY2.modulus())",
    "crate::storage_persist::enforce_verified_manifest_rollback(",
    "PERSISTENT_ROLLBACK_REJECTED format=1 reason=signature",
    "PERSISTENT_ROLLBACK_REJECTED format=1 reason=rollback",
    "PERSISTENT_ROLLBACK_LEDGER_OK format=1",
    "PERSISTENT_MANIFEST_PREPARATION.publish(evidence)",
    "persistent BMS1 requested before durable preparation",
    "Vmo::try_from_slice(manifest_bytes)",
):
    require(syscall, needle, "verify-persist-before-publish kernel path")
prepare = read(syscall).split(
    "pub fn prepare_persistent_verified_manifest(", 1
)[1].split(
    "pub fn persistent_manifest_rollback_snapshot", 1
)[0]
if not (
    prepare.index("verify_signed_service_manifest(")
    < prepare.index("enforce_verified_manifest_rollback(")
    < prepare.index("PERSISTENT_MANIFEST_PREPARATION.publish(evidence)")
):
    raise SystemExit("M75 preparation lost signature/ledger/publication order")
open_body = read(syscall).split("fn service_manifest_open(", 1)[1].split(
    "fn store_verified_manifest_digest", 1
)[0]
if not (
    open_body.index("PERSISTENT_MANIFEST_PREPARATION")
    < open_body.index("Vmo::try_from_slice(manifest_bytes)")
):
    raise SystemExit("M75 manifest VMO can precede persistent preparation")
if "enable_irq()" in open_body:
    raise SystemExit("M75 SVC path illegally enables IRQs for synchronous block I/O")

main = "kernel/src/main.rs"
for needle in (
    "syscall::prepare_persistent_verified_manifest(interrupt_info.timer_frequency_hz)",
    "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK format=1 abi={}",
    "manifest_generation=3",
    "key_id={} rollback_index={}",
    "manifest_published_after_persistent_commit=1",
    "kernel_pinned_fixture_key=1",
    "host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0",
    "rpmb_claim=0 efuse_claim=0",
    "BOOT_OK: M75 unified real UI is interactive; persistent-rollback event supervision is starting",
    "BOOT_OK: M75 persistent rollback ledger, verified external manifest, event supervision, AppData, and PSCI shutdown armed",
    "real_phone_claim=0",
):
    require(main, needle, "sealed M75 kernel evidence")
main_source = read(main)
if not (
    main_source.index("process::init();")
    < main_source.index(
        "syscall::prepare_persistent_verified_manifest(interrupt_info.timer_frequency_hz)"
    )
    < main_source.index("let user_info = userboot::start()")
):
    raise SystemExit("M75 persistent preparation is not between process reset and EL0 start")

product = "user/init/src/product_runtime.rs"
for needle in (
    "PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION",
    "let expected_generation = PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION;",
    "let generation_matches = manifest.generation() == expected_generation;",
):
    require(product, needle, "init generation-three transactional decode")

generator = "scripts/generate_verified_service_manifest_artifact.py"
for needle in (
    'parser.add_argument("--key-id", type=int, default=BMS1_DEFAULT_KEY_ID)',
    "if key_id <= 0 or key_id > 0xFF:",
    "header[6] = key_id",
    "--corrupt-signature",
    "signature[-1] ^= 1",
):
    require(generator, needle, "explicit external fixture-key generation")
if "genpkey" in read(generator):
    raise SystemExit("M75 artifact generator creates its own private key")

evidence = "scripts/unified_product_persistent_rollback_evidence.py"
for needle in (
    "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_EVIDENCE_PARSER_SELF_TEST_OK",
    "PERSISTENT_ROLLBACK_ARTIFACTS_OK",
    "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_REBOOT_OK",
    "pow(signature_value, RSA_EXPONENT, modulus)",
    "decode_rollback_record",
    "first_boot_advances=1 second_boot_repairs=1",
    "steady_boot_read_only=1",
    "rejected_boot_slot_mutations=0",
    "host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0",
):
    require(evidence, needle, "strict M75 artifact/log/disk parser")

negative = "scripts/persistent_rollback_negative_qemu.py"
for needle in (
    '"-nic",',
    '"none",',
    "PERSISTENT_ROLLBACK_REJECTED format=1 reason={reason}",
    "boot error: persistent manifest preparation failed:",
    '"USER_MAP_OK "',
    "pre_el0=1",
    "qemu_terminated_by_host=1",
):
    require(negative, needle, "pre-EL0 negative QEMU driver")

qmp = "scripts/unified_product_qmp.py"
for needle in (
    '"M75": (',
    '{"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    'if args.milestone in {"M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}:',
    '"PERSISTENT_ROLLBACK_LEDGER_OK "',
    '"UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_OK "',
    '"steady"',
):
    require(qmp, needle, "M75 positive QMP maintenance path")

runtime = "scripts/check-unified-product-persistent-rollback-runtime.sh"
for needle in (
    "unified_product_persistent_rollback_evidence.py\" self-test",
    "run_boot first",
    "run_boot second",
    "run_boot steady",
    "run_negative signature",
    "run_negative rollback",
    "--milestone M75",
    "cp \"$RUNTIME_IMAGE\" \"$SIGNATURE_IMAGE\"",
    "cp \"$RUNTIME_IMAGE\" \"$ROLLBACK_IMAGE\"",
):
    require(runtime, needle, "three-positive/two-negative M75 runtime gate")

for artifact in (
    "boot/product-service-manifest-v3.bms1.hex",
    "boot/test-fixtures/product-service-manifest-v2-persistent-rollback.bms1.hex",
    "boot/test-fixtures/product-service-manifest-v3-persistent-bad-signature.bms1.hex",
):
    digits = "".join(read(artifact).split())
    if len(digits) != 1024 or re.fullmatch(r"[0-9a-f]+", digits) is None:
        raise SystemExit(f"{artifact}: expected canonical 512-byte lowercase hex")

for path in root.rglob("*"):
    if not path.is_file() or "target" in path.relative_to(root).parts:
        continue
    if path.suffix.lower() in {".key", ".p12", ".pfx"}:
        raise SystemExit(f"M75 repository contains a private-key-shaped file: {path}")
    try:
        source = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    private_markers = (
        "-----BEGIN " + "PRIVATE KEY-----",
        "-----BEGIN RSA " + "PRIVATE KEY-----",
    )
    if any(marker in source for marker in private_markers):
        raise SystemExit(f"M75 repository contains private key material: {path}")

for relative in (runtime, evidence, qmp, negative, generator):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M75 evidence path gained network access")

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
    raise SystemExit("M75 static contract found no literal shell QEMU launches")

for needle in (
    '"$SCRIPT_DIR/check-unified-product-persistent-rollback-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-persistent-rollback-runtime.sh"',
    "unified_product_persistent_rollback_static=1",
    "unified_product_persistent_rollback_reboot=1",
    "persistent_rollback_negative_signature_boots=1",
    "persistent_rollback_negative_rollback_boots=1",
    "persistent_rollback_steady_read_only=1",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require("scripts/test.sh", needle, "full-suite M75 integration", count=1)

print(
    "UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_SOURCE_OK abi=36 syscalls=0-56 "
    "artifact=BMS1 bytes=512 algorithm=RSA2048-PKCS1-v1_5-SHA256 key_id=2 "
    "artifact_index=3 bootstrap_floor=2 committed_floor=3 slots=2 "
    "positive_boots=3 negative_boots=2 first_advance=1 second_repair=1 "
    "steady_read_only=1 rejected_slot_mutations=0 manifest_published_after_rejection=0 "
    f"qemu_launches={launches} nic_none={launches} network=0 semihosting=0 "
    "fixture_key=1 production_key_claim=0 rpmb_claim=0 efuse_claim=0 "
    "host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0 "
    "hardware_powercut_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/generate_verified_service_manifest_artifact.py" \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/persistent_rollback_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py"
python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" self-test
python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" artifacts \
  "$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v2-persistent-rollback.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v3-persistent-bad-signature.bms1.hex"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm verified_manifest::tests::
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-persistent-rollback-runtime \
  persist::tests::manifest_ --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-persistent-rollback-runtime \
  persist::tests::persistent_manifest_floor_rejects_a_valid_old_index_without_mutation --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-persistent-rollback-runtime \
  persist::tests::torn_inactive_manifest_record_preserves_and_repairs_the_old_floor --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-persistent-rollback-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-persistent-rollback-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-persistent-rollback-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_STATIC_OK source=1 feature_closure=1 abi=36 syscalls=0-56 artifact=BMS1 bytes=512 algorithm=RSA2048-PKCS1-v1_5-SHA256 key_id=2 artifact_index=3 bootstrap_floor=2 committed_floor=3 slots=2 positive_boots=3 negative_boots=2 first_advance=1 second_repair=1 steady_read_only=1 rejected_slot_mutations=0 manifest_published_after_rejection=0 parser_serial_negative=9 parser_fail_closed_negative=6 semihosting=0 network=0 fixture_key=1 production_key_claim=0 rpmb_claim=0 efuse_claim=0 host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0 hardware_powercut_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0'
