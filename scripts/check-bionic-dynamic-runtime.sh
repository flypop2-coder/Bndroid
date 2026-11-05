#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true
TARGET_ROOT="$ROOT/target/bionic-dynamic-build"
RUNTIME="${BNDROID_ANDROID_RUNTIME_DIR:-$ROOT/target/android-runtime-inputs/bionic}"
[[ -f "$RUNTIME/provenance.json" ]] || { echo "Supply BNDROID_ANDROID_RUNTIME_DIR; see fixtures/bionic-dynamic/README.md" >&2; exit 1; }
RUNTIME="$(cd -- "$RUNTIME" && pwd)"
mkdir -p "$ROOT/target/bionic-dynamic"
ARTIFACTS="$(mktemp -d "$ROOT/target/bionic-dynamic/check.XXXXXX")"
export BNDROID_DYNAMIC_PROBE_DIR="$ARTIFACTS"
"$ROOT/fixtures/bionic-dynamic/build.sh" >"$ARTIFACTS/native-build.log" 2>&1 || {
  cat "$ARTIFACTS/native-build.log" >&2; exit 1;
}
python3 "$SCRIPT_DIR/build-bionic-rootfs.py" --runtime "$RUNTIME" --probe "$ARTIFACTS" --output "$ARTIFACTS/rootfs.tar" >"$ARTIFACTS/rootfs-build.log"
export BNDROID_LINUX_APP_ELF="$RUNTIME/linker64"
CARGO_TARGET_DIR="$TARGET_ROOT" BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=bionic-dynamic-runtime BNDROID_USERSPACE_FEATURES=bionic-dynamic-runtime \
  "$SCRIPT_DIR/build-kernel.sh" >"$ARTIFACTS/kernel-build.log" 2>&1 || {
    tail -n 80 "$ARTIFACTS/kernel-build.log" >&2; exit 1;
  }
STORAGE_IMAGE="$ARTIFACTS/storage.raw"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >"$ARTIFACTS/storage-build.log"
build_storage_qemu_args "$STORAGE_IMAGE" modern writable-temporary
python3 - "$ARTIFACTS" \
  qemu-system-aarch64 -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none -nic none \
  -serial "file:$ARTIFACTS/serial.log" -no-reboot \
  -kernel "$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img" -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -fw_cfg "name=opt/bndroid/rootfs,file=$ARTIFACTS/rootfs.tar" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on <<'PY'
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
from scripts.linux_runtime_evidence import parse_marker

artifacts = Path(sys.argv[1])
timeout = float(os.environ.get("BNDROID_QEMU_TIMEOUT_SECONDS", "90"))
if not math.isfinite(timeout) or timeout <= 0:
    raise SystemExit("BNDROID_QEMU_TIMEOUT_SECONDS must be finite and positive")
disk_hash = hashlib.sha256((artifacts / "storage.raw").read_bytes()).hexdigest()
process = None
entropy = None
try:
    entropy = tempfile.NamedTemporaryFile(prefix="bndroid-entropy-", delete=False)
    entropy.write(os.urandom(65536))
    entropy.close()
    args = sys.argv[2:] + ["-fw_cfg", f"name=opt/bndroid/entropy,file={entropy.name}"]
    with (artifacts / "qemu.log").open("wb") as output:
        process = subprocess.Popen(args, stdout=output, stderr=subprocess.STDOUT)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        serial = artifacts / "serial.log"
        text = serial.read_text(errors="replace").replace("\r", "") if serial.exists() else ""
        if os.environ.get("BNDROID_DYNAMIC_FAULT_CHECK", "0") == "1":
            if "DYNAMIC_EOF_FAULT_MISSED" in text or "DYNAMIC_CHECK_FAILED" in text:
                raise RuntimeError("EOF fault containment regression")
            fault = parse_marker(text, "LINUX_USER_FAULT")
            if fault is None and text.rsplit("\n", 1)[-1].startswith("LINUX_USER_FAULT"):
                time.sleep(0.05)
                continue
            if fault is not None:
                armed = parse_marker(text, "DYNAMIC_EOF_FAULT_ARMED")
                assert armed is not None
                assert int(fault["far"], 0) == int(armed["address"], 0)
                assert int(fault["esr"], 0) >> 26 == 0x24
                cleanup = parse_marker(text, "LINUX_RUNTIME_FAILURE_CLEANUP")
                if cleanup is None:
                    if process.poll() is not None:
                        raise RuntimeError("Guest exited before fault cleanup proof")
                    time.sleep(0.05)
                    continue
                assert cleanup == {"live_processes":"1", "faulted":"1", "reaped":"1", "live_pages":"0", "live_tables":"0", "live_files":"0"}, cleanup
                assert "DYNAMIC_PROBE_OK" not in text
                assert "BIONIC_DYNAMIC_RUNTIME_OK" not in text
                assert hashlib.sha256((artifacts / "storage.raw").read_bytes()).hexdigest() == disk_hash
                (artifacts / "summary.json").write_text(json.dumps({"expected_fault": "file-page-beyond-eof", "fault": fault, "cleanup": cleanup}, indent=2) + "\n")
                print(f"BIONIC_DYNAMIC_EOF_FAULT_CHECK_OK evidence={artifacts}")
                break
        if re.search(r"kernel panic:|panicked at|boot error:|fatal exception:|LINUX_USER_FAULT|DYNAMIC_CHECK_FAILED|LINUX_RUNTIME_(FAIL|TIMEOUT)", text):
            raise RuntimeError("Bionic guest validation failed")
        if process.poll() is not None:
            raise RuntimeError(f"QEMU exited early: {process.returncode}")
        proof = parse_marker(text, "BIONIC_DYNAMIC_RUNTIME_OK")
        if proof is not None:
            for key, value in {"abi":"70", "native_processes":"2", "native_exits":"2", "native_reaped":"2", "remaining_processes":"1", "native_handles":"0", "live_anonymous_pages":"0", "live_anonymous_tables":"0", "bionic_startup":"1", "live_file_descriptions":"0", "loader":"upstream-android-linker", "art":"0"}.items():
                assert proof.get(key) == value, (key, proof.get(key), value)
            expected = "DYNAMIC_PROBE_OK linker=android shared_libc=1 dlopen=1 tls=1 files=1 private_mmap=1\n"
            assert text.count(expected) == 2
            assert text.count("DYNAMIC_ATEXIT_OK\n") == 2
            assert text.count("DYNAMIC_DESTRUCTOR_OK\n") == 2
            assert text.count("DYNAMIC_PLUGIN_UNLOADED\n") == 2
            assert text.count("DYNAMIC_MAIN_ENTERED\n") == 2
            exits = re.findall(r"^LINUX_PROCESS_EXIT pid=(\d+) code=0 native_handles=0$", text, re.MULTILINE)
            assert len(exits) == 2 and len(set(exits)) == 2
            assert int(proof["peak_anonymous_pages"]) >= 512
            assert int(proof["reclaimed_anonymous_pages"]) >= 2 * 512
            assert 32 < int(proof["entropy_bytes"]) <= 65536
            assert int(proof["bad_address"]) >= 4
            unsupported = re.findall(r"^LINUX_SYSCALL_UNSUPPORTED pid=\d+ number=(\d+)$", text, re.MULTILINE)
            assert len(unsupported) == int(proof["unsupported"])
            # These optional startup interfaces still return ENOSYS, not fake success.
            allowed = {98, 123, 132, 134, 135, 169, 198}
            assert set(map(int, unsupported)) <= allowed, sorted(set(unsupported))
            assert hashlib.sha256((artifacts / "storage.raw").read_bytes()).hexdigest() == disk_hash
            proof["unsupported_syscalls"] = sorted(set(map(int, unsupported)))
            proof["elf_sha256"] = hashlib.sha256((artifacts / "dynamic-probe").read_bytes()).hexdigest()
            proof["runtime_receipt"] = json.loads((artifacts / "rootfs.json").read_text())
            proof["entropy_source"] = "host-os.urandom-fw-cfg-consume-once"
            (artifacts / "summary.json").write_text(json.dumps(proof, indent=2) + "\n")
            print(f"BIONIC_DYNAMIC_QEMU_CHECK_OK processes=2 linker=upstream-android dlopen=1 tls=1 files=1 private_mmap=1 art=0 evidence={artifacts}")
            break
        time.sleep(0.05)
    else:
        raise TimeoutError("Bionic runtime did not complete")
except BaseException:
    for name in ("serial.log", "qemu.log"):
        path = artifacts / name
        if path.exists():
            print("\n".join(path.read_text(errors="replace").splitlines()[-60:]), file=sys.stderr)
    print(f"Bionic runtime evidence: {artifacts}", file=sys.stderr)
    raise
finally:
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    if entropy is not None:
        entropy.close()
        Path(entropy.name).unlink(missing_ok=True)
PY
