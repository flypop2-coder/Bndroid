#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/memory-evidence.sh"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "qemu-system-aarch64 not found. Install QEMU first." >&2
  exit 1
fi

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-10}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi

TARGET_ROOT="$WORKSPACE_ROOT/target/heap-no-reclaim-self-test"
CARGO_TARGET_DIR="$TARGET_ROOT" cargo build \
  --locked \
  --target aarch64-unknown-none \
  -p bndroid-kernel \
  --features heap-no-reclaim-self-test

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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-heap-negative.XXXXXX")"
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
  >"$LOG_FILE" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq '^HEAP_OK |^SCHED_OK |^SLEEP_OK |^BOOT_OK:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Heap no-reclaim injection reached a later success marker unexpectedly." >&2
    exit 1
  fi
  if grep -Fq 'HEAP_NO_RECLAIM_INJECTED: first heap release was suppressed' "$NORMALIZED_LOG" \
    && grep -Fq 'kernel panic:' "$NORMALIZED_LOG" \
    && grep -Fq 'kernel heap failed to reclaim a released allocation' "$NORMALIZED_LOG"; then
    if ! grep -Fq 'entered AArch64 EL1; running EL1' "$NORMALIZED_LOG" \
      || ! validate_memory_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Heap negative test did not reach the validated EL1 memory baseline." >&2
      exit 1
    fi
    cat "$NORMALIZED_LOG"
    echo "Heap negative self-test passed: a suppressed release was rejected."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before the expected heap no-reclaim failure (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for the expected heap no-reclaim failure." >&2
exit 1
