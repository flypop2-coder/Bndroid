# Project milestones

This index follows the milestone locations established on GitHub. English pages summarize historical scope; original multilingual evidence remains available in the [source archive](docs/archive/README.md). For the former long-form overview, see [the complete original milestone log](docs/archive/originals/PROJECT_MILESTONES.txt).

[TODO.md](TODO.md) is the sole forward-looking roadmap. [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) summarizes implemented behavior. Historical evidence does not imply that an old preview process is still running or that every historical gate was rerun for the latest publication.

## Current UI and graphics

- [Settings, Display, and Accessibility](docs/milestones/ui/SETTINGS_DISPLAY_UI.md): grouped settings, appearance, contrast, and guarded-stack regression.
- [Mapped buffers and region rendering](docs/milestones/ui/MOBILE_MAPPED_BUFFERQUEUE.md): mapped double buffering, BufferPresent v6, copied-layer savings, chrome reuse, and cancellation recovery.
- [Current session interactions](IMPLEMENTATION_STATUS.md#session-owned-interactions): minute clocks, lock notifications, and Overview gestures/icons.

## AndroidBox history

| Milestone | Recorded scope |
| --- | --- |
| [Local APK](docs/milestones/androidbox/ANDROIDBOX_LOCAL_APK.md) | Local APK ingestion and early compatibility evidence |
| [APK Install-0](docs/milestones/androidbox/ANDROIDBOX_APK_INSTALL_0.md) | Durable bounded package installation |
| [APK Update-0](docs/milestones/androidbox/ANDROIDBOX_APK_UPDATE_0.md) | Persistent package updates |
| [APK Uninstall-0](docs/milestones/androidbox/ANDROIDBOX_APK_UNINSTALL_0.md) | Persistent removal and recovery |
| [Restart-0](docs/milestones/androidbox/ANDROIDBOX_RESTART_0.md) | Authorized AndroidApp worker recovery, ABI 48 |
| [Scene-RPC-2](docs/milestones/androidbox/ANDROIDBOX_SCENE_RPC_2.md) | Bounded scene transfer, ABI 49 |
| [MultiAction-3](docs/milestones/androidbox/ANDROIDBOX_MULTIACTION_3.md) | Two real APK callbacks, ABI 50 |
| [APK Envelope-4](docs/milestones/androidbox/ANDROIDBOX_APK_ENVELOPE_4.md) | Bounded common APK ZIP envelopes, ABI 51 |
| [Runtime Uninstall-1](docs/milestones/androidbox/ANDROIDBOX_RUNTIME_UNINSTALL_1.md) | Settings-driven removal, ABI 52 |
| [Runtime Install-2](docs/milestones/androidbox/ANDROIDBOX_RUNTIME_INSTALL_2.md) | Confirmed install/update/reinstall, ABI 53 |
| [Manifest Catalog-3](docs/milestones/androidbox/ANDROIDBOX_MANIFEST_CATALOG_3.md) | Component and permission catalog, ABI 54 |
| [MultiPackage-4](docs/milestones/androidbox/ANDROIDBOX_MULTIPACKAGE_4.md) | Two-package storage and recovery, ABI 55 |
| [Icon Resources-5](docs/milestones/androidbox/ANDROIDBOX_ICON_RESOURCES_5.md) | Verified APK icons, ABI 56 |
| [Activity UI-6](docs/milestones/androidbox/ANDROIDBOX_ACTIVITY_UI_6.md) | Clean application content and Settings diagnostics |
| [Density Icons-7](docs/milestones/androidbox/ANDROIDBOX_DENSITY_ICONS_7.md) | mdpi selection and normalization, ABI 57 |
| [DEX Methods-8](docs/milestones/androidbox/ANDROIDBOX_DEX_METHODS_8.md) | Bounded static helper calls, ABI 58 |
| [DEX Instance-9](docs/milestones/androidbox/ANDROIDBOX_DEX_INSTANCE_9.md) | Private instance helper calls, ABI 59 |
| [Activity Fields-10](docs/milestones/androidbox/ANDROIDBOX_ACTIVITY_FIELDS_10.md) | Retained TextView references, ABI 60 |
| [Activity State-11](docs/milestones/androidbox/ANDROIDBOX_ACTIVITY_STATE_11.md) | Retained integer state, ABI 61 |
| [String Text-12](docs/milestones/androidbox/ANDROIDBOX_STRING_TEXT_12.md) | Literal String text updates, ABI 62 |
| [String Builder-13](docs/milestones/androidbox/ANDROIDBOX_STRING_BUILDER_13.md) | Dynamic counter strings, ABI 63 |
| [Layout Row-14](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_ROW_14.md) | Horizontal layout rows, ABI 64 |
| [Layout Weight-15](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_WEIGHT_15.md) | Integer-weighted Buttons, ABI 65 |
| [Layout Spacing-16](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_SPACING_16.md) | Uniform dp padding/margins, ABI 66 |
| [Layout Directional-17](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_DIRECTIONAL_17.md) | Per-side padding/margins, ABI 67 |
| [Layout Size-18](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_SIZE_18.md) | Exact positive integer dp dimensions, ABI 68 |
| [Layout Mixed-19](docs/milestones/androidbox/ANDROIDBOX_LAYOUT_MIXED_19.md) | Fixed-and-weighted horizontal Buttons, ABI 69 |

## Historical OS design records

- [Bndroid OS 3000](docs/milestones/Bndroid_OS_3000.md)
- [Bndroid OS 10000](docs/milestones/Bndroid_OS_10000.md)
- [Bndroid OS 30000](docs/milestones/Bndroid_OS_30000.md)
- [Bndroid OS 120000](docs/milestones/Bndroid_OS_120000.md)

These records contain both historical proposals and implementation evidence. Use the current roadmap to determine future priorities, and reproduce the relevant profile's gate before claiming a new validation result.
