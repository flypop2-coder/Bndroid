#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M77 maintenance authorization contracts." >&2
    exit 1
  fi
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import re
import sys
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M77 contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, label: str, *, count: int | None = None) -> None:
    actual = text.count(needle)
    if actual == 0:
        raise SystemExit(f"{label} is missing: {needle}")
    if count is not None and actual != count:
        raise SystemExit(f"{label} count changed for {needle}: {actual}, expected {count}")


abi_toml = read("crates/bndr-abi/Cargo.toml")
kernel_toml = read("kernel/Cargo.toml")
user_toml = read("user/init/Cargo.toml")
require(
    abi_toml,
    'unified-product-maintenance-authorization-runtime = ["unified-product-key-rotation-runtime"]',
    "ABI exact M77 feature closure",
    count=1,
)
for relative, text in (
    ("kernel/Cargo.toml", kernel_toml),
    ("user/init/Cargo.toml", user_toml),
):
    require(
        text,
        "unified-product-maintenance-authorization-runtime = [",
        f"{relative} M77 feature declaration",
        count=1,
    )
    require(
        text,
        '"unified-product-key-rotation-runtime",',
        f"{relative} M77 inherited feature",
    )
    require(
        text,
        '"bndr-abi/unified-product-maintenance-authorization-runtime",',
        f"{relative} M77 ABI feature forwarding",
        count=1,
    )

abi = read("crates/bndr-abi/src/lib.rs")
for needle in (
    '#[cfg(feature = "unified-product-maintenance-authorization-runtime")]',
    "pub const ABI_VERSION: u64 = 38;",
    "pub const MAINTENANCE_SESSION_OPEN_FLAGS_NONE: u64 = 0;",
    "MaintenanceSessionOpen = 57",
    "57 => Some(Self::MaintenanceSessionOpen)",
    "assert_eq!(SyscallNumber::from_raw(58), None);",
):
    require(abi, needle, "M77 ABI 38 syscall contract")

build_kernel = read("scripts/build-kernel.sh")
for needle in (
    "KERNEL_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME=1",
    "USER_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME=1",
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-maintenance-authorization-runtime"',
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-maintenance-authorization-runtime"',
    'append_feature "$USERSPACE_FEATURES" "unified-product-maintenance-authorization-runtime"',
):
    require(build_kernel, needle, "M77 kernel/userspace feature forwarding")

kernel_build = read("kernel/build.rs")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME",
    "BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX",
    'feature=\\"unified-product-maintenance-authorization-runtime\\"',
    "configure_maintenance_authorization_artifact",
    "maintenance-authorization-sequence1.bma1.hex",
    "BNDR_MAINTENANCE_AUTHORIZATION_ARTIFACT",
):
    require(kernel_build, needle, "M77 build-time artifact and cfg propagation")

user_main = read("user/init/src/main.rs")
require(
    user_main,
    '#[cfg(all(\n    feature = "unified-product-maintenance-authorization-runtime",\n    not(feature = "unified-product-maintenance-execution-runtime")\n))]\nconst _: [(); 38] = [(); ABI_VERSION as usize];',
    "M77 userspace ABI pin",
    count=1,
)
product = read("user/init/src/product_runtime.rs")
for needle in (
    "SyscallNumber::MaintenanceSessionOpen",
    "MAINTENANCE_SESSION_OPEN_FLAGS_NONE + 1",
    "Status::PermissionDenied",
    "Status::InvalidState",
    "MAINTENANCE_OPERATION_STORAGE_ROTATION",
):
    require(product, needle, "M77 init maintenance-session proof")
run_start = product.index("fn run_event_supervision")
run_end = product.index("let mut now_ns", run_start)
session_proof = product[run_start:run_end]
if not (
    session_proof.index("SyscallNumber::ServiceSupervisorReport")
    < session_proof.index("Status::PermissionDenied")
    < session_proof.index("SyscallNumber::MaintenanceSessionOpen")
    < session_proof.index("MAINTENANCE_OPERATION_STORAGE_ROTATION")
):
    raise SystemExit("M77 init did not prove the pre-session gate before session publication")

authorization = read("crates/bndr-sm/src/maintenance_authorization.rs")
for needle in (
    'pub const MAINTENANCE_AUTHORIZATION_MAGIC: [u8; 4] = *b"BMA1";',
    "pub const MAINTENANCE_AUTHORIZATION_SIZE: usize",
    "pub const MAINTENANCE_OPERATION_STORAGE_ROTATION: u64 = 1;",
    "pub const MAINTENANCE_AUTHORIZATION_MAX_USES: u32 = 2;",
    "pub const MAINTENANCE_AUTHORIZATION_ROOT_SHA256: [u8; 32]",
    "pub const MAINTENANCE_PRODUCT_MANIFEST_SHA256: [u8; 32]",
    "pub const MAINTENANCE_PRODUCT_KEY_POLICY_SHA256: [u8; 32]",
    "pub const MAINTENANCE_DEVICE_BINDING_SHA256: [u8; 32]",
    "pub const MAINTENANCE_POLICY_SHA256: [u8; 32]",
    "rsa2048_pkcs1_v15_sha256_verify",
    "MaintenanceAuthorizationError::Signature",
    "MaintenanceAuthorizationError::Binding",
    "authorization_id",
):
    require(authorization, needle, "M77 BMA1 verifier")
verify_start = authorization.index("pub fn verify_maintenance_authorization")
verify_end = authorization.index("\npub fn authorization_id", verify_start)
verify = authorization[verify_start:verify_end]
if not (
    verify.index("rsa2048_pkcs1_v15_sha256_verify")
    < verify.index("MaintenanceAuthorizationError::Binding")
    < verify.index("MaintenanceAuthorizationError::AuthorizationId")
):
    raise SystemExit("M77 verifier lost signature-before-binding-before-ID order")

persist = read("kernel/src/persist.rs")
for needle in (
    'pub const MAINTENANCE_AUDIT_STATE_MAGIC: [u8; 8] = *b"BNDRMAU1";',
    "pub const MAINTENANCE_AUDIT_STATE_BYTES: usize = 320;",
    "pub const MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [5, 6];",
    "pub struct MaintenanceAuthorizationAuditBinding",
    "pub fn maintenance_audit_chain_sha256",
    "pub fn commit_maintenance_authorization",
    "MaintenanceAuditError::BootstrapSequence",
    "MaintenanceAuditError::Replay",
    "MaintenanceAuditError::SequenceGap",
    "MaintenanceAuditError::NamespaceMismatch",
    "IoPhase::WriteInactiveSlot",
    "IoPhase::Flush",
    "IoPhase::ReadVerificationSlot",
):
    require(persist, needle, "M77 durable audit transaction")
commit_start = persist.index("pub fn commit_maintenance_authorization")
commit = persist[commit_start:]
if not (
    commit.index("MaintenanceAuditError::Replay")
    < commit.index("io.write_sector")
    < commit.index("io.flush()")
    < commit.index("IoPhase::ReadVerificationSlot")
):
    raise SystemExit("M77 replay rejection or write/flush/readback order changed")

adapter = read("kernel/src/storage_persist.rs")
for needle in (
    "MaintenanceAuditTransaction(MaintenanceAuditError)",
    "pub fn commit_verified_maintenance_authorization",
    "DATA_PARTITION_FIRST_LBA",
    "storage::durability_contract()",
    "storage::device_contract()",
    "commit_maintenance_authorization(&mut io, request)",
):
    require(adapter, needle, "IRQ-backed M77 persistence adapter")

syscall = read("kernel/src/syscall.rs")
for needle in (
    "MAINTENANCE_AUTHORIZATION_ARTIFACT",
    "MAINTENANCE_AUTHORIZATION_PREPARATION",
    "pub fn prepare_maintenance_authorization",
    "verify_maintenance_authorization",
    "commit_verified_maintenance_authorization",
    "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=signature",
    "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=binding",
    "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=replay",
    "MAINTENANCE_AUDIT_OK format=1 state=BNDRMAU1",
    "fn maintenance_session_open",
    "MAINTENANCE_SESSION_OPENED",
    "MAINTENANCE_REPORT_GATE_DENIALS",
    "SyscallNumber::MaintenanceSessionOpen",
):
    require(syscall, needle, "M77 pre-EL0 verification, session, and report gate")
prepare_start = syscall.index("pub fn prepare_maintenance_authorization")
prepare_end = syscall.index("\npub fn maintenance_authorization_snapshot", prepare_start)
prepare = syscall[prepare_start:prepare_end]
if not (
    prepare.index("verify_maintenance_authorization")
    < prepare.index("commit_verified_maintenance_authorization")
    < prepare.index("MAINTENANCE_AUTHORIZATION_PREPARATION.publish(evidence)")
):
    raise SystemExit("M77 preparation lost verify/audit/publication order")
report_start = syscall.index("fn service_supervisor_report")
report_end = syscall.index("\nfn file_open_at", report_start)
report = syscall[report_start:report_end]
if not (
    report.index("SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY")
    < report.index("MAINTENANCE_SESSION_OPENED")
    < report.index("event_supervision_trace::report")
):
    raise SystemExit("M77 report gate no longer preserves the read-only UI query")
if "enable_irq" in prepare or "restore_daif" in prepare:
    raise SystemExit("M77 pre-EL0 transaction gained an unreviewed IRQ state transition")

main = read("kernel/src/main.rs")
for needle in (
    "syscall::prepare_maintenance_authorization",
    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK format=1 abi={}",
    "trusted_monotonic_backend=0",
    "private_key_in_repository=0",
    "real_phone_claim=0",
    "BOOT_OK: M77 unified real UI is interactive; signed maintenance-session event supervision is starting",
    "BOOT_OK: M77 signed maintenance session, durable anti-replay audit, persistent key policy, event supervision, AppData, and PSCI shutdown armed",
):
    require(main, needle, "sealed M77 kernel evidence")
if not (
    main.index("syscall::prepare_key_rotation_verified_manifest")
    < main.index("syscall::prepare_maintenance_authorization")
    < main.index("userboot::start()")
):
    raise SystemExit("M77 preparation is not between key-policy preparation and EL0 start")

offline = read("scripts/offline_maintenance_authorization.py")
for needle in (
    'subparsers.add_parser("prepare")',
    'subparsers.add_parser("assemble")',
    "BMA1_OFFLINE_REQUEST_OK",
    "BMA1_OFFLINE_ASSEMBLY_OK",
    "private_key_consumed=0",
):
    require(offline, needle, "M77 offline split-signing workflow")
for forbidden in ("--private-key", "genrsa", "genpkey", "openssl dgst -sign"):
    if forbidden in offline:
        raise SystemExit(f"M77 offline assembler gained a signing primitive: {forbidden}")

evidence = read("scripts/unified_product_maintenance_authorization_evidence.py")
for needle in (
    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_EVIDENCE_PARSER_SELF_TEST_OK",
    "MAINTENANCE_AUTHORIZATION_ARTIFACTS_OK",
    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_REBOOT_OK",
    "decode_audit_record",
    "rejected boot mutated an audit slot",
    "trusted_monotonic_backend=0",
):
    require(evidence, needle, "strict M77 artifact/log/disk parser")

negative = read("scripts/maintenance_authorization_negative_qemu.py")
for needle in (
    'choices=("signature", "binding", "replay", "completed-replay")',
    "marker_format = 2 if reason == \"completed-replay\" else 1",
    "MAINTENANCE_AUTHORIZATION_NEGATIVE_BOOT_OK",
    '"-nic",\n            "none",',
):
    require(negative, needle, "pre-EL0 M77 negative QEMU driver")
if negative.count('"qemu-system-aarch64"') != 1 or negative.count('"-nic"') != 1:
    raise SystemExit("M77 negative QEMU launch must contain exactly one -nic none")

qmp = read("scripts/unified_product_qmp.py")
for needle in (
    '"M77": (',
    '{"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    '"MAINTENANCE_AUDIT_OK "',
    '"UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_OK "',
):
    require(qmp, needle, "M77 positive QMP maintenance path")
if qmp.count('"qemu-system-aarch64"') != 1 or qmp.count('"-nic"') != 1:
    raise SystemExit("unified product QMP launch must contain exactly one -nic none")

runtime = read("scripts/check-unified-product-maintenance-authorization-runtime.sh")
for needle in (
    "run_positive sequence1 M77",
    "run_positive sequence2 M77",
    'run_negative signature "$SIGNATURE_KERNEL"',
    'run_negative binding "$BINDING_KERNEL"',
    'run_negative replay "$SEQUENCE2_KERNEL"',
    "unified_product_maintenance_authorization_evidence.py",
    "offline_maintenance_authorization.py",
    "M77_BOOT_SEQUENCE phase=sequence1 qemu_psci_self_exit=1",
    "M77_NEGATIVE_BOOT_SEQUENCE reason=$reason host_terminated=1",
):
    require(runtime, needle, "six-positive/three-negative M77 runtime gate")

for relative in (
    "scripts/offline_maintenance_authorization.py",
    "scripts/unified_product_maintenance_authorization_evidence.py",
    "scripts/maintenance_authorization_negative_qemu.py",
    "scripts/check-unified-product-maintenance-authorization-runtime.sh",
):
    text = read(relative).lower()
    for forbidden in ("http://", "https://", "curl ", "wget ", "git clone"):
        if forbidden in text:
            raise SystemExit(f"{relative}: M77 evidence path gained network access")

scan_paths = [
    path
    for top in ("boot", "crates", "kernel", "scripts", "user")
    for path in (root / top).rglob("*")
]
scan_paths.extend(path for path in root.iterdir() if path.is_file())
for path in scan_paths:
    if not path.is_file():
        continue
    relative = path.relative_to(root)
    if relative == Path("scripts/check-unified-product-maintenance-authorization-static.sh"):
        continue
    lowered = path.name.lower()
    if "private" in lowered and lowered.endswith((".pem", ".key", ".der")):
        raise SystemExit(f"M77 repository contains a private-key-shaped file: {relative}")
    try:
        payload = path.read_bytes()
    except OSError:
        continue
    if b"BEGIN PRIVATE KEY" in payload or b"BEGIN RSA PRIVATE KEY" in payload:
        raise SystemExit(f"M77 repository contains private key material: {relative}")

test_sh = read("scripts/test.sh")
require(
    test_sh,
    "unified-product-maintenance-authorization-runtime",
    "full-suite M77 feature integration",
)
require(
    test_sh,
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-maintenance-authorization-runtime.sh"',
    "full-suite M77 runtime integration",
    count=1,
)

print(
    "UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_SOURCE_OK "
    "abi=38 syscalls=0-57 artifact=BMA1 signed_bytes=256 signature_first=1 "
    "manifest_binding=1 key_policy_binding=1 device_binding=1 policy_binding=1 "
    "audit_state=BNDRMAU1 audit_slots=5/6 exact_sequence=1 hash_chain=1 "
    "write_flush_readback=1 init_only_session=1 report_gate=1 "
    "ui_query_read_only=1 offline_split_signing=1 private_key_in_repository=0 "
    "network_access=0 trusted_monotonic_backend=0 production_key_claim=0 "
    "hsm_claim=0 rpmb_claim=0 efuse_claim=0 emulator_only=1 real_phone_claim=0"
)
PY

python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" self-test
python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
  artifacts \
  "$WORKSPACE_ROOT/boot/maintenance-authorization-sequence1.bma1.hex" \
  "$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/maintenance-authorization-sequence2-bad-signature.bma1.hex" \
  "$WORKSPACE_ROOT/boot/test-fixtures/maintenance-authorization-sequence3-wrong-binding.bma1.hex" \
  "$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence1.request.hex" \
  "$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence1.signature.hex" \
  "$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence2.request.hex" \
  "$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence2.signature.hex"

echo "M77 maintenance authorization static contract passed."
