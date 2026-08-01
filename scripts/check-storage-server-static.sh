#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found; cannot verify the M55 static contracts." >&2
  exit 1
fi

# Parse every shell entry point before checking the source-level contracts. This
# is deliberately non-executing: `bash -n` never builds or launches QEMU.
for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M55 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(relative: str, needle: str, description: str) -> None:
    if needle not in source(relative):
        raise SystemExit(f"M55 static contract lost {description}: {relative}")


# Every actual shell launch block must opt out of networking explicitly. Match
# only a command at the start of a line, so command discovery and help/error
# text containing the QEMU binary name are not mistaken for launches.
launch_pattern = re.compile(
    r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$"
)
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
qemu_launches = 0
for path in sorted((root / "scripts").glob("*.sh")):
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        if not launch_pattern.match(lines[index]):
            index += 1
            continue
        start = index + 1
        block = [lines[index]]
        while block[-1].rstrip().endswith("\\"):
            index += 1
            if index >= len(lines):
                raise SystemExit(f"unterminated QEMU launch block: {path}:{start}")
            block.append(lines[index])
        qemu_launches += 1
        nic_none = sum(bool(nic_none_pattern.match(line)) for line in block)
        if nic_none != 1:
            raise SystemExit(
                f"QEMU launch must contain exactly one literal '-nic none': "
                f"{path.relative_to(root)}:{start}"
            )
        index += 1
if qemu_launches == 0:
    raise SystemExit("M55 static contract found no QEMU launch blocks")

# ABI 25 remains selected by every historical StorageServer profile when the
# ABI-v26 M65 child is absent. StorageServer identity and the syscall 47--52
# tail stay stable. The request wire is the exact 64-byte header plus eight
# sectors.
require(
    "crates/bndr-abi/src/lib.rs",
    '''#[cfg(all(
    not(feature = "storage-server-shutdown-orchestration-runtime"),
    feature = "storage-server-runtime"
))]
pub const ABI_VERSION: u64 = 25;''',
    "historical feature-qualified ABI version 25",
)
for needle, description in (
    ("pub const IPC_BUFFER_PAYLOAD_MAX_BYTES: usize = 4_160;", "4,160-byte IPC payload bound"),
    ("pub const STORAGE_BLOCK_REQUEST_VERSION: u16 = 2;", "storage request version 2"),
    ("pub const STORAGE_BLOCK_MAX_SECTORS: usize = 8;", "eight-sector batch bound"),
    ("StorageServer = 9,", "StorageServer image identity 9"),
    ("IpcBufferCreate = 47,", "IPC-buffer syscall 47"),
    ("StorageAcquire = 48,", "storage-acquire syscall 48"),
    ("StorageSubmit = 49,", "storage-submit syscall 49"),
    ("StorageTake = 50,", "storage-take syscall 50"),
    ("StorageConnect = 51,", "storage-connect syscall 51"),
    ("StorageAccept = 52,", "storage-accept syscall 52"),
):
    require("crates/bndr-abi/src/lib.rs", needle, description)

# The StorageServer must remain a standalone, feature-gated ELF. The common
# build path must add it to the pairwise-distinct image set and pass it through
# the same strict ELF verifier as every other userspace executable.
for needle, description in (
    ('name = "bndroid-storage-server"', "StorageServer binary target"),
    ('path = "src/bin/bndroid-storage-server.rs"', "StorageServer binary source"),
    ('required-features = ["storage-server-runtime"]', "StorageServer binary feature gate"),
    ('"dep:bndr-storage",', "StorageServer protocol dependency"),
):
    require("user/init/Cargo.toml", needle, description)
for needle, description in (
    ("ELF_NAMES+=(bndroid-storage-server)", "conditional StorageServer ELF selection"),
    ("ELF_VARIABLES+=(BNDROID_STORAGE_SERVER_ELF)", "StorageServer ELF export"),
    ('"$SCRIPT_DIR/verify-init-elf.sh" "$elf"', "strict verification for every selected ELF"),
    ('if cmp -s "${ELF_PATHS[$left]}" "${ELF_PATHS[$right]}"; then', "pairwise-distinct ELF check"),
):
    require("scripts/build-userspace.sh", needle, description)
for needle, description in (
    ('BNDROID_USERSPACE_FEATURES=storage-server-runtime', "M55 userspace feature selection"),
    ('BNDROID_KERNEL_FEATURES=storage-server-runtime', "M55 kernel feature selection"),
    ('"$SCRIPT_DIR/build-kernel.sh"', "M55 shared build/ELF verification path"),
):
    require("scripts/check-storage-server-runtime.sh", needle, description)
require(
    "scripts/build-kernel.sh",
    'export BNDROID_STORAGE_SERVER_ELF="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-storage-server"',
    "kernel StorageServer ELF embedding input",
)
for needle, description in (
    ('("BNDROID_STORAGE_SERVER_ELF", "BNDR_STORAGE_SERVER_ELF")', "kernel build environment mapping"),
    ('("bndroid-storage-server", "bndroid-storage-server.rs")', "kernel fallback binary mapping"),
):
    require("kernel/build.rs", needle, description)

# M55 itself owns the synchronous fail-stop recovery coordinator, not only
# its M56/M57 fault-injection children. Keep the explicit admission primitive
# available to the base StorageServer feature and seal the masked
# rearm -> admission -> broker order plus rollback-on-any-commit-failure.
require(
    "kernel/src/storage.rs",
    '''#[cfg(any(
    feature = "cooperative-block-recovery",
    feature = "storage-server-runtime"
))]
pub(crate) fn open_recovery_admission''',
    "base-StorageServer explicit recovery-admission primitive",
)
storage_server_io = source("kernel/src/storage_server_io.rs")
sync_start = storage_server_io.index(
    '#[cfg(not(feature = "storage-server-async-recovery-runtime"))]\npub fn service_recovery()'
)
sync_end = storage_server_io.index("/// Advances M58 recovery", sync_start)
sync_recovery = storage_server_io[sync_start:sync_end]
for needle, description in (
    ("reenable_block_irq_masked", "synchronous IRQ rearm"),
    ("storage::open_recovery_admission", "synchronous admission open"),
    ("storage_broker::complete_recovery", "synchronous broker commit"),
    ("rollback_block_irq_rearm_masked", "synchronous rollback"),
):
    if sync_recovery.count(needle) != 1:
        raise SystemExit(
            f"M55 synchronous recovery lost exactly one {description}: {needle}"
        )
sync_rearm = sync_recovery.index("reenable_block_irq_masked")
sync_open = sync_recovery.index("storage::open_recovery_admission")
sync_broker = sync_recovery.index("storage_broker::complete_recovery")
sync_rollback = sync_recovery.index("rollback_block_irq_rearm_masked")
if not sync_rearm < sync_open < sync_broker < sync_rollback:
    raise SystemExit(
        "M55 synchronous recovery no longer orders rearm, admission, broker, then rollback guard"
    )
if "let broker_committed = admission_opened" not in sync_recovery:
    raise SystemExit("M55 broker commit no longer depends on successful admission open")

# The feature-qualified catalog is exactly the seven baseline images plus the
# StorageServer in M55; both lookup and snapshot enumeration must include it.
for needle, description in (
    ('static STORAGE_SERVER_ELF: &[u8] = include_bytes!(env!("BNDR_STORAGE_SERVER_ELF"));', "embedded StorageServer bytes"),
    ('+ cfg!(feature = "storage-server-runtime") as usize;', "feature-qualified catalog cardinality"),
    ('UserImageId::StorageServer => STORAGE_SERVER_ELF,', "StorageServer catalog lookup"),
    ('get(UserImageId::StorageServer),', "StorageServer catalog snapshot entry"),
):
    require("kernel/src/user_images.rs", needle, description)

boot_marker = "BOOT_OK: M55 userspace StorageServer ownership, restart, rebind, and AppData I/O verified"
for needle, description in (
    ("STORAGE_SERVER_IO_OK read_batches={}", "M55 I/O marker"),
    ("STORAGE_SERVER_SCHED_OK logical_hz=100", "M55 scheduler marker"),
    ("STORAGE_SERVER_RUNTIME_OK abi={}", "M55 ABI/runtime marker"),
    (boot_marker, "M55 final boot marker"),
    ("USER_IMAGE_CATALOG_OK count={} distinct={}", "catalog evidence marker"),
    ("storage_server_bytes={}", "StorageServer catalog size evidence"),
    ("storage_server_digest={:#018x}", "StorageServer catalog digest evidence"),
):
    require("kernel/src/main.rs", needle, description)
for needle, description in (
    (f'BOOT_MARKER="{boot_marker}"', "checker final boot marker"),
    ("STORAGE_SERVER_RUNTIME_OK abi=25", "checker ABI 25 evidence"),
    ("^STORAGE_SERVER_FAULT_POLICY_OK( |$)", "checker M60 policy-marker rejection"),
    ("^BOOT_OK: M60( |$)", "checker M60 BOOT_OK rejection"),
    ("^USER_IMAGE_CATALOG_OK ", "checker catalog evidence"),
    ("^ELF_LOAD_OK ", "checker ELF-load evidence"),
):
    require("scripts/check-storage-server-runtime.sh", needle, description)

for needle, description in (
    ('"$SCRIPT_DIR/check-storage-server-static.sh"', "static gate suite integration"),
    ('"$SCRIPT_DIR/check-storage-server-runtime.sh"', "runtime gate suite integration"),
):
    require("scripts/test.sh", needle, description)

shell_scripts = len(list((root / "scripts").glob("*.sh")))
print(
    "STORAGE_SERVER_STATIC_OK "
    f"shell_scripts={shell_scripts} qemu_launches={qemu_launches} nic_none={qemu_launches} "
    "abi=25 catalog_images=8 storage_elf_gate=1 markers=4 offline=1"
)
PY
