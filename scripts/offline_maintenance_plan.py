#!/usr/bin/env python3
"""Prepare and assemble signed BMP1 maintenance plans offline.

`prepare` emits the exact 256-byte region that an external signer signs.
`assemble` accepts only that canonical request, a detached RSA-2048 signature,
and the pinned public key.  It verifies the detached signature before writing
the 512-byte artifact.  There is intentionally no private-key or signing
input.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import subprocess
import tempfile
from pathlib import Path


BMP1_MAGIC = b"BMP1"
BMP1_VERSION = 1
BMP1_ALGORITHM = 1
BMP1_KEY_ID = 1
BMP1_HEADER_SIZE = 64
BMP1_PAYLOAD_SIZE = 192
BMP1_SIGNED_SIZE = 256
BMP1_SIGNATURE_SIZE = 256
BMP1_SIZE = 512
BMP1_OPERATION_COUNT = 3
BMP1_TRANSITION_COUNT = 9
BMP1_PROGRAM_VERSION = 1
BMP1_DESCRIPTOR_SIZE = 16
BMP1_DESCRIPTOR_CAPACITY = 4

OPERATION_STORAGE_ROTATION = 1
OPERATION_RESIDENT_DRAIN = 2
OPERATION_FLAG_IDEMPOTENT = 1 << 0
OPERATION_FLAG_PREAPPLY_CANCELLABLE = 1 << 1

BMA1_SIZE = 512
BMA1_SIGNED_SIZE = 256
BMA1_AUTHORIZATION_ROOT_SHA256 = bytes.fromhex(
    "683eafa41277e75ae8cd1332490839a9"
    "cab204c8e8b808e2ebdf77f37259b005"
)
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}")


def read_hex(path: Path, expected_bytes: int, label: str) -> bytes:
    try:
        text = path.read_text(encoding="ascii")
    except (OSError, UnicodeError) as error:
        raise ValueError(f"cannot read {label}: {path}") from error
    digits = "".join(text.split())
    if (
        len(digits) != expected_bytes * 2
        or digits != digits.lower()
        or re.fullmatch(r"[0-9a-f]+", digits) is None
    ):
        raise ValueError(
            f"{label} must be exactly {expected_bytes} bytes of lowercase hexadecimal"
        )
    return bytes.fromhex(digits)


def write_hex(path: Path, payload: bytes) -> None:
    encoded = payload.hex()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "\n".join(encoded[index : index + 64] for index in range(0, len(encoded), 64))
        + "\n",
        encoding="ascii",
    )


def inspect_authorization(artifact: bytes) -> dict[str, int | bytes]:
    if len(artifact) != BMA1_SIZE or artifact[:4] != b"BMA1":
        raise ValueError("maintenance authorization is not one complete BMA1 artifact")
    if artifact[4:8] != bytes([1, 1, 1, 0]):
        raise ValueError("maintenance authorization envelope is unsupported")
    if struct.unpack_from("<IIII", artifact, 8) != (512, 256, 64, 192):
        raise ValueError("maintenance authorization layout is unsupported")
    sequence = struct.unpack_from("<Q", artifact, 24)[0]
    operations = struct.unpack_from("<Q", artifact, 32)[0]
    max_uses = struct.unpack_from("<I", artifact, 40)[0]
    if sequence == 0 or operations != 1 or max_uses != 2:
        raise ValueError("maintenance authorization policy is unsupported")
    return {
        "sequence": sequence,
        "operations": operations,
        "max_uses": max_uses,
        "authorization_sha256": hashlib.sha256(
            artifact[:BMA1_SIGNED_SIZE]
        ).digest(),
        "authorization_id": artifact[192:224],
        "manifest_sha256": artifact[64:96],
        "policy_sha256": artifact[160:192],
        "device_binding_sha256": artifact[128:160],
    }


def maintenance_plan_id(authorization: dict[str, int | bytes]) -> bytes:
    material = (
        b"BNDRMPL1"
        + struct.pack(
            "<QQI",
            int(authorization["sequence"]),
            int(authorization["operations"]),
            int(authorization["max_uses"]),
        )
        + bytes(authorization["authorization_sha256"])
        + bytes(authorization["authorization_id"])
        + bytes(authorization["manifest_sha256"])
        + bytes(authorization["policy_sha256"])
        + BMA1_AUTHORIZATION_ROOT_SHA256
        + bytes(authorization["device_binding_sha256"])
    )
    if len(material) != 220:
        raise AssertionError("M80 plan-id material changed")
    return hashlib.sha256(material).digest()


def canonical_descriptors() -> bytes:
    descriptors = (
        struct.pack(
            "<IIII",
            OPERATION_STORAGE_ROTATION,
            OPERATION_FLAG_IDEMPOTENT | OPERATION_FLAG_PREAPPLY_CANCELLABLE,
            1,
            0x4D810001,
        )
        + struct.pack(
            "<IIII",
            OPERATION_STORAGE_ROTATION,
            OPERATION_FLAG_IDEMPOTENT,
            2,
            0x4D810002,
        )
        + struct.pack(
            "<IIII",
            OPERATION_RESIDENT_DRAIN,
            OPERATION_FLAG_IDEMPOTENT,
            2,
            0x4D810003,
        )
        + bytes(BMP1_DESCRIPTOR_SIZE)
    )
    if len(descriptors) != BMP1_DESCRIPTOR_SIZE * BMP1_DESCRIPTOR_CAPACITY:
        raise AssertionError("BMP1 descriptor table changed size")
    return descriptors


def encode_signed_region(authorization_artifact: bytes) -> bytes:
    authorization = inspect_authorization(authorization_artifact)
    sequence = int(authorization["sequence"])
    header = bytearray(BMP1_HEADER_SIZE)
    header[:4] = BMP1_MAGIC
    header[4] = BMP1_VERSION
    header[5] = BMP1_ALGORITHM
    header[6] = BMP1_KEY_ID
    struct.pack_into(
        "<IIIIQIIIIQ",
        header,
        8,
        BMP1_SIZE,
        BMP1_SIGNED_SIZE,
        BMP1_HEADER_SIZE,
        BMP1_PAYLOAD_SIZE,
        sequence,
        BMP1_OPERATION_COUNT,
        BMP1_TRANSITION_COUNT,
        BMP1_PROGRAM_VERSION,
        BMP1_DESCRIPTOR_SIZE,
        sequence,
    )
    payload = bytearray(BMP1_PAYLOAD_SIZE)
    payload[0:32] = bytes(authorization["authorization_sha256"])
    payload[32:64] = bytes(authorization["authorization_id"])
    payload[64:96] = bytes(authorization["device_binding_sha256"])
    payload[96:128] = maintenance_plan_id(authorization)
    payload[128:192] = canonical_descriptors()
    region = bytes(header + payload)
    if len(region) != BMP1_SIGNED_SIZE:
        raise AssertionError("BMP1 signed region changed size")
    return region


def inspect_request(region: bytes, authorization_artifact: bytes) -> int:
    if len(region) != BMP1_SIGNED_SIZE:
        raise ValueError("BMP1 signing request has the wrong size")
    expected = encode_signed_region(authorization_artifact)
    if region != expected:
        raise ValueError("BMP1 signing request is not canonical for the authorization")
    return struct.unpack_from("<Q", region, 24)[0]


def public_modulus(public_key: Path, openssl: str) -> bytes:
    result = subprocess.run(
        [openssl, "rsa", "-pubin", "-in", str(public_key), "-noout", "-modulus"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ValueError("OpenSSL rejected the RSA public key")
    line = result.stdout.strip()
    if not line.startswith("Modulus="):
        raise ValueError("OpenSSL did not return an RSA public modulus")
    try:
        modulus = bytes.fromhex(line[len("Modulus=") :])
    except ValueError as error:
        raise ValueError("OpenSSL returned a malformed RSA public modulus") from error
    if len(modulus) != BMP1_SIGNATURE_SIZE or modulus[0] & 0x80 == 0 or modulus[-1] & 1 == 0:
        raise ValueError("public key must use an odd, full-width RSA-2048 modulus")
    return modulus


def require_public_exponent_65537(public_key: Path, openssl: str) -> None:
    result = subprocess.run(
        [openssl, "rsa", "-pubin", "-in", str(public_key), "-text", "-noout"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or re.search(
        r"Exponent:\s*65537\s*\(0x10001\)", result.stdout
    ) is None:
        raise ValueError("public key exponent must be 65537")


def verify_signature(
    region: bytes,
    signature: bytes,
    public_key: Path,
    openssl: str,
) -> None:
    with tempfile.TemporaryDirectory(prefix="bndroid-bmp1-assemble-") as directory:
        request_path = Path(directory) / "request.bin"
        signature_path = Path(directory) / "signature.bin"
        request_path.write_bytes(region)
        signature_path.write_bytes(signature)
        result = subprocess.run(
            [
                openssl,
                "dgst",
                "-sha256",
                "-verify",
                str(public_key),
                "-signature",
                str(signature_path),
                str(request_path),
            ],
            check=False,
            capture_output=True,
            text=True,
        )
    if result.returncode != 0 or result.stdout.strip() != "Verified OK":
        raise ValueError("detached BMP1 signature verification failed")


def prepare(args: argparse.Namespace) -> int:
    authorization = read_hex(args.authorization, BMA1_SIZE, "BMA1 authorization")
    region = encode_signed_region(authorization)
    binding = inspect_authorization(authorization)
    write_hex(args.request, region)
    print(
        "BMP1_OFFLINE_REQUEST_OK "
        f"path={args.request} authorization_sequence={binding['sequence']} "
        f"operations={BMP1_OPERATION_COUNT} transitions={BMP1_TRANSITION_COUNT} "
        f"bytes={len(region)} signed_sha256={hashlib.sha256(region).hexdigest()} "
        f"plan_id={maintenance_plan_id(binding).hex()} "
        "contains_signature=0 contains_private_key=0"
    )
    return 0


def assemble(args: argparse.Namespace) -> int:
    if SHA256_PATTERN.fullmatch(args.expected_modulus_sha256) is None:
        raise ValueError("--expected-modulus-sha256 must be 64 lowercase hex digits")
    authorization = read_hex(args.authorization, BMA1_SIZE, "BMA1 authorization")
    region = read_hex(args.request, BMP1_SIGNED_SIZE, "BMP1 signing request")
    signature = read_hex(args.signature, BMP1_SIGNATURE_SIZE, "detached BMP1 signature")
    sequence = inspect_request(region, authorization)
    modulus = public_modulus(args.public_key, args.openssl)
    modulus_sha256 = hashlib.sha256(modulus).hexdigest()
    if modulus_sha256 != args.expected_modulus_sha256:
        raise ValueError(
            "public-key modulus digest does not match --expected-modulus-sha256"
        )
    require_public_exponent_65537(args.public_key, args.openssl)
    if int.from_bytes(signature, "big") >= int.from_bytes(modulus, "big"):
        raise ValueError("detached BMP1 signature is outside the RSA modulus")
    verify_signature(region, signature, args.public_key, args.openssl)
    artifact = region + signature
    if len(artifact) != BMP1_SIZE:
        raise AssertionError("assembled BMP1 artifact size changed")
    write_hex(args.output, artifact)
    print(
        "BMP1_OFFLINE_ASSEMBLY_OK "
        f"path={args.output} authorization_sequence={sequence} "
        f"operations={BMP1_OPERATION_COUNT} transitions={BMP1_TRANSITION_COUNT} "
        f"bytes={len(artifact)} signed_sha256={hashlib.sha256(region).hexdigest()} "
        f"modulus_sha256={modulus_sha256} signature_verified=1 "
        "private_key_consumed=0"
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--authorization", type=Path, required=True)
    prepare_parser.add_argument("--request", type=Path, required=True)
    prepare_parser.set_defaults(handler=prepare)

    assemble_parser = subparsers.add_parser("assemble")
    assemble_parser.add_argument("--authorization", type=Path, required=True)
    assemble_parser.add_argument("--request", type=Path, required=True)
    assemble_parser.add_argument("--signature", type=Path, required=True)
    assemble_parser.add_argument("--public-key", type=Path, required=True)
    assemble_parser.add_argument("--expected-modulus-sha256", required=True)
    assemble_parser.add_argument("--output", type=Path, required=True)
    assemble_parser.add_argument("--openssl", default="openssl")
    assemble_parser.set_defaults(handler=assemble)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        return args.handler(args)
    except ValueError as error:
        raise SystemExit(f"offline BMP1 error: {error}") from error


if __name__ == "__main__":
    raise SystemExit(main())
