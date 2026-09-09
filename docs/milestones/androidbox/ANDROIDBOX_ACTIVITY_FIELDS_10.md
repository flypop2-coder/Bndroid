# AndroidBox Activity Fields-10 (ABI 60)

Retains a private Activity field holding a layout TextView between onCreate(Bundle) and later onClick(View) callbacks. The bounded interpreter retrieves the reference from the same Activity session before updating text. This extends DEX Instance-9 without granting general Java object or Android Framework support.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-activity-fields10`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_ACTIVITY_FIELDS_10.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
