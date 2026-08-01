#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
export BNDROID_MULTIPACKAGE_GATE_ABI=56
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-icon-resources5
export BNDROID_ANDROID_ICON_RESOURCES=1
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_ICON_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-icon-resources5"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_ICON_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-icon-resources5-build"}"
exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh"
