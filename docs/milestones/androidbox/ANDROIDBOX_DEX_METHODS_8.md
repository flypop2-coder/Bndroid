# AndroidBox DEX Methods-8 (ABI 58)

Allows a real APK callback to call an APK-defined static helper and pass its returned resource ID to TextView.setText(int). The supported helper maps the clicked view ID to one of two status resources. This is a deliberately bounded method-call boundary in the existing DEX interpreter; earlier profiles retain their direct-callback behavior.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-dex-methods8`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_DEX_METHODS_8.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
