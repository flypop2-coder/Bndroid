# Native runtime service

`bndroid-runtime-service` is an owned userspace provider registered as
`bndroid.runtime` with the original Android ServiceManager. It uses the verified
Android Binder NDK and libselinux libraries. No source Android library or
service-context file is patched. A separately hashed product service-context
overlay declares `u:object_r:bndroid_runtime_service:s0`.

The service returns actual Bndroid Linux-ABI facts: an entropy-derived instance
ID, boot uptime, provider PID/UID, authenticated caller PID/UID, hostname, ABI
release and machine architecture. The release is Bndroid's compatibility ABI
version, not a claim that a Linux kernel is running.

## Wire protocol

The descriptor is `org.bndroid.runtime.IPlatformRuntime`. Requests use an ordinary
NDK Binder parcel/interface token followed by exactly one int32 version (`1`).
Missing, unsupported or trailing fields return `EX_ILLEGAL_ARGUMENT`.

| Transaction | Reply after the AIDL status header |
| --- | --- |
| `SNAPSHOT` (1) | int32 version, uint64 instance, int64 uptime in ns, int32 provider PID/UID and caller PID/UID, strings hostname/release/machine |
| `STOP` (2) | No payload; the service flushes the reply and exits normally |

Every call requires the authenticated Binder sender SID and an explicit native
FIND grant for the target service. STOP additionally requires a separate
`control_grant` issued by Init. Lookup permission or possession of a Binder
object gives no maintenance authority. The `bndroid_service:stop` access query
projects that native grant through the existing SELinux ABI adapter. There is
no dynamic Linux policy engine. Unknown transactions return
`STATUS_UNKNOWN_TRANSACTION` after version/authority validation.

## Running the lifecycle demonstration

With the verified local Framework/service inputs described in
[Service Policy-1](../../docs/milestones/androidbox/SERVICE_POLICY_1.md):

```sh
BNDROID_ART_MODE=framework-bridge scripts/check-art-runtime.sh
```

The immutable image starts four profiles: ServiceManager (UID 1000), provider
(1001), application client (10000), and maintenance controller (1002). The client
checks facts, caller identity, rejected registration/stop/malformed requests and
then links a death recipient. Its granted readiness property allows the
controller to request a normal provider shutdown. Init waits without blocking
on a live child, reaps the old generation and restarts the same provider profile.
The client receives death, confirms its old reference stays dead, discovers the
new registration and observes a different instance ID. The controller and
client exit normally; Init separately terminates/reaps its replacement provider
and daemon. No success record is accepted until all five process generations
and their resources are reclaimed and the temporary disk is unchanged.

The service implementation is reusable; the controller/client under
`fixtures/art-runtime` supply bounded demonstration orchestration. The current
Init restart policy is specific to this mode. General resident supervision,
CoreSystem-owned dynamic permissions, ActivityThread and Android application
services remain separate work. This mode does not establish general APK support.

## Java binding and Framework integration

`java/org/bndroid/runtime/PlatformRuntime.java` is a reusable Java binding that
accepts a discovered `IBinder`, checks its descriptor, decodes versioned snapshots,
and exposes maintenance requests with the same independent native permission.
Parcels are recycled on success and failure; malformed replies are rejected.
Its returned `IBinder` supports the original Framework death-recipient API.

```sh
BNDROID_ART_MODE=framework-client scripts/check-art-runtime.sh
```

This mode substitutes the original `app_process64`/ART and owned
`FrameworkServiceMain` for the native application client, with the same UID and
grants. The Java client uses the original Framework ServiceManager, Parcel,
Binder and SystemProperties implementations. A Binder-worker death notification
is posted to its main Looper before rediscovery, and acceptance requires normal
VM shutdown and full reclamation. Platform lookup/property methods omitted from
SDK stubs are reached by reflection in this integration harness; no hidden-API
exemption or replacement boot class is installed. The reusable binding itself
uses public Binder/Parcel APIs and does not perform hidden service lookup.

The maintenance controller waits on Bionic's shared property futex instead of
polling readiness. This mode allows up to 180 seconds for Framework initialization
and has a 240-second controller watchdog; native-only mode keeps its existing
5-second readiness bound and 20-second watchdog. Kernel/host, memory and stack
limits remain unchanged. This is a platform service integration, not APK launch
through ActivityThread or a general Android application lifecycle.
