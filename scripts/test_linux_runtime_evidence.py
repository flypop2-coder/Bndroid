import unittest
from linux_runtime_evidence import parse_marker


class SerialProofTests(unittest.TestCase):
    def test_every_partial_uart_prefix_is_pending(self):
        record = "BIONIC_RUNTIME_OK native_processes=2 reclaimed_anonymous_pages=1710 art=0\r\n"
        for length in range(len(record)):
            self.assertIsNone(parse_marker("booted\n" + record[:length], "BIONIC_RUNTIME_OK"))
        self.assertEqual(parse_marker(record, "BIONIC_RUNTIME_OK")["reclaimed_anonymous_pages"], "1710")

    def test_completed_record_ignores_unrelated_partial_output(self):
        self.assertEqual(parse_marker("LINUX_ABI_RUNTIME_OK abi=70\nnext line", "LINUX_ABI_RUNTIME_OK"), {"abi": "70"})

    def test_duplicate_proofs_are_rejected(self):
        with self.assertRaises(ValueError):
            parse_marker("OK abi=70\nOK abi=70\n", "OK")

    def test_malformed_and_ambiguous_fields_are_rejected(self):
        for record in ["OK abi=70 abi=23\n", "OK truncated\n", "OK abi=\n", "OK =70\n"]:
            with self.assertRaises(ValueError):
                parse_marker(record, "OK")


if __name__ == "__main__":
    unittest.main()
