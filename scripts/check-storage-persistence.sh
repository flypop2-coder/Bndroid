#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for the storage persistence reboot test." >&2
    exit 1
  fi
done

PROFILE="${BNDROID_PROFILE:-debug}"
CANONICAL_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$CANONICAL_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$CANONICAL_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $CANONICAL_IMAGE" >&2
  exit 1
fi

BNDROID_PROFILE="$PROFILE" "$SCRIPT_DIR/build-kernel.sh" >/dev/null
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Kernel image was not built: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-storage-persistence.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
RECOVERY_IMAGE="$TMP_DIR/recovery.raw"
FIRST_LOG="$TMP_DIR/boot-1.log"
SECOND_LOG="$TMP_DIR/boot-2.log"
RECOVERY_FIRST_LOG="$TMP_DIR/recovery-boot-1.log"
RECOVERY_SECOND_LOG="$TMP_DIR/recovery-boot-2.log"
cp "$CANONICAL_IMAGE" "$RUNTIME_IMAGE"
cp "$CANONICAL_IMAGE" "$RECOVERY_IMAGE"
QEMU_PID=""

cleanup() {
  if [[ -n "$QEMU_PID" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run_persistent_boot() {
  local log_file="$1"
  local expected_marker="$2"
  local runtime_image="${3:-$RUNTIME_IMAGE}"
  local deadline

  build_storage_qemu_args "$runtime_image" modern writable-persistent
  : >"$log_file"
  : >"$log_file.normalized"
  qemu-system-aarch64 \
    -machine virt,virtualization=off,gic-version=2 \
    -cpu "${BNDROID_QEMU_CPU:-cortex-a57}" \
    -m 1024M \
    -nographic \
    -monitor none \
    -nic none \
    -serial stdio \
    -kernel "$KERNEL_IMAGE" \
    "${BNDROID_STORAGE_QEMU_ARGS[@]}" >"$log_file" 2>&1 &
  QEMU_PID=$!
  deadline=$((SECONDS + ${BNDROID_QEMU_TIMEOUT_SECONDS:-12}))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$log_file" >"$log_file.normalized"
    if grep -Fqx "$expected_marker" "$log_file.normalized"; then
      kill "$QEMU_PID" 2>/dev/null || true
      wait "$QEMU_PID" 2>/dev/null || true
      QEMU_PID=""
      if [[ "$(grep -c '^DATA_PERSIST_OK ' "$log_file.normalized" || true)" != "1" ]] \
        || grep -q '^STORAGE_FAIL ' "$log_file.normalized"; then
        echo "Persistent boot emitted conflicting storage evidence." >&2
        return 1
      fi
      return 0
    fi
    if grep -q '^STORAGE_FAIL ' "$log_file.normalized"; then
      break
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done

  kill "$QEMU_PID" 2>/dev/null || true
  wait "$QEMU_PID" 2>/dev/null || true
  QEMU_PID=""
  echo "Persistent storage boot did not emit the expected generation transition." >&2
  tail -n 80 "$log_file.normalized" >&2 || true
  return 1
}

FIRST_MARKER="DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 slots=2 initial_generation=0 committed_generation=1 initial_slot=0 committed_slot=1 valid_slots=1 rejected_slots=1 reads=5 writes=1 flushes=1 write_completion=1 flush_completion=1 readback_verified=1 old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 crash_consistency=0 qemu_reboot_proof=0"
SECOND_MARKER="DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 slots=2 initial_generation=1 committed_generation=2 initial_slot=1 committed_slot=0 valid_slots=2 rejected_slots=0 reads=5 writes=1 flushes=1 write_completion=1 flush_completion=1 readback_verified=1 old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 crash_consistency=0 qemu_reboot_proof=0"

run_persistent_boot "$FIRST_LOG" "$FIRST_MARKER"
run_persistent_boot "$SECOND_LOG" "$SECOND_MARKER"

run_persistent_boot "$RECOVERY_FIRST_LOG" "$FIRST_MARKER" "$RECOVERY_IMAGE"
python3 - "$RECOVERY_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[2])
import build_storage_image as fixture

path = Path(sys.argv[1])
image = bytearray(path.read_bytes())
active_lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
image[active_lba * fixture.SECTOR_BYTES + 80] ^= 1
path.write_bytes(image)
PY
run_persistent_boot "$RECOVERY_SECOND_LOG" "$FIRST_MARKER" "$RECOVERY_IMAGE"

python3 - "$CANONICAL_IMAGE" "$RUNTIME_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[3])
import build_storage_image as fixture

canonical = Path(sys.argv[1]).read_bytes()
runtime = Path(sys.argv[2]).read_bytes()
start = fixture.DATA_PARTITION_FIRST_LBA * fixture.SECTOR_BYTES
end = (fixture.DATA_PARTITION_LAST_LBA + 1) * fixture.SECTOR_BYTES
if canonical[:start] != runtime[:start] or canonical[end:] != runtime[end:]:
    raise SystemExit("a persistent boot changed bytes outside BNDROID_DATA")

def sector(image: bytes, lba: int) -> bytes:
    offset = lba * fixture.SECTOR_BYTES
    return image[offset : offset + fixture.SECTOR_BYTES]

if sector(runtime, fixture.DATA_PARTITION_FIRST_LBA) != fixture.build_data_superblock():
    raise SystemExit("the immutable data superblock changed")
slot_zero = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[0]
slot_one = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
if sector(runtime, slot_zero) != fixture.build_data_record(2, fixture.build_boot_state(2)):
    raise SystemExit("slot zero does not contain committed generation two")
if sector(runtime, slot_one) != fixture.build_data_record(1, fixture.build_boot_state(1)):
    raise SystemExit("slot one does not preserve committed generation one")
unused_start = (fixture.DATA_PARTITION_FIRST_LBA + 3) * fixture.SECTOR_BYTES
if any(runtime[unused_start:end]):
    raise SystemExit("a persistent boot changed an unused data-partition sector")
changed = sum(left != right for left, right in zip(canonical[start:end], runtime[start:end]))
print(
    "STORAGE_PERSISTENCE_REBOOT_OK "
    "boots=2 initial_generation=0 first_committed=1 second_initial=1 "
    "second_committed=2 final_slot=0 prior_slot=1 write_completions=2 "
    "flush_completions=2 qemu_backend_persistence=1 outside_data_unchanged=1 "
    f"changed_data_bytes={changed} filesystem_write=0 crash_consistency=0"
)
PY

python3 - "$CANONICAL_IMAGE" "$RECOVERY_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[3])
import build_storage_image as fixture

canonical = Path(sys.argv[1]).read_bytes()
recovered = Path(sys.argv[2]).read_bytes()
start = fixture.DATA_PARTITION_FIRST_LBA * fixture.SECTOR_BYTES
end = (fixture.DATA_PARTITION_LAST_LBA + 1) * fixture.SECTOR_BYTES
if canonical[:start] != recovered[:start] or canonical[end:] != recovered[end:]:
    raise SystemExit("corrupt-slot recovery changed bytes outside BNDROID_DATA")

def sector(image: bytes, lba: int) -> bytes:
    offset = lba * fixture.SECTOR_BYTES
    return image[offset : offset + fixture.SECTOR_BYTES]

slot_zero = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[0]
slot_one = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
if sector(recovered, slot_zero) != fixture.build_data_record(0, fixture.build_boot_state(0)):
    raise SystemExit("corrupt-slot recovery did not preserve the older valid record")
if sector(recovered, slot_one) != fixture.build_data_record(1, fixture.build_boot_state(1)):
    raise SystemExit("corrupt-slot recovery did not replace the rejected record")
print(
    "STORAGE_PERSISTENCE_RECOVERY_OK corrupt_newest_slot=1 rejected_slots=1 "
    "fallback_generation=0 recommitted_generation=1 old_slot_preserved=1 "
    "outside_data_unchanged=1 filesystem_write=0 crash_consistency=0"
)
PY

BNDROID_STORAGE_IMAGE="$CANONICAL_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" --verify >/dev/null
