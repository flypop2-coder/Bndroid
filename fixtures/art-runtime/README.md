# Local ART bring-up

This fixture invokes the installed image's unmodified `dalvikvm64` and `libart.so` on Bndroid. The default `java` gate runs the standard DEX program twice, including collections, strings, arithmetic and a caught exception, and verifies normal exits and complete resource cleanup. The `help`, `memory` and `signals` gates separately verify the ART frontend and native kernel prerequisites. These are not APK, ActivityThread or Framework acceptance gates.

The `managed` gate loads DEX and an aligned JNI library directly from an owned, signed APK. Its command-line entry validates Java threads/monitors/interruption, moving GC and weak references, JNI arrays/exceptions and an attached native thread. It verifies classloader namespace isolation and complete cleanup in two generations. This exercises APK contents through ART and the linker; ActivityThread, package installation and app graphics remain separate work.

Prerequisites are the existing arm64 SDK system image, emulator/ADB, NDK r27b, SDK build-tools 36.1.0, JDK, Rust and QEMU. There are no downloads. [ADR 0001](../../docs/decisions/0001-android-runtime-baseline.md) records the frozen baseline and local-only distribution boundary.

```sh
python3 scripts/extract-bionic-runtime.py \
  --system-image "$HOME/Library/Android/sdk/system-images/android-36.1/google_apis_playstore/arm64-v8a" \
  --output target/android-runtime-inputs/art --include-art
BNDROID_ART_MODE=help scripts/check-art-runtime.sh
BNDROID_ART_MODE=memory scripts/check-art-runtime.sh
BNDROID_ART_MODE=memfd scripts/check-art-runtime.sh
BNDROID_ART_MODE=properties scripts/check-art-runtime.sh
scripts/check-art-signals.sh
scripts/check-art-runtime.sh
BNDROID_ART_MODE=managed scripts/check-art-runtime.sh
```

The extraction destination must not already exist. Set `BNDROID_ART_RUNTIME_DIR` for another receipt directory. The helper uses an isolated, read-only, offline temporary AVD, pins its own serial and identity, copies only the selected runtime/core-Java inputs, retains notices and provenance, and terminates its owned process. It never operates existing AVDs or physical devices. The measured extraction has 66 named inputs / 74,543,749 bytes, including resolved symlink sources and dependencies from the ART, runtime, i18n and statsd APEXes and platform shared libraries. APEX identities, linker configuration, boot-classpath environment and hashes remain beside the files under ignored `target/`. Receipts predating namespace metadata must be re-extracted into a new directory; the builder rejects missing or altered configuration hashes.

Managed mode also uses the existing Android 36 platform jar, aapt2, zipalign and apksigner. It signs the owned `org.bndroid.runtimeprobe` APK with the repository's public test-only AndroidBox key, verifies APK v2 signing and alignment, and retains `managed.apk`, `apk-signature.txt` and `apk-badging.txt`. The root contains the APK and an intentionally inaccessible system-library copy for the namespace negative check, with no loose managed DEX or extracted application library. The root receipt includes the APK hash. No production signing material or upstream runtime binary is imported.

The ART-only gates build a verified immutable USTAR root, a standard DEX program, native probes, and a separate `art-runtime` kernel. QEMU receives 512 MiB RAM, one CPU, a temporary writable disk overlay, and fresh host entropy. A supervisor retains `serial.log`, input receipts, build logs and `summary.json`, then stops only its QEMU process. Tracing is off by default; `BNDROID_ART_SYSCALL_TRACE=1` enables per-syscall/abort diagnostics, which add substantial overhead and can interleave console output. ART has a 90-second guest limit and a 120-second default host timeout. A missing proof, unexpected exit, fault, panic or timeout fails the command.

Java mode currently uses ART's interpreter option, a 4 MiB initial / 16 MiB maximum heap, matching core-library DEX jars, and no image compiler. These diagnostic limits reduce initial memory demand; they are not the final app runtime configuration. The program must produce its collection/arithmetic/exception marker in two distinct process generations before `managed_java_verified` becomes true.

`check-art-signals.sh` runs one positive handler/synchronization gate and five deliberately failing programs. They test corrupt PSTATE, kernel PC, malformed FPSIMD data, an inaccessible alternate stack and group SIGKILL with a waiting sibling and full realtime queue. Negative modes pass only when the expected group fault is followed by complete cleanup. They can be selected individually with `BNDROID_ART_MODE=signals`, `signal-bad-frame`, `signal-bad-pc`, `signal-bad-fp`, `signal-guard` or `signal-default`. Run ART gates sequentially: they share a build directory but own separate disks and QEMU processes.

The root includes verified global/ART linker namespace policy and canonical Bionic aliases; it no longer uses a global diagnostic library search path. The memory probe also covers process snapshots/accounting, stack guards, large reads, shared read-only files, anonymous remapping and the bounded Android log endpoint.

The memory probe now checks actual anonymous discard/RSS reduction, user/kernel demand-zero faults, retained protections and process/thread CPU clocks across sleep and thread reaping. Shared/file discard and a general demand-paged mapping policy remain unsupported. The managed workload retains a 16 MiB Java heap and performs 64 MiB allocation turnover per generation; this is a measured integration workload, not a broad app-memory guarantee.

The `memfd` gate checks real shared physical backing, positional/shared-offset I/O,
descriptor reuse, mapped lifetime after close, truncate/SIGBUS and zero growth,
write/grow/shrink/future-write seals, and generated AArch64 code through separate
RW/NX and R/X aliases. Each generation must return from three deliberate faults:
a write to RX, execution from RW, and a load beyond truncated EOF. All six signal
frames and all process resources must be reclaimed. `shared_memfd_verified` is
independent of managed Java and Framework acceptance. Single RWX mappings remain
forbidden; private memfd mappings require future COW work and return ENOSYS.
[ADR 0002](../../docs/decisions/0002-linux-abi-release-and-code-cache.md) explains
the partial ABI release string, code publication model and unchanged memory bounds.

See [ART Managed-0](../../docs/milestones/androidbox/ART_MANAGED_0.md) for current APK/GC/JNI evidence and limits. [ART Signals-0](../../docs/milestones/androidbox/ART_SIGNALS_0.md), [ART Process-0](../../docs/milestones/androidbox/ART_PROCESS_0.md) and [ART Frontend-0](../../docs/milestones/androidbox/ART_FRONTEND_0.md) retain earlier milestones. Missing timezone/proc/tracing data and unsupported optional scheduler/directory calls remain visible in the summary; `full_android_apps` remains false.

## Framework entry

Extract the full source boot classpath, platform resources, timezone data and the recursive native dependencies of unmodified `app_process64` into a separate receipt directory:

```sh
python3 scripts/extract-bionic-runtime.py \
  --system-image "$HOME/Library/Android/sdk/system-images/android-36.1/google_apis_playstore/arm64-v8a" \
  --output target/android-runtime-inputs/framework --include-framework
BNDROID_ART_MODE=framework-help scripts/check-art-runtime.sh
BNDROID_ART_MODE=framework scripts/check-art-runtime.sh
```

The measured input set contains 348 files / 265,050,082 bytes and 46 boot jars, including the source netbpfload version metadata read by libmeminfo. All 66 earlier ART inputs retain their original hashes. Required libraries are selected from the active Android APEX/platform inventory; the original global and ART namespace policy remains in force. This does not acquire Google applications, Play services or vendor APKs. Inputs/notices stay local and are not included in Git.

If the Framework extension is interrupted after its ART receipt was produced, repeat extraction with `--resume-framework --include-framework` and the same output. This also upgrades an older completed receipt missing the version metadata. The helper checks source identity, existing file/metadata hashes and each saved progress entry. It atomically checkpoints successful pulls and replaces the final receipt only after dependencies resolve. A completed receipt containing all required metadata returns without starting an emulator. Each actual extraction still uses and stops its own temporary offline AVD.

Framework modes select `framework-runtime`: one CPU, 1 GiB guest RAM, a 320 MiB immutable archive / 512-entry catalog, 512 file descriptors, 4,096 mapping regions, a combined 512 MiB physical budget for private pages and memfd backing, and 512 page-table pages. Shared aliases do not allocate a second physical copy. The ART-only bounds remain unchanged. The Framework guest/host timeouts are 300/360 seconds because loading and reclaiming hundreds of libraries exceeds the earlier two-launch bound. The source boot and compiler boot classpaths are supplied through the startup environment. No alternate Java/Framework implementation or global library search override is supplied.

`framework-help` requires authenticated AndroidRuntime start/zero-exit and ART help records from both process generations, followed by complete native cleanup. Help is logged through AndroidRuntime's `vm-printf` hook rather than standard output. It proves native loading and argument parsing, not Java VM or Activity startup. `framework` is a separate integration gate: only two actual Parcel/Bundle round trips and delayed Looper callbacks through upstream RuntimeInit may satisfy it. A missing stage remains failure; `full_android_apps` stays false even if this narrower probe eventually passes.

The read-only boot-property area contains declared public ABI, native-bridge and Bndroid product values, plus the version metadata of the frozen API 36.1 / Android 16 inputs. The builder rejects a different API source instead of guessing its build version. These values describe the supplied Framework libraries and do not claim full application compatibility. Unmodified Bionic reads its supported pre-split binary format; there is no writable property service or private property namespace. `properties` independently verifies lookup, callbacks, all 13 properties, real timed waiting, CLOCK_BOOTTIME advancement across that wait, and rejected file/shared-map writes. The memory gate additionally verifies fixed-address anonymous relocation, including equal-size/shrink/growth, preserved read-only access, destination replacement, neighboring mappings and atomic failure.
