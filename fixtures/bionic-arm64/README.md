# Static Bionic native program

This is an ordinary C program linked by the installed Android NDK with its unmodified static CRT, Bionic libc, Scudo allocator, TLS implementation, constructors, and exit machinery. There is no replacement libc or custom `_start` in this fixture. It runs in a separate EL0 process on the Bndroid kernel, without a Linux guest.

## Run

Use Android NDK 27.1.12297006 (r27b), the repository Rust toolchain, AArch64 QEMU, and Python 3.10+. SDK discovery follows `ANDROID_SDK_ROOT`, `ANDROID_HOME`, then the macOS default. `BNDROID_NDK_ROOT` can select an existing NDK, but the recorded acceptance result is for r27b/API 35.

```sh
./fixtures/bionic-arm64/build.sh
./scripts/check-bionic-runtime.sh
```

The standalone build writes `target/bionic-arm64/`. The gate builds in a unique `target/bionic/check.*/` directory, selects the isolated `bionic-runtime` profile, and runs QEMU with 256 MiB RAM and a temporary disk overlay. It stops its QEMU child after verification. `BNDROID_QEMU_TIMEOUT_SECONDS` sets a finite positive timeout, defaulting to 90 seconds.

The gate obtains 64 KiB of fresh host OS entropy for each boot and passes it through a private temporary `fw_cfg` file. That file is removed after the run; its contents are not copied into evidence or committed. The guest consumes each byte once and scrubs it. A missing entropy input fails boot; an exhausted pool returns an error rather than recycling data. This bounded emulator source is not a production device entropy service.

## Checks and artifacts

The program checks standard startup arguments and auxv, initialized/zero ELF TLS before constructors, TLS/errno preservation across preemption, descriptor flags and close behavior, `calloc`/`realloc`/`free`, a 2 MiB allocation, and distinct random reads. Memory tests cover inaccessible reservations, partial `mprotect`/`munmap`, fresh replacement pages, `MAP_FIXED_NOREPLACE`, W+X rejection, kernel-address rejection, failed-allocation rollback, and opt-in tagged user pointers. Registered exit callbacks and the ELF destructor must execute before both process instances exit successfully.

Each run retains:

- `bionic-probe.elf`, `elf.txt`, and `link.map` for the actual executable and linked components.
- `compiler.txt` and `build-receipt.json` with NDK version, API target, and CRT/libc/libm/libdl input hashes.
- Build/QEMU/serial logs and `summary.json`, including live/reclaimed mapping counts and unsupported syscall numbers.

Debug sections are stripped by the linker to fit the loader's existing 2 MiB file limit; symbols and the link map remain available. The build never modifies the SDK or downloads dependencies. Generated linked binaries contain NDK components and remain local ignored artifacts; consult the installed NDK's `NOTICE` files before redistributing them.

This gate validates static native Bionic execution. Threads, signals, file-backed mappings, the dynamic linker, JNI, ART, Binder, Framework, and general APK execution are not established by it. See [the implementation record](../../docs/milestones/androidbox/BIONIC_STATIC_0.md).
