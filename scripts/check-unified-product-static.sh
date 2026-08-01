#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M67 unified product static contract." >&2
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
    source = read(relative)
    observed = source.count(needle)
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
if abi_features.get("unified-product-runtime") != ["resident-platform-shutdown-runtime"]:
    raise SystemExit("ABI M67 feature must extend exactly the M66 contract")
for relative, selected in (
    ("user/init/Cargo.toml", user_features),
    ("kernel/Cargo.toml", kernel_features),
):
    if selected.get("unified-product-runtime") != [
        "resident-platform-shutdown-runtime",
        "bndr-abi/unified-product-runtime",
    ]:
        raise SystemExit(f"{relative}: M67 feature closure changed")

for needle in (
    'feature = "unified-product-runtime",',
    'not(feature = "unified-product-liveness-runtime")',
    "pub const ABI_VERSION: u64 = 28;",
    "assert_eq!(ABI_VERSION, 28);",
):
    require("crates/bndr-abi/src/lib.rs", needle, "ABI v28 gate")

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-runtime"',
    "KERNEL_UNIFIED_PRODUCT_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-runtime"',
    "USER_UNIFIED_PRODUCT_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-runtime"',
):
    require("scripts/build-kernel.sh", needle, "matched M67 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_RUNTIME",
    'feature=\\"unified-product-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M67 build.rs forwarding")

for relative, needle, label in (
    ("user/init/src/main.rs", "mod product_runtime;", "product runtime module"),
    (
        "user/init/src/main.rs",
        "product_runtime::init_runtime(startup_handle);",
        "product init entry",
    ),
    (
        "user/init/src/main.rs",
        "if product_runtime::dispatch_control(transport, &envelope)",
        "terminal control interception",
    ),
    (
        "user/init/src/lifecycle_runtime.rs",
        "super::product_runtime::complete_runtime(children, surface, launcher, app)",
        "M45-to-M67 handoff",
    ),
    (
        "user/init/src/multi_window_runtime.rs",
        "let _unexpected = read_channel_envelope(lifecycle.raw());",
        "App steady-state product handoff",
    ),
    (
        "user/init/src/input_server_runtime.rs",
        "if code == 116",
        "physical power-key route",
    ),
    (
        "user/init/src/input_server_runtime.rs",
        "super::product_runtime::request_power_key(sequence, value);",
        "authenticated power request",
    ),
):
    require(relative, needle, label)

product = "user/init/src/product_runtime.rs"
for needle in (
    "pub(super) const M67_STORAGE_READY_PREFIX: u64 = 0x4d36_3747_0000_0000;",
    "pub(super) const M67_STORAGE_PROOF: u64 = 0x4d36_3750_080f_0301;",
    "pub(super) fn dispatch_control(",
    "pub(super) fn request_power_key(",
    "pub(super) fn complete_runtime(",
    "wait_for_power_request(&nodes);",
    "let server = spawn(UserImageId::StorageServer, None);",
    "PRODUCT_COMMAND_STORAGE_WORK",
    "AppDataPrincipal::LAUNCHER",
    "AppDataPrincipal::PRIMARY_APP",
    "SYSTEM_SHUTDOWN_PREPARE",
    "SYSTEM_SHUTDOWN_COMMIT",
    "PRODUCT_COMMAND_PROBE_QUIESCE",
    "SHUTDOWN_SERVICE_ALL_MASK",
    "exercise_spawn_barrier();",
    "wait_for_exit(server, SERVER_SHUTDOWN_EXIT_CODE);",
):
    require(product, needle, "M67 unified product invariant")

for relative, needle, label in (
    ("kernel/src/lib.rs", "pub mod unified_product;", "kernel UI seal export"),
    (
        "kernel/src/unified_product.rs",
        "compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)",
        "one-shot kernel UI seal",
    ),
    (
        "kernel/src/main.rs",
        "bndroid_kernel::unified_product::publish_ui_converged()",
        "kernel-owned UI convergence",
    ),
    (
        "kernel/src/main.rs",
        "validate_persistent_window_runtime(Some((&storage_boot, interrupt_info.timer_frequency_hz)))",
        "unified UI-to-shutdown validator",
    ),
    (
        "kernel/src/main.rs",
        '"incompatible unified product feature set"',
        "fail-closed synthetic-runtime conflict gate",
    ),
    (
        "kernel/src/main.rs",
        "UNIFIED_PRODUCT_UI_OK format=1 abi={}",
        "interactive product evidence",
    ),
    (
        "kernel/src/main.rs",
        "UNIFIED_PRODUCT_SHUTDOWN_OK format=1 abi={}",
        "bounded shutdown evidence",
    ),
    (
        "kernel/src/main.rs",
        "BOOT_OK: M67 interactive product, AppData, bounded shutdown, and QEMU exit armed",
        "terminal M67 marker",
    ),
    (
        "kernel/src/limits.rs",
        "pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 9;",
        "nine-child product capacity",
    ),
    (
        "kernel/src/kernel_heap.rs",
        "pub const HEAP_PAGES: usize = 256;",
        "M67 one-MiB heap bound",
    ),
    (
        "kernel/src/scheduler.rs",
        "const WORKER_STACK_SIZE: usize = 64 * 1024;",
        "M67 storage-safe exception stack",
    ),
):
    require(relative, needle, label)

for relative in (
    "kernel/src/main.rs",
    "scripts/unified_product_evidence.py",
):
    require(relative, "real_phone_claim=0", "explicit non-phone claim")
for marker in (
    "M67_FRAME_TRACE",
    "M67_STACK_TRACE",
    "M67_EXIT_TRACE",
    "M67_SYSCALL_TRACE",
):
    for relative in ("kernel/src/scheduler.rs", "kernel/src/syscall.rs"):
        forbid(relative, marker, "temporary M67 diagnostic trace")

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
    raise SystemExit("unified_product_qmp.py must contain exactly one QEMU Popen argv")
argv = [
    element.value if isinstance(element, ast.Constant) and isinstance(element.value, str) else None
    for element in popen_lists[0]
]
nic_indices = [index for index, value in enumerate(argv) if value == "-nic"]
if nic_indices != [argv.index("-nic")] or len(nic_indices) != 1:
    raise SystemExit("M67 QEMU argv must contain exactly one -nic")
nic_index = nic_indices[0]
if nic_index + 1 >= len(argv) or argv[nic_index + 1] != "none":
    raise SystemExit("M67 QEMU argv must pair its only -nic with none")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M67 QEMU argv added a network backend")

for needle in (
    "FINAL_SCREEN_SHA256 =",
    "qmp.key(\"power\")",
    "qemu_self_exit=1 power_key=116",
    "process.terminate()",
    "if process.poll() is None:",
):
    require("scripts/unified_product_qmp.py", needle, "QMP interaction/cleanup contract")
require(
    "scripts/unified_product_qmp.py",
    "f\"ui_sha256={digest}{maintenance}\"",
    "exact screenshot evidence",
)

for needle in (
    "UNIFIED_PRODUCT_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_REBOOT_OK",
    "outside_data_appdata_unchanged=1",
    "real_phone_claim=0",
):
    require("scripts/unified_product_evidence.py", needle, "strict M67 evidence parser")
for needle in (
    "unified_product_evidence.py\" self-test",
    "run_boot first",
    "run_boot second",
    "BNDROID_KERNEL_FEATURES=\"$FEATURE\"",
    "BNDROID_USERSPACE_FEATURES=\"$FEATURE\"",
):
    require("scripts/check-unified-product-runtime.sh", needle, "two-boot runtime gate")

for needle in (
    '"$SCRIPT_DIR/check-unified-product-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-runtime.sh"',
    "unified_product_static=1",
    "unified_product_reboot=1",
    "unified_product_boots=2",
    "unified_product_ui_interactions=2",
):
    require("scripts/test.sh", needle, "full-suite M67 integration", count=1)

print(
    "UNIFIED_PRODUCT_SOURCE_OK abi=28 ui=m45-real-ui power_key=116 "
    "appdata=1 resident_nodes=8 dynamic_capacity=9 heap_pages=256 "
    "exception_stack_kib=64 qemu_nic_none=1 real_phone_claim=0"
)
PY

bash -n \
  "$SCRIPT_DIR/check-unified-product-static.sh" \
  "$SCRIPT_DIR/check-unified-product-runtime.sh"
python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_evidence.py"
python3 "$SCRIPT_DIR/unified_product_evidence.py" self-test
cargo check --locked -p bndr-abi --features unified-product-runtime
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-runtime
cargo check --locked -p bndroid-kernel --features unified-product-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_STATIC_OK source=1 feature_closure=1 abi=28 ui=real-m45 input=physical-qmp power_key=116 appdata=1 shutdown_graph=8 qemu_self_exit=1 parser_negative_cases=6 full_suite=1 emulator_only=1 real_phone_claim=0'
