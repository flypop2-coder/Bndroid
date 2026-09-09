# AndroidBox Runtime Install-2 (ABI 53)

Moves installation from automatic boot behavior to an explicit Settings > Apps confirmation transaction. A virgin volume offers Install, a newer matching identity/signer offers Update, and a matching non-rollback tombstone offers Reinstall. Opening or cancelling confirmation does not write the disk. Final confirmation starts a crash-safe asynchronous transaction, updates the live catalog, and permits same-boot launch. Source-free later recovery remains write/flush free.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-runtime-install2`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_RUNTIME_INSTALL_2.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
