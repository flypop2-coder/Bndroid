#!/usr/bin/env python3
"""Prepare and assemble BMA1 maintenance authorizations offline.

`prepare` emits the exact 256-byte region that an external signer must sign.
`assemble` accepts only that canonical request, a detached RSA-2048 signature,
and the pinned public key. It verifies every binding and the detached
signature before writing the 512-byte artifact. There is intentionally no
private-key or signing input.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import subprocess
import tempfile
from pathlib import Path


BMA1_MAGIC = b"BMA1"
BMA1_VERSION = 1
BMA1_ALGORITHM = 1
BMA1_KEY_ID = 1
BMA1_HEADER_SIZE = 64
BMA1_PAYLOAD_SIZE = 192
BMA1_SIGNED_SIZE = 256
BMA1_SIGNATURE_SIZE = 256
BMA1_SIZE = 512
BMA1_OPERATIONS = 1
BMA1_MAX_USES = 2
BMA1_MANIFEST_GENERATION = 5
BMA1_ACTIVE_KEY_EPOCH = 4
BMA1_AUDIT_VERSION = 1

MANIFEST_SHA256 = bytes.fromhex(
    "5a8ccd0ae601dad43356782ed0cc803fc"
    "3fe1232d3f42be981a71d94ca33e988"
)
KEY_POLICY_SHA256 = bytes.fromhex(
    "30676f357683b6c8d2c5d67755e48beb"
    "93bdfe8864eccc67c716a1785e935e01"
)
DEVICE_BINDING_SHA256 = bytes.fromhex(
    "04ed2c7c657ebe5147d3da7103ea7a7e"
    "0d4d9bbe808915898967259e1b04a555"
)
MAINTENANCE_POLICY_SHA256 = bytes.fromhex(
    "d51ffa50e8ae40f278b5243f9d86a985"
    "b6d2835be59ff4c262d6c6436efe8200"
)
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}")


def authorization_id(sequence: int) -> bytes:
    material = (
        b"BMA1-ID\0"
        + struct.pack(
            "<QQIII",
            sequence,
            BMA1_OPERATIONS,
            BMA1_MAX_USES,
            BMA1_MANIFEST_GENERATION,
            BMA1_ACTIVE_KEY_EPOCH,
        )
        + MANIFEST_SHA256
        + DEVICE_BINDING_SHA256
        + MAINTENANCE_POLICY_SHA256
    )
    if len(material) != 132:
        raise AssertionError("BMA1 authorization-id material changed")
    return hashlib.sha256(material).digest()


def encode_signed_region(sequence: int) -> bytes:
    if not 1 <= sequence <= (1 << 64) - 1:
        raise ValueError("BMA1 sequence must be in 1..=2^64-1")
    header = bytearray(BMA1_HEADER_SIZE)
    header[:4] = BMA1_MAGIC
    header[4] = BMA1_VERSION
    header[5] = BMA1_ALGORITHM
    header[6] = BMA1_KEY_ID
    struct.pack_into(
        "<IIIIQQIIII",
        header,
        8,
        BMA1_SIZE,
        BMA1_SIGNED_SIZE,
        BMA1_HEADER_SIZE,
        BMA1_PAYLOAD_SIZE,
        sequence,
        BMA1_OPERATIONS,
        BMA1_MAX_USES,
        BMA1_MANIFEST_GENERATION,
        BMA1_ACTIVE_KEY_EPOCH,
        BMA1_AUDIT_VERSION,
    )
    payload = bytearray(BMA1_PAYLOAD_SIZE)
    payload[0:32] = MANIFEST_SHA256
    payload[32:64] = KEY_POLICY_SHA256
    payload[64:96] = DEVICE_BINDING_SHA256
    payload[96:128] = MAINTENANCE_POLICY_SHA256
    payload[128:160] = authorization_id(sequence)
    region = bytes(header + payload)
    if len(region) != BMA1_SIGNED_SIZE:
        raise AssertionError("BMA1 signed region changed size")
    return region


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


def inspect_request(region: bytes) -> int:
    if len(region) != BMA1_SIGNED_SIZE:
        raise ValueError("BMA1 signing request has the wrong size")
    sequence = struct.unpack_from("<Q", region, 24)[0]
    if region != encode_signed_region(sequence):
        raise ValueError("BMA1 signing request is not canonical")
    return sequence


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
    if len(modulus) != BMA1_SIGNATURE_SIZE or modulus[0] & 0x80 == 0 or modulus[-1] & 1 == 0:
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
    with tempfile.TemporaryDirectory(prefix="bndroid-bma1-assemble-") as directory:
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
        raise ValueError("detached BMA1 signature verification failed")


def prepare(args: argparse.Namespace) -> int:
    region = encode_signed_region(args.sequence)
    write_hex(args.request, region)
    print(
        "BMA1_OFFLINE_REQUEST_OK "
        f"path={args.request} sequence={args.sequence} operations=storage-rotation "
        f"max_uses={BMA1_MAX_USES} bytes={len(region)} "
        f"signed_sha256={hashlib.sha256(region).hexdigest()} "
        f"authorization_id={authorization_id(args.sequence).hex()} "
        "contains_signature=0 contains_private_key=0"
    )
    return 0


def assemble(args: argparse.Namespace) -> int:
    if SHA256_PATTERN.fullmatch(args.expected_modulus_sha256) is None:
        raise ValueError("--expected-modulus-sha256 must be 64 lowercase hex digits")
    region = read_hex(args.request, BMA1_SIGNED_SIZE, "BMA1 signing request")
    signature = read_hex(args.signature, BMA1_SIGNATURE_SIZE, "detached BMA1 signature")
    sequence = inspect_request(region)

    modulus = public_modulus(args.public_key, args.openssl)
    modulus_sha256 = hashlib.sha256(modulus).hexdigest()
    if modulus_sha256 != args.expected_modulus_sha256:
        raise ValueError(
            "public-key modulus digest does not match --expected-modulus-sha256"
        )
    require_public_exponent_65537(args.public_key, args.openssl)
    if int.from_bytes(signature, "big") >= int.from_bytes(modulus, "big"):
        raise ValueError("detached BMA1 signature is outside the RSA modulus")
    verify_signature(region, signature, args.public_key, args.openssl)

    artifact = region + signature
    if len(artifact) != BMA1_SIZE:
        raise AssertionError("assembled BMA1 artifact size changed")
    write_hex(args.output, artifact)
    print(
        "BMA1_OFFLINE_ASSEMBLY_OK "
        f"path={args.output} sequence={sequence} operations=storage-rotation "
        f"max_uses={BMA1_MAX_USES} bytes={len(artifact)} "
        f"signed_sha256={hashlib.sha256(region).hexdigest()} "
        f"authorization_id={authorization_id(sequence).hex()} "
        f"modulus_sha256={modulus_sha256} signature_verified=1 "
        "private_key_consumed=0"
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--sequence", type=int, required=True)
    prepare_parser.add_argument("--request", type=Path, required=True)
    prepare_parser.set_defaults(action=prepare)

    assemble_parser = subparsers.add_parser("assemble")
    assemble_parser.add_argument("--request", type=Path, required=True)
    assemble_parser.add_argument("--signature", type=Path, required=True)
    assemble_parser.add_argument("--public-key", type=Path, required=True)
    assemble_parser.add_argument("--expected-modulus-sha256", required=True)
    assemble_parser.add_argument("--output", type=Path, required=True)
    assemble_parser.add_argument("--openssl", default="openssl")
    assemble_parser.set_defaults(action=assemble)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    return args.action(args)


if __name__ == "__main__":
    raise SystemExit(main())
