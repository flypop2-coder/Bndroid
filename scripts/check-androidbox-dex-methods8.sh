#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
WORKSPACE_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && /bin/pwd)
export BNDROID_MULTIPACKAGE_GATE_ABI=58
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-dex-methods8
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_ANDROID_DEX_METHODS=1
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_DEX_METHODS_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-dex-methods8"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_DEX_METHODS_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-dex-methods8-build"}"
exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh"
