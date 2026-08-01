#!/usr/bin/env bash

BNDROID_STORAGE_IMAGE_BYTES=8388608
BNDROID_STORAGE_SECTOR_BYTES=512
BNDROID_STORAGE_SECTOR_COUNT=16384
BNDROID_STORAGE_SECTOR0_FNV1A64=0xbebd264b8c14cd72
BNDROID_STORAGE_SECTOR1_FNV1A64=0x8294de399174037c

# Populates BNDROID_STORAGE_QEMU_ARGS with one virtio-mmio block device. Normal
# M25 boots use a writable temporary overlay so the canonical fixture remains
# immutable; the dedicated reboot test opts into a persistent writable copy.
# The M22 timeout/reset path deliberately retains a read-only mode.
build_storage_qemu_args() {
  local image_path="$1"
  local transport="${2:-modern}"
  local access="${3:-writable-temporary}"
  local force_legacy
  local drive_access
  local write_cache="on"

  if [[ ! -f "$image_path" ]]; then
    echo "Storage image does not exist: $image_path" >&2
    return 1
  fi
  if [[ "$image_path" == *','* || "$image_path" == *$'\n'* ]]; then
    echo "Storage image paths containing commas or newlines are unsupported by the QEMU drive argument." >&2
    return 1
  fi
  case "$transport" in
    modern) force_legacy="false" ;;
    legacy) force_legacy="true" ;;
    none)
      BNDROID_STORAGE_QEMU_ARGS=(
        -global "virtio-mmio.force-legacy=false"
      )
      return 0
      ;;
    *)
      echo "Unknown virtio-mmio transport '$transport'; expected modern, legacy, or none." >&2
      return 2
      ;;
  esac

  case "$access" in
    writable-temporary)
      drive_access="readonly=off,snapshot=on,cache=writeback"
      ;;
    writable-persistent)
      drive_access="readonly=off,snapshot=off,cache=writeback"
      ;;
    writable-no-flush)
      drive_access="readonly=off,snapshot=on,cache=writeback"
      write_cache="off"
      ;;
    readonly)
      drive_access="readonly=on"
      ;;
    *)
      echo "Unknown storage access '$access'; expected writable-temporary, writable-persistent, writable-no-flush, or readonly." >&2
      return 2
      ;;
  esac

  BNDROID_STORAGE_QEMU_ARGS=(
    -global "virtio-mmio.force-legacy=$force_legacy"
    -drive "if=none,file=$image_path,format=raw,$drive_access,id=bndroid-storage"
    -device "virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=$write_cache,discard=off,write-zeroes=off"
  )
}

validate_storage_image_geometry() {
  local image_path="$1"
  local actual_bytes

  [[ -f "$image_path" ]] || return 1
  actual_bytes="$(wc -c <"$image_path" | tr -d '[:space:]')"
  [[ "$actual_bytes" == "$BNDROID_STORAGE_IMAGE_BYTES" ]]
}

validate_storage_marker_once() {
  local log_file="$1"
  local marker="$2"
  [[ "$(grep -Fxc "$marker" "$log_file" || true)" == "1" ]]
}

# Stable M25 storage/catalog/persistence success contract, including exact field ordering.
validate_storage_success_evidence() {
  local log_file="$1"
  local fdt_marker="FDT_VIRTIO_OK nodes=32 coherent=32 irq_specifiers=32 active_blocks=1"
  local irq_marker="VIRTIO_IRQ_OK controller=gicv2 spec=0/47/1 irq=79 trigger=edge-rising target_cpu=0 enabled=1"
  local transport_marker="VIRTIO_BLK_OK transport=mmio version=2 queue=8 request_slots=2 request_stride=536 dma_frames=2 dma_coherent=1 mode=interrupt irq_enabled=1 irq=79 capacity_sectors=16384 device_read_only=0 flush_supported=1 system_policy=read_only data_policy=bounded_write"
  local read_marker="BLOCK_IRQ_OK requests=2 completions=2 irq_completions=2 sectors=0/1 bytes=1024 avail_idx=2 used_idx=2 statuses=0/0 digest0=$BNDROID_STORAGE_SECTOR0_FNV1A64 digest1=$BNDROID_STORAGE_SECTOR1_FNV1A64 batch_width=2 max_outstanding=2 distinct_heads=1 irq_observed=1 poll_fallbacks=0 out_of_range_rejected=1 timeouts=0 resets=0 stale_completions=0 queue_reused=1"
  local block_layer_marker="BLOCK_LAYER_OK sector_size=512 device_sectors=16384 parser_reads=273 requests=282 completions=282 read_requests=280 write_requests=1 flush_requests=1 bytes_read=143360 bytes_written=512 successful_flushes=1 irq_completions=282 poll_fallbacks=0 timeouts=0 dma_frames=2"
  local gpt_marker="GPT_OK protective_mbr=1 primary_crc=1 backup_crc=1 entry_crc=1 entries=128 entry_bytes=128 primary_entries_lba=2 backup_entries_lba=16351 partition_index=0 partition_lba=2048-16350 partition_sectors=14303 name=BNDROID_SYS"
  local data_gpt_marker="DATA_GPT_OK partition_index=1 partition_lba=64-127 partition_sectors=64 name=BNDROID_DATA type=private system_overlap=0"
  local fat_marker="FAT16_OK bytes_per_sector=512 sectors_per_cluster=1 reserved_sectors=1 fats=2 fat_sectors=56 root_entries=64 clusters=14186 first_data_lba=2165 mirror_verified=1"
  local vfs_marker="VFS_OK mount=/system root_file=/system/HELLO.TXT root_bytes=28 root_digest=0xdd2f71342016eede nested_file=/system/SYSTEM/BUILD.TXT nested_bytes=40 nested_digest=0xe57ce4ce9f1b4ec0 read_only=1 traversal_rejected=1 mount_escape_rejected=1"
  local persist_marker="DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 slots=2 initial_generation=0 committed_generation=1 initial_slot=0 committed_slot=1 valid_slots=1 rejected_slots=1 reads=5 writes=1 flushes=1 write_completion=1 flush_completion=1 readback_verified=1 old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 crash_consistency=0 qemu_reboot_proof=0"
  local limits_marker="STORAGE_LIMITS writes=1 partitions=2 filesystem=1 vfs=1 persistence=1 flush=1 readback=1 el0_storage=1 catalog_files=2 catalog_bytes=68 runtime_disk_io=0 mapped=0 shared_memory=0 filesystem_write=0 crash_consistency=0 general_runtime=0"

  [[ "$(grep -c '^FDT_VIRTIO_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^VIRTIO_IRQ_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^VIRTIO_BLK_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^BLOCK_IRQ_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^BLOCK_LAYER_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^GPT_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^DATA_GPT_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^FAT16_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^VFS_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^DATA_PERSIST_OK ' "$log_file" || true)" == "1" ]] \
    && [[ "$(grep -c '^STORAGE_LIMITS ' "$log_file" || true)" == "1" ]] \
    && validate_storage_marker_once "$log_file" "$fdt_marker" \
    && validate_storage_marker_once "$log_file" "$irq_marker" \
    && validate_storage_marker_once "$log_file" "$transport_marker" \
    && validate_storage_marker_once "$log_file" "$read_marker" \
    && validate_storage_marker_once "$log_file" "$block_layer_marker" \
    && validate_storage_marker_once "$log_file" "$gpt_marker" \
    && validate_storage_marker_once "$log_file" "$data_gpt_marker" \
    && validate_storage_marker_once "$log_file" "$fat_marker" \
    && validate_storage_marker_once "$log_file" "$vfs_marker" \
    && validate_storage_marker_once "$log_file" "$persist_marker" \
    && validate_storage_marker_once "$log_file" "$limits_marker"
}

# Stable negative contracts. No successful discovery, IRQ, block, GPT, FAT16, VFS, or
# storage-limit success evidence may be present after a rejection.
validate_storage_negative_evidence() {
  local log_file="$1"
  local case_name="$2"
  local marker

  case "$case_name" in
    legacy_transport)
      marker="STORAGE_FAIL reason=legacy_transport"
      ;;
    corrupt_image)
      marker="STORAGE_FAIL reason=image_mismatch"
      ;;
    corrupt_gpt)
      marker="STORAGE_FAIL reason=gpt_invalid"
      ;;
    corrupt_fat16)
      marker="STORAGE_FAIL reason=fat16_invalid"
      ;;
    corrupt_vfs)
      marker="STORAGE_FAIL reason=vfs_invalid"
      ;;
    corrupt_data_super|corrupt_data_slots|data_generation_gap)
      marker="STORAGE_FAIL reason=data_persistence_invalid"
      ;;
    read_only_device|missing_flush)
      marker="STORAGE_FAIL reason=driver_init"
      ;;
    no_block_device)
      marker="STORAGE_FAIL reason=no_block_device"
      ;;
    *)
      echo "Unknown storage negative case '$case_name'." >&2
      return 2
      ;;
  esac
  validate_storage_marker_once "$log_file" "$marker" \
    && [[ "$(grep -c '^STORAGE_FAIL ' "$log_file" || true)" == "1" ]] \
    && ! grep -Eq '^FDT_VIRTIO_OK |^VIRTIO_IRQ_OK |^VIRTIO_BLK_OK |^BLOCK_IRQ_OK |^BLOCK_LAYER_OK |^GPT_OK |^DATA_GPT_OK |^FAT16_OK |^VFS_OK |^DATA_PERSIST_OK |^STORAGE_LIMITS ' "$log_file"
}
