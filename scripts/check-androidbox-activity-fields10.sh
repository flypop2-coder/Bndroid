#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
WORKSPACE_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && /bin/pwd)
export BNDROID_MULTIPACKAGE_GATE_ABI=60
export BNDROID_MULTIPACKAGE_GATE_FEATURES=androidbox-activity-fields10
export BNDROID_ANDROID_ICON_RESOURCES=2
export BNDROID_ANDROID_DEX_METHODS=3
export BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT="${BNDROID_ACTIVITY_FIELDS_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/activity-fields10"}"
export BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT="${BNDROID_ACTIVITY_FIELDS_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-activity-fields10-build"}"
exec "$SCRIPT_DIR/check-androidbox-multipackage4.sh"
