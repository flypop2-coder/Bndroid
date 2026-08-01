#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M62 terminal-quarantine contracts." >&2
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
feature = "storage-server-terminal-quarantine-runtime"
parent = "storage-server-owner-liveness-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M62 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M62 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M62 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M62 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M62 static contract lost ordered {description}: "
                f"{relative}: {needle!r}"
            )
        cursor = position + len(needle)


def between(text: str, start: str, end: str, description: str, relative: str) -> str:
    if text.count(start) != 1:
        raise SystemExit(
            f"M62 static contract cannot isolate {description}: "
            f"{relative}: start_count={text.count(start)}"
        )
    begin = text.index(start)
    finish = text.find(end, begin + len(start))
    if finish < 0:
        raise SystemExit(
            f"M62 static contract lost end of {description}: {relative}: {end!r}"
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
    "kernel/src/lib.rs",
    "kernel/src/storage_terminal_quarantine.rs",
    "kernel/src/storage_server_io.rs",
    "kernel/src/syscall.rs",
    "kernel/src/main.rs",
    "user/init/src/storage_server_runtime.rs",
    "user/init/src/main.rs",
    "scripts/check-storage-server-terminal-quarantine-runtime.sh",
    "scripts/test.sh",
)
texts = {relative: source(relative) for relative in relative_sources}
kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
build_kernel = texts["scripts/build-kernel.sh"]
build_rs = texts["kernel/build.rs"]
lib_rs = texts["kernel/src/lib.rs"]
policy = texts["kernel/src/storage_terminal_quarantine.rs"]
coordinator = texts["kernel/src/storage_server_io.rs"]
syscall = texts["kernel/src/syscall.rs"]
main = texts["kernel/src/main.rs"]
user_runtime = texts["user/init/src/storage_server_runtime.rs"]
user_main = texts["user/init/src/main.rs"]
runtime = texts["scripts/check-storage-server-terminal-quarantine-runtime.sh"]
test_sh = texts["scripts/test.sh"]

for relative, manifest in (
    ("kernel/Cargo.toml", kernel_cargo),
    ("user/init/Cargo.toml", user_cargo),
):
    if feature_dependencies(manifest, feature, relative) != (parent,):
        raise SystemExit(f"M62 must have exactly one M61 parent: {relative}")

for variable in (
    "KERNEL_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME=0",
    "USER_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME=0",
):
    require_once(
        build_kernel,
        variable,
        "zero-initialized M62 feature ledger",
        "scripts/build-kernel.sh",
    )

kernel_leaf = between(
    build_kernel,
    'if feature_list_contains "$KERNEL_FEATURES" "storage-server-terminal-quarantine-runtime"; then',
    'if feature_list_contains "$KERNEL_FEATURES" "storage-server-owner-liveness-runtime"; then',
    "kernel M62 closure",
    "scripts/build-kernel.sh",
)
ordered(
    kernel_leaf,
    (
        "KERNEL_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RUNTIME=1",
    ),
    "kernel M62->M61->M60->M58->M57->M56->M55 closure",
    "scripts/build-kernel.sh",
)
user_leaf = between(
    build_kernel,
    'if feature_list_contains "$USERSPACE_FEATURES" "storage-server-terminal-quarantine-runtime"; then',
    'if feature_list_contains "$USERSPACE_FEATURES" "storage-server-owner-liveness-runtime"; then',
    "userspace M62 closure",
    "scripts/build-kernel.sh",
)
ordered(
    user_leaf,
    (
        "USER_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME=1",
        "USER_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=1",
        "USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1",
        "USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_RUNTIME=1",
    ),
    "userspace M62->M61->M60->M58->M57->M56->M55 closure",
    "scripts/build-kernel.sh",
)
for needle in (
    'if [[ "$USER_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME" -eq 1 \\\n'
    '  && "$KERNEL_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME" -ne 1 ]]; then',
    'echo "userspace terminal-quarantine profile exceeds the kernel storage profile." >&2',
    'if [[ "$KERNEL_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME" -eq 1 ]]; then',
    'USERSPACE_FEATURES="$(append_feature "$USERSPACE_FEATURES" '
    '"storage-server-terminal-quarantine-runtime")"',
):
    require(build_kernel, needle, "M62 mismatch/mirror wiring", "scripts/build-kernel.sh")

for needle in (
    'println!("cargo:rerun-if-env-changed='
    'CARGO_FEATURE_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME");',
    'if env::var_os("CARGO_FEATURE_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME").is_some() {',
    '.arg("feature=\\"storage-server-terminal-quarantine-runtime\\"");',
):
    require(build_rs, needle, "M62 embedded userspace cfg forwarding", "kernel/build.rs")

require_once(
    lib_rs,
    "pub mod storage_terminal_quarantine;",
    "terminal policy module export",
    "kernel/src/lib.rs",
)
for needle, description in (
    ("crate::", "kernel adapter dependency"),
    ("std::", "std dependency"),
    ("alloc::", "allocator dependency"),
    ("UnsafeCell", "interior-unsafe storage"),
    ("unsafe ", "unsafe code"),
    ("Atomic", "atomic synchronization"),
    ("Mutex", "mutex synchronization"),
    ("spin_loop", "busy wait"),
    ("Vec<", "dynamic vector"),
    ("Box<", "heap box"),
):
    forbid(policy, needle, description, "kernel/src/storage_terminal_quarantine.rs")
for needle in (
    "pub enum Phase",
    "Idle = 0",
    "Armed = 1",
    "Consumed = 2",
    "pub enum Decision",
    "Unsafe,",
    "RunFallback,",
    "Publish,",
    "pub struct TerminalProofGate",
    "pub fn arm(&mut self)",
    "pub fn evaluate(&mut self, proof_safe: bool)",
    "There is deliberately no reset or renew API",
):
    require(policy, needle, "non-renewable proof gate", "kernel/src/storage_terminal_quarantine.rs")
if re.search(r"\b(?:pub\s+)?fn\s+(?:reset|renew|heartbeat)\b", policy):
    raise SystemExit("M62 pure policy gained a reset/renew/heartbeat method")

expected_tests = (
    "new_gate_is_idle_and_empty",
    "arm_is_exactly_one_way",
    "first_safe_proof_is_deferred_once",
    "unsafe_proof_does_not_consume_the_gate",
    "fresh_safe_proof_after_fallback_is_publishable",
    "safe_proof_without_kernel_arm_fails_closed",
    "duplicate_arm_and_rearm_are_rejected",
    "repeated_unsafe_observations_cannot_renew_or_consume",
)
if policy.count("#[test]") != len(expected_tests):
    raise SystemExit(
        f"M62 pure policy test ledger changed: "
        f"found={policy.count('#[test]')} expected={len(expected_tests)}"
    )
for test in expected_tests:
    require_once(policy, f"fn {test}()", f"host test {test}", "kernel/src/storage_terminal_quarantine.rs")

gate_adapter = between(
    coordinator,
    "fn terminal_proof_gate_irq_masked<T>",
    '#[cfg(feature = "storage-server-owner-liveness-runtime")]\nfn owner_retirement_barrier_active',
    "M62 IRQ-masked gate adapter",
    "kernel/src/storage_server_io.rs",
)
for needle in (
    "irq_is_masked()",
    "TERMINAL_PROOF_GATE.0.get()",
    "terminal_proof_gate_snapshot",
    "save_and_mask_irq()",
    "fn arm_terminal_proof_deferral()",
    "gate.arm()",
):
    require(gate_adapter, needle, "single-core IRQ-masked gate access", "kernel/src/storage_server_io.rs")

attempt_failure = between(
    coordinator,
    "fn finish_fault_policy_attempt_failure(ticket: AttemptTicket, physical_failure: bool) {",
    "/// M60 adds a boot-local bounded policy",
    "fault-policy failure commit",
    "kernel/src/storage_server_io.rs",
)
ordered(
    attempt_failure,
    (
        "policy.complete_physical_failure(ticket, now)",
        "record_policy_disposition(disposition);",
        "FailureDisposition::Offline",
        "if !physical_failure",
        "arm_terminal_proof_deferral();",
        "publish_device_offline_if_ownerless();",
    ),
    "final physical failure->kernel arm->Offline publication",
    "kernel/src/storage_server_io.rs",
)
if coordinator.count("arm_terminal_proof_deferral();") != 1:
    raise SystemExit("M62 proof deferral must have exactly one kernel call site")

terminal_publisher = between(
    coordinator,
    "fn publish_verified_terminal_offline() -> bool {",
    "fn finish_terminal_quarantine(",
    "terminal Offline verifier",
    "kernel/src/storage_server_io.rs",
)
ordered(
    terminal_publisher,
    (
        "rollback_block_irq_rearm_masked(block_irq)",
        "storage::terminal_dma_snapshot()",
        "let safe = !irq.armed",
        "terminal_proof_gate_irq_masked(|gate| gate.evaluate(safe))",
        "TerminalProofDecision::Unsafe | TerminalProofDecision::RunFallback",
        "TerminalProofDecision::Publish",
        "storage_broker::enter_device_offline",
        "TERMINAL_DMA_VERIFICATIONS.fetch_add",
    ),
    "rollback->proof->one-shot deferral->broker Offline",
    "kernel/src/storage_server_io.rs",
)

terminal_fallback = between(
    coordinator,
    "fn finish_terminal_quarantine(",
    "fn finish_fault_policy_attempt_failure(",
    "independent terminal quarantine",
    "kernel/src/storage_server_io.rs",
)
for needle in (
    "TERMINAL_QUARANTINE_STEPS.fetch_add",
    "TERMINAL_QUARANTINE_PENDING_RETURNS.fetch_add",
    "TERMINAL_QUARANTINE_PHYSICAL_ERRORS.fetch_add",
    "TERMINAL_QUARANTINE_TIMER_PROGRESS_WINDOWS.fetch_add",
    "TERMINAL_QUARANTINE_WORKER_PROGRESS_WINDOWS.fetch_add",
    "TERMINAL_QUARANTINE_STARTS.fetch_add",
    "storage::begin_async_recovery",
    "storage::poll_async_recovery",
    "publish_verified_terminal_offline()",
):
    require(terminal_fallback, needle, "cooperative fallback evidence", "kernel/src/storage_server_io.rs")
for forbidden in (
    "begin_attempt(",
    "complete_physical_failure(",
    "complete_probation_failure(",
    "RECOVERY_ATTEMPTS.fetch_add",
    "ASYNC_COORDINATOR_ACTIVE.store(true",
):
    forbid(terminal_fallback, forbidden, "policy-ticket consumption", "kernel/src/storage_server_io.rs")

for relative, text in (
    ("user/init/src/storage_server_runtime.rs", user_runtime),
    ("user/init/src/main.rs", user_main),
):
    forbid(
        text,
        "storage-server-terminal-quarantine-runtime",
        "EL0 M62 behavior/control branch",
        relative,
    )
for forbidden in (
    "TerminalProofGate",
    "arm_terminal_proof_deferral",
    "proof_deferral",
):
    forbid(user_runtime, forbidden, "EL0 terminal proof authority", "user/init/src/storage_server_runtime.rs")

for needle in (
    "io.terminal_quarantine_starts == 1",
    "io.terminal_quarantine_steps == 3",
    "io.terminal_quarantine_pending_returns == 2",
    "io.terminal_quarantine_physical_errors == 1",
    "terminal_proof.arms == 1",
    "terminal_proof.proof_deferrals == 1",
    "terminal_proof.publications == 1",
    "terminal_proof.invariant_errors == 0",
):
    require(syscall, needle, "InitReady M62 seal", "kernel/src/syscall.rs")
    require(main, needle, "monitor M62 seal", "kernel/src/main.rs")

quarantine_markers = re.findall(
    r'"(STORAGE_SERVER_TERMINAL_QUARANTINE_OK [^"\n]+)"', main
)
if len(quarantine_markers) != 1:
    raise SystemExit(
        f"M62 evidence marker is not exact and unique: {len(quarantine_markers)}"
    )
quarantine_marker = quarantine_markers[0]
for needle in (
    "authority=kernel-offline-transition",
    "trigger=simulated-proof-unavailable",
    "gate=nonrenewable",
    "proof_arms={}",
    "proof_deferrals={}",
    "fallback_starts={}",
    "fallback_steps={}",
    "fallback_pending_returns={}",
    "policy_attempt_delta=0 policy_ticket_consumed=0 el0_controls=0",
    "simulated_fault=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0",
    "concurrency_claim=0 smp_claim=0 general_runtime=0 invariant_errors=0",
):
    require(quarantine_marker, needle, "bounded M62 evidence", "kernel/src/main.rs")
for needle in (
    "terminal_quarantine=1 proof_deferral=1",
    "BOOT_OK: M62 terminal StorageServer quarantine fallback and reverified Offline boundary verified",
):
    require_once(main, needle, "M62 runtime/boot marker", "kernel/src/main.rs")

qemu_launches = len(re.findall(r"^qemu-system-aarch64\s+\\$", runtime, re.MULTILINE))
if qemu_launches != 1 or runtime.count("-nic none") != 1:
    raise SystemExit("M62 runtime checker must have one QEMU launch and one -nic none")
for needle in (
    "storage-server-terminal-quarantine-runtime",
    "STORAGE_SERVER_TERMINAL_QUARANTINE_PARSER_OK",
    "negative_cases={cases}",
    "run_evidence_parser --self-test",
    "validate_disk_protection",
    "hardware_claim=0",
    "general_runtime=0",
):
    require(runtime, needle, "runtime parser/launch boundary", "scripts/check-storage-server-terminal-quarantine-runtime.sh")

for needle in (
    '"$SCRIPT_DIR/check-storage-server-terminal-quarantine-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-terminal-quarantine-runtime.sh"',
    "storage_server_terminal_quarantine_static=1",
    "storage_server_terminal_quarantine=1",
):
    require(test_sh, needle, "complete-suite M62 gate", "scripts/test.sh")

print(
    "STORAGE_SERVER_TERMINAL_QUARANTINE_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} host_tests={len(expected_tests)} "
    "feature_chain=M62-M61-M60-M58-M57-M56-M55 pure_policy=1 "
    "nonrenewable=1 kernel_offline_arm=1 el0_controls=0 "
    "fallback_policy_ticket=0 fallback_steps=3 fallback_pending_returns=2 "
    "reverified_dma=1 qemu_launches=1 nic_none=1"
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
  temp="$(mktemp -d "$WORKSPACE_ROOT/target/m62-feature-mismatch.XXXXXX")"
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
    echo "M62 feature mismatch case '$name' was not rejected exactly." >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_build_rejected user_over_m61 \
  storage-server-owner-liveness-runtime \
  storage-server-terminal-quarantine-runtime \
  "userspace terminal-quarantine profile exceeds the kernel storage profile."
expect_build_rejected user_over_m60 \
  storage-server-fault-policy-runtime \
  storage-server-terminal-quarantine-runtime \
  "userspace terminal-quarantine profile exceeds the kernel storage profile."
expect_build_rejected user_over_default \
  "" \
  storage-server-terminal-quarantine-runtime \
  "userspace terminal-quarantine profile exceeds the kernel storage profile."
expect_build_rejected user_over_appdata \
  app-data-runtime \
  storage-server-terminal-quarantine-runtime \
  "userspace terminal-quarantine profile exceeds the kernel storage profile."
expect_build_rejected kernel_with_appdata \
  storage-server-terminal-quarantine-runtime,app-data-runtime \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_build_rejected kernel_with_timeout \
  storage-server-terminal-quarantine-runtime,storage-irq-timeout-self-test \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
POLICY_OUTPUT="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features storage-server-terminal-quarantine-runtime \
    storage_terminal_quarantine:: --lib
)"
if ! grep -Fq "test result: ok. 8 passed; 0 failed;" <<<"$POLICY_OUTPUT"; then
  echo "M62 pure policy test ledger did not pass exactly eight tests." >&2
  echo "$POLICY_OUTPUT" >&2
  exit 1
fi

PARSER_OUTPUT="$("$SCRIPT_DIR/check-storage-server-terminal-quarantine-runtime.sh" --parser-self-test)"
EXPECTED_PARSER="STORAGE_SERVER_TERMINAL_QUARANTINE_PARSER_OK negative_cases=204 base_fields=97 owner_fields=39 quarantine_fields=34 runtime_fields=15"
if [[ "$PARSER_OUTPUT" != "$EXPECTED_PARSER" ]]; then
  echo "M62 parser self-test ledger changed." >&2
  echo "$PARSER_OUTPUT" >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"
printf '%s\n' "STORAGE_SERVER_TERMINAL_QUARANTINE_STATIC_OK feature_mismatch_cases=6 parser_negative_cases=204 parser_base_fields=97 parser_owner_fields=39 parser_quarantine_fields=34 parser_runtime_fields=15 host_tests=8 pure_policy=1 nonrenewable=1 kernel_offline_arm=1 el0_controls=0 fallback_policy_ticket=0 fallback_steps=3 fallback_pending_returns=2 reverified_dma=1 qemu_launches=1 nic_none=1"
