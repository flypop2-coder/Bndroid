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
PROFILE="${BNDROID_PROFILE:-debug}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi

"$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-storage-negative.XXXXXX")"
GOOD_IMAGE="$TMP_DIR/storage-good.raw"
CORRUPT_IMAGE="$TMP_DIR/storage-corrupt.raw"
CORRUPT_GPT_IMAGE="$TMP_DIR/storage-corrupt-gpt.raw"
CORRUPT_FAT16_IMAGE="$TMP_DIR/storage-corrupt-fat16.raw"
CORRUPT_VFS_IMAGE="$TMP_DIR/storage-corrupt-vfs.raw"
CORRUPT_DATA_SUPER_IMAGE="$TMP_DIR/storage-corrupt-data-super.raw"
CORRUPT_DATA_SLOTS_IMAGE="$TMP_DIR/storage-corrupt-data-slots.raw"
DATA_GENERATION_GAP_IMAGE="$TMP_DIR/storage-data-generation-gap.raw"
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

BNDROID_STORAGE_IMAGE="$GOOD_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_IMAGE"
printf 'X' | dd of="$CORRUPT_IMAGE" bs=1 seek=0 count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_GPT_IMAGE"
printf 'X' | dd of="$CORRUPT_GPT_IMAGE" bs=1 seek=$((2 * 512)) count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_FAT16_IMAGE"
printf '\003' | dd of="$CORRUPT_FAT16_IMAGE" bs=1 seek=$((2048 * 512 + 13)) count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_VFS_IMAGE"
printf 'X' | dd of="$CORRUPT_VFS_IMAGE" bs=1 seek=$((2165 * 512)) count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_DATA_SUPER_IMAGE"
printf 'X' | dd of="$CORRUPT_DATA_SUPER_IMAGE" bs=1 seek=$((64 * 512)) count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$CORRUPT_DATA_SLOTS_IMAGE"
printf 'X' | dd of="$CORRUPT_DATA_SLOTS_IMAGE" bs=1 seek=$((65 * 512 + 80)) count=1 conv=notrunc 2>/dev/null
cp "$GOOD_IMAGE" "$DATA_GENERATION_GAP_IMAGE"
python3 - "$DATA_GENERATION_GAP_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[2])
import build_storage_image as fixture

path = Path(sys.argv[1])
image = bytearray(path.read_bytes())
record = fixture.build_data_record(2, fixture.build_boot_state(2))
lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
offset = lba * fixture.SECTOR_BYTES
image[offset : offset + fixture.SECTOR_BYTES] = record
path.write_bytes(image)
PY

if ! validate_storage_image_geometry "$GOOD_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_GPT_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_FAT16_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_VFS_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_DATA_SUPER_IMAGE" \
  || ! validate_storage_image_geometry "$CORRUPT_DATA_SLOTS_IMAGE" \
  || ! validate_storage_image_geometry "$DATA_GENERATION_GAP_IMAGE"; then
  echo "Storage negative fixtures do not have the required 8 MiB geometry." >&2
  exit 1
fi
if BNDROID_STORAGE_IMAGE="$CORRUPT_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1; then
  echo "Corrupt storage fixture unexpectedly passed deterministic image validation." >&2
  exit 1
fi
if BNDROID_STORAGE_IMAGE="$CORRUPT_GPT_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1 \
  || BNDROID_STORAGE_IMAGE="$CORRUPT_FAT16_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1 \
  || BNDROID_STORAGE_IMAGE="$CORRUPT_VFS_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1 \
  || BNDROID_STORAGE_IMAGE="$CORRUPT_DATA_SUPER_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1 \
  || BNDROID_STORAGE_IMAGE="$CORRUPT_DATA_SLOTS_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1 \
  || BNDROID_STORAGE_IMAGE="$DATA_GENERATION_GAP_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null 2>&1; then
  echo "A structurally corrupt M25 fixture unexpectedly passed deterministic image validation." >&2
  exit 1
fi

run_negative_case() {
  local case_name="$1"
  local transport="$2"
  local image_path="$3"
  local access="${4:-writable-temporary}"
  local log_file="$TMP_DIR/$case_name.qemu.log"
  local normalized_log="$TMP_DIR/$case_name.qemu.normalized.log"
  local deadline status

  : >"$log_file"
  build_storage_qemu_args "$image_path" "$transport" "$access"
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
    "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
    >"$log_file" 2>&1 &
  QEMU_PID=$!
  deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))

  while ((SECONDS < deadline)); do
    tr -d '\r' <"$log_file" >"$normalized_log"
    if validate_storage_negative_evidence "$normalized_log" "$case_name"; then
      if grep -Eq '^FDT_VIRTIO_OK |^VIRTIO_IRQ_OK |^VIRTIO_BLK_OK |^BLOCK_IRQ_OK |^BLOCK_LAYER_OK |^GPT_OK |^DATA_GPT_OK |^FAT16_OK |^VFS_OK |^DATA_PERSIST_OK |^STORAGE_LIMITS ' "$normalized_log"; then
        cat "$normalized_log"
        echo "Storage negative case '$case_name' also emitted a success marker." >&2
        return 1
      fi
      kill "$QEMU_PID" 2>/dev/null || true
      wait "$QEMU_PID" 2>/dev/null || true
      QEMU_PID=""
      printf 'Storage negative case passed: %s\n' "$case_name"
      return 0
    fi
    if grep -Eq 'fatal exception:|COPYIO_FIXUP_MISS' "$normalized_log"; then
      cat "$normalized_log"
      echo "Storage negative case '$case_name' hit an unrelated fatal failure." >&2
      return 1
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      set +e
      wait "$QEMU_PID"
      status=$?
      set -e
      QEMU_PID=""
      cat "$normalized_log"
      echo "QEMU exited before the expected '$case_name' storage marker (status $status)." >&2
      return 1
    fi
    sleep 0.1
  done

  tr -d '\r' <"$log_file"
  echo "Timed out waiting for the '$case_name' M25 storage rejection marker." >&2
  return 1
}

run_negative_case legacy_transport legacy "$GOOD_IMAGE"
run_negative_case corrupt_image modern "$CORRUPT_IMAGE"
run_negative_case corrupt_gpt modern "$CORRUPT_GPT_IMAGE"
run_negative_case corrupt_fat16 modern "$CORRUPT_FAT16_IMAGE"
run_negative_case corrupt_vfs modern "$CORRUPT_VFS_IMAGE"
run_negative_case corrupt_data_super modern "$CORRUPT_DATA_SUPER_IMAGE"
run_negative_case corrupt_data_slots modern "$CORRUPT_DATA_SLOTS_IMAGE"
run_negative_case data_generation_gap modern "$DATA_GENERATION_GAP_IMAGE"
run_negative_case read_only_device modern "$GOOD_IMAGE" readonly
run_negative_case missing_flush modern "$GOOD_IMAGE" writable-no-flush
run_negative_case no_block_device none "$GOOD_IMAGE"
echo "Storage negative checks passed: legacy/read-only/missing-FLUSH devices, deterministic-image corruption, GPT/FAT16/VFS corruption, invalid data super/slots/generation gap, and an all-placeholder topology were rejected."
