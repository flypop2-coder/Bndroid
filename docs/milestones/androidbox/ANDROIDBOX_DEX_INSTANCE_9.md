# AndroidBox DEX Instance-9 (ABI 59)

Allows a real SDK/D8 onClick callback to invoke a private instance helper with the Activity receiver and clicked View. The helper calls View.getId() and returns the corresponding string resource ID. Receiver and argument semantics stay within the allocation-free, fail-closed DEX subset.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-dex-instance9`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_DEX_INSTANCE_9.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
