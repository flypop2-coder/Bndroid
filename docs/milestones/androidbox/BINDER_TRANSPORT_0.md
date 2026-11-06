# Binder Transport-0: context-manager byte IPC

Date: November 6, 2026.

The ART profile now exposes a real LP64 `/dev/binder` adapter on the custom
AArch64 kernel. Two concurrent native Bionic processes exchange payloads, receive
replies and observe service death. The original Framework starts its Binder pool
and completes the owned Java entry's Parcel/Bundle checks. Its Looper still fails
at eventfd creation. This milestone does not complete A1, A2 or general Android
application support.

## Implemented transport

The driver supports protocol version 8, context-manager registration, bounded
thread-pool controls and `BINDER_WRITE_READ`. Handle zero addresses the registered
context manager. Synchronous byte requests receive replies on the originating
thread; one-way work is FIFO for the context node and advances when the receiver
frees its buffer. Accepted one-way work survives sender exit. Payloads and command
headers are copied into kernel-owned storage before publication.

Caller PID/UID come from the kernel's connection owner, not user-supplied
transaction fields. Connection identity includes the native process generation;
wire PIDs use the existing Linux view. The runtime still has one application UID
(10000); host model tests also exercise different UIDs. Only one context manager
exists, and its UID remains pinned after exit. This is not a per-app permission
or capability-policy implementation.

Each open file owns a connection. Duplicates and in-flight ioctls retain that
connection across descriptor close/reuse. The receive mapping retains only its
backing pages, so it does not keep a closed service alive. It is read-only,
non-executable and not inherited; later mprotect cannot grant write or execute.
Unmapping/replacing any portion invalidates the receive pool and fails its pending
work. A live connection permits one mapping, up to 4 MiB.

Blocking reads use the existing scheduler wait queue. Read destinations are
revalidated after the receiver resumes with both its page table and kernel stack
installed. A prepared receive buffer cannot be freed before delivery commits.
Signals cancel the suspended read with EINTR; group termination cancels retained
work. Invalid user copies release the reserved receive slot and fail the caller.
Buffer identity is separate from its reusable offset, preventing an old failed
transaction from removing a newer allocation at the same address. Freed slots
are zeroed.

Strong/weak references and death cookies retain the original context-manager
identity. A replacement manager does not retarget a stale reference. Delivered
death notifications require acknowledgment; clearing waits for that acknowledgment.
Service close or receiver-thread exit wakes outstanding synchronous callers.
Closing a service and unmapping its pool are distinct failure paths: unmapping
first reports receive failure, while closing first reports peer death.

There are bounds of 16 connections, 32 threads per connection, 128 pending
transactions, 128 receive buffers per connection and 128 events per queue.
Allocations are fallible and use the existing heap/physical-page budgets. The
driver rejects unsupported transaction flags, Binder object arrays, FD/SG
transfer and unknown ioctls. There is no successful placeholder service reply.
Exported objects/nonzero handles, nested call stacks, priority inheritance,
Binderfs, SELinux security contexts, ServiceManager and native permission bridges
are not implemented. The forward plan remains in [TODO](../../../TODO.md).

## Boot and scheduler corrections

A resumed ioctl cannot perform its user copy inside scheduler selection: that
point has the next TTBR but still uses the previous kernel stack. Completion now
runs from the return hook after the assembly stack switch, before signal delivery.

The larger Framework build also exposed boot-stack corruption from copying the
512-entry root catalog through parser return values before heap initialization.
The parser now fills static catalog storage directly, clears it on failure, and
publishes readiness only after validation and reserved entries are complete.
Neither the 128 KiB boot stack nor the resource limits were increased. A host
test verifies failed parsing leaves no partial namespace and permits safe reuse.

## Native acceptance

Use the unchanged local ART receipt described in the
[fixture guide](../../../fixtures/art-runtime/README.md):

```sh
BNDROID_ART_MODE=binder scripts/check-art-runtime.sh
```

The script builds the owned NDK C probe and matching `binder-runtime` kernel/init
features, launches two concurrent peers, and uses a temporary disk overlay. No
upstream binary or generated runtime image is added to Git. ART modes share their
build directory and must run sequentially.

Final evidence: `.bndroid/art-runtime/check.OfR9ow`.

- Both peers exit zero and are reaped; kernel sender/receiver IDs match the two
  distinct process exit records.
- Eight submitted transactions, three replies, three ordered one-way messages,
  five expected failures and one death notification are recorded.
- An 8,201-byte request/reply spans pages. Malformed commands, a kernel-address
  source and a bad receive destination fail. A service closes with a synchronous
  call still outstanding, then its stale handle continues to report death.
- A blocking read survives sibling descriptor close/reuse; another returns EINTR
  after a real signal. Two actual writes to the read-only pools fault and return.
- All three signal handlers return, both worker threads are reaped, and no
  connections, transactions, buffers, waits, pages, tables or file descriptions
  remain. The disk is unchanged.
- The run records 1,188 Linux calls, a 2,236-page peak and 3,040 reclaimed pages.
  `binder_transport_verified=true`; `full_android_apps=false`.

The evidence parser rejects missing/duplicate roles, mismatched process identities,
insufficient counters, malformed records and nonzero cleanup. Ten protocol model
tests cover ownership, thread authority, one-way lifetime/order, stale death
references, reply failure, pool invalidation/exhaustion and buffer-offset reuse.

## Framework result and regressions

The source receipt remains 349 files / 265,085,778 bytes, 281 native dependency
records and 46 boot jars. The earlier ART subset and original linker namespaces
are unchanged; acquisition details remain in
[Framework JNI-0](FRAMEWORK_JNI_0.md).

The final full run `.bndroid/art-runtime/check.iyxIQ4` and the diagnostic traced
run `check.Qvb2mK` emit the authenticated `FRAMEWORK_STAGE parcel` after actual
Parcel/Bundle round trips. Each opens/maps
Binder and starts the upstream pool thread. Looper's native constructor then
aborts with `Could not make wake event fd: Function not implemented`.
Each result remains `guest-failure`, `framework_parcel_looper_verified=false` and
`full_android_apps=false`. The failed child and all seven workers are reclaimed;
Binder resources, pending signals, waits, pages, tables and files return to zero.
The disks are unchanged. These runs contain no cross-process Framework Binder
transaction or Looper callback; those are not inferred from the native probe.

Fresh regressions pass with unchanged temporary disks:

| Gate | Evidence |
| --- | --- |
| Shared code-cache/memfd | `.bndroid/art-runtime/check.nzYVLy` |
| Boot properties | `.bndroid/art-runtime/check.v5pipZ` |
| Memory/relocation | `.bndroid/art-runtime/check.F7n5Ba` |
| Managed APK/GC/JNI | `.bndroid/art-runtime/check.oW5V16` |
| Signal handlers | `.bndroid/art-runtime/check.vWT1Wq` |
| Bad signal frame / PC / FP | `.bndroid/art-runtime/check.jPsoep`, `check.h0feo3`, `check.o0fMgn` |
| Signal guard / group kill | `.bndroid/art-runtime/check.6BdTjg`, `check.y4PkHp` |
| Framework frontend | `.bndroid/art-runtime/check.7bRHse` |
| Dynamic Bionic | `.bndroid/bionic-dynamic/check.rYfqCs` |
| pthread normal / leader exit | `.bndroid/bionic-dynamic/check.FBj1TQ`, `check.4cf6Hs` |
| pthread group exit / worker fault | `.bndroid/bionic-dynamic/check.cSvxUZ`, `check.RwNoXe` |
| File EOF fault | `.bndroid/bionic-dynamic/check.jALC8P` |

Native boot and mobile UI (`target/mobile-ui/check-overview.YamOhY`) pass. The
existing interactive temporary-disk tour remains `target/mobile-ui/demo.e5WbNw`.
Host validation passes 1,139 workspace tests, 407 Framework library tests and
33 Python checks. Host-library Clippy, AArch64 Framework Clippy with tracing on/off
and traced Binder Clippy pass with warnings denied. Formatting, shell syntax and
diff whitespace checks pass. Generated evidence stays local and ignored.
