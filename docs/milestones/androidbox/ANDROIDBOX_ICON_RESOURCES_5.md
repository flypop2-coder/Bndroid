# AndroidBox Icon Resources-5 (ABI 56)

Resolves application android:icon references from real SDK APKs and binds the decoded icon to the installed package identity. Launcher and Settings display distinct E and C fixture icons. Resource admission requires a unique default drawable/mipmap PNG at its canonical APK path, a STORED entry with a valid CRC, and the documented 16 KiB entry bound. Unsupported resource/PNG forms fail closed.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-icon-resources5`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_ICON_RESOURCES_5.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
