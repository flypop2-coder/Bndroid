#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "qemu-system-aarch64 not found. Install QEMU first, then rerun this script." >&2
  exit 1
fi

"$SCRIPT_DIR/build-kernel.sh"

SUCCESS_MARKER="BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified"
EL0_STORAGE_MARKER="EL0_STORAGE_OK abi=23 root_capability=1 files=2 bytes=68 opens=2/6 vmo_reads=5/10 read_bytes=105 vmo_transfers=1/1 runtime_disk_reads=0 mapped=0 shared_memory=0 writes=0 persistence=0"
GRAPHICS_BUFFER_MARKER='GRAPHICS_BUFFER_OK abi=23 format=XRGB8888 width=208 height=368 logical_bytes=306176 backing_bytes=307200 slots=2 created=1 write_calls=1 write_bytes=4 presents=0 handles=2 producer_pid=0x0000000100000008 consumer_pid=0x0000000100000006 slot=0 allocation_generation=1 producer_rights=0x0000000f server_rights=0x00000009 owners_valid=1 rights_valid=1 generation_qualified=1 transferred=1 writable=1 copy_present=1'
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-10}"
VIRTUALIZATION="${BNDROID_QEMU_VIRTUALIZATION:-off}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if [[ "$VIRTUALIZATION" != "off" && "$VIRTUALIZATION" != "on" ]]; then
  echo "BNDROID_QEMU_VIRTUALIZATION must be 'on' or 'off'." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi

BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-qemu-boot.XXXXXX")"
LOG_FILE="$TMP_DIR/qemu.log"
NORMALIZED_LOG="$TMP_DIR/qemu.normalized.log"
PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
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
  -machine "virt,gic-version=2,secure=off,virtualization=$VIRTUALIZATION" \
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
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))

while ((SECONDS < deadline)); do
  tr -d '\r' <"$LOG_FILE" >"$NORMALIZED_LOG"
  if grep -Eq 'fatal exception:|kernel panic:|boot error:|FRAME_FAIL:|VM_FAIL:|HEAP_FAIL:|SCHED_FAIL:|SLEEP_FAIL:|USER_FAIL:|SYSCALL_FAIL:|HANDLE_FAIL:|IPC_FAIL:|IPC_IDENTITY_FAIL:|SERVICE_ACL_FAIL:|EL0_FAIL:|THREAD_EXITED:|_INJECTED:|VM_UNMAPPED_ACCESS_|COPYIO_FIXUP_MISS' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    echo "Kernel emitted a fatal error before completing scheduler validation." >&2
    exit 1
  fi
  if grep -Eq '^FRAME_OK .* unique=[0-9]+$' "$NORMALIZED_LOG" \
    && grep -Eq '^VM_OK .* missing_unmap=[0-9]+$' "$NORMALIZED_LOG" \
    && grep -Fxq 'MEMORY_OK: M1 reclaiming-frame and per-page-mapping foundation is alive' "$NORMALIZED_LOG" \
    && grep -Eq '^HEAP_OK .* checksum=0x[0-9a-f]+$' "$NORMALIZED_LOG" \
    && grep -Eq '^HEAP_IRQ_OK .* free_after=[0-9]+$' "$NORMALIZED_LOG" \
    && grep -Eq '^FRAMEBUFFER_OK transport=ramfb fw_cfg=mmio fw_cfg_base=0x[0-9a-f]+ fw_cfg_bytes=24 features=0x3 dma=1 dma_ops=3 directory_files=[1-9][0-9]* selector=0x[0-9a-f]{4} width=320 height=480 stride=1280 format=XRGB8888 bytes=614400 address=0x[0-9a-f]+ page_aligned=1 digest=0x6ef9c2b7d15fde25 sample_status=0x002e66f5 sample_cyan=0x0006b6d4 sample_purple=0x008b5cf6 sample_green=0x0022c55e configured=1 screenshot_verified=0 input=0 compositor=1$' "$NORMALIZED_LOG" \
    && grep -Eq '^COMPOSITOR_READY layers=2 scene=opaque cursor=alpha scene_bytes=614400 scanout_bytes=614400 scene_address=0x[0-9a-f]+ scanout_address=0x[0-9a-f]+ page_aligned=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25 cursor_hidden=1 dirty_rect=1 dynamic_redraw=1 input_bound=0$' "$NORMALIZED_LOG" \
    && grep -Eq '^VIRTIO_INPUT_OK transport=mmio version=2 device_id=18 base=0x000000000a003c00 queue=8 event_bytes=8 dma_frames=1 dma_coherent=1 mode=interrupt irq_enabled=1 irq=78 spec=0/46/1 trigger=edge-rising name_bytes=[1-9][0-9]* name_digest=0x[0-9a-f]+ key_bitmap_bytes=[1-9][0-9]* key_a=1 key_enter=1 writable_buffers=8 completions=0 recycled=0 dropped=0 invalid=0$' "$NORMALIZED_LOG" \
    && grep -Fxq 'INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1' "$NORMALIZED_LOG" \
    && grep -Eq '^VIRTIO_POINTER_OK transport=mmio version=2 device_id=18 base=0x000000000a003a00 queue=8 status_queue=present event_bytes=8 dma_frames=1 dma_coherent=1 mode=interrupt irq_enabled=1 irq=77 spec=0/45/1 trigger=edge-rising name_bytes=18 name_digest=0xc193f384ee1e1bac key_bitmap_bytes=43 abs_bitmap_bytes=1 abs_x=0/32767 abs_y=0/32767 btn_left=1 btn_touch=1 writable_buffers=8 completions=0 recycled=0 dropped=0 invalid=0$' "$NORMALIZED_LOG" \
    && grep -Fxq 'POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1' "$NORMALIZED_LOG" \
    && grep -Eq '^SCHED_OK .* stack_fail=[0-9]+$' "$NORMALIZED_LOG" \
    && grep -Eq '^SLEEP_OK .* svc_fail=[0-9]+$' "$NORMALIZED_LOG" \
    && [[ "$(grep -c '^SLEEP_CTX ' "$NORMALIZED_LOG" || true)" == "2" ]] \
    && grep -Eq '^USER_MAP_OK .* stack_pages=4 .* guards_unmapped=2 mapped_pages=[1-9][0-9]* unique_frames=[1-9][0-9]* leafs_verified=[1-9][0-9]*$' "$NORMALIZED_LOG" \
    && grep -Eq '^ELF_LOAD_OK elf=1 segments=3 load_pages=[1-9][0-9]* rx_pages=[1-9][0-9]* ro_pages=[1-9][0-9]* rw_pages=1 bss_bytes=[1-9][0-9]* elf_bytes=[1-9][0-9]* file_bytes=[1-9][0-9]* memory_bytes=[1-9][0-9]* wx=0$' "$NORMALIZED_LOG" \
    && grep -Fxq 'ELF_RUNTIME_OK ro_read=1 data_init=1 data_write=1 bss_zero=1 bss_write=1 stack_pages_touched=4 child_fresh=8 simultaneous=7 distinct_images=7 resident_children=7 client_instances=2 simultaneous_clients=2 ui_clients=2 transient_clients=0' "$NORMALIZED_LOG" \
    && grep -Eq '^ASPACE_OK kernel_root=0x[0-9a-f]+ user_root=0x[0-9a-f]+ asid=1 private_tables=4 boot_user_unmapped=1 heap_shared=1 tlbi=1$' "$NORMALIZED_LOG" \
    && grep -Eq '^USER_IMAGE_CATALOG_OK count=7 distinct=1 init_bytes=[1-9][0-9]* manager_bytes=[1-9][0-9]* provider_bytes=[1-9][0-9]* client_bytes=[1-9][0-9]* surface_bytes=[1-9][0-9]* launcher_bytes=[1-9][0-9]* app_bytes=[1-9][0-9]* init_digest=0x[0-9a-f]+ manager_digest=0x[0-9a-f]+ provider_digest=0x[0-9a-f]+ client_digest=0x[0-9a-f]+ surface_digest=0x[0-9a-f]+ launcher_digest=0x[0-9a-f]+ app_digest=0x[0-9a-f]+$' "$NORMALIZED_LOG" \
    && grep -Eq '^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=[1-9][0-9]* session=[1-9][0-9]* handle=[1-9][0-9]* rights=0x00000103 unique=1 duplicate=0 transferable=0 input_capacity=64$' "$NORMALIZED_LOG" \
    && grep -Eq '^SURFACE_HANDOFF_OK from=kernel-fallback to=userspace-bound caller=el0 pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25$' "$NORMALIZED_LOG" \
    && grep -Eq '^USER_SURFACE_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 commit=1 mode=full rects=4 local_damage=0/0/208/368 global_damage=56/64/208/368 raster_writes=124544 composition=56/64/208/368 restored=76544 blended=0 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25 input_enqueued=0 input_dequeued=0 input_pending=0 input_coalesced=0 cursor_preserved=1 dma_barrier=1$' "$NORMALIZED_LOG" \
    && grep -Fxq 'SYSCALL_OK abi=23 calls=599 successes=462 errors=136 unknown=1 private_svc=1 should_waits=3 pan_fail=0' "$NORMALIZED_LOG" \
    && grep -Eq '^COPYIO_OK max=64 in_calls=198 out_calls=198 in_bytes=7835 out_bytes=17487 in_faults=14 out_faults=11 range_rejects=1 fixups=25 misses=0 in_ec=37 in_dfsc=7 in_wnr=0 in_far=0x00000002001fa000 out_ec=37 out_dfsc=7 out_wnr=1 out_far=0x00000002001ff000 writes=54 reads=54 rollbacks=2 too_small=1 zero_len=2 roundtrip=1 pan_fail=0 uao_supported=[01] uao_fail=0$' "$NORMALIZED_LOG" \
    && grep -Fxq 'HANDLE_OK channel_pairs_created=48 events_created=6 duplicated=190 close_syscalls=269 stale_close_rejected=74 instrumented_rights_denials=18 init_handles=4' "$NORMALIZED_LOG" \
    && grep -Fxq 'IPC_OK channels=48 writes=142 reads=142 peeks=0 peek_kinds=0/0/0 tag=4660 payload=0x0123456789abcdef startup_queues_drained=1 resident_heap=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'IPC_IDENTITY_OK abi=23 atomic_envelope=1 envelope_size=112 reads=135 kinds=65/34/36 preserved_rejections=15 too_small=5 bad_address=5 table_full=5 identity_reads=2 errors=0 provider_pid=0x0000000100000003 client_pid=0x0000000100000004 generation_qualified=1 writer_identity=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'HANDLE_TRANSFER_OK startup_moves=8 channel_writes=65 channel_reads=64 read_rollbacks=30 self_rejected=5 order_rejected=5 write_badaddr_rollback=8 denied_rollback=4 rights_preserved=8 table_full_rollback=8 unread_dropped=1 init_handles=4' "$NORMALIZED_LOG" \
    && grep -Eq '^PROC_LIFECYCLE_OK .* reaped=1 terminated_exited=1 terminated_faulted=0 terminated_killed=0 .* monitor_asid=0$' "$NORMALIZED_LOG" \
    && grep -Fxq 'PROC_CAPACITY_OK capacity=8 dynamic=7 peak_live=8 full_rejected=1 startup_preserved=1 distinct_slots=1 distinct_asids=1 distinct_roots=1 distinct_frames=1 frame_delta=1 manager_pid=0x0000000100000002 provider_pid=0x0000000100000003 client_pid=0x0000000100000004 surface_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008' "$NORMALIZED_LOG" \
    && grep -Fxq 'PROCESS_IMAGE_OK abi=23 init=1 spawn_sequence=2/3/4/2/4/5/6/7 manager_reused=1 client_image_reused=1 distinct_catalog=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'UI_RUNTIME_OK protocol=3 server_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 distinct=1 ui_pairs=2 surface_owner=server launcher_surface=0 app_surface=0 server_handles=4 launcher_handles=1 app_handles=2 dual_clients=1 focus_routed=1 ready_before_present=1 single_outstanding=1 first_present_ack=1 graphics_buffer=1 resident=1' "$NORMALIZED_LOG" \
    && grep -Fxq "$GRAPHICS_BUFFER_MARKER" "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_RESTART_OK requests=1 restarts=1 sessions_rebound=2 dependents_survived=2 peer_close_acks=4 slot_reused=1 generation_advanced=1 stale_pid_rejected=1 stale_instance_rejected=1 old_pid=0x0000000100000002 new_pid=0x0000000200000002 old_instance=0x00010001 new_instance=0x00020001' "$NORMALIZED_LOG" \
    && grep -Eq '^PROCESS_WAIT_OK calls=3 completed=1 blocks=[12] wakes=[01] immediate=[01] stale=1 pending=1 supervisor_target=0x0000000200000002 spawn_retries=1 ready_retries=1$' "$NORMALIZED_LOG" \
    && grep -Eq '^OBJECT_WAIT_OK calls=455 blocks=[0-9]+ wakes=[0-9]+ immediate=[0-9]+ cancels=0 abandoned=0 pending=7 latest_epoch=[1-9][0-9]* protocol_retries=1$' "$NORMALIZED_LOG" \
    && grep -Eq '^WAIT_MANY_OK items=2 calls=37 immediate=([5-9]|1[0-9]|2[0-5]) blocks=([7-9]|1[0-9]|2[0-7]) wakes=([5-9]|1[0-9]|2[0-5]) signal_wakes=([0-9]|1[0-9]|20) finite_signal_wakes=[0-4] poll_timeouts=5 timeout_wakes=5 cancels=0 abandoned=0 early=0 pending=2 latest_epoch=[1-9][0-9]*$' "$NORMALIZED_LOG" \
    && grep -Eq '^WAIT_ARRAY_OK max_items=8 item_bytes=8 calls=57 immediate=([1-9]|[1-4][0-9]|5[0-2]) blocks=([2-9]|[1-4][0-9]|5[0-3]) wakes=([0-9]|[1-4][0-9]|50) signal_wakes=([0-9]|[1-4][0-9]|50) poll_timeouts=1 timeout_wakes=0 invalid_counts=2 bad_address=2 validation_rejected=1 lowest_multi_ready=1 scheduler_max_items=8 cancels=0 abandoned=0 early=0 pending=3 latest_epoch=[1-9][0-9]* canonical_le=1 full_validation=1 generation_qualified=1$' "$NORMALIZED_LOG" \
    && grep -Eq '^EVENT_OK created=6 signal_calls=16 signal_edges=11 clear_calls=16 clear_edges=11 signal_denied=5 waits=10 blocks=[0-9]+ wakes=[0-9]+ transfer_writes=5 transfer_reads=5 init_handles=4$' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_MANAGER_OK protocol=1 supervisor=1 daemon=1 supervised=1 bootstrap_identity=1 registry_capacity=4 registry_state_reachable=1 resident_roles=3 core_resident_processes=4 ui_processes=3 total_resident_processes=7 resident_clients=2 serving_loop=1 dynamic_lifecycle=1 dynamic_registration_loop=1 multi_session_round=1 general_runtime=0' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_LOOP_OK post_ready_requests=2 distinct_txids=2 client_echo_commits=2 manager_done_rounds=2 provider_done_rounds=2 done_commits=2 transcript_errors=0 reentrant=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_DYNAMIC_OK provider_driven=1 causal_order=1 startup_peer_bound=1 writer_identity=1 register=2 unregister=2 reregister=1 stale_rejected=1 gap_not_found=1 availability=2 phase=10 done_bitmap=0x7 old_instance=0x00020002 new_instance=0x00020003 txids=0x201/0x202/0x203/0x204/0x205/0x206/0x207/0x208 transcript_errors=0 idle_wait_graph=1 reusable=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_REUSE_OK round=1 step=5 completed_rounds=1 errors=0 instance=0x00020004 txids=0x301/0x302/0x303/0x304 done_reads=3 done_bitmap=0x7 register=1 lookup_echo=1 causal_confirm=1 unregister=1 not_found=1 cleanup=1 exact_idle_topology=1 constant_space=1 bounded_reuse=1 general_runtime=0 startup_peer_bound=1 writer_identity=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_ACL_OK manager_pid=0x0000000200000002 provider_pid=0x0000000100000003 client_pid=0x0000000100000004 authenticated_attach=1 owner_from_sender=1 malformed_rejected=4 manager_resilient=1 delegated_endpoint_denied=1 registry_unchanged=1 connector_closed=1 handle_leaks=0 acl_reads=1 errors=0 writer_identity=1' "$NORMALIZED_LOG" \
    && grep -Fxq 'SERVICE_MULTISESSION_OK phase=23 clients=2 resident_clients=2 manager_pid=0x0000000200000002 primary_pid=0x0000000100000004 secondary_pid=0x0000000100000005 manager_attaches=2 client_attaches=2 revoke_bitmap=0x3 stale_revoke_bitmap=0x3 primary_progress_bitmap=0x3 provider_accepts=4 provider_echoes=3 provider_aborts=1 secondary_echoes=1 final_idle_bitmap=0x3 sender_authenticated=1 generation_qualified=1 independent_sessions=2 secondary_lease_generations=2 errors=0 process_crash_restart=0 general_runtime=0' "$NORMALIZED_LOG" \
    && grep -Eq '^SERVICE_WAIT_TOPOLOGY_OK endpoints=20 pairs=10 core_cross_links=1/1/2/2/2/0 ui_pairs=2 unique=1 rights=1 queues_empty=1 manager_items=4 provider_items=2 primary_items=2 secondary_items=2 surface_items=3 launcher_items=1 app_items=1 wait_mask=0x5 infinite_deadlines=1 manager_epoch=[1-9][0-9]* provider_epoch=[1-9][0-9]* primary_epoch=[1-9][0-9]* secondary_epoch=[1-9][0-9]* surface_epoch=[1-9][0-9]* launcher_epoch=[1-9][0-9]* app_epoch=[1-9][0-9]* token_stable=1 supervisor_target=0x0000000200000002$' "$NORMALIZED_LOG" \
    && grep -Eq '^RESIDENT_STABILITY_OK stable_ticks=4 first_manager_heap_restored=1 first_manager_frames_restored=1 frame_delta=1 heap_baseline=[1-9][0-9]* heap_now=[1-9][0-9]* allocated_frames=[1-9][0-9]* init_handles=4 total_handles=23 handles_by_image=4/5/3/4/4/1/2 service_graph=1 all_queues_empty=1 wait_topology=1 object_wait_slots=7 wait_many_slots=2 wait_array_slots=3 supervisor_waits=1 completions_pending=0 contexts=7 kernel_stacks=7 aspace_live=8 live_processes=8 live_images=1/1/1/2/1/1/1$' "$NORMALIZED_LOG" \
    && grep -Eq '^EL0_OK .* pan=[01] user_asid=1 monitor_asid=0$' "$NORMALIZED_LOG" \
    && [[ "$(grep -c '^EL0_STORAGE_OK ' "$NORMALIZED_LOG" || true)" == "1" ]] \
    && grep -Fxq "$EL0_STORAGE_MARKER" "$NORMALIZED_LOG" \
    && grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG" \
    && validate_storage_success_evidence "$NORMALIZED_LOG"; then
    if ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "Kernel did not normalize its boot exception level to EL1." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "on" ]] && ! grep -Fq 'entered AArch64 EL2' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "EL2-entry smoke test did not enter through EL2." >&2
      exit 1
    fi
    if [[ "$VIRTUALIZATION" == "off" ]] && ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG"; then
      cat "$NORMALIZED_LOG"
      echo "EL1-entry smoke test did not enter directly through EL1." >&2
      exit 1
    fi
    frame_line_count="$(grep -c '^FRAME_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$frame_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one FRAME_OK evidence line, found $frame_line_count." >&2
      exit 1
    fi
    frame_line="$(grep '^FRAME_OK ' "$NORMALIZED_LOG")"
    frame_regex='^FRAME_OK managed=([0-9]+) unavailable=([0-9]+) free_before=([0-9]+) free_after=([0-9]+) allocated=([0-9]+) recycled=([0-9]+) unique=([0-9]+)$'
    if [[ ! "$frame_line" =~ $frame_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed FRAME_OK evidence line." >&2
      exit 1
    fi
    if ((${BASH_REMATCH[1]} == 0 || ${BASH_REMATCH[2]} == 0 || ${BASH_REMATCH[3]} == 0 || ${BASH_REMATCH[3]} != ${BASH_REMATCH[4]} || ${BASH_REMATCH[5]} != 0 || ${BASH_REMATCH[6]} != 1 || ${BASH_REMATCH[7]} != 3 || ${BASH_REMATCH[2]} + ${BASH_REMATCH[3]} != ${BASH_REMATCH[1]})); then
      cat "$NORMALIZED_LOG"
      echo "Frame reclaim, ownership, or accounting evidence failed." >&2
      exit 1
    fi

    vm_line_count="$(grep -c '^VM_OK ' "$NORMALIZED_LOG" || true)"
    memory_line_count="$(grep -c '^MEMORY_OK:' "$NORMALIZED_LOG" || true)"
    if [[ "$vm_line_count" != "1" || "$memory_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one VM_OK and MEMORY_OK evidence line." >&2
      exit 1
    fi
    vm_line="$(grep '^VM_OK ' "$NORMALIZED_LOG")"
    vm_regex='^VM_OK maps=([0-9]+) unmaps=([0-9]+) remaps=([0-9]+) tlbi=([0-9]+) table_alloc=([0-9]+) table_free=([0-9]+) alias_checks=([0-9]+) stale=([0-9]+) duplicate_map=([0-9]+) missing_unmap=([0-9]+)$'
    if [[ ! "$vm_line" =~ $vm_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed VM_OK evidence line." >&2
      exit 1
    fi
    if ((${BASH_REMATCH[1]} != 3 || ${BASH_REMATCH[2]} != 3 || ${BASH_REMATCH[3]} != 1 || ${BASH_REMATCH[4]} != 6 || ${BASH_REMATCH[5]} != 1 || ${BASH_REMATCH[6]} != 1 || ${BASH_REMATCH[7]} != 5 || ${BASH_REMATCH[8]} != 0 || ${BASH_REMATCH[9]} != 1 || ${BASH_REMATCH[10]} != 1)); then
      cat "$NORMALIZED_LOG"
      echo "Dynamic page mapping, TLB, or table-reclaim evidence failed." >&2
      exit 1
    fi

    heap_line_count="$(grep -c '^HEAP_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$heap_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one HEAP_OK evidence line, found $heap_line_count." >&2
      exit 1
    fi
    heap_line="$(grep '^HEAP_OK ' "$NORMALIZED_LOG")"
    heap_regex='^HEAP_OK start=(0x[0-9a-f]+) bytes=([0-9]+) backing_pages=([0-9]+) table_frames=([0-9]+) maps=([0-9]+) frame_alloc_before=([0-9]+) frame_alloc_after=([0-9]+) free_before=([0-9]+) free_after=([0-9]+) allocations=([0-9]+) deallocations=([0-9]+) aligned=([0-9]+) whole=([0-9]+) checksum=(0x[0-9a-f]+)$'
    if [[ ! "$heap_line" =~ $heap_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed HEAP_OK evidence line." >&2
      exit 1
    fi
    if (( ${BASH_REMATCH[1]} != 0x0000000102000000 \
      || ${BASH_REMATCH[2]} != 262144 \
      || ${BASH_REMATCH[3]} != 64 \
      || ${BASH_REMATCH[4]} != 1 \
      || ${BASH_REMATCH[5]} != ${BASH_REMATCH[3]} \
      || ${BASH_REMATCH[7]} - ${BASH_REMATCH[6]} != ${BASH_REMATCH[3]} + ${BASH_REMATCH[4]} \
      || ${BASH_REMATCH[8]} != ${BASH_REMATCH[2]} \
      || ${BASH_REMATCH[9]} != ${BASH_REMATCH[8]} \
      || ${BASH_REMATCH[10]} < 4 \
      || ${BASH_REMATCH[11]} != ${BASH_REMATCH[10]} \
      || ${BASH_REMATCH[12]} != 1 \
      || ${BASH_REMATCH[13]} != 1 \
      || ${BASH_REMATCH[14]} != 0x3d5b7904a2c18e5a )); then
      cat "$NORMALIZED_LOG"
      echo "Kernel heap mapping, alignment, checksum, or reclamation evidence failed." >&2
      exit 1
    fi

    heap_irq_line_count="$(grep -c '^HEAP_IRQ_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$heap_irq_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one HEAP_IRQ_OK evidence line, found $heap_irq_line_count." >&2
      exit 1
    fi
    heap_irq_line="$(grep '^HEAP_IRQ_OK ' "$NORMALIZED_LOG")"
    heap_irq_regex='^HEAP_IRQ_OK irq_before=([0-9]+) irq_after=([0-9]+) allocations=([0-9]+) deallocations=([0-9]+) free_before=([0-9]+) free_after=([0-9]+)$'
    if [[ ! "$heap_irq_line" =~ $heap_irq_regex ]] \
      || (( ${BASH_REMATCH[1]} != 0 \
        || ${BASH_REMATCH[2]} != 0 \
        || ${BASH_REMATCH[3]} != 1 \
        || ${BASH_REMATCH[4]} != 1 \
        || ${BASH_REMATCH[5]} != ${BASH_REMATCH[6]} )); then
      cat "$NORMALIZED_LOG"
      echo "IRQ-enabled heap allocation did not preserve DAIF or reclaim memory." >&2
      exit 1
    fi

    sched_line_count="$(grep -c '^SCHED_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$sched_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one SCHED_OK evidence line, found $sched_line_count." >&2
      exit 1
    fi
    sched_line="$(grep '^SCHED_OK ' "$NORMALIZED_LOG")"
    sched_regex='^SCHED_OK dispatches=([0-9]+) switches=([0-9]+) monitor_selected=([0-9]+) t0_selected=([0-9]+) t0_observed=([0-9]+) t0_work=([0-9]+) t1_selected=([0-9]+) t1_observed=([0-9]+) t1_work=([0-9]+) reg_fail=([0-9]+) epoch_fail=([0-9]+) stack_fail=([0-9]+)$'
    if [[ ! "$sched_line" =~ $sched_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed SCHED_OK evidence line." >&2
      exit 1
    fi

    dispatches="${BASH_REMATCH[1]}"
    switches="${BASH_REMATCH[2]}"
    monitor_selected="${BASH_REMATCH[3]}"
    t0_selected="${BASH_REMATCH[4]}"
    t0_observed="${BASH_REMATCH[5]}"
    t0_work="${BASH_REMATCH[6]}"
    t1_selected="${BASH_REMATCH[7]}"
    t1_observed="${BASH_REMATCH[8]}"
    t1_work="${BASH_REMATCH[9]}"
    reg_fail="${BASH_REMATCH[10]}"
    epoch_fail="${BASH_REMATCH[11]}"
    stack_fail="${BASH_REMATCH[12]}"
    selection_delta=$((t0_selected > t1_selected ? t0_selected - t1_selected : t1_selected - t0_selected))

    if ((dispatches < 96 || switches != dispatches)); then
      cat "$NORMALIZED_LOG"
      echo "Scheduler did not prove at least 96 timer-driven busy-context switches." >&2
      exit 1
    fi
    if ((dispatches != monitor_selected + t0_selected + t1_selected)); then
      cat "$NORMALIZED_LOG"
      echo "Scheduler selection accounting is inconsistent." >&2
      exit 1
    fi
    if ((monitor_selected == 0 || monitor_selected != t0_selected || monitor_selected != t1_selected || t0_selected != t0_observed || t1_selected != t1_observed || selection_delta != 0)); then
      cat "$NORMALIZED_LOG"
      echo "Both workers did not observe fair, continuous execution epochs." >&2
      exit 1
    fi
    if ((t0_work == 0 || t1_work == 0 || reg_fail != 0 || epoch_fail != 0 || stack_fail != 0)); then
      cat "$NORMALIZED_LOG"
      echo "Scheduler context or stack integrity evidence failed." >&2
      exit 1
    fi

    sleep_line_count="$(grep -c '^SLEEP_OK ' "$NORMALIZED_LOG" || true)"
    if [[ "$sleep_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one SLEEP_OK evidence line, found $sleep_line_count." >&2
      exit 1
    fi
    sleep_line="$(grep '^SLEEP_OK ' "$NORMALIZED_LOG")"
    sleep_regex='^SLEEP_OK requests=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) returns=([0-9]+) queue_peak=([0-9]+) overlap_ticks=([0-9]+) queue_left=([0-9]+) period=([0-9]+) early=([0-9]+) blocked=([0-9]+) svc_fail=([0-9]+)$'
    if [[ ! "$sleep_line" =~ $sleep_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed SLEEP_OK evidence line." >&2
      exit 1
    fi
    timer_period="${BASH_REMATCH[8]}"
    if ((${BASH_REMATCH[1]} != 2 || ${BASH_REMATCH[2]} != 2 || ${BASH_REMATCH[3]} != 2 || ${BASH_REMATCH[4]} != 2 || ${BASH_REMATCH[5]} != 2 || ${BASH_REMATCH[7]} != 0 || timer_period == 0 || ${BASH_REMATCH[9]} != 0 || ${BASH_REMATCH[10]} != 0 || ${BASH_REMATCH[11]} != 0)); then
      cat "$NORMALIZED_LOG"
      echo "Sleep/wake queue accounting or integrity evidence failed." >&2
      exit 1
    fi

    for expected_id in 0 1; do
      ctx_line_count="$(grep -c "^SLEEP_CTX id=$expected_id " "$NORMALIZED_LOG" || true)"
      if [[ "$ctx_line_count" != "1" ]]; then
        cat "$NORMALIZED_LOG"
        echo "Expected one SLEEP_CTX line for worker $expected_id." >&2
        exit 1
      fi
      ctx_line="$(grep "^SLEEP_CTX id=$expected_id " "$NORMALIZED_LOG")"
      ctx_regex='^SLEEP_CTX id=([01]) request=([0-9]+) block=([0-9]+) duration=([0-9]+) deadline=([0-9]+) ready=([0-9]+) resume=([0-9]+) ctr_request=([0-9]+) ctr_deadline=([0-9]+) ctr_ready=([0-9]+) sel_block=([0-9]+) sel_ready=([0-9]+) obs_block=([0-9]+) obs_ready=([0-9]+) work_block=([0-9]+) work_ready=([0-9]+) work_after=([0-9]+)$'
      if [[ ! "$ctx_line" =~ $ctx_regex ]]; then
        cat "$NORMALIZED_LOG"
        echo "Malformed SLEEP_CTX evidence for worker $expected_id." >&2
        exit 1
      fi
      ctx_id="${BASH_REMATCH[1]}"
      request_tick="${BASH_REMATCH[2]}"
      block_tick="${BASH_REMATCH[3]}"
      duration="${BASH_REMATCH[4]}"
      deadline_tick="${BASH_REMATCH[5]}"
      ready_tick="${BASH_REMATCH[6]}"
      resume_tick="${BASH_REMATCH[7]}"
      counter_request="${BASH_REMATCH[8]}"
      counter_deadline="${BASH_REMATCH[9]}"
      counter_ready="${BASH_REMATCH[10]}"
      sel_block="${BASH_REMATCH[11]}"
      sel_ready="${BASH_REMATCH[12]}"
      obs_block="${BASH_REMATCH[13]}"
      obs_ready="${BASH_REMATCH[14]}"
      work_block="${BASH_REMATCH[15]}"
      work_ready="${BASH_REMATCH[16]}"
      work_after="${BASH_REMATCH[17]}"
      expected_duration=$((expected_id == 0 ? 12 : 6))
      if ((ctx_id != expected_id || duration != expected_duration || request_tick != block_tick || deadline_tick != request_tick + duration || counter_deadline != counter_request + timer_period * duration || counter_ready < counter_deadline || ready_tick < deadline_tick || resume_tick < ready_tick || sel_block != sel_ready || obs_block != obs_ready || work_block != work_ready || work_after <= work_ready)); then
        cat "$NORMALIZED_LOG"
        echo "Worker $expected_id violated its sleep deadline or blocked-execution invariants." >&2
        exit 1
      fi
    done

    user_map_line_count="$(grep -c '^USER_MAP_OK ' "$NORMALIZED_LOG" || true)"
    elf_load_line_count="$(grep -c '^ELF_LOAD_OK ' "$NORMALIZED_LOG" || true)"
    elf_runtime_line_count="$(grep -c '^ELF_RUNTIME_OK ' "$NORMALIZED_LOG" || true)"
    aspace_line_count="$(grep -c '^ASPACE_OK ' "$NORMALIZED_LOG" || true)"
    user_image_catalog_line_count="$(grep -c '^USER_IMAGE_CATALOG_OK ' "$NORMALIZED_LOG" || true)"
    syscall_line_count="$(grep -c '^SYSCALL_OK ' "$NORMALIZED_LOG" || true)"
    copyio_line_count="$(grep -c '^COPYIO_OK ' "$NORMALIZED_LOG" || true)"
    handle_line_count="$(grep -c '^HANDLE_OK ' "$NORMALIZED_LOG" || true)"
    ipc_line_count="$(grep -c '^IPC_OK ' "$NORMALIZED_LOG" || true)"
    ipc_identity_line_count="$(grep -c '^IPC_IDENTITY_OK ' "$NORMALIZED_LOG" || true)"
    handle_transfer_line_count="$(grep -c '^HANDLE_TRANSFER_OK ' "$NORMALIZED_LOG" || true)"
    process_line_count="$(grep -c '^PROC_LIFECYCLE_OK ' "$NORMALIZED_LOG" || true)"
    process_capacity_line_count="$(grep -c '^PROC_CAPACITY_OK ' "$NORMALIZED_LOG" || true)"
    process_image_line_count="$(grep -c '^PROCESS_IMAGE_OK ' "$NORMALIZED_LOG" || true)"
    graphics_buffer_line_count="$(grep -c '^GRAPHICS_BUFFER_OK ' "$NORMALIZED_LOG" || true)"
    service_restart_line_count="$(grep -c '^SERVICE_RESTART_OK ' "$NORMALIZED_LOG" || true)"
    process_wait_line_count="$(grep -c '^PROCESS_WAIT_OK ' "$NORMALIZED_LOG" || true)"
    object_wait_line_count="$(grep -c '^OBJECT_WAIT_OK ' "$NORMALIZED_LOG" || true)"
    wait_many_line_count="$(grep -c '^WAIT_MANY_OK ' "$NORMALIZED_LOG" || true)"
    wait_array_line_count="$(grep -c '^WAIT_ARRAY_OK ' "$NORMALIZED_LOG" || true)"
    event_line_count="$(grep -c '^EVENT_OK ' "$NORMALIZED_LOG" || true)"
    service_manager_line_count="$(grep -c '^SERVICE_MANAGER_OK ' "$NORMALIZED_LOG" || true)"
    service_loop_line_count="$(grep -c '^SERVICE_LOOP_OK ' "$NORMALIZED_LOG" || true)"
    service_dynamic_line_count="$(grep -c '^SERVICE_DYNAMIC_OK ' "$NORMALIZED_LOG" || true)"
    service_reuse_line_count="$(grep -c '^SERVICE_REUSE_OK ' "$NORMALIZED_LOG" || true)"
    service_acl_line_count="$(grep -c '^SERVICE_ACL_OK ' "$NORMALIZED_LOG" || true)"
    service_multisession_line_count="$(grep -c '^SERVICE_MULTISESSION_OK ' "$NORMALIZED_LOG" || true)"
    service_wait_topology_line_count="$(grep -c '^SERVICE_WAIT_TOPOLOGY_OK ' "$NORMALIZED_LOG" || true)"
    resident_stability_line_count="$(grep -c '^RESIDENT_STABILITY_OK ' "$NORMALIZED_LOG" || true)"
    el0_line_count="$(grep -c '^EL0_OK ' "$NORMALIZED_LOG" || true)"
    el0_storage_line_count="$(grep -c '^EL0_STORAGE_OK ' "$NORMALIZED_LOG" || true)"
    boot_ok_line_count="$(grep -c '^BOOT_OK:' "$NORMALIZED_LOG" || true)"
    success_marker_line_count="$(grep -Fxc "$SUCCESS_MARKER" "$NORMALIZED_LOG" || true)"
    if [[ "$user_map_line_count" != "1" || "$elf_load_line_count" != "1" \
      || "$elf_runtime_line_count" != "1" || "$aspace_line_count" != "1" \
      || "$user_image_catalog_line_count" != "1" \
      || "$syscall_line_count" != "1" || "$copyio_line_count" != "1" \
      || "$handle_line_count" != "1" || "$ipc_line_count" != "1" \
      || "$ipc_identity_line_count" != "1" \
      || "$handle_transfer_line_count" != "1" || "$process_line_count" != "1" \
      || "$process_capacity_line_count" != "1" || "$process_image_line_count" != "1" \
      || "$graphics_buffer_line_count" != "1" \
      || "$service_restart_line_count" != "1" \
      || "$process_wait_line_count" != "1" || "$object_wait_line_count" != "1" \
      || "$wait_many_line_count" != "1" || "$wait_array_line_count" != "1" \
      || "$event_line_count" != "1" \
      || "$service_manager_line_count" != "1" || "$service_loop_line_count" != "1" \
      || "$service_dynamic_line_count" != "1" || "$service_reuse_line_count" != "1" \
      || "$service_acl_line_count" != "1" || "$service_multisession_line_count" != "1" \
      || "$service_wait_topology_line_count" != "1" || "$resident_stability_line_count" != "1" \
      || "$el0_line_count" != "1" || "$el0_storage_line_count" != "1" \
      || "$boot_ok_line_count" != "1" \
      || "$success_marker_line_count" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "Expected exactly one EL0 mapping, EL0 storage, ELF load/runtime, address-space, four-image catalog, syscall, copy-I/O, handle, handle-transfer, IPC, authenticated-envelope identity, process-lifecycle, process-capacity, process-image, service-restart, process-wait, object-wait, legacy wait-many, wait-array, Event, ServiceManager, static-loop, dynamic-lifecycle, post-cleanup reuse, service ACL, resilient multi-session, service-wait-topology, resident-stability, and context evidence line." >&2
      exit 1
    fi

    user_map_line="$(grep '^USER_MAP_OK ' "$NORMALIZED_LOG")"
    user_map_regex='^USER_MAP_OK code=(0x[0-9a-f]+) entry=(0x[0-9a-f]+) code_end=(0x[0-9a-f]+) code_bytes=([0-9]+) code_pa=(0x[0-9a-f]+) stack=(0x[0-9a-f]+) stack_end=(0x[0-9a-f]+) stack_pages=([0-9]+) stack_pa=(0x[0-9a-f]+) guard_low=(0x[0-9a-f]+) guard_high=(0x[0-9a-f]+) guards_unmapped=([0-9]+) mapped_pages=([0-9]+) unique_frames=([0-9]+) leafs_verified=([0-9]+)$'
    if [[ ! "$user_map_line" =~ $user_map_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed USER_MAP_OK evidence line." >&2
      exit 1
    fi
    user_code="${BASH_REMATCH[1]}"
    user_entry="${BASH_REMATCH[2]}"
    user_code_end="${BASH_REMATCH[3]}"
    user_code_bytes="${BASH_REMATCH[4]}"
    user_code_pa="${BASH_REMATCH[5]}"
    user_stack="${BASH_REMATCH[6]}"
    user_stack_end="${BASH_REMATCH[7]}"
    user_stack_pages="${BASH_REMATCH[8]}"
    user_stack_pa="${BASH_REMATCH[9]}"
    user_mapped_pages="${BASH_REMATCH[13]}"
    user_unique_frames="${BASH_REMATCH[14]}"
    user_verified_leafs="${BASH_REMATCH[15]}"
    if (( user_code != 0x0000000200000000 \
      || user_entry < user_code || user_entry >= user_code_end || user_entry % 4 != 0 \
      || user_code_bytes == 0 || user_code_bytes > 1048576 \
      || user_code_end != user_code + user_code_bytes \
      || user_stack != 0x00000002001fb000 \
      || user_stack_end != 0x00000002001ff000 \
      || user_code_pa == user_stack_pa \
      || user_code_pa % 4096 != 0 || user_stack_pa % 4096 != 0 \
      || user_stack_pages != 4 \
      || ${BASH_REMATCH[10]} != 0x00000002001fa000 \
      || ${BASH_REMATCH[11]} != 0x00000002001ff000 \
      || ${BASH_REMATCH[12]} != 2 \
      || user_mapped_pages < 7 \
      || user_unique_frames != user_mapped_pages \
      || user_verified_leafs != user_mapped_pages )); then
      cat "$NORMALIZED_LOG"
      echo "EL0 user page placement, guard pages, or permissions evidence failed." >&2
      exit 1
    fi

    elf_load_line="$(grep '^ELF_LOAD_OK ' "$NORMALIZED_LOG")"
    elf_load_regex='^ELF_LOAD_OK elf=([0-9]+) segments=([0-9]+) load_pages=([0-9]+) rx_pages=([0-9]+) ro_pages=([0-9]+) rw_pages=([0-9]+) bss_bytes=([0-9]+) elf_bytes=([0-9]+) file_bytes=([0-9]+) memory_bytes=([0-9]+) wx=([0-9]+)$'
    if [[ ! "$elf_load_line" =~ $elf_load_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed ELF_LOAD_OK evidence line." >&2
      exit 1
    fi
    elf_load_pages="${BASH_REMATCH[3]}"
    elf_rx_pages="${BASH_REMATCH[4]}"
    if (( ${BASH_REMATCH[1]} != 1 \
        || ${BASH_REMATCH[2]} != 3 \
        || elf_load_pages < 3 \
        || elf_rx_pages < 1 \
        || ${BASH_REMATCH[5]} < 1 || ${BASH_REMATCH[6]} != 1 \
        || elf_load_pages != elf_rx_pages + ${BASH_REMATCH[5]} + ${BASH_REMATCH[6]} \
        || ${BASH_REMATCH[7]} == 0 \
        || ${BASH_REMATCH[8]} == 0 || ${BASH_REMATCH[9]} == 0 \
        || ${BASH_REMATCH[10]} != ${BASH_REMATCH[9]} + ${BASH_REMATCH[7]} \
        || ${BASH_REMATCH[11]} != 0 \
        || user_mapped_pages != elf_load_pages + user_stack_pages \
        || user_unique_frames != user_mapped_pages \
        || user_verified_leafs != user_mapped_pages \
        || (user_code_bytes + 4095) / 4096 != elf_rx_pages )); then
      cat "$NORMALIZED_LOG"
      echo "Multi-segment ELF page, permission, BSS, or W^X evidence failed." >&2
      exit 1
    fi

    elf_runtime_line="$(grep '^ELF_RUNTIME_OK ' "$NORMALIZED_LOG")"
    if [[ "$elf_runtime_line" != 'ELF_RUNTIME_OK ro_read=1 data_init=1 data_write=1 bss_zero=1 bss_write=1 stack_pages_touched=4 child_fresh=8 simultaneous=7 distinct_images=7 resident_children=7 client_instances=2 simultaneous_clients=2 ui_clients=2 transient_clients=0' ]]; then
      cat "$NORMALIZED_LOG"
      echo "EL0 ELF runtime data, BSS, stack, distinct-image, or resident-child evidence failed." >&2
      exit 1
    fi

    aspace_line="$(grep '^ASPACE_OK ' "$NORMALIZED_LOG")"
    aspace_regex='^ASPACE_OK kernel_root=(0x[0-9a-f]+) user_root=(0x[0-9a-f]+) asid=([0-9]+) private_tables=([0-9]+) boot_user_unmapped=([0-9]+) heap_shared=([0-9]+) tlbi=([0-9]+)$'
    if [[ ! "$aspace_line" =~ $aspace_regex ]] \
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
      echo "Private TTBR0 root, ASID, shared-kernel mapping, or TLB publication evidence failed." >&2
      exit 1
    fi

    user_image_catalog_line="$(grep '^USER_IMAGE_CATALOG_OK ' "$NORMALIZED_LOG")"
    user_image_catalog_regex='^USER_IMAGE_CATALOG_OK count=([0-9]+) distinct=([0-9]+) init_bytes=([0-9]+) manager_bytes=([0-9]+) provider_bytes=([0-9]+) client_bytes=([0-9]+) surface_bytes=([0-9]+) launcher_bytes=([0-9]+) app_bytes=([0-9]+) init_digest=(0x[0-9a-f]{16}) manager_digest=(0x[0-9a-f]{16}) provider_digest=(0x[0-9a-f]{16}) client_digest=(0x[0-9a-f]{16}) surface_digest=(0x[0-9a-f]{16}) launcher_digest=(0x[0-9a-f]{16}) app_digest=(0x[0-9a-f]{16})$'
    if [[ ! "$user_image_catalog_line" =~ $user_image_catalog_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 7 \
        || ${BASH_REMATCH[2]:-0} != 1 \
        || ${BASH_REMATCH[3]:-0} == 0 \
        || ${BASH_REMATCH[4]:-0} == 0 \
        || ${BASH_REMATCH[5]:-0} == 0 \
        || ${BASH_REMATCH[6]:-0} == 0 \
        || ${BASH_REMATCH[7]:-0} == 0 \
        || ${BASH_REMATCH[8]:-0} == 0 \
        || ${BASH_REMATCH[9]:-0} == 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Seven-image catalog cardinality, nonempty-image, or distinctness evidence failed." >&2
      exit 1
    fi
    image_digests=(
      "${BASH_REMATCH[10]}"
      "${BASH_REMATCH[11]}"
      "${BASH_REMATCH[12]}"
      "${BASH_REMATCH[13]}"
      "${BASH_REMATCH[14]}"
      "${BASH_REMATCH[15]}"
      "${BASH_REMATCH[16]}"
    )
    for ((image_index = 0; image_index < ${#image_digests[@]}; image_index++)); do
      if [[ "${image_digests[$image_index]}" == "0x0000000000000000" ]]; then
        cat "$NORMALIZED_LOG"
        echo "Embedded user image $image_index has a zero digest." >&2
        exit 1
      fi
      for ((other_image_index = image_index + 1; other_image_index < ${#image_digests[@]}; other_image_index++)); do
        if [[ "${image_digests[$image_index]}" == "${image_digests[$other_image_index]}" ]]; then
          cat "$NORMALIZED_LOG"
          echo "Embedded user-image digest collision between catalog slots $image_index and $other_image_index." >&2
          exit 1
        fi
      done
    done

    process_line="$(grep '^PROC_LIFECYCLE_OK ' "$NORMALIZED_LOG")"
    process_regex='^PROC_LIFECYCLE_OK capacity=8 dynamic=7 created=9 exited=1 reaped=1 terminated_exited=1 terminated_faulted=0 terminated_killed=0 live=8 peak_live=8 spawn_waits=1 capacity_rejected=1 startup_moves=8 child_syscalls=1260 child_selections=([0-9]+) manager_v1_pid=0x0000000100000002 provider_pid=0x0000000100000003 client_pid=0x0000000100000004 manager_v2_pid=0x0000000200000002 secondary_pid=0x0000000100000005 surface_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 child_asid=8 root_isolated=1 child_frames_isolated=1 reaped_frame_restored=1 frame_delta=1 reaped_heap_restored=1 nonstandard_exits=1 non_target_reaps=0 distinct_slots=1 distinct_asids=1 distinct_roots=1 distinct_frames=1 retire_tlbi=1 aspace_tlbi=10 child_handle_isolated=1 exit_fail=0 monitor_asid=0$'
    if [[ ! "$process_line" =~ $process_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed PROC_LIFECYCLE_OK evidence line." >&2
      exit 1
    fi
    if (( ${BASH_REMATCH[1]} < 7 )); then
      cat "$NORMALIZED_LOG"
      echo "Resident process capacity, restart identity, isolation, first-manager teardown, or accounting evidence failed." >&2
      exit 1
    fi

    process_capacity_line="$(grep '^PROC_CAPACITY_OK ' "$NORMALIZED_LOG")"
    if [[ "$process_capacity_line" != 'PROC_CAPACITY_OK capacity=8 dynamic=7 peak_live=8 full_rejected=1 startup_preserved=1 distinct_slots=1 distinct_asids=1 distinct_roots=1 distinct_frames=1 frame_delta=1 manager_pid=0x0000000100000002 provider_pid=0x0000000100000003 client_pid=0x0000000100000004 surface_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Exact simultaneous manager/provider/client capacity evidence failed." >&2
      exit 1
    fi

    process_image_line="$(grep '^PROCESS_IMAGE_OK ' "$NORMALIZED_LOG")"
    if [[ "$process_image_line" != 'PROCESS_IMAGE_OK abi=23 init=1 spawn_sequence=2/3/4/2/4/5/6/7 manager_reused=1 client_image_reused=1 distinct_catalog=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "ABI-v23 startup image selection did not prove the seven distinct runtime roles and restarted-manager identity." >&2
      exit 1
    fi

    ui_runtime_line="$(grep '^UI_RUNTIME_OK ' "$NORMALIZED_LOG")"
    if [[ "$ui_runtime_line" != 'UI_RUNTIME_OK protocol=3 server_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 distinct=1 ui_pairs=2 surface_owner=server launcher_surface=0 app_surface=0 server_handles=4 launcher_handles=1 app_handles=2 dual_clients=1 focus_routed=1 ready_before_present=1 single_outstanding=1 first_present_ack=1 graphics_buffer=1 resident=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "The isolated SurfaceServer/Launcher/App channels, focus routing, capability boundary, or first present acknowledgement is incomplete." >&2
      exit 1
    fi

    graphics_buffer_line="$(grep '^GRAPHICS_BUFFER_OK ' "$NORMALIZED_LOG")"
    if [[ "$graphics_buffer_line" != "$GRAPHICS_BUFFER_MARKER" ]]; then
      cat "$NORMALIZED_LOG"
      echo "The generation-qualified App/SurfaceServer graphics-buffer ownership, transfer, rights, or bounded copy ledger is incomplete." >&2
      exit 1
    fi

    service_restart_line="$(grep '^SERVICE_RESTART_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_restart_line" != 'SERVICE_RESTART_OK requests=1 restarts=1 sessions_rebound=2 dependents_survived=2 peer_close_acks=4 slot_reused=1 generation_advanced=1 stale_pid_rejected=1 stale_instance_rejected=1 old_pid=0x0000000100000002 new_pid=0x0000000200000002 old_instance=0x00010001 new_instance=0x00020001' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Exact supervised ServiceManager restart, rebind, and ABA-resistance evidence failed." >&2
      exit 1
    fi

    process_wait_line="$(grep '^PROCESS_WAIT_OK ' "$NORMALIZED_LOG")"
    process_wait_regex='^PROCESS_WAIT_OK calls=([0-9]+) completed=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) immediate=([0-9]+) stale=([0-9]+) pending=([0-9]+) supervisor_target=(0x[0-9a-f]+) spawn_retries=([0-9]+) ready_retries=([0-9]+)$'
    if [[ ! "$process_wait_line" =~ $process_wait_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 3 \
        || ${BASH_REMATCH[2]:-0} != 1 \
        || ${BASH_REMATCH[3]:-0} != ${BASH_REMATCH[4]:-0} + 1 \
        || ${BASH_REMATCH[4]:-0} + ${BASH_REMATCH[5]:-0} != 1 \
        || ${BASH_REMATCH[6]:-0} != 1 \
        || ${BASH_REMATCH[7]:-0} != 1 \
        || ${BASH_REMATCH[8]:-0} != 0x0000000200000002 \
        || ${BASH_REMATCH[9]:-0} != 1 \
        || ${BASH_REMATCH[10]:-0} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Generation-safe targeted process-wait, resident supervisor block, capacity retry, or tombstone evidence failed." >&2
      exit 1
    fi

    object_wait_line="$(grep '^OBJECT_WAIT_OK ' "$NORMALIZED_LOG")"
    object_wait_regex='^OBJECT_WAIT_OK calls=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) immediate=([0-9]+) cancels=([0-9]+) abandoned=([0-9]+) pending=([0-9]+) latest_epoch=([0-9]+) protocol_retries=([0-9]+)$'
    if [[ ! "$object_wait_line" =~ $object_wait_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed OBJECT_WAIT_OK evidence line." >&2
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
      echo "Malformed WAIT_MANY_OK evidence line." >&2
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
      echo "Malformed WAIT_ARRAY_OK evidence line." >&2
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
      || object_wait_blocks + wait_many_pending + wait_array_pending != object_wait_wakes + object_wait_pending + object_wait_abandoned + wait_many_abandoned + wait_array_abandoned \
      || object_wait_blocks != object_wait_wakes + object_wait_abandoned + 2 \
      || object_wait_blocks + object_wait_immediate != object_wait_calls \
      || wait_many_pending + wait_array_pending + 2 != object_wait_pending \
      || object_wait_blocks < wait_many_blocks + wait_array_blocks \
      || object_wait_wakes < wait_many_wakes + wait_array_wakes \
      || object_wait_immediate < wait_many_immediate + wait_array_immediate + 6 \
      || object_wait_cancels != 0 \
      || object_wait_abandoned != 0 \
      || object_wait_pending != 7 \
      || object_wait_latest_epoch < 1 \
      || object_wait_latest_epoch > total_wait_blocks \
      || object_wait_protocol_retries != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Blocking object-wait wakeup, seven resident wait slots, stale-connector nonblocking probe, immediate completion, or epoch evidence failed." >&2
      exit 1
    fi
    if (( wait_many_items != 2 \
      || wait_many_calls != 37 \
      || wait_many_immediate < 5 \
      || wait_many_immediate > 25 \
      || wait_many_blocks < 7 \
      || wait_many_blocks > 27 \
      || wait_many_blocks != wait_many_wakes + wait_many_pending + wait_many_abandoned \
      || wait_many_signal_wakes > 20 \
      || wait_many_finite_signal_wakes > 4 \
      || wait_many_finite_signal_wakes > wait_many_signal_wakes \
      || wait_many_poll_timeouts != 5 \
      || wait_many_timeout_wakes != 5 \
      || wait_many_cancels != 0 \
      || wait_many_abandoned != 0 \
      || wait_many_early != 0 \
      || wait_many_pending != 2 \
      || wait_many_latest_epoch < 1 \
      || wait_many_latest_epoch > total_wait_blocks \
      || wait_many_calls != wait_many_immediate + wait_many_poll_timeouts + wait_many_blocks \
      || wait_many_immediate + wait_many_signal_wakes != 25 \
      || wait_many_wakes != wait_many_signal_wakes + wait_many_timeout_wakes + wait_many_cancels )); then
      cat "$NORMALIZED_LOG"
      echo "Bounded wait-any signal, poll-timeout, blocking-timeout, epoch, or cleanup evidence failed." >&2
      exit 1
    fi
    if (( wait_array_max_items != 8 \
      || wait_array_item_bytes != 8 \
      || wait_array_calls != 57 \
      || wait_array_immediate < 1 \
      || wait_array_immediate > 52 \
      || wait_array_blocks < 2 \
      || wait_array_blocks > 53 \
      || wait_array_blocks != wait_array_wakes + wait_array_pending + wait_array_abandoned \
      || wait_array_signal_wakes > 50 \
      || wait_array_poll_timeouts != 1 \
      || wait_array_timeout_wakes != 0 \
      || wait_array_invalid_counts != 2 \
      || wait_array_bad_address != 2 \
      || wait_array_validation_rejected != 1 \
      || wait_array_lowest_multi_ready != 1 \
      || wait_array_scheduler_max_items != 8 \
      || wait_array_cancels != 0 \
      || wait_array_abandoned != 0 \
      || wait_array_early != 0 \
      || wait_array_pending != 3 \
      || wait_array_latest_epoch < 1 \
      || wait_array_latest_epoch > total_wait_blocks \
      || wait_array_calls != wait_array_immediate + wait_array_poll_timeouts + wait_array_blocks \
      || wait_array_wakes != wait_array_signal_wakes + wait_array_timeout_wakes + wait_array_cancels \
      || wait_array_immediate + wait_array_signal_wakes != 53 \
      || wait_array_canonical_le != 1 \
      || wait_array_full_validation != 1 \
      || wait_array_generation_qualified != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Bounded eight-item wait-array validation, lowest-ready selection, PAN usercopy recovery, blocking wake, or scheduler accounting evidence failed." >&2
      exit 1
    fi

    event_line="$(grep '^EVENT_OK ' "$NORMALIZED_LOG")"
    event_regex='^EVENT_OK created=([0-9]+) signal_calls=([0-9]+) signal_edges=([0-9]+) clear_calls=([0-9]+) clear_edges=([0-9]+) signal_denied=([0-9]+) waits=([0-9]+) blocks=([0-9]+) wakes=([0-9]+) transfer_writes=([0-9]+) transfer_reads=([0-9]+) init_handles=([0-9]+)$'
    if [[ ! "$event_line" =~ $event_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 6 \
        || ${BASH_REMATCH[2]:-0} != 16 || ${BASH_REMATCH[3]:-0} != 11 \
        || ${BASH_REMATCH[4]:-0} != 16 || ${BASH_REMATCH[5]:-0} != 11 \
        || ${BASH_REMATCH[6]:-0} != 5 || ${BASH_REMATCH[7]:-0} != 10 \
        || ${BASH_REMATCH[8]:-0} != ${BASH_REMATCH[9]:-0} \
        || ${BASH_REMATCH[8]:-0} > ${BASH_REMATCH[7]:-0} \
        || ${BASH_REMATCH[10]:-0} != 5 || ${BASH_REMATCH[11]:-0} != 5 \
        || ${BASH_REMATCH[12]:-0} != 4 )); then
      cat "$NORMALIZED_LOG"
      echo "Transferable Event edge, blocking wakeup, rollback, or cleanup evidence failed." >&2
      exit 1
    fi

    service_manager_line="$(grep '^SERVICE_MANAGER_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_manager_line" != 'SERVICE_MANAGER_OK protocol=1 supervisor=1 daemon=1 supervised=1 bootstrap_identity=1 registry_capacity=4 registry_state_reachable=1 resident_roles=3 core_resident_processes=4 ui_processes=3 total_resident_processes=7 resident_clients=2 serving_loop=1 dynamic_lifecycle=1 dynamic_registration_loop=1 multi_session_round=1 general_runtime=0' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Reachable four-process ServiceManager state, resident clients, lifecycle, or bounded-runtime evidence failed." >&2
      exit 1
    fi

    service_dynamic_line="$(grep '^SERVICE_DYNAMIC_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_dynamic_line" != 'SERVICE_DYNAMIC_OK provider_driven=1 causal_order=1 startup_peer_bound=1 writer_identity=1 register=2 unregister=2 reregister=1 stale_rejected=1 gap_not_found=1 availability=2 phase=10 done_bitmap=0x7 old_instance=0x00020002 new_instance=0x00020003 txids=0x201/0x202/0x203/0x204/0x205/0x206/0x207/0x208 transcript_errors=0 idle_wait_graph=1 reusable=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "The authenticated provider-driven register/unregister/gap/re-register/stale-reject/cleanup transcript did not remain reusable." >&2
      exit 1
    fi

    service_reuse_line="$(grep '^SERVICE_REUSE_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_reuse_line" != 'SERVICE_REUSE_OK round=1 step=5 completed_rounds=1 errors=0 instance=0x00020004 txids=0x301/0x302/0x303/0x304 done_reads=3 done_bitmap=0x7 register=1 lookup_echo=1 causal_confirm=1 unregister=1 not_found=1 cleanup=1 exact_idle_topology=1 constant_space=1 bounded_reuse=1 general_runtime=0 startup_peer_bound=1 writer_identity=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "The authenticated post-cleanup register/lookup/direct-echo/confirm/unregister/not-found transcript or its truthful bounded-runtime boundary is incomplete." >&2
      exit 1
    fi

    service_acl_line="$(grep '^SERVICE_ACL_OK ' "$NORMALIZED_LOG")"
    service_acl_regex='^SERVICE_ACL_OK manager_pid=(0x[0-9a-f]+) provider_pid=(0x[0-9a-f]+) client_pid=(0x[0-9a-f]+) authenticated_attach=([0-9]+) owner_from_sender=([0-9]+) malformed_rejected=([0-9]+) manager_resilient=([0-9]+) delegated_endpoint_denied=([0-9]+) registry_unchanged=([0-9]+) connector_closed=([0-9]+) handle_leaks=([0-9]+) acl_reads=([0-9]+) errors=([0-9]+) writer_identity=([0-9]+)$'
    if [[ ! "$service_acl_line" =~ $service_acl_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 0x0000000200000002 \
        || ${BASH_REMATCH[2]:-0} != 0x0000000100000003 \
        || ${BASH_REMATCH[3]:-0} != 0x0000000100000004 \
        || ${BASH_REMATCH[4]:-0} != 1 || ${BASH_REMATCH[5]:-0} != 1 \
        || ${BASH_REMATCH[6]:-0} != 4 || ${BASH_REMATCH[7]:-0} != 1 \
        || ${BASH_REMATCH[8]:-0} != 1 || ${BASH_REMATCH[9]:-0} != 1 \
        || ${BASH_REMATCH[10]:-0} != 1 || ${BASH_REMATCH[11]:-1} != 0 \
        || ${BASH_REMATCH[12]:-0} != 1 || ${BASH_REMATCH[13]:-1} != 0 \
        || ${BASH_REMATCH[14]:-0} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Generation-qualified ServiceManager attachment, sender-owned registry, delegated-endpoint denial, rollback, or cleanup evidence failed." >&2
      exit 1
    fi

    service_multisession_line="$(grep '^SERVICE_MULTISESSION_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_multisession_line" != 'SERVICE_MULTISESSION_OK phase=23 clients=2 resident_clients=2 manager_pid=0x0000000200000002 primary_pid=0x0000000100000004 secondary_pid=0x0000000100000005 manager_attaches=2 client_attaches=2 revoke_bitmap=0x3 stale_revoke_bitmap=0x3 primary_progress_bitmap=0x3 provider_accepts=4 provider_echoes=3 provider_aborts=1 secondary_echoes=1 final_idle_bitmap=0x3 sender_authenticated=1 generation_qualified=1 independent_sessions=2 secondary_lease_generations=2 errors=0 process_crash_restart=0 general_runtime=0' ]]; then
      cat "$NORMALIZED_LOG"
      echo "The bounded two-client attach/revoke/reattach, stall isolation, lease rejection, provider abort, or final-idle transcript is incomplete." >&2
      exit 1
    fi

    service_loop_line="$(grep '^SERVICE_LOOP_OK ' "$NORMALIZED_LOG")"
    if [[ "$service_loop_line" != 'SERVICE_LOOP_OK post_ready_requests=2 distinct_txids=2 client_echo_commits=2 manager_done_rounds=2 provider_done_rounds=2 done_commits=2 transcript_errors=0 reentrant=1' ]]; then
      cat "$NORMALIZED_LOG"
      echo "The post-ready transcript did not prove two committed, distinct lookup/connect/direct-echo rounds and both daemon completions." >&2
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
      echo "The exact ten-pair core/UI channel graph, seven role-specific pending waits, stable wait tokens, or supervisor target is inconsistent." >&2
      exit 1
    fi

    resident_stability_line="$(grep '^RESIDENT_STABILITY_OK ' "$NORMALIZED_LOG")"
    resident_stability_regex='^RESIDENT_STABILITY_OK stable_ticks=4 first_manager_heap_restored=1 first_manager_frames_restored=1 frame_delta=1 heap_baseline=([0-9]+) heap_now=([0-9]+) allocated_frames=([0-9]+) init_handles=4 total_handles=23 handles_by_image=4/5/3/4/4/1/2 service_graph=1 all_queues_empty=1 wait_topology=1 object_wait_slots=7 wait_many_slots=2 wait_array_slots=3 supervisor_waits=1 completions_pending=0 contexts=7 kernel_stacks=7 aspace_live=8 live_processes=8 live_images=1/1/1/2/1/1/1$'
    if [[ ! "$resident_stability_line" =~ $resident_stability_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} <= ${BASH_REMATCH[2]:-0} \
        || ${BASH_REMATCH[2]:-0} == 0 \
        || ${BASH_REMATCH[3]:-0} == 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Stable resident heap, frame, handle, wait, stack, address-space, process, or image-ownership evidence failed." >&2
      exit 1
    fi

    syscall_line="$(grep '^SYSCALL_OK ' "$NORMALIZED_LOG")"
    syscall_regex='^SYSCALL_OK abi=([0-9]+) calls=([0-9]+) successes=([0-9]+) errors=([0-9]+) unknown=([0-9]+) private_svc=([0-9]+) should_waits=([0-9]+) pan_fail=([0-9]+)$'
    if [[ ! "$syscall_line" =~ $syscall_regex ]]; then
      cat "$NORMALIZED_LOG"
      echo "Malformed SYSCALL_OK evidence line." >&2
      exit 1
    fi
    if (( ${BASH_REMATCH[1]} != 23 \
      || ${BASH_REMATCH[2]} != ${BASH_REMATCH[3]} + ${BASH_REMATCH[4]} + 1 \
      || ${BASH_REMATCH[2]} != 599 \
      || ${BASH_REMATCH[3]} != 462 \
      || ${BASH_REMATCH[4]} != 136 \
      || ${BASH_REMATCH[5]} != 1 || ${BASH_REMATCH[6]} != 1 \
      || ${BASH_REMATCH[7]} != 3 || ${BASH_REMATCH[8]} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "ABI-v23 syscall dispatch, graphics-buffer transfer, dual UI channels, bounded wait-array, authenticated ChannelReadEnvelope, resident supervisor block, Event, private-SVC isolation, or accounting evidence failed." >&2
      exit 1
    fi

    copyio_line="$(grep '^COPYIO_OK ' "$NORMALIZED_LOG")"
    copyio_regex='^COPYIO_OK max=([0-9]+) in_calls=([0-9]+) out_calls=([0-9]+) in_bytes=([0-9]+) out_bytes=([0-9]+) in_faults=([0-9]+) out_faults=([0-9]+) range_rejects=([0-9]+) fixups=([0-9]+) misses=([0-9]+) in_ec=([0-9]+) in_dfsc=([0-9]+) in_wnr=([0-9]+) in_far=(0x[0-9a-f]+) out_ec=([0-9]+) out_dfsc=([0-9]+) out_wnr=([0-9]+) out_far=(0x[0-9a-f]+) writes=([0-9]+) reads=([0-9]+) rollbacks=([0-9]+) too_small=([0-9]+) zero_len=([0-9]+) roundtrip=([0-9]+) pan_fail=([0-9]+) uao_supported=([01]) uao_fail=([0-9]+)$'
    if [[ ! "$copyio_line" =~ $copyio_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 64 \
        || ${BASH_REMATCH[2]:-0} != 198 || ${BASH_REMATCH[3]:-0} != 198 \
        || ${BASH_REMATCH[4]:-0} != 7835 || ${BASH_REMATCH[5]:-0} != 17487 \
        || ${BASH_REMATCH[6]:-0} != 14 || ${BASH_REMATCH[7]:-0} != 11 \
        || ${BASH_REMATCH[8]:-0} != 1 || ${BASH_REMATCH[9]:-0} != 25 \
        || ${BASH_REMATCH[10]:-1} != 0 \
        || ${BASH_REMATCH[11]:-0} != 37 || ${BASH_REMATCH[12]:-0} != 7 \
        || ${BASH_REMATCH[13]:-1} != 0 \
        || ${BASH_REMATCH[14]:-0} != 0x00000002001fa000 \
        || ${BASH_REMATCH[15]:-0} != 37 || ${BASH_REMATCH[16]:-0} != 7 \
        || ${BASH_REMATCH[17]:-0} != 1 \
        || ${BASH_REMATCH[18]:-0} != 0x00000002001ff000 \
        || ${BASH_REMATCH[19]:-0} != 54 || ${BASH_REMATCH[20]:-0} != 54 \
        || ${BASH_REMATCH[21]:-0} != 2 || ${BASH_REMATCH[22]:-0} != 1 \
        || ${BASH_REMATCH[23]:-0} != 2 || ${BASH_REMATCH[24]:-0} != 1 \
        || ${BASH_REMATCH[25]:-1} != 0 || ${BASH_REMATCH[27]:-1} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "Exception-table copy-I/O recovery, transactional byte IPC, PAN, or UAO evidence failed." >&2
      exit 1
    fi
    uao_supported="${BASH_REMATCH[26]}"
    if [[ "$QEMU_CPU" == "max" && "$uao_supported" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "The max CPU smoke test did not exercise FEAT_UAO." >&2
      exit 1
    fi

    handle_line="$(grep '^HANDLE_OK ' "$NORMALIZED_LOG")"
    handle_regex='^HANDLE_OK channel_pairs_created=([0-9]+) events_created=([0-9]+) duplicated=([0-9]+) close_syscalls=([0-9]+) stale_close_rejected=([0-9]+) instrumented_rights_denials=([0-9]+) init_handles=([0-9]+)$'
    if [[ ! "$handle_line" =~ $handle_regex ]] \
      || (( ${BASH_REMATCH[1]} != 48 || ${BASH_REMATCH[2]} != 6 \
        || ${BASH_REMATCH[3]} != 190 || ${BASH_REMATCH[4]} != 269 \
        || ${BASH_REMATCH[5]} != 74 || ${BASH_REMATCH[6]} != 18 \
        || ${BASH_REMATCH[7]} != 4 )); then
      cat "$NORMALIZED_LOG"
      echo "Instrumented channel/Event creation, capability duplication, close, stale-value, rights, or resident-init evidence failed." >&2
      exit 1
    fi

    ipc_line="$(grep '^IPC_OK ' "$NORMALIZED_LOG")"
    ipc_regex='^IPC_OK channels=([0-9]+) writes=([0-9]+) reads=([0-9]+) peeks=([0-9]+) peek_kinds=([0-9]+)/([0-9]+)/([0-9]+) tag=([0-9]+) payload=(0x[0-9a-f]+) startup_queues_drained=([0-9]+) resident_heap=([0-9]+)$'
    if [[ ! "$ipc_line" =~ $ipc_regex ]] \
      || (( ${BASH_REMATCH[1]} != 48 || ${BASH_REMATCH[2]} != 142 \
        || ${BASH_REMATCH[3]} != 142 || ${BASH_REMATCH[4]} != 0 \
        || ${BASH_REMATCH[5]} != 0 || ${BASH_REMATCH[6]} != 0 \
        || ${BASH_REMATCH[7]} != 0 || ${BASH_REMATCH[8]} != 4660 \
        || ${BASH_REMATCH[9]} != 0x0123456789abcdef \
        || ${BASH_REMATCH[10]} != 1 || ${BASH_REMATCH[11]} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Bounded IPC FIFO, payload, drained transient queues, or resident-heap evidence failed." >&2
      exit 1
    fi

    ipc_identity_line="$(grep '^IPC_IDENTITY_OK ' "$NORMALIZED_LOG")"
    ipc_identity_regex='^IPC_IDENTITY_OK abi=([0-9]+) atomic_envelope=([0-9]+) envelope_size=([0-9]+) reads=([0-9]+) kinds=([0-9]+)/([0-9]+)/([0-9]+) preserved_rejections=([0-9]+) too_small=([0-9]+) bad_address=([0-9]+) table_full=([0-9]+) identity_reads=([0-9]+) errors=([0-9]+) provider_pid=(0x[0-9a-f]+) client_pid=(0x[0-9a-f]+) generation_qualified=([0-9]+) writer_identity=([0-9]+)$'
    if [[ ! "$ipc_identity_line" =~ $ipc_identity_regex ]] \
      || (( ${BASH_REMATCH[1]:-0} != 23 \
        || ${BASH_REMATCH[2]:-0} != 1 || ${BASH_REMATCH[3]:-0} != 112 \
        || ${BASH_REMATCH[4]:-0} != 135 \
        || ${BASH_REMATCH[4]:-0} != ${BASH_REMATCH[5]:-0} + ${BASH_REMATCH[6]:-0} + ${BASH_REMATCH[7]:-0} \
        || ${BASH_REMATCH[5]:-0} != 65 || ${BASH_REMATCH[6]:-0} != 34 \
        || ${BASH_REMATCH[7]:-0} != 36 || ${BASH_REMATCH[8]:-0} != 15 \
        || ${BASH_REMATCH[8]:-0} != ${BASH_REMATCH[9]:-0} + ${BASH_REMATCH[10]:-0} + ${BASH_REMATCH[11]:-0} \
        || ${BASH_REMATCH[9]:-0} != 5 || ${BASH_REMATCH[10]:-0} != 5 \
        || ${BASH_REMATCH[11]:-0} != 5 || ${BASH_REMATCH[12]:-0} != 2 \
        || ${BASH_REMATCH[13]:-1} != 0 \
        || ${BASH_REMATCH[14]:-0} != 0x0000000100000003 \
        || ${BASH_REMATCH[15]:-0} != 0x0000000100000004 \
        || ${BASH_REMATCH[16]:-0} != 1 || ${BASH_REMATCH[17]:-0} != 1 )); then
      cat "$NORMALIZED_LOG"
      echo "Atomic envelope kind accounting, non-consuming rejection rollback, generation-qualified sender, or writer-identity evidence failed." >&2
      exit 1
    fi

    handle_transfer_line="$(grep '^HANDLE_TRANSFER_OK ' "$NORMALIZED_LOG")"
    if [[ "$handle_transfer_line" != 'HANDLE_TRANSFER_OK startup_moves=8 channel_writes=65 channel_reads=64 read_rollbacks=30 self_rejected=5 order_rejected=5 write_badaddr_rollback=8 denied_rollback=4 rights_preserved=8 table_full_rollback=8 unread_dropped=1 init_handles=4' ]]; then
      cat "$NORMALIZED_LOG"
      echo "Cross-process handle transfer, rollback, rights, or cleanup evidence failed." >&2
      exit 1
    fi

    el0_line="$(grep '^EL0_OK ' "$NORMALIZED_LOG")"
    el0_regex='^EL0_OK selections=([0-9]+) lower_irq=([0-9]+) post_irq_svc=([0-9]+) preserved=([0-9]+) stack_rw=([0-9]+) faults=([0-9]+) sync_fault_dispatches=([0-9]+) mode=([0-9]+) pan=([01]) user_asid=([0-9]+) monitor_asid=([0-9]+)$'
    if [[ ! "$el0_line" =~ $el0_regex ]] \
      || (( ${BASH_REMATCH[1]} < 2 || ${BASH_REMATCH[2]} < 1 \
        || ${BASH_REMATCH[3]} != 1 || ${BASH_REMATCH[4]} != 1 \
        || ${BASH_REMATCH[5]} != 1 || ${BASH_REMATCH[6]} != 0 \
        || ${BASH_REMATCH[7]} != 0 || ${BASH_REMATCH[8]} != 0 \
        || ${BASH_REMATCH[10]} != 1 || ${BASH_REMATCH[11]} != 0 )); then
      cat "$NORMALIZED_LOG"
      echo "EL0 return mode, timer preemption, register, or stack evidence failed." >&2
      exit 1
    fi
    pan_enabled="${BASH_REMATCH[9]}"
    if [[ "$QEMU_CPU" == "max" && "$pan_enabled" != "1" ]]; then
      cat "$NORMALIZED_LOG"
      echo "The max CPU smoke test did not exercise FEAT_PAN." >&2
      exit 1
    fi

    cat "$NORMALIZED_LOG"
    echo "QEMU kernel smoke test passed: M32 transferable graphics buffers, M29 surface handoff, M25 durable data records, heap, $switches switches, ABI-v23 dual-UI/wait-array IPC, and M20 resilient two-client sessions verified."
    exit 0
  fi

  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    qemu_status=$?
    set -e
    QEMU_PID=""
    tr -d '\r' <"$LOG_FILE"
    echo "QEMU exited before emitting '$SUCCESS_MARKER' (status $qemu_status)." >&2
    if ((qemu_status == 0)); then
      exit 1
    fi
    exit "$qemu_status"
  fi

  sleep 0.1
done

tr -d '\r' <"$LOG_FILE"
echo "Timed out after ${BOOT_TIMEOUT_SECONDS}s waiting for '$SUCCESS_MARKER'." >&2
exit 1
