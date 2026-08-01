#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm grep; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M63 persistent device-health contracts." >&2
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
feature = "storage-server-persistent-health-runtime"
parent = "storage-server-terminal-quarantine-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M63 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M63 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M63 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M63 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M63 static contract lost ordered {description}: "
                f"{relative}: {needle!r}"
            )
        cursor = position + len(needle)


def between(text: str, start: str, end: str, description: str, relative: str) -> str:
    if text.count(start) != 1:
        raise SystemExit(
            f"M63 static contract cannot isolate {description}: "
            f"{relative}: start_count={text.count(start)}"
        )
    begin = text.index(start)
    finish = text.find(end, begin + len(start))
    if finish < 0:
        raise SystemExit(
            f"M63 static contract lost end of {description}: {relative}: {end!r}"
        )
    return text[begin:finish]


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
    "scripts/check-storage-server-persistent-health-runtime.sh",
    "scripts/test.sh",
)
texts = {relative: source(relative) for relative in relative_sources}
kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
build_kernel = texts["scripts/build-kernel.sh"]
build_rs = texts["kernel/build.rs"]
persist = texts["kernel/src/persist.rs"]
storage = texts["kernel/src/storage.rs"]
adapter = texts["kernel/src/storage_persist.rs"]
main = texts["kernel/src/main.rs"]
syscall = texts["kernel/src/syscall.rs"]
abi = texts["crates/bndr-abi/src/lib.rs"]
user_main = texts["user/init/src/main.rs"]
user_runtime = texts["user/init/src/storage_server_runtime.rs"]
runtime = texts["scripts/check-storage-server-persistent-health-runtime.sh"]
test_sh = texts["scripts/test.sh"]

if feature_dependencies(kernel_cargo, feature, "kernel/Cargo.toml") != (parent,):
    raise SystemExit("M63 must have exactly one M62 kernel parent")
forbid(
    user_cargo,
    feature,
    "userspace M63 Cargo feature",
    "user/init/Cargo.toml",
)

require_once(
    build_kernel,
    "KERNEL_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME=0",
    "zero-initialized kernel-only M63 ledger",
    "scripts/build-kernel.sh",
)
kernel_leaf = between(
    build_kernel,
    'if feature_list_contains "$KERNEL_FEATURES" "storage-server-persistent-health-runtime"; then',
    'if feature_list_contains "$KERNEL_FEATURES" "storage-server-terminal-quarantine-runtime"; then',
    "kernel M63 closure",
    "scripts/build-kernel.sh",
)
ordered(
    kernel_leaf,
    (
        "KERNEL_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RUNTIME=1",
    ),
    "M63->M62->M61->M60->M58->M57->M56->M55 closure",
    "scripts/build-kernel.sh",
)
for needle in (
    'if feature_list_contains "$USERSPACE_FEATURES" '
    '"storage-server-persistent-health-runtime"; then',
    'echo "persistent device-health evidence is kernel-only and has no userspace feature." >&2',
):
    require_once(
        build_kernel,
        needle,
        "userspace M63 rejection",
        "scripts/build-kernel.sh",
    )
for forbidden in (
    "USER_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME",
    'append_feature "$USERSPACE_FEATURES" "storage-server-persistent-health-runtime"',
):
    forbid(
        build_kernel,
        forbidden,
        "M63 userspace mirroring",
        "scripts/build-kernel.sh",
    )

require_once(
    build_rs,
    "cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME",
    "M63 build invalidation",
    "kernel/build.rs",
)
forbid(
    build_rs,
    '.arg("feature=\\"storage-server-persistent-health-runtime\\"")',
    "embedded userspace M63 cfg forwarding",
    "kernel/build.rs",
)

for needle in (
    'pub const DEVICE_HEALTH_STATE_MAGIC: [u8; 8] = *b"BNDRHLT1";',
    "pub const DEVICE_HEALTH_STATE_VERSION: u32 = 1;",
    "pub const DEVICE_HEALTH_STATE_BYTES: usize = 80;",
    "pub struct DeviceHealthContract",
    "pub struct DeviceHealthState",
    "pub boot_open: bool",
    "pub struct DeviceHealthAdvanceEvidence",
    "pub offline_persisted: bool",
    "pub offline_from_record: bool",
    "pub fn advance_device_health_after_reprobe",
):
    require(persist, needle, "persistent health record", "kernel/src/persist.rs")

health_transaction = between(
    persist,
    "pub fn advance_device_health_after_reprobe<D: DurableSectorIo>(",
    "pub fn crc32(bytes: &[u8]) -> u32 {",
    "M63 durable transaction",
    "kernel/src/persist.rs",
)
ordered(
    health_transaction,
    (
        "current_contract\n        .validate()",
        "io.read_sector(partition_first_lba",
        "DataSuperblock::decode",
        "io.read_sector(lba, &mut initial_slots[slot])",
        "let initial = recover(",
        "BootState::decode(initial.payload)",
        "DeviceHealthState::decode(initial.payload)",
        "state.contract != current_contract",
        ".next_generation()",
        "boot_open: true",
        "io.write_sector(committed_lba, &committed_sector)",
        "io.flush()",
        "io.read_sector(lba, &mut verified_slots[slot])",
        "let verified = recover(",
        "DeviceHealthState::decode(verified.payload)",
        "reprobe_required: prior_boot_open || contract_changed",
        "reprobe_verified: true",
        "offline_persisted: false",
        "offline_from_record: false",
    ),
    "read->classify->inactive-write->flush->readback->hint-only evidence",
    "kernel/src/persist.rs",
)
for forbidden in (
    "storage_broker",
    "enter_device_offline",
    "open_recovery_admission",
    "syscall",
    "EL0",
    "unsafe ",
):
    forbid(
        health_transaction,
        forbidden,
        "authority or unsafe dependency in pure transaction",
        "kernel/src/persist.rs",
    )

expected_tests = (
    "device_health_state_round_trips_exact_contract_and_open_hint",
    "device_health_state_rejects_flags_contract_corruption_and_witnesses",
    "device_health_transaction_upgrades_legacy_without_persisting_offline",
    "open_previous_boot_requires_and_records_a_fresh_reprobe",
    "changed_contract_is_a_reprobe_hint_and_not_an_offline_decision",
    "ordinary_boot_counter_preserves_an_existing_health_payload",
    "malformed_health_payload_fails_closed_before_any_mutation",
)
for test in expected_tests:
    require_once(persist, f"fn {test}()", f"host test {test}", "kernel/src/persist.rs")

for needle in (
    '#[cfg(feature = "storage-server-persistent-health-runtime")]\n'
    "pub fn selected_block_features()",
    "device.negotiated_features().selected()",
):
    require(storage, needle, "live negotiated-feature snapshot", "kernel/src/storage.rs")

adapter_function = between(
    adapter,
    "pub fn advance_verified_device_health(",
    "const fn map_read_error",
    "M63 kernel persistence adapter",
    "kernel/src/storage_persist.rs",
)
ordered(
    adapter_function,
    (
        "storage::durability_contract()",
        "storage::device_contract()",
        "storage::selected_block_features()",
        "let irq = storage::irq_snapshot();",
        "let stats = storage::stats()",
        "let status_ok = storage::last_status_ok()",
        "storage::irq_id() != Some(expected_irq_id)",
        "selected_features != WRITABLE_FEATURES",
        "!irq.armed",
        "irq.failed",
        "irq.recovery_required",
        "irq.completions < 2",
        "storage::recovery_required()",
        "storage::recovery_admission_closed()",
        "stats.requests != stats.completions",
        "stats.completions != stats.interrupt_completions",
        "stats.read_completions < 2",
        "stats.timeouts != 0",
        "stats.resets != 0",
        "DeviceHealthContract {",
        "advance_device_health_after_reprobe(",
    ),
    "fresh live reprobe gate before durable commit",
    "kernel/src/storage_persist.rs",
)
for forbidden in (
    "storage_broker",
    "enter_device_offline",
    "complete_recovery",
    "open_recovery_admission",
):
    forbid(
        adapter_function,
        forbidden,
        "broker/recovery authority",
        "kernel/src/storage_persist.rs",
    )

call = main.index("storage_persist::advance_verified_device_health(")
for earlier in (
    "validate_virtio_block_storage(",
    "storage::read_pair_irq(",
    "storage_fs::validate(counter_frequency)",
):
    if main.index(earlier) >= call:
        raise SystemExit(f"M63 commit moved before current-boot proof: {earlier}")
for needle in (
    "boot.transport_base",
    "boot.irq.id",
):
    require(main, needle, "M63 live kernel contract input", "kernel/src/main.rs")
for needle in (
    "STORAGE_DEVICE_HEALTH_OK format=1 state_version=1 "
    "authority=kernel-boot-probe record_role=unclosed-boot-hint",
    "offline_persisted={} offline_from_record={} el0_controls=0",
    "qemu_reboot_proof=0 hardware_identity_claim=0 hotplug_claim=0 "
    "powercut_claim=0 tamper_resistance_claim=0 general_runtime=0",
    "BOOT_OK: M63 persistent unclosed-boot hint and fresh kernel reprobe boundary verified",
):
    require_once(main, needle, "M63 runtime evidence", "kernel/src/main.rs")

for relative, text in (
    ("user/init/src/main.rs", user_main),
    ("user/init/src/storage_server_runtime.rs", user_runtime),
    ("kernel/src/syscall.rs", syscall),
    ("crates/bndr-abi/src/lib.rs", abi),
):
    forbid(text, feature, "M63 EL0/ABI feature branch", relative)
for relative, text in (
    ("user/init/src/main.rs", user_main),
    ("user/init/src/storage_server_runtime.rs", user_runtime),
    ("kernel/src/syscall.rs", syscall),
):
    for forbidden in (
        "STORAGE_DEVICE_HEALTH_OK",
        "advance_verified_device_health",
        "offline_from_record",
    ):
        forbid(text, forbidden, "M63 userspace/syscall authority", relative)
require_once(abi, "pub const ABI_VERSION: u64 = 25;", "unchanged ABI v25", "crates/bndr-abi/src/lib.rs")

qemu_launches = len(re.findall(r"^  qemu-system-aarch64\s+\\$", runtime, re.MULTILINE))
if qemu_launches != 1 or runtime.count("-nic none") != 1:
    raise SystemExit("M63 runtime checker must have one QEMU launch site and one -nic none")
for needle in (
    "STORAGE_SERVER_PERSISTENT_HEALTH_PARSER_OK",
    "negative_cases={cases}",
    "health_fields={len(COMMON_SCHEMA)}",
    "run_persistent_boot first",
    "run_persistent_boot second",
    "STORAGE_DEVICE_HEALTH_REBOOT_OK boots=2",
    "outside_data_appdata_unchanged=1",
    "offline_persisted=0",
    "el0_controls=0",
    "hardware_identity_claim=0",
    "hotplug_claim=0",
    "powercut_claim=0",
    "tamper_resistance_claim=0",
    "general_runtime=0",
):
    require(
        runtime,
        needle,
        "two-boot runtime/parser boundary",
        "scripts/check-storage-server-persistent-health-runtime.sh",
    )

for needle in (
    '"$SCRIPT_DIR/check-storage-server-persistent-health-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-persistent-health-runtime.sh"',
    "storage_server_persistent_health_static=1",
    "storage_server_persistent_health_reboot=1",
    "persistent_health_boots=2",
):
    require(test_sh, needle, "complete-suite M63 gate", "scripts/test.sh")

print(
    "STORAGE_SERVER_PERSISTENT_HEALTH_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} host_tests={len(expected_tests)} "
    "feature_chain=M63-M62-M61-M60-M58-M57-M56-M55 kernel_only=1 "
    "legacy_upgrade=1 unclosed_hint=1 current_reprobe=1 "
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
  temp="$(mktemp -d "$WORKSPACE_ROOT/target/m63-feature-mismatch.XXXXXX")"
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
    echo "M63 feature mismatch case '$name' was not rejected exactly." >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_build_rejected userspace_m63_over_kernel_m63 \
  storage-server-persistent-health-runtime \
  storage-server-persistent-health-runtime \
  "persistent device-health evidence is kernel-only and has no userspace feature."
expect_build_rejected userspace_m63_over_default \
  "" \
  storage-server-persistent-health-runtime \
  "persistent device-health evidence is kernel-only and has no userspace feature."
expect_build_rejected kernel_with_appdata \
  storage-server-persistent-health-runtime,app-data-runtime \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_build_rejected kernel_with_timeout \
  storage-server-persistent-health-runtime,storage-irq-timeout-self-test \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
HEALTH_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features storage-server-persistent-health-runtime \
    persist::tests:: --lib
)"
if ! grep -Fq "test result: ok. 23 passed; 0 failed;" <<<"$HEALTH_OUTPUT"; then
  echo "M63 persistence/health host test ledger did not pass exactly 23 tests." >&2
  echo "$HEALTH_OUTPUT" >&2
  exit 1
fi

PARSER_OUTPUT="$("$SCRIPT_DIR/check-storage-server-persistent-health-runtime.sh" --parser-self-test)"
EXPECTED_PARSER="STORAGE_SERVER_PERSISTENT_HEALTH_PARSER_OK negative_cases=54 health_fields=36 phases=2"
if [[ "$PARSER_OUTPUT" != "$EXPECTED_PARSER" ]]; then
  echo "M63 parser self-test ledger changed." >&2
  echo "$PARSER_OUTPUT" >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"
printf '%s\n' "STORAGE_SERVER_PERSISTENT_HEALTH_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=54 parser_health_fields=36 parser_phases=2 host_tests=7 persist_tests=23 kernel_only=1 legacy_upgrade=1 unclosed_hint=1 current_reprobe=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1"
