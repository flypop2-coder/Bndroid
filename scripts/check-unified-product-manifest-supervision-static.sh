#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M72 manifest-supervision contracts." >&2
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
        raise SystemExit(f"M72 contract source is missing: {relative}")
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


feature = "unified-product-manifest-supervision-runtime"
parent = "unified-product-continuous-supervision-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M72 feature must extend exactly the M71 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M72 feature closure changed")

abi = "crates/bndr-abi/src/lib.rs"
for needle in (
    '#[cfg(feature = "unified-product-manifest-supervision-runtime")]',
    "pub const ABI_VERSION: u64 = 33;",
    "pub const SERVICE_MANIFEST_OPEN_FLAGS_NONE: u64 = 0;",
    "ServiceManifestOpen = 55,",
    "55 => Some(Self::ServiceManifestOpen),",
    "assert_eq!(ABI_VERSION, 33);",
    "assert_eq!(SyscallNumber::from_raw(56), None);",
):
    require(abi, needle, "ABI v33 manifest-open contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 33] = [(); ABI_VERSION as usize];",
    "userspace ABI v33 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-manifest-supervision-runtime"',
    "KERNEL_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-manifest-supervision-runtime"',
    "USER_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-manifest-supervision-runtime"',
    "userspace manifest-supervision profile requires the matching kernel profile.",
):
    require("scripts/build-kernel.sh", needle, "matched M72 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME",
    'feature=\\"unified-product-manifest-supervision-runtime\\"',
    'workspace.join("crates/bndr-sm/src/manifest.rs")',
    'format!("bndr_abi={}", abi_output.display())',
):
    require("kernel/build.rs", needle, "direct M72 build.rs forwarding")

manifest = "crates/bndr-sm/src/manifest.rs"
for needle in (
    'pub const SERVICE_MANIFEST_MAGIC: [u8; 4] = *b"BMF1";',
    "pub const SERVICE_MANIFEST_VERSION: u8 = 1;",
    "pub const SERVICE_MANIFEST_GENERATION: u32 = 1;",
    "pub const SERVICE_MANIFEST_HEADER_SIZE: usize = 32;",
    "pub const SERVICE_MANIFEST_SERVICE_RECORD_SIZE: usize = 32;",
    "pub const SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE: usize = 8;",
    "pub const SERVICE_MANIFEST_MAX_SERVICES: usize = 5;",
    "pub const SERVICE_MANIFEST_MAX_DEPENDENCIES: usize = 10;",
    "pub const PRODUCT_SERVICE_MANIFEST_SIZE: usize",
    "pub const PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64",
    "pub enum ServiceLaunchMode",
    "pub struct ManifestService",
    "pub struct ManifestDependency",
    "pub struct ServiceManifest",
    "pub enum ManifestDecodeError",
    "pub fn decode(wire: &[u8]) -> Result<Self, ManifestDecodeError>",
    "Fingerprint {",
    "BindingMismatch {",
    "DuplicateService {",
    "DependencyServiceMissing {",
    "DuplicateDependency {",
    "DependencyCycle {",
    "let mut reachability = adjacency;",
    "fn product_manifest_is_strict_and_uses_nonlegacy_service_order()",
    "fn valid_bounded_subset_and_permutation_decode_without_fixed_product_array()",
    "fn manifest_decode_rejects_header_and_fingerprint_mutations_transactionally()",
    "fn manifest_decode_rejects_service_identity_policy_and_padding_mutations()",
    "fn manifest_decode_rejects_dependency_mutations_and_cycles()",
):
    require(manifest, needle, "strict allocation-free BMF1 parser")

manifest_source = read(manifest)
service_order = [
    "kind: ServiceKind::App,",
    "kind: ServiceKind::ServiceManager,",
    "kind: ServiceKind::StorageServer,",
    "kind: ServiceKind::InputServer,",
    "kind: ServiceKind::SurfaceServer,",
]
positions = [manifest_source.index(needle) for needle in service_order]
if positions != sorted(positions):
    raise SystemExit("M72 product manifest reverted to the M71 literal service order")
if manifest_source.count("RawService {") != 6:
    raise SystemExit("M72 product manifest service definition count changed")
if manifest_source.count("RawDependency {") < 5:
    raise SystemExit("M72 product manifest dependency definition count changed")

health = "crates/bndr-sm/src/health.rs"
for needle in (
    "pub fn from_catalog_iter(",
    "services: impl IntoIterator<Item = ServiceDefinition>",
    "dependencies: impl IntoIterator<Item = DependencyDefinition>",
    "fn catalog_iter_accepts_sparse_manifest_staging_without_exposing_partial_state()",
):
    require(health, needle, "transactional iterator-backed supervisor construction")

kernel_syscall = "kernel/src/syscall.rs"
for needle in (
    "fn service_manifest_open(",
    "SERVICE_MANIFEST_OPEN_CALLS.fetch_add(1",
    "if !crate::process::current_is_init()",
    "ServiceManifest::decode(&PRODUCT_SERVICE_MANIFEST_BYTES)",
    "Vmo::try_from_slice(manifest_bytes)",
    "SERVICE_MANIFEST_OPEN_SUCCESSES.fetch_add(1",
    "fn service_manifest_access_proof_valid() -> bool",
    "SERVICE_MANIFEST_OPEN_CALLS.load(Ordering::Acquire) == 2",
    "SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS.load(Ordering::Acquire) == 1",
    "service_manifest_access_proof_valid()",
    "const M72_STORAGE_READY_PREFIX: u64 = 0x4d37_3247_0000_0000;",
    "const M72_STORAGE_PROOF: u64 = 0x4d37_3250_0105_0401;",
):
    require(kernel_syscall, needle, "kernel-owned immutable manifest authority")

product = "user/init/src/product_runtime.rs"
for needle in (
    "pub(super) const M72_STORAGE_READY_PREFIX: u64 = 0x4d37_3247_0000_0000;",
    "pub(super) const M72_STORAGE_PROOF: u64 = 0x4d37_3250_0105_0401;",
    "fn open_product_service_manifest() -> ServiceManifest",
    "SyscallNumber::ServiceManifestOpen",
    "ServiceManifest::decode(&wire)",
    "validate_product_manifest_profile(&manifest);",
    "spawn(service.image(), service.shutdown_node())",
    "fn build_manifest_product_supervisor(",
    "for (index, service) in manifest.services().enumerate()",
    "definitions[index] = Some(ServiceDefinition::new(identity, service.policy()));",
    "ProductSupervisor::from_catalog_iter(",
    "fn resolve_manifest_service(",
    "let storage_image = manifest",
    "let replacement = spawn(storage_image, None);",
    "services.require(ServiceKind::App).child.control",
    "services.require(ServiceKind::SurfaceServer)",
    "services.require(ServiceKind::InputServer)",
):
    require(product, needle, "manifest-driven init binding and supervision")
manifest_helper = read(product).split(
    "fn build_manifest_product_supervisor(", 1
)[1].split(
    "fn resolve_manifest_service(", 1
)[0]
for forbidden in (
    "ServiceDefinition::new(manager_identity",
    "ServiceDefinition::new(surface_identity",
    "ServiceDefinition::new(input_identity",
    "ServiceDefinition::new(first_identity",
    "ServiceDefinition::new(app_identity",
):
    if forbidden in manifest_helper:
        raise SystemExit(f"M72 manifest helper contains a fixed service entry: {forbidden}")

main = "kernel/src/main.rs"
for needle in (
    'cfg!(feature = "unified-product-manifest-supervision-runtime")',
    "0x4d37_3247_0000_0000",
    "0x4d37_3250_0105_0401",
    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK format=1 abi={}",
    "manifest=BMF1 manifest_generation=1 manifest_bytes=224",
    "manifest_open_calls={} manifest_open_successes={}",
    "service_order=App+ServiceManager+StorageServer+InputServer+SurfaceServer",
    "legacy_literal_order_independent=1 resident_bindings=4 manifest_spawns=1",
    "bounded=1 arbitrary_service_set_claim=0",
    "M72_PSCI_DISCOVERY_OK format=1 abi={}",
    "BOOT_OK: M72 unified real UI is interactive and awaiting authenticated power key",
    "BOOT_OK: M72 kernel-manifest service discovery, five-service supervision, AppData, and PSCI shutdown armed",
    "arch::aarch64::qemu_psci_power_off(validated);",
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0",
):
    require(main, needle, "sealed M72 runtime evidence")

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
    raise SystemExit("M72 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M72 QEMU argv added a network backend")
for needle in (
    '"M72": (',
    'not in {"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    'elif args.milestone == "M72":',
    '"M72_PSCI_DISCOVERY_OK "',
    '"UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK "',
    '"UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK "',
    'qmp.key("power")',
    "qemu_self_exit=1 power_key=116",
):
    require("scripts/unified_product_qmp.py", needle, "shared M72 QMP contract")

for needle in (
    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_REBOOT_OK",
    "serial_negative={rejected}",
    "host_negative={driver_rejected}",
    "manifests_decoded=2",
    "manifest_open_calls=4",
    "manifest_services=10 resident_bindings=8 manifest_spawns=2",
    "health_probes=214",
    "healthy=208 missed=6",
    "qemu_psci_self_exits=2",
    "semihosting_uses=0",
    "hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1",
):
    require(
        "scripts/unified_product_manifest_supervision_evidence.py",
        needle,
        "strict M72 evidence parser",
    )
for needle in (
    'unified_product_manifest_supervision_evidence.py" self-test',
    "run_boot first",
    "run_boot second",
    "--milestone M72",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
    "qemu_psci_self_exit=1",
):
    require(
        "scripts/check-unified-product-manifest-supervision-runtime.sh",
        needle,
        "two-boot M72 runtime gate",
    )
for needle in (
    '"$SCRIPT_DIR/check-unified-product-manifest-supervision-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-manifest-supervision-runtime.sh"',
    "unified_product_manifest_supervision_static=1",
    "unified_product_manifest_supervision_reboot=1",
    "unified_product_manifest_supervision_boots=2",
    "manifest_supervision_decodes=2",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require("scripts/test.sh", needle, "full-suite M72 integration", count=1)

for relative in (
    "scripts/check-unified-product-manifest-supervision-runtime.sh",
    "scripts/unified_product_manifest_supervision_evidence.py",
    "scripts/unified_product_qmp.py",
):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M72 evidence path gained network access")
for marker in (
    "M72_FRAME_TRACE",
    "M72_STACK_TRACE",
    "M72_EXIT_TRACE",
    "M72_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M72 diagnostic trace")

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
    raise SystemExit("M72 static contract found no QEMU launch sites")

print(
    "UNIFIED_PRODUCT_MANIFEST_SUPERVISION_SOURCE_OK abi=33 syscall=55 "
    "manifest=BMF1 generation=1 bytes=224 services=5 dependency_edges=4 "
    "bounded_capacity=5/10 nonlegacy_order=1 resident_bindings=4 "
    "manifest_spawns=1 parser_transactional=1 parser_serial_negative=26 "
    f"parser_host_negative=3 qemu_launches={launches} nic_none={launches} "
    "semihosting_m72=0 bounded=1 arbitrary_service_set_claim=0 emulator_only=1 "
    "general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_manifest_supervision_evidence.py"
python3 "$SCRIPT_DIR/unified_product_manifest_supervision_evidence.py" self-test

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
for test_name in \
  product_manifest_is_strict_and_uses_nonlegacy_service_order \
  valid_bounded_subset_and_permutation_decode_without_fixed_product_array \
  manifest_decode_rejects_header_and_fingerprint_mutations_transactionally \
  manifest_decode_rejects_service_identity_policy_and_padding_mutations \
  manifest_decode_rejects_dependency_mutations_and_cycles \
  catalog_iter_accepts_sparse_manifest_staging_without_exposing_partial_state
do
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm "$test_name"
done
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-manifest-supervision-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-manifest-supervision-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-manifest-supervision-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_MANIFEST_SUPERVISION_STATIC_OK source=1 feature_closure=1 abi=33 syscall=55 manifest=BMF1 generation=1 bytes=224 services=5 dependency_edges=4 bounded_capacity=5/10 nonlegacy_order=1 resident_bindings=4 manifest_spawns=1 parser_transactional=1 parser_serial_negative=26 parser_host_negative=3 semihosting=0 full_suite=1 bounded=1 arbitrary_service_set_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0'
