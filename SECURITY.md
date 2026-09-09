# Security Policy

Bndroid OS is research system software. Report security issues responsibly and keep exploitable details out of public issues, pull requests, and chat.

## Supported versions

Only the latest code on `main` is supported. Historical milestones, old evidence, experimental branches, and local modifications do not receive long-term security support.

## Reporting a vulnerability

Report the following privately:

- Authorization bypass, unauthorized access, memory corruption, or persistent control.
- Storage-integrity, package-state, application-data, or recovery failures.
- AndroidBox parsing or execution issues that could form an exploit chain.
- Exposure of credentials, tokens, personal data, or confidential security materials.

The project has no dedicated security email address. Prefer GitHub private vulnerability reporting when available. If it is disabled, contact the repository owner to enable a private channel before sharing details. A public issue may request a private security reporting channel, but must not include exploit payloads, proof-of-concept exploits, or sensitive logs.

## Response process

Maintainers first establish impact and reproducibility. Public details should remain limited until a fix is available; afterward, publish the necessary summary, affected scope, and mitigations. Do not test against real users, third-party services, or devices without authorization.

Security research should improve reliability, isolation, and auditability. Contributions intended to attack, steal, damage, bypass authorization, or abuse existing platforms are not accepted.
