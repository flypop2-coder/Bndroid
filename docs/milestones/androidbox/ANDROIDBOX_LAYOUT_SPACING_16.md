# AndroidBox Layout Spacing-16 (ABI 66)

Supports uniform android:padding and android:layout_margin as exact integer dp dimensions in the range 0..16. Nonzero padding applies only to LinearLayout; nonzero margins apply only to Button. Values flow through the worker protocol into shared layout/raster/hit-test geometry. Unsupported forms and overflow are rejected.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-layout-spacing16`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_LAYOUT_SPACING_16.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
