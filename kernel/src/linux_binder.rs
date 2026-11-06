//! Binder transaction ownership, independent of user copies and page tables.
//! The first transport supports context-manager byte transactions and death
//! notifications. Binder-object/FD transfer is deliberately rejected, not faked.
#[cfg(test)]
mod tests;
pub mod wire;
use alloc::{collections::VecDeque, vec::Vec};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
use wire::*;

const MAX_CONNECTIONS: usize = 16;
const MAX_THREADS: usize = 32;
const MAX_TRANSACTIONS: usize = 128;
const MAX_BUFFERS: usize = 128;
const MAX_EVENTS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub process: u64,
    pub uid: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub base: u64,
    pub length: u64,
    pub inode: u64,
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Stats {
    pub opened: u64,
    pub closed: u64,
    pub submitted: u64,
    pub replied: u64,
    pub oneway: u64,
    pub failed: u64,
    pub deaths: u64,
    pub connections: usize,
    pub transactions: usize,
    pub buffers: usize,
    pub first_sender: u64,
    pub first_receiver: u64,
}

#[derive(Clone, Copy)]
enum Event {
    Code(u32),
    Request(u64),
    Reply(u64),
}
struct Thread {
    tid: u32,
    looper: u8,
    waiting: bool,
    outgoing: Option<u64>,
    incoming: Option<u64>,
    error: Option<u32>,
    events: VecDeque<Event>,
}
struct Buffer {
    id: u64,
    transaction_id: Option<u64>,
    offset: u64,
    length: u64,
    payload: Option<Vec<u8>>,
    transaction: Transaction,
    delivered: bool,
    asynchronous: bool,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum DeathState {
    Armed,
    Ready,
    Delivered,
    Acknowledged,
    ClearReady,
}
struct Death {
    cookie: u64,
    state: DeathState,
    clearing: bool,
}
struct Connection {
    id: u64,
    identity: Identity,
    mapping: Option<Mapping>,
    mapped_once: bool,
    threads: Vec<Thread>,
    process_events: VecDeque<Event>,
    buffers: Vec<Buffer>,
    max_threads: u32,
    requested: bool,
    asynchronous: Option<u64>,
    reference: Option<u64>,
    strong: u32,
    weak: u32,
    death: Option<Death>,
}
struct Pending {
    id: u64,
    from: u64,
    tid: u32,
    to: u64,
    receiver: Option<u32>,
    offset: u64,
    oneway: bool,
}
#[derive(Clone, Copy)]
struct Manager {
    connection: u64,
    pointer: u64,
    cookie: u64,
}

pub struct Delivery {
    pub bytes: [u8; 80],
    pub length: usize,
    pub offset: Option<u64>,
    pub payload: Option<Vec<u8>>,
    transaction: Option<u64>,
    buffer_id: Option<u64>,
}
impl Delivery {
    fn code(code: u32) -> Self {
        let mut bytes = [0; 80];
        put32(&mut bytes, 0, BR_NOOP);
        put32(&mut bytes, 4, code);
        Self {
            bytes,
            length: 8,
            offset: None,
            payload: None,
            transaction: None,
            buffer_id: None,
        }
    }
}

pub struct Router {
    connections: Vec<Connection>,
    pending: Vec<Pending>,
    next_id: u64,
    manager: Option<Manager>,
    manager_uid: Option<u32>,
    stats: Stats,
}
impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
impl Router {
    pub const fn new() -> Self {
        Self {
            connections: Vec::new(),
            pending: Vec::new(),
            next_id: 1,
            manager: None,
            manager_uid: None,
            stats: Stats {
                opened: 0,
                closed: 0,
                submitted: 0,
                replied: 0,
                oneway: 0,
                failed: 0,
                deaths: 0,
                connections: 0,
                transactions: 0,
                buffers: 0,
                first_sender: 0,
                first_receiver: 0,
            },
        }
    }
    fn index(&self, id: u64) -> Result<usize, i64> {
        self.connections.iter().position(|c| c.id == id).ok_or(9)
    }
    fn thread_index(&mut self, id: u64, tid: u32) -> Result<(usize, usize), i64> {
        let c = self.index(id)?;
        if let Some(t) = self.connections[c]
            .threads
            .iter()
            .position(|t| t.tid == tid)
        {
            return Ok((c, t));
        }
        let threads = &mut self.connections[c].threads;
        if tid == 0 || threads.len() >= MAX_THREADS {
            return Err(11);
        }
        threads.try_reserve(1).map_err(|_| 12)?;
        threads.push(Thread {
            tid,
            looper: 0,
            waiting: false,
            outgoing: None,
            incoming: None,
            error: None,
            events: VecDeque::new(),
        });
        Ok((c, threads.len() - 1))
    }
    pub fn open(&mut self, identity: Identity) -> Result<u64, i64> {
        if identity.process == 0 || self.connections.len() >= MAX_CONNECTIONS {
            return Err(24);
        }
        self.connections.try_reserve(1).map_err(|_| 12)?;
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(75)?;
        self.connections.push(Connection {
            id,
            identity,
            mapping: None,
            mapped_once: false,
            threads: Vec::new(),
            process_events: VecDeque::new(),
            buffers: Vec::new(),
            max_threads: 0,
            requested: false,
            asynchronous: None,
            reference: None,
            strong: 0,
            weak: 0,
            death: None,
        });
        self.stats.opened += 1;
        Ok(id)
    }
    pub fn authenticate(&self, id: u64, owner: u64) -> Result<(), i64> {
        if self.connections[self.index(id)?].identity.process != owner {
            return Err(1);
        }
        Ok(())
    }
    pub fn can_map(&self, id: u64) -> Result<(), i64> {
        if self.connections[self.index(id)?].mapped_once {
            Err(16)
        } else {
            Ok(())
        }
    }
    pub fn map(&mut self, id: u64, mapping: Mapping) -> Result<(), i64> {
        self.can_map(id)?;
        if mapping.base == 0
            || mapping.length == 0
            || mapping.length > MAX_MAPPING
            || !mapping.base.is_multiple_of(4096)
            || !mapping.length.is_multiple_of(4096)
            || mapping.base.checked_add(mapping.length).is_none()
        {
            return Err(22);
        }
        let c = self.index(id)?;
        self.connections[c].mapping = Some(mapping);
        self.connections[c].mapped_once = true;
        Ok(())
    }
    pub fn mapping(&self, id: u64) -> Option<Mapping> {
        self.index(id)
            .ok()
            .and_then(|i| self.connections[i].mapping)
    }
    pub fn mapping_for_inode(&self, inode: u64) -> Option<Mapping> {
        self.connections
            .iter()
            .filter_map(|c| c.mapping)
            .find(|m| m.inode == inode)
    }
    pub fn set_manager(&mut self, id: u64, pointer: u64, cookie: u64) -> Result<(), i64> {
        let c = self.index(id)?;
        if self.manager.is_some() {
            return Err(16);
        }
        let uid = self.connections[c].identity.uid;
        if self.manager_uid.is_some_and(|prior| prior != uid) {
            return Err(1);
        }
        self.manager_uid = Some(uid);
        self.manager = Some(Manager {
            connection: id,
            pointer,
            cookie,
        });
        Ok(())
    }
    pub fn set_max_threads(&mut self, id: u64, maximum: u32) -> Result<(), i64> {
        if maximum as usize > MAX_THREADS {
            return Err(22);
        }
        let c = self.index(id)?;
        self.connections[c].max_threads = maximum;
        Ok(())
    }
    pub fn looper(&mut self, id: u64, tid: u32, command: u32) -> Result<(), i64> {
        let (c, t) = self.thread_index(id, tid)?;
        let connection = &mut self.connections[c];
        match command {
            BC_ENTER_LOOPER if connection.threads[t].looper == 0 => {
                connection.threads[t].looper = 1
            }
            BC_REGISTER_LOOPER if connection.requested && connection.threads[t].looper == 0 => {
                connection.requested = false;
                connection.threads[t].looper = 2;
            }
            BC_EXIT_LOOPER if connection.threads[t].looper != 0 => connection.threads[t].looper = 3,
            _ => return Err(22),
        }
        Ok(())
    }
    pub fn waiting(&mut self, id: u64, tid: u32, waiting: bool) -> Result<(), i64> {
        let (c, t) = self.thread_index(id, tid)?;
        self.connections[c].threads[t].waiting = waiting;
        Ok(())
    }
    pub fn reference(&mut self, id: u64, handle: u32, command: u32) -> Result<(), i64> {
        if handle != 0 {
            return Err(22);
        }
        let c = self.index(id)?;
        let target = self.connections[c]
            .reference
            .or(self.manager.map(|m| m.connection))
            .ok_or(22)?;
        let connection = &mut self.connections[c];
        let count = match command {
            BC_INCREFS | BC_DECREFS => &mut connection.weak,
            BC_ACQUIRE | BC_RELEASE => &mut connection.strong,
            _ => return Err(22),
        };
        *count = if matches!(command, BC_INCREFS | BC_ACQUIRE) {
            count.checked_add(1)
        } else {
            count.checked_sub(1)
        }
        .ok_or(22)?;
        connection.reference = Some(target);
        if connection.strong == 0 && connection.weak == 0 && connection.death.is_none() {
            connection.reference = None;
        }
        Ok(())
    }
    fn reserve_event(queue: &mut VecDeque<Event>) -> Result<(), i64> {
        if queue.len() >= MAX_EVENTS {
            return Err(11);
        }
        queue.try_reserve(1).map_err(|_| 12)
    }
    fn buffer(
        &mut self,
        c: usize,
        mut transaction: Transaction,
        payload: Vec<u8>,
        transaction_id: Option<u64>,
    ) -> Result<u64, i64> {
        let connection = &mut self.connections[c];
        let mapping = connection.mapping.ok_or(12)?;
        if connection.buffers.len() >= MAX_BUFFERS || payload.len() as u64 != transaction.data_size
        {
            return Err(12);
        }
        let length = (transaction.data_size.max(1) + 7) & !7;
        let mut offset = 0;
        for b in &connection.buffers {
            if offset + length <= b.offset {
                break;
            }
            offset = b.offset + b.length;
        }
        if offset + length > mapping.length {
            return Err(28);
        }
        connection.buffers.try_reserve(1).map_err(|_| 12)?;
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(75)?;
        transaction.data = mapping.base + offset;
        transaction.offsets = transaction.data + length;
        let at = connection.buffers.partition_point(|b| b.offset < offset);
        connection.buffers.insert(
            at,
            Buffer {
                id,
                transaction_id,
                offset,
                length,
                payload: Some(payload),
                transaction,
                delivered: false,
                asynchronous: transaction.flags & ONE_WAY != 0,
            },
        );
        Ok(offset)
    }
    fn error(&mut self, id: u64, tid: u32, command: u32) {
        if let Ok(c) = self.index(id)
            && let Some(t) = self.connections[c]
                .threads
                .iter_mut()
                .find(|t| t.tid == tid)
        {
            t.error = Some(command);
        }
    }
    pub fn fail_transaction(&mut self, id: u64, tid: u32) -> Result<(), i64> {
        self.thread_index(id, tid)?;
        self.error(id, tid, BR_FAILED_REPLY);
        self.stats.failed += 1;
        Ok(())
    }
    pub fn fail_reply(&mut self, id: u64, tid: u32) -> Result<(), i64> {
        let (c, t) = self.thread_index(id, tid)?;
        if let Some(tx) = self.connections[c].threads[t].incoming {
            self.fail_pending(tx, BR_FAILED_REPLY);
        }
        self.fail_transaction(id, tid)
    }
    pub fn has_error(&self, id: u64, tid: u32) -> bool {
        self.index(id).is_ok_and(|c| {
            self.connections[c]
                .threads
                .iter()
                .any(|t| t.tid == tid && t.error.is_some())
        })
    }
    pub fn transact(
        &mut self,
        id: u64,
        tid: u32,
        mut data: Transaction,
        payload: Vec<u8>,
    ) -> Result<(), i64> {
        let (c, t) = self.thread_index(id, tid)?;
        data.validate_payload()?;
        if self.connections[c].threads[t].outgoing.is_some()
            || self.pending.len() >= MAX_TRANSACTIONS
        {
            return Err(11);
        }
        let target = self.connections[c]
            .reference
            .or(self.manager.map(|m| m.connection));
        let Some(manager) = self.manager.filter(|m| Some(m.connection) == target) else {
            self.error(id, tid, BR_DEAD_REPLY);
            self.stats.failed += 1;
            return Ok(());
        };
        if data.target != 0
            || self.connections[c].identity.process
                == self.connections[self.index(manager.connection)?]
                    .identity
                    .process
        {
            return self.fail_transaction(id, tid);
        }
        let receiver = self.index(manager.connection)?;
        Self::reserve_event(&mut self.connections[c].threads[t].events)?;
        Self::reserve_event(&mut self.connections[receiver].process_events)?;
        self.pending.try_reserve(1).map_err(|_| 12)?;
        let tx = self.next_id;
        self.next_id = tx.checked_add(1).ok_or(75)?;
        data.target = manager.pointer;
        data.cookie = manager.cookie;
        data.sender_uid = self.connections[c].identity.uid;
        data.sender_pid = if data.flags & ONE_WAY == 0 {
            self.connections[c].identity.process as u32
        } else {
            0
        };
        let offset = match self.buffer(receiver, data, payload, Some(tx)) {
            Ok(offset) => offset,
            Err(_) => return self.fail_transaction(id, tid),
        };
        let oneway = data.flags & ONE_WAY != 0;
        self.pending.push(Pending {
            id: tx,
            // Accepted oneway work is independent of the sender's lifetime.
            from: if oneway { 0 } else { id },
            tid: if oneway { 0 } else { tid },
            to: manager.connection,
            receiver: None,
            offset,
            oneway,
        });
        if !oneway {
            self.connections[c].threads[t].outgoing = Some(tx);
        }
        self.connections[c].threads[t]
            .events
            .push_back(Event::Code(BR_COMPLETE));
        self.connections[receiver]
            .process_events
            .push_back(Event::Request(tx));
        self.stats.submitted += 1;
        if self.stats.first_sender == 0 {
            self.stats.first_sender = self.connections[c].identity.process;
            self.stats.first_receiver = self.connections[receiver].identity.process;
        }
        self.stats.oneway += u64::from(oneway);
        Ok(())
    }
    pub fn reply(
        &mut self,
        id: u64,
        tid: u32,
        mut data: Transaction,
        payload: Vec<u8>,
    ) -> Result<(), i64> {
        let (c, t) = self.thread_index(id, tid)?;
        data.validate_payload()?;
        if data.flags & ONE_WAY != 0 {
            return Err(22);
        }
        let txid = self.connections[c].threads[t].incoming.ok_or(22)?;
        let p = self
            .pending
            .iter()
            .position(|p| p.id == txid && p.to == id && p.receiver == Some(tid))
            .ok_or(22)?;
        let from = self.pending[p].from;
        let caller_tid = self.pending[p].tid;
        let (caller, ct) = self.thread_index(from, caller_tid)?;
        if self.connections[caller].threads[ct].outgoing != Some(txid) {
            return Err(22);
        }
        Self::reserve_event(&mut self.connections[c].threads[t].events)?;
        Self::reserve_event(&mut self.connections[caller].threads[ct].events)?;
        data.target = 0;
        data.cookie = 0;
        data.sender_pid = 0;
        data.sender_uid = self.connections[c].identity.uid;
        let offset = match self.buffer(caller, data, payload, None) {
            Ok(offset) => offset,
            Err(_) => {
                self.fail_pending(txid, BR_FAILED_REPLY);
                self.connections[c].threads[t]
                    .events
                    .push_back(Event::Code(BR_COMPLETE));
                return Ok(());
            }
        };
        self.pending.remove(p);
        self.connections[c].threads[t].incoming = None;
        self.connections[caller].threads[ct].outgoing = None;
        self.connections[caller].threads[ct]
            .events
            .push_back(Event::Reply(offset));
        self.connections[c].threads[t]
            .events
            .push_back(Event::Code(BR_COMPLETE));
        self.stats.replied += 1;
        Ok(())
    }
    fn fail_pending(&mut self, id: u64, error: u32) {
        let Some(at) = self.pending.iter().position(|p| p.id == id) else {
            return;
        };
        let pending = self.pending.remove(at);
        if !pending.oneway {
            self.error(pending.from, pending.tid, error);
        }
        for c in &mut self.connections {
            c.process_events
                .retain(|e| !matches!(e,Event::Request(tx) if *tx == id));
            for t in &mut c.threads {
                if t.outgoing == Some(id) {
                    t.outgoing = None;
                }
                if t.incoming == Some(id) {
                    t.incoming = None;
                }
            }
            if c.id == pending.to {
                c.buffers
                    .retain(|b| b.transaction_id != Some(id) || b.delivered);
            }
        }
        self.stats.failed += 1;
    }
    pub fn free_buffer(&mut self, id: u64, address: u64) -> Result<(u64, u64), i64> {
        let c = self.index(id)?;
        let mapping = self.connections[c].mapping.ok_or(22)?;
        let offset = address.checked_sub(mapping.base).ok_or(22)?;
        let b = self.connections[c]
            .buffers
            .iter()
            .position(|b| b.offset == offset && b.delivered)
            .ok_or(22)?;
        let buffer = self.connections[c].buffers.remove(b);
        if buffer.asynchronous {
            self.connections[c].asynchronous = None;
        }
        Ok((offset, buffer.length))
    }
    pub fn request_death(&mut self, id: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        let c = self.index(id)?;
        if handle != 0 || self.connections[c].death.is_some() {
            return Err(22);
        }
        let reference = self.connections[c].reference.ok_or(22)?;
        let state = if self.index(reference).is_ok() {
            DeathState::Armed
        } else {
            DeathState::Ready
        };
        self.connections[c].death = Some(Death {
            cookie,
            state,
            clearing: false,
        });
        Ok(())
    }
    pub fn clear_death(&mut self, id: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        let c = self.index(id)?;
        let death = self.connections[c].death.as_mut().ok_or(22)?;
        if handle != 0 || cookie != death.cookie || death.clearing {
            return Err(22);
        }
        death.clearing = true;
        if matches!(death.state, DeathState::Armed | DeathState::Acknowledged) {
            death.state = DeathState::ClearReady;
        }
        Ok(())
    }
    pub fn dead_done(&mut self, id: u64, cookie: u64) -> Result<(), i64> {
        let c = self.index(id)?;
        let death = self.connections[c].death.as_mut().ok_or(22)?;
        if death.cookie != cookie || death.state != DeathState::Delivered {
            return Err(22);
        }
        death.state = if death.clearing {
            DeathState::ClearReady
        } else {
            DeathState::Acknowledged
        };
        Ok(())
    }
    pub fn read(&mut self, id: u64, tid: u32, capacity: usize) -> Result<Option<Delivery>, i64> {
        let (c, t) = self.thread_index(id, tid)?;
        // One transaction record plus BR_NOOP. Never consume work that does
        // not fit, including a short user buffer at the end of a stream.
        if capacity < 72 {
            return Err(22);
        }
        if let Some(error) = self.connections[c].threads[t].error.take() {
            return Ok(Some(Delivery::code(error)));
        }
        let own = self.connections[c].threads[t].events.pop_front();
        let idle = self.connections[c].threads[t].outgoing.is_none()
            && self.connections[c].threads[t].incoming.is_none();
        let event = own.or_else(|| {
            if !idle || self.connections[c].threads[t].looper == 3 {
                return None;
            }
            let at = self.connections[c]
                .process_events
                .iter()
                .position(|e| match e {
                    Event::Request(tx) => {
                        self.pending.iter().find(|p| p.id == *tx).is_some_and(|p| {
                            !p.oneway || self.connections[c].asynchronous.is_none()
                        })
                    }
                    _ => true,
                })?;
            self.connections[c].process_events.remove(at)
        });
        let Some(event) = event else {
            if idle
                && let Some(death) = self.connections[c].death.as_mut()
                && matches!(death.state, DeathState::Ready | DeathState::ClearReady)
            {
                let clear = death.state == DeathState::ClearReady;
                let mut delivery = Delivery::code(if clear {
                    BR_CLEAR_DEATH_DONE
                } else {
                    BR_DEAD_BINDER
                });
                put64(&mut delivery.bytes, 8, death.cookie);
                delivery.length = 16;
                if clear {
                    self.connections[c].death = None;
                    if self.connections[c].strong == 0 && self.connections[c].weak == 0 {
                        self.connections[c].reference = None;
                    }
                } else {
                    death.state = DeathState::Delivered;
                    self.stats.deaths += 1;
                }
                return Ok(Some(delivery));
            }
            return Ok(None);
        };
        let (command, offset, transaction) = match event {
            Event::Code(code) => return Ok(Some(Delivery::code(code))),
            Event::Reply(offset) => (BR_REPLY, offset, None),
            Event::Request(tx) => {
                let p = self.pending.iter().position(|p| p.id == tx).ok_or(22)?;
                let pending = &mut self.pending[p];
                let offset = pending.offset;
                if pending.oneway {
                    self.connections[c].asynchronous = Some(offset);
                    self.pending.remove(p);
                } else {
                    pending.receiver = Some(tid);
                    self.connections[c].threads[t].incoming = Some(tx);
                }
                (BR_TRANSACTION, offset, Some(tx))
            }
        };
        let b = self.connections[c]
            .buffers
            .iter_mut()
            .find(|b| b.offset == offset)
            .ok_or(22)?;
        let mut result = Delivery::code(command);
        result.bytes[8..72].copy_from_slice(&b.transaction.encode());
        result.length = 72;
        result.offset = Some(offset);
        result.payload = b.payload.take();
        result.transaction = transaction;
        result.buffer_id = Some(b.id);
        let connection = &mut self.connections[c];
        if capacity >= 76
            && !connection.process_events.is_empty()
            && !connection.requested
            && !connection.threads.iter().any(|t| t.waiting)
            && connection.threads.iter().filter(|t| t.looper == 2).count()
                < connection.max_threads as usize
        {
            connection.requested = true;
            put32(&mut result.bytes, 72, BR_SPAWN_LOOPER);
            result.length = 76;
        }
        Ok(Some(result))
    }
    pub fn discard_delivery(&mut self, id: u64, delivery: &Delivery) {
        if let Some(tx) = delivery.transaction {
            self.fail_pending(tx, BR_FAILED_REPLY);
        }
        if let Ok(c) = self.index(id)
            && let Some(offset) = delivery.offset
        {
            self.connections[c]
                .buffers
                .retain(|b| Some(b.id) != delivery.buffer_id);
            if self.connections[c].asynchronous == Some(offset) {
                self.connections[c].asynchronous = None;
            }
        }
    }
    pub fn validate_delivery(&self, id: u64, delivery: &Delivery) -> Result<(), i64> {
        let c = self.index(id)?;
        if let Some(buffer) = delivery.buffer_id
            && !self.connections[c]
                .buffers
                .iter()
                .any(|b| b.id == buffer && !b.delivered)
        {
            return Err(14);
        }
        Ok(())
    }
    pub fn finish_delivery(&mut self, id: u64, delivery: &Delivery) -> Result<(), i64> {
        self.validate_delivery(id, delivery)?;
        let c = self.index(id)?;
        if let Some(buffer) = delivery.buffer_id {
            self.connections[c]
                .buffers
                .iter_mut()
                .find(|b| b.id == buffer)
                .unwrap()
                .delivered = true;
        }
        Ok(())
    }
    pub fn thread_exit(&mut self, id: u64, tid: u32) {
        while let Some(tx) = self
            .pending
            .iter()
            .find(|p| p.from == id && p.tid == tid || p.to == id && p.receiver == Some(tid))
            .map(|p| p.id)
        {
            self.fail_pending(tx, BR_DEAD_REPLY);
        }
        if let Ok(c) = self.index(id)
            && let Some(t) = self.connections[c]
                .threads
                .iter()
                .position(|t| t.tid == tid)
        {
            let thread = self.connections[c].threads.remove(t);
            for event in thread.events {
                if let Event::Reply(offset) = event {
                    self.connections[c]
                        .buffers
                        .retain(|b| b.offset != offset || b.delivered);
                }
            }
        }
    }
    pub fn process_thread_exit(&mut self, owner: u64, tid: u32) {
        let mut ids = [0; MAX_CONNECTIONS];
        for (slot, c) in ids.iter_mut().zip(
            self.connections
                .iter()
                .filter(|c| c.identity.process == owner),
        ) {
            *slot = c.id;
        }
        for id in ids {
            if id != 0 {
                self.thread_exit(id, tid);
            }
        }
    }
    pub fn unmap_inode(&mut self, inode: u64) {
        if let Some(id) = self
            .connections
            .iter()
            .find(|c| c.mapping.is_some_and(|m| m.inode == inode))
            .map(|c| c.id)
        {
            while let Some(tx) = self.pending.iter().find(|p| p.to == id).map(|p| p.id) {
                self.fail_pending(tx, BR_FAILED_REPLY);
            }
            let c = self.index(id).unwrap();
            self.connections[c].mapping = None;
            self.connections[c].buffers.clear();
            self.connections[c].asynchronous = None;
        }
    }
    pub fn close(&mut self, id: u64) {
        let Ok(c) = self.index(id) else {
            return;
        };
        self.connections.remove(c);
        while let Some(tx) = self
            .pending
            .iter()
            .find(|p| p.from == id || p.to == id)
            .map(|p| p.id)
        {
            self.fail_pending(tx, BR_DEAD_REPLY);
        }
        if self.manager.is_some_and(|m| m.connection == id) {
            self.manager = None;
        }
        for c in &mut self.connections {
            if c.reference == Some(id)
                && let Some(death) = c.death.as_mut()
                && death.state == DeathState::Armed
            {
                death.state = DeathState::Ready;
            }
        }
        self.stats.closed += 1;
    }
    pub fn stats(&self) -> Stats {
        Stats {
            connections: self.connections.len(),
            transactions: self.pending.len(),
            buffers: self.connections.iter().map(|c| c.buffers.len()).sum(),
            ..self.stats
        }
    }
}

struct Device {
    busy: AtomicBool,
    router: UnsafeCell<Router>,
}
unsafe impl Sync for Device {}
static DEVICE: Device = Device {
    busy: AtomicBool::new(false),
    router: UnsafeCell::new(Router::new()),
};
pub fn with<R>(operation: impl FnOnce(&mut Router) -> R) -> R {
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
    operation(unsafe { &mut *DEVICE.router.get() })
}
