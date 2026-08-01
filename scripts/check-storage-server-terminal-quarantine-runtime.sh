#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

BASE_PREFIX="STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_PREFIX="STORAGE_SERVER_OWNER_LIVENESS_OK"
QUARANTINE_PREFIX="STORAGE_SERVER_TERMINAL_QUARANTINE_OK"
RUNTIME_MARKER="STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250 terminal_quarantine=1 proof_deferral=1"
BOOT_MARKER="BOOT_OK: M62 terminal StorageServer quarantine fallback and reverified Offline boundary verified"

run_evidence_parser() {
  python3 - "$@" <<'PY'
import sys
from pathlib import Path

BASE_PREFIX = "STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_PREFIX = "STORAGE_SERVER_OWNER_LIVENESS_OK"
QUARANTINE_PREFIX = "STORAGE_SERVER_TERMINAL_QUARANTINE_OK"
RUNTIME_PREFIX = "STORAGE_SERVER_RUNTIME_OK"
BOOT = (
    "BOOT_OK: M62 terminal StorageServer quarantine fallback and reverified "
    "Offline boundary verified"
)

BASE_CANONICAL = (
    "STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6 permanent_cases=1 "
    "fault_order=WRFWRFR read_requires_reset=3 mutation_outcome_unknown=4 "
    "transient_control_sequences=6 kernel_permanent_owner_requests=1 "
    "kernel_permanent_fault_arms=1 kernel_permanent_reads=1 "
    "el0_permanent_fault_arm_controls=0 permanent_epoch=7 "
    "permanent_authority=kernel-prearmed injected_reads=3 injected_writes=2 "
    "injected_flushes=2 owner_exits=7 offline_probe_exits=1 broker_releases=7 "
    "broker_abandoned=7 recovery_attempts=9 recovery_commits=6 "
    "recovery_failures=3 recovery_rollbacks=1 fail_closed_retries=1 "
    "physical_successes=7 physical_failures=2 attempt_failures=4 "
    "probation_io_failures=1 attempt_cap=3 backoffs=3 backoffs_completed=3 "
    "backoff_base_ticks=2 backoff_multiplier=2 backoff_ticks=8 probation=7/5/2 "
    "healthy_transitions=5 device_offline=1 offline_transitions=1 "
    "offline_denials=1 early_rejections=1 early_physical_starts=0 "
    "offline_rejections=0 stale_ticket_rejections=0 "
    "invalid_transition_rejections=0 generation_exhaustions=0 "
    "persistent_fault_armed=1 persistent_fault_hits=3 driver_timeouts=7 "
    "driver_resets=11 async_starts=9 physical_completions=7 "
    "async_physical_failures=2 async_steps=38 recovery_yields=28 "
    "timer_progress_windows=9 worker_progress_windows=9 el0_progress_windows=9 "
    "backoff_timer_progress_windows=3 backoff_worker_progress_windows=3 "
    "acquire_waits=26 acquire_dispatch_changes=26 terminal_quarantine_starts=1 "
    "terminal_quarantine_completions=0 terminal_quarantine_physical_errors=1 "
    "terminal_irq_rollbacks=2 terminal_dma_verifications=1 "
    "terminal_driver_state=2 terminal_transport_status=0 terminal_in_flight=0 "
    "masked_poll_iterations=0 max_step_masked_ticks=38438 "
    "max_control_masked_ticks=35250 timer_period_ticks=625000 long_daif_masks=0 "
    "offline_quiet_ticks=16 post_offline_attempt_delta=0 "
    "post_offline_reset_delta=0 post_offline_submission_delta=0 final_epoch=0 "
    "next_epoch=8 final_generation=1 final_irq_armed=0 final_irq_failed=1 "
    "final_recovery_required=1 final_admission_open=0 final_broker_state=4 "
    "final_broker_bound=0 final_broker_pending=0 requests=9023 completions=9016 "
    "simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 "
    "powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0"
)
OWNER_CANONICAL = (
    "STORAGE_SERVER_OWNER_LIVENESS_OK authority=kernel-fatal-completion "
    "ticket=pid-epoch-lease-generation deadline=physical-counter grace_ms=250 "
    "grace_counter_units=15625000 arms=7 cooperative_retirements=6 "
    "deadlines_expired=1 early_expirations=0 termination_requests=1 "
    "already_terminal=0 forced_retirements=1 forced_reaps=1 "
    "terminal_race_retirements=0 stalled_epoch=6 stalled_operation=flush "
    "stalled_completion=outcome-unknown live_volume_close=denied "
    "target_wait=single object_wait_abandoned=1 el0_process_terminate_calls=0 "
    "terminated_exited=7 terminated_killed=1 recovery_after_forced_retirement=1 "
    "replacement_epoch=7 final_phase=0 final_owner_pid=0 final_broker_epoch=0 "
    "next_lease_generation=8 el0_arm_controls=0 el0_renew_controls=0 "
    "simulated_fault=1 hardware_claim=0 smp_claim=0 arbitrary_soak_claim=0 "
    "powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0"
)
QUARANTINE_CANONICAL = (
    "STORAGE_SERVER_TERMINAL_QUARANTINE_OK authority=kernel-offline-transition "
    "trigger=simulated-proof-unavailable gate=nonrenewable gate_phase=2 "
    "proof_arms=1 proof_deferrals=1 unsafe_observations=0 proof_publications=1 "
    "fallback_starts=1 fallback_steps=3 fallback_pending_returns=2 "
    "fallback_completions=0 fallback_physical_errors=1 "
    "fallback_timer_progress_windows=1 fallback_worker_progress_windows=1 "
    "persistent_fault_hits=3 terminal_irq_rollbacks=2 "
    "terminal_dma_verifications=1 final_driver_state=2 "
    "final_transport_status=0 final_in_flight=0 final_recovery_active=0 "
    "final_requests_terminal=1 policy_attempt_delta=0 policy_ticket_consumed=0 "
    "el0_controls=0 simulated_fault=1 hardware_claim=0 arbitrary_soak_claim=0 "
    "powercut_claim=0 concurrency_claim=0 smp_claim=0 general_runtime=0 "
    "invariant_errors=0"
)
RUNTIME_CANONICAL = (
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 "
    "volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 "
    "repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 "
    "device_offline=1 owner_liveness=1 exit_grace_ms=250 "
    "terminal_quarantine=1 proof_deferral=1"
)

DYNAMIC_FIELDS = {
    BASE_PREFIX: {
        "acquire_waits",
        "acquire_dispatch_changes",
        "max_step_masked_ticks",
        "max_control_masked_ticks",
    },
    OWNER_PREFIX: set(),
    QUARANTINE_PREFIX: set(),
    RUNTIME_PREFIX: set(),
}


class EvidenceError(Exception):
    pass


def schema(canonical: str, prefix: str):
    tokens = canonical.split()
    if tokens[0] != prefix:
        raise AssertionError("bad parser fixture prefix")
    parsed = []
    for token in tokens[1:]:
        if token.count("=") != 1:
            raise AssertionError(f"bad parser fixture token: {token}")
        key, value = token.split("=", 1)
        parsed.append((key, None if key in DYNAMIC_FIELDS[prefix] else value))
    return parsed


SCHEMAS = {
    BASE_PREFIX: schema(BASE_CANONICAL, BASE_PREFIX),
    OWNER_PREFIX: schema(OWNER_CANONICAL, OWNER_PREFIX),
    QUARANTINE_PREFIX: schema(QUARANTINE_CANONICAL, QUARANTINE_PREFIX),
    RUNTIME_PREFIX: schema(RUNTIME_CANONICAL, RUNTIME_PREFIX),
}
CANONICALS = {
    BASE_PREFIX: BASE_CANONICAL,
    OWNER_PREFIX: OWNER_CANONICAL,
    QUARANTINE_PREFIX: QUARANTINE_CANONICAL,
    RUNTIME_PREFIX: RUNTIME_CANONICAL,
}


def parse_marker(text: str, prefix: str):
    lines = [line for line in text.splitlines() if line.startswith(prefix + " ")]
    if len(lines) != 1:
        raise EvidenceError(f"{prefix} cardinality={len(lines)}")
    tokens = lines[0].split()
    fields = []
    for token in tokens[1:]:
        if token.count("=") != 1:
            raise EvidenceError(f"{prefix} malformed token={token!r}")
        key, value = token.split("=", 1)
        if not key or not value:
            raise EvidenceError(f"{prefix} empty key/value")
        fields.append((key, value))
    expected = SCHEMAS[prefix]
    if [key for key, _ in fields] != [key for key, _ in expected]:
        raise EvidenceError(f"{prefix} field order/schema changed")
    values = dict(fields)
    for key, exact in expected:
        if exact is not None and values[key] != exact:
            raise EvidenceError(f"{prefix} {key}={values[key]!r}, expected={exact!r}")
    return values


def integer(values, key):
    try:
        return int(values[key], 10)
    except ValueError as error:
        raise EvidenceError(f"{key} is not decimal") from error


def validate_text(text: str):
    values = {prefix: parse_marker(text, prefix) for prefix in SCHEMAS}
    boots = [line for line in text.splitlines() if line.startswith("BOOT_OK:")]
    if boots != [BOOT]:
        raise EvidenceError(f"BOOT_OK schema changed: {boots!r}")
    base = values[BASE_PREFIX]
    waits = integer(base, "acquire_waits")
    dispatches = integer(base, "acquire_dispatch_changes")
    if waits not in (25, 26) or dispatches != waits:
        raise EvidenceError("M62 acquire wait/dispatch sample escaped {25,26} equality")
    timer = integer(base, "timer_period_ticks")
    step_mask = integer(base, "max_step_masked_ticks")
    control_mask = integer(base, "max_control_masked_ticks")
    if timer != 625000 or not (0 < step_mask < timer) or not (0 < control_mask < timer):
        raise EvidenceError("M62 DAIF-bound sample is invalid")
    if integer(base, "requests") != integer(base, "completions") + 7:
        raise EvidenceError("M62 request/completion fatal delta is not seven")
    stale_prefixes = (
        "STORAGE_SERVER_RECOVERY_OK ",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK ",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK ",
        "APPDATA_ASYNC_RECOVERY_OK ",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK ",
    )
    if any(line.startswith(stale_prefixes) for line in text.splitlines()):
        raise EvidenceError("M62 log contains a stale recovery success marker")
    return values


def expect_rejected(text: str):
    try:
        validate_text(text)
    except EvidenceError:
        return
    raise AssertionError("negative M62 parser fixture was accepted")


def self_test():
    canonical_lines = [
        BASE_CANONICAL,
        OWNER_CANONICAL,
        QUARANTINE_CANONICAL,
        RUNTIME_CANONICAL,
        BOOT,
    ]
    canonical = "\n".join(canonical_lines) + "\n"
    validate_text(canonical)
    cases = 0
    dynamic_bad = {
        "acquire_waits": "24",
        "acquire_dispatch_changes": "24",
        "max_step_masked_ticks": "0",
        "max_control_masked_ticks": "625000",
    }
    for prefix, expected in SCHEMAS.items():
        line_index = list(SCHEMAS).index(prefix)
        for key, exact in expected:
            lines = list(canonical_lines)
            old = next(token for token in lines[line_index].split()[1:] if token.startswith(key + "="))
            bad = dynamic_bad.get(key, "__bad__" if exact is not None else "0")
            lines[line_index] = lines[line_index].replace(old, f"{key}={bad}", 1)
            expect_rejected("\n".join(lines) + "\n")
            cases += 1
        for mutation in ("remove", "extra", "duplicate", "prefix"):
            lines = list(canonical_lines)
            if mutation == "remove":
                tokens = lines[line_index].split()
                lines[line_index] = " ".join(tokens[:-1])
            elif mutation == "extra":
                lines[line_index] += " extra=1"
            elif mutation == "duplicate":
                lines.insert(line_index, lines[line_index])
            else:
                lines[line_index] = "STALE_" + lines[line_index]
            expect_rejected("\n".join(lines) + "\n")
            cases += 1
    for mutation in ("missing", "duplicate", "stale"):
        lines = list(canonical_lines)
        if mutation == "missing":
            lines.pop()
        elif mutation == "duplicate":
            lines.append(BOOT)
        else:
            lines[-1] = "BOOT_OK: M61 stale"
        expect_rejected("\n".join(lines) + "\n")
        cases += 1
    print(
        "STORAGE_SERVER_TERMINAL_QUARANTINE_PARSER_OK "
        f"negative_cases={cases} base_fields={len(SCHEMAS[BASE_PREFIX])} "
        f"owner_fields={len(SCHEMAS[OWNER_PREFIX])} "
        f"quarantine_fields={len(SCHEMAS[QUARANTINE_PREFIX])} "
        f"runtime_fields={len(SCHEMAS[RUNTIME_PREFIX])}"
    )


if len(sys.argv) != 2:
    raise SystemExit("usage: parser (--self-test | LOG)")
if sys.argv[1] == "--self-test":
    self_test()
else:
    validate_text(Path(sys.argv[1]).read_text(encoding="utf-8"))
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  run_evidence_parser --self-test
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--parser-self-test]" >&2
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M62 terminal quarantine." >&2
    exit 1
  fi
done
if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M62 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be an integer from 1 through 600." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi
if [[ "$KEEP_TEMP" != "0" && "$KEEP_TEMP" != "1" ]]; then
  echo "BNDROID_KEEP_TEMP must be 0 or 1." >&2
  exit 2
fi

FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime,storage-server-async-recovery-runtime,storage-server-fault-policy-runtime,storage-server-owner-liveness-runtime,storage-server-terminal-quarantine-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_TERMINAL_QUARANTINE_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-terminal-quarantine-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m62"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m62.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
SERIAL_LOG="$TMP_DIR/qemu.serial.log"
NORMALIZED_LOG="$TMP_DIR/qemu.log"
QEMU_PID=""
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M62 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  [[ -f "$NORMALIZED_LOG" ]] && tail -n 360 "$NORMALIZED_LOG" >&2 || true
}

reject_failure() {
  [[ -f "$NORMALIZED_LOG" ]] || return
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M6[0-2]_[A-Z0-9_]*TIMEOUT|^BOOT_OK: M(5[5-9]|60|61) ' "$NORMALIZED_LOG"; then
    show_failure
    echo "M62 emitted a panic, failure, timeout, or stale M55-M61 BOOT marker." >&2
    exit 1
  fi
  if grep '^STORAGE_SERVER_RUNTIME_OK' "$NORMALIZED_LOG" | grep -Fvx "$RUNTIME_MARKER" >/dev/null; then
    show_failure
    echo "M62 published stale or malformed runtime evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M62 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_disk_protection() {
  python3 - "$PRISTINE_IMAGE" "$RUNTIME_IMAGE" <<'PY'
import hashlib
import sys
from pathlib import Path

before_path, after_path = (Path(argument) for argument in sys.argv[1:])
before = before_path.read_bytes()
after = after_path.read_bytes()
if len(before) != 8 * 1024 * 1024 or len(after) != 8 * 1024 * 1024:
    raise SystemExit("M62 runtime disk lost its exact writable 8 MiB geometry")
sector = 512
if before[: 64 * sector] != after[: 64 * sector] or before[2048 * sector :] != after[2048 * sector :]:
    raise SystemExit("M62 QEMU changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
if before[128 * sector : 2048 * sector] == after[128 * sector : 2048 * sector]:
    raise SystemExit("M62 runtime did not persist its AppData campaign")
if hashlib.sha256(before).digest() == hashlib.sha256(after).digest():
    raise SystemExit("M62 writable runtime image retained its pristine whole-disk hash")
PY
}

run_evidence_parser --self-test >/dev/null
python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M62 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

: >"$SERIAL_LOG"
: >"$NORMALIZED_LOG"
qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu "$QEMU_CPU" \
  -smp 1 \
  -m 128M \
  -display none \
  -monitor none \
  -nic none \
  -serial stdio \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  -global virtio-mmio.force-legacy=false \
  -drive "if=none,file=$RUNTIME_IMAGE,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
  -device "virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off" \
  >"$SERIAL_LOG" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  base_count="$(grep -Ec "^${BASE_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  owner_count="$(grep -Ec "^${OWNER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  quarantine_count="$(grep -Ec "^${QUARANTINE_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  runtime_count="$(grep -Fxc "$RUNTIME_MARKER" "$NORMALIZED_LOG" || true)"
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$base_count" == 1 && "$owner_count" == 1 && "$quarantine_count" == 1 \
    && "$runtime_count" == 1 && "$boot_count" == 1 ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    run_evidence_parser "$NORMALIZED_LOG"
    validate_disk_protection
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-server-terminal-quarantine-runtime.log"
    RUN_SUCCEEDED=1
    echo "M62 StorageServer terminal-quarantine runtime self-test passed."
    exit 0
  fi
  if ((base_count > 1 || owner_count > 1 || quarantine_count > 1 || runtime_count > 1 || boot_count > 1)); then
    show_failure
    echo "M62 published duplicate base, owner, quarantine, runtime, or BOOT evidence." >&2
    exit 1
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    qemu_status=$?
    set -e
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    show_failure
    echo "QEMU exited before M62 terminal-quarantine evidence (status $qemu_status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M62 StorageServer terminal-quarantine evidence." >&2
exit 1
