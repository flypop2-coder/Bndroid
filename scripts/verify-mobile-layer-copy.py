#!/usr/bin/env python3
"""Audit actual GraphicsBufferWrite costs against accepted QEMU layer frames."""

import argparse
import json
from pathlib import Path


FULL_FRAME_BYTES = 720 * 1600 * 4


def records(text, marker):
    return [dict(item.split("=", 1) for item in line.split()[1:] if "=" in item)
            for line in text.splitlines() if line.startswith(marker + " ")]


def verify(text, expected_pixels):
    commits = records(text, "USER_SURFACE_LAYERED_COMMIT_OK")
    costs = records(text, "MOBILE_LAYER_COPY_COST_OK")
    if not commits or len(commits) != len(costs):
        raise ValueError("every layered commit needs one kernel copy-cost record")
    callbacks = []
    reused_chrome = 0
    previous = None
    generations = {}
    for commit, cost in zip(commits, costs):
        for key in ("frame_id", "content_producer_pid", "content_generation", "chrome_generation"):
            if commit[key] != cost[key]:
                raise ValueError(f"copy-cost identity mismatch: {key}")
        producer = cost["content_producer_pid"]
        generation = int(cost["content_generation"])
        chrome = int(cost["chrome_generation"])
        calls = int(cost["write_calls_total"])
        size = int(cost["write_bytes_total"])
        if generation < generations.get(producer, 0) or generation <= 0 or chrome <= 0:
            raise ValueError("nonmonotonic producer generation")
        generations[producer] = generation
        if previous:
            if chrome < int(previous["chrome_generation"]):
                raise ValueError("chrome generation decreased")
            call_delta = calls - int(previous["write_calls_total"])
            byte_delta = size - int(previous["write_bytes_total"])
            if call_delta < 0 or byte_delta < 0 or not 4 * call_delta <= byte_delta <= 63360 * call_delta:
                raise ValueError("nonmonotonic or impossible copy counters")
            reused_chrome += int(chrome == int(previous["chrome_generation"]))
            count = int(commit["damage_rects"])
            pixels = int(commit["damage_pixels"])
            if commit["damage_mode"] == "damage" and count == 2 and pixels in expected_pixels:
                rects = [tuple(map(int, commit[f"damage{i}"].split("/"))) for i in range(count)]
                if any(len(r) != 4 or r[0] < 0 or r[1] < 64 or r[2] <= 0 or r[3] <= 0
                       or r[0] + r[2] > 720 or r[1] + r[3] > 1512 for r in rects):
                    raise ValueError("callback region escaped client content")
                (ax, ay, aw, ah), (bx, by, bw, bh) = rects
                if ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah:
                    raise ValueError("callback regions overlap")
                if sum(r[2] * r[3] for r in rects) != pixels:
                    raise ValueError("callback area mismatch")
                if producer != previous["content_producer_pid"]:
                    raise ValueError("callback has no consecutive same-producer baseline")
                if chrome != int(previous["chrome_generation"]):
                    raise ValueError("content callback rewrote unchanged trusted chrome")
                if byte_delta != pixels * 4:
                    raise ValueError(f"callback copied {byte_delta} bytes, expected only {pixels * 4}")
                if call_delta != generation - int(previous["content_generation"]):
                    raise ValueError("callback copy calls differ from producer generation delta")
                callbacks.append({
                    "frame_id": int(cost["frame_id"]), "pixels": pixels,
                    "copy_bytes": byte_delta, "copy_calls": call_delta,
                    "chrome_copy_calls": 0, "regions": rects,
                    "saved_bytes_vs_full_layers": FULL_FRAME_BYTES - byte_delta,
                })
        previous = cost
    observed = {entry["pixels"] for entry in callbacks}
    if not expected_pixels <= observed:
        raise ValueError(f"missing measured callback regions: {sorted(expected_pixels - observed)}")
    return {
        "terminal": "MOBILE_LAYER_COPY_CHECK_OK", "commits": len(commits),
        "reused_chrome_commits": reused_chrome, "callbacks": callbacks,
        "copy_calls_total": int(costs[-1]["write_calls_total"]),
        "copy_bytes_total": int(costs[-1]["write_bytes_total"]),
        "all_full_baseline_bytes_per_frame": FULL_FRAME_BYTES,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callback-pixels", type=int, action="append", required=True)
    parser.add_argument("log", type=Path)
    args = parser.parse_args()
    try:
        result = verify(args.log.read_text(), set(args.callback_pixels))
    except (KeyError, TypeError, ValueError) as error:
        raise SystemExit(f"layer copy verification failed: {error}") from error
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
