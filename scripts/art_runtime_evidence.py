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
        "help": "dalvikvm: [options] class [argument ...]\n",
        "java": "ART_JAVA_CORE_OK sum=4950 exceptions=1 collections=1\n",
        "signals": "ART_SIGNALS_OK handlers=1 altstack=1 registers=1 masks=1 realtime=1 faults=1 threads=1 restart=1 waits=1\n",
    }[mode]
    if complete_lines(text, expected.rstrip("\n")) != 2 or (mode == "help" and complete_lines(text, "The following ART arguments are supported:") != 2):
        return "exited-without-expected-proof"
    required = {"abi": "70", "native_processes": "2", "native_exits": "2", "native_reaped": "2",
                "remaining_processes": "1", "native_handles": "0", "live_anonymous_pages": "0",
                "live_anonymous_tables": "0", "live_file_descriptions": "0", "loader": "upstream-android-linker"}
    if any(proof.get(key) != value for key, value in required.items()):
        return "invalid-cleanup-proof"
    exits = re.findall(r"^LINUX_PROCESS_EXIT pid=(\d+) code=0 native_handles=0$", text, re.MULTILINE)
    if len(exits) != 2 or len(set(exits)) != 2 or not threads or any(threads.get(key) != "0" for key in ("live", "waiting")):
        return "invalid-process-proof"
    if not threads.get("created") or threads["created"] != threads.get("reaped"):
        return "invalid-thread-proof"
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
    timeout = float(os.environ.get("BNDROID_QEMU_TIMEOUT_SECONDS", "120"))
    if not math.isfinite(timeout) or timeout <= 0:
        raise SystemExit("Timeout must be finite and positive")
    disk = artifacts / "storage.raw"
    original_hash = hashlib.sha256(disk.read_bytes()).hexdigest()
    process = None
    text = ""
    status = "timeout"
    mode = os.environ.get("BNDROID_ART_MODE", "java")
    if mode not in ("java", "memory", "help", "signals", "signal-bad-frame", "signal-bad-pc", "signal-bad-fp", "signal-default", "signal-guard"):
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
               "managed_java_verified": mode == "java" and status == "java-passed",
               "abort_messages": re.findall(r"^LINUX_ABORT_MESSAGE pid=\d+ message=(.*)$", text, re.MULTILINE),
               "android_errors": re.findall(r"^ANDROID_LOG pid=\d+ buffer=\d+ priority=[67] tag=(.*)\n", text, re.MULTILINE)[-16:],
               "unsupported_syscalls": sorted(set(map(int, re.findall(r"LINUX_SYSCALL_UNSUPPORTED pid=\d+ number=(\d+)", text)))),
               "fault": diagnostic_record(text, "LINUX_USER_FAULT"),
               "cleanup": diagnostic_record(text, "LINUX_RUNTIME_FAILURE_CLEANUP"),
               "signals": diagnostic_record(text, "LINUX_SIGNALS_OK"),
               "runtime": diagnostic_record(text, "BIONIC_DYNAMIC_RUNTIME_OK")}
    (artifacts / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"ART_RUNTIME_RESULT status={status} evidence={artifacts}")
    if status != mode + "-passed" or not unchanged:
        print("\n".join(text.splitlines()[-45:]), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
