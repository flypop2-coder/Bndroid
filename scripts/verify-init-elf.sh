#!/usr/bin/env bash
set -euo pipefail

CALLER_CWD="$(pwd -P)"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
TARGET_TRIPLE="aarch64-unknown-none"
PACKAGE_NAME="bndroid-init"
cd "$WORKSPACE_ROOT"

PROFILE="${BNDROID_PROFILE:-debug}"
case "$PROFILE" in
  debug | release) ;;
  *)
    echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
    exit 2
    ;;
esac

if (( $# > 1 )); then
  echo "usage: $0 [path-to-userspace-elf]" >&2
  exit 2
fi

DEFAULT_ELF="$WORKSPACE_ROOT/target/$TARGET_TRIPLE/$PROFILE/$PACKAGE_NAME"
INIT_ELF="${1:-${BNDROID_INIT_ELF:-$DEFAULT_ELF}}"
if [[ "$INIT_ELF" != /* ]]; then
  INIT_ELF="$CALLER_CWD/$INIT_ELF"
fi
if [[ ! -f "$INIT_ELF" ]]; then
  echo "Userspace ELF not found: $INIT_ELF" >&2
  exit 1
fi
INIT_ELF="$(cd -- "$(dirname -- "$INIT_ELF")" && pwd)/$(basename -- "$INIT_ELF")"

resolve_command() {
  local candidate="$1"
  if [[ "$candidate" == */* ]]; then
    [[ -x "$candidate" ]] || return 1
    printf '%s\n' "$candidate"
  else
    command -v "$candidate" 2>/dev/null
  fi
}

READ_TOOL=""
READ_TOOL_KIND=""
if [[ -n "${BNDROID_LLVM_READOBJ:-}" ]]; then
  if ! READ_TOOL="$(resolve_command "$BNDROID_LLVM_READOBJ")"; then
    echo "BNDROID_LLVM_READOBJ is not an executable command: $BNDROID_LLVM_READOBJ" >&2
    exit 1
  fi
  READ_TOOL_KIND="llvm-readobj"
elif [[ -n "${BNDROID_READELF:-}" ]]; then
  if ! READ_TOOL="$(resolve_command "$BNDROID_READELF")"; then
    echo "BNDROID_READELF is not an executable command: $BNDROID_READELF" >&2
    exit 1
  fi
  READ_TOOL_KIND="readelf"
elif READ_TOOL="$(command -v llvm-readobj 2>/dev/null)"; then
  READ_TOOL_KIND="llvm-readobj"
elif command -v rustc >/dev/null 2>&1; then
  if RUST_SYSROOT="$(rustc --print sysroot 2>/dev/null)" \
    && HOST_TRIPLE="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')" \
    && [[ -n "$HOST_TRIPLE" ]]; then
    for candidate in \
      "$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/llvm-readobj" \
      "$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/rust-readobj"; do
      if [[ -x "$candidate" ]]; then
        READ_TOOL="$candidate"
        READ_TOOL_KIND="llvm-readobj"
        break
      fi
    done
  fi
fi

if [[ -z "$READ_TOOL" ]]; then
  if READ_TOOL="$(command -v llvm-readelf 2>/dev/null)"; then
    READ_TOOL_KIND="readelf"
  elif READ_TOOL="$(command -v readelf 2>/dev/null)"; then
    READ_TOOL_KIND="readelf"
  else
    echo "No ELF inspection tool found." >&2
    echo "Install rustup's llvm-tools-preview component (for llvm-readobj) or GNU/LLVM readelf." >&2
    echo "You can also set BNDROID_LLVM_READOBJ or BNDROID_READELF to an executable path." >&2
    exit 1
  fi
fi

INSPECTION=""
if [[ "$READ_TOOL_KIND" == "llvm-readobj" ]]; then
  if ! INSPECTION="$("$READ_TOOL" --elf-output-style=GNU --file-headers --program-headers "$INIT_ELF" 2>&1)"; then
    echo "llvm-readobj failed to inspect $INIT_ELF:" >&2
    echo "$INSPECTION" >&2
    exit 1
  fi
else
  if ! INSPECTION="$("$READ_TOOL" --wide --file-header --program-headers "$INIT_ELF" 2>&1)"; then
    echo "readelf failed to inspect $INIT_ELF:" >&2
    echo "$INSPECTION" >&2
    exit 1
  fi
fi

fail_elf() {
  echo "Userspace ELF verification failed: $1" >&2
  exit 1
}

grep -Eq '^[[:space:]]*Class:[[:space:]]+ELF64([[:space:]]|$)' <<<"$INSPECTION" \
  || fail_elf "expected an ELF64 image"
grep -Eq '^[[:space:]]*Machine:[[:space:]]+AArch64([[:space:]]|$)' <<<"$INSPECTION" \
  || fail_elf "expected the AArch64 machine type"
grep -Eq '^[[:space:]]*Type:[[:space:]]+EXEC([[:space:]]|$)' <<<"$INSPECTION" \
  || fail_elf "expected ET_EXEC rather than a relocatable, shared, or dynamic image"

ENTRY_HEX="$(sed -nE 's/^[[:space:]]*Entry point address:[[:space:]]*(0x[0-9A-Fa-f]+).*$/\1/p' <<<"$INSPECTION" | head -n 1)"
if [[ ! "$ENTRY_HEX" =~ ^0x[0-9A-Fa-f]+$ ]]; then
  fail_elf "expected a valid executable entry point"
fi

read -r LOAD_COUNT RX_LOAD_COUNT RO_LOAD_COUNT RW_LOAD_COUNT WRITABLE_EXEC_COUNT \
  FORBIDDEN_COUNT STACK_COUNT SAFE_STACK_COUNT \
  RX_OFFSET RX_VADDR RX_FILE_SIZE RX_MEM_SIZE RX_ALIGN \
  RO_OFFSET RO_VADDR RO_FILE_SIZE RO_MEM_SIZE RO_ALIGN \
  RW_OFFSET RW_VADDR RW_FILE_SIZE RW_MEM_SIZE RW_ALIGN FORBIDDEN_TYPES <<<"$(
  awk '
    $2 ~ /^0x[0-9A-Fa-f]+$/ && $3 ~ /^0x[0-9A-Fa-f]+$/ {
      type = $1
      flags = ""
      for (field = 7; field < NF; field++) {
        flags = flags $field
      }
      if (type == "LOAD") {
        loads++
        if (flags ~ /R/ && flags ~ /E/ && flags !~ /W/) {
          rx_loads++
          rx_offset = $2
          rx_vaddr = $3
          rx_file_size = $5
          rx_mem_size = $6
          rx_align = $NF
        } else if (flags ~ /R/ && flags !~ /W/ && flags !~ /E/) {
          ro_loads++
          ro_offset = $2
          ro_vaddr = $3
          ro_file_size = $5
          ro_mem_size = $6
          ro_align = $NF
        } else if (flags ~ /R/ && flags ~ /W/ && flags !~ /E/) {
          rw_loads++
          rw_offset = $2
          rw_vaddr = $3
          rw_file_size = $5
          rw_mem_size = $6
          rw_align = $NF
        }
      }
      if (flags ~ /W/ && flags ~ /E/) {
        writable_exec++
      }
      if (type == "INTERP" || type == "DYNAMIC" || type == "TLS") {
        forbidden++
        forbidden_types = forbidden_types type ","
      }
      if (type == "GNU_STACK") {
        stacks++
        if (flags ~ /R/ && flags ~ /W/ && flags !~ /E/) {
          safe_stacks++
        }
      }
    }
    END {
      if (rx_offset == "") rx_offset = "0x0"
      if (rx_vaddr == "") rx_vaddr = "0x0"
      if (rx_file_size == "") rx_file_size = "0x0"
      if (rx_mem_size == "") rx_mem_size = "0x0"
      if (rx_align == "") rx_align = "0x0"
      if (ro_offset == "") ro_offset = "0x0"
      if (ro_vaddr == "") ro_vaddr = "0x0"
      if (ro_file_size == "") ro_file_size = "0x0"
      if (ro_mem_size == "") ro_mem_size = "0x0"
      if (ro_align == "") ro_align = "0x0"
      if (rw_offset == "") rw_offset = "0x0"
      if (rw_vaddr == "") rw_vaddr = "0x0"
      if (rw_file_size == "") rw_file_size = "0x0"
      if (rw_mem_size == "") rw_mem_size = "0x0"
      if (rw_align == "") rw_align = "0x0"
      if (forbidden_types == "") forbidden_types = "none"
      print loads + 0, rx_loads + 0, ro_loads + 0, rw_loads + 0, \
        writable_exec + 0, forbidden + 0, stacks + 0, safe_stacks + 0, \
        rx_offset, rx_vaddr, rx_file_size, rx_mem_size, rx_align, \
        ro_offset, ro_vaddr, ro_file_size, ro_mem_size, ro_align, \
        rw_offset, rw_vaddr, rw_file_size, rw_mem_size, rw_align, forbidden_types
    }
  ' <<<"$INSPECTION"
)"

if (( LOAD_COUNT != 3 )); then
  fail_elf "expected exactly three PT_LOAD headers, found $LOAD_COUNT"
fi
if (( RX_LOAD_COUNT != 1 )); then
  fail_elf "expected exactly one non-writable RX PT_LOAD, found $RX_LOAD_COUNT"
fi
if (( RO_LOAD_COUNT != 1 )); then
  fail_elf "expected exactly one read-only non-executable PT_LOAD, found $RO_LOAD_COUNT"
fi
if (( RW_LOAD_COUNT != 1 )); then
  fail_elf "expected exactly one writable non-executable PT_LOAD, found $RW_LOAD_COUNT"
fi
if (( WRITABLE_EXEC_COUNT != 0 )); then
  fail_elf "found $WRITABLE_EXEC_COUNT writable-and-executable program header(s)"
fi
if (( FORBIDDEN_COUNT != 0 )); then
  fail_elf "forbidden program header(s) present: ${FORBIDDEN_TYPES%,}"
fi
if (( STACK_COUNT != 1 || SAFE_STACK_COUNT != 1 )); then
  fail_elf "expected one RW, non-executable GNU_STACK header"
fi

PAGE_SIZE=4096
IMAGE_BASE=$((0x0000000200000000))
IMAGE_END=$((IMAGE_BASE + 0x100000))
ENTRY_VALUE=$((16#${ENTRY_HEX#0x}))
RX_FILE_OFFSET=$((16#${RX_OFFSET#0x}))
RX_START=$((16#${RX_VADDR#0x}))
RX_FILE_BYTES=$((16#${RX_FILE_SIZE#0x}))
RX_SIZE=$((16#${RX_MEM_SIZE#0x}))
RX_ALIGNMENT=$((16#${RX_ALIGN#0x}))
RO_FILE_OFFSET=$((16#${RO_OFFSET#0x}))
RO_START=$((16#${RO_VADDR#0x}))
RO_FILE_BYTES=$((16#${RO_FILE_SIZE#0x}))
RO_SIZE=$((16#${RO_MEM_SIZE#0x}))
RO_ALIGNMENT=$((16#${RO_ALIGN#0x}))
RW_FILE_OFFSET=$((16#${RW_OFFSET#0x}))
RW_START=$((16#${RW_VADDR#0x}))
RW_FILE_BYTES=$((16#${RW_FILE_SIZE#0x}))
RW_SIZE=$((16#${RW_MEM_SIZE#0x}))
RW_ALIGNMENT=$((16#${RW_ALIGN#0x}))
if (( ENTRY_VALUE % 4 != 0 )); then
  fail_elf "entry $ENTRY_HEX is not four-byte aligned"
fi
if (( RX_FILE_BYTES == 0 || RX_SIZE == 0 || ENTRY_VALUE < RX_START || ENTRY_VALUE >= RX_START + RX_FILE_BYTES )); then
  fail_elf "entry $ENTRY_HEX does not lie inside the file-backed portion of the RX PT_LOAD ($RX_VADDR + $RX_FILE_SIZE)"
fi
if (( RO_FILE_BYTES == 0 || RO_SIZE == 0 || RW_FILE_BYTES == 0 || RW_SIZE == 0 )); then
  fail_elf "all three PT_LOAD headers must have non-empty file and memory ranges"
fi
if (( RX_FILE_BYTES > RX_SIZE || RO_FILE_BYTES > RO_SIZE || RW_FILE_BYTES > RW_SIZE )); then
  fail_elf "a PT_LOAD file range exceeds its memory range"
fi
if (( RW_SIZE <= RW_FILE_BYTES )); then
  fail_elf "the RW PT_LOAD must contain a zero-filled BSS tail"
fi

for alignment in "$RX_ALIGNMENT" "$RO_ALIGNMENT" "$RW_ALIGNMENT"; do
  if (( alignment != PAGE_SIZE )); then
    fail_elf "every PT_LOAD must declare 4 KiB alignment"
  fi
done
for value in "$RX_FILE_OFFSET" "$RX_START" "$RO_FILE_OFFSET" "$RO_START" \
  "$RW_FILE_OFFSET" "$RW_START"; do
  if (( value % PAGE_SIZE != 0 )); then
    fail_elf "every PT_LOAD file offset and virtual address must be page aligned"
  fi
done

RX_PAGE_END=$(( (RX_START + RX_SIZE + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE ))
RO_PAGE_END=$(( (RO_START + RO_SIZE + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE ))
RW_PAGE_END=$(( (RW_START + RW_SIZE + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE ))
if (( RX_START != IMAGE_BASE || RX_PAGE_END > RO_START || RO_PAGE_END > RW_START )); then
  fail_elf "PT_LOAD headers must be page-disjoint and ordered RX, R--, RW- from the fixed image base"
fi
if (( RW_PAGE_END > IMAGE_END )); then
  fail_elf "PT_LOAD memory extends beyond the lower 1 MiB image arena"
fi
LOAD_PAGES=$((
  (RX_PAGE_END - RX_START + RO_PAGE_END - RO_START + RW_PAGE_END - RW_START) / PAGE_SIZE
))
if (( LOAD_PAGES > 256 )); then
  fail_elf "PT_LOAD headers require $LOAD_PAGES pages; maximum is 256"
fi

echo "Verified userspace ELF: $INIT_ELF (AArch64 ET_EXEC, entry $ENTRY_HEX, RX/R--/RW- PT_LOADs, BSS, $LOAD_PAGES pages)"
