# Service Bridge-2: registered provider lifecycle

The owned `bndroid-runtime-service` now registers through the original Android
ServiceManager and serves original NDK Binder clients in separate application and
maintenance processes. Its snapshot contains actual Bndroid ABI system facts and
authenticated caller identity. This extends [Service Policy-1](SERVICE_POLICY_1.md)
with a usable provider protocol, normal service shutdown, supervisor restart and
client rediscovery. A1–A6 and general Android applications remain incomplete.

## Provider and authority

The implementation lives in `user/runtime-service`; its
[protocol and launch instructions](../../../user/runtime-service/README.md)
describe the versioned interface. Requests reject missing/unsupported versions and
trailing data. The provider requests kernel-authenticated Binder security contexts
and checks an explicit FIND grant on every service operation. STOP additionally
requires an independent Init-issued `control_grant=CONTEXT`. Neither lookup nor a
system UID grants that right. The existing SELinux query adapter exposes the
native decision as `bndroid_service:stop`; it remains an adapter to immutable
native authority rather than a Linux SELinux policy engine.

Profiles allow at most 16 distinct exact-target control grants. The build receipt
records every launch identity, executable and service/property/control grant.
An owned product service-context overlay maps the new service name to its label.
Its hash and the two owned native binary hashes are recorded separately. All 354
verified source inputs and their context metadata remain unchanged; a conflicting
source overlay causes packaging to fail instead of being replaced.

The provider generates a fresh instance ID from kernel entropy. It reports boot
uptime, hostname, machine, provider PID/UID and Binder caller PID/UID. The uname
release is the Bndroid compatibility ABI version, not a Linux kernel version.
It uses the original Binder polling entry points; a successful STOP reply is
flushed before normal process exit.

## Supervision and client behavior

`ProcessWait` now accepts `PROCESS_WAIT_NONBLOCK`. A live child returns
`ShouldWait`, leaving its future completion unconsumed. Completed results retain
generation-qualified identity, one-time consumption and the existing exit reason.
Only Init can wait; unknown flags and the reserved third argument are rejected.
Blocking waits retain their prior behavior.

The demonstration starts these independent profiles:

| Profile | UID | Responsibility |
| --- | --- | --- |
| 0 | 1000 | Original ServiceManager and its own readiness property |
| 1 | 1001 | Runtime provider, registered under its exact service label |
| 2 | 10000 | Application reads; no registration or maintenance grant |
| 3 | 1002 | Maintenance client with explicit STOP authority |

The application verifies snapshot facts, caller identity and denied replacement,
STOP and malformed calls, then registers a death recipient and publishes its
granted readiness property. The controller observes readiness and requests STOP.
Init supervises all children without blocking behind a live daemon, consumes the
provider's normal exit, restarts the same immutable profile, and confirms the old
native process ID cannot be waited again. The application observes death once,
confirms its old object stays dead, and discovers a live replacement with a new
instance ID. It tolerates the interval between independent client/manager death
notifications by retrying actual dead or absent registrations.

The client/controller exit normally. Init separately terminates and reaps its
replacement provider and daemon. These are reported as three normal exits and
two forced exits, not five normal exits. Unexpected child exit triggers cleanup
of the remaining owned children and failure. General restart/backoff policy is
not implemented by this mode-specific orchestration.

## Evidence

```sh
BNDROID_ART_MODE=framework-bridge scripts/check-art-runtime.sh
```

Runtime `check.EHFFep`, with syscall tracing enabled, passes 53 Binder requests
and replies with 53 authenticated security contexts, two delivered death
notifications, and the full stop/restart/rediscovery sequence. Its five process
generations and three workers are reaped. All Binder connections, transactions,
buffers, nodes, references, call frames and pending file transfers return to
zero. All nine event objects close, with no watches remaining. Anonymous pages,
tables, file descriptions and native handles return to zero; the temporary disk
is unchanged. The run makes 6,641 Linux calls, peaks at 13,268 anonymous pages,
and reclaims 18,998 pages. `service_bridge_verified=true` and
`service_manager_verified=true`; `full_android_apps=false`.

The evidence parser requires both distinct provider instances, matching client
and controller records, three distinct successful process owners, all five
reaped generations, denied unauthorized operations, real Binder traffic/deaths,
and complete cleanup. Duplicate, stale, malformed or missing records fail.
Host validation passes 1,178 workspace library tests, 446 service-profile kernel
library tests and 41 Python tests. Host library Clippy passes with warnings denied.
Complete AArch64 bridge builds also pass Clippy with and without syscall tracing.
Original ServiceManager regression `check.9MEH5C` and the default native boot
gate both pass, preserving blocking child waits and existing resident services.

This native provider is not ActivityManager, PackageManager or a substitute
ActivityThread. Managed Framework clients/providers, CoreSystem-owned dynamic
application policy, resident supervision and package/window/input integration
remain necessary. Current optional tracing/priority/socket gaps are retained in
runtime diagnostics; passing this lifecycle gate does not imply general Linux
syscall or Android application compatibility.
