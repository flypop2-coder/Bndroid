# ADR 0002: Linux ABI release and anonymous code-cache backing

Status: accepted for the bounded ART integration, November 6, 2026.

## Context and decision

The unmodified ART wrapper checks `uname` before attempting `memfd_create`, to
avoid a documented fault on older Linux kernels. Reporting Bndroid's native
`0.1.0` version in that release field therefore bypassed the syscall entirely.
AndroidRuntime's profiling cache then selected a legacy mapping that requires
RWX protection and aborted at Bndroid's W^X check. The dependency is explicit in
[ART's wrapper](https://android.googlesource.com/platform/art/+/master/libartbase/base/memfd.cc)
and [dual-view cache initialization](https://android.googlesource.com/platform/art/+/35c76e8563ee700faaa403de71999e6b5b622591/runtime/jit/jit_memory_region.cc).

The ART and Framework personalities now report `3.17.0-bndroid-abi` in the release
field, with `Bndroid 0.1.0; partial Linux ABI` in the version field. This is a
compatibility dispatch value for version-gated upstream clients, not the kernel's
implementation version or a claim of full Linux 3.17 conformance. Smaller
personalities retain `0.1.0-bndroid`. Unsupported operations continue to return
errors. The compatibility matrix and Android acceptance levels remain the source
of truth; a new release string cannot satisfy a runtime acceptance gate.

## Backing and protection model

`memfd_create`, `ftruncate`, descriptor I/O and `MAP_SHARED` now operate on a real
anonymous object with unique physical-page ownership. Multiple virtual views
reference those same pages. Descriptors and mappings independently retain the
object; closing or reusing a descriptor cannot invalidate its old mappings.
Truncation invalidates affected translations before releasing pages, and accesses
beyond EOF receive SIGBUS. Growth zeroes new pages and makes existing aliases
fault them back in. Prepared allocations fail before file contents or mappings
change. These objects use guest RAM, never host files or the demonstration disk.

Each PTE remains W^X and SCTLR.WXN stays enabled. A program can deliberately map
the same memfd pages through separate RW/NX and R/X views, as ART requires. This
is not a stronger physical-page immutability guarantee: the RW view can update
the bytes executed through the RX view. Immutable runtime files retain their
existing restrictions. `PROT_READ | PROT_WRITE | PROT_EXEC` is still rejected.

The ART profile enables CTR_EL0 reads and user-VA cache maintenance through
SCTLR.UCT/UCI, matching the controls used for user code publication in
[arm64 Linux](https://raw.githubusercontent.com/torvalds/linux/v5.15/arch/arm64/include/asm/sysreg.h).
Load/store/execute rights do not change; whole-cache and set/way operations remain
privileged. The native gate executes generated AArch64 code, modifies it through
the writable alias, flushes both views and observes the new result. QEMU proves
the exercised instructions and protection behavior, not a real-device cache
coherency certification.

## Bounds and unsupported behavior

Each file is limited to 128 MiB. Its pages share the existing physical budget with
private mappings: 256 MiB for ART and 512 MiB for Framework. Aliases are counted
once in physical ownership accounting and separately in per-process resident
mapping records. All allocations are fallible. No larger guest, heap or disk is
introduced by this decision. Compact frame ownership keeps a mapping record at
24 bytes so the existing 8 MiB heap can handle Framework's large DEX mappings.

The supported flags are CLOEXEC and ALLOW_SEALING. SEAL_SEAL, SEAL_SHRINK,
SEAL_GROW, SEAL_WRITE and SEAL_FUTURE_WRITE are enforced. Existing mappings keep
their mmap-time write entitlement; new read-only mappings after write sealing
cannot gain write permission through mprotect. This follows the
[kernel's sealing check](https://code.googlesource.com/linux/torvalds/linux/+/77dc55a978e69625f9718460012e5ef0172dc4de/include/linux/mm.h).

Private memfd mappings return ENOSYS until copy-on-write semantics exist. Shared
file remapping, shared MADV_DONTNEED, descriptor transfer between processes,
general fork, huge pages, NOEXEC/EXEC memfd flags and a writable filesystem remain
outside this increment. DONTFORK/DOFORK record mapping inheritance policy; they
do not make fork available. Runtime objects remain scoped to their process group.

## Acceptance

The native memfd gate must pass in two process generations, including shared
visibility, positional and shared-offset I/O, descriptor reuse, sealing,
truncation/SIGBUS, zero growth, generated-code execution, RX write denial, RW
execute denial, and final zero page/table/file/thread counts. Framework startup
and full Android applications have separate gates. Their status must be reported
independently even when the memfd gate passes.
