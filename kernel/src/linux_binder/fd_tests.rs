use super::*;
use crate::linux_fd::{CAPACITY, FileRef, Table, Target};
use alloc::vec;

fn setup(accept: bool) -> (Router, u64, u64) {
    let mut r = Router::new();
    let s = r
        .open(Identity {
            process: 1,
            uid: 1000,
        })
        .unwrap();
    let c = r
        .open(Identity {
            process: 2,
            uid: 10000,
        })
        .unwrap();
    for (id, base, inode) in [(s, 0x80000000, 1), (c, 0x90000000, 2)] {
        r.map(
            id,
            Mapping {
                base,
                length: 4096,
                inode,
            },
        )
        .unwrap();
    }
    r.set_manager_with_flags(s, 0x1234, 0x5678, if accept { 0x100 } else { 0 })
        .unwrap();
    (r, s, c)
}
fn packet(files: &[u32], flags: u32) -> (Transaction, Vec<u8>) {
    let size = files.len() * 24;
    let mut b = vec![0; size + files.len() * 8];
    for (i, &fd) in files.iter().enumerate() {
        put32(&mut b, i * 24, TYPE_FD);
        put32(&mut b, i * 24 + 8, fd);
        put64(&mut b, i * 24 + 16, 0xdeadbeef);
        put64(&mut b, size + i * 8, (i * 24) as u64);
    }
    (
        Transaction {
            target: 0,
            cookie: 0,
            code: 1,
            flags,
            sender_pid: 999,
            sender_uid: 0,
            data_size: size as u64,
            offsets_size: (files.len() * 8) as u64,
            data: 0,
            offsets: 0,
        },
        b,
    )
}
fn receive(r: &mut Router, id: u64, tid: u32) -> Delivery {
    let d = r.read(id, tid, 256).unwrap().unwrap();
    r.finish_delivery(id, &d).unwrap();
    d
}
fn opcode(d: &Delivery) -> u32 {
    u32_at(&d.bytes, 4)
}
fn free(r: &mut Router, id: u64, d: &Delivery) {
    let t = Transaction::decode(d.bytes[8..72].try_into().unwrap());
    r.free_buffer(id, t.data).unwrap();
}
fn close(r: &mut Router, ids: &[u64]) {
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
            s.references,
            s.call_frames,
            s.pending_fds
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
}
fn source() -> (Table, u32, FileRef) {
    let mut t = Table::standard().unwrap();
    let f = FileRef::try_snapshot("proc/self/maps", 99, 0, b"shared file description".to_vec())
        .unwrap();
    let fd = t.insert(f.clone(), 0, false).unwrap() as u32;
    (t, fd, f)
}

#[test]
fn captured_files_survive_close_reuse_and_roundtrip_with_shared_offsets() {
    let (mut r, s, c) = setup(true);
    let (mut from, fd, original) = source();
    let mut to = Table::standard().unwrap();
    to.insert(FileRef::try_new(Target::Null, 0).unwrap(), 0, false)
        .unwrap();
    original.seek(4, 0).unwrap();
    let (tx, b) = packet(&[fd], ACCEPT_FDS);
    let parcel = Parcel::capture(tx, b, &from).unwrap();
    from.close(fd as u64).unwrap();
    let replacement = FileRef::try_new(Target::Null, 0).unwrap();
    assert_eq!(from.insert(replacement, 0, false), Ok(fd as u64));
    r.transact_parcel(c, 2, tx, parcel).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 2)), BR_COMPLETE);
    assert_eq!(r.stats().pending_fds, 1);
    let d = r.read(s, 1, 256).unwrap().unwrap();
    let received = to.receive_files(&d.files).unwrap();
    assert_ne!(received[0], fd);
    assert_eq!(to.fd_flags(received[0] as u64), Ok(1));
    assert_eq!(to.file(received[0] as u64).unwrap().position(), 4);
    assert_eq!(u64_at(d.payload.as_ref().unwrap(), 8), 0);
    assert_eq!(u64_at(d.payload.as_ref().unwrap(), 16), 0xdeadbeef);
    r.finish_delivery(s, &d).unwrap();
    free(&mut r, s, &d);
    to.file(received[0] as u64).unwrap().seek(2, 1).unwrap();
    assert_eq!(original.position(), 6);
    let (tx, b) = packet(&received, 0);
    r.reply_parcel(s, 1, tx, Parcel::capture(tx, b, &to).unwrap())
        .unwrap();
    assert_eq!(opcode(&receive(&mut r, s, 1)), BR_COMPLETE);
    to.close(received[0] as u64).unwrap();
    let d = r.read(c, 2, 256).unwrap().unwrap();
    let back = from.receive_files(&d.files).unwrap();
    r.finish_delivery(c, &d).unwrap();
    free(&mut r, c, &d);
    assert!(
        from.file(back[0] as u64)
            .unwrap()
            .same_description(&original)
    );
    assert_eq!(
        (
            r.stats().fds_submitted,
            r.stats().fds_delivered,
            r.stats().pending_fds
        ),
        (2, 2, 0)
    );
    close(&mut r, &[s, c]);
}

#[test]
fn receiver_consent_and_caller_reply_consent_are_distinct() {
    let (mut r, s, c) = setup(false);
    let (table, fd, _) = source();
    let (tx, b) = packet(&[fd], ACCEPT_FDS);
    assert_eq!(
        r.transact_parcel(c, 2, tx, Parcel::capture(tx, b, &table).unwrap()),
        Err(1)
    );
    assert_eq!((r.stats().transactions, r.stats().pending_fds), (0, 0));
    close(&mut r, &[s, c]);
    let (mut r, s, c) = setup(true);
    let (tx, b) = packet(&[], 0);
    r.transact(c, 2, tx, b).unwrap();
    receive(&mut r, c, 2);
    let d = receive(&mut r, s, 1);
    free(&mut r, s, &d);
    let (tx, b) = packet(&[fd], 0);
    assert_eq!(
        r.reply_parcel(s, 1, tx, Parcel::capture(tx, b, &table).unwrap()),
        Err(1)
    );
    r.fail_reply(s, 1).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 2)), BR_FAILED_REPLY);
    assert_eq!(opcode(&receive(&mut r, s, 1)), BR_COMPLETE);
    close(&mut r, &[s, c]);
}

#[test]
fn rejected_reply_unblocks_caller_even_when_completion_queue_is_exhausted() {
    let (mut r, s, c) = setup(true);
    let (tx, b) = packet(&[], 0);
    r.transact(c, 2, tx, b).unwrap();
    receive(&mut r, c, 2);
    let d = receive(&mut r, s, 1);
    free(&mut r, s, &d);
    let (connection, thread) = r.thread_index(s, 1).unwrap();
    for _ in 0..MAX_EVENTS {
        r.connections[connection].threads[thread]
            .events
            .push_back(Event::Code(BR_COMPLETE));
    }
    r.fail_reply(s, 1).unwrap();
    assert_eq!(opcode(&receive(&mut r, c, 2)), BR_FAILED_REPLY);
    assert_eq!(opcode(&receive(&mut r, s, 1)), BR_COMPLETE);
    assert_eq!((r.stats().transactions, r.stats().call_frames), (0, 0));
    close(&mut r, &[s, c]);
}

#[test]
fn descriptor_batch_exhaustion_and_rollback_preserve_replacements() {
    let (_, _, original) = source();
    let files = vec![
        FileTransfer::new(8, original.clone()).unwrap(),
        FileTransfer::new(32, original.clone()).unwrap(),
    ];
    let mut t = Table::empty();
    for _ in 0..CAPACITY - 1 {
        t.insert(FileRef::try_new(Target::Null, 0).unwrap(), 0, false)
            .unwrap();
    }
    assert_eq!(t.receive_files(&files), Err(crate::linux_fd::EMFILE));
    assert_eq!(t.len(), CAPACITY - 1);
    t.close(0).unwrap();
    let numbers = t.receive_files(&files).unwrap();
    assert_eq!(numbers.len(), 2);
    let unrelated = t.file(1).unwrap();
    t.duplicate_to(1, numbers[0] as u64, false).unwrap();
    t.undo_received_files(&numbers, &files);
    assert!(
        t.file(numbers[0] as u64)
            .unwrap()
            .same_description(&unrelated)
    );
    assert!(t.file(numbers[1] as u64).is_err());
}

#[test]
fn malformed_fd_arrays_and_unpublished_transfers_release_buffer_holds() {
    let (table, fd, _) = source();
    let (tx, mut b) = packet(&[fd, fd], 0);
    put64(&mut b, 56, 4);
    assert!(Parcel::capture(tx, b, &table).is_err());
    let (tx, b) = packet(&[9999], 0);
    assert!(Parcel::capture(tx, b, &table).is_err());
    for action in 0..4 {
        let (mut r, s, c) = setup(true);
        let (tx, b) = packet(&[fd], 0);
        r.transact_parcel(c, 2, tx, Parcel::capture(tx, b, &table).unwrap())
            .unwrap();
        receive(&mut r, c, 2);
        if action == 0 {
            let d = r.read(s, 1, 256).unwrap().unwrap();
            r.discard_delivery(s, &d);
        } else if action == 1 {
            r.unmap_inode(1);
        } else if action == 2 {
            r.thread_exit(c, 2);
        } else {
            r.register_poll(s, 1, 1).unwrap();
            r.flush(s);
            let d = r.read(s, 1, 256).unwrap().unwrap();
            r.reject_files(s, 1, &d);
            assert_eq!(receive(&mut r, s, 1).length, 4);
        }
        assert_eq!(
            (
                r.stats().transactions,
                r.stats().pending_fds,
                r.stats().buffers
            ),
            (0, 0, 0)
        );
        close(&mut r, &[s, c]);
    }
    // A Binder descriptor would create a recursive router ownership cycle.
    let file = FileRef::try_binder(
        Identity {
            process: 99,
            uid: 10000,
        },
        Domain::Binder,
        2,
    )
    .unwrap();
    assert!(matches!(FileTransfer::new(8, file), Err(95)));
}

#[test]
fn binder_readiness_is_thread_owned_nonconsuming_and_gated_by_looper_entry() {
    let (mut r, s, c) = setup(true);
    r.register_poll(s, 1, 1).unwrap();
    r.register_poll(c, 2, 2).unwrap();
    let (tx, b) = packet(&[], 0);
    r.transact(c, 2, tx, b).unwrap();
    assert_eq!(r.readiness(2, 2).mask(c), 1);
    assert_eq!(r.readiness(2, 3).mask(c), 0);
    assert_eq!(r.readiness(1, 1).mask(s), 0);
    assert_eq!(r.readiness(99, 1).mask(s), 8);
    r.looper(s, 1, BC_ENTER_LOOPER).unwrap();
    let revision = r.readiness(1, 1).revision;
    assert_eq!(r.readiness(1, 1).mask(s), 1);
    assert_eq!(r.readiness(1, 1).revision, revision);
    let d = receive(&mut r, s, 1);
    free(&mut r, s, &d);
    assert_eq!(r.readiness(1, 1).mask(s), 0);
    receive(&mut r, c, 2);
    let (tx, b) = packet(&[], 0);
    r.reply(s, 1, tx, b).unwrap();
    assert_eq!(r.readiness(2, 2).mask(c), 1);
    let d = receive(&mut r, c, 2);
    free(&mut r, c, &d);
    receive(&mut r, s, 1);
    r.flush(s);
    assert_eq!(r.readiness(1, 1).mask(s), 1);
    let d = receive(&mut r, s, 1);
    assert_eq!(d.length, 4);
    assert_eq!(r.readiness(1, 1).mask(s), 0);
    close(&mut r, &[s, c]);
}
