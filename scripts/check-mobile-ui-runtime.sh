#!/usr/bin/env bash
set -euo pipefail

# Offline, local-QEMU acceptance gate for the 720x1600 mobile UI preview.
#
# This deliberately tests the system-navigation boundary through QMP tablet
# input: apps never receive or interpret the bottom navigation gesture.  The
# resulting Overview is a single, session-local app identity card; it is not a
# task manager, thumbnail store, or background-execution claim.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep awk; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot verify the local mobile UI preview." >&2
    exit 1
  }
done

TARGET_ROOT="${BNDROID_MOBILE_TARGET_DIR:-$WORKSPACE_ROOT/target/mobile-ui-build}"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
STORAGE_IMAGE="$TARGET_ROOT/bndroid-storage-m25.raw"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-60}"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"

[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

if [[ "${BNDROID_SKIP_BUILD:-0}" != "1" ]]; then
  CARGO_TARGET_DIR="$TARGET_ROOT" BNDROID_PROFILE=release \
    BNDROID_KERNEL_FEATURES=mobile-ui-runtime \
    BNDROID_USERSPACE_FEATURES=mobile-ui-runtime \
    "$SCRIPT_DIR/build-kernel.sh" >/dev/null
  BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
fi

[[ -f "$KERNEL_IMAGE" && -x "$QMP_HELPER" ]] || {
  echo "Mobile preview kernel or QMP helper is missing." >&2
  exit 1
}
validate_storage_image_geometry "$STORAGE_IMAGE" || {
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
}
build_storage_qemu_args "$STORAGE_IMAGE" modern

mkdir -p "$WORKSPACE_ROOT/target/mobile-ui"
ARTIFACT_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/mobile-ui/check-overview.XXXXXX")"
SERIAL_LOG="$ARTIFACT_DIR/serial.log"
NORMALIZED_LOG="$ARTIFACT_DIR/serial.normalized.log"
QEMU_LOG="$ARTIFACT_DIR/qemu.log"
QMP_SOCKET="$ARTIFACT_DIR/qmp.sock"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
}
normalize_log() { tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"; }
show_failure() {
  normalize_log
  tail -n 180 "$NORMALIZED_LOG" >&2
  tail -n 80 "$QEMU_LOG" >&2
}
reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|panicked at|boot error:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)|FRAMEBUFFER_FAIL:|INPUT_FAIL:|UI_FAIL:|USER_FAIL:|EL0_FAIL' "$NORMALIZED_LOG"; then
    show_failure
    echo "The mobile UI preview emitted a fatal runtime failure." >&2
    exit 1
  fi
}
wait_for_pattern() {
  local pattern="$1" description="$2" deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    normalize_log
    reject_failure
    if grep -Eq "$pattern" "$NORMALIZED_LOG"; then return; fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      show_failure
      echo "QEMU exited while waiting for $description." >&2
      exit 1
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
}
line_count() {
  local pattern="$1"
  normalize_log
  grep -Ec "$pattern" "$NORMALIZED_LOG" || true
}
wait_for_count_gt() {
  local pattern="$1" before="$2" description="$3" deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    normalize_log
    reject_failure
    local now
    now=$(grep -Ec "$pattern" "$NORMALIZED_LOG" || true)
    if (( now > before )); then return; fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
}
wait_for_new_commit() {
  local before
  before=$(line_count '^USER_SURFACE_BUFFER_COMMIT_OK ')
  wait_for_count_gt '^USER_SURFACE_BUFFER_COMMIT_OK ' "$before" "a rendered mobile frame"
}
wait_for_launcher_commit_after() {
  local before="$1"
  wait_for_count_gt "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} " "$before" "a settled Launcher frame"
}
wait_for_app_commit_after() {
  local before="$1"
  wait_for_count_gt "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${APP_PID} " "$before" "a settled App frame"
}
snapshot() {
  local name="$1"
  "$QMP_HELPER" "$QMP_SOCKET" move 719 1599
  "$QMP_HELPER" "$QMP_SOCKET" screenshot "$ARTIFACT_DIR/$name.ppm"
}
snapshot_current_contact() {
  local name="$1"
  "$QMP_HELPER" "$QMP_SOCKET" screenshot "$ARTIFACT_DIR/$name.ppm"
}
wait_for_overview_frame() {
  local identity="$1" output="$2" deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    # Normalize the trusted cursor before retaining a pixel artifact. The
    # scene digest excludes cursor position, while raw screenshots do not.
    "$QMP_HELPER" "$QMP_SOCKET" move 719 1599
    "$QMP_HELPER" "$QMP_SOCKET" screenshot "$output"
    if python3 - "$output" "$identity" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
identity = sys.argv[2]
parts = path.read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
def pixel(x, y):
    offset = (y * 720 + x) * 3
    return tuple(pixels[offset:offset + 3])

def blend(base, overlay, alpha):
    return tuple((left * (255 - alpha) + right * alpha) // 255 for left, right in zip(base, overlay))

ocean = (78, 127, 255)
panel = blend((13, 22, 41), ocean, 12)
surface = blend((24, 34, 56), ocean, 14)

# Full Overview has the semantic Ocean-dark panel at this unadorned point.
# The empty identity additionally has the matching surface inside its Info
# disc; Phone has the Phone-green icon field at the same point. Deriving the
# two palette values mirrors the renderer's integer blend without weakening
# the exact stable-frame predicate.
if pixel(360, 300) != panel:
    raise SystemExit(1)
expected = surface if identity == "empty" else (49, 200, 90)
raise SystemExit(0 if pixel(360, 580) == expected else 1)
PY
    then
      return
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for the stable $identity Overview pixels." >&2
  exit 1
}
wait_for_home_frame() {
  local output="$1" deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    "$QMP_HELPER" "$QMP_SOCKET" move 719 1599
    "$QMP_HELPER" "$QMP_SOCKET" screenshot "$output"
    if python3 - "$output" <<'PY'
from pathlib import Path
import sys

parts = Path(sys.argv[1]).read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)

def pixel(x, y):
    offset = (y * 720 + x) * 3
    return tuple(pixels[offset:offset + 3])

# These three unadorned points cover wallpaper, content, and dock. Requiring
# the complete default Home palette prevents an unlocked state notification
# from racing ahead of the Launcher frame that visually implements it.
expected = ((360, 300, (23, 40, 83)), (360, 580, (13, 23, 49)), (360, 1400, (35, 52, 88)))
raise SystemExit(0 if all(pixel(x, y) == color for x, y, color in expected) else 1)
PY
    then
      return
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for stable Home pixels." >&2
  exit 1
}
nav_gesture() {
  local end_y="$1"
  "$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
  "$QMP_HELPER" "$QMP_SOCKET" touch-move 360 "$end_y"
  "$QMP_HELPER" "$QMP_SOCKET" touch-up
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
  -nic none \
  -rtc base=2026-07-29T09:41:50,clock=vm \
  -serial "file:$SERIAL_LOG" \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -no-reboot -kernel "$KERNEL_IMAGE" -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  >"$QEMU_LOG" 2>&1 &
QEMU_PID=$!

wait_for_pattern '^MOBILE_UI_PREVIEW_OK ' "the isolated mobile preview readiness marker"
wait_for_pattern '^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=[0-9]+ ' "the initial Launcher frame"
normalize_log
LAUNCHER_PID=$(awk '/^USER_SURFACE_BUFFER_COMMIT_OK / { for (i = 1; i <= NF; i++) if ($i ~ /^producer_pid=/) { sub(/^producer_pid=/, "", $i); print $i; exit } }' "$NORMALIZED_LOG")
[[ "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ ]] || {
  echo "Could not derive the Launcher frame producer." >&2
  exit 1
}
"$QMP_HELPER" "$QMP_SOCKET" probe
wait_for_pattern '^POINTER_EVENT_OK ' "the canonical tablet probe"
wait_for_pattern '^USER_INPUT_READ_OK .* pressed=0 ' "the drained tablet probe"
# Start close enough to a minute edge that this gate proves the real bounded
# SurfaceServer timeout and PL031 reread without sleeping for a full minute.
# The initial frame must still be 09:41; after the authenticated revision-2
# broadcast, the active Launcher must commit the 09:42 Lock with only its two
# declared clock regions.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
snapshot lock-clock-before
wait_for_pattern '^MOBILE_UI_CLOCK_ADVANCE_OK .* direction=forward$' "the QEMU PL031 minute rollover"
wait_for_pattern '^UI_CLOCK_CHANGED_OK .* receiver_image=launcher .* revision=2$' "the Launcher minute revision"
wait_for_pattern '^UI_CLOCK_CHANGED_OK .* receiver_image=app .* revision=2$' "the App minute revision snapshot"
wait_for_launcher_commit_after "$launcher_before"
snapshot lock

# The boot-local System UI notice is actionable on Lock without becoming an
# unlock or navigation surface. First prove a 159px swipe renders a held frame
# and returns to the byte-identical Lock. Then cross the exact 160px threshold
# and require the one-way SurfaceServer state to reach both clients before the
# dismissed Lock is retained.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 770
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-move 519 770
wait_for_launcher_commit_after "$launcher_before"
snapshot_current_contact lock-notification-short-drag
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_launcher_commit_after "$launcher_before"
snapshot lock-notification-restored

launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 770
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-move 520 770
wait_for_launcher_commit_after "$launcher_before"
snapshot_current_contact lock-notification-dismiss-drag
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_BOOT_NOTIFICATION_CHANGED_OK .* receiver_image=launcher .* visible=0 revision=2$' "the Launcher notification dismissal"
wait_for_pattern '^UI_BOOT_NOTIFICATION_CHANGED_OK .* receiver_image=app .* visible=0 revision=2$' "the App notification dismissal snapshot"
wait_for_launcher_commit_after "$launcher_before"
snapshot lock-notification-dismissed

# Lock is deliberate: System navigation must never unlock it.  Unlocking is a
# Launcher control request and is therefore observable in the authenticated UI
# trace before Home appears.
request_before=$(line_count '^UI_SYSTEM_UI_REQUEST_OK ')
nav_before=$(line_count '^UI_SYSTEM_UI_CHANGED_OK ')
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1090
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_count_gt '^UI_SYSTEM_UI_CHANGED_OK ' "$nav_before" "locked bottom-navigation cancellation"
snapshot lock-after-nav

"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1200
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 800
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_count_gt '^UI_SYSTEM_UI_REQUEST_OK ' "$request_before" "the authenticated unlock request"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' "the unlocked Home state"
wait_for_home_frame "$ARTIFACT_DIR/home.ppm"

# Before any focused App has successfully presented, Overview is explicitly
# empty.  A 240px bottom gesture is the exact Overview threshold.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1330
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=none nav_pressed=0 nav_reveal_px=0 ' "the empty Overview state"
wait_for_launcher_commit_after "$launcher_before"
wait_for_overview_frame empty "$ARTIFACT_DIR/overview-empty.ppm"

# A 480px gesture is Home.  It is server-owned and must not manufacture a
# client control request or a recent app.
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1090
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' "Home after the 480px gesture"
wait_for_launcher_commit_after "$launcher_before"
snapshot home-before-phone

# Opening Phone first focuses App, then an accepted focused App present records
# Phone as the one recent identity and publishes Foreground to both clients.
"$QMP_HELPER" "$QMP_SOCKET" tap 160 1412
wait_for_pattern '^UI_ROUTE_FOCUS_OK .* active_client=app app=phone ' "Phone focus"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=foreground recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "the accepted Phone recent identity"
normalize_log
APP_PID=$(awk -v launcher="$LAUNCHER_PID" '/^USER_SURFACE_BUFFER_COMMIT_OK / { for (i = 1; i <= NF; i++) if ($i ~ /^producer_pid=/) { sub(/^producer_pid=/, "", $i); if ($i != launcher) { print $i; exit } } }' "$NORMALIZED_LOG")
[[ "$APP_PID" =~ ^[1-9][0-9]*$ ]] || {
  echo "Could not derive the App frame producer after the accepted Phone present." >&2
  exit 1
}
snapshot phone

# A 240px App gesture first yields a finger-follow frame then hands authority
# to Launcher in stable Overview.  The renderer shows an identity card only;
# no Phone pixels, thumbnail, or screenshot are supplied to Launcher.
app_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${APP_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1330
wait_for_app_commit_after "$app_before"
snapshot_current_contact overview-phone-partial
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "the Phone identity Overview"
wait_for_pattern '^UI_ROUTE_FOCUS_OK .* active_client=launcher app=none ' "Launcher Overview ownership"
wait_for_launcher_commit_after "$launcher_before"
wait_for_overview_frame phone "$ARTIFACT_DIR/overview-phone.ppm"

# Overview must not swallow the top system edge. The same Launcher-owned shade
# first follows the contact, then settles over the identity-only card without
# changing SurfaceServer's recent identity or manufacturing App pixels. Closing
# it must reveal the byte-identical Overview again.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 80
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 700
wait_for_launcher_commit_after "$launcher_before"
snapshot_current_contact overview-shade-partial
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_launcher_commit_after "$launcher_before"
snapshot overview-shade-open
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "the unchanged recent identity under Quick Settings"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" tap 648 160
wait_for_launcher_commit_after "$launcher_before"
wait_for_overview_frame phone "$ARTIFACT_DIR/overview-phone-after-shade.ppm"

# The card activation is a Launcher-only v7 request.  It returns to the same
# focused Phone identity; it does not create a second history entry.
request_before=$(line_count '^UI_SYSTEM_UI_REQUEST_OK ')
app_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${APP_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" tap 360 920
wait_for_count_gt '^UI_SYSTEM_UI_REQUEST_OK ' "$request_before" "the recent-card activation request"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=foreground recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "Foreground after card activation"
wait_for_pattern '^UI_ROUTE_FOCUS_OK .* active_client=app app=phone ' "Phone refocus"
wait_for_app_commit_after "$app_before"
snapshot phone-restored

# 480px from an App returns Home, preserving the single recent identity.
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1090
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "Home after App 480px gesture"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "final Home state delivered to Launcher"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=app .* mode=home recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "final Home state delivered to App"
wait_for_pattern '^UI_ROUTE_FOCUS_OK .* active_client=launcher app=none ' "Launcher Home ownership"
wait_for_launcher_commit_after "$launcher_before"
snapshot home-after-phone

# Re-enter Overview and exercise the identity-only recent-card dismissal. The
# first upward drag is captured as a real finger-follow frame. Only release
# beyond 224px may issue the exact Launcher-authenticated dismiss request; the
# resulting state stays in Overview and clears no package or process itself.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1330
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=phone nav_pressed=0 nav_reveal_px=0 ' "Overview before recent dismissal"
wait_for_launcher_commit_after "$launcher_before"

launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 920
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 660
wait_for_launcher_commit_after "$launcher_before"
snapshot_current_contact overview-phone-dismiss-drag
request_before=$(line_count '^UI_SYSTEM_UI_REQUEST_OK ')
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_count_gt '^UI_SYSTEM_UI_REQUEST_OK ' "$request_before" "the exact recent-card dismissal request"
wait_for_pattern '^UI_SYSTEM_UI_REQUEST_OK .* action=dismiss-recent app=phone ' "the Phone recent identity dismissal"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=none nav_pressed=0 nav_reveal_px=0 ' "empty Overview after dismissal"
wait_for_launcher_commit_after "$launcher_before"
wait_for_overview_frame empty "$ARTIFACT_DIR/overview-after-dismiss.ppm"

# System Home remains separately SurfaceServer-owned; dismissal itself does
# not smuggle navigation authority into the recent card gesture.
launcher_before=$(line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} ")
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1570
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 1090
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' "Home after recent dismissal"
wait_for_launcher_commit_after "$launcher_before"
snapshot home-after-dismiss

normalize_log
reject_failure
python3 - "$NORMALIZED_LOG" "$ARTIFACT_DIR" <<'PY'
from hashlib import sha256
from pathlib import Path
import re
import sys

log_path = Path(sys.argv[1])
artifact_dir = Path(sys.argv[2])
lines = log_path.read_text(encoding="utf-8").splitlines()

def field(line, name):
    match = re.search(rf"(?:^| ){re.escape(name)}=([^ ]+)", line)
    if not match:
        raise SystemExit(f"missing {name} in {line}")
    return match.group(1)

def all_lines(prefix):
    return [line for line in lines if line.startswith(prefix)]

preview = all_lines("MOBILE_UI_PREVIEW_OK ")
if len(preview) != 1:
    raise SystemExit(f"expected one preview marker, found {len(preview)}")
expected_preview = {
    "profile": "local-qemu", "abi": "24", "width": "720", "height": "1600",
    "design_width": "360", "design_height": "800", "scale": "2", "aspect": "20:9",
    "buffers": "4", "write_calls": "0", "write_bytes": "0", "presents": "1",
    "ui_client_control_version": "8", "ui_server_event_version": "6",
    "buffer_present_version": "6", "network": "disabled", "validation_scope": "ui-preview",
    "local_notification": "1", "notification_source": "boot-local-system-ui",
    "notification_scope": "surface-session-shared", "notification_persistence": "none",
    "notification_drag_quantum_px": "8", "notification_dismiss_threshold_px": "160",
    "notification_lock_swipe": "1", "notification_lock_tap_navigation": "0",
    "notification_lock_unlock_authority": "0", "notification_post_claim": "0",
    "push_notification_claim": "0", "background_delivery_claim": "0",
    "notification_service_claim": "0",
    "copy_path": "mapped-double-buffer", "rows_per_write": "0", "writes_per_frame": "0",
    "client_buffers_per_producer": "2", "frame_transaction_depth": "2",
    "transition_scheduling": "async-one-ahead", "stale_prepared_policy": "server-discard",
    "maps": "8", "map_successes": "8", "queues": "5", "queue_successes": "5",
    "acquires": "5", "acquire_successes": "5", "release_calls": "4",
    "release_successes": "4", "releases": "5", "mapped_presents": "1",
    "overview": "single-recent-identity", "overview_history_capacity": "1",
    "overview_source": "accepted-focused-present-or-compatible-commit",
    "overview_app_pixels_read": "0", "overview_activity_pixels": "0",
    "overview_thumbnail": "0", "overview_screenshot": "0", "overview_live_preview": "0",
    "overview_calculator_recent": "0", "overview_dismiss": "identity-only",
    "overview_compatible_icon": "verified-package-catalog",
    "overview_compatible_icon_binding": "session+generation+apk-digest",
    "overview_compatible_icon_activity_pixels": "0",
    "overview_compatible_icon_thumbnail": "0",
    "overview_dismiss_gesture": "upward", "overview_dismiss_activation_px": "32",
    "overview_dismiss_threshold_px": "224", "overview_dismiss_max_offset_px": "320",
    "overview_dismiss_quantum_px": "8", "overview_task_kill": "0", "overview_persistence": "none",
    "system_navigation_owner": "surface-server", "android_gesture_claim": "0",
    "background_execution_claim": "0", "hardware_compositor_claim": "0",
    "frame_pacing": "software", "logical_timer_hz": "100",
    "software_frame_rate_hz": "50", "frame_divider": "2",
    "clock_source": "qemu-pl031", "clock_transport": "surface-server-snapshot",
    "clock_refresh": "minute-boundary", "clock_scheduler": "bounded-wait-timeout",
    "clock_revision": "visible-minute-only", "client_rtc_authority": "0",
    "frame_acquires": "1", "frame_acquire_successes": "1", "frame_epoch": "1",
    "timer_pacing": "1", "real_phone_claim": "0", "hardware_vsync_claim": "0",
    "fps_claim": "0",
}
for name, expected in expected_preview.items():
    observed = field(preview[0], name)
    if observed != expected:
        raise SystemExit(f"preview boundary {name}: expected {expected}, observed {observed}")
if field(preview[0], "frame_pending") not in {"0", "1"}:
    raise SystemExit("preview frame clock escaped its Waiting/Ready quiescent boundary")

address_spaces = all_lines("ASPACE_OK ")
if len(address_spaces) != 1 or field(address_spaces[0], "private_tables") != "14":
    raise SystemExit("mobile address spaces must own eleven L3 tables and fourteen private tables total")
user_maps = all_lines("USER_MAP_OK ")
if len(user_maps) != 1 or field(user_maps[0], "stack_pages") != "4" \
        or field(user_maps[0], "guards_unmapped") != "2":
    raise SystemExit("pure mobile preview stack or guard geometry changed")

created_buffers = all_lines("GRAPHICS_BUFFER_CREATE_OK ")
if len(created_buffers) != 4:
    raise SystemExit(f"expected four mapped buffer creations, found {len(created_buffers)}")
for line in created_buffers:
    for name, expected in {
        "width": "720", "height": "1600", "logical_bytes": "4608000",
        "backing_bytes": "4608000", "rights": "0x0000012f", "mapped": "1",
    }.items():
        if field(line, name) != expected:
            raise SystemExit(f"mapped buffer creation boundary {name} changed")

maps = all_lines("GRAPHICS_BUFFER_MAP_OK ")
if len(maps) != 8:
    raise SystemExit(f"expected four producer and four consumer mappings, found {len(maps)}")
if [field(line, "role") for line in maps].count("producer") != 4 \
        or [field(line, "role") for line in maps].count("consumer") != 4:
    raise SystemExit("mapped producer/consumer role shape changed")
if {field(line, "pages") for line in maps} != {"1125"}:
    raise SystemExit("mapped buffer page geometry changed")
for line in maps:
    expected_access = "rw" if field(line, "role") == "producer" else "ro"
    if field(line, "access") != expected_access:
        raise SystemExit("mapped buffer role/access attenuation changed")
if {field(line, "address") for line in maps} != {
    "0x0000000200400000", "0x0000000200880000",
    "0x0000000200d00000", "0x0000000201180000",
}:
    raise SystemExit("mapped mobile buffer addresses changed")

double_buffer = all_lines("MOBILE_DOUBLE_BUFFER_RUNTIME_OK ")
if len(double_buffer) != 1:
    raise SystemExit(f"expected one asynchronous double-buffer marker, found {len(double_buffer)}")
for name, expected in {
    "format": "1", "client_buffers_per_producer": "2", "producer_count": "2",
    "transaction_depth": "2", "scheduling": "async-one-ahead",
    "stale_prepared_policy": "server-discard", "counter_overflowed": "0",
}.items():
    if field(double_buffer[0], name) != expected:
        raise SystemExit(f"double-buffer boundary {name} changed")
if int(field(double_buffer[0], "peak_queued")) < 2 \
        or int(field(double_buffer[0], "peak_in_flight")) < 2 \
        or int(field(double_buffer[0], "dual_in_flight_publications")) < 1:
    raise SystemExit("double-buffer runtime never held two bounded frames in flight")

clock_reads = all_lines("MOBILE_UI_CLOCK_READ_OK ")
if len(clock_reads) != 1 or field(clock_reads[0], "source") != "qemu-pl031":
    raise SystemExit("missing canonical initial QEMU PL031 clock snapshot")
if lines.index(clock_reads[0]) >= lines.index(preview[0]):
    raise SystemExit("clock snapshot must precede preview readiness")
clock_advances = all_lines("MOBILE_UI_CLOCK_ADVANCE_OK ")
if len(clock_advances) != 1 or field(clock_advances[0], "source") != "qemu-pl031" \
        or field(clock_advances[0], "direction") != "forward":
    raise SystemExit("QEMU gate did not observe exactly one forward PL031 minute transition")
initial_seconds = int(field(clock_reads[0], "seconds"))
initial_minute = int(field(clock_reads[0], "minute"))
advance_seconds = int(field(clock_advances[0], "seconds"))
advance_minute = int(field(clock_advances[0], "minute"))
if initial_seconds // 60 != initial_minute \
        or advance_seconds // 60 != advance_minute \
        or advance_minute != initial_minute + 1:
    raise SystemExit("PL031 minute evidence is not one exact forward boundary")
initial_display = f"{initial_seconds // 3600 % 24:02}:{initial_seconds // 60 % 60:02}"
advance_display = f"{advance_seconds // 3600 % 24:02}:{advance_seconds // 60 % 60:02}"
if (initial_display, advance_display) != ("09:41", "09:42"):
    raise SystemExit(
        f"fixed QEMU clock did not visibly cross 09:41 to 09:42: "
        f"{initial_display} to {advance_display}"
    )
if lines.index(clock_advances[0]) <= lines.index(preview[0]):
    raise SystemExit("PL031 minute advanced before the initial preview became observable")

clock_events = all_lines("UI_CLOCK_CHANGED_OK ")
if len(clock_events) != 4:
    raise SystemExit(f"expected two revisions for two clock clients, found {len(clock_events)}")
clock_revisions = {}
for line in clock_events:
    if field(line, "sender_image") != "surface-server" or field(line, "session") != "1":
        raise SystemExit("clock publication escaped SurfaceServer session authority")
    revision = int(field(line, "revision"))
    receiver = field(line, "receiver_image")
    if receiver in clock_revisions.setdefault(revision, {}):
        raise SystemExit("duplicate clock receiver revision")
    clock_revisions[revision][receiver] = int(field(line, "unix_seconds"))
if list(clock_revisions) != [1, 2]:
    raise SystemExit(f"clock revisions are not exactly [1, 2]: {list(clock_revisions)}")
for revision, expected_seconds in ((1, initial_seconds), (2, advance_seconds)):
    if clock_revisions[revision] != {
        "launcher": expected_seconds, "app": expected_seconds,
    }:
        raise SystemExit(f"clock revision {revision} was not an identical two-client broadcast")

commits = all_lines("USER_SURFACE_BUFFER_COMMIT_OK ")
if len(commits) < 12:
    raise SystemExit(f"expected a nontrivial rendered interaction, found {len(commits)} commits")
frame_acquires = all_lines("SURFACE_FRAME_ACQUIRE_OK ")
frame_commits = all_lines("SURFACE_FRAME_COMMIT_OK ")
if len(frame_acquires) != len(commits) or len(frame_commits) != len(commits):
    raise SystemExit(
        "every accepted mobile buffer commit must own exactly one software-frame grant"
    )
producers = []
previous_boundary = 0
damage_commits = 0
full_commits = 0
visible_damage_commits = 0
damage_pixels_total = 0
max_damage_pixels = 0
min_visible_damage_pixels = None
raster_writes_total = 0
previous_scene_digest = None

def verified_damage(line):
    try:
        count = int(field(line, "damage_rects"))
        raw = [tuple(map(int, field(line, f"damage{index}").split("/")))
               for index in range(2)]
        global_rect = tuple(map(int, field(line, "global_damage").split("/")))
        composition = tuple(map(int, field(line, "composition").split("/")))
    except ValueError as error:
        raise SystemExit("mobile commit emitted malformed multi-region damage") from error
    if count not in (1, 2) or any(len(rect) != 4 for rect in raw):
        raise SystemExit("mobile commit emitted an invalid damage-region count")
    if count == 1 and raw[1] != (0, 0, 0, 0):
        raise SystemExit("mobile single-region commit exposed a nonzero second slot")
    rects = raw[:count]
    for x, y, width, height in rects:
        if width <= 0 or height <= 0 or x < 0 or y < 0 \
                or x + width > 720 or y + height > 1600:
            raise SystemExit("mobile commit escaped the physical surface")
    if rects != sorted(rects, key=lambda rect: (rect[1], rect[0], rect[3], rect[2])):
        raise SystemExit("mobile damage regions are not canonically ordered")
    if count == 2:
        ax, ay, aw, ah = rects[0]
        bx, by, bw, bh = rects[1]
        if ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah:
            raise SystemExit("mobile damage regions overlap")
    left = min(rect[0] for rect in rects)
    top = min(rect[1] for rect in rects)
    right = max(rect[0] + rect[2] for rect in rects)
    bottom = max(rect[1] + rect[3] for rect in rects)
    bounds = (left, top, right - left, bottom - top)
    pixels = sum(rect[2] * rect[3] for rect in rects)
    if global_rect != bounds:
        raise SystemExit("mobile global damage is not the exact region-set bound")
    if int(field(line, "damage_pixels")) != pixels \
            or int(field(line, "raster_writes")) != pixels:
        raise SystemExit("mobile raster writes do not equal the disjoint damage pixels")
    cx, cy, cw, ch = composition
    if cw <= 0 or ch <= 0 or cx < 0 or cy < 0 \
            or cx + cw > 720 or cy + ch > 1600 \
            or cx > left or cy > top or cx + cw < right or cy + ch < bottom:
        raise SystemExit("mobile compositor evidence does not contain the damage regions")
    composition_rects = int(field(line, "composition_rects"))
    composition_pixels = int(field(line, "composition_pixels"))
    if composition_rects not in (count, count + 1) \
            or not pixels <= composition_pixels <= pixels + 12 * 22:
        raise SystemExit("mobile cursor-preserving composition accounting changed")
    return field(line, "mode"), bounds, pixels

for number, line in enumerate(commits, 1):
    if int(field(line, "frame_id")) != number or int(field(line, "commit")) != number:
        raise SystemExit("global frame/commit sequence is not contiguous")
    if (field(line, "width"), field(line, "height")) != ("720", "1600"):
        raise SystemExit("mobile scanout geometry changed")
    if field(line, "format") != "xrgb8888":
        raise SystemExit("mobile scanout format changed")
    mode, (x, y, width, height), damage_pixels = verified_damage(line)
    raster_writes_total += int(field(line, "raster_writes"))
    if mode == "full":
        if (x, y, width, height) != (0, 0, 720, 1600) \
                or damage_pixels != 720 * 1600:
            raise SystemExit("full mobile frame did not cover the complete surface")
        full_commits += 1
    elif mode == "damage":
        if damage_pixels >= 720 * 1600 // 2:
            raise SystemExit(
                f"mobile damage was not component-tight: pixels={damage_pixels} "
                f"bounds={x}/{y}/{width}/{height}"
            )
        damage_commits += 1
        damage_pixels_total += damage_pixels
        max_damage_pixels = max(max_damage_pixels, damage_pixels)
        if previous_scene_digest is not None \
                and field(line, "scene_digest") != previous_scene_digest:
            visible_damage_commits += 1
            min_visible_damage_pixels = (
                damage_pixels
                if min_visible_damage_pixels is None
                else min(min_visible_damage_pixels, damage_pixels)
            )
    else:
        raise SystemExit(f"unknown mobile buffer-present mode {mode}")
    if number == 1 and mode != "full":
        raise SystemExit("the initial committed mobile frame must establish a full base")
    acquire = frame_acquires[number - 1]
    frame_commit = frame_commits[number - 1]
    boundary = int(field(acquire, "boundary"))
    if int(field(acquire, "epoch")) != number or int(field(frame_commit, "epoch")) != number:
        raise SystemExit("software-frame epochs are not contiguous with accepted commits")
    if boundary <= previous_boundary or (previous_boundary and boundary - previous_boundary < 2):
        raise SystemExit("accepted mobile commits crossed fewer than two 100 Hz logical ticks")
    if any(
        field(evidence, name) != field(line, name)
        for evidence in (acquire, frame_commit)
        for name in ("pid", "session")
    ):
        raise SystemExit("software-frame evidence escaped the committing SurfaceServer session")
    if field(frame_commit, "frame_id") != field(line, "frame_id"):
        raise SystemExit("software-frame commit evidence identified a different global frame")
    if field(frame_commit, "client_buffer_slot") != field(line, "client_buffer_slot"):
        raise SystemExit("software-frame commit evidence identified a different client slot")
    acquire_index = lines.index(acquire)
    commit_index = lines.index(line)
    frame_commit_index = lines.index(frame_commit)
    if not acquire_index < commit_index < frame_commit_index:
        raise SystemExit("software-frame grant/commit publication ordering changed")
    previous_boundary = boundary
    previous_scene_digest = field(line, "scene_digest")
    producers.append(field(line, "producer_pid"))

lock_notification_damage_sequence = [
    ("48/652/624/232", "144768"),
    ("48/652/672/240", "161280"),
    ("48/652/672/240", "161280"),
    ("48/652/624/232", "144768"),
    ("48/652/672/240", "161280"),
    ("48/652/672/240", "161280"),
]
lock_notification_commits = [
    line for line in commits
    if field(line, "damage0") in {damage for damage, _ in lock_notification_damage_sequence}
]
if len(lock_notification_commits) != len(lock_notification_damage_sequence):
    raise SystemExit(
        "Lock notification press/drag/rebound/dismiss did not emit its six exact transactions"
    )
for line, (damage, pixels) in zip(
        lock_notification_commits, lock_notification_damage_sequence):
    for name, expected in {
        "mode": "damage", "damage_rects": "1", "damage0": damage,
        "damage1": "0/0/0/0", "global_damage": damage,
        "damage_pixels": pixels, "raster_writes": pixels,
        "composition": damage, "composition_rects": "1",
        "composition_pixels": pixels,
    }.items():
        if field(line, name) != expected:
            raise SystemExit(
                f"Lock notification bounded transaction {name} changed: {field(line, name)}"
            )

clock_damage_commits = [
    line for line in commits
    if field(line, "damage0") == "32/12/200/52"
    and field(line, "damage1") == "96/232/528/212"
]
if len(clock_damage_commits) != 1:
    raise SystemExit(
        f"expected one exact two-region Lock minute commit, found {len(clock_damage_commits)}"
    )
for name, expected in {
    "mode": "damage", "damage_rects": "2",
    "global_damage": "32/12/592/432", "damage_pixels": "122336",
    "raster_writes": "122336", "composition": "32/12/592/432",
    "composition_rects": "2", "composition_pixels": "122336",
}.items():
    if field(clock_damage_commits[0], name) != expected:
        raise SystemExit(
            f"Lock minute bounded transaction {name} changed: "
            f"{field(clock_damage_commits[0], name)}"
        )

if len(set(producers)) != 2:
    raise SystemExit("Launcher and App must remain distinct frame producers")
if damage_commits == 0:
    raise SystemExit("the interaction never exercised a real damage transaction")
if visible_damage_commits == 0:
    raise SystemExit("damage transactions never changed the committed UI scene")
if min_visible_damage_pixels is None or min_visible_damage_pixels > 100_000:
    raise SystemExit("no visible mobile interaction used a component-sized damage rectangle")

acquisitions = all_lines("GRAPHICS_BUFFER_ACQUIRE_OK ")
discard_releases = all_lines("GRAPHICS_BUFFER_RELEASE_OK ")
if len(acquisitions) != len(commits) + len(discard_releases):
    raise SystemExit("every acquired mobile generation must end in one commit or explicit discard")
allocation_slots = {}
for line in created_buffers:
    allocation_slots.setdefault(field(line, "producer_pid"), []).append(
        int(field(line, "slot"))
    )
if len(allocation_slots) != 2 or any(len(slots) != 2 for slots in allocation_slots.values()):
    raise SystemExit("Launcher and App must each own exactly two allocation identities")
for producer, producer_acquisitions in {
    producer: [line for line in acquisitions if field(line, "producer_pid") == producer]
    for producer in allocation_slots
}.items():
    slots = allocation_slots[producer]
    observed = [int(field(line, "allocation_slot")) for line in producer_acquisitions]
    if not observed or any(slot not in slots for slot in observed):
        raise SystemExit("an acquisition escaped its producer's two-slot allocation")
    if any(left == right for left, right in zip(observed, observed[1:])):
        raise SystemExit("a producer did not alternate its bounded acquisition slots")

pending = None
for index, line in enumerate(lines):
    if line.startswith("GRAPHICS_BUFFER_ACQUIRE_OK "):
        if pending is not None:
            raise SystemExit("a second buffer acquisition began before the prior transaction ended")
        pending = (index, line)
    elif line.startswith("GRAPHICS_BUFFER_RELEASE_OK "):
        if pending is None:
            raise SystemExit("an explicit discard release had no matching acquisition")
        acquire_index, acquire = pending
        for name in ("consumer_pid", "producer_pid", "allocation_slot",
                     "allocation_generation", "buffer_generation"):
            if field(acquire, name) != field(line, name):
                raise SystemExit(f"discard release changed acquired {name}")
        if any(candidate.startswith("SURFACE_FRAME_ACQUIRE_OK ")
               for candidate in lines[acquire_index + 1:index]):
            raise SystemExit("a discarded prepared frame consumed a display-frame grant")
        pending = None
    elif line.startswith("USER_SURFACE_BUFFER_COMMIT_OK "):
        if pending is None:
            raise SystemExit("a visible commit had no matching buffer acquisition")
        _, acquire = pending
        if field(acquire, "producer_pid") != field(line, "producer_pid") \
                or field(acquire, "buffer_generation") != field(line, "buffer_generation"):
            raise SystemExit("visible commit changed its acquired producer or generation")
        slots = allocation_slots[field(line, "producer_pid")]
        logical_slot = slots.index(int(field(acquire, "allocation_slot")))
        if logical_slot != int(field(line, "client_buffer_slot")):
            raise SystemExit("client slot did not select its exact allocation identity")
        pending = None
if pending is not None:
    raise SystemExit("the final acquired buffer transaction was left incomplete")

inputs = all_lines("USER_INPUT_READ_OK ")
if len(inputs) < 35:
    raise SystemExit(f"expected the QMP interaction input stream, found {len(inputs)} samples")
for sequence, line in enumerate(inputs, 1):
    if int(field(line, "sequence")) != sequence:
        raise SystemExit("input sequence is not contiguous")
    pending, enqueued, dequeued = map(lambda key: int(field(line, key)), ("pending", "enqueued", "dequeued"))
    if dequeued != sequence or pending != enqueued - dequeued or field(line, "coalesced") != "0":
        raise SystemExit(f"input accounting changed at sequence {sequence}")
if int(field(inputs[-1], "pending")) != 0:
    raise SystemExit("QMP input stream was not drained")

boot_notifications = all_lines("UI_BOOT_NOTIFICATION_CHANGED_OK ")
if len(boot_notifications) != 4:
    raise SystemExit(f"expected initial+dismissed two-client notification snapshots, found {len(boot_notifications)}")
notification_revisions = {}
for line in boot_notifications:
    if field(line, "sender_image") != "surface-server" or field(line, "session") != "1":
        raise SystemExit("boot notification escaped SurfaceServer session authority")
    revision = int(field(line, "revision"))
    receiver = field(line, "receiver_image")
    if receiver in notification_revisions.setdefault(revision, {}):
        raise SystemExit("duplicate boot-notification receiver revision")
    notification_revisions[revision][receiver] = field(line, "visible")
if list(notification_revisions) != [1, 2]:
    raise SystemExit(f"boot-notification revisions are not exactly [1, 2]: {list(notification_revisions)}")
for revision, expected_visible in ((1, "1"), (2, "0")):
    receivers = notification_revisions[revision]
    if receivers != {"launcher": expected_visible, "app": expected_visible}:
        raise SystemExit(f"boot-notification revision {revision} was not an identical two-client broadcast")

changed = all_lines("UI_SYSTEM_UI_CHANGED_OK ")
requests = all_lines("UI_SYSTEM_UI_REQUEST_OK ")
if len(changed) < 18 or len(changed) % 2:
    raise SystemExit(f"System UI must broadcast complete paired revisions, found {len(changed)} lines")
if {field(line, "receiver_image") for line in changed} != {"launcher", "app"}:
    raise SystemExit("System UI changes did not reach exactly Launcher and App")
by_revision = {}
for line in changed:
    if field(line, "sender_image") != "surface-server" or field(line, "session") != "1":
        raise SystemExit("System UI change escaped SurfaceServer session authority")
    revision = int(field(line, "revision"))
    receiver = field(line, "receiver_image")
    if receiver in by_revision.setdefault(revision, {}):
        raise SystemExit(f"duplicate System UI broadcast revision={revision} receiver={receiver}")
    by_revision[revision][receiver] = tuple(field(line, key) for key in ("mode", "recent_app", "nav_pressed", "nav_reveal_px"))
if list(by_revision) != list(range(1, max(by_revision) + 1)):
    raise SystemExit("System UI revisions are not contiguous")
for revision, receivers in by_revision.items():
    if set(receivers) != {"launcher", "app"} or len(set(receivers.values())) != 1:
        raise SystemExit(f"System UI revision {revision} was not an identical two-client broadcast")

states = [by_revision[revision]["launcher"] for revision in sorted(by_revision)]
if states[0] != ("locked", "none", "0", "0"):
    raise SystemExit(f"initial System UI state changed: {states[0]}")
if not any(state == ("overview", "none", "0", "0") for state in states):
    raise SystemExit("the empty single-recent Overview was not reached")
if not any(state == ("overview", "phone", "0", "0") for state in states):
    raise SystemExit("the Phone identity Overview was not reached")
if not any(state == ("foreground", "phone", "0", "0") for state in states):
    raise SystemExit("accepted focused Phone present did not establish Foreground recent identity")
if any(mode == "foreground" and recent == "none" for mode, recent, _pressed, _reveal in states):
    raise SystemExit("Foreground escaped without a recent identity")
for mode, _recent, pressed, reveal in states:
    reveal_px = int(reveal)
    if reveal_px % 8 or not 0 <= reveal_px <= 480:
        raise SystemExit("System navigation reveal escaped the 8px/480px protocol bounds")
    if reveal_px and pressed != "1":
        raise SystemExit("System navigation reveal appeared without a press")

expected_requests = [("unlock", "none"), ("activate-recent", "phone"), ("dismiss-recent", "phone")]
observed_requests = []
for line in requests:
    if field(line, "sender_image") != "launcher" or field(line, "receiver_image") != "surface-server":
        raise SystemExit("System UI request escaped Launcher-to-SurfaceServer authority")
    observed_requests.append((field(line, "action"), field(line, "app")))
if observed_requests != expected_requests:
    raise SystemExit(f"System UI request sequence changed: {observed_requests}")
for expected_id, line in enumerate(requests, 1):
    if int(field(line, "request_id")) != expected_id or int(field(line, "observed_revision")) < 1:
        raise SystemExit("System UI request IDs or observed revisions are noncanonical")

focuses = all_lines("UI_ROUTE_FOCUS_OK ")
focus_states = {}
for line in focuses:
    generation = int(field(line, "focus_generation"))
    focus_states.setdefault(generation, {})[field(line, "receiver_image")] = (field(line, "active_client"), field(line, "app"))
for generation, receivers in focus_states.items():
    if set(receivers) != {"launcher", "app"} or len(set(receivers.values())) != 1:
        raise SystemExit(f"focus generation {generation} did not broadcast identically")
if not any(value == ("app", "phone") for receivers in focus_states.values() for value in receivers.values()):
    raise SystemExit("Phone App focus never occurred")
if not any(value == ("launcher", "none") for receivers in focus_states.values() for value in receivers.values()):
    raise SystemExit("Launcher focus never occurred")

phone_focus_index = next(i for i, line in enumerate(lines) if line.startswith("UI_ROUTE_FOCUS_OK ") and field(line, "receiver_image") == "launcher" and field(line, "active_client") == "app" and field(line, "app") == "phone")
phone_foreground_index = next(i for i, line in enumerate(lines) if line.startswith("UI_SYSTEM_UI_CHANGED_OK ") and field(line, "receiver_image") == "launcher" and field(line, "mode") == "foreground" and field(line, "recent_app") == "phone")
if phone_focus_index >= phone_foreground_index:
    raise SystemExit("recent identity appeared before Phone was focused")

def ppm(name):
    payload = (artifact_dir / name).read_bytes().split(b"\n", 3)
    if len(payload) != 4 or payload[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{name} is not an exact 720x1600 PPM")
    if len(payload[3]) != 720 * 1600 * 3:
        raise SystemExit(f"{name} has an invalid pixel payload")
    return payload[3]

images = {name: ppm(name) for name in (
    "lock-clock-before.ppm", "lock.ppm",
    "lock-notification-short-drag.ppm", "lock-notification-restored.ppm",
    "lock-notification-dismiss-drag.ppm", "lock-notification-dismissed.ppm",
    "lock-after-nav.ppm", "home.ppm", "overview-empty.ppm",
    "home-before-phone.ppm", "phone.ppm", "overview-phone-partial.ppm",
    "overview-phone.ppm", "overview-shade-partial.ppm", "overview-shade-open.ppm",
    "overview-phone-after-shade.ppm", "phone-restored.ppm", "home-after-phone.ppm",
    "overview-phone-dismiss-drag.ppm", "overview-after-dismiss.ppm",
    "home-after-dismiss.ppm",
)}
def pixel(image, x, y):
    offset = (y * 720 + x) * 3
    return tuple(image[offset:offset + 3])

def blend(base, overlay, alpha):
    return tuple((left * (255 - alpha) + right * alpha) // 255 for left, right in zip(base, overlay))

ocean = (78, 127, 255)
panel = blend((13, 22, 41), ocean, 12)
surface = blend((24, 34, 56), ocean, 14)
if pixel(images["overview-empty.ppm"], 360, 300) != panel or pixel(images["overview-empty.ppm"], 360, 580) != surface:
    raise SystemExit("overview-empty.ppm is not the stable empty identity-card Overview")
if pixel(images["overview-phone.ppm"], 360, 300) != panel or pixel(images["overview-phone.ppm"], 360, 580) != (49, 200, 90):
    raise SystemExit("overview-phone.ppm is not the stable Phone identity-card Overview")
def pixel_changes(a, b, bounds=None):
    count = 0
    for index in range(720 * 1600):
        x, y = index % 720, index // 720
        if bounds is not None and not (bounds[0] <= x < bounds[2] and bounds[1] <= y < bounds[3]):
            continue
        offset = index * 3
        count += a[offset:offset + 3] != b[offset:offset + 3]
    return count

cursor = (719, 1599, 720, 1600)
clock_changes = pixel_changes(images["lock-clock-before.ppm"], images["lock.ppm"])
status_clock_changes = pixel_changes(
    images["lock-clock-before.ppm"], images["lock.ppm"], (32, 12, 232, 64)
)
lock_clock_changes = pixel_changes(
    images["lock-clock-before.ppm"], images["lock.ppm"], (96, 232, 624, 444)
)
if clock_changes != status_clock_changes + lock_clock_changes \
        or status_clock_changes < 20 or lock_clock_changes < 100:
    raise SystemExit("09:41 to 09:42 changed pixels outside the two Lock clock regions")
lock_notice_bounds = (48, 652, 720, 892)
short_drag_changes = pixel_changes(images["lock.ppm"], images["lock-notification-short-drag.ppm"])
short_drag_bounded = pixel_changes(images["lock.ppm"], images["lock-notification-short-drag.ppm"], lock_notice_bounds)
short_cursor_delta = int(pixel(images["lock.ppm"], 719, 1599) != pixel(images["lock-notification-short-drag.ppm"], 719, 1599))
if short_drag_changes - short_drag_bounded != short_cursor_delta or short_drag_bounded < 20_000:
    raise SystemExit("short Lock notification swipe was not a bounded visible finger-follow frame")
if images["lock-notification-restored.ppm"] != images["lock.ppm"]:
    raise SystemExit("159px Lock notification swipe did not restore the exact Lock frame")
dismiss_drag_changes = pixel_changes(images["lock.ppm"], images["lock-notification-dismiss-drag.ppm"])
dismiss_drag_bounded = pixel_changes(images["lock.ppm"], images["lock-notification-dismiss-drag.ppm"], lock_notice_bounds)
dismiss_cursor_delta = int(pixel(images["lock.ppm"], 719, 1599) != pixel(images["lock-notification-dismiss-drag.ppm"], 719, 1599))
if dismiss_drag_changes - dismiss_drag_bounded != dismiss_cursor_delta or dismiss_drag_bounded < 20_000:
    raise SystemExit("threshold Lock notification swipe escaped its notification bounds")
dismissed_changes = pixel_changes(images["lock.ppm"], images["lock-notification-dismissed.ppm"])
dismissed_bounded = pixel_changes(images["lock.ppm"], images["lock-notification-dismissed.ppm"], (48, 652, 672, 892))
if dismissed_changes != dismissed_bounded or dismissed_bounded < 2_000:
    raise SystemExit("canonical notification dismissal changed pixels outside the Lock card")
if pixel_changes(images["lock-notification-dismissed.ppm"], images["lock-after-nav.ppm"], (0, 0, 720, 1548)):
    raise SystemExit("bottom navigation unlocked or altered Lock")
if pixel_changes(images["overview-phone.ppm"], images["phone-restored.ppm"], (0, 64, 720, 1548)) < 200_000:
    raise SystemExit("recent-card activation did not materially leave the identity Overview")
if pixel_changes(images["overview-empty.ppm"], images["overview-phone.ppm"], (0, 424, 720, 1184)) < 2_000:
    raise SystemExit("empty and identity Overview cards are not materially distinct")
if pixel_changes(images["overview-phone-partial.ppm"], images["overview-phone.ppm"], (0, 64, 720, 1548)) < 20_000:
    raise SystemExit("240px Overview gesture did not render a distinct finger-follow frame")
if pixel_changes(images["overview-shade-partial.ppm"], images["overview-phone.ppm"], (0, 64, 720, 1548)) < 40_000:
    raise SystemExit("Overview top-edge drag did not render a distinct shade finger-follow frame")
if pixel_changes(images["overview-shade-partial.ppm"], images["overview-shade-open.ppm"], (0, 64, 720, 1548)) < 100_000:
    raise SystemExit("Overview shade did not settle from its partial frame")
if pixel_changes(images["overview-shade-open.ppm"], images["overview-phone.ppm"], (0, 64, 720, 1548)) < 300_000:
    raise SystemExit("settled Quick Settings did not materially cover Overview")
if images["overview-phone-after-shade.ppm"] != images["overview-phone.ppm"]:
    raise SystemExit("closing Quick Settings did not restore the exact Overview frame")
if pixel_changes(images["overview-phone-dismiss-drag.ppm"], images["overview-phone.ppm"], (0, 64, 720, 1548)) < 20_000:
    raise SystemExit("recent-card dismissal did not render a distinct finger-follow frame")
if images["overview-after-dismiss.ppm"] != images["overview-empty.ppm"]:
    raise SystemExit("dismissed Overview did not return to the canonical empty identity surface")
if images["home-after-dismiss.ppm"] != images["home.ppm"]:
    raise SystemExit("Home retained recent-card pixels or state after identity dismissal")
for name in ("overview-empty.ppm", "overview-phone.ppm"):
    colors = {images[name][offset:offset+3] for offset in range(0, len(images[name]), 3)}
    if len(colors) < 24:
        raise SystemExit(f"{name} is unexpectedly flat")

summary = {
    "commits": len(commits), "frame_acquires": len(frame_acquires),
    "full_commits": full_commits,
    "damage_commits": damage_commits,
    "visible_damage_commits": visible_damage_commits,
    "damage_pixels_total": damage_pixels_total,
    "max_damage_pixels": max_damage_pixels,
    "min_visible_damage_pixels": min_visible_damage_pixels,
    "raster_writes_total": raster_writes_total,
    "raster_pixels_saved": len(commits) * 720 * 1600 - raster_writes_total,
    "inputs": len(inputs), "system_ui_revisions": len(by_revision),
    "system_ui_requests": len(requests), "focus_generations": len(focus_states),
    "clock_revisions": len(clock_revisions),
    "clock_minute_transition": f"{initial_minute}-{advance_minute}",
    "clock_display_transition": f"{initial_display}-{advance_display}",
    "clock_damage_pixels": 122336,
    "clock_pixel_changes": clock_changes,
    "lock_clock_before_pixel_payload_sha256": sha256(images["lock-clock-before.ppm"]).hexdigest(),
    "lock_clock_after_pixel_payload_sha256": sha256(images["lock.ppm"]).hexdigest(),
    "lock_notification_short_restore_exact": 1,
    "lock_notification_dismiss_revision": 2,
    "lock_notification_drag_pixel_payload_sha256": sha256(images["lock-notification-dismiss-drag.ppm"]).hexdigest(),
    "lock_notification_dismissed_pixel_payload_sha256": sha256(images["lock-notification-dismissed.ppm"]).hexdigest(),
    "overview_empty_pixel_payload_sha256": sha256(images["overview-empty.ppm"]).hexdigest(),
    "overview_phone_pixel_payload_sha256": sha256(images["overview-phone.ppm"]).hexdigest(),
    "overview_shade_open_pixel_payload_sha256": sha256(images["overview-shade-open.ppm"]).hexdigest(),
    "overview_dismiss_drag_pixel_payload_sha256": sha256(images["overview-phone-dismiss-drag.ppm"]).hexdigest(),
    "overview_after_dismiss_pixel_payload_sha256": sha256(images["overview-after-dismiss.ppm"]).hexdigest(),
}
(artifact_dir / "summary.txt").write_text("\n".join(f"{k}={v}" for k, v in summary.items()) + "\n", encoding="utf-8")
print("MOBILE_UI_RUNTIME_CHECK_OK " + " ".join(f"{k}={v}" for k, v in summary.items()))
PY

echo "Mobile UI runtime evidence: $ARTIFACT_DIR"
