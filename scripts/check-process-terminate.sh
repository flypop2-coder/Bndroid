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

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-10}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="$WORKSPACE_ROOT/target/process-terminate-self-test"
CARGO_TARGET_DIR="$TARGET_ROOT" cargo build \
  --locked \
  --target aarch64-unknown-none \
  -p bndroid-kernel \
  --features process-terminate-self-test

KERNEL_ELF="$TARGET_ROOT/aarch64-unknown-none/debug/bndroid-kernel"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/debug/bndroid-kernel.img"
RUST_SYSROOT="$(rustc --print sysroot)"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
for candidate in \
  "$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/rust-objcopy" \
  "$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/llvm-objcopy"; do
  if [[ -x "$candidate" ]]; then
    OBJCOPY="$candidate"
    break
  fi
done
if [[ -z "${OBJCOPY:-}" ]]; then
  echo "rust-objcopy/llvm-objcopy not found under $RUST_SYSROOT." >&2
  exit 1
fi
"$OBJCOPY" -O binary "$KERNEL_ELF" "$KERNEL_IMAGE"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-process-terminate.XXXXXX")"
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
  -cpu "$QEMU_CPU" \
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

SUCCESS_MARKER="BOOT_OK: M33 ABI19 process termination self-test survived"
TERMINATE_REGEX='^PROCESS_TERMINATE_OK abi=([0-9]+) calls=([0-9]+) success=([0-9]+) invalid=([0-9]+) not_found=([0-9]+) invalid_state=([0-9]+) denied=([0-9]+) target_pid=(0x[0-9a-f]+) exit=([0-9]+) reason=([0-9]+) wait_calls=([0-9]+) wait_completed=([0-9]+) wait_blocks=([0-9]+) wait_wakes=([0-9]+) wait_immediate=([0-9]+) waiting_abandoned=([0-9]+) created=([0-9]+) reaped=([0-9]+) live=([0-9]+) peak_live=([0-9]+) child_syscalls=([0-9]+) child_selections=([0-9]+) handles=([0-9]+) frame_restored=([0-9]+) heap_restored=([0-9]+) dynamic_contexts=([0-9]+) dynamic_stacks=([0-9]+)$'
NORMAL_RUNTIME_REGEX='^USER_IMAGE_CATALOG_OK |^SYSCALL_OK |^COPYIO_OK |^HANDLE_OK |^IPC_OK |^IPC_IDENTITY_OK |^HANDLE_TRANSFER_OK |^PROC_LIFECYCLE_OK |^PROC_CAPACITY_OK |^PROCESS_IMAGE_OK |^UI_RUNTIME_OK |^GRAPHICS_BUFFER_CREATE_OK |^GRAPHICS_BUFFER_OK |^SERVICE_RESTART_OK |^PROCESS_WAIT_OK |^OBJECT_WAIT_OK |^WAIT_MANY_OK |^WAIT_ARRAY_OK |^EVENT_OK |^SERVICE_MANAGER_OK |^SERVICE_LOOP_OK |^SERVICE_DYNAMIC_OK |^SERVICE_REUSE_OK |^SERVICE_ACL_OK |^SERVICE_MULTICLIENT_OK |^SERVICE_MULTISESSION_OK |^SERVICE_WAIT_TOPOLOGY_OK |^RESIDENT_STABILITY_OK |^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^EL0_OK |^EL0_STORAGE_OK |^ELF_RUNTIME_OK |^INIT_M16_|^INIT_M17_|^INIT_M19_|^INIT_M20_'

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|process termination self-test failed|PROCESS_TERMINATE(_SELF_TEST)?_FAILED' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Process-termination self-test reported a kernel or contract failure." >&2
    exit 1
  fi
  if grep -Eq "$NORMAL_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Process-termination self-test incorrectly published normal userspace-runtime evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "Process-termination self-test published an unexpected BOOT_OK marker." >&2
    exit 1
  fi
  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    boot_count="$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)"
    terminate_count="$(grep -c '^PROCESS_TERMINATE_OK ' "$NORMALIZED_LOG" || true)"
    terminate_line="$(grep '^PROCESS_TERMINATE_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$boot_count" != "1" || "$terminate_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Process-termination self-test did not publish exactly one evidence and boot marker." >&2
      exit 1
    fi
    if [[ ! "$terminate_line" =~ $TERMINATE_REGEX ]]; then
      cat "$NORMALIZED_LOG"
      echo "PROCESS_TERMINATE_OK did not match the current ABI20 evidence schema." >&2
      exit 1
    fi

    abi="${BASH_REMATCH[1]}"
    calls="${BASH_REMATCH[2]}"
    successes="${BASH_REMATCH[3]}"
    invalid="${BASH_REMATCH[4]}"
    not_found="${BASH_REMATCH[5]}"
    invalid_state="${BASH_REMATCH[6]}"
    denied="${BASH_REMATCH[7]}"
    target_pid="${BASH_REMATCH[8]}"
    exit_code="${BASH_REMATCH[9]}"
    reason="${BASH_REMATCH[10]}"
    wait_calls="${BASH_REMATCH[11]}"
    wait_completed="${BASH_REMATCH[12]}"
    wait_blocks="${BASH_REMATCH[13]}"
    wait_wakes="${BASH_REMATCH[14]}"
    wait_immediate="${BASH_REMATCH[15]}"
    waiting_abandoned="${BASH_REMATCH[16]}"
    created="${BASH_REMATCH[17]}"
    reaped="${BASH_REMATCH[18]}"
    live="${BASH_REMATCH[19]}"
    peak_live="${BASH_REMATCH[20]}"
    child_syscalls="${BASH_REMATCH[21]}"
    child_selections="${BASH_REMATCH[22]}"
    handles="${BASH_REMATCH[23]}"
    frame_restored="${BASH_REMATCH[24]}"
    heap_restored="${BASH_REMATCH[25]}"
    dynamic_contexts="${BASH_REMATCH[26]}"
    dynamic_stacks="${BASH_REMATCH[27]}"

    if ((abi != 23 \
      || calls != 6 \
      || successes != 1 \
      || invalid != 3 \
      || not_found != 2 \
      || invalid_state != 0 \
      || denied != 0 \
      || target_pid == 0 \
      || exit_code != 137 \
      || reason != 3 \
      || wait_calls != 2 \
      || wait_completed != 1 \
      || wait_blocks > 1 \
      || wait_wakes != wait_blocks \
      || wait_immediate + wait_blocks != 1 \
      || waiting_abandoned != 1 \
      || created != 2 \
      || reaped != 1 \
      || live != 1 \
      || peak_live != 2 \
      || child_syscalls != 2 \
      || child_selections < 1 \
      || handles != 0 \
      || frame_restored != 1 \
      || heap_restored != 1 \
      || dynamic_contexts != 0 \
      || dynamic_stacks != 0)); then
      cat "$NORMALIZED_LOG"
      echo "PROCESS_TERMINATE_OK violated the ABI19 termination, wait, or reclamation ledger." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "Process termination self-test passed: a blocked child was killed, reaped, and fully reclaimed."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before process-termination evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for process-termination evidence." >&2
exit 1
