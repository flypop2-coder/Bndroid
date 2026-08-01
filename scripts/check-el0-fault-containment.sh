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

TARGET_ROOT="$WORKSPACE_ROOT/target/el0-fault-containment-self-test"
CARGO_TARGET_DIR="$TARGET_ROOT" cargo build \
  --locked \
  --target aarch64-unknown-none \
  -p bndroid-kernel \
  --features el0-fault-containment-self-test

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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-el0-fault.XXXXXX")"
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

SUCCESS_MARKER="BOOT_OK: M2 EL0 fault containment self-test survived"
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq 'fatal exception:|kernel panic:|boot error:|EL0 fault-containment self-test failed|_INJECTED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "EL0 permission fault escaped containment or corrupted kernel state." >&2
    exit 1
  fi
  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "EL0 fault-containment path did not complete the exact M25 durable-storage/M24 catalog contract before injection." >&2
      exit 1
    fi
    fault_count="$(grep -c '^EL0_FAULT_CONTAINED_OK ' "$NORMALIZED_LOG" || true)"
    fault_line="$(grep '^EL0_FAULT_CONTAINED_OK ' "$NORMALIZED_LOG" || true)"
    fault_regex='^EL0_FAULT_CONTAINED_OK reason=([0-9]+) ec=([0-9]+) dfsc=([0-9]+) wnr=([0-9]+) far=(0x[0-9a-f]+) elr=(0x[0-9a-f]+) sp=(0x[0-9a-f]+) faults=([0-9]+) sync_dispatches=([0-9]+) timer_after=([0-9]+) workers_alive=([0-9]+) heap_restored=([0-9]+) monitor_asid=([0-9]+) copy_fixups=([0-9]+) copy_misses=([0-9]+)$'
    if [[ "$fault_count" != "1" || ! "$fault_line" =~ $fault_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 2 \
        || ${BASH_REMATCH[2]:-0} != 36 \
        || ${BASH_REMATCH[3]:-0} != 15 \
        || ${BASH_REMATCH[4]:-1} != 0 \
        || ${BASH_REMATCH[5]:-0} != 0x0000000040080000 \
        || ${BASH_REMATCH[6]:-0} < 0x0000000200000000 \
        || ${BASH_REMATCH[6]:-0} >= 0x0000000200001000 \
        || ${BASH_REMATCH[7]:-0} != 0x00000002001ff000 \
        || ${BASH_REMATCH[8]:-0} != 1 \
        || ${BASH_REMATCH[9]:-0} != 1 \
        || ${BASH_REMATCH[10]:-0} < 6 \
        || ${BASH_REMATCH[11]:-0} != 1 \
        || ${BASH_REMATCH[12]:-0} != 1 \
        || ${BASH_REMATCH[13]:-1} != 0 \
        || ${BASH_REMATCH[14]:-1} != 0 \
        || ${BASH_REMATCH[15]:-1} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Contained EL0 fault syndrome or post-fault liveness evidence is inconsistent." >&2
      exit 1
    fi
    if grep -Eq '^USER_IMAGE_CATALOG_OK |^SYSCALL_OK |^COPYIO_OK |^HANDLE_OK |^IPC_OK |^IPC_IDENTITY_OK |^HANDLE_TRANSFER_OK |^PROC_LIFECYCLE_OK |^PROC_CAPACITY_OK |^PROCESS_IMAGE_OK |^UI_RUNTIME_OK |^GRAPHICS_BUFFER_CREATE_OK |^GRAPHICS_BUFFER_OK |^SERVICE_RESTART_OK |^PROCESS_WAIT_OK |^OBJECT_WAIT_OK |^WAIT_MANY_OK |^WAIT_ARRAY_OK |^EVENT_OK |^SERVICE_MANAGER_OK |^SERVICE_LOOP_OK |^SERVICE_DYNAMIC_OK |^SERVICE_REUSE_OK |^SERVICE_ACL_OK |^SERVICE_MULTICLIENT_OK |^SERVICE_MULTISESSION_OK |^SERVICE_WAIT_TOPOLOGY_OK |^RESIDENT_STABILITY_OK |^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^EL0_OK |^EL0_STORAGE_OK |^ELF_RUNTIME_OK |^INIT_M16_|^INIT_M17_|^INIT_M19_|^INIT_M20_|^BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified$|^BOOT_OK: M31 independent app focus routing, M25 durable data records, and M20 multi-session services verified$|^BOOT_OK: M30 userspace UI server and launcher, M25 durable data records, and M20 multi-session services verified$|^BOOT_OK: M25 durable data records, M24 EL0 storage VMOs, and M20 multi-session services verified$|^BOOT_OK: M24 EL0 read-only storage VMOs and M20 multi-session services verified$|^BOOT_OK: M23 read-only GPT/FAT16/VFS and M20 multi-session services verified$|^BOOT_OK: M20 bounded resilient multi-session services and M19 wait-array verified$|^BOOT_OK: M19 bounded eight-item wait-array ABI and M18 services verified$|^BOOT_OK: M18 authenticated concurrent two-client service round verified$|^BOOT_OK: M17 authenticated IPC identity and service ACL verified$|^BOOT_OK: M16 post-cleanup dynamic service reuse verified$|^BOOT_OK: M15 provider-driven dynamic service lifecycle verified$|^BOOT_OK: M14 repeated post-ready service echo and exact idle topology verified$|^BOOT_OK: M13 distinct user images and reachable resident state verified$|^BOOT_OK: M13 distinct user images and resident core services ready$|^BOOT_OK: M12 supervised ServiceManager restart verified$' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Fault-injection image incorrectly published normal ABI/storage, wait-array, image-catalog, atomic-envelope identity, M20 multi-session service, or resident evidence instead of the zeroed fault-only path." >&2
      exit 1
    fi
    user_map_count="$(grep -c '^USER_MAP_OK ' "$NORMALIZED_LOG" || true)"
    user_map_line="$(grep '^USER_MAP_OK ' "$NORMALIZED_LOG" || true)"
    user_map_regex='^USER_MAP_OK code=(0x[0-9a-f]+) entry=(0x[0-9a-f]+) code_end=(0x[0-9a-f]+) code_bytes=([0-9]+) code_pa=(0x[0-9a-f]+) stack=(0x[0-9a-f]+) stack_end=(0x[0-9a-f]+) stack_pages=([0-9]+) stack_pa=(0x[0-9a-f]+) guard_low=(0x[0-9a-f]+) guard_high=(0x[0-9a-f]+) guards_unmapped=([0-9]+) mapped_pages=([0-9]+) unique_frames=([0-9]+) leafs_verified=([0-9]+)$'
    if [[ "$user_map_count" != "1" || ! "$user_map_line" =~ $user_map_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 0x0000000200000000 \
        || ${BASH_REMATCH[2]:-0} < ${BASH_REMATCH[1]:-0} \
        || ${BASH_REMATCH[2]:-0} >= ${BASH_REMATCH[3]:-0} \
        || ${BASH_REMATCH[2]:-0} % 4 != 0 \
        || ${BASH_REMATCH[3]:-0} != ${BASH_REMATCH[1]:-0} + ${BASH_REMATCH[4]:-0} \
        || ${BASH_REMATCH[4]:-0} == 0 || ${BASH_REMATCH[4]:-0} > 4096 \
        || ${BASH_REMATCH[5]:-0} == ${BASH_REMATCH[9]:-0} \
        || ${BASH_REMATCH[5]:-0} % 4096 != 0 \
        || ${BASH_REMATCH[6]:-0} != 0x00000002001fb000 \
        || ${BASH_REMATCH[7]:-0} != 0x00000002001ff000 \
        || ${BASH_REMATCH[8]:-0} != 4 \
        || ${BASH_REMATCH[9]:-0} % 4096 != 0 \
        || ${BASH_REMATCH[10]:-0} != 0x00000002001fa000 \
        || ${BASH_REMATCH[11]:-0} != 0x00000002001ff000 \
        || ${BASH_REMATCH[12]:-0} != 2 \
        || ${BASH_REMATCH[13]:-0} != 5 \
        || ${BASH_REMATCH[14]:-0} != 5 \
        || ${BASH_REMATCH[15]:-0} != 5 )); then
      cat "$NORMALIZED_LOG"
      echo "Fault-injection image did not publish one internally consistent five-page protected user mapping." >&2
      exit 1
    fi

    elf_load_count="$(grep -c '^ELF_LOAD_OK ' "$NORMALIZED_LOG" || true)"
    elf_load_line="$(grep '^ELF_LOAD_OK ' "$NORMALIZED_LOG" || true)"
    elf_load_regex='^ELF_LOAD_OK elf=([0-9]+) segments=([0-9]+) load_pages=([0-9]+) rx_pages=([0-9]+) ro_pages=([0-9]+) rw_pages=([0-9]+) bss_bytes=([0-9]+) elf_bytes=([0-9]+) file_bytes=([0-9]+) memory_bytes=([0-9]+) wx=([0-9]+)$'
    if [[ "$elf_load_count" != "1" || ! "$elf_load_line" =~ $elf_load_regex ]] \
      || (( ${BASH_REMATCH[1]:-1} != 0 \
        || ${BASH_REMATCH[2]:-1} != 0 \
        || ${BASH_REMATCH[3]:-0} != 1 \
        || ${BASH_REMATCH[4]:-0} != 1 \
        || ${BASH_REMATCH[5]:-1} != 0 \
        || ${BASH_REMATCH[6]:-1} != 0 \
        || ${BASH_REMATCH[7]:-1} != 0 \
        || ${BASH_REMATCH[8]:-1} != 0 \
        || ${BASH_REMATCH[9]:-0} == 0 \
        || ${BASH_REMATCH[10]:-0} != ${BASH_REMATCH[9]:-0} \
        || ${BASH_REMATCH[11]:-1} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Fault-injection image did not publish the exact raw-memory RX load profile." >&2
      exit 1
    fi

    aspace_count="$(grep -c '^ASPACE_OK ' "$NORMALIZED_LOG" || true)"
    aspace_line="$(grep '^ASPACE_OK ' "$NORMALIZED_LOG" || true)"
    aspace_regex='^ASPACE_OK kernel_root=(0x[0-9a-f]+) user_root=(0x[0-9a-f]+) asid=([0-9]+) private_tables=([0-9]+) boot_user_unmapped=([0-9]+) heap_shared=([0-9]+) tlbi=([0-9]+)$'
    if [[ "$aspace_count" != "1" || ! "$aspace_line" =~ $aspace_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} == 0 \
        || ${BASH_REMATCH[2]:-0} == 0 \
        || ${BASH_REMATCH[1]:-0} == ${BASH_REMATCH[2]:-0} \
        || ${BASH_REMATCH[1]:-0} % 4096 != 0 \
        || ${BASH_REMATCH[2]:-0} % 4096 != 0 \
        || ${BASH_REMATCH[3]:-0} != 1 \
        || ${BASH_REMATCH[4]:-0} != 4 \
        || ${BASH_REMATCH[5]:-0} != 1 \
        || ${BASH_REMATCH[6]:-0} != 1 \
        || ${BASH_REMATCH[7]:-0} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Fault-injection image did not run in one isolated ASID-1 address space." >&2
      exit 1
    fi
    cat "$NORMALIZED_LOG"
    echo "EL0 fault containment self-test passed: kernel read was denied and scheduling survived."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before EL0 fault containment evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for EL0 fault containment evidence." >&2
exit 1
