//! Blocking eventfd/epoll calls retain the original open file. Timer wakeups
//! inspect readiness only; user copies and consumption run on the owner's stack.
use super::*;
use bndroid_kernel::{
    linux_fd::{FileRef, Target},
    linux_poll as poll,
};

#[derive(Clone, Copy, Default)]
struct Segment {
    address: u64,
    length: usize,
}
enum Operation {
    Read([Segment; 8]),
    Write(u64),
    Poll { address: u64, maximum: usize },
}
struct Call {
    file: FileRef,
    operation: Operation,
    deadline: Option<u64>,
    args: [u64; 6],
}
struct Calls(UnsafeCell<[Option<Call>; CONTEXT_COUNT]>);
unsafe impl Sync for Calls {}
static CALLS: Calls = Calls(UnsafeCell::new([const { None }; CONTEXT_COUNT]));
static BLOCKED: AtomicU64 = AtomicU64::new(0);
static READY: AtomicU64 = AtomicU64::new(0);
static EXPIRED: AtomicU64 = AtomicU64::new(0);
static CANCELLED: AtomicU64 = AtomicU64::new(0);
fn slot(c: usize) -> &'static mut Option<Call> {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut (*CALLS.0.get())[c] }
}
fn file(fd: u64) -> Result<FileRef, i64> {
    crate::process::with_linux_files(|t| t.file(fd))
}
fn insert(f: FileRef, flags: u64) -> Result<u64, i64> {
    crate::process::with_linux_files(|t| t.insert(f, 0, flags & 0x80000 != 0))
}

pub(super) fn dispatch(frame: *mut TrapFrame, r: &[u64; 31]) -> Option<*mut TrapFrame> {
    let result = match r[8] {
        19 => FileRef::try_eventfd(r[0] as u32, r[1] as u32).and_then(|f| insert(f, r[1])),
        20 => FileRef::try_epoll(r[0] as u32).and_then(|f| insert(f, r[0])),
        21 => control(r),
        22 | 441 => return Some(wait_epoll(frame, r)),
        63..=66 => {
            let f = match file(r[0]) {
                Ok(f) if matches!(f.target(), Target::EventFd | Target::Epoll) => f,
                _ => return None,
            };
            if matches!(f.target(), Target::Epoll) {
                Err(22)
            } else {
                return Some(counter_io(frame, r, f));
            }
        }
        _ => return None,
    };
    wake_ready(crate::arch::aarch64::timer::counter_value());
    Some(runtime::complete(frame, result.unwrap_or_else(errno)))
}
fn control(r: &[u64; 31]) -> Result<u64, i64> {
    let ep = file(r[0])?;
    let target = file(r[2])?;
    if !matches!(ep.target(), Target::Epoll) {
        return Err(22);
    }
    let target = target.poll().ok_or(1)?.id;
    let mut bytes = [0; 16];
    if r[1] as u32 != 2 {
        runtime::copy_in(r[3], &mut bytes)?;
    }
    let mask = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    let data = u64::from_le_bytes(bytes[8..].try_into().unwrap());
    poll::with(|h| {
        h.control(
            ep.poll().unwrap().id,
            r[1] as u32,
            r[2] as u32,
            target,
            mask,
            data,
        )
    })?;
    Ok(0)
}
fn segments(r: &[u64; 31]) -> Result<[Segment; 8], i64> {
    let mut result = [Segment::default(); 8];
    if r[8] == 63 || r[8] == 64 {
        if r[2] < 8 {
            return Err(22);
        }
        result[0] = Segment {
            address: r[1],
            length: 8,
        };
        return Ok(result);
    }
    if r[2] > 1024 {
        return Err(22);
    }
    let mut total = 0u64;
    let mut used = 0;
    let mut remaining = 8;
    for i in 0..r[2] {
        let mut b = [0; 16];
        runtime::copy_in(r[1].checked_add(i * 16).ok_or(14)?, &mut b)?;
        let address = u64::from_le_bytes(b[..8].try_into().unwrap());
        let length = u64::from_le_bytes(b[8..].try_into().unwrap());
        total = total
            .checked_add(length)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(22)?;
        if length != 0 && remaining != 0 {
            let n = length.min(remaining as u64) as usize;
            result[used] = Segment { address, length: n };
            used += 1;
            remaining -= n;
        }
    }
    if total < 8 {
        return Err(22);
    }
    Ok(result)
}
fn counter_io(frame: *mut TrapFrame, r: &[u64; 31], file: FileRef) -> *mut TrapFrame {
    let result = (|| {
        let parts = segments(r)?;
        let operation = if matches!(r[8], 63 | 65) {
            Operation::Read(parts)
        } else {
            let mut b = [0; 8];
            let mut at = 0;
            for s in parts {
                if s.length != 0 {
                    runtime::copy_in(s.address, &mut b[at..at + s.length])?;
                    at += s.length;
                }
            }
            let value = u64::from_le_bytes(b);
            if value == u64::MAX {
                return Err(22);
            }
            Operation::Write(value)
        };
        Ok(Call {
            file,
            operation,
            deadline: None,
            args: r[..6].try_into().unwrap(),
        })
    })();
    // Consume a signal-restored syscall record; eventfd has no timeout to reuse.
    let _ = signals::restart_deadline(frame, r);
    match result {
        Ok(call) => run(frame, call),
        Err(e) => runtime::complete(frame, errno(e)),
    }
}
fn wait_epoll(frame: *mut TrapFrame, r: &[u64; 31]) -> *mut TrapFrame {
    let result = (|| {
        let file = file(r[0])?;
        if !matches!(file.target(), Target::Epoll) {
            return Err(22);
        }
        let maximum = r[2] as i32;
        if maximum <= 0 || maximum > i32::MAX / 16 {
            return Err(22);
        }
        let now = crate::arch::aarch64::timer::counter_value();
        let frequency = crate::arch::aarch64::timer::frequency_hz();
        let deadline = if r[8] == 441 {
            if r[3] == 0 {
                None
            } else {
                let mut b = [0; 16];
                runtime::copy_in(r[3], &mut b)?;
                Some(linux_sync::deadline(
                    i64::from_le_bytes(b[..8].try_into().unwrap()),
                    i64::from_le_bytes(b[8..].try_into().unwrap()),
                    frequency,
                    now,
                    false,
                )?)
            }
        } else {
            let ms = r[3] as i32;
            if ms < 0 {
                None
            } else {
                Some(linux_sync::deadline(
                    ms as i64 / 1000,
                    ms as i64 % 1000 * 1_000_000,
                    frequency,
                    now,
                    false,
                )?)
            }
        };
        let mask = if r[4] == 0 {
            None
        } else {
            if r[5] != 8 {
                return Err(22);
            }
            let mut b = [0; 8];
            runtime::copy_in(r[4], &mut b)?;
            Some(u64::from_le_bytes(b))
        };
        signals::begin_wait_mask(mask);
        Ok(Call {
            file,
            operation: Operation::Poll {
                address: r[1],
                maximum: maximum as usize,
            },
            deadline,
            args: r[..6].try_into().unwrap(),
        })
    })();
    match result {
        Ok(call) => run(frame, call),
        Err(e) => runtime::complete(frame, errno(e)),
    }
}
fn ready(call: &Call) -> Result<bool, i64> {
    let id = call.file.poll().unwrap().id;
    poll::with(|h| match call.operation {
        Operation::Read(_) => h.can_read(id),
        Operation::Write(value) => h.can_write(id, value),
        Operation::Poll { .. } => h.epoll_ready(id),
    })
}
#[inline(never)]
fn attempt(call: &Call) -> Result<Option<u64>, i64> {
    let id = call.file.poll().unwrap().id;
    match &call.operation {
        Operation::Read(parts) => {
            let Some(value) = poll::with(|h| h.read(id))? else {
                return Ok(None);
            };
            let bytes = value.to_le_bytes();
            let mut at = 0;
            // Like Linux eventfd, a failed copy after consumption reports
            // EFAULT; no unchecked pointer is ever used on the timer path.
            for s in parts {
                if s.length != 0 {
                    output(s.address, &bytes[at..at + s.length])?;
                    at += s.length;
                }
            }
            Ok(Some(8))
        }
        Operation::Write(value) => Ok(poll::with(|h| h.write(id, *value))?.then_some(8)),
        Operation::Poll { address, maximum } => {
            let events = poll::with(|h| h.prepare(id, *maximum))?;
            if events.is_empty() {
                return Ok(call
                    .deadline
                    .filter(|end| {
                        deadline_reached(crate::arch::aarch64::timer::counter_value(), *end)
                    })
                    .map(|_| 0));
            }
            let mut count = 0;
            for event in events {
                let result = address
                    .checked_add(count * 16)
                    .ok_or(14)
                    .and_then(|a| output(a, &event.encode()));
                if let Err(error) = result {
                    return if count == 0 {
                        Err(error)
                    } else {
                        Ok(Some(count))
                    };
                }
                poll::with(|h| h.commit(id, event))?;
                count += 1;
            }
            Ok(Some(count))
        }
    }
}
fn run(frame: *mut TrapFrame, call: Call) -> *mut TrapFrame {
    let result = attempt(&call);
    wake_ready(crate::arch::aarch64::timer::counter_value());
    let result = match result {
        Ok(None) if call.file.flags() & 0x800 != 0 => Err(11),
        Ok(None) => return suspend(frame, call),
        Ok(Some(value)) => Ok(value),
        Err(e) => Err(e),
    };
    signals::restore_wait_mask();
    runtime::complete(frame, result.unwrap_or_else(errno))
}
fn suspend(frame: *mut TrapFrame, call: Call) -> *mut TrapFrame {
    let c = current_running_context();
    // Deadlines belong to this I/O call, not the futex timeout-result slot.
    if let Err(e) = queue().insert(
        c,
        Wait {
            process: owner(c),
            address: 0,
            bitset: !0,
            deadline: None,
        },
    ) {
        signals::restore_wait_mask();
        return runtime::complete(frame, errno(e));
    }
    if !matches!(call.operation, Operation::Poll { .. }) {
        signals::note_restartable(frame, call.args, None);
    }
    assert!(slot(c).is_none());
    *slot(c) = Some(call);
    BLOCKED.fetch_add(1, Ordering::Relaxed);
    WAITS.fetch_add(1, Ordering::Relaxed);
    block(frame, ThreadState::Waiting)
}
pub(super) fn wake_ready(now: u64) {
    for (c, state) in CONTEXTS.iter().enumerate().skip(DYNAMIC_USER_CONTEXT_START) {
        if state.state.load(Ordering::Relaxed) != STATE_WAITING {
            continue;
        }
        let Some(call) = slot(c).as_ref() else {
            continue;
        };
        let ready = ready(call).unwrap_or(true);
        let expired = call.deadline.is_some_and(|end| deadline_reached(now, end));
        if ready || expired {
            assert!(queue().cancel(c));
            wake(1 << c, 0);
            WAKES.fetch_add(1, Ordering::Relaxed);
            if ready {
                READY.fetch_add(1, Ordering::Relaxed);
            } else {
                EXPIRED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
pub(super) fn complete_ready(frame: *mut TrapFrame) -> Option<*mut TrapFrame> {
    let call = slot(current_running_context()).take()?;
    let next = run(frame, call);
    // A competing consumer can drain a ready fd before this context resumes.
    // Recheck and sleep again instead of returning a false success/EAGAIN.
    Some(next)
}
pub(super) fn cancel(c: usize) {
    if slot(c).take().is_some() {
        CANCELLED.fetch_add(1, Ordering::Relaxed);
    }
}
pub(super) fn evidence() {
    assert!((0..CONTEXT_COUNT).all(|c| slot(c).is_none()));
    let s = poll::with(|h| h.stats());
    crate::kprintln!(
        "LINUX_POLL_OK eventfds={} epolls={} closed={} reads={} writes={} delivered={} controls={} blocked={} ready={} expired={} cancelled={} live={} watches={}",
        s.eventfds,
        s.epolls,
        s.closed,
        s.reads,
        s.writes,
        s.delivered,
        s.controls,
        BLOCKED.load(Ordering::Relaxed),
        READY.load(Ordering::Relaxed),
        EXPIRED.load(Ordering::Relaxed),
        CANCELLED.load(Ordering::Relaxed),
        s.live,
        s.watches
    );
    assert_eq!((s.live, s.watches), (0, 0));
}
