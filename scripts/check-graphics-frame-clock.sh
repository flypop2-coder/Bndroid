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

TARGET_ROOT="${BNDROID_GRAPHICS_FRAME_CLOCK_TARGET_DIR:-$WORKSPACE_ROOT/target/graphics-frame-clock-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=graphics-frame-clock-runtime \
  BNDROID_KERNEL_FEATURES=graphics-frame-clock-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Graphics frame-clock kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-graphics-frame-clock.XXXXXX")"
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

SUCCESS_MARKER="BOOT_OK: M39 software frame clock and gated mapped presents verified"
EXPECTED_EVIDENCE="GRAPHICS_FRAME_CLOCK_OK abi=23 protocol=1 clock=software-timer tick_hz=100 frame_hz=50 divider=2 phase=ready calls=3/3/0/0 opportunities=4 edges=4 acquired=3 presented=3 suppression=observed pending=1 outstanding=0 discarded=0 cancelled=0 epochs=3/3 grant_preserved=1 ungated_rejects=3 messages=10 errors=0 transactions=2 completed=2 supervisor=4/0/2/2 created=9 exited=1 reaped=1 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=2/1/1 buffers=1 mappable=1 map=2/2 unmap=0/0 queue=4/4 acquire=4/4 explicit_release=1/1 releases=4 mapped_presents=3 total_presents=3 validated_pixels=306176 validated_bytes=1224704 copy_writes=0/0 mappings=2/150 protects=8 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 buffer_generation=1 write_generation=4 contexts_distinct=1 topology=resident final_state=ready final_app_resident=1"
OTHER_RUNTIME_REGEX='^APP_LIFECYCLE_OK |^APP_CRASH_RECOVERY_OK |^MAPPED_GRAPHICS_OK |^GRAPHICS_OWNER_DEATH_OK |^GRAPHICS_SURFACE_RESTART_OK |^GRAPHICS_PRODUCER_ORPHAN_OK '
NORMAL_RUNTIME_REGEX='^SYSCALL_OK |^COPYIO_OK |^HANDLE_OK |^IPC_OK |^IPC_IDENTITY_OK |^HANDLE_TRANSFER_OK |^PROC_LIFECYCLE_OK |^PROC_CAPACITY_OK |^PROCESS_IMAGE_OK |^UI_RUNTIME_OK |^GRAPHICS_BUFFER_OK |^SERVICE_RESTART_OK |^PROCESS_WAIT_OK |^OBJECT_WAIT_OK |^WAIT_MANY_OK |^WAIT_ARRAY_OK |^EVENT_OK |^SERVICE_MANAGER_OK |^SERVICE_LOOP_OK |^SERVICE_DYNAMIC_OK |^SERVICE_REUSE_OK |^SERVICE_ACL_OK |^SERVICE_MULTICLIENT_OK |^SERVICE_MULTISESSION_OK |^SERVICE_WAIT_TOPOLOGY_OK |^RESIDENT_STABILITY_OK |^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^EL0_OK |^EL0_STORAGE_OK |^ELF_RUNTIME_OK |^INIT_M16_|^INIT_M17_|^INIT_M19_|^INIT_M20_'

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|graphics frame-clock runtime failed|graphics frame-clock runtime timed out|GRAPHICS_FRAME_CLOCK_(PROBE|DIAG|TIMEOUT|FAILED)|USER_FAIL:|THREAD_EXITED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics frame-clock runtime reported a probe, diagnostic, timeout, kernel, EL0, protocol, pacing, or mapping failure." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX|$NORMAL_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics frame-clock image published another runtime's success evidence instead of final M39 evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "Graphics frame-clock image published an unexpected BOOT_OK marker." >&2
    exit 1
  fi

  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    if [[ "$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^GRAPHICS_FRAME_CLOCK_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^SURFACE_FRAME_ACQUIRE_OK ' "$NORMALIZED_LOG" || true)" != "3" \
      || "$(grep -c '^SURFACE_FRAME_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "3" \
      || "$(grep -c '^SURFACE_FRAME_PRESENT_REJECT_OK ' "$NORMALIZED_LOG" || true)" != "3" \
      || "$(grep -c '^SURFACE_FRAME_PRESENT_ABORT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Graphics frame-clock image did not publish each pacing proof and final marker exactly the required number of times." >&2
      exit 1
    fi
    if ! grep -Fxq "$EXPECTED_EVIDENCE" "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "GRAPHICS_FRAME_CLOCK_OK did not match the exact M39 clock, grant, mapping, lifecycle, and resident-topology ledger." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics frame-clock boot did not preserve the exact M25 storage evidence." >&2
      exit 1
    fi
    if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
      || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics frame-clock boot did not enter and normalize execution at EL1." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "Graphics frame-clock runtime passed: three mapped presents consumed distinct grants, failed present preserved its grant, and late timer boundaries were coalesced into one pending opportunity."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before graphics frame-clock evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for graphics frame-clock evidence." >&2
exit 1
