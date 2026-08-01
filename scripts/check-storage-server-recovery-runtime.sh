#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill mkdir rm sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M56 StorageServer recovery runtime." >&2
    exit 1
  fi
done

PROFILE="${BNDROID_PROFILE:-release}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-180}"
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

BOOT_MARKER="BOOT_OK: M56 fail-stop StorageServer recovery verified"
RECOVERY_MARKER_PREFIX="STORAGE_SERVER_RECOVERY_OK"
FEATURES="storage-server-runtime,storage-server-recovery-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_RECOVERY_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-recovery-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m56"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m56.XXXXXX")"
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
    echo "M56 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 200 "$NORMALIZED_LOG" >&2 || true
  fi
}

reject_failure() {
  if [[ ! -f "$NORMALIZED_LOG" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M56_[A-Z0-9_]*TIMEOUT|M57_[A-Z0-9_]*|STORAGE_SERVER_RECOVERY_TIMEOUT|^STORAGE_SERVER_REPEATED_RECOVERY_OK( |$)|^STORAGE_SERVER_FAULT_POLICY_OK( |$)|^APPDATA_ASYNC_RECOVERY_OK( |$)|^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK( |$)|^BOOT_OK: M60( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M56 emitted a panic, failure, diagnostic, or timeout marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M56 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_recovery_evidence() {
  if ! python3 - "$NORMALIZED_LOG" "$BOOT_MARKER" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
boot_marker = sys.argv[2]
lines = path.read_text().splitlines()

if any(
    line.startswith("M57_")
    or line == "STORAGE_SERVER_REPEATED_RECOVERY_OK"
    or line.startswith("STORAGE_SERVER_REPEATED_RECOVERY_OK ")
    or line.startswith("BOOT_OK: M57 ")
    for line in lines
):
    raise SystemExit("M56 checker observed M57 evidence")

if any(
    line == "APPDATA_ASYNC_RECOVERY_OK"
    or line.startswith("APPDATA_ASYNC_RECOVERY_OK ")
    or line == "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK"
    or line.startswith("STORAGE_IRQ_COOPERATIVE_RECOVERY_OK ")
    for line in lines
):
    raise SystemExit("M56 checker observed M59 evidence")

if any(
    line == "STORAGE_SERVER_FAULT_POLICY_OK"
    or line.startswith("STORAGE_SERVER_FAULT_POLICY_OK ")
    or line == "BOOT_OK: M60"
    or line.startswith("BOOT_OK: M60 ")
    for line in lines
):
    raise SystemExit("M56 checker observed M60 evidence")

recovery_lines = [
    line
    for line in lines
    if line == "STORAGE_SERVER_RECOVERY_OK"
    or line.startswith("STORAGE_SERVER_RECOVERY_OK ")
]
if len(recovery_lines) != 1:
    raise SystemExit("M56 recovery evidence was not unique")
if lines.count(boot_marker) != 1:
    raise SystemExit("M56 BOOT_OK evidence was not unique")
if lines.index(recovery_lines[0]) >= lines.index(boot_marker):
    raise SystemExit("M56 BOOT_OK preceded its recovery evidence")

recovery_pattern = re.compile(
    r"^STORAGE_SERVER_RECOVERY_OK "
    r"cases=3 read_requires_reset=1 mutation_outcome_unknown=2 "
    r"control_sequences=3 injected_reads=1 injected_writes=1 injected_flushes=1 "
    r"owner_exits=3 broker_releases=3 broker_abandoned=3 "
    r"reset_attempts=3 reset_successes=3 reset_failures=0 "
    r"driver_timeouts=3 driver_resets=3 final_epoch=4 "
    r"final_generation=([1-9][0-9]*) requests=([1-9][0-9]*) "
    r"completions=([1-9][0-9]*) invariant_errors=0$"
)
match = recovery_pattern.fullmatch(recovery_lines[0])
if match is None:
    raise SystemExit("M56 recovery evidence changed its exact field contract")
final_generation, requests, completions = map(int, match.groups())
if final_generation <= 0 or completions <= 0 or requests != completions + 3:
    raise SystemExit("M56 recovery request ledger did not isolate exactly three timeouts")

runtime_marker = (
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 "
    "volume_sectors=1920 fail_stop=1 kernel_reset_authority=1"
)
runtime_lines = [
    line
    for line in lines
    if line == "STORAGE_SERVER_RUNTIME_OK"
    or line.startswith("STORAGE_SERVER_RUNTIME_OK ")
]
if runtime_lines != [runtime_marker]:
    raise SystemExit("M56 ABI, geometry, or reset-authority evidence was not exact and unique")
if not (
    lines.index(recovery_lines[0])
    < lines.index(runtime_marker)
    < lines.index(boot_marker)
):
    raise SystemExit("M56 recovery, runtime, and BOOT_OK evidence was out of order")
PY
  then
    show_failure
    echo "M56 recovery evidence did not match its fail-closed schema." >&2
    exit 1
  fi
}

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
python3 - "$RUNTIME_IMAGE" <<'PY'
import sys
from pathlib import Path

image = Path(sys.argv[1])
if image.stat().st_size != 8 * 1024 * 1024:
    raise SystemExit("M56 runtime disk lost the exact writable 8 MiB geometry")
PY

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M56 kernel image is missing: $KERNEL_IMAGE" >&2
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
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$recovery_count" == "1" && "$boot_count" == "1" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    validate_recovery_evidence
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-server-recovery-runtime.log"
    RUN_SUCCEEDED=1
    echo "M56 fail-stop StorageServer recovery runtime self-test passed."
    exit 0
  fi
  if ((recovery_count > 1 || boot_count > 1)); then
    show_failure
    echo "M56 published duplicate recovery or BOOT_OK evidence." >&2
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
    echo "QEMU exited before M56 recovery evidence (status $qemu_status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M56 StorageServer recovery evidence." >&2
exit 1
