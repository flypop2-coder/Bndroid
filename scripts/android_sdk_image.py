"""Read selected logical partitions from a local raw GPT Android SDK image.

No mounts, adb privileges or writes to the source image. GPT CRCs and both
liblp geometry/slot-zero metadata copies must agree before any extraction.
Only bounded, single-device linear extents are supported.
"""
import hashlib
import re
import struct
import zlib


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


class Image:
    def __init__(self, path):
        self.path = path
        self.size = path.stat().st_size
        require(1024 * 1024 <= self.size <= 16 * 1024**3 and self.size % 512 == 0,
                "unsupported SDK image size")
        self.stream = path.open("rb")
        try:
            self.partitions = self._partitions()
        except BaseException:
            self.stream.close()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.stream.close()

    def read(self, offset, length):
        require(0 <= offset <= self.size and 0 <= length <= min(self.size - offset, 4 * 1024**2),
                "image read outside bounds")
        self.stream.seek(offset)
        data = self.stream.read(length)
        require(len(data) == length, "short image read")
        return data

    def _gpt(self, sector):
        header = self.read(sector * 512, 512)
        require(header[:8] == b"EFI PART", "missing GPT header")
        revision, size, checksum, reserved = struct.unpack_from("<IIII", header, 8)
        require(revision == 0x10000 and 92 <= size <= 512 and reserved == 0, "unsupported GPT header")
        checked = bytearray(header[:size]); checked[16:20] = bytes(4)
        require(zlib.crc32(checked) == checksum, "invalid GPT header CRC")
        here, other, first, last = struct.unpack_from("<QQQQ", header, 24)
        require(here == sector and other == (self.size // 512 - 1 if sector == 1 else 1)
                and 1 < first <= last < self.size // 512 - 1, "invalid GPT geometry")
        entries, count, width, crc = struct.unpack_from("<QIII", header, 72)
        require(0 < count <= 4096 and width == 128, "unsupported GPT entries")
        table = self.read(entries * 512, count * width)
        require(zlib.crc32(table) == crc, "invalid GPT entry CRC")
        return header[56:72], first, last, table

    def _partitions(self):
        primary = self._gpt(1)
        require(primary == self._gpt(self.size // 512 - 1), "GPT copies disagree")
        _, first, last, entries = primary
        supers = []
        ranges = []
        for at in range(0, len(entries), 128):
            entry = entries[at:at + 128]
            if not any(entry[:16]):
                continue
            begin, end = struct.unpack_from("<QQ", entry, 32)
            require(first <= begin <= end <= last, "GPT partition outside usable area")
            require(all(end < a or b < begin for a, b in ranges), "overlapping GPT partitions")
            ranges.append((begin, end))
            if entry[56:128].decode("utf-16le").rstrip("\0") == "super":
                supers.append((begin * 512, (end - begin + 1) * 512))
        require(len(supers) == 1, "exactly one super partition is required")
        base, length = supers[0]
        geometry = self.read(base + 4096, 4096)
        require(geometry == self.read(base + 8192, 4096), "liblp geometry copies disagree")
        magic, size = struct.unpack_from("<II", geometry)
        require(magic == 0x616c4467 and size == 52, "unsupported liblp geometry")
        checked = bytearray(geometry[:size]); checked[8:40] = bytes(32)
        require(hashlib.sha256(checked).digest() == geometry[8:40], "invalid liblp geometry checksum")
        maximum, slots, block = struct.unpack_from("<III", geometry, 40)
        require(512 <= maximum <= 4 * 1024**2 and maximum % 512 == 0
                and 1 <= slots <= 4 and block == 4096
                and 12288 + 2 * slots * maximum <= length, "unsupported liblp geometry limits")
        metadata = self.read(base + 12288, maximum)
        require(metadata == self.read(base + 12288 + slots * maximum, maximum), "liblp slot-zero copies disagree")
        magic, major, minor, size = struct.unpack_from("<IHHI", metadata)
        require(magic == 0x414c5030 and major == 10 and minor <= 2
                and size == (256 if minor == 2 else 128), "unsupported liblp version")
        checked = bytearray(metadata[:size]); checked[12:44] = bytes(32)
        require(hashlib.sha256(checked).digest() == metadata[12:44], "invalid liblp header checksum")
        table_size, = struct.unpack_from("<I", metadata, 44)
        require(table_size <= maximum - size, "liblp table exceeds metadata")
        table = metadata[size:size + table_size]
        require(hashlib.sha256(table).digest() == metadata[48:80], "invalid liblp table checksum")
        tables, ranges = [], []
        for offset, width in [(80, 52), (92, 24), (104, 48), (116, 64)]:
            at, count, entry_size = struct.unpack_from("<III", metadata, offset)
            require(entry_size == width and count <= 4096 and at + count * width <= len(table),
                    "invalid liblp table descriptor")
            require(all(at + count * width <= a or b <= at for a, b in ranges), "overlapping liblp tables")
            ranges.append((at, at + count * width))
            tables.append([table[at + i * width:at + (i + 1) * width] for i in range(count)])
        partitions, extents, groups, devices = tables
        require(len(devices) == 1, "only single-device super images are supported")
        first_sector, _, _, device_size = struct.unpack_from("<QIIQ", devices[0])
        require(device_size <= length and first_sector * 512 >= 12288 + 2 * slots * maximum,
                "invalid liblp block device")
        result, physical = {}, []
        for partition in partitions:
            name = partition[:36].rstrip(b"\0").decode("ascii")
            attributes, index, count, group = struct.unpack_from("<IIII", partition, 36)
            require(re.fullmatch(r"[A-Za-z0-9_]{1,36}", name) and name not in result
                    and attributes == 1 and group < len(groups) and count > 0
                    and index + count <= len(extents), "unsupported logical partition")
            spans = []
            for extent in extents[index:index + count]:
                sectors, kind, target, source = struct.unpack("<QIQI", extent)
                require(kind == 0 and source == 0 and sectors > 0 and sectors % 8 == 0
                        and target >= first_sector and (target + sectors) * 512 <= device_size,
                        "unsupported or out-of-bounds logical extent")
                require(all(target + sectors <= a or b <= target for a, b in physical), "overlapping logical extents")
                physical.append((target, target + sectors))
                spans.append((base + target * 512, sectors * 512))
            result[name] = spans
        return result

    def extract(self, name, destination):
        require(name in self.partitions, "missing logical partition")
        spans = self.partitions[name]
        require(sum(size for _, size in spans) <= 2 * 1024**3, "partition exceeds extraction bound")
        with destination.open("xb") as out:
            for offset, size in spans:
                while size:
                    chunk = self.read(offset, min(size, 1024**2))
                    if any(chunk):
                        out.write(chunk)
                    else:
                        out.seek(len(chunk), 1)
                    offset += len(chunk); size -= len(chunk)
            out.truncate()
