#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

# The runtime checker and its offline negative self-test execute this same
# parser.  The M54 transcript remains the immutable prefix; this parser owns
# only the new M59 recovery leaf and its augmented AppData terminal evidence.
run_evidence_parser() {
  python3 - "$@" <<'PY'
import re
import sys
from pathlib import Path


AUTHORITY_MARKER = (
    "APPDATA_AUTHORITY_OK principal=1 root_success=1/6 non_app_denied=4 "
    "identities=init/launcher/surface-v1/surface-v2 attenuated_write_denied=1 "
    "transfer_escalation_denied=1 validation_rejected=12 "
    "system_write_rejected=1 data_write_rejected=1"
)
BOOT_MARKER = "BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified"
RECOVERY_PREFIX = "APPDATA_ASYNC_RECOVERY_OK"
RUNTIME_PREFIX = "APPDATA_RUNTIME_OK"

RECOVERY_KEYS = [
    "abi", "cases", "fault_order", "read_requires_reset", "injected_reads",
    "unavailable_read_retries", "blind_mutation_retries", "recovery_starts",
    "physical_completions", "recovery_commits", "recovery_failures",
    "async_steps", "physical_pending_returns", "coordinator_yields",
    "step_completion_relation", "timer_progress_windows",
    "worker_progress_windows", "el0_progress_windows", "authenticated_waits",
    "wait_dispatch_changes", "driver_timeouts", "driver_resets",
    "masked_poll_iterations", "max_step_masked_ticks",
    "max_control_masked_ticks", "timer_period_ticks", "long_daif_masks",
    "final_active", "gate_open", "invariant_errors",
]
RECOVERY_EXACT = {
    "abi": "24",
    "cases": "1",
    "fault_order": "R",
    "read_requires_reset": "1",
    "injected_reads": "1",
    "unavailable_read_retries": "1",
    "blind_mutation_retries": "0",
    "recovery_starts": "1",
    "physical_completions": "1",
    "recovery_commits": "1",
    "recovery_failures": "0",
    "step_completion_relation": "1",
    "timer_progress_windows": "1",
    "worker_progress_windows": "1",
    "el0_progress_windows": "1",
    "driver_timeouts": "1",
    "driver_resets": "1",
    "masked_poll_iterations": "0",
    "long_daif_masks": "0",
    "final_active": "0",
    "gate_open": "1",
    "invariant_errors": "0",
}
RUNTIME_KEYS = [
    "abi", "phase", "version", "boot_generation", "committed_generation",
    "entries", "files", "directories", "submissions", "completions",
    "retrievals", "mutations", "reads", "lists", "conflicts",
    "expected_terminal", "disk_reads", "disk_writes", "disk_flushes",
    "old_or_new", "full_readback", "resident", "errors", "async_recovery",
]
RUNTIME_EXACT = {
    "abi": "24",
    "phase": "created",
    "version": "1",
    "boot_generation": "0",
    "committed_generation": "2",
    "entries": "2",
    "files": "1",
    "directories": "1",
    "submissions": "11",
    "completions": "11",
    "retrievals": "11",
    "mutations": "2",
    "reads": "1",
    "lists": "3",
    "conflicts": "1",
    "expected_terminal": "4",
    "disk_writes": "580",
    "disk_flushes": "4",
    "old_or_new": "1",
    "full_readback": "1",
    "resident": "1",
    "errors": "0",
    "async_recovery": "1",
}


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def fields(lines: list[str], prefix: str, ordered: list[str]) -> tuple[str, dict[str, str]]:
    found = [
        line
        for line in lines
        if line == prefix or line.startswith(prefix + " ")
    ]
    reject(len(found) != 1, f"expected exactly one {prefix}, observed {found!r}")
    line = found[0]
    tokens = line.split(" ")
    reject("" in tokens, f"{prefix} contains non-canonical spacing")
    keys: list[str] = []
    values: dict[str, str] = {}
    for token in tokens[1:]:
        reject(token.count("=") != 1, f"{prefix} has malformed token {token!r}")
        key, value = token.split("=", 1)
        reject(
            not key or not value or key in values,
            f"{prefix} has invalid field {token!r}",
        )
        keys.append(key)
        values[key] = value
    reject(keys != ordered, f"{prefix} schema {keys!r}, expected {ordered!r}")
    return line, values


def canonical_decimal(value: str) -> bool:
    return re.fullmatch(r"0|[1-9][0-9]*", value) is not None


def validate(lines: list[str]) -> None:
    failure = re.compile(
        r"fatal exception:|kernel panic:|panic:|panicked at|boot error:|"
        r"BOOT_FAIL|STORAGE_FAIL|USER_FAIL(?::| )|EL0_FAIL|THREAD_EXITED:|"
        r"runtime (?:failed|timed out)|^[A-Z0-9_]+_(?:DIAG|TIMEOUT|FAIL|FAILED)(?::| |$)",
        re.IGNORECASE,
    )
    reject(
        any(failure.search(line) for line in lines),
        "M59 AppData log contained panic, diagnostic, timeout, or failure evidence",
    )
    forbidden_prefixes = (
        "STORAGE_SERVER_RECOVERY_OK",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "STORAGE_IRQ_RACE_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "BOOT_OK: M60",
    )
    reject(
        any(
            line == prefix or line.startswith(prefix + " ")
            for line in lines
            for prefix in forbidden_prefixes
        ),
        "M59 AppData checker observed an M56--M60 or timeout-profile marker",
    )
    reject(
        any(line.startswith(("M56_", "M57_", "M58_", "M60_")) for line in lines),
        "M59 AppData checker observed an M56--M60 milestone marker",
    )
    reject(
        any(
            line.startswith("BOOT_OK:") and line != BOOT_MARKER
            for line in lines
        ),
        "M59 AppData log contained an M54 or unrelated BOOT_OK marker",
    )
    authority_lines = [
        line
        for line in lines
        if line == "APPDATA_AUTHORITY_OK" or line.startswith("APPDATA_AUTHORITY_OK ")
    ]
    reject(
        authority_lines != [AUTHORITY_MARKER],
        "M59 authority evidence was not exact and unique",
    )
    reject(lines.count(BOOT_MARKER) != 1, "M59 BOOT_OK evidence was not exact and unique")

    recovery_line, recovery = fields(lines, RECOVERY_PREFIX, RECOVERY_KEYS)
    runtime_line, runtime = fields(lines, RUNTIME_PREFIX, RUNTIME_KEYS)
    for key, expected in RECOVERY_EXACT.items():
        reject(
            recovery[key] != expected,
            f"{RECOVERY_PREFIX} {key}={recovery[key]!r}, expected {expected!r}",
        )
    for key, expected in RUNTIME_EXACT.items():
        reject(
            runtime[key] != expected,
            f"{RUNTIME_PREFIX} {key}={runtime[key]!r}, expected {expected!r}",
        )

    recovery_dynamic = (
        "async_steps",
        "physical_pending_returns",
        "coordinator_yields",
        "authenticated_waits",
        "wait_dispatch_changes",
        "max_step_masked_ticks",
        "max_control_masked_ticks",
        "timer_period_ticks",
    )
    for key in recovery_dynamic:
        reject(
            not canonical_decimal(recovery[key]),
            f"{RECOVERY_PREFIX} {key} is not canonical decimal",
        )
    reject(
        not re.fullmatch(r"[1-9][0-9]*", runtime["disk_reads"]),
        "M59 AppData runtime did not publish positive canonical disk_reads",
    )

    values = {key: int(recovery[key]) for key in recovery_dynamic}
    reject(
        values["physical_pending_returns"] < 3,
        "M59 physical recovery did not expose all cooperative Pending boundaries",
    )
    reject(
        values["async_steps"] != values["physical_pending_returns"] + 1,
        "M59 step ledger did not isolate exactly one terminal physical step",
    )
    reject(
        values["coordinator_yields"] < values["physical_pending_returns"] + 2,
        "M59 coordinator did not yield around physical completion and progress proof",
    )
    reject(
        values["authenticated_waits"] < 2,
        "M59 did not observe two authenticated EL0 waits",
    )
    reject(
        values["wait_dispatch_changes"] < 2
        or values["wait_dispatch_changes"] > values["authenticated_waits"],
        "M59 authenticated retries did not span distinct timer dispatches",
    )
    period = values["timer_period_ticks"]
    reject(period == 0, "M59 timer period was zero")
    for field in ("max_step_masked_ticks", "max_control_masked_ticks"):
        reject(values[field] == 0, f"M59 {field} did not measure a control window")
        reject(
            values[field] >= period,
            f"M59 {field} crossed a complete timer period",
        )

    authority_pos = lines.index(AUTHORITY_MARKER)
    recovery_pos = lines.index(recovery_line)
    runtime_pos = lines.index(runtime_line)
    boot_pos = lines.index(BOOT_MARKER)
    reject(
        not authority_pos < recovery_pos < runtime_pos < boot_pos,
        "M59 authority, recovery, runtime, and BOOT_OK evidence was out of order",
    )


def canonical_lines() -> list[str]:
    recovery = (
        "APPDATA_ASYNC_RECOVERY_OK abi=24 cases=1 fault_order=R "
        "read_requires_reset=1 injected_reads=1 unavailable_read_retries=1 "
        "blind_mutation_retries=0 recovery_starts=1 physical_completions=1 "
        "recovery_commits=1 recovery_failures=0 async_steps=4 "
        "physical_pending_returns=3 coordinator_yields=5 "
        "step_completion_relation=1 timer_progress_windows=1 "
        "worker_progress_windows=1 el0_progress_windows=1 authenticated_waits=3 "
        "wait_dispatch_changes=2 driver_timeouts=1 driver_resets=1 "
        "masked_poll_iterations=0 max_step_masked_ticks=20 "
        "max_control_masked_ticks=30 timer_period_ticks=100 long_daif_masks=0 "
        "final_active=0 gate_open=1 invariant_errors=0"
    )
    runtime = (
        "APPDATA_RUNTIME_OK abi=24 phase=created version=1 boot_generation=0 "
        "committed_generation=2 entries=2 files=1 directories=1 submissions=11 "
        "completions=11 retrievals=11 mutations=2 reads=1 lists=3 conflicts=1 "
        "expected_terminal=4 disk_reads=100 disk_writes=580 disk_flushes=4 "
        "old_or_new=1 full_readback=1 resident=1 errors=0 async_recovery=1"
    )
    return [AUTHORITY_MARKER, recovery, runtime, BOOT_MARKER]


def replace(lines: list[str], old: str, new: str, line_index: int = 1) -> list[str]:
    changed = list(lines)
    changed[line_index] = changed[line_index].replace(old, new, 1)
    if changed[line_index] == lines[line_index]:
        raise AssertionError(f"self-test mutation did not match {old!r}")
    return changed


def self_test() -> None:
    canonical = canonical_lines()
    validate(canonical)
    negative = [
        ("duplicate_recovery", [canonical[0], canonical[1], canonical[1], canonical[2], canonical[3]]),
        ("ordering", [canonical[0], canonical[2], canonical[1], canonical[3]]),
        ("schema_order", replace(canonical, "async_steps=4 physical_pending_returns=3", "physical_pending_returns=3 async_steps=4")),
        ("pending_floor", replace(canonical, "physical_pending_returns=3", "physical_pending_returns=2")),
        ("step_relation", replace(canonical, "async_steps=4", "async_steps=5")),
        ("yield_floor", replace(canonical, "coordinator_yields=5", "coordinator_yields=4")),
        ("wait_floor", replace(canonical, "authenticated_waits=3", "authenticated_waits=1")),
        ("dispatch_floor", replace(canonical, "wait_dispatch_changes=2", "wait_dispatch_changes=1")),
        ("dispatch_upper", replace(canonical, "wait_dispatch_changes=2", "wait_dispatch_changes=4")),
        ("step_mask_zero", replace(canonical, "max_step_masked_ticks=20", "max_step_masked_ticks=0")),
        ("step_mask_bound", replace(canonical, "max_step_masked_ticks=20", "max_step_masked_ticks=100")),
        ("control_mask_bound", replace(canonical, "max_control_masked_ticks=30", "max_control_masked_ticks=100")),
        ("noncanonical_decimal", replace(canonical, "async_steps=4", "async_steps=04")),
        ("runtime_base_count", replace(canonical, "submissions=11", "submissions=10", 2)),
        ("runtime_missing_child", replace(canonical, " errors=0 async_recovery=1", " errors=0", 2)),
        ("m54_boot", [canonical[0], canonical[1], canonical[2], "BOOT_OK: M54 capability-scoped crash-safe AppData runtime verified"]),
        ("m56_marker", canonical + ["STORAGE_SERVER_RECOVERY_OK cases=1"]),
        ("m57_marker", canonical + ["STORAGE_SERVER_REPEATED_RECOVERY_OK cases=1"]),
        ("m58_marker", canonical + ["STORAGE_SERVER_ASYNC_RECOVERY_OK cases=1"]),
        ("m60_marker", canonical + ["STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6"]),
        ("m60_boot", canonical + ["BOOT_OK: M60 bounded StorageServer fault policy verified"]),
        ("timeout_marker", canonical + ["STORAGE_IRQ_COOPERATIVE_RECOVERY_OK starts=1"]),
        ("legacy_timeout_marker", canonical + ["STORAGE_IRQ_RACE_OK cases=1"]),
        ("failure_marker", canonical + ["M59_APPDATA_ASYNC_RECOVERY_DIAG physical=0"]),
        ("malformed_authority_duplicate", canonical + ["APPDATA_AUTHORITY_OK principal=2"]),
    ]
    for name, lines in negative:
        try:
            validate(lines)
        except EvidenceError:
            continue
        raise SystemExit(f"M59 AppData parser negative self-test accepted {name}")
    print("APPDATA_ASYNC_RECOVERY_PARSER_OK negative_cases=25")


if len(sys.argv) < 2:
    raise SystemExit("M59 AppData evidence parser mode is missing")
mode = sys.argv[1]
if mode == "--self-test":
    if len(sys.argv) != 2:
        raise SystemExit("M59 AppData parser self-test takes no path")
    self_test()
elif mode == "--validate":
    if len(sys.argv) != 3:
        raise SystemExit("M59 AppData parser validation requires exactly one log path")
    validate(Path(sys.argv[2]).read_text(encoding="utf-8").splitlines())
else:
    raise SystemExit(f"unknown M59 AppData evidence parser mode: {mode}")
PY
}

case "${1:-}" in
  --parser-self-test)
    if [[ "$#" -ne 1 ]]; then
      echo "--parser-self-test takes no additional arguments." >&2
      exit 2
    fi
    if ! command -v python3 >/dev/null 2>&1; then
      echo "python3 not found; cannot run the M59 AppData parser self-test." >&2
      exit 1
    fi
    run_evidence_parser --self-test
    exit 0
    ;;
  --validate-log)
    if [[ "$#" -ne 2 ]]; then
      echo "--validate-log requires exactly one normalized serial-log path." >&2
      exit 2
    fi
    if ! command -v python3 >/dev/null 2>&1; then
      echo "python3 not found; cannot validate M59 AppData evidence." >&2
      exit 1
    fi
    run_evidence_parser --validate "$2"
    exit 0
    ;;
  "")
    ;;
  *)
    echo "usage: $0 [--parser-self-test | --validate-log PATH]" >&2
    exit 2
    ;;
esac

# Reuse the frozen M49--M54 QMP, screenshot, partition-boundary, and PID
# ownership machinery.  The explicit child mode changes only the feature set,
# leaf markers, fresh-boot ledger, and target/m59 artifact names.
export BNDROID_APP_DATA_ASYNC_RECOVERY_MODE=1
exec "$SCRIPT_DIR/check-app-data-runtime.sh"
