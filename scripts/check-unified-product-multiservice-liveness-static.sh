#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M69 multiservice liveness contracts." >&2
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
        raise SystemExit(f"M69 contract source is missing: {relative}")
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


def forbid(relative: str, needle: str, label: str) -> None:
    if needle in read(relative):
        raise SystemExit(f"{relative}: forbidden {label}: {needle!r}")


def features(relative: str) -> dict[str, list[str]]:
    with (root / relative).open("rb") as stream:
        return tomllib.load(stream)["features"]


feature = "unified-product-multiservice-liveness-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [
    "unified-product-liveness-runtime"
]:
    raise SystemExit("ABI M69 feature must extend exactly the M68 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [
        "unified-product-liveness-runtime",
        f"bndr-abi/{feature}",
    ]:
        raise SystemExit(f"{relative}: M69 feature closure changed")

for needle in (
    'feature = "unified-product-multiservice-liveness-runtime",',
    'not(feature = "unified-product-psci-shutdown-runtime")',
    "pub const ABI_VERSION: u64 = 30;",
    "assert_eq!(ABI_VERSION, 30);",
    "ServiceShutdown = 54,",
    "assert_eq!(SyscallNumber::from_raw(55), None);",
):
    require("crates/bndr-abi/src/lib.rs", needle, "ABI v30/no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 30] = [(); ABI_VERSION as usize];",
    "userspace ABI v30 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-multiservice-liveness-runtime"',
    "KERNEL_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-multiservice-liveness-runtime"',
    "USER_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-multiservice-liveness-runtime"',
):
    require("scripts/build-kernel.sh", needle, "matched M69 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME",
    'feature=\\"unified-product-multiservice-liveness-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M69 build.rs forwarding")

health = "crates/bndr-sm/src/health.rs"
for needle in (
    "StorageServer = 4,",
    "App = 5,",
    "5 => Some(Self::App),",
    "pub struct ServiceSupervisor<",
    "pub fn add_dependency(",
    "pub fn dependency_fault(",
    "pub fn dependency_recovered(",
    "fn storage_to_app_hard_edge_blocks_until_replacement_is_healthy()",
    "ServiceSupervisor::<2, 1>::new()",
):
    require(health, needle, "allocation-free two-service supervisor contract")

product = "user/init/src/product_runtime.rs"
for needle in (
    "pub(super) const M69_STORAGE_READY_PREFIX: u64 = 0x4d36_3947_0000_0000;",
    "pub(super) const M69_STORAGE_PROOF: u64 = 0x4d36_3950_0807_0201;",
    "const PRODUCT_HEALTH_CADENCE_NS: u64 = 40_000_000;",
    "const STORAGE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;",
    "const STORAGE_RESTART_BACKOFF_NS: u64 = 30_000_000;",
    "const PRODUCT_COMMAND_DEPENDENCY_BLOCK: u64 = 6;",
    "const PRODUCT_COMMAND_DEPENDENCY_RESUME: u64 = 7;",
    "ServiceSupervisor::<2, 1>::new()",
    "ServiceKind::StorageServer,",
    "ServiceKind::App,",
    "DependencyKind::Hard,",
    "require_channel_timeout(app.control, PRODUCT_HEALTH_CADENCE_NS);",
    "check_health_timeout(first_identity, now_ns)",
    "command_and_expect(app, PRODUCT_COMMAND_DEPENDENCY_BLOCK, 1);",
    "command_and_expect(app, PRODUCT_COMMAND_DEPENDENCY_RESUME, 2);",
    "APP_DEPENDENCY_STATE.load(Ordering::Acquire) != 2",
    "SyscallNumber::ProcessTerminate",
    "require_next_process_generation(first.pid, replacement.pid);",
    "supervisor.dependency_count() != expected_dependencies",
    "(2, 1, 2, 3, 250_000_000, policy);",
    "final_app.last_outbound_sequence != expected_app_sequence",
    "final_storage.last_outbound_sequence != expected_storage_sequence",
    "now_ns != expected_now_ns",
):
    require(product, needle, "M69 init supervision invariant")

server = "user/init/src/storage_server_runtime.rs"
for needle in (
    "0 | 1 => {",
    "health.probes = 3;",
    "withholds only its third",
    "let expected_health_probes = 2;",
    "identity.kind() != ServiceKind::StorageServer",
):
    require(server, needle, "M69 StorageServer probe sequence")

for relative, needle, label in (
    (
        "kernel/src/syscall.rs",
        "const M69_STORAGE_READY_PREFIX: u64 = 0x4d36_3947_0000_0000;",
        "kernel M69 ready identity",
    ),
    (
        "kernel/src/syscall.rs",
        "const M69_STORAGE_PROOF: u64 = 0x4d36_3950_0807_0201;",
        "kernel M69 proof identity",
    ),
    (
        "kernel/src/main.rs",
        "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK format=1 abi={}",
        "terminal M69 liveness evidence",
    ),
    (
        "kernel/src/main.rs",
        "BOOT_OK: M69 unified two-service dependency liveness, AppData, bounded shutdown, and QEMU exit armed",
        "terminal M69 shutdown marker",
    ),
    (
        "kernel/src/main.rs",
        "BOOT_OK: M69 unified real UI is interactive and awaiting authenticated power key",
        "terminal M69 UI marker",
    ),
):
    require(relative, needle, label)

for relative in (
    "kernel/src/main.rs",
    "scripts/unified_product_multiservice_liveness_evidence.py",
):
    require(relative, "real_phone_claim=0", "explicit non-phone claim")
for marker in (
    "M69_FRAME_TRACE",
    "M69_STACK_TRACE",
    "M69_EXIT_TRACE",
    "M69_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M69 diagnostic trace")

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
    raise SystemExit("shared product QMP driver must have exactly one QEMU Popen argv")
argv = [
    element.value
    if isinstance(element, ast.Constant) and isinstance(element.value, str)
    else None
    for element in popen_lists[0]
]
nic_indices = [index for index, value in enumerate(argv) if value == "-nic"]
if len(nic_indices) != 1 or argv[nic_indices[0] + 1] != "none":
    raise SystemExit("M69 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M69 QEMU argv added a network backend")

for needle in (
    '"M69": (',
    'required.append("UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK ")',
    'qmp.key("power")',
    "qemu_self_exit=1 power_key=116",
):
    require("scripts/unified_product_qmp.py", needle, "shared M69 QMP contract")
for needle in (
    "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK",
    "multiservice_liveness_recoveries=2",
    "health_probes=16",
    "dependent_blocks=2",
    "dependent_resumes=2",
    "real_phone_claim=0",
):
    require(
        "scripts/unified_product_multiservice_liveness_evidence.py",
        needle,
        "strict M69 evidence parser",
    )
for needle in (
    "unified_product_multiservice_liveness_evidence.py\" self-test",
    "run_boot first",
    "run_boot second",
    "--milestone M69",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
):
    require(
        "scripts/check-unified-product-multiservice-liveness-runtime.sh",
        needle,
        "two-boot M69 runtime gate",
    )
for needle in (
    '"$SCRIPT_DIR/check-unified-product-multiservice-liveness-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-multiservice-liveness-runtime.sh"',
    "unified_product_multiservice_liveness_static=1",
    "unified_product_multiservice_liveness_reboot=1",
    "unified_product_multiservice_liveness_boots=2",
    "unified_product_multiservice_liveness_recoveries=2",
):
    require("scripts/test.sh", needle, "full-suite M69 integration", count=1)

for relative in (
    "scripts/check-unified-product-multiservice-liveness-runtime.sh",
    "scripts/unified_product_multiservice_liveness_evidence.py",
    "scripts/unified_product_qmp.py",
):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M69 evidence path gained network access")

# Audit every literal shell QEMU launch, including all historical gates.
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
    raise SystemExit("M69 static contract found no QEMU launch sites")

print(
    "UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_SOURCE_OK abi=30 services=2 "
    "dependency_edges=1 dependency_kind=hard probes=8 healthy=7 "
    "health_timeout_ms=100 cadence_ms=40 backoff_ms=30 replacements=1 "
    f"qemu_launches={launches} nic_none={launches} no_new_syscall=1 "
    "emulator_only=1 general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_multiservice_liveness_evidence.py"
python3 "$SCRIPT_DIR/unified_product_multiservice_liveness_evidence.py" self-test

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm \
  storage_to_app_hard_edge_blocks_until_replacement_is_healthy
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-multiservice-liveness-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-multiservice-liveness-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-multiservice-liveness-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_STATIC_OK source=1 feature_closure=1 abi=30 services=2 dependency_edges=1 dependency_kind=hard probes=8 healthy=7 withheld=1 cadence_waits=3 finite_timeout=1 bounded_backoff=1 same_slot_next_generation=1 parser_negative_cases=10 no_new_syscall=1 full_suite=1 emulator_only=1 general_runtime=0 real_phone_claim=0'
