# Linux arm64 native entry foundation

Recorded November 6, 2026. The isolated `linux-abi-runtime` profile, ABI 70, runs an Android NDK-compiled C executable as native AArch64 code in Bndroid. This establishes an entry/syscall foundation for upstream Android userspace. It does not yet initialize Bionic or run ART, JNI, the Android Framework, or a general APK. AndroidBox A1 through A4 remain open in [the roadmap](../../../TODO.md#p4-androidbox-a1-through-a4).

## Execution path

1. NDK Clang/lld builds the repository-owned [entry probe](../../../fixtures/linux-arm64-probe/README.md) as static `ET_EXEC` with a conventional mapped program-header table.
2. The loader validates each LOAD segment and its permissions, allocates private frames and a separate translation context, zeroes BSS, and creates a guarded four-page stack. The highest page contains `argc`, `argv`, `envp`, and auxiliary data; SP is aligned to 16 bytes.
3. Native Bndroid Init spawns `LinuxApp` using the existing supervised process path. The temporary startup channel is consumed and closed before the child is runnable, leaving no native capability handles. Linux entry does not receive the native startup handle/image-ID register arguments.
4. The kernel selects syscall semantics from its own process image identity. Linux arm64 and Bndroid both use `svc #0` and x8, so dispatch must happen before native syscall decoding. Linux results use x0, with negative errno on failure; native IPC return registers are not overwritten.
5. Init waits for normal exit and repeats the spawn. Live-process accounting and final reaping checks cover both generations, fresh BSS, distinct kernel PIDs, and complete handle cleanup.

The pure stack/time helpers are in `kernel/src/linux_abi.rs`; the AArch64 dispatcher and guest acceptance monitor are in `kernel/src/linux_runtime.rs`. Default ABI 23 and the mobile/AndroidBox profiles retain their prior behavior. The build script rejects mismatched kernel/Init Linux features; combining this profile with mobile, app-data, or storage-server runtime profiles is not supported.

## Implemented syscall subset

| Linux arm64 call | Current behavior |
| --- | --- |
| `write` (64) | stdout/stderr only, at most 256 bytes per call, positive short writes, validated user reads |
| `exit`, `exit_group` (93, 94) | Single-thread process exit, low eight bits of status, normal supervisor wait/reap |
| `clock_gettime` (113) | `CLOCK_MONOTONIC` only, using the architectural timer, validated user writes |
| `uname` (160) | Explicit Bndroid identity and `aarch64` machine name |
| `getpid`, `gettid` (172, 178) | Current process slot ID; this profile has one thread per process |
| `getppid` (173) | Init identity 1 in this supervisor-only profile |
| `getuid`, `geteuid`, `getgid`, `getegid` (174–177) | One application principal, ID 10000 |
| All other calls | `-ENOSYS`, with unsupported-call diagnostics |

Nonzero SVC immediates return `-ENOSYS`. Bad descriptors, addresses, and clock IDs return their corresponding Linux errors. User copies require both the current process's mapped permissions and the guarded copy mechanism; kernel physical addresses are rejected.

## Loader and runtime limits

The Linux parser permits an optional, validated `PT_PHDR` and requires the entire header table to be backed by readable LOAD file bytes. It retains the existing limits of eight program headers, 256 load pages, a 2 MiB ELF file, page-separated mappings, and W^X. The kernel requires page-aligned LOAD starts within its fixed 1 MiB image window at `0x200000000`. The default native parser retains its stricter original metadata rules.

There is no dynamic linking, `ET_DYN`, TLS, relocation support, `mmap`, thread creation, futexes, signals, filesystem descriptor table, sockets, or Binder interface in this profile. `AT_RANDOM` is not fabricated, and the probe disables stack protection because it supplies its own startup without libc. These constraints exclude normal Bionic/ART initialization and are not claims of general Linux compatibility.

The existing local NDK is used read-only as a compiler and header source. No AOSP source tree, Bionic implementation, proprietary system image, or prebuilt binary is imported into the repository. Generated ELF/kernel/disk artifacts remain under ignored `target/`. This validation does not select the future full Android distribution baseline.

## Reproduction and recorded evidence

```sh
./scripts/check-linux-abi-runtime.sh
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --offline --target "$HOST_TARGET" --workspace --lib
cargo test --offline --target "$HOST_TARGET" -p bndr-abi --features linux-abi-runtime --lib
cargo test --offline --target "$HOST_TARGET" -p bndroid-kernel --features linux-abi-runtime --lib linux_abi::tests
```

The QEMU gate checks two native process exits/reaps, 52 Linux calls, eight unsupported-call rejections, two private-SVC rejections, ten address failures, 750 console bytes, zero remaining application handles, and an unchanged base disk. The supervisor creates two native channel pairs solely for the existing spawn protocol; the Linux children cannot use them.

The default workspace suite passed 1,096 host tests. The Linux ABI feature suite passed 36 ABI tests and four stack/time tests. Default kernel boot and the mobile UI QEMU acceptance gate also passed. Per-run `summary.json` records the probe ELF digest and evidence fields. Logs and binaries are local generated evidence, not committed artifacts.
