# AndroidBox Restart-0 (ABI 48)

Adds one explicitly authorized recovery of an AndroidApp EL0 worker after an injected lower-stack-guard fault. Authorization binds the exact App PID, worker PID, compatible session, package generation, and fault point. The reaper validates the observed process fault against that authorization before recovery. Process-0 remains unchanged when the child profile is disabled.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-restart0`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_RESTART_0.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
