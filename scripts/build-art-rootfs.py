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
    parser.add_argument("--mode", choices=("java", "memory", "help", "signals", "signal-bad-frame", "signal-bad-pc", "signal-bad-fp", "signal-default", "signal-guard"), default="java")
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
    # Use the source linker's real namespace policy. Old extraction receipts
    # must be refreshed: an unverified config or global LD_LIBRARY_PATH would
    # silently change which copies of APEX/platform libraries get loaded.
    for name, destination in (("ld.config.txt", "linkerconfig/ld.config.txt"),
                              ("ld.art.config.txt", "linkerconfig/com.android.art/ld.config.txt")):
        receipt = provenance.get("metadata_files", {}).get(name)
        if receipt is None or receipt["path"] != "/" + destination:
            raise SystemExit("Re-extract ART inputs with namespace metadata using --include-art")
        data = (args.runtime / name).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Linker configuration changed since extraction: {name}")
        inputs[destination] = data
    # System search paths reach the exact same Bionic files as the runtime APEX.
    # Resolve aliases inside the guest catalog, retaining canonical inode/path.
    aliases = {"system/lib64/" + name: "/apex/com.android.runtime/lib64/bionic/" + name
               for name in ("libc.so", "libm.so", "libdl.so", "libdl_android.so")}
    for name, destination in aliases.items():
        if destination.lstrip("/") not in inputs:
            raise SystemExit(f"Missing Bionic alias target: {destination}")
        if name in inputs:
            if inputs.pop(name) != inputs[destination.lstrip("/")]:
                raise SystemExit(f"Bionic alias differs from canonical input: {name}")
    inputs["system/framework/bndroid-probe.dex"] = args.dex.read_bytes()
    # Bndroid's initial public native-library policy lists actual supplied
    # libraries. It does not advertise missing Android graphics/media APIs.
    inputs["system/etc/public.libraries.txt"] = b"libc.so\nlibdl.so\nlibm.so\nliblog.so\nlibz.so\n"
    if args.mode == "memory":
        inputs["system/bin/art-memory-probe"] = (args.dex.parent / "memory-probe").read_bytes()
    if args.mode.startswith("signal"):
        inputs["system/bin/art-signals-probe"] = (args.dex.parent / "signals-probe").read_bytes()
    if not inputs["system/framework/bndroid-probe.dex"].startswith(b"dex\n"):
        raise SystemExit("Expected a standard DEX input")
    if len(inputs) + len(aliases) + 4 + 7 > 128:
        raise SystemExit("Rootfs exceeds catalog capacity")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(args.output, "w", format=tarfile.USTAR_FORMAT) as archive:
        for name, data in sorted(inputs.items()):
            entry = tarfile.TarInfo(name)
            entry.size, entry.mode = len(data), 0o555
            archive.addfile(entry, io.BytesIO(data))
        for name, destination in sorted(aliases.items()):
            entry = tarfile.TarInfo(name)
            entry.type, entry.mode, entry.linkname = tarfile.SYMTYPE, 0o777, destination
            archive.addfile(entry)
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
    if args.mode.startswith("signal"):
        arguments = [arguments[0], "/system/bin/art-signals-probe", args.mode]
    environment = ["LANG=C", "ANDROID_ROOT=/system", "ANDROID_ART_ROOT=/apex/com.android.art",
                   "ANDROID_I18N_ROOT=/apex/com.android.i18n", "ANDROID_TZDATA_ROOT=/apex/com.android.tzdata",
                   "ANDROID_DATA=/data"]
    args.output.with_suffix(".launch").write_text("\n".join(["arg=" + s for s in arguments] + ["env=" + s for s in environment]) + "\n")
    receipt = {"source": provenance, "rootfs_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
               "dex_sha256": hashlib.sha256(args.dex.read_bytes()).hexdigest(), "mode": args.mode,
               "files": len(inputs) + len(aliases) + 4, "bytes": args.output.stat().st_size,
               "aliases": aliases, "namespace_mode": "verified source global and ART linker configuration"}
    args.output.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"ART_ROOTFS_BUILD_OK files={receipt['files']} bytes={receipt['bytes']} path={args.output}")


if __name__ == "__main__":
    main()
