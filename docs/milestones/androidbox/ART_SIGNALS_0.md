# ART Signals-0: real signals and first managed Java execution

Date: November 6, 2026. The isolated ABI 70 profile now runs the owned standard DEX program on unmodified Android 16 ART. ActivityThread, Framework and general APK acceptance remain incomplete.

## Observed behavior

`scripts/check-art-runtime.sh` completes two cold interpreter launches of the installed image's `dalvikvm64`, shared Bionic, ART and core Java jars. Both run the unchanged `Main.java`, verify arithmetic, collections, strings and a caught Java exception, print `ART_JAVA_CORE_OK sum=4950 exceptions=1 collections=1`, and exit with status zero. The supervisor reaps both process generations and all ten runtime threads, leaving zero owned sparse pages, page tables, file descriptions or waiters. The temporary disk base is unchanged.

The recorded Java run is `target/art-runtime/check.X9Dk4V`: 45,988 Linux calls, 64,085 peak sparse pages, 183,086 reclaimed pages, and `managed_java_verified=true`. The generic native cleanup record still carries `art=0`: it does not inspect Java execution. The separate host evidence parser requires both actual Java output lines and complete cleanup before asserting managed execution. `full_android_apps` remains false.

This advances [ART Process-0](ART_PROCESS_0.md). The earlier `rt_sigaction` initialization failure, Signal Catcher futex-requeue failure, and missing realtime-clock error are resolved. The runtime reaches and starts its Java daemons and executes the application entry point. The program is a command-line DEX fixture, not an APK or an Android Activity.

## Signal semantics

- Process-owned dispositions and pending queues, per-thread masks and alternate stacks, `kill`/`tkill`/`tgkill`, queued signals, pending queries, suspend, timed/untimed signal waits, and AArch64 `rt_sigreturn` are implemented in the ART profile. Standard signals coalesce; realtime signals preserve values/FIFO order within a signal number. Queue capacity is 32 per thread/group, with a separate SIGKILL slot so queue saturation cannot prevent group termination.
- Delivery writes the actual AArch64 siginfo/ucontext/FPSIMD frame into checked user memory. A supplied restorer or a private RX syscall trampoline returns through kernel validation. GP/SIMD registers, FPCR/FPSR, stack, masks and user condition flags are restored. Frames cannot select EL1, alter privileged return controls, jump to kernel addresses, or read/write another owner's memory. Oversized frame copies use bounded recoverable usercopy chunks.
- `SA_SIGINFO`, alternate stacks, nodefer nesting, reset-on-delivery, auto-disarmed alternate stacks and restart behavior are exercised. A futex interrupted with `SA_RESTART` retains its original deadline; handler-side waits cannot consume that saved restart. Restart records are bounded to 16 outstanding interrupted frames per thread. Nanosleep reports EINTR with remaining time; signal waits consume matching blocked signals without running a handler.
- Recoverable EL0 memory faults reach SIGSEGV/SIGBUS handlers; illegal instructions and breakpoints have their corresponding signal classifications. File pages beyond EOF remain SIGBUS rather than zero-filled success. User VMA ownership determines MAPERR versus ACCERR, including privileged identity mappings. Fatal/default signals terminate and reap the entire owned group.
- Return assembly switches to the scheduler-selected context's kernel stack before delivery performs user copies. Large logging buffers are outlined so console `writev` does not stack two 4 KiB log packets. Existing 16 KiB thread kernel stacks, canaries and nested-exception headroom checks remain enforced; bounded frame-pointer diagnostics explain failures.

This is not full POSIX/Linux signal compatibility: foreign-process signaling, sessions/job control, ptrace, signal fds, timers, core dumps, SVE/SME and priority-inheritance futexes remain absent. Default stop requests return ENOSYS rather than pretending to stop the group. The profile remains single CPU.

## Synchronization, time and bounds

`FUTEX_REQUEUE` and `FUTEX_CMP_REQUEUE` wake a requested number of waiters and migrate remaining waiters without losing owner generation, deadline or bitset. Invalid counts/addresses and failed comparisons leave the queues unchanged. All operations are serialized with IRQs masked on the profile's single CPU.

`CLOCK_REALTIME` and its coarse form sample QEMU virt's read-only PL031 Unix timestamp, then advance from the architectural counter. Initial accuracy is limited by the RTC's one-second resolution. Realtime absolute futex waits and `clock_nanosleep` use the same anchor; time is not fabricated from boot uptime. Clock setters, host clock changes after sampling, secure time and physical RTC claims are outside this profile.

ART's region limit is now 1,024 after a real daemon-startup failure exhausted 512 records. The 256 MiB sparse-page bound, 256 table bound, 8 MiB kernel heap and 512 MiB guest remain unchanged. A host test verifies exhaustion preserves mappings and capacity becomes available after unmap. The measured Java run is close to the sparse-page limit; it does not establish a general application memory envelope.

The ART monitor allows 90 guest seconds for both cold launches; other native profiles retain 30 seconds. The host runner retains its finite 120-second default timeout. Per-syscall tracing is opt-in because it materially slows this workload and can interleave console output; failed runs still retain errors and evidence without it.

## Validation and reproduction

Use the local-only [fixture guide](../../../fixtures/art-runtime/README.md). No SDK binary is modified or committed; the frozen 66-input runtime set and namespace metadata from Process-0 are unchanged.

```sh
scripts/check-art-runtime.sh
scripts/check-art-signals.sh
BNDROID_ART_MODE=memory scripts/check-art-runtime.sh
BNDROID_ART_MODE=help scripts/check-art-runtime.sh
```

The positive signal fixture checks real handlers, masks, alternate-stack execution, full SIMD preservation, synchronous faults, queued values, interrupted/restarted waits, pthread signaling, requeue operations, realtime absolute deadlines and console writev. Both generations must exit normally and release all resources. Five negative modes deliberately corrupt return privilege, PC or FPSIMD data, protect the alternate stack, or issue SIGKILL while a sibling waits and the realtime queue is full. A negative mode passes only for its expected group fault and complete cleanup; a random panic, wrong fault owner or leaked resource fails it.

The final positive receipt is `check.fNhwQ7`: 30 handlers delivered/returned, eight interrupted waits and 24 test threads reaped. Negative receipts are `check.TnENCU`, `check.j8F1gm`, `check.CWH77e`, `check.FAkJPc` and `check.huEHr0` in the mode order above. Memory (`check.TeE6fc`) and help (`check.mZ21I8`) also pass. All are under ignored `target/art-runtime/`.

Host validation passes 1,126 default workspace tests, 392 ART kernel-library tests, eight ART evidence-parser checks and four existing UART checks. Host and AArch64 ART Clippy pass with warnings denied, including the tracing build. The parser rejects missing/forged/duplicate/incomplete proof lines, wrong process identities and resource leaks. See the implementation status for the final QEMU regression receipts.

## Remaining work

The default command still records missing tracing/proc-cmdline/timezone data and unsupported optional affinity, niceness and gettimeofday calls. These warnings are retained in `summary.json`; they are not hidden by success. Writable runtime files, discard/demand-paging behavior, richer GC/JNI/thread stress, accurate runtime properties and timezone data, Binder, Framework/service bridges, APK loading and ActivityThread remain necessary. A1 and broader Android acceptance gates stay unchecked.
