#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/heap-evidence.sh"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "qemu-system-aarch64 not found. Install QEMU first." >&2
  exit 1
fi

PROFILE="${BNDROID_PROFILE:-debug}"
VIRTUALIZATION="${BNDROID_QEMU_VIRTUALIZATION:-off}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-10}"
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi
if [[ "$VIRTUALIZATION" != "off" && "$VIRTUALIZATION" != "on" ]]; then
  echo "BNDROID_QEMU_VIRTUALIZATION must be 'on' or 'off'." >&2
  exit 2
fi
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="$WORKSPACE_ROOT/target/scheduler-blocked-run-self-test"
CARGO_ARGS=(build --locked --target aarch64-unknown-none -p bndroid-kernel --features scheduler-blocked-run-self-test)
if [[ "$PROFILE" == "release" ]]; then
  CARGO_ARGS+=(--release)
fi
CARGO_TARGET_DIR="$TARGET_ROOT" cargo "${CARGO_ARGS[@]}"

KERNEL_ELF="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-sleep-negative.XXXXXX")"
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

qemu-system-aarch64 \
  -machine "virt,gic-version=2,secure=off,virtualization=$VIRTUALIZATION" \
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

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq '^SLEEP_OK |^BOOT_OK:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Blocked-run fault injection incorrectly reached a sleep success marker." >&2
    exit 1
  fi
  if grep -Fq 'SLEEP_BLOCKED_RUN_INJECTED: queued context changed its work counter' "$NORMALIZED_LOG" \
    && grep -Fq 'kernel panic:' "$NORMALIZED_LOG" \
    && grep -Fq 'scheduler blocked context executed while asleep' "$NORMALIZED_LOG"; then
    if ! validate_heap_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Sleep negative test did not reach a valid IRQ-safe heap baseline." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Sleep negative test entered fault injection before completing the exact M25 durable-storage/M24 catalog contract." >&2
      exit 1
    fi
    if ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Blocked-run test did not normalize execution to EL1." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "on" ]] && ! grep -Fq 'entered AArch64 EL2' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Blocked-run EL2 test did not enter through EL2." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "off" ]] && ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Blocked-run EL1 test did not enter directly through EL1." >&2
      exit 1
    fi
    cat "$NORMALIZED_LOG"
    echo "Sleep negative self-test passed: execution of a queued context was rejected."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before the expected blocked-run invariant failure (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for the expected blocked-run invariant failure." >&2
exit 1
