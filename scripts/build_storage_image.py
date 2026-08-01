#!/usr/bin/env python3
"""Build and verify deterministic Bndroid storage fixtures."""

from __future__ import annotations

import argparse
import binascii
import hashlib
import struct
import sys
import uuid
from pathlib import Path


SECTOR_BYTES = 512
SECTOR_COUNT = 16_384
IMAGE_BYTES = SECTOR_BYTES * SECTOR_COUNT

PACKAGE_STORE_SECTOR_COUNT = 32_768
PACKAGE_STORE_IMAGE_BYTES = SECTOR_BYTES * PACKAGE_STORE_SECTOR_COUNT

GPT_HEADER_SIZE = 92
GPT_ENTRY_COUNT = 128
GPT_ENTRY_BYTES = 128
GPT_ENTRY_SECTORS = GPT_ENTRY_COUNT * GPT_ENTRY_BYTES // SECTOR_BYTES
PRIMARY_HEADER_LBA = 1
PRIMARY_ENTRIES_LBA = 2
FIRST_USABLE_LBA = 34
BACKUP_ENTRIES_LBA = 16_351
BACKUP_HEADER_LBA = 16_383
LAST_USABLE_LBA = 16_350

PACKAGE_STORE_BACKUP_ENTRIES_LBA = 32_735
PACKAGE_STORE_BACKUP_HEADER_LBA = 32_767
PACKAGE_STORE_LAST_USABLE_LBA = 32_734

SYSTEM_PARTITION_FIRST_LBA = 2_048
SYSTEM_PARTITION_LAST_LBA = 16_350
SYSTEM_PARTITION_SECTORS = SYSTEM_PARTITION_LAST_LBA - SYSTEM_PARTITION_FIRST_LBA + 1
SYSTEM_PARTITION_NAME = "BNDROID_SYS"

DATA_PARTITION_FIRST_LBA = 64
DATA_PARTITION_LAST_LBA = 127
DATA_PARTITION_SECTORS = DATA_PARTITION_LAST_LBA - DATA_PARTITION_FIRST_LBA + 1
DATA_PARTITION_NAME = "BNDROID_DATA"

APPDATA_PARTITION_FIRST_LBA = 128
APPDATA_PARTITION_LAST_LBA = 2_047
APPDATA_PARTITION_SECTORS = (
    APPDATA_PARTITION_LAST_LBA - APPDATA_PARTITION_FIRST_LBA + 1
)
APPDATA_PARTITION_NAME = "BNDROID_APPDATA"

PACKAGE_STORE_PARTITION_FIRST_LBA = 16_384
PACKAGE_STORE_PARTITION_LAST_LBA = 16_895
PACKAGE_STORE_PARTITION_SECTORS = (
    PACKAGE_STORE_PARTITION_LAST_LBA - PACKAGE_STORE_PARTITION_FIRST_LBA + 1
)
PACKAGE_STORE_PARTITION_NAME = "BNDROID_PACKAGES"

# GPT stores the first three GUID fields little-endian. The requested raw byte
# sequence is the on-disk Microsoft Basic Data type GUID.
BASIC_DATA_TYPE_GUID_RAW = bytes.fromhex(
    "a2 a0 d0 eb e5 b9 33 44 87 c0 68 b6 b7 26 99 c7"
)
DISK_GUID = uuid.UUID("3b5d2c74-8f27-4d8a-9e3b-6f1c2a7d4501")
SYSTEM_PARTITION_GUID = uuid.UUID("a2c1146e-6b75-4a2d-8f40-71d3ea9c2301")
DATA_TYPE_GUID = uuid.UUID("b8f4d2a1-7c3e-4b91-a6d5-0f2e9c781355")
DATA_TYPE_GUID_RAW = DATA_TYPE_GUID.bytes_le
DATA_PARTITION_GUID = uuid.UUID("d25cf134-a879-4e5a-9364-67b2d8902501")
APPDATA_TYPE_GUID = uuid.UUID("b8f4d2a2-7c3e-4b91-a6d5-0f2e9c781355")
APPDATA_TYPE_GUID_RAW = APPDATA_TYPE_GUID.bytes_le
APPDATA_PARTITION_GUID = uuid.UUID("d25cf154-a879-4e5a-9364-67b2d8905401")
APPDATA_FORMAT_EPOCH = APPDATA_PARTITION_GUID.bytes_le
PACKAGE_STORE_TYPE_GUID = uuid.UUID("b8f4d2a3-7c3e-4b91-a6d5-0f2e9c781355")
PACKAGE_STORE_TYPE_GUID_RAW = PACKAGE_STORE_TYPE_GUID.bytes_le
PACKAGE_STORE_PARTITION_GUID = uuid.UUID("d25cf174-a879-4e5a-9364-67b2d8905801")

DATA_SUPERBLOCK_MAGIC = b"BNDRDAT1"
DATA_RECORD_MAGIC = b"BNDRREC1"
BOOT_STATE_MAGIC = b"BNDRBOOT"
DATA_FORMAT_VERSION = 1
DATA_SLOT_RELATIVE_LBAS = (1, 2)
DATA_FORMAT_EPOCH = DATA_PARTITION_GUID.bytes_le
DATA_RECORD_HEADER_BYTES = 80
DATA_RECORD_CRC_OFFSET = SECTOR_BYTES - 4
DATA_RECORD_COMMITTED = 1
DATA_RECORD_COMMIT_COOKIE = 0x434F_4D4D_4954_2131
BOOT_STATE_WITNESS = 0xB24D_25DA_7A5E_C001

FAT_BYTES_PER_SECTOR = 512
FAT_SECTORS_PER_CLUSTER = 1
FAT_RESERVED_SECTORS = 1
FAT_COPY_COUNT = 2
FAT_ROOT_ENTRIES = 64
FAT_SECTORS = 56
FAT_MEDIA = 0xF8
FAT_ROOT_SECTORS = FAT_ROOT_ENTRIES * 32 // FAT_BYTES_PER_SECTOR
FAT_FIRST_LBA = SYSTEM_PARTITION_FIRST_LBA
FAT1_LBA = FAT_FIRST_LBA + FAT_RESERVED_SECTORS
FAT2_LBA = FAT1_LBA + FAT_SECTORS
FAT_ROOT_LBA = FAT2_LBA + FAT_SECTORS
FAT_DATA_LBA = FAT_ROOT_LBA + FAT_ROOT_SECTORS

HELLO_CLUSTER = 2
SYSTEM_CLUSTER = 3
BUILD_CLUSTER = 4
HELLO_CONTENT = b"Bndroid M23 FAT16 is alive.\n"
BUILD_CONTENT = b"build=m23\nfilesystem=fat16\nvfs=readonly\n"

# Fixed fingerprints of the canonical image and its externally consumed data.
EXPECTED_SHA256 = "6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838"
EXPECTED_APPDATA_SHA256 = "577a9c422b0edfb05f18f02120c0632d9f64c632664465106fb9876b08ffc1cf"
EXPECTED_PACKAGE_STORE_SHA256 = (
    "b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801"
)
EXPECTED_LBA0_FNV1A64 = 0xBEBD264B8C14CD72
EXPECTED_LBA1_FNV1A64 = 0x8294DE399174037C
EXPECTED_APPDATA_LBA1_FNV1A64 = 0xB7E1FFE0836BF29F
EXPECTED_PACKAGE_STORE_LBA0_FNV1A64 = 0x806A383295FA5D32
EXPECTED_PACKAGE_STORE_LBA1_FNV1A64 = 0xDC1C9C6D8BCCF79A
EXPECTED_APPDATA_PRIMARY_HEADER_CRC32 = 0x61A24D8E
EXPECTED_APPDATA_BACKUP_HEADER_CRC32 = 0xD59758BB
EXPECTED_APPDATA_PARTITION_ENTRY_CRC32 = 0xE8BA27AF
EXPECTED_PACKAGE_STORE_PRIMARY_HEADER_CRC32 = 0xCF2787AD
EXPECTED_PACKAGE_STORE_BACKUP_HEADER_CRC32 = 0x36D1D613
EXPECTED_PACKAGE_STORE_PARTITION_ENTRY_CRC32 = 0x13648066
EXPECTED_HELLO_FNV1A64 = 0xDD2F71342016EEDE
EXPECTED_BUILD_FNV1A64 = 0xE57CE4CE9F1B4EC0

FNV1A64_OFFSET = 0xCBF29CE484222325
FNV1A64_PRIME = 0x100000001B3


class FixtureError(RuntimeError):
    pass


def fnv1a64(data: bytes) -> int:
    value = FNV1A64_OFFSET
    for byte in data:
        value ^= byte
        value = (value * FNV1A64_PRIME) & 0xFFFF_FFFF_FFFF_FFFF
    return value


def sector_slice(
    lba: int, count: int = 1, *, sector_count: int = SECTOR_COUNT
) -> slice:
    if lba < 0 or count < 0 or lba + count > sector_count:
        raise FixtureError(f"sector range {lba}+{count} is outside the image")
    start = lba * SECTOR_BYTES
    return slice(start, start + count * SECTOR_BYTES)


def image_sector_slice(image: bytes | bytearray, lba: int, count: int = 1) -> slice:
    if len(image) % SECTOR_BYTES != 0:
        raise FixtureError("image size is not a whole number of sectors")
    return sector_slice(lba, count, sector_count=len(image) // SECTOR_BYTES)


def crc32(data: bytes) -> int:
    return binascii.crc32(data) & 0xFFFF_FFFF


def build_protective_mbr(*, sector_count: int = SECTOR_COUNT) -> bytes:
    sector = bytearray(SECTOR_BYTES)
    # One protective partition spanning every LBA after the MBR.
    struct.pack_into(
        "<B3sB3sII",
        sector,
        446,
        0,
        b"\x00\x02\x00",
        0xEE,
        b"\xff\xff\xff",
        1,
        sector_count - 1,
    )
    sector[510:512] = b"\x55\xaa"
    return bytes(sector)


def build_partition_entries(
    *, include_appdata: bool = False, include_package_store: bool = False
) -> bytes:
    include_appdata = include_appdata or include_package_store
    entries = bytearray(GPT_ENTRY_COUNT * GPT_ENTRY_BYTES)
    put_partition_entry(
        entries,
        0,
        BASIC_DATA_TYPE_GUID_RAW,
        SYSTEM_PARTITION_GUID,
        SYSTEM_PARTITION_FIRST_LBA,
        SYSTEM_PARTITION_LAST_LBA,
        SYSTEM_PARTITION_NAME,
    )
    put_partition_entry(
        entries,
        1,
        DATA_TYPE_GUID_RAW,
        DATA_PARTITION_GUID,
        DATA_PARTITION_FIRST_LBA,
        DATA_PARTITION_LAST_LBA,
        DATA_PARTITION_NAME,
    )
    if include_appdata:
        put_partition_entry(
            entries,
            2,
            APPDATA_TYPE_GUID_RAW,
            APPDATA_PARTITION_GUID,
            APPDATA_PARTITION_FIRST_LBA,
            APPDATA_PARTITION_LAST_LBA,
            APPDATA_PARTITION_NAME,
        )
    if include_package_store:
        put_partition_entry(
            entries,
            3,
            PACKAGE_STORE_TYPE_GUID_RAW,
            PACKAGE_STORE_PARTITION_GUID,
            PACKAGE_STORE_PARTITION_FIRST_LBA,
            PACKAGE_STORE_PARTITION_LAST_LBA,
            PACKAGE_STORE_PARTITION_NAME,
        )
    return bytes(entries)


def put_partition_entry(
    entries: bytearray,
    index: int,
    type_guid_raw: bytes,
    unique_guid: uuid.UUID,
    first_lba: int,
    last_lba: int,
    name: str,
) -> None:
    start = index * GPT_ENTRY_BYTES
    entry = memoryview(entries)[start : start + GPT_ENTRY_BYTES]
    entry[0:16] = type_guid_raw
    entry[16:32] = unique_guid.bytes_le
    struct.pack_into("<QQQ", entry, 32, first_lba, last_lba, 0)
    encoded_name = name.encode("utf-16le")
    if len(encoded_name) > 72:
        raise FixtureError("GPT partition name exceeds 36 UTF-16 code units")
    entry[56 : 56 + len(encoded_name)] = encoded_name


def build_gpt_header(
    *,
    current_lba: int,
    backup_lba: int,
    entries_lba: int,
    entries_crc: int,
    last_usable_lba: int = LAST_USABLE_LBA,
) -> bytes:
    sector = bytearray(SECTOR_BYTES)
    struct.pack_into(
        "<8sIIIIQQQQ16sQIII",
        sector,
        0,
        b"EFI PART",
        0x0001_0000,
        GPT_HEADER_SIZE,
        0,
        0,
        current_lba,
        backup_lba,
        FIRST_USABLE_LBA,
        last_usable_lba,
        DISK_GUID.bytes_le,
        entries_lba,
        GPT_ENTRY_COUNT,
        GPT_ENTRY_BYTES,
        entries_crc,
    )
    struct.pack_into("<I", sector, 16, crc32(bytes(sector[:GPT_HEADER_SIZE])))
    return bytes(sector)


def seal_data_sector(sector: bytearray) -> bytes:
    struct.pack_into("<I", sector, DATA_RECORD_CRC_OFFSET, crc32(sector[:DATA_RECORD_CRC_OFFSET]))
    return bytes(sector)


def build_data_superblock() -> bytes:
    sector = bytearray(SECTOR_BYTES)
    sector[0:8] = DATA_SUPERBLOCK_MAGIC
    struct.pack_into(
        "<IIIIQQQ",
        sector,
        8,
        DATA_FORMAT_VERSION,
        SECTOR_BYTES,
        len(DATA_SLOT_RELATIVE_LBAS),
        SECTOR_BYTES,
        DATA_SLOT_RELATIVE_LBAS[0],
        DATA_SLOT_RELATIVE_LBAS[1],
        DATA_PARTITION_SECTORS,
    )
    sector[48:64] = DATA_FORMAT_EPOCH
    return seal_data_sector(sector)


def build_boot_state(boot_count: int) -> bytes:
    payload = bytearray(32)
    payload[0:8] = BOOT_STATE_MAGIC
    struct.pack_into("<I", payload, 8, DATA_FORMAT_VERSION)
    struct.pack_into("<Q", payload, 16, boot_count)
    struct.pack_into("<Q", payload, 24, boot_count ^ BOOT_STATE_WITNESS)
    return bytes(payload)


def build_data_record(generation: int, payload: bytes) -> bytes:
    if len(payload) > DATA_RECORD_CRC_OFFSET - DATA_RECORD_HEADER_BYTES:
        raise FixtureError("persistent-data payload exceeds one record sector")
    sector = bytearray(SECTOR_BYTES)
    sector[0:8] = DATA_RECORD_MAGIC
    struct.pack_into("<I", sector, 8, DATA_FORMAT_VERSION)
    struct.pack_into("<I", sector, 12, DATA_RECORD_HEADER_BYTES)
    struct.pack_into("<Q", sector, 16, generation)
    struct.pack_into("<I", sector, 24, len(payload))
    struct.pack_into("<I", sector, 28, DATA_RECORD_COMMITTED)
    struct.pack_into("<I", sector, 32, crc32(payload))
    struct.pack_into("<Q", sector, 40, fnv1a64(payload))
    struct.pack_into("<Q", sector, 48, generation ^ 0xFFFF_FFFF_FFFF_FFFF)
    struct.pack_into("<Q", sector, 56, DATA_RECORD_COMMIT_COOKIE)
    sector[64:80] = DATA_FORMAT_EPOCH
    sector[DATA_RECORD_HEADER_BYTES : DATA_RECORD_HEADER_BYTES + len(payload)] = payload
    return seal_data_sector(sector)


def build_fat_boot_sector() -> bytes:
    sector = bytearray(SECTOR_BYTES)
    sector[0:3] = b"\xeb\x3c\x90"
    sector[3:11] = b"BNDRM23 "
    struct.pack_into("<H", sector, 11, FAT_BYTES_PER_SECTOR)
    sector[13] = FAT_SECTORS_PER_CLUSTER
    struct.pack_into("<H", sector, 14, FAT_RESERVED_SECTORS)
    sector[16] = FAT_COPY_COUNT
    struct.pack_into("<H", sector, 17, FAT_ROOT_ENTRIES)
    struct.pack_into("<H", sector, 19, SYSTEM_PARTITION_SECTORS)
    sector[21] = FAT_MEDIA
    struct.pack_into("<H", sector, 22, FAT_SECTORS)
    struct.pack_into("<H", sector, 24, 63)
    struct.pack_into("<H", sector, 26, 255)
    struct.pack_into("<I", sector, 28, SYSTEM_PARTITION_FIRST_LBA)
    struct.pack_into("<I", sector, 32, 0)
    sector[36] = 0x80
    sector[37] = 0
    sector[38] = 0x29
    struct.pack_into("<I", sector, 39, 0x2307_2026)
    sector[43:54] = b"BNDROID_SYS"
    sector[54:62] = b"FAT16   "
    sector[510:512] = b"\x55\xaa"
    return bytes(sector)


def build_fat() -> bytes:
    table = bytearray(FAT_SECTORS * SECTOR_BYTES)
    for cluster, value in (
        (0, 0xFFF8),
        (1, 0xFFFF),
        (HELLO_CLUSTER, 0xFFFF),
        (SYSTEM_CLUSTER, 0xFFFF),
        (BUILD_CLUSTER, 0xFFFF),
    ):
        struct.pack_into("<H", table, cluster * 2, value)
    return bytes(table)


def directory_entry(name: bytes, attributes: int, cluster: int, size: int) -> bytes:
    if len(name) != 11:
        raise FixtureError("FAT short names must be exactly 11 bytes")
    entry = bytearray(32)
    entry[0:11] = name
    entry[11] = attributes
    struct.pack_into("<H", entry, 26, cluster)
    struct.pack_into("<I", entry, 28, size)
    return bytes(entry)


def build_root_directory() -> bytes:
    root = bytearray(FAT_ROOT_SECTORS * SECTOR_BYTES)
    root[0:32] = directory_entry(
        b"HELLO   TXT", 0x20, HELLO_CLUSTER, len(HELLO_CONTENT)
    )
    root[32:64] = directory_entry(b"SYSTEM     ", 0x10, SYSTEM_CLUSTER, 0)
    return bytes(root)


def build_system_directory() -> bytes:
    directory = bytearray(SECTOR_BYTES)
    directory[0:32] = directory_entry(b".          ", 0x10, SYSTEM_CLUSTER, 0)
    directory[32:64] = directory_entry(b"..         ", 0x10, 0, 0)
    directory[64:96] = directory_entry(
        b"BUILD   TXT", 0x20, BUILD_CLUSTER, len(BUILD_CONTENT)
    )
    return bytes(directory)


def cluster_lba(cluster: int) -> int:
    if cluster < 2:
        raise FixtureError("FAT data clusters begin at two")
    return FAT_DATA_LBA + (cluster - 2) * FAT_SECTORS_PER_CLUSTER


def put(image: bytearray, lba: int, data: bytes) -> None:
    if len(data) % SECTOR_BYTES != 0:
        raise FixtureError("image components must occupy whole sectors")
    destination = image_sector_slice(image, lba, len(data) // SECTOR_BYTES)
    image[destination] = data


def build_image(
    *, include_appdata: bool = False, include_package_store: bool = False
) -> bytes:
    include_appdata = include_appdata or include_package_store
    if include_package_store:
        sector_count = PACKAGE_STORE_SECTOR_COUNT
        image_bytes = PACKAGE_STORE_IMAGE_BYTES
        backup_entries_lba = PACKAGE_STORE_BACKUP_ENTRIES_LBA
        backup_header_lba = PACKAGE_STORE_BACKUP_HEADER_LBA
        last_usable_lba = PACKAGE_STORE_LAST_USABLE_LBA
    else:
        sector_count = SECTOR_COUNT
        image_bytes = IMAGE_BYTES
        backup_entries_lba = BACKUP_ENTRIES_LBA
        backup_header_lba = BACKUP_HEADER_LBA
        last_usable_lba = LAST_USABLE_LBA

    image = bytearray(image_bytes)
    entries = build_partition_entries(
        include_appdata=include_appdata,
        include_package_store=include_package_store,
    )
    entries_crc = crc32(entries)

    put(image, 0, build_protective_mbr(sector_count=sector_count))
    put(
        image,
        PRIMARY_HEADER_LBA,
        build_gpt_header(
            current_lba=PRIMARY_HEADER_LBA,
            backup_lba=backup_header_lba,
            entries_lba=PRIMARY_ENTRIES_LBA,
            entries_crc=entries_crc,
            last_usable_lba=last_usable_lba,
        ),
    )
    put(image, PRIMARY_ENTRIES_LBA, entries)
    put(image, backup_entries_lba, entries)
    put(
        image,
        backup_header_lba,
        build_gpt_header(
            current_lba=backup_header_lba,
            backup_lba=PRIMARY_HEADER_LBA,
            entries_lba=backup_entries_lba,
            entries_crc=entries_crc,
            last_usable_lba=last_usable_lba,
        ),
    )

    put(image, DATA_PARTITION_FIRST_LBA, build_data_superblock())
    put(
        image,
        DATA_PARTITION_FIRST_LBA + DATA_SLOT_RELATIVE_LBAS[0],
        build_data_record(0, build_boot_state(0)),
    )

    put(image, FAT_FIRST_LBA, build_fat_boot_sector())
    fat = build_fat()
    put(image, FAT1_LBA, fat)
    put(image, FAT2_LBA, fat)
    put(image, FAT_ROOT_LBA, build_root_directory())

    hello_sector = bytearray(SECTOR_BYTES)
    hello_sector[: len(HELLO_CONTENT)] = HELLO_CONTENT
    put(image, cluster_lba(HELLO_CLUSTER), bytes(hello_sector))
    put(image, cluster_lba(SYSTEM_CLUSTER), build_system_directory())
    build_sector = bytearray(SECTOR_BYTES)
    build_sector[: len(BUILD_CONTENT)] = BUILD_CONTENT
    put(image, cluster_lba(BUILD_CLUSTER), bytes(build_sector))

    result = bytes(image)
    validate_layout(
        result,
        include_appdata=include_appdata,
        include_package_store=include_package_store,
    )
    return result


def checked_header(
    image: bytes,
    lba: int,
    *,
    current_lba: int,
    backup_lba: int,
    first_usable_lba: int,
    last_usable_lba: int,
    entries_lba: int,
) -> tuple[int, int, int]:
    sector = bytearray(image[image_sector_slice(image, lba)])
    if sector[:8] != b"EFI PART":
        raise FixtureError(f"LBA {lba} has no GPT signature")
    revision = struct.unpack_from("<I", sector, 8)[0]
    header_size = struct.unpack_from("<I", sector, 12)[0]
    reserved = struct.unpack_from("<I", sector, 20)[0]
    if (
        revision != 0x0001_0000
        or header_size != GPT_HEADER_SIZE
        or reserved != 0
        or any(sector[GPT_HEADER_SIZE:])
    ):
        raise FixtureError(f"LBA {lba} has invalid GPT header metadata")
    recorded_crc = struct.unpack_from("<I", sector, 16)[0]
    struct.pack_into("<I", sector, 16, 0)
    if crc32(bytes(sector[:header_size])) != recorded_crc:
        raise FixtureError(f"LBA {lba} has an invalid GPT header CRC32")
    actual_geometry = struct.unpack_from("<QQQQ", sector, 24)
    if actual_geometry != (
        current_lba,
        backup_lba,
        first_usable_lba,
        last_usable_lba,
    ):
        raise FixtureError(f"LBA {lba} has invalid GPT disk bounds")
    if bytes(sector[56:72]) != DISK_GUID.bytes_le:
        raise FixtureError(f"LBA {lba} has an invalid GPT disk GUID")
    actual_entries_lba = struct.unpack_from("<Q", sector, 72)[0]
    entry_count, entry_bytes = struct.unpack_from("<II", sector, 80)
    if (
        actual_entries_lba != entries_lba
        or entry_count != GPT_ENTRY_COUNT
        or entry_bytes != GPT_ENTRY_BYTES
    ):
        raise FixtureError(f"LBA {lba} has invalid GPT entry-array metadata")
    entries_crc = struct.unpack_from("<I", sector, 88)[0]
    return actual_entries_lba, entries_crc, recorded_crc


def validate_layout(
    image: bytes,
    *,
    include_appdata: bool = False,
    include_package_store: bool = False,
) -> None:
    include_appdata = include_appdata or include_package_store
    if include_package_store:
        sector_count = PACKAGE_STORE_SECTOR_COUNT
        image_bytes = PACKAGE_STORE_IMAGE_BYTES
        expected_backup_entries_lba = PACKAGE_STORE_BACKUP_ENTRIES_LBA
        backup_header_lba = PACKAGE_STORE_BACKUP_HEADER_LBA
        last_usable_lba = PACKAGE_STORE_LAST_USABLE_LBA
    else:
        sector_count = SECTOR_COUNT
        image_bytes = IMAGE_BYTES
        expected_backup_entries_lba = BACKUP_ENTRIES_LBA
        backup_header_lba = BACKUP_HEADER_LBA
        last_usable_lba = LAST_USABLE_LBA

    if len(image) != image_bytes:
        raise FixtureError(f"image has {len(image)} bytes; expected {image_bytes}")

    mbr = image[image_sector_slice(image, 0)]
    if mbr[510:512] != b"\x55\xaa" or mbr[450] != 0xEE:
        raise FixtureError("protective MBR signature or partition type is invalid")
    if (
        any(mbr[:446])
        or mbr[446:454] != b"\x00\x00\x02\x00\xee\xff\xff\xff"
        or any(mbr[462:510])
        or struct.unpack_from("<II", mbr, 454) != (1, sector_count - 1)
    ):
        raise FixtureError("protective MBR does not cover the disk")

    primary_entries_lba, primary_entries_crc, primary_header_crc = checked_header(
        image,
        PRIMARY_HEADER_LBA,
        current_lba=PRIMARY_HEADER_LBA,
        backup_lba=backup_header_lba,
        first_usable_lba=FIRST_USABLE_LBA,
        last_usable_lba=last_usable_lba,
        entries_lba=PRIMARY_ENTRIES_LBA,
    )
    backup_entries_lba, backup_entries_crc, backup_header_crc = checked_header(
        image,
        backup_header_lba,
        current_lba=backup_header_lba,
        backup_lba=PRIMARY_HEADER_LBA,
        first_usable_lba=FIRST_USABLE_LBA,
        last_usable_lba=last_usable_lba,
        entries_lba=expected_backup_entries_lba,
    )
    if (primary_entries_lba, backup_entries_lba) != (
        PRIMARY_ENTRIES_LBA,
        expected_backup_entries_lba,
    ):
        raise FixtureError("GPT headers point to unexpected entry arrays")
    primary_entries = image[
        image_sector_slice(image, PRIMARY_ENTRIES_LBA, GPT_ENTRY_SECTORS)
    ]
    backup_entries = image[
        image_sector_slice(image, backup_entries_lba, GPT_ENTRY_SECTORS)
    ]
    if primary_entries != backup_entries:
        raise FixtureError("primary and backup GPT entry arrays differ")
    if crc32(primary_entries) != primary_entries_crc or primary_entries_crc != backup_entries_crc:
        raise FixtureError("GPT partition-entry CRC32 is invalid")

    partition_names = [SYSTEM_PARTITION_NAME, DATA_PARTITION_NAME]
    if include_appdata:
        partition_names.append(APPDATA_PARTITION_NAME)
    if include_package_store:
        partition_names.append(PACKAGE_STORE_PARTITION_NAME)
    partition_bounds = []
    for index, name in enumerate(partition_names):
        entry = primary_entries[
            index * GPT_ENTRY_BYTES : (index + 1) * GPT_ENTRY_BYTES
        ]
        first_lba, last_lba, attributes = struct.unpack_from("<QQQ", entry, 32)
        if attributes != 0:
            raise FixtureError(f"GPT {name} partition attributes are invalid")
        if (
            first_lba < FIRST_USABLE_LBA
            or first_lba > last_lba
            or last_lba > last_usable_lba
        ):
            raise FixtureError(f"GPT {name} partition is outside usable disk bounds")
        partition_bounds.append((name, first_lba, last_lba))
    partition_bounds.sort(key=lambda partition: partition[1])
    for left, right in zip(partition_bounds, partition_bounds[1:]):
        if left[2] >= right[1]:
            raise FixtureError(
                f"GPT partitions {left[0]} and {right[0]} overlap"
            )

    system_entry = primary_entries[:GPT_ENTRY_BYTES]
    data_entry = primary_entries[GPT_ENTRY_BYTES : 2 * GPT_ENTRY_BYTES]
    appdata_entry = primary_entries[2 * GPT_ENTRY_BYTES : 3 * GPT_ENTRY_BYTES]
    package_store_entry = primary_entries[
        3 * GPT_ENTRY_BYTES : 4 * GPT_ENTRY_BYTES
    ]
    if system_entry[:16] != BASIC_DATA_TYPE_GUID_RAW:
        raise FixtureError("system partition type GUID raw bytes are invalid")
    if system_entry[16:32] != SYSTEM_PARTITION_GUID.bytes_le:
        raise FixtureError("system partition unique GUID is invalid")
    if struct.unpack_from("<QQ", system_entry, 32) != (
        SYSTEM_PARTITION_FIRST_LBA,
        SYSTEM_PARTITION_LAST_LBA,
    ):
        raise FixtureError("GPT system partition bounds are invalid")
    system_name = system_entry[56:128].decode("utf-16le").rstrip("\0")
    if system_name != SYSTEM_PARTITION_NAME:
        raise FixtureError("GPT system partition name is invalid")
    if data_entry[:16] != DATA_TYPE_GUID_RAW:
        raise FixtureError("data partition type GUID raw bytes are invalid")
    if data_entry[16:32] != DATA_PARTITION_GUID.bytes_le:
        raise FixtureError("data partition unique GUID is invalid")
    if struct.unpack_from("<QQ", data_entry, 32) != (
        DATA_PARTITION_FIRST_LBA,
        DATA_PARTITION_LAST_LBA,
    ):
        raise FixtureError("GPT data partition bounds are invalid")
    data_name = data_entry[56:128].decode("utf-16le").rstrip("\0")
    if data_name != DATA_PARTITION_NAME:
        raise FixtureError("GPT data partition name is invalid")
    if include_appdata:
        if appdata_entry[:16] != APPDATA_TYPE_GUID_RAW:
            raise FixtureError("appdata partition type GUID raw bytes are invalid")
        if appdata_entry[16:32] != APPDATA_PARTITION_GUID.bytes_le:
            raise FixtureError("appdata partition unique GUID is invalid")
        if struct.unpack_from("<QQ", appdata_entry, 32) != (
            APPDATA_PARTITION_FIRST_LBA,
            APPDATA_PARTITION_LAST_LBA,
        ):
            raise FixtureError("GPT appdata partition bounds are invalid")
        appdata_name = appdata_entry[56:128].decode("utf-16le").rstrip("\0")
        if appdata_name != APPDATA_PARTITION_NAME:
            raise FixtureError("GPT appdata partition name is invalid")
        if not (
            DATA_PARTITION_LAST_LBA < APPDATA_PARTITION_FIRST_LBA
            and APPDATA_PARTITION_LAST_LBA < SYSTEM_PARTITION_FIRST_LBA
            and APPDATA_PARTITION_SECTORS == 1_920
        ):
            raise FixtureError("GPT appdata partition overlaps an adjacent partition")
        if any(
            image[
                image_sector_slice(
                    image,
                    APPDATA_PARTITION_FIRST_LBA,
                    APPDATA_PARTITION_SECTORS,
                )
            ]
        ):
            raise FixtureError("initial appdata partition is not empty")
        if include_package_store:
            if package_store_entry[:16] != PACKAGE_STORE_TYPE_GUID_RAW:
                raise FixtureError(
                    "package-store partition type GUID raw bytes are invalid"
                )
            if package_store_entry[16:32] != PACKAGE_STORE_PARTITION_GUID.bytes_le:
                raise FixtureError("package-store partition unique GUID is invalid")
            if struct.unpack_from("<QQ", package_store_entry, 32) != (
                PACKAGE_STORE_PARTITION_FIRST_LBA,
                PACKAGE_STORE_PARTITION_LAST_LBA,
            ):
                raise FixtureError("GPT package-store partition bounds are invalid")
            package_store_name = package_store_entry[56:128].decode(
                "utf-16le"
            ).rstrip("\0")
            if package_store_name != PACKAGE_STORE_PARTITION_NAME:
                raise FixtureError("GPT package-store partition name is invalid")
            if PACKAGE_STORE_PARTITION_SECTORS != 512:
                raise FixtureError("GPT package-store partition size is invalid")
            if any(
                image[
                    image_sector_slice(
                        image,
                        PACKAGE_STORE_PARTITION_FIRST_LBA,
                        PACKAGE_STORE_PARTITION_SECTORS,
                    )
                ]
            ):
                raise FixtureError("initial package-store partition is not empty")
            first_unused_entry = 4
        else:
            first_unused_entry = 3
    else:
        first_unused_entry = 2
    if any(primary_entries[first_unused_entry * GPT_ENTRY_BYTES :]):
        raise FixtureError("unused GPT partition entries are nonzero")
    if include_package_store:
        if (
            primary_header_crc,
            backup_header_crc,
            primary_entries_crc,
        ) != (
            EXPECTED_PACKAGE_STORE_PRIMARY_HEADER_CRC32,
            EXPECTED_PACKAGE_STORE_BACKUP_HEADER_CRC32,
            EXPECTED_PACKAGE_STORE_PARTITION_ENTRY_CRC32,
        ):
            raise FixtureError(
                "package-store GPT CRC32 values differ from the fixed contract"
            )
    elif include_appdata and (
        primary_header_crc,
        backup_header_crc,
        primary_entries_crc,
    ) != (
        EXPECTED_APPDATA_PRIMARY_HEADER_CRC32,
        EXPECTED_APPDATA_BACKUP_HEADER_CRC32,
        EXPECTED_APPDATA_PARTITION_ENTRY_CRC32,
    ):
        raise FixtureError("M54 GPT CRC32 values differ from the fixed contract")

    superblock = image[sector_slice(DATA_PARTITION_FIRST_LBA)]
    initial_record = image[
        sector_slice(DATA_PARTITION_FIRST_LBA + DATA_SLOT_RELATIVE_LBAS[0])
    ]
    empty_record = image[
        sector_slice(DATA_PARTITION_FIRST_LBA + DATA_SLOT_RELATIVE_LBAS[1])
    ]
    if superblock != build_data_superblock():
        raise FixtureError("persistent-data superblock is invalid")
    if initial_record != build_data_record(0, build_boot_state(0)):
        raise FixtureError("initial persistent-data record is invalid")
    if any(empty_record):
        raise FixtureError("inactive persistent-data record is not empty")
    unused_data = image[
        sector_slice(
            DATA_PARTITION_FIRST_LBA + 3,
            DATA_PARTITION_SECTORS - 3,
        )
    ]
    if any(unused_data):
        raise FixtureError("unused persistent-data partition sectors are nonzero")

    boot = image[sector_slice(FAT_FIRST_LBA)]
    expected_bpb = (
        struct.unpack_from("<H", boot, 11)[0],
        boot[13],
        struct.unpack_from("<H", boot, 14)[0],
        boot[16],
        struct.unpack_from("<H", boot, 17)[0],
        struct.unpack_from("<H", boot, 19)[0],
        boot[21],
        struct.unpack_from("<H", boot, 22)[0],
        struct.unpack_from("<I", boot, 28)[0],
    )
    if expected_bpb != (
        512,
        1,
        1,
        2,
        64,
        SYSTEM_PARTITION_SECTORS,
        0xF8,
        56,
        SYSTEM_PARTITION_FIRST_LBA,
    ):
        raise FixtureError("FAT16 BPB geometry is invalid")
    if boot[54:62] != b"FAT16   " or boot[510:512] != b"\x55\xaa":
        raise FixtureError("FAT16 type label or boot signature is invalid")

    fat1 = image[sector_slice(FAT1_LBA, FAT_SECTORS)]
    fat2 = image[sector_slice(FAT2_LBA, FAT_SECTORS)]
    if fat1 != fat2:
        raise FixtureError("FAT copies differ")
    if tuple(struct.unpack_from("<H", fat1, cluster * 2)[0] for cluster in range(5)) != (
        0xFFF8,
        0xFFFF,
        0xFFFF,
        0xFFFF,
        0xFFFF,
    ):
        raise FixtureError("FAT16 cluster chains are invalid")

    root = image[sector_slice(FAT_ROOT_LBA, FAT_ROOT_SECTORS)]
    if root[0:11] != b"HELLO   TXT" or root[32:43] != b"SYSTEM     ":
        raise FixtureError("FAT16 root entries are invalid")
    system = image[sector_slice(cluster_lba(SYSTEM_CLUSTER))]
    if system[0:11] != b".          " or system[32:43] != b"..         ":
        raise FixtureError("SYSTEM dot entries are invalid")
    if system[64:75] != b"BUILD   TXT":
        raise FixtureError("SYSTEM/BUILD.TXT entry is invalid")

    hello = image[sector_slice(cluster_lba(HELLO_CLUSTER))]
    build = image[sector_slice(cluster_lba(BUILD_CLUSTER))]
    if hello[: len(HELLO_CONTENT)] != HELLO_CONTENT or any(hello[len(HELLO_CONTENT) :]):
        raise FixtureError("HELLO.TXT content or cluster padding is invalid")
    if build[: len(BUILD_CONTENT)] != BUILD_CONTENT or any(build[len(BUILD_CONTENT) :]):
        raise FixtureError("SYSTEM/BUILD.TXT content or cluster padding is invalid")


def image_hashes(image: bytes) -> tuple[str, int, int, int, int]:
    return (
        hashlib.sha256(image).hexdigest(),
        fnv1a64(image[image_sector_slice(image, 0)]),
        fnv1a64(image[image_sector_slice(image, 1)]),
        fnv1a64(HELLO_CONTENT),
        fnv1a64(BUILD_CONTENT),
    )


def validate_fixed_hashes(
    image: bytes,
    *,
    include_appdata: bool = False,
    include_package_store: bool = False,
) -> None:
    include_appdata = include_appdata or include_package_store
    actual = image_hashes(image)
    if include_package_store:
        expected = (
            EXPECTED_PACKAGE_STORE_SHA256,
            EXPECTED_PACKAGE_STORE_LBA0_FNV1A64,
            EXPECTED_PACKAGE_STORE_LBA1_FNV1A64,
            EXPECTED_HELLO_FNV1A64,
            EXPECTED_BUILD_FNV1A64,
        )
    else:
        expected = (
            EXPECTED_APPDATA_SHA256 if include_appdata else EXPECTED_SHA256,
            EXPECTED_LBA0_FNV1A64,
            (
                EXPECTED_APPDATA_LBA1_FNV1A64
                if include_appdata
                else EXPECTED_LBA1_FNV1A64
            ),
            EXPECTED_HELLO_FNV1A64,
            EXPECTED_BUILD_FNV1A64,
        )
    if expected[0] and actual != expected:
        raise FixtureError(
            "fixture hashes differ from fixed constants: "
            f"actual={actual!r} expected={expected!r}"
        )


def verify_image(
    path: Path,
    *,
    include_appdata: bool = False,
    include_package_store: bool = False,
) -> bytes:
    include_appdata = include_appdata or include_package_store
    try:
        actual = path.read_bytes()
    except OSError as error:
        raise FixtureError(f"cannot read storage image {path}: {error}") from error
    validate_layout(
        actual,
        include_appdata=include_appdata,
        include_package_store=include_package_store,
    )
    expected = build_image(
        include_appdata=include_appdata,
        include_package_store=include_package_store,
    )
    if actual != expected:
        limit = min(len(actual), len(expected))
        mismatch = next((index for index in range(limit) if actual[index] != expected[index]), limit)
        if include_package_store:
            fixture = "APK-install0 package-store"
        elif include_appdata:
            fixture = "M54 appdata"
        else:
            fixture = "M25"
        raise FixtureError(
            f"storage image differs from the deterministic {fixture} fixture at byte {mismatch}"
        )
    validate_fixed_hashes(
        actual,
        include_appdata=include_appdata,
        include_package_store=include_package_store,
    )
    return actual


def marker(
    image: bytes,
    path: Path,
    *,
    include_appdata: bool = False,
    include_package_store: bool = False,
) -> str:
    include_appdata = include_appdata or include_package_store
    if include_package_store:
        image_bytes = PACKAGE_STORE_IMAGE_BYTES
        sector_count = PACKAGE_STORE_SECTOR_COUNT
        backup_entries_lba = PACKAGE_STORE_BACKUP_ENTRIES_LBA
        backup_header_lba = PACKAGE_STORE_BACKUP_HEADER_LBA
        last_usable_lba = PACKAGE_STORE_LAST_USABLE_LBA
        primary_header_crc = EXPECTED_PACKAGE_STORE_PRIMARY_HEADER_CRC32
        backup_header_crc = EXPECTED_PACKAGE_STORE_BACKUP_HEADER_CRC32
        partition_entry_crc = EXPECTED_PACKAGE_STORE_PARTITION_ENTRY_CRC32
        variant = "variant=apk-install0 "
    else:
        image_bytes = IMAGE_BYTES
        sector_count = SECTOR_COUNT
        backup_entries_lba = BACKUP_ENTRIES_LBA
        backup_header_lba = BACKUP_HEADER_LBA
        last_usable_lba = LAST_USABLE_LBA
        primary_header_crc = EXPECTED_APPDATA_PRIMARY_HEADER_CRC32
        backup_header_crc = EXPECTED_APPDATA_BACKUP_HEADER_CRC32
        partition_entry_crc = EXPECTED_APPDATA_PARTITION_ENTRY_CRC32
        variant = ""

    sha256, lba0, lba1, hello, build = image_hashes(image)
    appdata = ""
    if include_appdata:
        appdata = (
            f"gpt_primary_header_crc32=0x{primary_header_crc:08x} "
            f"gpt_backup_header_crc32=0x{backup_header_crc:08x} "
            f"gpt_partition_entry_crc32=0x{partition_entry_crc:08x} "
            f"appdata_type_guid={APPDATA_TYPE_GUID} "
            f"appdata_partition_guid={APPDATA_PARTITION_GUID} "
            f"appdata_format_epoch={APPDATA_FORMAT_EPOCH.hex()} "
            f"appdata_partition_lba={APPDATA_PARTITION_FIRST_LBA}-{APPDATA_PARTITION_LAST_LBA} "
            f"appdata_partition_sectors={APPDATA_PARTITION_SECTORS} appdata_initial_zero=1 "
        )
    package_store = ""
    if include_package_store:
        package_store = (
            "package_store=1 appdata_implied=1 package_gpt_index=3 "
            f"package_type_guid={PACKAGE_STORE_TYPE_GUID} "
            f"package_type_guid_raw={PACKAGE_STORE_TYPE_GUID_RAW.hex()} "
            f"package_partition_guid={PACKAGE_STORE_PARTITION_GUID} "
            f"package_partition_name={PACKAGE_STORE_PARTITION_NAME} "
            f"package_partition_lba={PACKAGE_STORE_PARTITION_FIRST_LBA}-"
            f"{PACKAGE_STORE_PARTITION_LAST_LBA} "
            f"package_partition_sectors={PACKAGE_STORE_PARTITION_SECTORS} "
            "package_initial_zero=1 "
        )
    return (
        "STORAGE_IMAGE_OK "
        f"bytes={image_bytes} sectors={sector_count} sector_bytes={SECTOR_BYTES} "
        f"{variant}"
        f"sha256={sha256} lba0_fnv1a64=0x{lba0:016x} lba1_fnv1a64=0x{lba1:016x} "
        f"gpt_primary_header={PRIMARY_HEADER_LBA} gpt_primary_entries=2-33 "
        f"gpt_first_usable={FIRST_USABLE_LBA} gpt_last_usable={last_usable_lba} "
        f"gpt_backup_entries={backup_entries_lba}-"
        f"{backup_entries_lba + GPT_ENTRY_SECTORS - 1} "
        f"gpt_backup_header={backup_header_lba} "
        f"gpt_entry_count={GPT_ENTRY_COUNT} gpt_entry_bytes={GPT_ENTRY_BYTES} "
        f"disk_guid={DISK_GUID} system_partition_guid={SYSTEM_PARTITION_GUID} "
        f"system_partition_lba={SYSTEM_PARTITION_FIRST_LBA}-{SYSTEM_PARTITION_LAST_LBA} "
        f"system_partition_sectors={SYSTEM_PARTITION_SECTORS} "
        f"data_type_guid={DATA_TYPE_GUID} data_partition_guid={DATA_PARTITION_GUID} "
        f"data_partition_lba={DATA_PARTITION_FIRST_LBA}-{DATA_PARTITION_LAST_LBA} "
        f"data_partition_sectors={DATA_PARTITION_SECTORS} data_format=1 "
        f"data_format_epoch={DATA_FORMAT_EPOCH.hex()} data_slots=2 "
        f"initial_generation=0 initial_slot=0 fat=fat16 fat_lba={FAT_FIRST_LBA} "
        f"fat_bps={FAT_BYTES_PER_SECTOR} fat_spc={FAT_SECTORS_PER_CLUSTER} "
        f"fat_reserved={FAT_RESERVED_SECTORS} fat_copies={FAT_COPY_COUNT} "
        f"fat_sectors={FAT_SECTORS} fat_root_entries={FAT_ROOT_ENTRIES} "
        f"hello_bytes={len(HELLO_CONTENT)} hello_fnv1a64=0x{hello:016x} "
        f"build_bytes={len(BUILD_CONTENT)} build_fnv1a64=0x{build:016x} "
        f"{appdata}{package_store}path={path}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("build", "verify", "marker"):
        child = subparsers.add_parser(command)
        child.add_argument("image", type=Path)
        variants = child.add_mutually_exclusive_group()
        variants.add_argument(
            "--appdata",
            "--with-appdata",
            dest="appdata",
            action="store_true",
            help="select the deterministic M54 image with an empty BNDROID_APPDATA partition",
        )
        variants.add_argument(
            "--with-package-store",
            action="store_true",
            help=(
                "select the deterministic 16 MiB image with empty "
                "BNDROID_APPDATA and BNDROID_PACKAGES partitions"
            ),
        )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "build":
            image = build_image(
                include_appdata=args.appdata,
                include_package_store=args.with_package_store,
            )
            validate_fixed_hashes(
                image,
                include_appdata=args.appdata,
                include_package_store=args.with_package_store,
            )
            args.image.write_bytes(image)
        elif args.command == "verify":
            verify_image(
                args.image,
                include_appdata=args.appdata,
                include_package_store=args.with_package_store,
            )
        else:
            image = verify_image(
                args.image,
                include_appdata=args.appdata,
                include_package_store=args.with_package_store,
            )
            print(
                marker(
                    image,
                    args.image,
                    include_appdata=args.appdata,
                    include_package_store=args.with_package_store,
                )
            )
    except (FixtureError, OSError) as error:
        print(f"build_storage_image.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
