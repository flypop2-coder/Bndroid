#!/usr/bin/env python3
"""Run one owned QEMU process; retain honest ART bring-up evidence on failure."""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

from linux_runtime_evidence import parse_marker


def complete_lines(text, line):
    return len(re.findall("^" + re.escape(line + "\n"), text, re.MULTILINE))


def completion_status(text, mode):
    try:
        proof = parse_marker(text, "BIONIC_DYNAMIC_RUNTIME_OK")
        threads = parse_marker(text, "LINUX_THREADS_OK")
    except ValueError:
        return "invalid-record"
    if proof is None:
        return None
    expected = {
        "memory": "ART_MEMORY_OK low_64m=1 protections=1 holes=1 shared_threads=1 kernel_isolation=1\n",
        "memfd": "ART_MEMFD_OK aliases=1 code=1 truncate=1 seals=1 lifetime=1 wx=1\n",
        "binder": "BINDER_CLIENT_OK identity=1 reply=1 oneway=3 death=1 interrupted=1 fd_reuse=1 readonly=1\n",
        "poll": "POLL_NATIVE_OK counter=1 semaphore=1 epoll=1 vectors=1 nested=1 lifetime=1 race=1 signals=1 timeout=1 pending_exit=1\n",
        "properties": "ART_PROPERTIES_OK bionic_reader=1 callback=1 enumerate=1 wait=1 immutable=1\n",
        "help": "dalvikvm: [options] class [argument ...]\n",
        "framework-help": "dalvikvm: [options] class [argument ...]\n",
        "framework": "FRAMEWORK_MAIN_OK parcel=1 bundle=1 looper=1 delayed=1 wake=1\n",
        "java": "ART_JAVA_CORE_OK sum=4950 exceptions=1 collections=1\n",
        "managed": "ART_MANAGED_OK java_threads=3 monitors=1 interrupt=1 gc=1 jni=1 native_thread=1 clocks=1\n",
        "signals": "ART_SIGNALS_OK handlers=1 altstack=1 registers=1 masks=1 realtime=1 faults=1 threads=1 restart=1 waits=1\n",
    }[mode]
    if mode == "framework-help":
        records = re.findall(r"^ANDROID_LOG pid=(\d+) buffer=0 priority=(\d+) tag=([^ ]+) message=(.*)\n", text, re.MULTILINE)
        successes = [pid for pid, priority, tag, message in records
                     if priority == "4" and tag == "vm-printf" and message.rstrip() == expected.rstrip()]
        if len(successes) != 2 or len(set(successes)) != 2:
            return "exited-without-expected-proof"
        for owner in successes:
            for level, message in (("3", ">>>>>> START com.android.internal.os.RuntimeInit uid 10000 <<<<<<"),
                                   ("4", "VM exiting with result code 0.")):
                if sum(pid == owner and priority == level and tag == "AndroidRuntime" and value.rstrip() == message
                       for pid, priority, tag, value in records) != 1:
                    return "missing-framework-frontend-proof"
    elif mode == "framework":
        messages = re.findall(r"^ANDROID_LOG pid=(\d+) buffer=0 priority=4 tag=bndroid-framework message=(.*)\n", text, re.MULTILINE)
        successes = [pid for pid, message in messages if message == expected.rstrip("\n")]
        if len(successes) != 2 or len(set(successes)) != 2:
            return "exited-without-expected-proof"
        if any(sum(pid == owner and message == "FRAMEWORK_STAGE " + stage for pid, message in messages) != 1
               for owner in successes for stage in ("parcel", "wake", "looper")):
            return "missing-framework-stage"
        shutdowns = re.findall(r"^ANDROID_LOG pid=(\d+) buffer=0 priority=3 tag=AndroidRuntime message=Shutting down VM *\n", text, re.MULTILINE)
        if sorted(shutdowns) != sorted(successes) or "Warning: unable to detach main thread" in text or "Warning: VM did not shut down cleanly" in text:
            return "missing-framework-shutdown-proof"
        try:
            poll = parse_marker(text, "LINUX_POLL_OK")
            if not poll or any(poll.get(key) != "0" for key in ("live", "watches")) or any(int(poll.get(key, 0)) < 2 for key in ("eventfds", "epolls", "reads", "writes", "delivered", "blocked", "ready")):
                return "missing-framework-wake-proof"
            binder = parse_marker(text, "LINUX_BINDER_OK")
            if not binder or any(binder.get(key) != "0" for key in ("connections", "transactions", "buffers", "nodes", "references", "call_frames")) or int(binder.get("opened", 0)) < 4 or binder["opened"] != binder.get("closed") or int(binder.get("flush_returns", 0)) < 2:
                return "missing-framework-shutdown-proof"
            if int(poll["closed"]) != int(poll["eventfds"]) + int(poll["epolls"]):
                return "missing-framework-shutdown-proof"
        except (ValueError, KeyError):
            return "invalid-poll-record"
    elif mode == "binder":
        if complete_lines(text, expected.rstrip("\n")) != 1 or complete_lines(text, "BINDER_SERVER_OK identity=1 reply=1 oneway=3 receive_pool=1 readonly=1 domains=3") != 1:
            return "missing-binder-process-proof"
        for marker in ("BINDER_OBJECT_CLIENT_OK export=2 handles=2 roundtrip=1 malformed=3 deaths=2 refs=1",
                       "BINDER_OBJECT_SERVER_OK export=2 callback=1 nested=2 per_node_oneway=1 refs=1"):
            if complete_lines(text, marker) != 1:
                return "missing-binder-object-proof"
        try:
            binder = parse_marker(text, "LINUX_BINDER_OK")
            if not binder or any(binder.get(key) != "0" for key in ("connections", "transactions", "buffers", "nodes", "references", "call_frames")):
                return "invalid-binder-cleanup"
            if any(int(binder.get(key, 0)) < count for key, count in (("opened", 5), ("submitted", 16), ("replied", 8), ("oneway", 6), ("failed", 10), ("deaths", 3), ("flush_returns", 1), ("objects", 7), ("reference_events", 10), ("nested", 2), ("max_depth", 3))) or binder["opened"] != binder["closed"]:
                return "missing-binder-transport-proof"
        except (ValueError, KeyError):
            return "invalid-binder-record"
    elif complete_lines(text, expected.rstrip("\n")) != 2 or (mode == "help" and complete_lines(text, "The following ART arguments are supported:") != 2):
        return "exited-without-expected-proof"
    required = {"abi": "70", "native_processes": "2", "native_exits": "2", "native_reaped": "2",
                "remaining_processes": "1", "native_handles": "0", "live_anonymous_pages": "0",
                "live_anonymous_tables": "0", "live_file_descriptions": "0", "loader": "upstream-android-linker"}
    if any(proof.get(key) != value for key, value in required.items()):
        return "invalid-cleanup-proof"
    exits = re.findall(r"^LINUX_PROCESS_EXIT pid=(\d+) code=0 native_handles=0$", text, re.MULTILINE)
    if len(exits) != 2 or len(set(exits)) != 2 or not threads or any(threads.get(key) != "0" for key in ("live", "waiting")):
        return "invalid-process-proof"
    if mode.startswith("framework") and set(successes) != set(exits):
        return "invalid-process-proof"
    if not threads.get("created") or threads["created"] != threads.get("reaped"):
        return "invalid-thread-proof"
    if mode == "poll":
        try:
            poll = parse_marker(text, "LINUX_POLL_OK")
            signals = parse_marker(text, "LINUX_SIGNALS_OK")
            if not poll or poll.get("live") != "0" or poll.get("watches") != "0" or int(poll["closed"]) != int(poll["eventfds"]) + int(poll["epolls"]):
                return "invalid-poll-cleanup"
            if any(int(poll.get(key, 0)) < count for key, count in (("eventfds", 20), ("epolls", 10), ("reads", 30), ("writes", 30), ("delivered", 20), ("blocked", 16), ("ready", 10), ("expired", 4), ("cancelled", 8))) or int(threads["created"]) < 20:
                return "missing-poll-transport-proof"
            if not signals or any(signals.get(key) != value for key, value in (("delivered", "8"), ("returned", "8"), ("pending", "0"))) or int(signals.get("interrupted", 0)) < 8:
                return "invalid-poll-signal-proof"
        except (ValueError, KeyError):
            return "invalid-poll-record"
    if mode == "binder":
        if {binder.get("first_sender"), binder.get("first_receiver")} != set(exits):
            return "invalid-binder-process-identity"
        try:
            signals = parse_marker(text, "LINUX_SIGNALS_OK")
            if not signals or signals.get("pending") != "0" or signals.get("delivered") != "3" or signals.get("returned") != "3" or int(signals.get("interrupted", 0)) < 1 or int(threads["created"]) < 2:
                return "invalid-binder-signal-proof"
        except ValueError:
            return "invalid-binder-signal-proof"
    if mode == "memfd":
        try:
            signals = parse_marker(text, "LINUX_SIGNALS_OK")
            if not signals or signals.get("pending") != "0" or int(signals.get("delivered", 0)) != 6 or signals.get("returned") != signals.get("delivered"):
                return "invalid-memfd-fault-proof"
            if not 16384 <= int(proof.get("peak_anonymous_pages", 0)) < 20000 or int(proof.get("reclaimed_anonymous_pages", 0)) < 32768:
                return "invalid-memfd-backing-proof"
        except ValueError:
            return "invalid-memfd-fault-proof"
    if mode == "properties":
        try:
            if int(threads.get("waits", 0)) < 2 or int(threads.get("timeouts", 0)) < 2:
                return "missing-property-wait-proof"
        except ValueError:
            return "missing-property-wait-proof"
    if mode == "managed":
        if complete_lines(text, "ART_APK_INPUT_OK dex=apk native=apk namespace=isolated") != 2:
            return "missing-apk-input-proof"
        if any(complete_lines(text, "ART_MANAGED_STAGE " + stage) != 2 for stage in ("clocks", "threads", "gc", "jni")):
            return "missing-managed-stage"
        try:
            paging = parse_marker(text, "LINUX_PAGING_OK")
            if int(threads["created"]) < 20 or not paging or int(paging.get("discarded", 0)) == 0 or int(paging.get("demand_zero", 0)) == 0:
                return "invalid-managed-runtime-proof"
        except ValueError:
            return "invalid-managed-runtime-proof"
    if mode == "signals":
        if complete_lines(text, "ART_SYNC_OK requeue=1 realtime_clock=1 absolute_wait=1") != 2:
            return "missing-sync-proof"
        try:
            signals = parse_marker(text, "LINUX_SIGNALS_OK")
            if not signals or signals.get("pending") != "0" or int(signals.get("delivered", 0)) < 24 or signals.get("returned") != signals.get("delivered") or int(signals.get("interrupted", 0)) < 8:
                return "invalid-signal-proof"
        except ValueError:
            return "invalid-signal-proof"
    if mode == "memory":
        if complete_lines(text, "ART_REMAP_OK fixed=1 replaced=1 readonly=1 growth_zero=1 source_unmapped=1 atomic_failure=1") != 2:
            return "missing-fixed-remap-proof"
        if complete_lines(text, "ART_PAGING_OK discard=1 rss=1 zero_fault=1 usercopy=1 protections=1 atomic_failure=1 cpu_time=1") != 2:
            return "missing-paging-proof"
        try:
            paging = parse_marker(text, "LINUX_PAGING_OK")
            if not paging or int(paging.get("discarded", 0)) < 4096 or int(paging.get("demand_zero", 0)) < 2048:
                return "invalid-paging-proof"
        except ValueError:
            return "invalid-paging-proof"
        if complete_lines(text, "ART_PROC_OK stack=1 maps=1 counters=1 snapshots=1 large_read=1 shared_ro=1 mremap=1 log=1") != 2:
            return "missing-proc-proof"
        logged = re.findall(r"^ANDROID_LOG pid=(\d+) buffer=0 priority=4 tag=bndroid-probe message=PROC_LOG_TRANSPORT_OK\n", text, re.MULTILINE)
        if len(logged) != 2 or set(logged) != set(exits):
            return "missing-log-proof"
        try:
            if int(proof["peak_anonymous_pages"]) < 16384 or int(proof["reclaimed_anonymous_pages"]) < 32768:
                return "invalid-memory-proof"
        except (KeyError, ValueError):
            return "invalid-memory-proof"
    return mode + "-passed"


def diagnostic_record(text, marker):
    try:
        return parse_marker(text, marker)
    except ValueError as error:
        return {"invalid_record": str(error)}


def signal_failure_status(text, mode):
    failed = diagnostic_record(text, "LINUX_RUNTIME_FAILURE_CLEANUP")
    if failed is None:
        return None
    fatal = diagnostic_record(text, "LINUX_SIGNAL_FATAL")
    expected_signal = "9" if mode == "signal-default" else "11"
    expected_threads = "1" if mode == "signal-default" else "0"
    armed = {"signal-default": "SIGNAL_DEFAULT_ARMED", "signal-guard": "SIGNAL_GUARD_ARMED"}.get(mode, "SIGNAL_BAD_FRAME_ARMED")
    fault = diagnostic_record(text, "LINUX_USER_FAULT")
    group = diagnostic_record(text, "LINUX_THREADS_FAILURE_CLEANUP")
    signals = diagnostic_record(text, "LINUX_SIGNALS_OK")
    valid = (complete_lines(text, armed) == 1 and fatal and fatal.get("signal") == expected_signal
             and fault and fault.get("reason") == "4" and fault.get("pid") == fatal.get("pid")
             and all(failed.get(key) == "0" for key in ("live_pages", "live_tables", "live_files"))
             and failed.get("live_processes") == "1" and failed.get("reaped") == "1" and failed.get("faulted") == "1"
             and group and group.get("live") == "0" and group.get("waiting") == "0"
             and group.get("created") == expected_threads and group.get("reaped") == expected_threads
             and signals and signals.get("pending") == "0")
    return mode + "-passed" if valid else "invalid-signal-containment"


def main():
    artifacts = Path(sys.argv[1])
    mode = os.environ.get("BNDROID_ART_MODE", "java")
    timeout = float(os.environ.get("BNDROID_QEMU_TIMEOUT_SECONDS", "360" if mode.startswith("framework") else "120"))
    if not math.isfinite(timeout) or timeout <= 0:
        raise SystemExit("Timeout must be finite and positive")
    disk = artifacts / "storage.raw"
    original_hash = hashlib.sha256(disk.read_bytes()).hexdigest()
    process = None
    text = ""
    status = "timeout"
    if mode not in ("java", "managed", "memory", "memfd", "binder", "poll", "properties", "help", "framework", "framework-help", "signals", "signal-bad-frame", "signal-bad-pc", "signal-bad-fp", "signal-default", "signal-guard"):
        raise SystemExit("Unknown ART mode")
    with tempfile.NamedTemporaryFile(prefix="bndroid-art-entropy-") as entropy:
        entropy.write(os.urandom(65536))
        entropy.flush()
        try:
            with (artifacts / "qemu.log").open("wb") as output:
                process = subprocess.Popen(sys.argv[2:] + ["-fw_cfg", f"name=opt/bndroid/entropy,file={entropy.name}"], stdout=output, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                serial = artifacts / "serial.log"
                text = serial.read_text(errors="replace").replace("\r", "") if serial.exists() else ""
                if mode.startswith("signal-"):
                    completed = signal_failure_status(text, mode)
                    if completed is not None:
                        status = completed
                        break
                if any(error in text for error in ("kernel panic:", "panicked at", "boot error:", "fatal exception:")):
                    status = "guest-failure"
                    break
                completed = completion_status(text, mode) if not mode.startswith("signal-") else None
                if completed is not None:
                    status = completed
                    break
                if process.poll() is not None:
                    status = "qemu-exited"
                    break
                time.sleep(0.05)
        finally:
            if process is not None and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    unchanged = hashlib.sha256(disk.read_bytes()).hexdigest() == original_hash
    summary = {"status": status, "mode": mode, "full_android_apps": False, "disk_unchanged": unchanged,
               "art_frontend_verified": mode == "help" and status == "help-passed",
               "managed_java_verified": mode in ("java", "managed") and status == mode + "-passed",
               "managed_runtime_verified": mode == "managed" and status == "managed-passed",
               "apk_dex_jni_verified": mode == "managed" and status == "managed-passed",
               "framework_frontend_verified": mode == "framework-help" and status == "framework-help-passed",
               "shared_memfd_verified": mode == "memfd" and status == "memfd-passed",
               "binder_transport_verified": mode == "binder" and status == "binder-passed",
               "binder": diagnostic_record(text, "LINUX_BINDER_OK"),
               "event_poll_verified": mode == "poll" and status == "poll-passed",
               "poll": diagnostic_record(text, "LINUX_POLL_OK"),
               "boot_properties_verified": mode == "properties" and status == "properties-passed",
               "framework_parcel_looper_verified": mode == "framework" and status == "framework-passed",
               "abort_messages": re.findall(r"^LINUX_ABORT_MESSAGE pid=\d+ message=(.*)$", text, re.MULTILINE),
               "android_errors": re.findall(r"^ANDROID_LOG pid=\d+ buffer=\d+ priority=[67] tag=(.*)\n", text, re.MULTILINE)[-16:],
               "unsupported_syscalls": sorted(set(map(int, re.findall(r"LINUX_SYSCALL_UNSUPPORTED pid=\d+ number=(\d+)", text)))),
               "fault": diagnostic_record(text, "LINUX_USER_FAULT"),
               "cleanup": diagnostic_record(text, "LINUX_RUNTIME_FAILURE_CLEANUP"),
               "signals": diagnostic_record(text, "LINUX_SIGNALS_OK"),
               "paging": diagnostic_record(text, "LINUX_PAGING_OK"),
               "runtime": diagnostic_record(text, "BIONIC_DYNAMIC_RUNTIME_OK")}
    (artifacts / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"ART_RUNTIME_RESULT status={status} evidence={artifacts}")
    if status != mode + "-passed" or not unchanged:
        print("\n".join(text.splitlines()[-45:]), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
