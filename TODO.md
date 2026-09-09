# Bndroid architecture and product roadmap

This is the single source of truth for future work. [README.md](README.md) introduces the project; [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) records implemented behavior. Historical design documents preserve evidence rather than competing task lists. The full original planning record is retained in the [source archive](docs/archive/originals/TODO.txt).

## 1. Product goal

Build a secure, responsive, maintainable mobile operating system capable of running Android applications. Comparisons with Android or iOS must be supported by measurements:

- Complete phone interaction: lock screen, home, notifications, control center, settings, multitasking, IME, and accessibility.
- Stable frame times, low input latency, correct window and application lifecycles, and recoverable services.
- Networking, audio, cameras, sensors, telephony, storage, permissions, OTA, and backup/recovery.
- Least privilege, application isolation, signed installation, verified boot, protected keys, and auditable access.
- Progressive validation of real APKs, ART, Bionic, Binder, JNI, Framework APIs, and compatibility tests.

The current QEMU prototype validates a bounded APK/DEX/resource/layout subset. It does not yet provide a general Android runtime or a physical-phone product.

## 2. Architecture principles

1. Split processes when authority, recovery, real-time requirements, or third-party ABIs differ.
2. Split crates when dependency direction, reuse, or independent testing requires it.
3. Give each product fact one owner. Android adapters query native services instead of duplicating permission, package, window, networking, or media databases.
4. Keep scheduling, memory, IPC, handles/capabilities, interrupts, and minimal hardware mechanisms in the kernel; put product policy in userspace.
5. Centralize public wire types and versions in one ABI layer. Use ordinary Rust types internally.
6. Establish semantic equivalence and regression evidence before consolidation or deletion; apply the removal criteria below.
7. Reuse established upstream Android runtime implementations. Do not grow the bounded interpreter into a second complete ART, Bionic, or Framework implementation.

## 3. Target architecture

### Protection domains

These are responsibility boundaries, not a requirement for one crate or repository per component.

| Domain | Responsibility and isolation rationale |
| --- | --- |
| Kernel | Memory, scheduling, IPC, capabilities, interrupts, and minimal device mechanisms; keep privileged code small and auditable |
| Init / Supervisor | Boot, identities, dependency order, restart budgets, degradation, and shutdown; must survive other service failures |
| Runtime Broker | Service discovery, endpoint publication, identity binding, and revocation |
| Core System | Package catalog, permission policy, application lifecycle, account/settings coordination |
| Display Server | Surfaces, window tree, composition, animation, VSync, screenshots, and secure display policy under one frame owner |
| Input Server | Device-event normalization, focus routing, gestures, and IME interfaces |
| Storage Server | Block I/O, journal, app data, package blobs, and durable recovery under one storage-write authority |
| Device Hosts | Driver families isolated by authority and fault domain |
| Network / Media | Network policy, audio, cameras, and codecs with separate untrusted-data and real-time boundaries |
| AndroidBox | Per-app Android sandboxes, runtime, and Binder/Framework adaptation |
| Shell / Apps | Launcher, System UI, Settings, and normal apps with distinct identities |

Early QEMU builds may temporarily host inactive Network/Media control modules in Core System while keeping interfaces and capabilities separate. Before admitting real networking, codecs, camera, or microphone data, move processing into independent domains or restricted workers.

### Planned crate consolidation

This is a proposed migration, not an instruction to rename crates immediately.

| Target | Existing code and intended treatment |
| --- | --- |
| `bndr-abi` | Retain as the public wire, error-code, and version owner |
| `bndr-loader` | Generalize `bndr-elf`; keep Android parsing separate |
| `bndr-runtime` | Consolidate `bndr-sm` and shared registry, generation, supervisor, and restart state machines |
| `bndr-graphics` | Consolidate `bndr-ui` and `bndr-compositor`; share scene/layout/raster/composition |
| `bndr-input` | Retain device parsing; reuse shared graphics geometry for hit testing |
| `bndr-persistence` | Consolidate storage, appdata, and package-store libraries around shared transaction/recovery primitives |
| `bndr-androidbox` | Retain compatibility code using public ABI, loader, graphics, and persistence interfaces |

Library consolidation must not change process authority. Sharing transaction code does not give Android applications StorageServer handles.

### Service consolidation

Window management, composition, VSync, animation, screenshots, and secure display are DisplayServer modules sharing one frame transaction. Driver discovery, startup, and recovery belong to Init/Supervisor; drivers retain DeviceHost isolation. Package, permission, and application control belong to typed CoreSystem modules with separate schemas and checks. Audio, media, and camera control may share a Media service while untrusted processing stays in restricted workers. Logs, traces, and audits share event/timestamp formats with distinct access and retention policies. Shell applications share UI components and tokens but retain separate identities.

Never merge kernel and product/runtime policy, supervisor and untrusted parsers, application and input authority, storage-write authority and package control, DisplayServer and application renderers, or the address spaces/data/permissions/Binder identities of separate applications. AndroidBox maps CoreSystem permission decisions rather than maintaining an independent authority.

## 4. Android compatibility route

Use upstream Android runtime components plus a Bndroid adaptation layer. Keep the strict APK/DEX/Resources implementation for admission, format validation, test oracles, and minimal recovery UI. Select a frozen Android API/AOSP baseline before integrating ART, Bionic, the linker, userspace Binder, core Framework, and resources.

The AndroidBox personality supplies required syscall, memory, thread, signal, file, time, and shared-memory semantics. The Android Service Bridge maps Package/Activity/Window/Input/Permission/Connectivity/Media APIs to native services. Each Android app has a separate UID, process, data directory, and capability set; JNI and native libraries remain in its sandbox.

Decide versions, licensing, disk/build resources, updates, and maintenance costs before acquiring external components. AOSP downloads, large third-party imports, and local SDK modification require explicit authorization. Google Play services and proprietary components require separate licensing and distribution decisions.

### Acceptance levels

- [x] A0: Repository-owned bounded APK signing, installation, resources, DEX subset, and QEMU UI loop.
- [ ] A1: ART and a real ActivityThread running an offline Java/Kotlin APK.
- [ ] A2: Binder transaction/reply/death, ServiceManager, and a minimal Framework service loop.
- [ ] A3: Bionic, dynamic linking, JNI, and an APK containing an arm64 native library.
- [ ] A4: General Resources/View, lifecycle, intents, permissions, scoped storage, networking, and media.
- [ ] A5: A target-API compatibility matrix and a set of real, non-customized applications.
- [ ] A6: Product-level performance, power, background policy, security, and upgrade compatibility.

Before A5, describe only the validated subsets. Required capabilities include ART/GC/JIT or AOT, DEX/OAT, ActivityThread, Bionic/TLS/signals/JNI, Binder identity propagation, Framework/resources/graphics, service bridges, API/ABI tracking, crash attribution, and per-app kill switches. These remain necessary even when deferred.

## 5. Product work

Tasks follow dependencies, not optimistic calendar dates. Each stage requires code, tests, appropriate QEMU or device evidence, failure paths, and documented limits.

### P0: Establish the baseline

- [x] Record ABI 69 Layout Mixed-19 host tests, parent-profile regressions, QEMU evidence, and screenshots.
- [ ] Inventory crate, binary, feature, wire, and persistent-schema dependencies.
- [ ] Record every runtime process's owner, authority, inputs, outputs, restart policy, and resource bounds.
- [x] Make this file the only forward-looking task list.

### P1: Consolidate without changing behavior

- [ ] Introduce shared runtime generation/registry/supervisor state machines.
- [ ] Share scene geometry across layout, rasterization, hit testing, and composition.
- [ ] Share journal/checksum/dual-slot/recovery primitives before migrating appdata and package store; preserve disk formats initially.
- [ ] Move echo and milestone-only binaries into test/fixture profiles outside product images.
- [ ] Retain thin compatibility facades until the full migration gates pass.

### P2: Consolidate protocols and builds

- [ ] Migrate milestone feature chains toward `product-phone`, `android-compat`, and `qemu-ci` profiles.
- [ ] Retain old feature aliases until parent ABI and disk-recovery gates pass.
- [ ] Use a stable envelope with capability negotiation; change ABI versions for wire changes.
- [ ] Generate codecs, length assertions, and compatibility matrices instead of duplicated constants.
- [ ] Parameterize repeated fixture builds, QEMU launches, screenshots, and log checks.

### P3: Phone experience

- [x] Give Launcher/App two mapped 720x1600 backings each in mobile and bounded DEX0 profiles, with producer RW and SurfaceServer RO mappings.
- [x] Use exact logical slots and Present/Discard transactions, up to two canonical non-overlapping regions in 64-byte BufferPresent v6, and v5 decoding compatibility.
- [x] Share one software frame clock: a 100 Hz logical timer divided by two provides nominal 50 Hz grants; accepted commits acquire one consecutive epoch and cancelled frames consume none.
- [x] Support depth-two asynchronous one-ahead frame transactions, with QEMU checks for dual queueing, in-flight frames, and publication.
- [x] Force Full for first frames, focus-baseline changes, and Discard; use clipped raster, regional copy, and matching compositor damage for supported changes.
- [x] Bound pressed-target damage to shared control geometry, including scrolling, notification displacement, package selection, and Android scene buttons.
- [x] Plan up to two semantic regions for clocks, Phone/Calculator output plus keys, and real APK text/button callbacks without writing the intervening gap.
- [x] Publish SurfaceServer-owned minute-clock revisions after bounded PL031 rereads; recorded QEMU evidence covers 09:41 to 09:42 and 122,336 pixels across two clock regions.
- [x] Render/copy compact Android content regions and independently cache system chrome; preserve layer ownership, generations, and complete-source XRGB validation.
- [x] Keep written and presented scene baselines separate so cancelled frames recover correctly. Fall back to Full for dynamic-scene structure/identity/geometry changes, text overflow, or excess region count.
- [ ] Measure P50/P95/P99 frame times, input latency, and memory before choosing mapped multi-slot layers or triple buffering; define mapping rights and lifecycle first.
- [ ] Integrate hardware vblank/page-flip feedback and GPU/display hosts while retaining one compositor and frame clock.
- [x] Share semantic dark/light palettes, typography roles, spacing, radii, and bounded elevation across shell and projected Android UI.
- [x] Implement Ocean/Violet dynamic color v2 with contrast gates and unchanged hit/damage geometry.
- [x] Implement Accessibility appearance v3: two semantic text sizes, explicit high contrast, one UiAppearance session owner, shell/Android synchronization, and bounded text ellipsis.
- [ ] Migrate remaining hard-coded spacing/radii and complete Unicode shaping, broader font scaling, locale/RTL, and semantic accessibility.
- [x] Implement single-card Overview dismissal with identity-qualified authorization; retain no process-kill, background-task, thumbnail, or Activity-pixel authority.
- [x] Bind recent-card icons to validated catalog/session/package-generation/APK-digest identities, with explicit unavailable or metadata-derived fallback behavior.
- [x] Reserve the Overview top 120 pixels for Quick Settings gestures and restore the underlying Overview pixels on close.
- [x] Implement session-local lock notification swipes: 8-pixel quantization, exact restoration at 159 pixels, dismissal at 160 pixels, and no navigation/unlock on tap.
- [ ] Complete multi-card history, actual task/process lifecycle, persistent recovery, memory-pressure handling, transitions, and accessible gestures.
- [x] Implement grouped Settings, Display, dark/accent/software-dimming controls, explicit unavailable states, and guard-stack regression checks.
- [x] Add the Display-to-Accessibility page and synchronized visual preferences without new service/device rights.
- [ ] Connect sound, networking, peripherals, permissions, storage, privacy, security, screen-reader semantics, and updates only when the responsible services and capabilities exist.
- [ ] Complete IME, Unicode shaping, locale, RTL, dynamic fonts, and the accessibility semantics tree.
- [ ] Establish measured 60 Hz and 120 Hz paths, latency distributions, peak memory, and cold-start measurements.
- [ ] Automate screenshot/pixel, semantic, keyboard/touch, and rotation regressions.

### P4: AndroidBox A1 through A4

- [ ] Write an ADR selecting the Android baseline, source/prebuilt strategy, licensing, updates, and maintenance budget before authorized acquisition.
- [ ] Boot minimal ART/Bionic userspace and test a restricted Linux ABI personality.
- [ ] Implement Binder identity, transaction/reply/death, and capability mapping.
- [ ] Run a non-customized managed APK through ActivityThread and Framework.
- [ ] Run a JNI/native-library APK with W^X, RELRO, ASLR, and crash-isolation checks.
- [ ] Bridge packages, permissions, windows/input, storage, networking, and media in order.
- [ ] Build an API-gap database from real application behavior instead of fixture-specific exceptions.

### P5: Platform capabilities

- [ ] Networking: IPv4/IPv6, DNS, TLS, Wi-Fi, VPN, hotspot, firewall, and per-app accounting.
- [ ] Media: audio focus, recording/playback, codecs, cameras, Bluetooth audio, and permission indicators.
- [ ] Telephony: SIM/eSIM, modem interfaces, calls, SMS, emergency calling, and regulatory requirements.
- [ ] Power: suspend/resume, DVFS, thermal policy, background freezing, alarms, and battery accounting.
- [ ] Security: verified boot, file-based encryption, Keystore/hardware keys, policy enforcement, vulnerability response, and secure OTA.
- [ ] Data: multiple users, backup/recovery, quotas, media library, uninstall erasure, and privacy export.

### P6: Hardware and release

- [ ] Select the first development device explicitly before planning BSP, bootloader, device tree, HAL, display/GPU, touch, storage, networking, audio, cameras, sensors, and modem support.
- [ ] Obtain authorization before connecting, unlocking, flashing, or modifying physical devices.
- [ ] Implement recovery, A/B OTA, rollback protection, factory reset, and unbricking procedures.
- [ ] Validate power, thermals, standby, calls, cameras, network switching, and actual power-loss recovery on hardware.
- [ ] Define a daily-use release only after security review, soak testing, compatibility, performance, and upgrade matrices pass.

## 6. Single-owner interfaces

| Product fact | Owner | Native/Android access |
| --- | --- | --- |
| App identity and package version | Core System catalog | Native API / Package bridge |
| Permission decision | Core System policy | Capabilities / Permission bridge |
| Windows and surfaces | Display Server | Native / Android Surface APIs |
| Input focus | Input Server | Events / Input bridge |
| Application data | Storage Server | Storage API / Scoped-storage bridge |
| Network policy | Network service | Socket policy / Connectivity bridge |
| Audio and camera state | Media service | Native / Android media APIs |
| Logs and audit timeline | Runtime observability | Shared event format with separate access policies |

A second writable copy requires an ADR explaining consistency, recovery, and authority. Prefer querying or referencing the owner.

## 7. Consolidation and removal criteria

Remove an old module, feature, protocol, or implementation only when all conditions hold:

1. Success and failure semantics are fully covered.
2. Capabilities, identities, resource bounds, and durable recovery remain intact.
3. Old profiles, parent ABIs, disk recovery, and at least one negative security gate pass.
4. Observable outputs have an explicit migration mapping and faults remain diagnosable.
5. No product binary, fixture, or documentation still needs the old interface.
6. The removed code is duplicate implementation, a compatibility facade, or test scaffolding, not required product functionality.

Candidates include duplicated gate logic, disconnected historical demos, migrated state-machine/checksum/recovery code, obsolete milestone aliases, and redundant planning templates. Preserve security-negative tests, crash/power-loss recovery tests, compatibility evidence, required decoders within their support window, isolation boundaries, Android runtime semantics, and hardware boot/BSP/HAL requirements.

## 8. Definition of done

- Code and documentation agree on interfaces, ABI versions, and capability boundaries.
- Relevant host tests, static checks, and QEMU/device gates pass.
- At least one failure path is verified; security work includes permission-denial and identity-spoofing negatives.
- Persistent work covers reboot/torn-write/power-loss models; UI work covers screenshots, semantics, and hit testing.
- Evidence paths, commands, summaries, and limitations are recorded without presenting a subset as general compatibility.
- External dependencies, network/AOSP acquisition, SDK modifications, and physical-device operations have explicit authorization.
