#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm grep; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M66 resident-platform contracts." >&2
    exit 1
  fi
done

for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
feature = "resident-platform-shutdown-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M66 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M66 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M66 static contract requires one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M66 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M66 static contract lost ordered {description}: "
                f"{relative}: {needle!r}"
            )
        cursor = position + len(needle)


def feature_dependencies(text: str, name: str, relative: str) -> tuple[str, ...]:
    matches = list(
        re.finditer(
            rf"^{re.escape(name)}\s*=\s*\[(.*?)\]\s*$",
            text,
            flags=re.MULTILINE | re.DOTALL,
        )
    )
    if len(matches) != 1:
        raise SystemExit(f"Cargo feature is not exact and unique: {relative}:{name}")
    body = matches[0].group(1)
    dependencies = tuple(re.findall(r'"([^"]+)"', body))
    residue = re.sub(r'"[^"]+"\s*,?', "", body)
    if residue.strip():
        raise SystemExit(f"Cargo feature has an unparsed dependency: {relative}:{name}")
    return dependencies


relative_sources = (
    "crates/bndr-abi/Cargo.toml",
    "crates/bndr-abi/src/lib.rs",
    "kernel/Cargo.toml",
    "kernel/build.rs",
    "kernel/src/arch/aarch64/mod.rs",
    "kernel/src/kernel_heap.rs",
    "kernel/src/lib.rs",
    "kernel/src/limits.rs",
    "kernel/src/main.rs",
    "kernel/src/platform_shutdown.rs",
    "kernel/src/process.rs",
    "kernel/src/service_shutdown.rs",
    "kernel/src/syscall.rs",
    "scripts/build-kernel.sh",
    "scripts/check-resident-platform-shutdown-runtime.sh",
    "scripts/check-resident-platform-shutdown-static.sh",
    "scripts/test.sh",
    "user/init/Cargo.toml",
    "user/init/src/main.rs",
    "user/init/src/resident_shutdown_runtime.rs",
    "user/init/src/storage_server_runtime.rs",
)
texts = {relative: source(relative) for relative in relative_sources}

abi_cargo = texts["crates/bndr-abi/Cargo.toml"]
abi = texts["crates/bndr-abi/src/lib.rs"]
kernel_cargo = texts["kernel/Cargo.toml"]
build_rs = texts["kernel/build.rs"]
arch = texts["kernel/src/arch/aarch64/mod.rs"]
heap = texts["kernel/src/kernel_heap.rs"]
kernel_lib = texts["kernel/src/lib.rs"]
limits = texts["kernel/src/limits.rs"]
main = texts["kernel/src/main.rs"]
platform = texts["kernel/src/platform_shutdown.rs"]
process = texts["kernel/src/process.rs"]
services = texts["kernel/src/service_shutdown.rs"]
syscall = texts["kernel/src/syscall.rs"]
build = texts["scripts/build-kernel.sh"]
runtime = texts["scripts/check-resident-platform-shutdown-runtime.sh"]
test_sh = texts["scripts/test.sh"]
user_cargo = texts["user/init/Cargo.toml"]
user_main = texts["user/init/src/main.rs"]
resident = texts["user/init/src/resident_shutdown_runtime.rs"]
storage_runtime = texts["user/init/src/storage_server_runtime.rs"]

if feature_dependencies(abi_cargo, feature, "crates/bndr-abi/Cargo.toml") != (
    "storage-server-shutdown-orchestration-runtime",
):
    raise SystemExit("M66 ABI feature must depend on exactly the M65 ABI")
if feature_dependencies(kernel_cargo, feature, "kernel/Cargo.toml") != (
    "storage-server-shutdown-orchestration-runtime",
    "input-server-runtime",
    "bndr-abi/resident-platform-shutdown-runtime",
):
    raise SystemExit("M66 kernel feature closure changed")
if feature_dependencies(user_cargo, feature, "user/init/Cargo.toml") != (
    "storage-server-shutdown-orchestration-runtime",
    "input-server-runtime",
    "bndr-abi/resident-platform-shutdown-runtime",
):
    raise SystemExit("M66 userspace feature closure changed")

for needle in (
    'feature = "resident-platform-shutdown-runtime"',
    "pub const ABI_VERSION: u64 = 27;",
    "pub const SERVICE_SHUTDOWN_REGISTER: u64 = 1;",
    "pub const SERVICE_SHUTDOWN_QUIESCE: u64 = 2;",
    "pub const SHUTDOWN_SERVICE_NODE_COUNT: usize = 8;",
    "pub const SHUTDOWN_SERVICE_EDGE_COUNT: usize = 10;",
    "pub const SHUTDOWN_SERVICE_WAVE_COUNT: usize = 3;",
    "pub enum ShutdownServiceNode",
    "pub const fn dependency_mask(self) -> u64",
    "pub const fn dependent_mask(self) -> u64",
    "ServiceShutdown = 54,",
    "54 => Some(Self::ServiceShutdown),",
    "assert_eq!(SyscallNumber::from_raw(55), None);",
    "fn resident_shutdown_graph_is_stable_acyclic_and_complete()",
):
    require(abi, needle, "ABI v27 resident graph", "crates/bndr-abi/src/lib.rs")
require_once(
    user_main,
    "const _: [(); 27] = [(); ABI_VERSION as usize];",
    "userspace ABI v27 compile-time lock",
    "user/init/src/main.rs",
)

for needle in (
    "KERNEL_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME=0",
    "USER_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" "resident-platform-shutdown-runtime"',
    "KERNEL_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "resident-platform-shutdown-runtime"',
    "USER_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME=1",
    "userspace resident-platform-shutdown profile requires the matching kernel profile.",
    'append_feature "$USERSPACE_FEATURES" "resident-platform-shutdown-runtime"',
):
    require(build, needle, "matched kernel/userspace M66 closure", "scripts/build-kernel.sh")
for needle in (
    "CARGO_FEATURE_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME",
    'feature=\\"resident-platform-shutdown-runtime\\"',
    "resident_shutdown_runtime.rs",
):
    require(build_rs, needle, "direct M66 userspace forwarding", "kernel/build.rs")

for needle in (
    "pub mod platform_shutdown;",
    "pub mod service_shutdown;",
):
    require_once(kernel_lib, needle, "M66 kernel module export", "kernel/src/lib.rs")
for needle in (
    "pub const HEAP_PAGES: usize = 128;",
    "pub const HEAP_PAGES: usize = 64;",
):
    require(heap, needle, "profile-bounded resident heap", "kernel/src/kernel_heap.rs")
require(limits, "pub(crate) const DYNAMIC_PROCESS_CAPACITY: usize = 9;", "nine-child capacity", "kernel/src/limits.rs")

for needle in (
    "pub const fn reduce_register(",
    "pub const fn reduce_quiesce(",
    "node.dependent_mask() & !quiesced_mask != 0",
    "REGISTERED_MASK",
    "QUIESCED_MASK",
    "compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)",
    "fn all_nodes_register_only_with_exact_identity_and_dependencies()",
    "fn reverse_topological_quiesce_rejects_live_dependents()",
):
    require(services, needle, "authenticated reverse-order service ledger", "kernel/src/service_shutdown.rs")

for needle in (
    "pub enum Backend",
    "QemuSemihosting = 1",
    "pub struct Contract",
    "pub struct ValidatedShutdown",
    "pub const fn validate(contract: Contract)",
    "ContractError::ServiceGraphIncomplete",
    "ContractError::ProcessesLive",
    "ContractError::StorageAdmissionOpen",
    "ContractError::BlockIrqArmed",
    "ContractError::DurableSessionOpen",
    "fn every_platform_precondition_fails_closed()",
):
    require(platform, needle, "opaque fail-closed platform token", "kernel/src/platform_shutdown.rs")
require(
    platform,
    "pub struct ValidatedShutdown {\n    backend: Backend,\n    psci: Option<PsciDescriptor>,\n    generation: u64,\n}",
    "opaque private validated-token fields",
    "kernel/src/platform_shutdown.rs",
)

for needle in (
    "pub struct ResidentShutdownTopologySnapshot",
    "pub fn resident_shutdown_node_identity_valid(",
    "pub fn resident_shutdown_topology_snapshot()",
    "const M66_RESIDENT_ENDPOINT_COUNT: usize = 38;",
    "const M66_RESIDENT_TOTAL_HANDLE_COUNT: usize = 39;",
    "control_pairs == M66_RESIDENT_CONTROL_PAIR_COUNT",
    "dependency_pairs == SHUTDOWN_SERVICE_EDGE_COUNT",
    "scheduler::live_dynamic_user_context_count() == 9",
    "scheduler::live_dynamic_kernel_stack_count() == 9",
):
    require(process, needle, "kernel-inspected live graph topology", "kernel/src/process.rs")

for needle in (
    "SyscallNumber::ServiceShutdown => service_shutdown(frame, arg0, arg1, arg2)",
    "fn service_shutdown(",
    "resident_shutdown_node_identity_valid(pid, node)",
    "m66_shutdown_prepare_ready(generation)",
    "resident_shutdown_topology_snapshot()",
    "topology.control_pairs == 9",
    "topology.dependency_pairs == bndr_abi::SHUTDOWN_SERVICE_EDGE_COUNT",
    "services.calls == 18",
    "services.order_rejections == 1",
    "shutdown.connect_rejections == 2",
):
    require(syscall, needle, "M66 identity, topology, and phase gate", "kernel/src/syscall.rs")

for needle in (
    "const GRAPH_EDGES: [GraphEdge; SHUTDOWN_SERVICE_EDGE_COUNT]",
    "consumer: ShutdownServiceNode::Provider",
    "consumer: ShutdownServiceNode::PrimaryClient",
    "consumer: ShutdownServiceNode::SecondaryClient",
    "consumer: ShutdownServiceNode::InputServer",
    "consumer: ShutdownServiceNode::Launcher",
    "consumer: ShutdownServiceNode::App",
    "distribute_graph_edges(&children);",
    "SERVICE_SHUTDOWN_REGISTER",
    "GRAPH_COMMAND_PROBE_QUIESCE",
    "Status::InvalidState",
    "SERVICE_SHUTDOWN_QUIESCE",
    "resident_storage_admission_probe();",
    "SERVER_SHUTDOWN_COMMAND_TAG",
    "SERVER_SHUTDOWN_ACK_TAG",
    "M66_STORAGE_READY_PREFIX | final_generation",
    "SYSTEM_SHUTDOWN_COMMIT",
):
    require(resident, needle, "nine-process userspace resident graph", "user/init/src/resident_shutdown_runtime.rs")
ordered(
    resident,
    (
        "let prepared = syscall(",
        "exercise_spawn_barrier();",
        "GRAPH_COMMAND_PROBE_QUIESCE,",
        "ShutdownServiceNode::PrimaryClient,",
        "ShutdownServiceNode::Provider,",
        "ShutdownServiceNode::ServiceManager,",
        "SERVER_SHUTDOWN_COMMAND_TAG,",
        "SERVER_SHUTDOWN_ACK_TAG",
        "SyscallNumber::InitReady",
        "let committed = syscall(",
    ),
    "Prepare, reverse waves, StorageServer drain, and Commit",
    "user/init/src/resident_shutdown_runtime.rs",
)
for needle in (
    "pub(super) fn resident_client_work(",
    "pub(super) fn resident_storage_admission_probe()",
    "io.flush().unwrap_or_else",
    ".recover_with_policy(&mut io, &policy)",
    "write_scalar(startup, SERVER_SHUTDOWN_ACK_TAG, current_generation);",
    "exit_child(SERVER_SHUTDOWN_EXIT_CODE);",
):
    require(storage_runtime, needle, "real AppData and StorageServer drain", "user/init/src/storage_server_runtime.rs")

for needle in (
    "fn validate_resident_platform_shutdown_runtime(",
    "services.complete()",
    "storage_persist::close_verified_device_health(",
    "storage::seal_clean_shutdown_admission_masked",
    "interrupt::disable_block_irq_masked",
    "bndroid_kernel::shutdown::seal(appdata_generation)",
    "bndroid_kernel::platform_shutdown::validate(",
    "Backend::QemuSemihosting",
    "RESIDENT_PLATFORM_SHUTDOWN_OK format=1 abi={}",
    "BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed",
    "arch::aarch64::qemu_semihosting_power_off(validated);",
):
    require(main, needle, "durable close and platform boundary", "kernel/src/main.rs")
ordered(
    main,
    (
        "fn validate_resident_platform_shutdown_runtime(",
        "services.complete()",
        "storage_persist::close_verified_device_health(",
        "storage::seal_clean_shutdown_admission_masked",
        "interrupt::disable_block_irq_masked",
        "bndroid_kernel::shutdown::seal(appdata_generation)",
        "bndroid_kernel::platform_shutdown::validate(",
        "RESIDENT_PLATFORM_SHUTDOWN_OK",
        "qemu_semihosting_power_off(validated)",
    ),
    "graph proof, durable close, seal, token, evidence, and backend",
    "kernel/src/main.rs",
)

for needle in (
    "pub fn qemu_semihosting_power_off(",
    "ValidatedShutdown",
    "const SYS_EXIT_EXTENDED: u64 = 0x20;",
    "const ADP_STOPPED_APPLICATION_EXIT: u64 = 0x0002_0026;",
    '"hlt #0xf000"',
    "panic!(\"M66 QEMU semihosting poweroff returned unexpectedly\")",
):
    require(arch, needle, "QEMU-only non-returning backend", "kernel/src/arch/aarch64/mod.rs")

qemu_launches = len(re.findall(r"^  qemu-system-aarch64\s+\\$", runtime, re.MULTILINE))
if qemu_launches != 1 or runtime.count("-nic none") != 1:
    raise SystemExit("M66 runtime checker must have one launch site and one -nic none")
run_start = runtime.index("run_persistent_boot()")
run_end = runtime.index("run_evidence_parser --self-test", run_start)
run_body = runtime[run_start:run_end]
for needle in (
    "-semihosting-config enable=on,target=native",
    'if ! kill -0 "$QEMU_PID"',
    'wait "$QEMU_PID"',
    'if [[ "$qemu_status" != 0 ]]',
    'QEMU_SELF_EXITS=$((QEMU_SELF_EXITS + 1))',
):
    require(run_body, needle, "guest self-exit requirement", "scripts/check-resident-platform-shutdown-runtime.sh")
forbid(run_body, '\n    kill "$QEMU_PID"', "host-kill-as-success path", "scripts/check-resident-platform-shutdown-runtime.sh")
for needle in (
    "RESIDENT_PLATFORM_SHUTDOWN_PARSER_OK",
    "negative_cases={cases}",
    "run_persistent_boot first",
    "run_persistent_boot second",
    'if [[ "$QEMU_SELF_EXITS" != 2 ]]',
    "RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2",
    "emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8",
    "outside_data_appdata_unchanged=1",
    "emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0",
):
    require(runtime, needle, "two-boot parser and disk boundary", "scripts/check-resident-platform-shutdown-runtime.sh")

for needle in (
    '"$SCRIPT_DIR/check-resident-platform-shutdown-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-resident-platform-shutdown-runtime.sh"',
    "resident_platform_shutdown_static=1",
    "resident_platform_shutdown_reboot=1",
    "resident_platform_shutdown_boots=2",
):
    require(test_sh, needle, "complete-suite M66 gate", "scripts/test.sh")

for relative, text in (
    ("kernel/src/main.rs", main),
    ("scripts/check-resident-platform-shutdown-runtime.sh", runtime),
):
    for forbidden in (
        "full_userspace_shutdown_claim=1",
        "hardware_poweroff_claim=1",
        "psci_claim=1",
        "powercut_claim=1",
        "smp_claim=1",
        "general_runtime=1",
    ):
        forbid(text, forbidden, "scope overclaim", relative)

print(
    "RESIDENT_PLATFORM_SHUTDOWN_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} feature_chain=M66-M65-M64-M63-M62-M61-M60-M58-M57-M56-M55 "
    "abi=27 resident_nodes=8 dependency_edges=10 quiesce_waves=3 "
    "authenticated_nodes=1 kernel_topology=1 reverse_order=1 durable_close=1 "
    "platform_token=opaque fail_closed=1 qemu_backend=semihosting qemu_launches=1 "
    "qemu_boots=2 nic_none=1 hardware_poweroff_claim=0 psci_claim=0 "
    "powercut_claim=0 smp_claim=0 general_runtime=0"
)
PY

expect_build_rejected() {
  local name="$1"
  local kernel_features="$2"
  local user_features="$3"
  local expected="$4"
  local temp
  local output
  local status
  temp="$(mktemp -d "$WORKSPACE_ROOT/target/m66-feature-mismatch.XXXXXX")"
  set +e
  output="$(
    CARGO_TARGET_DIR="$temp" \
      BNDROID_KERNEL_FEATURES="$kernel_features" \
      BNDROID_USERSPACE_FEATURES="$user_features" \
      "$SCRIPT_DIR/build-kernel.sh" 2>&1
  )"
  status=$?
  set -e
  rm -rf "$temp"
  if [[ "$status" != 2 ]] || ! grep -Fq "$expected" <<<"$output"; then
    echo "M66 feature mismatch case '$name' was not rejected exactly." >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_build_rejected userspace_m66_over_default \
  "" \
  resident-platform-shutdown-runtime \
  "userspace resident-platform-shutdown profile requires the matching kernel profile."
expect_build_rejected userspace_m66_over_m65 \
  storage-server-shutdown-orchestration-runtime \
  resident-platform-shutdown-runtime \
  "userspace resident-platform-shutdown profile requires the matching kernel profile."
expect_build_rejected kernel_with_appdata \
  resident-platform-shutdown-runtime,app-data-runtime \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_build_rejected kernel_with_timeout \
  resident-platform-shutdown-runtime,storage-irq-timeout-self-test \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

ABI_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features resident-platform-shutdown-runtime \
    syscall_numbers_are_stable_and_unknown_values_are_rejected --lib
)"
if ! grep -Fq "test result: ok. 1 passed; 0 failed;" <<<"$ABI_OUTPUT"; then
  echo "M66 ABI v27 stability test did not pass exactly once." >&2
  echo "$ABI_OUTPUT" >&2
  exit 1
fi

GRAPH_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features resident-platform-shutdown-runtime \
    resident_shutdown_graph_is_stable_acyclic_and_complete --lib
)"
if ! grep -Fq "test result: ok. 1 passed; 0 failed;" <<<"$GRAPH_OUTPUT"; then
  echo "M66 ABI graph test did not pass exactly once." >&2
  echo "$GRAPH_OUTPUT" >&2
  exit 1
fi

SERVICE_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features resident-platform-shutdown-runtime \
    service_shutdown::tests:: --lib
)"
if ! grep -Fq "test result: ok. 4 passed; 0 failed;" <<<"$SERVICE_OUTPUT"; then
  echo "M66 service shutdown ledger did not pass exactly four tests." >&2
  echo "$SERVICE_OUTPUT" >&2
  exit 1
fi

PLATFORM_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features resident-platform-shutdown-runtime \
    platform_shutdown::tests:: --lib
)"
if ! grep -Fq "test result: ok. 2 passed; 0 failed;" <<<"$PLATFORM_OUTPUT"; then
  echo "M66 platform contract did not pass exactly two tests." >&2
  echo "$PLATFORM_OUTPUT" >&2
  exit 1
fi

PARSER_OUTPUT="$("$SCRIPT_DIR/check-resident-platform-shutdown-runtime.sh" --parser-self-test)"
EXPECTED_PARSER="RESIDENT_PLATFORM_SHUTDOWN_PARSER_OK negative_cases=144 health_fields=36 platform_fields=59 phases=2"
if [[ "$PARSER_OUTPUT" != "$EXPECTED_PARSER" ]]; then
  echo "M66 parser self-test ledger changed." >&2
  echo "$PARSER_OUTPUT" >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"
printf '%s\n' "RESIDENT_PLATFORM_SHUTDOWN_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=144 parser_health_fields=36 parser_platform_fields=59 parser_phases=2 service_tests=4 platform_tests=2 abi_tests=2 abi=27 resident_nodes=8 dependency_edges=10 quiesce_waves=3 authenticated_nodes=1 kernel_topology=1 reverse_order=1 durable_close=1 platform_token=opaque fail_closed=1 qemu_backend=semihosting qemu_launches=1 qemu_boots=2 nic_none=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0"
