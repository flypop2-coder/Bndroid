#!/usr/bin/env python3
"""Package explicit local Android runtime inputs; never downloads or scans devices."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--probe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    provenance = json.loads((args.runtime / "provenance.json").read_text())
    inputs = {}
    for name in ("linker64", "libc.so", "libm.so", "libdl.so"):
        data = (args.runtime / name).read_bytes()
        receipt = provenance["files"][name]
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Runtime input changed since extraction: {name}")
        if data[:6] != b"\x7fELF\x02\x01" or int.from_bytes(data[18:20], "little") != 183:
            raise SystemExit(f"Not an AArch64 ELF input: {name}")
        if name != "linker64":
            inputs[f"apex/com.android.runtime/lib64/bionic/{name}"] = data
    inputs["system/bin/dynamic-probe"] = (args.probe / "dynamic-probe").read_bytes()
    inputs["system/lib64/libbndroidplugin.so"] = (args.probe / "libbndroidplugin.so").read_bytes()
    inputs["system/etc/runtime-data"] = b"BNDR" + bytes(8187) + b"\x5a"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(args.output, "w", format=tarfile.USTAR_FORMAT) as archive:
        for name, data in sorted(inputs.items()):
            entry = tarfile.TarInfo(name)
            entry.size = len(data)
            entry.mode = 0o444 if name == "system/etc/runtime-data" else 0o555
            archive.addfile(entry, io.BytesIO(data))
        entry = tarfile.TarInfo("system/etc/runtime-link")
        entry.type = tarfile.SYMTYPE
        entry.linkname = "runtime-data"
        entry.mode = 0o777
        archive.addfile(entry)
    if args.output.stat().st_size > 8 * 1024 * 1024:
        raise SystemExit("Runtime rootfs exceeds the bounded 8 MiB input")
    receipt = {
        "source": provenance,
        "rootfs_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
        "files": {name: hashlib.sha256(data).hexdigest() for name, data in inputs.items()},
        "runtime_binaries_in_git": False,
    }
    args.output.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"BIONIC_ROOTFS_BUILD_OK files={len(inputs) + 1} path={args.output}")


if __name__ == "__main__":
    main()
