# Real Android dynamic linker fixture

This ordinary NDK-built PIE runs on the Bndroid kernel through unmodified Android `linker64`, `libc.so`, `libdl.so`, and `libm.so`. It verifies constructors, shared-libc identity, `dlopen`/`dlsym`/`dlclose`, dynamically loaded ELF TLS across timer preemption, destructors, allocation, file descriptors, and private file mappings. It is a native-runtime check, not an APK/ART test.

Prerequisites are the repository Rust toolchain, QEMU, Python 3.10+, NDK r27b (`27.1.12297006`), and a locally installed arm64 Android SDK system image plus SDK emulator/ADB. No script downloads components. Set `ANDROID_SDK_ROOT` and optionally `BNDROID_NDK_ROOT` when they are not in the usual macOS SDK location. The recorded source is Android 16, API 36.1, image revision 3; the C fixture targets NDK API 35. See [the milestone](../../docs/milestones/androidbox/BIONIC_DYNAMIC_0.md) for the exact fingerprint and limitations.

From the repository root, extract the four runtime files into a **new** output directory:

```sh
python3 scripts/extract-bionic-runtime.py \
  --system-image "$HOME/Library/Android/sdk/system-images/android-36.1/google_apis_playstore/arm64-v8a" \
  --output target/android-runtime-inputs/bionic
./scripts/check-bionic-dynamic-runtime.sh
```

The extractor creates its own temporary AVD, boots read-only with QEMU networking disabled, checks its unique AVD identity, addresses it by explicit emulator serial, pulls only the four runtime files, records hashes and source metadata, then stops its own process and removes the AVD. It never issues commands to physical devices. This source emulator is only a file acquisition helper; the compatibility gate runs separately on Bndroid's custom kernel.

For an alternative extraction directory:

```sh
BNDROID_ANDROID_RUNTIME_DIR=/absolute/path/to/runtime \
  ./scripts/check-bionic-dynamic-runtime.sh
```

The gate builds the PIE and plugin, validates extracted input hashes, packages an immutable USTAR root, boots Bndroid using temporary disk storage and fresh host entropy, and verifies two successful process generations with no remaining dynamic page/table owners or open file descriptions. Output goes to `target/bionic-dynamic/check.*/`: `summary.json`, UART logs, rootfs input receipts, compiler metadata, ELF metadata, and generated binaries. No upstream binary is checked into Git.

An intentional access to a complete page beyond file EOF must fault and release all child resources:

```sh
BNDROID_DYNAMIC_FAULT_CHECK=1 ./scripts/check-bionic-dynamic-runtime.sh
```

That negative gate expects `BIONIC_DYNAMIC_EOF_FAULT_CHECK_OK`; the supervisor intentionally reports the failed child afterward. This validates containment, not POSIX signal-handler delivery. For syscall diagnostics, add the kernel feature `linux-syscall-trace` to a manual build; keep it off for normal output assertions because tracing interleaves UART writes.
