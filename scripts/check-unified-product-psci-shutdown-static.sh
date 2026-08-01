#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M70 FDT/PSCI contracts." >&2
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
        raise SystemExit(f"M70 contract source is missing: {relative}")
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


feature = "unified-product-psci-shutdown-runtime"
parent = "unified-product-multiservice-liveness-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M70 feature must extend exactly the M69 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M70 feature closure changed")

for needle in (
    'feature = "unified-product-psci-shutdown-runtime",',
    'not(feature = "unified-product-continuous-supervision-runtime")',
    "pub const ABI_VERSION: u64 = 31;",
    "assert_eq!(ABI_VERSION, 31);",
    "ServiceShutdown = 54,",
    "assert_eq!(SyscallNumber::from_raw(55), None);",
):
    require("crates/bndr-abi/src/lib.rs", needle, "ABI v31/no-new-syscall contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 31] = [(); ABI_VERSION as usize];",
    "userspace ABI v31 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-psci-shutdown-runtime"',
    "KERNEL_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-psci-shutdown-runtime"',
    "USER_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-psci-shutdown-runtime"',
    "userspace PSCI-shutdown profile requires the matching kernel profile.",
):
    require("scripts/build-kernel.sh", needle, "matched M70 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME",
    'feature=\\"unified-product-psci-shutdown-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M70 build.rs forwarding")

for relative, ready, proof in (
    (
        "user/init/src/product_runtime.rs",
        "pub(super) const M70_STORAGE_READY_PREFIX: u64 = 0x4d37_3047_0000_0000;",
        "pub(super) const M70_STORAGE_PROOF: u64 = 0x4d37_3050_0807_0201;",
    ),
    (
        "kernel/src/syscall.rs",
        "const M70_STORAGE_READY_PREFIX: u64 = 0x4d37_3047_0000_0000;",
        "const M70_STORAGE_PROOF: u64 = 0x4d37_3050_0807_0201;",
    ),
):
    require(relative, ready, "M70 ready identity", count=1)
    require(relative, proof, "M70 proof identity", count=1)
for needle in (
    'cfg!(feature = "unified-product-psci-shutdown-runtime")',
    "0x4d37_3047_0000_0000",
    "0x4d37_3050_0807_0201",
):
    require("kernel/src/main.rs", needle, "M70 kernel seal identity")

fdt = "kernel/src/fdt.rs"
for needle in (
    "pub enum PsciMethod",
    "Hvc = 1,",
    "Smc = 2,",
    "pub enum PsciCompatibleVersion",
    "pub struct PsciInfo",
    "MissingPsciNode",
    "MultiplePsciNodes",
    "InvalidPsciCompatible",
    "InvalidPsciStatus",
    "MissingPsciMethod",
    "InvalidPsciMethod",
    "DuplicatePsciProperty",
    "pub fn psci(&self) -> Result<PsciInfo, FdtError>",
    'depth == 1 && bytes_equal(name, b"psci")',
    'string_list_contains(value, b"arm,psci-1.0")',
    'string_list_contains(value, b"arm,psci-0.2")',
    'dt_string_equals(value, b"hvc")',
    'dt_string_equals(value, b"smc")',
    "discovers_strict_qemu_psci_hvc_profile",
    "accepts_enabled_psci_v0_2_smc_profile",
    "psci_discovery_rejects_missing_nested_and_duplicate_nodes",
    "psci_discovery_rejects_legacy_or_malformed_compatible_lists",
    "psci_discovery_rejects_disabled_malformed_or_duplicate_status",
    "psci_discovery_rejects_missing_invalid_or_duplicate_method",
    "psci_discovery_rejects_duplicate_compatible_property",
):
    require(fdt, needle, "strict allocation-free FDT/PSCI discovery")

contract = "kernel/src/platform_shutdown.rs"
for needle in (
    "QemuPsci = 2,",
    "pub struct PsciDescriptor",
    "pub compatible: PsciCompatibleVersion,",
    "pub const fn psci_version_supported(version: u32) -> bool",
    "static INSTALLED_PSCI: AtomicU64",
    "pub fn install_psci(",
    "pub fn installed_psci() -> Option<PsciDescriptor>",
    "UnexpectedPsciDescriptor",
    "MissingPsciDescriptor",
    "UnsupportedPsciVersion",
    "(Backend::QemuPsci, Some(descriptor))",
    "complete_contract_binds_the_probed_psci_descriptor",
    "psci_version_and_descriptor_encoding_are_bounded",
):
    require(contract, needle, "opaque fail-closed PSCI shutdown contract")

arch = "kernel/src/arch/aarch64/mod.rs"
for needle in (
    "pub fn psci_version(",
    "const PSCI_VERSION: u64 = 0x8400_0000;",
    "pub fn qemu_psci_power_off(",
    "const PSCI_SYSTEM_OFF: u64 = 0x8400_0008;",
    'asm!("dsb sy", "isb"',
    '"hvc #0"',
    '"smc #0"',
    'clobber_abi("C")',
    "installed_psci() != Some(descriptor)",
    'panic!("M70 QEMU PSCI SYSTEM_OFF returned unexpectedly")',
):
    require(arch, needle, "AArch64 PSCI conduit")

main = "kernel/src/main.rs"
for needle in (
    'fatal_boot_error("cannot validate /psci"',
    "arch::aarch64::psci_version(psci.method)",
    "platform_shutdown::install_psci(descriptor)",
    "M70_PSCI_DISCOVERY_OK format=1 abi={}",
    "Backend::QemuPsci",
    "psci: platform_psci,",
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK format=1 abi={}",
    '("qemu-psci", 1)',
    "emulator_backend={}",
    "psci_claim={}",
    "BOOT_OK: M70 unified real UI is interactive and awaiting authenticated power key",
    "BOOT_OK: M70 FDT-validated QEMU PSCI SYSTEM_OFF, two-service liveness, AppData, and bounded shutdown armed",
    "arch::aarch64::qemu_psci_power_off(validated);",
    "hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0",
):
    require(main, needle, "terminal M70 runtime evidence")

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
    raise SystemExit("M70 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M70 QEMU argv added a network backend")
if qmp_source.count('"-semihosting-config"') != 1:
    raise SystemExit("shared QMP semihosting option count changed")
for needle in (
    '"M70": (',
    'not in {"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    '"M70_PSCI_DISCOVERY_OK "',
    '"UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK "',
    'qmp.key("power")',
    "qemu_self_exit=1 power_key=116",
):
    require("scripts/unified_product_qmp.py", needle, "shared M70 QMP contract")

for needle in (
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_REBOOT_OK",
    "serial_negative={rejected}",
    "host_negative={driver_rejected}",
    "psci_discoveries=2",
    "psci_version_probes=2",
    "psci_system_off_requests=2",
    "qemu_psci_self_exits=2",
    "semihosting_uses=0",
    "hardware_poweroff_claim=0 pmic_claim=0",
    "real_phone_claim=0",
):
    require(
        "scripts/unified_product_psci_shutdown_evidence.py",
        needle,
        "strict M70 evidence parser",
    )
for needle in (
    'unified_product_psci_shutdown_evidence.py" self-test',
    "run_boot first",
    "run_boot second",
    "--milestone M70",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
    "qemu_psci_self_exit=1",
):
    require(
        "scripts/check-unified-product-psci-shutdown-runtime.sh",
        needle,
        "two-boot M70 runtime gate",
    )
for needle in (
    '"$SCRIPT_DIR/check-unified-product-psci-shutdown-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-psci-shutdown-runtime.sh"',
    "unified_product_psci_shutdown_static=1",
    "unified_product_psci_shutdown_reboot=1",
    "unified_product_psci_shutdown_boots=2",
    "qemu_psci_self_exits=57",
):
    require("scripts/test.sh", needle, "full-suite M70 integration", count=1)

for relative in (
    "scripts/check-unified-product-psci-shutdown-runtime.sh",
    "scripts/unified_product_psci_shutdown_evidence.py",
    "scripts/unified_product_qmp.py",
):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M70 evidence path gained network access")
for marker in (
    "M70_FRAME_TRACE",
    "M70_STACK_TRACE",
    "M70_EXIT_TRACE",
    "M70_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M70 diagnostic trace")

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
    raise SystemExit("M70 static contract found no QEMU launch sites")

print(
    "UNIFIED_PRODUCT_PSCI_SHUTDOWN_SOURCE_OK abi=31 fdt_node=/psci "
    "compatible=arm,psci-1.0 method=hvc psci_version=1.1 "
    "version_raw=0x00010001 version_function_id=0x84000000 "
    "system_off_function_id=0x84000008 fdt_negative_classes=7 "
    "contract_negative_classes=4 parser_serial_negative=13 "
    "parser_host_negative=3 "
    f"qemu_launches={launches} nic_none={launches} semihosting_m70=0 "
    "no_new_syscall=1 emulator_only=1 hardware_poweroff_claim=0 "
    "pmic_claim=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_psci_shutdown_evidence.py"
python3 "$SCRIPT_DIR/unified_product_psci_shutdown_evidence.py" self-test

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-psci-shutdown-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-psci-shutdown-runtime \
  fdt::tests:: --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-psci-shutdown-runtime \
  platform_shutdown::tests:: --lib
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-psci-shutdown-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-psci-shutdown-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_PSCI_SHUTDOWN_STATIC_OK source=1 feature_closure=1 abi=31 fdt_strict=1 psci_version_probe=1 system_off=1 opaque_seal=1 two_boot=1 parser_serial_negative=13 parser_host_negative=3 no_new_syscall=1 semihosting=0 full_suite=1 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0'
