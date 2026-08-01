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

TARGET_ROOT="${BNDROID_GRAPHICS_SURFACE_RESTART_TARGET_DIR:-$WORKSPACE_ROOT/target/graphics-surface-restart-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=graphics-surface-restart-runtime \
  BNDROID_KERNEL_FEATURES=graphics-surface-restart-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Graphics surface-restart kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-graphics-surface-restart.XXXXXX")"
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

SUCCESS_MARKER="BOOT_OK: M37 SurfaceServer restart, client rebind, and mapped frame recovery verified"
EXPECTED_EVIDENCE="GRAPHICS_SURFACE_RESTART_OK abi=23 protocol=1 messages=10 errors=0 transactions=2 completed=2 supervisor=4/0/2/2 old_surface=0x0000000100000006 new_surface=0x0000000200000006 app=0x0000000100000008 sessions=1/2 created=10 exited=2 reaped=2 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=3/2/1 owner=1/1/0/1/1/0/1 wake_nonzero=1 reacquire=1 discarded_input=0 buffers=1 map=3/3 unmap_syscalls=1/0 mappings_removed=1 queue=2/2 acquire=2/2 explicit_release=0/0 releases=2 mapped_presents=1 total_presents=1 validated_pixels=153088 validated_bytes=612352 mappings=2/150 protects=4 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 surface_session=2 surface_owner=replacement frame=1 topology=exact final_state=active final_app_resident=1"
EXPECTED_REACQUIRE="SURFACE_REACQUIRE_OK old_pid=4294967302 new_pid=8589934598 old_session=1 new_session=2 discarded_input=0 frozen_scene=0x6ef9c2b7d15fde25 frozen_scanout=0x6ef9c2b7d15fde25 session_reset=1"
EXPECTED_ABANDON="GRAPHICS_CONSUMER_ABANDON_OK consumer_pid=4294967302 producer_pid=4294967304 generation=1 state=acquired producer=restored consumer_unmapped=1 writable=1"
EXPECTED_DEGRADED="SURFACE_DEGRADED owner=userspace reason=process-exit pid=4294967302 session=1 last_frame=0 commits=0 pending=0 peer_closed_edge=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25"
EXPECTED_COMMIT="USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=8589934598 session=2 producer_pid=4294967304 client_frame_id=1 frame_id=1 commit=1 mode=full buffer_generation=2 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=0x4ff7cb118bd51225 scanout_digest=0x4ff7cb118bd51225 cursor_preserved=1 dma_barrier=1"
OLD_ACQUIRE_REGEX='^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=4294967302 session=1 handle=[1-9][0-9]* rights=0x00000103 unique=1 duplicate=0 transferable=0 input_capacity=64$'
NEW_ACQUIRE_REGEX='^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=8589934598 session=2 handle=[1-9][0-9]* rights=0x00000103 unique=1 duplicate=0 transferable=0 input_capacity=64$'
OTHER_RUNTIME_REGEX='^APP_LIFECYCLE_OK |^APP_CRASH_RECOVERY_OK |^MAPPED_GRAPHICS_OK |^GRAPHICS_OWNER_DEATH_OK |^GRAPHICS_SURFACE_RESTART_PROBE '

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|graphics surface restart runtime failed|graphics surface restart runtime timed out|GRAPHICS_SURFACE_RESTART_(DIAG|TIMEOUT|FAILED)|USER_FAIL:|THREAD_EXITED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics surface-restart runtime reported a kernel, EL0, protocol, or recovery failure." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics surface-restart image published M33--M36 or probe evidence instead of final M37 evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "Graphics surface-restart image published an unexpected BOOT_OK marker." >&2
    exit 1
  fi

  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    if [[ "$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^GRAPHICS_SURFACE_RESTART_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^SURFACE_REACQUIRE_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^GRAPHICS_CONSUMER_ABANDON_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^SURFACE_DEGRADED ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^SURFACE_ACQUIRE_OK ' "$NORMALIZED_LOG" || true)" != "2" \
      || "$(grep -c '^USER_SURFACE_BUFFER_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Graphics surface-restart image did not publish each M37 proof marker exactly the required number of times." >&2
      exit 1
    fi
    if ! grep -Fxq "$EXPECTED_EVIDENCE" "$NORMALIZED_LOG" \
      || ! grep -Fxq "$EXPECTED_REACQUIRE" "$NORMALIZED_LOG" \
      || ! grep -Fxq "$EXPECTED_ABANDON" "$NORMALIZED_LOG" \
      || ! grep -Fxq "$EXPECTED_DEGRADED" "$NORMALIZED_LOG" \
      || ! grep -Fxq "$EXPECTED_COMMIT" "$NORMALIZED_LOG" \
      || [[ "$(grep -Ec "$OLD_ACQUIRE_REGEX" "$NORMALIZED_LOG" || true)" != "1" ]] \
      || [[ "$(grep -Ec "$NEW_ACQUIRE_REGEX" "$NORMALIZED_LOG" || true)" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Graphics surface-restart evidence did not match the exact M37 recovery and frame ledger." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics surface-restart boot did not preserve the exact M25 storage evidence." >&2
      exit 1
    fi
    if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
      || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics surface-restart boot did not enter and normalize execution at EL1." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "Graphics surface-restart runtime passed: the dead consumer was retired, SurfaceServer generation 2 rebound both clients, and the resident App presented its rewritten mapped frame."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before graphics surface-restart evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for graphics surface-restart evidence." >&2
exit 1
