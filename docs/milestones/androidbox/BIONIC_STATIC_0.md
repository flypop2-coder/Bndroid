# Bionic Static-0

Recorded November 6, 2026. Bndroid executes a standard NDK-linked C program through real Bionic initialization, main, registered exit handlers, and ELF destructors. Two independent supervised processes pass the fixture and are reclaimed. This advances the [native entry foundation](LINUX_ARM64_ENTRY.md); it does not establish ART or general APK compatibility.

## Reused implementation and selected native toolchain

The native bootstrap uses the already installed Android NDK r27b (`27.1.12297006`), targeting API 35. The NDK's CRT, Bionic, Scudo, compiler support, and TLS code are linked without modification. The checked-in code is the fixture and the Bndroid adaptation, not imported upstream libraries. A generated link map identifies linked objects; `build-receipt.json` records input hashes and compiler/version metadata. The validated `libc.a` SHA-256 is `6aae08fc80f058d90305d302a98fd6849480644ef25e14c56b906f137eb80d0d`.

This freezes a reproducible native test toolchain. It does not select an AOSP Framework/ART distribution, arrange proprietary service licensing, or authorize a large source acquisition. Those decisions remain in [the roadmap](../../../TODO.md#p4-androidbox-a1-through-a4). No system image or generated third-party binary is committed.

## Kernel adaptation

The `bionic-runtime` feature extends `linux-abi-runtime` and keeps native Bndroid ABI 70. Linux syscall numbers retain their Linux meanings; no new native IPC syscall is introduced. Kernel/Init builds must agree on the Linux personality. The existing mobile and package profiles remain separate.

- **ELF admission:** an explicit Bionic static profile accepts up to 16 program headers and validates TLS, GNU_RELRO, GNU_EH_FRAME, and NOTE metadata against readable LOAD file bytes. TLS/RELRO must lie in originally writable LOAD ranges. Duplicate singleton metadata, forged ranges, ET_DYN/dynamic linking, and executable stacks are rejected. The default native parser and minimal Linux entry profile retain their earlier bounds.
- **Thread context:** IRQ and synchronous exception frames save/restore `TPIDR_EL0` and `TPIDRRO_EL0`, with zero values for new contexts. Both the raw entry fixture and the real Bionic fixture verify preservation across timer preemption and fresh state on respawn.
- **Sparse memory:** anonymous reservations occupy a distinct 32–512 GiB virtual arena in each private address space. `PROT_NONE` reserves address space without allocating physical pages. `mmap`, `munmap`, and `mprotect` support bounded private anonymous R/RW mappings, fixed replacement, no-replace, partial unmapping, and rights changes. The allocator's multi-gigabyte reservation therefore does not consume multi-gigabyte RAM.
- **Ownership:** mapping plans, frame allocations, and metadata capacity are prepared before publishing changes. Rights changes use break-before-make and ASID invalidation. Unmapped/replaced pages are released after invalidation. Stopped-process cleanup verifies and detaches sparse tables before reclaiming their owners. User-copy permission checks include these mappings.
- **Tagged addresses:** the Linux profile enables architectural top-byte ignore for Android heap accesses. `PR_SET_TAGGED_ADDR_CTRL`/`PR_GET_TAGGED_ADDR_CTRL` maintain per-process opt-in state for syscall pointer normalization. Normalized pointers must still belong to the caller's readable/writable mappings; tagging does not grant kernel-memory access. MTE is not advertised or implemented. This follows the distinction between userspace tagging and the opted-in syscall ABI in the [Linux arm64 tagged-address specification](https://www.kernel.org/doc/html/v5.19/arm64/tagged-address-abi.html).
- **Entropy:** each QEMU boot supplies 64 KiB from host `os.urandom` through `fw_cfg`. The kernel validates the exact input, supplies fresh `AT_RANDOM` bytes and bounded `getrandom` results, and consumes/scrubs each byte once. No fixed stack guard, predictable fallback, or reusable seed is substituted. Missing input rejects startup, and exhaustion returns `EAGAIN`.

Sparse memory is currently bounded to 128 regions, 8,192 committed page owners, and 256 intermediate tables per process, with fallible kernel allocation potentially imposing a lower available limit. Anonymous executable mappings and file-backed mappings remain unsupported. Originally writable static LOAD pages can switch R/RW for Bionic's protected globals and RELRO; original code/rodata cannot become writable through this path.

Kernel image placement and anonymous first-fit selection are deterministic; this profile does not provide ASLR. Scudo's internal randomized choices do not establish kernel address-space randomization.

## Additional syscall behavior

| Interface | Verified behavior |
| --- | --- |
| Standard descriptors | stdin has null-input semantics; stdout/stderr use UART; descriptor close state and `F_GETFD`/`F_SETFD`/`F_GETFL` are per-process |
| `writev` | Validated user vectors, bounded positive short output, Linux error returns |
| `set_tid_address` | Registers the current single-thread process's clear-TID pointer; normal exit attempts a checked zero write |
| `sched_getscheduler`, `sched_getparam` | Current process only, normal time sharing, priority zero |
| `getrlimit` | Actual fixed stack size and three standard descriptor slots |
| `clock_gettime` | Monotonic timer and a coarse monotonic view |
| `getrandom` | Checked output buffers, at most 256 bytes per call, fresh host-supplied bytes, explicit invalid-flag/address/exhaustion errors |

The tested Bionic startup tolerates `ENOSYS` for `faccessat`, `openat`, `newfstatat`, `fstat`, `futex`, `sched_getaffinity`, `sigaltstack`, `rt_sigaction`, and `rt_sigprocmask`. These remain missing capabilities, not successful stubs. Unsupported `prctl` options return `EINVAL`. There is no general filesystem descriptor table, thread creation, signal delivery, dynamic linker, JNI, ART, Binder, Framework, or APK execution in this profile.

## Reproduction and verification

```sh
./scripts/check-bionic-runtime.sh
./scripts/check-linux-abi-runtime.sh
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --offline --target "$HOST_TARGET" --workspace --lib
python3 scripts/test_linux_runtime_evidence.py
```

See [the fixture guide](../../../fixtures/bionic-arm64/README.md) for prerequisites and artifact layout. The gate uses temporary disk storage, verifies the unchanged base image, accepts only complete newline-terminated kernel proof records, and stops only the QEMU process it created. Four parser regressions cover every partial UART prefix, malformed fields, duplicate fields, and duplicate proof records.

The recorded successful run checked constructors/destructors, atexit, ELF TLS, errno, Scudo allocation including 2 MiB, region splitting/merging, permission failures, fresh replacement pages, allocation-limit rollback, random reads, tagged-pointer opt-in, descriptor state, and stdio in both process generations. It reported 220 Linux calls, 12 bad-address results, 24 explicitly unsupported calls, and 288 consumed entropy bytes. Sparse page/table owners returned to zero; a recorded peak was 791 pages with 1,594 pages reclaimed across both lifetimes. Allocator addresses and page peaks vary with fresh entropy.

The 1,104 default workspace host tests, Bionic QEMU gate, original Linux entry gate, default ABI 23 kernel boot, mobile UI QEMU gate, and Bionic-profile Clippy check with warnings denied passed. The EL0 fault-containment gate verified that a denied kernel-memory read leaves scheduling alive; this also caught and corrected a loader-helper feature guard. An omitted entropy input was verified to stop at boot with the specific missing-input error. This evidence does not mark AndroidBox A1–A4 complete.
