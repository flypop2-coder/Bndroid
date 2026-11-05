# Bionic Dynamic-0

Recorded November 6, 2026. An ordinary NDK PIE now reaches `main` through the unmodified Android dynamic linker on the Bndroid kernel. Real shared Bionic, libm, libdl, and a separately compiled plugin execute in two supervised, isolated process generations. No Linux guest kernel is involved in this execution gate. ART, JNI, Binder, Framework, and general APK execution remain open.

## Inputs and reproduction

The native toolchain remains NDK r27b/API 35. Runtime inputs come from the already installed arm64 SDK Android 16/API 36.1 revision 3 image, fingerprint:

```text
google/sdk_gphone64_arm64/emu64a:16/BP41.250916.009.A1/14246511:user/release-keys
```

| Component | SHA-256 |
| --- | --- |
| `linker64` | `1e7ad0d91d533678422aad564058207d5674b42d327f1cf4cba43617c379485e` |
| `libc.so` | `320357ed3926e86783543258b7c6fa9f2a16ffcc90aa8caaaec8fce2bab4c13c` |
| `libm.so` | `dc5d266059409ed43c377fe75e7777c1999b48fa6601395b936e1f569a701b87` |
| `libdl.so` | `d7cd599c51586ff6f42799805ea406685a50adc3d52afa83a1b127433ffb3497` |

The [fixture guide](../../../fixtures/bionic-dynamic/README.md) describes extraction and both QEMU gates. A second isolated extraction reproduced all four hashes. Extracted runtime files, source notices, temporary AVDs, rootfs archives, and generated binaries stay outside Git. This is a frozen native bootstrap input set; it does not choose or import a complete AOSP Framework/ART distribution.

## Loading and memory

`bionic-dynamic-runtime` extends the existing Linux/Bionic personality and retains native ABI 70. Kernel and Init must both select it. An explicit ET_DYN interpreter parser validates LOAD permissions, mapped metadata, and one self-contained dynamic table; ET_EXEC-only native/static parsers keep their earlier policy. The interpreter is rebased at `0x200000000`, with a 4 MiB file/load limit, a 16 MiB initial mapping window, and a 256 KiB guarded stack. Only this opt-in profile widens those bounds.

The startup stack invokes the real interpreter with `/system/bin/dynamic-probe` and a bounded library search path. The interpreter performs its own relocation, loads the executable and dependencies, initializes Bionic, and calls program constructors and `main`. This uses the direct-execution path implemented in [upstream linker_main.cpp](https://android.googlesource.com/platform/bionic/+/refs/heads/main/linker/linker_main.cpp); the kernel does not duplicate the dynamic linker.

Private file mappings occupy the existing sparse 32–512 GiB arena. File bytes populate owned pages before publication, writable private changes leave the source immutable, and instruction-cache maintenance precedes executable mappings. W+X remains rejected. Protection changes, fixed replacement, partial unmap, cache/TLB updates, and stopped-process cleanup use the existing ownership transaction. Scheduler return validation consults actual executable page ownership for Linux processes, allowing code in mapped libraries while retaining native validation.

Linux permits a file mapping to extend beyond EOF without reading those bytes. Complete pages beyond EOF remain absent, including after `mprotect`; the last partial file page is zero-filled. Access to an absent EOF page currently terminates the process through the existing contained-user-fault path. SIGBUS handler delivery is not yet implemented.

`MADV_WIPEONFORK` and `MADV_KEEPONFORK` record inheritance policy in anonymous VMAs and preserve it across protection/split/unmap operations. File-backed wipe requests fail. Process duplication is still `ENOSYS`; future clone/fork must honor these stored policies before it is enabled. Other advice is rejected rather than reported as completed.

## Files and descriptors

An explicitly supplied, read-only `fw_cfg` USTAR archive provides runtime files. It is checked before userspace publication: checksum, bounds, canonical archive names, duplicate paths, parent conflicts, permitted types, and bounded symlink traversal. Runtime `..` resolution follows symlinks first and verifies intermediate directories, so a missing prefix cannot disappear through lexical normalization. The root never resolves host filesystem paths.

Each Linux process owns a 64-slot descriptor table. Independent opens have independent offsets; dup operations share an open description while close-on-exec flags stay per descriptor. References and intentionally unclosed descriptors are reclaimed on exit. The initial process directory is `/`; changing it is unsupported. The archive is limited to 8 MiB, 64 entries including built-ins, and 256-byte UTF-8 paths. This is not a writable application filesystem.

Implemented interfaces include openat, close, read/pread64, lseek, dup/dup3/fcntl, write/writev, stat/fstat, statfs/fstatfs, readlinkat, access, and getcwd, with checked user copies. AArch64-specific open flag values are used. `/dev/null`, finite host-backed `/dev/urandom`, and the required self-executable/fd link queries are bounded built-ins. No general procfs, device filesystem, directory enumeration, file mutation, socket transport, or thread implementation is implied.

## Evidence and remaining limits

The positive gate verifies real shared-libc identity; plugin constructors, ELF TLS, symbols, errors, and unload; TLS across explicit timer preemption; main/atexit/destructor order; private file data and permission changes; offset sharing and pread independence; invalid pointers; descriptor exhaustion; read-only/path/symlink failures; and full process cleanup. A recorded run passed 1,014 Linux calls, four bad-address checks, 208 consumed entropy bytes, and two zero-status exits. It reclaimed 2,376 sparse pages after a 964-page peak; live sparse pages, tables, and file descriptions were all zero. Allocation counts can vary with fresh entropy.

The deliberate EOF-read gate verifies the exact fault address and EL0 data-abort class, then observes one reaped child, one surviving Init, and zero dynamic pages/tables/file descriptions. The 1,114 default host tests, 19 Linux-feature helper tests, four UART parser tests, predecessor runtime checks, native boot/fault containment, mobile UI acceptance, and profile Clippy passed. Missing or corrupt root archives reject boot before userspace.

The tested runtime tolerates 60 `ENOSYS` results for futex, sched_getaffinity, sigaltstack, rt_sigaction, rt_sigprocmask, gettimeofday, and socket. Missing platform properties and generated linker configuration use the upstream fallback; the latter emits a warning. These are explicit gaps. There is no signal delivery, native thread creation, wall clock, network, ASLR, MTE, writable Linux filesystem, APK JNI loading, ART, Binder, or full Android Framework in this profile. AndroidBox A1–A4 remain incomplete.
