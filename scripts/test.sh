#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_TARGET_DIR="$WORKSPACE_ROOT/target"
export CARGO_NET_OFFLINE=true

"$SCRIPT_DIR/check-storage-server-static.sh"
"$SCRIPT_DIR/check-storage-server-recovery-static.sh"
"$SCRIPT_DIR/check-storage-server-repeated-recovery-static.sh"
"$SCRIPT_DIR/check-storage-server-async-recovery-static.sh"
"$SCRIPT_DIR/check-storage-server-fault-policy-static.sh"
"$SCRIPT_DIR/check-storage-server-owner-liveness-static.sh"
"$SCRIPT_DIR/check-storage-server-terminal-quarantine-static.sh"
"$SCRIPT_DIR/check-storage-server-persistent-health-static.sh"
"$SCRIPT_DIR/check-storage-server-clean-shutdown-static.sh"
"$SCRIPT_DIR/check-storage-server-shutdown-orchestration-static.sh"
"$SCRIPT_DIR/check-resident-platform-shutdown-static.sh"
"$SCRIPT_DIR/check-unified-product-static.sh"
"$SCRIPT_DIR/check-unified-product-liveness-static.sh"
"$SCRIPT_DIR/check-unified-product-multiservice-liveness-static.sh"
"$SCRIPT_DIR/check-unified-product-psci-shutdown-static.sh"
"$SCRIPT_DIR/check-unified-product-continuous-supervision-static.sh"
"$SCRIPT_DIR/check-unified-product-manifest-supervision-static.sh"
"$SCRIPT_DIR/check-unified-product-event-supervision-static.sh"
"$SCRIPT_DIR/check-unified-product-verified-manifest-static.sh"
"$SCRIPT_DIR/check-unified-product-persistent-rollback-static.sh"
"$SCRIPT_DIR/check-unified-product-key-rotation-static.sh"
"$SCRIPT_DIR/check-unified-product-maintenance-authorization-static.sh"
"$SCRIPT_DIR/check-unified-product-maintenance-execution-static.sh"
"$SCRIPT_DIR/check-unified-product-maintenance-step-static.sh"
"$SCRIPT_DIR/check-unified-product-maintenance-plan-static.sh"
"$SCRIPT_DIR/check-unified-product-signed-maintenance-plan-static.sh"
"$SCRIPT_DIR/check-storage-recovery-unification-static.sh"
cargo fmt --all -- --check
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --locked --target "$HOST_TRIPLE" --workspace --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-key-rotation-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-maintenance-authorization-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-maintenance-execution-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-maintenance-step-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-maintenance-plan-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi --features unified-product-signed-maintenance-plan-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndr-sm --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-key-rotation-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-maintenance-authorization-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-maintenance-execution-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-maintenance-step-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-maintenance-plan-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features unified-product-signed-maintenance-plan-runtime --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features multi-window-runtime ui_trace:: --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features multi-window-runtime window_trace:: --lib
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --features persistent-window-runtime persistent_window_trace:: --lib
cargo clippy --locked --target aarch64-unknown-none --workspace --bins -- -D warnings
cargo clippy --locked --target aarch64-unknown-none --workspace --all-features --bins -- -D warnings
BNDROID_PROFILE=debug "$SCRIPT_DIR/build-kernel.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=off "$SCRIPT_DIR/check-qemu-boot.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=on "$SCRIPT_DIR/check-qemu-boot.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=off BNDROID_QEMU_CPU=max "$SCRIPT_DIR/check-qemu-boot.sh"
BNDROID_PROFILE=release BNDROID_QEMU_VIRTUALIZATION=off "$SCRIPT_DIR/check-qemu-boot.sh"
BNDROID_PROFILE=release BNDROID_QEMU_VIRTUALIZATION=on "$SCRIPT_DIR/check-qemu-boot.sh"
BNDROID_PROFILE=debug "$SCRIPT_DIR/check-framebuffer.sh"
BNDROID_PROFILE=debug "$SCRIPT_DIR/check-input.sh"
BNDROID_PROFILE=debug "$SCRIPT_DIR/check-compositor.sh"
BNDROID_PROFILE=debug "$SCRIPT_DIR/check-ui.sh"
"$SCRIPT_DIR/check-scheduler-negative.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=off "$SCRIPT_DIR/check-sleep-negative.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=on "$SCRIPT_DIR/check-sleep-negative.sh"
BNDROID_PROFILE=release BNDROID_QEMU_VIRTUALIZATION=off "$SCRIPT_DIR/check-sleep-negative.sh"
BNDROID_PROFILE=release BNDROID_QEMU_VIRTUALIZATION=on "$SCRIPT_DIR/check-sleep-negative.sh"
BNDROID_PROFILE=debug BNDROID_QEMU_VIRTUALIZATION=off "$SCRIPT_DIR/check-vm-unmapped-access.sh"
BNDROID_PROFILE=release BNDROID_QEMU_VIRTUALIZATION=on "$SCRIPT_DIR/check-vm-unmapped-access.sh"
"$SCRIPT_DIR/check-frame-negative.sh"
"$SCRIPT_DIR/check-heap-negative.sh"
"$SCRIPT_DIR/check-heap-rollback.sh"
"$SCRIPT_DIR/check-vm-stale-negative.sh"
"$SCRIPT_DIR/check-mmu-protection.sh"
"$SCRIPT_DIR/check-el0-fault-containment.sh"
"$SCRIPT_DIR/check-process-terminate.sh"
"$SCRIPT_DIR/check-app-lifecycle.sh"
"$SCRIPT_DIR/check-app-crash-recovery.sh"
"$SCRIPT_DIR/check-mapped-graphics.sh"
"$SCRIPT_DIR/check-graphics-owner-death.sh"
"$SCRIPT_DIR/check-graphics-surface-restart.sh"
"$SCRIPT_DIR/check-graphics-producer-orphan.sh"
"$SCRIPT_DIR/check-graphics-frame-clock.sh"
"$SCRIPT_DIR/check-graphics-swapchain.sh"
"$SCRIPT_DIR/check-multi-window.sh"
"$SCRIPT_DIR/check-persistent-window.sh"
"$SCRIPT_DIR/check-text-input.sh"
"$SCRIPT_DIR/check-soft-keyboard.sh"
"$SCRIPT_DIR/check-input-server.sh"
"$SCRIPT_DIR/check-input-server-surface-restart.sh"
"$SCRIPT_DIR/check-input-server-restart.sh"
"$SCRIPT_DIR/check-service-supervisor.sh"
"$SCRIPT_DIR/check-service-dependency.sh"
"$SCRIPT_DIR/check-post-recovery-interaction.sh"
"$SCRIPT_DIR/check-post-recovery-focus.sh"
bash "$SCRIPT_DIR/check-post-recovery-focus-roundtrip.sh"
"$SCRIPT_DIR/check-post-recovery-lifecycle-focus.sh"
"$SCRIPT_DIR/check-app-data-runtime.sh"
"$SCRIPT_DIR/check-app-data-async-recovery-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-recovery-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-repeated-recovery-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-async-recovery-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-fault-policy-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-owner-liveness-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-terminal-quarantine-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-persistent-health-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-clean-shutdown-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-shutdown-orchestration-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-resident-platform-shutdown-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-liveness-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-multiservice-liveness-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-psci-shutdown-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-continuous-supervision-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-manifest-supervision-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-event-supervision-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-verified-manifest-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-persistent-rollback-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-key-rotation-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-maintenance-authorization-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-maintenance-execution-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-maintenance-step-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-maintenance-plan-runtime.sh"
BNDROID_PROFILE=release "$SCRIPT_DIR/check-unified-product-signed-maintenance-plan-runtime.sh"
"$SCRIPT_DIR/check-storage-negative.sh"
"$SCRIPT_DIR/check-storage-persistence.sh"
"$SCRIPT_DIR/check-storage-irq-race.sh"
printf '%s\n' 'BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 app_data_async_recovery=1 storage_server_static=1 storage_server_recovery_static=1 storage_server_repeated_recovery_static=1 storage_server_async_recovery_static=1 storage_server_fault_policy_static=1 storage_server_owner_liveness_static=1 storage_server_terminal_quarantine_static=1 storage_server_persistent_health_static=1 storage_server_clean_shutdown_static=1 storage_server_shutdown_orchestration_static=1 resident_platform_shutdown_static=1 unified_product_static=1 unified_product_liveness_static=1 unified_product_multiservice_liveness_static=1 unified_product_psci_shutdown_static=1 unified_product_continuous_supervision_static=1 unified_product_manifest_supervision_static=1 unified_product_event_supervision_static=1 unified_product_verified_manifest_static=1 unified_product_persistent_rollback_static=1 unified_product_key_rotation_static=1 unified_product_maintenance_authorization_static=1 unified_product_maintenance_execution_static=1 unified_product_maintenance_step_static=1 unified_product_maintenance_plan_static=1 unified_product_signed_maintenance_plan_static=1 storage_recovery_unification_static=1 storage_server_runtime_boots=3 storage_server_recovery=1 storage_server_repeated_recovery=1 storage_server_async_recovery=1 storage_server_fault_policy=1 storage_server_owner_liveness=1 storage_server_terminal_quarantine=1 storage_server_persistent_health_reboot=1 persistent_health_boots=2 storage_server_clean_shutdown_reboot=1 clean_shutdown_boots=2 storage_server_shutdown_orchestration_reboot=1 shutdown_orchestration_boots=2 resident_platform_shutdown_reboot=1 resident_platform_shutdown_boots=2 unified_product_reboot=1 unified_product_boots=2 unified_product_ui_interactions=2 unified_product_liveness_reboot=1 unified_product_liveness_boots=2 unified_product_liveness_recoveries=2 unified_product_multiservice_liveness_reboot=1 unified_product_multiservice_liveness_boots=2 unified_product_multiservice_liveness_recoveries=2 unified_product_psci_shutdown_reboot=1 unified_product_psci_shutdown_boots=2 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 unified_product_manifest_supervision_reboot=1 unified_product_manifest_supervision_boots=2 manifest_supervision_decodes=2 manifest_supervision_services=10 manifest_supervision_resident_bindings=8 manifest_supervision_spawns=2 manifest_open_calls=4 manifest_open_successes=2 manifest_argument_rejections=2 unified_product_event_supervision_reboot=1 unified_product_event_supervision_boots=2 event_supervision_sessions=2 event_supervision_services=10 event_supervision_clean_rotations=4 event_supervision_cancel_windows=2 event_supervision_cancelled_pending=8 event_supervision_drained_pending=0 event_supervision_process_terminate_calls=0 event_supervision_terminated_killed=0 event_supervision_restart_budget_rearms=4 unified_product_verified_manifest_reboot=1 unified_product_verified_manifest_boots=2 verified_manifest_artifact_verifications=2 verified_manifest_signature_successes=2 verified_manifest_negative_signature_boots=1 verified_manifest_negative_rollback_boots=1 verified_manifest_fail_closed_boots=2 verified_manifest_published_after_rejection=0 verified_manifest_rollback_floor=2 unified_product_persistent_rollback_reboot=1 unified_product_persistent_rollback_boots=3 persistent_rollback_artifact_verifications=5 persistent_rollback_signature_successes=4 persistent_rollback_negative_signature_boots=1 persistent_rollback_negative_rollback_boots=1 persistent_rollback_fail_closed_pre_el0_boots=2 persistent_rollback_floor=3 persistent_rollback_slot_writes=2 persistent_rollback_steady_read_only=1 unified_product_key_rotation_reboot=1 unified_product_key_rotation_boots=5 key_rotation_artifact_verifications=7 key_rotation_signature_successes=6 key_rotation_key_transitions=2 key_rotation_negative_signature_boots=1 key_rotation_negative_retired_key_boots=1 key_rotation_fail_closed_pre_el0_boots=2 key_rotation_floor=5 key_rotation_policy_writes=3 key_rotation_steady_read_only=1 unified_product_maintenance_authorization_reboot=1 unified_product_maintenance_authorization_boots=6 maintenance_authorization_sessions=2 maintenance_authorization_artifact_verifications=5 maintenance_authorization_signature_valid=4 maintenance_authorization_negative_signature_boots=1 maintenance_authorization_negative_binding_boots=1 maintenance_authorization_negative_replay_boots=1 maintenance_authorization_fail_closed_pre_el0_boots=3 maintenance_authorization_audit_writes=2 maintenance_authorization_audit_flushes=2 maintenance_authorization_report_gate_denials=2 unified_product_maintenance_execution_reboot=1 unified_product_maintenance_execution_positive_boots=2 maintenance_execution_interrupted_boots=1 maintenance_execution_fail_closed_pre_el0_boots=3 maintenance_execution_completed_replay_rejections=1 maintenance_execution_audit_sequence=2 maintenance_execution_completion_sequence=2 maintenance_execution_resume_audit_writes=0 maintenance_execution_completed_replay_slot_mutations=0 maintenance_execution_exact_binding_resume=1 maintenance_execution_predecessor_completion_required=1 unified_product_maintenance_step_reboot=1 unified_product_maintenance_step_positive_boots=5 maintenance_step_interrupted_boots=3 maintenance_step_fail_closed_pre_el0_boots=3 maintenance_step_idempotent_replays=7 maintenance_step_corruption_fallbacks=1 maintenance_step_cutpoints=3 maintenance_step_terminal_chain_bound=1 maintenance_step_completed_replay_slot_mutations=0 maintenance_step_fixed_program_steps=3 unified_product_maintenance_plan_reboot=1 maintenance_plan_positive_boots=5 maintenance_plan_interrupted_boots=4 maintenance_plan_corruption_fallbacks=1 maintenance_plan_result_unknown_reconciliations=1 maintenance_plan_effect_observed_reconciliations=1 maintenance_plan_preapply_compensations=1 maintenance_plan_fixed_transitions=9 unified_product_signed_maintenance_plan_reboot=1 signed_maintenance_plan_positive_boots=2 signed_maintenance_plan_interrupted_boots=1 signed_maintenance_plan_fail_closed_pre_el0_boots=4 signed_maintenance_plan_signature_rejections=1 signed_maintenance_plan_binding_rejections=1 signed_maintenance_plan_program_rejections=1 signed_maintenance_plan_program_ledger_rejections=1 signed_maintenance_plan_program_writes=2 signed_maintenance_plan_program_replays=1 signed_maintenance_plan_corruption_fail_closed=1 signed_plan_fixed_operations=3 signed_plan_fixed_transitions=9 signed_maintenance_plan_descriptor_bound_phases=18 trusted_monotonic_backend=0 offline_split_signing=1 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6 qemu_psci_self_exits=57 qemu_self_exits=65 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1 storage_irq_cooperative_recovery=1'
