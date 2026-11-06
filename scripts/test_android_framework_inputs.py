import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from android_framework_inputs import canonical_path, validate_inputs


class FrameworkInputsTests(unittest.TestCase):
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
