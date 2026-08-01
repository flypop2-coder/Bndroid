#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
WORKSPACE_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && /bin/pwd)
export BNDROID_MULTIPACKAGE_GATE_ABI=61
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-activity-state11
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_ANDROID_DEX_METHODS=4
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_ACTIVITY_STATE_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/activity-state11"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_ACTIVITY_STATE_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-activity-state11-build"}"
exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh"
