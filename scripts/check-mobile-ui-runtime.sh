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

# Full Overview has an opaque dark panel at this unadorned point.  The empty
# identity additionally has the dark Info-disc interior at (360,580); Phone
# has the Phone-green icon field at the same point.  These are exact stable
# raster colors, so an old Home/App frame cannot satisfy the predicate.
if pixel(360, 300) != (13, 22, 41):
    raise SystemExit(1)
expected = (24, 34, 56) if identity == "empty" else (49, 200, 90)
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
  -rtc base=2026-07-29T09:41:00,clock=vm \
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
snapshot lock

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
snapshot home

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
wait_for_pattern '^UI_ROUTE_FOCUS_OK .* active_client=launcher app=none ' "Launcher Home ownership"
wait_for_launcher_commit_after "$launcher_before"
snapshot home-after-phone

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
    "ui_client_control_version": "8", "ui_server_event_version": "6",
    "buffer_present_version": "2", "network": "disabled", "validation_scope": "ui-preview",
    "overview": "single-recent-identity", "overview_history_capacity": "1",
    "overview_source": "accepted-focused-present-or-compatible-commit",
    "overview_app_pixels_read": "0", "overview_activity_pixels": "0",
    "overview_thumbnail": "0", "overview_screenshot": "0", "overview_live_preview": "0",
    "overview_calculator_recent": "0", "overview_task_kill": "0", "overview_persistence": "none",
    "system_navigation_owner": "surface-server", "android_gesture_claim": "0",
    "background_execution_claim": "0", "hardware_compositor_claim": "0",
    "real_phone_claim": "0", "hardware_vsync_claim": "0", "fps_claim": "0",
}
for name, expected in expected_preview.items():
    observed = field(preview[0], name)
    if observed != expected:
        raise SystemExit(f"preview boundary {name}: expected {expected}, observed {observed}")

clock_reads = all_lines("MOBILE_UI_CLOCK_READ_OK ")
if len(clock_reads) != 1 or field(clock_reads[0], "source") != "qemu-pl031":
    raise SystemExit("missing canonical one-shot QEMU PL031 clock snapshot")
if lines.index(clock_reads[0]) >= lines.index(preview[0]):
    raise SystemExit("clock snapshot must precede preview readiness")

commits = all_lines("USER_SURFACE_BUFFER_COMMIT_OK ")
if len(commits) < 12:
    raise SystemExit(f"expected a nontrivial rendered interaction, found {len(commits)} commits")
producers = []
for number, line in enumerate(commits, 1):
    if int(field(line, "frame_id")) != number or int(field(line, "commit")) != number:
        raise SystemExit("global frame/commit sequence is not contiguous")
    if (field(line, "width"), field(line, "height")) != ("720", "1600"):
        raise SystemExit("mobile scanout geometry changed")
    if field(line, "global_damage") != "0/0/720/1600" or field(line, "format") != "xrgb8888":
        raise SystemExit("full-frame composition contract changed")
    producers.append(field(line, "producer_pid"))
if len(set(producers)) != 2:
    raise SystemExit("Launcher and App must remain distinct frame producers")

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

expected_requests = [("unlock", "none"), ("activate-recent", "phone")]
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
    "lock.ppm", "lock-after-nav.ppm", "home.ppm", "overview-empty.ppm",
    "home-before-phone.ppm", "phone.ppm", "overview-phone-partial.ppm",
    "overview-phone.ppm", "phone-restored.ppm", "home-after-phone.ppm",
)}
def pixel(image, x, y):
    offset = (y * 720 + x) * 3
    return tuple(image[offset:offset + 3])

if pixel(images["overview-empty.ppm"], 360, 300) != (13, 22, 41) or pixel(images["overview-empty.ppm"], 360, 580) != (24, 34, 56):
    raise SystemExit("overview-empty.ppm is not the stable empty identity-card Overview")
if pixel(images["overview-phone.ppm"], 360, 300) != (13, 22, 41) or pixel(images["overview-phone.ppm"], 360, 580) != (49, 200, 90):
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
if pixel_changes(images["lock.ppm"], images["lock-after-nav.ppm"], (0, 0, 720, 1548)):
    raise SystemExit("bottom navigation unlocked or altered Lock")
if pixel_changes(images["overview-phone.ppm"], images["phone-restored.ppm"], (0, 64, 720, 1548)) < 200_000:
    raise SystemExit("recent-card activation did not materially leave the identity Overview")
if pixel_changes(images["overview-empty.ppm"], images["overview-phone.ppm"], (0, 424, 720, 1184)) < 2_000:
    raise SystemExit("empty and identity Overview cards are not materially distinct")
if pixel_changes(images["overview-phone-partial.ppm"], images["overview-phone.ppm"], (0, 64, 720, 1548)) < 20_000:
    raise SystemExit("240px Overview gesture did not render a distinct finger-follow frame")
for name in ("overview-empty.ppm", "overview-phone.ppm"):
    colors = {images[name][offset:offset+3] for offset in range(0, len(images[name]), 3)}
    if len(colors) < 24:
        raise SystemExit(f"{name} is unexpectedly flat")

summary = {
    "commits": len(commits), "inputs": len(inputs), "system_ui_revisions": len(by_revision),
    "system_ui_requests": len(requests), "focus_generations": len(focus_states),
    "overview_empty_pixel_payload_sha256": sha256(images["overview-empty.ppm"]).hexdigest(),
    "overview_phone_pixel_payload_sha256": sha256(images["overview-phone.ppm"]).hexdigest(),
}
(artifact_dir / "summary.txt").write_text("\n".join(f"{k}={v}" for k, v in summary.items()) + "\n", encoding="utf-8")
print("MOBILE_UI_RUNTIME_CHECK_OK " + " ".join(f"{k}={v}" for k, v in summary.items()))
PY

echo "Mobile UI runtime evidence: $ARTIFACT_DIR"
