"""Bounded, local-only ART input acquisition from the extractor's owned emulator."""
import hashlib
import json
from pathlib import Path
import re
import subprocess


LIMIT = 512 * 1024 * 1024


def extract_art(command, output, sdk, bionic_receipt):
    readelf = sdk / "ndk/27.1.12297006/toolchains/llvm/prebuilt"
    candidates = sorted(readelf.glob("*/bin/llvm-readelf"))
    if not candidates:
        raise RuntimeError("Installed NDK llvm-readelf is required for the ART dependency inventory")
    readelf = candidates[0]
    root = output / "root"
    root.mkdir()
    inventory = {}
    inventory_warnings = []
    for directory in ("/apex/com.android.art/bin", "/apex/com.android.art/lib64", "/apex/com.android.art/javalib",
                      "/apex/com.android.i18n/lib64", "/apex/com.android.i18n/javalib", "/apex/com.android.i18n/etc/icu",
                      "/apex/com.android.runtime/lib64/bionic", "/apex/com.android.os.statsd/lib64",
                      "/apex/com.android.adbd/lib64", "/system/lib64"):
        try:
            listing = command("shell", "find", directory, "-type", "f", timeout=30)
        except subprocess.CalledProcessError as error:
            # A release image can hide unrelated files even inside an APEX.
            # Preserve diagnostics; required dependencies must still resolve.
            listing = error.stdout
            inventory_warnings.append({"directory": directory, "stderr": error.stderr})
        for path in listing.splitlines():
            if re.fullmatch(r"/[A-Za-z0-9_./@+-]+", path):
                inventory[path] = path.rsplit("/", 1)[-1]
    (output / "art-inventory.txt").write_text("\n".join(sorted(inventory)) + "\n")
    metadata = {}
    for variable in ("BOOTCLASSPATH", "DEX2OATBOOTCLASSPATH", "ANDROID_ROOT", "ANDROID_ART_ROOT", "ANDROID_I18N_ROOT", "ANDROID_TZDATA_ROOT"):
        metadata[variable] = command("shell", "printenv", variable)
    (output / "art-environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for remote, name in (("/linkerconfig/ld.config.txt", "ld.config.txt"), ("/apex/apex-info-list.xml", "apex-info-list.xml")):
        command("pull", remote, str(output / name), timeout=30)
    files = {}
    total = 0

    def pull(path):
        nonlocal total
        if path in files:
            return root / path.lstrip("/")
        if path not in inventory:
            raise RuntimeError(f"ART dependency absent from installed source: {path}")
        size = int(command("shell", "stat", "-Lc", "%s", path))
        if size < 0 or total + size > LIMIT:
            raise RuntimeError("ART inputs exceed the 512 MiB acquisition budget")
        target = root / path.lstrip("/")
        target.parent.mkdir(parents=True, exist_ok=True)
        command("pull", path, str(target), timeout=60)
        data = target.read_bytes()
        if len(data) != size:
            raise RuntimeError(f"Source size changed during extraction: {path}")
        total += size
        files[path] = {"size": size, "sha256": hashlib.sha256(data).hexdigest()}
        return target

    # Freeze dependencies from the runtime APEX, with only its platform shared
    # dependencies taken from /system. Keep the namespace config for auditing.
    search = ("/apex/com.android.art/lib64", "/apex/com.android.runtime/lib64/bionic",
              "/apex/com.android.i18n/lib64", "/apex/com.android.os.statsd/lib64",
              "/apex/com.android.adbd/lib64", "/system/lib64")
    queue = ["/apex/com.android.art/bin/dalvikvm64"]
    for name in ("libart.so", "libjavacore.so", "libopenjdk.so", "libopenjdkjvm.so", "libicu_jni.so", "libartpalette-system.so"):
        matches = [f"{directory}/{name}" for directory in search if f"{directory}/{name}" in inventory]
        if not matches:
            raise RuntimeError(f"Required runtime library missing: {name}")
        queue.append(matches[0])
    dependencies = {}
    while queue:
        path = queue.pop(0)
        if path in dependencies:
            continue
        target = pull(path)
        dynamic = subprocess.run([str(readelf), "--dynamic", "--wide", str(target)], check=True, capture_output=True, text=True).stdout
        names = re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", dynamic)
        resolved = []
        for name in names:
            own = str(Path(path).parent)
            ordered = [own, "/apex/com.android.runtime/lib64/bionic", "/system/lib64", *search]
            matches = [f"{directory}/{name}" for directory in ordered if f"{directory}/{name}" in inventory]
            if not matches:
                raise RuntimeError(f"Unresolved ELF dependency: {path} -> {name}")
            resolved.append(matches[0])
        dependencies[path] = resolved
        queue.extend(resolved)
    # Copy core-library DEX and their matching prebuilt boot images only. Do not
    # copy Framework, GMS, Play Store or application/service APKs.
    for path in sorted(inventory):
        if (path.startswith("/apex/com.android.art/javalib/") and path.endswith((".jar", ".art", ".oat", ".vdex"))) or path == "/apex/com.android.i18n/javalib/core-icu4j.jar":
            pull(path)
        elif path.startswith("/apex/com.android.i18n/etc/icu/") and path.endswith(".dat"):
            pull(path)
    receipt = {"source": bionic_receipt, "environment": metadata, "files": files,
               "dependencies": dependencies, "total_bytes": total, "inventory_warnings": inventory_warnings,
               "dependency_search_paths": list(search), "acquisition_limit_bytes": LIMIT,
               "note": "Local integration inputs; namespace configuration retained; no redistribution grant."}
    (output / "art-provenance.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"ART_RUNTIME_EXTRACT_OK files={len(files)} bytes={total} path={output}", flush=True)
