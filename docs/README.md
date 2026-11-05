# Documentation

Public Markdown documentation is maintained in English. Paths already established on GitHub remain the canonical locations for historical milestones.

## Start here

- [Project overview](../README.md)
- [Interactive demo and smoke checks](DEMO.md)
- [Whitepaper](../WHITEPAPER.md)
- [Roadmap](../TODO.md)
- [Implementation status](../IMPLEMENTATION_STATUS.md)
- [Milestone index](../PROJECT_MILESTONES.md)

## Directory layout

| Location | Contents |
| --- | --- |
| Repository root | Project entry points, contribution policies, licenses, and build configuration |
| `docs/milestones/` | Historical OS design milestones |
| `docs/milestones/androidbox/` | AndroidBox compatibility milestones |
| `docs/milestones/ui/` | Mobile UI and graphics evidence summaries |
| `docs/assets/` | Published documentation images |
| `docs/decisions/` | Current architecture and integration decisions |
| `docs/archive/originals/` | Verbatim original-language historical records in text format |
| `crates/bndr-ui/assets/` | UI font binary asset |
| `target/` (ignored) | Local build outputs, logs, and generated evidence |

## Current UI and compatibility work

- [Settings, Display, and Accessibility](milestones/ui/SETTINGS_DISPLAY_UI.md)
- [Mapped buffering and region rendering](milestones/ui/MOBILE_MAPPED_BUFFERQUEUE.md)
- [AndroidBox Layout Mixed-19, ABI 69](milestones/androidbox/ANDROIDBOX_LAYOUT_MIXED_19.md)
- [Native arm64 entry and Linux ABI subset, ABI 70](milestones/androidbox/LINUX_ARM64_ENTRY.md)
- [Real static Bionic startup and native runtime checks](milestones/androidbox/BIONIC_STATIC_0.md)
- [Android dynamic linker, shared Bionic, and runtime files](milestones/androidbox/BIONIC_DYNAMIC_0.md)
- [Bionic threads, futex synchronization, and group cleanup](milestones/androidbox/BIONIC_THREADS_0.md)
- [ART frontend loading and low-address memory](milestones/androidbox/ART_FRONTEND_0.md)
- [ART process metadata, runtime files and native initialization](milestones/androidbox/ART_PROCESS_0.md)

Historical pages report their original profile and validation date, not universal compatibility or a fresh test run. See [the archive policy](archive/README.md) for source preservation.
