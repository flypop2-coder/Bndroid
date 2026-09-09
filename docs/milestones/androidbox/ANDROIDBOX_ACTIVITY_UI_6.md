# AndroidBox Activity UI-6

Separates application content from developer diagnostics. Foreground Activities display trusted system bars, the application title and verified icon, and bounded APK TextView/Button content. Package identity, version, signature, digest, and compatibility details remain in Settings > Apps. Existing callback geometry remains unchanged, and internal revision-only updates do not draw diagnostic pixels. This UI milestone adds no syscall or wire version.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-icon-resources5`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_ACTIVITY_UI_6.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
