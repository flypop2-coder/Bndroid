#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
MANIFEST_PATH="$WORKSPACE_ROOT/user/init/Cargo.toml"
TARGET_TRIPLE="aarch64-unknown-none"
PACKAGE_NAME="bndroid-init"
cd "$WORKSPACE_ROOT"

feature_list_contains() {
  local requested="${1//,/ }"
  local expected="$2"
  local feature
  for feature in $requested; do
    if [[ "$feature" == "$expected" || "$feature" == */"$expected" ]]; then
      return 0
    fi
  done
  return 1
}

PROFILE="${BNDROID_PROFILE:-debug}"
case "$PROFILE" in
  debug)
    ;;
  release)
    ;;
  *)
    echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
    exit 2
    ;;
esac

if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "Userspace manifest not found: $MANIFEST_PATH" >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install the Rust toolchain before building userspace." >&2
  exit 1
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_ROOT/target}"
CARGO_ARGS=(
  build
  --locked
  --manifest-path "$MANIFEST_PATH"
  --package "$PACKAGE_NAME"
  --bins
  --target "$TARGET_TRIPLE"
)
if [[ -n "${BNDROID_USERSPACE_FEATURES:-}" ]]; then
  CARGO_ARGS+=(--features "$BNDROID_USERSPACE_FEATURES")
fi
if [[ "$PROFILE" == "release" ]]; then
  CARGO_ARGS+=(--release)
fi
cargo "${CARGO_ARGS[@]}"

ELF_NAMES=(
  bndroid-init
  bndroid-service-manager
  bndroid-echo-provider
  bndroid-echo-client
  bndroid-surface-server
  bndroid-input-server
  bndroid-launcher
  bndroid-app
)
ELF_VARIABLES=(
  BNDROID_INIT_ELF
  BNDROID_SERVICE_MANAGER_ELF
  BNDROID_ECHO_PROVIDER_ELF
  BNDROID_ECHO_CLIENT_ELF
  BNDROID_SURFACE_SERVER_ELF
  BNDROID_INPUT_SERVER_ELF
  BNDROID_LAUNCHER_ELF
  BNDROID_APP_ELF
)
if feature_list_contains "${BNDROID_USERSPACE_FEATURES:-}" "androidbox-process0"; then
  ELF_NAMES+=(bndroid-android-app)
  ELF_VARIABLES+=(BNDROID_ANDROID_APP_ELF)
fi
if feature_list_contains "${BNDROID_USERSPACE_FEATURES:-}" "storage-server-runtime"; then
  ELF_NAMES+=(bndroid-storage-server)
  ELF_VARIABLES+=(BNDROID_STORAGE_SERVER_ELF)
fi
ELF_PATHS=()

for name in "${ELF_NAMES[@]}"; do
  elf="$CARGO_TARGET_DIR/$TARGET_TRIPLE/$PROFILE/$name"
  if [[ ! -f "$elf" ]]; then
    echo "Cargo succeeded but the expected userspace ELF is missing: $elf" >&2
    exit 1
  fi
  "$SCRIPT_DIR/verify-init-elf.sh" "$elf"
  ELF_PATHS+=("$elf")
done

for ((left = 0; left < ${#ELF_PATHS[@]}; left++)); do
  for ((right = left + 1; right < ${#ELF_PATHS[@]}; right++)); do
    if cmp -s "${ELF_PATHS[$left]}" "${ELF_PATHS[$right]}"; then
      echo "Userspace ELF images must be pairwise distinct: ${ELF_PATHS[$left]} and ${ELF_PATHS[$right]}" >&2
      exit 1
    fi
  done
done

for index in "${!ELF_PATHS[@]}"; do
  echo "${ELF_VARIABLES[$index]}=${ELF_PATHS[$index]}"
done
