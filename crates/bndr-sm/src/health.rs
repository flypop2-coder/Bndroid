//! Strict, allocation-free service health protocol and fixed-capacity service
//! supervisor.
//!
//! `BSH1` frames are always exactly 64 bytes. Service identity is the tuple of
//! service kind, process generation, and process ID. Sequence numbers are
//! scoped to that identity and tracked independently in each direction.

use core::array;

pub const HEALTH_FRAME_SIZE: usize = 64;
pub const HEALTH_PROTOCOL_MAGIC: [u8; 4] = *b"BSH1";
pub const HEALTH_PROTOCOL_VERSION: u8 = 1;
pub const DEFAULT_SUPERVISOR_CAPACITY: usize = 3;
/// Maximum number of consecutive expired probes that one bounded policy may
/// tolerate before the next miss is classified as a service fault.
pub const MAX_MISSED_PROBES_TOLERATED: u32 = 8;

const MAGIC_RANGE: core::ops::Range<usize> = 0..4;
const VERSION_OFFSET: usize = 4;
const OPCODE_OFFSET: usize = 5;
const FLAGS_OFFSET: usize = 6;
const SERVICE_KIND_OFFSET: usize = 7;
const SEQUENCE_RANGE: core::ops::Range<usize> = 8..16;
const GENERATION_RANGE: core::ops::Range<usize> = 16..20;
const HEADER_RESERVED_RANGE: core::ops::Range<usize> = 20..24;
const PID_RANGE: core::ops::Range<usize> = 24..32;
const FAULT_CLASS_OFFSET: usize = 32;
const SEMANTIC_RESERVED_RANGE: core::ops::Range<usize> = 33..36;
const RESTART_ATTEMPT_RANGE: core::ops::Range<usize> = 36..40;
const RESTART_BUDGET_RANGE: core::ops::Range<usize> = 40..44;
const INTERVAL_NS_RANGE: core::ops::Range<usize> = 44..52;
const PADDING_RANGE: core::ops::Range<usize> = 52..64;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServiceKind {
    ServiceManager = 1,
    SurfaceServer = 2,
    InputServer = 3,
    StorageServer = 4,
    App = 5,
}

impl ServiceKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::ServiceManager),
            2 => Some(Self::SurfaceServer),
            3 => Some(Self::InputServer),
            4 => Some(Self::StorageServer),
            5 => Some(Self::App),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthOpcode {
    Probe = 1,
    Healthy = 2,
    Fault = 3,
    Quarantine = 4,
    DegradedAck = 5,
}

impl HealthOpcode {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Probe),
            2 => Some(Self::Healthy),
            3 => Some(Self::Fault),
            4 => Some(Self::Quarantine),
            5 => Some(Self::DegradedAck),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultClass {
    ProcessExit = 1,
    HealthTimeout = 2,
    ProtocolViolation = 3,
    ReplacementFailed = 4,
    RestartBudgetExhausted = 5,
}

impl FaultClass {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::ProcessExit),
            2 => Some(Self::HealthTimeout),
            3 => Some(Self::ProtocolViolation),
            4 => Some(Self::ReplacementFailed),
            5 => Some(Self::RestartBudgetExhausted),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthFlags(u8);

impl HealthFlags {
    pub const NONE: Self = Self(0);
    pub const ACK_REQUIRED: Self = Self(1);

    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Self::NONE),
            1 => Some(Self::ACK_REQUIRED),
            _ => None,
        }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn ack_required(self) -> bool {
        self.0 & Self::ACK_REQUIRED.0 != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceIdentityError {
    GenerationMustBeNonZero,
    PidMustBeNonZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceIdentity {
    kind: ServiceKind,
    generation: u32,
    pid: u64,
}

impl ServiceIdentity {
    pub const fn new(
        kind: ServiceKind,
        generation: u32,
        pid: u64,
    ) -> Result<Self, ServiceIdentityError> {
        if generation == 0 {
            return Err(ServiceIdentityError::GenerationMustBeNonZero);
        }
        if pid == 0 {
            return Err(ServiceIdentityError::PidMustBeNonZero);
        }
        Ok(Self {
            kind,
            generation,
            pid,
        })
    }

    pub const fn kind(self) -> ServiceKind {
        self.kind
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn pid(self) -> u64 {
        self.pid
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthFrameError {
    SequenceMustBeNonZero,
    Identity(ServiceIdentityError),
    FlagsMismatch { expected: u8, actual: u8 },
    FaultRequired,
    FaultForbidden,
    RestartBudgetExhaustedOnlyForQuarantine,
    QuarantineFaultRequired,
    RestartAttemptMustBeZero,
    RestartBudgetMustBeZero,
    RestartAttemptMustExceedBudgetByOne,
    RestartBudgetMustBeNonZero,
    IntervalMustBeZero,
    IntervalMustBeNonZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthDecodeError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownOpcode(u8),
    UnknownFlags(u8),
    UnknownServiceKind(u8),
    UnknownFaultClass(u8),
    ReservedNonZero { offset: u8 },
    PaddingNonZero { offset: u8 },
    InvalidFrame(HealthFrameError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthFrame {
    opcode: HealthOpcode,
    flags: HealthFlags,
    sequence: u64,
    identity: ServiceIdentity,
    fault_class: Option<FaultClass>,
    restart_attempt: u32,
    restart_budget: u32,
    interval_ns: u64,
}

impl HealthFrame {
    pub fn probe(
        sequence: u64,
        identity: ServiceIdentity,
        health_timeout_ns: u64,
    ) -> Result<Self, HealthFrameError> {
        Self::validated(
            HealthOpcode::Probe,
            HealthFlags::ACK_REQUIRED,
            sequence,
            identity,
            None,
            0,
            0,
            health_timeout_ns,
        )
    }

    pub fn healthy(sequence: u64, identity: ServiceIdentity) -> Result<Self, HealthFrameError> {
        Self::validated(
            HealthOpcode::Healthy,
            HealthFlags::NONE,
            sequence,
            identity,
            None,
            0,
            0,
            0,
        )
    }

    pub fn fault(
        sequence: u64,
        identity: ServiceIdentity,
        fault_class: FaultClass,
    ) -> Result<Self, HealthFrameError> {
        Self::validated(
            HealthOpcode::Fault,
            HealthFlags::NONE,
            sequence,
            identity,
            Some(fault_class),
            0,
            0,
            0,
        )
    }

    pub fn quarantine(
        sequence: u64,
        identity: ServiceIdentity,
        restart_attempt: u32,
        restart_budget: u32,
    ) -> Result<Self, HealthFrameError> {
        Self::validated(
            HealthOpcode::Quarantine,
            HealthFlags::ACK_REQUIRED,
            sequence,
            identity,
            Some(FaultClass::RestartBudgetExhausted),
            restart_attempt,
            restart_budget,
            0,
        )
    }

    pub fn degraded_ack(
        sequence: u64,
        identity: ServiceIdentity,
    ) -> Result<Self, HealthFrameError> {
        Self::validated(
            HealthOpcode::DegradedAck,
            HealthFlags::NONE,
            sequence,
            identity,
            None,
            0,
            0,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn validated(
        opcode: HealthOpcode,
        flags: HealthFlags,
        sequence: u64,
        identity: ServiceIdentity,
        fault_class: Option<FaultClass>,
        restart_attempt: u32,
        restart_budget: u32,
        interval_ns: u64,
    ) -> Result<Self, HealthFrameError> {
        if sequence == 0 {
            return Err(HealthFrameError::SequenceMustBeNonZero);
        }
        ServiceIdentity::new(identity.kind, identity.generation, identity.pid)
            .map_err(HealthFrameError::Identity)?;

        let expected_flags = match opcode {
            HealthOpcode::Probe | HealthOpcode::Quarantine => HealthFlags::ACK_REQUIRED,
            HealthOpcode::Healthy | HealthOpcode::Fault | HealthOpcode::DegradedAck => {
                HealthFlags::NONE
            }
        };
        if flags != expected_flags {
            return Err(HealthFrameError::FlagsMismatch {
                expected: expected_flags.bits(),
                actual: flags.bits(),
            });
        }

        match opcode {
            HealthOpcode::Probe => {
                require_no_fault(fault_class)?;
                require_zero_restart(restart_attempt, restart_budget)?;
                if interval_ns == 0 {
                    return Err(HealthFrameError::IntervalMustBeNonZero);
                }
            }
            HealthOpcode::Healthy | HealthOpcode::DegradedAck => {
                require_no_fault(fault_class)?;
                require_zero_restart(restart_attempt, restart_budget)?;
                require_zero_interval(interval_ns)?;
            }
            HealthOpcode::Fault => {
                let fault_class = fault_class.ok_or(HealthFrameError::FaultRequired)?;
                if fault_class == FaultClass::RestartBudgetExhausted {
                    return Err(HealthFrameError::RestartBudgetExhaustedOnlyForQuarantine);
                }
                require_zero_restart(restart_attempt, restart_budget)?;
                require_zero_interval(interval_ns)?;
            }
            HealthOpcode::Quarantine => {
                if fault_class != Some(FaultClass::RestartBudgetExhausted) {
                    return Err(HealthFrameError::QuarantineFaultRequired);
                }
                if restart_budget == 0 {
                    return Err(HealthFrameError::RestartBudgetMustBeNonZero);
                }
                if restart_budget.checked_add(1) != Some(restart_attempt) {
                    return Err(HealthFrameError::RestartAttemptMustExceedBudgetByOne);
                }
                require_zero_interval(interval_ns)?;
            }
        }

        Ok(Self {
            opcode,
            flags,
            sequence,
            identity,
            fault_class,
            restart_attempt,
            restart_budget,
            interval_ns,
        })
    }

    pub fn decode(bytes: &[u8; HEALTH_FRAME_SIZE]) -> Result<Self, HealthDecodeError> {
        if bytes[MAGIC_RANGE] != HEALTH_PROTOCOL_MAGIC {
            return Err(HealthDecodeError::BadMagic);
        }
        if bytes[VERSION_OFFSET] != HEALTH_PROTOCOL_VERSION {
            return Err(HealthDecodeError::UnsupportedVersion(bytes[VERSION_OFFSET]));
        }
        let opcode = HealthOpcode::from_raw(bytes[OPCODE_OFFSET])
            .ok_or(HealthDecodeError::UnknownOpcode(bytes[OPCODE_OFFSET]))?;
        let flags = HealthFlags::from_bits(bytes[FLAGS_OFFSET])
            .ok_or(HealthDecodeError::UnknownFlags(bytes[FLAGS_OFFSET]))?;
        let kind = ServiceKind::from_raw(bytes[SERVICE_KIND_OFFSET]).ok_or(
            HealthDecodeError::UnknownServiceKind(bytes[SERVICE_KIND_OFFSET]),
        )?;
        let sequence = read_u64(bytes, SEQUENCE_RANGE);
        let generation = read_u32(bytes, GENERATION_RANGE);
        let pid = read_u64(bytes, PID_RANGE);
        let raw_fault = bytes[FAULT_CLASS_OFFSET];
        let fault_class = if raw_fault == 0 {
            None
        } else {
            Some(
                FaultClass::from_raw(raw_fault)
                    .ok_or(HealthDecodeError::UnknownFaultClass(raw_fault))?,
            )
        };
        let restart_attempt = read_u32(bytes, RESTART_ATTEMPT_RANGE);
        let restart_budget = read_u32(bytes, RESTART_BUDGET_RANGE);
        let interval_ns = read_u64(bytes, INTERVAL_NS_RANGE);

        validate_zeroes(bytes, HEADER_RESERVED_RANGE, false)?;
        validate_zeroes(bytes, SEMANTIC_RESERVED_RANGE, false)?;
        validate_zeroes(bytes, PADDING_RANGE, true)?;

        let identity = ServiceIdentity::new(kind, generation, pid)
            .map_err(|error| HealthDecodeError::InvalidFrame(HealthFrameError::Identity(error)))?;
        Self::validated(
            opcode,
            flags,
            sequence,
            identity,
            fault_class,
            restart_attempt,
            restart_budget,
            interval_ns,
        )
        .map_err(HealthDecodeError::InvalidFrame)
    }

    pub fn encode(self) -> [u8; HEALTH_FRAME_SIZE] {
        let mut bytes = [0_u8; HEALTH_FRAME_SIZE];
        bytes[MAGIC_RANGE].copy_from_slice(&HEALTH_PROTOCOL_MAGIC);
        bytes[VERSION_OFFSET] = HEALTH_PROTOCOL_VERSION;
        bytes[OPCODE_OFFSET] = self.opcode.raw();
        bytes[FLAGS_OFFSET] = self.flags.bits();
        bytes[SERVICE_KIND_OFFSET] = self.identity.kind.raw();
        write_u64(&mut bytes, SEQUENCE_RANGE, self.sequence);
        write_u32(&mut bytes, GENERATION_RANGE, self.identity.generation);
        write_u64(&mut bytes, PID_RANGE, self.identity.pid);
        if let Some(fault_class) = self.fault_class {
            bytes[FAULT_CLASS_OFFSET] = fault_class.raw();
        }
        write_u32(&mut bytes, RESTART_ATTEMPT_RANGE, self.restart_attempt);
        write_u32(&mut bytes, RESTART_BUDGET_RANGE, self.restart_budget);
        write_u64(&mut bytes, INTERVAL_NS_RANGE, self.interval_ns);
        bytes
    }

    pub const fn opcode(self) -> HealthOpcode {
        self.opcode
    }

    pub const fn flags(self) -> HealthFlags {
        self.flags
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn identity(self) -> ServiceIdentity {
        self.identity
    }

    pub const fn fault_class(self) -> Option<FaultClass> {
        self.fault_class
    }

    pub const fn restart_attempt(self) -> u32 {
        self.restart_attempt
    }

    pub const fn restart_budget(self) -> u32 {
        self.restart_budget
    }

    pub const fn interval_ns(self) -> u64 {
        self.interval_ns
    }
}

fn require_no_fault(fault_class: Option<FaultClass>) -> Result<(), HealthFrameError> {
    if fault_class.is_none() {
        Ok(())
    } else {
        Err(HealthFrameError::FaultForbidden)
    }
}

fn require_zero_restart(restart_attempt: u32, restart_budget: u32) -> Result<(), HealthFrameError> {
    if restart_attempt != 0 {
        return Err(HealthFrameError::RestartAttemptMustBeZero);
    }
    if restart_budget != 0 {
        return Err(HealthFrameError::RestartBudgetMustBeZero);
    }
    Ok(())
}

fn require_zero_interval(interval_ns: u64) -> Result<(), HealthFrameError> {
    if interval_ns == 0 {
        Ok(())
    } else {
        Err(HealthFrameError::IntervalMustBeZero)
    }
}

fn validate_zeroes(
    bytes: &[u8; HEALTH_FRAME_SIZE],
    range: core::ops::Range<usize>,
    padding: bool,
) -> Result<(), HealthDecodeError> {
    for (offset, byte) in bytes[range.clone()].iter().copied().enumerate() {
        if byte != 0 {
            let offset = (range.start + offset) as u8;
            return Err(if padding {
                HealthDecodeError::PaddingNonZero { offset }
            } else {
                HealthDecodeError::ReservedNonZero { offset }
            });
        }
    }
    Ok(())
}

fn read_u32(bytes: &[u8; HEALTH_FRAME_SIZE], range: core::ops::Range<usize>) -> u32 {
    u32::from_le_bytes(bytes[range].try_into().expect("u32 wire range"))
}

fn write_u32(bytes: &mut [u8; HEALTH_FRAME_SIZE], range: core::ops::Range<usize>, value: u32) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

fn read_u64(bytes: &[u8; HEALTH_FRAME_SIZE], range: core::ops::Range<usize>) -> u64 {
    u64::from_le_bytes(bytes[range].try_into().expect("u64 wire range"))
}

fn write_u64(bytes: &mut [u8; HEALTH_FRAME_SIZE], range: core::ops::Range<usize>, value: u64) {
    bytes[range].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceError {
    Zero,
    NotIncreasing { last: u64, received: u64 },
    Exhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BidirectionalSequenceTracker {
    last_outbound: u64,
    last_inbound: u64,
}

impl BidirectionalSequenceTracker {
    pub const fn new() -> Self {
        Self {
            last_outbound: 0,
            last_inbound: 0,
        }
    }

    pub fn next_outbound(&mut self) -> Result<u64, SequenceError> {
        let next = self
            .last_outbound
            .checked_add(1)
            .ok_or(SequenceError::Exhausted)?;
        self.last_outbound = next;
        Ok(next)
    }

    pub fn accept_inbound(&mut self, received: u64) -> Result<(), SequenceError> {
        if received == 0 {
            return Err(SequenceError::Zero);
        }
        if received <= self.last_inbound {
            return Err(SequenceError::NotIncreasing {
                last: self.last_inbound,
                received,
            });
        }
        self.last_inbound = received;
        Ok(())
    }

    pub const fn last_outbound(self) -> u64 {
        self.last_outbound
    }

    pub const fn last_inbound(self) -> u64 {
        self.last_inbound
    }
}

impl Default for BidirectionalSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServicePolicyError {
    RestartBudgetMustBeNonZero,
    RestartBudgetTooLarge,
    RestartBackoffMustBeNonZero,
    HealthTimeoutMustBeNonZero,
    MissedProbeToleranceTooLarge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServicePolicy {
    restart_budget: u32,
    restart_backoff_ns: u64,
    health_timeout_ns: u64,
    missed_probe_tolerance: u32,
}

impl ServicePolicy {
    pub const fn new(
        restart_budget: u32,
        restart_backoff_ns: u64,
        health_timeout_ns: u64,
    ) -> Result<Self, ServicePolicyError> {
        if restart_budget == 0 {
            return Err(ServicePolicyError::RestartBudgetMustBeNonZero);
        }
        if restart_budget == u32::MAX {
            return Err(ServicePolicyError::RestartBudgetTooLarge);
        }
        if restart_backoff_ns == 0 {
            return Err(ServicePolicyError::RestartBackoffMustBeNonZero);
        }
        if health_timeout_ns == 0 {
            return Err(ServicePolicyError::HealthTimeoutMustBeNonZero);
        }
        Ok(Self {
            restart_budget,
            restart_backoff_ns,
            health_timeout_ns,
            missed_probe_tolerance: 0,
        })
    }

    pub const fn with_missed_probe_tolerance(
        mut self,
        missed_probe_tolerance: u32,
    ) -> Result<Self, ServicePolicyError> {
        if missed_probe_tolerance > MAX_MISSED_PROBES_TOLERATED {
            return Err(ServicePolicyError::MissedProbeToleranceTooLarge);
        }
        self.missed_probe_tolerance = missed_probe_tolerance;
        Ok(self)
    }

    pub const fn restart_budget(self) -> u32 {
        self.restart_budget
    }

    pub const fn restart_backoff_ns(self) -> u64 {
        self.restart_backoff_ns
    }

    pub const fn health_timeout_ns(self) -> u64 {
        self.health_timeout_ns
    }

    pub const fn missed_probe_tolerance(self) -> u32 {
        self.missed_probe_tolerance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServicePhase {
    Registered,
    ProbeArmed,
    Healthy,
    Faulted,
    Backoff,
    AwaitingReplacement,
    QuarantinePending,
    Quarantined,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorOperation {
    ArmProbe,
    RecordHealthy,
    RecordFault,
    BeginBackoff,
    BeginReplacement,
    InstallReplacement,
    RearmRestartBudget,
    Quarantine,
    DegradedAck,
}

/// A replacement must answer this many consecutive probes before a caller may
/// open a fresh restart-budget window. This prevents an immediately flapping
/// generation from laundering a consumed budget through a single reply.
pub const RECOVERY_STABILITY_HEALTHY_ROUNDS: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultDisposition {
    RestartAllowed {
        attempt: u32,
        budget: u32,
        class: FaultClass,
    },
    QuarantineRequired {
        attempt: u32,
        budget: u32,
        class: FaultClass,
    },
}

/// Result of evaluating one armed probe against its absolute deadline.
///
/// `ToleratedMiss` is deliberately distinct from `Pending`: callers can audit
/// a real expired heartbeat without immediately replacing a still-live
/// process. The next miss beyond the configured tolerance is classified
/// through the ordinary bounded restart/quarantine path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthDeadlineOutcome {
    Pending { deadline_ns: u64 },
    ToleratedMiss { consecutive: u32, tolerance: u32 },
    Fault(FaultDisposition),
}

/// Strength of a directed dependency from a prerequisite service to a
/// dependent service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyKind {
    /// The dependent cannot safely run while the prerequisite is unavailable.
    Hard,
    /// The dependent remains live, but must expose degraded behaviour.
    Soft,
}

/// Effective dependency state of a registered service.
///
/// Multiple failed roots may affect the same service. `HardBlocked` always
/// dominates `SoftDegraded`, and an impact is removed only after every root
/// contributing that impact has recovered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyImpact {
    Unaffected,
    SoftDegraded,
    HardBlocked,
}

impl DependencyImpact {
    const fn rank(self) -> u8 {
        match self {
            Self::Unaffected => 0,
            Self::SoftDegraded => 1,
            Self::HardBlocked => 2,
        }
    }

    const fn strongest(self, other: Self) -> Self {
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyError {
    Full,
    ServiceNotFound {
        kind: ServiceKind,
    },
    IdentityMismatch {
        expected: ServiceIdentity,
        actual: ServiceIdentity,
    },
    SelfDependency {
        service: ServiceKind,
    },
    Duplicate {
        prerequisite: ServiceKind,
        dependent: ServiceKind,
    },
    Cycle {
        prerequisite: ServiceKind,
        dependent: ServiceKind,
    },
    TopologyActive,
    RootStillAvailable {
        kind: ServiceKind,
        phase: ServicePhase,
    },
    RootNotHealthy {
        kind: ServiceKind,
        phase: ServicePhase,
    },
}

/// One deterministic runtime action caused by an effective dependency-state
/// change. Callers map `HardBlocked` to quiesce/block and `SoftDegraded` to a
/// live degraded mode; transitions back to `Unaffected` resume normal service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyTransition {
    pub service: ServiceIdentity,
    pub previous: DependencyImpact,
    pub current: DependencyImpact,
}

/// Allocation-free transition batch. Entries are ordered topologically, with
/// prerequisites before dependents and `ServiceKind::raw()` breaking ties.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyTransitions<const CAPACITY: usize> {
    entries: [Option<DependencyTransition>; CAPACITY],
    length: usize,
}

impl<const CAPACITY: usize> DependencyTransitions<CAPACITY> {
    fn new() -> Self {
        Self {
            entries: array::from_fn(|_| None),
            length: 0,
        }
    }

    fn push(&mut self, transition: DependencyTransition) {
        debug_assert!(self.length < CAPACITY);
        self.entries[self.length] = Some(transition);
        self.length += 1;
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn get(&self, index: usize) -> Option<DependencyTransition> {
        self.entries.get(index).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = DependencyTransition> + '_ {
        self.entries[..self.length].iter().copied().flatten()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DependencyEdge {
    prerequisite: usize,
    dependent: usize,
    kind: DependencyKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorError {
    Full,
    DuplicateService {
        kind: ServiceKind,
    },
    ServiceNotFound {
        kind: ServiceKind,
    },
    IdentityMismatch {
        expected: ServiceIdentity,
        actual: ServiceIdentity,
    },
    UnexpectedOpcode {
        expected: HealthOpcode,
        actual: HealthOpcode,
    },
    InvalidTransition {
        phase: ServicePhase,
        operation: SupervisorOperation,
    },
    Sequence(SequenceError),
    DeadlineOverflow,
    BackoffNotElapsed {
        deadline_ns: u64,
        now_ns: u64,
    },
    ReplacementGenerationExhausted,
    ReplacementGenerationMismatch {
        expected: u32,
        actual: u32,
    },
    ReplacementPidMustChange,
    FaultClassReservedForSupervisor,
    MissedProbeCounterExhausted,
    HealthyStreakExhausted,
    RestartBudgetRearmCounterExhausted,
    RestartBudgetNotConsumed,
    RecoveryStabilityPending {
        observed: u32,
        required: u32,
    },
    DependencyRootStillActive {
        kind: ServiceKind,
    },
    Frame(HealthFrameError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceSnapshot {
    pub identity: ServiceIdentity,
    pub policy: ServicePolicy,
    pub phase: ServicePhase,
    pub restarts_used: u32,
    pub probe_deadline_ns: Option<u64>,
    pub backoff_deadline_ns: Option<u64>,
    pub last_fault: Option<FaultClass>,
    pub last_outbound_sequence: u64,
    pub last_inbound_sequence: u64,
    pub consecutive_missed_probes: u32,
    pub total_missed_probes: u64,
    pub healthy_streak: u32,
    pub restart_budget_rearms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ServiceRecord {
    identity: ServiceIdentity,
    policy: ServicePolicy,
    phase: ServicePhase,
    restarts_used: u32,
    probe_deadline_ns: Option<u64>,
    backoff_deadline_ns: Option<u64>,
    last_fault: Option<FaultClass>,
    sequences: BidirectionalSequenceTracker,
    consecutive_missed_probes: u32,
    total_missed_probes: u64,
    healthy_streak: u32,
    restart_budget_rearms: u32,
}

impl ServiceRecord {
    const fn snapshot(self) -> ServiceSnapshot {
        ServiceSnapshot {
            identity: self.identity,
            policy: self.policy,
            phase: self.phase,
            restarts_used: self.restarts_used,
            probe_deadline_ns: self.probe_deadline_ns,
            backoff_deadline_ns: self.backoff_deadline_ns,
            last_fault: self.last_fault,
            last_outbound_sequence: self.sequences.last_outbound(),
            last_inbound_sequence: self.sequences.last_inbound(),
            consecutive_missed_probes: self.consecutive_missed_probes,
            total_missed_probes: self.total_missed_probes,
            healthy_streak: self.healthy_streak,
            restart_budget_rearms: self.restart_budget_rearms,
        }
    }
}

/// One allocation-free service-catalog entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceDefinition {
    pub identity: ServiceIdentity,
    pub policy: ServicePolicy,
}

impl ServiceDefinition {
    pub const fn new(identity: ServiceIdentity, policy: ServicePolicy) -> Self {
        Self { identity, policy }
    }
}

/// One allocation-free dependency-catalog entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyDefinition {
    pub prerequisite: ServiceKind,
    pub dependent: ServiceKind,
    pub kind: DependencyKind,
}

impl DependencyDefinition {
    pub const fn new(
        prerequisite: ServiceKind,
        dependent: ServiceKind,
        kind: DependencyKind,
    ) -> Self {
        Self {
            prerequisite,
            dependent,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogError {
    Service {
        index: usize,
        source: SupervisorError,
    },
    Dependency {
        index: usize,
        source: DependencyError,
    },
}

/// Transactionally armed batch of one probe per registered service.
///
/// Entries retain catalog registration order. Either every eligible service
/// enters `ProbeArmed`, or the supervisor is left unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbeBatch<const CAPACITY: usize> {
    frames: [Option<HealthFrame>; CAPACITY],
    length: usize,
}

impl<const CAPACITY: usize> ProbeBatch<CAPACITY> {
    const fn new() -> Self {
        Self {
            frames: [None; CAPACITY],
            length: 0,
        }
    }

    fn push(&mut self, frame: HealthFrame) {
        debug_assert!(self.length < CAPACITY);
        self.frames[self.length] = Some(frame);
        self.length += 1;
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn get(&self, index: usize) -> Option<HealthFrame> {
        self.frames.get(index).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = HealthFrame> + '_ {
        self.frames[..self.length].iter().copied().flatten()
    }
}

pub struct ServiceSupervisor<
    const CAPACITY: usize = DEFAULT_SUPERVISOR_CAPACITY,
    const DEPENDENCY_CAPACITY: usize = DEFAULT_SUPERVISOR_CAPACITY,
> {
    records: [Option<ServiceRecord>; CAPACITY],
    length: usize,
    dependencies: [Option<DependencyEdge>; DEPENDENCY_CAPACITY],
    dependency_length: usize,
    active_dependency_roots: [bool; CAPACITY],
    dependency_impacts: [[DependencyImpact; CAPACITY]; CAPACITY],
}

impl<const CAPACITY: usize, const DEPENDENCY_CAPACITY: usize>
    ServiceSupervisor<CAPACITY, DEPENDENCY_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            records: array::from_fn(|_| None),
            length: 0,
            dependencies: array::from_fn(|_| None),
            dependency_length: 0,
            active_dependency_roots: [false; CAPACITY],
            dependency_impacts: [[DependencyImpact::Unaffected; CAPACITY]; CAPACITY],
        }
    }

    /// Builds a complete fixed-capacity supervisor from caller-owned catalog
    /// slices. No partially populated supervisor escapes when any service or
    /// dependency entry is invalid.
    pub fn from_catalog(
        services: &[ServiceDefinition],
        dependencies: &[DependencyDefinition],
    ) -> Result<Self, CatalogError> {
        Self::from_catalog_iter(services.iter().copied(), dependencies.iter().copied())
    }

    /// Builds one complete supervisor from allocation-free caller iterators.
    ///
    /// This is the manifest-facing equivalent of [`Self::from_catalog`]:
    /// neither an invalid service nor an invalid edge can expose a partially
    /// populated supervisor.
    pub fn from_catalog_iter(
        services: impl IntoIterator<Item = ServiceDefinition>,
        dependencies: impl IntoIterator<Item = DependencyDefinition>,
    ) -> Result<Self, CatalogError> {
        let mut supervisor = Self::new();
        for (index, service) in services.into_iter().enumerate() {
            supervisor
                .register(service.identity, service.policy)
                .map_err(|source| CatalogError::Service { index, source })?;
        }
        for (index, dependency) in dependencies.into_iter().enumerate() {
            supervisor
                .add_dependency(
                    dependency.prerequisite,
                    dependency.dependent,
                    dependency.kind,
                )
                .map_err(|source| CatalogError::Dependency { index, source })?;
        }
        Ok(supervisor)
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

    pub const fn dependency_count(&self) -> usize {
        self.dependency_length
    }

    pub const fn dependency_capacity(&self) -> usize {
        DEPENDENCY_CAPACITY
    }

    pub fn register(
        &mut self,
        identity: ServiceIdentity,
        policy: ServicePolicy,
    ) -> Result<(), SupervisorError> {
        if self.record(identity.kind).is_some() {
            return Err(SupervisorError::DuplicateService {
                kind: identity.kind,
            });
        }
        let Some(slot) = self.records.iter().position(Option::is_none) else {
            return Err(SupervisorError::Full);
        };
        self.records[slot] = Some(ServiceRecord {
            identity,
            policy,
            phase: ServicePhase::Registered,
            restarts_used: 0,
            probe_deadline_ns: None,
            backoff_deadline_ns: None,
            last_fault: None,
            sequences: BidirectionalSequenceTracker::new(),
            consecutive_missed_probes: 0,
            total_missed_probes: 0,
            healthy_streak: 0,
            restart_budget_rearms: 0,
        });
        self.length += 1;
        Ok(())
    }

    pub fn service(&self, kind: ServiceKind) -> Option<ServiceSnapshot> {
        self.record(kind).copied().map(ServiceRecord::snapshot)
    }

    /// Adds a directed prerequisite -> dependent edge.
    ///
    /// Topology is immutable while any dependency-root fault is active, so an
    /// in-flight impact cannot silently change underneath the runtime.
    pub fn add_dependency(
        &mut self,
        prerequisite: ServiceKind,
        dependent: ServiceKind,
        kind: DependencyKind,
    ) -> Result<(), DependencyError> {
        if prerequisite == dependent {
            return Err(DependencyError::SelfDependency {
                service: prerequisite,
            });
        }
        let prerequisite_index = self
            .record_index(prerequisite)
            .ok_or(DependencyError::ServiceNotFound { kind: prerequisite })?;
        let dependent_index = self
            .record_index(dependent)
            .ok_or(DependencyError::ServiceNotFound { kind: dependent })?;
        if self.dependencies.iter().flatten().any(|edge| {
            edge.prerequisite == prerequisite_index && edge.dependent == dependent_index
        }) {
            return Err(DependencyError::Duplicate {
                prerequisite,
                dependent,
            });
        }
        if self.active_dependency_roots.iter().any(|active| *active) {
            return Err(DependencyError::TopologyActive);
        }
        if self.path_exists(dependent_index, prerequisite_index) {
            return Err(DependencyError::Cycle {
                prerequisite,
                dependent,
            });
        }
        let Some(slot) = self.dependencies.iter().position(Option::is_none) else {
            return Err(DependencyError::Full);
        };
        self.dependencies[slot] = Some(DependencyEdge {
            prerequisite: prerequisite_index,
            dependent: dependent_index,
            kind,
        });
        self.dependency_length += 1;
        Ok(())
    }

    pub fn dependency_kind(
        &self,
        prerequisite: ServiceKind,
        dependent: ServiceKind,
    ) -> Option<DependencyKind> {
        let prerequisite = self.record_index(prerequisite)?;
        let dependent = self.record_index(dependent)?;
        self.dependencies
            .iter()
            .flatten()
            .find(|edge| edge.prerequisite == prerequisite && edge.dependent == dependent)
            .map(|edge| edge.kind)
    }

    pub fn dependency_impact(
        &self,
        service: ServiceKind,
    ) -> Result<DependencyImpact, DependencyError> {
        let index = self
            .record_index(service)
            .ok_or(DependencyError::ServiceNotFound { kind: service })?;
        Ok(self.effective_impact(index))
    }

    /// Propagates one already-classified root fault through the dependency DAG.
    ///
    /// Repeating this call for the same active root is idempotent and returns an
    /// empty batch. Dependency propagation never changes restart counters,
    /// health sequences, or service phases.
    pub fn dependency_fault(
        &mut self,
        root: ServiceIdentity,
    ) -> Result<DependencyTransitions<CAPACITY>, DependencyError> {
        let root_index = self.checked_dependency_index(root)?;
        let phase = self.records[root_index]
            .expect("checked dependency root is registered")
            .phase;
        if matches!(
            phase,
            ServicePhase::Registered | ServicePhase::ProbeArmed | ServicePhase::Healthy
        ) {
            return Err(DependencyError::RootStillAvailable {
                kind: root.kind,
                phase,
            });
        }
        if self.active_dependency_roots[root_index] {
            return Ok(DependencyTransitions::new());
        }

        let order = self.topological_order();
        let previous = self.effective_impacts();
        self.dependency_impacts[root_index] = self.root_dependency_impacts(root_index, &order);
        self.active_dependency_roots[root_index] = true;
        let current = self.effective_impacts();
        Ok(self.dependency_transitions(previous, current, &order))
    }

    /// Clears one root fault after that service has completed a healthy probe.
    /// Remaining roots continue to contribute their impacts. Effective changes
    /// are returned in prerequisite-first topological order.
    pub fn dependency_recovered(
        &mut self,
        root: ServiceIdentity,
    ) -> Result<DependencyTransitions<CAPACITY>, DependencyError> {
        let root_index = self.checked_dependency_index(root)?;
        let phase = self.records[root_index]
            .expect("checked dependency root is registered")
            .phase;
        if phase != ServicePhase::Healthy {
            return Err(DependencyError::RootNotHealthy {
                kind: root.kind,
                phase,
            });
        }
        if !self.active_dependency_roots[root_index] {
            return Ok(DependencyTransitions::new());
        }

        let order = self.topological_order();
        let previous = self.effective_impacts();
        self.dependency_impacts[root_index] = [DependencyImpact::Unaffected; CAPACITY];
        self.active_dependency_roots[root_index] = false;
        let current = self.effective_impacts();
        Ok(self.dependency_transitions(previous, current, &order))
    }

    pub fn arm_probe(
        &mut self,
        identity: ServiceIdentity,
        now_ns: u64,
    ) -> Result<HealthFrame, SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if !matches!(
            record.phase,
            ServicePhase::Registered | ServicePhase::Healthy
        ) {
            return Err(invalid_transition(record, SupervisorOperation::ArmProbe));
        }
        let deadline_ns = now_ns
            .checked_add(record.policy.health_timeout_ns)
            .ok_or(SupervisorError::DeadlineOverflow)?;
        let sequence = record
            .sequences
            .next_outbound()
            .map_err(SupervisorError::Sequence)?;
        let frame = HealthFrame::probe(sequence, identity, record.policy.health_timeout_ns)
            .map_err(SupervisorError::Frame)?;
        record.phase = ServicePhase::ProbeArmed;
        record.probe_deadline_ns = Some(deadline_ns);
        Ok(frame)
    }

    /// Arms one probe for every registered service in catalog order.
    ///
    /// Phase, deadline, and outbound-sequence preconditions are validated for
    /// the complete batch before any record is changed.
    pub fn arm_probe_batch(
        &mut self,
        now_ns: u64,
    ) -> Result<ProbeBatch<CAPACITY>, SupervisorError> {
        let mut identities = [None; CAPACITY];
        let mut length = 0;
        for record in self.records.iter().flatten() {
            if !matches!(
                record.phase,
                ServicePhase::Registered | ServicePhase::Healthy
            ) {
                return Err(invalid_transition(record, SupervisorOperation::ArmProbe));
            }
            now_ns
                .checked_add(record.policy.health_timeout_ns)
                .ok_or(SupervisorError::DeadlineOverflow)?;
            record
                .sequences
                .last_outbound
                .checked_add(1)
                .ok_or(SupervisorError::Sequence(SequenceError::Exhausted))?;
            identities[length] = Some(record.identity);
            length += 1;
        }

        let mut batch = ProbeBatch::new();
        for identity in identities[..length].iter().copied().flatten() {
            batch.push(self.arm_probe(identity, now_ns)?);
        }
        Ok(batch)
    }

    pub fn record_healthy(&mut self, frame: HealthFrame) -> Result<(), SupervisorError> {
        require_opcode(frame, HealthOpcode::Healthy)?;
        let record = self.checked_record_mut(frame.identity)?;
        if record.phase != ServicePhase::ProbeArmed {
            return Err(invalid_transition(
                record,
                SupervisorOperation::RecordHealthy,
            ));
        }
        let healthy_streak = record
            .healthy_streak
            .checked_add(1)
            .ok_or(SupervisorError::HealthyStreakExhausted)?;
        record
            .sequences
            .accept_inbound(frame.sequence)
            .map_err(SupervisorError::Sequence)?;
        record.phase = ServicePhase::Healthy;
        record.probe_deadline_ns = None;
        record.consecutive_missed_probes = 0;
        record.healthy_streak = healthy_streak;
        Ok(())
    }

    pub fn record_fault(
        &mut self,
        frame: HealthFrame,
    ) -> Result<FaultDisposition, SupervisorError> {
        require_opcode(frame, HealthOpcode::Fault)?;
        let record = self.checked_record_mut(frame.identity)?;
        if !matches!(
            record.phase,
            ServicePhase::Registered | ServicePhase::ProbeArmed | ServicePhase::Healthy
        ) {
            return Err(invalid_transition(record, SupervisorOperation::RecordFault));
        }
        record
            .sequences
            .accept_inbound(frame.sequence)
            .map_err(SupervisorError::Sequence)?;
        Ok(apply_fault(
            record,
            frame
                .fault_class
                .expect("validated Fault frame has a fault class"),
        ))
    }

    pub fn classify_fault(
        &mut self,
        identity: ServiceIdentity,
        class: FaultClass,
    ) -> Result<FaultDisposition, SupervisorError> {
        if class == FaultClass::RestartBudgetExhausted {
            return Err(SupervisorError::FaultClassReservedForSupervisor);
        }
        let record = self.checked_record_mut(identity)?;
        if !matches!(
            record.phase,
            ServicePhase::Registered | ServicePhase::ProbeArmed | ServicePhase::Healthy
        ) {
            return Err(invalid_transition(record, SupervisorOperation::RecordFault));
        }
        Ok(apply_fault(record, class))
    }

    pub fn check_health_timeout(
        &mut self,
        identity: ServiceIdentity,
        now_ns: u64,
    ) -> Result<Option<FaultDisposition>, SupervisorError> {
        match self.check_health_deadline(identity, now_ns)? {
            HealthDeadlineOutcome::Pending { .. } | HealthDeadlineOutcome::ToleratedMiss { .. } => {
                Ok(None)
            }
            HealthDeadlineOutcome::Fault(disposition) => Ok(Some(disposition)),
        }
    }

    pub fn check_health_deadline(
        &mut self,
        identity: ServiceIdentity,
        now_ns: u64,
    ) -> Result<HealthDeadlineOutcome, SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if record.phase != ServicePhase::ProbeArmed {
            return Err(invalid_transition(record, SupervisorOperation::RecordFault));
        }
        let deadline_ns = record
            .probe_deadline_ns
            .expect("ProbeArmed record has a deadline");
        if now_ns < deadline_ns {
            return Ok(HealthDeadlineOutcome::Pending { deadline_ns });
        }
        let consecutive = record
            .consecutive_missed_probes
            .checked_add(1)
            .ok_or(SupervisorError::MissedProbeCounterExhausted)?;
        let total = record
            .total_missed_probes
            .checked_add(1)
            .ok_or(SupervisorError::MissedProbeCounterExhausted)?;
        record.consecutive_missed_probes = consecutive;
        record.total_missed_probes = total;
        if consecutive <= record.policy.missed_probe_tolerance {
            record.phase = ServicePhase::Healthy;
            record.probe_deadline_ns = None;
            record.healthy_streak = 0;
            return Ok(HealthDeadlineOutcome::ToleratedMiss {
                consecutive,
                tolerance: record.policy.missed_probe_tolerance,
            });
        }
        Ok(HealthDeadlineOutcome::Fault(apply_fault(
            record,
            FaultClass::HealthTimeout,
        )))
    }

    pub fn begin_backoff(
        &mut self,
        identity: ServiceIdentity,
        now_ns: u64,
    ) -> Result<u64, SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if record.phase != ServicePhase::Faulted {
            return Err(invalid_transition(
                record,
                SupervisorOperation::BeginBackoff,
            ));
        }
        let deadline_ns = now_ns
            .checked_add(record.policy.restart_backoff_ns)
            .ok_or(SupervisorError::DeadlineOverflow)?;
        record.restarts_used += 1;
        record.phase = ServicePhase::Backoff;
        record.probe_deadline_ns = None;
        record.backoff_deadline_ns = Some(deadline_ns);
        Ok(deadline_ns)
    }

    pub fn begin_replacement(
        &mut self,
        identity: ServiceIdentity,
        now_ns: u64,
    ) -> Result<(), SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if record.phase != ServicePhase::Backoff {
            return Err(invalid_transition(
                record,
                SupervisorOperation::BeginReplacement,
            ));
        }
        let deadline_ns = record
            .backoff_deadline_ns
            .expect("Backoff record has a deadline");
        if now_ns < deadline_ns {
            return Err(SupervisorError::BackoffNotElapsed {
                deadline_ns,
                now_ns,
            });
        }
        record.phase = ServicePhase::AwaitingReplacement;
        Ok(())
    }

    pub fn install_replacement(
        &mut self,
        previous: ServiceIdentity,
        replacement_generation: u32,
        replacement_pid: u64,
    ) -> Result<ServiceIdentity, SupervisorError> {
        let record = self.checked_record_mut(previous)?;
        if record.phase != ServicePhase::AwaitingReplacement {
            return Err(invalid_transition(
                record,
                SupervisorOperation::InstallReplacement,
            ));
        }
        let expected = previous
            .generation
            .checked_add(1)
            .ok_or(SupervisorError::ReplacementGenerationExhausted)?;
        if replacement_generation != expected {
            return Err(SupervisorError::ReplacementGenerationMismatch {
                expected,
                actual: replacement_generation,
            });
        }
        if replacement_pid == previous.pid {
            return Err(SupervisorError::ReplacementPidMustChange);
        }
        let replacement =
            ServiceIdentity::new(previous.kind, replacement_generation, replacement_pid)
                .map_err(|error| SupervisorError::Frame(HealthFrameError::Identity(error)))?;
        record.identity = replacement;
        record.phase = ServicePhase::Registered;
        record.probe_deadline_ns = None;
        record.backoff_deadline_ns = None;
        record.sequences = BidirectionalSequenceTracker::new();
        record.consecutive_missed_probes = 0;
        record.healthy_streak = 0;
        Ok(replacement)
    }

    /// Opens a new restart-budget window after a replacement has demonstrated
    /// a minimum stable healthy streak and its dependency root has recovered.
    ///
    /// The manifest budget remains the maximum for one recovery window. This
    /// explicit rearm is deliberately separate from `record_healthy`: callers
    /// must first clear dependency impact, and a single opportunistic response
    /// cannot make a flapping generation restartable forever.
    pub fn rearm_restart_budget(
        &mut self,
        identity: ServiceIdentity,
    ) -> Result<(), SupervisorError> {
        let index = self
            .record_index(identity.kind)
            .ok_or(SupervisorError::ServiceNotFound {
                kind: identity.kind,
            })?;
        let expected = self.records[index]
            .expect("record index points to a registered service")
            .identity;
        if expected != identity {
            return Err(SupervisorError::IdentityMismatch {
                expected,
                actual: identity,
            });
        }
        if self.active_dependency_roots[index] {
            return Err(SupervisorError::DependencyRootStillActive {
                kind: identity.kind,
            });
        }
        let record = self.records[index]
            .as_mut()
            .expect("record index points to a registered service");
        if record.phase != ServicePhase::Healthy
            || record.probe_deadline_ns.is_some()
            || record.backoff_deadline_ns.is_some()
        {
            return Err(invalid_transition(
                record,
                SupervisorOperation::RearmRestartBudget,
            ));
        }
        if record.restarts_used == 0 {
            return Err(SupervisorError::RestartBudgetNotConsumed);
        }
        if record.healthy_streak < RECOVERY_STABILITY_HEALTHY_ROUNDS {
            return Err(SupervisorError::RecoveryStabilityPending {
                observed: record.healthy_streak,
                required: RECOVERY_STABILITY_HEALTHY_ROUNDS,
            });
        }
        let restart_budget_rearms = record
            .restart_budget_rearms
            .checked_add(1)
            .ok_or(SupervisorError::RestartBudgetRearmCounterExhausted)?;
        record.restarts_used = 0;
        record.restart_budget_rearms = restart_budget_rearms;
        Ok(())
    }

    pub fn replacement_failed(
        &mut self,
        identity: ServiceIdentity,
    ) -> Result<FaultDisposition, SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if record.phase != ServicePhase::AwaitingReplacement {
            return Err(invalid_transition(record, SupervisorOperation::RecordFault));
        }
        Ok(apply_fault(record, FaultClass::ReplacementFailed))
    }

    pub fn quarantine(
        &mut self,
        identity: ServiceIdentity,
    ) -> Result<HealthFrame, SupervisorError> {
        let record = self.checked_record_mut(identity)?;
        if record.phase != ServicePhase::QuarantinePending {
            return Err(invalid_transition(record, SupervisorOperation::Quarantine));
        }
        let attempt = record
            .policy
            .restart_budget
            .checked_add(1)
            .expect("policy rejects an unrepresentable over-budget attempt");
        let sequence = record
            .sequences
            .next_outbound()
            .map_err(SupervisorError::Sequence)?;
        let frame =
            HealthFrame::quarantine(sequence, identity, attempt, record.policy.restart_budget)
                .map_err(SupervisorError::Frame)?;
        record.phase = ServicePhase::Quarantined;
        Ok(frame)
    }

    pub fn acknowledge_degraded(&mut self, frame: HealthFrame) -> Result<(), SupervisorError> {
        require_opcode(frame, HealthOpcode::DegradedAck)?;
        let record = self.checked_record_mut(frame.identity)?;
        if record.phase != ServicePhase::Quarantined {
            return Err(invalid_transition(record, SupervisorOperation::DegradedAck));
        }
        record
            .sequences
            .accept_inbound(frame.sequence)
            .map_err(SupervisorError::Sequence)?;
        record.phase = ServicePhase::Degraded;
        Ok(())
    }

    fn record(&self, kind: ServiceKind) -> Option<&ServiceRecord> {
        self.records
            .iter()
            .flatten()
            .find(|record| record.identity.kind == kind)
    }

    fn record_index(&self, kind: ServiceKind) -> Option<usize> {
        self.records.iter().position(|record| {
            record
                .as_ref()
                .is_some_and(|record| record.identity.kind == kind)
        })
    }

    fn checked_dependency_index(&self, actual: ServiceIdentity) -> Result<usize, DependencyError> {
        let Some(index) = self.record_index(actual.kind) else {
            return Err(DependencyError::ServiceNotFound { kind: actual.kind });
        };
        let expected = self.records[index]
            .expect("record index points to a registered service")
            .identity;
        if expected != actual {
            return Err(DependencyError::IdentityMismatch { expected, actual });
        }
        Ok(index)
    }

    fn path_exists(&self, start: usize, target: usize) -> bool {
        let mut visited = [false; CAPACITY];
        let mut stack = [0_usize; CAPACITY];
        let mut stack_length = 1;
        stack[0] = start;
        visited[start] = true;

        while stack_length != 0 {
            stack_length -= 1;
            let current = stack[stack_length];
            if current == target {
                return true;
            }
            for edge in self
                .dependencies
                .iter()
                .flatten()
                .filter(|edge| edge.prerequisite == current)
            {
                if !visited[edge.dependent] {
                    visited[edge.dependent] = true;
                    stack[stack_length] = edge.dependent;
                    stack_length += 1;
                }
            }
        }
        false
    }

    fn topological_order(&self) -> [Option<usize>; CAPACITY] {
        let mut order = [None; CAPACITY];
        let mut indegree = [0_usize; CAPACITY];
        let mut selected = [false; CAPACITY];
        for edge in self.dependencies.iter().flatten() {
            indegree[edge.dependent] += 1;
        }

        for output in order.iter_mut().take(self.length) {
            let next = self
                .records
                .iter()
                .enumerate()
                .filter(|(index, record)| {
                    record.is_some() && !selected[*index] && indegree[*index] == 0
                })
                .min_by_key(|(_, record)| {
                    record
                        .as_ref()
                        .expect("topological candidate is registered")
                        .identity
                        .kind
                        .raw()
                })
                .map(|(index, _)| index)
                .expect("dependency graph is acyclic");
            *output = Some(next);
            selected[next] = true;
            for edge in self
                .dependencies
                .iter()
                .flatten()
                .filter(|edge| edge.prerequisite == next)
            {
                indegree[edge.dependent] -= 1;
            }
        }
        order
    }

    fn root_dependency_impacts(
        &self,
        root: usize,
        order: &[Option<usize>; CAPACITY],
    ) -> [DependencyImpact; CAPACITY] {
        let mut impacts = [DependencyImpact::Unaffected; CAPACITY];
        let mut unavailable = [false; CAPACITY];
        impacts[root] = DependencyImpact::HardBlocked;
        unavailable[root] = true;

        for prerequisite in order.iter().copied().flatten() {
            if !unavailable[prerequisite] {
                continue;
            }
            for edge in self
                .dependencies
                .iter()
                .flatten()
                .filter(|edge| edge.prerequisite == prerequisite)
            {
                let impact = match edge.kind {
                    DependencyKind::Hard => DependencyImpact::HardBlocked,
                    DependencyKind::Soft => DependencyImpact::SoftDegraded,
                };
                impacts[edge.dependent] = impacts[edge.dependent].strongest(impact);
                if edge.kind == DependencyKind::Hard {
                    unavailable[edge.dependent] = true;
                }
            }
        }
        impacts
    }

    fn effective_impact(&self, service: usize) -> DependencyImpact {
        self.dependency_impacts
            .iter()
            .enumerate()
            .filter(|(root, _)| self.active_dependency_roots[*root])
            .fold(DependencyImpact::Unaffected, |impact, (_, root)| {
                impact.strongest(root[service])
            })
    }

    fn effective_impacts(&self) -> [DependencyImpact; CAPACITY] {
        array::from_fn(|service| self.effective_impact(service))
    }

    fn dependency_transitions(
        &self,
        previous: [DependencyImpact; CAPACITY],
        current: [DependencyImpact; CAPACITY],
        order: &[Option<usize>; CAPACITY],
    ) -> DependencyTransitions<CAPACITY> {
        let mut transitions = DependencyTransitions::new();
        for service in order.iter().copied().flatten() {
            if previous[service] == current[service] {
                continue;
            }
            transitions.push(DependencyTransition {
                service: self.records[service]
                    .expect("topological service is registered")
                    .identity,
                previous: previous[service],
                current: current[service],
            });
        }
        transitions
    }

    fn checked_record_mut(
        &mut self,
        actual: ServiceIdentity,
    ) -> Result<&mut ServiceRecord, SupervisorError> {
        let Some(record) = self
            .records
            .iter_mut()
            .flatten()
            .find(|record| record.identity.kind == actual.kind)
        else {
            return Err(SupervisorError::ServiceNotFound { kind: actual.kind });
        };
        if record.identity != actual {
            return Err(SupervisorError::IdentityMismatch {
                expected: record.identity,
                actual,
            });
        }
        Ok(record)
    }
}

impl<const CAPACITY: usize, const DEPENDENCY_CAPACITY: usize> Default
    for ServiceSupervisor<CAPACITY, DEPENDENCY_CAPACITY>
{
    fn default() -> Self {
        Self::new()
    }
}

fn require_opcode(frame: HealthFrame, expected: HealthOpcode) -> Result<(), SupervisorError> {
    if frame.opcode == expected {
        Ok(())
    } else {
        Err(SupervisorError::UnexpectedOpcode {
            expected,
            actual: frame.opcode,
        })
    }
}

fn invalid_transition(record: &ServiceRecord, operation: SupervisorOperation) -> SupervisorError {
    SupervisorError::InvalidTransition {
        phase: record.phase,
        operation,
    }
}

fn apply_fault(record: &mut ServiceRecord, class: FaultClass) -> FaultDisposition {
    record.last_fault = Some(class);
    record.probe_deadline_ns = None;
    record.healthy_streak = 0;
    let attempt = record.restarts_used + 1;
    if record.restarts_used < record.policy.restart_budget {
        record.phase = ServicePhase::Faulted;
        FaultDisposition::RestartAllowed {
            attempt,
            budget: record.policy.restart_budget,
            class,
        }
    } else {
        record.phase = ServicePhase::QuarantinePending;
        FaultDisposition::QuarantineRequired {
            attempt,
            budget: record.policy.restart_budget,
            class,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(kind: ServiceKind, generation: u32, pid: u64) -> ServiceIdentity {
        ServiceIdentity::new(kind, generation, pid).unwrap()
    }

    fn policy(budget: u32) -> ServicePolicy {
        ServicePolicy::new(budget, 30, 50).unwrap()
    }

    fn mutate(frame: HealthFrame, offset: usize, value: u8) -> [u8; HEALTH_FRAME_SIZE] {
        let mut bytes = frame.encode();
        bytes[offset] = value;
        bytes
    }

    #[test]
    fn every_opcode_round_trips_in_exact_little_endian_frames() {
        let service = identity(ServiceKind::InputServer, 0x1122_3344, 0x0102_0304_0506_0708);
        let frames = [
            HealthFrame::probe(0x1122_3344_5566_7788, service, 50).unwrap(),
            HealthFrame::healthy(2, service).unwrap(),
            HealthFrame::fault(3, service, FaultClass::ProcessExit).unwrap(),
            HealthFrame::quarantine(4, service, 2, 1).unwrap(),
            HealthFrame::degraded_ack(5, service).unwrap(),
        ];
        for frame in frames {
            let bytes = frame.encode();
            assert_eq!(bytes.len(), HEALTH_FRAME_SIZE);
            assert_eq!(&bytes[MAGIC_RANGE], b"BSH1");
            assert_eq!(HealthFrame::decode(&bytes), Ok(frame));
        }
        let bytes = frames[0].encode();
        assert_eq!(
            &bytes[SEQUENCE_RANGE],
            &[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
        );
        assert_eq!(&bytes[GENERATION_RANGE], &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(&bytes[PID_RANGE], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert!(
            bytes[HEADER_RESERVED_RANGE]
                .iter()
                .chain(bytes[SEMANTIC_RESERVED_RANGE].iter())
                .chain(bytes[PADDING_RANGE].iter())
                .all(|byte| *byte == 0)
        );
    }

    #[test]
    fn service_identity_and_common_frame_fields_reject_zeroes() {
        assert_eq!(
            ServiceIdentity::new(ServiceKind::ServiceManager, 0, 1),
            Err(ServiceIdentityError::GenerationMustBeNonZero)
        );
        assert_eq!(
            ServiceIdentity::new(ServiceKind::SurfaceServer, 1, 0),
            Err(ServiceIdentityError::PidMustBeNonZero)
        );
        let service = identity(ServiceKind::InputServer, 1, 9);
        assert_eq!(
            HealthFrame::healthy(0, service),
            Err(HealthFrameError::SequenceMustBeNonZero)
        );

        let base = HealthFrame::healthy(1, service).unwrap();
        let mut zero_generation = base.encode();
        zero_generation[GENERATION_RANGE].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&zero_generation),
            Err(HealthDecodeError::InvalidFrame(HealthFrameError::Identity(
                ServiceIdentityError::GenerationMustBeNonZero
            )))
        );
        let mut zero_pid = base.encode();
        zero_pid[PID_RANGE].copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&zero_pid),
            Err(HealthDecodeError::InvalidFrame(HealthFrameError::Identity(
                ServiceIdentityError::PidMustBeNonZero
            )))
        );
        let mut zero_sequence = base.encode();
        zero_sequence[SEQUENCE_RANGE].copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&zero_sequence),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::SequenceMustBeNonZero
            ))
        );
    }

    #[test]
    fn decoder_rejects_every_unknown_header_discriminant() {
        let service = identity(ServiceKind::SurfaceServer, 1, 7);
        let base = HealthFrame::healthy(1, service).unwrap();
        assert_eq!(
            HealthFrame::decode(&mutate(base, 0, b'X')),
            Err(HealthDecodeError::BadMagic)
        );
        assert_eq!(
            HealthFrame::decode(&mutate(base, VERSION_OFFSET, 2)),
            Err(HealthDecodeError::UnsupportedVersion(2))
        );
        assert_eq!(
            HealthFrame::decode(&mutate(base, OPCODE_OFFSET, 0xff)),
            Err(HealthDecodeError::UnknownOpcode(0xff))
        );
        assert_eq!(
            HealthFrame::decode(&mutate(base, FLAGS_OFFSET, 0x80)),
            Err(HealthDecodeError::UnknownFlags(0x80))
        );
        assert_eq!(
            HealthFrame::decode(&mutate(base, SERVICE_KIND_OFFSET, 6)),
            Err(HealthDecodeError::UnknownServiceKind(6))
        );
        assert_eq!(
            HealthFrame::decode(&mutate(base, FAULT_CLASS_OFFSET, 0xff)),
            Err(HealthDecodeError::UnknownFaultClass(0xff))
        );
    }

    #[test]
    fn product_service_kinds_round_trip_without_weakening_unknown_kind_rejection() {
        let storage = identity(ServiceKind::StorageServer, 3, 0x0000_0003_0000_000a);
        let app = identity(ServiceKind::App, 4, 0x0000_0004_0000_0009);
        let healthy = HealthFrame::healthy(1, storage).unwrap();
        let app_healthy = HealthFrame::healthy(2, app).unwrap();
        assert_eq!(ServiceKind::from_raw(4), Some(ServiceKind::StorageServer));
        assert_eq!(ServiceKind::from_raw(5), Some(ServiceKind::App));
        assert_eq!(HealthFrame::decode(&healthy.encode()), Ok(healthy));
        assert_eq!(HealthFrame::decode(&app_healthy.encode()), Ok(app_healthy));
        assert_eq!(ServiceKind::from_raw(6), None);
    }

    #[test]
    fn decoder_rejects_every_reserved_and_padding_byte() {
        let base = HealthFrame::healthy(1, identity(ServiceKind::ServiceManager, 1, 2)).unwrap();
        for offset in HEADER_RESERVED_RANGE.chain(SEMANTIC_RESERVED_RANGE) {
            assert_eq!(
                HealthFrame::decode(&mutate(base, offset, 1)),
                Err(HealthDecodeError::ReservedNonZero {
                    offset: offset as u8
                })
            );
        }
        for offset in PADDING_RANGE {
            assert_eq!(
                HealthFrame::decode(&mutate(base, offset, 1)),
                Err(HealthDecodeError::PaddingNonZero {
                    offset: offset as u8
                })
            );
        }
    }

    #[test]
    fn opcode_flags_are_strict_and_opcode_specific() {
        let service = identity(ServiceKind::InputServer, 1, 2);
        let probe = HealthFrame::probe(1, service, 50).unwrap();
        let healthy = HealthFrame::healthy(2, service).unwrap();
        assert!(probe.flags().ack_required());
        assert!(!healthy.flags().ack_required());
        assert_eq!(
            HealthFrame::decode(&mutate(probe, FLAGS_OFFSET, 0)),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::FlagsMismatch {
                    expected: 1,
                    actual: 0
                }
            ))
        );
        assert_eq!(
            HealthFrame::decode(&mutate(healthy, FLAGS_OFFSET, 1)),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::FlagsMismatch {
                    expected: 0,
                    actual: 1
                }
            ))
        );
    }

    #[test]
    fn fault_and_quarantine_semantics_are_not_interchangeable() {
        let service = identity(ServiceKind::InputServer, 1, 2);
        assert_eq!(
            HealthFrame::fault(1, service, FaultClass::RestartBudgetExhausted),
            Err(HealthFrameError::RestartBudgetExhaustedOnlyForQuarantine)
        );
        assert_eq!(
            HealthFrame::quarantine(1, service, 1, 1),
            Err(HealthFrameError::RestartAttemptMustExceedBudgetByOne)
        );
        assert_eq!(
            HealthFrame::quarantine(1, service, 1, 0),
            Err(HealthFrameError::RestartBudgetMustBeNonZero)
        );

        let fault = HealthFrame::fault(1, service, FaultClass::ProtocolViolation).unwrap();
        assert_eq!(
            HealthFrame::decode(&mutate(fault, FAULT_CLASS_OFFSET, 0)),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::FaultRequired
            ))
        );
        let quarantine = HealthFrame::quarantine(2, service, 2, 1).unwrap();
        assert_eq!(
            HealthFrame::decode(&mutate(
                quarantine,
                FAULT_CLASS_OFFSET,
                FaultClass::ProcessExit.raw()
            )),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::QuarantineFaultRequired
            ))
        );
    }

    #[test]
    fn interval_attempt_and_budget_fields_are_opcode_scoped() {
        let service = identity(ServiceKind::SurfaceServer, 3, 8);
        assert_eq!(
            HealthFrame::probe(1, service, 0),
            Err(HealthFrameError::IntervalMustBeNonZero)
        );
        let healthy = HealthFrame::healthy(2, service).unwrap();
        assert_eq!(
            HealthFrame::decode(&mutate(
                healthy,
                FAULT_CLASS_OFFSET,
                FaultClass::ProcessExit.raw()
            )),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::FaultForbidden
            ))
        );
        let mut zero_probe_interval = HealthFrame::probe(1, service, 50).unwrap().encode();
        zero_probe_interval[INTERVAL_NS_RANGE].copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&zero_probe_interval),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::IntervalMustBeNonZero
            ))
        );
        let mut interval = healthy.encode();
        interval[INTERVAL_NS_RANGE].copy_from_slice(&1_u64.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&interval),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::IntervalMustBeZero
            ))
        );
        let mut attempt = healthy.encode();
        attempt[RESTART_ATTEMPT_RANGE].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&attempt),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::RestartAttemptMustBeZero
            ))
        );
        let mut budget = healthy.encode();
        budget[RESTART_BUDGET_RANGE].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&budget),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::RestartBudgetMustBeZero
            ))
        );
        let mut bad_quarantine_attempt =
            HealthFrame::quarantine(3, service, 2, 1).unwrap().encode();
        bad_quarantine_attempt[RESTART_ATTEMPT_RANGE].copy_from_slice(&3_u32.to_le_bytes());
        assert_eq!(
            HealthFrame::decode(&bad_quarantine_attempt),
            Err(HealthDecodeError::InvalidFrame(
                HealthFrameError::RestartAttemptMustExceedBudgetByOne
            ))
        );
    }

    #[test]
    fn all_fault_classes_round_trip_when_the_opcode_allows_them() {
        let service = identity(ServiceKind::ServiceManager, 4, 9);
        for (sequence, class) in [
            FaultClass::ProcessExit,
            FaultClass::HealthTimeout,
            FaultClass::ProtocolViolation,
            FaultClass::ReplacementFailed,
        ]
        .into_iter()
        .enumerate()
        {
            let frame = HealthFrame::fault(sequence as u64 + 1, service, class).unwrap();
            assert_eq!(HealthFrame::decode(&frame.encode()), Ok(frame));
            assert_eq!(frame.fault_class(), Some(class));
        }
        let exhausted = HealthFrame::quarantine(5, service, 3, 2).unwrap();
        assert_eq!(
            exhausted.fault_class(),
            Some(FaultClass::RestartBudgetExhausted)
        );
    }

    #[test]
    fn bidirectional_sequences_are_independent_monotonic_and_fail_closed() {
        let mut tracker = BidirectionalSequenceTracker::new();
        assert_eq!(tracker.next_outbound(), Ok(1));
        assert_eq!(tracker.next_outbound(), Ok(2));
        assert_eq!(tracker.accept_inbound(7), Ok(()));
        assert_eq!(
            tracker.accept_inbound(7),
            Err(SequenceError::NotIncreasing {
                last: 7,
                received: 7
            })
        );
        assert_eq!(
            tracker.accept_inbound(6),
            Err(SequenceError::NotIncreasing {
                last: 7,
                received: 6
            })
        );
        assert_eq!(tracker.accept_inbound(0), Err(SequenceError::Zero));
        assert_eq!(tracker.last_outbound(), 2);
        assert_eq!(tracker.last_inbound(), 7);

        tracker.last_outbound = u64::MAX;
        assert_eq!(tracker.next_outbound(), Err(SequenceError::Exhausted));
        assert_eq!(tracker.last_outbound(), u64::MAX);
    }

    #[test]
    fn service_policy_rejects_unbounded_or_zero_timing_values() {
        assert_eq!(
            ServicePolicy::new(0, 30, 50),
            Err(ServicePolicyError::RestartBudgetMustBeNonZero)
        );
        assert_eq!(
            ServicePolicy::new(u32::MAX, 30, 50),
            Err(ServicePolicyError::RestartBudgetTooLarge)
        );
        assert_eq!(
            ServicePolicy::new(1, 0, 50),
            Err(ServicePolicyError::RestartBackoffMustBeNonZero)
        );
        assert_eq!(
            ServicePolicy::new(1, 30, 0),
            Err(ServicePolicyError::HealthTimeoutMustBeNonZero)
        );
        let default = ServicePolicy::new(1, 30, 50).unwrap();
        assert_eq!(default.missed_probe_tolerance(), 0);
        assert_eq!(
            default.with_missed_probe_tolerance(MAX_MISSED_PROBES_TOLERATED + 1),
            Err(ServicePolicyError::MissedProbeToleranceTooLarge)
        );
        assert_eq!(
            default
                .with_missed_probe_tolerance(MAX_MISSED_PROBES_TOLERATED)
                .unwrap()
                .missed_probe_tolerance(),
            MAX_MISSED_PROBES_TOLERATED
        );
    }

    #[test]
    fn supervisor_registers_each_kind_once_and_is_fixed_capacity() {
        let manager = identity(ServiceKind::ServiceManager, 1, 10);
        let surface = identity(ServiceKind::SurfaceServer, 1, 11);
        let input = identity(ServiceKind::InputServer, 1, 12);
        let mut supervisor = ServiceSupervisor::<2>::new();
        assert_eq!(supervisor.register(manager, policy(1)), Ok(()));
        assert_eq!(
            supervisor.register(identity(ServiceKind::ServiceManager, 2, 20), policy(1)),
            Err(SupervisorError::DuplicateService {
                kind: ServiceKind::ServiceManager
            })
        );
        assert_eq!(supervisor.register(surface, policy(1)), Ok(()));
        assert_eq!(
            supervisor.register(input, policy(1)),
            Err(SupervisorError::Full)
        );
        assert_eq!(supervisor.len(), 2);
        assert_eq!(supervisor.available(), 0);

        let mut zero = ServiceSupervisor::<0>::new();
        assert_eq!(
            zero.register(manager, policy(1)),
            Err(SupervisorError::Full)
        );
        assert!(zero.is_empty());
    }

    #[test]
    fn five_service_catalog_is_transactional_and_preserves_declared_topology() {
        let services = [
            ServiceDefinition::new(identity(ServiceKind::ServiceManager, 1, 20), policy(1)),
            ServiceDefinition::new(identity(ServiceKind::SurfaceServer, 1, 21), policy(1)),
            ServiceDefinition::new(identity(ServiceKind::InputServer, 1, 22), policy(1)),
            ServiceDefinition::new(identity(ServiceKind::StorageServer, 1, 23), policy(1)),
            ServiceDefinition::new(identity(ServiceKind::App, 1, 24), policy(1)),
        ];
        let dependencies = [
            DependencyDefinition::new(
                ServiceKind::ServiceManager,
                ServiceKind::SurfaceServer,
                DependencyKind::Hard,
            ),
            DependencyDefinition::new(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            ),
            DependencyDefinition::new(
                ServiceKind::InputServer,
                ServiceKind::App,
                DependencyKind::Soft,
            ),
            DependencyDefinition::new(
                ServiceKind::StorageServer,
                ServiceKind::App,
                DependencyKind::Hard,
            ),
        ];
        let supervisor = ServiceSupervisor::<5, 4>::from_catalog(&services, &dependencies).unwrap();
        assert_eq!(supervisor.len(), 5);
        assert_eq!(supervisor.dependency_count(), 4);
        assert_eq!(
            supervisor.dependency_kind(ServiceKind::InputServer, ServiceKind::App),
            Some(DependencyKind::Soft)
        );
        assert_eq!(
            supervisor.dependency_kind(ServiceKind::StorageServer, ServiceKind::App),
            Some(DependencyKind::Hard)
        );

        let duplicate = [services[0], services[0]];
        let duplicate_error = match ServiceSupervisor::<5, 4>::from_catalog(&duplicate, &[]) {
            Err(error) => error,
            Ok(_) => panic!("duplicate catalog unexpectedly succeeded"),
        };
        assert_eq!(
            duplicate_error,
            CatalogError::Service {
                index: 1,
                source: SupervisorError::DuplicateService {
                    kind: ServiceKind::ServiceManager
                }
            }
        );
        let full_error = match ServiceSupervisor::<1, 0>::from_catalog(&services[..2], &[]) {
            Err(error) => error,
            Ok(_) => panic!("over-capacity catalog unexpectedly succeeded"),
        };
        assert_eq!(
            full_error,
            CatalogError::Service {
                index: 1,
                source: SupervisorError::Full
            }
        );

        let cycle = [
            DependencyDefinition::new(
                ServiceKind::ServiceManager,
                ServiceKind::SurfaceServer,
                DependencyKind::Hard,
            ),
            DependencyDefinition::new(
                ServiceKind::SurfaceServer,
                ServiceKind::ServiceManager,
                DependencyKind::Hard,
            ),
        ];
        let cycle_error = match ServiceSupervisor::<5, 4>::from_catalog(&services, &cycle) {
            Err(error) => error,
            Ok(_) => panic!("cyclic catalog unexpectedly succeeded"),
        };
        assert_eq!(
            cycle_error,
            CatalogError::Dependency {
                index: 1,
                source: DependencyError::Cycle {
                    prerequisite: ServiceKind::SurfaceServer,
                    dependent: ServiceKind::ServiceManager
                }
            }
        );
    }

    #[test]
    fn catalog_iter_accepts_sparse_manifest_staging_without_exposing_partial_state() {
        let manager =
            ServiceDefinition::new(identity(ServiceKind::ServiceManager, 1, 25), policy(1));
        let surface =
            ServiceDefinition::new(identity(ServiceKind::SurfaceServer, 1, 26), policy(1));
        let staged_services = [Some(surface), None, Some(manager)];
        let staged_dependencies = [
            None,
            Some(DependencyDefinition::new(
                ServiceKind::ServiceManager,
                ServiceKind::SurfaceServer,
                DependencyKind::Hard,
            )),
        ];
        let supervisor = ServiceSupervisor::<5, 10>::from_catalog_iter(
            staged_services.into_iter().flatten(),
            staged_dependencies.into_iter().flatten(),
        )
        .unwrap();
        assert_eq!(supervisor.len(), 2);
        assert_eq!(supervisor.dependency_count(), 1);
        assert_eq!(
            supervisor.dependency_kind(ServiceKind::ServiceManager, ServiceKind::SurfaceServer),
            Some(DependencyKind::Hard)
        );
    }

    #[test]
    fn probe_batch_is_all_or_none_and_keeps_catalog_order() {
        let manager = identity(ServiceKind::ServiceManager, 1, 30);
        let surface = identity(ServiceKind::SurfaceServer, 1, 31);
        let storage = identity(ServiceKind::StorageServer, 1, 32);
        let services = [
            ServiceDefinition::new(manager, policy(1)),
            ServiceDefinition::new(surface, policy(1)),
            ServiceDefinition::new(storage, policy(1)),
        ];
        let mut supervisor = ServiceSupervisor::<3, 0>::from_catalog(&services, &[]).unwrap();
        let first = supervisor.arm_probe_batch(100).unwrap();
        assert_eq!(first.len(), 3);
        assert!(!first.is_empty());
        assert_eq!(
            first
                .iter()
                .map(HealthFrame::identity)
                .collect::<std::vec::Vec<_>>(),
            std::vec![manager, surface, storage]
        );
        assert!(first.iter().all(|frame| frame.sequence() == 1));

        assert_eq!(
            supervisor.arm_probe_batch(101),
            Err(SupervisorError::InvalidTransition {
                phase: ServicePhase::ProbeArmed,
                operation: SupervisorOperation::ArmProbe
            })
        );
        for service in [manager, surface, storage] {
            let snapshot = supervisor.service(service.kind()).unwrap();
            assert_eq!(snapshot.last_outbound_sequence, 1);
            assert_eq!(snapshot.probe_deadline_ns, Some(150));
            supervisor
                .record_healthy(HealthFrame::healthy(1, service).unwrap())
                .unwrap();
        }
        let second = supervisor.arm_probe_batch(200).unwrap();
        assert_eq!(second.len(), 3);
        assert!(second.iter().all(|frame| frame.sequence() == 2));
    }

    #[test]
    fn concurrent_missed_probes_are_tolerated_and_recover_independently() {
        let surface = identity(ServiceKind::SurfaceServer, 1, 40);
        let input = identity(ServiceKind::InputServer, 1, 41);
        let tolerant = policy(1).with_missed_probe_tolerance(1).unwrap();
        let services = [
            ServiceDefinition::new(surface, tolerant),
            ServiceDefinition::new(input, tolerant),
        ];
        let mut supervisor = ServiceSupervisor::<2, 0>::from_catalog(&services, &[]).unwrap();
        let missed = supervisor.arm_probe_batch(0).unwrap();
        assert_eq!(missed.len(), 2);
        for service in [surface, input] {
            assert_eq!(
                supervisor.check_health_deadline(service, 49),
                Ok(HealthDeadlineOutcome::Pending { deadline_ns: 50 })
            );
            assert_eq!(
                supervisor.check_health_deadline(service, 50),
                Ok(HealthDeadlineOutcome::ToleratedMiss {
                    consecutive: 1,
                    tolerance: 1
                })
            );
        }

        let recovery = supervisor.arm_probe_batch(80).unwrap();
        for frame in recovery.iter() {
            assert_eq!(frame.sequence(), 2);
            supervisor
                .record_healthy(HealthFrame::healthy(2, frame.identity()).unwrap())
                .unwrap();
        }
        for service in [surface, input] {
            let snapshot = supervisor.service(service.kind()).unwrap();
            assert_eq!(snapshot.phase, ServicePhase::Healthy);
            assert_eq!(snapshot.consecutive_missed_probes, 0);
            assert_eq!(snapshot.total_missed_probes, 1);
            assert_eq!(snapshot.last_outbound_sequence, 2);
            assert_eq!(snapshot.last_inbound_sequence, 2);
        }
    }

    #[test]
    fn consecutive_miss_beyond_tolerance_uses_existing_restart_budget() {
        let app = identity(ServiceKind::App, 1, 50);
        let tolerant = policy(1).with_missed_probe_tolerance(1).unwrap();
        let mut supervisor = ServiceSupervisor::<1, 0>::new();
        supervisor.register(app, tolerant).unwrap();
        supervisor.arm_probe(app, 0).unwrap();
        assert_eq!(
            supervisor.check_health_deadline(app, 50),
            Ok(HealthDeadlineOutcome::ToleratedMiss {
                consecutive: 1,
                tolerance: 1
            })
        );
        supervisor.arm_probe(app, 60).unwrap();
        assert_eq!(
            supervisor.check_health_deadline(app, 110),
            Ok(HealthDeadlineOutcome::Fault(
                FaultDisposition::RestartAllowed {
                    attempt: 1,
                    budget: 1,
                    class: FaultClass::HealthTimeout
                }
            ))
        );
        let snapshot = supervisor.service(ServiceKind::App).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::Faulted);
        assert_eq!(snapshot.consecutive_missed_probes, 2);
        assert_eq!(snapshot.total_missed_probes, 2);
        assert_eq!(snapshot.last_fault, Some(FaultClass::HealthTimeout));
    }

    #[test]
    fn probe_and_healthy_transition_use_fixed_timeout_and_sequences() {
        let input = identity(ServiceKind::InputServer, 1, 42);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(input, policy(1)).unwrap();
        let probe = supervisor.arm_probe(input, 100).unwrap();
        assert_eq!(probe.opcode(), HealthOpcode::Probe);
        assert_eq!(probe.sequence(), 1);
        assert_eq!(probe.interval_ns(), 50);
        assert_eq!(
            supervisor.service(input.kind()).unwrap().probe_deadline_ns,
            Some(150)
        );
        supervisor
            .record_healthy(HealthFrame::healthy(1, input).unwrap())
            .unwrap();
        let snapshot = supervisor.service(input.kind()).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::Healthy);
        assert_eq!(snapshot.probe_deadline_ns, None);
        assert_eq!(snapshot.last_outbound_sequence, 1);
        assert_eq!(snapshot.last_inbound_sequence, 1);
    }

    #[test]
    fn health_timeout_is_edge_triggered_and_classified() {
        let surface = identity(ServiceKind::SurfaceServer, 1, 51);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(surface, policy(1)).unwrap();
        supervisor.arm_probe(surface, 10).unwrap();
        assert_eq!(supervisor.check_health_timeout(surface, 59), Ok(None));
        assert_eq!(
            supervisor.service(surface.kind()).unwrap().phase,
            ServicePhase::ProbeArmed
        );
        assert_eq!(
            supervisor.check_health_timeout(surface, 60),
            Ok(Some(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::HealthTimeout
            }))
        );
        let snapshot = supervisor.service(surface.kind()).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::Faulted);
        assert_eq!(snapshot.last_fault, Some(FaultClass::HealthTimeout));
    }

    #[test]
    fn budget_one_allows_one_replacement_then_quarantines_second_fault() {
        let first = identity(ServiceKind::InputServer, 1, 60);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(first, policy(1)).unwrap();
        supervisor.arm_probe(first, 0).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, first).unwrap())
            .unwrap();
        assert_eq!(
            supervisor.record_fault(HealthFrame::fault(2, first, FaultClass::ProcessExit).unwrap()),
            Ok(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::ProcessExit
            })
        );
        assert_eq!(supervisor.begin_backoff(first, 100), Ok(130));
        assert_eq!(
            supervisor.begin_replacement(first, 129),
            Err(SupervisorError::BackoffNotElapsed {
                deadline_ns: 130,
                now_ns: 129
            })
        );
        assert_eq!(
            supervisor.service(first.kind()).unwrap().phase,
            ServicePhase::Backoff
        );
        supervisor.begin_replacement(first, 130).unwrap();
        let second = supervisor.install_replacement(first, 2, 61).unwrap();
        assert_eq!(second, identity(ServiceKind::InputServer, 2, 61));
        assert_eq!(supervisor.service(second.kind()).unwrap().restarts_used, 1);

        supervisor.arm_probe(second, 200).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, second).unwrap())
            .unwrap();
        assert_eq!(
            supervisor.record_fault(
                HealthFrame::fault(2, second, FaultClass::ProtocolViolation).unwrap()
            ),
            Ok(FaultDisposition::QuarantineRequired {
                attempt: 2,
                budget: 1,
                class: FaultClass::ProtocolViolation
            })
        );
        let quarantine = supervisor.quarantine(second).unwrap();
        assert_eq!(quarantine.opcode(), HealthOpcode::Quarantine);
        assert_eq!(quarantine.sequence(), 2);
        assert_eq!(quarantine.restart_attempt(), 2);
        assert_eq!(quarantine.restart_budget(), 1);
        assert_eq!(
            supervisor.service(second.kind()).unwrap().phase,
            ServicePhase::Quarantined
        );

        supervisor
            .acknowledge_degraded(HealthFrame::degraded_ack(3, second).unwrap())
            .unwrap();
        assert_eq!(
            supervisor.service(second.kind()).unwrap().phase,
            ServicePhase::Degraded
        );
    }

    #[test]
    fn replacement_requires_exact_next_generation_and_new_pid() {
        let first = identity(ServiceKind::SurfaceServer, 7, 70);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(first, policy(2)).unwrap();
        supervisor
            .classify_fault(first, FaultClass::ProcessExit)
            .unwrap();
        supervisor.begin_backoff(first, 0).unwrap();
        supervisor.begin_replacement(first, 30).unwrap();
        assert_eq!(
            supervisor.install_replacement(first, 9, 71),
            Err(SupervisorError::ReplacementGenerationMismatch {
                expected: 8,
                actual: 9
            })
        );
        assert_eq!(
            supervisor.install_replacement(first, 8, 70),
            Err(SupervisorError::ReplacementPidMustChange)
        );
        assert_eq!(
            supervisor.service(first.kind()).unwrap().phase,
            ServicePhase::AwaitingReplacement
        );
        assert_eq!(
            supervisor.install_replacement(first, 8, 71),
            Ok(identity(ServiceKind::SurfaceServer, 8, 71))
        );
    }

    #[test]
    fn stale_generation_wrong_pid_and_unknown_service_are_rejected() {
        let first = identity(ServiceKind::InputServer, 1, 80);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(first, policy(1)).unwrap();
        let stale_generation = identity(ServiceKind::InputServer, 2, 80);
        let wrong_pid = identity(ServiceKind::InputServer, 1, 81);
        for actual in [stale_generation, wrong_pid] {
            assert_eq!(
                supervisor.arm_probe(actual, 0),
                Err(SupervisorError::IdentityMismatch {
                    expected: first,
                    actual
                })
            );
        }
        let surface = identity(ServiceKind::SurfaceServer, 1, 90);
        assert_eq!(
            supervisor.arm_probe(surface, 0),
            Err(SupervisorError::ServiceNotFound {
                kind: ServiceKind::SurfaceServer
            })
        );
        assert_eq!(
            supervisor.service(first.kind()).unwrap().phase,
            ServicePhase::Registered
        );
    }

    #[test]
    fn inbound_sequence_rollback_leaves_probe_armed() {
        let manager = identity(ServiceKind::ServiceManager, 1, 90);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(manager, policy(1)).unwrap();
        supervisor.arm_probe(manager, 0).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(5, manager).unwrap())
            .unwrap();
        supervisor.arm_probe(manager, 100).unwrap();
        assert_eq!(
            supervisor.record_healthy(HealthFrame::healthy(5, manager).unwrap()),
            Err(SupervisorError::Sequence(SequenceError::NotIncreasing {
                last: 5,
                received: 5
            }))
        );
        let snapshot = supervisor.service(manager.kind()).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::ProbeArmed);
        assert_eq!(snapshot.last_inbound_sequence, 5);
    }

    #[test]
    fn services_have_independent_state_budgets_and_sequences() {
        let manager = identity(ServiceKind::ServiceManager, 1, 100);
        let surface = identity(ServiceKind::SurfaceServer, 3, 101);
        let input = identity(ServiceKind::InputServer, 8, 102);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [manager, surface, input] {
            supervisor.register(service, policy(1)).unwrap();
            let probe = supervisor.arm_probe(service, 10).unwrap();
            assert_eq!(probe.sequence(), 1);
        }
        supervisor
            .record_healthy(HealthFrame::healthy(1, manager).unwrap())
            .unwrap();
        supervisor
            .record_fault(HealthFrame::fault(7, input, FaultClass::ProcessExit).unwrap())
            .unwrap();

        assert_eq!(
            supervisor.service(manager.kind()).unwrap().phase,
            ServicePhase::Healthy
        );
        assert_eq!(
            supervisor.service(surface.kind()).unwrap().phase,
            ServicePhase::ProbeArmed
        );
        let input_snapshot = supervisor.service(input.kind()).unwrap();
        assert_eq!(input_snapshot.phase, ServicePhase::Faulted);
        assert_eq!(input_snapshot.last_inbound_sequence, 7);
        assert_eq!(
            supervisor
                .service(manager.kind())
                .unwrap()
                .last_inbound_sequence,
            1
        );
    }

    #[test]
    fn replacement_failure_enters_terminal_quarantine_path() {
        let manager = identity(ServiceKind::ServiceManager, 1, 110);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(manager, policy(1)).unwrap();
        supervisor
            .classify_fault(manager, FaultClass::ProtocolViolation)
            .unwrap();
        supervisor.begin_backoff(manager, 0).unwrap();
        supervisor.begin_replacement(manager, 30).unwrap();
        assert_eq!(
            supervisor.replacement_failed(manager),
            Ok(FaultDisposition::QuarantineRequired {
                attempt: 2,
                budget: 1,
                class: FaultClass::ReplacementFailed
            })
        );
        let snapshot = supervisor.service(manager.kind()).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::QuarantinePending);
        assert_eq!(snapshot.last_fault, Some(FaultClass::ReplacementFailed));
        assert_eq!(
            supervisor.quarantine(manager).unwrap().fault_class(),
            Some(FaultClass::RestartBudgetExhausted)
        );
    }

    #[test]
    fn invalid_transitions_and_deadline_overflow_are_fail_closed() {
        let input = identity(ServiceKind::InputServer, 1, 120);
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor.register(input, policy(1)).unwrap();
        assert_eq!(
            supervisor.classify_fault(input, FaultClass::RestartBudgetExhausted),
            Err(SupervisorError::FaultClassReservedForSupervisor)
        );
        assert_eq!(
            supervisor.begin_backoff(input, 0),
            Err(SupervisorError::InvalidTransition {
                phase: ServicePhase::Registered,
                operation: SupervisorOperation::BeginBackoff
            })
        );
        assert_eq!(
            supervisor.arm_probe(input, u64::MAX),
            Err(SupervisorError::DeadlineOverflow)
        );
        let snapshot = supervisor.service(input.kind()).unwrap();
        assert_eq!(snapshot.phase, ServicePhase::Registered);
        assert_eq!(snapshot.last_outbound_sequence, 0);
        assert_eq!(snapshot.restarts_used, 0);
    }

    #[test]
    fn dependency_graph_rejects_unregistered_self_duplicate_and_cycle_edges() {
        let manager = identity(ServiceKind::ServiceManager, 1, 200);
        let surface = identity(ServiceKind::SurfaceServer, 1, 201);
        let input = identity(ServiceKind::InputServer, 1, 202);
        let mut supervisor = ServiceSupervisor::<3>::new();
        supervisor.register(surface, policy(1)).unwrap();
        supervisor.register(input, policy(1)).unwrap();

        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::SurfaceServer,
                DependencyKind::Hard
            ),
            Err(DependencyError::SelfDependency {
                service: ServiceKind::SurfaceServer
            })
        );
        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Hard
            ),
            Err(DependencyError::ServiceNotFound {
                kind: ServiceKind::ServiceManager
            })
        );
        supervisor.register(manager, policy(1)).unwrap();
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Soft
            ),
            Err(DependencyError::Duplicate {
                prerequisite: ServiceKind::SurfaceServer,
                dependent: ServiceKind::InputServer
            })
        );
        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::InputServer,
                ServiceKind::SurfaceServer,
                DependencyKind::Soft
            ),
            Err(DependencyError::Cycle {
                prerequisite: ServiceKind::InputServer,
                dependent: ServiceKind::SurfaceServer
            })
        );
        assert_eq!(supervisor.dependency_count(), 1);
        assert_eq!(supervisor.dependency_capacity(), 3);
        assert_eq!(
            supervisor.dependency_kind(ServiceKind::SurfaceServer, ServiceKind::InputServer),
            Some(DependencyKind::Hard)
        );
    }

    #[test]
    fn dependency_edge_storage_is_strictly_fixed_capacity() {
        let mut supervisor = ServiceSupervisor::<3, 1>::new();
        for service in [
            identity(ServiceKind::ServiceManager, 1, 210),
            identity(ServiceKind::SurfaceServer, 1, 211),
            identity(ServiceKind::InputServer, 1, 212),
        ] {
            supervisor.register(service, policy(1)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Soft
            ),
            Err(DependencyError::Full)
        );
        assert_eq!(supervisor.dependency_count(), 1);
        assert_eq!(supervisor.dependency_capacity(), 1);
    }

    #[test]
    fn storage_to_app_hard_edge_blocks_until_replacement_is_healthy() {
        let storage = identity(ServiceKind::StorageServer, 1, 0x0000_0001_0000_000a);
        let app = identity(ServiceKind::App, 1, 0x0000_0001_0000_0009);
        let mut supervisor = ServiceSupervisor::<2, 1>::new();
        supervisor.register(storage, policy(1)).unwrap();
        supervisor.register(app, policy(1)).unwrap();
        supervisor
            .add_dependency(
                ServiceKind::StorageServer,
                ServiceKind::App,
                DependencyKind::Hard,
            )
            .unwrap();

        assert_eq!(
            supervisor.classify_fault(storage, FaultClass::HealthTimeout),
            Ok(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::HealthTimeout,
            })
        );
        let blocked = supervisor.dependency_fault(storage).unwrap();
        assert_eq!(blocked.len(), 2);
        assert_eq!(blocked.get(0).unwrap().service, storage);
        assert_eq!(
            blocked.get(1),
            Some(DependencyTransition {
                service: app,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::HardBlocked,
            })
        );

        supervisor.begin_backoff(storage, 0).unwrap();
        supervisor.begin_replacement(storage, 30).unwrap();
        let replacement = supervisor
            .install_replacement(storage, 2, 0x0000_0002_0000_000a)
            .unwrap();
        supervisor.arm_probe(replacement, 40).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, replacement).unwrap())
            .unwrap();
        let resumed = supervisor.dependency_recovered(replacement).unwrap();
        assert_eq!(resumed.len(), 2);
        assert_eq!(resumed.get(0).unwrap().service, replacement);
        assert_eq!(
            resumed.get(1),
            Some(DependencyTransition {
                service: app,
                previous: DependencyImpact::HardBlocked,
                current: DependencyImpact::Unaffected,
            })
        );
        assert_eq!(
            supervisor.dependency_impact(ServiceKind::StorageServer),
            Ok(DependencyImpact::Unaffected)
        );
        assert_eq!(
            supervisor.dependency_impact(ServiceKind::App),
            Ok(DependencyImpact::Unaffected)
        );
    }

    #[test]
    fn hard_and_soft_fault_impacts_are_deterministic_and_keep_soft_live() {
        let manager = identity(ServiceKind::ServiceManager, 1, 220);
        let surface = identity(ServiceKind::SurfaceServer, 1, 221);
        let input = identity(ServiceKind::InputServer, 1, 222);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [input, manager, surface] {
            supervisor.register(service, policy(2)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::ServiceManager,
                DependencyKind::Soft,
            )
            .unwrap();
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();

        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();
        let transitions = supervisor.dependency_fault(surface).unwrap();
        assert_eq!(transitions.len(), 3);
        assert_eq!(
            transitions.get(0),
            Some(DependencyTransition {
                service: surface,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::HardBlocked
            })
        );
        assert_eq!(
            transitions.get(1),
            Some(DependencyTransition {
                service: manager,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::SoftDegraded
            })
        );
        assert_eq!(
            transitions.get(2),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::HardBlocked
            })
        );
        assert_eq!(
            supervisor
                .dependency_impact(ServiceKind::ServiceManager)
                .unwrap(),
            DependencyImpact::SoftDegraded
        );
        assert_eq!(
            supervisor
                .dependency_impact(ServiceKind::InputServer)
                .unwrap(),
            DependencyImpact::HardBlocked
        );
        assert_eq!(
            supervisor
                .service(ServiceKind::ServiceManager)
                .unwrap()
                .phase,
            ServicePhase::Registered
        );
        assert_eq!(
            supervisor.service(ServiceKind::InputServer).unwrap().phase,
            ServicePhase::Registered
        );
    }

    #[test]
    fn two_node_soft_edge_blocks_root_but_keeps_dependent_live() {
        let surface = identity(ServiceKind::SurfaceServer, 1, 230);
        let input = identity(ServiceKind::InputServer, 1, 231);
        let mut supervisor = ServiceSupervisor::<2>::new();
        supervisor.register(surface, policy(1)).unwrap();
        supervisor.register(input, policy(1)).unwrap();
        supervisor
            .add_dependency(
                ServiceKind::InputServer,
                ServiceKind::SurfaceServer,
                DependencyKind::Soft,
            )
            .unwrap();
        supervisor
            .classify_fault(input, FaultClass::HealthTimeout)
            .unwrap();

        let transitions = supervisor.dependency_fault(input).unwrap();
        assert_eq!(transitions.len(), 2);
        assert_eq!(
            transitions.get(0),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::HardBlocked
            })
        );
        assert_eq!(
            transitions.get(1),
            Some(DependencyTransition {
                service: surface,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::SoftDegraded
            })
        );
        assert_eq!(
            supervisor
                .dependency_impact(ServiceKind::InputServer)
                .unwrap(),
            DependencyImpact::HardBlocked
        );
        assert_eq!(
            supervisor
                .dependency_impact(ServiceKind::SurfaceServer)
                .unwrap(),
            DependencyImpact::SoftDegraded
        );
        assert_eq!(
            supervisor
                .service(ServiceKind::SurfaceServer)
                .unwrap()
                .phase,
            ServicePhase::Registered
        );
    }

    #[test]
    fn soft_impact_does_not_propagate_as_an_outage() {
        let manager = identity(ServiceKind::ServiceManager, 1, 240);
        let surface = identity(ServiceKind::SurfaceServer, 1, 241);
        let input = identity(ServiceKind::InputServer, 1, 242);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [manager, surface, input] {
            supervisor.register(service, policy(1)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::ServiceManager,
                DependencyKind::Soft,
            )
            .unwrap();
        supervisor
            .add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();

        let transitions = supervisor.dependency_fault(surface).unwrap();
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions.get(0).unwrap().service, surface);
        assert_eq!(transitions.get(1).unwrap().service, manager);
        assert_eq!(
            supervisor
                .dependency_impact(ServiceKind::InputServer)
                .unwrap(),
            DependencyImpact::Unaffected
        );
    }

    #[test]
    fn hard_impacts_and_recovery_follow_topological_order() {
        let manager = identity(ServiceKind::ServiceManager, 1, 250);
        let surface = identity(ServiceKind::SurfaceServer, 1, 251);
        let input = identity(ServiceKind::InputServer, 1, 252);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [input, manager, surface] {
            supervisor.register(service, policy(2)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::ServiceManager,
                DependencyKind::Hard,
            )
            .unwrap();
        supervisor
            .add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();
        let fault = supervisor.dependency_fault(surface).unwrap();
        assert_eq!(fault.len(), 3);
        assert_eq!(fault.get(0).unwrap().service, surface);
        assert_eq!(fault.get(1).unwrap().service, manager);
        assert_eq!(fault.get(2).unwrap().service, input);

        supervisor.begin_backoff(surface, 0).unwrap();
        supervisor.begin_replacement(surface, 30).unwrap();
        let replacement = supervisor.install_replacement(surface, 2, 253).unwrap();
        supervisor.arm_probe(replacement, 40).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, replacement).unwrap())
            .unwrap();
        let recovery = supervisor.dependency_recovered(replacement).unwrap();
        assert_eq!(recovery.len(), 3);
        assert_eq!(recovery.get(0).unwrap().service, replacement);
        assert_eq!(recovery.get(1).unwrap().service, manager);
        assert_eq!(recovery.get(2).unwrap().service, input);
        assert!(recovery.iter().all(|transition| transition.previous
            == DependencyImpact::HardBlocked
            && transition.current == DependencyImpact::Unaffected));
    }

    #[test]
    fn same_root_fault_is_idempotent_and_cannot_create_a_restart_storm() {
        let surface = identity(ServiceKind::SurfaceServer, 1, 260);
        let input = identity(ServiceKind::InputServer, 1, 261);
        let mut supervisor = ServiceSupervisor::<2>::new();
        supervisor.register(surface, policy(2)).unwrap();
        supervisor.register(input, policy(2)).unwrap();
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        assert_eq!(
            supervisor.classify_fault(surface, FaultClass::ProcessExit),
            Ok(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 2,
                class: FaultClass::ProcessExit
            })
        );
        assert_eq!(supervisor.dependency_fault(surface).unwrap().len(), 2);
        assert!(supervisor.dependency_fault(surface).unwrap().is_empty());
        assert_eq!(supervisor.begin_backoff(surface, 0), Ok(30));
        assert!(supervisor.dependency_fault(surface).unwrap().is_empty());
        assert_eq!(
            supervisor.classify_fault(surface, FaultClass::HealthTimeout),
            Err(SupervisorError::InvalidTransition {
                phase: ServicePhase::Backoff,
                operation: SupervisorOperation::RecordFault
            })
        );
        let root = supervisor.service(ServiceKind::SurfaceServer).unwrap();
        let dependent = supervisor.service(ServiceKind::InputServer).unwrap();
        assert_eq!(root.restarts_used, 1);
        assert_eq!(dependent.restarts_used, 0);
        assert_eq!(dependent.last_outbound_sequence, 0);
        assert_eq!(dependent.last_inbound_sequence, 0);
    }

    #[test]
    fn multiple_roots_are_aggregated_and_released_independently() {
        let manager = identity(ServiceKind::ServiceManager, 1, 270);
        let surface = identity(ServiceKind::SurfaceServer, 1, 271);
        let input = identity(ServiceKind::InputServer, 1, 272);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [manager, surface, input] {
            supervisor.register(service, policy(2)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Soft,
            )
            .unwrap();
        supervisor
            .add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();

        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();
        let surface_fault = supervisor.dependency_fault(surface).unwrap();
        assert_eq!(surface_fault.get(0).unwrap().service, surface);
        assert_eq!(
            surface_fault.get(1),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::Unaffected,
                current: DependencyImpact::SoftDegraded
            })
        );
        supervisor
            .classify_fault(manager, FaultClass::HealthTimeout)
            .unwrap();
        let manager_fault = supervisor.dependency_fault(manager).unwrap();
        assert_eq!(manager_fault.get(0).unwrap().service, manager);
        assert_eq!(
            manager_fault.get(1),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::SoftDegraded,
                current: DependencyImpact::HardBlocked
            })
        );

        supervisor.begin_backoff(manager, 0).unwrap();
        supervisor.begin_replacement(manager, 30).unwrap();
        let manager = supervisor.install_replacement(manager, 2, 273).unwrap();
        supervisor.arm_probe(manager, 40).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, manager).unwrap())
            .unwrap();
        let manager_recovery = supervisor.dependency_recovered(manager).unwrap();
        assert_eq!(manager_recovery.get(0).unwrap().service, manager);
        assert_eq!(
            manager_recovery.get(1),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::HardBlocked,
                current: DependencyImpact::SoftDegraded
            })
        );

        supervisor.begin_backoff(surface, 100).unwrap();
        supervisor.begin_replacement(surface, 130).unwrap();
        let surface = supervisor.install_replacement(surface, 2, 274).unwrap();
        supervisor.arm_probe(surface, 140).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, surface).unwrap())
            .unwrap();
        let surface_recovery = supervisor.dependency_recovered(surface).unwrap();
        assert_eq!(surface_recovery.get(0).unwrap().service, surface);
        assert_eq!(
            surface_recovery.get(1),
            Some(DependencyTransition {
                service: input,
                previous: DependencyImpact::SoftDegraded,
                current: DependencyImpact::Unaffected
            })
        );
        assert!(supervisor.dependency_recovered(surface).unwrap().is_empty());
    }

    #[test]
    fn hard_path_dominates_soft_path_from_the_same_root() {
        let manager = identity(ServiceKind::ServiceManager, 1, 280);
        let surface = identity(ServiceKind::SurfaceServer, 1, 281);
        let input = identity(ServiceKind::InputServer, 1, 282);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [manager, surface, input] {
            supervisor.register(service, policy(1)).unwrap();
        }
        for (prerequisite, dependent, kind) in [
            (
                ServiceKind::SurfaceServer,
                ServiceKind::ServiceManager,
                DependencyKind::Hard,
            ),
            (
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Soft,
            ),
            (
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            ),
        ] {
            supervisor
                .add_dependency(prerequisite, dependent, kind)
                .unwrap();
        }
        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();
        let transitions = supervisor.dependency_fault(surface).unwrap();
        assert_eq!(transitions.get(0).unwrap().service, surface);
        assert_eq!(transitions.get(1).unwrap().service, manager);
        assert_eq!(transitions.get(2).unwrap().service, input);
        assert_eq!(
            transitions.get(2).unwrap().current,
            DependencyImpact::HardBlocked
        );
    }

    #[test]
    fn dependency_reporting_rejects_stale_identity_wrong_phase_and_live_topology_changes() {
        let manager = identity(ServiceKind::ServiceManager, 1, 290);
        let surface = identity(ServiceKind::SurfaceServer, 1, 291);
        let input = identity(ServiceKind::InputServer, 1, 292);
        let mut supervisor = ServiceSupervisor::<3>::new();
        for service in [manager, surface, input] {
            supervisor.register(service, policy(1)).unwrap();
        }
        supervisor
            .add_dependency(
                ServiceKind::SurfaceServer,
                ServiceKind::InputServer,
                DependencyKind::Hard,
            )
            .unwrap();
        assert_eq!(
            supervisor.dependency_fault(surface),
            Err(DependencyError::RootStillAvailable {
                kind: ServiceKind::SurfaceServer,
                phase: ServicePhase::Registered
            })
        );
        let stale = identity(ServiceKind::SurfaceServer, 2, 291);
        assert_eq!(
            supervisor.dependency_fault(stale),
            Err(DependencyError::IdentityMismatch {
                expected: surface,
                actual: stale
            })
        );
        assert_eq!(
            supervisor.dependency_impact(ServiceKind::ServiceManager),
            Ok(DependencyImpact::Unaffected)
        );
        supervisor
            .classify_fault(surface, FaultClass::ProcessExit)
            .unwrap();
        supervisor.dependency_fault(surface).unwrap();
        assert_eq!(
            supervisor.add_dependency(
                ServiceKind::ServiceManager,
                ServiceKind::InputServer,
                DependencyKind::Soft
            ),
            Err(DependencyError::TopologyActive)
        );
        assert_eq!(
            supervisor.dependency_recovered(surface),
            Err(DependencyError::RootNotHealthy {
                kind: ServiceKind::SurfaceServer,
                phase: ServicePhase::Faulted
            })
        );
    }

    #[test]
    fn stable_replacements_rearm_one_window_without_weakening_each_budget() {
        let storage = identity(ServiceKind::StorageServer, 1, 300);
        let app = identity(ServiceKind::App, 1, 301);
        let mut supervisor = ServiceSupervisor::<2, 1>::new();
        supervisor.register(storage, policy(1)).unwrap();
        supervisor.register(app, policy(1)).unwrap();
        supervisor
            .add_dependency(
                ServiceKind::StorageServer,
                ServiceKind::App,
                DependencyKind::Hard,
            )
            .unwrap();

        assert_eq!(
            supervisor.classify_fault(storage, FaultClass::ProcessExit),
            Ok(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::ProcessExit
            })
        );
        assert_eq!(supervisor.dependency_fault(storage).unwrap().len(), 2);
        supervisor.begin_backoff(storage, 0).unwrap();
        supervisor.begin_replacement(storage, 30).unwrap();
        let replacement = supervisor.install_replacement(storage, 2, 302).unwrap();

        supervisor.arm_probe(replacement, 40).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(1, replacement).unwrap())
            .unwrap();
        assert_eq!(
            supervisor.rearm_restart_budget(replacement),
            Err(SupervisorError::DependencyRootStillActive {
                kind: ServiceKind::StorageServer
            })
        );
        supervisor.dependency_recovered(replacement).unwrap();
        assert_eq!(
            supervisor.rearm_restart_budget(replacement),
            Err(SupervisorError::RecoveryStabilityPending {
                observed: 1,
                required: RECOVERY_STABILITY_HEALTHY_ROUNDS
            })
        );
        supervisor.arm_probe(replacement, 80).unwrap();
        supervisor
            .record_healthy(HealthFrame::healthy(2, replacement).unwrap())
            .unwrap();
        supervisor.rearm_restart_budget(replacement).unwrap();

        let first_window = supervisor.service(ServiceKind::StorageServer).unwrap();
        assert_eq!(first_window.restarts_used, 0);
        assert_eq!(first_window.healthy_streak, 2);
        assert_eq!(first_window.restart_budget_rearms, 1);
        assert_eq!(
            supervisor.rearm_restart_budget(replacement),
            Err(SupervisorError::RestartBudgetNotConsumed)
        );

        assert_eq!(
            supervisor.classify_fault(replacement, FaultClass::ProcessExit),
            Ok(FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::ProcessExit
            })
        );
        supervisor.dependency_fault(replacement).unwrap();
        supervisor.begin_backoff(replacement, 100).unwrap();
        supervisor.begin_replacement(replacement, 130).unwrap();
        let second = supervisor.install_replacement(replacement, 3, 303).unwrap();
        for (sequence, now) in [(1, 140), (2, 180)] {
            supervisor.arm_probe(second, now).unwrap();
            supervisor
                .record_healthy(HealthFrame::healthy(sequence, second).unwrap())
                .unwrap();
        }
        supervisor.dependency_recovered(second).unwrap();
        supervisor.rearm_restart_budget(second).unwrap();

        let second_window = supervisor.service(ServiceKind::StorageServer).unwrap();
        assert_eq!(second_window.identity, second);
        assert_eq!(second_window.restarts_used, 0);
        assert_eq!(second_window.healthy_streak, 2);
        assert_eq!(second_window.restart_budget_rearms, 2);
        assert_eq!(second_window.last_fault, Some(FaultClass::ProcessExit));
    }
}
