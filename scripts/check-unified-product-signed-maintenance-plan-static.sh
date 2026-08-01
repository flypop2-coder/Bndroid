#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc openssl cmp mktemp rm grep; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M81 signed maintenance-plan contracts." >&2
    exit 1
  fi
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import hashlib
import re
import struct
import sys
import tomllib
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M81 contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def read_hex(relative: str, expected_bytes: int) -> bytes:
    text = read(relative)
    digits = "".join(text.split())
    if (
        len(digits) != expected_bytes * 2
        or digits != digits.lower()
        or re.fullmatch(r"[0-9a-f]+", digits) is None
    ):
        raise SystemExit(f"M81 artifact is not canonical lowercase hex: {relative}")
    return bytes.fromhex(digits)


def require(
    text: str,
    needle: str,
    label: str,
    *,
    count: int | None = None,
) -> None:
    actual = text.count(needle)
    if actual == 0:
        raise SystemExit(f"{label} is missing: {needle}")
    if count is not None and actual != count:
        raise SystemExit(
            f"{label} count changed for {needle}: {actual}, expected {count}"
        )


def features(relative: str) -> dict[str, list[str]]:
    return tomllib.loads(read(relative))["features"]


feature = "unified-product-signed-maintenance-plan-runtime"
parent = "unified-product-maintenance-plan-runtime"
abi_features = features("crates/bndr-abi/Cargo.toml")
kernel_features = features("kernel/Cargo.toml")
user_features = features("user/init/Cargo.toml")
if abi_features.get(feature) != [parent]:
    raise SystemExit("M81 ABI feature must extend exactly M80")
forwarding = [parent, f"bndr-abi/{feature}"]
if kernel_features.get(feature) != forwarding:
    raise SystemExit("M81 kernel feature closure changed")
if user_features.get(feature) != forwarding:
    raise SystemExit("M81 userspace feature closure changed")

abi = read("crates/bndr-abi/src/lib.rs")
for needle in (
    '#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]\n'
    "pub const ABI_VERSION: u64 = 42;",
    'not(feature = "unified-product-signed-maintenance-plan-runtime")',
    "MaintenanceSessionOpen = 57",
    "57 => Some(Self::MaintenanceSessionOpen)",
    "assert_eq!(ABI_VERSION, 42);",
    "assert_eq!(SyscallNumber::from_raw(58), None);",
):
    require(abi, needle, "M81 ABI 42 with unchanged syscall 0-57")
if "SignedMaintenancePlan" in abi or "MaintenancePlan" in abi:
    raise SystemExit("M81 unexpectedly exposed a maintenance-plan syscall")

user_main = read("user/init/src/main.rs")
require(
    user_main,
    '#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]\n'
    "const _: [(); 42] = [(); ABI_VERSION as usize];",
    "M81 userspace ABI pin",
    count=1,
)

plan = read("crates/bndr-sm/src/maintenance_plan.rs")
for needle in (
    'pub const MAINTENANCE_PLAN_MAGIC: [u8; 4] = *b"BMP1";',
    "pub const MAINTENANCE_PLAN_SIGNED_SIZE: usize =",
    "pub const MAINTENANCE_PLAN_SIGNATURE_SIZE: usize = 256;",
    "pub const MAINTENANCE_PLAN_OPERATION_COUNT: u32 = 3;",
    "pub const MAINTENANCE_PLAN_TRANSITION_COUNT: u32 = 9;",
    "pub const MAINTENANCE_PLAN_DESCRIPTOR_CAPACITY: usize = 4;",
    "pub struct MaintenancePlanBinding",
    "pub struct MaintenancePlanOperation",
    "pub struct VerifiedMaintenancePlan",
    "pub fn verify_maintenance_plan(",
    "rsa2048_pkcs1_v15_sha256_verify(",
    "MaintenancePlanError::Signature",
    "MaintenancePlanError::Binding",
    "MaintenancePlanError::InvalidProgram",
    "fixture_plan_has_real_signature_and_bounded_program",
    "signature_binding_and_program_fail_closed_independently",
):
    require(plan, needle, "M81 allocation-free signed descriptor parser")
if not (
    plan.index("rsa2048_pkcs1_v15_sha256_verify(")
    < plan.index("let authorization_sequence = read_u64(")
    < plan.index("let operations = [")
):
    raise SystemExit("M81 no longer verifies the signature before signed semantics")

request = read_hex(
    "boot/maintenance/maintenance-plan-sequence2.request.hex", 256
)
signature = read_hex(
    "boot/maintenance/maintenance-plan-sequence2.signature.hex", 256
)
artifact = read_hex("boot/maintenance-plan-sequence2.bmp1.hex", 512)
bad_signature = read_hex(
    "boot/test-fixtures/maintenance-plan-sequence2-bad-signature.bmp1.hex", 512
)
wrong_binding = read_hex(
    "boot/test-fixtures/maintenance-plan-sequence2-wrong-binding.bmp1.hex", 512
)
invalid_program = read_hex(
    "boot/test-fixtures/maintenance-plan-sequence2-invalid-program.bmp1.hex", 512
)
if artifact != request + signature:
    raise SystemExit("M81 BMP1 artifact is not request plus detached signature")
if (
    request[:4] != b"BMP1"
    or request[4:8] != bytes((1, 1, 1, 0))
    or struct.unpack_from("<IIII", request, 8) != (512, 256, 64, 192)
    or struct.unpack_from("<QIIIIQ", request, 24) != (2, 3, 9, 1, 16, 2)
    or request[56:64] != bytes(8)
    or request[240:256] != bytes(16)
):
    raise SystemExit("M81 BMP1 fixed envelope or bounded program changed")
if hashlib.sha256(request).hexdigest() != (
    "9cb363be1d17d9ac88f7e51e59a95171666a15975c3549f1b574f65cc859d57d"
):
    raise SystemExit("M81 signed-region digest changed")
if hashlib.sha256(request[192:256]).hexdigest() != (
    "a3a2298b7dc38de92f5fd753aff767334d567dada2af1b5bbc96988685508602"
):
    raise SystemExit("M81 descriptor-table digest changed")
if request[160:192].hex() != (
    "8e5d4822f651f504ca58825bdac408ccf86bb5b2d52ac58ca9a44433796fbfca"
):
    raise SystemExit("M81 no longer binds the selected M80 plan identity")
if (
    bad_signature[:256] != request
    or bad_signature[256:] == signature
    or wrong_binding[:256] == request
    or invalid_program[:256] == request
):
    raise SystemExit("M81 independent fail-closed fixtures changed shape")

source_roots = ("boot", "crates", "kernel", "scripts", "user")
private_pem_markers = (
    b"-----BEGIN " + b"PRIVATE KEY-----",
    b"-----BEGIN RSA " + b"PRIVATE KEY-----",
)
for source_root in source_roots:
    for path in (root / source_root).rglob("*"):
        if path.is_file():
            payload = path.read_bytes().lstrip()
            if payload.startswith(private_pem_markers):
                raise SystemExit(f"M81 source tree contains a private key: {path}")

build = read("scripts/build-kernel.sh")
for needle in (
    "KERNEL_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME=0",
    "USER_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" '
    '"unified-product-signed-maintenance-plan-runtime"',
    'feature_list_contains "$USERSPACE_FEATURES" '
    '"unified-product-signed-maintenance-plan-runtime"',
    'append_feature "$USERSPACE_FEATURES" '
    '"unified-product-signed-maintenance-plan-runtime"',
    "userspace signed maintenance-plan profile requires the matching kernel profile.",
):
    require(build, needle, "M81 kernel/userspace feature forwarding")

build_rs = read("kernel/build.rs")
for needle in (
    'println!("cargo:rerun-if-env-changed=BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX");',
    'println!("cargo:rerun-if-env-changed=BNDROID_M81_TEST_MODE");',
    "configure_maintenance_plan_artifact(&manifest_dir);",
    'workspace.join("boot/maintenance-plan-sequence2.bmp1.hex")',
    'join("maintenance-plan.bmp1")',
    "BNDR_MAINTENANCE_PLAN_ARTIFACT",
    'feature=\\"unified-product-signed-maintenance-plan-runtime\\"',
):
    require(build_rs, needle, "M81 embedded artifact and userspace cfg propagation")

persist = read("kernel/src/persist.rs")
for needle in (
    'pub const SIGNED_MAINTENANCE_PLAN_STATE_MAGIC: [u8; 8] = *b"BNDRMPB1";',
    "pub const SIGNED_MAINTENANCE_PLAN_STATE_BYTES: usize = 336;",
    "SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS: "
    "[u64; DATA_SLOT_COUNT] = [13, 14];",
    "pub struct SignedMaintenancePlanBinding",
    "pub struct SignedMaintenancePlanState",
    "pub struct SignedMaintenancePlanAdmissionEvidence",
    "pub struct SignedMaintenancePlanEvidence",
    "pub struct SignedMaintenancePlanTerminalEvidence",
    "pub fn signed_maintenance_plan_chain_sha256",
    "pub fn preflight_signed_maintenance_plan",
    "pub fn commit_signed_maintenance_plan",
    "pub fn validate_signed_maintenance_plan_terminal",
    "SignedMaintenancePlanError::ProgramBindingMismatch",
    "signed_maintenance_plan_state_round_trips_and_seals_program_chain",
    "signed_maintenance_plan_commits_once_replays_read_only_and_rejects_substitution",
    "signed_maintenance_plan_terminal_binds_the_confirmed_m80_head",
):
    require(persist, needle, "M81 durable five-ledger program binding")
if persist.count("SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS") < 6:
    raise SystemExit("M81 program slots are not enforced at every transaction boundary")
if not (
    persist.index("let plan = preflight_maintenance_plan_admission(")
    < persist.index("for (slot, relative_lba) in SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS")
):
    raise SystemExit("M81 program preflight no longer checks the predecessor first")

storage = read("kernel/src/storage_persist.rs")
for needle in (
    "SignedMaintenancePlanTransaction(SignedMaintenancePlanError)",
    "pub fn preflight_verified_signed_maintenance_plan",
    "pub fn commit_verified_signed_maintenance_plan",
    "pub fn validate_verified_signed_maintenance_plan_terminal",
    "preflight_signed_maintenance_plan(&mut io, request)",
    "commit_signed_maintenance_plan(&mut io, request)",
    "validate_signed_maintenance_plan_terminal(&mut io, request)",
):
    require(storage, needle, "M81 IRQ-backed kernel-only program adapter")

syscall = read("kernel/src/syscall.rs")
for needle in (
    "struct SignedMaintenancePlanPreparation",
    "verify_maintenance_plan(",
    "preflight_verified_signed_maintenance_plan(",
    "commit_verified_signed_maintenance_plan(",
    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=signature",
    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=binding",
    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=program",
    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=program-ledger",
    "SIGNED_MAINTENANCE_PLAN_TEST_PAUSE",
    "SIGNED_MAINTENANCE_PLAN_OK",
    "program_bound_before_audit_mutation=1",
    "SIGNED_MAINTENANCE_PLAN_PREPARATION.publish(",
):
    require(syscall, needle, "M81 verify-before-mutate pre-EL0 admission")
if not (
    syscall.index("verify_maintenance_plan(")
    < syscall.index("preflight_verified_signed_maintenance_plan(")
    < syscall.index("commit_verified_signed_maintenance_plan(")
    < syscall.index("preflight_verified_maintenance_plan(")
    < syscall.index("admit_verified_maintenance_authorization(")
    < syscall.index("SIGNED_MAINTENANCE_PLAN_PREPARATION.publish(")
):
    raise SystemExit("M81 signature, program binding, audit, and publication order changed")

main = read("kernel/src/main.rs")
for needle in (
    "fn validate_m81_signed_operation(",
    "validate_m81_signed_operation(operation_ordinal, phase);",
    "SIGNED_MAINTENANCE_OPERATION_BOUND",
    "fn validate_m81_signed_maintenance_plan_terminal(",
    "SIGNED_MAINTENANCE_PLAN_TERMINAL_OK",
    "runtime_material[224..256]",
    "runtime_material[256..288]",
    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK",
    "bounded_descriptor_execution=1",
    "arbitrary_program_claim=0",
    "external_effect_exactly_once_claim=0",
    "private_key_in_repository=0",
    "BOOT_OK: M81 unified real UI is interactive",
    "BOOT_OK: M81 signed descriptor-bound maintenance plan",
):
    require(main, needle, "M81 descriptor-driven resident runtime")
require(
    main,
    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK format=1",
    "single M81 final marker",
    count=1,
)

qmp = read("scripts/unified_product_qmp.py")
for needle in (
    '"M81": (',
    '"recover-binding",',
    '"SIGNED_MAINTENANCE_PLAN_OK "',
    '"SIGNED_MAINTENANCE_PLAN_TERMINAL_OK "',
    '"UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_OK "',
    '"SIGNED_MAINTENANCE_OPERATION_BOUND "',
):
    require(qmp, needle, "M81 positive QMP driver")
if qmp.count('"qemu-system-aarch64"') != 1 or qmp.count('"-nic"') != 1:
    raise SystemExit("M81 positive QMP launch must contain exactly one -nic none")

interrupt = read("scripts/signed_maintenance_plan_interrupt_qemu.py")
negative = read("scripts/signed_maintenance_plan_negative_qemu.py")
for text, label, needles in (
    (
        interrupt,
        "M81 deterministic host interruption",
        (
            "SIGNED_MAINTENANCE_PLAN_TEST_PAUSE",
            "program-bound-before-audit",
            "SIGNED_MAINTENANCE_PLAN_INTERRUPT_BOOT_OK",
            "hardware_powercut_claim=0",
            '"-nic",\n            "none",',
        ),
    ),
    (
        negative,
        "M81 pre-EL0 negative boots",
        (
            '"signature", "binding", "program", "program-ledger"',
            "SIGNED_MAINTENANCE_PLAN_REJECTED",
            "SIGNED_MAINTENANCE_PLAN_NEGATIVE_BOOT_OK",
            "program_slots_mutated=0",
            '"-nic",\n            "none",',
        ),
    ),
):
    for needle in needles:
        require(text, needle, label)
    if text.count('"qemu-system-aarch64"') != 1 or text.count('"-nic"') != 1:
        raise SystemExit(f"{label} must contain exactly one -nic none")

evidence = read("scripts/unified_product_signed_maintenance_plan_evidence.py")
runtime = read("scripts/check-unified-product-signed-maintenance-plan-runtime.sh")
offline = read("scripts/offline_maintenance_plan.py")
for needle in (
    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_PARSER_SELF_TEST_OK",
    "decode_program_record",
    "program_chain",
    "runtime_digest",
    "corrupt_selected",
    "assert_empty_program",
    "hardware_powercut_claim",
):
    require(evidence, needle, "strict M81 log and disk parser")
for needle in (
    "run_positive normal M81",
    'run_interrupt "$CUT_KERNEL"',
    "run_positive recover-binding M81",
    "run_negative signature",
    "run_negative binding",
    "run_negative program",
    "run_negative program-ledger",
    "corrupt-selected",
):
    require(runtime, needle, "M81 positive/interrupted/negative runtime gate")
for needle in (
    "prepare",
    "assemble",
    "verify_signature",
    "private_key_consumed=0",
):
    require(offline, needle, "M81 public-only offline split workflow")
private_pem_fragment = "BEGIN " + "PRIVATE KEY"
for forbidden in (
    "--private-key",
    private_pem_fragment,
    "openssl genrsa",
    "openssl pkey",
):
    if forbidden in offline:
        raise SystemExit(f"M81 offline tool gained private-key behavior: {forbidden}")
for forbidden in ("curl ", "wget ", "requests.", "urllib."):
    if forbidden in evidence or forbidden in runtime or forbidden in offline:
        raise SystemExit(f"M81 local-only gate gained network access: {forbidden}")

test_sh = read("scripts/test.sh")
for needle in (
    '"$SCRIPT_DIR/check-unified-product-signed-maintenance-plan-static.sh"',
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndr-abi "
    "--features unified-product-signed-maintenance-plan-runtime --lib",
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndroid-kernel "
    "--features unified-product-signed-maintenance-plan-runtime --lib",
    'BNDROID_PROFILE=release "$SCRIPT_DIR/'
    'check-unified-product-signed-maintenance-plan-runtime.sh"',
    "unified_product_signed_maintenance_plan_static=1",
    "unified_product_signed_maintenance_plan_reboot=1",
    "signed_maintenance_plan_positive_boots=2",
    "signed_maintenance_plan_interrupted_boots=1",
    "signed_maintenance_plan_fail_closed_pre_el0_boots=4",
    "signed_maintenance_plan_descriptor_bound_phases=18",
):
    require(test_sh, needle, "full-suite M81 integration", count=1)

print(
    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_SOURCE_OK "
    "abi=42 syscalls=0-57 artifact=BMP1 artifact_bytes=512 signed_bytes=256 "
    "signature_bytes=256 descriptors=3 descriptor_capacity=4 transitions=9 "
    "program_state=BNDRMPB1 program_bytes=336 program_slots=13/14 "
    "signature_before_semantics=1 predecessor_before_program=1 "
    "program_before_audit=1 exact_replay_read_only=1 substitution_fail_closed=1 "
    "negative_signature=1 negative_binding=1 negative_program=1 "
    "negative_program_ledger=1 offline_split_signing=1 private_key_in_source=0 "
    "network_access=0 arbitrary_program_claim=0 "
    "external_effect_exactly_once_claim=0 trusted_monotonic_backend=0 "
    "production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0 "
    "hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/offline_maintenance_plan.py" \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/signed_maintenance_plan_interrupt_qemu.py" \
  "$SCRIPT_DIR/signed_maintenance_plan_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py"
python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" self-test
bash -n \
  "$SCRIPT_DIR/build-kernel.sh" \
  "$SCRIPT_DIR/check-unified-product-signed-maintenance-plan-runtime.sh"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-m81-static.XXXXXX")"
cleanup() {
  local status="$?"
  trap - EXIT
  rm -rf "$TMP_DIR"
  exit "$status"
}
trap cleanup EXIT

python3 "$SCRIPT_DIR/offline_maintenance_plan.py" prepare \
  --authorization "$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex" \
  --request "$TMP_DIR/request.hex"
cmp "$TMP_DIR/request.hex" \
  "$WORKSPACE_ROOT/boot/maintenance/maintenance-plan-sequence2.request.hex"
python3 "$SCRIPT_DIR/offline_maintenance_plan.py" assemble \
  --authorization "$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex" \
  --request "$TMP_DIR/request.hex" \
  --signature "$WORKSPACE_ROOT/boot/maintenance/maintenance-plan-sequence2.signature.hex" \
  --public-key "$WORKSPACE_ROOT/boot/trust/fixture-maintenance-plan-key1-public.pem" \
  --expected-modulus-sha256 \
    30f139bf41df6593a0db7c80be1b806f56954977c92276765e4156685b10a24e \
  --output "$TMP_DIR/assembled.hex"
cmp "$TMP_DIR/assembled.hex" \
  "$WORKSPACE_ROOT/boot/maintenance-plan-sequence2.bmp1.hex"
if python3 "$SCRIPT_DIR/offline_maintenance_plan.py" assemble \
  --authorization "$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex" \
  --request "$TMP_DIR/request.hex" \
  --signature "$WORKSPACE_ROOT/boot/maintenance/maintenance-plan-sequence2.signature.hex" \
  --public-key "$WORKSPACE_ROOT/boot/trust/fixture-maintenance-plan-key1-public.pem" \
  --expected-modulus-sha256 \
    00f139bf41df6593a0db7c80be1b806f56954977c92276765e4156685b10a24e \
  --output "$TMP_DIR/rejected.hex" >/dev/null 2>&1; then
  echo "M81 offline assembler accepted the wrong pinned modulus." >&2
  exit 1
fi

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
ABI_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features unified-product-signed-maintenance-plan-runtime --lib 2>&1
)"
printf '%s\n' "$ABI_LOG"
if ! grep -Eq \
  'test result: ok\. [0-9]+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' \
  <<<"$ABI_LOG"; then
  echo "M81 ABI host-test ledger changed." >&2
  exit 1
fi

SM_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm \
    maintenance_plan::tests:: --lib 2>&1
)"
printf '%s\n' "$SM_LOG"
if ! grep -Eq \
  'test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$SM_LOG"; then
  echo "M81 signed BMP1 parser host-test ledger changed." >&2
  exit 1
fi

KERNEL_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features unified-product-signed-maintenance-plan-runtime \
    persist::tests::signed_maintenance_plan_ --lib 2>&1
)"
printf '%s\n' "$KERNEL_LOG"
if ! grep -Eq \
  'test result: ok\. 3 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$KERNEL_LOG"; then
  echo "M81 durable program-binding host-test ledger changed." >&2
  exit 1
fi

cargo check --locked -p bndroid-init \
  --features unified-product-signed-maintenance-plan-runtime
cargo check --locked -p bndroid-kernel \
  --features unified-product-signed-maintenance-plan-runtime \
  --bin bndroid-kernel

echo "M81 signed maintenance-plan static contract passed."
