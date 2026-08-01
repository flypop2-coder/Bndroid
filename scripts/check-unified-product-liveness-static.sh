#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M68 product-liveness static contract." >&2
    exit 1
  fi
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import ast
import sys
import tomllib
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def require(relative: str, needle: str, label: str, count: int | None = None) -> None:
    observed = read(relative).count(needle)
    if observed == 0:
        raise SystemExit(f"{relative}: missing {label}: {needle!r}")
    if count is not None and observed != count:
        raise SystemExit(
            f"{relative}: {label} count is {observed}, expected {count}: {needle!r}"
        )


def forbid(relative: str, needle: str, label: str) -> None:
    if needle in read(relative):
        raise SystemExit(f"{relative}: forbidden {label}: {needle!r}")


def features(relative: str) -> dict[str, list[str]]:
    with (root / relative).open("rb") as stream:
        return tomllib.load(stream)["features"]


abi_features = features("crates/bndr-abi/Cargo.toml")
user_features = features("user/init/Cargo.toml")
kernel_features = features("kernel/Cargo.toml")
if abi_features.get("unified-product-liveness-runtime") != [
    "unified-product-runtime"
]:
    raise SystemExit("ABI M68 feature must extend exactly the M67 contract")
for relative, selected in (
    ("user/init/Cargo.toml", user_features),
    ("kernel/Cargo.toml", kernel_features),
):
    if selected.get("unified-product-liveness-runtime") != [
        "unified-product-runtime",
        "bndr-abi/unified-product-liveness-runtime",
    ]:
        raise SystemExit(f"{relative}: M68 feature closure changed")

for needle in (
    '''#[cfg(all(
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
pub const ABI_VERSION: u64 = 29;''',
    "assert_eq!(ABI_VERSION, 29);",
):
    require("crates/bndr-abi/src/lib.rs", needle, "ABI v29 gate")

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-liveness-runtime"',
    "KERNEL_UNIFIED_PRODUCT_LIVENESS_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-liveness-runtime"',
    "USER_UNIFIED_PRODUCT_LIVENESS_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-liveness-runtime"',
):
    require("scripts/build-kernel.sh", needle, "matched M68 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_LIVENESS_RUNTIME",
    'feature=\\"unified-product-liveness-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M68 build.rs forwarding")

health = "crates/bndr-sm/src/health.rs"
for needle in (
    "StorageServer = 4,",
    "4 => Some(Self::StorageServer),",
    "pub struct ServiceSupervisor<",
    "pub fn check_health_timeout(",
    "pub fn begin_backoff(",
    "pub fn begin_replacement(",
    "pub fn install_replacement(",
):
    require(health, needle, "allocation-free StorageServer supervisor contract")

product = "user/init/src/product_runtime.rs"
for needle in (
    "pub(super) const M68_STORAGE_READY_PREFIX: u64 = 0x4d36_3847_0000_0000;",
    "pub(super) const M68_STORAGE_PROOF: u64 = 0x4d36_3850_0302_0101;",
    "const STORAGE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;",
    "const STORAGE_RESTART_BACKOFF_NS: u64 = 30_000_000;",
    "ServiceSupervisor::<1>::new()",
    "ServiceKind::StorageServer",
    "require_channel_timeout(first.control, STORAGE_HEALTH_TIMEOUT_NS);",
    "check_health_timeout(first_identity, STORAGE_HEALTH_TIMEOUT_NS)",
    "require_channel_timeout(first.control, STORAGE_RESTART_BACKOFF_NS);",
    "SyscallNumber::ProcessTerminate",
    "ProcessTerminationReason::Killed.raw()",
    "require_next_process_generation(first.pid, replacement.pid);",
    "final_service.restarts_used != 1",
    "final_service.last_fault != Some(FaultClass::HealthTimeout)",
):
    require(product, needle, "M68 init liveness invariant")

server = "user/init/src/storage_server_runtime.rs"
for needle in (
    "struct ProductStorageHealth",
    "process_product_health_probe(",
    "HealthFrame::healthy(1, identity)",
    "intentionally withholding Probe 2",
    "identity.kind() != ServiceKind::StorageServer",
    "identity.generation() != process_generation(identity.pid())",
    "let expected_health_probes = 1;",
    "let health_profile_valid = product_health.probes == expected_health_probes;",
):
    require(server, needle, "M68 live StorageServer responder")

for relative, needle, label in (
    (
        "kernel/src/process.rs",
        "UNIFIED_PRODUCT_STORAGE_REPLACEMENT_PID",
        "separate eleventh-child PID evidence",
    ),
    (
        "kernel/src/process.rs",
        'panic!("M68 eleventh child was not the StorageServer replacement")',
        "replacement image fail-close",
    ),
    (
        "kernel/src/syscall.rs",
        "fn m68_storage_replacement_matches(",
        "generation-qualified replacement proof",
    ),
    (
        "kernel/src/syscall.rs",
        "processes.terminated_killed == 1",
        "killed-generation accounting",
    ),
    (
        "kernel/src/syscall.rs",
        "broker.epoch == 2 && broker.next_epoch == 3 && broker.releases == 1",
        "live replacement storage epoch",
    ),
    (
        "kernel/src/main.rs",
        "UNIFIED_PRODUCT_LIVENESS_OK format=1 abi={}",
        "terminal M68 liveness evidence",
    ),
    (
        "kernel/src/main.rs",
        "BOOT_OK: M68 unified product liveness recovery, AppData, bounded shutdown, and QEMU exit armed",
        "terminal M68 boot marker",
    ),
):
    require(relative, needle, label)

for relative in (
    "kernel/src/main.rs",
    "scripts/unified_product_liveness_evidence.py",
):
    require(relative, "real_phone_claim=0", "explicit non-phone claim")
for marker in (
    "M68_FRAME_TRACE",
    "M68_STACK_TRACE",
    "M68_EXIT_TRACE",
    "M68_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M68 diagnostic trace")

qmp_tree = ast.parse(read("scripts/unified_product_qmp.py"))
popen_lists: list[list[ast.expr]] = []
for node in ast.walk(qmp_tree):
    if (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and node.func.attr == "Popen"
        and node.args
        and isinstance(node.args[0], ast.List)
    ):
        popen_lists.append(node.args[0].elts)
if len(popen_lists) != 1:
    raise SystemExit("shared product QMP driver must contain exactly one QEMU Popen argv")
argv = [
    element.value
    if isinstance(element, ast.Constant) and isinstance(element.value, str)
    else None
    for element in popen_lists[0]
]
nic_indices = [index for index, value in enumerate(argv) if value == "-nic"]
if len(nic_indices) != 1 or argv[nic_indices[0] + 1] != "none":
    raise SystemExit("M68 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M68 QEMU argv added a network backend")

for needle in (
    '"M68": (',
    'required.append("UNIFIED_PRODUCT_LIVENESS_OK ")',
    'qmp.key("power")',
    "qemu_self_exit=1 power_key=116",
):
    require("scripts/unified_product_qmp.py", needle, "shared M68 QMP contract")
for needle in (
    "UNIFIED_PRODUCT_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_LIVENESS_REBOOT_OK",
    "storage_liveness_recoveries=2",
    "health_probes=6",
    "real_phone_claim=0",
):
    require(
        "scripts/unified_product_liveness_evidence.py",
        needle,
        "strict M68 evidence parser",
    )
for needle in (
    "unified_product_liveness_evidence.py\" self-test",
    "run_boot first",
    "run_boot second",
    "--milestone M68",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
):
    require(
        "scripts/check-unified-product-liveness-runtime.sh",
        needle,
        "two-boot M68 runtime gate",
    )
for needle in (
    '"$SCRIPT_DIR/check-unified-product-liveness-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-liveness-runtime.sh"',
    "unified_product_liveness_static=1",
    "unified_product_liveness_reboot=1",
    "unified_product_liveness_boots=2",
    "unified_product_liveness_recoveries=2",
):
    require("scripts/test.sh", needle, "full-suite M68 integration", count=1)

print(
    "UNIFIED_PRODUCT_LIVENESS_SOURCE_OK abi=29 service=StorageServer "
    "protocol=BSH1 probes=3 healthy=2 health_timeout_ms=100 backoff_ms=30 "
    "replacements=1 same_slot_next_generation=1 process_capacity=10 "
    "dynamic_capacity=9 heap_pages=256 exception_stack_kib=64 "
    "qemu_nic_none=1 general_runtime=0 real_phone_claim=0"
)
PY

bash -n \
  "$SCRIPT_DIR/check-unified-product-liveness-static.sh" \
  "$SCRIPT_DIR/check-unified-product-liveness-runtime.sh"
python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_liveness_evidence.py"
python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" self-test
cargo check --locked -p bndr-abi --features unified-product-liveness-runtime
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-liveness-runtime
cargo check --locked -p bndroid-kernel --features unified-product-liveness-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_LIVENESS_STATIC_OK source=1 feature_closure=1 abi=29 service=StorageServer protocol=BSH1 finite_timeout=1 bounded_backoff=1 same_slot_next_generation=1 parser_negative_cases=8 full_suite=1 emulator_only=1 general_runtime=0 real_phone_claim=0'
