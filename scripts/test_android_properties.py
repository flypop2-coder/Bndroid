import unittest

from android_properties import BOOT_PROPERTIES, encode_properties, runtime_properties


class PropertyEncodingTests(unittest.TestCase):
    def test_framework_version_requires_the_frozen_source(self):
        props = runtime_properties({"package_properties": "AndroidVersion.ApiLevel=36.1\nPkg.Revision=3\n"})
        self.assertEqual(len(props), 13)
        self.assertEqual(props["ro.build.version.sdk_full"], "36.1")
        self.assertEqual(props["ro.build.version.all_codenames"], "REL")
        self.assertEqual(props["ro.product.name"], "bndroid")
        self.assertEqual(len(encode_properties(props)), 128 * 1024)
        for version in ("", "35", "36", "37.1"):
            with self.assertRaises(ValueError):
                runtime_properties({"package_properties": "AndroidVersion.ApiLevel=" + version})

    def test_boot_image_is_deterministic_and_bounded(self):
        encoded = encode_properties(BOOT_PROPERTIES)
        self.assertEqual(encoded, encode_properties(dict(reversed(list(BOOT_PROPERTIES.items())))))
        self.assertEqual(len(encoded), 128 * 1024)

    def test_mutable_names_ambiguous_names_and_unrepresentable_values_are_rejected(self):
        for name, value in (("persist.example", "1"), ("ro..example", "1"), ("ro.x/escape", "1"),
                            ("ro.example", "a\0b"), ("ro.example", "a" * 92),
                            ("ro.example", "界" * 31)):
            with self.assertRaises(ValueError):
                encode_properties({name: value})
        with self.assertRaisesRegex(ValueError, "exceeds 128 KiB"):
            encode_properties({"ro." + "a" * (128 * 1024): "value"})


if __name__ == "__main__":
    unittest.main()
