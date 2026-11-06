import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

from android_framework_inputs import (REQUIRED_METADATA, RUNTIME_JNI_ENTRYPOINTS,
                                      canonical_path, extract_framework, framework_inputs_complete, validate_inputs)


class FrameworkInputsTests(unittest.TestCase):
    def test_resume_extends_runtime_jni_without_reclassifying_core_or_losing_dependencies(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "inputs"
            output.mkdir()
            sdk = Path(temporary) / "sdk"
            readelf = sdk / "ndk/27.1.12297006/toolchains/llvm/prebuilt/host/bin/llvm-readelf"
            readelf.parent.mkdir(parents=True)
            readelf.touch()
            core = "/apex/com.android.art/lib64/libart.so"
            library = "/system/lib64/libandroid.so"
            missing = "/system/lib64/liblog.so"
            jni, = RUNTIME_JNI_ENTRYPOINTS
            names = [core, library, missing, jni, "/system/bin/app_process64",
                     "/system/lib64/libandroid_runtime.so", "/system/framework/framework.jar",
                     "/system/framework/framework-res.apk", *REQUIRED_METADATA]
            payloads = {name: name.encode() for name in names}
            # Keep a prior transitive edge whose file is absent, as well as a
            # library search ambiguity from the previous extraction pass.
            dependencies = {name: [] for name in names if name.endswith(".so") or name.endswith("app_process64")}
            del dependencies[jni]
            dependencies[library] = [missing]
            dependencies[missing] = [library]
            files = {}
            for name, data in payloads.items():
                if name in (jni, missing):
                    continue
                target = output / "root" / name.lstrip("/")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                files[name] = {"size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
            candidates = {"previous:libexample.so": [library, core]}
            receipt = {"files": files, "dependencies": dependencies, "total_bytes": sum(len(payloads[n]) for n in files),
                       "environment": {"BOOTCLASSPATH": "/system/framework/framework.jar"},
                       "framework": {"core_art_files": [core], "library_candidates": candidates}}
            (output / "art-provenance.json").write_text(json.dumps(receipt))
            (output / "art-inventory.txt").write_text("\n".join(names) + "\n")
            (output / "apex-info-list.xml").write_text('<apex-info-list><apex-info moduleName="com.android.tethering" isActive="true"/></apex-info-list>')
            pulled = []

            def command(*args, **kwargs):
                if args[:2] == ("shell", "find"):
                    return "\n".join(names)
                if args[:2] == ("shell", "stat"):
                    return str(len(payloads[args[-1]]))
                if args[:2] == ("shell", "readlink"):
                    return args[-1]
                if args[0] == "pull":
                    pulled.append(args[1])
                    Path(args[2]).write_bytes(payloads[args[1]])
                    return ""
                self.fail(f"Unexpected extraction command: {args}")

            with patch("android_framework_inputs.subprocess.run", return_value=SimpleNamespace(stdout="(NEEDED) Shared library: [libandroid.so]\n")) as elf:
                with patch("builtins.print"):
                    extract_framework(command, output, sdk)
            self.assertCountEqual(pulled, [missing, jni])
            elf.assert_called_once()
            updated = json.loads((output / "art-provenance.json").read_text())
            self.assertTrue(framework_inputs_complete(updated))
            self.assertEqual(updated["framework"]["core_art_files"], [core])
            self.assertEqual(updated["framework"]["runtime_jni_entrypoints"], [jni])
            self.assertEqual(updated["framework"]["library_candidates"], candidates)
            self.assertEqual(updated["dependencies"][jni], [library])
            self.assertEqual(updated["total_bytes"], sum(map(len, payloads.values())))
            self.assertEqual(updated["files"][core], files[core])
            self.assertFalse((output / "framework-progress.json").exists())

    def test_completion_requires_dynamic_jni_and_its_transitive_dependencies(self):
        jni, = RUNTIME_JNI_ENTRYPOINTS
        library = "/system/lib64/libandroid.so"
        libc = "/apex/com.android.runtime/lib64/bionic/libc.so"
        receipt = {"framework": {}, "files": dict.fromkeys(REQUIRED_METADATA, {}),
                   "dependencies": {jni: [library], library: [libc], libc: []}}
        # Version metadata alone used to make the early-return path accept a
        # receipt that still crashed in TrafficStats's System.loadLibrary.
        self.assertFalse(framework_inputs_complete(receipt))
        receipt["files"][jni] = {}
        receipt["files"][library] = {}
        self.assertFalse(framework_inputs_complete(receipt))
        receipt["files"][libc] = {}
        self.assertTrue(framework_inputs_complete(receipt))
        for path in (jni, library, libc):
            saved = receipt["dependencies"].pop(path)
            self.assertFalse(framework_inputs_complete(receipt))
            receipt["dependencies"][path] = saved
        # Cycles/shared dependencies are legal ELF graphs, not missing inputs.
        receipt["dependencies"][libc] = [library]
        self.assertTrue(framework_inputs_complete(receipt))
        del receipt["files"][REQUIRED_METADATA[0]]
        self.assertFalse(framework_inputs_complete(receipt))

    def test_namespaced_android_library_names_are_allowed_but_escapes_are_not(self):
        for name in ("/system/lib64/android.hardware.camera.device@3.2.so",
                     "/apex/com.android.art@361153120/lib64/libart.so"):
            self.assertTrue(canonical_path(name))
        for name in ("/system/../data/app/base.apk", "/system//lib64/libc.so",
                     "/system/./bin/app_process64", "/data/app/base.apk", "/vendor/bin/service"):
            self.assertFalse(canonical_path(name))

    def test_resume_rejects_changed_files_metadata_and_accounting(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            root = output / "root/system/bin"
            root.mkdir(parents=True)
            payload = b"owned fixture"
            entry = {"size": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}
            (root / "app_process64").write_bytes(payload)
            (output / "ld.config.txt").write_bytes(payload)
            receipt = {"files": {"/system/bin/app_process64": entry}, "total_bytes": len(payload),
                       "source": {"files": {}}, "metadata_files": {"ld.config.txt": entry}}
            manifest = output / "art-provenance.json"
            manifest.write_text(json.dumps(receipt))
            self.assertEqual(validate_inputs(output), receipt)
            for changed in (root / "app_process64", output / "ld.config.txt"):
                changed.write_bytes(b"altered input")
                with self.assertRaisesRegex(RuntimeError, "changed since extraction"):
                    validate_inputs(output)
                changed.write_bytes(payload)
            receipt["total_bytes"] += 1
            manifest.write_text(json.dumps(receipt))
            with self.assertRaisesRegex(RuntimeError, "byte total"):
                validate_inputs(output)


if __name__ == "__main__":
    unittest.main()
