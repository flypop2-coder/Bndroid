#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true
TARGET_ROOT="$ROOT/target/linux-abi-build"
mkdir -p "$ROOT/target/linux-abi"
ARTIFACTS="$(mktemp -d "$ROOT/target/linux-abi/check.XXXXXX")"
export BNDROID_LINUX_PROBE_DIR="$ARTIFACTS"
"$ROOT/fixtures/linux-arm64-probe/build.sh" >"$ARTIFACTS/native-build.log" 2>&1 || {
  cat "$ARTIFACTS/native-build.log" >&2; exit 1;
}
export BNDROID_LINUX_APP_ELF="$ARTIFACTS/linux-probe.elf"
CARGO_TARGET_DIR="$TARGET_ROOT" BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=linux-abi-runtime BNDROID_USERSPACE_FEATURES=linux-abi-runtime \
  "$SCRIPT_DIR/build-kernel.sh" >"$ARTIFACTS/kernel-build.log" 2>&1 || {
    tail -n 80 "$ARTIFACTS/kernel-build.log" >&2; exit 1;
  }
STORAGE_IMAGE="$ARTIFACTS/storage.raw"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >"$ARTIFACTS/storage-build.log"
build_storage_qemu_args "$STORAGE_IMAGE" modern writable-temporary
python3 - "$ARTIFACTS" \
  qemu-system-aarch64 -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m 128M -display none -monitor none -nic none \
  -serial "file:$ARTIFACTS/serial.log" -no-reboot \
  -kernel "$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img" -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
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
import time
from scripts.linux_runtime_evidence import parse_marker

artifacts = Path(sys.argv[1])
timeout = float(os.environ.get("BNDROID_QEMU_TIMEOUT_SECONDS", "60"))
if not math.isfinite(timeout) or timeout <= 0:
    raise SystemExit("BNDROID_QEMU_TIMEOUT_SECONDS must be finite and positive")
disk_hash = hashlib.sha256((artifacts / "storage.raw").read_bytes()).hexdigest()
process = None
try:
    with (artifacts / "qemu.log").open("wb") as output:
        process = subprocess.Popen(sys.argv[2:], stdout=output, stderr=subprocess.STDOUT)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        serial = artifacts / "serial.log"
        text = serial.read_text(errors="replace") if serial.exists() else ""
        text = text.replace("\r", "")
        if re.search(r"kernel panic:|panicked at|boot error:|fatal exception:|LINUX_RUNTIME_(FAIL|TIMEOUT)", text):
            raise RuntimeError("Guest rejected the native runtime")
        if process.poll() is not None:
            raise RuntimeError(f"QEMU exited early: {process.returncode}")
        proof = parse_marker(text, "LINUX_ABI_RUNTIME_OK")
        if proof is not None:
            expected = {"abi": "70", "native_processes": "2", "native_exits": "2", "native_reaped": "2", "remaining_processes": "1", "native_handles": "0", "supervisor_channels": "2", "unsupported": "8", "private_svc_rejected": "2", "bad_address": "10", "linux_kernel": "0", "bionic_startup": "0", "art": "0"}
            for key, value in expected.items():
                assert proof.get(key) == value, (key, proof.get(key), value)
            exits = re.findall(r"^LINUX_PROCESS_EXIT pid=(\d+) code=0 native_handles=0$", text, re.MULTILINE)
            assert len(exits) == 2 and len(set(exits)) == 2
            assert text.count("LINUX_ARM64_PROBE_OK entry=elf stack=linux syscalls=linux memory=isolated\n") == 2
            assert int(proof["linux_calls"]) == 52 and int(proof["console_bytes"]) == 750
            assert hashlib.sha256((artifacts / "storage.raw").read_bytes()).hexdigest() == disk_hash
            proof["elf_sha256"] = hashlib.sha256((artifacts / "linux-probe.elf").read_bytes()).hexdigest()
            (artifacts / "summary.json").write_text(json.dumps(proof, indent=2) + "\n")
            print(f"LINUX_ABI_QEMU_CHECK_OK processes=2 isolated=1 negative_syscalls=8 bad_address=10 evidence={artifacts}")
            break
        time.sleep(0.05)
    else:
        raise TimeoutError("Native runtime did not finish")
except BaseException:
    for name in ("serial.log", "qemu.log"):
        path = artifacts / name
        if path.exists():
            print("\n".join(path.read_text(errors="replace").splitlines()[-60:]), file=sys.stderr)
    print(f"Native runtime evidence: {artifacts}", file=sys.stderr)
    raise
finally:
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
PY
