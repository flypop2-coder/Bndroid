import unittest

from art_runtime_evidence import completion_status, signal_failure_status


HELP = "dalvikvm: [options] class [argument ...]\nThe following ART arguments are supported:\n"
MEMORY = "ART_MEMORY_OK low_64m=1 protections=1 holes=1 shared_threads=1 kernel_isolation=1\n"
MEMORY += "ART_PROC_OK stack=1 maps=1 counters=1 snapshots=1 large_read=1 shared_ro=1 mremap=1 log=1\n"
MEMORY += "ART_PAGING_OK discard=1 rss=1 zero_fault=1 usercopy=1 protections=1 atomic_failure=1 cpu_time=1\n"
MEMORY += "ART_REMAP_OK fixed=1 replaced=1 readonly=1 growth_zero=1 source_unmapped=1 atomic_failure=1\n"
PAGING = "LINUX_PAGING_OK discarded=5000 demand_zero=3000\n"
LOG = "ANDROID_LOG pid={} buffer=0 priority=4 tag=bndroid-probe message=PROC_LOG_TRANSPORT_OK\n"
LOGS = LOG.format(4294967298) + LOG.format(8589934594)
JAVA = "ART_JAVA_CORE_OK sum=4950 exceptions=1 collections=1\n"
SIGNALS = "ART_SIGNALS_OK handlers=1 altstack=1 registers=1 masks=1 realtime=1 faults=1 threads=1 restart=1 waits=1\n"
SIGNALS += "ART_SYNC_OK requeue=1 realtime_clock=1 absolute_wait=1\n"
SIGNAL_PROOF = "LINUX_SIGNALS_OK delivered=30 returned=30 interrupted=10 pending=0\n"
PROOF = "BIONIC_DYNAMIC_RUNTIME_OK abi=70 native_processes=2 native_exits=2 native_reaped=2 remaining_processes=1 native_handles=0 live_anonymous_pages=0 live_anonymous_tables=0 live_file_descriptions=0 loader=upstream-android-linker peak_anonymous_pages=17000 reclaimed_anonymous_pages=34000\n"
PROCESS = "LINUX_PROCESS_EXIT pid=4294967298 code=0 native_handles=0\nLINUX_PROCESS_EXIT pid=8589934594 code=0 native_handles=0\nLINUX_THREADS_OK created=2 reaped=2 live=0 waiting=0\n"


class EvidenceTests(unittest.TestCase):
    def test_memfd_needs_both_processes_fault_returns_and_zero_live_resources(self):
        marker = "ART_MEMFD_OK aliases=1 code=1 truncate=1 seals=1 lifetime=1 wx=1\n"
        signals = "LINUX_SIGNALS_OK delivered=6 returned=6 interrupted=0 pending=0\n"
        log = marker * 2 + signals + PROCESS + PROOF
        self.assertEqual(completion_status(log, "memfd"), "memfd-passed")
        for old, new in (("returned=6", "returned=5"), ("delivered=6", "delivered=0"), ("pending=0", "pending=1")):
            self.assertEqual(completion_status(log.replace(old, new), "memfd"), "invalid-memfd-fault-proof")
        self.assertNotEqual(completion_status(log.replace("wx=1", "wx=0"), "memfd"), "memfd-passed")
        for peak in ("1000", "40000"):
            self.assertEqual(completion_status(log.replace("peak_anonymous_pages=17000", "peak_anonymous_pages=" + peak), "memfd"), "invalid-memfd-backing-proof")
        self.assertEqual(completion_status(log.replace("live_anonymous_pages=0", "live_anonymous_pages=1"), "memfd"), "invalid-cleanup-proof")

    def test_framework_requires_both_native_operations_and_authenticated_process_logs(self):
        def messages(pid):
            return "".join(f"ANDROID_LOG pid={pid} buffer=0 priority=4 tag=bndroid-framework message={message}\n"
                           for message in ("FRAMEWORK_STAGE parcel", "FRAMEWORK_STAGE looper",
                                           "FRAMEWORK_MAIN_OK parcel=1 bundle=1 looper=1 delayed=1"))
        log = messages(4294967298) + messages(8589934594) + PROCESS + PROOF
        self.assertEqual(completion_status(log, "framework"), "framework-passed")
        self.assertEqual(completion_status(log.replace("FRAMEWORK_STAGE looper", "wrong-stage"), "framework"), "missing-framework-stage")
        self.assertEqual(completion_status(log.replace("ANDROID_LOG pid=8589934594", "ANDROID_LOG pid=13"), "framework"), "invalid-process-proof")
        self.assertNotEqual(completion_status(HELP * 2 + PROCESS + PROOF, "framework"), "framework-passed")
        self.assertNotEqual(completion_status(HELP * 2 + PROCESS + PROOF, "framework-help"), "framework-help-passed")
        help_records = ""
        for pid in (4294967298, 8589934594):
            for priority, tag, message in ((3, "AndroidRuntime", ">>>>>> START com.android.internal.os.RuntimeInit uid 10000 <<<<<< "),
                                           (4, "vm-printf", "dalvikvm: [options] class [argument ...] "),
                                           (4, "AndroidRuntime", "VM exiting with result code 0.")):
                help_records += f"ANDROID_LOG pid={pid} buffer=0 priority={priority} tag={tag} message={message}\n"
        self.assertEqual(completion_status(help_records + PROCESS + PROOF, "framework-help"), "framework-help-passed")
        self.assertEqual(completion_status(help_records.replace("VM exiting with result code 0.", "wrong-exit") + PROCESS + PROOF, "framework-help"), "missing-framework-frontend-proof")
        properties = "ART_PROPERTIES_OK bionic_reader=1 callback=1 enumerate=1 wait=1 immutable=1\n"
        property_process = PROCESS.replace("live=0 waiting=0", "live=0 waiting=0 waits=2 timeouts=2")
        self.assertEqual(completion_status(properties * 2 + property_process + PROOF, "properties"), "properties-passed")
        self.assertEqual(completion_status(properties * 2 + PROCESS + PROOF, "properties"), "missing-property-wait-proof")
        self.assertNotEqual(completion_status(properties + PROCESS + PROOF, "properties"), "properties-passed")

    def test_managed_workload_requires_all_stages_and_real_reclaim_and_thread_cleanup(self):
        marker="ART_MANAGED_OK java_threads=3 monitors=1 interrupt=1 gc=1 jni=1 native_thread=1 clocks=1\n"
        marker+="ART_APK_INPUT_OK dex=apk native=apk namespace=isolated\n"
        stages="".join("ART_MANAGED_STAGE "+stage+"\n" for stage in ("clocks","threads","gc","jni"))
        log=(marker+stages)*2+PROCESS.replace("created=2 reaped=2", "created=20 reaped=20")+PROOF+PAGING
        self.assertEqual(completion_status(log,"managed"),"managed-passed")
        self.assertEqual(completion_status(log.replace("native=apk","native=file"),"managed"),"missing-apk-input-proof")
        self.assertEqual(completion_status(log.replace("ART_MANAGED_STAGE jni\n",""),"managed"),"missing-managed-stage")
        self.assertEqual(completion_status(log.replace("demand_zero=3000","demand_zero=0"),"managed"),"invalid-managed-runtime-proof")
        memory=MEMORY*2+LOGS+PROCESS+PROOF+PAGING
        self.assertEqual(completion_status(memory,"memory"),"memory-passed")
        self.assertEqual(completion_status(memory.replace("discarded=5000","discarded=0"),"memory"),"invalid-paging-proof")
        self.assertEqual(completion_status(memory.replace("fixed=1", "fixed=0"),"memory"),"missing-fixed-remap-proof")
    def test_signal_proof_requires_returns_and_complete_thread_cleanup(self):
        log=SIGNALS*2+PROCESS+PROOF+SIGNAL_PROOF
        self.assertEqual(completion_status(log,"signals"),"signals-passed")
        self.assertEqual(completion_status(log.replace("requeue=1", "requeue=0"),"signals"),"missing-sync-proof")
        for bad in ("returned=29","pending=1"):
            original="returned=30" if bad.startswith("returned") else "pending=0"
            self.assertEqual(completion_status(log.replace(original,bad),"signals"),"invalid-signal-proof")

    def test_bad_signal_frames_require_the_expected_fault_and_all_resource_cleanup(self):
        log=("SIGNAL_BAD_FRAME_ARMED\nLINUX_SIGNAL_FATAL pid=42 tid=2 signal=11\nLINUX_USER_FAULT pid=42 reason=4\n"
             "LINUX_THREADS_FAILURE_CLEANUP created=0 reaped=0 live=0 waiting=0\nLINUX_SIGNALS_OK delivered=1 returned=0 pending=0\n"
             "LINUX_RUNTIME_FAILURE_CLEANUP live_processes=1 faulted=1 reaped=1 live_pages=0 live_tables=0 live_files=0\n")
        self.assertEqual(signal_failure_status(log,"signal-bad-frame"),"signal-bad-frame-passed")
        self.assertIsNone(signal_failure_status(log.rstrip(),"signal-bad-frame"))
        for old,new in (("reason=4","reason=2"),("live_pages=0","live_pages=1"),("pending=0","pending=1"),("faulted=1","faulted=0"),("pid=42 reason","pid=43 reason")):
            self.assertEqual(signal_failure_status(log.replace(old,new),"signal-bad-frame"),"invalid-signal-containment")
        self.assertEqual(signal_failure_status(log+"LINUX_SIGNAL_FATAL pid=42 tid=2 signal=11\n","signal-bad-frame"),"invalid-signal-containment")
    def test_frontend_cannot_pass_managed_gate(self):
        log = HELP * 2 + PROCESS + PROOF
        self.assertEqual(completion_status(log, "help"), "help-passed")
        self.assertEqual(completion_status(log, "java"), "exited-without-expected-proof")

    def test_managed_and_memory_results_need_their_own_markers(self):
        for mode, marker in (("java", JAVA), ("memory", MEMORY)):
            self.assertEqual(completion_status(marker * 2 + LOGS + PROCESS + PROOF + PAGING, mode), mode + "-passed")
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
        self.assertEqual(completion_status(MEMORY * 2 + LOGS + PROCESS + PROOF.replace("17000", "100") + PAGING, "memory"), "invalid-memory-proof")

    def test_prefixed_markers_and_misattributed_logs_cannot_claim_success(self):
        self.assertEqual(completion_status(("ANDROID_LOG message=" + JAVA) * 2 + PROCESS + PROOF, "java"), "exited-without-expected-proof")
        self.assertEqual(completion_status(MEMORY * 2 + LOGS.replace("8589934594", "4294967298") + PROCESS + PROOF + PAGING, "memory"), "missing-log-proof")


if __name__ == "__main__":
    unittest.main()
