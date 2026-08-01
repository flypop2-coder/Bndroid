#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

HEALTH_PREFIX="STORAGE_DEVICE_HEALTH_OK"
DATA_PREFIX="DATA_PERSIST_OK"
PLATFORM_PREFIX="RESIDENT_PLATFORM_SHUTDOWN_OK"
BOOT_MARKER="BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed"

run_evidence_parser() {
  python3 - "$@" <<'PY'
import sys
from pathlib import Path

HEALTH_PREFIX = "STORAGE_DEVICE_HEALTH_OK"
DATA_PREFIX = "DATA_PERSIST_OK"
PLATFORM_PREFIX = "RESIDENT_PLATFORM_SHUTDOWN_OK"
BOOT_MARKER = (
    "BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed"
)

HEALTH_SCHEMA = [
    ("format", "1"),
    ("state_version", "1"),
    ("authority", "kernel-boot-probe"),
    ("record_role", "unclosed-boot-hint"),
    ("legacy_upgrade", None),
    ("persisted_contract", None),
    ("prior_boot_open", "0"),
    ("contract_changed", "0"),
    ("reprobe_required", "0"),
    ("reprobe_verified", "1"),
    ("current_boot_open", "1"),
    ("offline_persisted", "0"),
    ("offline_from_record", "0"),
    ("el0_controls", "0"),
    ("capacity_sectors", "16384"),
    ("selected_features", "0x0000000100000200"),
    ("transport_base", "0x000000000a003e00"),
    ("irq", "79"),
    ("sector_bytes", "512"),
    ("queue_size", "8"),
    ("device_read_only", "0"),
    ("flush_supported", "1"),
    ("initial_generation", None),
    ("committed_generation", None),
    ("initial_slot", "0"),
    ("committed_slot", "1"),
    ("reads", "5"),
    ("writes", "1"),
    ("flushes", "1"),
    ("readback_verified", "1"),
    ("qemu_reboot_proof", "0"),
    ("hardware_identity_claim", "0"),
    ("hotplug_claim", "0"),
    ("powercut_claim", "0"),
    ("tamper_resistance_claim", "0"),
    ("general_runtime", "0"),
]

PLATFORM_SCHEMA = [
    ("format", "1"),
    ("abi", "27"),
    ("authority", "init-resident-graph-plus-kernel-seal"),
    ("appdata_generation", None),
    ("open_generation", None),
    ("closed_generation", None),
    ("open_slot", "1"),
    ("closed_slot", "0"),
    ("prior_boot_open", "1"),
    ("current_boot_open", "0"),
    ("contract_matched", "1"),
    ("health_reads", "5"),
    ("health_writes", "1"),
    ("health_flushes", "1"),
    ("resident_nodes", "8"),
    ("dependency_edges", "10"),
    ("quiesce_waves", "3"),
    ("registered_mask", "0xff"),
    ("quiesced_mask", "0xff"),
    ("registration_calls", "9"),
    ("quiesce_calls", "9"),
    ("order_rejections", "1"),
    ("children_created", "10"),
    ("children_exited", "9"),
    ("children_reaped", "9"),
    ("live_processes", "1"),
    ("dynamic_contexts", "0"),
    ("dynamic_stacks", "0"),
    ("prepare_calls", "4"),
    ("prepares", "1"),
    ("commit_calls", "1"),
    ("commits", "1"),
    ("permission_denied", "2"),
    ("early_rejections", "1"),
    ("spawn_rejections", "1"),
    ("connect_rejections", "2"),
    ("broker_bound", "0"),
    ("broker_pending", "0"),
    ("broker_state", "0"),
    ("storage_server_submissions", None),
    ("admission_closed", "1"),
    ("recovery_required", "0"),
    ("irq_armed", "0"),
    ("irq_failed", "0"),
    ("in_flight", "0"),
    ("requests_terminal", "1"),
    ("post_close_storage_mutations", "0"),
    ("storage_server_flushed", "1"),
    ("storage_server_readback", "1"),
    ("storage_server_exited", "1"),
    ("emulator_backend", "qemu-semihosting"),
    ("emulator_exit_requested", "1"),
    ("emulator_exit_verified_by_host", "0"),
    ("full_userspace_shutdown_claim", "0"),
    ("hardware_poweroff_claim", "0"),
    ("psci_claim", "0"),
    ("powercut_claim", "0"),
    ("smp_claim", "0"),
    ("general_runtime", "0"),
]

PHASES = {
    "first": {
        "legacy_upgrade": "1",
        "persisted_contract": "0",
        "initial_generation": "0",
        "committed_generation": "1",
        "appdata_generation": "5",
        "open_generation": "1",
        "closed_generation": "2",
        "storage_server_submissions": "1878",
        "valid_slots": "1",
        "rejected_slots": "1",
    },
    "second": {
        "legacy_upgrade": "0",
        "persisted_contract": "1",
        "initial_generation": "2",
        "committed_generation": "3",
        "appdata_generation": "6",
        "open_generation": "3",
        "closed_generation": "4",
        "storage_server_submissions": "660",
        "valid_slots": "2",
        "rejected_slots": "0",
    },
}


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def exact_marker(lines: list[str], prefix: str) -> str:
    matches = [line for line in lines if line == prefix or line.startswith(prefix + " ")]
    reject(len(matches) != 1, f"{prefix} count is not exactly one")
    return matches[0]


def marker(prefix: str, schema: list[tuple[str, str | None]], phase: str) -> str:
    values = PHASES[phase]
    fields = [
        f"{name}={expected if expected is not None else values[name]}"
        for name, expected in schema
    ]
    return f"{prefix} {' '.join(fields)}"


def data_marker(phase: str) -> str:
    values = PHASES[phase]
    return (
        "DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 "
        f"slots=2 initial_generation={values['initial_generation']} "
        f"committed_generation={values['committed_generation']} initial_slot=0 "
        f"committed_slot=1 valid_slots={values['valid_slots']} "
        f"rejected_slots={values['rejected_slots']} reads=5 writes=1 "
        "flushes=1 write_completion=1 flush_completion=1 readback_verified=1 "
        "old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 "
        "rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 "
        "crash_consistency=0 qemu_reboot_proof=0"
    )


def validate_text(text: str, phase: str) -> None:
    reject(phase not in PHASES, "unknown M66 boot phase")
    lines = text.splitlines()
    reject(
        exact_marker(lines, HEALTH_PREFIX) != marker(HEALTH_PREFIX, HEALTH_SCHEMA, phase),
        "M66 health marker changed exact structure",
    )
    reject(
        exact_marker(lines, DATA_PREFIX) != data_marker(phase),
        "M66 DATA_PERSIST marker changed exact structure",
    )
    reject(
        exact_marker(lines, PLATFORM_PREFIX)
        != marker(PLATFORM_PREFIX, PLATFORM_SCHEMA, phase),
        "M66 platform marker changed exact structure",
    )
    reject(lines.count(BOOT_MARKER) != 1, "M66 BOOT marker changed")
    reject(
        any(line.startswith("BOOT_OK: M") and line != BOOT_MARKER for line in lines),
        "M66 emitted a stale BOOT marker",
    )
    reject(
        any(
            line.startswith(prefix)
            for line in lines
            for prefix in (
                "STORAGE_SERVER_RUNTIME_OK ",
                "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_OK ",
                "STORAGE_SERVER_CLEAN_SHUTDOWN_OK ",
            )
        ),
        "M66 emitted a parent runtime marker",
    )


def canonical_text(phase: str) -> str:
    return "\n".join(
        [
            marker(HEALTH_PREFIX, HEALTH_SCHEMA, phase),
            data_marker(phase),
            marker(PLATFORM_PREFIX, PLATFORM_SCHEMA, phase),
            BOOT_MARKER,
        ]
    ) + "\n"


def expect_rejected(text: str, phase: str) -> None:
    try:
        validate_text(text, phase)
    except EvidenceError:
        return
    raise AssertionError("negative M66 parser fixture was accepted")


def self_test() -> None:
    first = canonical_text("first")
    second = canonical_text("second")
    validate_text(first, "first")
    validate_text(second, "second")
    cases = 0

    marker_specs = (
        (marker(HEALTH_PREFIX, HEALTH_SCHEMA, "first"), 0),
        (data_marker("first"), 1),
        (marker(PLATFORM_PREFIX, PLATFORM_SCHEMA, "first"), 2),
    )
    for canonical, line_index in marker_specs:
        for token in canonical.split()[1:]:
            name = token.split("=", 1)[0]
            mutated = canonical.replace(token, f"{name}=__bad__", 1)
            expect_rejected(first.replace(canonical, mutated, 1), "first")
            cases += 1
        for mutation in ("remove", "extra", "duplicate", "prefix"):
            lines = first.splitlines()
            if mutation == "remove":
                lines[line_index] = " ".join(lines[line_index].split()[:-1])
            elif mutation == "extra":
                lines[line_index] += " extra=1"
            elif mutation == "duplicate":
                lines.insert(line_index, lines[line_index])
            else:
                lines[line_index] = "STALE_" + lines[line_index]
            expect_rejected("\n".join(lines) + "\n", "first")
            cases += 1

    for line_index in range(4):
        lines = first.splitlines()
        lines.pop(line_index)
        expect_rejected("\n".join(lines) + "\n", "first")
        cases += 1
        lines = first.splitlines()
        lines.insert(line_index, lines[line_index])
        expect_rejected("\n".join(lines) + "\n", "first")
        cases += 1

    expect_rejected(second, "first")
    expect_rejected(first, "second")
    cases += 2
    for stale in (
        "BOOT_OK: M65 stale parent",
        "STORAGE_SERVER_RUNTIME_OK stale=1",
        "STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_OK stale=1",
    ):
        expect_rejected(first + stale + "\n", "first")
        cases += 1

    print(
        "RESIDENT_PLATFORM_SHUTDOWN_PARSER_OK "
        f"negative_cases={cases} health_fields={len(HEALTH_SCHEMA)} "
        f"platform_fields={len(PLATFORM_SCHEMA)} phases=2"
    )


if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
    self_test()
elif len(sys.argv) == 3:
    validate_text(Path(sys.argv[2]).read_text(encoding="utf-8"), sys.argv[1])
else:
    raise SystemExit("usage: parser (--self-test | PHASE LOG)")
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  run_evidence_parser --self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  run_evidence_parser "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--parser-self-test | --parse PHASE LOG]" >&2
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tr grep kill sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M66 resident platform shutdown." >&2
    exit 1
  fi
done
if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M66 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
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

FEATURE="resident-platform-shutdown-runtime"
TARGET_ROOT="${BNDROID_RESIDENT_PLATFORM_SHUTDOWN_TARGET_DIR:-$WORKSPACE_ROOT/target/resident-platform-shutdown-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m66"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-resident-platform-m66.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
FIRST_SERIAL="$TMP_DIR/boot-1.serial.log"
FIRST_LOG="$TMP_DIR/boot-1.log"
SECOND_SERIAL="$TMP_DIR/boot-2.serial.log"
SECOND_LOG="$TMP_DIR/boot-2.log"
QEMU_PID=""
QEMU_SELF_EXITS=0
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M66 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  local log_file="$1"
  [[ -f "$log_file" ]] && tail -n 420 "$log_file" >&2 || true
}

reject_failure() {
  local log_file="$1"
  [[ -f "$log_file" ]] || return
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$log_file" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M66_[A-Z0-9_]*TIMEOUT|^BOOT_OK: M(5[5-9]|6[0-5]) ' "$log_file"; then
    show_failure "$log_file"
    echo "M66 emitted a panic, failure, timeout, diagnostic, or stale BOOT marker." >&2
    exit 1
  fi
  if grep -Eq '^STORAGE_SERVER_(SHUTDOWN_ORCHESTRATION|CLEAN_SHUTDOWN|RUNTIME)_OK' "$log_file"; then
    show_failure "$log_file"
    echo "M66 emitted a parent runtime marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$log_file" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure "$log_file"
    echo "M66 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

run_persistent_boot() {
  local phase="$1"
  local serial_log="$2"
  local normalized_log="$3"
  local deadline
  local health_count
  local data_count
  local platform_count
  local boot_count
  local qemu_status

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
    -semihosting-config enable=on,target=native \
    -kernel "$KERNEL_IMAGE" \
    -device ramfb \
    -global virtio-mmio.force-legacy=false \
    -drive "if=none,file=$RUNTIME_IMAGE,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
    -device "virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off" \
    >"$serial_log" 2>&1 &
  QEMU_PID=$!

  deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$serial_log" >"$normalized_log"
    reject_failure "$normalized_log"
    health_count="$(grep -Ec "^${HEALTH_PREFIX}( |$)" "$normalized_log" || true)"
    data_count="$(grep -Ec "^${DATA_PREFIX}( |$)" "$normalized_log" || true)"
    platform_count="$(grep -Ec "^${PLATFORM_PREFIX}( |$)" "$normalized_log" || true)"
    boot_count="$(grep -Fxc "$BOOT_MARKER" "$normalized_log" || true)"
    if ((health_count > 1 || data_count > 1 || platform_count > 1 || boot_count > 1)); then
      show_failure "$normalized_log"
      echo "M66 published duplicate health, persistence, platform, or BOOT evidence." >&2
      exit 1
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      set +e
      wait "$QEMU_PID"
      qemu_status=$?
      set -e
      QEMU_PID=""
      tr -d '\r' <"$serial_log" >"$normalized_log"
      reject_failure "$normalized_log"
      # QEMU can exit after the loop sampled the serial file but before this
      # branch observes the process death.  Recount the final, flushed log so
      # the exact-evidence decision never uses that earlier partial snapshot.
      health_count="$(grep -Ec "^${HEALTH_PREFIX}( |$)" "$normalized_log" || true)"
      data_count="$(grep -Ec "^${DATA_PREFIX}( |$)" "$normalized_log" || true)"
      platform_count="$(grep -Ec "^${PLATFORM_PREFIX}( |$)" "$normalized_log" || true)"
      boot_count="$(grep -Fxc "$BOOT_MARKER" "$normalized_log" || true)"
      if [[ "$qemu_status" != 0 ]]; then
        show_failure "$normalized_log"
        echo "QEMU did not self-exit successfully for M66 $phase boot (status $qemu_status)." >&2
        exit 1
      fi
      if [[ "$health_count" != 1 || "$data_count" != 1 \
        || "$platform_count" != 1 || "$boot_count" != 1 ]]; then
        show_failure "$normalized_log"
        echo "QEMU self-exited before exact M66 $phase-boot evidence was complete." >&2
        exit 1
      fi
      run_evidence_parser "$phase" "$normalized_log"
      QEMU_SELF_EXITS=$((QEMU_SELF_EXITS + 1))
      return 0
    fi
    sleep 0.05
  done

  tr -d '\r' <"$serial_log" >"$normalized_log"
  reject_failure "$normalized_log"
  show_failure "$normalized_log"
  echo "Timed out waiting for M66 $phase boot to self-exit." >&2
  exit 1
}

run_evidence_parser --self-test >/dev/null
python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_KERNEL_FEATURES="$FEATURE" \
  BNDROID_USERSPACE_FEATURES="$FEATURE" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M66 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

run_persistent_boot first "$FIRST_SERIAL" "$FIRST_LOG"
run_persistent_boot second "$SECOND_SERIAL" "$SECOND_LOG"
if [[ "$QEMU_SELF_EXITS" != 2 ]]; then
  echo "M66 did not observe exactly two successful QEMU self-exits." >&2
  exit 1
fi

REBOOT_MARKER="$(
  python3 - "$PRISTINE_IMAGE" "$RUNTIME_IMAGE" "$SCRIPT_DIR" <<'PY'
import struct
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[3])
import build_storage_image as fixture

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
sector_bytes = fixture.SECTOR_BYTES
if len(before) != 8 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit("M66 runtime disk lost its exact writable 8 MiB geometry")

data_start = fixture.DATA_PARTITION_FIRST_LBA * sector_bytes
data_end = (fixture.DATA_PARTITION_LAST_LBA + 1) * sector_bytes
appdata_start = fixture.APPDATA_PARTITION_FIRST_LBA * sector_bytes
appdata_end = (fixture.APPDATA_PARTITION_LAST_LBA + 1) * sector_bytes
if before[:data_start] != after[:data_start] or before[appdata_end:] != after[appdata_end:]:
    raise SystemExit("M66 QEMU changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
if before[appdata_start:appdata_end] == after[appdata_start:appdata_end]:
    raise SystemExit("M66 did not persist its bounded AppData workload")


def sector(image: bytes, lba: int) -> bytes:
    start = lba * sector_bytes
    return image[start : start + sector_bytes]


def fnv1a64(data: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x00000100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def decode_health_record(raw: bytes, expected_generation: int, boot_open: bool) -> None:
    if raw[:8] != fixture.DATA_RECORD_MAGIC:
        raise SystemExit("M66 health record magic changed")
    if struct.unpack_from("<I", raw, 8)[0] != fixture.DATA_FORMAT_VERSION:
        raise SystemExit("M66 health record version changed")
    if struct.unpack_from("<I", raw, 12)[0] != fixture.DATA_RECORD_HEADER_BYTES:
        raise SystemExit("M66 health record header size changed")
    generation = struct.unpack_from("<Q", raw, 16)[0]
    payload_bytes = struct.unpack_from("<I", raw, 24)[0]
    if generation != expected_generation or payload_bytes != 80:
        raise SystemExit("M66 health record generation or payload size changed")
    if struct.unpack_from("<I", raw, 28)[0] != fixture.DATA_RECORD_COMMITTED:
        raise SystemExit("M66 health record is not committed")
    if struct.unpack_from("<Q", raw, 48)[0] != ((~generation) & 0xFFFFFFFFFFFFFFFF):
        raise SystemExit("M66 health generation witness changed")
    if struct.unpack_from("<Q", raw, 56)[0] != fixture.DATA_RECORD_COMMIT_COOKIE:
        raise SystemExit("M66 health commit cookie changed")
    if raw[64:80] != fixture.DATA_FORMAT_EPOCH:
        raise SystemExit("M66 health format epoch changed")
    payload = raw[80:160]
    if fixture.crc32(payload) != struct.unpack_from("<I", raw, 32)[0]:
        raise SystemExit("M66 health payload CRC is invalid")
    if fnv1a64(payload) != struct.unpack_from("<Q", raw, 40)[0]:
        raise SystemExit("M66 health payload digest is invalid")
    if any(raw[160:fixture.DATA_RECORD_CRC_OFFSET]):
        raise SystemExit("M66 health record padding changed")
    if fixture.crc32(raw[:fixture.DATA_RECORD_CRC_OFFSET]) != struct.unpack_from(
        "<I", raw, fixture.DATA_RECORD_CRC_OFFSET
    )[0]:
        raise SystemExit("M66 health record CRC is invalid")
    if payload[:8] != b"BNDRHLT1":
        raise SystemExit("M66 health payload magic changed")
    if struct.unpack_from("<I", payload, 8)[0] != 1:
        raise SystemExit("M66 health payload version changed")
    if struct.unpack_from("<I", payload, 12)[0] != int(boot_open):
        raise SystemExit("M66 persisted boot-open state changed")
    if struct.unpack_from("<Q", payload, 16)[0] != generation:
        raise SystemExit("M66 health boot count does not match its generation")
    expected_contract = (
        16_384,
        0x0000000100000200,
        0x000000000A003E00,
        79,
        512,
        8,
        2,
    )
    observed_contract = (
        struct.unpack_from("<Q", payload, 24)[0],
        struct.unpack_from("<Q", payload, 32)[0],
        struct.unpack_from("<Q", payload, 40)[0],
        struct.unpack_from("<I", payload, 48)[0],
        struct.unpack_from("<I", payload, 52)[0],
        struct.unpack_from("<I", payload, 56)[0],
        struct.unpack_from("<I", payload, 60)[0],
    )
    if observed_contract != expected_contract:
        raise SystemExit("M66 persisted device contract changed")
    contract_digest = struct.unpack_from("<Q", payload, 64)[0]
    if contract_digest != fnv1a64(payload[24:64]):
        raise SystemExit("M66 health contract digest is invalid")
    witness = generation ^ contract_digest ^ 0xB24D63DA7A5EC001
    if struct.unpack_from("<Q", payload, 72)[0] != witness:
        raise SystemExit("M66 health state witness is invalid")


if sector(after, fixture.DATA_PARTITION_FIRST_LBA) != sector(
    before, fixture.DATA_PARTITION_FIRST_LBA
):
    raise SystemExit("M66 immutable data superblock changed")
slot_zero_lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[0]
slot_one_lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
decode_health_record(sector(after, slot_zero_lba), 4, False)
decode_health_record(sector(after, slot_one_lba), 3, True)
unused_start = (fixture.DATA_PARTITION_FIRST_LBA + 3) * sector_bytes
if after[unused_start:data_end] != before[unused_start:data_end] or any(
    after[unused_start:data_end]
):
    raise SystemExit("M66 changed an unused BNDROID_DATA sector")
changed_data_bytes = sum(
    left != right for left, right in zip(before[data_start:data_end], after[data_start:data_end])
)
changed_appdata_bytes = sum(
    left != right
    for left, right in zip(
        before[appdata_start:appdata_end], after[appdata_start:appdata_end]
    )
)
print(
    "RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 "
    "emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8 "
    "dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 "
    "order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 "
    "storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 "
    "spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 "
    "final_health_generation=4 final_health_slot=0 prior_health_generation=3 "
    "prior_health_slot=1 final_boot_open=0 prior_boot_open=1 "
    "qemu_backend_persistence=1 outside_data_appdata_unchanged=1 "
    "unused_data_unchanged=1 "
    f"appdata_changed=1 changed_data_bytes={changed_data_bytes} "
    f"changed_appdata_bytes={changed_appdata_bytes} "
    "emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 "
    "psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0"
)
PY
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "M66_BOOT_SEQUENCE phase=first qemu_self_exit=1"
  sed '/^qemu-system-aarch64:/d' "$FIRST_LOG"
  printf '%s\n' "M66_BOOT_SEQUENCE phase=second qemu_self_exit=1"
  sed '/^qemu-system-aarch64:/d' "$SECOND_LOG"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/resident-platform-shutdown-runtime.log"

RUN_SUCCEEDED=1
echo "M66 resident-platform shutdown two-boot self-exit test passed."
