# AndroidBox Density Icons-7 (ABI 57)

Selects a unique mdpi 160 dpi drawable/mipmap PNG from the verified APK resource table, falling back to the supported default 16x16 icon. A 48x48 RGBA8 source is normalized with a premultiplied-alpha-aware 3x3 box filter into the canonical 16x16 system icon. Deterministic quantization preserves the fixed palette budget. BNDAIC01 v2 carries source size, density, normalization, and quantization provenance.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-density-icons7`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_DENSITY_ICONS_7.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
