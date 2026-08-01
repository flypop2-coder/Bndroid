#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm grep; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M65 shutdown-orchestration contracts." >&2
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
feature = "storage-server-shutdown-orchestration-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M65 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M65 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M65 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M65 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M65 static contract lost ordered {description}: "
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
    "kernel/src/lib.rs",
    "kernel/src/main.rs",
    "kernel/src/scheduler.rs",
    "kernel/src/shutdown.rs",
    "kernel/src/syscall.rs",
    "scripts/build-kernel.sh",
    "scripts/check-storage-server-shutdown-orchestration-runtime.sh",
    "scripts/check-storage-server-shutdown-orchestration-static.sh",
    "scripts/test.sh",
    "user/init/Cargo.toml",
    "user/init/src/main.rs",
    "user/init/src/storage_server_runtime.rs",
)
texts = {relative: source(relative) for relative in relative_sources}
abi_cargo = texts["crates/bndr-abi/Cargo.toml"]
abi = texts["crates/bndr-abi/src/lib.rs"]
kernel_cargo = texts["kernel/Cargo.toml"]
build_rs = texts["kernel/build.rs"]
kernel_lib = texts["kernel/src/lib.rs"]
main = texts["kernel/src/main.rs"]
scheduler = texts["kernel/src/scheduler.rs"]
shutdown = texts["kernel/src/shutdown.rs"]
syscall = texts["kernel/src/syscall.rs"]
build = texts["scripts/build-kernel.sh"]
runtime = texts["scripts/check-storage-server-shutdown-orchestration-runtime.sh"]
test_sh = texts["scripts/test.sh"]
user_cargo = texts["user/init/Cargo.toml"]
user_main = texts["user/init/src/main.rs"]
user_runtime = texts["user/init/src/storage_server_runtime.rs"]

if feature_dependencies(abi_cargo, feature, "crates/bndr-abi/Cargo.toml") != (
    "storage-server-runtime",
):
    raise SystemExit("M65 ABI feature must depend on exactly the storage-server ABI")
if feature_dependencies(kernel_cargo, feature, "kernel/Cargo.toml") != (
    "storage-server-clean-shutdown-runtime",
    "bndr-abi/storage-server-shutdown-orchestration-runtime",
):
    raise SystemExit("M65 kernel feature must depend on exactly M64 and ABI v26")
if feature_dependencies(user_cargo, feature, "user/init/Cargo.toml") != (
    "storage-server-terminal-quarantine-runtime",
    "bndr-abi/storage-server-shutdown-orchestration-runtime",
):
    raise SystemExit("M65 userspace feature closure changed")

for needle in (
    'pub const ABI_VERSION: u64 = 26;',
    'pub const SYSTEM_SHUTDOWN_FLAGS_NONE: u64 = 0;',
    'pub const SYSTEM_SHUTDOWN_PREPARE: u64 = 1;',
    'pub const SYSTEM_SHUTDOWN_COMMIT: u64 = 2;',
    'SystemShutdown = 53,',
    '53 => Some(Self::SystemShutdown),',
    'assert_eq!(SyscallNumber::from_raw(54), None);',
):
    require(abi, needle, "ABI v26 shutdown syscall", "crates/bndr-abi/src/lib.rs")
require_once(
    user_main,
    "const _: [(); 26] = [(); ABI_VERSION as usize];",
    "userspace ABI v26 compile-time lock",
    "user/init/src/main.rs",
)

for needle in (
    "KERNEL_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME=0",
    "USER_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" "storage-server-shutdown-orchestration-runtime"',
    "KERNEL_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME=1",
    "KERNEL_STORAGE_SERVER_CLEAN_SHUTDOWN_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "storage-server-shutdown-orchestration-runtime"',
    "USER_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME=1",
    "userspace shutdown-orchestration profile requires the matching kernel profile.",
    'append_feature "$USERSPACE_FEATURES" "storage-server-shutdown-orchestration-runtime"',
):
    require(build, needle, "matched kernel/userspace feature closure", "scripts/build-kernel.sh")
for needle in (
    "CARGO_FEATURE_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME",
    'feature=\\"storage-server-shutdown-orchestration-runtime\\"',
):
    require(build_rs, needle, "direct userspace feature forwarding", "kernel/build.rs")
require_once(
    kernel_lib,
    "pub mod shutdown;",
    "kernel shutdown module export",
    "kernel/src/lib.rs",
)

for needle in (
    "pub enum Phase",
    "Open = 0",
    "Quiescing = 1",
    "Requested = 2",
    "Sealed = 3",
    "static STATE: AtomicU64",
    "pub const fn reduce(",
    "pub fn prepare(generation: u64)",
    "pub fn commit(generation: u64)",
    "pub fn seal(generation: u64)",
    "pub fn admission_closed() -> bool",
    "pub fn requested_generation() -> Option<u64>",
    "compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)",
):
    require(shutdown, needle, "atomic two-phase shutdown gate", "kernel/src/shutdown.rs")
shutdown_tests = (
    "strict_prepare_commit_seal_sequence_preserves_generation",
    "commit_before_prepare_and_replay_fail_closed",
    "generation_mismatch_never_advances_the_gate",
    "zero_and_unencodable_generations_are_rejected",
)
for test in shutdown_tests:
    require_once(shutdown, f"fn {test}()", "M65 host test", "kernel/src/shutdown.rs")

for needle in (
    "SyscallNumber::SystemShutdown => system_shutdown(frame, arg0, arg1, arg2)",
    "SyscallNumber::SystemShutdown => complete(frame, Status::Unsupported, 0, 0)",
    "fn system_shutdown(",
    "if !crate::process::current_is_init()",
    "record_permission_denied();",
    "m65_shutdown_prepare_ready(generation)",
    "bndroid_kernel::shutdown::prepare(generation)",
    "m65_storage_ready_proof_valid(generation)",
    "bndroid_kernel::shutdown::commit(generation)",
    "if bndroid_kernel::shutdown::admission_closed()",
    "record_spawn_rejection();",
    "record_connect_rejection();",
    "fn m65_storage_device_quiescent() -> bool",
    "fn m65_storage_io_ledger_valid(",
):
    require(syscall, needle, "init-only syscall and admission gates", "kernel/src/syscall.rs")
ordered(
    syscall,
    (
        "fn system_shutdown(",
        "if !crate::process::current_is_init()",
        "m65_shutdown_prepare_ready(generation)",
        "bndroid_kernel::shutdown::prepare(generation)",
        "m65_storage_ready_proof_valid(generation)",
        "bndroid_kernel::shutdown::commit(generation)",
    ),
    "permission, Prepare, proof, and Commit checks",
    "kernel/src/syscall.rs",
)

for needle in (
    "pub(super) fn init_runtime(system_root: u64) -> !",
    "SYSTEM_SHUTDOWN_PREPARE",
    "exercise_shutdown_spawn_barrier();",
    "SERVER_SHUTDOWN_COMMAND_TAG",
    "SERVER_SHUTDOWN_ACK_TAG",
    "SyscallNumber::ProcessWait",
    "M65_STORAGE_READY_PREFIX | final_generation",
    "SYSTEM_SHUTDOWN_COMMIT",
    "fn accept_session_or_shutdown(",
    "ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits()",
):
    require(
        user_runtime,
        needle,
        "bounded init and StorageServer shutdown protocol",
        "user/init/src/storage_server_runtime.rs",
    )
ordered(
    user_runtime,
    (
        "let prepared = syscall(",
        "exercise_shutdown_spawn_barrier();",
        "SERVER_SHUTDOWN_COMMAND_TAG,",
        "SERVER_SHUTDOWN_ACK_TAG",
        "SyscallNumber::ProcessWait",
        "SyscallNumber::InitReady",
        "let committed = syscall(",
    ),
    "init Prepare, drain, proof, and Commit transcript",
    "user/init/src/storage_server_runtime.rs",
)
ordered(
    user_runtime,
    (
        "ServerWork::Shutdown(generation) => {",
        "io.flush().unwrap_or_else",
        ".recover_with_policy(&mut io, &policy)",
        "write_scalar(startup, SERVER_SHUTDOWN_ACK_TAG, current_generation);",
        "exit_child(SERVER_SHUTDOWN_EXIT_CODE);",
    ),
    "StorageServer flush, readback, acknowledge, and exit",
    "user/init/src/storage_server_runtime.rs",
)

for needle in (
    "fn validate_storage_server_shutdown_orchestration_runtime(",
    "let shutdown = bndroid_kernel::shutdown::snapshot();",
    "if shutdown.phase == bndroid_kernel::shutdown::Phase::Requested",
    "storage_persist::close_verified_device_health",
    "storage::seal_clean_shutdown_admission_masked",
    "interrupt::disable_block_irq_masked",
    "bndroid_kernel::shutdown::seal(appdata_generation)",
    "bounded_userspace_shutdown=1 clients_drained=2",
    "el0_started=1 el0_controls=1",
    "full_userspace_shutdown_claim=0 hardware_poweroff_claim=0",
    "BOOT_OK: M65 userspace StorageServer shutdown orchestration and durable close verified",
):
    require(main, needle, "EL0 shutdown monitor and durable close", "kernel/src/main.rs")
if "bndroid_kernel::shutdown::requested_generation()" in main:
    raise SystemExit("M65/M66 monitor regained a cross-phase snapshot race")
ordered(
    main,
    (
        "fn validate_storage_server_shutdown_orchestration_runtime(",
        "let shutdown = bndroid_kernel::shutdown::snapshot();",
        "let syscalls = syscall::snapshot();",
        "let processes = process::snapshot();",
        "if shutdown.phase == bndroid_kernel::shutdown::Phase::Requested",
    ),
    "phase-first coherent shutdown evidence",
    "kernel/src/main.rs",
)
for needle in (
    'feature = "storage-server-shutdown-orchestration-runtime"',
    "pub fn live_dynamic_user_context_count() -> usize",
    "pub fn live_dynamic_kernel_stack_count() -> usize",
):
    require(
        scheduler,
        needle,
        "M65 all-feature scheduler resource ledger",
        "kernel/src/scheduler.rs",
    )
ordered(
    main,
    (
        "fn validate_storage_server_shutdown_orchestration_runtime(",
        "storage_persist::close_verified_device_health(",
        "storage::seal_clean_shutdown_admission_masked",
        "interrupt::disable_block_irq_masked",
        "bndroid_kernel::shutdown::seal(appdata_generation)",
        "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_OK",
    ),
    "commit, durable close, seal, and evidence",
    "kernel/src/main.rs",
)

qemu_launches = len(re.findall(r"^  qemu-system-aarch64\s+\\$", runtime, re.MULTILINE))
if qemu_launches != 1 or runtime.count("-nic none") != 1:
    raise SystemExit("M65 runtime checker must have one QEMU launch site and one -nic none")
for needle in (
    "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_PARSER_OK",
    "negative_cases={cases}",
    "health_fields={len(HEALTH_SCHEMA)}",
    "shutdown_fields={len(SHUTDOWN_SCHEMA)}",
    "run_persistent_boot first",
    "run_persistent_boot second",
    "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_REBOOT_OK boots=2",
    "userspace_shutdowns=2 clients_drained=4",
    "final_appdata_generation=6 final_health_generation=4",
    "outside_data_appdata_unchanged=1",
    "full_userspace_shutdown_claim=0 hardware_poweroff_claim=0",
    "powercut_claim=0 smp_claim=0 general_runtime=0",
):
    require(
        runtime,
        needle,
        "two-boot parser and disk boundary",
        "scripts/check-storage-server-shutdown-orchestration-runtime.sh",
    )

for needle in (
    '"$SCRIPT_DIR/check-storage-server-shutdown-orchestration-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-shutdown-orchestration-runtime.sh"',
    "storage_server_shutdown_orchestration_static=1",
    "storage_server_shutdown_orchestration_reboot=1",
    "shutdown_orchestration_boots=2",
):
    require(test_sh, needle, "complete-suite M65 gate", "scripts/test.sh")

for relative, text in (
    ("kernel/src/main.rs", main),
    ("scripts/check-storage-server-shutdown-orchestration-runtime.sh", runtime),
):
    for forbidden in (
        "full_userspace_shutdown_claim=1",
        "hardware_poweroff_claim=1",
        "powercut_claim=1",
        "smp_claim=1",
        "general_runtime=1",
    ):
        forbid(text, forbidden, "scope overclaim", relative)

print(
    "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} host_tests={len(shutdown_tests)} "
    "feature_chain=M65-M64-M63-M62-M61-M60-M58-M57-M56-M55 abi=26 "
    "init_only=1 prepare_commit=1 generation_bound=1 process_gate=1 "
    "storage_gate=1 server_flush=1 server_readback=1 durable_close=1 "
    "admission_seal=1 el0_controls=1 hardware_poweroff_claim=0 "
    "powercut_claim=0 smp_claim=0 general_runtime=0 "
    "qemu_launches=1 qemu_boots=2 nic_none=1"
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
  temp="$(mktemp -d "$WORKSPACE_ROOT/target/m65-feature-mismatch.XXXXXX")"
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
    echo "M65 feature mismatch case '$name' was not rejected exactly." >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_build_rejected userspace_m65_over_default \
  "" \
  storage-server-shutdown-orchestration-runtime \
  "userspace shutdown-orchestration profile requires the matching kernel profile."
expect_build_rejected userspace_m65_over_m64 \
  storage-server-clean-shutdown-runtime \
  storage-server-shutdown-orchestration-runtime \
  "userspace shutdown-orchestration profile requires the matching kernel profile."
expect_build_rejected kernel_with_appdata \
  storage-server-shutdown-orchestration-runtime,app-data-runtime \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_build_rejected kernel_with_timeout \
  storage-server-shutdown-orchestration-runtime,storage-irq-timeout-self-test \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
SHUTDOWN_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features storage-server-shutdown-orchestration-runtime \
    shutdown::tests:: --lib
)"
if ! grep -Fq "test result: ok. 4 passed; 0 failed;" <<<"$SHUTDOWN_OUTPUT"; then
  echo "M65 shutdown state-machine ledger did not pass exactly four tests." >&2
  echo "$SHUTDOWN_OUTPUT" >&2
  exit 1
fi

ABI_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features storage-server-shutdown-orchestration-runtime \
    syscall_numbers_are_stable_and_unknown_values_are_rejected --lib
)"
if ! grep -Fq "test result: ok. 1 passed; 0 failed;" <<<"$ABI_OUTPUT"; then
  echo "M65 ABI v26 stability test did not pass exactly once." >&2
  echo "$ABI_OUTPUT" >&2
  exit 1
fi

PARSER_OUTPUT="$("$SCRIPT_DIR/check-storage-server-shutdown-orchestration-runtime.sh" --parser-self-test)"
EXPECTED_PARSER="STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_PARSER_OK negative_cases=133 health_fields=36 shutdown_fields=52 phases=2"
if [[ "$PARSER_OUTPUT" != "$EXPECTED_PARSER" ]]; then
  echo "M65 parser self-test ledger changed." >&2
  echo "$PARSER_OUTPUT" >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"
printf '%s\n' "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=133 parser_health_fields=36 parser_shutdown_fields=52 parser_phases=2 host_tests=4 abi_tests=1 abi=26 init_only=1 prepare_commit=1 generation_bound=1 process_gate=1 storage_gate=1 server_flush=1 server_readback=1 durable_close=1 admission_seal=1 el0_controls=1 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 qemu_launches=1 qemu_boots=2 nic_none=1"
