#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M74 signed-manifest contracts." >&2
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
        raise SystemExit(f"M74 contract source is missing: {relative}")
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


feature = "unified-product-verified-manifest-runtime"
parent = "unified-product-event-supervision-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M74 feature must extend exactly the M73 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M74 feature closure changed")

abi = "crates/bndr-abi/src/lib.rs"
for needle in (
    'feature = "unified-product-verified-manifest-runtime",',
    'not(feature = "unified-product-persistent-rollback-runtime")',
    "pub const ABI_VERSION: u64 = 35;",
    "assert_eq!(ABI_VERSION, 35);",
    "assert_eq!(SyscallNumber::from_raw(57), None);",
    "ServiceManifestOpen = 55,",
    "ServiceSupervisorReport = 56,",
):
    require(abi, needle, "ABI v35 no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 35] = [(); ABI_VERSION as usize];",
    "userspace ABI v35 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-verified-manifest-runtime"',
    "KERNEL_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-verified-manifest-runtime"',
    "USER_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-verified-manifest-runtime"',
    "userspace verified-manifest profile requires the matching kernel profile.",
):
    require("scripts/build-kernel.sh", needle, "matched M74 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME",
    "BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX",
    "configure_verified_manifest_artifact(&manifest_dir);",
    "fn configure_verified_manifest_artifact(manifest_dir: &Path)",
    "product-service-manifest-v2.bms1.hex",
    "decoded.len() != EXPECTED_BYTES",
    "cargo:rustc-env=BNDR_VERIFIED_MANIFEST_ARTIFACT",
    'feature=\\"unified-product-verified-manifest-runtime\\"',
):
    require("kernel/build.rs", needle, "strict external BMS1 build embedding")

manifest = "crates/bndr-sm/src/manifest.rs"
for needle in (
    "pub const VERIFIED_SERVICE_MANIFEST_GENERATION: u32 = 2;",
    "pub const VERIFIED_PRODUCT_SERVICE_MANIFEST_BYTES",
    "encode_product_service_manifest(VERIFIED_SERVICE_MANIFEST_GENERATION)",
    "VERIFIED_PRODUCT_SERVICE_MANIFEST_FINGERPRINT",
):
    require(manifest, needle, "generation-two signed BMF1 payload")

verifier = "crates/bndr-sm/src/verified_manifest.rs"
for needle in (
    'pub const SIGNED_SERVICE_MANIFEST_MAGIC: [u8; 4] = *b"BMS1";',
    "SIGNED_SERVICE_MANIFEST_ALGORITHM_RSA2048_PKCS1_V15_SHA256",
    "pub const PRODUCT_MANIFEST_ROLLBACK_FLOOR: u32 = 2;",
    "pub const PRODUCT_MANIFEST_RSA2048_KEY1",
    "pub fn verify_signed_service_manifest(",
    "rsa2048_pkcs1_v15_sha256_verify(",
    "pub fn sha256(input: &[u8]) -> [u8; 32]",
    "65537 = 2^16 + 1",
    "SHA256_DIGEST_INFO_PREFIX",
    "manifest.generation() != rollback_index",
    "rollback_index < minimum_rollback_index",
    "product_artifact_has_a_real_signature_and_meets_the_floor",
    "signature_mutations_fail_closed",
    "valid_old_signature_is_rejected_by_the_rollback_floor",
    "sha256_matches_standard_vectors",
):
    require(verifier, needle, "allocation-free signed-manifest verifier")
for forbidden in (
    "private_exponent",
    "PRIVATE KEY",
    "rsa_private",
    "sign(",
):
    if forbidden in read(verifier):
        raise SystemExit(f"{verifier}: verifier gained signing authority: {forbidden}")

syscall = "kernel/src/syscall.rs"
for needle in (
    'include_bytes!(env!("BNDR_VERIFIED_MANIFEST_ARTIFACT"))',
    "VERIFIED_MANIFEST_ATTEMPTS.fetch_add(1",
    "verify_signed_service_manifest(",
    "Err(SignedManifestError::Signature)",
    "Err(SignedManifestError::Rollback { actual, minimum })",
    "VERIFIED_MANIFEST_REJECTED format=1 reason=signature",
    "VERIFIED_MANIFEST_REJECTED format=1 reason=rollback",
    "Status::DataCorrupt",
    "store_verified_manifest_digest(",
    "Vmo::try_from_slice(manifest_bytes)",
    "VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.load(Ordering::Acquire) == 1",
    "VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.load(Ordering::Acquire) == 0",
):
    require(syscall, needle, "verify-before-publish kernel path")
open_body = read(syscall).split("fn service_manifest_open(", 1)[1].split(
    "\n#[cfg(feature = \"unified-product-verified-manifest-runtime\")]\nfn store_verified_manifest_digest",
    1,
)[0]
verification_position = open_body.index("verify_signed_service_manifest(")
publication_position = open_body.index("Vmo::try_from_slice(manifest_bytes)")
if verification_position >= publication_position:
    raise SystemExit("M74 manifest VMO can be published before signature verification")

product = "user/init/src/product_runtime.rs"
for needle in (
    "use bndr_sm::manifest::VERIFIED_SERVICE_MANIFEST_GENERATION;",
    "let expected_generation = VERIFIED_SERVICE_MANIFEST_GENERATION;",
    "let generation_matches = manifest.generation() == expected_generation;",
    "SyscallNumber::ServiceManifestOpen",
    "ServiceManifest::decode(&wire)",
):
    require(product, needle, "init generation-two transactional decode")

main = "kernel/src/main.rs"
for needle in (
    "external-bms1-plus-kernel-pinned-rsa2048-plus-static-rollback-floor",
    "UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK format=1 abi={}",
    "artifact=BMS1 artifact_bytes=512 signed_bytes=256",
    "algorithm=RSA2048-PKCS1-v1_5-SHA256",
    "rollback_floor=2",
    "manifest_published_after_verification=1",
    "private_key_in_repository=0 production_key_claim=0",
    "rpmb_claim=0 efuse_claim=0 hardware_rollback_claim=0",
    "BOOT_OK: M74 unified real UI is interactive; verified-manifest event supervision is starting",
    "BOOT_OK: M74 verified external manifest, rollback floor, event supervision, AppData, and PSCI shutdown armed",
    "real_phone_claim=0",
):
    require(main, needle, "sealed M74 kernel evidence")

generator = "scripts/generate_verified_service_manifest_artifact.py"
for needle in (
    'parser.add_argument("--key", type=Path, required=True)',
    'parser.add_argument("--openssl", default="openssl")',
    '"dgst",',
    '"-sha256",',
    '"-sign",',
    "signing key must use an odd, full-width RSA-2048 modulus",
    "BMS1_ARTIFACT_OK",
):
    require(generator, needle, "explicit external-key artifact generator")
if "genpkey" in read(generator) or "PRIVATE KEY" in read(generator):
    raise SystemExit("M74 artifact generator creates or embeds private keys")

evidence = "scripts/unified_product_verified_manifest_evidence.py"
for needle in (
    "UNIFIED_PRODUCT_VERIFIED_MANIFEST_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_VERIFIED_MANIFEST_REBOOT_OK",
    "VERIFIED_MANIFEST_ARTIFACTS_OK",
    "pow(signature_value, RSA_EXPONENT, modulus)",
    "negative_signature_boots=1 negative_rollback_boots=1",
    "fail_closed_boots=2",
    "manifests_published_after_rejection=0",
    "serial_negative={rejected}",
    "fail_closed_negative={negative_rejected}",
    "production_key_claim=0",
    "hardware_rollback_claim=0",
):
    require(evidence, needle, "strict M74 host evidence parser")

negative_driver = "scripts/verified_manifest_negative_qemu.py"
for needle in (
    '"-nic",',
    '"none",',
    "product.replay_interaction(",
    "VERIFIED_MANIFEST_REJECTED format=1 reason={reason}",
    "M66_PLATFORM_SHUTDOWN_DIAG failure={EXPECTED_INIT_FAILURE}",
    "manifest_published=0 init_ready=0",
    "qemu_terminated_by_host=1",
):
    require(negative_driver, needle, "real-UI fail-closed negative QEMU path")

qmp = "scripts/unified_product_qmp.py"
for needle in (
    '"M74": (',
    '{"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    'if args.milestone in {"M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}:',
    '"UNIFIED_PRODUCT_VERIFIED_MANIFEST_OK "',
    'qmp.key("f5")',
    'qmp.key("power")',
):
    require(qmp, needle, "M74 positive QMP maintenance path")

runtime = "scripts/check-unified-product-verified-manifest-runtime.sh"
for needle in (
    "unified_product_verified_manifest_evidence.py\" self-test",
    "build_kernel \"$TARGET_ROOT\" \"$PRODUCT_ARTIFACT\"",
    "build_kernel \"$SIGNATURE_TARGET_ROOT\" \"$SIGNATURE_ARTIFACT\"",
    "build_kernel \"$ROLLBACK_TARGET_ROOT\" \"$ROLLBACK_ARTIFACT\"",
    "run_negative signature",
    "run_negative rollback",
    "run_boot first",
    "run_boot second",
    "--milestone M74",
    "qemu_psci_self_exit=1",
):
    require(runtime, needle, "two-positive/two-negative M74 runtime gate")

for artifact in (
    "boot/product-service-manifest-v2.bms1.hex",
    "boot/test-fixtures/product-service-manifest-v1-rollback.bms1.hex",
    "boot/test-fixtures/product-service-manifest-v2-bad-signature.bms1.hex",
):
    source = read(artifact)
    digits = "".join(source.split())
    if len(digits) != 1024 or re.fullmatch(r"[0-9a-f]+", digits) is None:
        raise SystemExit(f"{artifact}: expected canonical 512-byte lowercase hex")

for path in root.rglob("*"):
    if not path.is_file() or "target" in path.relative_to(root).parts:
        continue
    if path.suffix.lower() in {".key", ".p12", ".pfx"}:
        raise SystemExit(f"M74 repository contains a private-key-shaped file: {path}")
    try:
        source = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    private_markers = (
        "-----BEGIN " + "PRIVATE KEY-----",
        "-----BEGIN RSA " + "PRIVATE KEY-----",
    )
    if any(marker in source for marker in private_markers):
        raise SystemExit(f"M74 repository contains private key material: {path}")

for relative in (runtime, evidence, qmp, negative_driver, generator):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M74 evidence path gained network access")

# Every Python QEMU argv must contain one disabled NIC and no network backend.
for relative in (qmp, negative_driver):
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
    raise SystemExit("M74 static contract found no literal shell QEMU launches")

for needle in (
    '"$SCRIPT_DIR/check-unified-product-verified-manifest-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-verified-manifest-runtime.sh"',
    "unified_product_verified_manifest_static=1",
    "unified_product_verified_manifest_reboot=1",
    "verified_manifest_negative_signature_boots=1",
    "verified_manifest_negative_rollback_boots=1",
):
    require("scripts/test.sh", needle, "full-suite M74 integration", count=1)

print(
    "UNIFIED_PRODUCT_VERIFIED_MANIFEST_SOURCE_OK abi=35 syscalls=0-56 "
    "artifact=BMS1 bytes=512 algorithm=RSA2048-PKCS1-v1_5-SHA256 key_id=1 "
    "rollback_index=2 rollback_floor=2 positive_boots=2 negative_boots=2 "
    "signature_rejection=1 rollback_rejection=1 manifest_published_after_rejection=0 "
    f"qemu_launches={launches} nic_none={launches} network=0 semihosting=0 "
    "production_key_claim=0 hardware_rollback_claim=0 emulator_only=1 "
    "general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/generate_verified_service_manifest_artifact.py" \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/verified_manifest_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_verified_manifest_evidence.py"
python3 "$SCRIPT_DIR/unified_product_verified_manifest_evidence.py" self-test
python3 "$SCRIPT_DIR/unified_product_verified_manifest_evidence.py" artifacts \
  "$WORKSPACE_ROOT/boot/product-service-manifest-v2.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v1-rollback.bms1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v2-bad-signature.bms1.hex"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm verified_manifest::tests::
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-verified-manifest-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-verified-manifest-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-verified-manifest-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_VERIFIED_MANIFEST_STATIC_OK source=1 feature_closure=1 abi=35 syscalls=0-56 artifact=BMS1 bytes=512 algorithm=RSA2048-PKCS1-v1_5-SHA256 key_id=1 rollback_index=2 rollback_floor=2 positive_boots=2 negative_boots=2 signature_rejection=1 rollback_rejection=1 manifest_published_after_rejection=0 parser_serial_negative=15 parser_fail_closed_negative=8 semihosting=0 network=0 production_key_claim=0 hardware_rollback_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0'
