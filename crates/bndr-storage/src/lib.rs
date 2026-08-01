#![no_std]
#![deny(unsafe_code)]

//! Allocation-free wire ABI for Bndroid's userspace StorageServer.
//!
//! Requests and responses are encoded field-by-field into exactly 64 bytes;
//! Rust layout and enum representation are never exposed on the wire. Every
//! request is sent with exactly one immutable VMO through Channel handle
//! transfer. That VMO contains the exact concatenation `path || value`, where
//! the two lengths come from the request frame. A successful read or list
//! response similarly carries one immutable result VMO when its payload flag
//! is set.
//!
//! This protocol deliberately contains no principal, process ID, or caller-
//! selected namespace. The StorageServer must authenticate the Channel sender
//! from the kernel-stamped envelope and bind the session from trusted startup
//! state before interpreting a request. [`Opcode::Bind`] is only a wire-level
//! handshake; accepting it does not by itself authenticate the caller.

use core::ops::Range;

/// Exact size of every request and response frame.
pub const FRAME_SIZE: usize = 64;
/// Stable wire ABI version.
pub const ABI_VERSION: u16 = 1;
/// Magic for a storage request (`BSQ1`).
pub const REQUEST_MAGIC: [u8; 4] = *b"BSQ1";
/// Magic for a storage response (`BSP1`).
pub const RESPONSE_MAGIC: [u8; 4] = *b"BSP1";

/// Longest canonical root-relative path accepted by this protocol.
pub const PATH_MAX_BYTES: usize = 64;
/// Largest whole-file value accepted by one replace or read result.
pub const VALUE_MAX_BYTES: usize = 4096;
/// Maximum number of non-empty components in one path.
pub const PATH_MAX_DEPTH: usize = 4;
/// Largest bounded directory cursor. Cursor zero names the first entry.
pub const DIRECTORY_MAX_ENTRIES: u64 = 32;
/// Largest request VMO: a maximum path followed by a maximum value.
pub const REQUEST_PAYLOAD_MAX_BYTES: usize = PATH_MAX_BYTES + VALUE_MAX_BYTES;

const MAGIC_RANGE: Range<usize> = 0..4;
const VERSION_RANGE: Range<usize> = 4..6;
const OPCODE_OFFSET: usize = 6;
const FLAGS_OFFSET: usize = 7;
const TXID_RANGE: Range<usize> = 8..16;

const REQUEST_PATH_LENGTH_RANGE: Range<usize> = 16..20;
const REQUEST_VALUE_LENGTH_RANGE: Range<usize> = 20..24;
const REQUEST_CAS_GENERATION_RANGE: Range<usize> = 24..32;
const REQUEST_CURSOR_RANGE: Range<usize> = 32..40;
const REQUEST_RESERVED_RANGE: Range<usize> = 40..48;
const REQUEST_PADDING_RANGE: Range<usize> = 48..64;

const RESPONSE_STATUS_RANGE: Range<usize> = 16..18;
const RESPONSE_RESULT_RANGE: Range<usize> = 18..20;
const RESPONSE_PAYLOAD_LENGTH_RANGE: Range<usize> = 20..24;
const RESPONSE_GENERATION_RANGE: Range<usize> = 24..32;
const RESPONSE_CURSOR_RANGE: Range<usize> = 32..40;
const RESPONSE_OBJECT_SIZE_RANGE: Range<usize> = 40..44;
const RESPONSE_ENTRY_KIND_OFFSET: usize = 44;
const RESPONSE_RESERVED_RANGE: Range<usize> = 45..48;
const RESPONSE_PADDING_RANGE: Range<usize> = 48..64;

const REQUEST_FLAG_PAYLOAD_VMO: u8 = 1 << 0;
const REQUEST_FLAG_CREATE_ONLY: u8 = 1 << 1;
const REQUEST_FLAG_EXACT_GENERATION: u8 = 1 << 2;
const REQUEST_FLAGS_KNOWN: u8 =
    REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_CREATE_ONLY | REQUEST_FLAG_EXACT_GENERATION;
const RESPONSE_FLAG_PAYLOAD_VMO: u8 = 1 << 0;
const RESPONSE_FLAGS_KNOWN: u8 = RESPONSE_FLAG_PAYLOAD_VMO;

const _: () = assert!(FRAME_SIZE == 64);
const _: () = assert!(REQUEST_PAYLOAD_MAX_BYTES == 4160);
const _: () = assert!(PATH_MAX_BYTES <= u32::MAX as usize);
const _: () = assert!(VALUE_MAX_BYTES <= u32::MAX as usize);

/// One stable StorageServer operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opcode {
    /// Establishes a session that has already been authenticated externally.
    Bind = 1,
    CreateDirectory = 2,
    Replace = 3,
    Read = 4,
    List = 5,
    Unlink = 6,
}

impl Opcode {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Bind),
            2 => Some(Self::CreateDirectory),
            3 => Some(Self::Replace),
            4 => Some(Self::Read),
            5 => Some(Self::List),
            6 => Some(Self::Unlink),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

/// Compare-and-swap condition for [`Opcode::Replace`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplaceCondition {
    /// Replace either an existing file or an absent path.
    Any,
    /// Succeed only when the path is absent.
    CreateOnly,
    /// Succeed only when the current file generation matches exactly.
    Exact(u64),
}

/// Stable service status. Existing Bndroid kernel status numbers retain their
/// values through `NoSpace`; storage-domain results are appended afterwards.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    PermissionDenied = 2,
    NotFound = 3,
    AlreadyExists = 4,
    Unavailable = 5,
    Timeout = 6,
    OutOfMemory = 7,
    InvalidState = 8,
    Unsupported = 9,
    ShouldWait = 10,
    PeerClosed = 11,
    BadAddress = 12,
    BufferTooSmall = 13,
    Conflict = 14,
    OutcomeUnknown = 15,
    DataCorrupt = 16,
    NoSpace = 17,
    NotDirectory = 18,
    IsDirectory = 19,
    NotEmpty = 20,
    RequiresReset = 21,
    ProtocolError = 22,
}

impl Status {
    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(Self::Ok),
            1 => Some(Self::InvalidArgument),
            2 => Some(Self::PermissionDenied),
            3 => Some(Self::NotFound),
            4 => Some(Self::AlreadyExists),
            5 => Some(Self::Unavailable),
            6 => Some(Self::Timeout),
            7 => Some(Self::OutOfMemory),
            8 => Some(Self::InvalidState),
            9 => Some(Self::Unsupported),
            10 => Some(Self::ShouldWait),
            11 => Some(Self::PeerClosed),
            12 => Some(Self::BadAddress),
            13 => Some(Self::BufferTooSmall),
            14 => Some(Self::Conflict),
            15 => Some(Self::OutcomeUnknown),
            16 => Some(Self::DataCorrupt),
            17 => Some(Self::NoSpace),
            18 => Some(Self::NotDirectory),
            19 => Some(Self::IsDirectory),
            20 => Some(Self::NotEmpty),
            21 => Some(Self::RequiresReset),
            22 => Some(Self::ProtocolError),
            _ => None,
        }
    }

    pub const fn raw(self) -> u16 {
        self as u16
    }

    /// The StorageServer must tear down its epoch after returning either of
    /// these results. `OutcomeUnknown` requires durable-state reconciliation;
    /// `RequiresReset` closes the transport barrier. Neither is a retry hint
    /// for the current session.
    pub const fn is_session_fatal(self) -> bool {
        matches!(self, Self::OutcomeUnknown | Self::RequiresReset)
    }
}

/// Stable interpretation of the response result fields.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultKind {
    /// Required for every non-success status.
    None = 0,
    /// Successful trusted-session bind.
    Bound = 1,
    /// Successful create, replace, or unlink.
    Mutation = 2,
    /// Successful file read; the response transfers a value VMO.
    File = 3,
    /// One list entry; the response transfers its canonical path in a VMO.
    DirectoryEntry = 4,
    /// The requested list cursor is at end of directory.
    EndOfDirectory = 5,
}

impl ResultKind {
    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::Bound),
            2 => Some(Self::Mutation),
            3 => Some(Self::File),
            4 => Some(Self::DirectoryEntry),
            5 => Some(Self::EndOfDirectory),
            _ => None,
        }
    }

    pub const fn raw(self) -> u16 {
        self as u16
    }
}

/// Stable kind of one successful list entry.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File = 1,
    Directory = 2,
}

impl EntryKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::File),
            2 => Some(Self::Directory),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RequestFlags(u8);

impl RequestFlags {
    const PAYLOAD: Self = Self(REQUEST_FLAG_PAYLOAD_VMO);

    const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !REQUEST_FLAGS_KNOWN == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    const fn for_replace(condition: ReplaceCondition) -> Self {
        match condition {
            ReplaceCondition::Any => Self::PAYLOAD,
            ReplaceCondition::CreateOnly => {
                Self(REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_CREATE_ONLY)
            }
            ReplaceCondition::Exact(_) => {
                Self(REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_EXACT_GENERATION)
            }
        }
    }

    const fn bits(self) -> u8 {
        self.0
    }

    const fn has_payload_vmo(self) -> bool {
        self.0 & REQUEST_FLAG_PAYLOAD_VMO != 0
    }

    const fn create_only(self) -> bool {
        self.0 & REQUEST_FLAG_CREATE_ONLY != 0
    }

    const fn exact_generation(self) -> bool {
        self.0 & REQUEST_FLAG_EXACT_GENERATION != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResponseFlags(u8);

impl ResponseFlags {
    const NONE: Self = Self(0);
    const PAYLOAD: Self = Self(RESPONSE_FLAG_PAYLOAD_VMO);

    const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !RESPONSE_FLAGS_KNOWN == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    const fn bits(self) -> u8 {
        self.0
    }

    const fn has_payload_vmo(self) -> bool {
        self.0 & RESPONSE_FLAG_PAYLOAD_VMO != 0
    }
}

/// Semantic errors in a storage request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestError {
    TransactionIdMustBeNonZero,
    PayloadVmoRequired,
    PathTooLong,
    ValueTooLong,
    PayloadLengthOverflow,
    PathRequired,
    UnexpectedPath,
    UnexpectedValue,
    ReplaceFlagsRequired,
    ReplaceFlagsConflict,
    CasGenerationMustBeZero,
    ExactGenerationMustBeNonZero,
    UnexpectedCursor,
    CursorOutOfRange,
}

/// Strict wire-decoding failures for a request frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestDecodeError {
    BadMagic,
    UnsupportedVersion(u16),
    UnknownOpcode(u8),
    UnknownFlags(u8),
    ReservedNonZero { offset: u8 },
    PaddingNonZero { offset: u8 },
    InvalidRequest(RequestError),
}

/// Content-validation failures for the immutable request VMO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadError {
    LengthMismatch { expected: usize, actual: usize },
    InvalidUtf8,
    ContainsNul,
    AbsolutePath,
    TrailingSlash,
    EmptyComponent,
    CurrentDirectoryComponent,
    ParentDirectoryComponent,
    TooDeep,
}

/// Borrowed, validated slices from one immutable request VMO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestPayload<'a> {
    path: &'a str,
    value: &'a [u8],
}

impl<'a> RequestPayload<'a> {
    pub const fn path(self) -> &'a str {
        self.path
    }

    pub const fn value(self) -> &'a [u8] {
        self.value
    }
}

/// One canonical storage request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Request {
    opcode: Opcode,
    flags: RequestFlags,
    transaction_id: u64,
    path_length: u32,
    value_length: u32,
    cas_generation: u64,
    cursor: u64,
}

impl Request {
    pub fn bind(transaction_id: u64) -> Result<Self, RequestError> {
        Self::validated(
            Opcode::Bind,
            RequestFlags::PAYLOAD,
            transaction_id,
            0,
            0,
            0,
            0,
        )
    }

    pub fn create_directory(transaction_id: u64, path_length: usize) -> Result<Self, RequestError> {
        Self::validated(
            Opcode::CreateDirectory,
            RequestFlags::PAYLOAD,
            transaction_id,
            length_u32(path_length).ok_or(RequestError::PathTooLong)?,
            0,
            0,
            0,
        )
    }

    pub fn replace(
        transaction_id: u64,
        path_length: usize,
        value_length: usize,
        condition: ReplaceCondition,
    ) -> Result<Self, RequestError> {
        let generation = match condition {
            ReplaceCondition::Any | ReplaceCondition::CreateOnly => 0,
            ReplaceCondition::Exact(generation) => generation,
        };
        Self::validated(
            Opcode::Replace,
            RequestFlags::for_replace(condition),
            transaction_id,
            length_u32(path_length).ok_or(RequestError::PathTooLong)?,
            length_u32(value_length).ok_or(RequestError::ValueTooLong)?,
            generation,
            0,
        )
    }

    pub fn read(transaction_id: u64, path_length: usize) -> Result<Self, RequestError> {
        Self::validated(
            Opcode::Read,
            RequestFlags::PAYLOAD,
            transaction_id,
            length_u32(path_length).ok_or(RequestError::PathTooLong)?,
            0,
            0,
            0,
        )
    }

    /// Lists one entry at `cursor`. An empty path names the session root.
    pub fn list(
        transaction_id: u64,
        path_length: usize,
        cursor: u64,
    ) -> Result<Self, RequestError> {
        Self::validated(
            Opcode::List,
            RequestFlags::PAYLOAD,
            transaction_id,
            length_u32(path_length).ok_or(RequestError::PathTooLong)?,
            0,
            0,
            cursor,
        )
    }

    pub fn unlink(transaction_id: u64, path_length: usize) -> Result<Self, RequestError> {
        Self::validated(
            Opcode::Unlink,
            RequestFlags::PAYLOAD,
            transaction_id,
            length_u32(path_length).ok_or(RequestError::PathTooLong)?,
            0,
            0,
            0,
        )
    }

    fn validated(
        opcode: Opcode,
        flags: RequestFlags,
        transaction_id: u64,
        path_length: u32,
        value_length: u32,
        cas_generation: u64,
        cursor: u64,
    ) -> Result<Self, RequestError> {
        if transaction_id == 0 {
            return Err(RequestError::TransactionIdMustBeNonZero);
        }
        if !flags.has_payload_vmo() {
            return Err(RequestError::PayloadVmoRequired);
        }
        if path_length as usize > PATH_MAX_BYTES {
            return Err(RequestError::PathTooLong);
        }
        if value_length as usize > VALUE_MAX_BYTES {
            return Err(RequestError::ValueTooLong);
        }
        if path_length.checked_add(value_length).is_none() {
            return Err(RequestError::PayloadLengthOverflow);
        }

        match opcode {
            Opcode::Bind => {
                if path_length != 0 {
                    return Err(RequestError::UnexpectedPath);
                }
                if value_length != 0 {
                    return Err(RequestError::UnexpectedValue);
                }
                validate_non_replace(flags, cas_generation, cursor)?;
            }
            Opcode::CreateDirectory | Opcode::Read | Opcode::Unlink => {
                if path_length == 0 {
                    return Err(RequestError::PathRequired);
                }
                if value_length != 0 {
                    return Err(RequestError::UnexpectedValue);
                }
                validate_non_replace(flags, cas_generation, cursor)?;
            }
            Opcode::List => {
                if value_length != 0 {
                    return Err(RequestError::UnexpectedValue);
                }
                if flags != RequestFlags::PAYLOAD {
                    return Err(RequestError::ReplaceFlagsRequired);
                }
                if cas_generation != 0 {
                    return Err(RequestError::CasGenerationMustBeZero);
                }
                if cursor > DIRECTORY_MAX_ENTRIES {
                    return Err(RequestError::CursorOutOfRange);
                }
            }
            Opcode::Replace => {
                if path_length == 0 {
                    return Err(RequestError::PathRequired);
                }
                if cursor != 0 {
                    return Err(RequestError::UnexpectedCursor);
                }
                let create_only = flags.create_only();
                let exact = flags.exact_generation();
                if create_only && exact {
                    return Err(RequestError::ReplaceFlagsConflict);
                }
                if exact {
                    if cas_generation == 0 {
                        return Err(RequestError::ExactGenerationMustBeNonZero);
                    }
                } else if cas_generation != 0 {
                    return Err(RequestError::CasGenerationMustBeZero);
                }
            }
        }

        Ok(Self {
            opcode,
            flags,
            transaction_id,
            path_length,
            value_length,
            cas_generation,
            cursor,
        })
    }

    pub fn decode(bytes: &[u8; FRAME_SIZE]) -> Result<Self, RequestDecodeError> {
        if bytes[MAGIC_RANGE] != REQUEST_MAGIC {
            return Err(RequestDecodeError::BadMagic);
        }
        let version = read_u16(bytes, VERSION_RANGE);
        if version != ABI_VERSION {
            return Err(RequestDecodeError::UnsupportedVersion(version));
        }
        let opcode = Opcode::from_raw(bytes[OPCODE_OFFSET])
            .ok_or(RequestDecodeError::UnknownOpcode(bytes[OPCODE_OFFSET]))?;
        let flags = RequestFlags::from_bits(bytes[FLAGS_OFFSET])
            .ok_or(RequestDecodeError::UnknownFlags(bytes[FLAGS_OFFSET]))?;
        require_zero(bytes, REQUEST_RESERVED_RANGE, false).map_err(|offset| {
            RequestDecodeError::ReservedNonZero {
                offset: offset as u8,
            }
        })?;
        require_zero(bytes, REQUEST_PADDING_RANGE, true).map_err(|offset| {
            RequestDecodeError::PaddingNonZero {
                offset: offset as u8,
            }
        })?;
        Self::validated(
            opcode,
            flags,
            read_u64(bytes, TXID_RANGE),
            read_u32(bytes, REQUEST_PATH_LENGTH_RANGE),
            read_u32(bytes, REQUEST_VALUE_LENGTH_RANGE),
            read_u64(bytes, REQUEST_CAS_GENERATION_RANGE),
            read_u64(bytes, REQUEST_CURSOR_RANGE),
        )
        .map_err(RequestDecodeError::InvalidRequest)
    }

    pub fn encode(self) -> [u8; FRAME_SIZE] {
        let mut bytes = [0; FRAME_SIZE];
        bytes[MAGIC_RANGE].copy_from_slice(&REQUEST_MAGIC);
        write_u16(&mut bytes, VERSION_RANGE, ABI_VERSION);
        bytes[OPCODE_OFFSET] = self.opcode.raw();
        bytes[FLAGS_OFFSET] = self.flags.bits();
        write_u64(&mut bytes, TXID_RANGE, self.transaction_id);
        write_u32(&mut bytes, REQUEST_PATH_LENGTH_RANGE, self.path_length);
        write_u32(&mut bytes, REQUEST_VALUE_LENGTH_RANGE, self.value_length);
        write_u64(
            &mut bytes,
            REQUEST_CAS_GENERATION_RANGE,
            self.cas_generation,
        );
        write_u64(&mut bytes, REQUEST_CURSOR_RANGE, self.cursor);
        bytes
    }

    pub const fn opcode(self) -> Opcode {
        self.opcode
    }

    pub const fn transaction_id(self) -> u64 {
        self.transaction_id
    }

    pub const fn path_length(self) -> usize {
        self.path_length as usize
    }

    pub const fn value_length(self) -> usize {
        self.value_length as usize
    }

    pub const fn payload_length(self) -> usize {
        self.path_length as usize + self.value_length as usize
    }

    pub const fn cursor(self) -> u64 {
        self.cursor
    }

    pub const fn replace_condition(self) -> Option<ReplaceCondition> {
        if !matches!(self.opcode, Opcode::Replace) {
            return None;
        }
        if self.flags.create_only() {
            Some(ReplaceCondition::CreateOnly)
        } else if self.flags.exact_generation() {
            Some(ReplaceCondition::Exact(self.cas_generation))
        } else {
            Some(ReplaceCondition::Any)
        }
    }

    /// Validates and splits the immutable request VMO.
    ///
    /// The caller must separately prove that `payload` came from exactly one
    /// transferred immutable VMO. This crate has no handle or process types.
    pub fn validate_payload<'a>(
        self,
        payload: &'a [u8],
    ) -> Result<RequestPayload<'a>, PayloadError> {
        let expected = self.payload_length();
        if payload.len() != expected {
            return Err(PayloadError::LengthMismatch {
                expected,
                actual: payload.len(),
            });
        }
        let (path, value) = payload.split_at(self.path_length());
        let path = core::str::from_utf8(path).map_err(|_| PayloadError::InvalidUtf8)?;
        validate_path(self.opcode, path)?;
        Ok(RequestPayload { path, value })
    }
}

fn validate_non_replace(
    flags: RequestFlags,
    cas_generation: u64,
    cursor: u64,
) -> Result<(), RequestError> {
    if flags != RequestFlags::PAYLOAD {
        return Err(RequestError::ReplaceFlagsRequired);
    }
    if cas_generation != 0 {
        return Err(RequestError::CasGenerationMustBeZero);
    }
    if cursor != 0 {
        return Err(RequestError::UnexpectedCursor);
    }
    Ok(())
}

fn validate_path(opcode: Opcode, path: &str) -> Result<(), PayloadError> {
    if path.is_empty() {
        return if matches!(opcode, Opcode::Bind | Opcode::List) {
            Ok(())
        } else {
            Err(PayloadError::EmptyComponent)
        };
    }
    if path.as_bytes().contains(&0) {
        return Err(PayloadError::ContainsNul);
    }
    if path.starts_with('/') {
        return Err(PayloadError::AbsolutePath);
    }
    if path.ends_with('/') {
        return Err(PayloadError::TrailingSlash);
    }
    let mut depth = 0usize;
    for component in path.split('/') {
        if component.is_empty() {
            return Err(PayloadError::EmptyComponent);
        }
        if component == "." {
            return Err(PayloadError::CurrentDirectoryComponent);
        }
        if component == ".." {
            return Err(PayloadError::ParentDirectoryComponent);
        }
        depth += 1;
        if depth > PATH_MAX_DEPTH {
            return Err(PayloadError::TooDeep);
        }
    }
    Ok(())
}

/// Semantic errors in a response frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseError {
    TransactionIdMustBeNonZero,
    ErrorMustHaveNoResult,
    SuccessMustHaveResult,
    ResultDoesNotMatchOpcode,
    UnexpectedPayload,
    PayloadVmoRequired,
    PayloadTooLarge,
    GenerationMustBeNonZero,
    UnexpectedGeneration,
    UnexpectedCursor,
    CursorOutOfRange,
    UnexpectedObjectSize,
    ObjectTooLarge,
    EntryKindRequired,
    UnexpectedEntryKind,
    DirectoryMetadataMustBeZero,
    FileGenerationMustBeNonZero,
}

/// Strict wire-decoding failures for a response frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseDecodeError {
    BadMagic,
    UnsupportedVersion(u16),
    UnknownOpcode(u8),
    UnknownFlags(u8),
    UnknownStatus(u16),
    UnknownResultKind(u16),
    UnknownEntryKind(u8),
    ReservedNonZero { offset: u8 },
    PaddingNonZero { offset: u8 },
    InvalidResponse(ResponseError),
}

/// One canonical response. Optional VMO payload bytes are transferred out of
/// band and their exact length is recorded in `payload_length`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Response {
    opcode: Opcode,
    flags: ResponseFlags,
    transaction_id: u64,
    status: Status,
    result_kind: ResultKind,
    payload_length: u32,
    generation: u64,
    cursor: u64,
    object_size: u32,
    entry_kind: Option<EntryKind>,
}

impl Response {
    pub fn error(
        opcode: Opcode,
        transaction_id: u64,
        status: Status,
    ) -> Result<Self, ResponseError> {
        Self::validated(
            opcode,
            ResponseFlags::NONE,
            transaction_id,
            status,
            ResultKind::None,
            0,
            0,
            0,
            0,
            None,
        )
    }

    pub fn bound(transaction_id: u64) -> Result<Self, ResponseError> {
        Self::validated(
            Opcode::Bind,
            ResponseFlags::NONE,
            transaction_id,
            Status::Ok,
            ResultKind::Bound,
            0,
            0,
            0,
            0,
            None,
        )
    }

    pub fn mutation(
        opcode: Opcode,
        transaction_id: u64,
        generation: u64,
        object_size: usize,
    ) -> Result<Self, ResponseError> {
        Self::validated(
            opcode,
            ResponseFlags::NONE,
            transaction_id,
            Status::Ok,
            ResultKind::Mutation,
            0,
            generation,
            0,
            length_u32(object_size).ok_or(ResponseError::ObjectTooLarge)?,
            None,
        )
    }

    pub fn file(
        transaction_id: u64,
        generation: u64,
        value_length: usize,
    ) -> Result<Self, ResponseError> {
        let value_length = length_u32(value_length).ok_or(ResponseError::PayloadTooLarge)?;
        Self::validated(
            Opcode::Read,
            ResponseFlags::PAYLOAD,
            transaction_id,
            Status::Ok,
            ResultKind::File,
            value_length,
            generation,
            0,
            value_length,
            None,
        )
    }

    pub fn directory_entry(
        transaction_id: u64,
        next_cursor: u64,
        path_length: usize,
        entry_kind: EntryKind,
        generation: u64,
        object_size: usize,
    ) -> Result<Self, ResponseError> {
        Self::validated(
            Opcode::List,
            ResponseFlags::PAYLOAD,
            transaction_id,
            Status::Ok,
            ResultKind::DirectoryEntry,
            length_u32(path_length).ok_or(ResponseError::PayloadTooLarge)?,
            generation,
            next_cursor,
            length_u32(object_size).ok_or(ResponseError::ObjectTooLarge)?,
            Some(entry_kind),
        )
    }

    pub fn end_of_directory(transaction_id: u64, cursor: u64) -> Result<Self, ResponseError> {
        Self::validated(
            Opcode::List,
            ResponseFlags::NONE,
            transaction_id,
            Status::Ok,
            ResultKind::EndOfDirectory,
            0,
            0,
            cursor,
            0,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn validated(
        opcode: Opcode,
        flags: ResponseFlags,
        transaction_id: u64,
        status: Status,
        result_kind: ResultKind,
        payload_length: u32,
        generation: u64,
        cursor: u64,
        object_size: u32,
        entry_kind: Option<EntryKind>,
    ) -> Result<Self, ResponseError> {
        if transaction_id == 0 {
            return Err(ResponseError::TransactionIdMustBeNonZero);
        }
        if status != Status::Ok {
            if result_kind != ResultKind::None
                || flags != ResponseFlags::NONE
                || payload_length != 0
                || generation != 0
                || cursor != 0
                || object_size != 0
                || entry_kind.is_some()
            {
                return Err(ResponseError::ErrorMustHaveNoResult);
            }
            return Ok(Self {
                opcode,
                flags,
                transaction_id,
                status,
                result_kind,
                payload_length,
                generation,
                cursor,
                object_size,
                entry_kind,
            });
        }
        if result_kind == ResultKind::None {
            return Err(ResponseError::SuccessMustHaveResult);
        }

        match result_kind {
            ResultKind::None => return Err(ResponseError::SuccessMustHaveResult),
            ResultKind::Bound => {
                if opcode != Opcode::Bind {
                    return Err(ResponseError::ResultDoesNotMatchOpcode);
                }
                require_empty_result(
                    flags,
                    payload_length,
                    generation,
                    cursor,
                    object_size,
                    entry_kind,
                )?;
            }
            ResultKind::Mutation => {
                if !matches!(
                    opcode,
                    Opcode::CreateDirectory | Opcode::Replace | Opcode::Unlink
                ) {
                    return Err(ResponseError::ResultDoesNotMatchOpcode);
                }
                if flags != ResponseFlags::NONE || payload_length != 0 {
                    return Err(ResponseError::UnexpectedPayload);
                }
                if generation == 0 {
                    return Err(ResponseError::GenerationMustBeNonZero);
                }
                if cursor != 0 {
                    return Err(ResponseError::UnexpectedCursor);
                }
                if object_size as usize > VALUE_MAX_BYTES {
                    return Err(ResponseError::ObjectTooLarge);
                }
                if opcode != Opcode::Replace && object_size != 0 {
                    return Err(ResponseError::UnexpectedObjectSize);
                }
                if entry_kind.is_some() {
                    return Err(ResponseError::UnexpectedEntryKind);
                }
            }
            ResultKind::File => {
                if opcode != Opcode::Read {
                    return Err(ResponseError::ResultDoesNotMatchOpcode);
                }
                if flags != ResponseFlags::PAYLOAD {
                    return Err(ResponseError::PayloadVmoRequired);
                }
                if payload_length as usize > VALUE_MAX_BYTES {
                    return Err(ResponseError::PayloadTooLarge);
                }
                if generation == 0 {
                    return Err(ResponseError::GenerationMustBeNonZero);
                }
                if cursor != 0 {
                    return Err(ResponseError::UnexpectedCursor);
                }
                if object_size != payload_length {
                    return Err(ResponseError::UnexpectedObjectSize);
                }
                if entry_kind.is_some() {
                    return Err(ResponseError::UnexpectedEntryKind);
                }
            }
            ResultKind::DirectoryEntry => {
                if opcode != Opcode::List {
                    return Err(ResponseError::ResultDoesNotMatchOpcode);
                }
                if flags != ResponseFlags::PAYLOAD {
                    return Err(ResponseError::PayloadVmoRequired);
                }
                if payload_length == 0 || payload_length as usize > PATH_MAX_BYTES {
                    return Err(ResponseError::PayloadTooLarge);
                }
                if cursor == 0 || cursor > DIRECTORY_MAX_ENTRIES {
                    return Err(ResponseError::CursorOutOfRange);
                }
                match entry_kind {
                    None => return Err(ResponseError::EntryKindRequired),
                    Some(EntryKind::Directory) => {
                        if generation != 0 || object_size != 0 {
                            return Err(ResponseError::DirectoryMetadataMustBeZero);
                        }
                    }
                    Some(EntryKind::File) => {
                        if generation == 0 {
                            return Err(ResponseError::FileGenerationMustBeNonZero);
                        }
                        if object_size as usize > VALUE_MAX_BYTES {
                            return Err(ResponseError::ObjectTooLarge);
                        }
                    }
                }
            }
            ResultKind::EndOfDirectory => {
                if opcode != Opcode::List {
                    return Err(ResponseError::ResultDoesNotMatchOpcode);
                }
                if cursor > DIRECTORY_MAX_ENTRIES {
                    return Err(ResponseError::CursorOutOfRange);
                }
                if flags != ResponseFlags::NONE || payload_length != 0 {
                    return Err(ResponseError::UnexpectedPayload);
                }
                if generation != 0 {
                    return Err(ResponseError::UnexpectedGeneration);
                }
                if object_size != 0 {
                    return Err(ResponseError::UnexpectedObjectSize);
                }
                if entry_kind.is_some() {
                    return Err(ResponseError::UnexpectedEntryKind);
                }
            }
        }

        Ok(Self {
            opcode,
            flags,
            transaction_id,
            status,
            result_kind,
            payload_length,
            generation,
            cursor,
            object_size,
            entry_kind,
        })
    }

    pub fn decode(bytes: &[u8; FRAME_SIZE]) -> Result<Self, ResponseDecodeError> {
        if bytes[MAGIC_RANGE] != RESPONSE_MAGIC {
            return Err(ResponseDecodeError::BadMagic);
        }
        let version = read_u16(bytes, VERSION_RANGE);
        if version != ABI_VERSION {
            return Err(ResponseDecodeError::UnsupportedVersion(version));
        }
        let opcode = Opcode::from_raw(bytes[OPCODE_OFFSET])
            .ok_or(ResponseDecodeError::UnknownOpcode(bytes[OPCODE_OFFSET]))?;
        let flags = ResponseFlags::from_bits(bytes[FLAGS_OFFSET])
            .ok_or(ResponseDecodeError::UnknownFlags(bytes[FLAGS_OFFSET]))?;
        let status_raw = read_u16(bytes, RESPONSE_STATUS_RANGE);
        let status =
            Status::from_raw(status_raw).ok_or(ResponseDecodeError::UnknownStatus(status_raw))?;
        let result_raw = read_u16(bytes, RESPONSE_RESULT_RANGE);
        let result_kind = ResultKind::from_raw(result_raw)
            .ok_or(ResponseDecodeError::UnknownResultKind(result_raw))?;
        let entry_raw = bytes[RESPONSE_ENTRY_KIND_OFFSET];
        let entry_kind = if entry_raw == 0 {
            None
        } else {
            Some(
                EntryKind::from_raw(entry_raw)
                    .ok_or(ResponseDecodeError::UnknownEntryKind(entry_raw))?,
            )
        };
        require_zero(bytes, RESPONSE_RESERVED_RANGE, false).map_err(|offset| {
            ResponseDecodeError::ReservedNonZero {
                offset: offset as u8,
            }
        })?;
        require_zero(bytes, RESPONSE_PADDING_RANGE, true).map_err(|offset| {
            ResponseDecodeError::PaddingNonZero {
                offset: offset as u8,
            }
        })?;
        Self::validated(
            opcode,
            flags,
            read_u64(bytes, TXID_RANGE),
            status,
            result_kind,
            read_u32(bytes, RESPONSE_PAYLOAD_LENGTH_RANGE),
            read_u64(bytes, RESPONSE_GENERATION_RANGE),
            read_u64(bytes, RESPONSE_CURSOR_RANGE),
            read_u32(bytes, RESPONSE_OBJECT_SIZE_RANGE),
            entry_kind,
        )
        .map_err(ResponseDecodeError::InvalidResponse)
    }

    pub fn encode(self) -> [u8; FRAME_SIZE] {
        let mut bytes = [0; FRAME_SIZE];
        bytes[MAGIC_RANGE].copy_from_slice(&RESPONSE_MAGIC);
        write_u16(&mut bytes, VERSION_RANGE, ABI_VERSION);
        bytes[OPCODE_OFFSET] = self.opcode.raw();
        bytes[FLAGS_OFFSET] = self.flags.bits();
        write_u64(&mut bytes, TXID_RANGE, self.transaction_id);
        write_u16(&mut bytes, RESPONSE_STATUS_RANGE, self.status.raw());
        write_u16(&mut bytes, RESPONSE_RESULT_RANGE, self.result_kind.raw());
        write_u32(
            &mut bytes,
            RESPONSE_PAYLOAD_LENGTH_RANGE,
            self.payload_length,
        );
        write_u64(&mut bytes, RESPONSE_GENERATION_RANGE, self.generation);
        write_u64(&mut bytes, RESPONSE_CURSOR_RANGE, self.cursor);
        write_u32(&mut bytes, RESPONSE_OBJECT_SIZE_RANGE, self.object_size);
        bytes[RESPONSE_ENTRY_KIND_OFFSET] = self.entry_kind.map_or(0, EntryKind::raw);
        bytes
    }

    pub const fn opcode(self) -> Opcode {
        self.opcode
    }

    pub const fn transaction_id(self) -> u64 {
        self.transaction_id
    }

    pub const fn status(self) -> Status {
        self.status
    }

    pub const fn result_kind(self) -> ResultKind {
        self.result_kind
    }

    pub const fn has_payload_vmo(self) -> bool {
        self.flags.has_payload_vmo()
    }

    pub const fn payload_length(self) -> usize {
        self.payload_length as usize
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn cursor(self) -> u64 {
        self.cursor
    }

    pub const fn object_size(self) -> usize {
        self.object_size as usize
    }

    pub const fn entry_kind(self) -> Option<EntryKind> {
        self.entry_kind
    }

    /// Validates a response VMO after its handle type and immutability have
    /// been checked by the service client transport.
    pub fn validate_payload(self, payload: &[u8]) -> Result<(), PayloadError> {
        let expected = self.payload_length();
        if payload.len() != expected {
            return Err(PayloadError::LengthMismatch {
                expected,
                actual: payload.len(),
            });
        }
        if self.result_kind == ResultKind::DirectoryEntry {
            let path = core::str::from_utf8(payload).map_err(|_| PayloadError::InvalidUtf8)?;
            validate_path(Opcode::List, path)?;
        }
        Ok(())
    }
}

fn require_empty_result(
    flags: ResponseFlags,
    payload_length: u32,
    generation: u64,
    cursor: u64,
    object_size: u32,
    entry_kind: Option<EntryKind>,
) -> Result<(), ResponseError> {
    if flags != ResponseFlags::NONE || payload_length != 0 {
        return Err(ResponseError::UnexpectedPayload);
    }
    if generation != 0 {
        return Err(ResponseError::UnexpectedGeneration);
    }
    if cursor != 0 {
        return Err(ResponseError::UnexpectedCursor);
    }
    if object_size != 0 {
        return Err(ResponseError::UnexpectedObjectSize);
    }
    if entry_kind.is_some() {
        return Err(ResponseError::UnexpectedEntryKind);
    }
    Ok(())
}

const fn length_u32(length: usize) -> Option<u32> {
    if length <= u32::MAX as usize {
        Some(length as u32)
    } else {
        None
    }
}

fn require_zero(
    bytes: &[u8; FRAME_SIZE],
    range: Range<usize>,
    _padding: bool,
) -> Result<(), usize> {
    for offset in range {
        if bytes[offset] != 0 {
            return Err(offset);
        }
    }
    Ok(())
}

fn read_u16(bytes: &[u8; FRAME_SIZE], range: Range<usize>) -> u16 {
    u16::from_le_bytes([bytes[range.start], bytes[range.start + 1]])
}

fn read_u32(bytes: &[u8; FRAME_SIZE], range: Range<usize>) -> u32 {
    u32::from_le_bytes([
        bytes[range.start],
        bytes[range.start + 1],
        bytes[range.start + 2],
        bytes[range.start + 3],
    ])
}

fn read_u64(bytes: &[u8; FRAME_SIZE], range: Range<usize>) -> u64 {
    u64::from_le_bytes([
        bytes[range.start],
        bytes[range.start + 1],
        bytes[range.start + 2],
        bytes[range.start + 3],
        bytes[range.start + 4],
        bytes[range.start + 5],
        bytes[range.start + 6],
        bytes[range.start + 7],
    ])
}

fn write_u16(bytes: &mut [u8; FRAME_SIZE], range: Range<usize>, value: u16) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8; FRAME_SIZE], range: Range<usize>, value: u32) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8; FRAME_SIZE], range: Range<usize>, value: u64) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests;
