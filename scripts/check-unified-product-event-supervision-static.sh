#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M73 event-supervision contracts." >&2
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
        raise SystemExit(f"M73 contract source is missing: {relative}")
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


feature = "unified-product-event-supervision-runtime"
parent = "unified-product-manifest-supervision-runtime"
if features("crates/bndr-abi/Cargo.toml").get(feature) != [parent]:
    raise SystemExit("ABI M73 feature must extend exactly the M72 contract")
for relative in ("user/init/Cargo.toml", "kernel/Cargo.toml"):
    if features(relative).get(feature) != [parent, f"bndr-abi/{feature}"]:
        raise SystemExit(f"{relative}: M73 feature closure changed")

abi = "crates/bndr-abi/src/lib.rs"
for needle in (
    '#[cfg(feature = "unified-product-event-supervision-runtime")]',
    "pub const ABI_VERSION: u64 = 34;",
    "pub const SERVICE_SUPERVISOR_REPORT_FLAGS_NONE: u64 = 0;",
    "pub const SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY: u64 = 0;",
    "pub const SERVICE_SUPERVISOR_REPORT_ACTIVE: u64 = 1;",
    "pub const SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN: u64 = 2;",
    "pub const SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE: u64 = 3;",
    "pub const SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW: u64 = 4;",
    "pub const SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED: u64 = 5;",
    "pub const SERVICE_SUPERVISOR_REPORT_STOP_DRAINED: u64 = 6;",
    "ServiceSupervisorReport = 56,",
    "56 => Some(Self::ServiceSupervisorReport),",
    "assert_eq!(ABI_VERSION, 34);",
    "assert_eq!(SyscallNumber::from_raw(57), None);",
    "returns [`Status::ShouldWait`] until the",
    "The call grants no",
):
    require(abi, needle, "ABI v34 init-only event-report contract")
require(
    "user/init/src/main.rs",
    "const _: [(); 34] = [(); ABI_VERSION as usize];",
    "userspace ABI v34 compile-time assertion",
    count=1,
)

for needle in (
    'feature_list_contains "$KERNEL_FEATURES" "unified-product-event-supervision-runtime"',
    "KERNEL_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "unified-product-event-supervision-runtime"',
    "USER_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME=1",
    'append_feature "$USERSPACE_FEATURES" "unified-product-event-supervision-runtime"',
    "userspace event-supervision profile requires the matching kernel profile.",
):
    require("scripts/build-kernel.sh", needle, "matched M73 build forwarding")
for needle in (
    "CARGO_FEATURE_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME",
    'feature=\\"unified-product-event-supervision-runtime\\"',
):
    require("kernel/build.rs", needle, "direct M73 build.rs forwarding")

syscall = "kernel/src/syscall.rs"
for needle in (
    "fn service_supervisor_report(",
    "if !crate::process::current_is_init()",
    "SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY",
    "if argument1 != 0 || argument2 != 0",
    "bndroid_kernel::unified_product::ui_converged()",
    "event_supervision_trace::record_ui_query(converged)",
    "Status::ShouldWait",
    "unique_live_process_id_for_image(UserImageId::StorageServer)",
    "event_supervision_trace::report(operation, argument1, argument2, context)",
    "SyscallNumber::ServiceSupervisorReport => {",
):
    require(syscall, needle, "kernel-authenticated M73 report syscall")
syscall_body = read(syscall).split("fn service_supervisor_report(", 1)[1].split(
    "\nfn file_open_at(", 1
)[0]
for forbidden in ("with_table(", ".insert(", "ProcessTerminate", "process_terminate("):
    if forbidden in syscall_body:
        raise SystemExit(
            f"M73 read/report syscall grants authority or terminates a process: {forbidden}"
        )

trace = "kernel/src/event_supervision_trace.rs"
for needle in (
    "static UI_QUERY_CALLS: AtomicU64",
    "static UI_QUERY_WAITS: AtomicU64",
    "static UI_QUERY_SUCCESSES: AtomicU64",
    "pub fn record_ui_query(converged: bool)",
    "ui_query_calls == ui_query_waits + ui_query_successes",
    "calls == 9",
    "successes == 8",
    "argument_rejections == 1",
    "rotations_started == REQUIRED_ROTATIONS",
    "rotations_completed == REQUIRED_ROTATIONS",
    "rotation_new_pids[0] == rotation_old_pids[1]",
    "cancel_pending == REQUIRED_CANCEL_PENDING",
    "stop_batch == cancel_batch",
    "stop_pending == cancel_pending",
    "old_pid != context.current_storage_pid",
    "!next_generation_same_slot(old_pid, new_pid)",
    "context.terminated_killed == 0",
    "exact_two_rotation_and_inflight_cancel_transcript_completes",
    "wrong_process_ledger_or_generation_is_fail_closed",
    "static TEST_LOCK: Mutex<()>",
):
    require(trace, needle, "kernel-owned M73 transition ledger")

health = "crates/bndr-sm/src/health.rs"
for needle in (
    "pub restart_budget_rearms: u32,",
    "pub fn rearm_restart_budget(",
    "RecoveryStabilityPending {",
    "RestartBudgetNotConsumed",
    "record.restarts_used = 0;",
    "stable_replacements_rearm_one_window_without_weakening_each_budget",
):
    require(health, needle, "stable replacement restart-budget rearm")

product = "user/init/src/product_runtime.rs"
for needle in (
    "wait_for_kernel_ui_convergence(&nodes);",
    "fn wait_for_kernel_ui_convergence(nodes: &[Child; 8])",
    "SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY",
    "Status::ShouldWait.raw()",
    "object_wait_many_array(&items, nodes.len(), PRODUCT_HEALTH_CADENCE_NS)",
    "fn run_event_supervision(",
    "for ordinal in 1..=3",
    "for expected_ordinal in 1..=2",
    "m73_wait_for_action(&services, PRODUCT_HEALTH_CADENCE_NS)",
    "m73_arm_and_send_batch(&mut supervisor, &services, now_ns)",
    "m73_wait_for_input_action(input.child, OBJECT_WAIT_TIMEOUT_INFINITE)",
    "SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW",
    "SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED",
    "SERVICE_SUPERVISOR_REPORT_STOP_DRAINED",
    "write_scalar(\n        previous.control,\n        SERVER_SHUTDOWN_COMMAND_TAG,",
    "wait_for_exit(previous, SERVER_SHUTDOWN_EXIT_CODE);",
    "classify_fault(previous_identity, FaultClass::ProcessExit)",
    "require_next_process_generation(previous.pid, replacement.pid);",
    ".rearm_restart_budget(replacement_identity)",
    "snapshot.restart_budget_rearms != ordinal as u32",
    "snapshot.total_missed_probes != 0",
):
    require(product, needle, "open-ended event loop and clean service rotation")
rotation_body = read(product).split("fn m73_rotate_storage(", 1)[1].split(
    "\n#[cfg(feature = \"unified-product-event-supervision-runtime\")]\nfn m73_run_healthy_batch",
    1,
)[0]
for forbidden in (
    "SyscallNumber::ProcessTerminate",
    "process_terminate",
    "PRODUCT_COMMAND_HEALTH_MISS",
    "FaultClass::Unresponsive",
):
    if forbidden in rotation_body:
        raise SystemExit(f"M73 rotation contains an injected/kill shortcut: {forbidden}")

input_server = "user/init/src/input_server_runtime.rs"
for needle in (
    "runtime.process_control_message()",
    "fn process_control_message(&mut self)",
    "super::read_channel_envelope_once(self.control.raw())",
    "super::product_runtime::dispatch_control(self.control.raw(), &envelope)",
    "self.replace_surface_route_from_envelope(envelope);",
):
    require(input_server, needle, "single-envelope InputServer control dispatch")
control_body = read(input_server).split("fn process_control_message(&mut self)", 1)[1].split(
    "\n    fn replace_surface_route_from_envelope", 1
)[0]
if "read_channel_envelope_now" in control_body:
    raise SystemExit("M73 InputServer control dispatch can still block on a second envelope")

main = "kernel/src/main.rs"
for needle in (
    'cfg!(feature = "unified-product-event-supervision-runtime")',
    "event_supervision.ui_query_calls",
    "event_supervision.complete",
    "M73_EVENT_SUPERVISOR_ACTIVE_OK format=1 abi={}",
    "M73_STORAGE_ROTATION_OK format=1 abi={}",
    "M73_CANCEL_WINDOW_OK format=1 abi={}",
    "UNIFIED_PRODUCT_EVENT_SUPERVISION_OK format=1 abi={}",
    "clean_rotations=2",
    "process_terminate_calls={}",
    "drained_pending=0",
    "injected_health_faults=0 injected_process_kills=0",
    "M73_PSCI_DISCOVERY_OK format=1 abi={}",
    "BOOT_OK: M73 unified real UI is interactive; event supervision and external maintenance controls are starting",
    "BOOT_OK: M73 event-driven five-service supervision, two clean storage rotations, in-flight drain, AppData, and PSCI shutdown armed",
    "arch::aarch64::qemu_psci_power_off(validated);",
    "real_phone_claim=0",
):
    require(main, needle, "sealed M73 kernel evidence")

qmp_relative = "scripts/unified_product_qmp.py"
qmp_source = read(qmp_relative)
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
    raise SystemExit("M73 QEMU argv must contain exactly one '-nic none'")
if any(value in {"-net", "-netdev"} for value in argv):
    raise SystemExit("M73 QEMU argv added a network backend")
for needle in (
    '"M73": (',
    'not in {"M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81"}',
    'if args.milestone == "M73":',
    '"M73_EVENT_SUPERVISOR_ACTIVE_OK"',
    '"M73_STORAGE_ROTATION_OK"',
    '"M73_CANCEL_WINDOW_OK"',
    '"UNIFIED_PRODUCT_EVENT_SUPERVISION_OK "',
    'qmp.key("f5")',
    'qmp.key("power")',
    "maintenance_rotations=2 cancel_pending=4",
):
    require(qmp_relative, needle, "shared M73 QMP maintenance contract")
require(qmp_relative, 'qmp.key("f5")', "two M73 F5 maintenance requests", count=2)

evidence = "scripts/unified_product_event_supervision_evidence.py"
for needle in (
    "UNIFIED_PRODUCT_EVENT_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK",
    "UNIFIED_PRODUCT_EVENT_SUPERVISION_REBOOT_OK",
    "calls != waits + successes",
    "cancel_batch <= active_batches",
    "serial_negative={rejected}",
    "host_negative={driver_rejected}",
    "clean_rotations=4",
    "process_terminate_calls=0",
    "cancel_windows=2 cancelled_pending=8 drained_pending=0",
    "qemu_psci_self_exits=2",
    "real_phone_claim",
):
    require(evidence, needle, "strict M73 relational evidence parser")
runtime = "scripts/check-unified-product-event-supervision-runtime.sh"
for needle in (
    'unified_product_event_supervision_evidence.py" self-test',
    "run_boot first",
    "run_boot second",
    "--milestone M73",
    'BNDROID_KERNEL_FEATURES="$FEATURE"',
    'BNDROID_USERSPACE_FEATURES="$FEATURE"',
    "maintenance_rotations=2 cancel_pending=4",
    "qemu_psci_self_exit=1",
):
    require(runtime, needle, "two-boot M73 runtime gate")
for needle in (
    '"$SCRIPT_DIR/check-unified-product-event-supervision-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-event-supervision-runtime.sh"',
    "unified_product_event_supervision_static=1",
    "unified_product_event_supervision_reboot=1",
    "unified_product_event_supervision_boots=2",
    "event_supervision_clean_rotations=4",
    "event_supervision_cancelled_pending=8",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require("scripts/test.sh", needle, "full-suite M73 integration", count=1)

for relative in (runtime, evidence, qmp_relative):
    lowered = read(relative).lower()
    for forbidden in ("curl ", "wget ", "requests.", "urllib.request"):
        if forbidden in lowered:
            raise SystemExit(f"{relative}: M73 evidence path gained network access")
for marker in (
    "M73_F5_INPUT_SEEN",
    "M73_F5_STALL_SEEN",
    "M73_UNFOCUSED_KEY_INPUT_SEEN",
    "M73_UNFOCUSED_KEY_STALL_SEEN",
):
    for relative in (
        "kernel/src/scheduler.rs",
        "kernel/src/syscall.rs",
        "user/init/src/input_server_runtime.rs",
        "user/init/src/product_runtime.rs",
    ):
        forbid(relative, marker, "temporary M73 diagnostic trace")

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
    raise SystemExit("M73 static contract found no QEMU launch sites")

print(
    "UNIFIED_PRODUCT_EVENT_SUPERVISION_SOURCE_OK abi=34 syscall=56 "
    "ui_query=read-only event_driven=1 services=5 rotations=2 clean_exit=2 "
    "process_terminate_calls=0 restart_budget_rearms=2 cancel_pending=4 "
    "drained_pending=0 parser_serial_negative=26 parser_host_negative=4 "
    f"qemu_launches={launches} nic_none={launches} semihosting_m73=0 "
    "emulator_only=1 general_runtime=0 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_event_supervision_evidence.py"
python3 "$SCRIPT_DIR/unified_product_event_supervision_evidence.py" self-test

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm \
  stable_replacements_rearm_one_window_without_weakening_each_budget
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
  --features unified-product-event-supervision-runtime \
  syscall_numbers_are_stable_and_unknown_values_are_rejected
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
  --features unified-product-event-supervision-runtime \
  event_supervision_trace:: --lib
cargo check --locked -p bndroid-init \
  --target aarch64-unknown-none \
  --features unified-product-event-supervision-runtime
cargo check --locked -p bndroid-kernel \
  --target aarch64-unknown-none \
  --features unified-product-event-supervision-runtime

printf '%s\n' \
  'UNIFIED_PRODUCT_EVENT_SUPERVISION_STATIC_OK source=1 feature_closure=1 abi=34 syscall=56 ui_query=read-only event_driven=1 services=5 rotations=2 clean_exit=2 process_terminate_calls=0 restart_budget_rearms=2 cancel_pending=4 drained_pending=0 parser_serial_negative=26 parser_host_negative=4 semihosting=0 full_suite=1 emulator_only=1 general_runtime=0 real_phone_claim=0'
