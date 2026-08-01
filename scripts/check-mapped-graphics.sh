#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "qemu-system-aarch64 not found. Install QEMU first." >&2
  exit 1
fi

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-12}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="${BNDROID_MAPPED_GRAPHICS_TARGET_DIR:-$WORKSPACE_ROOT/target/mapped-graphics-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=mapped-graphics-runtime \
  BNDROID_KERNEL_FEATURES=mapped-graphics-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Mapped-graphics kernel image is missing after the isolated build: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-mapped-graphics.XXXXXX")"
LOG_FILE="$TMP_DIR/qemu.log"
NORMALIZED_LOG="$TMP_DIR/qemu.normalized.log"
: >"$LOG_FILE"
QEMU_PID=""
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
  -cpu "$QEMU_CPU" \
  -smp 1 \
  -m 128M \
  -nographic \
  -monitor none \
  -nic none \
  -serial stdio \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  >"$LOG_FILE" 2>&1 &
QEMU_PID=$!

SUCCESS_MARKER="BOOT_OK: M35 shared mapped BufferQueue and acquire/release fences verified"
EXPECTED_EVIDENCE="MAPPED_GRAPHICS_OK abi=23 protocol=1 messages=35 errors=0 transactions=7 completed=7 supervisor=14/0/7/7 first_instance=1 last_instance=2 first_pid=0x0000000100000008 last_pid=0x0000000200000008 created=10 exited=2 reaped=2 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=3/2/1 buffers=2 mappable=2 map=4/4 unmap=2/2 queue=4/4 acquire=4/4 explicit_release=2/2 releases=4 mapped_presents=2 total_presents=2 validated_pixels=306176 validated_bytes=1224704 copy_writes=0/0 mappings=2/150 protects=8 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 buffer_generation=2 contexts_distinct=1 topology=exact final_state=active final_app_resident=1"
NORMAL_RUNTIME_REGEX='^SYSCALL_OK |^COPYIO_OK |^HANDLE_OK |^IPC_OK |^IPC_IDENTITY_OK |^HANDLE_TRANSFER_OK |^PROC_LIFECYCLE_OK |^PROC_CAPACITY_OK |^PROCESS_IMAGE_OK |^UI_RUNTIME_OK |^GRAPHICS_BUFFER_OK |^SERVICE_RESTART_OK |^PROCESS_WAIT_OK |^OBJECT_WAIT_OK |^WAIT_MANY_OK |^WAIT_ARRAY_OK |^EVENT_OK |^SERVICE_MANAGER_OK |^SERVICE_LOOP_OK |^SERVICE_DYNAMIC_OK |^SERVICE_REUSE_OK |^SERVICE_ACL_OK |^SERVICE_MULTICLIENT_OK |^SERVICE_MULTISESSION_OK |^SERVICE_WAIT_TOPOLOGY_OK |^RESIDENT_STABILITY_OK |^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^EL0_OK |^EL0_STORAGE_OK |^ELF_RUNTIME_OK |^INIT_M16_|^INIT_M17_|^INIT_M19_|^INIT_M20_'

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|mapped graphics runtime failed|mapped graphics runtime timed out|MAPPED_GRAPHICS_(DIAG|TIMEOUT|FAILED)|USER_FAIL:|THREAD_EXITED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Mapped-graphics runtime reported a kernel, EL0, protocol, or mapping failure." >&2
    exit 1
  fi
  if grep -Eq '^APP_LIFECYCLE_OK |^APP_CRASH_RECOVERY_OK ' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Mapped-graphics image incorrectly published an M33 or M34 success marker." >&2
    exit 1
  fi
  if grep -Eq "$NORMAL_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Mapped-graphics image incorrectly published normal M32 userspace-runtime evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "Mapped-graphics image published an unexpected M32, M33, M34, or other BOOT_OK marker." >&2
    exit 1
  fi

  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    boot_count="$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)"
    evidence_count="$(grep -c '^MAPPED_GRAPHICS_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$boot_count" != "1" || "$evidence_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Mapped-graphics image did not publish exactly one evidence and boot marker." >&2
      exit 1
    fi
    if ! grep -Fxq "$EXPECTED_EVIDENCE" "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "MAPPED_GRAPHICS_OK did not match the exact M35 mapping, queue, fence, and topology ledger." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Mapped-graphics boot did not preserve the exact M25 storage evidence." >&2
      exit 1
    fi
    if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Mapped-graphics boot did not enter and normalize execution at EL1." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "Mapped graphics runtime passed: two generations exercised shared mappings, queue/acquire, explicit and present release fences, and zero copy writes."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before mapped-graphics evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for mapped-graphics evidence." >&2
exit 1
