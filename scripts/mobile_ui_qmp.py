#!/usr/bin/env python3
"""Small QMP client for deterministic Bndroid mobile-preview interaction."""

from __future__ import annotations

import json
import math
from pathlib import Path
import socket
import sys
import time

SCANOUT_WIDTH = 720
SCANOUT_HEIGHT = 1600


class Qmp:
    def __init__(self, socket_path: str, timeout: float = 5.0) -> None:
        if not math.isfinite(timeout) or timeout <= 0:
            raise ValueError("QMP timeout must be finite and positive")
        self.timeout = timeout
        self.connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.pending = bytearray()
        self.request_id = 0
        try:
            deadline = time.monotonic() + timeout
            while True:
                self._set_deadline(deadline)
                try:
                    self.connection.connect(socket_path)
                    break
                except (FileNotFoundError, ConnectionRefusedError):
                    time.sleep(min(0.01, max(0, deadline - time.monotonic())))
            greeting = self._read(deadline)
            if "QMP" not in greeting:
                raise SystemExit("invalid QMP greeting")
            self.command({"execute": "qmp_capabilities"})
        except BaseException:
            self.close()
            raise

    def _set_deadline(self, deadline: float) -> None:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("QMP deadline exceeded")
        self.connection.settimeout(remaining)

    def _read(self, deadline: float) -> dict[str, object]:
        while b"\n" not in self.pending:
            self._set_deadline(deadline)
            chunk = self.connection.recv(4096)
            if not chunk:
                raise SystemExit("QMP connection closed")
            self.pending.extend(chunk)
            if len(self.pending) > 1024 * 1024:
                raise SystemExit("oversized QMP response")
        line, _, remaining = self.pending.partition(b"\n")
        self.pending = bytearray(remaining)
        response = json.loads(line)
        if not isinstance(response, dict):
            raise SystemExit("invalid QMP response")
        return response

    def close(self) -> None:
        self.connection.close()

    def command(self, payload: dict[str, object]) -> object:
        self.request_id += 1
        payload = {**payload, "id": self.request_id}
        deadline = time.monotonic() + self.timeout
        self._set_deadline(deadline)
        self.connection.sendall(json.dumps(payload).encode("ascii") + b"\n")
        while True:
            response = self._read(deadline)
            if "event" in response:
                continue
            if response.get("id") != self.request_id:
                raise SystemExit("QMP response ID did not match the request")
            if "error" in response:
                raise SystemExit(f"QMP error: {response['error']}")
            if "return" in response:
                return response["return"]
            raise SystemExit("QMP response has neither return nor error")

    def input_events(self, events: list[dict[str, object]]) -> None:
        self.command(
            {
                "execute": "input-send-event",
                "arguments": {"events": events},
            }
        )


def raw_axis(pixel: int, extent: int) -> int:
    if pixel < 0 or pixel >= extent:
        raise SystemExit(f"pixel {pixel} is outside extent {extent}")
    raw = (pixel * 32767 + extent - 2) // (extent - 1)
    if raw * (extent - 1) // 32767 != pixel:
        raise SystemExit("absolute-axis inverse mapping was not exact")
    return raw


def position(qmp: Qmp, x: int, y: int) -> None:
    qmp.input_events(
        [
            {
                "type": "abs",
                "data": {"axis": "x", "value": raw_axis(x, SCANOUT_WIDTH)},
            },
            {
                "type": "abs",
                "data": {"axis": "y", "value": raw_axis(y, SCANOUT_HEIGHT)},
            },
        ]
    )


def move(qmp: Qmp, x: int, y: int) -> None:
    position(qmp, x, y)
    time.sleep(0.05)


def touch_down(qmp: Qmp, x: int, y: int) -> None:
    move(qmp, x, y)
    qmp.input_events(
        [{"type": "btn", "data": {"down": True, "button": "touch"}}]
    )
    time.sleep(0.05)


def touch_move(qmp: Qmp, x: int, y: int) -> None:
    move(qmp, x, y)


def touch_up(qmp: Qmp) -> None:
    qmp.input_events(
        [{"type": "btn", "data": {"down": False, "button": "touch"}}]
    )
    time.sleep(0.05)


def tap(qmp: Qmp, x: int, y: int) -> None:
    move(qmp, x, y)
    qmp.input_events(
        [{"type": "btn", "data": {"down": True, "button": "touch"}}]
    )
    time.sleep(0.05)
    qmp.input_events(
        [{"type": "btn", "data": {"down": False, "button": "touch"}}]
    )
    time.sleep(0.05)


def drag(qmp: Qmp, start_x: int, start_y: int, end_x: int, end_y: int) -> None:
    move(qmp, start_x, start_y)
    qmp.input_events(
        [{"type": "btn", "data": {"down": True, "button": "touch"}}]
    )
    time.sleep(0.05)
    move(qmp, end_x, end_y)
    qmp.input_events(
        [{"type": "btn", "data": {"down": False, "button": "touch"}}]
    )
    time.sleep(0.05)


def burst_taps(qmp: Qmp, taps: list[tuple[int, int]]) -> None:
    for x, y in taps:
        position(qmp, x, y)
        time.sleep(0.005)
        qmp.input_events(
            [{"type": "btn", "data": {"down": True, "button": "touch"}}]
        )
        time.sleep(0.005)
        qmp.input_events(
            [{"type": "btn", "data": {"down": False, "button": "touch"}}]
        )
        time.sleep(0.005)


def probe(qmp: Qmp) -> None:
    qmp.input_events(
        [
            {"type": "abs", "data": {"axis": "x", "value": 1234}},
            {"type": "abs", "data": {"axis": "y", "value": 23456}},
        ]
    )
    time.sleep(0.05)
    qmp.input_events(
        [{"type": "btn", "data": {"down": True, "button": "touch"}}]
    )
    time.sleep(0.05)
    qmp.input_events(
        [{"type": "btn", "data": {"down": False, "button": "touch"}}]
    )
    time.sleep(0.05)


def screenshot(qmp: Qmp, output: str) -> None:
    path = str(Path(output).resolve())
    qmp.command({"execute": "stop"})
    try:
        qmp.command(
            {
                "execute": "screendump",
                "arguments": {"filename": path, "format": "ppm"},
            }
        )
    finally:
        qmp.command({"execute": "cont"})


def usage() -> None:
    raise SystemExit(
        "usage: mobile_ui_qmp.py SOCKET "
        "(probe | move X Y | touch-down X Y | touch-move X Y | touch-up "
        "| tap X Y | drag START_X START_Y END_X END_Y "
        "| burst-taps X1 Y1 X2 Y2 "
        "| screenshot OUTPUT)"
    )


def main() -> None:
    if len(sys.argv) < 3:
        usage()
    qmp = Qmp(sys.argv[1])
    try:
        action = sys.argv[2]
        if action == "probe" and len(sys.argv) == 3:
            probe(qmp)
        elif action == "move" and len(sys.argv) == 5:
            move(qmp, int(sys.argv[3]), int(sys.argv[4]))
        elif action == "touch-down" and len(sys.argv) == 5:
            touch_down(qmp, int(sys.argv[3]), int(sys.argv[4]))
        elif action == "touch-move" and len(sys.argv) == 5:
            touch_move(qmp, int(sys.argv[3]), int(sys.argv[4]))
        elif action == "touch-up" and len(sys.argv) == 3:
            touch_up(qmp)
        elif action == "tap" and len(sys.argv) == 5:
            tap(qmp, int(sys.argv[3]), int(sys.argv[4]))
        elif action == "drag" and len(sys.argv) == 7:
            drag(
                qmp,
                int(sys.argv[3]),
                int(sys.argv[4]),
                int(sys.argv[5]),
                int(sys.argv[6]),
            )
        elif action == "burst-taps" and len(sys.argv) == 7:
            burst_taps(
                qmp,
                [
                    (int(sys.argv[3]), int(sys.argv[4])),
                    (int(sys.argv[5]), int(sys.argv[6])),
                ],
            )
        elif action == "screenshot" and len(sys.argv) == 4:
            screenshot(qmp, sys.argv[3])
        else:
            usage()
    finally:
        qmp.close()


if __name__ == "__main__":
    main()
