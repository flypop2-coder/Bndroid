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

TARGET_ROOT="${BNDROID_GRAPHICS_OWNER_DEATH_TARGET_DIR:-$WORKSPACE_ROOT/target/graphics-owner-death-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=graphics-owner-death-runtime \
  BNDROID_KERNEL_FEATURES=graphics-owner-death-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Graphics owner-death kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-graphics-owner-death.XXXXXX")"
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

SUCCESS_MARKER="BOOT_OK: M36 graphics consumer owner-death release and producer recovery verified"
EXPECTED_EVIDENCE="GRAPHICS_OWNER_DEATH_OK abi=23 protocol=1 messages=5 errors=0 transactions=1 completed=1 supervisor=2/0/1/1 consumer_pid=0x0000000100000006 producer_pid=0x0000000100000008 created=9 exited=3 reaped=3 live=6 handles=19 endpoints=19 waits=4/2/2 process_waits=4/3/1 owner=1/1/0/1/1/0/1 wake_nonzero=1 buffers=1 map=2/2 unmap_syscalls=2/1 mappings_removed=2 denied_consumer_unmap=1 denied_mapped_close=1 queue=1/1 acquire=1/1 explicit_release=0/0 releases=1 mappings=0/0 protects=2 surfaces=0 graphics_handles=0 producer=0/unmapped consumer=0/unmapped shared_pairs=0 el0_rewrite=all-pages explicit_cleanup=1 surface_absent=1 app_exited=1"
EXPECTED_ABANDON="GRAPHICS_CONSUMER_ABANDON_OK consumer_pid=4294967302 producer_pid=4294967304 generation=1 state=acquired producer=restored consumer_unmapped=1 writable=1"

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|graphics owner-death runtime failed|graphics owner-death runtime timed out|GRAPHICS_OWNER_DEATH_(DIAG|TIMEOUT|FAILED)|USER_FAIL:|THREAD_EXITED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics owner-death runtime reported a kernel, EL0, protocol, or cleanup failure." >&2
    exit 1
  fi
  if grep -Eq '^MAPPED_GRAPHICS_OK |^APP_LIFECYCLE_OK |^APP_CRASH_RECOVERY_OK |^UI_RUNTIME_OK ' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Graphics owner-death image published evidence from another runtime profile." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "Graphics owner-death image published an unexpected BOOT_OK marker." >&2
    exit 1
  fi

  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    if [[ "$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^GRAPHICS_OWNER_DEATH_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^GRAPHICS_CONSUMER_ABANDON_OK ' "$NORMALIZED_LOG" || true)" != "1" \
      || "$(grep -c '^SURFACE_DEGRADED ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Graphics owner-death image did not publish each proof marker exactly once." >&2
      exit 1
    fi
    if ! grep -Fxq "$EXPECTED_EVIDENCE" "$NORMALIZED_LOG" \
      || ! grep -Fxq "$EXPECTED_ABANDON" "$NORMALIZED_LOG" \
      || ! grep -Eq '^SURFACE_DEGRADED owner=userspace reason=process-exit pid=4294967302 ' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics owner-death evidence did not match the exact M36 cleanup ledger." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics owner-death boot did not preserve the exact M25 storage evidence." >&2
      exit 1
    fi
    if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
      || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Graphics owner-death boot did not enter and normalize execution at EL1." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "Graphics owner-death runtime passed: dead consumer aliases were invalidated, the producer was restored and woken, all 75 pages were rewritten, and both mappings were reclaimed."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before graphics owner-death evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for graphics owner-death evidence." >&2
exit 1
