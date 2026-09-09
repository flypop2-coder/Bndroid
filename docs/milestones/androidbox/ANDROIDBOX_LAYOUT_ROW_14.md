# AndroidBox Layout Row-14 (ABI 64)

Supports nested horizontal LinearLayout in controlled SDK/D8 APKs on the 720x1600 UI. The root stays vertical; supported nodes are LinearLayout, TextView, and Button, with at most eight nodes and depth three in preorder. The original row contract divides visible children into bounded equal columns. Later weight, spacing, and exact-size behavior is covered by separate profiles.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-layout-row14`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_LAYOUT_ROW_14.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
