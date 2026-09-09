# Contributing to Bndroid OS

Bndroid is research software. Contributions should make a focused, reproducible improvement with clear ownership and compatibility boundaries.

## Workflow

1. Fork the repository and create a feature branch in your fork.
2. Keep each pull request focused on one problem or a small related topic.
3. Explain the motivation, changed behavior, validation commands, and known risks.
4. Document ABI, protocol, storage-format, and user-visible compatibility changes.
5. Wait for maintainer review; external contributions should not push directly to `main`.

## Priorities

- Kernel objects, processes, memory, scheduling, channels, and events.
- Storage, package state, application data, and recovery.
- Graphics, input, surfaces, composition, and mobile UI.
- AndroidBox APK envelopes, manifests, resources, bounded DEX execution, Activity lifecycle, and layout semantics.
- Reproducible host tests, QEMU gates, stable scripts, and evidence records.
- English documentation, architecture explanations, and developer onboarding.

See [TODO.md](TODO.md) for the authoritative roadmap. Keep subsystem documentation next to its code and milestone reports under `docs/milestones/`. Do not add ad-hoc reports, generated binaries, screenshots, or logs to the repository root. Local generated evidence belongs in `target/`.

## Validation

New behavior needs tests or reproducible validation steps. Security-sensitive changes must describe the threat model and failure modes. Retain negative tests and recovery evidence. Run relevant host tests and QEMU gates, and report exactly which checks passed or could not run.

Do not commit personal tokens, private keys, sensitive logs, or large generated evidence directories. Repository-owned test signing fixtures are explicitly documented as public test material and must never be used for production signing.

## Contribution boundaries

The project does not accept changes intended to attack, damage, or abuse Android, Google services, device vendors, third-party applications, or other platforms. This includes unauthorized access, payment or DRM bypass, credential theft, detection evasion, and unauthorized persistent control.

Do not download or embed large third-party source trees, SDKs, system images, or proprietary components without explicit authorization. Avoid broad, untested rewrites and unrelated promotion, arguments, or personal attacks.

Merge decisions depend on evidence, clear boundaries, and maintainability.
