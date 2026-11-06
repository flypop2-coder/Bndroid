"""Encode public immutable boot properties for Bionic's pre-split reader.

ABI reference: platform_bionic/android16-release/libc/system_properties/
include/system_properties/{prop_area,prop_info}.h and prop_area.cpp.
This is a read-only bootstrap area, not a property-service implementation.
"""
import re
import struct


BOOT_PROPERTIES = {
    "ro.product.cpu.abilist": "arm64-v8a",
    "ro.product.cpu.abilist64": "arm64-v8a",
    "ro.product.cpu.abilist32": "",
    "ro.dalvik.vm.native.bridge": "0",
    "ro.product.name": "bndroid",
    "ro.product.manufacturer": "Bndroid",
}


def runtime_properties(source):
    """Version metadata describes the frozen Framework inputs, not app acceptance."""
    package = dict(line.split("=", 1) for line in source["package_properties"].splitlines() if "=" in line)
    if package.get("AndroidVersion.ApiLevel") != "36.1":
        raise ValueError("Bootstrap version properties require the frozen API 36.1 inputs")
    return {
        **BOOT_PROPERTIES,
        "ro.build.version.sdk": "36",
        "ro.build.version.sdk_full": "36.1",
        "ro.build.version.preview_sdk": "0",
        "ro.build.version.codename": "REL",
        "ro.build.version.all_codenames": "REL",
        "ro.build.version.release": "16",
        "ro.build.version.release_or_codename": "16",
    }


def encode_properties(properties):
    # Header is 128 bytes; offsets are relative to its data section. Reserve
    # the 20-byte root node and 92-byte dirty backup even though no writes occur.
    data = bytearray(112)
    trie = {}
    for name, value in properties.items():
        if not re.fullmatch(r"ro(?:\.[A-Za-z0-9_-]+)+", name):
            raise ValueError("Only canonical read-only properties are supported")
        raw = value.encode("utf-8")
        if len(raw) >= 92 or b"\0" in raw:
            raise ValueError("Bootstrap property values must fit the short-value ABI")
        branch = trie
        for token in name.split("."):
            branch = branch.setdefault(token, {})
        branch[None] = (name.encode("ascii"), raw)

    def allocate(payload):
        offset = len(data)
        aligned = (len(payload) + 3) & ~3
        if offset + aligned > 128 * 1024 - 128:
            raise ValueError("Bootstrap property area exceeds 128 KiB")
        data.extend(payload)
        data.extend(bytes(aligned - len(payload)))
        return offset

    def siblings(items):
        if not items:
            return 0
        middle = len(items) // 2
        token, branch = items[middle]
        raw = token.encode("ascii")
        offset = allocate(bytes(20) + raw + b"\0")
        prop = 0
        if None in branch:
            name, value = branch[None]
            prop = allocate(struct.pack("<I", len(value) << 24) + value.ljust(92, b"\0") + name + b"\0")
        children = siblings(ordered(branch))
        left, right = siblings(items[:middle]), siblings(items[middle + 1:])
        struct.pack_into("<IIIII", data, offset, len(raw), prop, left, right, children)
        return offset

    def ordered(branch):
        return sorted(((key, value) for key, value in branch.items() if key is not None),
                      key=lambda item: (len(item[0]), item[0]))

    root = siblings(ordered(trie))
    struct.pack_into("<I", data, 16, root)
    header = struct.pack("<IIII", len(data), 0, 0x504f5250, 0xfc6ed0ab) + bytes(112)
    return (header + data).ljust(128 * 1024, b"\0")
