#!/usr/bin/env python3
"""Package verified local ART inputs and an owned DEX program for Bndroid."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--dex", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--mode", choices=("java", "memory", "help"), default="java")
    args = parser.parse_args()
    provenance = json.loads((args.runtime / "art-provenance.json").read_text())
    inputs = {}
    for name, receipt in provenance["files"].items():
        path = PurePosixPath(name)
        if not name.startswith("/") or ".." in path.parts or str(path) != name:
            raise SystemExit(f"Noncanonical input path: {name}")
        data = (args.runtime / "root" / name.lstrip("/")).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Runtime input changed since extraction: {name}")
        inputs[name.lstrip("/")] = data
    for name, receipt in provenance["source"]["files"].items():
        data = (args.runtime / name).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Bionic input changed since extraction: {name}")
    inputs["system/framework/bndroid-probe.dex"] = args.dex.read_bytes()
    if args.mode == "memory":
        inputs["system/bin/art-memory-probe"] = (args.dex.parent / "memory-probe").read_bytes()
    if not inputs["system/framework/bndroid-probe.dex"].startswith(b"dex\n"):
        raise SystemExit("Expected a standard DEX input")
    if len(inputs) + 4 + 3 > 128:
        raise SystemExit("Rootfs exceeds catalog capacity")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(args.output, "w", format=tarfile.USTAR_FORMAT) as archive:
        for name, data in sorted(inputs.items()):
            entry = tarfile.TarInfo(name)
            entry.size, entry.mode = len(data), 0o555
            archive.addfile(entry, io.BytesIO(data))
        for name in ("data", "data/dalvik-cache", "system/etc", "apex/com.android.tzdata"):
            entry = tarfile.TarInfo(name)
            entry.type, entry.mode = tarfile.DIRTYPE, 0o555
            archive.addfile(entry)
    if args.output.stat().st_size > 96 * 1024 * 1024:
        raise SystemExit("Rootfs exceeds the measured ART profile's 96 MiB capacity")
    jars = ["/apex/com.android.art/javalib/" + name + ".jar" for name in
            ("core-oj", "core-libart", "okhttp", "bouncycastle", "apache-xml")]
    jars.append("/apex/com.android.i18n/javalib/core-icu4j.jar")
    arguments = ["/apex/com.android.runtime/bin/linker64", "/apex/com.android.art/bin/dalvikvm64"]
    arguments += ["-help"] if args.mode == "help" else ["-Xint", "-Xusejit:false", "-Xms4m", "-Xmx16m", "-Xnoimage-dex2oat", "-Xbootclasspath:" + ":".join(jars), "-cp", "/system/framework/bndroid-probe.dex", "Main"]
    if args.mode == "memory":
        arguments = [arguments[0], "/system/bin/art-memory-probe"]
    environment = ["LANG=C", "ANDROID_ROOT=/system", "ANDROID_ART_ROOT=/apex/com.android.art",
                   "ANDROID_I18N_ROOT=/apex/com.android.i18n", "ANDROID_TZDATA_ROOT=/apex/com.android.tzdata",
                   "ANDROID_DATA=/data", "LD_LIBRARY_PATH=" + ":".join(provenance["dependency_search_paths"])]
    args.output.with_suffix(".launch").write_text("\n".join(["arg=" + s for s in arguments] + ["env=" + s for s in environment]) + "\n")
    receipt = {"source": provenance, "rootfs_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
               "dex_sha256": hashlib.sha256(args.dex.read_bytes()).hexdigest(), "mode": args.mode,
               "files": len(inputs) + 4, "bytes": args.output.stat().st_size,
               "namespace_mode": "diagnostic LD_LIBRARY_PATH; full Android linker namespaces pending"}
    args.output.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"ART_ROOTFS_BUILD_OK files={receipt['files']} bytes={receipt['bytes']} path={args.output}")


if __name__ == "__main__":
    main()
