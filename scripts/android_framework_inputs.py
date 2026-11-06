"""Extend verified local ART inputs with the source Framework boot/runtime set.

Only the extractor's owned offline emulator is accessed. No application, GMS,
vendor service package, source checkout or repository binary import is used.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

from android_art_inputs import LIMIT

REQUIRED_METADATA = ("/system/etc/init/netbpfload.rc",)


def validate_inputs(output):
    receipt = json.loads((output / "art-provenance.json").read_text())
    for name, entry in receipt["files"].items():
        validate_input(output, name, entry)
    for name, entry in {**receipt["source"]["files"], **receipt["metadata_files"]}.items():
        if Path(name).name != name:
            raise RuntimeError(f"Noncanonical metadata file: {name}")
        data = (output / name).read_bytes()
        if len(data) != entry["size"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
            raise RuntimeError(f"Runtime metadata changed since extraction: {name}")
    if sum(entry["size"] for entry in receipt["files"].values()) != receipt["total_bytes"]:
        raise RuntimeError("Runtime receipt byte total is inconsistent")
    if receipt["total_bytes"] > LIMIT:
        raise RuntimeError("Runtime receipt exceeds the acquisition limit")
    return receipt


def canonical_path(name):
    return (re.fullmatch(r"/(apex|system|system_ext)/[A-Za-z0-9_./@+-]+", name)
            and not any(part in ("", ".", "..") for part in name.split("/")[1:]))


def validate_input(output, name, entry):
    if not canonical_path(name):
        raise RuntimeError(f"Noncanonical Framework input path: {name}")
    data = (output / "root" / name.lstrip("/")).read_bytes()
    if len(data) != entry["size"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
        raise RuntimeError(f"Framework input changed since extraction: {name}")


def extract_framework(command, output, sdk):
    receipt_path = output / "art-provenance.json"
    receipt = json.loads(receipt_path.read_text())
    files = receipt["files"]
    core_files = receipt.get("framework", {}).get("core_art_files", sorted(files))
    progress_path = output / "framework-progress.json"
    source_hash = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
    if progress_path.exists():
        progress = json.loads(progress_path.read_text())
        if progress["source_receipt_sha256"] != source_hash:
            raise RuntimeError("Framework progress belongs to a different ART receipt")
        for name, entry in progress["files"].items():
            validate_input(output, name, entry)
            if name in files and files[name] != entry:
                raise RuntimeError(f"Framework progress changed a core ART input: {name}")
        files.update(progress["files"])
    root = output / "root"
    readelf = sorted((sdk / "ndk/27.1.12297006/toolchains/llvm/prebuilt").glob("*/bin/llvm-readelf"))
    if not readelf:
        raise RuntimeError("Installed NDK llvm-readelf is required for the Framework dependency inventory")
    readelf = readelf[0]
    inventory = set((output / "art-inventory.txt").read_text().splitlines())
    modules = sorted({node.attrib["moduleName"] for node in ET.parse(output / "apex-info-list.xml").getroot()
                      if node.attrib.get("isActive") == "true" and re.fullmatch(r"com\.android\.[a-z0-9.]+", node.attrib.get("moduleName", ""))})
    directories = ["/system/bin", "/system/framework", "/system/etc/init", "/system_ext/lib64", "/apex/com.android.tzdata/etc"]
    directories += [f"/apex/{module}/{part}" for module in modules for part in ("lib64", "javalib")]
    warnings = []
    for directory in directories:
        try:
            listing = command("shell", "find", directory, "-type", "f", "-o", "-type", "l", timeout=30)
        except subprocess.CalledProcessError as error:
            listing = error.stdout
            warnings.append({"directory": directory, "stderr": error.stderr})
        for path in listing.splitlines():
            if canonical_path(path):
                inventory.add(path)
    (output / "framework-inventory.txt").write_text("\n".join(sorted(inventory)) + "\n")
    total = sum(entry["size"] for entry in files.values())
    if total > LIMIT:
        raise RuntimeError("Framework checkpoint exceeds the acquisition limit")

    def pull(path):
        nonlocal total
        if not canonical_path(path):
            raise RuntimeError(f"Noncanonical Framework input path: {path}")
        target = root / path.lstrip("/")
        if path in files:
            return target
        if path not in inventory:
            raise RuntimeError(f"Required Framework input absent: {path}")
        size = int(command("shell", "stat", "-Lc", "%s", path))
        if size < 0 or total + size > LIMIT:
            raise RuntimeError(f"Framework inputs exceed 512 MiB at {path}; copied {total} bytes")
        source = command("shell", "readlink", "-f", path)
        if not canonical_path(source):
            raise RuntimeError(f"Unexpected Framework source target: {path} -> {source}")
        target.parent.mkdir(parents=True, exist_ok=True)
        command("pull", source, str(target), timeout=60)
        data = target.read_bytes()
        if len(data) != size:
            raise RuntimeError(f"Source size changed during Framework extraction: {path}")
        files[path] = {"size": size, "sha256": hashlib.sha256(data).hexdigest(), "source_path": source}
        total += size
        checkpoint = progress_path.with_suffix(".tmp")
        checkpoint.write_text(json.dumps({"source_receipt_sha256": source_hash, "files": files}, indent=2) + "\n")
        checkpoint.replace(progress_path)
        return target

    bootclasspath = receipt["environment"]["BOOTCLASSPATH"].split(":")
    for path in bootclasspath:
        if not re.fullmatch(r"/(system/framework|apex/com\.android\.[a-z0-9.]+/javalib)/[A-Za-z0-9_.-]+\.jar", path):
            raise RuntimeError(f"Boot classpath is outside the approved platform/APEX set: {path}")
        pull(path)
    pull("/system/framework/framework-res.apk")
    for path in REQUIRED_METADATA:
        # libmeminfo's API-36 constructor reads the version header. This is
        # passive source metadata; Bndroid does not execute Android init rules.
        pull(path)
    for path in sorted(inventory):
        if path.startswith("/apex/com.android.tzdata/etc/"):
            pull(path)

    search = ["/apex/com.android.runtime/lib64/bionic", "/system/lib64", "/system_ext/lib64"]
    search += [f"/apex/{module}/lib64" for module in modules]
    entrypoints = ["/system/bin/app_process64"]
    queue = entrypoints + ["/system/lib64/libandroid_runtime.so", "/system/lib64/libandroid.so"]
    dependencies = receipt["dependencies"]
    alternatives = {}
    while queue:
        path = queue.pop(0)
        if path in dependencies:
            continue
        target = pull(path)
        dynamic = subprocess.run([str(readelf), "--dynamic", "--wide", str(target)], check=True, capture_output=True, text=True).stdout
        names = re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", dynamic)
        resolved = []
        for name in names:
            if not re.fullmatch(r"[A-Za-z0-9_+@.-]+", name):
                raise RuntimeError(f"Noncanonical DT_NEEDED name: {name}")
            ordered = list(dict.fromkeys([str(Path(path).parent), *search]))
            matches = [f"{directory}/{name}" for directory in ordered if f"{directory}/{name}" in inventory]
            if not matches:
                raise RuntimeError(f"Unresolved Framework ELF dependency: {path} -> {name}")
            resolved.append(matches[0])
            if len(matches) > 1:
                alternatives[path + ":" + name] = matches
        dependencies[path] = resolved
        queue.extend(resolved)
    framework = {"entrypoints": entrypoints, "bootclasspath": bootclasspath, "core_art_files": core_files,
                 "dependency_search_paths": search, "library_candidates": alternatives,
                 "inventory_warnings": warnings, "total_files": len(files), "total_bytes": total,
                 "note": "Local platform Framework/runtime inputs only; no Google apps, Play services or vendor APKs."}
    receipt["framework"] = framework
    receipt["total_bytes"] = total
    (output / "framework-inventory.json").write_text(json.dumps(framework, indent=2) + "\n")
    complete = receipt_path.with_suffix(".tmp")
    complete.write_text(json.dumps(receipt, indent=2) + "\n")
    complete.replace(receipt_path)
    progress_path.unlink(missing_ok=True)
    print(f"FRAMEWORK_RUNTIME_EXTRACT_OK files={len(files)} bytes={total} boot_jars={len(bootclasspath)} path={output}", flush=True)
