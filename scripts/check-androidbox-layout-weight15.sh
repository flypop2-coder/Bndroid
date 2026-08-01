#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

export BNDROID_MULTIPACKAGE_GATE_ABI=65
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-layout-weight15
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_LAYOUT_WEIGHT_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/layout-weight15"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_LAYOUT_WEIGHT_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-layout-weight15-build"}"
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_ANDROID_DEX_METHODS=6
export BNDROID_ANDROID_LAYOUT_ROWS=2

exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh" "$@"
