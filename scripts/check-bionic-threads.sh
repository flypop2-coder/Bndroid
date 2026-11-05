#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export BNDROID_DYNAMIC_THREADS_CHECK=1
export BNDROID_DYNAMIC_FAULT_CHECK=0
for mode in normal leader-exit group-exit worker-fault; do
  BNDROID_THREAD_TEST_MODE="$mode" "$SCRIPT_DIR/check-bionic-dynamic-runtime.sh"
done
