#!/usr/bin/env bash

validate_memory_evidence() {
  local log_file="$1"
  local frame_count frame_line frame_regex
  local vm_count vm_line vm_regex memory_count

  frame_count="$(grep -c '^FRAME_OK ' "$log_file" || true)"
  frame_line="$(grep '^FRAME_OK ' "$log_file" || true)"
  frame_regex='^FRAME_OK managed=([0-9]+) unavailable=([0-9]+) free_before=([0-9]+) free_after=([0-9]+) allocated=([0-9]+) recycled=([0-9]+) unique=([0-9]+)$'
  if [[ "$frame_count" != "1" || ! "$frame_line" =~ $frame_regex ]]; then
    return 1
  fi
  if (( ${BASH_REMATCH[1]} == 0 \
    || ${BASH_REMATCH[2]} == 0 \
    || ${BASH_REMATCH[3]} == 0 \
    || ${BASH_REMATCH[3]} != ${BASH_REMATCH[4]} \
    || ${BASH_REMATCH[5]} != 0 \
    || ${BASH_REMATCH[6]} != 1 \
    || ${BASH_REMATCH[7]} != 3 \
    || ${BASH_REMATCH[2]} + ${BASH_REMATCH[3]} != ${BASH_REMATCH[1]} )); then
    return 1
  fi

  vm_count="$(grep -c '^VM_OK ' "$log_file" || true)"
  memory_count="$(grep -c '^MEMORY_OK: M1 reclaiming-frame and per-page-mapping foundation is alive$' "$log_file" || true)"
  vm_line="$(grep '^VM_OK ' "$log_file" || true)"
  vm_regex='^VM_OK maps=([0-9]+) unmaps=([0-9]+) remaps=([0-9]+) tlbi=([0-9]+) table_alloc=([0-9]+) table_free=([0-9]+) alias_checks=([0-9]+) stale=([0-9]+) duplicate_map=([0-9]+) missing_unmap=([0-9]+)$'
  if [[ "$vm_count" != "1" || "$memory_count" != "1" || ! "$vm_line" =~ $vm_regex ]]; then
    return 1
  fi
  (( ${BASH_REMATCH[1]} == 3 \
    && ${BASH_REMATCH[2]} == 3 \
    && ${BASH_REMATCH[3]} == 1 \
    && ${BASH_REMATCH[4]} == 6 \
    && ${BASH_REMATCH[5]} == 1 \
    && ${BASH_REMATCH[6]} == 1 \
    && ${BASH_REMATCH[7]} == 5 \
    && ${BASH_REMATCH[8]} == 0 \
    && ${BASH_REMATCH[9]} == 1 \
    && ${BASH_REMATCH[10]} == 1 ))
}
