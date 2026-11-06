# ADR 0001: Frozen local Android runtime inputs

Status: accepted for the local integration work, November 6, 2026.

## Decision

Use the already installed arm64 Android 16 SDK system image, API 36.1 / extension 20, revision 3, as the initial coherent prebuilt runtime source. Its observed fingerprint is `google/sdk_gphone64_arm64/emu64a:16/BP41.250916.009.A1/14246511:user/release-keys`. Keep the tested NDK r27b/API 35 native toolchain. The final application compatibility matrix remains broader work; this selection does not redefine the goal as compatibility with one fixture or API level.

Reuse the image's unmodified ART, Bionic, linker, core Java libraries, and their required open-source runtime dependencies. Begin with the actual `dalvikvm64`/ART entry, then move to ActivityThread and Framework integration. Interpret-only ART startup is a diagnostic option while JIT/AOT memory and file requirements are implemented; it is not a replacement runtime or the final performance target.

Read these inputs through an isolated temporary, offline, read-only SDK emulator. Select only the owned emulator serial, retain per-file paths/sizes/SHA-256 hashes and source metadata, and stop the owned process after extraction. Keep all extracted binaries, archives and source notices in ignored local test directories. Do not modify the installed SDK, import third-party source/binaries into Git, download AOSP, or operate physical devices under this decision.

## Provenance, licensing and distribution

Preserve the installed image's notices alongside local inputs. Record component identities and dependencies before using them. The source image also contains proprietary services; do not copy Play services or other Google application/service packages into the runtime root. The extraction manifest is an inventory and reproducibility record, not a license grant.

Repository deliverables are Bndroid adaptation code, owned test applications, input preparation tools, documentation, and compatibility evidence. Any redistributable runtime package needs a separate component/license/source review. A full AOSP checkout, a large repository import, or SDK changes remain separate actions under the roadmap's authorization rule. This decision uses local copies of already installed runtime files and introduces no paid service or additional download.

## Resource and maintenance policy

Keep each initial extracted runtime set below 512 MiB and measure actual component sizes before increasing kernel limits. Use one QEMU CPU and an initial 512 MiB guest-RAM budget for the ART bring-up; preserve the native and mobile regression profiles. The temporary source emulator retains its existing two-core/2 GiB configuration and disposable disk. Current host storage availability is approximately 13 GiB, so an AOSP source/build tree is outside this iteration's resource envelope.

Do not silently consume newer SDK components. An input change must produce a new fingerprint/hash receipt and pass native, thread, managed-runtime, fault-containment, and existing mobile regressions appropriate to the change. Keep the old receipt for comparison. Pin additional ART/APEX identities in the generated manifest once inspected. No automatic updater or unassigned ongoing staffing commitment is implied.

The subsequent Framework inventory measures 347 files / 265,049,386 bytes, including 46 source boot jars and platform resources. It retains all 66 earlier ART inputs unchanged and remains below the 512 MiB acquisition cap. This measured set justifies a separate `framework-runtime` profile with 1 GiB guest RAM, a 320 MiB immutable root and a 352 MiB bounded RW/NX kernel image window. ART-only and mobile profiles keep their original limits. Its public boot properties describe the implemented arm64 ABI, absence of a native bridge and Bndroid product identity; they do not reproduce the SDK device's private state or supply a writable Android property service.

## Consequences and remaining work

This provides a concrete, reproducible source for testing the custom kernel against real Android runtime behavior without a large source acquisition. Android's namespace, property, signal, memory, filesystem, Binder, and service semantics still have to be implemented and verified. Booting an SDK emulator to read files is never evidence that Bndroid runs Android applications. The acceptance levels in `TODO.md` remain unchanged and incomplete until their full requirements pass on Bndroid.
