# Framework Looper-0: event notification and shutdown

Date: November 6, 2026.

The ART profile now supplies eventfd counters and epoll readiness through actual
kernel objects. The original Android Framework executes the owned probe's
Parcel/Bundle checks, a message posted by a different Java thread and a delayed
Looper callback. The complete process acceptance result and remaining boundaries
are recorded below; these operations do not establish ActivityThread or general
Android application compatibility.

## Event notification

Eventfd supports counter accumulation, semaphore reads, overflow blocking,
nonblocking errors, close-on-exec flags and shared status flags across duplicates.
Read/write and readv/writev transfer one native 64-bit value, including split
iovecs. Blocking calls retain the original open file across close and descriptor
reuse. A signal can interrupt a read/write, or restart it when SA_RESTART applies.
As on Linux, an eventfd read that consumes the counter and then faults while
copying out reports EFAULT; the counter is not silently restored.

Epoll implements add/modify/delete, level and edge notification, one-shot rearming,
bounded nested interest lists, cycle rejection and round-robin delivery when the
output array is smaller than the ready set. Registrations bind both descriptor
number and open-file identity. A replacement fd does not inherit an old watch;
last release removes its registrations. Watches do not retain targets or create
reference cycles. An output-copy failure leaves undelivered events available.

`epoll_pwait` and `epoll_pwait2` use monotonic deadlines and atomic temporary signal
masks. The original mask is restored on completion and signal return. Epoll waits
return EINTR rather than restarting after a caught signal. A competing reader
can drain readiness before a waiter resumes; the resumed call rechecks and sleeps
again. Timer wakeups never copy foreign user memory. All consumption and copies
run after the resumed process's page table and kernel stack are installed.

The native gate is:

```sh
BNDROID_ART_MODE=poll scripts/check-art-runtime.sh
```

It exercises counter limits, semaphore contention, vectored I/O, notification
modes, nested epoll, timeout accuracy, fd reuse during blocked eventfd and epoll
calls, invalid buffers, unmapping a waiting reader's destination, signal restart,
temporary signal masks and process exit with a blocked sibling. The verifier
requires two successful processes, all workers reclaimed, actual wait/wake/timeout
counters and zero live event objects or watches.

Bounds are 256 event objects, 256 watches per epoll, 4,096 total watches and five
epoll levels. Allocations remain fallible within the existing kernel heap. Epoll
currently accepts eventfd and other epoll objects; Binder, socket/pipe/device
readiness, poll/select, timerfd, exclusive wake policy and suspend-prevention
capabilities remain unimplemented. Unsupported operations fail explicitly. No
physical memory, heap, stack or guest RAM increase is part of the event driver.

## Binder close and device domains

`/dev/binder`, `/dev/hwbinder` and `/dev/vndbinder` now have independent context
managers and independently pinned manager UIDs. They share the tested byte
transport and resource accounting, without sharing handle-zero authority.
Opening a hardware Binder device does not implement a HAL or hardware service.

Descriptor close now flushes Binder threads independently of final open-file
release. An idle read returns BR_NOOP, allowing upstream libbinder to observe its
closed driver and end the pool thread. This also applies when a dup replacement
or process-table destruction closes a descriptor. Duplicates/in-flight calls
still retain the connection until their last reference is released. A native test
closes/reuses a descriptor during a request, observes the flush return through
the retained call, and receives the actual reply through a surviving duplicate.
Queued transactions and death references remain intact across a flush.

This corrects the missing close-flush behavior in
[Binder Transport-0](BINDER_TRANSPORT_0.md). The original `app_process` stops both
regular and hardware Binder after returning from the Java entry. Its close path
must wake the attached pool thread before VM destruction can finish. The source
behavior is visible in [app_process](https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/android16-release/cmds/app_process/app_main.cpp)
and [Linux Binder's flush/release hooks](https://raw.githubusercontent.com/torvalds/linux/v6.6/drivers/android/binder.c).

## Process map snapshots

After the pool thread could detach, VM teardown reached Bionic's process-map
scan. The earlier 128 KiB text limit rejected the complete snapshot: the
diagnostic run recorded exactly 131,072 encoded bytes, 2,063 sparse regions,
526 native pages and 4,507,024 free heap bytes. The reported ENOMEM came from the
snapshot bound while the heap still had capacity.

Framework's snapshot text bound is now 512 KiB, proportional to its existing
fourfold VMA allowance; the ART-only snapshot bound stays 128 KiB. Allocation
is still fallible within the same heap. No rows are truncated or fabricated.
Host checks cover a complete 2,063-row snapshot above the old limit and rejection
beyond the selected bound. Device paths are preserved in maps without adding a
second leading slash. Failure diagnostics distinguish row allocation from text
formatting and report the retained bound and heap availability.

## Framework verification

Run the full probe with the unchanged local Framework receipt:

```sh
BNDROID_ART_MODE=framework scripts/check-art-runtime.sh
```

The Java probe uses an idle-handler/latch handshake before another thread posts
to the sleeping Looper, then verifies callback ownership and a delayed message.
The verifier requires authenticated Parcel, wake and Looper stages from both
process generations, actual eventfd reads/writes and epoll blocking/wakeup,
Binder close-flush returns, two zero exits and full cleanup. A Java marker alone
cannot satisfy the gate. Normal Java return and VM destruction are part of
acceptance.

The runtime receipt remains 349 files / 265,085,778 bytes and 46 boot jars, with
unchanged hashes for the 66 earlier ART inputs. Original ART, Framework, Bionic
and linker namespace policy remain in use. Acquired binaries, temporary disks,
compiled probes and raw evidence remain local and ignored by Git.

ActivityThread, exported Binder objects/FDs, nested Binder calls, ServiceManager,
native capability/permission bridges, app installation/data paths and Framework
graphics remain in [the roadmap](../../../TODO.md). The mobile UI demo is still
the bounded AndroidBox/UI profile; it does not display this command-line
Framework probe. `full_android_apps` remains false and A1–A6 remain incomplete.

## Recorded results

The final Framework run, `.bndroid/art-runtime/check.qpDz2A`, passes with both
zero-status exits, all 16 workers reaped and no live pages, page tables, file
descriptions, waits, event objects or Binder resources. It records 163,452 Linux
calls, a 102,855-page physical peak and 335,965 reclaimed pages. The two eventfds
and two epolls are all closed; two eventfd reads, four writes, four blocked waits,
two readiness wakeups and two timeouts establish actual event-loop activity.
Both regular Binder pool reads return after close flush. Framework and hardware
Binder account for four opened/closed connections, with no live transactions or
buffers. Complete map snapshots contain 132,147 and 132,315 bytes. The temporary
disk is unchanged and `framework_parcel_looper_verified=true`.

Native event evidence `.bndroid/art-runtime/check.UZ3JIl` passes in two generations
with all 20 workers reaped. It records 28 eventfds, 14 epolls, 42 closes, 34 reads,
42 writes, 26 delivered notifications, 30 blocked waits, 16 readiness wakes,
four expirations and ten cancellations. All eight signal handlers return;
pending signals, event objects, watches and process resources are zero. The
1,838-call run has a 1,490-page peak, 7,810 reclaimed pages and an unchanged disk.

The updated Binder gate `.bndroid/art-runtime/check.sGBxui` passes with independent
device-manager registration, a retained ioctl's close-flush return, continued
reply delivery through the surviving duplicate and the existing transaction,
one-way, death and fault checks. Five connections open/close, eight transactions
are submitted, three reply, three are one-way, five fail as expected, one death
notification is delivered and one blocked read returns after flush. All resources
are reclaimed and the disk is unchanged.

Earlier development runs remain failures: `check.1iNvoy` completed the original
single-thread callback but aborted at missing hwbinder; `check.qkMs11` completed
the stronger wake/callback probe but timed out at VM shutdown before close-flush
support, without a cleanup proof. `check.SBRnE1` and diagnostic `check.KfBol4`
aborted at the old proc-map bound and did fully reclaim the failed child. Their
Java markers were never treated as complete acceptance.

Fresh regression evidence, all with unchanged temporary disks:

| Gate | Evidence |
| --- | --- |
| Shared memfd/code cache | `.bndroid/art-runtime/check.6KxDoK` |
| Boot properties | `.bndroid/art-runtime/check.FcFXal` |
| Memory/relocation | `.bndroid/art-runtime/check.3PM5um` |
| Managed APK/GC/JNI | `.bndroid/art-runtime/check.uXOEnZ` |
| Signal handlers | `.bndroid/art-runtime/check.p0qpeh` |
| Bad frame / PC / FP | `.bndroid/art-runtime/check.TGzxpz`, `check.PcGEbh`, `check.NYcSPy` |
| Signal guard / group kill | `.bndroid/art-runtime/check.5RLR9q`, `check.OzKlq7` |
| Framework frontend | `.bndroid/art-runtime/check.Im1lwH` |
| Dynamic Bionic | `.bndroid/bionic-dynamic/check.YSAHEv` |
| pthread normal / leader exit | `.bndroid/bionic-dynamic/check.nMUFKh`, `check.NfDuoU` |
| pthread group exit / worker fault | `.bndroid/bionic-dynamic/check.FA1Cud`, `check.kDj6PD` |
| File EOF fault | `.bndroid/bionic-dynamic/check.JWjUwB` |

Native boot and mobile UI (`target/mobile-ui/check-overview.Q0SeWk`) pass. The
interactive temporary-disk demo remains `target/mobile-ui/demo.e5WbNw`. Validation
also passes 1,148 workspace host tests, 417 Framework library tests and 34 Python
checks. Six event models cover lifetime/readiness/fairness/nesting; twelve Binder
models include device-domain isolation and close-flush ownership. Host-library
Clippy and AArch64 Framework Clippy with tracing enabled/disabled pass with
warnings denied. Formatting, script syntax, Python compilation and diff whitespace
checks pass. These are bounded integration results, not Android conformance or
hardware performance measurements.
