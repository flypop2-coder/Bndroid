#!/usr/bin/env python3
"""Assemble owned DEX/JNI inputs; SDK tools align/sign/verify the result."""
from pathlib import Path
import sys
import zipfile

out = Path(sys.argv[1])
with zipfile.ZipFile(out / "managed-resources.apk") as source:
    if source.testzip() is not None:
        raise SystemExit("Invalid aapt2 archive")
    entries = [(entry.filename, source.read(entry), entry.compress_type) for entry in source.infolist()]
if "AndroidManifest.xml" not in {name for name, _, _ in entries}:
    raise SystemExit("Missing compiled manifest")
entries += [("classes.dex", (out / "managed.dex").read_bytes(), zipfile.ZIP_STORED),
            ("lib/arm64-v8a/libbndroidjni.so", (out / "libbndroidjni.so").read_bytes(), zipfile.ZIP_STORED)]
with zipfile.ZipFile(out / "managed-unaligned.apk", "w") as archive:
    for name, data, method in entries:
        info = zipfile.ZipInfo(name, date_time=(2000, 1, 1, 0, 0, 0))
        info.compress_type = method
        info.external_attr = 0o100644 << 16
        archive.writestr(info, data)
