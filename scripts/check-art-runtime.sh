#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/lib/storage-evidence.sh"
export BNDROID_ART_MODE="${BNDROID_ART_MODE:-java}"
INPUT_SET=art
[[ "$BNDROID_ART_MODE" != framework* ]] || INPUT_SET=framework
RUNTIME="${BNDROID_ART_RUNTIME_DIR:-$ROOT/.bndroid/runtime-inputs/$INPUT_SET}"
[[ -f "$RUNTIME/art-provenance.json" ]] || { echo "Extract local inputs with --include-$INPUT_SET first" >&2; exit 1; }
RUNTIME="$(cd -- "$RUNTIME" && pwd)"
mkdir -p "$ROOT/.bndroid/art-runtime"
ARTIFACTS="$(mktemp -d "$ROOT/.bndroid/art-runtime/check.XXXXXX")"
export BNDROID_ART_PROBE_DIR="$ARTIFACTS"
"$ROOT/fixtures/art-runtime/build.sh" >"$ARTIFACTS/dex-build.log" 2>&1
python3 "$ROOT/scripts/build-art-rootfs.py" --runtime "$RUNTIME" --dex "$ARTIFACTS/probe.dex" --mode "$BNDROID_ART_MODE" --output "$ARTIFACTS/rootfs.tar" >"$ARTIFACTS/rootfs-build.log"
export BNDROID_LINUX_APP_ELF="$RUNTIME/linker64"
export BNDROID_LINUX_LAUNCH_CONFIG="$ARTIFACTS/rootfs.launch"
FEATURES=art-runtime
USER_FEATURES=art-runtime
if [[ "$BNDROID_ART_MODE" == binder ]]; then
  FEATURES+=,binder-runtime
  USER_FEATURES+=,binder-runtime
fi
MEMORY=512M
if [[ "$BNDROID_ART_MODE" == framework* ]]; then
  FEATURES=framework-runtime
  MEMORY=1G
fi
if [[ "$BNDROID_ART_MODE" == framework-identities ]]; then
  FEATURES+=,linux-launch-runtime
  USER_FEATURES+=,linux-launch-runtime
elif [[ "$BNDROID_ART_MODE" == framework-services ]]; then
  FEATURES+=,service-manager-runtime
  USER_FEATURES+=,service-manager-runtime
elif [[ "$BNDROID_ART_MODE" == framework-bridge ]]; then
  FEATURES+=,service-bridge-runtime
  USER_FEATURES+=,service-bridge-runtime
fi
[[ "${BNDROID_ART_SYSCALL_TRACE:-0}" != 1 ]] || FEATURES+=,linux-syscall-trace
CARGO_TARGET_DIR="$ROOT/target/art-build" BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$FEATURES" BNDROID_USERSPACE_FEATURES="$USER_FEATURES" \
  "$ROOT/scripts/build-kernel.sh" >"$ARTIFACTS/kernel-build.log" 2>&1 || { tail -60 "$ARTIFACTS/kernel-build.log" >&2; exit 1; }
BNDROID_STORAGE_IMAGE="$ARTIFACTS/storage.raw" "$ROOT/scripts/build-storage-image.sh" >"$ARTIFACTS/storage-build.log"
build_storage_qemu_args "$ARTIFACTS/storage.raw" modern writable-temporary
python3 "$ROOT/scripts/art_runtime_evidence.py" "$ARTIFACTS" \
  qemu-system-aarch64 -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m "$MEMORY" -display none -monitor none -nic none \
  -serial "file:$ARTIFACTS/serial.log" -no-reboot \
  -kernel "$ROOT/target/art-build/aarch64-unknown-none/release/bndroid-kernel.img" -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -fw_cfg "name=opt/bndroid/rootfs,file=$ARTIFACTS/rootfs.tar" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on
