#!/usr/bin/env bash

# Validates the strict heap and IRQ-restoration evidence emitted before any
# scheduler fault injection can fire. Intended to be sourced by test scripts.
validate_heap_evidence() {
  local log_file="$1"
  local heap_count heap_line heap_regex
  local irq_count irq_line irq_regex

  heap_count="$(grep -c '^HEAP_OK ' "$log_file" || true)"
  heap_line="$(grep '^HEAP_OK ' "$log_file" || true)"
  heap_regex='^HEAP_OK start=(0x[0-9a-f]+) bytes=([0-9]+) backing_pages=([0-9]+) table_frames=([0-9]+) maps=([0-9]+) frame_alloc_before=([0-9]+) frame_alloc_after=([0-9]+) free_before=([0-9]+) free_after=([0-9]+) allocations=([0-9]+) deallocations=([0-9]+) aligned=([0-9]+) whole=([0-9]+) checksum=(0x[0-9a-f]+)$'
  if [[ "$heap_count" != "1" || ! "$heap_line" =~ $heap_regex ]]; then
    return 1
  fi
  if (( ${BASH_REMATCH[1]} != 0x0000000102000000 \
    || ${BASH_REMATCH[2]} != 262144 \
    || ${BASH_REMATCH[3]} != 64 \
    || ${BASH_REMATCH[4]} != 1 \
    || ${BASH_REMATCH[5]} != ${BASH_REMATCH[3]} \
    || ${BASH_REMATCH[7]} - ${BASH_REMATCH[6]} != ${BASH_REMATCH[3]} + ${BASH_REMATCH[4]} \
    || ${BASH_REMATCH[8]} != ${BASH_REMATCH[2]} \
    || ${BASH_REMATCH[9]} != ${BASH_REMATCH[8]} \
    || ${BASH_REMATCH[10]} < 4 \
    || ${BASH_REMATCH[11]} != ${BASH_REMATCH[10]} \
    || ${BASH_REMATCH[12]} != 1 \
    || ${BASH_REMATCH[13]} != 1 \
    || ${BASH_REMATCH[14]} != 0x3d5b7904a2c18e5a )); then
    return 1
  fi

  irq_count="$(grep -c '^HEAP_IRQ_OK ' "$log_file" || true)"
  irq_line="$(grep '^HEAP_IRQ_OK ' "$log_file" || true)"
  irq_regex='^HEAP_IRQ_OK irq_before=([0-9]+) irq_after=([0-9]+) allocations=([0-9]+) deallocations=([0-9]+) free_before=([0-9]+) free_after=([0-9]+)$'
  if [[ "$irq_count" != "1" || ! "$irq_line" =~ $irq_regex ]]; then
    return 1
  fi
  (( ${BASH_REMATCH[1]} == 0 \
    && ${BASH_REMATCH[2]} == 0 \
    && ${BASH_REMATCH[3]} == 1 \
    && ${BASH_REMATCH[4]} == 1 \
    && ${BASH_REMATCH[5]} == ${BASH_REMATCH[6]} ))
}
