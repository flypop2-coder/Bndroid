# Bndroid OS

Bndroid OS is an experimental Rust-first operating system project that explores
Android application compatibility without becoming a Linux distribution.

The repository currently contains a custom AArch64 kernel, a supervised service
model, storage and UI subsystems, and AndroidBox: a compatibility layer for
verifiable APK ingestion, resource parsing, DEX interpretation, Activity/UI
projection, lifecycle supervision, and deterministic evidence-driven tests.

This is research software. It is not a drop-in Android replacement, not a
general-purpose APK runtime, and not ready for production devices.

## What Is Here

- A small Rust kernel for the QEMU `virt` AArch64 platform.
- Capability-oriented kernel objects, handles, VMOs, channels, events, and
  supervised processes.
- Storage, package, graphics, input, and application-data services with
  recovery-oriented tests.
- AndroidBox milestones through layout size parsing and scene projection for
  controlled Android SDK/D8/AAPT2 fixtures.
- Offline UI font data bundled as Bndroid Sans Raster.

## Quick Start

```sh
./scripts/build-kernel.sh
./scripts/check-qemu-boot.sh
```

The focused roadmap lives in [TODO.md](TODO.md). The detailed historical
milestone log that used to live on this page is preserved in
[PROJECT_MILESTONES.md](PROJECT_MILESTONES.md).

## Documents

- [WHITEPAPER.md](WHITEPAPER.md): short project whitepaper and technical
  positioning.
- [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md): implementation status
  notes.
- [PROJECT_MILESTONES.md](PROJECT_MILESTONES.md): long-form AndroidBox and OS
  milestone archive.

## License

Source code is licensed under the Apache License, Version 2.0. Font assets named
Bndroid Sans Raster remain under the SIL Open Font License 1.1 because they are
derived from OFL-licensed font software.

See [LICENSE](LICENSE), [NOTICE](NOTICE), and [.reuse/dep5](.reuse/dep5) for the
repository-level license map.
