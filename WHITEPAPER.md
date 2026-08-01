# Bndroid OS Whitepaper

[中文](WHITEPAPER.zh-CN.md) | English

## Summary

Bndroid OS is a research operating system for exploring Android-compatible user
experiences on a Rust-first, non-Linux stack. The project asks a narrow question:
how much of the Android application model can be projected onto a capability
oriented kernel, supervised services, deterministic storage, and a small native
UI runtime before a full Android framework or ART dependency is required?

The current answer is intentionally bounded. Bndroid can boot in QEMU, run its
own userspace services, render mobile UI surfaces, and exercise AndroidBox
fixtures through evidence-driven tests. It is not yet an Android distribution,
not a production mobile OS, and not a compatibility promise for arbitrary APKs.

## Design Goals

- Keep the trusted computing base understandable.
- Make service failure and restart behavior explicit.
- Treat storage, package state, and UI updates as recoverable system contracts.
- Ingest APK evidence through bounded, auditable parsers.
- Prefer deterministic tests over broad claims of compatibility.
- Avoid accidental dependency on a full Android runtime until that tradeoff is
  deliberately evaluated.

## Architecture

Bndroid is organized around a custom AArch64 kernel and a set of supervised
userspace services. Kernel objects expose bounded capabilities through handles;
processes communicate through channels, events, VMOs, and service protocols.

The UI path uses a native compositor, frame clock, input broker, surface model,
and mobile text/font pipeline. The storage path separates block access, package
state, app data, persistence, and recovery policy. These boundaries are tested
as product behavior, not only as internal implementation details.

AndroidBox is the compatibility layer. Today it focuses on controlled APK
fixtures: envelope validation, manifest/resource parsing, selected DEX
execution, Activity state, UI scene projection, layout geometry, and click hit
testing. Each milestone records the protocol version, fixture evidence, QEMU
runs, and host test counts needed to prove the behavior.

## Current Boundaries

Bndroid currently targets QEMU `virt` on AArch64. AndroidBox does not include
ART, the Android framework, Play services, Binder compatibility, hardware
device integration, or arbitrary third-party APK support. Network use in the
recorded AndroidBox gates is intentionally disabled unless a future task
explicitly authorizes otherwise.

These limits are part of the research method: the project grows by adding one
observable contract at a time and preserving the evidence that the previous
contracts still hold.

## Licensing Position

The source code uses Apache-2.0 because it is permissive, widely understood, and
includes an express patent grant. That is a better default for an OS and Android
compatibility research project than a minimal permissive license with no patent
language.

Bundled Bndroid Sans Raster font data is separate from the source-code license.
It remains under OFL-1.1 because it is derived from OFL-licensed font software.
The repository records that split in `LICENSE`, `NOTICE`, `LICENSES/`, and
`.reuse/dep5`.

## Roadmap

The near-term path is to keep turning AndroidBox behavior into opt-in,
evidence-backed gates: richer layout contracts, resource semantics, Activity
lifecycle coverage, storage-backed package behavior, and tighter restart
supervision. A full Android runtime/container is a separate architectural
decision and should be evaluated explicitly before adoption.
