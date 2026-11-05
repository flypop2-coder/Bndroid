import unittest

from art_runtime_evidence import completion_status


HELP = "dalvikvm: [options] class [argument ...]\nThe following ART arguments are supported:\n"
MEMORY = "ART_MEMORY_OK low_64m=1 protections=1 holes=1 shared_threads=1 kernel_isolation=1\n"
JAVA = "ART_JAVA_CORE_OK sum=4950 exceptions=1 collections=1\n"
PROOF = "BIONIC_DYNAMIC_RUNTIME_OK abi=70 native_processes=2 native_exits=2 native_reaped=2 remaining_processes=1 native_handles=0 live_anonymous_pages=0 live_anonymous_tables=0 live_file_descriptions=0 loader=upstream-android-linker peak_anonymous_pages=17000 reclaimed_anonymous_pages=34000\n"
PROCESS = "LINUX_PROCESS_EXIT pid=4294967298 code=0 native_handles=0\nLINUX_PROCESS_EXIT pid=8589934594 code=0 native_handles=0\nLINUX_THREADS_OK created=2 reaped=2 live=0 waiting=0\n"


class EvidenceTests(unittest.TestCase):
    def test_frontend_cannot_pass_managed_gate(self):
        log = HELP * 2 + PROCESS + PROOF
        self.assertEqual(completion_status(log, "help"), "help-passed")
        self.assertEqual(completion_status(log, "java"), "exited-without-expected-proof")

    def test_managed_and_memory_results_need_their_own_markers(self):
        for mode, marker in (("java", JAVA), ("memory", MEMORY)):
            self.assertEqual(completion_status(marker * 2 + PROCESS + PROOF, mode), mode + "-passed")
            self.assertNotEqual(completion_status(marker + PROCESS + PROOF, mode), mode + "-passed")

    def test_incomplete_or_duplicate_uart_record_is_not_success(self):
        log = HELP * 2 + PROCESS
        self.assertIsNone(completion_status(log + PROOF.rstrip(), "help"))
        self.assertEqual(completion_status(log + PROOF * 2, "help"), "invalid-record")

    def test_leaked_resources_reject_success(self):
        for key in ("live_anonymous_pages", "live_anonymous_tables", "live_file_descriptions", "native_handles"):
            log = HELP * 2 + PROCESS + PROOF.replace(key + "=0", key + "=1")
            self.assertEqual(completion_status(log, "help"), "invalid-cleanup-proof")
        self.assertEqual(completion_status(HELP * 2 + PROCESS.replace("reaped=2", "reaped=1") + PROOF, "help"), "invalid-thread-proof")

    def test_repeated_pid_or_insufficient_memory_coverage_is_rejected(self):
        self.assertEqual(completion_status(HELP * 2 + PROCESS.replace("8589934594", "4294967298") + PROOF, "help"), "invalid-process-proof")
        self.assertEqual(completion_status(MEMORY * 2 + PROCESS + PROOF.replace("17000", "100"), "memory"), "invalid-memory-proof")


if __name__ == "__main__":
    unittest.main()
