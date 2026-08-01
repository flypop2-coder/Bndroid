#!/usr/bin/env python3
"""Generate one BMS1 signed product-service-manifest artifact.

The signing key is supplied explicitly and is never generated, copied, or
stored by this script. Output is lowercase hexadecimal so the reviewed source
artifact remains text while kernel/build.rs embeds the exact decoded bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import struct
import subprocess
import tempfile
from pathlib import Path


BMF1_MAGIC = b"BMF1"
BMF1_VERSION = 1
BMF1_HEADER_SIZE = 32
BMF1_SERVICE_RECORD_SIZE = 32
BMF1_DEPENDENCY_RECORD_SIZE = 8
BMF1_SERVICE_COUNT = 5
BMF1_DEPENDENCY_COUNT = 4
BMF1_SIZE = 224

BMS1_MAGIC = b"BMS1"
BMS1_VERSION = 1
BMS1_ALGORITHM_RSA2048_PKCS1_V15_SHA256 = 1
BMS1_DEFAULT_KEY_ID = 1
BMS1_HEADER_SIZE = 32
BMS1_SIGNED_SIZE = 256
BMS1_SIGNATURE_SIZE = 256
BMS1_SIZE = 512

FNV_OFFSET_BASIS = 0xCBF29CE484222325
FNV_PRIME = 0x00000100000001B3


def fnv1a64(data: bytes) -> int:
    value = FNV_OFFSET_BASIS
    for byte in data:
        value ^= byte
        value = (value * FNV_PRIME) & ((1 << 64) - 1)
    return value


def encode_service(
    kind: int,
    image: int,
    launch_mode: int,
    shutdown_node: int,
    restart_budget: int,
    restart_backoff_ns: int,
    health_timeout_ns: int,
    missed_probe_tolerance: int,
) -> bytes:
    record = bytearray(BMF1_SERVICE_RECORD_SIZE)
    record[0] = kind
    record[1] = image
    record[2] = launch_mode
    record[3] = shutdown_node
    struct.pack_into("<I", record, 4, restart_budget)
    struct.pack_into("<Q", record, 8, restart_backoff_ns)
    struct.pack_into("<Q", record, 16, health_timeout_ns)
    struct.pack_into("<I", record, 24, missed_probe_tolerance)
    return bytes(record)


def encode_product_bmf1(generation: int) -> bytes:
    if generation <= 0 or generation > 0xFFFFFFFF:
        raise ValueError("BMF1 generation must fit one non-zero u32")

    # kind, image, launch mode, shutdown node, missed-probe tolerance.
    profile = [
        (5, 7, 1, 7, 1),  # App
        (1, 2, 1, 0, 1),  # ServiceManager
        (4, 9, 2, 0xFF, 0),  # StorageServer
        (3, 8, 1, 5, 1),  # InputServer
        (2, 5, 1, 4, 1),  # SurfaceServer
    ]
    services = b"".join(
        encode_service(
            kind,
            image,
            launch_mode,
            shutdown_node,
            1,
            30_000_000,
            100_000_000,
            tolerance,
        )
        for kind, image, launch_mode, shutdown_node, tolerance in profile
    )
    dependencies = b"".join(
        bytes((prerequisite, dependent, kind, 0, 0, 0, 0, 0))
        for prerequisite, dependent, kind in [
            (4, 5, 1),  # StorageServer -> App, hard
            (3, 5, 2),  # InputServer -> App, soft
            (2, 3, 1),  # SurfaceServer -> InputServer, hard
            (1, 2, 1),  # ServiceManager -> SurfaceServer, hard
        ]
    )
    payload = services + dependencies
    header = bytearray(BMF1_HEADER_SIZE)
    header[:4] = BMF1_MAGIC
    header[4] = BMF1_VERSION
    header[5] = BMF1_HEADER_SIZE
    header[6] = BMF1_SERVICE_RECORD_SIZE
    header[7] = BMF1_DEPENDENCY_RECORD_SIZE
    struct.pack_into("<I", header, 8, BMF1_SIZE)
    struct.pack_into("<I", header, 12, generation)
    header[16] = BMF1_SERVICE_COUNT
    header[17] = BMF1_DEPENDENCY_COUNT
    struct.pack_into("<Q", header, 24, fnv1a64(payload))
    wire = bytes(header) + payload
    if len(wire) != BMF1_SIZE:
        raise AssertionError(f"BMF1 size changed: {len(wire)}")
    return wire


def encode_signed_region(generation: int, rollback_index: int, key_id: int) -> bytes:
    if rollback_index <= 0 or rollback_index > 0xFFFFFFFF:
        raise ValueError("BMS1 rollback index must fit one non-zero u32")
    if key_id <= 0 or key_id > 0xFF:
        raise ValueError("BMS1 key id must fit one non-zero u8")
    payload = encode_product_bmf1(generation)
    header = bytearray(BMS1_HEADER_SIZE)
    header[:4] = BMS1_MAGIC
    header[4] = BMS1_VERSION
    header[5] = BMS1_ALGORITHM_RSA2048_PKCS1_V15_SHA256
    header[6] = key_id
    struct.pack_into("<I", header, 8, BMS1_SIZE)
    struct.pack_into("<I", header, 12, BMS1_SIGNED_SIZE)
    struct.pack_into("<I", header, 16, BMS1_HEADER_SIZE)
    struct.pack_into("<I", header, 20, BMF1_SIZE)
    struct.pack_into("<I", header, 24, rollback_index)
    signed = bytes(header) + payload
    if len(signed) != BMS1_SIGNED_SIZE:
        raise AssertionError(f"BMS1 signed-region size changed: {len(signed)}")
    return signed


def rsa_modulus(key: Path, openssl: str) -> bytes:
    result = subprocess.run(
        [openssl, "rsa", "-in", str(key), "-noout", "-modulus"],
        check=True,
        capture_output=True,
        text=True,
    )
    prefix = "Modulus="
    line = result.stdout.strip()
    if not line.startswith(prefix):
        raise RuntimeError("OpenSSL did not return an RSA modulus")
    modulus = bytes.fromhex(line[len(prefix) :])
    if len(modulus) != 256 or modulus[0] & 0x80 == 0 or modulus[-1] & 1 == 0:
        raise ValueError("signing key must use an odd, full-width RSA-2048 modulus")
    return modulus


def sign_region(region: bytes, key: Path, openssl: str) -> bytes:
    with tempfile.TemporaryDirectory(prefix="bndroid-bms1-") as directory:
        input_path = Path(directory) / "signed-region.bin"
        signature_path = Path(directory) / "signature.bin"
        input_path.write_bytes(region)
        subprocess.run(
            [
                openssl,
                "dgst",
                "-sha256",
                "-sign",
                str(key),
                "-out",
                str(signature_path),
                str(input_path),
            ],
            check=True,
        )
        signature = signature_path.read_bytes()
    if len(signature) != BMS1_SIGNATURE_SIZE:
        raise ValueError(
            f"signing key produced {len(signature)} bytes, expected RSA-2048 signature"
        )
    return signature


def write_hex(path: Path, artifact: bytes) -> None:
    encoded = artifact.hex()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "\n".join(encoded[index : index + 64] for index in range(0, len(encoded), 64))
        + "\n",
        encoding="ascii",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--key", type=Path, required=True)
    parser.add_argument("--generation", type=int, required=True)
    parser.add_argument("--rollback-index", type=int, required=True)
    parser.add_argument("--key-id", type=int, default=BMS1_DEFAULT_KEY_ID)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--openssl", default="openssl")
    parser.add_argument(
        "--corrupt-signature",
        action="store_true",
        help="flip one signature bit after signing (negative-test fixtures only)",
    )
    args = parser.parse_args()

    modulus = rsa_modulus(args.key, args.openssl)
    signed = encode_signed_region(args.generation, args.rollback_index, args.key_id)
    signature = bytearray(sign_region(signed, args.key, args.openssl))
    if args.corrupt_signature:
        signature[-1] ^= 1
    artifact = signed + bytes(signature)
    if len(artifact) != BMS1_SIZE:
        raise AssertionError(f"BMS1 artifact size changed: {len(artifact)}")
    write_hex(args.output, artifact)

    print(
        "BMS1_ARTIFACT_OK "
        f"path={args.output} generation={args.generation} "
        f"rollback_index={args.rollback_index} key_id={args.key_id} "
        f"bytes={len(artifact)} "
        f"signed_sha256={hashlib.sha256(signed).hexdigest()} "
        f"modulus_sha256={hashlib.sha256(modulus).hexdigest()} "
        f"signature_corrupted={int(args.corrupt_signature)}"
    )
    print(f"RSA2048_MODULUS_HEX={modulus.hex()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
