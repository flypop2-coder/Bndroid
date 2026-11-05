#!/usr/bin/env python3
"""Host-side failure-path checks for the demo's QMP control connection."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
import unittest

from mobile_ui_demo import ROOT, validate_socket
from mobile_ui_qmp import Qmp


@contextmanager
def server(handler):
    # Keep AF_UNIX paths short even when macOS uses a long default TMPDIR.
    with tempfile.TemporaryDirectory(prefix="bndr-qmp-", dir="/tmp") as directory:
        path = str(Path(directory) / "qmp.sock")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(path)
            listener.listen(1)
            listener.settimeout(2)
            errors = []

            def run():
                try:
                    with listener.accept()[0] as connection:
                        connection.settimeout(2)
                        handler(connection)
                except (BrokenPipeError, ConnectionResetError):
                    pass  # Expected when a client rejects malformed/late data.
                except Exception as error:
                    errors.append(error)

            worker = threading.Thread(target=run, daemon=True)
            worker.start()
            try:
                yield path
            finally:
                worker.join(3)
                if worker.is_alive():
                    raise AssertionError("QMP fixture server failed to terminate")
                if errors:
                    raise errors[0]


def send(connection, payload):
    connection.sendall(json.dumps(payload).encode() + b"\n")


@contextmanager
def handshake(connection):
    send(connection, {"QMP": {"version": {}, "capabilities": []}})
    with connection.makefile("rb") as reader:
        request = json.loads(reader.readline())
        assert request["execute"] == "qmp_capabilities"
        send(connection, {"return": {}, "id": request["id"]})
        yield reader


class QmpTests(unittest.TestCase):
    def test_interleaved_events_do_not_hide_the_matching_reply(self):
        def handle(connection):
            with handshake(connection) as reader:
                request = json.loads(reader.readline())
                send(connection, {"event": "STOP"})
                send(connection, {"return": {"running": True}, "id": request["id"]})
                # close() must close every descriptor, so the peer sees EOF.
                self.assertEqual(reader.read(1), b"")

        with server(handle) as path:
            qmp = Qmp(path)
            try:
                self.assertEqual(qmp.command({"execute": "query-status"}), {"running": True})
            finally:
                qmp.close()

    def test_missing_socket_times_out(self):
        with tempfile.TemporaryDirectory(dir="/tmp") as directory:
            start = time.monotonic()
            with self.assertRaises(TimeoutError):
                Qmp(str(Path(directory) / "missing.sock"), timeout=0.05)
            self.assertLess(time.monotonic() - start, 1)

    def test_stalled_greeting_times_out_and_closes(self):
        def handle(connection):
            self.assertEqual(connection.recv(1), b"")

        with server(handle) as path:
            with self.assertRaises(TimeoutError):
                Qmp(path, timeout=0.05)

    def test_stalled_reply_is_bounded_even_with_incoming_events(self):
        def handle(connection):
            with handshake(connection) as reader:
                json.loads(reader.readline())
                for _ in range(100):
                    send(connection, {"event": "RESET"})
                    time.sleep(0.01)

        with server(handle) as path:
            qmp = Qmp(path, timeout=0.1)
            try:
                start = time.monotonic()
                with self.assertRaises(TimeoutError):
                    qmp.command({"execute": "query-status"})
                self.assertLess(time.monotonic() - start, 1)
            finally:
                qmp.close()

    def test_trickling_incomplete_json_cannot_extend_the_deadline(self):
        def handle(connection):
            for byte in b'{"QMP": {"version": {}, "capabilities": []}}\n':
                connection.sendall(bytes([byte]))
                time.sleep(0.01)

        with server(handle) as path:
            start = time.monotonic()
            with self.assertRaises(TimeoutError):
                Qmp(path, timeout=0.05)
            self.assertLess(time.monotonic() - start, 1)

    def test_mismatched_reply_is_rejected(self):
        def handle(connection):
            with handshake(connection) as reader:
                request = json.loads(reader.readline())
                send(connection, {"return": {}, "id": request["id"] + 1})

        with server(handle) as path:
            qmp = Qmp(path)
            try:
                with self.assertRaisesRegex(SystemExit, "ID did not match"):
                    qmp.command({"execute": "query-status"})
            finally:
                qmp.close()

    def test_command_errors_are_not_success(self):
        def handle(connection):
            with handshake(connection) as reader:
                request = json.loads(reader.readline())
                send(connection, {"error": {"desc": "denied"}, "id": request["id"]})

        with server(handle) as path:
            qmp = Qmp(path)
            try:
                with self.assertRaisesRegex(SystemExit, "QMP error"):
                    qmp.command({"execute": "query-status"})
            finally:
                qmp.close()


class DemoSocketTests(unittest.TestCase):
    def test_existing_socket_is_never_unlinked(self):
        root = ROOT / "target/mobile-ui"
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="socket-test.", dir=root) as directory:
            path = Path(directory) / "live.sock"
            path.write_bytes(b"owned by another session")
            with self.assertRaisesRegex(ValueError, "already exists"):
                validate_socket(str(path))
            self.assertEqual(path.read_bytes(), b"owned by another session")

    def test_escaped_socket_paths_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "inside target/mobile-ui"):
            validate_socket(str(ROOT / "target/mobile-ui/../elsewhere.sock"))


class DemoLaunchTests(unittest.TestCase):
    def test_plain_launch_reaches_qemu_and_reports_early_exit(self):
        # Exercise the actual shell/Python launch boundary, including macOS
        # Bash 3.2's empty-array behavior, without needing a guest build.
        with tempfile.TemporaryDirectory(dir="/tmp") as directory:
            root = Path(directory)
            image = root / "aarch64-unknown-none/release/bndroid-kernel.img"
            image.parent.mkdir(parents=True)
            image.touch()
            with (root / "bndroid-storage-m25.raw").open("wb") as disk:
                disk.truncate(8 * 1024 * 1024)
            qemu = root / "qemu-system-aarch64"
            qemu.write_text("#!/bin/sh\necho QEMU_LAUNCH_FIXTURE\nexit 7\n")
            qemu.chmod(0o755)
            environment = {key: value for key, value in os.environ.items() if not key.startswith("BNDROID_")}
            environment.update({
                "PATH": f"{root}{os.pathsep}{os.environ['PATH']}",
                "BNDROID_SKIP_BUILD": "1", "BNDROID_MOBILE_TARGET_DIR": str(root),
                "BNDROID_QEMU_DISPLAY": "none", "BNDROID_QEMU_TIMEOUT_SECONDS": "2",
            })
            result = subprocess.run(
                [str(ROOT / "scripts/run-mobile-ui.sh")], cwd=ROOT, env=environment,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=10,
            )
            self.assertEqual(result.returncode, 1, result.stdout)
            self.assertIn("QEMU exited unexpectedly (status 7)", result.stdout)
            self.assertIn("QEMU_LAUNCH_FIXTURE", result.stdout)


if __name__ == "__main__":
    unittest.main()
