# Bionic Threads-0

Recorded November 6, 2026. The existing `bionic-dynamic-runtime` profile now runs unmodified Bionic pthread creation, join, detach, TLS, normal mutexes/condition variables/barriers, and thread-specific destructors. This removes the earlier one-thread-per-process limitation. It does not establish ART, signal-handler delivery, or general APK compatibility.

## Ownership and scheduling

Linux threads share their process's private address space and file table. Each has a separate owned kernel exception stack, saved registers/FP state, `TPIDR_EL0`, monotonic TID, clear-child-TID pointer, signal mask, and name. Native process identity and generation remain the authority for memory and descriptors; a Linux thread does not gain a new native process or native handles.

The clone path accepts shared-thread-group flags, validates stack and TID destinations before publication, prepares the kernel stack, copies the saved EL0 state, installs the child TLS value and zero child return value, and only then makes the child runnable. The actual [upstream Bionic pthread implementation](https://android.googlesource.com/platform/bionic.git/+/master/libc/bionic/pthread_create.cpp) supplies the trampoline, user stack, TLS allocation, and pthread bookkeeping; Bndroid does not replace them.

The dynamic profile provides at most 32 additional thread contexts globally and a 2 MiB kernel heap. Available heap, frame, and VMA quotas can impose lower limits. Native/default/static profiles retain their earlier scheduler and heap bounds. Scheduling is single-core and preemptive; SMP is not implemented.

Linux return validation checks owned executable pages and aligned bounded EL0 SP values. It permits a temporarily unmapped user stack because Bionic detached-thread teardown unmaps that stack immediately before `exit`. The saved exception frame and kernel stack remain independently validated, and any actual illegal EL0 access is still hardware-fault contained.

## Synchronization and lifetime

- Futex wait/wake and bitset operations use a generation-qualified address-space owner and virtual address. Value checking and queue publication share one IRQ-masked transaction, avoiding a compare-to-sleep lost-wakeup window on the current single CPU. Wake counts, cancellation, and expiration remove exact queued slots before making contexts runnable.
- Relative waits and absolute `CLOCK_MONOTONIC` waits use checked counter conversion rounded upward. `nanosleep`, monotonic `clock_nanosleep`, and `sched_yield` really deschedule the caller. Unsupported realtime/PI/requeue operations return errors.
- `rt_sigprocmask` stores and queries per-thread masks, excludes SIGKILL/SIGSTOP, and copies the parent mask into new threads. `PR_SET_NAME`/`PR_GET_NAME` maintain thread names. This is mask/name state, not a claim of signal delivery or signal-based cancellation.
- A normal thread exit clears its registered TID word and wakes matching join waiters. Kernel stacks are reclaimed only after the context has stopped. A main thread can exit while workers remain alive; the final worker completes the native process lifetime.
- `exit_group` from any worker retires the leader and siblings. A worker fault terminates the group and cancels all futex/sleep waits before the shared address space or descriptors can be destroyed. Native supervisor termination is also routed through group retirement.

There is no process fork, exec replacement, process-shared mapping synchronization, realtime-clock futex deadline, priority inheritance, robust-futex recovery, signal-handler delivery, pthread cancellation, or SMP guarantee. Remaining syscall gaps in the tested shared runtime are sched_getaffinity, sigaltstack, rt_sigaction, gettimeofday, and socket.

## Reproduction and evidence

Use the same local runtime inputs documented in [the dynamic fixture guide](../../../fixtures/bionic-dynamic/README.md), then run:

```sh
./scripts/check-bionic-threads.sh
```

The runner performs four independent temporary-disk boots:

1. **Normal:** each of two process generations runs four concurrent joinable workers and eight detached workers. It checks distinct TIDs/shared PID, initialized executable and plugin TLS across timer preemption, isolated errno, shared fd offsets, contended mutexes, barriers, joins, destructors, inherited/private masks, names, timed condition waits, and invalid futex/clone inputs. The combined 24 kernel thread stacks and every wait are reclaimed.
2. **Leader exits first:** `pthread_exit` removes the main thread while a worker continues and completes the process. Both supervised generations exit normally.
3. **Worker exits the group:** a worker calls `exit_group` while the leader sleeps and another worker is blocked in futex wait. Both generations retire all contexts and release the process.
4. **Worker faults:** a worker attempts to read a privileged kernel address while its siblings wait. The gate verifies the exact fault address and EL0 data-abort class, then observes one reaped process, two reclaimed worker stacks, no pending waits, and zero live sparse pages/tables/file descriptions. Init survives; its expected failed-child diagnostic is not treated as an application success.

Each boot retains UART logs and a JSON summary in `target/bionic-dynamic/check.*/`. Fresh entropy and scheduling can change allocation and wait counts. The gates assert invariants rather than one accidental interleaving.

The default 1,117 workspace host tests, three synchronization-model tests, dynamic-profile Clippy with warnings denied, the four lifecycle gates, original dynamic/static/raw Linux checks, EOF containment, native boot/fault containment, and mobile UI acceptance passed. No upstream binaries or local test artifacts are committed. AndroidBox A1–A4 remain incomplete.
