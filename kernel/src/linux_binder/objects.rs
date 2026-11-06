//! Connection-scoped Binder handles and owner reference handshakes. Buffer
//! holds are distinct from userspace references, including during failed reads.
use super::wire::*;
use alloc::vec::Vec;

const MAX_NODES: usize = 512;
const MAX_REFS: usize = 4096;
const MAX_REFS_PER_CONNECTION: usize = 256;

struct Node {
    id: u64,
    owner: u64,
    alive: bool,
    manager: bool,
    pointer: u64,
    cookie: u64,
    flags: u32,
    strong: u32,
    weak: u32,
    has_strong: bool,
    has_weak: bool,
    pending_strong: bool,
    pending_weak: bool,
    reserved: Option<u32>,
}
#[derive(Clone, Copy, PartialEq)]
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
    reserved: bool,
}
struct Reference {
    owner: u64,
    handle: u32,
    node: u64,
    strong: u32,
    weak: u32,
    buffer_strong: u32,
    buffer_weak: u32,
    death: Option<Death>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Hold {
    node: u64,
    recipient: Option<u64>,
    strong: bool,
}
#[derive(Clone, Copy)]
pub(super) enum Control {
    Node {
        node: u64,
        command: u32,
    },
    Death {
        owner: u64,
        handle: u32,
        clear: bool,
    },
}
pub(super) struct Objects {
    nodes: Vec<Node>,
    references: Vec<Reference>,
    next_node: u64,
    next_handle: u32,
    pub translated: u64,
    pub handshakes: u64,
}
impl Objects {
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            references: Vec::new(),
            next_node: 1,
            next_handle: 1,
            translated: 0,
            handshakes: 0,
        }
    }
    fn node_index(&self, node: u64) -> Result<usize, i64> {
        self.nodes.iter().position(|n| n.id == node).ok_or(22)
    }
    fn ref_index(&self, owner: u64, handle: u32) -> Result<usize, i64> {
        self.references
            .iter()
            .position(|r| r.owner == owner && r.handle == handle)
            .ok_or(22)
    }
    pub fn node(
        &mut self,
        owner: u64,
        pointer: u64,
        cookie: u64,
        flags: u32,
        manager: bool,
    ) -> Result<u64, i64> {
        // FD acceptance is metadata; FD and SG objects remain unsupported.
        // Security contexts and RT inheritance require a separate policy.
        if flags & !0x1ff != 0 {
            return Err(95);
        }
        if let Some(n) = self
            .nodes
            .iter_mut()
            .find(|n| n.owner == owner && n.alive && n.pointer == pointer)
        {
            if n.cookie != cookie || n.flags != flags {
                return Err(22);
            }
            n.manager |= manager;
            return Ok(n.id);
        }
        if self.nodes.len() >= MAX_NODES {
            return Err(28);
        }
        self.nodes.try_reserve(1).map_err(|_| 12)?;
        let id = self.next_node;
        self.next_node = id.checked_add(1).ok_or(75)?;
        self.nodes.push(Node {
            id,
            owner,
            alive: true,
            manager,
            pointer,
            cookie,
            flags,
            strong: 0,
            weak: 0,
            has_strong: manager,
            has_weak: manager,
            pending_strong: false,
            pending_weak: false,
            reserved: None,
        });
        Ok(id)
    }
    pub fn target(&self, node: u64) -> Result<(u64, u64, u64), i64> {
        let n = &self.nodes[self.node_index(node)?];
        if !n.alive {
            return Err(32);
        }
        Ok((n.owner, n.pointer, n.cookie))
    }
    pub fn lookup(&self, owner: u64, handle: u32, strong: bool) -> Result<u64, i64> {
        let r = &self.references[self.ref_index(owner, handle)?];
        if strong && r.strong == 0 && r.buffer_strong == 0 {
            return Err(22);
        }
        Ok(r.node)
    }
    fn reference(&mut self, owner: u64, node: u64, zero: bool) -> Result<usize, i64> {
        if let Some(i) = self
            .references
            .iter()
            .position(|r| r.owner == owner && r.node == node)
        {
            return Ok(i);
        }
        if self.references.len() >= MAX_REFS
            || self.references.iter().filter(|r| r.owner == owner).count()
                >= MAX_REFS_PER_CONNECTION
        {
            return Err(28);
        }
        self.references.try_reserve(1).map_err(|_| 12)?;
        let handle = if zero {
            if self.ref_index(owner, 0).is_ok() {
                return Err(22);
            }
            0
        } else {
            let handle = self.next_handle;
            self.next_handle = handle.checked_add(1).ok_or(75)?;
            handle
        };
        self.references.push(Reference {
            owner,
            handle,
            node,
            strong: 0,
            weak: 0,
            buffer_strong: 0,
            buffer_weak: 0,
            death: None,
        });
        Ok(self.references.len() - 1)
    }
    pub fn change(
        &mut self,
        owner: u64,
        handle: u32,
        command: u32,
        manager: Option<u64>,
    ) -> Result<(), i64> {
        let i = match self.ref_index(owner, handle) {
            Ok(i) => i,
            Err(_) if handle == 0 && matches!(command, BC_INCREFS | BC_ACQUIRE) => {
                self.reference(owner, manager.ok_or(22)?, true)?
            }
            Err(e) => return Err(e),
        };
        let r = &self.references[i];
        if command == BC_ACQUIRE && r.strong == 0 && r.buffer_strong == 0 {
            let node = self.node_index(r.node)?;
            if !self.nodes[node].manager
                && !self.references.iter().any(|other| {
                    other.node == r.node && (other.strong > 0 || other.buffer_strong > 0)
                })
            {
                return Err(22);
            }
        }
        let r = &mut self.references[i];
        let count = match command {
            BC_INCREFS | BC_DECREFS => &mut r.weak,
            BC_ACQUIRE | BC_RELEASE => &mut r.strong,
            _ => return Err(22),
        };
        *count = if matches!(command, BC_ACQUIRE | BC_INCREFS) {
            count.checked_add(1)
        } else {
            count.checked_sub(1)
        }
        .ok_or(22)?;
        self.collect();
        Ok(())
    }
    fn hold(&mut self, recipient: Option<u64>, node: u64, strong: bool) -> Result<Hold, i64> {
        if let Some(owner) = recipient {
            let zero = self.nodes[self.node_index(node)?].manager;
            let i = self.reference(owner, node, zero)?;
            let r = &mut self.references[i];
            let count = if strong {
                &mut r.buffer_strong
            } else {
                &mut r.buffer_weak
            };
            *count = count.checked_add(1).ok_or(75)?;
        } else {
            let i = self.node_index(node)?;
            let n = &mut self.nodes[i];
            let count = if strong { &mut n.strong } else { &mut n.weak };
            *count = count.checked_add(1).ok_or(75)?;
        }
        Ok(Hold {
            node,
            recipient,
            strong,
        })
    }
    pub fn release(&mut self, holds: &[Hold]) {
        for h in holds {
            if let Some(owner) = h.recipient {
                if let Some(r) = self
                    .references
                    .iter_mut()
                    .find(|r| r.owner == owner && r.node == h.node)
                {
                    let count = if h.strong {
                        &mut r.buffer_strong
                    } else {
                        &mut r.buffer_weak
                    };
                    debug_assert!(*count > 0);
                    *count -= 1;
                }
            } else if let Ok(i) = self.node_index(h.node) {
                let n = &mut self.nodes[i];
                let count = if h.strong { &mut n.strong } else { &mut n.weak };
                debug_assert!(*count > 0);
                *count -= 1;
            }
        }
        self.collect();
    }
    pub fn translate(
        &mut self,
        from: u64,
        to: u64,
        tx: Transaction,
        bytes: &mut [u8],
        target: Option<u64>,
    ) -> Result<Vec<Hold>, i64> {
        tx.validate_payload()?;
        if bytes.len() != tx.payload_length()? {
            return Err(22);
        }
        let mut holds = Vec::new();
        holds
            .try_reserve_exact(tx.offsets_size as usize / 8 + usize::from(target.is_some()))
            .map_err(|_| 12)?;
        let result = (|| {
            if let Some(node) = target {
                holds.push(self.hold(None, node, true)?);
            }
            let offsets = (tx.data_size as usize + 7) & !7;
            let mut minimum = 0;
            for i in 0..tx.offsets_size as usize / 8 {
                let at = usize::try_from(u64_at(bytes, offsets + i * 8)).map_err(|_| 22)?;
                if !at.is_multiple_of(4)
                    || at < minimum
                    || at
                        .checked_add(24)
                        .is_none_or(|end| end > tx.data_size as usize)
                {
                    return Err(22);
                }
                minimum = at + 24;
                let kind = u32_at(bytes, at);
                let strong = match kind {
                    TYPE_BINDER | TYPE_HANDLE => true,
                    TYPE_WEAK_BINDER | TYPE_WEAK_HANDLE => false,
                    _ => return Err(95),
                };
                let flags = u32_at(bytes, at + 4);
                let node = match kind {
                    TYPE_BINDER | TYPE_WEAK_BINDER => self.node(
                        from,
                        u64_at(bytes, at + 8),
                        u64_at(bytes, at + 16),
                        flags,
                        false,
                    )?,
                    _ => self.lookup(from, u32_at(bytes, at + 8), strong)?,
                };
                let index = self.node_index(node)?;
                let local = self.nodes[index].owner == to && self.nodes[index].alive;
                holds.push(self.hold(if local { None } else { Some(to) }, node, strong)?);
                if local {
                    let n = &self.nodes[index];
                    put32(
                        bytes,
                        at,
                        if strong {
                            TYPE_BINDER
                        } else {
                            TYPE_WEAK_BINDER
                        },
                    );
                    put64(bytes, at + 8, n.pointer);
                    put64(bytes, at + 16, n.cookie);
                } else {
                    let r = self
                        .references
                        .iter()
                        .find(|r| r.owner == to && r.node == node)
                        .unwrap();
                    put32(
                        bytes,
                        at,
                        if strong {
                            TYPE_HANDLE
                        } else {
                            TYPE_WEAK_HANDLE
                        },
                    );
                    put64(bytes, at + 8, u64::from(r.handle));
                    put64(bytes, at + 16, 0);
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.release(&holds);
            return Err(error);
        }
        self.translated += tx.offsets_size / 8;
        Ok(holds)
    }
    fn wanted(&self, n: &Node) -> (bool, bool) {
        let strong = n.manager
            || n.strong > 0
            || self
                .references
                .iter()
                .any(|r| r.node == n.id && (r.strong > 0 || r.buffer_strong > 0));
        let weak = strong || n.weak > 0 || self.references.iter().any(|r| r.node == n.id);
        (strong, weak)
    }
    pub fn unannounced(&self, owner: u64) -> bool {
        self.nodes.iter().any(|n| {
            if n.owner != owner || !n.alive || n.manager {
                return false;
            }
            let (strong, weak) = self.wanted(n);
            n.pending_strong
                || n.pending_weak
                || matches!(n.reserved, Some(BR_INCREFS | BR_ACQUIRE))
                || (strong && !n.has_strong)
                || (weak && !n.has_weak)
        })
    }
    pub fn prepare(&mut self, owner: u64, deaths: bool) -> Option<(Control, u32, u64, u64)> {
        for i in 0..self.nodes.len() {
            let n = &self.nodes[i];
            if n.owner != owner || !n.alive || n.manager || n.reserved.is_some() {
                continue;
            }
            let (strong, weak) = self.wanted(n);
            let command = if weak && !n.has_weak {
                BR_INCREFS
            } else if strong && !n.has_strong {
                BR_ACQUIRE
            } else if !strong && n.has_strong && !n.pending_strong {
                BR_RELEASE
            } else if !weak && !n.has_strong && n.has_weak && !n.pending_weak {
                BR_DECREFS
            } else {
                continue;
            };
            let n = &mut self.nodes[i];
            n.reserved = Some(command);
            return Some((
                Control::Node {
                    node: n.id,
                    command,
                },
                command,
                n.pointer,
                n.cookie,
            ));
        }
        if deaths {
            for r in &mut self.references {
                if r.owner != owner {
                    continue;
                }
                let Some(d) = &mut r.death else {
                    continue;
                };
                if d.reserved || !matches!(d.state, DeathState::Ready | DeathState::ClearReady) {
                    continue;
                }
                let clear = d.state == DeathState::ClearReady;
                d.reserved = true;
                return Some((
                    Control::Death {
                        owner,
                        handle: r.handle,
                        clear,
                    },
                    if clear {
                        BR_CLEAR_DEATH_DONE
                    } else {
                        BR_DEAD_BINDER
                    },
                    d.cookie,
                    0,
                ));
            }
        }
        None
    }
    pub fn validate(&self, control: Control) -> Result<(), i64> {
        let valid = match control {
            Control::Node { node, command } => self
                .node_index(node)
                .is_ok_and(|i| self.nodes[i].alive && self.nodes[i].reserved == Some(command)),
            Control::Death { owner, handle, .. } => self.ref_index(owner, handle).is_ok_and(|i| {
                self.references[i]
                    .death
                    .as_ref()
                    .is_some_and(|d| d.reserved)
            }),
        };
        if valid { Ok(()) } else { Err(14) }
    }
    pub fn finish(&mut self, control: Control, success: bool) {
        match control {
            Control::Node { node, command } => {
                let Ok(i) = self.node_index(node) else {
                    return;
                };
                let n = &mut self.nodes[i];
                n.reserved = None;
                if success {
                    match command {
                        BR_INCREFS => {
                            n.has_weak = true;
                            n.pending_weak = true;
                        }
                        BR_ACQUIRE => {
                            n.has_strong = true;
                            n.pending_strong = true;
                        }
                        BR_RELEASE => n.has_strong = false,
                        BR_DECREFS => n.has_weak = false,
                        _ => unreachable!(),
                    }
                    self.handshakes += 1;
                }
            }
            Control::Death {
                owner,
                handle,
                clear,
            } => {
                let Ok(i) = self.ref_index(owner, handle) else {
                    return;
                };
                let Some(d) = &mut self.references[i].death else {
                    return;
                };
                d.reserved = false;
                if success {
                    if clear {
                        self.references[i].death = None;
                    } else {
                        d.state = DeathState::Delivered;
                    }
                }
            }
        }
        self.collect();
    }
    pub fn acknowledge(
        &mut self,
        owner: u64,
        pointer: u64,
        cookie: u64,
        command: u32,
    ) -> Result<(), i64> {
        let n = self
            .nodes
            .iter_mut()
            .find(|n| n.owner == owner && n.pointer == pointer && n.alive && n.cookie == cookie)
            .ok_or(22)?;
        let pending = match command {
            BC_INCREFS_DONE => &mut n.pending_weak,
            BC_ACQUIRE_DONE => &mut n.pending_strong,
            _ => return Err(22),
        };
        if !*pending {
            return Err(22);
        }
        *pending = false;
        Ok(())
    }
    pub fn request_death(&mut self, owner: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        let i = self.ref_index(owner, handle)?;
        if self.references[i].death.is_some()
            || self
                .references
                .iter()
                .any(|r| r.owner == owner && r.death.as_ref().is_some_and(|d| d.cookie == cookie))
        {
            return Err(22);
        }
        let alive = self.target(self.references[i].node).is_ok();
        self.references[i].death = Some(Death {
            cookie,
            state: if alive {
                DeathState::Armed
            } else {
                DeathState::Ready
            },
            clearing: false,
            reserved: false,
        });
        Ok(())
    }
    pub fn clear_death(&mut self, owner: u64, handle: u32, cookie: u64) -> Result<(), i64> {
        let i = self.ref_index(owner, handle)?;
        let d = self.references[i].death.as_mut().ok_or(22)?;
        if d.cookie != cookie || d.clearing {
            return Err(22);
        }
        d.clearing = true;
        if matches!(d.state, DeathState::Armed | DeathState::Acknowledged) {
            d.state = DeathState::ClearReady;
        }
        Ok(())
    }
    pub fn dead_done(&mut self, owner: u64, cookie: u64) -> Result<(), i64> {
        let d = self
            .references
            .iter_mut()
            .filter(|r| r.owner == owner)
            .filter_map(|r| r.death.as_mut())
            .find(|d| d.cookie == cookie)
            .ok_or(22)?;
        if d.state != DeathState::Delivered {
            return Err(22);
        }
        d.state = if d.clearing {
            DeathState::ClearReady
        } else {
            DeathState::Acknowledged
        };
        Ok(())
    }
    pub fn close(&mut self, owner: u64) {
        self.references.retain(|r| r.owner != owner);
        for n in &mut self.nodes {
            if n.owner != owner {
                continue;
            }
            n.alive = false;
            n.manager = false;
            n.has_strong = false;
            n.has_weak = false;
            n.pending_strong = false;
            n.pending_weak = false;
            n.reserved = None;
            for r in &mut self.references {
                if r.node == n.id
                    && let Some(d) = &mut r.death
                    && d.state == DeathState::Armed
                {
                    d.state = DeathState::Ready;
                }
            }
        }
        self.collect();
    }
    fn collect(&mut self) {
        self.references.retain(|r| {
            r.strong > 0
                || r.weak > 0
                || r.buffer_strong > 0
                || r.buffer_weak > 0
                || r.death.is_some()
        });
        let refs = &self.references;
        self.nodes.retain(|n| {
            n.manager
                || n.strong > 0
                || n.weak > 0
                || n.has_strong
                || n.has_weak
                || n.pending_strong
                || n.pending_weak
                || n.reserved.is_some()
                || refs.iter().any(|r| r.node == n.id)
        });
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.nodes.len(), self.references.len())
    }
}
