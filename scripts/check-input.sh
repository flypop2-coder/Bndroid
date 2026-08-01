#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify virtio input." >&2
    exit 1
  fi
done

"$SCRIPT_DIR/build-kernel.sh" >/dev/null
"$SCRIPT_DIR/build-storage-image.sh" >/dev/null

PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-15}"
READY_MARKER="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-input.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}

show_failure() {
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  cat "$NORMALIZED_LOG"
  cat "$QEMU_LOG" >&2
}

reject_kernel_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|INPUT_FAIL:|input (probe|install|IRQ validation|event|stats|event validation) failed' "$NORMALIZED_LOG"; then
    show_failure
    echo "Kernel emitted a fatal input or boot failure." >&2
    exit 1
  fi
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 128M \
  -display none \
  -monitor none \
  -nic none \
  -serial "file:$SERIAL_LOG" \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  >"$QEMU_LOG" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_kernel_failure
  if grep -Fxq "$READY_MARKER" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the keyboard became ready." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if [[ "$(grep -Fxc "$READY_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^INPUT_READY ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "Timed out waiting for one exact INPUT_READY marker." >&2
  exit 1
fi

python3 - "$QMP_SOCKET" <<'PY'
import json
import socket
import sys
import time

socket_path = sys.argv[1]
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(200):
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available")

stream = connection.makefile("rwb", buffering=0)
greeting = json.loads(stream.readline())
if "QMP" not in greeting:
    raise SystemExit("invalid QMP greeting")

def command(payload):
    stream.write(json.dumps(payload).encode("ascii") + b"\n")
    while True:
        line = stream.readline()
        if not line:
            raise SystemExit("QMP connection closed")
        response = json.loads(line)
        if "error" in response:
            raise SystemExit(f"QMP error: {response['error']}")
        if "return" in response:
            return response["return"]

def send_a(down):
    command({
        "execute": "input-send-event",
        "arguments": {
            "events": [{
                "type": "key",
                "data": {
                    "down": down,
                    "key": {"type": "qcode", "data": "a"},
                },
            }],
        },
    })

command({"execute": "qmp_capabilities"})
send_a(True)
time.sleep(0.05)
send_a(False)
connection.close()
PY

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_kernel_failure
  # A serial-file reader can observe the marker prefix before QEMU has
  # appended the rest of the same line.  Wait for the canonical suffix so the
  # strict parser below never consumes a transient partial record.
  if grep -Eq '^INPUT_EVENT_OK .* descriptor_reuse=1$' "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before publishing input event evidence." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_kernel_failure
if [[ "$(grep -c '^INPUT_EVENT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "Timed out waiting for exactly one INPUT_EVENT_OK marker." >&2
  exit 1
fi

event_line="$(grep '^INPUT_EVENT_OK ' "$NORMALIZED_LOG")"
event_regex='^INPUT_EVENT_OK device=keyboard sequence=a_down/syn/a_up/syn events=4 key_code=30 down=1 up=1 syn=2 completions=4 delivered=4 recycled=4 buffered=0 max_buffered=([1-9][0-9]*) avail_idx=12 used_idx=4 irq_entries=([1-9][0-9]*) queue_irqs=([1-9][0-9]*) config_irqs=0 spurious=0 dropped=0 invalid=0 descriptor_reuse=1$'
if [[ ! "$event_line" =~ $event_regex ]]; then
  show_failure
  echo "INPUT_EVENT_OK did not prove the exact four-event, lossless descriptor-reuse contract." >&2
  exit 1
fi

max_buffered="${BASH_REMATCH[1]}"
irq_entries="${BASH_REMATCH[2]}"
queue_irqs="${BASH_REMATCH[3]}"
printf '%s\n' "INPUT_QMP_OK qemu=1 transport=virtio-mmio device=keyboard qcode=a qmp_commands=2 events=4 key_code=30 down=1 up=1 syn=2 completions=4 delivered=4 recycled=4 buffered=0 max_buffered=$max_buffered avail_idx=12 used_idx=4 irq_entries=$irq_entries queue_irqs=$queue_irqs config_irqs=0 spurious=0 dropped=0 invalid=0 descriptor_reuse=1 event_idx=0 indirect_desc=0 in_order=0 packed=0 queue_reset=0"
