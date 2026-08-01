#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

BASE_MARKER_PREFIX="STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_MARKER_PREFIX="STORAGE_SERVER_OWNER_LIVENESS_OK"
RUNTIME_MARKER="STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250"
BOOT_MARKER="BOOT_OK: M61 fault-latched StorageServer owner retirement and recovery convergence verified"

# Runtime validation and the negative fixtures execute this exact parser body.
# Marker schemas are ordered deliberately: missing, extra, duplicate, or moved
# fields are evidence-contract failures even when the individual values look sane.
run_evidence_parser() {
  python3 - "$@" <<'PY'
import re
import sys
from pathlib import Path
from typing import Optional

BASE_PREFIX = "STORAGE_SERVER_FAULT_POLICY_OK"
OWNER_PREFIX = "STORAGE_SERVER_OWNER_LIVENESS_OK"
BOOT_MARKER = (
    "BOOT_OK: M61 fault-latched StorageServer owner retirement and recovery "
    "convergence verified"
)

# None means that the field is runtime-dependent and is checked below against
# an explicit invariant. Every non-None value is byte-for-byte exact.
BASE_SCHEMA = [
    ("recoverable_cases", "6"),
    ("permanent_cases", "1"),
    ("fault_order", "WRFWRFR"),
    ("read_requires_reset", "3"),
    ("mutation_outcome_unknown", "4"),
    ("transient_control_sequences", "6"),
    ("kernel_permanent_owner_requests", "1"),
    ("kernel_permanent_fault_arms", "1"),
    ("kernel_permanent_reads", "1"),
    ("el0_permanent_fault_arm_controls", "0"),
    ("permanent_epoch", "7"),
    ("permanent_authority", "kernel-prearmed"),
    ("injected_reads", "3"),
    ("injected_writes", "2"),
    ("injected_flushes", "2"),
    ("owner_exits", "7"),
    ("offline_probe_exits", "1"),
    ("broker_releases", "7"),
    ("broker_abandoned", "7"),
    ("recovery_attempts", "9"),
    ("recovery_commits", "6"),
    ("recovery_failures", "3"),
    ("recovery_rollbacks", "1"),
    ("fail_closed_retries", "1"),
    ("physical_successes", "7"),
    ("physical_failures", "2"),
    ("attempt_failures", "4"),
    ("probation_io_failures", "1"),
    ("attempt_cap", "3"),
    ("backoffs", "3"),
    ("backoffs_completed", "3"),
    ("backoff_base_ticks", "2"),
    ("backoff_multiplier", "2"),
    ("backoff_ticks", "8"),
    ("probation", "7/5/2"),
    ("healthy_transitions", "5"),
    ("device_offline", "1"),
    ("offline_transitions", "1"),
    ("offline_denials", "1"),
    ("early_rejections", None),
    ("early_physical_starts", "0"),
    ("offline_rejections", "0"),
    ("stale_ticket_rejections", "0"),
    ("invalid_transition_rejections", "0"),
    ("generation_exhaustions", "0"),
    ("persistent_fault_armed", "1"),
    ("persistent_fault_hits", "2"),
    ("driver_timeouts", "7"),
    ("driver_resets", "10"),
    ("async_starts", "9"),
    ("physical_completions", "7"),
    ("async_physical_failures", "2"),
    ("async_steps", None),
    ("recovery_yields", None),
    ("timer_progress_windows", "9"),
    ("worker_progress_windows", "9"),
    ("el0_progress_windows", "9"),
    ("backoff_timer_progress_windows", "3"),
    ("backoff_worker_progress_windows", "3"),
    ("acquire_waits", None),
    ("acquire_dispatch_changes", None),
    ("terminal_quarantine_starts", "0"),
    ("terminal_quarantine_completions", "0"),
    ("terminal_quarantine_physical_errors", "0"),
    ("terminal_irq_rollbacks", "1"),
    ("terminal_dma_verifications", "1"),
    ("terminal_driver_state", "2"),
    ("terminal_transport_status", "0"),
    ("terminal_in_flight", "0"),
    ("masked_poll_iterations", "0"),
    ("max_step_masked_ticks", None),
    ("max_control_masked_ticks", None),
    ("timer_period_ticks", None),
    ("long_daif_masks", "0"),
    ("offline_quiet_ticks", None),
    ("post_offline_attempt_delta", "0"),
    ("post_offline_reset_delta", "0"),
    ("post_offline_submission_delta", "0"),
    ("final_epoch", "0"),
    ("next_epoch", "8"),
    ("final_generation", None),
    ("final_irq_armed", "0"),
    ("final_irq_failed", "1"),
    ("final_recovery_required", "1"),
    ("final_admission_open", "0"),
    ("final_broker_state", "4"),
    ("final_broker_bound", "0"),
    ("final_broker_pending", "0"),
    ("requests", None),
    ("completions", None),
    ("simulated_permanent", "1"),
    ("hardware_claim", "0"),
    ("arbitrary_soak_claim", "0"),
    ("powercut_claim", "0"),
    ("concurrency_claim", "0"),
    ("general_runtime", "0"),
    ("invariant_errors", "0"),
]

OWNER_SCHEMA = [
    ("authority", "kernel-fatal-completion"),
    ("ticket", "pid-epoch-lease-generation"),
    ("deadline", "physical-counter"),
    ("grace_ms", "250"),
    ("grace_counter_units", None),
    ("arms", "7"),
    ("cooperative_retirements", "6"),
    ("deadlines_expired", "1"),
    ("early_expirations", "0"),
    ("termination_requests", "1"),
    ("already_terminal", "0"),
    ("forced_retirements", "1"),
    ("forced_reaps", "1"),
    ("terminal_race_retirements", "0"),
    ("stalled_epoch", "6"),
    ("stalled_operation", "flush"),
    ("stalled_completion", "outcome-unknown"),
    ("live_volume_close", "denied"),
    ("target_wait", "single"),
    ("object_wait_abandoned", "1"),
    ("el0_process_terminate_calls", "0"),
    ("terminated_exited", "7"),
    ("terminated_killed", "1"),
    ("recovery_after_forced_retirement", "1"),
    ("replacement_epoch", "7"),
    ("final_phase", "0"),
    ("final_owner_pid", "0"),
    ("final_broker_epoch", "0"),
    ("next_lease_generation", "8"),
    ("el0_arm_controls", "0"),
    ("el0_renew_controls", "0"),
    ("simulated_fault", "1"),
    ("hardware_claim", "0"),
    ("smp_claim", "0"),
    ("arbitrary_soak_claim", "0"),
    ("powercut_claim", "0"),
    ("concurrency_claim", "0"),
    ("general_runtime", "0"),
    ("invariant_errors", "0"),
]

RUNTIME_SCHEMA = [
    ("abi", "25"),
    ("sector_bytes", "512"),
    ("batch_max", "8"),
    ("volume_sectors", "1920"),
    ("fail_stop", "1"),
    ("kernel_reset_authority", "1"),
    ("repeated_recovery", "1"),
    ("async_recovery", "1"),
    ("fault_policy", "1"),
    ("attempt_cap", "3"),
    ("device_offline", "1"),
    ("owner_liveness", "1"),
    ("exit_grace_ms", "250"),
]

DYNAMIC_CANONICAL = {
    "early_rejections": "1",
    "async_steps": "35",
    "recovery_yields": "26",
    "acquire_waits": "25",
    "acquire_dispatch_changes": "25",
    "max_step_masked_ticks": "20",
    "max_control_masked_ticks": "30",
    "timer_period_ticks": "100",
    "offline_quiet_ticks": "16",
    "final_generation": "1",
    "requests": "120",
    "completions": "113",
    "grace_counter_units": "1000",
}

DYNAMIC_INVALID = {
    "early_rejections": "0",
    "async_steps": "34",
    "recovery_yields": "25",
    "acquire_waits": "24",
    "acquire_dispatch_changes": "24",
    "max_step_masked_ticks": "0",
    "max_control_masked_ticks": "0",
    "timer_period_ticks": "0",
    "offline_quiet_ticks": "15",
    "final_generation": "0",
    "requests": "119",
    "completions": "112",
    "grace_counter_units": "0",
}


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def marker_line(prefix: str, schema: list[tuple[str, Optional[str]]]) -> str:
    fields = []
    for name, expected in schema:
        value = expected if expected is not None else DYNAMIC_CANONICAL[name]
        fields.append(f"{name}={value}")
    return f"{prefix} {' '.join(fields)}"


RUNTIME_MARKER = marker_line("STORAGE_SERVER_RUNTIME_OK", RUNTIME_SCHEMA)


def parse_marker(
    line: str, prefix: str, schema: list[tuple[str, Optional[str]]], label: str
) -> dict[str, str]:
    tokens = line.split(" ")
    reject(
        len(tokens) != len(schema) + 1 or tokens[0] != prefix,
        f"M61 {label} marker changed its exact ordered structure",
    )
    values: dict[str, str] = {}
    for token, (wanted_name, wanted_value) in zip(tokens[1:], schema):
        reject("=" not in token, f"M61 {label} field {wanted_name} lost its value")
        name, value = token.split("=", 1)
        reject(
            name != wanted_name or not value,
            f"M61 {label} field order or spelling changed at {wanted_name}",
        )
        reject(
            wanted_value is not None and value != wanted_value,
            f"M61 {label} {wanted_name} was {value}, expected {wanted_value}",
        )
        values[name] = value
    return values


def number(values: dict[str, str], field: str, label: str) -> int:
    value = values[field]
    reject(
        re.fullmatch(r"[0-9]+", value) is None,
        f"M61 {label} {field} was not an unsigned decimal counter",
    )
    return int(value)


def validate(lines: list[str]) -> None:
    stale_prefixes = (
        "STORAGE_SERVER_RECOVERY_OK",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
    )
    reject(
        any(
            any(line == prefix or line.startswith(prefix + " ") for prefix in stale_prefixes)
            for line in lines
        ),
        "M61 checker observed stale pre-M60 storage recovery evidence",
    )

    base_lines = [line for line in lines if line.startswith(BASE_PREFIX)]
    owner_lines = [line for line in lines if line.startswith(OWNER_PREFIX)]
    runtime_lines = [line for line in lines if line.startswith("STORAGE_SERVER_RUNTIME_OK")]
    reject(len(base_lines) != 1, "M61 base fault-policy evidence was not unique")
    reject(len(owner_lines) != 1, "M61 owner-liveness evidence was not unique")
    reject(runtime_lines != [RUNTIME_MARKER], "M61 runtime evidence was not exact and unique")
    reject(lines.count(BOOT_MARKER) != 1, "M61 BOOT_OK evidence was not exact and unique")
    unrelated_boots = [line for line in lines if line.startswith("BOOT_OK:") and line != BOOT_MARKER]
    reject(bool(unrelated_boots), "M61 log contained a stale or unrelated BOOT_OK marker")

    base = base_lines[0]
    owner = owner_lines[0]
    reject(
        not (
            lines.index(base)
            < lines.index(owner)
            < lines.index(RUNTIME_MARKER)
            < lines.index(BOOT_MARKER)
        ),
        "M61 base, owner, runtime, and BOOT_OK evidence was out of order",
    )

    base_values = parse_marker(base, BASE_PREFIX, BASE_SCHEMA, "base")
    owner_values = parse_marker(owner, OWNER_PREFIX, OWNER_SCHEMA, "owner")
    parse_marker(RUNTIME_MARKER, "STORAGE_SERVER_RUNTIME_OK", RUNTIME_SCHEMA, "runtime")

    base_numbers = {
        name: number(base_values, name, "base")
        for name, _ in BASE_SCHEMA
        if name != "fault_order" and name != "permanent_authority" and name != "probation"
    }
    owner_numbers = {
        name: number(owner_values, name, "owner")
        for name, _ in OWNER_SCHEMA
        if name
        not in {
            "authority",
            "ticket",
            "deadline",
            "stalled_operation",
            "stalled_completion",
            "live_volume_close",
            "target_wait",
        }
    }

    reject(base_numbers["early_rejections"] < 1, "M61 observed no early owner rejection")
    reject(
        base_numbers["recovery_attempts"]
        != base_numbers["physical_successes"] + base_numbers["physical_failures"],
        "M61 attempt ledger did not split into physical successes and failures",
    )
    reject(
        base_numbers["physical_successes"]
        != base_numbers["recovery_commits"] + base_numbers["recovery_rollbacks"],
        "M61 physical-success ledger did not split into commits and rollback",
    )
    reject(
        base_numbers["recovery_failures"]
        != base_numbers["recovery_rollbacks"] + base_numbers["physical_failures"],
        "M61 coordinator failures did not isolate rollback and physical failures",
    )
    reject(
        base_numbers["attempt_failures"]
        != base_numbers["recovery_failures"] + base_numbers["probation_io_failures"],
        "M61 policy failures did not include coordinator and probation failures",
    )
    probation_failures = int(base_values["probation"].split("/")[2])
    reject(
        probation_failures
        != base_numbers["recovery_rollbacks"] + base_numbers["probation_io_failures"],
        "M61 probation failures did not isolate rollback and probation I/O failure",
    )
    reject(
        base_numbers["driver_resets"]
        != base_numbers["driver_timeouts"]
        + base_numbers["recovery_rollbacks"]
        + base_numbers["persistent_fault_hits"],
        "M61 reset ledger did not isolate timeout, rollback, and persistent hits",
    )
    reject(
        base_numbers["async_starts"] != base_numbers["recovery_attempts"]
        or base_numbers["physical_completions"] != base_numbers["physical_successes"]
        or base_numbers["async_physical_failures"] != base_numbers["physical_failures"],
        "M61 coordinator and physical-attempt ledgers diverged",
    )
    reject(
        base_numbers["async_steps"]
        != base_numbers["recovery_yields"] + base_numbers["recovery_attempts"],
        "M61 step/yield ledger did not isolate nine terminal physical steps",
    )
    reject(
        base_numbers["recovery_yields"] < 26,
        "M61 cooperative recovery exposed fewer than 26 yields",
    )
    reject(
        base_numbers["acquire_waits"] not in {25, 26},
        "M61 authenticated EL0 acquire waits left the measured 25/26 phase set",
    )
    reject(
        base_numbers["acquire_dispatch_changes"] not in {25, 26},
        "M61 acquire retries left the measured 25/26 dispatch phase set",
    )
    reject(
        base_numbers["acquire_dispatch_changes"] != base_numbers["acquire_waits"],
        "M61 acquire retries did not each span a distinct timer dispatch",
    )
    reject(
        base_numbers["offline_quiet_ticks"] < 16,
        "M61 Offline state was not observed for at least 16 ticks",
    )

    period = base_numbers["timer_period_ticks"]
    reject(period == 0, "M61 timer period was zero")
    reject(
        base_numbers["max_step_masked_ticks"] == 0
        or base_numbers["max_step_masked_ticks"] >= period,
        "M61 cooperative driver held DAIF across a full timer period",
    )
    reject(
        base_numbers["max_control_masked_ticks"] == 0
        or base_numbers["max_control_masked_ticks"] >= period,
        "M61 recovery control held DAIF across a full timer period",
    )
    reject(base_numbers["final_generation"] == 0, "M61 lost its persisted generation proof")
    reject(
        base_numbers["requests"] != base_numbers["completions"] + 7,
        "M61 request ledger did not isolate exactly seven timed-out requests",
    )
    reject(
        owner_numbers["grace_counter_units"] == 0,
        "M61 owner retirement grace converted to zero physical-counter units",
    )


def canonical_lines() -> list[str]:
    return [
        marker_line(BASE_PREFIX, BASE_SCHEMA),
        marker_line(OWNER_PREFIX, OWNER_SCHEMA),
        RUNTIME_MARKER,
        BOOT_MARKER,
    ]


def mutate_field(
    lines: list[str], marker_index: int, field_index: int, value: str
) -> list[str]:
    changed = list(lines)
    tokens = changed[marker_index].split(" ")
    name = tokens[field_index + 1].split("=", 1)[0]
    tokens[field_index + 1] = f"{name}={value}"
    changed[marker_index] = " ".join(tokens)
    return changed


def changed_exact_value(value: str) -> str:
    if re.fullmatch(r"[0-9]+", value):
        return str(int(value) + 1)
    return value + "-mutated"


def structural_mutation(
    lines: list[str], marker_index: int, operation: str
) -> list[str]:
    changed = list(lines)
    tokens = changed[marker_index].split(" ")
    if operation == "missing":
        tokens.pop()
    elif operation == "extra":
        tokens.append("unexpected=1")
    elif operation == "reordered":
        tokens[1], tokens[2] = tokens[2], tokens[1]
    else:
        raise AssertionError(f"unknown structural mutation {operation}")
    changed[marker_index] = " ".join(tokens)
    return changed


def self_test() -> None:
    canonical = canonical_lines()
    validate(canonical)
    negative: list[tuple[str, list[str]]] = []

    for label, marker_index, schema in (
        ("base", 0, BASE_SCHEMA),
        ("owner", 1, OWNER_SCHEMA),
        ("runtime", 2, RUNTIME_SCHEMA),
    ):
        for field_index, (name, expected) in enumerate(schema):
            value = DYNAMIC_INVALID[name] if expected is None else changed_exact_value(expected)
            negative.append(
                (f"{label}_field_{name}", mutate_field(canonical, marker_index, field_index, value))
            )
        for operation in ("missing", "extra", "reordered"):
            negative.append(
                (
                    f"{label}_{operation}",
                    structural_mutation(canonical, marker_index, operation),
                )
            )

    # Exercise both upper masked-time bounds independently in addition to the
    # per-field zero cases above.
    base_index = {name: index for index, (name, _) in enumerate(BASE_SCHEMA)}
    negative.extend(
        [
            (
                "step_mask_equals_period",
                mutate_field(canonical, 0, base_index["max_step_masked_ticks"], "100"),
            ),
            (
                "control_mask_equals_period",
                mutate_field(canonical, 0, base_index["max_control_masked_ticks"], "100"),
            ),
            (
                "acquire_waits_above_phase_set",
                mutate_field(canonical, 0, base_index["acquire_waits"], "27"),
            ),
            (
                "acquire_changes_above_phase_set",
                mutate_field(canonical, 0, base_index["acquire_dispatch_changes"], "27"),
            ),
            (
                "acquire_waits_exceed_changes",
                mutate_field(canonical, 0, base_index["acquire_waits"], "26"),
            ),
            (
                "acquire_changes_exceed_waits",
                mutate_field(canonical, 0, base_index["acquire_dispatch_changes"], "26"),
            ),
            ("duplicate_base", [canonical[0], *canonical]),
            ("duplicate_owner", [canonical[0], canonical[1], *canonical[1:]]),
            ("duplicate_runtime", [*canonical[:3], canonical[2], canonical[3]]),
            ("duplicate_boot", [*canonical, canonical[3]]),
            ("owner_before_base", [canonical[1], canonical[0], canonical[2], canonical[3]]),
            ("runtime_before_owner", [canonical[0], canonical[2], canonical[1], canonical[3]]),
            ("boot_before_runtime", [canonical[0], canonical[1], canonical[3], canonical[2]]),
            ("changed_boot", [*canonical[:3], BOOT_MARKER + " changed"]),
            ("unrelated_boot", [*canonical, "BOOT_OK: unrelated system evidence"]),
        ]
    )

    for index, prefix in enumerate(
        (
            "STORAGE_SERVER_RECOVERY_OK",
            "STORAGE_SERVER_REPEATED_RECOVERY_OK",
            "STORAGE_SERVER_ASYNC_RECOVERY_OK",
            "APPDATA_ASYNC_RECOVERY_OK",
            "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        ),
        55,
    ):
        negative.append((f"stale_storage_{index}", [*canonical, f"{prefix} stale=1"]))
    for milestone in range(55, 61):
        negative.append((f"stale_boot_m{milestone}", [*canonical, f"BOOT_OK: M{milestone} stale"]))

    for name, lines in negative:
        try:
            validate(lines)
        except EvidenceError:
            continue
        raise SystemExit(f"M61 parser negative self-test accepted {name}")

    print(
        "STORAGE_SERVER_OWNER_LIVENESS_PARSER_OK "
        f"negative_cases={len(negative)} base_fields={len(BASE_SCHEMA)} "
        f"owner_fields={len(OWNER_SCHEMA)} runtime_fields={len(RUNTIME_SCHEMA)}"
    )


if len(sys.argv) < 2:
    raise SystemExit("M61 evidence parser mode is missing")
mode = sys.argv[1]
if mode == "--self-test":
    if len(sys.argv) != 2:
        raise SystemExit("M61 parser self-test takes no path")
    self_test()
elif mode == "--validate":
    if len(sys.argv) != 3:
        raise SystemExit("M61 parser validation requires exactly one log path")
    validate(Path(sys.argv[2]).read_text(encoding="utf-8").splitlines())
else:
    raise SystemExit(f"unknown M61 evidence parser mode: {mode}")
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  if [[ "$#" -ne 1 ]]; then
    echo "--parser-self-test takes no additional arguments." >&2
    exit 2
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found; cannot run the M61 parser self-test." >&2
    exit 1
  fi
  run_evidence_parser --self-test
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  echo "usage: $0 [--parser-self-test]" >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill mkdir rm sleep tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M61 StorageServer owner-liveness runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M61 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
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

FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime,storage-server-async-recovery-runtime,storage-server-fault-policy-runtime,storage-server-owner-liveness-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_OWNER_LIVENESS_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-owner-liveness-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m61"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m61.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
SERIAL_LOG="$TMP_DIR/qemu.serial.log"
NORMALIZED_LOG="$TMP_DIR/qemu.log"
QEMU_PID=""
RUN_SUCCEEDED=0

cleanup() {
  local exit_status
  exit_status="$1"
  trap - EXIT INT TERM
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M61 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 340 "$NORMALIZED_LOG" >&2 || true
  fi
}

reject_failure() {
  if [[ ! -f "$NORMALIZED_LOG" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M6[01]_[A-Z0-9_]*TIMEOUT|STORAGE_SERVER_FAULT_POLICY_TIMEOUT|^STORAGE_SERVER_RECOVERY_OK( |$)|^STORAGE_SERVER_REPEATED_RECOVERY_OK( |$)|^STORAGE_SERVER_ASYNC_RECOVERY_OK( |$)|^APPDATA_ASYNC_RECOVERY_OK( |$)|^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK( |$)|^BOOT_OK: M(5[5-9]|60) ' "$NORMALIZED_LOG"; then
    show_failure
    echo "M61 emitted a panic, failure, timeout, or stale M55-M60 marker." >&2
    exit 1
  fi
  if grep '^STORAGE_SERVER_RUNTIME_OK' "$NORMALIZED_LOG" | grep -Fvx "$RUNTIME_MARKER" >/dev/null; then
    show_failure
    echo "M61 published stale or malformed ABI/runtime evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M61 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_runtime_evidence() {
  if ! run_evidence_parser --validate "$NORMALIZED_LOG"; then
    show_failure
    echo "M61 evidence did not match its owner-retirement and recovery schema." >&2
    exit 1
  fi
}

validate_disk_protection() {
  python3 - "$PRISTINE_IMAGE" "$RUNTIME_IMAGE" <<'PY'
import hashlib
import sys
from pathlib import Path

before_path, after_path = (Path(argument) for argument in sys.argv[1:])
before = before_path.read_bytes()
after = after_path.read_bytes()
expected_size = 8 * 1024 * 1024
if len(before) != expected_size or len(after) != expected_size:
    raise SystemExit("M61 runtime disk lost the exact writable 8 MiB geometry")

sector = 512
data_start = 64 * sector
appdata_end = 2048 * sector
if before[:data_start] != after[:data_start] or before[appdata_end:] != after[appdata_end:]:
    raise SystemExit("M61 QEMU changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
if before[128 * sector:appdata_end] == after[128 * sector:appdata_end]:
    raise SystemExit("M61 runtime did not persist its AppData campaign")
if hashlib.sha256(before).digest() == hashlib.sha256(after).digest():
    raise SystemExit("M61 writable runtime image retained its pristine whole-disk hash")
PY
}

# Prove the production parser rejects its complete field-by-field fixture set
# before building or launching any guest code.
run_evidence_parser --self-test >/dev/null

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"
python3 - "$RUNTIME_IMAGE" "$PRISTINE_IMAGE" <<'PY'
import hashlib
import sys
from pathlib import Path

runtime, pristine = (Path(argument) for argument in sys.argv[1:])
if runtime.stat().st_size != 8 * 1024 * 1024 or pristine.stat().st_size != 8 * 1024 * 1024:
    raise SystemExit("M61 pristine disk lost the exact writable 8 MiB geometry")
if hashlib.sha256(runtime.read_bytes()).digest() != hashlib.sha256(pristine.read_bytes()).digest():
    raise SystemExit("M61 pristine disk hash changed before QEMU launch")
PY

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M61 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

: >"$SERIAL_LOG"
: >"$NORMALIZED_LOG"
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
  >"$SERIAL_LOG" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  base_count="$(grep -Ec "^${BASE_MARKER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  owner_count="$(grep -Ec "^${OWNER_MARKER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  runtime_count="$(grep -Fxc "$RUNTIME_MARKER" "$NORMALIZED_LOG" || true)"
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$base_count" == "1" && "$owner_count" == "1" && "$runtime_count" == "1" && "$boot_count" == "1" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    validate_runtime_evidence
    validate_disk_protection
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-server-owner-liveness-runtime.log"
    RUN_SUCCEEDED=1
    echo "M61 StorageServer owner-liveness runtime self-test passed."
    exit 0
  fi
  if ((base_count > 1 || owner_count > 1 || runtime_count > 1 || boot_count > 1)); then
    show_failure
    echo "M61 published duplicate base, owner, runtime, or BOOT_OK evidence." >&2
    exit 1
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    qemu_status=$?
    set -e
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    show_failure
    echo "QEMU exited before M61 owner-liveness evidence (status $qemu_status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M61 StorageServer owner-liveness evidence." >&2
exit 1
