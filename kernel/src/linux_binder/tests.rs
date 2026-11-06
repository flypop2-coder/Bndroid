use super::*;
use alloc::vec;

#[test]
fn descriptor_flush_returns_idle_reads_without_releasing_connection_or_work() {
    let (mut r, s, c) = setup();
    assert!(r.read(s, 2, 256).unwrap().is_none());
    r.waiting(s, 2, true).unwrap();
    r.flush(s);
    let noop = receive(&mut r, s, 2);
    assert_eq!(noop.length, 4);
    assert_eq!(u32_at(&noop.bytes, 0), BR_NOOP);
    assert!(r.read(s, 2, 256).unwrap().is_none());
    r.transact(c, 3, data(b"work", 0), b"work".to_vec())
        .unwrap();
    r.flush(s);
    r.flush(c);
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_COMPLETE);
    let request = receive(&mut r, s, 2);
    assert_eq!(request.payload.as_deref(), Some(b"work".as_slice()));
    r.free_buffer(s, transaction(&request).data).unwrap();
    r.reply(s, 2, data(b"reply", 0), b"reply".to_vec()).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_REPLY);
    assert_eq!(r.stats().flush_returns, 1);
    r.close(c);
    r.close(s);
    assert_eq!(r.stats().connections, 0);
}

#[test]
fn device_domains_route_handle_zero_and_pin_manager_uids_independently() {
    let (mut r, service, client) = setup();
    let hardware = r
        .open_in(
            Identity {
                process: 0x100000004,
                uid: 2000,
            },
            Domain::Hardware,
        )
        .unwrap();
    let hw_client = r
        .open_in(
            Identity {
                process: 0x100000005,
                uid: 10000,
            },
            Domain::Hardware,
        )
        .unwrap();
    let vendor = r
        .open_in(
            Identity {
                process: 0x100000006,
                uid: 3000,
            },
            Domain::Vendor,
        )
        .unwrap();
    r.map(
        hardware,
        Mapping {
            base: 0xa00000000,
            length: 4096,
            inode: 3,
        },
    )
    .unwrap();
    r.set_manager(hardware, 1, 2).unwrap();
    r.set_manager(vendor, 3, 4).unwrap();
    r.transact(hw_client, 5, data(b"hardware", 0), b"hardware".to_vec())
        .unwrap();
    receive(&mut r, hw_client, 5);
    assert!(r.read(service, 2, 256).unwrap().is_none());
    let request = receive(&mut r, hardware, 4);
    assert_eq!(request.payload.as_deref(), Some(b"hardware".as_slice()));
    r.close(hardware);
    assert_eq!(opcode(&receive(&mut r, hw_client, 5)), BR_DEAD_REPLY);
    assert_eq!(r.set_manager(hw_client, 0, 0), Err(1));
    r.transact(client, 3, data(b"framework", 0), b"framework".to_vec())
        .unwrap();
    assert_eq!(
        receive(&mut r, service, 2).payload.as_deref(),
        Some(b"framework".as_slice())
    );
    for id in [service, client, hw_client, vendor] {
        r.close(id);
    }
    assert_eq!(
        (
            r.stats().connections,
            r.stats().transactions,
            r.stats().buffers
        ),
        (0, 0, 0)
    );
}

fn data(bytes: &[u8], flags: u32) -> Transaction {
    Transaction {
        target: 0,
        cookie: 999,
        code: 42,
        flags,
        sender_pid: 777,
        sender_uid: 0,
        data_size: bytes.len() as u64,
        offsets_size: 0,
        data: 1234,
        offsets: 5678,
    }
}
fn setup() -> (Router, u64, u64) {
    let mut r = Router::new();
    let service = r
        .open(Identity {
            process: 0x100000002,
            uid: 1000,
        })
        .unwrap();
    let client = r
        .open(Identity {
            process: 0x100000003,
            uid: 10000,
        })
        .unwrap();
    r.map(
        service,
        Mapping {
            base: 0x800000000,
            length: 4096,
            inode: 1,
        },
    )
    .unwrap();
    r.map(
        client,
        Mapping {
            base: 0x900000000,
            length: 4096,
            inode: 2,
        },
    )
    .unwrap();
    r.set_manager(service, 0x12340, 0x56780).unwrap();
    (r, service, client)
}
fn receive(r: &mut Router, id: u64, tid: u32) -> Delivery {
    let d = r.read(id, tid, 256).unwrap().unwrap();
    r.finish_delivery(id, &d).unwrap();
    d
}
fn opcode(d: &Delivery) -> u32 {
    u32_at(&d.bytes, 4)
}
fn transaction(d: &Delivery) -> Transaction {
    Transaction::decode(d.bytes[8..72].try_into().unwrap())
}

#[test]
fn request_reply_copies_payload_and_authenticates_identity_and_thread() {
    let (mut r, s, c) = setup();
    r.transact(c, 3, data(b"hello", 0), b"hello".to_vec())
        .unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_COMPLETE);
    assert!(r.read(c, 3, 256).unwrap().is_none());
    let request = receive(&mut r, s, 2);
    let tx = transaction(&request);
    assert_eq!(
        (tx.target, tx.cookie, tx.sender_pid, tx.sender_uid),
        (0x12340, 0x56780, 3, 10000)
    );
    assert_eq!(request.payload.as_deref(), Some(b"hello".as_slice()));
    assert_eq!(r.reply(s, 900, data(b"x", 0), vec![b'x']), Err(22));
    r.free_buffer(s, tx.data).unwrap();
    r.reply(s, 2, data(b"world", 0), b"world".to_vec()).unwrap();
    let reply = receive(&mut r, c, 3);
    assert_eq!(opcode(&reply), BR_REPLY);
    assert_eq!(
        (
            transaction(&reply).sender_pid,
            transaction(&reply).sender_uid
        ),
        (0, 1000)
    );
    assert_eq!(reply.payload.as_deref(), Some(b"world".as_slice()));
    r.free_buffer(c, transaction(&reply).data).unwrap();
    assert_eq!(opcode(&receive(&mut r, s, 2)), BR_COMPLETE);
    r.close(s);
    r.close(c);
    let stats = r.stats();
    assert_eq!(
        (stats.opened, stats.closed, stats.submitted, stats.replied),
        (2, 2, 1, 1)
    );
    assert_eq!(
        (stats.connections, stats.transactions, stats.buffers),
        (0, 0, 0)
    );
}
#[test]
fn prepared_reads_are_not_freeable_and_faulted_delivery_releases_ownership() {
    let (mut r, s, c) = setup();
    r.transact(c, 3, data(b"hello", 0), b"hello".to_vec())
        .unwrap();
    receive(&mut r, c, 3);
    assert_eq!(r.read(s, 2, 71).err(), Some(22));
    let d = r.read(s, 2, 72).unwrap().unwrap();
    assert_eq!(r.free_buffer(s, transaction(&d).data), Err(22));
    r.discard_delivery(s, &d);
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_FAILED_REPLY);
    assert_eq!((r.stats().transactions, r.stats().buffers), (0, 0));
}
#[test]
fn oneway_is_ordered_until_receiver_frees_each_buffer() {
    let (mut r, s, c) = setup();
    for byte in [1, 2] {
        r.transact(c, 3, data(&[byte], ONE_WAY), vec![byte])
            .unwrap();
        receive(&mut r, c, 3);
    }
    let first = receive(&mut r, s, 2);
    assert_eq!(transaction(&first).sender_pid, 0);
    assert_eq!(first.payload.as_deref(), Some([1].as_slice()));
    assert!(r.read(s, 4, 256).unwrap().is_none());
    r.free_buffer(s, transaction(&first).data).unwrap();
    let second = receive(&mut r, s, 4);
    assert_eq!(second.payload.as_deref(), Some([2].as_slice()));
    r.free_buffer(s, transaction(&second).data).unwrap();
    assert_eq!(r.stats().buffers, 0);
}
#[test]
fn closing_service_wakes_callers_and_death_cookie_survives_replacement() {
    let (mut r, s, c) = setup();
    r.reference(c, 0, BC_INCREFS).unwrap();
    r.reference(c, 0, BC_ACQUIRE).unwrap();
    r.request_death(c, 0, 0xaabb).unwrap();
    r.transact(c, 3, data(b"hello", 0), b"hello".to_vec())
        .unwrap();
    receive(&mut r, c, 3);
    let request = receive(&mut r, s, 2);
    r.free_buffer(s, transaction(&request).data).unwrap();
    r.close(s);
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_DEAD_REPLY);
    let death = receive(&mut r, c, 3);
    assert_eq!(
        (opcode(&death), u64_at(&death.bytes, 8)),
        (BR_DEAD_BINDER, 0xaabb)
    );
    assert_eq!(r.dead_done(c, 9), Err(22));
    r.clear_death(c, 0, 0xaabb).unwrap();
    assert!(r.read(c, 3, 256).unwrap().is_none());
    r.dead_done(c, 0xaabb).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_CLEAR_DEATH_DONE);
    let replacement = r
        .open(Identity {
            process: 0x200000002,
            uid: 1000,
        })
        .unwrap();
    r.map(
        replacement,
        Mapping {
            base: 0x800000000,
            length: 4096,
            inode: 3,
        },
    )
    .unwrap();
    r.set_manager(replacement, 1, 2).unwrap();
    r.transact(c, 3, data(b"stale", 0), b"stale".to_vec())
        .unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_DEAD_REPLY);
    assert!(r.read(replacement, 2, 256).unwrap().is_none());
    r.close(c);
    r.close(replacement);
    assert_eq!(
        (
            r.stats().connections,
            r.stats().transactions,
            r.stats().buffers
        ),
        (0, 0, 0)
    );
}
#[test]
fn freed_sync_buffer_cannot_delete_a_later_transaction_at_the_same_address() {
    let (mut r, s, c) = setup();
    r.transact(c, 3, data(b"old", 0), b"old".to_vec()).unwrap();
    receive(&mut r, c, 3);
    let first = receive(&mut r, s, 2);
    r.free_buffer(s, transaction(&first).data).unwrap();
    r.transact(c, 4, data(b"new", 0), b"new".to_vec()).unwrap();
    receive(&mut r, c, 4);
    r.thread_exit(c, 3);
    let second = receive(&mut r, s, 5);
    assert_eq!(transaction(&first).data, transaction(&second).data);
    assert_eq!(second.payload.as_deref(), Some(b"new".as_slice()));
    r.reply(s, 5, data(b"yes", 0), b"yes".to_vec()).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 4)), BR_REPLY);
}
#[test]
fn invalid_credentials_unmap_exhaustion_and_unsupported_objects_fail_closed() {
    let (mut r, s, c) = setup();
    assert_eq!(r.authenticate(s, 0x200000002), Err(1));
    assert_eq!(r.set_manager(c, 0, 0), Err(16));
    let mut bad = data(b"x", 0);
    bad.offsets_size = 8;
    assert_eq!(r.transact(c, 3, bad, vec![1]), Err(95));
    r.transact(c, 3, data(&[1; 4096], 0), vec![1; 4096])
        .unwrap();
    receive(&mut r, c, 3);
    r.transact(c, 4, data(b"x", 0), vec![1]).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 4)), BR_FAILED_REPLY);
    r.unmap_inode(1);
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_FAILED_REPLY);
    assert!(r.read(s, 2, 256).unwrap().is_none());
    assert_eq!(r.can_map(s), Err(16));
    assert_eq!((r.stats().transactions, r.stats().buffers), (0, 0));
    r.close(s);
    assert_eq!(r.set_manager(c, 0, 0), Err(1));
}
#[test]
fn wire_consumed_counts_and_pointer_overflows_are_rejected() {
    let mut b = [0; 48];
    put64(&mut b, 0, 4);
    put64(&mut b, 8, 5);
    assert_eq!(WriteRead::decode(&b).err(), Some(22));
    put64(&mut b, 8, 0);
    put64(&mut b, 16, u64::MAX - 1);
    assert_eq!(WriteRead::decode(&b).err(), Some(22));
    assert_eq!(command_bytes(BC_REQUEST_DEATH), Ok(12));
    assert_eq!(command_bytes(0x4000_6300), Err(22));
}

#[test]
fn accepted_oneway_survives_sender_exit_and_bad_reply_unblocks_caller() {
    let (mut r, s, c) = setup();
    r.transact(c, 3, data(b"async", ONE_WAY), b"async".to_vec())
        .unwrap();
    r.thread_exit(c, 3);
    r.close(c);
    let message = receive(&mut r, s, 2);
    assert_eq!(message.payload.as_deref(), Some(b"async".as_slice()));
    r.free_buffer(s, transaction(&message).data).unwrap();
    let client = r
        .open(Identity {
            process: 0x200000003,
            uid: 10000,
        })
        .unwrap();
    r.map(
        client,
        Mapping {
            base: 0x900000000,
            length: 4096,
            inode: 9,
        },
    )
    .unwrap();
    r.transact(client, 3, data(b"sync", 0), b"sync".to_vec())
        .unwrap();
    receive(&mut r, client, 3);
    let request = receive(&mut r, s, 2);
    r.fail_reply(s, 2).unwrap();
    assert_eq!(opcode(&receive(&mut r, client, 3)), BR_FAILED_REPLY);
    r.free_buffer(s, transaction(&request).data).unwrap();
    assert_eq!(r.stats().transactions, 0);
    r.close(s);
    r.close(client);
    assert_eq!((r.stats().connections, r.stats().buffers), (0, 0));
}

#[test]
fn thread_pool_requests_obey_limits_and_require_a_registered_response() {
    let (mut r, s, c) = setup();
    r.set_max_threads(s, 1).unwrap();
    r.looper(s, 2, BC_ENTER_LOOPER).unwrap();
    assert_eq!(r.looper(s, 4, BC_REGISTER_LOOPER), Err(22));
    for tid in [3, 4, 5] {
        r.transact(c, tid, data(b"work", 0), b"work".to_vec())
            .unwrap();
    }
    let first = receive(&mut r, s, 2);
    assert_eq!(first.length, 76);
    assert_eq!(u32_at(&first.bytes, 72), BR_SPAWN_LOOPER);
    r.looper(s, 4, BC_REGISTER_LOOPER).unwrap();
    let second = receive(&mut r, s, 4);
    assert_eq!(second.length, 72);
    assert_eq!(r.looper(s, 5, BC_REGISTER_LOOPER), Err(22));
    r.close(s);
    r.close(c);
    assert_eq!(
        (
            r.stats().connections,
            r.stats().transactions,
            r.stats().buffers
        ),
        (0, 0, 0)
    );
}

#[test]
fn reply_to_unmapped_receiver_reports_failure_and_completes_replier() {
    let (mut r, s, c) = setup();
    r.transact(c, 3, data(b"hello", 0), b"hello".to_vec())
        .unwrap();
    receive(&mut r, c, 3);
    let request = receive(&mut r, s, 2);
    r.free_buffer(s, transaction(&request).data).unwrap();
    r.unmap_inode(2);
    r.reply(s, 2, data(b"reply", 0), b"reply".to_vec()).unwrap();
    assert_eq!(opcode(&receive(&mut r, s, 2)), BR_COMPLETE);
    assert_eq!(opcode(&receive(&mut r, c, 3)), BR_FAILED_REPLY);
    assert_eq!((r.stats().transactions, r.stats().buffers), (0, 0));
}
