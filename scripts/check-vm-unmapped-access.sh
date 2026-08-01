#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"

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

TARGET_ROOT="$WORKSPACE_ROOT/target/vm-unmapped-access-self-test"
CARGO_ARGS=(build --locked --target aarch64-unknown-none -p bndroid-kernel --features vm-unmapped-access-self-test)
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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-vm-unmapped.XXXXXX")"
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
  >"$LOG_FILE" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq '^FRAME_OK |^VM_OK |^MEMORY_OK:|^HEAP_OK |^HEAP_IRQ_OK |^SCHED_OK |^SLEEP_OK |^BOOT_OK:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Unmapped-access self-test reached a success marker unexpectedly." >&2
    exit 1
  fi
  if grep -Fq 'VM_UNMAPPED_ACCESS_FAILED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Hardware allowed a read from the unmapped dynamic page." >&2
    exit 1
  fi
  if grep -Fxq 'VM_UNMAPPED_ACCESS_ARMED: va=0x0000000100000000' "$NORMALIZED_LOG" \
    && grep -Fq 'fatal exception: synchronous from current EL using SPx' "$NORMALIZED_LOG" \
    && grep -Eq 'ESR=0x0000000096000007 .* FAR=0x0000000100000000 ' "$NORMALIZED_LOG"; then
    miss_count="$(grep -c '^COPYIO_FIXUP_MISS ' "$NORMALIZED_LOG" || true)"
    miss_line="$(grep '^COPYIO_FIXUP_MISS ' "$NORMALIZED_LOG" || true)"
    fatal_state_line="$(grep '^  ESR=' "$NORMALIZED_LOG" || true)"
    miss_regex='^COPYIO_FIXUP_MISS pc=(0x[0-9a-f]+) ec=([0-9]+) dfsc=([0-9]+) wnr=([0-9]+) far=(0x[0-9a-f]+) match=([0-9]+)$'
    fatal_state_regex='^  ESR=(0x[0-9a-f]+) ELR=(0x[0-9a-f]+) FAR=(0x[0-9a-f]+) SPSR=(0x[0-9a-f]+)$'
    if [[ "$miss_count" != "1" || ! "$miss_line" =~ $miss_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected one exact COPYIO exception-table miss for the unlisted kernel load." >&2
      exit 1
    fi
    miss_pc="${BASH_REMATCH[1]}"
    if (( ${BASH_REMATCH[2]} != 37 \
      || ${BASH_REMATCH[3]} != 7 \
      || ${BASH_REMATCH[4]} != 0 \
      || ${BASH_REMATCH[5]} != 0x0000000100000000 \
      || ${BASH_REMATCH[6]} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "COPYIO exception-table miss syndrome was inconsistent." >&2
      exit 1
    fi
    if [[ ! "$fatal_state_line" =~ $fatal_state_regex ]] \
      || [[ "$miss_pc" != "${BASH_REMATCH[2]}" ]] \
      || (( ${BASH_REMATCH[1]:-0} != 0x0000000096000007 \
        || ${BASH_REMATCH[3]:-0} != 0x0000000100000000 )); then
      cat "$NORMALIZED_LOG"
      echo "COPYIO table miss PC/FAR did not match the fatal exception state." >&2
      exit 1
    fi
    if grep -Eq '^COPYIO_OK |^SYSCALL_OK |^EL0_OK |^BOOT_OK:' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Unlisted current-EL access incorrectly reached a recoverable success path." >&2
      exit 1
    fi
    if ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Unmapped-access test did not normalize execution to EL1." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "on" ]] && ! grep -Fq 'entered AArch64 EL2' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Unmapped-access EL2 test did not enter through EL2." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "off" ]] && ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Unmapped-access EL1 test did not enter directly through EL1." >&2
      exit 1
    fi
    cat "$NORMALIZED_LOG"
    echo "VM unmap protection self-test passed: the stale virtual read faulted at L3."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before the expected unmapped-access fault (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for the expected unmapped-access fault." >&2
exit 1
