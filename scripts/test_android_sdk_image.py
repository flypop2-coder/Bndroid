import hashlib
from pathlib import Path
import struct
import tempfile
import unittest
import zlib

from android_sdk_image import Image


def fixture():
    disk = bytearray(4096 * 512)
    table = bytearray(128 * 128)
    table[:16] = bytes([1]) * 16
    struct.pack_into("<QQ", table, 32, 64, 4000)
    table[56:66] = "super".encode("utf-16le")
    for here, other, entries in [(1, 4095, 2), (4095, 1, 4063)]:
        h = bytearray(512); h[:8] = b"EFI PART"
        struct.pack_into("<IIIIQQQQ", h, 8, 0x10000, 92, 0, 0, here, other, 34, 4062)
        struct.pack_into("<QIII", h, 72, entries, 128, 128, zlib.crc32(table))
        struct.pack_into("<I", h, 16, zlib.crc32(h[:92]))
        disk[here * 512:(here + 1) * 512] = h
        disk[entries * 512:entries * 512 + len(table)] = table
    geometry = bytearray(4096)
    struct.pack_into("<II", geometry, 0, 0x616c4467, 52)
    struct.pack_into("<III", geometry, 40, 4096, 1, 4096)
    geometry[8:40] = hashlib.sha256(geometry[:52]).digest()
    base = 64 * 512
    disk[base + 4096:base + 8192] = geometry
    disk[base + 8192:base + 12288] = geometry
    partition = struct.pack("<36sIIII", b"system", 1, 0, 1, 0)
    extent = struct.pack("<QIQI", 8, 0, 64, 0)
    group = struct.pack("<36sIQ", b"default", 0, 0)
    device = struct.pack("<QIIQ36sI", 64, 4096, 0, (4000 - 64 + 1) * 512, b"super", 0)
    tables = partition + extent + group + device
    metadata = bytearray(4096)
    struct.pack_into("<IHHI", metadata, 0, 0x414c5030, 10, 0, 128)
    struct.pack_into("<I", metadata, 44, len(tables))
    metadata[48:80] = hashlib.sha256(tables).digest()
    offset = 0
    for at, width in [(80, 52), (92, 24), (104, 48), (116, 64)]:
        struct.pack_into("<III", metadata, at, offset, 1, width); offset += width
    metadata[12:44] = hashlib.sha256(metadata[:128]).digest()
    metadata[128:128 + len(tables)] = tables
    for offset in [12288, 16384]:
        disk[base + offset:base + offset + 4096] = metadata
    disk[base + 64 * 512:base + 72 * 512] = b"owned" + bytes(4091)
    return disk


class ImageTests(unittest.TestCase):
    def test_checked_extent_extraction_preserves_source_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root / "source.img"; original = fixture(); source.write_bytes(original)
            with Image(source) as image:
                output = root / "system.img"; image.extract("system", output)
                self.assertEqual(output.read_bytes(), b"owned" + bytes(4091))
                with self.assertRaises(FileExistsError): image.extract("system", output)
                with self.assertRaises(ValueError): image.extract("vendor", root / "missing.img")
            self.assertEqual(source.read_bytes(), original)

    def test_corrupt_metadata_and_backups_fail_before_output(self):
        original = fixture(); base = 64 * 512
        for offset in [512 + 16, 1024, 4095 * 512 + 16, base + 4096 + 8,
                       base + 8192 + 44, base + 12288 + 12, base + 16384 + 128]:
            with self.subTest(offset=offset), tempfile.TemporaryDirectory() as directory:
                data = original.copy(); data[offset] ^= 1
                source = Path(directory) / "bad.img"; source.write_bytes(data)
                with self.assertRaises(ValueError): Image(source)

    def test_valid_checksums_do_not_authorize_out_of_bounds_or_unsupported_extents(self):
        for sectors, kind, target, device in [(4096, 0, 64, 0), (8, 1, 64, 0), (8, 0, 0, 0), (8, 0, 64, 1)]:
            with self.subTest(extent=(sectors, kind, target, device)), tempfile.TemporaryDirectory() as directory:
                data = fixture(); base = 64 * 512
                for offset in [base + 12288, base + 16384]:
                    struct.pack_into("<QIQI", data, offset + 128 + 52, sectors, kind, target, device)
                    data[offset + 48:offset + 80] = hashlib.sha256(data[offset + 128:offset + 316]).digest()
                    data[offset + 12:offset + 44] = bytes(32)
                    data[offset + 12:offset + 44] = hashlib.sha256(data[offset:offset + 128]).digest()
                source = Path(directory) / "bad.img"; source.write_bytes(data)
                with self.assertRaises(ValueError): Image(source)


if __name__ == "__main__":
    unittest.main()
