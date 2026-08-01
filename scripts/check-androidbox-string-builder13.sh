#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

export BNDROID_MULTIPACKAGE_GATE_ABI=63
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-string-builder13
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_STRING_BUILDER_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/string-builder13"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_STRING_BUILDER_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-string-builder13-build"}"
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_ANDROID_DEX_METHODS=6

exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh" "$@"
