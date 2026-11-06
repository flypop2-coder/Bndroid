# Binder Files-2: descriptor transfer and service event loops

Date: November 6, 2026.

This extends [Binder Objects-1](BINDER_OBJECTS_1.md) with flat file-descriptor
objects, cross-process memory-file ownership and Binder readiness through epoll.
It supplies service transport prerequisites; real Android ServiceManager,
ActivityThread and complete APK acceptance remain open.

## File ownership

The sender's descriptor is resolved and its open description retained before
publication. Closing/reusing the original number cannot change the in-flight
file. The destination receives a new descriptor with close-on-exec set, while
open-description offsets and status flags remain shared. A receive-buffer free
does not close a successfully installed descriptor.

Requests require the target node's FD-acceptance flag. Replies require the
original caller's TF_ACCEPT_FDS flag. Invalid descriptors, unsupported object
types and malformed offset arrays fail before publication. Flat objects are
validated against the captured descriptor list; FD union padding is cleared
before receiver descriptor numbers are written. FD cookies remain userspace data,
including Parcel ownership metadata; they are not treated as kernel pointers.

The receiver reserves capacity for the whole descriptor batch before installing
any member. User-copy failure rolls back only matching open descriptions, so a
replacement at a reused descriptor number cannot be closed accidentally.
Descriptor-table exhaustion produces BR_FAILED_REPLY for the caller. A failed
request is discarded and the receiving thread resumes its receive loop, including
safe resuspension on its own kernel stack. Close-flush state survives this retry.
Rejected replies complete the replier and report failure to the caller.

Binder device descriptors themselves are explicitly non-transferable in this
profile. Retaining a Binder file in its own transaction router would permit
cycles and recursive close under the router lock; no success is fabricated for
that case. Other existing file types retain their existing access semantics.
Scatter/gather and FD-array objects remain unsupported. The transaction's
existing 256-object bound also bounds flat FD transfers, and receiver descriptor
limits remain 64 for ART or 512 for Framework.

## Shared memory across processes

Transferred memfd objects retain their actual physical backing through both
descriptors and mappings. Resize first prepares all allocation, then acquires one
page-table guard and invalidates aliases beyond EOF in every address space.
Only after all affected ASIDs are invalidated may backing frames be freed. Every
mapping's EOF metadata is updated, including for growth caused by pwrite.
Regrowth faults repopulate aliases from the same newly zeroed backing.

F_SEAL_WRITE checks writable-capable shared mappings in every process, including
read-only mappings that could later regain write access. Seal changes are visible
through all transferred references. These operations run in the existing
single-CPU, IRQ-masked runtime domain; no SMP or private-memory-file COW claim is
made. Per-mapping write/execute restrictions remain unchanged.

## Binder epoll integration

A Binder open file owns an external poll source. Watches retain only source
identities; they do not keep a Binder device alive. Last close removes watches,
and a replacement descriptor does not inherit them.

Readiness is computed for the original waiting process and thread. Own replies,
nested callbacks, reference transitions and close returns are distinct from
process work, which requires an idle entered/registered looper. Snapshot queries
do not consume Binder messages. Foreign/private connections return an error mask.
External notifications propagate through nested epoll and honor edge and one-shot
registrations. Notifications are conservative and current readiness is rechecked
before delivery; consumers still drain nonblocking Binder reads normally.

The event layer uses a small immutable Binder snapshot outside its lock. It never
calls back into the Binder router while holding the event lock. This avoids lock
inversion when a captured eventfd/epoll file is released by Binder cleanup. Timer
wakes inspect state only; user copies and descriptor installation occur after the
receiver's page table and kernel stack are installed.

The ABI reference is the primary
[Linux Binder UAPI](https://raw.githubusercontent.com/torvalds/linux/v6.6/include/uapi/linux/android/binder.h)
and [Binder driver](https://raw.githubusercontent.com/torvalds/linux/v6.6/drivers/android/binder.c).
The repository implementation is original Rust; acquired Android binaries remain
local and ignored.

## Combined verification

The native Binder mode now also transfers memory/event files between concurrent
Bionic processes, closes/reuses source numbers, checks shared offsets and
close-on-exec flags, truncates a foreign mapping into a caught SIGBUS, regrows it,
checks foreign writable-map seals and retains a file after its creator exits.
Both receiver-table exhaustion directions and failed output copies must reclaim
all pending transfers. Real nested epoll waits drive Binder reads, with a separate
thread checking that it cannot consume another thread's reply readiness.

The verifier requires separate process/FD markers, kernel transfer/install-failure
counts, actual external epoll events, normal exits and zero live descriptors,
pending transfers, Binder nodes/handles/frames, watches, pages and page tables.
The combined batch passes 1,165 workspace library tests, 434 Framework library
tests and 34 Python verifier/input/demo checks. Host library Clippy and complete
AArch64 Framework binary Clippy pass with warnings denied, including builds with
syscall tracing enabled and disabled. A host test also exhausts the replier's
completion queue: the allocation-free error slot still completes the replier
and unblocks the rejected caller.

Final native Binder evidence `.bndroid/art-runtime/check.ixHTrO` records 25
transactions, 14 replies, six one-way calls, eight translated Binder objects,
14 reference events, two nested callbacks and three death notifications.
Eleven FD submissions include five delivered descriptors and two receiver-table
installation failures. Sixteen expected negative cases complete. All five
connections, pending transfers, nodes, references, buffers and call frames are
reclaimed. Fifteen external Binder events traverse the nested epoll path; all ten
event objects and all watches close. Both processes exit zero, all three workers
are reaped and all four delivered signals return. The run executes 1,747 Linux
calls, peaks at 2,312 anonymous pages and reclaims 3,325, with no live pages,
page tables or file descriptions. The disk is unchanged. The earlier positive
run `check.gVgzep` is retained; the final run includes the reply-queue fallback.

The same batch passes shared memfd/generated-code/fault handling
(`check.07gKAt`, 1,170 Linux calls), managed APK/GC/JNI (`check.djAc0O`, 61,503
calls) and eventfd/epoll (`check.KEh89r`, 1,838 calls). Each has two zero-status
process exits, complete resource reclamation and an unchanged temporary disk.

The first full Framework run, `check.HbAayw`, completes the first generation's
Parcel/Looper sequence and normal VM shutdown, then reaches Parcel in the second
generation before the existing total deadline (159,071 Linux calls). It remains
a failed diagnostic without complete cleanup proof. The owned interactive demo
was paused during this run and restored afterward; guest/host deadlines, RAM,
heap and stack bounds were unchanged.

The final Framework repeat `check.mY1RXD` passes under the same limits: both
Parcel/Bundle/cross-thread wake/delayed-callback sequences finish, both VMs shut
down normally and both processes exit zero. All 16 workers, four Binder
connections, eight event objects, watches, pages, page tables and file descriptions
are reclaimed. It records 163,446 Linux calls, a 102,853-page peak and 335,999
reclaimed pages. The disk is unchanged and `framework_parcel_looper_verified` is
true. The complete build-and-run command took 314.1 host seconds. This repeat does
not establish startup performance margin or a cause for the earlier deadline;
both results remain recorded. The temporary-disk interactive demo is restored to
running after validation.

Further work includes SG/FD-array ownership, Binder-device transfer lifecycle,
native permission/credential and security-context policy, broader poll/device
coverage, real service registration/lookup and Android activity/graphics bridges.
`full_android_apps` remains false and A1–A6 remain incomplete.
