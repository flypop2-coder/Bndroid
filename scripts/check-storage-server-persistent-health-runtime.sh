#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

HEALTH_PREFIX="STORAGE_DEVICE_HEALTH_OK"
DATA_PREFIX="DATA_PERSIST_OK"
BASE_PREFIX="STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_PREFIX="STORAGE_SERVER_OWNER_LIVENESS_OK"
QUARANTINE_PREFIX="STORAGE_SERVER_TERMINAL_QUARANTINE_OK"
RUNTIME_MARKER="STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250 terminal_quarantine=1 proof_deferral=1"
BOOT_MARKER="BOOT_OK: M63 persistent unclosed-boot hint and fresh kernel reprobe boundary verified"

run_evidence_parser() {
  python3 - "$@" <<'PY'
import sys
from pathlib import Path

HEALTH_PREFIX = "STORAGE_DEVICE_HEALTH_OK"
DATA_PREFIX = "DATA_PERSIST_OK"
BASE_PREFIX = "STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_PREFIX = "STORAGE_SERVER_OWNER_LIVENESS_OK"
QUARANTINE_PREFIX = "STORAGE_SERVER_TERMINAL_QUARANTINE_OK"
RUNTIME_MARKER = (
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 "
    "volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 "
    "repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 "
    "device_offline=1 owner_liveness=1 exit_grace_ms=250 "
    "terminal_quarantine=1 proof_deferral=1"
)
BOOT_MARKER = (
    "BOOT_OK: M63 persistent unclosed-boot hint and fresh kernel reprobe "
    "boundary verified"
)

COMMON_SCHEMA = [
    ("format", "1"),
    ("state_version", "1"),
    ("authority", "kernel-boot-probe"),
    ("record_role", "unclosed-boot-hint"),
    ("legacy_upgrade", None),
    ("persisted_contract", None),
    ("prior_boot_open", None),
    ("contract_changed", "0"),
    ("reprobe_required", None),
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
    ("initial_slot", None),
    ("committed_slot", None),
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

PHASE_VALUES = {
    "first": {
        "legacy_upgrade": "1",
        "persisted_contract": "0",
        "prior_boot_open": "0",
        "reprobe_required": "0",
        "initial_generation": "0",
        "committed_generation": "1",
        "initial_slot": "0",
        "committed_slot": "1",
    },
    "second": {
        "legacy_upgrade": "0",
        "persisted_contract": "1",
        "prior_boot_open": "1",
        "reprobe_required": "1",
        "initial_generation": "1",
        "committed_generation": "2",
        "initial_slot": "1",
        "committed_slot": "0",
    },
}

DATA_MARKERS = {
    "first": (
        "DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 "
        "slots=2 initial_generation=0 committed_generation=1 initial_slot=0 "
        "committed_slot=1 valid_slots=1 rejected_slots=1 reads=5 writes=1 "
        "flushes=1 write_completion=1 flush_completion=1 readback_verified=1 "
        "old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 "
        "rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 "
        "crash_consistency=0 qemu_reboot_proof=0"
    ),
    "second": (
        "DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 "
        "slots=2 initial_generation=1 committed_generation=2 initial_slot=1 "
        "committed_slot=0 valid_slots=2 rejected_slots=0 reads=5 writes=1 "
        "flushes=1 write_completion=1 flush_completion=1 readback_verified=1 "
        "old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 "
        "rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 "
        "crash_consistency=0 qemu_reboot_proof=0"
    ),
}


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def marker_once(lines: list[str], prefix: str) -> str:
    matches = [line for line in lines if line == prefix or line.startswith(prefix + " ")]
    reject(len(matches) != 1, f"{prefix} count is not exactly one")
    return matches[0]


def parse_fields(line: str, prefix: str) -> dict[str, str]:
    tokens = line.split(" ")
    reject(not tokens or tokens[0] != prefix, f"{prefix} prefix changed")
    values: dict[str, str] = {}
    for token in tokens[1:]:
        reject("=" not in token, f"{prefix} contains a field without '='")
        name, value = token.split("=", 1)
        reject(not name or not value or name in values, f"{prefix} field is invalid")
        values[name] = value
    return values


def health_marker(phase: str) -> str:
    phase_values = PHASE_VALUES[phase]
    fields = []
    for name, expected in COMMON_SCHEMA:
        value = expected if expected is not None else phase_values[name]
        fields.append(f"{name}={value}")
    return f"{HEALTH_PREFIX} {' '.join(fields)}"


def require_fields(
    line: str, prefix: str, expected: dict[str, str], label: str
) -> None:
    values = parse_fields(line, prefix)
    for name, value in expected.items():
        reject(values.get(name) != value, f"{label} field {name} is not {value}")


def validate_text(text: str, phase: str) -> None:
    reject(phase not in PHASE_VALUES, "unknown M63 boot phase")
    lines = text.splitlines()
    health = marker_once(lines, HEALTH_PREFIX)
    reject(health != health_marker(phase), "M63 health marker changed exact structure")
    data = marker_once(lines, DATA_PREFIX)
    reject(data != DATA_MARKERS[phase], "M63 DATA_PERSIST marker changed")

    base = marker_once(lines, BASE_PREFIX)
    require_fields(
        base,
        BASE_PREFIX,
        {
            "device_offline": "1",
            "offline_transitions": "1",
            "simulated_permanent": "1",
            "hardware_claim": "0",
            "powercut_claim": "0",
            "general_runtime": "0",
            "invariant_errors": "0",
        },
        "M62 parent",
    )
    owner = marker_once(lines, OWNER_PREFIX)
    require_fields(
        owner,
        OWNER_PREFIX,
        {
            "authority": "kernel-fatal-completion",
            "el0_arm_controls": "0",
            "el0_renew_controls": "0",
            "simulated_fault": "1",
            "hardware_claim": "0",
            "powercut_claim": "0",
            "general_runtime": "0",
            "invariant_errors": "0",
        },
        "M61 parent",
    )
    quarantine = marker_once(lines, QUARANTINE_PREFIX)
    require_fields(
        quarantine,
        QUARANTINE_PREFIX,
        {
            "authority": "kernel-offline-transition",
            "proof_arms": "1",
            "proof_deferrals": "1",
            "proof_publications": "1",
            "policy_attempt_delta": "0",
            "policy_ticket_consumed": "0",
            "el0_controls": "0",
            "simulated_fault": "1",
            "hardware_claim": "0",
            "powercut_claim": "0",
            "general_runtime": "0",
            "invariant_errors": "0",
        },
        "M62 quarantine parent",
    )
    reject(lines.count(RUNTIME_MARKER) != 1, "M63 parent runtime marker changed")
    reject(lines.count(BOOT_MARKER) != 1, "M63 BOOT marker changed")
    reject(
        any(line.startswith("BOOT_OK: M62 ") for line in lines),
        "M63 emitted the stale M62 BOOT marker",
    )


def expect_rejected(text: str, phase: str) -> None:
    try:
        validate_text(text, phase)
    except EvidenceError:
        return
    raise AssertionError("negative M63 parser fixture was accepted")


def canonical_text(phase: str) -> str:
    return "\n".join(
        [
            health_marker(phase),
            DATA_MARKERS[phase],
            (
                "STORAGE_SERVER_FAULT_POLICY_OK device_offline=1 "
                "offline_transitions=1 simulated_permanent=1 hardware_claim=0 "
                "powercut_claim=0 general_runtime=0 invariant_errors=0"
            ),
            (
                "STORAGE_SERVER_OWNER_LIVENESS_OK authority=kernel-fatal-completion "
                "el0_arm_controls=0 el0_renew_controls=0 simulated_fault=1 "
                "hardware_claim=0 powercut_claim=0 general_runtime=0 "
                "invariant_errors=0"
            ),
            (
                "STORAGE_SERVER_TERMINAL_QUARANTINE_OK "
                "authority=kernel-offline-transition proof_arms=1 "
                "proof_deferrals=1 proof_publications=1 policy_attempt_delta=0 "
                "policy_ticket_consumed=0 el0_controls=0 simulated_fault=1 "
                "hardware_claim=0 powercut_claim=0 general_runtime=0 "
                "invariant_errors=0"
            ),
            RUNTIME_MARKER,
            BOOT_MARKER,
        ]
    ) + "\n"


def self_test() -> None:
    first = canonical_text("first")
    second = canonical_text("second")
    validate_text(first, "first")
    validate_text(second, "second")
    cases = 0

    canonical_health = health_marker("first")
    for name, _ in COMMON_SCHEMA:
        token = next(
            field for field in canonical_health.split()[1:] if field.startswith(name + "=")
        )
        bad = canonical_health.replace(token, f"{name}=__bad__", 1)
        expect_rejected(first.replace(canonical_health, bad, 1), "first")
        cases += 1

    for mutation in ("remove", "extra", "duplicate", "prefix"):
        lines = first.splitlines()
        if mutation == "remove":
            lines[0] = " ".join(lines[0].split()[:-1])
        elif mutation == "extra":
            lines[0] += " extra=1"
        elif mutation == "duplicate":
            lines.insert(0, lines[0])
        else:
            lines[0] = "STALE_" + lines[0]
        expect_rejected("\n".join(lines) + "\n", "first")
        cases += 1

    for line_index in range(1, 7):
        lines = first.splitlines()
        lines.pop(line_index)
        expect_rejected("\n".join(lines) + "\n", "first")
        cases += 1
        lines = first.splitlines()
        lines.insert(line_index, lines[line_index])
        expect_rejected("\n".join(lines) + "\n", "first")
        cases += 1

    expect_rejected(second, "first")
    cases += 1
    expect_rejected(first, "second")
    cases += 1
    print(
        "STORAGE_SERVER_PERSISTENT_HEALTH_PARSER_OK "
        f"negative_cases={cases} health_fields={len(COMMON_SCHEMA)} phases=2"
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

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M63 persistent device health." >&2
    exit 1
  fi
done
if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M63 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
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

FEATURE="storage-server-persistent-health-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_PERSISTENT_HEALTH_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-persistent-health-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m63"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m63.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
FIRST_SERIAL="$TMP_DIR/boot-1.serial.log"
FIRST_LOG="$TMP_DIR/boot-1.log"
SECOND_SERIAL="$TMP_DIR/boot-2.serial.log"
SECOND_LOG="$TMP_DIR/boot-2.log"
QEMU_PID=""
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
    echo "M63 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  local log_file="$1"
  [[ -f "$log_file" ]] && tail -n 360 "$log_file" >&2 || true
}

reject_failure() {
  local log_file="$1"
  [[ -f "$log_file" ]] || return
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$log_file" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M6[0-3]_[A-Z0-9_]*TIMEOUT|^BOOT_OK: M(5[5-9]|60|61|62) ' "$log_file"; then
    show_failure "$log_file"
    echo "M63 emitted a panic, failure, timeout, or stale M55-M62 BOOT marker." >&2
    exit 1
  fi
  if grep '^STORAGE_SERVER_RUNTIME_OK' "$log_file" | grep -Fvx "$RUNTIME_MARKER" >/dev/null; then
    show_failure "$log_file"
    echo "M63 published stale or malformed parent runtime evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$log_file" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure "$log_file"
    echo "M63 published a stale or unrelated BOOT_OK marker." >&2
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
  local base_count
  local owner_count
  local quarantine_count
  local runtime_count
  local boot_count

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

  deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$serial_log" >"$normalized_log"
    reject_failure "$normalized_log"
    health_count="$(grep -Ec "^${HEALTH_PREFIX}( |$)" "$normalized_log" || true)"
    data_count="$(grep -Ec "^${DATA_PREFIX}( |$)" "$normalized_log" || true)"
    base_count="$(grep -Ec "^${BASE_PREFIX}( |$)" "$normalized_log" || true)"
    owner_count="$(grep -Ec "^${OWNER_PREFIX}( |$)" "$normalized_log" || true)"
    quarantine_count="$(grep -Ec "^${QUARANTINE_PREFIX}( |$)" "$normalized_log" || true)"
    runtime_count="$(grep -Fxc "$RUNTIME_MARKER" "$normalized_log" || true)"
    boot_count="$(grep -Fxc "$BOOT_MARKER" "$normalized_log" || true)"
    if [[ "$health_count" == 1 && "$data_count" == 1 && "$base_count" == 1 \
      && "$owner_count" == 1 && "$quarantine_count" == 1 \
      && "$runtime_count" == 1 && "$boot_count" == 1 ]]; then
      kill "$QEMU_PID" 2>/dev/null || true
      wait "$QEMU_PID" 2>/dev/null || true
      QEMU_PID=""
      tr -d '\r' <"$serial_log" >"$normalized_log"
      reject_failure "$normalized_log"
      run_evidence_parser "$phase" "$normalized_log"
      return 0
    fi
    if ((health_count > 1 || data_count > 1 || base_count > 1 || owner_count > 1 \
      || quarantine_count > 1 || runtime_count > 1 || boot_count > 1)); then
      show_failure "$normalized_log"
      echo "M63 published duplicate health, persistence, parent, runtime, or BOOT evidence." >&2
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
      show_failure "$normalized_log"
      echo "QEMU exited before M63 $phase-boot evidence (status $qemu_status)." >&2
      exit 1
    fi
    sleep 0.05
  done

  tr -d '\r' <"$serial_log" >"$normalized_log"
  reject_failure "$normalized_log"
  show_failure "$normalized_log"
  echo "Timed out waiting for exact M63 $phase-boot evidence." >&2
  exit 1
}

run_evidence_parser --self-test >/dev/null
python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_KERNEL_FEATURES="$FEATURE" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M63 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

run_persistent_boot first "$FIRST_SERIAL" "$FIRST_LOG"
run_persistent_boot second "$SECOND_SERIAL" "$SECOND_LOG"

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
    raise SystemExit("M63 runtime disk lost its exact writable 8 MiB geometry")

data_start = fixture.DATA_PARTITION_FIRST_LBA * sector_bytes
data_end = (fixture.DATA_PARTITION_LAST_LBA + 1) * sector_bytes
appdata_start = fixture.APPDATA_PARTITION_FIRST_LBA * sector_bytes
appdata_end = (fixture.APPDATA_PARTITION_LAST_LBA + 1) * sector_bytes
if before[:data_start] != after[:data_start] or before[appdata_end:] != after[appdata_end:]:
    raise SystemExit("M63 QEMU changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
if before[appdata_start:appdata_end] == after[appdata_start:appdata_end]:
    raise SystemExit("M63 parent runtime did not persist its AppData campaign")


def sector(image: bytes, lba: int) -> bytes:
    start = lba * sector_bytes
    return image[start : start + sector_bytes]


def fnv1a64(data: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x00000100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def decode_health_record(raw: bytes, expected_generation: int) -> None:
    if raw[:8] != fixture.DATA_RECORD_MAGIC:
        raise SystemExit("M63 record magic changed")
    if struct.unpack_from("<I", raw, 8)[0] != fixture.DATA_FORMAT_VERSION:
        raise SystemExit("M63 record version changed")
    if struct.unpack_from("<I", raw, 12)[0] != fixture.DATA_RECORD_HEADER_BYTES:
        raise SystemExit("M63 record header size changed")
    generation = struct.unpack_from("<Q", raw, 16)[0]
    payload_bytes = struct.unpack_from("<I", raw, 24)[0]
    if generation != expected_generation or payload_bytes != 80:
        raise SystemExit("M63 record generation or payload size changed")
    if struct.unpack_from("<I", raw, 28)[0] != fixture.DATA_RECORD_COMMITTED:
        raise SystemExit("M63 record is not committed")
    if struct.unpack_from("<Q", raw, 48)[0] != ((~generation) & 0xFFFFFFFFFFFFFFFF):
        raise SystemExit("M63 record generation witness changed")
    if struct.unpack_from("<Q", raw, 56)[0] != fixture.DATA_RECORD_COMMIT_COOKIE:
        raise SystemExit("M63 record commit cookie changed")
    if raw[64:80] != fixture.DATA_FORMAT_EPOCH:
        raise SystemExit("M63 record format epoch changed")
    payload = raw[80:160]
    if fixture.crc32(payload) != struct.unpack_from("<I", raw, 32)[0]:
        raise SystemExit("M63 payload CRC is invalid")
    if fnv1a64(payload) != struct.unpack_from("<Q", raw, 40)[0]:
        raise SystemExit("M63 payload digest is invalid")
    if any(raw[160:fixture.DATA_RECORD_CRC_OFFSET]):
        raise SystemExit("M63 record padding changed")
    if fixture.crc32(raw[:fixture.DATA_RECORD_CRC_OFFSET]) != struct.unpack_from(
        "<I", raw, fixture.DATA_RECORD_CRC_OFFSET
    )[0]:
        raise SystemExit("M63 record CRC is invalid")

    if payload[:8] != b"BNDRHLT1":
        raise SystemExit("M63 health magic changed")
    if struct.unpack_from("<I", payload, 8)[0] != 1:
        raise SystemExit("M63 health version changed")
    if struct.unpack_from("<I", payload, 12)[0] != 1:
        raise SystemExit("M63 persisted state is not exactly boot-open")
    if struct.unpack_from("<Q", payload, 16)[0] != generation:
        raise SystemExit("M63 health boot count does not match its generation")
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
        raise SystemExit("M63 persisted device contract changed")
    contract_digest = struct.unpack_from("<Q", payload, 64)[0]
    if contract_digest != fnv1a64(payload[24:64]):
        raise SystemExit("M63 health contract digest is invalid")
    witness = generation ^ contract_digest ^ 0xB24D63DA7A5EC001
    if struct.unpack_from("<Q", payload, 72)[0] != witness:
        raise SystemExit("M63 health state witness is invalid")


if sector(after, fixture.DATA_PARTITION_FIRST_LBA) != sector(
    before, fixture.DATA_PARTITION_FIRST_LBA
):
    raise SystemExit("M63 immutable data superblock changed")
slot_zero_lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[0]
slot_one_lba = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[1]
decode_health_record(sector(after, slot_zero_lba), 2)
decode_health_record(sector(after, slot_one_lba), 1)
unused_start = (fixture.DATA_PARTITION_FIRST_LBA + 3) * sector_bytes
if after[unused_start:data_end] != before[unused_start:data_end] or any(
    after[unused_start:data_end]
):
    raise SystemExit("M63 changed an unused BNDROID_DATA sector")
changed_data_bytes = sum(
    left != right for left, right in zip(before[data_start:data_end], after[data_start:data_end])
)
print(
    "STORAGE_DEVICE_HEALTH_REBOOT_OK boots=2 legacy_upgrades=1 "
    "unclosed_hints=1 reprobe_required=1 reprobe_verified=2 contract_changes=0 "
    "final_generation=2 final_slot=0 prior_slot=1 qemu_backend_persistence=1 "
    "outside_data_appdata_unchanged=1 unused_data_unchanged=1 "
    f"changed_data_bytes={changed_data_bytes} offline_persisted=0 "
    "offline_from_record=0 el0_controls=0 hardware_identity_claim=0 "
    "hotplug_claim=0 powercut_claim=0 tamper_resistance_claim=0 general_runtime=0"
)
PY
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "M63_BOOT_SEQUENCE phase=first"
  cat "$FIRST_LOG"
  printf '%s\n' "M63_BOOT_SEQUENCE phase=second"
  cat "$SECOND_LOG"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/storage-server-persistent-health-runtime.log"

RUN_SUCCEEDED=1
echo "M63 persistent device-health reboot self-test passed."
