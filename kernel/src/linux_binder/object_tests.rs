use super::*;
use alloc::vec;

fn endpoint(r: &mut Router, pid: u64) -> u64 {
    let id = r
        .open(Identity {
            process: pid,
            uid: pid as u32 + 1000,
        })
        .unwrap();
    r.map(
        id,
        Mapping {
            base: pid * 0x1000000,
            length: 16384,
            inode: pid,
        },
    )
    .unwrap();
    id
}
fn packet(target: u32, objects: &[(u32, u64, u64)]) -> (Transaction, Vec<u8>) {
    let size = objects.len() * 24;
    let mut bytes = vec![0; size + objects.len() * 8];
    for (i, &(kind, pointer, cookie)) in objects.iter().enumerate() {
        put32(&mut bytes, i * 24, kind);
        put32(&mut bytes, i * 24 + 4, 0x100);
        put64(&mut bytes, i * 24 + 8, pointer);
        put64(&mut bytes, i * 24 + 16, cookie);
        put64(&mut bytes, size + i * 8, (i * 24) as u64);
    }
    (
        Transaction {
            target: target as u64,
            cookie: 0,
            code: 1,
            flags: 0,
            sender_pid: 99,
            sender_uid: 0,
            data_size: size as u64,
            offsets_size: (objects.len() * 8) as u64,
            data: 0,
            offsets: 0,
        },
        bytes,
    )
}
fn read(r: &mut Router, id: u64, tid: u32) -> Delivery {
    let d = r.read(id, tid, 256).unwrap().unwrap();
    r.finish_delivery(id, &d).unwrap();
    d
}
fn code(d: &Delivery) -> u32 {
    u32_at(&d.bytes, 4)
}
fn tx(d: &Delivery) -> Transaction {
    Transaction::decode(d.bytes[8..72].try_into().unwrap())
}
fn free(r: &mut Router, id: u64, d: &Delivery) {
    r.free_buffer(id, tx(d).data).unwrap();
}
fn send(r: &mut Router, from: u64, tid: u32, packet: (Transaction, Vec<u8>)) {
    r.transact(from, tid, packet.0, packet.1).unwrap();
    complete(r, from, tid);
}
fn complete(r: &mut Router, owner: u64, tid: u32) {
    loop {
        let d = read(r, owner, tid);
        if code(&d) == BR_COMPLETE {
            return;
        }
        let command = match code(&d) {
            BR_INCREFS => BC_INCREFS_DONE,
            BR_ACQUIRE => BC_ACQUIRE_DONE,
            _ => panic!("unexpected completion event {}", code(&d)),
        };
        let pointer = u64_at(&d.bytes, 8);
        let cookie = u64_at(&d.bytes, 16);
        assert_eq!(r.acknowledge(owner, pointer, cookie + 1, command), Err(22));
        r.acknowledge(owner, pointer, cookie, command).unwrap();
        assert_eq!(r.acknowledge(owner, pointer, cookie, command), Err(22));
    }
}
fn releases(r: &mut Router, owner: u64, tid: u32) {
    while let Some(d) = r.read(owner, tid, 256).unwrap() {
        assert!(matches!(code(&d), BR_RELEASE | BR_DECREFS));
        r.finish_delivery(owner, &d).unwrap();
    }
}
fn reply(r: &mut Router, owner: u64, tid: u32, packet: (Transaction, Vec<u8>)) {
    r.reply(owner, tid, packet.0, packet.1).unwrap();
    complete(r, owner, tid);
}
fn retained_handle(r: &mut Router, id: u64, d: &Delivery, index: usize, strong: bool) -> u32 {
    let bytes = d.payload.as_ref().unwrap();
    assert_eq!(
        u32_at(bytes, index * 24),
        if strong {
            TYPE_HANDLE
        } else {
            TYPE_WEAK_HANDLE
        }
    );
    assert_eq!(u64_at(bytes, index * 24 + 16), 0);
    let handle = u32_at(bytes, index * 24 + 8);
    assert_ne!(handle, 0);
    r.reference(id, handle, BC_INCREFS).unwrap();
    if strong {
        r.reference(id, handle, BC_ACQUIRE).unwrap();
    }
    handle
}
fn closed(r: &mut Router, ids: &[u64]) {
    for &id in ids {
        r.close(id);
    }
    let s = r.stats();
    assert_eq!(
        (
            s.connections,
            s.transactions,
            s.buffers,
            s.nodes,
            s.references
        ),
        (0, 0, 0, 0, 0)
    );
}

#[test]
fn exported_handle_calls_return_to_original_pointer_and_can_be_passed_back() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    send(&mut r, c, 2, packet(0, &[]));
    let request = read(&mut r, s, 1);
    free(&mut r, s, &request);
    reply(&mut r, s, 1, packet(0, &[(TYPE_BINDER, 200, 201)]));
    let response = read(&mut r, c, 2);
    let handle = retained_handle(&mut r, c, &response, 0, true);
    assert_eq!(tx(&response).offsets, tx(&response).data + 24);
    free(&mut r, c, &response);
    send(
        &mut r,
        c,
        2,
        packet(handle, &[(TYPE_HANDLE, handle as u64, 999)]),
    );
    let request = read(&mut r, s, 1);
    assert_eq!((tx(&request).target, tx(&request).cookie), (200, 201));
    let b = request.payload.as_ref().unwrap();
    assert_eq!(
        (u32_at(b, 0), u64_at(b, 8), u64_at(b, 16)),
        (TYPE_BINDER, 200, 201)
    );
    free(&mut r, s, &request);
    reply(&mut r, s, 1, packet(0, &[]));
    let response = read(&mut r, c, 2);
    free(&mut r, c, &response);
    r.reference(c, handle, BC_RELEASE).unwrap();
    assert_eq!(code(&read(&mut r, s, 1)), BR_RELEASE);
    assert_eq!(r.reference(c, handle, BC_ACQUIRE), Err(22));
    r.reference(c, handle, BC_DECREFS).unwrap();
    assert_eq!(code(&read(&mut r, s, 1)), BR_DECREFS);
    assert_eq!((r.stats().nodes, r.stats().references), (1, 0));
    closed(&mut r, &[s, c]);
}

#[test]
fn handle_union_padding_is_not_part_of_the_u32_authority() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    let (mut t, b) = packet(0, &[]);
    t.target = 0xaabbccdd00000000;
    send(&mut r, c, 2, (t, b));
    let request = read(&mut r, s, 1);
    assert_eq!(tx(&request).target, 100);
    closed(&mut r, &[s, c]);
}

#[test]
fn prepared_retain_events_block_completion_until_userspace_has_the_object() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    let (mut t, b) = packet(0, &[(TYPE_BINDER, 200, 201)]);
    t.flags = ONE_WAY;
    r.transact(c, 2, t, b).unwrap();
    let weak = r.read(c, 3, 256).unwrap().unwrap();
    assert_eq!(code(&weak), BR_INCREFS);
    assert!(r.read(c, 2, 256).unwrap().is_none());
    r.finish_delivery(c, &weak).unwrap();
    let strong = r.read(c, 3, 256).unwrap().unwrap();
    assert_eq!(code(&strong), BR_ACQUIRE);
    assert!(r.read(c, 2, 256).unwrap().is_none());
    r.discard_delivery(c, &strong);
    let strong = read(&mut r, c, 2);
    assert_eq!(code(&strong), BR_ACQUIRE);
    assert!(r.read(c, 2, 256).unwrap().is_none());
    r.acknowledge(c, 200, 201, BC_INCREFS_DONE).unwrap();
    assert!(r.read(c, 2, 256).unwrap().is_none());
    r.acknowledge(c, 200, 201, BC_ACQUIRE_DONE).unwrap();
    assert_eq!(code(&read(&mut r, c, 2)), BR_COMPLETE);
    closed(&mut r, &[s, c]);
}

#[test]
fn nested_callbacks_return_to_waiting_threads_and_unwind_in_reverse_order() {
    for fail in [false, true] {
        let mut r = Router::new();
        let s = endpoint(&mut r, 1);
        let c = endpoint(&mut r, 2);
        r.set_manager(s, 100, 101).unwrap();
        send(&mut r, c, 2, packet(0, &[(TYPE_BINDER, 200, 201)]));
        let outer = read(&mut r, s, 1);
        let h = retained_handle(&mut r, s, &outer, 0, true);
        free(&mut r, s, &outer);
        send(&mut r, s, 1, packet(h, &[]));
        assert!(r.read(c, 7, 256).unwrap().is_none());
        let callback = read(&mut r, c, 2);
        assert_eq!(tx(&callback).target, 200);
        free(&mut r, c, &callback);
        send(&mut r, c, 2, packet(0, &[]));
        assert!(r.read(s, 7, 256).unwrap().is_none());
        let reentry = read(&mut r, s, 1);
        free(&mut r, s, &reentry);
        assert_eq!(
            (
                r.stats().nested,
                r.stats().max_call_depth,
                r.stats().call_frames
            ),
            (2, 3, 6)
        );
        assert_eq!(r.reply(s, 7, packet(0, &[]).0, vec![]), Err(22));
        if fail {
            r.close(s);
            assert_eq!(code(&read(&mut r, c, 2)), BR_DEAD_REPLY);
            assert_eq!((r.stats().transactions, r.stats().call_frames), (0, 0));
        } else {
            reply(&mut r, s, 1, packet(0, &[]));
            let d = read(&mut r, c, 2);
            assert_eq!(code(&d), BR_REPLY);
            free(&mut r, c, &d);
            reply(&mut r, c, 2, packet(0, &[]));
            let d = read(&mut r, s, 1);
            assert_eq!(code(&d), BR_REPLY);
            free(&mut r, s, &d);
            reply(&mut r, s, 1, packet(0, &[]));
            let d = read(&mut r, c, 2);
            assert_eq!(code(&d), BR_REPLY);
            free(&mut r, c, &d);
            assert_eq!((r.stats().transactions, r.stats().call_frames), (0, 0));
        }
        closed(&mut r, &[s, c]);
    }
}

#[test]
fn handle_table_exhaustion_and_nested_depth_failure_preserve_existing_work() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    let objects: Vec<_> = (0..256)
        .map(|i| (TYPE_BINDER, 200 + i * 8, 201 + i * 8))
        .collect();
    send(&mut r, c, 2, packet(0, &objects));
    let extra = packet(0, &[(TYPE_BINDER, 9999, 10000)]);
    r.transact(c, 3, extra.0, extra.1).unwrap();
    assert_eq!(code(&read(&mut r, c, 3)), BR_FAILED_REPLY);
    assert_eq!(
        (r.stats().nodes, r.stats().references, r.stats().buffers),
        (257, 256, 1)
    );
    r.unmap_inode(1);
    assert_eq!(code(&read(&mut r, c, 2)), BR_FAILED_REPLY);
    releases(&mut r, c, 2);
    assert_eq!(
        (r.stats().nodes, r.stats().references, r.stats().buffers),
        (1, 0, 0)
    );
    closed(&mut r, &[s, c]);

    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    send(&mut r, c, 2, packet(0, &[(TYPE_BINDER, 200, 201)]));
    let d = read(&mut r, s, 1);
    let h = retained_handle(&mut r, s, &d, 0, true);
    free(&mut r, s, &d);
    for depth in 2..=MAX_CALL_DEPTH {
        let (from, from_tid, to, to_tid, target) = if depth % 2 == 0 {
            (s, 1, c, 2, h)
        } else {
            (c, 2, s, 1, 0)
        };
        send(&mut r, from, from_tid, packet(target, &[]));
        let d = read(&mut r, to, to_tid);
        free(&mut r, to, &d);
    }
    let (t, b) = packet(0, &[]);
    assert_eq!(r.transact(c, 2, t, b), Err(11));
    r.thread_exit(s, 1);
    assert_eq!((r.stats().transactions, r.stats().call_frames), (0, 0));
    closed(&mut r, &[s, c]);
}

#[test]
fn handle_forwarding_is_scoped_and_weak_handles_cannot_invoke_or_export_strong() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let a = endpoint(&mut r, 2);
    let b = endpoint(&mut r, 3);
    r.set_manager(s, 100, 101).unwrap();
    send(
        &mut r,
        a,
        2,
        packet(0, &[(TYPE_BINDER, 200, 201), (TYPE_WEAK_BINDER, 300, 301)]),
    );
    let request = read(&mut r, s, 1);
    let h = retained_handle(&mut r, s, &request, 0, true);
    let w = retained_handle(&mut r, s, &request, 1, false);
    free(&mut r, s, &request);
    reply(&mut r, s, 1, packet(0, &[]));
    let response = read(&mut r, a, 2);
    free(&mut r, a, &response);
    r.transact(b, 3, packet(h, &[]).0, vec![]).unwrap();
    assert_eq!(code(&read(&mut r, b, 3)), BR_FAILED_REPLY);
    r.transact(s, 1, packet(w, &[]).0, vec![]).unwrap();
    assert_eq!(code(&read(&mut r, s, 1)), BR_FAILED_REPLY);
    send(&mut r, b, 3, packet(0, &[]));
    let request = read(&mut r, s, 1);
    free(&mut r, s, &request);
    reply(
        &mut r,
        s,
        1,
        packet(
            0,
            &[(TYPE_HANDLE, h as u64, 0), (TYPE_WEAK_HANDLE, w as u64, 0)],
        ),
    );
    let response = read(&mut r, b, 3);
    let bh = retained_handle(&mut r, b, &response, 0, true);
    let bw = retained_handle(&mut r, b, &response, 1, false);
    assert_ne!(h, bh);
    assert_ne!(w, bw);
    free(&mut r, b, &response);
    send(&mut r, b, 3, packet(bh, &[]));
    let request = read(&mut r, a, 2);
    assert_eq!(tx(&request).target, 200);
    free(&mut r, a, &request);
    reply(&mut r, a, 2, packet(0, &[]));
    let response = read(&mut r, b, 3);
    free(&mut r, b, &response);
    // A fabricated strong object referring only to a weak reference fails and
    // cannot publish any new handles in the receiver.
    let bad = packet(0, &[(TYPE_HANDLE, bw as u64, 0)]);
    r.transact(b, 3, bad.0, bad.1).unwrap();
    assert_eq!(code(&read(&mut r, b, 3)), BR_FAILED_REPLY);
    closed(&mut r, &[a, b, s]);
}

#[test]
fn invalid_offsets_partial_translation_and_fd_objects_rollback_all_ownership() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    for bad_offset in [0, 4, 25, 48, u64::MAX] {
        let (t, mut b) = packet(0, &[(TYPE_BINDER, 200, 201), (TYPE_BINDER, 300, 301)]);
        put64(&mut b, 56, bad_offset);
        r.transact(c, 2, t, b).unwrap();
        assert_eq!(code(&read(&mut r, c, 2)), BR_FAILED_REPLY);
        assert_eq!(
            (r.stats().nodes, r.stats().references, r.stats().buffers),
            (1, 0, 0)
        );
    }
    for kind in [0x6664_2a85, 0x7074_2a85, TYPE_HANDLE] {
        let (t, b) = packet(0, &[(TYPE_BINDER, 200, 201), (kind, 999, 301)]);
        r.transact(c, 2, t, b).unwrap();
        assert_eq!(code(&read(&mut r, c, 2)), BR_FAILED_REPLY);
        assert_eq!(
            (r.stats().nodes, r.stats().references, r.stats().buffers),
            (1, 0, 0)
        );
    }
    closed(&mut r, &[s, c]);
}

#[test]
fn reference_and_death_notifications_are_not_lost_on_failed_user_copies() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    let (t, b) = packet(0, &[(TYPE_BINDER, 200, 201)]);
    r.transact(c, 2, t, b).unwrap();
    let request = read(&mut r, s, 1);
    let h = retained_handle(&mut r, s, &request, 0, true);
    free(&mut r, s, &request);
    let failed = r.read(c, 2, 256).unwrap().unwrap();
    assert_eq!(code(&failed), BR_INCREFS);
    r.discard_delivery(c, &failed);
    complete(&mut r, c, 2);
    r.request_death(s, h, 0xabc).unwrap();
    r.close(c);
    let failed = r.read(s, 7, 256).unwrap().unwrap();
    assert_eq!(code(&failed), BR_DEAD_BINDER);
    r.discard_delivery(s, &failed);
    assert_eq!(r.stats().deaths, 0);
    let death = read(&mut r, s, 7);
    assert_eq!(
        (code(&death), u64_at(&death.bytes, 8)),
        (BR_DEAD_BINDER, 0xabc)
    );
    r.clear_death(s, h, 0xabc).unwrap();
    r.dead_done(s, 0xabc).unwrap();
    let failed = r.read(s, 7, 256).unwrap().unwrap();
    r.discard_delivery(s, &failed);
    assert_eq!(code(&read(&mut r, s, 7)), BR_CLEAR_DEATH_DONE);
    r.transact(s, 7, packet(h, &[]).0, vec![]).unwrap();
    assert_eq!(code(&read(&mut r, s, 7)), BR_DEAD_REPLY);
    closed(&mut r, &[s]);
}

#[test]
fn oneway_serialization_is_per_object_and_buffers_pin_exported_objects() {
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    send(&mut r, c, 2, packet(0, &[]));
    let request = read(&mut r, s, 1);
    free(&mut r, s, &request);
    reply(
        &mut r,
        s,
        1,
        packet(0, &[(TYPE_BINDER, 200, 201), (TYPE_BINDER, 300, 301)]),
    );
    let response = read(&mut r, c, 2);
    let h = retained_handle(&mut r, c, &response, 0, true);
    let j = retained_handle(&mut r, c, &response, 1, true);
    free(&mut r, c, &response);
    for handle in [h, h, j] {
        let (mut t, b) = packet(handle, &[]);
        t.flags = ONE_WAY;
        send(&mut r, c, 2, (t, b));
    }
    let first = read(&mut r, s, 1);
    assert_eq!(tx(&first).target, 200);
    let other = read(&mut r, s, 4);
    assert_eq!(tx(&other).target, 300);
    assert!(r.read(s, 5, 256).unwrap().is_none());
    r.close(c); // Accepted buffers keep both local objects pinned after sender exit.
    assert!(r.read(s, 5, 256).unwrap().is_none());
    free(&mut r, s, &first);
    let second = read(&mut r, s, 1);
    assert_eq!(tx(&second).target, 200);
    free(&mut r, s, &second);
    free(&mut r, s, &other);
    for _ in 0..2 {
        assert_eq!(code(&read(&mut r, s, 1)), BR_RELEASE);
        assert_eq!(code(&read(&mut r, s, 1)), BR_DECREFS);
    }
    closed(&mut r, &[s]);
}

#[test]
fn discarded_requests_replies_and_unmaps_release_translated_buffer_references() {
    for action in 0..3 {
        let mut r = Router::new();
        let s = endpoint(&mut r, 1);
        let c = endpoint(&mut r, 2);
        r.set_manager(s, 100, 101).unwrap();
        send(&mut r, c, 2, packet(0, &[(TYPE_BINDER, 200, 201)]));
        if action == 0 {
            let d = r.read(s, 1, 256).unwrap().unwrap();
            r.discard_delivery(s, &d);
        } else if action == 1 {
            r.unmap_inode(1);
        } else {
            r.thread_exit(c, 2);
        }
        if action != 2 {
            assert_eq!(code(&read(&mut r, c, 2)), BR_FAILED_REPLY);
        }
        releases(&mut r, c, if action == 2 { 3 } else { 2 });
        assert_eq!(
            (r.stats().nodes, r.stats().references, r.stats().buffers),
            (1, 0, 0)
        );
        closed(&mut r, &[s, c]);
    }
    let mut r = Router::new();
    let s = endpoint(&mut r, 1);
    let c = endpoint(&mut r, 2);
    r.set_manager(s, 100, 101).unwrap();
    send(&mut r, c, 2, packet(0, &[]));
    let request = read(&mut r, s, 1);
    free(&mut r, s, &request);
    reply(&mut r, s, 1, packet(0, &[(TYPE_BINDER, 200, 201)]));
    r.thread_exit(c, 2);
    releases(&mut r, s, 1);
    assert_eq!(
        (r.stats().nodes, r.stats().references, r.stats().buffers),
        (1, 0, 0)
    );
    closed(&mut r, &[s, c]);
}
