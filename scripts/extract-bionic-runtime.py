#!/usr/bin/env python3
"""Read Bionic from one isolated, temporary emulator using an installed SDK image.

No downloads, existing AVD changes, physical-device commands, or binary imports
into Git are performed. Every adb command issued here selects the owned emulator.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import uuid


def free_port():
    for port in range(5584, 5683, 2):
        held = []
        try:
            for candidate in (port, port + 1):
                stream = socket.socket()
                held.append(stream)
                stream.bind(("127.0.0.1", candidate))
            return port
        except OSError:
            continue
        finally:
            for stream in held:
                stream.close()
    raise RuntimeError("No unused emulator console/ADB port pair")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk", type=Path, default=Path(os.environ.get("ANDROID_SDK_ROOT", os.environ.get("ANDROID_HOME", str(Path.home() / "Library/Android/sdk")))))
    parser.add_argument("--system-image", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--include-art", action="store_true", help="Also inventory and extract bounded ART/core-Java runtime inputs")
    parser.add_argument("--include-framework", action="store_true", help="Include ART plus the source Framework boot jars and app_process dependencies")
    parser.add_argument("--resume-framework", action="store_true", help="Resume an interrupted Framework extension of an already extracted ART set")
    args = parser.parse_args()
    sdk, image, output = args.sdk.resolve(), args.system_image.resolve(), args.output.resolve()
    properties = (image / "source.properties").read_text()
    if "SystemImage.Abi=arm64-v8a" not in properties:
        raise SystemExit("An installed arm64-v8a SDK system image is required")
    for required in (image / "system.img", sdk / "emulator/emulator", sdk / "platform-tools/adb"):
        if not required.is_file():
            raise SystemExit(f"Missing local SDK prerequisite: {required}")
    if args.resume_framework:
        if not args.include_framework or not (output / "art-provenance.json").is_file():
            raise SystemExit("--resume-framework requires --include-framework and an existing ART receipt")
        from android_framework_inputs import framework_inputs_complete, validate_inputs
        verified = validate_inputs(output)
        previous = json.loads((output / "provenance.json").read_text())
        if previous != verified["source"]:
            raise SystemExit("ART and Bionic source receipts are inconsistent")
        if previous["package_properties"] != properties:
            raise SystemExit("Cannot resume from a different SDK system image")
        if framework_inputs_complete(verified):
            print(f"FRAMEWORK_RUNTIME_ALREADY_COMPLETE path={output}", flush=True)
            return
    else:
        output.mkdir(parents=True, exist_ok=False)
    process = None
    port = free_port()
    serial = f"emulator-{port}"
    name = "BndroidSource" + uuid.uuid4().hex[:12]
    adb = [str(sdk / "platform-tools/adb"), "-s", serial]

    def command(*arguments, timeout=10):
        return subprocess.run(adb + list(arguments), check=True, capture_output=True, text=True, timeout=timeout).stdout.strip()

    # All writable AVD data lives here and is removed after the owned process
    # exits. Read-only boot additionally keeps source image files untouched.
    with tempfile.TemporaryDirectory(prefix="bndroid-source-avd-") as temporary:
        avd_root = Path(temporary)
        avd = avd_root / f"{name}.avd"
        avd.mkdir()
        (avd_root / f"{name}.ini").write_text(f"avd.ini.encoding=UTF-8\npath={avd}\n")
        (avd / "config.ini").write_text("\n".join([
            f"AvdId={name}", f"avd.ini.displayname={name}", "avd.ini.encoding=UTF-8",
            "abi.type=arm64-v8a", "hw.cpu.arch=arm64", "hw.cpu.ncore=2",
            "hw.ramSize=2048", "hw.gpu.enabled=yes", "hw.gpu.mode=swiftshader",
            "hw.lcd.width=720", "hw.lcd.height=1280", "hw.lcd.density=320",
            "hw.keyboard=yes", "hw.mainKeys=no", "hw.sdCard=no",
            "hw.camera.back=none", "hw.camera.front=none", "PlayStore.enabled=false",
            "disk.dataPartition.size=6G", "fastboot.forceColdBoot=yes",
            f"image.sysdir.1={image}/", "showDeviceFrame=no", "",
        ]))
        environment = dict(os.environ, ANDROID_AVD_HOME=str(avd_root), ANDROID_SDK_ROOT=str(sdk))
        with (output / "emulator.log").open("wb") as log:
            try:
                process = subprocess.Popen([
                    str(sdk / "emulator/emulator"), "-avd", name, "-port", str(port),
                    "-no-window", "-no-audio", "-no-snapshot", "-no-cache", "-no-boot-anim",
                    "-gpu", "swiftshader", "-camera-back", "none", "-camera-front", "none",
                    "-read-only", "-no-metrics", "-qemu", "-net", "none",
                ], env=environment, stdout=log, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 180
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise RuntimeError("Temporary source emulator exited; inspect emulator.log")
                    try:
                        # Check AVD identity before issuing any shell or pull.
                        identity = command("emu", "avd", "name", timeout=3).splitlines()[0]
                        if identity != name:
                            raise RuntimeError(f"Port ownership changed: expected {name}, got {identity}")
                        if command("shell", "getprop", "sys.boot_completed", timeout=3) == "1":
                            break
                    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, IndexError):
                        pass
                    time.sleep(1)
                else:
                    raise TimeoutError("Temporary source emulator did not boot within 180 seconds")
                fingerprint = command("shell", "getprop", "ro.build.fingerprint")
                if args.resume_framework:
                    if fingerprint != previous["build_fingerprint"]:
                        raise RuntimeError("Cannot resume from a different source build fingerprint")
                    from android_framework_inputs import extract_framework
                    extract_framework(command, output, sdk)
                    print(f"FRAMEWORK_RUNTIME_RESUME_OK source={fingerprint} path={output}", flush=True)
                    return
                files = {}
                for filename in ("linker64", "libc.so", "libm.so", "libdl.so"):
                    subdirectory = "bin" if filename == "linker64" else "lib64/bionic"
                    remote = f"/apex/com.android.runtime/{subdirectory}/{filename}"
                    command("pull", remote, str(output / filename), timeout=30)
                    data = (output / filename).read_bytes()
                    files[filename] = {"size": len(data), "sha256": hashlib.sha256(data).hexdigest(), "path": remote}
                receipt = {"source": "existing-local-android-sdk", "build_fingerprint": fingerprint,
                           "package_properties": properties, "files": files}
                notice = image / "NOTICE.txt"
                if notice.is_file():
                    shutil.copyfile(notice, output / "source-NOTICE.txt")
                    receipt["notice_sha256"] = hashlib.sha256(notice.read_bytes()).hexdigest()
                (output / "provenance.json").write_text(json.dumps(receipt, indent=2) + "\n")
                if args.include_art or args.include_framework:
                    from android_art_inputs import extract_art
                    extract_art(command, output, sdk, receipt)
                if args.include_framework:
                    from android_framework_inputs import extract_framework
                    extract_framework(command, output, sdk)
                print(f"BIONIC_RUNTIME_EXTRACT_OK source={fingerprint} path={output}", flush=True)
            finally:
                # Terminate the Popen owner directly: never kill a possibly
                # reused ADB serial, unrelated emulator, or physical transport.
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)


if __name__ == "__main__":
    main()
