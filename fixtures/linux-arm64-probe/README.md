# Linux arm64 native entry probe

This fixture is compiled with the locally installed Android NDK and executes as native AArch64 machine code in a separate Bndroid EL0 process. It uses NDK Linux UAPI declarations, its own small startup assembly, and direct syscalls. It does **not** link Bionic, run ART, or execute an APK.

## Build and run

Prerequisites are the repository's Rust toolchain, AArch64 QEMU, Python 3.10+, and Android NDK 27.1.12297006. The script reads the SDK location from `ANDROID_SDK_ROOT`, then `ANDROID_HOME`, then the macOS default. Set `BNDROID_NDK_ROOT` to use another existing NDK with the `aarch64-linux-android35-clang` driver. No dependencies are downloaded and the SDK is not modified.

From the repository root:

```sh
./fixtures/linux-arm64-probe/build.sh
./scripts/check-linux-abi-runtime.sh
```

The first command produces `target/linux-arm64-probe/linux-probe.elf` and `elf.txt`. The second builds a fresh copy inside `target/linux-abi/check.*/`, builds the isolated `linux-abi-runtime` kernel/Init profile, and boots QEMU with a temporary disk overlay. It terminates its QEMU child after validation and preserves build logs, serial output, the ELF description, and `summary.json`. `BNDROID_QEMU_TIMEOUT_SECONDS` sets a finite positive guest timeout, defaulting to 60 seconds.

The ELF uses standard NDK Clang/lld output: static `ET_EXEC`, a mapped program-header table, separate R/RX/RW LOAD segments, and a non-executable stack. Only the fixed image base and page separation are configured; there is no custom linker script. `-nostdlib` deliberately excludes Bionic startup from this first test.

## What is checked

- Standard `argc`, `argv`, `envp`, and auxiliary-vector pointers, page size, entry address, and mapped ELF headers.
- Initialized data, zeroed BSS, and writable private memory across two successive process generations.
- Linux process/thread/identity calls, monotonic clock, `uname`, console output, and normal process exit.
- Negative `ENOSYS`, `EBADF`, `EFAULT`, and `EINVAL` returns; unknown and overlapping Bndroid syscall numbers cannot reach native IPC.
- Invalid SVC immediates, null/overflow/kernel pointers, read-only output buffers, and bounded positive short writes.
- Live-process accounting, both exits/reaps, no inherited Bndroid handles, and an unchanged storage base image.

Each nonzero exit code identifies a `CHECK` in `main.c`. Successful runs emit two `LINUX_ARM64_PROBE_OK` lines and one kernel `LINUX_ABI_RUNTIME_OK` line; the host verifies their fields and the distinct generation-qualified kernel PIDs.

The gate is a controlled native-entry validation profile, not a general executable launcher. See [the implementation record](../../docs/milestones/androidbox/LINUX_ARM64_ENTRY.md) for the current ABI and loader limits.
