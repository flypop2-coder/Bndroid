#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

BOOT_MARKER="BOOT_OK: M32 transferable graphics buffers, M22 storage IRQ timeout/reset self-test, and M20 multi-session services verified"
RACE_MARKER="STORAGE_IRQ_RACE_OK timeout_requests=2 timeouts=1 resets=1 reset_tokens_invalidated=2 simulated_late_irq=1 spurious_acked=1 recovered_requests=2 recovered_completions=2 double_completions=0 dma_frames_before=2 dma_frames_after=2 digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c"
COOPERATIVE_MARKER_PREFIX="STORAGE_IRQ_COOPERATIVE_RECOVERY_OK"

# The live checker and its offline fixtures deliberately share one parser. A
# full-line schema prevents reordered, omitted, duplicated, or extra fields
# from being accepted as M59 evidence.
run_evidence_parser() {
  python3 - "$@" <<'PY'
import re
import sys
from pathlib import Path

BOOT_MARKER = (
    "BOOT_OK: M32 transferable graphics buffers, M22 storage IRQ "
    "timeout/reset self-test, and M20 multi-session services verified"
)
RACE_MARKER = (
    "STORAGE_IRQ_RACE_OK timeout_requests=2 timeouts=1 resets=1 "
    "reset_tokens_invalidated=2 simulated_late_irq=1 spurious_acked=1 "
    "recovered_requests=2 recovered_completions=2 double_completions=0 "
    "dma_frames_before=2 dma_frames_after=2 "
    "digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c"
)

COOPERATIVE_PATTERN = re.compile(
    r"^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK "
    r"starts=1 physical=1 "
    r"steps=(?P<steps>[1-9][0-9]*) "
    r"yields=(?P<yields>[1-9][0-9]*) "
    r"step_yield_relation=1 "
    r"timer_progress_windows=(?P<timer_progress_windows>[1-9][0-9]*) "
    r"worker_progress_windows=(?P<worker_progress_windows>[1-9][0-9]*) "
    r"timer_dispatches=(?P<timer_dispatches>[1-9][0-9]*) "
    r"worker0_work=(?P<worker0_work>[1-9][0-9]*) "
    r"worker1_work=(?P<worker1_work>[1-9][0-9]*) "
    r"masked_poll_iterations=0 "
    r"max_step_masked_ticks=(?P<max_step_masked_ticks>[1-9][0-9]*) "
    r"max_control_masked_ticks=(?P<max_control_masked_ticks>[1-9][0-9]*) "
    r"timer_period_ticks=(?P<timer_period_ticks>[1-9][0-9]*) "
    r"long_daif_masks=0 final_active=0 gate_open=1 el0_progress_claim=0$"
)


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def has_prefix(line: str, prefix: str) -> bool:
    return line == prefix or line.startswith(prefix + " ")


def validate(lines: list[str]) -> None:
    stale_prefixes = (
        "APPDATA_ASYNC_RECOVERY_OK",
        "APPDATA_RUNTIME_OK",
        "STORAGE_SERVER_RUNTIME_OK",
        "STORAGE_SERVER_RECOVERY_OK",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "BOOT_OK: M54",
        "BOOT_OK: M56",
        "BOOT_OK: M57",
        "BOOT_OK: M58",
        "BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified",
        "BOOT_OK: M60",
    )
    reject(
        any(has_prefix(line, prefix) for line in lines for prefix in stale_prefixes),
        "M59 low-level checker observed AppData or M56-M60 evidence",
    )

    cooperative_lines = [
        line
        for line in lines
        if has_prefix(line, "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK")
    ]
    race_lines = [line for line in lines if has_prefix(line, "STORAGE_IRQ_RACE_OK")]
    reject(
        len(cooperative_lines) != 1,
        "cooperative recovery evidence was not unique",
    )
    reject(
        race_lines != [RACE_MARKER],
        "legacy M22 race evidence changed or was duplicated",
    )
    reject(lines.count(BOOT_MARKER) != 1, "M22 BOOT_OK evidence was not unique")

    unrelated_boots = [
        line for line in lines if line.startswith("BOOT_OK:") and line != BOOT_MARKER
    ]
    reject(bool(unrelated_boots), "log contained a stale or unrelated BOOT_OK marker")

    cooperative = cooperative_lines[0]
    reject(
        not (
            lines.index(cooperative)
            < lines.index(RACE_MARKER)
            < lines.index(BOOT_MARKER)
        ),
        "cooperative, legacy race, and BOOT_OK evidence was out of order",
    )
    match = COOPERATIVE_PATTERN.fullmatch(cooperative)
    reject(match is None, "cooperative evidence changed its exact field contract")
    assert match is not None
    values = {name: int(value) for name, value in match.groupdict().items()}

    # One successful physical attempt has one terminal step. Every Pending
    # phase must return to the IRQ-enabled scheduler before the next phase.
    reject(values["steps"] < 4, "cooperative recovery omitted a physical phase")
    reject(values["yields"] < 3, "cooperative recovery omitted a scheduling boundary")
    reject(
        values["steps"] != values["yields"] + 1,
        "step/yield ledger did not isolate one terminal physical step",
    )

    # Every Pending return must span a timer window in which both unrelated
    # worker classes remain live. The raw scheduler ledgers are additional,
    # non-zero proof rather than substitutes for the per-yield windows.
    reject(
        values["timer_progress_windows"] != values["yields"],
        "timer progress did not cover every cooperative yield",
    )
    reject(
        values["worker_progress_windows"] != values["yields"],
        "worker progress did not cover every cooperative yield",
    )
    reject(
        values["timer_dispatches"] < values["yields"],
        "timer dispatch ledger did not cover every cooperative yield",
    )
    reject(
        values["worker0_work"] == 0 or values["worker1_work"] == 0,
        "both unrelated workers did not make progress",
    )

    period = values["timer_period_ticks"]
    reject(
        values["max_step_masked_ticks"] >= period,
        "a cooperative physical step held DAIF across a timer period",
    )
    reject(
        values["max_control_masked_ticks"] >= period,
        "recovery control work held DAIF across a timer period",
    )


def canonical_lines() -> list[str]:
    cooperative = (
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK starts=1 physical=1 "
        "steps=5 yields=4 step_yield_relation=1 timer_progress_windows=4 "
        "worker_progress_windows=4 timer_dispatches=5 worker0_work=2 "
        "worker1_work=3 masked_poll_iterations=0 max_step_masked_ticks=20 "
        "max_control_masked_ticks=30 timer_period_ticks=100 "
        "long_daif_masks=0 final_active=0 gate_open=1 el0_progress_claim=0"
    )
    return [cooperative, RACE_MARKER, BOOT_MARKER]


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
        ("starts", replace(canonical, "starts=1", "starts=2")),
        ("physical", replace(canonical, "physical=1", "physical=2")),
        ("minimum_steps", replace(canonical, "steps=5", "steps=3")),
        (
            "minimum_yields",
            replace(
                replace(canonical, "steps=5", "steps=3"),
                "yields=4",
                "yields=2",
            ),
        ),
        ("step_yield_relation_field", replace(canonical, "step_yield_relation=1", "step_yield_relation=0")),
        ("step_yield_relation", replace(canonical, "steps=5", "steps=6")),
        ("timer_window", replace(canonical, "timer_progress_windows=4", "timer_progress_windows=3")),
        ("worker_window", replace(canonical, "worker_progress_windows=4", "worker_progress_windows=3")),
        ("timer_dispatch", replace(canonical, "timer_dispatches=5", "timer_dispatches=3")),
        ("worker0_progress", replace(canonical, "worker0_work=2", "worker0_work=0")),
        ("worker1_progress", replace(canonical, "worker1_work=3", "worker1_work=0")),
        ("masked_poll", replace(canonical, "masked_poll_iterations=0", "masked_poll_iterations=1")),
        ("step_mask_bound", replace(canonical, "max_step_masked_ticks=20", "max_step_masked_ticks=100")),
        ("control_mask_bound", replace(canonical, "max_control_masked_ticks=30", "max_control_masked_ticks=100")),
        ("long_daif", replace(canonical, "long_daif_masks=0", "long_daif_masks=1")),
        ("final_active", replace(canonical, "final_active=0", "final_active=1")),
        ("gate_closed", replace(canonical, "gate_open=1", "gate_open=0")),
        ("el0_claim", replace(canonical, "el0_progress_claim=0", "el0_progress_claim=1")),
        ("extra_field", replace(canonical, " el0_progress_claim=0", " extra=1 el0_progress_claim=0")),
        ("legacy_changed", [canonical[0], RACE_MARKER.replace("timeouts=1", "timeouts=2"), canonical[2]]),
        ("duplicate", [canonical[0], canonical[0], canonical[1], canonical[2]]),
        ("ordering", [canonical[1], canonical[0], canonical[2]]),
        ("appdata", canonical + ["APPDATA_ASYNC_RECOVERY_OK cases=1"]),
        ("m56", canonical + ["STORAGE_SERVER_RECOVERY_OK cases=3"]),
        ("m57", canonical + ["STORAGE_SERVER_REPEATED_RECOVERY_OK cycles=2"]),
        ("m58", canonical + ["STORAGE_SERVER_ASYNC_RECOVERY_OK cycles=2"]),
        ("m60", canonical + ["STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6"]),
        ("m60_boot", canonical + ["BOOT_OK: M60 bounded StorageServer fault policy verified"]),
        ("unrelated_boot", canonical + ["BOOT_OK: M58 cooperative fail-stop StorageServer recovery verified"]),
    ]
    for name, lines in negative:
        try:
            validate(lines)
        except EvidenceError:
            continue
        raise SystemExit(f"M59 low-level parser negative self-test accepted {name}")
    print(f"STORAGE_IRQ_COOPERATIVE_RECOVERY_PARSER_OK negative_cases={len(negative)}")


if len(sys.argv) < 2:
    raise SystemExit("M59 low-level evidence parser mode is missing")
mode = sys.argv[1]
if mode == "--self-test":
    if len(sys.argv) != 2:
        raise SystemExit("M59 low-level parser self-test takes no path")
    self_test()
elif mode == "--validate":
    if len(sys.argv) != 3:
        raise SystemExit("M59 low-level parser validation requires exactly one log path")
    validate(Path(sys.argv[2]).read_text(encoding="utf-8").splitlines())
else:
    raise SystemExit(f"unknown M59 low-level evidence parser mode: {mode}")
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  if [[ "$#" -ne 1 ]]; then
    echo "--parser-self-test takes no additional arguments." >&2
    exit 2
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found; cannot run the M59 low-level parser self-test." >&2
    exit 1
  fi
  run_evidence_parser --self-test
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--parser-self-test]" >&2
  exit 2
fi

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-12}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be an integer from 1 through 600." >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 cargo rustc sed mktemp tr grep kill mkdir rm cp sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M59 low-level cooperative recovery." >&2
    exit 1
  fi
done

# Refuse to trust guest evidence unless the exact same parser first rejects its
# local negative fixtures. This path is offline and does not launch QEMU.
run_evidence_parser --self-test >/dev/null

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
validate_storage_image_geometry "$STORAGE_IMAGE"
build_storage_qemu_args "$STORAGE_IMAGE" modern readonly

BNDROID_PROFILE=debug "$SCRIPT_DIR/build-userspace.sh" >/dev/null
export BNDROID_INIT_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-init"
export BNDROID_SERVICE_MANAGER_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-service-manager"
export BNDROID_ECHO_PROVIDER_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-echo-provider"
export BNDROID_ECHO_CLIENT_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-echo-client"
export BNDROID_SURFACE_SERVER_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-surface-server"
export BNDROID_LAUNCHER_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-launcher"
export BNDROID_APP_ELF="$WORKSPACE_ROOT/target/aarch64-unknown-none/debug/bndroid-app"

TARGET_ROOT="$WORKSPACE_ROOT/target/storage-irq-timeout-self-test"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m59"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"
CARGO_TARGET_DIR="$TARGET_ROOT" cargo build \
  --locked \
  --target aarch64-unknown-none \
  -p bndroid-kernel \
  --features storage-irq-timeout-self-test

KERNEL_ELF="$TARGET_ROOT/aarch64-unknown-none/debug/bndroid-kernel"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/debug/bndroid-kernel.img"
RUST_SYSROOT="$(rustc --print sysroot)"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
RUST_OBJCOPY="$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/rust-objcopy"
LLVM_OBJCOPY="$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/llvm-objcopy"
if [[ -x "$RUST_OBJCOPY" ]]; then
  OBJCOPY="$RUST_OBJCOPY"
elif [[ -x "$LLVM_OBJCOPY" ]]; then
  OBJCOPY="$LLVM_OBJCOPY"
else
  echo "rust-objcopy/llvm-objcopy not found under $RUST_SYSROOT." >&2
  exit 1
fi
"$OBJCOPY" -O binary "$KERNEL_ELF" "$KERNEL_IMAGE"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-irq-race.XXXXXX")"
LOG_FILE="$TMP_DIR/qemu.log"
NORMALIZED_LOG="$TMP_DIR/qemu.normalized.log"
QEMU_PID=""

# QEMU appends serial output one byte at a time.  Only expose newline-committed
# records to the live evidence checks so a partially written BOOT_OK line
# cannot be mistaken for a stale full-line marker.
normalize_complete_log() {
  local source_log="$1"
  local normalized_log="$2"
  python3 - "$source_log" "$normalized_log" <<'PY'
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_bytes().replace(b"\r", b"")
last_newline = source.rfind(b"\n")
Path(sys.argv[2]).write_bytes(
    source[: last_newline + 1] if last_newline >= 0 else b""
)
PY
}

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 128M \
  -nographic \
  -monitor none \
  -nic none \
  -serial stdio \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  >"$LOG_FILE" 2>&1 &
QEMU_PID=$!

GRAPHICS_BUFFER_MARKER='GRAPHICS_BUFFER_OK abi=23 format=XRGB8888 width=208 height=368 logical_bytes=306176 backing_bytes=307200 slots=2 created=1 write_calls=1 write_bytes=4 presents=0 handles=2 producer_pid=0x0000000100000008 consumer_pid=0x0000000100000006 slot=0 allocation_generation=1 producer_rights=0x0000000f server_rights=0x00000009 owners_valid=1 rights_valid=1 generation_qualified=1 transferred=1 writable=1 copy_present=1'

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 260 "$NORMALIZED_LOG" >&2 || true
  fi
}

reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:' "$NORMALIZED_LOG" \
    || grep -Eq '^STORAGE_FAIL( |$)|(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '^APPDATA_(ASYNC_RECOVERY|RUNTIME)_OK( |$)|^STORAGE_SERVER_(RUNTIME|RECOVERY|REPEATED_RECOVERY|ASYNC_RECOVERY|FAULT_POLICY)_OK( |$)|^BOOT_OK: M54( |$)|^BOOT_OK: M56( |$)|^BOOT_OK: M57( |$)|^BOOT_OK: M58( |$)|^BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified$|^BOOT_OK: M60( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M59 low-level profile emitted a panic, failure, or stale AppData/M56-M60 marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M59 low-level profile published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_ui_contract() {
  if [[ "$(grep -c '^SURFACE_ACQUIRE_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
    || [[ "$(grep -c '^SURFACE_HANDOFF_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
    || [[ "$(grep -c '^USER_SURFACE_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
    || [[ "$(grep -c '^UI_RUNTIME_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
    || [[ "$(grep -c '^GRAPHICS_BUFFER_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
    || [[ "$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)" != "0" ]] \
    || grep -Eq '^FRAMEBUFFER_OK |^COMPOSITOR_READY |^SURFACE_DEGRADED ' "$NORMALIZED_LOG" \
    || ! grep -Eq '^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=[1-9][0-9]* session=[1-9][0-9]* handle=[1-9][0-9]* rights=0x00000103 unique=1 duplicate=0 transferable=0 input_capacity=64$' "$NORMALIZED_LOG" \
    || ! grep -Eq '^SURFACE_HANDOFF_OK from=kernel-fallback to=userspace-bound caller=el0 pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25$' "$NORMALIZED_LOG" \
    || ! grep -Eq '^USER_SURFACE_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 commit=1 mode=full rects=4 local_damage=0/0/208/368 global_damage=56/64/208/368 raster_writes=124544 composition=56/64/208/368 restored=76544 blended=0 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25 input_enqueued=0 input_dequeued=0 input_pending=0 input_coalesced=0 cursor_preserved=1 dma_barrier=1$' "$NORMALIZED_LOG" \
    || ! grep -Fqx 'UI_RUNTIME_OK protocol=3 server_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 distinct=1 ui_pairs=2 surface_owner=server launcher_surface=0 app_surface=0 server_handles=4 launcher_handles=1 app_handles=2 dual_clients=1 focus_routed=1 ready_before_present=1 single_outstanding=1 first_present_ack=1 graphics_buffer=1 resident=1' "$NORMALIZED_LOG" \
    || ! grep -Fqx "$GRAPHICS_BUFFER_MARKER" "$NORMALIZED_LOG" \
    || grep -Eq '^FDT_VIRTIO_OK |^VIRTIO_IRQ_OK |^VIRTIO_BLK_OK |^BLOCK_IRQ_OK |^BLOCK_LAYER_OK |^GPT_OK |^DATA_GPT_OK |^FAT16_OK |^VFS_OK |^DATA_PERSIST_OK |^STORAGE_LIMITS ' "$NORMALIZED_LOG"; then
    show_failure
    echo "M59 low-level profile emitted inconsistent headless UI evidence." >&2
    exit 1
  fi
}

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  normalize_complete_log "$LOG_FILE" "$NORMALIZED_LOG"
  reject_failure
  cooperative_count="$(grep -Ec "^${COOPERATIVE_MARKER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  race_count="$(grep -Ec '^STORAGE_IRQ_RACE_OK( |$)' "$NORMALIZED_LOG" || true)"
  exact_race_count="$(grep -Fxc "$RACE_MARKER" "$NORMALIZED_LOG" || true)"
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$cooperative_count" == "1" && "$race_count" == "1" && "$exact_race_count" == "1" && "$boot_count" == "1" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    normalize_complete_log "$LOG_FILE" "$NORMALIZED_LOG"
    reject_failure
    if ! run_evidence_parser --validate "$NORMALIZED_LOG"; then
      show_failure
      echo "M59 low-level cooperative recovery evidence violated its exact schema." >&2
      exit 1
    fi
    validate_ui_contract
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-irq-cooperative-recovery.log"
    echo "M59 cooperative storage IRQ recovery, legacy M22 race, and headless UI self-test passed."
    exit 0
  fi
  if ((cooperative_count > 1 || race_count > 1 || exact_race_count > 1 || boot_count > 1)); then
    show_failure
    echo "M59 low-level profile published duplicate recovery, race, or BOOT_OK evidence." >&2
    exit 1
  fi
  if ((race_count > 0 && exact_race_count != race_count)); then
    show_failure
    echo "M59 low-level profile changed the legacy STORAGE_IRQ_RACE_OK contract." >&2
    exit 1
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    normalize_complete_log "$LOG_FILE" "$NORMALIZED_LOG"
    reject_failure
    show_failure
    echo "QEMU exited before M59 cooperative storage IRQ recovery evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

normalize_complete_log "$LOG_FILE" "$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M59 cooperative storage IRQ recovery evidence." >&2
exit 1
