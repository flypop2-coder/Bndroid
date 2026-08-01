#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 mktemp mkdir rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M59 unified recovery contracts." >&2
    exit 1
  fi
done

# Syntax checking is non-executing. In particular it cannot reach any QEMU
# launch in the scripts inspected below.
for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
shared = "cooperative-block-recovery"
appdata = "app-data-runtime"
appdata_async = "app-data-async-recovery-runtime"
storage = "storage-server-runtime"
storage_recovery = "storage-server-recovery-runtime"
storage_repeated = "storage-server-repeated-recovery-runtime"
storage_async = "storage-server-async-recovery-runtime"
storage_fault_policy = "storage-server-fault-policy-runtime"
timeout = "storage-irq-timeout-self-test"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M59 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def feature_dependencies(text: str, name: str, relative: str) -> tuple[str, ...]:
    matches = list(
        re.finditer(
            rf"^{re.escape(name)}\s*=\s*\[(.*?)\]\s*$",
            text,
            flags=re.MULTILINE | re.DOTALL,
        )
    )
    if len(matches) != 1:
        raise SystemExit(f"Cargo feature is not exact and unique: {relative}:{name}")
    body = matches[0].group(1)
    dependencies = tuple(re.findall(r'"([^"]+)"', body))
    residue = re.sub(r'"[^"]+"\s*,?', "", body)
    residue = re.sub(r"#[^\n]*", "", residue)
    if residue.strip():
        raise SystemExit(f"Cargo feature has an unparsed dependency: {relative}:{name}")
    return dependencies


def require_once(text: str, needle: str, description: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M59 static contract requires exactly one {description}: "
            f"found={count} needle={needle!r}"
        )


def cfg_attributes(text: str, symbol: str) -> tuple[str, ...]:
    if text.count(symbol) != 1:
        raise SystemExit(f"M59 shared item is not exact and unique: {symbol}")
    lines = text[: text.index(symbol)].rstrip().splitlines()
    attributes: list[str] = []
    while lines and lines[-1].lstrip().startswith("#["):
        attributes.append(lines.pop().strip())
    return tuple(attributes)


def require_exact_cfg(text: str, symbol: str, expected: str, relative: str) -> None:
    attributes = cfg_attributes(text, symbol)
    if expected not in attributes:
        raise SystemExit(f"M59 item lost its exact cfg: {relative}:{symbol}")
    profile_cfgs = tuple(attribute for attribute in attributes if attribute.startswith("#[cfg("))
    if profile_cfgs != (expected,):
        raise SystemExit(
            f"M59 item gained a foreign profile cfg: {relative}:{symbol}:{profile_cfgs}"
        )


def without_comments(text: str) -> str:
    text = re.sub(r"//[^\n]*", "", text)
    return re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)


def reject_sync_calls(label: str, text: str) -> None:
    code = without_comments(text)
    for function in ("recover_after_timeout", "reset_and_wait", "stable_capacity"):
        if re.search(rf"\b{function}\s*\(", code):
            raise SystemExit(f"{label} calls synchronous recovery function {function}")


relative_sources = (
    "kernel/Cargo.toml",
    "user/init/Cargo.toml",
    "kernel/build.rs",
    "scripts/build-kernel.sh",
    "kernel/src/driver/virtio/block.rs",
    "kernel/src/storage.rs",
    "kernel/src/app_data.rs",
    "kernel/src/storage_server_io.rs",
    "kernel/src/main.rs",
    "user/init/src/multi_window_runtime.rs",
)
texts = {relative: source(relative) for relative in relative_sources}
kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]

# The two M59 leaf parsers and the shared AppData runtime wrapper must reject
# the newer M60 success evidence explicitly, not merely through a coincidental
# schema mismatch. Their parser self-tests below exercise both negative cases.
for relative in (
    "scripts/check-app-data-async-recovery-runtime.sh",
    "scripts/check-app-data-runtime.sh",
    "scripts/check-storage-irq-race.sh",
):
    checker = source(relative)
    for needle in ("STORAGE_SERVER_FAULT_POLICY_OK", "BOOT_OK: M60"):
        if needle not in checker:
            raise SystemExit(f"M59 checker lost explicit M60 isolation: {relative}:{needle}")

# Cargo is the authority for profile inheritance. The common feature contains
# only the physical cooperative engine. M58, AppData, and the low-level timeout
# profile select that engine without inheriting one another's coordinator.
expected_kernel_features = {
    shared: (),
    appdata: (
        "post-recovery-lifecycle-focus-runtime",
        shared,
        "bndr-abi/app-data-runtime",
        "dep:bndr-appdata",
    ),
    appdata_async: (appdata,),
    storage: ("bndr-abi/storage-server-runtime",),
    storage_recovery: (storage,),
    storage_repeated: (storage_recovery,),
    storage_async: (storage_repeated, shared),
    storage_fault_policy: (storage_async,),
    timeout: (shared,),
}
for feature, expected in expected_kernel_features.items():
    actual = feature_dependencies(kernel_cargo, feature, "kernel/Cargo.toml")
    if actual != expected:
        raise SystemExit(
            f"kernel feature closure changed: {feature}: expected={expected} actual={actual}"
        )

expected_user_features = {
    appdata: (
        "post-recovery-lifecycle-focus-runtime",
        "bndr-abi/app-data-runtime",
    ),
    appdata_async: (appdata,),
    storage: (
        "bndr-abi/storage-server-runtime",
        "dep:bndr-appdata",
        "dep:bndr-storage",
    ),
    storage_recovery: (storage,),
    storage_repeated: (storage_recovery,),
    storage_async: (storage_repeated,),
    storage_fault_policy: (storage_async,),
}
for feature, expected in expected_user_features.items():
    actual = feature_dependencies(user_cargo, feature, "user/init/Cargo.toml")
    if actual != expected:
        raise SystemExit(
            f"userspace feature closure changed: {feature}: expected={expected} actual={actual}"
        )
if shared in user_cargo or timeout in user_cargo:
    raise SystemExit("kernel-only recovery features leaked into userspace Cargo features")

# build.rs may activate the low-level validator only for its isolated kernel:
# no AppData runtime and no StorageServer runtime of any generation. It also
# mirrors the AppData child cfg into the manually compiled userspace images.
build_rs = texts["kernel/build.rs"]
for environment in (
    "CARGO_FEATURE_COOPERATIVE_BLOCK_RECOVERY",
    "CARGO_FEATURE_APP_DATA_ASYNC_RECOVERY_RUNTIME",
    "CARGO_FEATURE_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
    "CARGO_FEATURE_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
    "CARGO_FEATURE_STORAGE_IRQ_TIMEOUT_SELF_TEST",
):
    require_once(
        build_rs,
        f'cargo:rerun-if-env-changed={environment}',
        f"build.rs rerun declaration for {environment}",
    )
profile_match = re.search(
    r"if\s+(.*?)\s*\{\s*println!\(\"cargo:rustc-cfg=bndroid_storage_irq_timeout_profile\"\);\s*\}",
    build_rs,
    flags=re.DOTALL,
)
if profile_match is None:
    raise SystemExit("build.rs lost the effective low-level timeout cfg")
profile_terms = tuple(
    (name, state)
    for name, state in re.findall(
        r'env::var_os\("CARGO_FEATURE_([A-Z0-9_]+)"\)\.is_(some|none)\(\)',
        profile_match.group(1),
    )
)
expected_profile_terms = (
    ("STORAGE_IRQ_TIMEOUT_SELF_TEST", "some"),
    ("STORAGE_SERVER_RUNTIME", "none"),
    ("APP_DATA_RUNTIME", "none"),
)
if profile_terms != expected_profile_terms or profile_match.group(1).count("&&") != 2:
    raise SystemExit(
        "build.rs timeout cfg is not exactly timeout && !StorageServer && !AppData: "
        f"{profile_terms}"
    )
require_once(
    build_rs,
    'feature=\\\"app-data-async-recovery-runtime\\\"',
    "userspace AppData async cfg forwarding",
)

# The shell wrapper computes the same closure before invoking Cargo, rejects
# incompatible ledgers before any build, and mirrors each runtime feature once.
wrapper = texts["scripts/build-kernel.sh"]
for variable in (
    "KERNEL_APP_DATA_RUNTIME",
    "KERNEL_APP_DATA_ASYNC_RECOVERY_RUNTIME",
    "KERNEL_STORAGE_IRQ_TIMEOUT_SELF_TEST",
    "KERNEL_STORAGE_SERVER_RUNTIME",
    "KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME",
    "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
    "KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
    "KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
    "USER_APP_DATA_RUNTIME",
    "USER_APP_DATA_ASYNC_RECOVERY_RUNTIME",
    "USER_STORAGE_SERVER_RUNTIME",
    "USER_STORAGE_SERVER_RECOVERY_RUNTIME",
    "USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
    "USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
    "USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
):
    require_once(wrapper, f"{variable}=0", f"zero initialization of {variable}")

closure_patterns = {
    "kernel AppData child": (
        r'if feature_list_contains "\$KERNEL_FEATURES" "app-data-async-recovery-runtime"; then\s+'
        r'KERNEL_APP_DATA_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_APP_DATA_RUNTIME=1'
    ),
    "userspace AppData child": (
        r'if feature_list_contains "\$USERSPACE_FEATURES" "app-data-async-recovery-runtime"; then\s+'
        r'USER_APP_DATA_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'USER_APP_DATA_RUNTIME=1'
    ),
    "kernel M58 chain": (
        r'if feature_list_contains "\$KERNEL_FEATURES" "storage-server-async-recovery-runtime"; then\s+'
        r'KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RUNTIME=1'
    ),
    "kernel M60 chain": (
        r'if feature_list_contains "\$KERNEL_FEATURES" "storage-server-fault-policy-runtime"; then\s+'
        r'KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RUNTIME=1'
    ),
    "userspace M58 chain": (
        r'if feature_list_contains "\$USERSPACE_FEATURES" "storage-server-async-recovery-runtime"; then\s+'
        r'USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RUNTIME=1'
    ),
    "userspace M60 chain": (
        r'if feature_list_contains "\$USERSPACE_FEATURES" "storage-server-fault-policy-runtime"; then\s+'
        r'USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RUNTIME=1'
    ),
}
for description, pattern in closure_patterns.items():
    if re.search(pattern, wrapper) is None:
        raise SystemExit(f"build-kernel lost its exact {description} closure")

for mirrored in (
    appdata,
    appdata_async,
    storage,
    storage_recovery,
    storage_repeated,
    storage_async,
    storage_fault_policy,
):
    require_once(
        wrapper,
        f'append_feature "$USERSPACE_FEATURES" "{mirrored}"',
        f"userspace mirroring of {mirrored}",
    )

def require_wrapper_guard(left: str, right: str, description: str) -> None:
    patterns = (
        rf'"\${left}" -eq 1\s+\\\s*\n\s*&& "\${right}" -eq 1',
        rf'"\${right}" -eq 1\s+\\\s*\n\s*&& "\${left}" -eq 1',
    )
    if not any(re.search(pattern, wrapper) for pattern in patterns):
        raise SystemExit(f"build-kernel lost the {description} mutual exclusion")


require_wrapper_guard(
    "KERNEL_APP_DATA_RUNTIME",
    "KERNEL_STORAGE_SERVER_RUNTIME",
    "any-AppData/StorageServer",
)
require_wrapper_guard(
    "KERNEL_STORAGE_IRQ_TIMEOUT_SELF_TEST",
    "KERNEL_APP_DATA_RUNTIME",
    "timeout/any-AppData",
)
require_wrapper_guard(
    "KERNEL_STORAGE_IRQ_TIMEOUT_SELF_TEST",
    "KERNEL_STORAGE_SERVER_RUNTIME",
    "timeout/any-StorageServer",
)
if re.search(
    r'"\$USER_APP_DATA_ASYNC_RECOVERY_RUNTIME" -eq 1\s+\\\s*\n\s*'
    r'&& "\$KERNEL_APP_DATA_ASYNC_RECOVERY_RUNTIME" -ne 1',
    wrapper,
) is None:
    raise SystemExit("build-kernel lost the userspace-above-kernel AppData-child rejection")

# The driver and storage facade are selected only by the dependency-free
# shared feature. No branch-specific feature may guard these reusable items.
driver = texts["kernel/src/driver/virtio/block.rs"]
storage_rs = texts["kernel/src/storage.rs"]
shared_cfg = '#[cfg(feature = "cooperative-block-recovery")]'
for relative, text, symbol in (
    ("kernel/src/driver/virtio/block.rs", driver, "pub enum CooperativeRecoveryProgress"),
    ("kernel/src/driver/virtio/block.rs", driver, "pub fn begin_cooperative_recovery"),
    ("kernel/src/driver/virtio/block.rs", driver, "pub fn poll_cooperative_recovery"),
    ("kernel/src/driver/virtio/block.rs", driver, "pub const fn cooperative_recovery_phase"),
    ("kernel/src/driver/virtio/block.rs", driver, "fn begin_cooperative_cleanup"),
    ("kernel/src/driver/virtio/block.rs", driver, "fn observe_recovery_capacity_once"),
    ("kernel/src/driver/virtio/block.rs", driver, "const fn map_block_recovery_transition_error"),
    ("kernel/src/storage.rs", storage_rs, "pub struct AsyncRecoverySnapshot"),
    ("kernel/src/storage.rs", storage_rs, "pub fn begin_async_recovery"),
    ("kernel/src/storage.rs", storage_rs, "pub fn poll_async_recovery"),
    ("kernel/src/storage.rs", storage_rs, "pub fn async_recovery_active"),
    ("kernel/src/storage.rs", storage_rs, "pub fn async_recovery_snapshot"),
    ("kernel/src/storage.rs", storage_rs, "fn with_async_recovery_device"),
    ("kernel/src/storage.rs", storage_rs, "fn publish_async_recovery_progress"),
    ("kernel/src/storage.rs", storage_rs, "const fn async_recovery_phase_code"),
):
    require_exact_cfg(text, symbol, shared_cfg, relative)

explicit_admission_signature = '''#[cfg(any(
    feature = "cooperative-block-recovery",
    feature = "storage-server-runtime"
))]
pub(crate) fn open_recovery_admission'''
if storage_rs.count(explicit_admission_signature) != 1:
    raise SystemExit(
        "the explicit admission gate is not shared by cooperative and every StorageServer profile"
    )

for atomic in (
    "static ASYNC_RECOVERY_ACTIVE",
    "static ASYNC_RECOVERY_PHASE",
    "static ASYNC_RECOVERY_STEPS",
    "static ASYNC_RECOVERY_PENDING_RETURNS",
    "static ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS",
):
    require_exact_cfg(storage_rs, atomic, shared_cfg, "kernel/src/storage.rs")

# The shared storage facade advances one masked device step per call. Physical
# IRQ commit never opens admission as a side effect; every policy coordinator
# must do so explicitly in its own masked transaction.
facade_slice = storage_rs[
    storage_rs.index("pub fn begin_async_recovery") :
    storage_rs.index("pub fn recover_after_timeout", storage_rs.index("pub fn begin_async_recovery"))
]
for call in ("device.begin_cooperative_recovery", "device.poll_cooperative_recovery"):
    require_once(facade_slice, call, f"shared facade call {call}")
reject_sync_calls("shared storage facade", facade_slice)
commit_irq_slice = storage_rs[
    storage_rs.index("pub(crate) fn commit_irq_rearm") :
    storage_rs.index("pub(crate) fn rollback_irq_rearm", storage_rs.index("pub(crate) fn commit_irq_rearm"))
]
if "RECOVERY_ADMISSION_CLOSED.store(false" in commit_irq_slice:
    raise SystemExit("physical IRQ commit can still reopen admission implicitly")

# AppData owns its own policy coordinator. Inspect only that coordinator slice:
# one begin, one poll, no calls into the synchronous reset/capacity path.
app_data_rs = texts["kernel/src/app_data.rs"]
appdata_recovery_slice = app_data_rs[
    app_data_rs.index("fn start_async_recovery_attempt") :
    app_data_rs.index("fn finalize_service_irq_masked", app_data_rs.index("fn start_async_recovery_attempt"))
]
require_once(
    appdata_recovery_slice,
    "storage::begin_async_recovery",
    "AppData cooperative begin call",
)
require_once(
    appdata_recovery_slice,
    "storage::poll_async_recovery",
    "AppData cooperative poll call",
)
reject_sync_calls("AppData recovery coordinator", appdata_recovery_slice)
appdata_commit = appdata_recovery_slice[
    appdata_recovery_slice.index("fn commit_async_recovery") :
    appdata_recovery_slice.index("fn fail_async_recovery_attempt")
]
require_once(
    appdata_commit,
    "reenable_block_irq_masked",
    "AppData explicit IRQ rearm",
)
require_once(
    appdata_commit,
    "storage::open_recovery_admission",
    "AppData explicit admission open",
)
appdata_rearm = appdata_commit.index("reenable_block_irq_masked")
appdata_open = appdata_commit.index("storage::open_recovery_admission")
appdata_finalize = appdata_commit.index("finalize_service_irq_masked")
if not appdata_rearm < appdata_open < appdata_finalize:
    raise SystemExit("AppData commit no longer rearms IRQ before explicitly opening admission")
for needle in (
    "rollback_block_irq_rearm_masked",
    "ASYNC_COORDINATOR_ACTIVE.store(false",
):
    if needle not in appdata_commit:
        raise SystemExit(f"M59 AppData commit lost {needle!r}")

# The ABI-v23 low-level validator uses the same facade under its effective cfg,
# but owns a separate ledger and makes no EL0-progress claim.
main_rs = texts["kernel/src/main.rs"]
require_once(
    main_rs,
    '#[cfg(feature = "storage-server-runtime")]\nmod storage_server_io;',
    "StorageServer-only coordinator module gate",
)
require_once(
    main_rs,
    "APPDATA_ASYNC_RECOVERY_OK abi={}",
    "M59 AppData recovery marker",
)
require_once(
    main_rs,
    "unavailable_read_retries=1 blind_mutation_retries=0",
    "M59 read-only retry marker fields",
)
timeout_cfg = "#[cfg(bndroid_storage_irq_timeout_profile)]"
timeout_declaration = timeout_cfg + "\nfn validate_irq_virtio_block_storage"
require_once(
    main_rs,
    timeout_declaration,
    "effective-cfg low-level timeout validator",
)
timeout_start = main_rs.index(timeout_declaration) + len(timeout_cfg) + 1
timeout_slice = main_rs[
    timeout_start :
    main_rs.index("fn storage_timeout_recovery_fail", timeout_start)
]
require_once(
    timeout_slice,
    "storage::begin_async_recovery",
    "low-level timeout cooperative begin call",
)
require_once(
    timeout_slice,
    "storage::poll_async_recovery",
    "low-level timeout cooperative poll call",
)
reject_sync_calls("low-level timeout validator", timeout_slice)
require_once(
    timeout_slice,
    "interrupt::reenable_block_irq_masked",
    "low-level timeout explicit IRQ rearm",
)
require_once(
    timeout_slice,
    "storage::open_recovery_admission",
    "low-level timeout explicit admission open",
)
timeout_rearm = timeout_slice.index("interrupt::reenable_block_irq_masked")
timeout_open = timeout_slice.index("storage::open_recovery_admission")
if timeout_rearm >= timeout_open:
    raise SystemExit("low-level timeout commit opens admission before IRQ rearm")
for needle in (
    "interrupt::rollback_block_irq_rearm_masked",
    "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK starts=1 physical=1",
    "long_daif_masks=0 final_active=0 gate_open=1 el0_progress_claim=0",
    "STORAGE_IRQ_RACE_OK timeout_requests=2",
):
    if needle not in timeout_slice:
        raise SystemExit(f"low-level timeout evidence lost {needle!r}")

# M58 policy remains isolated in storage_server_io. Merely selecting the shared
# engine cannot compile or call this broker/owner coordinator.
coordinator = texts["kernel/src/storage_server_io.rs"]
for forbidden_cfg in (shared, appdata_async, "bndroid_storage_irq_timeout_profile"):
    if forbidden_cfg in coordinator:
        raise SystemExit(f"M58 coordinator leaked under foreign cfg {forbidden_cfg}")
async_pattern = re.compile(
    r'#\[cfg\(all\(\s*feature = "storage-server-async-recovery-runtime",\s*'
    r'not\(feature = "storage-server-fault-policy-runtime"\)\s*\)\)\]\s*'
    r'pub fn service_recovery\(\)'
)
fault_policy_pattern = re.compile(
    r'#\[cfg\(feature = "storage-server-fault-policy-runtime"\)\]\s*'
    r'pub fn service_recovery\(\)'
)
sync_pattern = re.compile(
    r'#\[cfg\(not\(feature = "storage-server-async-recovery-runtime"\)\)\]\s*'
    r'pub fn service_recovery\(\)'
)
if (
    len(async_pattern.findall(coordinator)) != 1
    or len(fault_policy_pattern.findall(coordinator)) != 1
    or len(sync_pattern.findall(coordinator)) != 1
):
    raise SystemExit("StorageServer recovery coordinators are not exactly split at M58/M60")
m58_slice = coordinator[
    coordinator.index("/// Advances M58 recovery") :
    coordinator.index("fn active_fault_policy_ticket", coordinator.index("/// Advances M58 recovery"))
]
for call in ("storage::begin_async_recovery", "storage::poll_async_recovery"):
    require_once(m58_slice, call, f"M58 coordinator call {call}")
reject_sync_calls("M58 StorageServer coordinator", m58_slice)
require_once(m58_slice, "reenable_block_irq_masked", "M58 explicit IRQ rearm")
require_once(
    m58_slice,
    "storage::open_recovery_admission",
    "M58 explicit admission open",
)
m58_rearm = m58_slice.index("reenable_block_irq_masked")
m58_open = m58_slice.index("storage::open_recovery_admission")
if m58_rearm >= m58_open:
    raise SystemExit("M58 commit opens admission before IRQ rearm")

m60_slice = coordinator[
    coordinator.index("/// M60 adds a boot-local bounded policy") :
    coordinator.index("fn record_async_control_masked_since", coordinator.index("/// M60 adds a boot-local bounded policy"))
]
for call in ("storage::begin_async_recovery", "storage::poll_async_recovery"):
    require_once(m60_slice, call, f"M60 coordinator call {call}")
reject_sync_calls("M60 StorageServer coordinator", m60_slice)
m60_rearm = m60_slice.index("reenable_block_irq_masked")
m60_open = m60_slice.index("storage::open_recovery_admission")
m60_broker = m60_slice.index("storage_broker::complete_recovery")
if not m60_rearm < m60_open < m60_broker:
    raise SystemExit("M60 commit no longer orders rearm, admission, then broker")

# Historical M56/M57 keep their synchronous physical reset ledger, but their
# policy commit now follows the same explicit rearm -> admission -> broker
# order. This also makes an incidental shared-engine feature fail-safe instead
# of leaving the gate permanently closed.
legacy_slice = coordinator[
    coordinator.index("#[cfg(not(feature = \"storage-server-async-recovery-runtime\"))]") :
    coordinator.index("/// Advances M58 recovery")
]
require_once(
    legacy_slice,
    "storage::open_recovery_admission",
    "legacy StorageServer explicit admission open",
)
legacy_rearm = legacy_slice.index("reenable_block_irq_masked")
legacy_open = legacy_slice.index("storage::open_recovery_admission")
legacy_broker = legacy_slice.index("storage_broker::complete_recovery")
if not legacy_rearm < legacy_open < legacy_broker:
    raise SystemExit(
        "legacy StorageServer commit no longer orders rearm, admission, then broker"
    )

# Userspace grants one and only one zero-output Unavailable retry, globally and
# locally, and only to read-only FileOpenAt. Mutation syscall numbers therefore
# take the ordinary terminal-return path. The runtime must assert the count.
user_runtime = texts["user/init/src/multi_window_runtime.rs"]
retry_slice = user_runtime[
    user_runtime.index("fn retry_app_data_syscall") :
    user_runtime.index("fn expect_app_data_error", user_runtime.index("fn retry_app_data_syscall"))
]
for needle in (
    '#[cfg(feature = "app-data-async-recovery-runtime")]\n'
    "    let mut unavailable_retry_available = number == SyscallNumber::FileOpenAt;",
    "result.status == Status::Unavailable.raw()",
    "result.out1 == 0",
    "result.out2 == 0",
    ".compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)",
    "unavailable_retry_available = false;",
):
    require_once(retry_slice, needle, f"read-only retry condition {needle!r}")
for mutation in (
    "SyscallNumber::FileReplaceAt",
    "SyscallNumber::DirectoryCreateAt",
    "SyscallNumber::UnlinkAt",
):
    if mutation in retry_slice:
        raise SystemExit(f"mutation syscall gained an Unavailable retry branch: {mutation}")
require_once(
    user_runtime,
    "if app_data_unavailable_retries() != 1",
    "end-of-run AppData retry assertion",
)
require_once(
    user_runtime,
    "static APP_DATA_UNAVAILABLE_RETRIES: AtomicU64 = AtomicU64::new(0);",
    "global AppData Unavailable retry ledger",
)

# Every repository QEMU launch is part of the same offline authority boundary.
# A launch block has exactly one NIC option, and that option is `-nic none`.
launch_pattern = re.compile(r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$")
nic_pattern = re.compile(r"^\s*-nic(?:\s|$)")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
launches = 0
for path in sorted((root / "scripts").glob("*.sh")):
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        if not launch_pattern.match(lines[index]):
            index += 1
            continue
        block = [lines[index]]
        while block[-1].rstrip().endswith("\\"):
            index += 1
            if index >= len(lines):
                raise SystemExit(f"unterminated QEMU launch block: {path.relative_to(root)}")
            block.append(lines[index])
        launches += 1
        nic_lines = sum(bool(nic_pattern.match(line)) for line in block)
        nic_none = sum(bool(nic_none_pattern.match(line)) for line in block)
        if nic_lines != 1 or nic_none != 1:
            raise SystemExit(
                "QEMU launch must contain exactly one NIC option, `-nic none`: "
                f"{path.relative_to(root)}"
            )
        index += 1
if launches == 0:
    raise SystemExit("M59 static contract found no QEMU launch blocks")

print(
    "STORAGE_RECOVERY_UNIFICATION_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} qemu_launches={launches} nic_none={launches} "
    "shared_engine=1 appdata_coordinator=1 timeout_coordinator=1 m58_coordinator=1 m60_coordinator=1 "
    "read_only_retry=1 mutation_retries=0"
)
PY

# Exercise the real pre-Cargo rejection paths. These invalid profiles terminate
# in build-kernel.sh with status 2, so no compiler or QEMU process is started.
mkdir -p "$WORKSPACE_ROOT/target"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-m59-static.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

expect_profile_rejection() {
  local kernel_features="$1"
  local userspace_features="$2"
  local expected="$3"
  local output status
  set +e
  output="$(
    CARGO_TARGET_DIR="$TMP_DIR" \
      CARGO_NET_OFFLINE=true \
      BNDROID_KERNEL_FEATURES="$kernel_features" \
      BNDROID_USERSPACE_FEATURES="$userspace_features" \
      "$SCRIPT_DIR/build-kernel.sh" 2>&1
  )"
  status=$?
  set -e
  if [[ "$status" -ne 2 || "$output" != *"$expected"* ]]; then
    echo "$output" >&2
    echo "build-kernel did not reject the expected M59 feature mismatch." >&2
    exit 1
  fi
}

expect_profile_rejection \
  "app-data-runtime,storage-server-runtime" \
  "" \
  "AppData runtime and StorageServer runtime profiles are mutually exclusive."
expect_profile_rejection \
  "storage-irq-timeout-self-test,app-data-runtime" \
  "" \
  "AppData runtime and low-level timeout self-test profiles are mutually exclusive."
expect_profile_rejection \
  "storage-irq-timeout-self-test,storage-server-runtime" \
  "" \
  "low-level timeout self-test and StorageServer runtime profiles are mutually exclusive."
expect_profile_rejection \
  "app-data-runtime" \
  "app-data-async-recovery-runtime" \
  "userspace AppData async-recovery profile exceeds the kernel AppData profile."
expect_profile_rejection \
  "storage-server-async-recovery-runtime" \
  "storage-server-fault-policy-runtime" \
  "userspace fault-policy profile exceeds the kernel storage profile."

# Both modes are parser-only: they do not build and do not launch QEMU.
"$SCRIPT_DIR/check-app-data-async-recovery-runtime.sh" --parser-self-test
"$SCRIPT_DIR/check-storage-irq-race.sh" --parser-self-test

echo "STORAGE_RECOVERY_UNIFICATION_STATIC_OK feature_mismatch_cases=5 parser_self_tests=2 shared_engine=1 explicit_gate_commits=5 offline=1"
