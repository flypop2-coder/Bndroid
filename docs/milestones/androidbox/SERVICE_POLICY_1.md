# Service Policy-1: original ServiceManager and native authority

Date: November 6, 2026.

The unmodified SDK ServiceManager now starts on Bndroid, registers itself,
publishes its own readiness property and answers the original libbinder_ndk
lookup/ping path from a separate application process. This extends
[Service Bootstrap-0](SERVICE_BOOTSTRAP_0.md); it does not complete ActivityThread,
general Android applications or the full A1–A6 acceptance matrix.

## Native service grants and the SELinux ABI

Init's immutable launch profiles now carry explicit `service_grant=MASK CONTEXT`
and `property_grant=NAME` entries. `bndr-sm::service_authority` owns the exact-name
grant calculation; the Android adapter projects those decisions into the
`service_manager` class. UID alone grants no service or property rights. Profiles
permit at most 32 service grants and 16 property grants, reject duplicates and
unknown rights, and cannot grant writes to `ro.*` boot properties.

The adapter exposes an enforcing status page, class/permission discovery,
context validation and per-open access-query replies. Unknown classes, contexts,
permissions and ungranted services fail closed. Process contexts come from the
actual selected launch profile through proc attributes. The immutable source
service-context files supply target labels, not allow rules. Directory reads
cover the catalog's explicit and implied children with independent open offsets
and checked copyout.

The SELinux ABI is an adapter to native authority, not a Linux SELinux engine.
There is no policy-binary interpreter, policy reload, permissive switch, general
MAC implementation or dynamic CoreSystem permission database in this milestone.
The status page's enforcing flag and fixed boot revision describe the immutable
native grant policy. Unsupported policy interfaces remain absent.
The relevant upstream contracts are
[libselinux mount discovery](https://raw.githubusercontent.com/SELinuxProject/selinux/main/libselinux/src/init.c),
[class discovery](https://raw.githubusercontent.com/SELinuxProject/selinux/main/libselinux/src/stringrep.c)
and [access queries](https://raw.githubusercontent.com/SELinuxProject/selinux/main/libselinux/src/compute_av.c).

Binder supports requesting a sender SID. The connection captures its native
context at open; the receiver gets a NUL-terminated context in its read-only
receive allocation and a `BR_TRANSACTION_SEC_CTX` record. A caller's claimed UID
or parcel contents cannot choose that context. Short receive buffers preserve
queued work; failed delivery releases the reserved payload without reporting a
committed SID. The SID survives the sender's userspace descriptor lifetime.

## Live readiness properties

The bounded native property endpoint accepts the original Bionic protocol-v2
stream request, including fragmented writes and partial/peek response reads.
Only the authenticated current process's explicit property grants authorize a
mutation. Read-only facts and undeclared keys cannot be changed or inserted.
Requests and values have fixed size limits; malformed packets fail before a
successful publication. This is a native property endpoint adapter, not general
Unix-stream networking, Android init, persistent properties or arbitrary socket
listen/accept support.

Each property file open has its own description while mappings retain the same
read-only, non-executable backing. Updates publish the dirty backup, value and
clean serial in order, then advance the area serial and wake waiters. Shared
memory-file futexes use inode/offset identities across different virtual
addresses; private futexes retain process-generation/VA identities. Requeue
preserves deadlines and bitsets across both kinds of keys. Final Linux process
teardown releases the property area's retained backing.

The boot image declares the property protocol version and an empty
`servicemanager.ready`. The original daemon must actually write `true`; the
build never pre-publishes readiness. Application writes to that key are denied.
The wire and reader references are
[Bionic property requests](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/bionic/system_property_set.cpp)
and [Bionic serial publication and waits](https://raw.githubusercontent.com/aosp-mirror/platform_bionic/android16-release/libc/system_properties/system_properties.cpp).

## Commands and current evidence

Use the unchanged 354-file local input set acquired in Service Bootstrap-0.
No additional upstream binary, SDK image or source notice is added to Git.

```sh
BNDROID_ART_MODE=framework-identities scripts/check-art-runtime.sh
BNDROID_ART_MODE=framework-services scripts/check-art-runtime.sh
```

The final owned gate `check.kavy9Z` checks both native identities through the
original libselinux client, allowed lookup, system-only registration authority,
denied unknown services, immutable enforcement/process contexts, denied property
spoofing and shared publication through Bionic. Its Binder request delivers the
authenticated application SID; timers, ppoll and all four signal returns pass.
Both workers/processes are reaped with zero live pages, tables, file descriptions,
event objects, watches or Binder resources. The temporary disk is unchanged.

Original-service evidence `check.bj4Ejz` completes four request/reply exchanges
with four authenticated SIDs, the daemon's own property publication, original
lookup/ping and denied unknown-service lookup. Its native event loop creates a
timer and receives six external Binder notifications. Init reaps the successful
client, terminates only its owned long-lived daemon and reaps it separately.
Both workers, all six event sources and all remaining resources are reclaimed;
the disk is unchanged. `service_manager_verified=true`, while
`full_android_apps=false`. Earlier successes `check.boDV7b` and `check.hJWopw` remain retained.

The owned diagnostic `check.CGqXGL` exited normally but failed evidence parsing:
buffered stdout crossed the console's short-write boundary and a syscall trace
split an identity record. The fixture now line-buffers its records. The final
owned and original-service gates above both pass with syscall tracing enabled.

Host validation passes 1,177 workspace library tests, 445 service-profile kernel
library tests and 40 Python checks. Host library and complete AArch64 service
builds pass Clippy with warnings denied, including both syscall-tracing settings.
The tests include malformed property packets,
unauthorized grants, corrupt area pointers, private/shared futex separation,
SID receive failure and strict negative evidence checks. Binder regression
`check.mWZCh0` passes FD transfer, failed installation, nested callbacks and
complete cleanup. Full Framework regression `check.Jy0dCA` completes two normal
Parcel/Bundle/Looper and VM-shutdown generations with 163,361 Linux calls,
a 102,853-page peak and 335,990 reclaimed pages. All 16 workers and all resources
return to zero; its disk remains unchanged.

Earlier Framework attempt `check.xmjHo2` completes both business checks but
exceeds the existing guest deadline during the second VM shutdown. This is a
retained failure, not cleanup proof. Shared futex lookup now uses binary search
in ordered VMAs, and ordinary file descriptions no longer allocate inline native
endpoint exchange state. The repeat above passes under unchanged limits. This
single repeat does not establish a performance margin or eliminate the broader
runtime timing work required by A6.

Guest/host deadlines, heap, stack and RAM bounds remain unchanged. The interactive
temporary-disk preview is restored after the runtime batch. Next integration
requires separately registered providers and app clients, service death/restart,
CoreSystem-owned dynamic permissions, ActivityThread, package/window/input
bridges and the actual application compatibility matrix. This bounded daemon
acceptance does not mark A2 or the complete application goal achieved.
