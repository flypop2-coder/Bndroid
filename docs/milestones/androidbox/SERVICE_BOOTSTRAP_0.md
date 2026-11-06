# Service Bootstrap-0: launch identities and service event loops

Date: November 6, 2026.

The subsequent [Service Policy-1](SERVICE_POLICY_1.md) milestone completes the
original daemon lookup/ping gate. This earlier bootstrap record retains its
initial failures and boundaries.

This extends [Binder Files-2](BINDER_FILES_2.md) toward real Android service
startup. It adds Init-selected executable/credential profiles, native authority
over Binder context-manager registration, timerfd and direct ppoll readiness.
The unmodified source ServiceManager is now an acquired input and a separate
integration command. Its acceptance remains independent of the owned transport
fixture; general Android applications and A1–A6 are still incomplete.

## Controlled launches

The build-time launch manifest accepts up to eight consecutive `profile=N`
sections, each with its own arguments, environment, UID/GID and
`binder_managers` domain bitmask. Values are checked before kernel generation:
canonical executable paths, unique environment names and credentials, bounded
entry counts and a complete startup stack within one page. The source interpreter
remains fixed. Profile zero preserves legacy single-program launches and their
application credentials.

Native `ProcessSpawn` keeps its low flag bits reserved and uses the high 32 bits
as a Linux profile selector. Only Init can invoke it. Non-Linux images reject
nonzero selectors, and undefined profiles fail before process publication or
startup-handle transfer. There is no guest-writable launch/credential file.
Credentials are immutable and process-owned; all threads use the same identity.
Bionic UID/GID/res-ID calls, AT_UID/AT_EUID/AT_GID/AT_EGID, generated signal sender
identities and Binder sender UIDs agree. This does not implement Linux set-ID,
supplementary-group mutation, capabilities or a complete per-application policy.

The Binder manager mask is separate from UID. A service can be authorized for one
device domain while an application is denied all three; having a system UID does
not manufacture that capability. Legacy transport fixtures retain their previous
domain authority. Actual Android service/permission and security-context policy
still requires integration with native authority.

## Event interfaces

Timerfd implements monotonic, boottime and RTC-anchored realtime timers, relative
and absolute deadlines, one-shot/periodic expiration accounting, disarming,
gettime/old-value reporting, CLOEXEC/nonblocking flags and shared descriptor
lifetime. Missed periods accumulate without shifting the next scheduled period.
Timer reads and nested/edge/one-shot epoll watches consume actual expirations.
Blocking reads use the existing signal interrupt/restart and owner-stack return
paths. Cancellation on wall-clock changes and alarm wakeup clocks are rejected;
this runtime has no clock setters or suspend support.

Direct ppoll uses retained descriptions and caller-specific Binder snapshots,
including nested epoll readiness. Invalid descriptors report POLLNVAL; negative
descriptors are ignored; O_PATH descriptors report POLLNVAL. Readable/writable normal-event bits, timeouts, empty
arrays and temporary signal masks use the existing wait machinery. Timer wakeups
do not copy into a foreign address space. Socket/pipe polling and select remain
open; the raw ppoll syscall does not yet write back its remaining timeout (Bionic
copies that input for its public ppoll wrapper). This is not full Linux polling
or networking compatibility.

## Source inputs and commands

The SDK user image denies adb-shell reads of servicemanager. The offline
extension validates matching primary/backup GPT CRCs and liblp geometry/metadata
SHA-256 checksums, bounded single-device linear extents, and regular EROFS inputs.
It checks the existing app_process/libbinder anchors before adding files and
hashes the original image before and after extraction. Source images are never
mounted or modified. erofs-utils checks and reads temporary partition copies;
those copies are removed afterward.

The local set grows from 349 to 354 files, from 265,085,778 to 265,938,573 bytes.
The five additions are the original servicemanager, libvintf, its init metadata
and platform/system_ext service-context metadata. The preceding receipt is kept
as `services-base-provenance.json`; all prior file records remain unchanged.
Runtime binaries, source notices, SDK files and execution artifacts stay ignored
and local. No application/GMS/vendor package is imported.

```sh
python3 scripts/extract-android-services.py \
  --runtime .bndroid/runtime-inputs/framework \
  --system-image "$ANDROID_SDK_ROOT/system-images/android-36.1/google_apis_playstore/arm64-v8a"
BNDROID_ART_MODE=framework-identities scripts/check-art-runtime.sh
BNDROID_ART_MODE=framework-services scripts/check-art-runtime.sh
```

`framework-identities` launches an owned system peer (UID/GID 1000) and an
application peer (10000), authenticates their Binder round trip, rejects manager
registration by the application, and exercises timers, signals and ppoll.
`framework-services` starts the unmodified daemon and an owned client using the
original libbinder_ndk service lookup/ping path. The client has a bounded failure
watchdog. Init reaps the client, terminates only its own long-lived daemon, then
reaps that daemon; this is distinguished from two normal application exits.
The service verifier requires real lookup/ping, Binder traffic, a service timer,
external Binder poll events and complete reclamation before reporting success.
Neither command reports general Android compatibility.

The source/ABI references are the primary
[ServiceManager startup](https://android.googlesource.com/platform/frameworks/native/+/master/cmds/servicemanager/main.cpp),
[access policy](https://android.googlesource.com/platform/frameworks/native/+/master/cmds/servicemanager/Access.cpp)
and [liblp metadata format](https://android.googlesource.com/platform/system/core/+/master/fs_mgr/liblp/include/liblp/metadata_format.h).
No upstream implementation source was copied into this repository.

## Verification and remaining integration

The combined host checks pass 1,171 workspace library tests, 440 service-profile
kernel library tests and 39 Python checks. Host library Clippy passes with warnings
denied. Complete AArch64 service-profile Clippy covers both tracing settings; a
redundant cast found in that final check was removed without changing behavior.
Parser cases reject corrupted GPT/liblp checksums/copies and out-of-bounds or
unsupported extents before extraction. The evidence parser rejects
missing identities, timers, ppoll, service traffic and daemon shutdown proof.

Final native identity/event evidence `check.5ahc7q` passes both credential profiles,
the real authenticated Binder request/reply, application-manager denial, four
returned signals, ten timers, two epoll instances and ppoll's invalid/negative/
O_PATH descriptor handling. Both workers and both processes exit/reap normally;
all 14 event sources, watches, Binder resources, pages, page tables and file
descriptions are reclaimed. The disk is unchanged. Earlier positive receipts
`check.JF0370` (before ppoll) and `check.QwW8BV` remain retained; the final receipt
includes the O_PATH check and the client watchdog source.

Full Framework regression `check.5PPxia` completes both normal Parcel/Bundle/Looper
startup/shutdown generations, with 163,372 Linux calls, a 102,853-page peak and
335,917 reclaimed pages. All 16 workers, Binder/event objects, pages, page tables
and file descriptions return to zero. Binder FD/object/nested-callback regression
`check.sOj8Xg` and eventfd/epoll regression `check.6TGEzH` also pass. All three
temporary disks remain unchanged. Default native boot passes as well, preserving
the existing native ProcessSpawn behavior.

The first original-service run `check.krA4SY` fails because Perfetto's task runner
calls the missing ppoll syscall (73). The daemon exits one; its client remains in
the upstream readiness-property wait. This diagnostic was stopped after that
failure was established; it is not cleanup or acceptance proof. Direct ppoll and
the bounded client watchdog were then added.

The corrected service attempt `check.XqLSfG` gets through Perfetto's ppoll path and
opens/maps Binder, then aborts at `selinux_status_open(true) >= 0`. The owned client
reports its readiness wait timeout, and Init's failure cleanup reaps both native
processes and both workers with zero live pages, tables, files, event objects,
watches or Binder connections. The disk is unchanged. This is a retained failed
integration result: `service_manager_verified=false`, and no ready property,
SELinux decision or successful lookup is fabricated.

Guest/host deadlines, heap, stack and RAM bounds remain unchanged. The interactive
temporary-disk preview is paused only during runtime batches and restored
afterward. Service readiness properties, SELinux/native policy mapping, service
registration/lookup completion, ActivityThread, package/window/input bridges and
complete application acceptance remain open. `full_android_apps` stays false.
