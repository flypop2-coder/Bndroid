"""Read complete kernel proof records from a concurrently written serial log."""


def parse_marker(text: str, marker: str) -> dict[str, str] | None:
    # QEMU may have written only half of the last UART line. A successful boot
    # is not observable until the record's terminating newline is present.
    lines = text.replace("\r", "").split("\n")[:-1]
    matches = [line for line in lines if line.startswith(marker + " ")]
    if not matches:
        return None
    if len(matches) != 1:
        raise ValueError(f"Multiple {marker} records")
    proof = {}
    for field in matches[0].split()[1:]:
        key, separator, value = field.partition("=")
        if not separator or not key or not value or key in proof:
            raise ValueError(f"Invalid {marker} field: {field!r}")
        proof[key] = value
    return proof
