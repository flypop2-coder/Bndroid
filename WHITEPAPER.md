# Bndroid OS Whitepaper

## Abstract

Bndroid OS is a research operating system that explores Android-compatible user experiences on a Rust-first, non-Linux technology stack. The central question is how much of the Android application model can be projected onto a capability-oriented kernel, supervised services, deterministic storage, and a small native UI runtime before adopting the full Android Framework or ART.

The current answer is intentionally narrow. Bndroid can boot in QEMU, run its own user-space services, render a mobile UI surface, and execute AndroidBox fixtures through evidence-driven tests. It is not an Android distribution, a production mobile operating system, or a promise of compatibility with arbitrary APKs.

## Design goals

- Keep the trusted computing base understandable.
- Model service failure and restart behavior explicitly.
- Treat storage, package state, and UI updates as recoverable system contracts.
- Ingest APK evidence through bounded, auditable parsers.
- Use deterministic tests instead of broad compatibility claims.
- Avoid accidental dependencies on a full Android runtime until that becomes an explicit architectural choice.

## Architecture

Bndroid is organized around a custom AArch64 kernel and a set of supervised user-space services. Kernel objects expose bounded capabilities through handles, while processes communicate through channels, events, VMOs, and versioned service protocols.

The UI path contains a native compositor, frame clock, input broker, surface model, and mobile text and font pipeline. The storage path separates block access, package state, application data, persistence, and recovery policy. These boundaries are tested as product behavior, not treated as incidental implementation details.

AndroidBox is the compatibility layer. It currently focuses on controlled APK fixtures: envelope validation, manifest and resource parsing, selected DEX execution, Activity state, UI scene projection, layout geometry, and click-target tests. Each milestone records protocol versions, fixture evidence, QEMU runs, and host-test counts to make the behavior boundary verifiable.

## Current boundaries

Bndroid currently targets QEMU `virt` AArch64. AndroidBox does not yet include ART, the Android Framework, Play services, Binder compatibility, hardware-device integration, or arbitrary third-party APK support. AndroidBox gates are network-disabled by default unless a future task explicitly authorizes otherwise.

These limits are part of the research method: the project grows by adding one observable contract at a time while preserving evidence that older contracts still hold.

## Licensing position

The source code uses Apache-2.0 because it is permissive, broadly usable, and includes an explicit patent grant. For an operating-system and Android-compatibility research project, this is preferable to a minimal permissive license without patent terms.

Bundled Bndroid Sans Raster font data is tracked separately from the source-code license. It is derived from OFL-licensed font software and remains available under OFL-1.1. The repository records this split through `LICENSE`, `NOTICE`, `LICENSES/`, and `.reuse/dep5`.

## Roadmap

The near-term direction is to turn AndroidBox behavior into opt-in, evidence-driven gates covering richer layout contracts, resource semantics, Activity lifecycle coverage, storage-backed package behavior, and stricter restart supervision. A complete Android runtime or container is a separate architectural decision that requires an explicit evaluation before adoption.# Bndroid OS Whitepaper

## Abstract

Bndroid OS is a research operating system that explores Android-compatible user experiences on a Rust-first, non-Linux technology stack. The central question is how much of the Android application model can be projected onto a capability-oriented kernel, supervised services, deterministic storage, and a small native UI runtime before adopting the full Android Framework or ART.

The current answer is intentionally narrow. Bndroid can boot in QEMU, run its own user-space services, render a mobile UI surface, and execute AndroidBox fixtures through evidence-driven tests. It is not an Android distribution, a production mobile operating system, or a promise of compatibility with arbitrary APKs.

## Design goals

The project keeps the trusted computing base understandable and models service failure and restart behavior explicitly. Storage, package state, and UI updates are treated as recoverable system contracts. APK evidence is ingested through bounded, auditable parsers, and deterministic tests are preferred over broad compatibility claims. Bndroid avoids accidental dependencies on a full Android runtime until that becomes an explicit architectural choice.

## Architecture

Bndroid is organized around a custom AArch64 kernel and a set of supervised user-space services. Kernel objects expose bounded capabilities through handles, while processes communicate through channels, events, VMOs, and versioned service protocols.

The UI path contains a native compositor, frame clock, input broker, surface model, and mobile text and font pipeline. The storage path separates block access, package state, application data, persistence, and recovery policy. These boundaries are tested as product behavior, not treated as incidental implementation details.

AndroidBox is the compatibility layer. It currently focuses on controlled APK fixtures: envelope validation, manifest and resource parsing, selected DEX execution, Activity state, UI scene projection, layout geometry, and click-target tests. Each milestone records protocol versions, fixture evidence, QEMU runs, and host-test counts to make the behavior boundary verifiable.

## Current boundaries

Bndroid currently targets QEMU `virt` AArch64. AndroidBox does not yet include ART, the Android Framework, Play services, Binder compatibility, hardware-device integration, or arbitrary third-party APK support. AndroidBox gates are network-disabled by default unless a future task explicitly authorizes otherwise.

These limits are part of the research method: the project grows by adding one observable contract at a time while preserving evidence that older contracts still hold.

## Licensing position

The source code uses Apache-2.0 because it is permissive, broadly usable, and includes an explicit patent grant. For an operating-system and Android-compatibility research project, this is preferable to a minimal permissive license without patent terms.

Bundled Bndroid Sans Raster font data is tracked separately from the source-code license. It is derived from OFL-licensed font software and remains available under OFL-1.1. The repository records this split through `LICENSE`, `NOTICE`, `LICENSES/`, and `.reuse/dep5`.

## Roadmap

The near-term direction is to turn AndroidBox behavior into opt-in, evidence-driven gates covering richer layout contracts, resource semantics, Activity lifecycle coverage, storage-backed package behavior, and stricter restart supervision. A complete Android runtime or container is a separate architectural decision that requires an explicit evaluation before adoption.
