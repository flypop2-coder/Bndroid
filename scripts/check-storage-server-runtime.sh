#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill mkdir rm sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M55 StorageServer runtime." >&2
    exit 1
  fi
done

PROFILE="${BNDROID_PROFILE:-release}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-180}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be an integer from 1 through 600." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi
if [[ "$KEEP_TEMP" != "0" && "$KEEP_TEMP" != "1" ]]; then
  echo "BNDROID_KEEP_TEMP must be 0 or 1." >&2
  exit 2
fi

BOOT_MARKER="BOOT_OK: M55 userspace StorageServer ownership, restart, rebind, and AppData I/O verified"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_RUNTIME_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-runtime-check}"
CANONICAL_IMAGE="$TARGET_ROOT/bndroid-storage-m55.raw"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m55"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$CANONICAL_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$CANONICAL_IMAGE" --appdata

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=storage-server-runtime \
  BNDROID_KERNEL_FEATURES=storage-server-runtime \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M55 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m55.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
cp "$CANONICAL_IMAGE" "$RUNTIME_IMAGE"
QEMU_PID=""

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M55 diagnostic artifacts retained at $TMP_DIR" >&2
  else
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  local normalized_log="$1"
  if [[ -f "$normalized_log" ]]; then
    tail -n 160 "$normalized_log" >&2 || true
  fi
}

reject_failure() {
  local normalized_log="$1"
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|STORAGE_FAIL|USER_FAIL:|EL0_FAIL|M55_STORAGE_(DIAG|READY_DIAG|TIMEOUT)' "$normalized_log" \
    || grep -Eq '^STORAGE_SERVER_FAULT_POLICY_OK( |$)|^BOOT_OK: M60( |$)' "$normalized_log"; then
    show_failure "$normalized_log"
    echo "M55 emitted a panic, failure, diagnostic, timeout, or M60 marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$normalized_log" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure "$normalized_log"
    echo "M55 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_boot_evidence() {
  local boot="$1"
  local normalized_log="$2"
  local generation="$3"
  local app_stage="$4"
  local expected_io
  local expected_runtime

  case "$boot" in
    1)
      expected_io="STORAGE_SERVER_IO_OK read_batches=1425 write_batches=289 read_sectors=11400 write_sectors=2121 max_batch_sectors=8 flushes=12 completions=1726 errors=0 submissions=1726 retrievals=1726 bounds_rejections=0 irq_monitor=1 namespace_policy=0"
      ;;
    2)
      expected_io="STORAGE_SERVER_IO_OK read_batches=765 write_batches=45 read_sectors=6120 write_sectors=290 max_batch_sectors=8 flushes=2 completions=812 errors=0 submissions=812 retrievals=812 bounds_rejections=0 irq_monitor=1 namespace_policy=0"
      ;;
    3)
      expected_io="STORAGE_SERVER_IO_OK read_batches=606 write_batches=0 read_sectors=4848 write_sectors=0 max_batch_sectors=8 flushes=0 completions=606 errors=0 submissions=606 retrievals=606 bounds_rejections=0 irq_monitor=1 namespace_policy=0"
      ;;
    *)
      echo "invalid M55 boot index: $boot" >&2
      exit 2
      ;;
  esac
  expected_runtime="STORAGE_SERVER_RUNTIME_OK abi=25 disk_generation=$generation launcher_stage=2 app_stage=$app_stage server_v1=0x0000000100000002 server_v2=0x0000000200000002 same_slot=1 generation_advanced=1 epoch=2 releases=1 sessions=4 pending=0 resident=1 init_handles=1 server_handles=2 errors=0"

  if [[ "$(grep -Fxc "$expected_io" "$normalized_log" || true)" != "1" ]] \
    || [[ "$(grep -Fxc "$expected_runtime" "$normalized_log" || true)" != "1" ]] \
    || [[ "$(grep -Fxc "$BOOT_MARKER" "$normalized_log" || true)" != "1" ]] \
    || [[ "$(grep -c '^USER_IMAGE_CATALOG_OK ' "$normalized_log" || true)" != "1" ]] \
    || [[ "$(grep -c '^ELF_LOAD_OK ' "$normalized_log" || true)" != "1" ]] \
    || [[ "$(grep -c '^STORAGE_SERVER_SCHED_OK ' "$normalized_log" || true)" != "1" ]]; then
    show_failure "$normalized_log"
    echo "M55 boot $boot did not publish its exact I/O, generation, or runtime evidence." >&2
    exit 1
  fi

  if ! python3 - "$normalized_log" "$expected_io" <<'PY'
import re
import sys
from pathlib import Path

log = Path(sys.argv[1]).read_text()
io_line = sys.argv[2]
completion_match = re.search(r"\bcompletions=(\d+)\b", io_line)
if completion_match is None:
    raise SystemExit("M55 checker lost its completion expectation")
completions = int(completion_match.group(1))
scheduler_lines = [line for line in log.splitlines() if line.startswith("STORAGE_SERVER_SCHED_OK ")]
if len(scheduler_lines) != 1:
    raise SystemExit("M55 scheduler evidence was not unique")
fields = dict(re.findall(r"([a-z_]+)=([^ ]+)", scheduler_lines[0]))
required = {
    "logical_hz": "100",
    "preferred_pending": "0",
    "stale": "0",
    "trapframe_switch": "1",
}
if any(fields.get(key) != value for key, value in required.items()):
    raise SystemExit("M55 scheduler evidence changed its safety contract")
for key in (
    "immediate_requests",
    "immediate_interrupts",
    "completion_scans",
    "candidate_scans",
    "preferred_requests",
    "preferred_dispatches",
):
    if int(fields.get(key, "0")) <= 0:
        raise SystemExit(f"M55 scheduler evidence did not exercise {key}")
completion_scans = int(fields["completion_scans"])
candidate_scans = int(fields["candidate_scans"])
no_candidate_scans = int(fields.get("no_candidate_scans", "-1"))
preferred_requests = int(fields["preferred_requests"])
preferred_coalesced = int(fields.get("preferred_coalesced", "-1"))
preferred_dispatches = int(fields["preferred_dispatches"])
preferred_pending = int(fields["preferred_pending"])
stale = int(fields["stale"])
immediate_requests = int(fields["immediate_requests"])
immediate_interrupts = int(fields["immediate_interrupts"])
if not (0 < immediate_interrupts <= immediate_requests <= completions):
    raise SystemExit("M55 immediate-reschedule ledger exceeded its legal completion bounds")
if completion_scans != completions:
    raise SystemExit("M55 completion scan ledger did not match broker completions")
if candidate_scans < 0 or no_candidate_scans < 0 or candidate_scans + no_candidate_scans != completion_scans:
    raise SystemExit("M55 completion scans were not exactly classified")
if preferred_coalesced < 0 or preferred_requests + preferred_coalesced != candidate_scans:
    raise SystemExit("M55 preference candidates were not exactly classified")
if preferred_dispatches + stale + preferred_pending != preferred_requests:
    raise SystemExit("M55 preference requests were not exactly retired")

catalog_lines = [line for line in log.splitlines() if line.startswith("USER_IMAGE_CATALOG_OK ")]
catalog_pattern = re.compile(
    r"^USER_IMAGE_CATALOG_OK count=8 distinct=1 "
    r"init_bytes=([1-9][0-9]*) manager_bytes=([1-9][0-9]*) "
    r"provider_bytes=([1-9][0-9]*) client_bytes=([1-9][0-9]*) "
    r"surface_bytes=([1-9][0-9]*) launcher_bytes=([1-9][0-9]*) "
    r"app_bytes=([1-9][0-9]*) storage_server_bytes=([1-9][0-9]*) "
    r"init_digest=(0x[0-9a-f]{16}) manager_digest=(0x[0-9a-f]{16}) "
    r"provider_digest=(0x[0-9a-f]{16}) client_digest=(0x[0-9a-f]{16}) "
    r"surface_digest=(0x[0-9a-f]{16}) launcher_digest=(0x[0-9a-f]{16}) "
    r"app_digest=(0x[0-9a-f]{16}) storage_server_digest=(0x[0-9a-f]{16})$"
)
catalog_match = catalog_pattern.fullmatch(catalog_lines[0]) if len(catalog_lines) == 1 else None
if catalog_match is None or len(set(catalog_match.groups()[8:])) != 8:
    raise SystemExit("M55 did not publish an exact nonempty, pairwise-distinct eight-image catalog")

elf_lines = [line for line in log.splitlines() if line.startswith("ELF_LOAD_OK ")]
elf_pattern = re.compile(
    r"^ELF_LOAD_OK elf=1 segments=3 load_pages=[1-9][0-9]* "
    r"rx_pages=[1-9][0-9]* ro_pages=[1-9][0-9]* rw_pages=[1-9][0-9]* "
    r"bss_bytes=[1-9][0-9]* elf_bytes=[1-9][0-9]* "
    r"file_bytes=[1-9][0-9]* memory_bytes=[1-9][0-9]* wx=0$"
)
if len(elf_lines) != 1 or elf_pattern.fullmatch(elf_lines[0]) is None:
    raise SystemExit("M55 did not publish the exact bounded W^X ELF-load evidence")
PY
  then
    show_failure "$normalized_log"
    echo "M55 boot $boot failed semantic scheduler, catalog, or ELF evidence validation." >&2
    exit 1
  fi
}

run_boot() {
  local boot="$1"
  local generation="$2"
  local app_stage="$3"
  local serial_log="$TMP_DIR/boot-$boot.serial.log"
  local normalized_log="$TMP_DIR/boot-$boot.log"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))

  : >"$serial_log"
  : >"$normalized_log"
  qemu-system-aarch64 \
    -machine virt,gic-version=2,secure=off,virtualization=off \
    -cpu "$QEMU_CPU" \
    -smp 1 \
    -m 128M \
    -display none \
    -monitor none \
    -nic none \
    -serial stdio \
    -no-reboot \
    -kernel "$KERNEL_IMAGE" \
    -device ramfb \
    -global virtio-mmio.force-legacy=false \
    -drive "if=none,file=$RUNTIME_IMAGE,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
    -device "virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off" \
    >"$serial_log" 2>&1 &
  QEMU_PID=$!

  while ((SECONDS < deadline)); do
    tr -d '\r' <"$serial_log" >"$normalized_log"
    reject_failure "$normalized_log"
    if [[ "$(grep -Fxc "$BOOT_MARKER" "$normalized_log" || true)" == "1" ]]; then
      kill "$QEMU_PID" 2>/dev/null || true
      wait "$QEMU_PID" 2>/dev/null || true
      QEMU_PID=""
      tr -d '\r' <"$serial_log" >"$normalized_log"
      reject_failure "$normalized_log"
      validate_boot_evidence "$boot" "$normalized_log" "$generation" "$app_stage"
      cp "$normalized_log" "$OUTPUT_DIR/boot-$boot.log"
      cp "$RUNTIME_IMAGE" "$TMP_DIR/after-boot-$boot.raw"
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      show_failure "$normalized_log"
      echo "QEMU exited before M55 boot $boot completed." >&2
      exit 1
    fi
    sleep 0.05
  done

  show_failure "$normalized_log"
  echo "Timed out waiting for M55 boot $boot." >&2
  exit 1
}

run_boot 1 4 1
run_boot 2 5 2
run_boot 3 5 2

python3 - "$CANONICAL_IMAGE" "$TMP_DIR/after-boot-1.raw" "$TMP_DIR/after-boot-2.raw" "$TMP_DIR/after-boot-3.raw" <<'PY'
import hashlib
import struct
import sys
from pathlib import Path

paths = [Path(argument) for argument in sys.argv[1:]]
canonical, boot1, boot2, boot3 = [path.read_bytes() for path in paths]
sector = 512
data_start, data_end = 64 * sector, 128 * sector
app_start, app_end = 128 * sector, 2048 * sector

if any(len(image) != 8 * 1024 * 1024 for image in (canonical, boot1, boot2, boot3)):
    raise SystemExit("M55 persistence evidence lost the exact 8 MiB geometry")
if any(canonical[app_start:app_end]):
    raise SystemExit("M55 canonical AppData partition was not virgin")
for index, image in enumerate((boot1, boot2, boot3), 1):
    if canonical[:data_start] != image[:data_start] or canonical[app_end:] != image[app_end:]:
        raise SystemExit(f"M55 boot {index} changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
    if image[app_start:app_start + 8] != b"BNDAPS01":
        raise SystemExit(f"M55 boot {index} did not retain the AppData superblock")

def app_hash(image: bytes) -> str:
    return hashlib.sha256(image[app_start:app_end]).hexdigest()

def full_hash(image: bytes) -> str:
    return hashlib.sha256(image).hexdigest()

def checkpoint_generations(image: bytes) -> tuple[int, int]:
    generations = []
    for relative_lba in (1, 2):
        offset = app_start + relative_lba * sector
        if image[offset:offset + 8] != b"BNDAPC01":
            raise SystemExit("M55 AppData checkpoint magic is missing")
        generations.append(struct.unpack_from("<Q", image, offset + 16)[0])
    return tuple(generations)

app_hashes = [app_hash(image) for image in (canonical, boot1, boot2, boot3)]
if app_hashes[0] == app_hashes[1] or app_hashes[1] == app_hashes[2] or app_hashes[2] != app_hashes[3]:
    raise SystemExit("M55 AppData hashes did not prove first mutation, second mutation, and stable third boot")
if checkpoint_generations(boot1) != (4, 3):
    raise SystemExit("M55 first boot checkpoints are not generations 4/3")
if checkpoint_generations(boot2) != (4, 5) or checkpoint_generations(boot3) != (4, 5):
    raise SystemExit("M55 second/stable checkpoints are not generations 4/5")
if boot2[app_start:app_end] != boot3[app_start:app_end]:
    raise SystemExit("M55 stable third boot changed AppData bytes")

full_hashes = [full_hash(image) for image in (canonical, boot1, boot2, boot3)]
if len(set(full_hashes)) != 4:
    raise SystemExit("M55 independent BNDROID_DATA boot generations did not advance")

print(
    "STORAGE_SERVER_REBOOT_OK "
    "boots=3 appdata_generations=0-4-5-5 app_stages=1-2-2 "
    "first_changed=1 second_changed=1 third_stable=1 third_write_batches=0 "
    "third_flushes=0 outside_data_and_appdata_unchanged=1 "
    f"appdata_sha256={app_hashes[1]}/{app_hashes[2]}/{app_hashes[3]} "
    "whole_disk_changes_explained_by_bndroid_data_boot_counter=1"
)
PY
