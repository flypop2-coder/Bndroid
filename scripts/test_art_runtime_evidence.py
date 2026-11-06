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
    def test_service_bridge_requires_restart_authority_and_all_process_owners(self):
        provider = "RUNTIME_SERVICE_READY pid=3 uid=1001 instance=100\nRUNTIME_SERVICE_STOPPED instance=100\nRUNTIME_SERVICE_READY pid=3 uid=1001 instance=200\n"
        controller = "SERVICE_BRIDGE_CONTROLLER_OK pid=5 uid=1002 instance=100 control=authorized\n"
        client = "SERVICE_BRIDGE_CLIENT_OK pid=4 uid=10000 first=100 second=200 facts=1 identity=1 add_denied=1 stop_denied=1 malformed=1 death=1 stale=1 rediscovery=1 daemon_ready=1\n"
        binder = "LINUX_BINDER_OK opened=5 closed=5 submitted=12 replied=12 security_contexts=8 deaths=2 connections=0 transactions=0 buffers=0 nodes=0 references=0 call_frames=0 pending_fds=0\n"
        poll = "LINUX_POLL_OK eventfds=1 epolls=1 external=5 timers=1 closed=8 external_delivered=3 live=0 watches=0\n"
        processes = "".join(f"LINUX_PROCESS_EXIT pid={4294967296 + pid} code=0 native_handles=0\n" for pid in (3, 4, 5))
        processes += "LINUX_THREADS_OK created=2 reaped=2 live=0 waiting=0\n"
        proof = PROOF.replace("native_processes=2 native_exits=2 native_reaped=2", "native_processes=5 native_exits=3 native_killed=2 native_reaped=5")
        log = provider + controller + client + binder + poll + processes + proof
        self.assertEqual(completion_status(log, "framework-bridge"), "framework-bridge-passed")
        for old, new, expected in (
                ("instance=200", "instance=100", "invalid-provider-restart-proof"),
                ("control=authorized", "control=denied", "missing-service-control-proof"),
                ("stop_denied=1", "stop_denied=0", "missing-service-client-proof"),
                ("second=200", "second=100", "missing-service-client-proof"),
                ("pid=4294967301", "pid=4294967302", "invalid-service-owner-proof"),
                ("security_contexts=8", "security_contexts=0", "missing-service-bridge-transport-proof"),
                ("deaths=2", "deaths=0", "missing-service-bridge-transport-proof"),
                ("native_killed=2", "native_killed=0", "invalid-cleanup-proof"),
                ("nodes=0", "nodes=1", "invalid-service-binder-proof"),
                ("instance=200", "instance=bad", "invalid-service-bridge-record")):
            self.assertEqual(completion_status(log.replace(old, new), "framework-bridge"), expected)
        self.assertEqual(completion_status(log + controller, "framework-bridge"), "invalid-service-bridge-record")
        java_client = "ANDROID_LOG pid=4294967300 buffer=0 priority=4 tag=bndroid-service message=" + client.replace("SERVICE_BRIDGE_CLIENT_OK", "FRAMEWORK_SERVICE_CLIENT_OK").rstrip() + " looper=1\n"
        stages = "".join("ANDROID_LOG pid=4294967300 buffer=0 priority=4 tag=bndroid-service message=FRAMEWORK_SERVICE_STAGE " + stage + "\n" for stage in ("snapshot", "authority", "death"))
        shutdown = "ANDROID_LOG pid=4294967300 buffer=0 priority=3 tag=AndroidRuntime message=Shutting down VM\n"
        managed = log.replace(client, java_client + stages + shutdown)
        self.assertEqual(completion_status(managed, "framework-client"), "framework-client-passed")
        for old, new, expected in (
                ("looper=1", "looper=0", "missing-framework-service-proof"),
                (stages, "", "missing-framework-service-proof"),
                ("\nANDROID_LOG pid=4294967300 buffer=0 priority=4 tag=bndroid-service message=FRAMEWORK_SERVICE_STAGE authority", "\naudit without newline ANDROID_LOG pid=4294967300 buffer=0 priority=4 tag=bndroid-service message=FRAMEWORK_SERVICE_STAGE authority", "missing-framework-service-proof"),
                ("ANDROID_LOG pid=4294967300", "ANDROID_LOG pid=4294967301", "missing-framework-service-proof"),
                (shutdown, "", "missing-framework-shutdown-proof"),
                (java_client, client, "missing-service-client-proof")):
            self.assertEqual(completion_status(managed.replace(old, new), "framework-client"), expected)

    def test_launch_requires_both_identities_ppoll_timers_and_cleanup(self):
        timers = "LAUNCH_TIMERS_OK periodic=1 absolute=1 poll=1 lifetime=1 interrupted=1\n"
        ppoll = "LAUNCH_PPOLL_OK invalid=1 ignored=1 timer_wake=1 timeout=1\n"
        identities = "".join(f"LAUNCH_IDENTITY_OK role={role} uid={uid} gid={uid} auxv=1 signals=1 binder=1 authority=1\n" for role, uid in (("system", 1000), ("app", 10000)))
        binder = "LINUX_BINDER_OK opened=2 closed=2 submitted=1 replied=1 connections=0 transactions=0 buffers=0 nodes=0 references=0 call_frames=0 pending_fds=0\n"
        poll = "LINUX_POLL_OK eventfds=0 epolls=2 external=2 timers=10 closed=14 ready=4 cancelled=2 live=0 watches=0\n"
        signals = "LINUX_SIGNALS_OK delivered=4 returned=4 pending=0\n"
        policy = "LAUNCH_POLICY_OK context=1 find=1 add_authority=1 unknown_denied=1 property_authority=1 shared_property=1\n"
        binder = binder.rstrip("\n") + " security_contexts=1\n"
        log = policy * 2 + timers * 2 + ppoll * 2 + identities + binder + poll + signals + PROCESS + PROOF
        self.assertEqual(completion_status(log, "framework-identities"), "framework-identities-passed")
        for old, new, expected in ((ppoll, "", "missing-ppoll-proof"),
                                   (policy, "", "missing-native-policy-proof"),
                                   ("security_contexts=1", "security_contexts=0", "missing-binder-security-proof"),
                                   ("role=app uid=10000", "role=app uid=1000", "missing-distinct-identity-proof"),
                                   ("pending_fds=0", "pending_fds=1", "invalid-service-binder-proof"),
                                   ("closed=14", "closed=13", "invalid-service-poll-proof"),
                                   ("ready=4", "ready=0", "invalid-launch-timer-proof")):
            self.assertEqual(completion_status(log.replace(old, new), "framework-identities"), expected)

    def test_service_success_requires_real_daemon_loop_and_distinct_forced_shutdown(self):
        marker = "SERVICE_MANAGER_CLIENT_OK upstream=1 lookup=1 ping=1 uid=10000\n"
        binder = "LINUX_BINDER_OK opened=2 closed=2 submitted=2 replied=2 connections=0 transactions=0 buffers=0 nodes=0 references=0 call_frames=0 pending_fds=0\n"
        poll = "LINUX_POLL_OK eventfds=1 epolls=1 external=2 timers=1 closed=5 external_delivered=2 live=0 watches=0\n"
        processes = PROCESS.replace("LINUX_PROCESS_EXIT pid=4294967298 code=0 native_handles=0\n", "")
        proof = PROOF.replace("native_exits=2", "native_exits=1 native_killed=1")
        binder = binder.rstrip("\n") + " security_contexts=2\n"
        log = "SERVICE_MANAGER_POLICY_OK ready=1 spoof_denied=1 unknown_service_denied=1\n" + marker + binder + poll + processes + proof
        self.assertEqual(completion_status(log, "framework-services"), "framework-services-passed")
        for old, new, expected in (("native_killed=1", "native_killed=0", "invalid-cleanup-proof"),
                                   ("SERVICE_MANAGER_POLICY_OK", "REMOVED", "missing-native-policy-proof"),
                                   ("security_contexts=2", "security_contexts=0", "missing-binder-security-proof"),
                                   ("external_delivered=2", "external_delivered=0", "missing-upstream-service-loop-proof"),
                                   (marker, "SERVICE_MANAGER_CLIENT_TIMEOUT\n", "missing-service-launch-proof")):
            self.assertEqual(completion_status(log.replace(old, new), "framework-services"), expected)

    def test_poll_requires_real_waits_signals_and_complete_lifetime_cleanup(self):
        marker = "POLL_NATIVE_OK counter=1 semaphore=1 epoll=1 vectors=1 nested=1 lifetime=1 race=1 signals=1 timeout=1 pending_exit=1\n"
        poll = "LINUX_POLL_OK eventfds=28 epolls=14 closed=42 reads=34 writes=42 delivered=26 controls=26 blocked=30 ready=16 expired=4 cancelled=10 live=0 watches=0\n"
        signals = "LINUX_SIGNALS_OK delivered=8 returned=8 interrupted=8 pending=0\n"
        log = marker * 2 + poll + signals + PROCESS.replace("created=2 reaped=2", "created=20 reaped=20") + PROOF
        self.assertEqual(completion_status(log, "poll"), "poll-passed")
        for old, new, expected in (("watches=0", "watches=1", "invalid-poll-cleanup"),
                                   ("closed=42", "closed=41", "invalid-poll-cleanup"),
                                   ("expired=4", "expired=0", "missing-poll-transport-proof"),
                                   ("cancelled=10", "cancelled=0", "missing-poll-transport-proof"),
                                   ("returned=8", "returned=7", "invalid-poll-signal-proof"),
                                   ("closed=42", "closed=invalid", "invalid-poll-record")):
            self.assertEqual(completion_status(log.replace(old,new), "poll"), expected)
        self.assertEqual(completion_status(log+poll, "poll"), "invalid-poll-record")
        self.assertNotEqual(completion_status(log.replace(marker,"",1), "poll"), "poll-passed")

    def test_binder_requires_cross_process_transport_signal_and_cleanup_proof(self):
        client = "BINDER_CLIENT_OK identity=1 reply=1 oneway=3 death=1 interrupted=1 fd_reuse=1 readonly=1\n"
        server = "BINDER_SERVER_OK identity=1 reply=1 oneway=3 receive_pool=1 readonly=1 domains=3\n"
        binder = "LINUX_BINDER_OK opened=5 closed=5 submitted=25 replied=14 oneway=6 failed=16 deaths=3 connections=0 transactions=0 buffers=0 nodes=0 references=0 call_frames=0 pending_fds=0 fds_submitted=11 fds_delivered=5 fd_install_failures=2 objects=8 reference_events=14 nested=2 max_depth=3 first_sender=4294967298 first_receiver=8589934594 flush_returns=1\n"
        signals = "LINUX_SIGNALS_OK delivered=4 returned=4 interrupted=1 pending=0\n"
        objects = "BINDER_OBJECT_CLIENT_OK export=2 handles=2 roundtrip=1 malformed=3 deaths=2 refs=1\nBINDER_OBJECT_SERVER_OK export=2 callback=1 nested=2 per_node_oneway=1 refs=1\n"
        fds = "BINDER_FD_CLIENT_OK shared=1 offset=1 cloexec=1 truncate=1 regrow=1 seals=1 full=1 peer_exit=1 poll=1\nBINDER_FD_SERVER_OK accept=1 capture=1 full=1 copy_fault=1 cross_process_maps=1 poll=1\n"
        poll = "LINUX_POLL_OK external=5 external_delivered=2 epolls=4 eventfds=1 closed=10 live=0 watches=0\n"
        log = client + server + objects + binder + fds + poll + signals + PROCESS.replace("created=2 reaped=2", "created=3 reaped=3") + PROOF
        self.assertEqual(completion_status(log, "binder"), "binder-passed")
        for old, new, expected in (("connections=0", "connections=1", "invalid-binder-cleanup"),
                                   ("replied=14", "replied=0", "missing-binder-transport-proof"),
                                   ("first_receiver=8589934594", "first_receiver=4294967298", "invalid-binder-process-identity"),
                                   ("returned=4", "returned=2", "invalid-binder-signal-proof"),
                                   ("submitted=25", "submitted=oops", "invalid-binder-record")):
            self.assertEqual(completion_status(log.replace(old,new), "binder"), expected)
        for old, new in (("nodes=0", "nodes=1"), ("references=0", "references=1")):
            self.assertEqual(completion_status(log.replace(old, new), "binder"), "invalid-binder-cleanup")
        for key in ("objects=8", "reference_events=14", "fds_delivered=5", "fd_install_failures=2"):
            self.assertEqual(completion_status(log.replace(key, key.split("=")[0] + "=0"), "binder"), "missing-binder-transport-proof")
        self.assertEqual(completion_status(log.replace(objects, ""), "binder"), "missing-binder-object-proof")
        self.assertEqual(completion_status(log+objects, "binder"), "missing-binder-object-proof")
        self.assertEqual(completion_status(log.replace(fds,""), "binder"), "missing-binder-fd-proof")
        self.assertEqual(completion_status(log+fds, "binder"), "missing-binder-fd-proof")
        self.assertEqual(completion_status(log.replace("pending_fds=0","pending_fds=1"), "binder"), "invalid-binder-cleanup")
        self.assertEqual(completion_status(log.replace("external_delivered=2","external_delivered=0"), "binder"), "missing-binder-poll-proof")
        self.assertEqual(completion_status(log.replace("closed=10","closed=9"), "binder"), "invalid-binder-poll-cleanup")
        self.assertEqual(completion_status(log+binder, "binder"), "invalid-binder-record")
        self.assertEqual(completion_status(log.replace(server,""), "binder"), "missing-binder-process-proof")

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
                           for message in ("FRAMEWORK_STAGE parcel", "FRAMEWORK_STAGE wake", "FRAMEWORK_STAGE looper",
                                           "FRAMEWORK_MAIN_OK parcel=1 bundle=1 looper=1 delayed=1 wake=1")) + f"ANDROID_LOG pid={pid} buffer=0 priority=3 tag=AndroidRuntime message=Shutting down VM \n"
        poll = "LINUX_POLL_OK eventfds=2 epolls=2 closed=4 reads=2 writes=2 delivered=2 blocked=2 ready=2 live=0 watches=0\n"
        binder = "LINUX_BINDER_OK opened=4 closed=4 connections=0 transactions=0 buffers=0 nodes=0 references=0 call_frames=0 pending_fds=0 flush_returns=2\n"
        log = messages(4294967298) + messages(8589934594) + PROCESS + PROOF + poll + binder
        self.assertEqual(completion_status(log, "framework"), "framework-passed")
        self.assertEqual(completion_status(log.replace("FRAMEWORK_STAGE looper", "wrong-stage"), "framework"), "missing-framework-stage")
        self.assertEqual(completion_status(log.replace("FRAMEWORK_STAGE wake", "wrong-stage"), "framework"), "missing-framework-stage")
        self.assertEqual(completion_status(log.replace("blocked=2", "blocked=0"), "framework"), "missing-framework-wake-proof")
        self.assertEqual(completion_status(log.replace("flush_returns=2", "flush_returns=0"), "framework"), "missing-framework-shutdown-proof")
        self.assertEqual(completion_status(log.replace("closed=4", "closed=3"), "framework"), "missing-framework-shutdown-proof")
        self.assertEqual(completion_status(log.replace("Shutting down VM", "unproven"), "framework"), "missing-framework-shutdown-proof")
        self.assertEqual(completion_status(log + "Warning: VM did not shut down cleanly\n", "framework"), "missing-framework-shutdown-proof")
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
