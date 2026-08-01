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
SUCCESS_MARKER="BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified"
EL0_STORAGE_MARKER="EL0_STORAGE_OK abi=23 root_capability=1 files=2 bytes=68 opens=2/6 vmo_reads=5/10 read_bytes=105 vmo_transfers=1/1 runtime_disk_reads=0 mapped=0 shared_memory=0 writes=0 persistence=0"
GRAPHICS_BUFFER_MARKER='GRAPHICS_BUFFER_OK abi=23 format=XRGB8888 width=208 height=368 logical_bytes=306176 backing_bytes=307200 slots=2 created=1 write_calls=1 write_bytes=4 presents=0 handles=2 producer_pid=0x0000000100000008 consumer_pid=0x0000000100000006 slot=0 allocation_generation=1 producer_rights=0x0000000f server_rights=0x00000009 owners_valid=1 rights_valid=1 generation_qualified=1 transferred=1 writable=1 copy_present=1'
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

TARGET_ROOT="$WORKSPACE_ROOT/target/heap-partial-map-failure-self-test"
CARGO_TARGET_DIR="$TARGET_ROOT" cargo build \
  --locked \
  --target aarch64-unknown-none \
  -p bndroid-kernel \
  --features heap-partial-map-failure-self-test

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

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-heap-rollback.XXXXXX")"
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
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  >"$LOG_FILE" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq 'fatal exception:|kernel panic:|boot error:|_INJECTED:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Heap rollback image emitted an unexpected fatal marker." >&2
    exit 1
  fi
  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Heap rollback retry did not recover the exact M25 durable-storage/M24 catalog contract." >&2
      exit 1
    fi
    rollback_count="$(grep -c '^HEAP_ROLLBACK_OK ' "$NORMALIZED_LOG" || true)"
    rollback_line="$(grep '^HEAP_ROLLBACK_OK ' "$NORMALIZED_LOG" || true)"
    rollback_regex='^HEAP_ROLLBACK_OK attempted=([0-9]+) maps=([0-9]+) unmaps=([0-9]+) table_alloc=([0-9]+) table_free=([0-9]+) frames_restored=([0-9]+) retry=([0-9]+)$'
    if [[ "$rollback_count" != "1" || ! "$rollback_line" =~ $rollback_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 17 \
        || ${BASH_REMATCH[2]:-0} != ${BASH_REMATCH[1]:-0} \
        || ${BASH_REMATCH[3]:-0} != ${BASH_REMATCH[2]:-0} \
        || ${BASH_REMATCH[4]:-0} != 1 \
        || ${BASH_REMATCH[5]:-0} != ${BASH_REMATCH[4]:-0} \
        || ${BASH_REMATCH[6]:-0} != 1 \
        || ${BASH_REMATCH[7]:-0} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Partial heap-map rollback evidence is missing or inconsistent." >&2
      exit 1
    fi
    for marker in FRAME_OK VM_OK MEMORY_OK HEAP_OK HEAP_IRQ_OK FRAMEBUFFER_OK COMPOSITOR_READY SURFACE_ACQUIRE_OK SURFACE_HANDOFF_OK USER_SURFACE_COMMIT_OK FDT_VIRTIO_OK VIRTIO_IRQ_OK VIRTIO_BLK_OK BLOCK_IRQ_OK BLOCK_LAYER_OK GPT_OK DATA_GPT_OK FAT16_OK VFS_OK DATA_PERSIST_OK STORAGE_LIMITS SCHED_OK SLEEP_OK USER_IMAGE_CATALOG_OK SYSCALL_OK COPYIO_OK HANDLE_OK IPC_OK IPC_IDENTITY_OK HANDLE_TRANSFER_OK PROC_LIFECYCLE_OK PROC_CAPACITY_OK PROCESS_IMAGE_OK UI_RUNTIME_OK GRAPHICS_BUFFER_OK SERVICE_RESTART_OK PROCESS_WAIT_OK OBJECT_WAIT_OK WAIT_MANY_OK WAIT_ARRAY_OK EVENT_OK SERVICE_MANAGER_OK SERVICE_LOOP_OK SERVICE_DYNAMIC_OK SERVICE_REUSE_OK SERVICE_ACL_OK SERVICE_MULTISESSION_OK SERVICE_WAIT_TOPOLOGY_OK RESIDENT_STABILITY_OK EL0_OK EL0_STORAGE_OK ELF_RUNTIME_OK BOOT_OK; do
      if [[ "$(grep -c "^$marker" "$NORMALIZED_LOG" || true)" != "1" ]]; then
        cat "$NORMALIZED_LOG"
        echo "Expected one $marker marker after heap initialization retry." >&2
        exit 1
      fi
    done
    if grep -Eq '^USER_INPUT_READ_OK |^SURFACE_DEGRADED ' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry unexpectedly routed input or degraded the userspace surface." >&2
      exit 1
    fi
    if ! grep -Fxq "$EL0_STORAGE_MARKER" "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact ABI-v23 EL0 storage capability and read-only VMO ledger." >&2
      exit 1
    fi
    catalog_line="$(grep '^USER_IMAGE_CATALOG_OK ' "$NORMALIZED_LOG")"
    catalog_regex='^USER_IMAGE_CATALOG_OK count=7 distinct=1 init_bytes=[1-9][0-9]* manager_bytes=[1-9][0-9]* provider_bytes=[1-9][0-9]* client_bytes=[1-9][0-9]* surface_bytes=[1-9][0-9]* launcher_bytes=[1-9][0-9]* app_bytes=[1-9][0-9]* init_digest=0x[0-9a-f]{16} manager_digest=0x[0-9a-f]{16} provider_digest=0x[0-9a-f]{16} client_digest=0x[0-9a-f]{16} surface_digest=0x[0-9a-f]{16} launcher_digest=0x[0-9a-f]{16} app_digest=0x[0-9a-f]{16}$'
    if [[ ! "$catalog_line" =~ $catalog_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the seven-image catalog contract." >&2
      exit 1
    fi
    process_image_line="$(grep '^PROCESS_IMAGE_OK ' "$NORMALIZED_LOG")"
    if [[ "$process_image_line" != 'PROCESS_IMAGE_OK abi=23 init=1 spawn_sequence=2/3/4/2/4/5/6/7 manager_reused=1 client_image_reused=1 distinct_catalog=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover ABI-v23 process-image selection." >&2
      exit 1
    fi
    ui_runtime_line="$(grep '^UI_RUNTIME_OK ' "$NORMALIZED_LOG")"
    if [[ "$ui_runtime_line" != 'UI_RUNTIME_OK protocol=3 server_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 distinct=1 ui_pairs=2 surface_owner=server launcher_surface=0 app_surface=0 server_handles=4 launcher_handles=1 app_handles=2 dual_clients=1 focus_routed=1 ready_before_present=1 single_outstanding=1 first_present_ack=1 graphics_buffer=1 resident=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the isolated userspace UI runtime." >&2
      exit 1
    fi
    graphics_buffer_line="$(grep '^GRAPHICS_BUFFER_OK ' "$NORMALIZED_LOG")"
    if [[ "$graphics_buffer_line" != "$GRAPHICS_BUFFER_MARKER" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact generation-qualified graphics-buffer transfer and ownership ledger." >&2
      exit 1
    fi
    syscall_line="$(grep '^SYSCALL_OK ' "$NORMALIZED_LOG")"
    if [[ "$syscall_line" != 'SYSCALL_OK abi=23 calls=599 successes=462 errors=136 unknown=1 private_svc=1 should_waits=3 pan_fail=0' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact ABI-v23 syscall ledger." >&2
      exit 1
    fi
    copyio_line="$(grep '^COPYIO_OK ' "$NORMALIZED_LOG")"
    copyio_regex='^COPYIO_OK max=64 in_calls=198 out_calls=198 in_bytes=7835 out_bytes=17487 in_faults=14 out_faults=11 range_rejects=1 fixups=25 misses=0 in_ec=37 in_dfsc=7 in_wnr=0 in_far=0x00000002001fa000 out_ec=37 out_dfsc=7 out_wnr=1 out_far=0x00000002001ff000 writes=54 reads=54 rollbacks=2 too_small=1 zero_len=2 roundtrip=1 pan_fail=0 uao_supported=[01] uao_fail=0$'
    if [[ ! "$copyio_line" =~ $copyio_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M24 aggregate copy-I/O ledger with the M20 multi-session envelope and byte-message transcript." >&2
      exit 1
    fi
    handle_line="$(grep '^HANDLE_OK ' "$NORMALIZED_LOG")"
    if [[ "$handle_line" != 'HANDLE_OK channel_pairs_created=48 events_created=6 duplicated=190 close_syscalls=269 stale_close_rejected=74 instrumented_rights_denials=18 init_handles=4' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M24 aggregate capability ledger." >&2
      exit 1
    fi
    ipc_line="$(grep '^IPC_OK ' "$NORMALIZED_LOG")"
    if [[ "$ipc_line" != 'IPC_OK channels=48 writes=142 reads=142 peeks=0 peek_kinds=0/0/0 tag=4660 payload=0x0123456789abcdef startup_queues_drained=1 resident_heap=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M24 aggregate channel ledger or the M20 zero-peek and authenticated final-idle transcript." >&2
      exit 1
    fi
    ipc_identity_line="$(grep '^IPC_IDENTITY_OK ' "$NORMALIZED_LOG")"
    ipc_identity_regex='^IPC_IDENTITY_OK abi=23 atomic_envelope=1 envelope_size=112 reads=135 kinds=65/34/36 preserved_rejections=15 too_small=5 bad_address=5 table_full=5 identity_reads=2 errors=0 provider_pid=(0x[0-9a-f]{16}) client_pid=(0x[0-9a-f]{16}) generation_qualified=1 writer_identity=1$'
    if [[ ! "$ipc_identity_line" =~ $ipc_identity_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M20 atomic-envelope identity and rollback ledger." >&2
      exit 1
    fi
    identity_provider_pid="${BASH_REMATCH[1]}"
    identity_client_pid="${BASH_REMATCH[2]}"
    handle_transfer_line="$(grep '^HANDLE_TRANSFER_OK ' "$NORMALIZED_LOG")"
    if [[ "$handle_transfer_line" != 'HANDLE_TRANSFER_OK startup_moves=8 channel_writes=65 channel_reads=64 read_rollbacks=30 self_rejected=5 order_rejected=5 write_badaddr_rollback=8 denied_rollback=4 rights_preserved=8 table_full_rollback=8 unread_dropped=1 init_handles=4' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M24 aggregate handle-transfer ledger." >&2
      exit 1
    fi
    process_line="$(grep '^PROC_LIFECYCLE_OK ' "$NORMALIZED_LOG")"
    process_regex='^PROC_LIFECYCLE_OK capacity=8 dynamic=7 created=9 exited=1 reaped=1 terminated_exited=1 terminated_faulted=0 terminated_killed=0 live=8 peak_live=8 spawn_waits=1 capacity_rejected=1 startup_moves=8 child_syscalls=1260 child_selections=([0-9]+) manager_v1_pid=(0x[0-9a-f]{16}) provider_pid=(0x[0-9a-f]{16}) client_pid=(0x[0-9a-f]{16}) manager_v2_pid=(0x[0-9a-f]{16}) secondary_pid=0x0000000100000005 surface_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 child_asid=8 root_isolated=1 child_frames_isolated=1 reaped_frame_restored=1 frame_delta=1 reaped_heap_restored=1 nonstandard_exits=1 non_target_reaps=0 distinct_slots=1 distinct_asids=1 distinct_roots=1 distinct_frames=1 retire_tlbi=1 aspace_tlbi=10 child_handle_isolated=1 exit_fail=0 monitor_asid=0$'
    if [[ ! "$process_line" =~ $process_regex ]] || (( ${BASH_REMATCH[1]:-0} < 7 )); then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M20 child-syscall and generation-qualified PID accounting." >&2
      exit 1
    fi
    provider_pid="${BASH_REMATCH[3]}"
    client_pid="${BASH_REMATCH[4]}"
    manager_v2_pid="${BASH_REMATCH[5]}"
    if [[ "$identity_provider_pid" != "$provider_pid" || "$identity_client_pid" != "$client_pid" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry identity transcript was not bound to the live provider/client generations." >&2
      exit 1
    fi
    object_wait_line="$(grep '^OBJECT_WAIT_OK ' "$NORMALIZED_LOG")"
    object_wait_regex='^OBJECT_WAIT_OK calls=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) immediate=([0-9]+) cancels=([0-9]+) abandoned=([0-9]+) pending=([0-9]+) latest_epoch=([0-9]+) protocol_retries=([0-9]+)$'
    if [[ ! "$object_wait_line" =~ $object_wait_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry emitted malformed object-wait evidence." >&2
      exit 1
    fi
    object_wait_calls="${BASH_REMATCH[1]}"
    object_wait_blocks="${BASH_REMATCH[2]}"
    object_wait_wakes="${BASH_REMATCH[3]}"
    object_wait_immediate="${BASH_REMATCH[4]}"
    object_wait_cancels="${BASH_REMATCH[5]}"
    object_wait_abandoned="${BASH_REMATCH[6]}"
    object_wait_pending="${BASH_REMATCH[7]}"
    object_wait_latest_epoch="${BASH_REMATCH[8]}"
    object_wait_protocol_retries="${BASH_REMATCH[9]}"
    wait_many_line="$(grep '^WAIT_MANY_OK ' "$NORMALIZED_LOG")"
    wait_many_regex='^WAIT_MANY_OK items=([0-9]+) calls=([0-9]+) immediate=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) signal_wakes=([0-9]+) finite_signal_wakes=([0-9]+) poll_timeouts=([0-9]+) timeout_wakes=([0-9]+) cancels=([0-9]+) abandoned=([0-9]+) early=([0-9]+) pending=([0-9]+) latest_epoch=([0-9]+)$'
    if [[ ! "$wait_many_line" =~ $wait_many_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry emitted malformed legacy wait-many evidence." >&2
      exit 1
    fi
    wait_many_items="${BASH_REMATCH[1]}"
    wait_many_calls="${BASH_REMATCH[2]}"
    wait_many_immediate="${BASH_REMATCH[3]}"
    wait_many_blocks="${BASH_REMATCH[4]}"
    wait_many_wakes="${BASH_REMATCH[5]}"
    wait_many_signal_wakes="${BASH_REMATCH[6]}"
    wait_many_finite_signal_wakes="${BASH_REMATCH[7]}"
    wait_many_poll_timeouts="${BASH_REMATCH[8]}"
    wait_many_timeout_wakes="${BASH_REMATCH[9]}"
    wait_many_cancels="${BASH_REMATCH[10]}"
    wait_many_abandoned="${BASH_REMATCH[11]}"
    wait_many_early="${BASH_REMATCH[12]}"
    wait_many_pending="${BASH_REMATCH[13]}"
    wait_many_latest_epoch="${BASH_REMATCH[14]}"
    wait_array_line="$(grep '^WAIT_ARRAY_OK ' "$NORMALIZED_LOG")"
    wait_array_regex='^WAIT_ARRAY_OK max_items=([0-9]+) item_bytes=([0-9]+) calls=([0-9]+) immediate=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) signal_wakes=([0-9]+) poll_timeouts=([0-9]+) timeout_wakes=([0-9]+) invalid_counts=([0-9]+) bad_address=([0-9]+) validation_rejected=([0-9]+) lowest_multi_ready=([0-9]+) scheduler_max_items=([0-9]+) cancels=([0-9]+) abandoned=([0-9]+) early=([0-9]+) pending=([0-9]+) latest_epoch=([0-9]+) canonical_le=([0-9]+) full_validation=([0-9]+) generation_qualified=([0-9]+)$'
    if [[ ! "$wait_array_line" =~ $wait_array_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry emitted malformed M19 wait-array evidence." >&2
      exit 1
    fi
    wait_array_max_items="${BASH_REMATCH[1]}"
    wait_array_item_bytes="${BASH_REMATCH[2]}"
    wait_array_calls="${BASH_REMATCH[3]}"
    wait_array_immediate="${BASH_REMATCH[4]}"
    wait_array_blocks="${BASH_REMATCH[5]}"
    wait_array_wakes="${BASH_REMATCH[6]}"
    wait_array_signal_wakes="${BASH_REMATCH[7]}"
    wait_array_poll_timeouts="${BASH_REMATCH[8]}"
    wait_array_timeout_wakes="${BASH_REMATCH[9]}"
    wait_array_invalid_counts="${BASH_REMATCH[10]}"
    wait_array_bad_address="${BASH_REMATCH[11]}"
    wait_array_validation_rejected="${BASH_REMATCH[12]}"
    wait_array_lowest_multi_ready="${BASH_REMATCH[13]}"
    wait_array_scheduler_max_items="${BASH_REMATCH[14]}"
    wait_array_cancels="${BASH_REMATCH[15]}"
    wait_array_abandoned="${BASH_REMATCH[16]}"
    wait_array_early="${BASH_REMATCH[17]}"
    wait_array_pending="${BASH_REMATCH[18]}"
    wait_array_latest_epoch="${BASH_REMATCH[19]}"
    wait_array_canonical_le="${BASH_REMATCH[20]}"
    wait_array_full_validation="${BASH_REMATCH[21]}"
    wait_array_generation_qualified="${BASH_REMATCH[22]}"
    total_wait_blocks=$((object_wait_blocks + wait_many_blocks + wait_array_blocks))
    if (( object_wait_calls != 455 \
      || object_wait_immediate + object_wait_blocks != object_wait_calls \
      || object_wait_blocks + wait_many_pending + wait_array_pending != object_wait_wakes + object_wait_pending + object_wait_abandoned + wait_many_abandoned + wait_array_abandoned \
      || object_wait_blocks != object_wait_wakes + object_wait_abandoned + 2 \
      || object_wait_cancels != 0 || object_wait_abandoned != 0 || object_wait_pending != 7 \
      || wait_many_pending + wait_array_pending + 2 != object_wait_pending \
      || object_wait_latest_epoch < 1 || object_wait_latest_epoch > total_wait_blocks \
      || object_wait_protocol_retries != 1 \
      || wait_many_items != 2 || wait_many_calls != 37 \
      || wait_many_immediate < 5 || wait_many_immediate > 25 \
      || wait_many_blocks < 7 || wait_many_blocks > 27 \
      || wait_many_blocks != wait_many_wakes + wait_many_pending + wait_many_abandoned \
      || wait_many_signal_wakes > 20 \
      || wait_many_finite_signal_wakes > 4 \
      || wait_many_finite_signal_wakes > wait_many_signal_wakes \
      || wait_many_poll_timeouts != 5 || wait_many_timeout_wakes != 5 \
      || wait_many_cancels != 0 || wait_many_abandoned != 0 || wait_many_early != 0 || wait_many_pending != 2 \
      || wait_many_latest_epoch < 1 || wait_many_latest_epoch > total_wait_blocks \
      || wait_many_calls != wait_many_immediate + wait_many_poll_timeouts + wait_many_blocks \
      || wait_many_wakes != wait_many_signal_wakes + wait_many_timeout_wakes + wait_many_cancels \
      || wait_many_immediate + wait_many_signal_wakes != 25 \
      || wait_array_max_items != 8 || wait_array_item_bytes != 8 \
      || wait_array_calls != 57 || wait_array_immediate < 1 || wait_array_immediate > 52 \
      || wait_array_blocks < 2 || wait_array_blocks > 53 \
      || wait_array_blocks != wait_array_wakes + wait_array_pending + wait_array_abandoned \
      || wait_array_signal_wakes > 50 || wait_array_poll_timeouts != 1 \
      || wait_array_timeout_wakes != 0 || wait_array_invalid_counts != 2 \
      || wait_array_bad_address != 2 || wait_array_validation_rejected != 1 \
      || wait_array_lowest_multi_ready != 1 || wait_array_scheduler_max_items != 8 \
      || wait_array_cancels != 0 || wait_array_abandoned != 0 || wait_array_early != 0 || wait_array_pending != 3 \
      || wait_array_latest_epoch < 1 || wait_array_latest_epoch > total_wait_blocks \
      || wait_array_calls != wait_array_immediate + wait_array_poll_timeouts + wait_array_blocks \
      || wait_array_wakes != wait_array_signal_wakes + wait_array_timeout_wakes + wait_array_cancels \
      || wait_array_immediate + wait_array_signal_wakes != 53 \
      || wait_array_canonical_le != 1 || wait_array_full_validation != 1 \
      || wait_array_generation_qualified != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M20 object-wait, legacy wait-many, wait-array ledgers, and scheduler-tolerant conservation laws." >&2
      exit 1
    fi
    event_line="$(grep '^EVENT_OK ' "$NORMALIZED_LOG")"
    event_regex='^EVENT_OK created=6 signal_calls=16 signal_edges=11 clear_calls=16 clear_edges=11 signal_denied=5 waits=10 blocks=([0-9]+) wakes=([0-9]+) transfer_writes=5 transfer_reads=5 init_handles=4$'
    if [[ ! "$event_line" =~ $event_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != ${BASH_REMATCH[2]:-0} \
        || ${BASH_REMATCH[1]:-0} > 10 )); then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M20 temporary Event and wait-array wake ledger." >&2
      exit 1
    fi
    service_manager_line="$(grep '^SERVICE_MANAGER_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_manager_line" != 'SERVICE_MANAGER_OK protocol=1 supervisor=1 daemon=1 supervised=1 bootstrap_identity=1 registry_capacity=4 registry_state_reachable=1 resident_roles=3 core_resident_processes=4 ui_processes=3 total_resident_processes=7 resident_clients=2 serving_loop=1 dynamic_lifecycle=1 dynamic_registration_loop=1 multi_session_round=1 general_runtime=0' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the M20 ServiceManager resident multi-session state." >&2
      exit 1
    fi
    service_dynamic_line="$(grep '^SERVICE_DYNAMIC_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_dynamic_line" != 'SERVICE_DYNAMIC_OK provider_driven=1 causal_order=1 startup_peer_bound=1 writer_identity=1 register=2 unregister=2 reregister=1 stale_rejected=1 gap_not_found=1 availability=2 phase=10 done_bitmap=0x7 old_instance=0x00020002 new_instance=0x00020003 txids=0x201/0x202/0x203/0x204/0x205/0x206/0x207/0x208 transcript_errors=0 idle_wait_graph=1 reusable=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not preserve the exact authenticated M15 predecessor transcript while proving its M20 reusable state." >&2
      exit 1
    fi
    service_reuse_line="$(grep '^SERVICE_REUSE_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_reuse_line" != 'SERVICE_REUSE_OK round=1 step=5 completed_rounds=1 errors=0 instance=0x00020004 txids=0x301/0x302/0x303/0x304 done_reads=3 done_bitmap=0x7 register=1 lookup_echo=1 causal_confirm=1 unregister=1 not_found=1 cleanup=1 exact_idle_topology=1 constant_space=1 bounded_reuse=1 general_runtime=0 startup_peer_bound=1 writer_identity=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact authenticated bounded post-cleanup reuse transcript and truthful production boundary." >&2
      exit 1
    fi
    service_acl_line="$(grep '^SERVICE_ACL_OK ' "$NORMALIZED_LOG")"
    expected_service_acl_line="SERVICE_ACL_OK manager_pid=$manager_v2_pid provider_pid=$provider_pid client_pid=$client_pid authenticated_attach=1 owner_from_sender=1 malformed_rejected=4 manager_resilient=1 delegated_endpoint_denied=1 registry_unchanged=1 connector_closed=1 handle_leaks=0 acl_reads=1 errors=0 writer_identity=1"
    if [[ "$service_acl_line" != "$expected_service_acl_line" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact delegated-endpoint denial or bind it to the resident generation-qualified PIDs." >&2
      exit 1
    fi
    service_multisession_line="$(grep '^SERVICE_MULTISESSION_OK ' "$NORMALIZED_LOG")"
    service_multisession_regex="^SERVICE_MULTISESSION_OK phase=23 clients=2 resident_clients=2 manager_pid=$manager_v2_pid primary_pid=$client_pid secondary_pid=0x0000000100000005 manager_attaches=2 client_attaches=2 revoke_bitmap=0x3 stale_revoke_bitmap=0x3 primary_progress_bitmap=0x3 provider_accepts=4 provider_echoes=3 provider_aborts=1 secondary_echoes=1 final_idle_bitmap=0x3 sender_authenticated=1 generation_qualified=1 independent_sessions=2 secondary_lease_generations=2 errors=0 process_crash_restart=0 general_runtime=0$"
    if [[ ! "$service_multisession_line" =~ $service_multisession_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M20 authenticated two-session attach, revoke, isolation, or reattach transcript." >&2
      exit 1
    fi
    service_loop_line="$(grep '^SERVICE_LOOP_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_loop_line" != 'SERVICE_LOOP_OK post_ready_requests=2 distinct_txids=2 client_echo_commits=2 manager_done_rounds=2 provider_done_rounds=2 done_commits=2 transcript_errors=0 reentrant=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the two-round committed post-ready service transcript." >&2
      exit 1
    fi
    service_wait_topology_line="$(grep '^SERVICE_WAIT_TOPOLOGY_OK ' "$NORMALIZED_LOG")"
    service_wait_topology_regex='^SERVICE_WAIT_TOPOLOGY_OK endpoints=20 pairs=10 core_cross_links=1/1/2/2/2/0 ui_pairs=2 unique=1 rights=1 queues_empty=1 manager_items=4 provider_items=2 primary_items=2 secondary_items=2 surface_items=3 launcher_items=1 app_items=1 wait_mask=0x5 infinite_deadlines=1 manager_epoch=([0-9]+) provider_epoch=([0-9]+) primary_epoch=([0-9]+) secondary_epoch=([0-9]+) surface_epoch=([0-9]+) launcher_epoch=([0-9]+) app_epoch=([0-9]+) token_stable=1 supervisor_target=0x0000000200000002$'
    if [[ ! "$service_wait_topology_line" =~ $service_wait_topology_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} == 0 \
        || ${BASH_REMATCH[2]:-0} == 0 \
        || ${BASH_REMATCH[3]:-0} == 0 \
        || ${BASH_REMATCH[4]:-0} == 0 \
        || ${BASH_REMATCH[5]:-0} == 0 \
        || ${BASH_REMATCH[6]:-0} == 0 \
        || ${BASH_REMATCH[7]:-0} == 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the exact M32 capability/channel graph and seven-token idle wait topology." >&2
      exit 1
    fi
    resident_line="$(grep '^RESIDENT_STABILITY_OK ' "$NORMALIZED_LOG")"
    resident_regex='^RESIDENT_STABILITY_OK stable_ticks=4 first_manager_heap_restored=1 first_manager_frames_restored=1 frame_delta=1 heap_baseline=([0-9]+) heap_now=([0-9]+) allocated_frames=([0-9]+) init_handles=4 total_handles=23 handles_by_image=4/5/3/4/4/1/2 service_graph=1 all_queues_empty=1 wait_topology=1 object_wait_slots=7 wait_many_slots=2 wait_array_slots=3 supervisor_waits=1 completions_pending=0 contexts=7 kernel_stacks=7 aspace_live=8 live_processes=8 live_images=1/1/1/2/1/1/1$'
    if [[ ! "$resident_line" =~ $resident_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} <= ${BASH_REMATCH[2]:-0} \
        || ${BASH_REMATCH[2]:-0} == 0 \
        || ${BASH_REMATCH[3]:-0} == 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Heap retry did not recover the stable eight-process, isolated dual-client UI supervision topology." >&2
      exit 1
    fi
    if grep -Eq '^RESIDENT_RESOURCE_OK |^SERVICE_RESOURCE_OK |^BOOT_OK: M20 bounded resilient multi-session services and M19 wait-array verified$|^BOOT_OK: M19 bounded eight-item wait-array ABI and M18 services verified$|^BOOT_OK: M18 authenticated concurrent two-client service round verified$|^BOOT_OK: M17 authenticated IPC identity and service ACL verified$|^BOOT_OK: M16 post-cleanup dynamic service reuse verified$|^BOOT_OK: M15 provider-driven dynamic service lifecycle verified$|^BOOT_OK: M14 repeated post-ready service echo and exact idle topology verified$|^BOOT_OK: M13 distinct user images and reachable resident state verified$|^BOOT_OK: M13 distinct user images and resident core services ready$|^BOOT_OK: M12 supervised ServiceManager restart verified$' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Heap rollback recovery published stale pre-M20 multi-client, cleanup, or resident evidence." >&2
      exit 1
    fi
    cat "$NORMALIZED_LOG"
    echo "Heap rollback self-test passed: 17 partial mappings were reclaimed, M32 graphics-buffer UI isolation plus M25 durable data recovered, and all eight processes converged to their exact idle topology."
    exit 0
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    cat "$NORMALIZED_LOG"
    echo "QEMU exited before heap rollback recovery completed (status $status)." >&2
    exit 1
  fi
  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out waiting for heap rollback recovery." >&2
exit 1
