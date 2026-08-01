#!/usr/bin/env python3
"""Build and verify the ABI-55 two-volume package partition image."""

from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_storage_image as base


MULTI_PACKAGE_CAPACITY = 2
MULTI_PACKAGE_PARTITION_SECTORS = 512 * MULTI_PACKAGE_CAPACITY
MULTI_PACKAGE_PARTITION_LAST_LBA = (
    base.PACKAGE_STORE_PARTITION_FIRST_LBA + MULTI_PACKAGE_PARTITION_SECTORS - 1
)
PACKAGE_ENTRY_INDEX = 3


class MultiPackageImageError(RuntimeError):
    pass


def _entry_range(entries_lba: int) -> slice:
    start = (
        entries_lba * base.SECTOR_BYTES
        + PACKAGE_ENTRY_INDEX * base.GPT_ENTRY_BYTES
    )
    return slice(start, start + base.GPT_ENTRY_BYTES)


def _reseal_header(image: bytearray, header_lba: int, entry_crc: int) -> None:
    offset = header_lba * base.SECTOR_BYTES
    struct.pack_into("<I", image, offset + 88, entry_crc)
    struct.pack_into("<I", image, offset + 16, 0)
    header_crc = base.crc32(bytes(image[offset : offset + base.GPT_HEADER_SIZE]))
    struct.pack_into("<I", image, offset + 16, header_crc)


def _set_partition_last_lba(image: bytearray, last_lba: int) -> None:
    primary_entries = base.image_sector_slice(
        image, base.PRIMARY_ENTRIES_LBA, base.GPT_ENTRY_SECTORS
    )
    backup_entries = base.image_sector_slice(
        image,
        base.PACKAGE_STORE_BACKUP_ENTRIES_LBA,
        base.GPT_ENTRY_SECTORS,
    )
    primary = bytearray(image[primary_entries])
    backup = bytearray(image[backup_entries])
    offset = PACKAGE_ENTRY_INDEX * base.GPT_ENTRY_BYTES + 40
    struct.pack_into("<Q", primary, offset, last_lba)
    struct.pack_into("<Q", backup, offset, last_lba)
    if primary != backup:
        raise MultiPackageImageError("primary and backup GPT entries diverged")
    image[primary_entries] = primary
    image[backup_entries] = backup
    entry_crc = base.crc32(primary)
    _reseal_header(image, base.PRIMARY_HEADER_LBA, entry_crc)
    _reseal_header(image, base.PACKAGE_STORE_BACKUP_HEADER_LBA, entry_crc)


def build_image() -> bytes:
    image = bytearray(base.build_image(include_package_store=True))
    _set_partition_last_lba(image, MULTI_PACKAGE_PARTITION_LAST_LBA)
    validate_image(bytes(image))
    return bytes(image)


def validate_image(image: bytes) -> None:
    if len(image) != base.PACKAGE_STORE_IMAGE_BYTES:
        raise MultiPackageImageError(
            f"image has {len(image)} bytes; expected {base.PACKAGE_STORE_IMAGE_BYTES}"
        )
    entry_tables = []
    for entries_lba, header_lba in (
        (base.PRIMARY_ENTRIES_LBA, base.PRIMARY_HEADER_LBA),
        (
            base.PACKAGE_STORE_BACKUP_ENTRIES_LBA,
            base.PACKAGE_STORE_BACKUP_HEADER_LBA,
        ),
    ):
        table_slice = base.image_sector_slice(
            image, entries_lba, base.GPT_ENTRY_SECTORS
        )
        table = image[table_slice]
        entry_tables.append(table)
        header_offset = header_lba * base.SECTOR_BYTES
        header = bytearray(
            image[header_offset : header_offset + base.GPT_HEADER_SIZE]
        )
        stored_header_crc = struct.unpack_from("<I", header, 16)[0]
        struct.pack_into("<I", header, 16, 0)
        if base.crc32(header) != stored_header_crc:
            raise MultiPackageImageError("GPT header CRC32 is invalid")
        stored_entry_crc = struct.unpack_from(
            "<I", image, header_offset + 88
        )[0]
        if base.crc32(table) != stored_entry_crc:
            raise MultiPackageImageError("GPT entry array CRC32 is invalid")
        entry = image[_entry_range(entries_lba)]
        first_lba, last_lba = struct.unpack_from("<QQ", entry, 32)
        if first_lba != base.PACKAGE_STORE_PARTITION_FIRST_LBA:
            raise MultiPackageImageError("package partition first LBA changed")
        if last_lba != MULTI_PACKAGE_PARTITION_LAST_LBA:
            raise MultiPackageImageError("package partition is not 1024 sectors")
        if entry[:16] != base.PACKAGE_STORE_TYPE_GUID_RAW:
            raise MultiPackageImageError("package partition type GUID changed")
        if entry[16:32] != base.PACKAGE_STORE_PARTITION_GUID.bytes_le:
            raise MultiPackageImageError("package partition unique GUID changed")
    if entry_tables[0] != entry_tables[1]:
        raise MultiPackageImageError("primary and backup GPT entry arrays differ")

    package_slice = base.image_sector_slice(
        image,
        base.PACKAGE_STORE_PARTITION_FIRST_LBA,
        MULTI_PACKAGE_PARTITION_SECTORS,
    )
    if any(image[package_slice]):
        raise MultiPackageImageError("multi-package partition is not virgin zero storage")

    # Reverse only the intended GPT extent change, then reuse the complete
    # package-store image verifier for every other byte and invariant.
    legacy = bytearray(image)
    _set_partition_last_lba(legacy, base.PACKAGE_STORE_PARTITION_LAST_LBA)
    base.validate_layout(bytes(legacy), include_package_store=True)


def marker(path: Path) -> str:
    image = path.read_bytes()
    validate_image(image)
    return (
        "MULTIPACKAGE_STORAGE_IMAGE_OK "
        f"bytes={len(image)} sectors={len(image) // base.SECTOR_BYTES} "
        f"sha256={hashlib.sha256(image).hexdigest()} "
        f"package_partition_lba={base.PACKAGE_STORE_PARTITION_FIRST_LBA}-"
        f"{MULTI_PACKAGE_PARTITION_LAST_LBA} "
        f"package_partition_sectors={MULTI_PACKAGE_PARTITION_SECTORS} "
        f"package_capacity={MULTI_PACKAGE_CAPACITY} "
        "legacy_first_volume=1 package_initial_zero=1"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    build_parser = subparsers.add_parser("build")
    build_parser.add_argument("path", type=Path)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("path", type=Path)
    marker_parser = subparsers.add_parser("marker")
    marker_parser.add_argument("path", type=Path)
    args = parser.parse_args()

    try:
        if args.command == "build":
            args.path.parent.mkdir(parents=True, exist_ok=True)
            args.path.write_bytes(build_image())
        elif args.command == "verify":
            validate_image(args.path.read_bytes())
        else:
            print(marker(args.path))
    except (OSError, base.FixtureError, MultiPackageImageError) as error:
        print(f"multipackage storage image error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
