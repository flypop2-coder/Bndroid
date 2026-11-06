# ART Memfd-0: shared code-cache backing

Date: November 6, 2026. Native ABI remains 70. Android acceptance A1–A6 remains incomplete.

The raw `target/` artifacts referenced here were subsequently lost during Cargo
cleanup. [Framework JNI-0](FRAMEWORK_JNI_0.md) records recovery, fresh regression
evidence and the new local storage policy; the results below are historical.

## Implemented behavior

The ART personality supplies anonymous memory files, real shared physical views,
descriptor I/O, truncation, seals and mapping-owned lifetime. ART's upstream
64 MiB code-cache layout can retain separate writable and executable views,
including the fixed-address R-to-RX replacement. Single RWX mappings are still
rejected. [ADR 0002](../../decisions/0002-linux-abi-release-and-code-cache.md)
records the exact protection model, partial ABI dispatch version and remaining
unsupported operations.

Private pages and memfd backing now use one bounded physical-page owner and
counter. Shared aliases do not allocate duplicate frames. Unmapping or shrinking
invalidates translations before releasing frames; failed preparation leaves the
old contents intact. Mapping records remain 24 bytes, enforced at compile time.
Equal-size replacement reserves space for the final mapping count, avoiding a
spurious doubling of the large metadata vector. Guest RAM, kernel heap and
physical-page limits have not increased.

The native `memfd` gate uses actual Bionic syscalls and generated AArch64 code. It
checks shared visibility, pread/pwrite and duplicated offsets, fstat/readlink,
descriptor reuse, mapped lifetime after close, seals, partial/full truncation,
SIGBUS after EOF, zero growth and the 64 MiB ART mapping layout. Each generation
returns from three deliberate faults: writing RX, executing RW and reading beyond
truncated EOF. It requires physical peak usage below 20,000 pages, which rejects
duplicated backing for the large shared views.

Evidence `target/art-runtime/check.Zb1UFr` passes twice with 1,170 Linux calls,
17,335 peak physical pages, 35,222 reclaimed pages and all six signal frames
returned. Pages, tables, file descriptions, live threads and waiters return to
zero. The temporary disk is unchanged. `shared_memfd_verified=true` is separate
from Java, Framework and full Android application acceptance.

## Framework bootstrap metadata

The public immutable property area now contains the six existing Bndroid/ABI
values and seven version values describing the frozen API 36.1 / Android 16
libraries. `Build.VERSION` requires a parseable `ro.build.version.sdk_full` and
the release codename list; these values do not claim implementation of every
Android API. The builder rejects another API source instead of assigning guessed
version metadata. See the upstream
[Build version reader](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/core/java/android/os/Build.java).

At API 36+, the supplied libmeminfo constructor also reads the version header of
`/system/etc/init/netbpfload.rc`. The exact 696-byte source file has been added to
the local input set; it is passive metadata, not a running BPF/network service or
an Android init implementation. Its SHA-256 is
`1c50bee6dbdb53577f710779a19b79a1c2d44ee3904419f88685b836fd796f9f`.
The set now contains 348 files / 265,050,082 bytes; all prior 347 file records are
unchanged. There are still 280 native dependency entries and 46 boot jars. The
archive contains 358 entries / 264,192,000 bytes before the kernel's seven entries.

For this local addition, the file was read directly from the existing SDK image's
GPT/super/EROFS data, with no mount, SDK write, emulator launch or download. GPT
checksums, logical-partition header/table hashes and a complete source-image
SHA-256 were retained alongside the addition receipt under ignored `target/`.
The pre-addition receipt remains available. Normal extraction now pulls this
file from its owned offline emulator; resume upgrades older complete inventories,
and the root builder refuses a missing record. Runtime/configuration bytes and
their notices stay local and are not imported into Git.

`CLOCK_BOOTTIME` reads the same always-running boot counter as monotonic time in
this profile, which has no system suspend. The properties gate
(`check.Pxb0Zz`) validates all 13 properties, timed property waiting, boot-clock
advancement, immutable mappings and complete cleanup in two generations.

## Validation and scope

The final full Framework attempt (`target/art-runtime/check.fDK3by`) successfully
creates the 64 MiB memfd and all cache views, including the fixed RX mapping at
`0x89040000`. It reaches Java `RuntimeInit.commonInit` with the version metadata
loaded. Its first remaining Java failure is an `UnsatisfiedLinkError` for
`libframework-connectivity-tiramisu-jni.so`, loaded dynamically by TrafficStats;
the static ELF dependency closure does not yet include that library. The
uncaught-exception path then attempts ActivityManager through Binder and aborts
because `/dev/binder` is absent. The failed process and all six workers are reaped
with zero remaining pages/tables/files/waiters and an unchanged temporary disk.
Parcel/Looper and application-main proof markers are absent. This is recorded as
`guest-failure`, never as completed Framework or Android app support.

The ordinary APK/GC/JNI gate passes (`check.8rZwR4`): 61,263 Linux calls, 84,561
discarded pages, 36,415 demand-zero pages, 51,537 peak pages, 223,491 reclaimed
pages, and all 20 threads reaped with no remaining resources. The extended
anonymous-memory/relocation gate (`check.x2KT7Q`) passes with both deliberate
read-only write faults returned and complete cleanup. All listed temporary disks
are unchanged.

Also passed: 1,128 workspace host tests, 395 Framework kernel-library tests,
20 input/property/evidence tests; host library and AArch64 Framework Clippy with
warnings denied, tracing enabled and disabled; formatting and script syntax
checks. A broader all-targets host lint attempt exposed existing test-only lints
outside this change; it is not claimed as passing.

Native boot and mobile UI (`target/mobile-ui/check-overview.T7acWI`) pass. Dynamic
Bionic (`target/bionic-dynamic/check.7917Fa`), pthread normal (`check.3iwORu`),
leader exit (`check.L8BlNb`), group exit (`check.FqoaRM`), worker fault
(`check.e2DeCf`) and file-EOF containment (`check.wyXE6P`) pass. These checks retain
the existing native/mobile profiles while the larger Framework route is developed.

The narrow gates are reproducible with the commands in the
[fixture guide](../../../fixtures/art-runtime/README.md). They do not establish
ActivityThread, Binder transaction/reply/death, app graphics, general storage,
network/media APIs, arbitrary APK compatibility or real-device performance.
