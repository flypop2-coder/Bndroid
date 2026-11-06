//! Event counters and epoll ownership. No user pointers or FileRefs live here:
//! watches do not retain their targets and cannot create reference cycles.
#[cfg(test)]
mod tests;
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

pub const IN: u32 = 1;
pub const OUT: u32 = 4;
pub const ERR: u32 = 8;
pub const HUP: u32 = 16;
pub const EDGE: u32 = 1 << 31;
pub const ONESHOT: u32 = 1 << 30;
pub const MAX_OBJECTS: usize = 256;
pub const MAX_WATCHES: usize = 256;
const MAX_TOTAL_WATCHES: usize = 4096;
const MAX_DEPTH: u8 = 5;
const MAX_COUNTER: u64 = u64::MAX - 1;

#[derive(Clone, Copy, Default, Debug)]
pub struct Stats {
    pub eventfds: u64,
    pub epolls: u64,
    pub closed: u64,
    pub reads: u64,
    pub writes: u64,
    pub delivered: u64,
    pub controls: u64,
    pub live: usize,
    pub watches: usize,
}
struct Watch {
    id: u64,
    target: u64,
    fd: u32,
    mask: u32,
    data: u64,
    armed: bool,
    pending: bool,
}
enum Object {
    Counter { value: u64, semaphore: bool },
    Epoll { watches: Vec<Watch>, cursor: usize },
}
struct Entry {
    id: u64,
    object: Object,
    notified: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub mask: u32,
    pub data: u64,
    watch: u64,
}
impl Event {
    // AArch64 epoll_event has four padding bytes, unlike packed x86-64.
    pub fn encode(self) -> [u8; 16] {
        let mut b = [0; 16];
        b[..4].copy_from_slice(&self.mask.to_le_bytes());
        b[8..].copy_from_slice(&self.data.to_le_bytes());
        b
    }
}
pub struct Hub {
    entries: Vec<Entry>,
    next: u64,
    notification: u64,
    stats: Stats,
}
impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}
impl Hub {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next: 1,
            notification: 0,
            stats: Stats {
                eventfds: 0,
                epolls: 0,
                closed: 0,
                reads: 0,
                writes: 0,
                delivered: 0,
                controls: 0,
                live: 0,
                watches: 0,
            },
        }
    }
    fn index(&self, id: u64) -> Result<usize, i64> {
        self.entries.iter().position(|e| e.id == id).ok_or(9)
    }
    fn token(&mut self) -> Result<u64, i64> {
        let id = self.next;
        self.next = id.checked_add(1).ok_or(75)?;
        Ok(id)
    }
    fn create(&mut self, object: Object) -> Result<u64, i64> {
        if self.entries.len() >= MAX_OBJECTS {
            return Err(23);
        }
        self.entries.try_reserve(1).map_err(|_| 12)?;
        let id = self.token()?;
        self.entries.push(Entry {
            id,
            object,
            notified: 0,
        });
        Ok(id)
    }
    pub fn eventfd(&mut self, value: u32, semaphore: bool) -> Result<u64, i64> {
        let id = self.create(Object::Counter {
            value: value as u64,
            semaphore,
        })?;
        self.stats.eventfds += 1;
        Ok(id)
    }
    pub fn epoll(&mut self) -> Result<u64, i64> {
        let id = self.create(Object::Epoll {
            watches: Vec::new(),
            cursor: 0,
        })?;
        self.stats.epolls += 1;
        Ok(id)
    }
    pub fn can_read(&self, id: u64) -> Result<bool, i64> {
        match self.entries[self.index(id)?].object {
            Object::Counter { value, .. } => Ok(value != 0),
            _ => Err(22),
        }
    }
    pub fn can_write(&self, id: u64, add: u64) -> Result<bool, i64> {
        if add == u64::MAX {
            return Err(22);
        }
        match self.entries[self.index(id)?].object {
            Object::Counter { value, .. } => Ok(add <= MAX_COUNTER - value),
            _ => Err(22),
        }
    }
    pub fn read(&mut self, id: u64) -> Result<Option<u64>, i64> {
        let i = self.index(id)?;
        let Object::Counter { value, semaphore } = &mut self.entries[i].object else {
            return Err(22);
        };
        if *value == 0 {
            return Ok(None);
        }
        let result = if *semaphore { 1 } else { *value };
        *value -= result;
        self.stats.reads += 1;
        self.notify(id, OUT);
        Ok(Some(result))
    }
    pub fn write(&mut self, id: u64, add: u64) -> Result<bool, i64> {
        if !self.can_write(id, add)? {
            return Ok(false);
        }
        let i = self.index(id)?;
        let Object::Counter { value, .. } = &mut self.entries[i].object else {
            return Err(22);
        };
        *value += add;
        self.stats.writes += 1;
        self.notify(id, IN);
        Ok(true)
    }
    fn notify(&mut self, id: u64, mask: u32) {
        self.notification = self.notification.wrapping_add(1);
        if self.notification == 0 {
            for e in &mut self.entries {
                e.notified = 0;
            }
            self.notification = 1;
        }
        self.propagate(id, mask);
    }
    fn propagate(&mut self, id: u64, mask: u32) {
        for i in 0..self.entries.len() {
            let e = &mut self.entries[i];
            let Object::Epoll { watches, .. } = &mut e.object else {
                continue;
            };
            let mut changed = false;
            for w in watches {
                if w.target == id && w.armed && w.mask & mask != 0 {
                    w.pending = true;
                    changed = true;
                }
            }
            if changed && e.notified != self.notification {
                e.notified = self.notification;
                let parent = e.id;
                self.propagate(parent, IN);
            }
        }
    }
    fn watches(&self, id: u64) -> Result<(&[Watch], usize), i64> {
        match &self.entries[self.index(id)?].object {
            Object::Epoll { watches, cursor } => Ok((watches, *cursor)),
            _ => Err(22),
        }
    }
    fn graph_depth(&self, i: usize, depths: &mut [u8; MAX_OBJECTS]) -> Result<u8, i64> {
        if depths[i] == 255 {
            return Err(40);
        }
        if depths[i] != 0 {
            return Ok(depths[i] - 1);
        }
        depths[i] = 255;
        let mut depth = 0;
        if let Object::Epoll { watches, .. } = &self.entries[i].object {
            depth = 1;
            for w in watches {
                depth = depth.max(self.graph_depth(self.index(w.target)?, depths)? + 1);
                if depth > MAX_DEPTH {
                    return Err(40);
                }
            }
        }
        depths[i] = depth + 1;
        Ok(depth)
    }
    pub fn control(
        &mut self,
        id: u64,
        operation: u32,
        fd: u32,
        target: u64,
        mask: u32,
        data: u64,
    ) -> Result<(), i64> {
        if id == target {
            return Err(22);
        }
        self.index(target)?;
        let (watches, _) = self.watches(id)?;
        let found = watches
            .iter()
            .position(|w| w.target == target && w.fd == fd);
        if !matches!(operation, 1..=3) {
            return Err(22);
        }
        // Power-management and exclusive wait-queue policies are unsupported.
        if operation != 2 && mask & !(0x3fff | EDGE | ONESHOT) != 0 {
            return Err(22);
        }
        if operation == 1 && found.is_some() {
            return Err(17);
        }
        if operation != 1 && found.is_none() {
            return Err(2);
        }
        if operation == 1
            && (watches.len() >= MAX_WATCHES || self.stats().watches >= MAX_TOTAL_WATCHES)
        {
            return Err(28);
        }
        let i = self.index(id)?;
        let token = if operation == 1 { self.token()? } else { 0 };
        let Object::Epoll { watches, .. } = &mut self.entries[i].object else {
            unreachable!()
        };
        match operation {
            1 => {
                watches.try_reserve(1).map_err(|_| 12)?;
                watches.push(Watch {
                    id: token,
                    target,
                    fd,
                    mask: mask | ERR | HUP,
                    data,
                    armed: true,
                    pending: true,
                });
            }
            2 => {
                watches.remove(found.unwrap());
            }
            3 => {
                let w = &mut watches[found.unwrap()];
                w.mask = mask | ERR | HUP;
                w.data = data;
                w.armed = true;
                w.pending = true;
            }
            _ => unreachable!(),
        }
        if operation == 1 {
            let mut depths = [0; MAX_OBJECTS];
            for at in 0..self.entries.len() {
                if let Err(error) = self.graph_depth(at, &mut depths) {
                    let Object::Epoll { watches, .. } = &mut self.entries[i].object else {
                        unreachable!()
                    };
                    watches.pop();
                    return Err(error);
                }
            }
        }
        self.stats.controls += 1;
        self.notify(id, IN);
        Ok(())
    }
    fn readiness(&self) -> [u32; MAX_OBJECTS] {
        let mut ready = [0; MAX_OBJECTS];
        for (i, e) in self.entries.iter().enumerate() {
            if let Object::Counter { value, .. } = e.object {
                ready[i] =
                    if value > 0 { IN } else { 0 } | if value < MAX_COUNTER { OUT } else { 0 };
            }
        }
        // The acyclic graph has at most MAX_DEPTH epoll levels. Bounded passes
        // avoid exponential recursion through shared nested interest lists.
        for _ in 0..MAX_DEPTH {
            for (i, e) in self.entries.iter().enumerate() {
                if let Object::Epoll { watches, .. } = &e.object {
                    ready[i] = if watches.iter().any(|w| {
                        w.armed
                            && (w.mask & EDGE == 0 || w.pending)
                            && self.index(w.target).is_ok_and(|at| ready[at] & w.mask != 0)
                    }) {
                        IN
                    } else {
                        0
                    };
                }
            }
        }
        ready
    }
    pub fn epoll_ready(&self, id: u64) -> Result<bool, i64> {
        self.watches(id)?;
        Ok(self.readiness()[self.index(id)?] != 0)
    }
    pub fn prepare(&self, id: u64, maximum: usize) -> Result<Vec<Event>, i64> {
        if maximum == 0 {
            return Err(22);
        }
        let (watches, cursor) = self.watches(id)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(maximum.min(watches.len()))
            .map_err(|_| 12)?;
        let ready = self.readiness();
        for at in 0..watches.len() {
            let w = &watches[(at + cursor) % watches.len()];
            if !w.armed || w.mask & EDGE != 0 && !w.pending {
                continue;
            }
            let mask = ready[self.index(w.target)?] & w.mask;
            if mask != 0 {
                result.push(Event {
                    mask,
                    data: w.data,
                    watch: w.id,
                });
                if result.len() == maximum {
                    break;
                }
            }
        }
        Ok(result)
    }
    pub fn commit(&mut self, id: u64, event: Event) -> Result<(), i64> {
        let i = self.index(id)?;
        let Object::Epoll { watches, cursor } = &mut self.entries[i].object else {
            return Err(22);
        };
        let at = watches.iter().position(|w| w.id == event.watch).ok_or(2)?;
        let w = &mut watches[at];
        w.pending = false;
        if w.mask & ONESHOT != 0 {
            w.armed = false;
        }
        *cursor = (at + 1) % watches.len();
        self.stats.delivered += 1;
        Ok(())
    }
    pub fn close(&mut self, id: u64) {
        let Ok(i) = self.index(id) else { return };
        self.entries.remove(i);
        for e in &mut self.entries {
            if let Object::Epoll { watches, .. } = &mut e.object {
                watches.retain(|w| w.target != id);
            }
        }
        self.stats.closed += 1;
    }
    pub fn stats(&self) -> Stats {
        Stats {
            live: self.entries.len(),
            watches: self
                .entries
                .iter()
                .map(|e| match &e.object {
                    Object::Epoll { watches, .. } => watches.len(),
                    _ => 0,
                })
                .sum(),
            ..self.stats
        }
    }
}
struct Device {
    busy: AtomicBool,
    hub: UnsafeCell<Hub>,
}
unsafe impl Sync for Device {}
static DEVICE: Device = Device {
    busy: AtomicBool::new(false),
    hub: UnsafeCell::new(Hub::new()),
};
pub fn with<R>(f: impl FnOnce(&mut Hub) -> R) -> R {
    while DEVICE
        .busy
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    struct Unlock;
    impl Drop for Unlock {
        fn drop(&mut self) {
            DEVICE.busy.store(false, Ordering::Release);
        }
    }
    let _unlock = Unlock;
    f(unsafe { &mut *DEVICE.hub.get() })
}
