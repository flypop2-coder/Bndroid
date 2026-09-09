# AndroidBox MultiPackage-4 (ABI 55)

Combines two crash-safe single-package volumes into a fixed two-package store with stable volume order. Two separately named, SDK-built, APK-v2-signed packages install on one disk, appear as separate Launcher icons, and open distinct Activities. A later boot recovers and launches both without fw_cfg APK sources and without recovery writes or disk-byte changes. Capacity remains two; this is not a general PackageManager.

## Scope and evidence

This page is an English summary of the historical milestone, not a new compatibility claim or a fresh validation run. The original record preserves the complete acceptance rules, negative cases, protocol details, commands, artifact paths, and measurements.

- Profile: `androidbox-multipackage4`.
- [Complete original evidence](../../archive/originals/ANDROIDBOX_MULTIPACKAGE_4.txt).
- [Milestone index](../../../PROJECT_MILESTONES.md).
- [Current roadmap](../../../TODO.md).

Commands in the source record run from the repository root. Generated `target/` evidence is local and ignored by Git. Tests described by these milestones use controlled fixtures and do not establish arbitrary APK compatibility or physical-phone readiness.
