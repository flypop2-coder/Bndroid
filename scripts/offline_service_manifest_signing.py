#!/usr/bin/env python3
"""Prepare and assemble a BMS1 artifact across an offline signing boundary.

`prepare` emits the exact 256-byte region that an external signer must sign.
`assemble` accepts only that request, a detached RSA-2048 signature, and the
corresponding public key. It verifies the request and signature before writing
the complete 512-byte artifact. This tool has no private-key input and performs
no signing operation.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import subprocess
import tempfile
from pathlib import Path

from generate_verified_service_manifest_artifact import (
    BMS1_SIGNATURE_SIZE,
    BMS1_SIGNED_SIZE,
    BMS1_SIZE,
    encode_signed_region,
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


def inspect_request(region: bytes) -> tuple[int, int, int]:
    if len(region) != BMS1_SIGNED_SIZE:
        raise ValueError("BMS1 signing request has the wrong size")
    key_id = region[6]
    rollback_index = struct.unpack_from("<I", region, 24)[0]
    generation = struct.unpack_from("<I", region, 32 + 12)[0]
    expected = encode_signed_region(generation, rollback_index, key_id)
    if region != expected:
        raise ValueError("BMS1 signing request is not the canonical product manifest")
    return generation, rollback_index, key_id


def public_modulus(public_key: Path, openssl: str) -> bytes:
    result = subprocess.run(
        [
            openssl,
            "rsa",
            "-pubin",
            "-in",
            str(public_key),
            "-noout",
            "-modulus",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ValueError("OpenSSL rejected the RSA public key")
    prefix = "Modulus="
    line = result.stdout.strip()
    if not line.startswith(prefix):
        raise ValueError("OpenSSL did not return an RSA public modulus")
    try:
        modulus = bytes.fromhex(line[len(prefix) :])
    except ValueError as error:
        raise ValueError("OpenSSL returned a malformed RSA public modulus") from error
    if len(modulus) != BMS1_SIGNATURE_SIZE or modulus[0] & 0x80 == 0 or modulus[-1] & 1 == 0:
        raise ValueError("public key must use an odd, full-width RSA-2048 modulus")
    return modulus


def require_public_exponent_65537(public_key: Path, openssl: str) -> None:
    result = subprocess.run(
        [
            openssl,
            "rsa",
            "-pubin",
            "-in",
            str(public_key),
            "-text",
            "-noout",
        ],
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
    with tempfile.TemporaryDirectory(prefix="bndroid-bms1-assemble-") as directory:
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
        raise ValueError("detached BMS1 signature verification failed")


def prepare(args: argparse.Namespace) -> int:
    region = encode_signed_region(args.generation, args.rollback_index, args.key_id)
    write_hex(args.request, region)
    print(
        "BMS1_OFFLINE_REQUEST_OK "
        f"path={args.request} generation={args.generation} "
        f"rollback_index={args.rollback_index} key_id={args.key_id} "
        f"bytes={len(region)} signed_sha256={hashlib.sha256(region).hexdigest()} "
        "contains_signature=0 contains_private_key=0"
    )
    return 0


def assemble(args: argparse.Namespace) -> int:
    if SHA256_PATTERN.fullmatch(args.expected_modulus_sha256) is None:
        raise ValueError("--expected-modulus-sha256 must be 64 lowercase hex digits")
    region = read_hex(args.request, BMS1_SIGNED_SIZE, "BMS1 signing request")
    signature = read_hex(args.signature, BMS1_SIGNATURE_SIZE, "detached BMS1 signature")
    generation, rollback_index, key_id = inspect_request(region)
    if key_id != args.key_id:
        raise ValueError(
            f"request key id {key_id} does not match required key id {args.key_id}"
        )

    modulus = public_modulus(args.public_key, args.openssl)
    modulus_sha256 = hashlib.sha256(modulus).hexdigest()
    if modulus_sha256 != args.expected_modulus_sha256:
        raise ValueError(
            "public-key modulus digest does not match --expected-modulus-sha256"
        )
    require_public_exponent_65537(args.public_key, args.openssl)
    if int.from_bytes(signature, "big") >= int.from_bytes(modulus, "big"):
        raise ValueError("detached BMS1 signature is outside the RSA modulus")
    verify_signature(region, signature, args.public_key, args.openssl)

    artifact = region + signature
    if len(artifact) != BMS1_SIZE:
        raise AssertionError("assembled BMS1 artifact size changed")
    write_hex(args.output, artifact)
    print(
        "BMS1_OFFLINE_ASSEMBLY_OK "
        f"path={args.output} generation={generation} "
        f"rollback_index={rollback_index} key_id={key_id} "
        f"bytes={len(artifact)} signed_sha256={hashlib.sha256(region).hexdigest()} "
        f"modulus_sha256={modulus_sha256} signature_verified=1 "
        "private_key_consumed=0"
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--generation", type=int, required=True)
    prepare_parser.add_argument("--rollback-index", type=int, required=True)
    prepare_parser.add_argument("--key-id", type=int, required=True)
    prepare_parser.add_argument("--request", type=Path, required=True)
    prepare_parser.set_defaults(action=prepare)

    assemble_parser = subparsers.add_parser("assemble")
    assemble_parser.add_argument("--request", type=Path, required=True)
    assemble_parser.add_argument("--signature", type=Path, required=True)
    assemble_parser.add_argument("--public-key", type=Path, required=True)
    assemble_parser.add_argument("--key-id", type=int, required=True)
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
