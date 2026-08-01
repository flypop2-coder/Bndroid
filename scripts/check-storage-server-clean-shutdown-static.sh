#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm grep; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M64 clean-shutdown contracts." >&2
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
feature = "storage-server-clean-shutdown-runtime"
parent = "storage-server-persistent-health-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M64 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M64 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M64 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M64 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M64 static contract lost ordered {description}: "
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
    "kernel/Cargo.toml",
    "user/init/Cargo.toml",
    "scripts/build-kernel.sh",
    "kernel/build.rs",
    "kernel/src/persist.rs",
    "kernel/src/storage.rs",
    "kernel/src/storage_persist.rs",
    "kernel/src/main.rs",
    "kernel/src/syscall.rs",
    "crates/bndr-abi/src/lib.rs",
    "user/init/src/main.rs",
    "user/init/src/storage_server_runtime.rs",
    "scripts/check-storage-server-clean-shutdown-runtime.sh",
    "scripts/check-storage-server-clean-shutdown-static.sh",
    "scripts/test.sh",
)
texts = {relative: source(relative) for relative in relative_sources}
cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
build = texts["scripts/build-kernel.sh"]
build_rs = texts["kernel/build.rs"]
persist = texts["kernel/src/persist.rs"]
storage = texts["kernel/src/storage.rs"]
adapter = texts["kernel/src/storage_persist.rs"]
main = texts["kernel/src/main.rs"]
syscall = texts["kernel/src/syscall.rs"]
abi = texts["crates/bndr-abi/src/lib.rs"]
user_main = texts["user/init/src/main.rs"]
user_runtime = texts["user/init/src/storage_server_runtime.rs"]
runtime = texts["scripts/check-storage-server-clean-shutdown-runtime.sh"]
test_sh = texts["scripts/test.sh"]

if feature_dependencies(cargo, feature, "kernel/Cargo.toml") != (parent,):
    raise SystemExit("M64 feature must depend on exactly M63")
for relative, text in (
    ("user/init/Cargo.toml", user_cargo),
    ("user/init/src/main.rs", user_main),
    ("user/init/src/storage_server_runtime.rs", user_runtime),
    ("kernel/src/syscall.rs", syscall),
    ("crates/bndr-abi/src/lib.rs", abi),
):
    forbid(text, feature, "M64 EL0/ABI feature branch", relative)
require_once(abi, "pub const ABI_VERSION: u64 = 25;", "unchanged ABI v25", "crates/bndr-abi/src/lib.rs")

for needle in (
    "KERNEL_STORAGE_SERVER_CLEAN_SHUTDOWN_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" "storage-server-clean-shutdown-runtime"',
    "KERNEL_STORAGE_SERVER_CLEAN_SHUTDOWN_RUNTIME=1",
    "KERNEL_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME=1",
    'feature_list_contains "$USERSPACE_FEATURES" "storage-server-clean-shutdown-runtime"',
    "clean-shutdown boot-session closure is kernel-only and has no userspace feature.",
):
    require(build, needle, "kernel-only feature closure", "scripts/build-kernel.sh")
forbid(
    build,
    'append_feature "$USERSPACE_FEATURES" "storage-server-clean-shutdown-runtime"',
    "M64 forwarding into userspace",
    "scripts/build-kernel.sh",
)
for needle in (
    "CARGO_FEATURE_STORAGE_SERVER_CLEAN_SHUTDOWN_RUNTIME",
    'feature=\\"storage-server-clean-shutdown-runtime\\"',
):
    forbid(build_rs, needle, "M64 userspace build.rs forwarding", "kernel/build.rs")

for needle in (
    "pub enum DeviceHealthCloseError",
    "SessionGenerationMismatch",
    "SessionAlreadyClosed",
    "ContractMismatch",
    "pub struct DeviceHealthCloseEvidence",
    "pub fn close_device_health_for_clean_shutdown",
    "initial.generation != expected_open_generation",
    "!initial_state.boot_open",
    "initial_state.contract != current_contract",
    "boot_open: false",
    "offline_persisted: false",
    "offline_from_record: false",
):
    require(persist, needle, "fail-closed durable close primitive", "kernel/src/persist.rs")
expected_tests = (
    "clean_shutdown_closes_the_exact_open_session_in_strict_order",
    "closed_session_removes_only_the_next_boot_unclosed_hint",
    "clean_shutdown_rejects_a_stale_session_generation_before_mutation",
    "clean_shutdown_rejects_a_changed_contract_before_mutation",
    "clean_shutdown_rejects_replay_of_an_already_closed_session",
    "clean_shutdown_does_not_verify_a_torn_close_record",
    "clean_shutdown_stops_after_a_failed_flush",
    "clean_shutdown_rejects_generation_exhaustion_before_mutation",
)
for test in expected_tests:
    require_once(persist, f"fn {test}()", "M64 host test", "kernel/src/persist.rs")

for needle in (
    "pub struct CleanShutdownGateSnapshot",
    "static CLEAN_SHUTDOWN_ACTIVE: AtomicBool",
    "pub(crate) fn seal_clean_shutdown_admission_masked",
    "RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);",
    "pub fn clean_shutdown_gate_snapshot",
):
    require(storage, needle, "kernel shutdown admission seal", "kernel/src/storage.rs")

for needle in (
    "static OPEN_SESSION_GENERATION: AtomicU64",
    "static OPEN_SESSION_EPOCH_LOW: AtomicU64",
    "static OPEN_SESSION_EPOCH_HIGH: AtomicU64",
    "publish_open_session(format_epoch, evidence.persistence.committed_generation)",
    "pub fn close_verified_device_health",
    "close_device_health_for_clean_shutdown",
    "OPEN_SESSION_GENERATION.store(0, Ordering::Release);",
    "CleanShutdownNotQuiescent",
):
    require(adapter, needle, "in-memory boot-session authority", "kernel/src/storage_persist.rs")

for needle in (
    "fn validate_storage_server_clean_shutdown_runtime",
    "scheduler::user_init_selection_count() == 0",
    "storage_persist::close_verified_device_health",
    "storage::seal_clean_shutdown_admission_masked",
    "interrupt::disable_block_irq_masked",
    "post_close_storage_mutations=0",
    "el0_started=0 el0_controls=0",
    "full_userspace_shutdown_claim=0 hardware_poweroff_claim=0",
    "BOOT_OK: M64 kernel-owned clean boot-session close and no-later-storage boundary verified",
):
    require(main, needle, "bounded pre-EL0 shutdown runtime", "kernel/src/main.rs")
ordered(
    main,
    (
        "maybe_validate_storage_server_clean_shutdown_runtime(",
        "process::init();",
        "let user_info = userboot::start()",
    ),
    "pre-EL0 shutdown selection",
    "kernel/src/main.rs",
)
for relative, text in (
    ("user/init/src/main.rs", user_main),
    ("user/init/src/storage_server_runtime.rs", user_runtime),
    ("kernel/src/syscall.rs", syscall),
):
    for forbidden in (
        "STORAGE_SERVER_CLEAN_SHUTDOWN_OK",
        "close_verified_device_health",
        "seal_clean_shutdown_admission_masked",
    ):
        forbid(text, forbidden, "M64 userspace/syscall shutdown authority", relative)

qemu_launches = len(re.findall(r"^  qemu-system-aarch64\s+\\$", runtime, re.MULTILINE))
if qemu_launches != 1 or runtime.count("-nic none") != 1:
    raise SystemExit("M64 runtime checker must have one QEMU launch site and one -nic none")
for needle in (
    "STORAGE_SERVER_CLEAN_SHUTDOWN_PARSER_OK",
    "negative_cases={cases}",
    "health_fields={len(HEALTH_COMMON)}",
    "close_fields={len(CLOSE_COMMON)}",
    "run_persistent_boot first",
    "run_persistent_boot second",
    "STORAGE_CLEAN_SHUTDOWN_REBOOT_OK boots=2",
    "final_generation=4",
    "final_boot_open=0 prior_boot_open=1",
    "outside_data_unchanged=1 appdata_unchanged=1",
    "offline_persisted=0 offline_from_record=0",
    "el0_started=0 el0_controls=0",
    "full_userspace_shutdown_claim=0 hardware_poweroff_claim=0",
    "powercut_claim=0",
    "smp_claim=0 general_runtime=0",
):
    require(
        runtime,
        needle,
        "two-boot close/parser boundary",
        "scripts/check-storage-server-clean-shutdown-runtime.sh",
    )

for needle in (
    '"$SCRIPT_DIR/check-storage-server-clean-shutdown-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-clean-shutdown-runtime.sh"',
    "storage_server_clean_shutdown_static=1",
    "storage_server_clean_shutdown_reboot=1",
    "clean_shutdown_boots=2",
):
    require(test_sh, needle, "complete-suite M64 gate", "scripts/test.sh")

print(
    "STORAGE_SERVER_CLEAN_SHUTDOWN_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} host_tests={len(expected_tests)} "
    "feature_chain=M64-M63-M62-M61-M60-M58-M57-M56-M55 kernel_only=1 "
    "pre_el0=1 clean_close=1 session_capability=1 admission_seal=1 "
    "offline_persisted=0 offline_from_record=0 el0_controls=0 "
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
  temp="$(mktemp -d "$WORKSPACE_ROOT/target/m64-feature-mismatch.XXXXXX")"
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
    echo "M64 feature mismatch case '$name' was not rejected exactly." >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_build_rejected userspace_m64_over_kernel_m64 \
  storage-server-clean-shutdown-runtime \
  storage-server-clean-shutdown-runtime \
  "clean-shutdown boot-session closure is kernel-only and has no userspace feature."
expect_build_rejected userspace_m64_over_default \
  "" \
  storage-server-clean-shutdown-runtime \
  "clean-shutdown boot-session closure is kernel-only and has no userspace feature."
expect_build_rejected kernel_with_appdata \
  storage-server-clean-shutdown-runtime,app-data-runtime \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_build_rejected kernel_with_timeout \
  storage-server-clean-shutdown-runtime,storage-irq-timeout-self-test \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
CLOSE_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features storage-server-clean-shutdown-runtime \
    persist::tests:: --lib
)"
if ! grep -Fq "test result: ok. 31 passed; 0 failed;" <<<"$CLOSE_OUTPUT"; then
  echo "M64 persistence/close host test ledger did not pass exactly 31 tests." >&2
  echo "$CLOSE_OUTPUT" >&2
  exit 1
fi

PARSER_OUTPUT="$("$SCRIPT_DIR/check-storage-server-clean-shutdown-runtime.sh" --parser-self-test)"
EXPECTED_PARSER="STORAGE_SERVER_CLEAN_SHUTDOWN_PARSER_OK negative_cases=93 health_fields=36 close_fields=37 phases=2"
if [[ "$PARSER_OUTPUT" != "$EXPECTED_PARSER" ]]; then
  echo "M64 parser self-test ledger changed." >&2
  echo "$PARSER_OUTPUT" >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"
printf '%s\n' "STORAGE_SERVER_CLEAN_SHUTDOWN_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=93 parser_health_fields=36 parser_close_fields=37 parser_phases=2 host_tests=8 persist_tests=31 kernel_only=1 pre_el0=1 clean_close=1 session_capability=1 admission_seal=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1"
