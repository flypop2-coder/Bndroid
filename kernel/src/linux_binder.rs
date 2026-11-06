//! Binder transaction ownership, independent of user copies and page tables.
//! Byte and Binder-object transport with bounded, connection-scoped handles.
//! Flat FD transfer is receiver-owned; SG and Android service policy remain open.
#[cfg(test)]
mod fd_tests;
#[cfg(test)]
mod object_tests;
mod objects;
mod parcel;
pub use parcel::{FileTransfer, Parcel};
#[cfg(test)]
mod tests;
pub mod wire;
use alloc::{collections::VecDeque, vec::Vec};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
use objects::{Control, Hold, Objects};
use wire::*;

const MAX_CONNECTIONS: usize = 16;
const MAX_THREADS: usize = 32;
const MAX_TRANSACTIONS: usize = 128;
const MAX_BUFFERS: usize = 128;
const MAX_EVENTS: usize = 128;
const MAX_CALL_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum Domain {
    Binder,
    Hardware,
    Vendor,
}
impl Domain {
    pub const ALL: [Self; 3] = [Self::Binder, Self::Hardware, Self::Vendor];
    pub const fn path(self) -> &'static str {
        match self {
            Self::Binder => "/dev/binder",
            Self::Hardware => "/dev/hwbinder",
            Self::Vendor => "/dev/vndbinder",
        }
    }
}

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
    pub last_transfer_error: i64,
    pub opened: u64,
    pub closed: u64,
    pub submitted: u64,
    pub replied: u64,
    pub oneway: u64,
    pub failed: u64,
    pub deaths: u64,
    pub flushes: u64,
    pub flush_returns: u64,
    pub nodes: usize,
    pub references: usize,
    pub objects_translated: u64,
    pub reference_events: u64,
    pub nested: u64,
    pub max_call_depth: usize,
    pub call_frames: usize,
    pub fds_submitted: u64,
    pub fds_delivered: u64,
    pub fd_install_failures: u64,
    pub pending_fds: usize,
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
    need_return: bool,
    stack: Vec<CallFrame>,
    error: Option<u32>,
    events: VecDeque<Event>,
}
#[derive(Clone, Copy)]
struct CallFrame {
    transaction: u64,
    incoming: bool,
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
    node: Option<u64>,
    holds: Vec<Hold>,
    files: Vec<FileTransfer>,
    file_count: usize,
}
struct Connection {
    id: u64,
    domain: Domain,
    identity: Identity,
    mapping: Option<Mapping>,
    mapped_once: bool,
    threads: Vec<Thread>,
    process_events: VecDeque<Event>,
    buffers: Vec<Buffer>,
    max_threads: u32,
    requested: bool,
}
struct Pending {
    id: u64,
    from: u64,
    tid: u32,
    to: u64,
    receiver: Option<u32>,
    offset: u64,
    oneway: bool,
    node: u64,
    parent: Option<u64>,
    accepts_files: bool,
}
#[derive(Clone, Copy)]
struct Manager {
    connection: u64,
    node: u64,
}

pub struct Delivery {
    pub bytes: [u8; 80],
    pub length: usize,
    pub offset: Option<u64>,
    pub payload: Option<Vec<u8>>,
    pub files: Vec<FileTransfer>,
    transaction: Option<u64>,
    buffer_id: Option<u64>,
    control: Option<Control>,
    flushed: bool,
}
impl Delivery {
    pub fn failed_reply() -> Self {
        Self::code(BR_FAILED_REPLY)
    }
    pub fn is_reply(&self) -> bool {
        u32_at(&self.bytes, 4) == BR_REPLY
    }
    fn control(control: Control, command: u32, first: u64, second: u64) -> Self {
        let mut delivery = Self::code(command);
        delivery.control = Some(control);
        put64(&mut delivery.bytes, 8, first);
        delivery.length = 16;
        if matches!(control, Control::Node { .. }) {
            put64(&mut delivery.bytes, 16, second);
            delivery.length = 24;
        }
        delivery
    }
    fn code(code: u32) -> Self {
        let mut bytes = [0; 80];
        put32(&mut bytes, 0, BR_NOOP);
        put32(&mut bytes, 4, code);
        Self {
            bytes,
            length: 8,
            offset: None,
            payload: None,
            files: Vec::new(),
            transaction: None,
            buffer_id: None,
            control: None,
            flushed: false,
        }
    }
}

/// A small immutable snapshot keeps the epoll lock out of Binder's lock order.
/// Masks are computed for the original waiting process/thread, never the timer's
/// current address space. An unavailable/private connection reports an error.
pub struct Readiness {
    pub revision: u64,
    entries: [(u64, u32); MAX_CONNECTIONS],
}
impl Readiness {
    pub fn mask(&self, connection: u64) -> u32 {
        self.entries
            .iter()
            .find(|e| e.0 == connection)
            .map(|e| e.1)
            .unwrap_or(0x18)
    }
}
pub struct Router {
    connections: Vec<Connection>,
    pending: Vec<Pending>,
    next_id: u64,
    managers: [Option<Manager>; 3],
    manager_uids: [Option<u32>; 3],
    stats: Stats,
    objects: Objects,
    revision: u64,
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
            objects: Objects::new(),
            revision: 1,
            managers: [None; 3],
            manager_uids: [None; 3],
            stats: Stats {
                last_transfer_error: 0,
                opened: 0,
                closed: 0,
                submitted: 0,
                replied: 0,
                oneway: 0,
                failed: 0,
                deaths: 0,
                flushes: 0,
                flush_returns: 0,
                nodes: 0,
                references: 0,
                objects_translated: 0,
                reference_events: 0,
                nested: 0,
                max_call_depth: 0,
                call_frames: 0,
                fds_submitted: 0,
                fds_delivered: 0,
                fd_install_failures: 0,
                pending_fds: 0,
                connections: 0,
                transactions: 0,
                buffers: 0,
                first_sender: 0,
                first_receiver: 0,
            },
        }
    }
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1).max(1);
    }
    pub fn register_poll(&mut self, id: u64, owner: u64, tid: u32) -> Result<(), i64> {
        self.authenticate(id, owner)?;
        self.thread_index(id, tid)?;
        self.changed();
        Ok(())
    }
    pub fn readiness(&self, owner: u64, tid: u32) -> Readiness {
        let mut snapshot = Readiness {
            revision: self.revision,
            entries: [(0, 0); MAX_CONNECTIONS],
        };
        for (slot, c) in snapshot.entries.iter_mut().zip(&self.connections) {
            *slot = (
                c.id,
                if c.identity.process == owner {
                    self.poll_mask(c, tid)
                } else {
                    8
                },
            );
        }
        snapshot
    }
    fn request_ready(&self, c: &Connection, event: &Event) -> bool {
        match event {
            Event::Request(tx) => self.pending.iter().find(|p| p.id == *tx).is_some_and(|p| {
                !p.oneway
                    || !c
                        .buffers
                        .iter()
                        .any(|b| b.asynchronous && b.node == Some(p.node) && b.payload.is_none())
            }),
            _ => true,
        }
    }
    fn poll_mask(&self, c: &Connection, tid: u32) -> u32 {
        let Some(t) = c.threads.iter().find(|t| t.tid == tid) else {
            return 0;
        };
        let control = self.objects.ready(c.id, false);
        if self.objects.unannounced(c.id) {
            return u32::from(control);
        }
        let process = t.stack.is_empty() && matches!(t.looper, 1 | 2);
        u32::from(
            control
                || t.need_return
                || t.error.is_some()
                || !t.events.is_empty()
                || (process
                    && (self.objects.ready(c.id, true)
                        || c.process_events.iter().any(|e| self.request_ready(c, e)))),
        )
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
            need_return: false,
            stack: Vec::new(),
            error: None,
            events: VecDeque::new(),
        });
        Ok((c, threads.len() - 1))
    }
    pub fn open(&mut self, identity: Identity) -> Result<u64, i64> {
        self.open_in(identity, Domain::Binder)
    }
    pub fn open_in(&mut self, identity: Identity, domain: Domain) -> Result<u64, i64> {
        self.changed();
        if identity.process == 0 || self.connections.len() >= MAX_CONNECTIONS {
            return Err(24);
        }
        self.connections.try_reserve(1).map_err(|_| 12)?;
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(75)?;
        self.connections.push(Connection {
            id,
            domain,
            identity,
            mapping: None,
            mapped_once: false,
            threads: Vec::new(),
            process_events: VecDeque::new(),
            buffers: Vec::new(),
            max_threads: 0,
            requested: false,
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
        self.changed();
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
        self.set_manager_with_flags(id, pointer, cookie, 0)
    }
    pub fn set_manager_with_flags(
        &mut self,
        id: u64,
        pointer: u64,
        cookie: u64,
        flags: u32,
    ) -> Result<(), i64> {
        self.changed();
        let c = self.index(id)?;
        let domain = self.connections[c].domain as usize;
        if self.managers[domain].is_some() {
            return Err(16);
        }
        let uid = self.connections[c].identity.uid;
        if self.manager_uids[domain].is_some_and(|prior| prior != uid) {
            return Err(1);
        }
        let node = self.objects.node(id, pointer, cookie, flags, true)?;
        self.manager_uids[domain] = Some(uid);
        self.managers[domain] = Some(Manager {
            connection: id,
            node,
        });
        Ok(())
    }
    pub fn set_max_threads(&mut self, id: u64, maximum: u32) -> Result<(), i64> {
        self.changed();
        if maximum as usize > MAX_THREADS {
            return Err(22);
        }
        let c = self.index(id)?;
        self.connections[c].max_threads = maximum;
        Ok(())
    }
    pub fn looper(&mut self, id: u64, tid: u32, command: u32) -> Result<(), i64> {
        self.changed();
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
    /// Linux filp_close flushes Binder even while dup/in-flight references
    /// retain the open file. Idle loopers must return so libbinder can stop.
    pub fn flush(&mut self, id: u64) {
        self.changed();
        if let Ok(c) = self.index(id) {
            for thread in &mut self.connections[c].threads {
                thread.need_return = true;
            }
            self.stats.flushes += 1;
        }
    }
    pub fn reference(&mut self, id: u64, handle: u32, command: u32) -> Result<(), i64> {
        self.changed();
        let c = self.index(id)?;
        let manager = self.managers[self.connections[c].domain as usize].map(|m| m.node);
        self.objects.change(id, handle, command, manager)
    }
    pub fn acknowledge(
        &mut self,
        id: u64,
        pointer: u64,
        cookie: u64,
        command: u32,
    ) -> Result<(), i64> {
        self.changed();
        self.index(id)?;
        self.objects.acknowledge(id, pointer, cookie, command)
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
        from: u64,
        node: Option<u64>,
        mut transaction: Transaction,
        mut payload: Parcel,
        transaction_id: Option<u64>,
    ) -> Result<u64, i64> {
        let connection = &mut self.connections[c];
        let mapping = connection.mapping.ok_or(12)?;
        if connection.buffers.len() >= MAX_BUFFERS
            || payload.bytes.len() != transaction.payload_length()?
        {
            return Err(12);
        }
        let length = (payload.bytes.len().max(1) as u64 + 7) & !7;
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
        transaction.offsets = transaction.data + ((transaction.data_size + 7) & !7);
        let holds = self
            .objects
            .translate(from, connection.id, transaction, &mut payload, node)?;
        self.stats.fds_submitted += payload.files.len() as u64;
        let at = connection.buffers.partition_point(|b| b.offset < offset);
        connection.buffers.insert(
            at,
            Buffer {
                id,
                transaction_id,
                offset,
                length,
                payload: Some(payload.bytes),
                file_count: payload.files.len(),
                files: payload.files,
                transaction,
                delivered: false,
                asynchronous: transaction.flags & ONE_WAY != 0,
                node,
                holds,
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
        self.changed();
        self.thread_index(id, tid)?;
        self.error(id, tid, BR_FAILED_REPLY);
        self.stats.failed += 1;
        Ok(())
    }
    pub fn fail_reply(&mut self, id: u64, tid: u32) -> Result<(), i64> {
        self.changed();
        let (c, t) = self.thread_index(id, tid)?;
        if let Some(frame) = self.connections[c].threads[t].stack.last().copied()
            && frame.incoming
        {
            let queued = Self::reserve_event(&mut self.connections[c].threads[t].events).is_ok();
            self.fail_pending(frame.transaction, BR_FAILED_REPLY);
            if queued {
                self.connections[c].threads[t]
                    .events
                    .push_back(Event::Code(BR_COMPLETE));
            } else {
                // A full queue or allocation failure must not leave the
                // caller blocked on an already rejected reply.
                self.error(id, tid, BR_COMPLETE);
            }
            return Ok(());
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
        data: Transaction,
        payload: Vec<u8>,
    ) -> Result<(), i64> {
        self.transact_parcel(id, tid, data, payload.into())
    }
    pub fn transact_parcel(
        &mut self,
        id: u64,
        tid: u32,
        mut data: Transaction,
        payload: Parcel,
    ) -> Result<(), i64> {
        self.changed();
        let (c, t) = self.thread_index(id, tid)?;
        data.validate_payload()?;
        self.stats.last_transfer_error = 0;
        let stack = &mut self.connections[c].threads[t].stack;
        if stack.last().is_some_and(|f| !f.incoming)
            || stack.len() >= MAX_CALL_DEPTH
            || self.pending.len() >= MAX_TRANSACTIONS
        {
            return Err(11);
        }
        stack.try_reserve(1).map_err(|_| 12)?;
        let parent = stack.last().map(|f| f.transaction);
        // BC_TRANSACTION uses the u32 handle member of the LP64 union. The
        // upper four bytes are C padding, not part of the handle authority.
        let handle = data.target as u32;
        let manager = self.managers[self.connections[c].domain as usize];
        let node = if handle == 0 {
            self.objects
                .lookup(id, 0, false)
                .ok()
                .or(manager.map(|m| m.node))
        } else {
            self.objects.lookup(id, handle, true).ok()
        };
        let Some(node) = node else {
            return if handle == 0 {
                self.error(id, tid, BR_DEAD_REPLY);
                self.stats.failed += 1;
                Ok(())
            } else {
                self.fail_transaction(id, tid)
            };
        };
        let Ok((target, pointer, cookie)) = self.objects.target(node) else {
            self.error(id, tid, BR_DEAD_REPLY);
            self.stats.failed += 1;
            return Ok(());
        };
        if !payload.files.is_empty() && !self.objects.accepts_files(node)? {
            return Err(1);
        }
        let receiver = self.index(target)?;
        if self.connections[c].identity.process == self.connections[receiver].identity.process {
            return self.fail_transaction(id, tid);
        }
        Self::reserve_event(&mut self.connections[c].threads[t].events)?;
        let mut ancestor = parent;
        let mut receiver_thread = None;
        if data.flags & ONE_WAY == 0 {
            for _ in 0..MAX_CALL_DEPTH {
                let Some(ancestor_id) = ancestor else {
                    break;
                };
                let previous = self
                    .pending
                    .iter()
                    .find(|p| p.id == ancestor_id)
                    .ok_or(22)?;
                let next = previous.parent;
                if previous.from == target && receiver_thread.is_none() {
                    receiver_thread = Some(self.thread_index(target, previous.tid)?.1);
                }
                ancestor = next;
            }
            if ancestor.is_some() {
                return Err(11);
            }
        }
        if let Some(rt) = receiver_thread {
            let thread = &mut self.connections[receiver].threads[rt];
            if thread.stack.len() >= MAX_CALL_DEPTH {
                return Err(11);
            }
            Self::reserve_event(&mut thread.events)?;
        } else {
            Self::reserve_event(&mut self.connections[receiver].process_events)?;
        }
        self.pending.try_reserve(1).map_err(|_| 12)?;
        let tx = self.next_id;
        self.next_id = tx.checked_add(1).ok_or(75)?;
        data.target = pointer;
        data.cookie = cookie;
        data.sender_uid = self.connections[c].identity.uid;
        data.sender_pid = if data.flags & ONE_WAY == 0 {
            self.connections[c].identity.process as u32
        } else {
            0
        };
        let offset = match self.buffer(receiver, id, Some(node), data, payload, Some(tx)) {
            Ok(offset) => offset,
            Err(error) => {
                self.stats.last_transfer_error = error;
                return self.fail_transaction(id, tid);
            }
        };
        let oneway = data.flags & ONE_WAY != 0;
        self.pending.push(Pending {
            id: tx,
            // Accepted oneway work is independent of the sender's lifetime.
            from: if oneway { 0 } else { id },
            tid: if oneway { 0 } else { tid },
            to: target,
            receiver: None,
            offset,
            oneway,
            node,
            parent: if oneway { None } else { parent },
            accepts_files: data.flags & ACCEPT_FDS != 0,
        });
        if !oneway {
            self.connections[c].threads[t].stack.push(CallFrame {
                transaction: tx,
                incoming: false,
            });
            self.stats.max_call_depth = self
                .stats
                .max_call_depth
                .max(self.connections[c].threads[t].stack.len());
        }
        self.connections[c].threads[t]
            .events
            .push_back(Event::Code(BR_COMPLETE));
        if let Some(rt) = receiver_thread {
            self.connections[receiver].threads[rt]
                .events
                .push_back(Event::Request(tx));
            self.stats.nested += 1;
        } else {
            self.connections[receiver]
                .process_events
                .push_back(Event::Request(tx));
        }
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
        data: Transaction,
        payload: Vec<u8>,
    ) -> Result<(), i64> {
        self.reply_parcel(id, tid, data, payload.into())
    }
    pub fn reply_parcel(
        &mut self,
        id: u64,
        tid: u32,
        mut data: Transaction,
        payload: Parcel,
    ) -> Result<(), i64> {
        self.changed();
        let (c, t) = self.thread_index(id, tid)?;
        data.validate_payload()?;
        self.stats.last_transfer_error = 0;
        if data.flags & ONE_WAY != 0 {
            return Err(22);
        }
        let frame = self.connections[c].threads[t]
            .stack
            .last()
            .filter(|f| f.incoming)
            .ok_or(22)?;
        let txid = frame.transaction;
        let p = self
            .pending
            .iter()
            .position(|p| p.id == txid && p.to == id && p.receiver == Some(tid))
            .ok_or(22)?;
        if !payload.files.is_empty() && !self.pending[p].accepts_files {
            return Err(1);
        }
        let from = self.pending[p].from;
        let caller_tid = self.pending[p].tid;
        let (caller, ct) = self.thread_index(from, caller_tid)?;
        if !self.connections[caller].threads[ct]
            .stack
            .last()
            .is_some_and(|f| !f.incoming && f.transaction == txid)
        {
            return Err(22);
        }
        Self::reserve_event(&mut self.connections[c].threads[t].events)?;
        Self::reserve_event(&mut self.connections[caller].threads[ct].events)?;
        data.target = 0;
        data.cookie = 0;
        data.sender_pid = 0;
        data.sender_uid = self.connections[c].identity.uid;
        let offset = match self.buffer(caller, id, None, data, payload, None) {
            Ok(offset) => offset,
            Err(error) => {
                self.stats.last_transfer_error = error;
                self.fail_pending(txid, BR_FAILED_REPLY);
                self.connections[c].threads[t]
                    .events
                    .push_back(Event::Code(BR_COMPLETE));
                return Ok(());
            }
        };
        self.pending.remove(p);
        self.connections[c].threads[t].stack.pop();
        self.connections[caller].threads[ct].stack.pop();
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
        // Unwind descendants before their parents. Fixed scratch storage keeps
        // cancellation allocation-free and bounds malicious call chains.
        let mut doomed = [0; MAX_TRANSACTIONS];
        doomed[0] = id;
        let mut count = 1;
        loop {
            let before = count;
            for p in &self.pending {
                if p.parent
                    .is_some_and(|parent| doomed[..count].contains(&parent))
                    && !doomed[..count].contains(&p.id)
                {
                    doomed[count] = p.id;
                    count += 1;
                }
            }
            if count == before {
                break;
            }
        }
        for &tx in doomed[..count].iter().rev() {
            self.fail_one_pending(tx, error);
        }
    }
    fn fail_one_pending(&mut self, id: u64, error: u32) {
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
                t.stack.retain(|f| f.transaction != id);
                t.events
                    .retain(|e| !matches!(e, Event::Request(tx) if *tx == id));
            }
            if c.id == pending.to {
                c.buffers.retain(|b| {
                    let keep = b.transaction_id != Some(id) || b.delivered;
                    if !keep {
                        self.objects.release(&b.holds);
                    }
                    keep
                });
            }
        }
        self.stats.failed += 1;
    }
    pub fn free_buffer(&mut self, id: u64, address: u64) -> Result<(u64, u64), i64> {
        self.changed();
        let c = self.index(id)?;
        let mapping = self.connections[c].mapping.ok_or(22)?;
        let offset = address.checked_sub(mapping.base).ok_or(22)?;
        let b = self.connections[c]
            .buffers
            .iter()
            .position(|b| b.offset == offset && b.delivered)
            .ok_or(22)?;
        let buffer = self.connections[c].buffers.remove(b);
        self.objects.release(&buffer.holds);
        Ok((offset, buffer.length))
    }
    pub fn request_death(&mut self, id: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        self.changed();
        self.index(id)?;
        self.objects.request_death(id, handle, cookie)
    }
    pub fn clear_death(&mut self, id: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        self.changed();
        self.index(id)?;
        self.objects.clear_death(id, handle, cookie)
    }
    pub fn dead_done(&mut self, id: u64, cookie: u64) -> Result<(), i64> {
        self.changed();
        self.index(id)?;
        self.objects.dead_done(id, cookie)
    }
    pub fn read(&mut self, id: u64, tid: u32, capacity: usize) -> Result<Option<Delivery>, i64> {
        let flushed = self.index(id).is_ok_and(|c| {
            self.connections[c]
                .threads
                .iter()
                .any(|t| t.tid == tid && t.need_return)
        });
        let mut result = self.read_inner(id, tid, capacity);
        if let Ok(Some(delivery)) = &mut result {
            delivery.flushed = flushed;
            self.changed();
        }
        result
    }
    fn read_inner(&mut self, id: u64, tid: u32, capacity: usize) -> Result<Option<Delivery>, i64> {
        let (c, t) = self.thread_index(id, tid)?;
        // One transaction record plus BR_NOOP. Never consume work that does
        // not fit, including a short user buffer at the end of a stream.
        if capacity < 72 {
            return Err(22);
        }
        let flushed = core::mem::take(&mut self.connections[c].threads[t].need_return);
        // Retain exported objects in userspace before BR_TRANSACTION_COMPLETE
        // permits the sender to destroy its Parcel (especially for oneway).
        if self.objects.unannounced(id) {
            if let Some((control, command, first, second)) = self.objects.prepare(id, false) {
                return Ok(Some(Delivery::control(control, command, first, second)));
            }
            self.connections[c].threads[t].need_return |= flushed;
            // Another thread may own the retain. Kernel delivery alone does
            // not prove that userspace has incremented the reference yet.
            return Ok(None);
        }
        if let Some(error) = self.connections[c].threads[t].error.take() {
            return Ok(Some(Delivery::code(error)));
        }
        if let Some((control, command, first, second)) = self.objects.prepare(id, false) {
            return Ok(Some(Delivery::control(control, command, first, second)));
        }
        let own = self.connections[c].threads[t].events.pop_front();
        let idle = self.connections[c].threads[t].stack.is_empty();
        let event = own.or_else(|| {
            if !idle || self.connections[c].threads[t].looper == 3 {
                return None;
            }
            let at = self.connections[c]
                .process_events
                .iter()
                .position(|e| self.request_ready(&self.connections[c], e))?;
            self.connections[c].process_events.remove(at)
        });
        let Some(event) = event else {
            if let Some((control, command, first, second)) = self.objects.prepare(id, idle) {
                return Ok(Some(Delivery::control(control, command, first, second)));
            }
            if flushed {
                let mut delivery = Delivery::code(BR_NOOP);
                delivery.length = 4;
                self.stats.flush_returns += 1;
                return Ok(Some(delivery));
            }
            return Ok(None);
        };
        let (command, offset, transaction) = match event {
            Event::Code(code) => return Ok(Some(Delivery::code(code))),
            Event::Reply(offset) => (BR_REPLY, offset, None),
            Event::Request(tx) => {
                let p = self.pending.iter().position(|p| p.id == tx).ok_or(22)?;
                if !self.pending[p].oneway {
                    let stack = &mut self.connections[c].threads[t].stack;
                    if stack.len() >= MAX_CALL_DEPTH || stack.try_reserve(1).is_err() {
                        self.fail_pending(tx, BR_FAILED_REPLY);
                        return Err(12);
                    }
                    stack.push(CallFrame {
                        transaction: tx,
                        incoming: true,
                    });
                    self.stats.max_call_depth = self.stats.max_call_depth.max(stack.len());
                }
                let pending = &mut self.pending[p];
                let offset = pending.offset;
                if pending.oneway {
                    self.pending.remove(p);
                } else {
                    pending.receiver = Some(tid);
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
        result.files = core::mem::take(&mut b.files);
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
        self.changed();
        if let Some(control) = delivery.control {
            self.objects.finish(control, false);
        }
        if let Some(tx) = delivery.transaction {
            self.fail_pending(tx, BR_FAILED_REPLY);
        }
        if let Ok(c) = self.index(id)
            && delivery.offset.is_some()
        {
            self.connections[c].buffers.retain(|b| {
                let keep = Some(b.id) != delivery.buffer_id;
                if !keep {
                    self.objects.release(&b.holds);
                }
                keep
            });
        }
    }
    pub fn validate_delivery(&self, id: u64, delivery: &Delivery) -> Result<(), i64> {
        let c = self.index(id)?;
        if let Some(control) = delivery.control {
            self.objects.validate(control)?;
        }
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
    pub fn reject_files(&mut self, id: u64, tid: u32, delivery: &Delivery) {
        if delivery.flushed
            && !delivery.is_reply()
            && let Ok((c, t)) = self.thread_index(id, tid)
        {
            self.connections[c].threads[t].need_return = true;
        }
        self.stats.fd_install_failures += 1;
        if !delivery
            .transaction
            .is_some_and(|id| self.pending.iter().any(|p| p.id == id))
        {
            self.stats.failed += 1;
        }
        self.discard_delivery(id, delivery);
    }
    pub fn finish_delivery(&mut self, id: u64, delivery: &Delivery) -> Result<(), i64> {
        self.changed();
        self.validate_delivery(id, delivery)?;
        if let Some(control) = delivery.control {
            self.objects.finish(control, true);
            if matches!(control, Control::Death { clear: false, .. }) {
                self.stats.deaths += 1;
            }
        }
        let c = self.index(id)?;
        if let Some(buffer) = delivery.buffer_id {
            let buffer = self.connections[c]
                .buffers
                .iter_mut()
                .find(|b| b.id == buffer)
                .unwrap();
            buffer.delivered = true;
            buffer.file_count = 0;
            self.stats.fds_delivered += delivery.files.len() as u64;
        }
        Ok(())
    }
    pub fn thread_exit(&mut self, id: u64, tid: u32) {
        self.changed();
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
                    self.connections[c].buffers.retain(|b| {
                        let keep = b.offset != offset || b.delivered;
                        if !keep {
                            self.objects.release(&b.holds);
                        }
                        keep
                    });
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
        self.changed();
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
            for b in self.connections[c].buffers.drain(..) {
                self.objects.release(&b.holds);
            }
            for t in &mut self.connections[c].threads {
                t.events.retain(|e| !matches!(e, Event::Reply(_)));
            }
        }
    }
    pub fn close(&mut self, id: u64) {
        self.changed();
        let Ok(c) = self.index(id) else {
            return;
        };
        let connection = self.connections.remove(c);
        for b in &connection.buffers {
            self.objects.release(&b.holds);
        }
        self.objects.close(id);
        while let Some(tx) = self
            .pending
            .iter()
            .find(|p| p.from == id || p.to == id)
            .map(|p| p.id)
        {
            self.fail_pending(tx, BR_DEAD_REPLY);
        }
        for manager in &mut self.managers {
            if manager.is_some_and(|m| m.connection == id) {
                *manager = None;
            }
        }
        self.stats.closed += 1;
    }
    pub fn stats(&self) -> Stats {
        Stats {
            nodes: self.objects.counts().0,
            references: self.objects.counts().1,
            objects_translated: self.objects.translated,
            reference_events: self.objects.handshakes,
            call_frames: self
                .connections
                .iter()
                .flat_map(|c| &c.threads)
                .map(|t| t.stack.len())
                .sum(),
            pending_fds: self
                .connections
                .iter()
                .flat_map(|c| &c.buffers)
                .map(|b| b.file_count)
                .sum(),
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
