import unittest

from android_properties import BOOT_PROPERTIES, encode_properties


class PropertyEncodingTests(unittest.TestCase):
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
