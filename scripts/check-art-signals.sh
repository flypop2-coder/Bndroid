#!/usr/bin/env bash
# Each case owns its temporary storage and QEMU child; stop at first failure.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
for mode in signals signal-bad-frame signal-bad-pc signal-bad-fp signal-guard signal-default; do
  BNDROID_ART_MODE="$mode" "$ROOT/scripts/check-art-runtime.sh"
done
