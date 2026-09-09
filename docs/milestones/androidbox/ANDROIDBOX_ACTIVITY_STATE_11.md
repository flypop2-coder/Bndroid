# AndroidBox Activity State-11 (ABI 61)

Retains both a private TextView reference and a private integer counter in one Activity session. Real SDK/D8 callback code reads, increments, and writes the counter initialized by onCreate. Session state remains bounded and identity-qualified; it is not a general managed heap or persistent Activity-state service.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-activity-state11`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_ACTIVITY_STATE_11.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
