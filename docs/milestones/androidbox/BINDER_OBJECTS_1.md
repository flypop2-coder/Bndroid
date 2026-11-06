# Binder Objects-1: handles, references and nested callbacks

Date: November 6, 2026.

This extends [Binder Transport-0](BINDER_TRANSPORT_0.md) and the device-domain /
close-flush behavior in [Framework Looper-0](FRAMEWORK_LOOPER_0.md). The custom
kernel now translates strong and weak Binder objects, routes nonzero handles,
and returns synchronous callbacks to the original waiting thread. It remains a
bounded Binder implementation, not complete Android service or APK support.

## Object ownership

Each exported object belongs to one open Binder connection and has its own node
identity, opaque pointer and cookie. Recipient handles are connection-scoped;
guessing another connection's handle does not grant authority. Handle numbers
are monotonic, and live/dead nodes retain their generation across process exit.
Framework, hardware and vendor Binder domains retain separate context managers.

Flat strong/weak Binder objects become recipient handles. Forwarding a handle
creates a reference in the next recipient; returning it to its original owner
restores the original pointer and cookie. Remote-object cookies and union padding
are cleared. Object-offset arrays occupy aligned space after the payload in the
read-only receive pool. The driver checks ordered, nonoverlapping, four-byte
aligned offsets and complete 24-byte objects before using each entry. A later
invalid object rolls back all earlier holds from the same failed transfer.

Buffer holds remain distinct from explicit userspace strong/weak references.
Freeing or cancelling a buffer releases only its own holds. Queued requests,
prepared reads, replies, unmapping, thread exit and connection close all release
their associated ownership. A delivered request can free its receive buffer
before replying without losing the synchronous call identity.

Owners receive BR_INCREFS/BR_ACQUIRE and acknowledge the matching pointer/cookie
with BC_INCREFS_DONE/BC_ACQUIRE_DONE. These retains precede transaction completion,
so an asynchronous sender can safely release its Parcel after completion. A
retain reserved by another resumed thread prevents completion until userspace
acknowledges the retain; kernel delivery alone is insufficient. BR_RELEASE/BR_DECREFS follow the last relevant hold. An isolated
weak handle cannot resurrect an object after its strong references are gone.

Reference and death notifications use prepare/commit/cancel delivery. An invalid
read destination does not lose a reference transition or death cookie. Death
subscriptions are per handle, with distinct outstanding cookies within a
connection and separate acknowledgment/clear completion;
retained handles to dead nodes never resolve to a replacement service.

## Call routing

The synchronous call stack records direction, thread and parent transaction.
A callback into an ancestor process is queued to that ancestor's waiting thread.
Replies must match the top incoming frame and the caller's corresponding outgoing
frame. Nested replies restore the previous frame; failure unwinds descendants
before parents without allocating memory during cancellation.

One-way work is serialized per target node until its prior receive buffer is
freed. Independent nodes can make progress while another node's buffer is held.
Accepted one-way work survives sender exit. Reference handshakes still precede
the sender's completion, and target-buffer holds keep receiver objects alive.

The LP64 transaction target is a union. BC_TRANSACTION uses its 32-bit handle,
while BR_TRANSACTION delivers a 64-bit owner pointer. The upper four input bytes
of the handle union are ignored as padding, matching the UAPI. The native fixture
deliberately fills that padding with nonzero bytes.

## Bounds and remaining work

The existing 16 connections, 32 threads per connection, 128 pending transactions,
128 buffers per connection, 128 queued events and 4 MiB receive-pool limits stay
in place. Object limits are 512 live nodes, 256 handles per connection, 4,096
references overall, 256 objects per transaction and 16 call frames per thread.
Ancestor lookup is bounded to 16 frames. Allocations are fallible within the
existing 8 MiB kernel heap; no RAM, stack or timeout increase is included.

FD/FD-array ownership, scatter/gather buffers, security-context transactions,
scheduler/RT priority inheritance, frozen-process extensions, Binder readiness
through epoll, per-app credentials, SELinux/capability policy and real Android
ServiceManager integration remain open. FD-acceptance metadata can be retained,
but actual FD objects fail explicitly. Weak-reference promotion requiring an
owner-side attempt-acquire protocol is not implemented. This is not an A1/A2 or
ActivityThread acceptance claim; `full_android_apps` remains false.

The ABI and ownership reference is the primary
[Linux Binder UAPI](https://raw.githubusercontent.com/torvalds/linux/v6.6/include/uapi/linux/android/binder.h)
and [Binder driver](https://raw.githubusercontent.com/torvalds/linux/v6.6/drivers/android/binder.c).
The repository implementation is original Rust; no upstream source or runtime
binaries were copied into the repository for this change.

## Verification

The native gate remains:

```sh
BNDROID_ART_MODE=binder scripts/check-art-runtime.sh
```

In addition to the byte-transport cases, two real Bionic processes export strong
and weak objects, retain translated handles, pass objects back to their owner,
invoke nonzero targets, perform two nested callbacks on their original threads,
exercise independent one-way node queues, reject malformed/FD/forged objects,
observe three separate deaths and exit normally. The verifier requires both
process markers, actual kernel object/reference/nesting counters, authenticated
identities and zero live nodes, references, call frames, transactions and buffers.
Java Framework, managed APK/GC/JNI and eventfd/epoll regressions share a sequential
runtime-validation batch. Their acquired runtime inputs remain immutable/local.

The combined validation batch passes 1,158 workspace library tests, 427 Framework
library tests and 34 Python verifier/input/demo checks. Host library Clippy runs
with warnings denied. Host cases cover forwarding through a third connection,
strong/weak authority, handle-table exhaustion, maximum nested depth, descendant
failure, failed-copy retries and prepared/acknowledged reference ordering.

The initial native object runs `.bndroid/art-runtime/check.21b3WL`, `check.Rjp1gE`
and `check.eKwQPF` failed when the client received BR_FAILED_REPLY for its first
object call; the other peer then reached the existing runtime deadline. These
are failed diagnostics, not cleanup or acceptance proof. Code inspection found
that the LP64 input target's upper union padding was being validated as part of
the handle. The final fixture explicitly poisons these bytes, and the corrected
32-bit handle routing passes.

Final native evidence `.bndroid/art-runtime/check.x1QDbZ` includes the tightened
acknowledgment ordering. It records 16 submitted transactions, eight replies, six
one-way calls, seven translated objects, ten reference events, two nested callbacks
and maximum thread-stack depth three. Ten expected negative cases and three death
notifications complete; all five connections, nodes, references, frames and
buffers are reclaimed. Both processes exit zero, all workers/signals are reclaimed
and the disk is unchanged. It executes 1,303 Linux calls, peaks at 2,240 anonymous
pages and reclaims 3,046. Earlier success `check.NuZilv` remains retained.

The full Framework regression `.bndroid/art-runtime/check.b7VdzY` completes both
Parcel/Bundle/cross-thread wake/delayed-callback sequences and normal VM shutdown.
Both processes and all 16 workers exit/reap; all event, Binder, mapping and file
resources return to zero. The unchanged disk receipt records 163,398 Linux calls,
102,853 peak anonymous pages and 335,956 reclaimed pages. Its two eventfds and two
epoll instances produce real wakeups and both Binder pools return through close
flush. `framework_parcel_looper_verified=true`; `full_android_apps=false`.

An earlier combined Framework run, `check.U6aU69`, completed its first generation
normally but hit the existing deadline during the second startup (125,070 Linux
calls). It is retained as a failure without full cleanup proof. The successful
repeat temporarily paused only the owned interactive preview, then restored it
at batch completion. Host load is a possible timing factor, not an established
performance result. The guest/host deadlines and memory limits were unchanged;
the successful full-Framework run took approximately 306 host seconds.

The same final batch passes managed APK/GC/JNI (`check.ETQcxp`, 61,505 Linux
calls) and native eventfd/epoll (`check.4meoGE`, 1,838 Linux calls). Both have two
zero-status exits, complete resource reclamation and unchanged temporary disks.
Host library Clippy and complete AArch64 Framework binary Clippy pass with warnings
denied, with syscall tracing both disabled and enabled. Formatting, diff checks
and documentation links pass. No additional unrelated runtime matrix was rerun.

The temporary-disk interactive demo was also launched successfully at
`target/mobile-ui/demo.5oqar9`: the guided tour retained 15 screenshots. This UI
preview is the bounded mobile/AndroidBox demo, separate from Framework CLI gates.
