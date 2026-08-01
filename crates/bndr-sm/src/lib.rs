#![no_std]

//! Allocation-free wire types and fixed-capacity registry for Bndroid's
//! userspace ServiceManager protocol and its supervisor attach handshake.
//!
//! Frames are encoded field-by-field into exactly 64 bytes. No Rust layout or
//! enum representation is exposed on the wire, and decoding validates every
//! field, all padding, and every reserved byte before returning a frame.

use core::array;

pub mod health;
pub mod maintenance_authorization;
pub mod maintenance_plan;
pub mod manifest;
pub mod verified_manifest;

pub const FRAME_SIZE: usize = 64;
pub const SERVICE_NAME_MAX_BYTES: usize = 32;
pub const DEFAULT_REGISTRY_CAPACITY: usize = 4;
pub const PROTOCOL_VERSION: u8 = 1;
pub const PROTOCOL_MAGIC: [u8; 4] = *b"BSM1";

/// Exact wire size of the supervisor-to-process attach handshake.
pub const SUPERVISOR_ATTACH_FRAME_SIZE: usize = 64;
/// The attach handshake has its own version namespace, independent of BSM1.
pub const SUPERVISOR_ATTACH_VERSION: u8 = 1;
/// The attach handshake has its own magic, independent of BSM1.
pub const SUPERVISOR_ATTACH_MAGIC: [u8; 4] = *b"BSA1";

const MAGIC_RANGE: core::ops::Range<usize> = 0..4;
const VERSION_OFFSET: usize = 4;
const OPCODE_OFFSET: usize = 5;
const FLAGS_OFFSET: usize = 6;
const ROLE_OFFSET: usize = 7;
const TXID_RANGE: core::ops::Range<usize> = 8..12;
const STATUS_RANGE: core::ops::Range<usize> = 12..16;
const INSTANCE_RANGE: core::ops::Range<usize> = 16..20;
const NAME_LENGTH_OFFSET: usize = 20;
const FIRST_RESERVED_RANGE: core::ops::Range<usize> = 21..24;
const NAME_RANGE: core::ops::Range<usize> = 24..56;
const FINAL_RESERVED_RANGE: core::ops::Range<usize> = 56..64;

const ATTACH_MAGIC_RANGE: core::ops::Range<usize> = 0..4;
const ATTACH_VERSION_OFFSET: usize = 4;
const ATTACH_FLAGS_OFFSET: usize = 5;
const ATTACH_ROLE_OFFSET: usize = 6;
const ATTACH_FIRST_RESERVED_RANGE: core::ops::Range<usize> = 7..8;
const ATTACH_MANAGER_EPOCH_RANGE: core::ops::Range<usize> = 8..10;
const ATTACH_SECOND_RESERVED_RANGE: core::ops::Range<usize> = 10..16;
const ATTACH_OWNER_PID_RANGE: core::ops::Range<usize> = 16..24;
const ATTACH_PADDING_RANGE: core::ops::Range<usize> = 24..64;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opcode {
    Boot = 1,
    Register = 16,
    RegisterReply = 17,
    Unregister = 18,
    UnregisterReply = 19,
    Lookup = 32,
    LookupReply = 33,
    LookupEndpoint = 34,
    Connect = 48,
}

impl Opcode {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Boot),
            16 => Some(Self::Register),
            17 => Some(Self::RegisterReply),
            18 => Some(Self::Unregister),
            19 => Some(Self::UnregisterReply),
            32 => Some(Self::Lookup),
            33 => Some(Self::LookupReply),
            34 => Some(Self::LookupEndpoint),
            48 => Some(Self::Connect),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Unspecified = 0,
    Provider = 1,
    Client = 2,
    ServiceManager = 3,
}

impl Role {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Unspecified),
            1 => Some(Self::Provider),
            2 => Some(Self::Client),
            3 => Some(Self::ServiceManager),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameFlags(u8);

impl FrameFlags {
    pub const NONE: Self = Self(0);
    pub const HAS_HANDLE: Self = Self(1);

    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Self::NONE),
            1 => Some(Self::HAS_HANDLE),
            _ => None,
        }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn has_handle(self) -> bool {
        self.0 & Self::HAS_HANDLE.0 != 0
    }
}

/// Semantic validation failures for a supervisor attach handshake.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorAttachError {
    ManagerEpochMustBeNonZero,
    RoleForbidden(Role),
    OwnerPidMustBeNonZero,
    HandleRequired,
}

/// Strict wire-decoding failures for a supervisor attach handshake.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorAttachDecodeError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownFlags(u8),
    UnknownRole(u8),
    ReservedNonZero { offset: u8 },
    PaddingNonZero { offset: u8 },
    InvalidFrame(SupervisorAttachError),
}

/// A fixed-size supervisor handshake that attaches one provider or client to
/// a particular live ServiceManager generation.
///
/// A valid frame always carries exactly one transferred handle. The handle is
/// transported out of band; [`FrameFlags::HAS_HANDLE`] records that semantic
/// requirement on the wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupervisorAttachFrame {
    manager_epoch: u16,
    flags: FrameFlags,
    role: Role,
    owner_pid: u64,
}

impl SupervisorAttachFrame {
    pub fn new(
        manager_epoch: u16,
        role: Role,
        owner_pid: u64,
    ) -> Result<Self, SupervisorAttachError> {
        Self::validated(manager_epoch, FrameFlags::HAS_HANDLE, role, owner_pid)
    }

    fn validated(
        manager_epoch: u16,
        flags: FrameFlags,
        role: Role,
        owner_pid: u64,
    ) -> Result<Self, SupervisorAttachError> {
        if manager_epoch == 0 {
            return Err(SupervisorAttachError::ManagerEpochMustBeNonZero);
        }
        if !matches!(role, Role::Provider | Role::Client) {
            return Err(SupervisorAttachError::RoleForbidden(role));
        }
        if owner_pid == 0 {
            return Err(SupervisorAttachError::OwnerPidMustBeNonZero);
        }
        if flags != FrameFlags::HAS_HANDLE {
            return Err(SupervisorAttachError::HandleRequired);
        }
        Ok(Self {
            manager_epoch,
            flags,
            role,
            owner_pid,
        })
    }

    pub fn decode(
        bytes: &[u8; SUPERVISOR_ATTACH_FRAME_SIZE],
    ) -> Result<Self, SupervisorAttachDecodeError> {
        if bytes[ATTACH_MAGIC_RANGE] != SUPERVISOR_ATTACH_MAGIC {
            return Err(SupervisorAttachDecodeError::BadMagic);
        }
        if bytes[ATTACH_VERSION_OFFSET] != SUPERVISOR_ATTACH_VERSION {
            return Err(SupervisorAttachDecodeError::UnsupportedVersion(
                bytes[ATTACH_VERSION_OFFSET],
            ));
        }
        let flags = FrameFlags::from_bits(bytes[ATTACH_FLAGS_OFFSET]).ok_or(
            SupervisorAttachDecodeError::UnknownFlags(bytes[ATTACH_FLAGS_OFFSET]),
        )?;
        let role = Role::from_raw(bytes[ATTACH_ROLE_OFFSET]).ok_or(
            SupervisorAttachDecodeError::UnknownRole(bytes[ATTACH_ROLE_OFFSET]),
        )?;
        validate_attach_zeroes(bytes, ATTACH_FIRST_RESERVED_RANGE, false)?;
        validate_attach_zeroes(bytes, ATTACH_SECOND_RESERVED_RANGE, false)?;
        validate_attach_zeroes(bytes, ATTACH_PADDING_RANGE, true)?;

        let manager_epoch = read_u16(bytes, ATTACH_MANAGER_EPOCH_RANGE);
        let owner_pid = read_u64(bytes, ATTACH_OWNER_PID_RANGE);
        Self::validated(manager_epoch, flags, role, owner_pid)
            .map_err(SupervisorAttachDecodeError::InvalidFrame)
    }

    pub fn encode(self) -> [u8; SUPERVISOR_ATTACH_FRAME_SIZE] {
        let mut bytes = [0_u8; SUPERVISOR_ATTACH_FRAME_SIZE];
        bytes[ATTACH_MAGIC_RANGE].copy_from_slice(&SUPERVISOR_ATTACH_MAGIC);
        bytes[ATTACH_VERSION_OFFSET] = SUPERVISOR_ATTACH_VERSION;
        bytes[ATTACH_FLAGS_OFFSET] = self.flags.bits();
        bytes[ATTACH_ROLE_OFFSET] = self.role.raw();
        write_u16(&mut bytes, ATTACH_MANAGER_EPOCH_RANGE, self.manager_epoch);
        write_u64(&mut bytes, ATTACH_OWNER_PID_RANGE, self.owner_pid);
        bytes
    }

    pub const fn manager_epoch(self) -> u16 {
        self.manager_epoch
    }

    pub const fn flags(self) -> FrameFlags {
        self.flags
    }

    pub const fn role(self) -> Role {
        self.role
    }

    pub const fn owner_pid(self) -> u64 {
        self.owner_pid
    }
}

/// ServiceManager response values use the matching stable `bndr-abi::Status`
/// numbers, but only statuses meaningful to this protocol are accepted.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceStatus {
    Ok = 0,
    InvalidArgument = 1,
    PermissionDenied = 2,
    NotFound = 3,
    AlreadyExists = 4,
    Unavailable = 5,
    OutOfMemory = 7,
    InvalidState = 8,
}

impl ServiceStatus {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Ok),
            1 => Some(Self::InvalidArgument),
            2 => Some(Self::PermissionDenied),
            3 => Some(Self::NotFound),
            4 => Some(Self::AlreadyExists),
            5 => Some(Self::Unavailable),
            7 => Some(Self::OutOfMemory),
            8 => Some(Self::InvalidState),
            _ => None,
        }
    }

    pub const fn raw(self) -> u32 {
        self as u32
    }

    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameError {
    Empty,
    TooLong { length: usize },
    InvalidByte { index: u8, byte: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ServiceName {
    length: u8,
    bytes: [u8; SERVICE_NAME_MAX_BYTES],
}

impl ServiceName {
    pub fn new(source: &[u8]) -> Result<Self, NameError> {
        if source.is_empty() {
            return Err(NameError::Empty);
        }
        if source.len() > SERVICE_NAME_MAX_BYTES {
            return Err(NameError::TooLong {
                length: source.len(),
            });
        }
        let mut bytes = [0_u8; SERVICE_NAME_MAX_BYTES];
        for (index, byte) in source.iter().copied().enumerate() {
            if !is_name_byte(byte) {
                return Err(NameError::InvalidByte {
                    index: index as u8,
                    byte,
                });
            }
            bytes[index] = byte;
        }
        Ok(Self {
            length: source.len() as u8,
            bytes,
        })
    }

    pub const fn len(self) -> usize {
        self.length as usize
    }

    pub const fn is_empty(self) -> bool {
        self.length == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }
}

impl TryFrom<&[u8]> for ServiceName {
    type Error = NameError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ServiceName {
    type Error = NameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.as_bytes())
    }
}

impl core::str::FromStr for ServiceName {
    type Err = NameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value.as_bytes())
    }
}

const fn is_name_byte(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-')
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InstanceId(u32);

impl InstanceId {
    pub const fn new(raw: u32) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    BootRoleRequired,
    RoleForbidden,
    TransactionMustBeZero,
    TransactionMustBeNonZero,
    StatusNotAllowed {
        opcode: Opcode,
        status: ServiceStatus,
    },
    InstanceMustBeZero,
    InstanceMustBeNonZero,
    NameRequired,
    NameForbidden,
    FlagsMismatch {
        expected: u8,
        actual: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownOpcode(u8),
    UnknownFlags(u8),
    UnknownRole(u8),
    UnknownStatus(u32),
    ReservedNonZero { offset: u8 },
    NamePaddingNonZero { offset: u8 },
    Name(NameError),
    InvalidFrame(FrameError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame {
    opcode: Opcode,
    flags: FrameFlags,
    role: Role,
    transaction_id: u32,
    status: ServiceStatus,
    instance: u32,
    name: Option<ServiceName>,
}

impl Frame {
    pub fn boot(role: Role) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::Boot,
            FrameFlags::NONE,
            role,
            0,
            ServiceStatus::Ok,
            0,
            None,
        )
    }

    pub fn register(transaction_id: u32, name: ServiceName) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::Register,
            FrameFlags::HAS_HANDLE,
            Role::Unspecified,
            transaction_id,
            ServiceStatus::Ok,
            0,
            Some(name),
        )
    }

    pub fn register_reply(
        transaction_id: u32,
        name: ServiceName,
        status: ServiceStatus,
        instance: u32,
    ) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::RegisterReply,
            FrameFlags::NONE,
            Role::Unspecified,
            transaction_id,
            status,
            instance,
            Some(name),
        )
    }

    pub fn unregister(
        transaction_id: u32,
        name: ServiceName,
        instance: InstanceId,
    ) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::Unregister,
            FrameFlags::NONE,
            Role::Unspecified,
            transaction_id,
            ServiceStatus::Ok,
            instance.raw(),
            Some(name),
        )
    }

    pub fn unregister_reply(
        transaction_id: u32,
        name: ServiceName,
        status: ServiceStatus,
        instance: u32,
    ) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::UnregisterReply,
            FrameFlags::NONE,
            Role::Unspecified,
            transaction_id,
            status,
            instance,
            Some(name),
        )
    }

    pub fn lookup(transaction_id: u32, name: ServiceName) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::Lookup,
            FrameFlags::NONE,
            Role::Unspecified,
            transaction_id,
            ServiceStatus::Ok,
            0,
            Some(name),
        )
    }

    pub fn lookup_reply(
        transaction_id: u32,
        name: ServiceName,
        status: ServiceStatus,
        instance: u32,
    ) -> Result<Self, FrameError> {
        let flags = if status.is_ok() {
            FrameFlags::HAS_HANDLE
        } else {
            FrameFlags::NONE
        };
        Self::validated(
            Opcode::LookupReply,
            flags,
            Role::Unspecified,
            transaction_id,
            status,
            instance,
            Some(name),
        )
    }

    pub fn lookup_endpoint(
        transaction_id: u32,
        name: ServiceName,
        instance: InstanceId,
    ) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::LookupEndpoint,
            FrameFlags::HAS_HANDLE,
            Role::Unspecified,
            transaction_id,
            ServiceStatus::Ok,
            instance.raw(),
            Some(name),
        )
    }

    pub fn connect(
        transaction_id: u32,
        name: ServiceName,
        instance: InstanceId,
    ) -> Result<Self, FrameError> {
        Self::validated(
            Opcode::Connect,
            FrameFlags::HAS_HANDLE,
            Role::Unspecified,
            transaction_id,
            ServiceStatus::Ok,
            instance.raw(),
            Some(name),
        )
    }

    fn validated(
        opcode: Opcode,
        flags: FrameFlags,
        role: Role,
        transaction_id: u32,
        status: ServiceStatus,
        instance: u32,
        name: Option<ServiceName>,
    ) -> Result<Self, FrameError> {
        validate_fields(
            opcode,
            flags,
            role,
            transaction_id,
            status,
            instance,
            name.is_some(),
        )?;
        Ok(Self {
            opcode,
            flags,
            role,
            transaction_id,
            status,
            instance,
            name,
        })
    }

    pub fn decode(bytes: &[u8; FRAME_SIZE]) -> Result<Self, DecodeError> {
        if bytes[MAGIC_RANGE] != PROTOCOL_MAGIC {
            return Err(DecodeError::BadMagic);
        }
        if bytes[VERSION_OFFSET] != PROTOCOL_VERSION {
            return Err(DecodeError::UnsupportedVersion(bytes[VERSION_OFFSET]));
        }
        let opcode = Opcode::from_raw(bytes[OPCODE_OFFSET])
            .ok_or(DecodeError::UnknownOpcode(bytes[OPCODE_OFFSET]))?;
        let flags = FrameFlags::from_bits(bytes[FLAGS_OFFSET])
            .ok_or(DecodeError::UnknownFlags(bytes[FLAGS_OFFSET]))?;
        let role = Role::from_raw(bytes[ROLE_OFFSET])
            .ok_or(DecodeError::UnknownRole(bytes[ROLE_OFFSET]))?;
        let transaction_id = read_u32(bytes, TXID_RANGE);
        let raw_status = read_u32(bytes, STATUS_RANGE);
        let status =
            ServiceStatus::from_raw(raw_status).ok_or(DecodeError::UnknownStatus(raw_status))?;
        let instance = read_u32(bytes, INSTANCE_RANGE);

        validate_reserved(bytes, FIRST_RESERVED_RANGE)?;
        validate_reserved(bytes, FINAL_RESERVED_RANGE)?;

        let name_length = usize::from(bytes[NAME_LENGTH_OFFSET]);
        if name_length > SERVICE_NAME_MAX_BYTES {
            return Err(DecodeError::Name(NameError::TooLong {
                length: name_length,
            }));
        }
        for (relative, byte) in bytes[NAME_RANGE.clone()][name_length..]
            .iter()
            .copied()
            .enumerate()
        {
            if byte != 0 {
                return Err(DecodeError::NamePaddingNonZero {
                    offset: (NAME_RANGE.start + name_length + relative) as u8,
                });
            }
        }
        let name = if name_length == 0 {
            None
        } else {
            Some(
                ServiceName::new(&bytes[NAME_RANGE.start..NAME_RANGE.start + name_length])
                    .map_err(DecodeError::Name)?,
            )
        };
        Self::validated(opcode, flags, role, transaction_id, status, instance, name)
            .map_err(DecodeError::InvalidFrame)
    }

    pub fn encode(self) -> [u8; FRAME_SIZE] {
        let mut bytes = [0_u8; FRAME_SIZE];
        bytes[MAGIC_RANGE].copy_from_slice(&PROTOCOL_MAGIC);
        bytes[VERSION_OFFSET] = PROTOCOL_VERSION;
        bytes[OPCODE_OFFSET] = self.opcode.raw();
        bytes[FLAGS_OFFSET] = self.flags.bits();
        bytes[ROLE_OFFSET] = self.role.raw();
        write_u32(&mut bytes, TXID_RANGE, self.transaction_id);
        write_u32(&mut bytes, STATUS_RANGE, self.status.raw());
        write_u32(&mut bytes, INSTANCE_RANGE, self.instance);
        if let Some(name) = self.name {
            bytes[NAME_LENGTH_OFFSET] = name.length;
            bytes[NAME_RANGE.start..NAME_RANGE.start + name.len()].copy_from_slice(name.as_bytes());
        }
        bytes
    }

    pub const fn opcode(self) -> Opcode {
        self.opcode
    }

    pub const fn flags(self) -> FrameFlags {
        self.flags
    }

    pub const fn role(self) -> Role {
        self.role
    }

    pub const fn transaction_id(self) -> u32 {
        self.transaction_id
    }

    pub const fn status(self) -> ServiceStatus {
        self.status
    }

    pub const fn instance_raw(self) -> u32 {
        self.instance
    }

    pub const fn instance(self) -> Option<InstanceId> {
        InstanceId::new(self.instance)
    }

    pub const fn name(self) -> Option<ServiceName> {
        self.name
    }
}

fn validate_fields(
    opcode: Opcode,
    flags: FrameFlags,
    role: Role,
    transaction_id: u32,
    status: ServiceStatus,
    instance: u32,
    has_name: bool,
) -> Result<(), FrameError> {
    if opcode == Opcode::Boot {
        if !matches!(role, Role::Provider | Role::Client | Role::ServiceManager) {
            return Err(FrameError::BootRoleRequired);
        }
        if transaction_id != 0 {
            return Err(FrameError::TransactionMustBeZero);
        }
        if !status.is_ok() {
            return Err(FrameError::StatusNotAllowed { opcode, status });
        }
        if instance != 0 {
            return Err(FrameError::InstanceMustBeZero);
        }
        if has_name {
            return Err(FrameError::NameForbidden);
        }
        return require_flags(flags, FrameFlags::NONE);
    }

    if role != Role::Unspecified {
        return Err(FrameError::RoleForbidden);
    }
    if transaction_id == 0 {
        return Err(FrameError::TransactionMustBeNonZero);
    }
    if !has_name {
        return Err(FrameError::NameRequired);
    }

    match opcode {
        Opcode::Boot => unreachable!(),
        Opcode::Register => {
            require_status(opcode, status, &[ServiceStatus::Ok])?;
            require_zero_instance(instance)?;
            require_flags(flags, FrameFlags::HAS_HANDLE)
        }
        Opcode::RegisterReply => {
            require_status(
                opcode,
                status,
                &[
                    ServiceStatus::Ok,
                    ServiceStatus::InvalidArgument,
                    ServiceStatus::PermissionDenied,
                    ServiceStatus::AlreadyExists,
                    ServiceStatus::Unavailable,
                    ServiceStatus::OutOfMemory,
                    ServiceStatus::InvalidState,
                ],
            )?;
            require_reply_instance(status, instance)?;
            require_flags(flags, FrameFlags::NONE)
        }
        Opcode::Unregister => {
            require_status(opcode, status, &[ServiceStatus::Ok])?;
            require_nonzero_instance(instance)?;
            require_flags(flags, FrameFlags::NONE)
        }
        Opcode::UnregisterReply => {
            require_status(
                opcode,
                status,
                &[
                    ServiceStatus::Ok,
                    ServiceStatus::PermissionDenied,
                    ServiceStatus::NotFound,
                    ServiceStatus::InvalidState,
                ],
            )?;
            require_reply_instance(status, instance)?;
            require_flags(flags, FrameFlags::NONE)
        }
        Opcode::Lookup => {
            require_status(opcode, status, &[ServiceStatus::Ok])?;
            require_zero_instance(instance)?;
            require_flags(flags, FrameFlags::NONE)
        }
        Opcode::LookupReply => {
            require_status(
                opcode,
                status,
                &[
                    ServiceStatus::Ok,
                    ServiceStatus::InvalidArgument,
                    ServiceStatus::PermissionDenied,
                    ServiceStatus::NotFound,
                    ServiceStatus::Unavailable,
                    ServiceStatus::OutOfMemory,
                    ServiceStatus::InvalidState,
                ],
            )?;
            require_reply_instance(status, instance)?;
            let expected = if status.is_ok() {
                FrameFlags::HAS_HANDLE
            } else {
                FrameFlags::NONE
            };
            require_flags(flags, expected)
        }
        Opcode::LookupEndpoint | Opcode::Connect => {
            require_status(opcode, status, &[ServiceStatus::Ok])?;
            require_nonzero_instance(instance)?;
            require_flags(flags, FrameFlags::HAS_HANDLE)
        }
    }
}

fn require_status(
    opcode: Opcode,
    actual: ServiceStatus,
    allowed: &[ServiceStatus],
) -> Result<(), FrameError> {
    if allowed.contains(&actual) {
        Ok(())
    } else {
        Err(FrameError::StatusNotAllowed {
            opcode,
            status: actual,
        })
    }
}

const fn require_zero_instance(instance: u32) -> Result<(), FrameError> {
    if instance == 0 {
        Ok(())
    } else {
        Err(FrameError::InstanceMustBeZero)
    }
}

const fn require_nonzero_instance(instance: u32) -> Result<(), FrameError> {
    if instance == 0 {
        Err(FrameError::InstanceMustBeNonZero)
    } else {
        Ok(())
    }
}

const fn require_reply_instance(status: ServiceStatus, instance: u32) -> Result<(), FrameError> {
    if status.is_ok() {
        require_nonzero_instance(instance)
    } else {
        require_zero_instance(instance)
    }
}

const fn require_flags(actual: FrameFlags, expected: FrameFlags) -> Result<(), FrameError> {
    if actual.bits() == expected.bits() {
        Ok(())
    } else {
        Err(FrameError::FlagsMismatch {
            expected: expected.bits(),
            actual: actual.bits(),
        })
    }
}

fn validate_reserved(
    bytes: &[u8; FRAME_SIZE],
    range: core::ops::Range<usize>,
) -> Result<(), DecodeError> {
    for (offset, byte) in bytes[range.clone()].iter().copied().enumerate() {
        if byte != 0 {
            return Err(DecodeError::ReservedNonZero {
                offset: (range.start + offset) as u8,
            });
        }
    }
    Ok(())
}

fn validate_attach_zeroes(
    bytes: &[u8; SUPERVISOR_ATTACH_FRAME_SIZE],
    range: core::ops::Range<usize>,
    padding: bool,
) -> Result<(), SupervisorAttachDecodeError> {
    for (offset, byte) in bytes[range.clone()].iter().copied().enumerate() {
        if byte != 0 {
            let offset = (range.start + offset) as u8;
            return Err(if padding {
                SupervisorAttachDecodeError::PaddingNonZero { offset }
            } else {
                SupervisorAttachDecodeError::ReservedNonZero { offset }
            });
        }
    }
    Ok(())
}

fn read_u16(bytes: &[u8; SUPERVISOR_ATTACH_FRAME_SIZE], range: core::ops::Range<usize>) -> u16 {
    u16::from_le_bytes(bytes[range].try_into().expect("u16 wire range"))
}

fn write_u16(
    bytes: &mut [u8; SUPERVISOR_ATTACH_FRAME_SIZE],
    range: core::ops::Range<usize>,
    value: u16,
) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

fn read_u64(bytes: &[u8; SUPERVISOR_ATTACH_FRAME_SIZE], range: core::ops::Range<usize>) -> u64 {
    u64::from_le_bytes(bytes[range].try_into().expect("u64 wire range"))
}

fn write_u64(
    bytes: &mut [u8; SUPERVISOR_ATTACH_FRAME_SIZE],
    range: core::ops::Range<usize>,
    value: u64,
) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8; FRAME_SIZE], range: core::ops::Range<usize>) -> u32 {
    u32::from_le_bytes(bytes[range].try_into().expect("u32 wire range"))
}

fn write_u32(bytes: &mut [u8; FRAME_SIZE], range: core::ops::Range<usize>, value: u32) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegistryEntry<T> {
    name: ServiceName,
    instance: InstanceId,
    resource: T,
}

impl<T> RegistryEntry<T> {
    pub const fn name(&self) -> ServiceName {
        self.name
    }

    pub const fn instance(&self) -> InstanceId {
        self.instance
    }

    pub const fn resource(&self) -> &T {
        &self.resource
    }

    pub const fn resource_mut(&mut self) -> &mut T {
        &mut self.resource
    }

    pub fn into_parts(self) -> (ServiceName, InstanceId, T) {
        (self.name, self.instance, self.resource)
    }
}

#[derive(Debug)]
pub enum RegisterError<T> {
    AlreadyExists { instance: InstanceId, resource: T },
    Full { resource: T },
    InstanceExhausted { resource: T },
}

impl<T> RegisterError<T> {
    pub const fn existing_instance(&self) -> Option<InstanceId> {
        match self {
            Self::AlreadyExists { instance, .. } => Some(*instance),
            Self::Full { .. } | Self::InstanceExhausted { .. } => None,
        }
    }

    pub fn into_resource(self) -> T {
        match self {
            Self::AlreadyExists { resource, .. }
            | Self::Full { resource }
            | Self::InstanceExhausted { resource } => resource,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoveError {
    NotFound,
    InstanceMismatch { current: InstanceId },
}

pub struct Registry<T, const CAPACITY: usize = DEFAULT_REGISTRY_CAPACITY> {
    entries: [Option<RegistryEntry<T>>; CAPACITY],
    length: usize,
    last_instance: u32,
    instance_limit: u32,
}

impl<T, const CAPACITY: usize> Registry<T, CAPACITY> {
    pub fn new() -> Self {
        Self {
            entries: array::from_fn(|_| None),
            length: 0,
            last_instance: 0,
            instance_limit: u32::MAX,
        }
    }

    /// Creates an empty registry whose instance IDs are scoped to one nonzero
    /// manager epoch. Epoch `E` allocates `(E << 16) | 1` first and never
    /// allocates a local generation greater than `0xffff`.
    ///
    /// Epoch zero is rejected because it overlaps the unscoped ID namespace
    /// retained by [`Self::new`].
    pub fn with_epoch(manager_epoch: u16) -> Option<Self> {
        if manager_epoch == 0 {
            return None;
        }
        let epoch_base = u32::from(manager_epoch) << 16;
        Some(Self {
            entries: array::from_fn(|_| None),
            length: 0,
            last_instance: epoch_base,
            instance_limit: epoch_base | u32::from(u16::MAX),
        })
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub const fn capacity(&self) -> usize {
        CAPACITY
    }

    pub const fn available(&self) -> usize {
        CAPACITY - self.length
    }

    pub const fn latest_instance_raw(&self) -> u32 {
        self.last_instance
    }

    pub fn register(
        &mut self,
        name: ServiceName,
        resource: T,
    ) -> Result<InstanceId, RegisterError<T>> {
        if let Some(existing) = self.lookup(&name) {
            return Err(RegisterError::AlreadyExists {
                instance: existing.instance,
                resource,
            });
        }
        let Some(slot) = self.entries.iter().position(Option::is_none) else {
            return Err(RegisterError::Full { resource });
        };
        let Some(next) = self
            .last_instance
            .checked_add(1)
            .filter(|next| *next <= self.instance_limit)
            .and_then(InstanceId::new)
        else {
            return Err(RegisterError::InstanceExhausted { resource });
        };
        self.entries[slot] = Some(RegistryEntry {
            name,
            instance: next,
            resource,
        });
        self.last_instance = next.raw();
        self.length += 1;
        Ok(next)
    }

    pub fn lookup(&self, name: &ServiceName) -> Option<&RegistryEntry<T>> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.name == *name)
    }

    pub fn lookup_mut(&mut self, name: &ServiceName) -> Option<&mut RegistryEntry<T>> {
        self.entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.name == *name)
    }

    pub fn remove(&mut self, name: &ServiceName) -> Option<RegistryEntry<T>> {
        let slot = self
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.name == *name))?;
        self.length -= 1;
        self.entries[slot].take()
    }

    pub fn remove_exact(
        &mut self,
        name: &ServiceName,
        instance: InstanceId,
    ) -> Result<RegistryEntry<T>, RemoveError> {
        let slot = self
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.name == *name))
            .ok_or(RemoveError::NotFound)?;
        let current = self.entries[slot]
            .as_ref()
            .expect("located registry entry disappeared")
            .instance;
        if current != instance {
            return Err(RemoveError::InstanceMismatch { current });
        }
        self.length -= 1;
        Ok(self.entries[slot]
            .take()
            .expect("located registry entry disappeared"))
    }

    pub fn iter(&self) -> impl Iterator<Item = &RegistryEntry<T>> {
        self.entries.iter().flatten()
    }
}

impl<T, const CAPACITY: usize> Default for Registry<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn name(value: &str) -> ServiceName {
        ServiceName::new(value.as_bytes()).unwrap()
    }

    fn mutate_valid_frame(frame: Frame, offset: usize, value: u8) -> [u8; FRAME_SIZE] {
        let mut bytes = frame.encode();
        bytes[offset] = value;
        bytes
    }

    fn mutate_valid_attach(
        frame: SupervisorAttachFrame,
        offset: usize,
        value: u8,
    ) -> [u8; SUPERVISOR_ATTACH_FRAME_SIZE] {
        let mut bytes = frame.encode();
        bytes[offset] = value;
        bytes
    }

    #[test]
    fn service_names_accept_exact_canonical_alphabet_and_boundaries() {
        let one = ServiceName::new(b"a").unwrap();
        let max = ServiceName::new(b"abcdefghijklmnopqrstuvwxyz012345").unwrap();
        let alphabet = ServiceName::new(b"a-z_0.9").unwrap();
        assert_eq!(one.as_bytes(), b"a");
        assert_eq!(max.len(), SERVICE_NAME_MAX_BYTES);
        assert_eq!(alphabet.as_bytes(), b"a-z_0.9");
        assert!(!max.is_empty());
    }

    #[test]
    fn service_names_reject_empty_long_uppercase_slash_and_non_ascii() {
        assert_eq!(ServiceName::new(b""), Err(NameError::Empty));
        assert_eq!(
            ServiceName::new(b"123456789012345678901234567890123"),
            Err(NameError::TooLong { length: 33 })
        );
        for (source, index, byte) in [
            (&b"Core.log"[..], 0, b'C'),
            (&b"core/log"[..], 4, b'/'),
            (&b"core log"[..], 4, b' '),
            (&b"core\0log"[..], 4, 0),
            (&b"core\xff"[..], 4, 0xff),
        ] {
            assert_eq!(
                ServiceName::new(source),
                Err(NameError::InvalidByte { index, byte })
            );
        }
    }

    #[test]
    fn boot_frames_round_trip_for_all_boot_roles() {
        for role in [Role::Provider, Role::Client, Role::ServiceManager] {
            let frame = Frame::boot(role).unwrap();
            assert_eq!(Frame::decode(&frame.encode()), Ok(frame));
            assert_eq!(frame.opcode(), Opcode::Boot);
            assert_eq!(frame.role(), role);
            assert_eq!(frame.name(), None);
        }
        assert_eq!(
            Frame::boot(Role::Unspecified),
            Err(FrameError::BootRoleRequired)
        );
    }

    #[test]
    fn supervisor_attach_round_trips_with_exact_little_endian_fields() {
        for role in [Role::Provider, Role::Client] {
            let frame = SupervisorAttachFrame::new(0x1234, role, 0x1122_3344_5566_7788)
                .expect("valid attach");
            let bytes = frame.encode();
            assert_eq!(bytes.len(), SUPERVISOR_ATTACH_FRAME_SIZE);
            assert_eq!(&bytes[0..4], b"BSA1");
            assert_eq!(bytes[ATTACH_VERSION_OFFSET], SUPERVISOR_ATTACH_VERSION);
            assert_eq!(bytes[ATTACH_FLAGS_OFFSET], FrameFlags::HAS_HANDLE.bits());
            assert_eq!(bytes[ATTACH_ROLE_OFFSET], role.raw());
            assert_eq!(&bytes[ATTACH_MANAGER_EPOCH_RANGE], &[0x34, 0x12]);
            assert_eq!(
                &bytes[ATTACH_OWNER_PID_RANGE],
                &[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
            );
            assert!(
                bytes[ATTACH_FIRST_RESERVED_RANGE]
                    .iter()
                    .chain(bytes[ATTACH_SECOND_RESERVED_RANGE].iter())
                    .chain(bytes[ATTACH_PADDING_RANGE].iter())
                    .all(|byte| *byte == 0)
            );
            assert_eq!(SupervisorAttachFrame::decode(&bytes), Ok(frame));
            assert_eq!(frame.manager_epoch(), 0x1234);
            assert_eq!(frame.role(), role);
            assert_eq!(frame.owner_pid(), 0x1122_3344_5566_7788);
            assert_eq!(frame.flags(), FrameFlags::HAS_HANDLE);
        }
    }

    #[test]
    fn supervisor_attach_constructor_rejects_zeroes_and_forbidden_roles() {
        assert_eq!(
            SupervisorAttachFrame::new(0, Role::Provider, 1),
            Err(SupervisorAttachError::ManagerEpochMustBeNonZero)
        );
        for role in [Role::Unspecified, Role::ServiceManager] {
            assert_eq!(
                SupervisorAttachFrame::new(1, role, 1),
                Err(SupervisorAttachError::RoleForbidden(role))
            );
        }
        assert_eq!(
            SupervisorAttachFrame::new(1, Role::Client, 0),
            Err(SupervisorAttachError::OwnerPidMustBeNonZero)
        );
    }

    #[test]
    fn supervisor_attach_decoder_rejects_header_role_handle_and_zero_fields() {
        let base = SupervisorAttachFrame::new(7, Role::Provider, 9).unwrap();
        assert_eq!(
            SupervisorAttachFrame::decode(&mutate_valid_attach(base, 0, b'X')),
            Err(SupervisorAttachDecodeError::BadMagic)
        );
        assert_eq!(
            SupervisorAttachFrame::decode(&mutate_valid_attach(base, ATTACH_VERSION_OFFSET, 2)),
            Err(SupervisorAttachDecodeError::UnsupportedVersion(2))
        );
        assert_eq!(
            SupervisorAttachFrame::decode(&mutate_valid_attach(base, ATTACH_FLAGS_OFFSET, 0x80)),
            Err(SupervisorAttachDecodeError::UnknownFlags(0x80))
        );
        assert_eq!(
            SupervisorAttachFrame::decode(&mutate_valid_attach(
                base,
                ATTACH_FLAGS_OFFSET,
                FrameFlags::NONE.bits()
            )),
            Err(SupervisorAttachDecodeError::InvalidFrame(
                SupervisorAttachError::HandleRequired
            ))
        );
        assert_eq!(
            SupervisorAttachFrame::decode(&mutate_valid_attach(base, ATTACH_ROLE_OFFSET, 4)),
            Err(SupervisorAttachDecodeError::UnknownRole(4))
        );
        for role in [Role::Unspecified, Role::ServiceManager] {
            assert_eq!(
                SupervisorAttachFrame::decode(&mutate_valid_attach(
                    base,
                    ATTACH_ROLE_OFFSET,
                    role.raw()
                )),
                Err(SupervisorAttachDecodeError::InvalidFrame(
                    SupervisorAttachError::RoleForbidden(role)
                ))
            );
        }

        let mut zero_epoch = base.encode();
        zero_epoch[ATTACH_MANAGER_EPOCH_RANGE].copy_from_slice(&0_u16.to_le_bytes());
        assert_eq!(
            SupervisorAttachFrame::decode(&zero_epoch),
            Err(SupervisorAttachDecodeError::InvalidFrame(
                SupervisorAttachError::ManagerEpochMustBeNonZero
            ))
        );
        let mut zero_pid = base.encode();
        zero_pid[ATTACH_OWNER_PID_RANGE].copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            SupervisorAttachFrame::decode(&zero_pid),
            Err(SupervisorAttachDecodeError::InvalidFrame(
                SupervisorAttachError::OwnerPidMustBeNonZero
            ))
        );
    }

    #[test]
    fn supervisor_attach_decoder_rejects_every_reserved_and_padding_byte() {
        let base = SupervisorAttachFrame::new(7, Role::Client, 9).unwrap();
        for offset in ATTACH_FIRST_RESERVED_RANGE.chain(ATTACH_SECOND_RESERVED_RANGE) {
            assert_eq!(
                SupervisorAttachFrame::decode(&mutate_valid_attach(base, offset, 1)),
                Err(SupervisorAttachDecodeError::ReservedNonZero {
                    offset: offset as u8
                })
            );
        }
        for offset in ATTACH_PADDING_RANGE {
            assert_eq!(
                SupervisorAttachFrame::decode(&mutate_valid_attach(base, offset, 1)),
                Err(SupervisorAttachDecodeError::PaddingNonZero {
                    offset: offset as u8
                })
            );
        }
    }

    #[test]
    fn register_and_lookup_requests_round_trip() {
        let service = name("core.log");
        let register = Frame::register(0x1122_3344, service).unwrap();
        let lookup = Frame::lookup(0xaabb_ccdd, service).unwrap();
        for frame in [register, lookup] {
            assert_eq!(Frame::decode(&frame.encode()), Ok(frame));
            assert_eq!(frame.status(), ServiceStatus::Ok);
            assert_eq!(frame.instance(), None);
            assert_eq!(frame.name(), Some(service));
        }
        assert!(register.flags().has_handle());
        assert!(!lookup.flags().has_handle());
    }

    #[test]
    fn unregister_request_and_replies_round_trip_in_v1_frames() {
        let service = name("core.dynamic");
        let instance = InstanceId::new(0x0002_0002).unwrap();
        let request = Frame::unregister(0x1122_3344, service, instance).unwrap();
        let success =
            Frame::unregister_reply(0x1122_3344, service, ServiceStatus::Ok, instance.raw())
                .unwrap();
        let denied =
            Frame::unregister_reply(0x2233_4455, service, ServiceStatus::PermissionDenied, 0)
                .unwrap();
        let missing =
            Frame::unregister_reply(0x3344_5566, service, ServiceStatus::NotFound, 0).unwrap();
        let stale =
            Frame::unregister_reply(0x4455_6677, service, ServiceStatus::InvalidState, 0).unwrap();

        for frame in [request, success, denied, missing, stale] {
            let bytes = frame.encode();
            assert_eq!(&bytes[MAGIC_RANGE], b"BSM1");
            assert_eq!(bytes[VERSION_OFFSET], PROTOCOL_VERSION);
            assert_eq!(Frame::decode(&bytes), Ok(frame));
            assert!(!frame.flags().has_handle());
            assert_eq!(frame.name(), Some(service));
        }
        assert_eq!(request.opcode(), Opcode::Unregister);
        assert_eq!(request.status(), ServiceStatus::Ok);
        assert_eq!(request.instance(), Some(instance));
        assert_eq!(success.opcode(), Opcode::UnregisterReply);
        assert_eq!(success.instance(), Some(instance));
        for failure in [denied, missing, stale] {
            assert_eq!(failure.opcode(), Opcode::UnregisterReply);
            assert_eq!(failure.instance(), None);
        }
        assert_eq!(Opcode::Unregister.raw(), 18);
        assert_eq!(Opcode::UnregisterReply.raw(), 19);
    }

    #[test]
    fn unregister_constructors_enforce_status_and_instance_relationships() {
        let service = name("core.dynamic");
        let instance = InstanceId::new(7).unwrap();
        assert_eq!(
            Frame::unregister(0, service, instance),
            Err(FrameError::TransactionMustBeNonZero)
        );
        assert_eq!(
            Frame::unregister_reply(1, service, ServiceStatus::Ok, 0),
            Err(FrameError::InstanceMustBeNonZero)
        );
        for status in [
            ServiceStatus::PermissionDenied,
            ServiceStatus::NotFound,
            ServiceStatus::InvalidState,
        ] {
            assert_eq!(
                Frame::unregister_reply(1, service, status, instance.raw()),
                Err(FrameError::InstanceMustBeZero)
            );
        }
        for status in [
            ServiceStatus::InvalidArgument,
            ServiceStatus::AlreadyExists,
            ServiceStatus::Unavailable,
            ServiceStatus::OutOfMemory,
        ] {
            assert_eq!(
                Frame::unregister_reply(1, service, status, 0),
                Err(FrameError::StatusNotAllowed {
                    opcode: Opcode::UnregisterReply,
                    status,
                })
            );
        }
    }

    #[test]
    fn unregister_decoder_rejects_flags_status_and_instance_corruption() {
        let service = name("core.dynamic");
        let instance = InstanceId::new(7).unwrap();
        let request = Frame::unregister(1, service, instance).unwrap();

        assert_eq!(
            Frame::decode(&mutate_valid_frame(
                request,
                FLAGS_OFFSET,
                FrameFlags::HAS_HANDLE.bits()
            )),
            Err(DecodeError::InvalidFrame(FrameError::FlagsMismatch {
                expected: FrameFlags::NONE.bits(),
                actual: FrameFlags::HAS_HANDLE.bits(),
            }))
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(request, FLAGS_OFFSET, 0x80)),
            Err(DecodeError::UnknownFlags(0x80))
        );

        let mut unknown_status = request.encode();
        unknown_status[STATUS_RANGE].copy_from_slice(&6_u32.to_le_bytes());
        assert_eq!(
            Frame::decode(&unknown_status),
            Err(DecodeError::UnknownStatus(6))
        );
        let mut forbidden_status = request.encode();
        forbidden_status[STATUS_RANGE]
            .copy_from_slice(&ServiceStatus::NotFound.raw().to_le_bytes());
        assert_eq!(
            Frame::decode(&forbidden_status),
            Err(DecodeError::InvalidFrame(FrameError::StatusNotAllowed {
                opcode: Opcode::Unregister,
                status: ServiceStatus::NotFound,
            }))
        );

        let mut zero_request_instance = request.encode();
        zero_request_instance[INSTANCE_RANGE].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            Frame::decode(&zero_request_instance),
            Err(DecodeError::InvalidFrame(FrameError::InstanceMustBeNonZero))
        );

        let success =
            Frame::unregister_reply(2, service, ServiceStatus::Ok, instance.raw()).unwrap();
        let mut zero_success_instance = success.encode();
        zero_success_instance[INSTANCE_RANGE].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            Frame::decode(&zero_success_instance),
            Err(DecodeError::InvalidFrame(FrameError::InstanceMustBeNonZero))
        );

        let missing = Frame::unregister_reply(3, service, ServiceStatus::NotFound, 0).unwrap();
        let mut nonzero_error_instance = missing.encode();
        nonzero_error_instance[INSTANCE_RANGE].copy_from_slice(&instance.raw().to_le_bytes());
        assert_eq!(
            Frame::decode(&nonzero_error_instance),
            Err(DecodeError::InvalidFrame(FrameError::InstanceMustBeZero))
        );
    }

    #[test]
    fn reply_frames_enforce_status_instance_and_handle_relationships() {
        let service = name("core.log");
        let success = Frame::register_reply(1, service, ServiceStatus::Ok, 9).unwrap();
        let duplicate = Frame::register_reply(2, service, ServiceStatus::AlreadyExists, 0).unwrap();
        let found = Frame::lookup_reply(3, service, ServiceStatus::Ok, 9).unwrap();
        let missing = Frame::lookup_reply(4, service, ServiceStatus::NotFound, 0).unwrap();
        for frame in [success, duplicate, found, missing] {
            assert_eq!(Frame::decode(&frame.encode()), Ok(frame));
        }
        assert!(!success.flags().has_handle());
        assert!(found.flags().has_handle());
        assert!(!missing.flags().has_handle());
        assert_eq!(found.instance().unwrap().raw(), 9);
        assert_eq!(
            Frame::lookup_reply(3, service, ServiceStatus::Ok, 0),
            Err(FrameError::InstanceMustBeNonZero)
        );
        assert_eq!(
            Frame::lookup_reply(3, service, ServiceStatus::NotFound, 9),
            Err(FrameError::InstanceMustBeZero)
        );
    }

    #[test]
    fn endpoint_and_connect_transfers_round_trip() {
        let service = name("core.echo");
        let instance = InstanceId::new(7).unwrap();
        for frame in [
            Frame::lookup_endpoint(1, service, instance).unwrap(),
            Frame::connect(1, service, instance).unwrap(),
        ] {
            assert!(frame.flags().has_handle());
            assert_eq!(frame.instance(), Some(instance));
            assert_eq!(Frame::decode(&frame.encode()), Ok(frame));
        }
    }

    #[test]
    fn encoding_is_exactly_64_bytes_and_explicit_little_endian() {
        let frame = Frame::lookup_endpoint(
            0x1122_3344,
            name("core.log"),
            InstanceId::new(0xa1b2_c3d4).unwrap(),
        )
        .unwrap();
        let bytes = frame.encode();
        assert_eq!(bytes.len(), FRAME_SIZE);
        assert_eq!(&bytes[0..4], b"BSM1");
        assert_eq!(&bytes[8..12], &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(&bytes[12..16], &[0, 0, 0, 0]);
        assert_eq!(&bytes[16..20], &[0xd4, 0xc3, 0xb2, 0xa1]);
        assert_eq!(bytes[20], 8);
        assert_eq!(&bytes[24..32], b"core.log");
        assert!(bytes[32..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn decoder_rejects_magic_version_opcode_flags_role_and_status() {
        let base = Frame::lookup(1, name("core.log")).unwrap();
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, 0, b'X')),
            Err(DecodeError::BadMagic)
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, VERSION_OFFSET, 2)),
            Err(DecodeError::UnsupportedVersion(2))
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, OPCODE_OFFSET, 0xff)),
            Err(DecodeError::UnknownOpcode(0xff))
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, FLAGS_OFFSET, 0x80)),
            Err(DecodeError::UnknownFlags(0x80))
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, ROLE_OFFSET, 4)),
            Err(DecodeError::UnknownRole(4))
        );
        let mut status = base.encode();
        status[STATUS_RANGE].copy_from_slice(&6_u32.to_le_bytes());
        assert_eq!(Frame::decode(&status), Err(DecodeError::UnknownStatus(6)));
    }

    #[test]
    fn decoder_rejects_every_reserved_region() {
        let base = Frame::lookup(1, name("core.log")).unwrap();
        for offset in FIRST_RESERVED_RANGE.chain(FINAL_RESERVED_RANGE) {
            assert_eq!(
                Frame::decode(&mutate_valid_frame(base, offset, 1)),
                Err(DecodeError::ReservedNonZero {
                    offset: offset as u8
                })
            );
        }
    }

    #[test]
    fn decoder_rejects_nonzero_name_padding_and_invalid_name_bytes() {
        let base = Frame::lookup(1, name("a")).unwrap();
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, NAME_RANGE.start + 1, b'x')),
            Err(DecodeError::NamePaddingNonZero { offset: 25 })
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, NAME_RANGE.start, b'A')),
            Err(DecodeError::Name(NameError::InvalidByte {
                index: 0,
                byte: b'A'
            }))
        );
        assert_eq!(
            Frame::decode(&mutate_valid_frame(base, NAME_LENGTH_OFFSET, 33)),
            Err(DecodeError::Name(NameError::TooLong { length: 33 }))
        );
    }

    #[test]
    fn decoder_revalidates_cross_field_semantics() {
        let lookup = Frame::lookup(1, name("core.log")).unwrap();
        let mut zero_txid = lookup.encode();
        zero_txid[TXID_RANGE].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            Frame::decode(&zero_txid),
            Err(DecodeError::InvalidFrame(
                FrameError::TransactionMustBeNonZero
            ))
        );

        let mut named_boot = Frame::boot(Role::Provider).unwrap().encode();
        named_boot[NAME_LENGTH_OFFSET] = 1;
        named_boot[NAME_RANGE.start] = b'a';
        assert_eq!(
            Frame::decode(&named_boot),
            Err(DecodeError::InvalidFrame(FrameError::NameForbidden))
        );

        let mut forbidden_role = lookup.encode();
        for role in [Role::Client, Role::ServiceManager] {
            forbidden_role[ROLE_OFFSET] = role.raw();
            assert_eq!(
                Frame::decode(&forbidden_role),
                Err(DecodeError::InvalidFrame(FrameError::RoleForbidden))
            );
        }
    }

    #[test]
    fn opcode_specific_statuses_are_strict() {
        let service = name("core.log");
        assert_eq!(
            Frame::register_reply(1, service, ServiceStatus::NotFound, 0),
            Err(FrameError::StatusNotAllowed {
                opcode: Opcode::RegisterReply,
                status: ServiceStatus::NotFound
            })
        );
        assert_eq!(
            Frame::lookup_reply(1, service, ServiceStatus::AlreadyExists, 0),
            Err(FrameError::StatusNotAllowed {
                opcode: Opcode::LookupReply,
                status: ServiceStatus::AlreadyExists
            })
        );
    }

    #[test]
    fn registry_registers_and_looks_up_resources() {
        let mut registry = Registry::<u64>::new();
        let service = name("core.log");
        let instance = registry.register(service, 0x1234).unwrap();
        let entry = registry.lookup(&service).unwrap();
        assert_eq!(instance.raw(), 1);
        assert_eq!(entry.name(), service);
        assert_eq!(entry.instance(), instance);
        assert_eq!(*entry.resource(), 0x1234);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.available(), 3);
    }

    #[test]
    fn epoch_seeded_registries_start_distinct_non_aba_instance_spaces() {
        let service = name("core.log");
        let mut epoch_one = Registry::<u64>::with_epoch(1).unwrap();
        let mut epoch_two = Registry::<u64>::with_epoch(2).unwrap();
        let first = epoch_one.register(service, 11).unwrap();
        let second = epoch_two.register(service, 22).unwrap();
        assert_eq!(first.raw(), 0x0001_0001);
        assert_eq!(second.raw(), 0x0002_0001);
        assert_ne!(first, second);
        assert_eq!(epoch_one.latest_instance_raw(), first.raw());
        assert_eq!(epoch_two.latest_instance_raw(), second.raw());
    }

    #[test]
    fn epoch_seeded_registry_exhausts_before_crossing_epoch_boundary() {
        let mut registry = Registry::<u64>::with_epoch(0x1234).unwrap();
        registry.last_instance = 0x1234_fffe;
        let last = registry.register(name("a"), 1).unwrap();
        assert_eq!(last.raw(), 0x1234_ffff);
        registry.remove(&name("a")).unwrap();

        let error = registry.register(name("b"), 77).unwrap_err();
        assert!(matches!(&error, RegisterError::InstanceExhausted { .. }));
        assert_eq!(error.into_resource(), 77);
        assert!(registry.is_empty());
        assert_eq!(registry.latest_instance_raw(), 0x1234_ffff);
    }

    #[test]
    fn epoch_seeded_registry_rejects_zero_epoch_namespace_overlap() {
        assert!(Registry::<u64>::with_epoch(0).is_none());
    }

    #[test]
    fn duplicate_registration_returns_resource_and_does_not_advance_generation() {
        let mut registry = Registry::<u64>::new();
        let service = name("core.log");
        let first = registry.register(service, 10).unwrap();
        let error = registry.register(service, 20).unwrap_err();
        assert_eq!(error.existing_instance(), Some(first));
        assert_eq!(error.into_resource(), 20);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.latest_instance_raw(), 1);
        registry.remove(&service).unwrap();
        assert_eq!(registry.register(service, 30).unwrap().raw(), 2);
    }

    #[test]
    fn registry_capacity_is_fixed_and_full_failure_preserves_resource() {
        let mut registry = Registry::<u8, 2>::new();
        registry.register(name("a"), 1).unwrap();
        registry.register(name("b"), 2).unwrap();
        let error = registry.register(name("c"), 3).unwrap_err();
        assert!(matches!(&error, RegisterError::Full { .. }));
        assert_eq!(error.into_resource(), 3);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.available(), 0);
        assert_eq!(registry.latest_instance_raw(), 2);
    }

    #[test]
    fn zero_capacity_registry_is_well_defined() {
        let mut registry = Registry::<u8, 0>::new();
        assert_eq!(registry.capacity(), 0);
        assert!(registry.is_empty());
        let error = registry.register(name("a"), 7).unwrap_err();
        assert!(matches!(error, RegisterError::Full { resource: 7 }));
    }

    #[test]
    fn removal_and_reregistration_increment_instance_generation() {
        let mut registry = Registry::<u64>::new();
        let service = name("core.log");
        let first = registry.register(service, 1).unwrap();
        let removed = registry.remove_exact(&service, first).unwrap();
        assert_eq!(removed.into_parts(), (service, first, 1));
        assert!(registry.is_empty());
        let second = registry.register(service, 2).unwrap();
        assert_eq!(second.raw(), first.raw() + 1);
    }

    #[test]
    fn exact_remove_rejects_stale_instance_without_removing_replacement() {
        let mut registry = Registry::<u64>::new();
        let service = name("core.log");
        let first = registry.register(service, 1).unwrap();
        registry.remove_exact(&service, first).unwrap();
        let second = registry.register(service, 2).unwrap();
        assert_eq!(
            registry.remove_exact(&service, first),
            Err(RemoveError::InstanceMismatch { current: second })
        );
        assert_eq!(*registry.lookup(&service).unwrap().resource(), 2);
    }

    #[test]
    fn removing_unknown_name_or_instance_is_non_destructive() {
        let mut registry = Registry::<u64>::new();
        registry.register(name("core.log"), 1).unwrap();
        assert!(registry.remove(&name("missing")).is_none());
        assert_eq!(
            registry.remove_exact(&name("missing"), InstanceId::new(1).unwrap()),
            Err(RemoveError::NotFound)
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn lookup_mut_and_iteration_cover_only_live_entries() {
        let mut registry = Registry::<u64, 3>::new();
        registry.register(name("a"), 1).unwrap();
        registry.register(name("b"), 2).unwrap();
        *registry.lookup_mut(&name("a")).unwrap().resource_mut() = 9;
        registry.remove(&name("b"));
        let resources: Vec<u64> = registry.iter().map(|entry| *entry.resource()).collect();
        assert_eq!(resources, [9]);
    }

    #[test]
    fn exhausted_instance_generation_returns_resource_without_publication() {
        let mut registry = Registry::<u64>::new();
        registry.last_instance = u32::MAX;
        let error = registry.register(name("core.log"), 77).unwrap_err();
        assert!(matches!(&error, RegisterError::InstanceExhausted { .. }));
        assert_eq!(error.into_resource(), 77);
        assert!(registry.is_empty());
        assert_eq!(registry.latest_instance_raw(), u32::MAX);
    }
}
