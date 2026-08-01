#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
WORKSPACE_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && /bin/pwd)
export BNDROID_MULTIPACKAGE_GATE_ABI=57
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-density-icons7
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_DENSITY_ICON_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-density-icons7"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_DENSITY_ICON_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-density-icons7-build"}"
exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh"
