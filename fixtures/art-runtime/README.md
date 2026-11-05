# Local ART bring-up

This fixture invokes the installed image's unmodified `dalvikvm64` and `libart.so` on Bndroid. The `help` gate verifies ART's own command-line frontend and complete native process cleanup. The `memory` gate verifies kernel prerequisites with an owned NDK program. Neither is a managed Java or APK success claim. The default `java` gate loads ICU/core/OpenJDK native libraries, then fails during signal-dependent core initialization; it preserves that failure rather than reporting success.

Prerequisites are the existing arm64 SDK system image, emulator/ADB, NDK r27b, SDK build-tools 36.1.0, JDK, Rust and QEMU. There are no downloads. [ADR 0001](../../docs/decisions/0001-android-runtime-baseline.md) records the frozen baseline and local-only distribution boundary.

```sh
python3 scripts/extract-bionic-runtime.py \
  --system-image "$HOME/Library/Android/sdk/system-images/android-36.1/google_apis_playstore/arm64-v8a" \
  --output target/android-runtime-inputs/art --include-art
BNDROID_ART_MODE=help scripts/check-art-runtime.sh
BNDROID_ART_MODE=memory scripts/check-art-runtime.sh
scripts/check-art-runtime.sh
```

The extraction destination must not already exist. Set `BNDROID_ART_RUNTIME_DIR` for another receipt directory. The helper uses an isolated, read-only, offline temporary AVD, pins its own serial and identity, copies only the selected runtime/core-Java inputs, retains notices and provenance, and terminates its owned process. It never operates existing AVDs or physical devices. The measured extraction has 66 named inputs / 74,543,749 bytes, including resolved symlink sources and dependencies from the ART, runtime, i18n and statsd APEXes and platform shared libraries. APEX identities, linker configuration, boot-classpath environment and hashes remain beside the files under ignored `target/`. Receipts predating namespace metadata must be re-extracted into a new directory; the builder rejects missing or altered configuration hashes.

Each gate builds a verified immutable USTAR root, a standard DEX program, a native memory probe, and a separate `art-runtime` kernel. QEMU receives 512 MiB RAM, one CPU, a temporary writable disk overlay, and fresh host entropy. A supervisor retains `serial.log`, input receipts, build logs and `summary.json`, then stops only its QEMU process. Java mode enables syscall/abort-message diagnostics by default; `BNDROID_ART_SYSCALL_TRACE=0` disables them. A missing proof, unexpected exit, fault, panic or timeout fails the command.

Java mode currently uses ART's interpreter option, a 4 MiB initial / 16 MiB maximum heap, matching core-library DEX jars, and no image compiler. These diagnostic limits reduce initial memory demand; they are not the final app runtime configuration. The program must produce its collection/arithmetic/exception marker in two distinct process generations before `managed_java_verified` becomes true.

The root includes verified global/ART linker namespace policy and canonical Bionic aliases; it no longer uses a global diagnostic library search path. The memory probe also covers process snapshots/accounting, stack guards, large reads, shared read-only files, anonymous remapping and the bounded Android log endpoint.

See [ART Process-0](../../docs/milestones/androidbox/ART_PROCESS_0.md) for current kernel changes, actual evidence and remaining work, and [ART Frontend-0](../../docs/milestones/androidbox/ART_FRONTEND_0.md) for the earlier loading milestone.
