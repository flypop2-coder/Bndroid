#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M71 continuous-supervision contracts." >&2
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
        raise SystemExit(f"M71 contract source is missing: {relative}")
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


feature = "unified-product-continuous-supervision-runtime"
parent = "unified-product-psci-shutdown-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M71 feature must extend exactly the M70 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M71 feature closure changed")

for needle in (
    'feature = "unified-product-continuous-supervision-runtime",',
    "pub const ABI_VERSION: u64 = 32;",
    "assert_eq!(ABI_VERSION, 32);",
    "ServiceShutdown = 54,",
    "assert_eq!(SyscallNumber::from_raw(55), None);",
):
    require("crates/bndr-abi/src/lib.rs", needle, "ABI v32/no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 32] = [(); ABI_VERSION as usize];",
    "userspace ABI v32 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-continuous-supervision-runtime"',
    "KERNEL_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-continuous-supervision-runtime"',
    "USER_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-continuous-supervision-runtime"',
    "userspace continuous-supervision profile requires the matching kernel profile.",
):
    require("scripts/build-kernel.sh", needle, "matched M71 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME",
    'feature=\\"unified-product-continuous-supervision-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M71 build.rs forwarding")

health = "crates/bndr-sm/src/health.rs"
for needle in (
    "pub const MAX_MISSED_PROBES_TOLERATED: u32 = 8;",
    "MissedProbeToleranceTooLarge,",
    "pub const fn with_missed_probe_tolerance(",
    "pub enum HealthDeadlineOutcome",
    "ToleratedMiss { consecutive: u32, tolerance: u32 },",
    "pub struct ServiceDefinition",
    "pub struct DependencyDefinition",
    "pub enum CatalogError",
    "pub struct ProbeBatch<const CAPACITY: usize>",
    "pub fn from_catalog(",
    "pub fn arm_probe_batch(",
    "pub fn check_health_deadline(",
    "pub consecutive_missed_probes: u32,",
    "pub total_missed_probes: u64,",
    "fn five_service_catalog_is_transactional_and_preserves_declared_topology()",
    "fn probe_batch_is_all_or_none_and_keeps_catalog_order()",
    "fn concurrent_missed_probes_are_tolerated_and_recover_independently()",
    "fn consecutive_miss_beyond_tolerance_uses_existing_restart_budget()",
):
    require(health, needle, "transactional continuous-supervisor core")

for relative, ready, proof in (
    (
        "user/init/src/product_runtime.rs",
        "pub(super) const M71_STORAGE_READY_PREFIX: u64 = 0x4d37_3147_0000_0000;",
        "pub(super) const M71_STORAGE_PROOF: u64 = 0x4d37_3150_1505_0401;",
    ),
    (
        "kernel/src/syscall.rs",
        "const M71_STORAGE_READY_PREFIX: u64 = 0x4d37_3147_0000_0000;",
        "const M71_STORAGE_PROOF: u64 = 0x4d37_3150_1505_0401;",
    ),
):
    require(relative, ready, "M71 ready identity", count=1)
    require(relative, proof, "M71 proof identity", count=1)

product = "user/init/src/product_runtime.rs"
for needle in (
    "const M71_HEALTHY_SOAK_ROUNDS: u64 = 16;",
    "const M71_RESIDENT_MISS_SEQUENCE: u64 = 20;",
    "fn dispatch_resident_health(",
    "let sequence_in_profile = expected_sequence <= 21;",
    "|| !sequence_in_profile",
    "ServiceKind::SurfaceServer | ServiceKind::InputServer",
    "let resilient_policy = policy",
    ".with_missed_probe_tolerance(1)",
    "ServiceSupervisor::<5, 4>::from_catalog(&services, &dependencies)",
    "ServiceDefinition::new(manager_identity, resilient_policy)",
    "ServiceDefinition::new(surface_identity, resilient_policy)",
    "ServiceDefinition::new(input_identity, resilient_policy)",
    "ServiceDefinition::new(first_identity, policy)",
    "ServiceDefinition::new(app_identity, resilient_policy)",
    "ServiceKind::ServiceManager,",
    "ServiceKind::SurfaceServer,",
    "ServiceKind::InputServer,",
    "ServiceKind::StorageServer,",
    "ServiceKind::App,",
    "DependencyKind::Hard,",
    "DependencyKind::Soft,",
    "fn run_continuous_supervision(",
    "for _ in 0..M71_HEALTHY_SOAK_ROUNDS",
    "fn run_continuous_healthy_batch(",
    "fn require_concurrent_health_timeout(",
    "fn validate_continuous_supervision_final(",
    ".check_health_deadline(service.identity, now_ns)",
    "(5, 4, 20, 21, 1_070_000_000, resilient_policy);",
    "snapshot.last_outbound_sequence != 21",
    "snapshot.last_inbound_sequence != 21",
    "snapshot.consecutive_missed_probes != 0",
    "snapshot.total_missed_probes != missed",
):
    require(product, needle, "five-service M71 runtime invariant")

server = "user/init/src/storage_server_runtime.rs"
for needle in (
    "let expected_health_probes = 20;",
    "2 if identity.generation() == 1 => {",
    "2..=19 => {",
    "Preserve M69's real initial StorageServer failure",
):
    require(server, needle, "M71 StorageServer probe sequence")

main = "kernel/src/main.rs"
for needle in (
    'cfg!(feature = "unified-product-continuous-supervision-runtime")',
    "0x4d37_3147_0000_0000",
    "0x4d37_3150_1505_0401",
    "UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_OK format=1 abi={}",
    "services=5 service_set=ServiceManager+SurfaceServer+InputServer+StorageServer+App",
    "catalog_transactional=1 catalog_capacity=5/4",
    "healthy_soak_rounds=16 batch_rounds=18 batched_probes=90",
    "concurrent_miss_windows=1 concurrent_miss_services=2",
    "transient_miss_recoveries=2 escalated_faults=1",
    "arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0",
    "M71_PSCI_DISCOVERY_OK format=1 abi={}",
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK format=1 abi={}",
    "BOOT_OK: M71 unified real UI is interactive and awaiting authenticated power key",
    "BOOT_OK: M71 five-service continuous supervision, concurrent miss recovery, AppData, and PSCI shutdown armed",
    "arch::aarch64::qemu_psci_power_off(validated);",
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0",
):
    require(main, needle, "sealed M71 runtime evidence")

qmp_source = read("scripts/unified_product_qmp.py")
qmp_tree = ast.parse(qmp_source)
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
    raise SystemExit("M71 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M71 QEMU argv added a network backend")
if qmp_source.count('"-semihosting-config"') != 1:
    raise SystemExit("shared QMP semihosting option count changed")
for needle in (
    '"M71": (',
    'not in {"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    'elif args.milestone == "M71":',
    '"M71_PSCI_DISCOVERY_OK "',
    '"UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_OK "',
    '"UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK "',
    'qmp.key("power")',
    "qemu_self_exit=1 power_key=116",
):
    require("scripts/unified_product_qmp.py", needle, "shared M71 QMP contract")

for needle in (
    "UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_REBOOT_OK",
    "serial_negative={rejected}",
    "host_negative={driver_rejected}",
    "supervised_services=10",
    "liveness_dependency_edges=8",
    "health_probes=214",
    "healthy=208 missed=6",
    "healthy_soak_rounds=32",
    "batch_rounds=36 batched_probes=180",
    "concurrent_miss_services=4",
    "transient_miss_recoveries=4",
    "qemu_psci_self_exits=2",
    "semihosting_uses=0",
    "hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1",
):
    require(
        "scripts/unified_product_continuous_supervision_evidence.py",
        needle,
        "strict M71 evidence parser",
    )
for needle in (
    'unified_product_continuous_supervision_evidence.py" self-test',
    "run_boot first",
    "run_boot second",
    "--milestone M71",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
    "qemu_psci_self_exit=1",
):
    require(
        "scripts/check-unified-product-continuous-supervision-runtime.sh",
        needle,
        "two-boot M71 runtime gate",
    )
for needle in (
    '"$SCRIPT_DIR/check-unified-product-continuous-supervision-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-continuous-supervision-runtime.sh"',
    "unified_product_continuous_supervision_static=1",
    "unified_product_continuous_supervision_reboot=1",
    "unified_product_continuous_supervision_boots=2",
    "unified_product_continuous_supervision_recoveries=2",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require("scripts/test.sh", needle, "full-suite M71 integration", count=1)

for relative in (
    "scripts/check-unified-product-continuous-supervision-runtime.sh",
    "scripts/unified_product_continuous_supervision_evidence.py",
    "scripts/unified_product_qmp.py",
):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M71 evidence path gained network access")
for marker in (
    "M71_FRAME_TRACE",
    "M71_STACK_TRACE",
    "M71_EXIT_TRACE",
    "M71_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M71 diagnostic trace")

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
    raise SystemExit("M71 static contract found no QEMU launch sites")

print(
    "UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_SOURCE_OK abi=32 services=5 "
    "dependency_edges=4 healthy_soak_rounds=16 batch_rounds=18 "
    "concurrent_miss_services=2 transient_miss_recoveries=2 "
    "parser_serial_negative=18 parser_host_negative=3 "
    f"qemu_launches={launches} nic_none={launches} no_new_syscall=1 "
    "semihosting_m71=0 bounded=1 arbitrary_soak_claim=0 emulator_only=1 "
    "general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_continuous_supervision_evidence.py"
python3 "$SCRIPT_DIR/unified_product_continuous_supervision_evidence.py" self-test

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
for test_name in \
  five_service_catalog_is_transactional_and_preserves_declared_topology \
  probe_batch_is_all_or_none_and_keeps_catalog_order \
  concurrent_missed_probes_are_tolerated_and_recover_independently \
  consecutive_miss_beyond_tolerance_uses_existing_restart_budget
do
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm "$test_name"
done
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-continuous-supervision-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-continuous-supervision-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-continuous-supervision-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_STATIC_OK source=1 feature_closure=1 abi=32 services=5 dependency_edges=4 healthy_soak_rounds=16 batch_rounds=18 concurrent_miss_services=2 transient_miss_recoveries=2 parser_serial_negative=18 parser_host_negative=3 no_new_syscall=1 semihosting=0 full_suite=1 bounded=1 arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0'
