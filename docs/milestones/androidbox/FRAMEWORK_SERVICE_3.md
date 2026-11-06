# Framework Service-3: Java client and native provider

This extends [Service Bridge-2](SERVICE_BRIDGE_2.md) with a reusable Java binding
and a service client running through the original `app_process64`, ART and
Framework. The original ServiceManager and native runtime provider remain
independent processes. No boot class, original library or service daemon is
replaced. ActivityThread, normal APK launch and the complete A1–A6 matrix remain
unfinished.

## Interface and lifecycle

`user/runtime-service/java/org/bndroid/runtime/PlatformRuntime.java` accepts a
discovered `IBinder`, validates its descriptor and decodes the versioned native
snapshot protocol through public Framework Parcel APIs. It rejects malformed
replies and recycles every request/reply parcel, including exception paths.
Its maintenance method preserves the native provider's independent control grant;
the ordinary application UID has no such authority.

The integration client reaches the original platform ServiceManager and
SystemProperties APIs through reflection because public SDK compile stubs omit
them. It does not install API exemptions or replacements. The reusable binding
does not contain hidden service discovery. The upstream contracts are
[ServiceManager](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/core/java/android/os/ServiceManager.java),
[SystemProperties](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/core/java/android/os/SystemProperties.java)
and [app_process startup/shutdown](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/cmds/app_process/app_main.cpp).

The application verifies actual kernel facts and caller identity, denied service
replacement and STOP, and native argument exceptions decoded by Java Parcel.
A Binder worker receives provider death and posts it to the application's main
Looper. After that handoff, the old proxy must remain dead and a fresh lookup must
return the restarted provider's new instance. Acceptance also requires a normal
VM shutdown and full native resource reclamation.

The controller waits on the original Bionic property reader/wait interface,
using shared serial/futex wakeups rather than repeated readiness polling. This
managed-start mode has a 180-second readiness bound and a 240-second controller
watchdog. Native mode retains its 5-second readiness and 20-second watchdog
bounds. Kernel/host deadlines, RAM, stack and kernel-heap limits are unchanged.

## Concurrent mapping metadata

The first concurrent ART attempt, `check.kGPqVF`, fails during Framework DEX
mapping with out-of-memory errors. ART exits with code 1; Init cleans up the other
three children, and pages, tables, files and worker threads return to zero. The
failure evidence and unchanged temporary disk are retained; this is not a
successful Framework run.

Review found that every Linux VMA change preallocated the full 4,096-entry
Framework region quota, including tiny native services sharing the 8 MiB kernel
heap with ART. Planning now reserves the existing region count plus room for the
two interval boundaries and replacement. It still allocates before publication,
enforces the same region limit, and leaves the original layout intact on failure.
The change also applies to inheritance advice; it does not raise memory limits.

The next run, `check.5Q4Bky`, gets through ART initialization and original service
discovery but fails its first Java snapshot transaction. The old transport flag
mask rejects Framework's `FLAG_COLLECT_NOTED_APP_OPS` bit (2). Both the frozen
local BinderProxy DEX and the upstream
[BinderProxy implementation](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/core/java/android/os/BinderProxy.java)
show that Framework may add this bit before JNI dispatch. The adapter now forwards
that userspace metadata unchanged, preserving authenticated sender identity and
normal reply routing. It does not implement AppOps policy or grant permissions.
Unsupported kernel operations such as async transaction replacement remain
rejected. Failed-transfer diagnostics now include flags. This failure's nine
workers and all resources are reclaimed; its fatal Java exit is not accepted as
a normal VM lifecycle.

`check.olTqDH` completes Java calls, permissions, Binder-worker/Looper death
handoff, rediscovery and normal VM exit with all resources reclaimed. Its strict
evidence gate still fails: libselinux's default stderr audit callback omits a
newline, joining the next process's authority-stage record on the UART stream.
The owned platform adapter now uses the standard libselinux logging callback to
send audits through original Android liblog. This preserves structured record
boundaries and the audit message. The parser continues to reject joined records.

Host checks cover small-service metadata use, boundary splitting, preservation of
original mappings and recovery after the hard region limit. The updated default
kernel library passes 442 tests; the Framework/service profile passes 448, with
warnings denied by host Clippy. All 41 Python evidence tests pass, including
rejection of missing/wrong-owner Framework stages, native-only client records and
missing VM shutdown.

```sh
BNDROID_ART_MODE=framework-client scripts/check-art-runtime.sh
```

The final runtime `check.p0YgWP` passes the complete Java/Framework service path
and normal VM shutdown. It completes 37 Binder requests/replies with 37
authenticated security contexts and two delivered death notifications. All five
process generations and nine workers are reaped, with three normal exits and two
owned daemon/provider terminations. All 12 event objects close; Binder objects,
pending work, watches, pages, tables, file descriptions and native handles return
to zero. The run makes 92,763 Linux calls, peaks at 112,922 anonymous pages and
reclaims 182,989 pages. The temporary disk is unchanged.
`framework_service_client_verified=true`, while `full_android_apps=false`.
Build and execution take 204.8 host seconds; this is functional evidence, not a
claim of product performance or a measured timing margin.
The final native-client regression `check.kEo7NP` and Binder transport regression
`check.Gh7ux2` also pass with complete cleanup and unchanged temporary disks.
Complete AArch64 bridge builds pass Clippy with warnings denied, with and without
syscall tracing. No broader application or performance acceptance is inferred
from these runs.

The remaining application work includes managed providers, resident supervision,
CoreSystem-owned dynamic permissions, ActivityThread and package/window/input
integration. This command is a platform service integration; it does not launch
an unmodified APK activity.
