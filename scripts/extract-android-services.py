#!/usr/bin/env python3
"""Extend local Framework inputs with unmodified system service binaries.

Some SDK user images deny adb-shell reads of servicemanager. Read the owned
local image offline instead, using checked GPT/liblp metadata and read-only
erofs-utils against temporary partition copies. Never modify or mount the SDK image.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from android_art_inputs import LIMIT
from android_framework_inputs import (SERVICE_ENTRYPOINTS, SERVICE_METADATA,
                                     canonical_path, framework_inputs_complete,
                                     service_inputs_complete, validate_inputs)
from android_sdk_image import Image, digest, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--system-image", type=Path, required=True, help="Installed SDK system-image directory")
    parser.add_argument("--dump-erofs", type=Path, default=Path(shutil.which("dump.erofs") or "/opt/homebrew/opt/erofs-utils/bin/dump.erofs"))
    parser.add_argument("--fsck-erofs", type=Path, default=Path(shutil.which("fsck.erofs") or "/opt/homebrew/opt/erofs-utils/bin/fsck.erofs"))
    parser.add_argument("--sdk", type=Path, default=Path.home() / "Library/Android/sdk")
    args = parser.parse_args()
    output = args.runtime.resolve()
    receipt = validate_inputs(output)
    require(framework_inputs_complete(receipt), "complete verified Framework inputs are required")
    require((args.system_image / "source.properties").read_text() == receipt["source"]["package_properties"],
            "SDK package differs from the verified runtime source")
    source_image = args.system_image / "system.img"
    source_hash = digest(source_image)
    if service_inputs_complete(receipt):
        require(receipt["services"]["source_image_sha256"] == source_hash, "SDK image changed since service extraction")
        print(f"ANDROID_SERVICE_INPUTS_ALREADY_COMPLETE path={output}")
        return
    require(args.dump_erofs.is_file() and args.fsck_erofs.is_file(), "erofs-utils dump.erofs and fsck.erofs are required")
    baseline = output / "services-base-provenance.json"
    if not baseline.exists():
        with baseline.open("x") as stream:
            stream.write(json.dumps(receipt, indent=2) + "\n")
    readelf = sorted((args.sdk / "ndk/27.1.12297006/toolchains/llvm/prebuilt").glob("*/bin/llvm-readelf"))
    require(bool(readelf), "installed NDK llvm-readelf is required")
    files, dependencies = receipt["files"], receipt["dependencies"]
    total = receipt["total_bytes"]
    added = []
    with tempfile.TemporaryDirectory(prefix="bndroid-service-inputs-") as temporary, Image(source_image) as image:
        temporary = Path(temporary)
        partitions = {}

        def extract(path):
            require(canonical_path(path), "noncanonical service input")
            partition = path.split("/")[1]
            require(partition in ("system", "system_ext"), "missing APEX input; extend the verified Framework acquisition first")
            if partition not in partitions:
                target = temporary / (partition + ".img")
                image.extract(partition, target)
                partitions[partition] = target
            # SDK layouts may use a mounted /system root or system-as-root.
            # Exact binary anchors below bind either layout to this runtime.
            candidates = [path[len('/' + partition):]]
            if partition == "system":
                candidates.append(path)
            size = None
            for inside in candidates:
                stat = subprocess.run([str(args.dump_erofs), "--path=" + inside, str(partitions[partition])],
                                      check=True, capture_output=True, text=True, timeout=30)
                size = re.search(r"\bSize:\s+(\d+)", stat.stdout)
                if size is not None and "regular file" in stat.stdout:
                    break
            else:
                raise ValueError(f"not a readable regular file: {path}: {stat.stderr.strip()}")
            size = int(size[1])
            require(0 <= size <= 64 * 1024**2 and total + size <= LIMIT, f"service input exceeds acquisition bounds: {path} ({size} bytes)")
            subprocess.run([str(args.fsck_erofs), "--extract", "--path=" + inside, str(partitions[partition])],
                           check=True, capture_output=True, timeout=30)
            data = subprocess.run([str(args.dump_erofs), "--cat", "--path=" + inside, str(partitions[partition])],
                                  check=True, capture_output=True, timeout=30).stdout
            require(len(data) == size, f"incomplete extraction: {path}")
            return data


        # Bind offline files to the exact already-verified running-image inputs,
        # not merely a package label. A different build must fail before writes.
        anchors = ["/system/bin/app_process64", "/system/lib64/libbinder.so"]
        for path in anchors:
            data = extract(path)
            require(hashlib.sha256(data).hexdigest() == files[path]["sha256"]
                    and len(data) == files[path]["size"], f"offline source differs from runtime anchor: {path}")

        def pull(path):
            nonlocal total
            target = output / "root" / path.lstrip("/")
            if path in files:
                return target
            data = extract(path)
            if target.exists():
                require(target.read_bytes() == data, f"unreceipted input differs: {path}")
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("xb") as stream:
                    stream.write(data)
            files[path] = {"size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                           "source_path": path, "source_image_sha256": source_hash}
            total += len(data)
            added.append(path)
            return target

        for path in SERVICE_METADATA:
            pull(path)
        inventory = set((output / "framework-inventory.txt").read_text().splitlines()) | set(files) | set(SERVICE_ENTRYPOINTS)
        search = receipt["framework"]["dependency_search_paths"]
        queue, visited = list(SERVICE_ENTRYPOINTS), set()
        while queue:
            path = queue.pop(0)
            if path in visited:
                continue
            visited.add(path)
            target = pull(path)
            if path not in dependencies:
                dynamic = subprocess.run([str(readelf[0]), "--dynamic", "--wide", str(target)],
                                         check=True, capture_output=True, text=True).stdout
                resolved = []
                for name in re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", dynamic):
                    require(re.fullmatch(r"[A-Za-z0-9_+@.-]+", name), "noncanonical DT_NEEDED name")
                    candidates = [str(Path(path).parent), *search]
                    matches = [directory + "/" + name for directory in candidates if directory + "/" + name in inventory]
                    require(matches, f"unresolved service dependency: {path} -> {name}")
                    resolved.append(matches[0])
                dependencies[path] = resolved
            queue.extend(dependencies[path])
        require(digest(source_image) == source_hash, "source image changed during extraction")
        receipt["services"] = {"entrypoints": list(SERVICE_ENTRYPOINTS), "metadata": list(SERVICE_METADATA),
                               "source_image_sha256": source_hash, "anchors": anchors,
                               "acquisition": "read-only GPT/liblp and erofs-utils; no mounts or image modifications"}
        receipt["total_bytes"] = total
        receipt["framework"].update(total_files=len(files), total_bytes=total)
        require(service_inputs_complete(receipt), "incomplete service dependency graph")
        staged = output / "art-provenance.services.tmp"
        staged.write_text(json.dumps(receipt, indent=2) + "\n")
        staged.replace(output / "art-provenance.json")
        (output / "framework-inventory.json").write_text(json.dumps(receipt["framework"], indent=2) + "\n")
    print(f"ANDROID_SERVICE_INPUTS_OK added={len(added)} files={len(files)} bytes={total} path={output}")
    for path in added:
        print(path)


if __name__ == "__main__":
    main()
