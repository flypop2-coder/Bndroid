use super::*;
fn take(h: &mut Hub, ep: u64, n: usize) -> Vec<Event> {
    let events = h.prepare(ep, n).unwrap();
    for e in &events {
        h.commit(ep, *e).unwrap();
    }
    events
}
#[test]
fn counters_block_without_overflow_and_semaphore_consumes_one() {
    let mut h = Hub::new();
    let fd = h.eventfd(0, false).unwrap();
    assert_eq!(h.read(fd), Ok(None));
    assert!(h.write(fd, MAX_COUNTER).unwrap());
    assert!(!h.write(fd, 1).unwrap());
    assert_eq!(h.write(fd, u64::MAX), Err(22));
    assert_eq!(h.read(fd), Ok(Some(MAX_COUNTER)));
    assert!(h.write(fd, 0).unwrap());
    assert_eq!(h.read(fd), Ok(None));
    let semaphore = h.eventfd(3, true).unwrap();
    for _ in 0..3 {
        assert_eq!(h.read(semaphore), Ok(Some(1)));
    }
    assert_eq!(h.read(semaphore), Ok(None));
    let ep = h.epoll().unwrap();
    h.control(ep, 1, 3, fd, IN | OUT, 3).unwrap();
    assert_eq!(take(&mut h, ep, 1)[0].mask, OUT);
    h.write(fd, 1).unwrap();
    assert_eq!(take(&mut h, ep, 1)[0].mask, IN | OUT);
    h.write(fd, MAX_COUNTER - 1).unwrap();
    assert_eq!(take(&mut h, ep, 1)[0].mask, IN);
    h.close(ep);
    h.close(fd);
    h.close(semaphore);
    assert_eq!((h.stats().live, h.stats().watches), (0, 0));
}
#[test]
fn level_edge_oneshot_and_failed_copy_keep_distinct_delivery_state() {
    let mut h = Hub::new();
    let fd = h.eventfd(0, false).unwrap();
    let ep = h.epoll().unwrap();
    h.control(ep, 1, 3, fd, IN, 0x1122334455667788).unwrap();
    assert!(take(&mut h, ep, 8).is_empty());
    h.write(fd, 2).unwrap();
    for _ in 0..2 {
        assert_eq!(take(&mut h, ep, 1)[0].data, 0x1122334455667788);
    }
    h.control(ep, 3, 3, fd, IN | EDGE, 4).unwrap();
    let prepared = h.prepare(ep, 1).unwrap();
    assert_eq!(prepared.len(), 1);
    // A failed user copy never commits the prepared notification.
    assert_eq!(h.prepare(ep, 1).unwrap()[0].data, 4);
    h.commit(ep, prepared[0]).unwrap();
    assert!(take(&mut h, ep, 1).is_empty());
    h.write(fd, 1).unwrap();
    assert_eq!(take(&mut h, ep, 1).len(), 1);
    h.control(ep, 3, 3, fd, IN | ONESHOT, 9).unwrap();
    assert_eq!(take(&mut h, ep, 1)[0].data, 9);
    h.write(fd, 1).unwrap();
    assert!(take(&mut h, ep, 1).is_empty());
    h.control(ep, 3, 3, fd, IN | ONESHOT, 10).unwrap();
    assert_eq!(take(&mut h, ep, 1)[0].data, 10);
}
#[test]
fn dup_registration_uses_open_file_and_number_and_last_close_detaches() {
    let mut h = Hub::new();
    let fd = h.eventfd(1, false).unwrap();
    let ep = h.epoll().unwrap();
    h.control(ep, 1, 3, fd, IN, 30).unwrap();
    h.control(ep, 1, 4, fd, IN, 40).unwrap();
    assert_eq!(h.control(ep, 1, 3, fd, IN, 99), Err(17));
    let replacement = h.eventfd(0, false).unwrap();
    h.control(ep, 1, 3, replacement, IN, 50).unwrap();
    assert_eq!(take(&mut h, ep, 8).len(), 2);
    h.close(fd);
    assert!(!h.epoll_ready(ep).unwrap());
    assert_eq!(h.stats().watches, 1);
    assert_eq!(h.control(ep, 2, 4, replacement, 0, 0), Err(2));
    h.write(replacement, 1).unwrap();
    assert_eq!(take(&mut h, ep, 8)[0].data, 50);
    h.control(ep, 2, 3, replacement, 0, 0).unwrap();
    assert_eq!(h.stats().watches, 0);
    h.close(ep);
    h.close(replacement);
    assert_eq!(h.stats().live, 0);
}
#[test]
fn fairness_preserves_unreturned_events_and_ignores_drained_edges() {
    let mut h = Hub::new();
    let ep = h.epoll().unwrap();
    let a = h.eventfd(1, false).unwrap();
    let b = h.eventfd(1, false).unwrap();
    h.control(ep, 1, 3, a, IN, 1).unwrap();
    h.control(ep, 1, 4, b, IN, 2).unwrap();
    for expected in [1, 2, 1, 2] {
        assert_eq!(take(&mut h, ep, 1)[0].data, expected);
    }
    h.control(ep, 3, 3, a, IN | EDGE, 1).unwrap();
    h.control(ep, 3, 4, b, IN | EDGE, 2).unwrap();
    h.read(a).unwrap();
    h.read(b).unwrap();
    assert!(!h.epoll_ready(ep).unwrap());
    h.write(a, 1).unwrap();
    let event = take(&mut h, ep, 1)[0];
    assert_eq!((event.mask, event.data), (IN, 1));
    assert!(!h.epoll_ready(ep).unwrap());
}
#[test]
fn nested_readiness_tracks_child_consumption_and_cycles_fail_atomically() {
    let mut h = Hub::new();
    let a = h.epoll().unwrap();
    let b = h.epoll().unwrap();
    let fd = h.eventfd(0, false).unwrap();
    h.control(b, 1, 3, fd, IN | EDGE, 3).unwrap();
    h.control(a, 1, 4, b, IN | EDGE, 4).unwrap();
    assert_eq!(h.control(b, 1, 5, a, IN, 5), Err(40));
    assert_eq!(h.stats().watches, 2);
    h.write(fd, 1).unwrap();
    assert_eq!(take(&mut h, a, 1)[0].data, 4);
    assert_eq!(take(&mut h, b, 1)[0].data, 3);
    assert!(!h.epoll_ready(a).unwrap());
    h.write(fd, 1).unwrap();
    assert!(h.epoll_ready(a).unwrap());
    h.close(fd);
    assert!(!h.epoll_ready(a).unwrap());
    h.close(a);
    h.close(b);
    assert_eq!((h.stats().live, h.stats().watches), (0, 0));
}
#[test]
fn nested_depth_and_registration_limits_have_no_partial_publication() {
    let mut h = Hub::new();
    let mut child = h.eventfd(1, false).unwrap();
    for _ in 0..MAX_DEPTH {
        let parent = h.epoll().unwrap();
        h.control(parent, 1, 3, child, IN, 0).unwrap();
        child = parent;
    }
    let excessive = h.epoll().unwrap();
    assert_eq!(h.control(excessive, 1, 3, child, IN, 0), Err(40));
    assert_eq!(h.stats().watches, MAX_DEPTH as usize);
    assert_eq!(h.control(child, 1, 3, child, IN, 0), Err(22));
    assert_eq!(
        h.control(excessive, 1, 3, child, IN | (1 << 28), 0),
        Err(22)
    );
    assert_eq!(h.prepare(excessive, 0).err(), Some(22));
}
