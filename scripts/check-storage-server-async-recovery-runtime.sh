#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

BOOT_MARKER="BOOT_OK: M58 cooperative fail-stop StorageServer recovery verified"
RECOVERY_MARKER_PREFIX="STORAGE_SERVER_ASYNC_RECOVERY_OK"
RUNTIME_MARKER="STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1"

# The runtime and its offline negative self-test use this exact parser body.
# Keeping it in one function prevents the self-test from validating a weaker
# duplicate of the production log contract.
run_evidence_parser() {
  python3 - "$@" <<'PY'
import re
import sys
from pathlib import Path

BOOT_MARKER = "BOOT_OK: M58 cooperative fail-stop StorageServer recovery verified"
RUNTIME_MARKER = (
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 "
    "volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 "
    "repeated_recovery=1 async_recovery=1"
)

RECOVERY_PATTERN = re.compile(
    r"^STORAGE_SERVER_ASYNC_RECOVERY_OK "
    r"cycles=2 cases=6 fault_order=WRFWRF "
    r"read_requires_reset=2 mutation_outcome_unknown=4 "
    r"control_sequences=6 injected_reads=2 injected_writes=2 injected_flushes=2 "
    r"owner_exits=6 broker_releases=6 broker_abandoned=6 "
    r"recovery_attempts=7 recovery_commits=6 recovery_rollbacks=1 "
    r"injected_rearm_aborts=1 fail_closed_retries=1 "
    r"driver_timeouts=6 driver_resets=7 "
    r"async_starts=7 physical_completions=7 "
    r"async_steps=(?P<async_steps>[1-9][0-9]*) "
    r"recovery_yields=(?P<recovery_yields>[1-9][0-9]*) "
    r"timer_progress_windows=(?P<timer_progress_windows>[1-9][0-9]*) "
    r"worker_progress_windows=(?P<worker_progress_windows>[1-9][0-9]*) "
    r"el0_progress_windows=(?P<el0_progress_windows>[1-9][0-9]*) "
    r"acquire_waits=(?P<acquire_waits>[1-9][0-9]*) "
    r"acquire_dispatch_changes=(?P<acquire_dispatch_changes>[1-9][0-9]*) "
    r"masked_poll_iterations=0 "
    r"max_step_masked_ticks=(?P<max_step_masked_ticks>[0-9]+) "
    r"max_control_masked_ticks=(?P<max_control_masked_ticks>[0-9]+) "
    r"timer_period_ticks=(?P<timer_period_ticks>[1-9][0-9]*) "
    r"long_daif_masks=0 final_epoch=7 "
    r"final_generation=(?P<final_generation>[1-9][0-9]*) "
    r"requests=(?P<requests>[1-9][0-9]*) "
    r"completions=(?P<completions>[1-9][0-9]*) invariant_errors=0$"
)


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def validate(lines: list[str]) -> None:
    stale_prefixes = (
        "STORAGE_SERVER_RECOVERY_OK",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "BOOT_OK: M56",
        "BOOT_OK: M57",
        "BOOT_OK: M60",
    )
    reject(
        any(any(line == prefix or line.startswith(prefix + " ") for prefix in stale_prefixes) for line in lines),
        "M58 checker observed stale M56/M57/M59 evidence",
    )

    recovery_lines = [
        line
        for line in lines
        if line == "STORAGE_SERVER_ASYNC_RECOVERY_OK"
        or line.startswith("STORAGE_SERVER_ASYNC_RECOVERY_OK ")
    ]
    reject(len(recovery_lines) != 1, "M58 async recovery evidence was not unique")
    reject(lines.count(RUNTIME_MARKER) != 1, "M58 runtime marker was not exact and unique")
    reject(lines.count(BOOT_MARKER) != 1, "M58 BOOT_OK evidence was not unique")

    runtime_lines = [
        line
        for line in lines
        if line == "STORAGE_SERVER_RUNTIME_OK"
        or line.startswith("STORAGE_SERVER_RUNTIME_OK ")
    ]
    reject(runtime_lines != [RUNTIME_MARKER], "M58 ABI/runtime evidence changed or duplicated")
    unrelated_boots = [line for line in lines if line.startswith("BOOT_OK:") and line != BOOT_MARKER]
    reject(bool(unrelated_boots), "M58 log contained a stale or unrelated BOOT_OK marker")

    recovery = recovery_lines[0]
    reject(
        not (lines.index(recovery) < lines.index(RUNTIME_MARKER) < lines.index(BOOT_MARKER)),
        "M58 recovery, runtime, and BOOT_OK evidence was out of order",
    )
    match = RECOVERY_PATTERN.fullmatch(recovery)
    reject(match is None, "M58 async recovery evidence changed its exact field contract")
    assert match is not None
    values = {name: int(value) for name, value in match.groupdict().items()}

    # Every successful physical attempt has exactly one non-Pending step; all
    # other cooperative driver calls must return to the IRQ-enabled monitor.
    reject(
        values["async_steps"] != values["recovery_yields"] + 7,
        "M58 step/yield ledger did not isolate seven terminal physical steps",
    )
    reject(
        values["recovery_yields"] < 21,
        "M58 did not preserve three scheduling boundaries per physical attempt",
    )

    # Every physical attempt must independently span real timer dispatches
    # while both unrelated workers and authenticated EL0 acquisition retries
    # make progress.
    for field in (
        "timer_progress_windows",
        "worker_progress_windows",
        "el0_progress_windows",
    ):
        reject(values[field] != 7, f"M58 {field} did not cover all seven windows")
    reject(
        values["acquire_waits"] < 14,
        "M58 EL0 progress windows lacked two authenticated acquire waits",
    )
    reject(
        values["acquire_dispatch_changes"] < 14
        or values["acquire_dispatch_changes"] > values["acquire_waits"],
        "M58 EL0 acquire retries did not span distinct timer dispatches",
    )

    period = values["timer_period_ticks"]
    reject(
        values["max_step_masked_ticks"] >= period,
        "M58 cooperative driver held DAIF across a full timer period",
    )
    reject(
        values["max_control_masked_ticks"] >= period,
        "M58 recovery control commit held DAIF across a full timer period",
    )
    reject(
        values["requests"] != values["completions"] + 6,
        "M58 request ledger did not isolate exactly six timed-out requests",
    )


def canonical_lines() -> list[str]:
    recovery = (
        "STORAGE_SERVER_ASYNC_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF "
        "read_requires_reset=2 mutation_outcome_unknown=4 control_sequences=6 "
        "injected_reads=2 injected_writes=2 injected_flushes=2 owner_exits=6 "
        "broker_releases=6 broker_abandoned=6 recovery_attempts=7 recovery_commits=6 "
        "recovery_rollbacks=1 injected_rearm_aborts=1 fail_closed_retries=1 "
        "driver_timeouts=6 driver_resets=7 async_starts=7 physical_completions=7 "
        "async_steps=42 recovery_yields=35 timer_progress_windows=7 "
        "worker_progress_windows=7 el0_progress_windows=7 acquire_waits=21 "
        "acquire_dispatch_changes=14 masked_poll_iterations=0 "
        "max_step_masked_ticks=20 max_control_masked_ticks=30 timer_period_ticks=100 "
        "long_daif_masks=0 final_epoch=7 final_generation=1 requests=100 "
        "completions=94 invariant_errors=0"
    )
    return [recovery, RUNTIME_MARKER, BOOT_MARKER]


def replace(lines: list[str], old: str, new: str) -> list[str]:
    changed = list(lines)
    changed[0] = changed[0].replace(old, new, 1)
    if changed[0] == lines[0]:
        raise AssertionError(f"self-test mutation did not match {old!r}")
    return changed


def self_test() -> None:
    canonical = canonical_lines()
    validate(canonical)
    negative = [
        ("timer_progress", replace(canonical, "timer_progress_windows=7", "timer_progress_windows=6")),
        ("worker_progress", replace(canonical, "worker_progress_windows=7", "worker_progress_windows=6")),
        ("el0_progress", replace(canonical, "el0_progress_windows=7", "el0_progress_windows=6")),
        (
            "yield_progress",
            replace(
                replace(canonical, "async_steps=42", "async_steps=27"),
                "recovery_yields=35",
                "recovery_yields=20",
            ),
        ),
        ("step_yield_relation", replace(canonical, "async_steps=42", "async_steps=41")),
        (
            "authenticated_wait",
            replace(
                replace(canonical, "acquire_waits=21", "acquire_waits=13"),
                "acquire_dispatch_changes=14",
                "acquire_dispatch_changes=13",
            ),
        ),
        ("masked_poll", replace(canonical, "masked_poll_iterations=0", "masked_poll_iterations=1")),
        ("step_mask_bound", replace(canonical, "max_step_masked_ticks=20", "max_step_masked_ticks=100")),
        ("control_mask_bound", replace(canonical, "max_control_masked_ticks=30", "max_control_masked_ticks=100")),
        ("long_daif", replace(canonical, "long_daif_masks=0", "long_daif_masks=1")),
        ("request_delta", replace(canonical, "requests=100", "requests=99")),
        ("duplicate", [canonical[0], canonical[0], canonical[1], canonical[2]]),
        ("ordering", [canonical[2], canonical[0], canonical[1]]),
        (
            "stale_marker",
            canonical + ["STORAGE_SERVER_REPEATED_RECOVERY_OK cycles=2"],
        ),
        (
            "appdata_async_marker",
            canonical + ["APPDATA_ASYNC_RECOVERY_OK recoveries=1"],
        ),
        (
            "storage_irq_cooperative_marker",
            canonical + ["STORAGE_IRQ_COOPERATIVE_RECOVERY_OK recoveries=1"],
        ),
        (
            "m60_fault_policy_marker",
            canonical + ["STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6"],
        ),
        (
            "m60_boot_marker",
            canonical + ["BOOT_OK: M60 bounded StorageServer fault policy verified"],
        ),
    ]
    for name, lines in negative:
        try:
            validate(lines)
        except EvidenceError:
            continue
        raise SystemExit(f"M58 parser negative self-test accepted {name}")
    print("STORAGE_SERVER_ASYNC_RECOVERY_PARSER_OK negative_cases=18")


if len(sys.argv) < 2:
    raise SystemExit("M58 evidence parser mode is missing")
mode = sys.argv[1]
if mode == "--self-test":
    if len(sys.argv) != 2:
        raise SystemExit("M58 parser self-test takes no path")
    self_test()
elif mode == "--validate":
    if len(sys.argv) != 3:
        raise SystemExit("M58 parser validation requires exactly one log path")
    validate(Path(sys.argv[2]).read_text(encoding="utf-8").splitlines())
else:
    raise SystemExit(f"unknown M58 evidence parser mode: {mode}")
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  if [[ "$#" -ne 1 ]]; then
    echo "--parser-self-test takes no additional arguments." >&2
    exit 2
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found; cannot run the M58 parser self-test." >&2
    exit 1
  fi
  run_evidence_parser --self-test
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--parser-self-test]" >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill mkdir rm sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M58 asynchronous StorageServer recovery runtime." >&2
    exit 1
  fi
done

PROFILE="${BNDROID_PROFILE:-release}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-240}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi
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

FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime,storage-server-async-recovery-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_ASYNC_RECOVERY_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-async-recovery-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m58"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m58.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
SERIAL_LOG="$TMP_DIR/qemu.serial.log"
NORMALIZED_LOG="$TMP_DIR/qemu.log"
QEMU_PID=""
RUN_SUCCEEDED=0

cleanup() {
  local exit_status
  exit_status="$1"
  trap - EXIT INT TERM
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M58 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 260 "$NORMALIZED_LOG" >&2 || true
  fi
}

reject_failure() {
  if [[ ! -f "$NORMALIZED_LOG" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M58_[A-Z0-9_]*TIMEOUT|STORAGE_SERVER_ASYNC_RECOVERY_TIMEOUT|^STORAGE_SERVER_RECOVERY_OK( |$)|^STORAGE_SERVER_REPEATED_RECOVERY_OK( |$)|^STORAGE_SERVER_FAULT_POLICY_OK( |$)|^APPDATA_ASYNC_RECOVERY_OK( |$)|^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK( |$)|^BOOT_OK: M56 |^BOOT_OK: M57 |^BOOT_OK: M60( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M58 emitted a panic, failure, timeout, or stale M56/M57 marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M58 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_recovery_evidence() {
  if ! run_evidence_parser --validate "$NORMALIZED_LOG"; then
    show_failure
    echo "M58 evidence did not match its cooperative recovery/timing schema." >&2
    exit 1
  fi
}

# Prove the parser rejects its negative fixtures before trusting it with guest
# evidence. This is CPU-local, offline, and does not touch the QEMU process.
run_evidence_parser --self-test >/dev/null

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
python3 - "$RUNTIME_IMAGE" <<'PY'
import sys
from pathlib import Path

image = Path(sys.argv[1])
if image.stat().st_size != 8 * 1024 * 1024:
    raise SystemExit("M58 runtime disk lost the exact writable 8 MiB geometry")
PY

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M58 kernel image is missing: $KERNEL_IMAGE" >&2
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
  recovery_count="$(grep -Ec "^${RECOVERY_MARKER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  runtime_count="$(grep -Fxc "$RUNTIME_MARKER" "$NORMALIZED_LOG" || true)"
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$recovery_count" == "1" && "$runtime_count" == "1" && "$boot_count" == "1" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    validate_recovery_evidence
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-server-async-recovery-runtime.log"
    RUN_SUCCEEDED=1
    echo "M58 cooperative fail-stop StorageServer recovery runtime self-test passed."
    exit 0
  fi
  if ((recovery_count > 1 || runtime_count > 1 || boot_count > 1)); then
    show_failure
    echo "M58 published duplicate recovery, runtime, or BOOT_OK evidence." >&2
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
    echo "QEMU exited before M58 cooperative recovery evidence (status $qemu_status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M58 cooperative StorageServer recovery evidence." >&2
exit 1
