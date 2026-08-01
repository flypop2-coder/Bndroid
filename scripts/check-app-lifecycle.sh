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

TARGET_ROOT="${BNDROID_APP_LIFECYCLE_TARGET_DIR:-$WORKSPACE_ROOT/target/app-lifecycle-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=app-lifecycle-runtime \
  BNDROID_KERNEL_FEATURES=app-lifecycle-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "App-lifecycle kernel image is missing after the isolated build: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-app-lifecycle.XXXXXX")"
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

SUCCESS_MARKER="BOOT_OK: M33 app lifecycle and generation-safe window replacement verified"
APP_REGEX='^APP_LIFECYCLE_OK protocol=([0-9]+) messages=([0-9]+) errors=([0-9]+) transactions=([0-9]+) completed=([0-9]+) requests=([0-9]+) states=([0-9]+) commands=([0-9]+) acks=([0-9]+) actions=([0-9]+)/([0-9]+)/([0-9]+)/([0-9]+)/([0-9]+) transfer_commands=([0-9]+) first_instance=([0-9]+) last_instance=([0-9]+) first_pid=(0x[0-9a-f]{16}) last_pid=(0x[0-9a-f]{16}) slot_reused=1 generation_advanced=1 created=([0-9]+) exited=([0-9]+) reaped=([0-9]+) live=([0-9]+) reasons=([0-9]+)/([0-9]+)/([0-9]+) handles=([0-9]+) endpoints=([0-9]+) object_pending=([0-9]+) many_pending=([0-9]+) array_pending=([0-9]+) process_waits=([0-9]+)/([0-9]+)/([0-9]+) graphics_generation=([0-9]+) supervisor=([0-9]+)/([0-9]+)/([0-9]+)/([0-9]+) supervisor_operations=([0-9]+)/([0-9]+)/([0-9]+)/([0-9]+) supervisor_transfer=([0-9]+) topology=exact final_state=active final_app_resident=1$'
NORMAL_RUNTIME_REGEX='^SYSCALL_OK |^COPYIO_OK |^HANDLE_OK |^IPC_OK |^IPC_IDENTITY_OK |^HANDLE_TRANSFER_OK |^PROC_LIFECYCLE_OK |^PROC_CAPACITY_OK |^PROCESS_IMAGE_OK |^UI_RUNTIME_OK |^GRAPHICS_BUFFER_OK |^SERVICE_RESTART_OK |^PROCESS_WAIT_OK |^OBJECT_WAIT_OK |^WAIT_MANY_OK |^WAIT_ARRAY_OK |^EVENT_OK |^SERVICE_MANAGER_OK |^SERVICE_LOOP_OK |^SERVICE_DYNAMIC_OK |^SERVICE_REUSE_OK |^SERVICE_ACL_OK |^SERVICE_MULTICLIENT_OK |^SERVICE_MULTISESSION_OK |^SERVICE_WAIT_TOPOLOGY_OK |^RESIDENT_STABILITY_OK |^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^EL0_OK |^EL0_STORAGE_OK |^ELF_RUNTIME_OK |^INIT_M16_|^INIT_M17_|^INIT_M19_|^INIT_M20_'

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|app lifecycle runtime failed|app lifecycle runtime timed out|APP_LIFECYCLE_(DIAG|TIMEOUT|FAILED)|USER_FAIL:|THREAD_EXITED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "App-lifecycle runtime reported a kernel, EL0, or protocol failure." >&2
    exit 1
  fi
  if grep -Eq "$NORMAL_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "App-lifecycle image incorrectly published normal M32 userspace-runtime evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    cat "$NORMALIZED_LOG"
    echo "App-lifecycle image published an unexpected BOOT_OK marker." >&2
    exit 1
  fi

  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    boot_count="$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)"
    app_count="$(grep -c '^APP_LIFECYCLE_OK ' "$NORMALIZED_LOG" || true)"
    app_line="$(grep '^APP_LIFECYCLE_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$boot_count" != "1" || "$app_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "App-lifecycle image did not publish exactly one evidence and boot marker." >&2
      exit 1
    fi
    if [[ ! "$app_line" =~ $APP_REGEX ]]; then
      cat "$NORMALIZED_LOG"
      echo "APP_LIFECYCLE_OK did not match the fixed M33 evidence schema." >&2
      exit 1
    fi

    protocol="${BASH_REMATCH[1]}"
    messages="${BASH_REMATCH[2]}"
    errors="${BASH_REMATCH[3]}"
    transactions="${BASH_REMATCH[4]}"
    completed="${BASH_REMATCH[5]}"
    requests="${BASH_REMATCH[6]}"
    states="${BASH_REMATCH[7]}"
    commands="${BASH_REMATCH[8]}"
    acks="${BASH_REMATCH[9]}"
    launch_actions="${BASH_REMATCH[10]}"
    activate_actions="${BASH_REMATCH[11]}"
    suspend_actions="${BASH_REMATCH[12]}"
    resume_actions="${BASH_REMATCH[13]}"
    terminate_actions="${BASH_REMATCH[14]}"
    transfer_commands="${BASH_REMATCH[15]}"
    first_instance="${BASH_REMATCH[16]}"
    last_instance="${BASH_REMATCH[17]}"
    first_pid="${BASH_REMATCH[18]}"
    last_pid="${BASH_REMATCH[19]}"
    created="${BASH_REMATCH[20]}"
    exited="${BASH_REMATCH[21]}"
    reaped="${BASH_REMATCH[22]}"
    live="${BASH_REMATCH[23]}"
    exited_reasons="${BASH_REMATCH[24]}"
    faulted_reasons="${BASH_REMATCH[25]}"
    killed_reasons="${BASH_REMATCH[26]}"
    handles="${BASH_REMATCH[27]}"
    endpoints="${BASH_REMATCH[28]}"
    object_pending="${BASH_REMATCH[29]}"
    many_pending="${BASH_REMATCH[30]}"
    array_pending="${BASH_REMATCH[31]}"
    process_wait_calls="${BASH_REMATCH[32]}"
    process_wait_completed="${BASH_REMATCH[33]}"
    process_wait_stale="${BASH_REMATCH[34]}"
    graphics_generation="${BASH_REMATCH[35]}"
    supervisor_messages="${BASH_REMATCH[36]}"
    supervisor_errors="${BASH_REMATCH[37]}"
    supervisor_transactions="${BASH_REMATCH[38]}"
    supervisor_completed="${BASH_REMATCH[39]}"
    supervisor_install="${BASH_REMATCH[40]}"
    supervisor_activate="${BASH_REMATCH[41]}"
    supervisor_show="${BASH_REMATCH[42]}"
    supervisor_retire="${BASH_REMATCH[43]}"
    supervisor_transfer="${BASH_REMATCH[44]}"

    if ((protocol != 1 \
      || messages != 35 \
      || errors != 0 \
      || transactions != 7 \
      || completed != 7 \
      || requests != 7 \
      || states != 14 \
      || commands != 7 \
      || acks != 7 \
      || launch_actions != 2 \
      || activate_actions != 2 \
      || suspend_actions != 1 \
      || resume_actions != 1 \
      || terminate_actions != 1 \
      || transfer_commands != 2 \
      || first_instance == 0 \
      || last_instance <= first_instance \
      || created != 10 \
      || exited != 2 \
      || reaped != 2 \
      || live != 8 \
      || exited_reasons != 2 \
      || faulted_reasons != 0 \
      || killed_reasons != 0 \
      || handles != 29 \
      || endpoints != 26 \
      || object_pending != 8 \
      || many_pending != 2 \
      || array_pending != 6 \
      || process_wait_calls != 3 \
      || process_wait_completed != 2 \
      || process_wait_stale != 1 \
      || graphics_generation != 2 \
      || supervisor_messages != 14 \
      || supervisor_errors != 0 \
      || supervisor_transactions != 7 \
      || supervisor_completed != 7 \
      || supervisor_install != 2 \
      || supervisor_activate != 3 \
      || supervisor_show != 1 \
      || supervisor_retire != 1 \
      || supervisor_transfer != 2)); then
      cat "$NORMALIZED_LOG"
      echo "APP_LIFECYCLE_OK violated the M33 transaction, wait, or resource ledger." >&2
      exit 1
    fi
    if [[ "$first_pid" != "0x0000000100000008" || "$last_pid" != "0x0000000200000008" ]]; then
      cat "$NORMALIZED_LOG"
      echo "APP_LIFECYCLE_OK did not prove same-slot app replacement with an advanced PID generation." >&2
      exit 1
    fi
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "App-lifecycle boot did not preserve the exact M25 storage evidence." >&2
      exit 1
    fi
    if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "App-lifecycle boot did not enter and normalize execution at EL1." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "App lifecycle runtime passed: seven authenticated transitions replaced the app in-place and retained the second generation window."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before app-lifecycle evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for app-lifecycle evidence." >&2
exit 1
