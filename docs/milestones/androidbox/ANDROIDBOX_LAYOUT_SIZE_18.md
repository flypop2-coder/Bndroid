# AndroidBox Layout Size-18 (ABI 68)

Supports exact positive integer dp layout widths and heights in AAPT2 TYPE_DIMENSION encoding, limited to 1..255dp. Zero width remains reserved for weighted horizontal Buttons. Fractional, negative, oversized, unknown, duplicate, and unsupported-unit values fail closed. Exact dimensions and per-side spacing use checked arithmetic and the same final rectangles for rendering, pressed feedback, and hit testing. BNDAPC14 v14 uses canonical 24-byte scene descriptors v5. The ABI 69 child adds fixed-and-weighted rows; general ViewGroup, RTL, scrolling, and arbitrary APK support remain unimplemented.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-layout-size18`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_LAYOUT_SIZE_18.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
