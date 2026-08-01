#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

BOOT_MARKER="BOOT_OK: M60 bounded StorageServer fault policy and boot-local Offline state verified"
RECOVERY_MARKER_PREFIX="STORAGE_SERVER_FAULT_POLICY_OK"
RUNTIME_MARKER="STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1"

# Runtime evidence and the CPU-local negative fixtures share this exact parser
# body. That keeps the negative self-test from exercising a weaker duplicate.
run_evidence_parser() {
  python3 - "$@" <<'PY'
import re
import sys
from pathlib import Path

BOOT_MARKER = (
    "BOOT_OK: M60 bounded StorageServer fault policy and boot-local Offline state verified"
)
RUNTIME_MARKER = (
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 "
    "volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 "
    "repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 "
    "device_offline=1"
)

RECOVERY_PATTERN = re.compile(
    r"^STORAGE_SERVER_FAULT_POLICY_OK "
    r"recoverable_cases=6 permanent_cases=1 fault_order=WRFWRFR "
    r"read_requires_reset=3 mutation_outcome_unknown=4 "
    r"transient_control_sequences=(?P<transient_control_sequences>[0-9]+) "
    r"kernel_permanent_owner_requests=(?P<kernel_permanent_owner_requests>[0-9]+) "
    r"kernel_permanent_fault_arms=(?P<kernel_permanent_fault_arms>[0-9]+) "
    r"kernel_permanent_reads=(?P<kernel_permanent_reads>[0-9]+) "
    r"el0_permanent_fault_arm_controls=(?P<el0_permanent_fault_arm_controls>[0-9]+) "
    r"permanent_epoch=(?P<permanent_epoch>[0-9]+) permanent_authority=kernel-prearmed "
    r"injected_reads=3 injected_writes=2 injected_flushes=2 "
    r"owner_exits=7 offline_probe_exits=1 broker_releases=7 broker_abandoned=7 "
    r"recovery_attempts=(?P<recovery_attempts>[0-9]+) "
    r"recovery_commits=(?P<recovery_commits>[0-9]+) "
    r"recovery_failures=(?P<recovery_failures>[0-9]+) "
    r"recovery_rollbacks=(?P<recovery_rollbacks>[0-9]+) fail_closed_retries=1 "
    r"physical_successes=(?P<physical_successes>[0-9]+) "
    r"physical_failures=(?P<physical_failures>[0-9]+) "
    r"attempt_failures=(?P<attempt_failures>[0-9]+) "
    r"probation_io_failures=(?P<probation_io_failures>[0-9]+) "
    r"attempt_cap=(?P<attempt_cap>[0-9]+) "
    r"backoffs=(?P<backoffs>[0-9]+) "
    r"backoffs_completed=(?P<backoffs_completed>[0-9]+) "
    r"backoff_base_ticks=(?P<backoff_base_ticks>[0-9]+) "
    r"backoff_multiplier=(?P<backoff_multiplier>[0-9]+) "
    r"backoff_ticks=(?P<backoff_ticks>[0-9]+) "
    r"probation=(?P<probation_entries>[0-9]+)/(?P<probation_successes>[0-9]+)/(?P<probation_failures>[0-9]+) "
    r"healthy_transitions=(?P<healthy_transitions>[0-9]+) "
    r"device_offline=(?P<device_offline>[0-9]+) "
    r"offline_transitions=(?P<offline_transitions>[0-9]+) "
    r"offline_denials=(?P<offline_denials>[0-9]+) "
    r"early_rejections=(?P<early_rejections>[0-9]+) "
    r"early_physical_starts=(?P<early_physical_starts>[0-9]+) "
    r"offline_rejections=(?P<offline_rejections>[0-9]+) "
    r"stale_ticket_rejections=(?P<stale_ticket_rejections>[0-9]+) "
    r"invalid_transition_rejections=(?P<invalid_transition_rejections>[0-9]+) "
    r"generation_exhaustions=(?P<generation_exhaustions>[0-9]+) "
    r"persistent_fault_armed=(?P<persistent_fault_armed>[0-9]+) "
    r"persistent_fault_hits=(?P<persistent_fault_hits>[0-9]+) "
    r"driver_timeouts=(?P<driver_timeouts>[0-9]+) "
    r"driver_resets=(?P<driver_resets>[0-9]+) "
    r"async_starts=(?P<async_starts>[0-9]+) "
    r"physical_completions=(?P<physical_completions>[0-9]+) "
    r"async_physical_failures=(?P<async_physical_failures>[0-9]+) "
    r"async_steps=(?P<async_steps>[0-9]+) "
    r"recovery_yields=(?P<recovery_yields>[0-9]+) "
    r"timer_progress_windows=(?P<timer_progress_windows>[0-9]+) "
    r"worker_progress_windows=(?P<worker_progress_windows>[0-9]+) "
    r"el0_progress_windows=(?P<el0_progress_windows>[0-9]+) "
    r"backoff_timer_progress_windows=(?P<backoff_timer_progress_windows>[0-9]+) "
    r"backoff_worker_progress_windows=(?P<backoff_worker_progress_windows>[0-9]+) "
    r"acquire_waits=(?P<acquire_waits>[0-9]+) "
    r"acquire_dispatch_changes=(?P<acquire_dispatch_changes>[0-9]+) "
    r"terminal_quarantine_starts=(?P<terminal_quarantine_starts>[0-9]+) "
    r"terminal_quarantine_completions=(?P<terminal_quarantine_completions>[0-9]+) "
    r"terminal_quarantine_physical_errors=(?P<terminal_quarantine_physical_errors>[0-9]+) "
    r"terminal_irq_rollbacks=(?P<terminal_irq_rollbacks>[0-9]+) "
    r"terminal_dma_verifications=(?P<terminal_dma_verifications>[0-9]+) "
    r"terminal_driver_state=(?P<terminal_driver_state>[0-9]+) "
    r"terminal_transport_status=(?P<terminal_transport_status>[0-9]+) "
    r"terminal_in_flight=(?P<terminal_in_flight>[0-9]+) "
    r"masked_poll_iterations=(?P<masked_poll_iterations>[0-9]+) "
    r"max_step_masked_ticks=(?P<max_step_masked_ticks>[0-9]+) "
    r"max_control_masked_ticks=(?P<max_control_masked_ticks>[0-9]+) "
    r"timer_period_ticks=(?P<timer_period_ticks>[0-9]+) "
    r"long_daif_masks=(?P<long_daif_masks>[0-9]+) "
    r"offline_quiet_ticks=(?P<offline_quiet_ticks>[0-9]+) "
    r"post_offline_attempt_delta=(?P<post_offline_attempt_delta>[0-9]+) "
    r"post_offline_reset_delta=(?P<post_offline_reset_delta>[0-9]+) "
    r"post_offline_submission_delta=(?P<post_offline_submission_delta>[0-9]+) "
    r"final_epoch=(?P<final_epoch>[0-9]+) next_epoch=(?P<next_epoch>[0-9]+) "
    r"final_generation=(?P<final_generation>[0-9]+) "
    r"final_irq_armed=(?P<final_irq_armed>[0-9]+) "
    r"final_irq_failed=(?P<final_irq_failed>[0-9]+) "
    r"final_recovery_required=(?P<final_recovery_required>[0-9]+) "
    r"final_admission_open=(?P<final_admission_open>[0-9]+) "
    r"final_broker_state=(?P<final_broker_state>[0-9]+) "
    r"final_broker_bound=(?P<final_broker_bound>[0-9]+) "
    r"final_broker_pending=(?P<final_broker_pending>[0-9]+) "
    r"requests=(?P<requests>[0-9]+) completions=(?P<completions>[0-9]+) "
    r"simulated_permanent=(?P<simulated_permanent>[0-9]+) "
    r"hardware_claim=(?P<hardware_claim>[0-9]+) "
    r"arbitrary_soak_claim=(?P<arbitrary_soak_claim>[0-9]+) "
    r"powercut_claim=(?P<powercut_claim>[0-9]+) "
    r"concurrency_claim=(?P<concurrency_claim>[0-9]+) "
    r"general_runtime=(?P<general_runtime>[0-9]+) "
    r"invariant_errors=(?P<invariant_errors>[0-9]+)$"
)


class EvidenceError(ValueError):
    pass


def reject(condition: bool, message: str) -> None:
    if condition:
        raise EvidenceError(message)


def exact(values: dict[str, int], field: str, wanted: int) -> None:
    reject(values[field] != wanted, f"M60 {field} was {values[field]}, expected {wanted}")


def validate(lines: list[str]) -> None:
    stale_prefixes = (
        "STORAGE_SERVER_RECOVERY_OK",
        "STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "BOOT_OK: M55",
        "BOOT_OK: M56",
        "BOOT_OK: M57",
        "BOOT_OK: M58",
        "BOOT_OK: M59",
    )
    reject(
        any(
            any(line == prefix or line.startswith(prefix + " ") for prefix in stale_prefixes)
            for line in lines
        ),
        "M60 checker observed stale M55-M59 recovery evidence",
    )

    recovery_lines = [
        line
        for line in lines
        if line == "STORAGE_SERVER_FAULT_POLICY_OK"
        or line.startswith("STORAGE_SERVER_FAULT_POLICY_OK ")
    ]
    reject(len(recovery_lines) != 1, "M60 fault-policy evidence was not unique")
    reject(lines.count(RUNTIME_MARKER) != 1, "M60 runtime marker was not exact and unique")
    reject(lines.count(BOOT_MARKER) != 1, "M60 BOOT_OK evidence was not exact and unique")

    runtime_lines = [
        line
        for line in lines
        if line == "STORAGE_SERVER_RUNTIME_OK"
        or line.startswith("STORAGE_SERVER_RUNTIME_OK ")
    ]
    reject(runtime_lines != [RUNTIME_MARKER], "M60 ABI/runtime evidence changed or duplicated")
    unrelated_boots = [line for line in lines if line.startswith("BOOT_OK:") and line != BOOT_MARKER]
    reject(bool(unrelated_boots), "M60 log contained a stale or unrelated BOOT_OK marker")

    recovery = recovery_lines[0]
    reject(
        not (lines.index(recovery) < lines.index(RUNTIME_MARKER) < lines.index(BOOT_MARKER)),
        "M60 recovery, runtime, and BOOT_OK evidence was out of order",
    )
    match = RECOVERY_PATTERN.fullmatch(recovery)
    reject(match is None, "M60 fault-policy evidence changed its exact field contract")
    assert match is not None
    values = {name: int(value) for name, value in match.groupdict().items()}

    expected = {
        "transient_control_sequences": 6,
        "kernel_permanent_owner_requests": 1,
        "kernel_permanent_fault_arms": 1,
        "kernel_permanent_reads": 1,
        "el0_permanent_fault_arm_controls": 0,
        "permanent_epoch": 7,
        "recovery_attempts": 9,
        "recovery_commits": 6,
        "recovery_failures": 3,
        "recovery_rollbacks": 1,
        "physical_successes": 7,
        "physical_failures": 2,
        "attempt_failures": 4,
        "probation_io_failures": 1,
        "attempt_cap": 3,
        "backoffs": 3,
        "backoffs_completed": 3,
        "backoff_base_ticks": 2,
        "backoff_multiplier": 2,
        "backoff_ticks": 8,
        "probation_entries": 7,
        "probation_successes": 5,
        "probation_failures": 2,
        "healthy_transitions": 5,
        "device_offline": 1,
        "offline_transitions": 1,
        "offline_denials": 1,
        "early_rejections": 0,
        "early_physical_starts": 0,
        "offline_rejections": 0,
        "stale_ticket_rejections": 0,
        "invalid_transition_rejections": 0,
        "generation_exhaustions": 0,
        "persistent_fault_armed": 1,
        "persistent_fault_hits": 2,
        "driver_timeouts": 7,
        "driver_resets": 10,
        "async_starts": 9,
        "physical_completions": 7,
        "async_physical_failures": 2,
        "backoff_timer_progress_windows": 3,
        "backoff_worker_progress_windows": 3,
        "terminal_quarantine_starts": 0,
        "terminal_quarantine_completions": 0,
        "terminal_quarantine_physical_errors": 0,
        "terminal_irq_rollbacks": 1,
        "terminal_dma_verifications": 1,
        "terminal_driver_state": 2,
        "terminal_transport_status": 0,
        "terminal_in_flight": 0,
        "masked_poll_iterations": 0,
        "long_daif_masks": 0,
        "post_offline_attempt_delta": 0,
        "post_offline_reset_delta": 0,
        "post_offline_submission_delta": 0,
        "final_epoch": 0,
        "next_epoch": 8,
        "final_irq_armed": 0,
        "final_irq_failed": 1,
        "final_recovery_required": 1,
        "final_admission_open": 0,
        "final_broker_state": 4,
        "final_broker_bound": 0,
        "final_broker_pending": 0,
        "simulated_permanent": 1,
        "hardware_claim": 0,
        "arbitrary_soak_claim": 0,
        "powercut_claim": 0,
        "concurrency_claim": 0,
        "general_runtime": 0,
        "invariant_errors": 0,
    }
    for field, wanted in expected.items():
        exact(values, field, wanted)

    reject(
        values["recovery_attempts"]
        != values["physical_successes"] + values["physical_failures"],
        "M60 attempt ledger did not split into seven completions and two failures",
    )
    reject(
        values["physical_successes"]
        != values["recovery_commits"] + values["recovery_rollbacks"],
        "M60 physical-success ledger did not split into six commits and one rollback",
    )
    reject(
        values["recovery_failures"]
        != values["recovery_rollbacks"] + values["physical_failures"],
        "M60 coordinator failures did not isolate rollback plus physical failures",
    )
    reject(
        values["attempt_failures"]
        != values["recovery_failures"] + values["probation_io_failures"],
        "M60 policy failures did not include coordinator and probation I/O failures",
    )
    reject(
        values["probation_failures"]
        != values["recovery_rollbacks"] + values["probation_io_failures"],
        "M60 probation failures did not isolate rollback and probation I/O failure",
    )
    reject(
        values["driver_resets"]
        != values["driver_timeouts"]
        + values["recovery_rollbacks"]
        + values["persistent_fault_hits"],
        "M60 reset ledger did not isolate timeouts, rollback, and persistent hits",
    )
    reject(
        values["async_starts"] != values["recovery_attempts"]
        or values["physical_completions"] != values["physical_successes"]
        or values["async_physical_failures"] != values["physical_failures"],
        "M60 coordinator and physical attempt ledgers diverged",
    )
    reject(
        values["async_steps"] != values["recovery_yields"] + values["recovery_attempts"],
        "M60 step/yield ledger did not isolate nine terminal physical steps",
    )
    reject(values["recovery_yields"] < 26, "M60 cooperative recovery exposed fewer than 26 yields")

    for field in (
        "timer_progress_windows",
        "worker_progress_windows",
        "el0_progress_windows",
    ):
        reject(
            values[field] < 7 or values[field] > 9,
            f"M60 {field} did not cover every successful physical window",
        )
    reject(values["acquire_waits"] < 18, "M60 lacked authenticated EL0 acquire waits")
    reject(
        values["acquire_dispatch_changes"] < 18
        or values["acquire_dispatch_changes"] > values["acquire_waits"],
        "M60 EL0 acquire retries did not span distinct timer dispatches",
    )
    reject(values["offline_quiet_ticks"] < 16, "M60 Offline state was not observed for 16 ticks")

    period = values["timer_period_ticks"]
    reject(period == 0, "M60 timer period was zero")
    reject(
        values["max_step_masked_ticks"] == 0 or values["max_step_masked_ticks"] >= period,
        "M60 cooperative driver held DAIF across a full timer period",
    )
    reject(
        values["max_control_masked_ticks"] == 0
        or values["max_control_masked_ticks"] >= period,
        "M60 recovery control held DAIF across a full timer period",
    )
    reject(values["final_generation"] == 0, "M60 lost its persisted generation proof")
    reject(
        values["requests"] != values["completions"] + 7,
        "M60 request ledger did not isolate exactly seven timed-out requests",
    )


def canonical_lines() -> list[str]:
    recovery = (
        "STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6 permanent_cases=1 "
        "fault_order=WRFWRFR read_requires_reset=3 mutation_outcome_unknown=4 "
        "transient_control_sequences=6 kernel_permanent_owner_requests=1 "
        "kernel_permanent_fault_arms=1 kernel_permanent_reads=1 "
        "el0_permanent_fault_arm_controls=0 permanent_epoch=7 "
        "permanent_authority=kernel-prearmed injected_reads=3 "
        "injected_writes=2 injected_flushes=2 "
        "owner_exits=7 offline_probe_exits=1 broker_releases=7 broker_abandoned=7 "
        "recovery_attempts=9 recovery_commits=6 recovery_failures=3 "
        "recovery_rollbacks=1 fail_closed_retries=1 physical_successes=7 "
        "physical_failures=2 attempt_failures=4 probation_io_failures=1 "
        "attempt_cap=3 backoffs=3 backoffs_completed=3 backoff_base_ticks=2 "
        "backoff_multiplier=2 backoff_ticks=8 probation=7/5/2 healthy_transitions=5 "
        "device_offline=1 offline_transitions=1 offline_denials=1 "
        "early_rejections=0 early_physical_starts=0 offline_rejections=0 "
        "stale_ticket_rejections=0 invalid_transition_rejections=0 "
        "generation_exhaustions=0 persistent_fault_armed=1 persistent_fault_hits=2 "
        "driver_timeouts=7 driver_resets=10 async_starts=9 physical_completions=7 "
        "async_physical_failures=2 async_steps=35 recovery_yields=26 "
        "timer_progress_windows=7 worker_progress_windows=7 el0_progress_windows=7 "
        "backoff_timer_progress_windows=3 backoff_worker_progress_windows=3 "
        "acquire_waits=20 acquire_dispatch_changes=18 terminal_quarantine_starts=0 "
        "terminal_quarantine_completions=0 terminal_quarantine_physical_errors=0 "
        "terminal_irq_rollbacks=1 terminal_dma_verifications=1 terminal_driver_state=2 "
        "terminal_transport_status=0 terminal_in_flight=0 masked_poll_iterations=0 "
        "max_step_masked_ticks=20 max_control_masked_ticks=30 timer_period_ticks=100 "
        "long_daif_masks=0 offline_quiet_ticks=16 post_offline_attempt_delta=0 "
        "post_offline_reset_delta=0 post_offline_submission_delta=0 final_epoch=0 "
        "next_epoch=8 final_generation=1 final_irq_armed=0 final_irq_failed=1 "
        "final_recovery_required=1 final_admission_open=0 final_broker_state=4 "
        "final_broker_bound=0 final_broker_pending=0 requests=120 completions=113 "
        "simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 "
        "powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0"
    )
    return [recovery, RUNTIME_MARKER, BOOT_MARKER]


def replace_recovery(lines: list[str], old: str, new: str) -> list[str]:
    changed = list(lines)
    changed[0] = changed[0].replace(old, new, 1)
    if changed[0] == lines[0]:
        raise AssertionError(f"self-test mutation did not match {old!r}")
    return changed


def replace_runtime(lines: list[str], old: str, new: str) -> list[str]:
    changed = list(lines)
    changed[1] = changed[1].replace(old, new, 1)
    if changed[1] == lines[1]:
        raise AssertionError(f"runtime self-test mutation did not match {old!r}")
    return changed


def self_test() -> None:
    canonical = canonical_lines()
    validate(canonical)
    mutations = [
        ("recoverable_cases=6", "recoverable_cases=5"),
        ("permanent_cases=1", "permanent_cases=2"),
        ("fault_order=WRFWRFR", "fault_order=RWFWRFW"),
        ("read_requires_reset=3", "read_requires_reset=2"),
        ("mutation_outcome_unknown=4", "mutation_outcome_unknown=3"),
        ("transient_control_sequences=6", "transient_control_sequences=5"),
        ("kernel_permanent_owner_requests=1", "kernel_permanent_owner_requests=0"),
        ("kernel_permanent_fault_arms=1", "kernel_permanent_fault_arms=0"),
        ("kernel_permanent_reads=1", "kernel_permanent_reads=0"),
        (
            "el0_permanent_fault_arm_controls=0",
            "el0_permanent_fault_arm_controls=1",
        ),
        ("permanent_epoch=7", "permanent_epoch=6"),
        ("permanent_authority=kernel-prearmed", "permanent_authority=el0-armed"),
        ("injected_reads=3", "injected_reads=2"),
        ("injected_writes=2", "injected_writes=1"),
        ("injected_flushes=2", "injected_flushes=1"),
        ("owner_exits=7", "owner_exits=6"),
        ("offline_probe_exits=1", "offline_probe_exits=0"),
        ("broker_releases=7", "broker_releases=6"),
        ("broker_abandoned=7", "broker_abandoned=6"),
        ("recovery_attempts=9", "recovery_attempts=8"),
        ("recovery_commits=6", "recovery_commits=5"),
        ("recovery_failures=3", "recovery_failures=2"),
        ("recovery_rollbacks=1", "recovery_rollbacks=0"),
        ("fail_closed_retries=1", "fail_closed_retries=0"),
        ("physical_successes=7", "physical_successes=6"),
        ("physical_failures=2", "physical_failures=1"),
        ("attempt_failures=4", "attempt_failures=3"),
        ("probation_io_failures=1", "probation_io_failures=0"),
        ("attempt_cap=3", "attempt_cap=4"),
        ("backoffs=3", "backoffs=2"),
        ("backoffs_completed=3", "backoffs_completed=2"),
        ("backoff_base_ticks=2", "backoff_base_ticks=1"),
        ("backoff_multiplier=2", "backoff_multiplier=3"),
        ("backoff_ticks=8", "backoff_ticks=7"),
        ("probation=7/5/2", "probation=7/6/1"),
        ("healthy_transitions=5", "healthy_transitions=6"),
        ("device_offline=1", "device_offline=0"),
        ("offline_transitions=1", "offline_transitions=0"),
        ("offline_denials=1", "offline_denials=0"),
        ("early_rejections=0", "early_rejections=1"),
        ("early_physical_starts=0", "early_physical_starts=1"),
        ("offline_rejections=0", "offline_rejections=1"),
        ("stale_ticket_rejections=0", "stale_ticket_rejections=1"),
        ("invalid_transition_rejections=0", "invalid_transition_rejections=1"),
        ("generation_exhaustions=0", "generation_exhaustions=1"),
        ("persistent_fault_armed=1", "persistent_fault_armed=0"),
        ("persistent_fault_hits=2", "persistent_fault_hits=1"),
        ("driver_timeouts=7", "driver_timeouts=6"),
        ("driver_resets=10", "driver_resets=9"),
        ("async_starts=9", "async_starts=8"),
        ("physical_completions=7", "physical_completions=6"),
        ("async_physical_failures=2", "async_physical_failures=1"),
        ("async_steps=35", "async_steps=34"),
        ("recovery_yields=26", "recovery_yields=25"),
        ("timer_progress_windows=7", "timer_progress_windows=6"),
        ("worker_progress_windows=7", "worker_progress_windows=10"),
        ("el0_progress_windows=7", "el0_progress_windows=6"),
        ("backoff_timer_progress_windows=3", "backoff_timer_progress_windows=2"),
        ("backoff_worker_progress_windows=3", "backoff_worker_progress_windows=2"),
        ("acquire_waits=20", "acquire_waits=17"),
        ("acquire_dispatch_changes=18", "acquire_dispatch_changes=17"),
        ("acquire_dispatch_changes=18", "acquire_dispatch_changes=21"),
        ("terminal_quarantine_starts=0", "terminal_quarantine_starts=1"),
        ("terminal_quarantine_completions=0", "terminal_quarantine_completions=1"),
        ("terminal_quarantine_physical_errors=0", "terminal_quarantine_physical_errors=1"),
        ("terminal_irq_rollbacks=1", "terminal_irq_rollbacks=0"),
        ("terminal_dma_verifications=1", "terminal_dma_verifications=0"),
        ("terminal_driver_state=2", "terminal_driver_state=1"),
        ("terminal_transport_status=0", "terminal_transport_status=1"),
        ("terminal_in_flight=0", "terminal_in_flight=1"),
        ("masked_poll_iterations=0", "masked_poll_iterations=1"),
        ("max_step_masked_ticks=20", "max_step_masked_ticks=0"),
        ("max_step_masked_ticks=20", "max_step_masked_ticks=100"),
        ("max_control_masked_ticks=30", "max_control_masked_ticks=0"),
        ("max_control_masked_ticks=30", "max_control_masked_ticks=100"),
        ("timer_period_ticks=100", "timer_period_ticks=0"),
        ("long_daif_masks=0", "long_daif_masks=1"),
        ("offline_quiet_ticks=16", "offline_quiet_ticks=15"),
        ("post_offline_attempt_delta=0", "post_offline_attempt_delta=1"),
        ("post_offline_reset_delta=0", "post_offline_reset_delta=1"),
        ("post_offline_submission_delta=0", "post_offline_submission_delta=1"),
        ("final_epoch=0", "final_epoch=7"),
        ("next_epoch=8", "next_epoch=9"),
        ("final_generation=1", "final_generation=0"),
        ("final_irq_armed=0", "final_irq_armed=1"),
        ("final_irq_failed=1", "final_irq_failed=0"),
        ("final_recovery_required=1", "final_recovery_required=0"),
        ("final_admission_open=0", "final_admission_open=1"),
        ("final_broker_state=4", "final_broker_state=0"),
        ("final_broker_bound=0", "final_broker_bound=1"),
        ("final_broker_pending=0", "final_broker_pending=1"),
        ("requests=120", "requests=119"),
        ("simulated_permanent=1", "simulated_permanent=0"),
        ("hardware_claim=0", "hardware_claim=1"),
        ("arbitrary_soak_claim=0", "arbitrary_soak_claim=1"),
        ("powercut_claim=0", "powercut_claim=1"),
        ("concurrency_claim=0", "concurrency_claim=1"),
        ("general_runtime=0", "general_runtime=1"),
        ("invariant_errors=0", "invariant_errors=1"),
    ]
    negative = [
        (f"field_{index}", replace_recovery(canonical, old, new))
        for index, (old, new) in enumerate(mutations, 1)
    ]
    negative.extend(
        [
            ("duplicate_recovery", [canonical[0], canonical[0], canonical[1], canonical[2]]),
            ("duplicate_runtime", [canonical[0], canonical[1], canonical[1], canonical[2]]),
            ("duplicate_boot", [canonical[0], canonical[1], canonical[2], canonical[2]]),
            ("ordering", [canonical[2], canonical[0], canonical[1]]),
            ("stale_m58", canonical + ["STORAGE_SERVER_ASYNC_RECOVERY_OK cases=6"]),
            ("stale_m57", canonical + ["STORAGE_SERVER_REPEATED_RECOVERY_OK cycles=2"]),
            ("stale_appdata", canonical + ["APPDATA_ASYNC_RECOVERY_OK recoveries=1"]),
            ("stale_irq", canonical + ["STORAGE_IRQ_COOPERATIVE_RECOVERY_OK recoveries=1"]),
            ("unrelated_boot", canonical + ["BOOT_OK: M59 cooperative storage verified"]),
            (
                "runtime_fault_policy",
                replace_runtime(canonical, "fault_policy=1", "fault_policy=0"),
            ),
            (
                "runtime_attempt_cap",
                replace_runtime(canonical, "attempt_cap=3", "attempt_cap=4"),
            ),
            (
                "runtime_offline",
                replace_runtime(canonical, "device_offline=1", "device_offline=0"),
            ),
        ]
    )
    for name, lines in negative:
        try:
            validate(lines)
        except EvidenceError:
            continue
        raise SystemExit(f"M60 parser negative self-test accepted {name}")
    print(f"STORAGE_SERVER_FAULT_POLICY_PARSER_OK negative_cases={len(negative)}")


if len(sys.argv) < 2:
    raise SystemExit("M60 evidence parser mode is missing")
mode = sys.argv[1]
if mode == "--self-test":
    if len(sys.argv) != 2:
        raise SystemExit("M60 parser self-test takes no path")
    self_test()
elif mode == "--validate":
    if len(sys.argv) != 3:
        raise SystemExit("M60 parser validation requires exactly one log path")
    validate(Path(sys.argv[2]).read_text(encoding="utf-8").splitlines())
else:
    raise SystemExit(f"unknown M60 evidence parser mode: {mode}")
PY
}

if [[ "${1:-}" == "--parser-self-test" ]]; then
  if [[ "$#" -ne 1 ]]; then
    echo "--parser-self-test takes no additional arguments." >&2
    exit 2
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found; cannot run the M60 parser self-test." >&2
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
    echo "$tool not found; cannot verify the M60 StorageServer fault policy runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M60 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
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

FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime,storage-server-async-recovery-runtime,storage-server-fault-policy-runtime"
TARGET_ROOT="${BNDROID_STORAGE_SERVER_FAULT_POLICY_TARGET_DIR:-$WORKSPACE_ROOT/target/storage-server-fault-policy-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m60"
mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m60.XXXXXX")"
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
    echo "M60 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 300 "$NORMALIZED_LOG" >&2 || true
  fi
}

reject_failure() {
  if [[ ! -f "$NORMALIZED_LOG" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|(^|[[:space:]_:])(failed|failure|diag|diagnostic)([[:space:]_:]|$)' "$NORMALIZED_LOG" \
    || grep -Eq '(^|[[:space:]_:])[A-Z0-9_]*FAIL(:|[[:space:]]|$)|M60_[A-Z0-9_]*TIMEOUT|STORAGE_SERVER_FAULT_POLICY_TIMEOUT|^STORAGE_SERVER_RECOVERY_OK( |$)|^STORAGE_SERVER_REPEATED_RECOVERY_OK( |$)|^STORAGE_SERVER_ASYNC_RECOVERY_OK( |$)|^APPDATA_ASYNC_RECOVERY_OK( |$)|^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK( |$)|^BOOT_OK: M5[5-9] ' "$NORMALIZED_LOG"; then
    show_failure
    echo "M60 emitted a panic, failure, timeout, or stale M55-M59 marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M60 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

validate_recovery_evidence() {
  if ! run_evidence_parser --validate "$NORMALIZED_LOG"; then
    show_failure
    echo "M60 evidence did not match its bounded retry/Offline schema." >&2
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
    raise SystemExit("M60 runtime disk lost the exact writable 8 MiB geometry")

sector = 512
data_start = 64 * sector
appdata_end = 2048 * sector
if before[:data_start] != after[:data_start] or before[appdata_end:] != after[appdata_end:]:
    raise SystemExit("M60 QEMU changed bytes outside BNDROID_DATA/BNDROID_APPDATA")
if before[128 * sector:appdata_end] == after[128 * sector:appdata_end]:
    raise SystemExit("M60 runtime did not persist its AppData campaign")
if hashlib.sha256(before).digest() == hashlib.sha256(after).digest():
    raise SystemExit("M60 writable runtime image retained its pristine whole-disk hash")
PY
}

# Prove the exact production parser rejects its negative fixtures before any
# build or guest process is started.
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
    raise SystemExit("M60 pristine disk lost the exact writable 8 MiB geometry")
if hashlib.sha256(runtime.read_bytes()).digest() != hashlib.sha256(pristine.read_bytes()).digest():
    raise SystemExit("M60 pristine disk hash changed before QEMU launch")
PY

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M60 kernel image is missing: $KERNEL_IMAGE" >&2
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
  recovery_count="$(grep -Ec "^${RECOVERY_MARKER_PREFIX}( |$)" "$NORMALIZED_LOG" || true)"
  runtime_count="$(grep -Fxc "$RUNTIME_MARKER" "$NORMALIZED_LOG" || true)"
  boot_count="$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)"
  if [[ "$recovery_count" == "1" && "$runtime_count" == "1" && "$boot_count" == "1" ]]; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
    QEMU_PID=""
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    validate_recovery_evidence
    validate_disk_protection
    cp "$NORMALIZED_LOG" "$OUTPUT_DIR/storage-server-fault-policy-runtime.log"
    RUN_SUCCEEDED=1
    echo "M60 bounded StorageServer fault policy runtime self-test passed."
    exit 0
  fi
  if ((recovery_count > 1 || runtime_count > 1 || boot_count > 1)); then
    show_failure
    echo "M60 published duplicate recovery, runtime, or BOOT_OK evidence." >&2
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
    echo "QEMU exited before M60 fault-policy evidence (status $qemu_status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
show_failure
echo "Timed out waiting for exact M60 bounded StorageServer fault-policy evidence." >&2
exit 1
