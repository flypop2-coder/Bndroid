//! Kernel-owned, immutable service-manifest wire format.
//!
//! `BMF1` is deliberately small and allocation-free. The kernel publishes one
//! exact immutable VMO, while init decodes the complete payload before it
//! binds or spawns any service. A decoded manifest therefore never exposes a
//! partially validated service or dependency table.

use bndr_abi::{ShutdownServiceNode, UserImageId};

use crate::health::{DependencyKind, ServiceKind, ServicePolicy, ServicePolicyError};

pub const SERVICE_MANIFEST_MAGIC: [u8; 4] = *b"BMF1";
pub const SERVICE_MANIFEST_VERSION: u8 = 1;
pub const SERVICE_MANIFEST_GENERATION: u32 = 1;
pub const VERIFIED_SERVICE_MANIFEST_GENERATION: u32 = 2;
pub const PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION: u32 = 3;
pub const KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION: u32 = 4;
pub const KEY_ROTATION_SERVICE_MANIFEST_GENERATION: u32 = 5;
pub const KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION: u32 = 6;
pub const SERVICE_MANIFEST_HEADER_SIZE: usize = 32;
pub const SERVICE_MANIFEST_SERVICE_RECORD_SIZE: usize = 32;
pub const SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE: usize = 8;
pub const SERVICE_MANIFEST_MAX_SERVICES: usize = 5;
pub const SERVICE_MANIFEST_MAX_DEPENDENCIES: usize = 10;
pub const PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT: usize = 5;
pub const PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT: usize = 4;
pub const PRODUCT_SERVICE_MANIFEST_SIZE: usize = SERVICE_MANIFEST_HEADER_SIZE
    + PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT * SERVICE_MANIFEST_SERVICE_RECORD_SIZE
    + PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
pub const PRODUCT_SERVICE_MANIFEST_BYTES: [u8; PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(SERVICE_MANIFEST_GENERATION);
pub const PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&PRODUCT_SERVICE_MANIFEST_BYTES);
pub const VERIFIED_PRODUCT_SERVICE_MANIFEST_BYTES: [u8; PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(VERIFIED_SERVICE_MANIFEST_GENERATION);
pub const VERIFIED_PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&VERIFIED_PRODUCT_SERVICE_MANIFEST_BYTES);
pub const PERSISTENT_ROLLBACK_PRODUCT_SERVICE_MANIFEST_BYTES: [u8; PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION);
pub const PERSISTENT_ROLLBACK_PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&PERSISTENT_ROLLBACK_PRODUCT_SERVICE_MANIFEST_BYTES);
pub const KEY_ROTATION_TRANSITION_PRODUCT_SERVICE_MANIFEST_BYTES: [u8;
    PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION);
pub const KEY_ROTATION_TRANSITION_PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&KEY_ROTATION_TRANSITION_PRODUCT_SERVICE_MANIFEST_BYTES);
pub const KEY_ROTATION_PRODUCT_SERVICE_MANIFEST_BYTES: [u8; PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(KEY_ROTATION_SERVICE_MANIFEST_GENERATION);
pub const KEY_ROTATION_PRODUCT_SERVICE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&KEY_ROTATION_PRODUCT_SERVICE_MANIFEST_BYTES);
pub const KEY_ROTATION_RETIRED_KEY_FIXTURE_MANIFEST_BYTES: [u8; PRODUCT_SERVICE_MANIFEST_SIZE] =
    encode_product_service_manifest(KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION);
pub const KEY_ROTATION_RETIRED_KEY_FIXTURE_MANIFEST_FINGERPRINT: u64 =
    service_manifest_payload_fingerprint(&KEY_ROTATION_RETIRED_KEY_FIXTURE_MANIFEST_BYTES);

const VERSION_OFFSET: usize = 4;
const HEADER_SIZE_OFFSET: usize = 5;
const SERVICE_RECORD_SIZE_OFFSET: usize = 6;
const DEPENDENCY_RECORD_SIZE_OFFSET: usize = 7;
const TOTAL_SIZE_RANGE: core::ops::Range<usize> = 8..12;
const GENERATION_RANGE: core::ops::Range<usize> = 12..16;
const SERVICE_COUNT_OFFSET: usize = 16;
const DEPENDENCY_COUNT_OFFSET: usize = 17;
const HEADER_RESERVED_RANGE: core::ops::Range<usize> = 18..24;
const FINGERPRINT_RANGE: core::ops::Range<usize> = 24..32;

const SERVICE_KIND_OFFSET: usize = 0;
const SERVICE_IMAGE_OFFSET: usize = 1;
const SERVICE_LAUNCH_MODE_OFFSET: usize = 2;
const SERVICE_NODE_OFFSET: usize = 3;
const SERVICE_RESTART_BUDGET_RANGE: core::ops::Range<usize> = 4..8;
const SERVICE_RESTART_BACKOFF_RANGE: core::ops::Range<usize> = 8..16;
const SERVICE_HEALTH_TIMEOUT_RANGE: core::ops::Range<usize> = 16..24;
const SERVICE_MISSED_TOLERANCE_RANGE: core::ops::Range<usize> = 24..28;
const SERVICE_RESERVED_RANGE: core::ops::Range<usize> = 28..32;

const DEPENDENCY_PREREQUISITE_OFFSET: usize = 0;
const DEPENDENCY_DEPENDENT_OFFSET: usize = 1;
const DEPENDENCY_KIND_OFFSET: usize = 2;
const DEPENDENCY_RESERVED_RANGE: core::ops::Range<usize> = 3..8;
const NO_SHUTDOWN_NODE: u8 = u8::MAX;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceLaunchMode {
    BindResident = 1,
    Spawn = 2,
}

impl ServiceLaunchMode {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::BindResident),
            2 => Some(Self::Spawn),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestService {
    kind: ServiceKind,
    image: UserImageId,
    launch_mode: ServiceLaunchMode,
    shutdown_node: Option<ShutdownServiceNode>,
    policy: ServicePolicy,
}

impl ManifestService {
    pub const fn kind(self) -> ServiceKind {
        self.kind
    }

    pub const fn image(self) -> UserImageId {
        self.image
    }

    pub const fn launch_mode(self) -> ServiceLaunchMode {
        self.launch_mode
    }

    pub const fn shutdown_node(self) -> Option<ShutdownServiceNode> {
        self.shutdown_node
    }

    pub const fn policy(self) -> ServicePolicy {
        self.policy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestDependency {
    prerequisite: ServiceKind,
    dependent: ServiceKind,
    kind: DependencyKind,
}

impl ManifestDependency {
    pub const fn prerequisite(self) -> ServiceKind {
        self.prerequisite
    }

    pub const fn dependent(self) -> ServiceKind {
        self.dependent
    }

    pub const fn kind(self) -> DependencyKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceManifest {
    generation: u32,
    fingerprint: u64,
    wire_size: usize,
    services: [Option<ManifestService>; SERVICE_MANIFEST_MAX_SERVICES],
    service_count: usize,
    dependencies: [Option<ManifestDependency>; SERVICE_MANIFEST_MAX_DEPENDENCIES],
    dependency_count: usize,
}

impl ServiceManifest {
    /// Decodes and validates an entire immutable manifest transactionally.
    pub fn decode(wire: &[u8]) -> Result<Self, ManifestDecodeError> {
        if wire.len() < SERVICE_MANIFEST_HEADER_SIZE {
            return Err(ManifestDecodeError::TruncatedHeader { actual: wire.len() });
        }
        if wire[..SERVICE_MANIFEST_MAGIC.len()] != SERVICE_MANIFEST_MAGIC {
            return Err(ManifestDecodeError::BadMagic);
        }
        if wire[VERSION_OFFSET] != SERVICE_MANIFEST_VERSION {
            return Err(ManifestDecodeError::UnsupportedVersion(
                wire[VERSION_OFFSET],
            ));
        }
        if usize::from(wire[HEADER_SIZE_OFFSET]) != SERVICE_MANIFEST_HEADER_SIZE {
            return Err(ManifestDecodeError::HeaderSize(wire[HEADER_SIZE_OFFSET]));
        }
        if usize::from(wire[SERVICE_RECORD_SIZE_OFFSET]) != SERVICE_MANIFEST_SERVICE_RECORD_SIZE {
            return Err(ManifestDecodeError::ServiceRecordSize(
                wire[SERVICE_RECORD_SIZE_OFFSET],
            ));
        }
        if usize::from(wire[DEPENDENCY_RECORD_SIZE_OFFSET])
            != SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE
        {
            return Err(ManifestDecodeError::DependencyRecordSize(
                wire[DEPENDENCY_RECORD_SIZE_OFFSET],
            ));
        }
        if let Some(offset) = first_nonzero(wire, HEADER_RESERVED_RANGE) {
            return Err(ManifestDecodeError::ReservedNonZero { offset });
        }

        let service_count = usize::from(wire[SERVICE_COUNT_OFFSET]);
        let dependency_count = usize::from(wire[DEPENDENCY_COUNT_OFFSET]);
        if service_count == 0 {
            return Err(ManifestDecodeError::Empty);
        }
        if service_count > SERVICE_MANIFEST_MAX_SERVICES {
            return Err(ManifestDecodeError::ServiceCapacity {
                actual: service_count,
                maximum: SERVICE_MANIFEST_MAX_SERVICES,
            });
        }
        if dependency_count > SERVICE_MANIFEST_MAX_DEPENDENCIES {
            return Err(ManifestDecodeError::DependencyCapacity {
                actual: dependency_count,
                maximum: SERVICE_MANIFEST_MAX_DEPENDENCIES,
            });
        }
        let expected_size = SERVICE_MANIFEST_HEADER_SIZE
            + service_count * SERVICE_MANIFEST_SERVICE_RECORD_SIZE
            + dependency_count * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
        let declared_size = read_u32(wire, TOTAL_SIZE_RANGE) as usize;
        if declared_size != expected_size || wire.len() != expected_size {
            return Err(ManifestDecodeError::Length {
                declared: declared_size,
                expected: expected_size,
                actual: wire.len(),
            });
        }
        let generation = read_u32(wire, GENERATION_RANGE);
        if generation == 0 {
            return Err(ManifestDecodeError::GenerationMustBeNonZero);
        }
        let fingerprint = read_u64(wire, FINGERPRINT_RANGE);
        let computed_fingerprint = service_manifest_payload_fingerprint(wire);
        if fingerprint != computed_fingerprint {
            return Err(ManifestDecodeError::Fingerprint {
                expected: fingerprint,
                actual: computed_fingerprint,
            });
        }

        let mut services = [None; SERVICE_MANIFEST_MAX_SERVICES];
        let mut service_present = [false; SERVICE_MANIFEST_MAX_SERVICES];
        let mut resident_nodes = [false; bndr_abi::SHUTDOWN_SERVICE_NODE_COUNT];
        for (index, slot) in services.iter_mut().take(service_count).enumerate() {
            let offset =
                SERVICE_MANIFEST_HEADER_SIZE + index * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
            let record = &wire[offset..offset + SERVICE_MANIFEST_SERVICE_RECORD_SIZE];
            if let Some(relative) = first_nonzero(record, SERVICE_RESERVED_RANGE) {
                return Err(ManifestDecodeError::ReservedNonZero {
                    offset: offset + relative,
                });
            }
            let raw_kind = record[SERVICE_KIND_OFFSET];
            let kind =
                ServiceKind::from_raw(raw_kind).ok_or(ManifestDecodeError::UnknownServiceKind {
                    index,
                    raw: raw_kind,
                })?;
            let kind_index = usize::from(kind.raw() - 1);
            if service_present[kind_index] {
                return Err(ManifestDecodeError::DuplicateService { index, kind });
            }
            let raw_image = record[SERVICE_IMAGE_OFFSET];
            let image = UserImageId::from_raw(u64::from(raw_image)).ok_or(
                ManifestDecodeError::UnknownImage {
                    index,
                    raw: raw_image,
                },
            )?;
            let raw_launch_mode = record[SERVICE_LAUNCH_MODE_OFFSET];
            let launch_mode = ServiceLaunchMode::from_raw(raw_launch_mode).ok_or(
                ManifestDecodeError::UnknownLaunchMode {
                    index,
                    raw: raw_launch_mode,
                },
            )?;
            let raw_node = record[SERVICE_NODE_OFFSET];
            let shutdown_node = if raw_node == NO_SHUTDOWN_NODE {
                None
            } else {
                Some(ShutdownServiceNode::from_raw(u64::from(raw_node)).ok_or(
                    ManifestDecodeError::UnknownShutdownNode {
                        index,
                        raw: raw_node,
                    },
                )?)
            };
            let (expected_image, expected_launch_mode, expected_node) = canonical_binding(kind);
            if image != expected_image
                || launch_mode != expected_launch_mode
                || shutdown_node != expected_node
            {
                return Err(ManifestDecodeError::BindingMismatch {
                    index,
                    kind,
                    image,
                    launch_mode,
                    shutdown_node,
                });
            }
            if let Some(node) = shutdown_node {
                let node_index = node.raw() as usize;
                if resident_nodes[node_index] {
                    return Err(ManifestDecodeError::DuplicateShutdownNode { index, node });
                }
                resident_nodes[node_index] = true;
            }

            let policy = ServicePolicy::new(
                read_u32(record, SERVICE_RESTART_BUDGET_RANGE),
                read_u64(record, SERVICE_RESTART_BACKOFF_RANGE),
                read_u64(record, SERVICE_HEALTH_TIMEOUT_RANGE),
            )
            .and_then(|policy| {
                policy.with_missed_probe_tolerance(read_u32(record, SERVICE_MISSED_TOLERANCE_RANGE))
            })
            .map_err(|source| ManifestDecodeError::Policy { index, source })?;
            *slot = Some(ManifestService {
                kind,
                image,
                launch_mode,
                shutdown_node,
                policy,
            });
            service_present[kind_index] = true;
        }

        let dependency_start =
            SERVICE_MANIFEST_HEADER_SIZE + service_count * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
        let mut dependencies = [None; SERVICE_MANIFEST_MAX_DEPENDENCIES];
        let mut adjacency = [[false; SERVICE_MANIFEST_MAX_SERVICES]; SERVICE_MANIFEST_MAX_SERVICES];
        for (index, slot) in dependencies.iter_mut().take(dependency_count).enumerate() {
            let offset = dependency_start + index * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
            let record = &wire[offset..offset + SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE];
            if let Some(relative) = first_nonzero(record, DEPENDENCY_RESERVED_RANGE) {
                return Err(ManifestDecodeError::ReservedNonZero {
                    offset: offset + relative,
                });
            }
            let raw_prerequisite = record[DEPENDENCY_PREREQUISITE_OFFSET];
            let prerequisite = ServiceKind::from_raw(raw_prerequisite).ok_or(
                ManifestDecodeError::UnknownDependencyService {
                    index,
                    raw: raw_prerequisite,
                },
            )?;
            let raw_dependent = record[DEPENDENCY_DEPENDENT_OFFSET];
            let dependent = ServiceKind::from_raw(raw_dependent).ok_or(
                ManifestDecodeError::UnknownDependencyService {
                    index,
                    raw: raw_dependent,
                },
            )?;
            if prerequisite == dependent {
                return Err(ManifestDecodeError::SelfDependency {
                    index,
                    service: prerequisite,
                });
            }
            let prerequisite_index = usize::from(prerequisite.raw() - 1);
            let dependent_index = usize::from(dependent.raw() - 1);
            if !service_present[prerequisite_index] {
                return Err(ManifestDecodeError::DependencyServiceMissing {
                    index,
                    service: prerequisite,
                });
            }
            if !service_present[dependent_index] {
                return Err(ManifestDecodeError::DependencyServiceMissing {
                    index,
                    service: dependent,
                });
            }
            if adjacency[prerequisite_index][dependent_index] {
                return Err(ManifestDecodeError::DuplicateDependency {
                    index,
                    prerequisite,
                    dependent,
                });
            }
            let raw_kind = record[DEPENDENCY_KIND_OFFSET];
            let kind = match raw_kind {
                1 => DependencyKind::Hard,
                2 => DependencyKind::Soft,
                _ => {
                    return Err(ManifestDecodeError::UnknownDependencyKind {
                        index,
                        raw: raw_kind,
                    });
                }
            };
            adjacency[prerequisite_index][dependent_index] = true;
            *slot = Some(ManifestDependency {
                prerequisite,
                dependent,
                kind,
            });
        }

        let mut reachability = adjacency;
        for intermediate in 0..SERVICE_MANIFEST_MAX_SERVICES {
            for source in 0..SERVICE_MANIFEST_MAX_SERVICES {
                for destination in 0..SERVICE_MANIFEST_MAX_SERVICES {
                    reachability[source][destination] |= reachability[source][intermediate]
                        && reachability[intermediate][destination];
                }
            }
        }
        if let Some(index) =
            (0..SERVICE_MANIFEST_MAX_SERVICES).find(|index| reachability[*index][*index])
        {
            let kind = ServiceKind::from_raw(index as u8 + 1)
                .expect("manifest service-kind domain is contiguous");
            return Err(ManifestDecodeError::DependencyCycle { service: kind });
        }

        Ok(Self {
            generation,
            fingerprint,
            wire_size: wire.len(),
            services,
            service_count,
            dependencies,
            dependency_count,
        })
    }

    pub const fn generation(&self) -> u32 {
        self.generation
    }

    pub const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub const fn wire_size(&self) -> usize {
        self.wire_size
    }

    pub const fn service_count(&self) -> usize {
        self.service_count
    }

    pub const fn dependency_count(&self) -> usize {
        self.dependency_count
    }

    pub fn services(&self) -> impl Iterator<Item = ManifestService> + '_ {
        self.services[..self.service_count]
            .iter()
            .copied()
            .flatten()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = ManifestDependency> + '_ {
        self.dependencies[..self.dependency_count]
            .iter()
            .copied()
            .flatten()
    }

    pub fn service(&self, kind: ServiceKind) -> Option<ManifestService> {
        self.services().find(|service| service.kind == kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestDecodeError {
    TruncatedHeader {
        actual: usize,
    },
    BadMagic,
    UnsupportedVersion(u8),
    HeaderSize(u8),
    ServiceRecordSize(u8),
    DependencyRecordSize(u8),
    ReservedNonZero {
        offset: usize,
    },
    Empty,
    ServiceCapacity {
        actual: usize,
        maximum: usize,
    },
    DependencyCapacity {
        actual: usize,
        maximum: usize,
    },
    Length {
        declared: usize,
        expected: usize,
        actual: usize,
    },
    GenerationMustBeNonZero,
    Fingerprint {
        expected: u64,
        actual: u64,
    },
    UnknownServiceKind {
        index: usize,
        raw: u8,
    },
    UnknownImage {
        index: usize,
        raw: u8,
    },
    UnknownLaunchMode {
        index: usize,
        raw: u8,
    },
    UnknownShutdownNode {
        index: usize,
        raw: u8,
    },
    BindingMismatch {
        index: usize,
        kind: ServiceKind,
        image: UserImageId,
        launch_mode: ServiceLaunchMode,
        shutdown_node: Option<ShutdownServiceNode>,
    },
    DuplicateService {
        index: usize,
        kind: ServiceKind,
    },
    DuplicateShutdownNode {
        index: usize,
        node: ShutdownServiceNode,
    },
    Policy {
        index: usize,
        source: ServicePolicyError,
    },
    UnknownDependencyService {
        index: usize,
        raw: u8,
    },
    DependencyServiceMissing {
        index: usize,
        service: ServiceKind,
    },
    SelfDependency {
        index: usize,
        service: ServiceKind,
    },
    DuplicateDependency {
        index: usize,
        prerequisite: ServiceKind,
        dependent: ServiceKind,
    },
    UnknownDependencyKind {
        index: usize,
        raw: u8,
    },
    DependencyCycle {
        service: ServiceKind,
    },
}

/// Fingerprints only the record payload. Header integrity is covered by the
/// strict field and exact-length validation performed before this comparison.
pub const fn service_manifest_payload_fingerprint(wire: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    let mut index = SERVICE_MANIFEST_HEADER_SIZE;
    while index < wire.len() {
        hash ^= wire[index] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        index += 1;
    }
    hash
}

const fn canonical_binding(
    kind: ServiceKind,
) -> (UserImageId, ServiceLaunchMode, Option<ShutdownServiceNode>) {
    match kind {
        ServiceKind::ServiceManager => (
            UserImageId::ServiceManager,
            ServiceLaunchMode::BindResident,
            Some(ShutdownServiceNode::ServiceManager),
        ),
        ServiceKind::SurfaceServer => (
            UserImageId::SurfaceServer,
            ServiceLaunchMode::BindResident,
            Some(ShutdownServiceNode::SurfaceServer),
        ),
        ServiceKind::InputServer => (
            UserImageId::InputServer,
            ServiceLaunchMode::BindResident,
            Some(ShutdownServiceNode::InputServer),
        ),
        ServiceKind::StorageServer => (UserImageId::StorageServer, ServiceLaunchMode::Spawn, None),
        ServiceKind::App => (
            UserImageId::App,
            ServiceLaunchMode::BindResident,
            Some(ShutdownServiceNode::App),
        ),
    }
}

const fn first_nonzero(wire: &[u8], range: core::ops::Range<usize>) -> Option<usize> {
    let mut index = range.start;
    while index < range.end {
        if wire[index] != 0 {
            return Some(index);
        }
        index += 1;
    }
    None
}

const fn read_u32(wire: &[u8], range: core::ops::Range<usize>) -> u32 {
    u32::from_le_bytes([
        wire[range.start],
        wire[range.start + 1],
        wire[range.start + 2],
        wire[range.start + 3],
    ])
}

const fn read_u64(wire: &[u8], range: core::ops::Range<usize>) -> u64 {
    u64::from_le_bytes([
        wire[range.start],
        wire[range.start + 1],
        wire[range.start + 2],
        wire[range.start + 3],
        wire[range.start + 4],
        wire[range.start + 5],
        wire[range.start + 6],
        wire[range.start + 7],
    ])
}

#[derive(Clone, Copy)]
struct RawService {
    kind: ServiceKind,
    image: UserImageId,
    launch_mode: ServiceLaunchMode,
    shutdown_node: Option<ShutdownServiceNode>,
    restart_budget: u32,
    restart_backoff_ns: u64,
    health_timeout_ns: u64,
    missed_probe_tolerance: u32,
}

#[derive(Clone, Copy)]
struct RawDependency {
    prerequisite: ServiceKind,
    dependent: ServiceKind,
    kind: DependencyKind,
}

const PRODUCT_SERVICES: [RawService; PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT] = [
    RawService {
        kind: ServiceKind::App,
        image: UserImageId::App,
        launch_mode: ServiceLaunchMode::BindResident,
        shutdown_node: Some(ShutdownServiceNode::App),
        restart_budget: 1,
        restart_backoff_ns: 30_000_000,
        health_timeout_ns: 100_000_000,
        missed_probe_tolerance: 1,
    },
    RawService {
        kind: ServiceKind::ServiceManager,
        image: UserImageId::ServiceManager,
        launch_mode: ServiceLaunchMode::BindResident,
        shutdown_node: Some(ShutdownServiceNode::ServiceManager),
        restart_budget: 1,
        restart_backoff_ns: 30_000_000,
        health_timeout_ns: 100_000_000,
        missed_probe_tolerance: 1,
    },
    RawService {
        kind: ServiceKind::StorageServer,
        image: UserImageId::StorageServer,
        launch_mode: ServiceLaunchMode::Spawn,
        shutdown_node: None,
        restart_budget: 1,
        restart_backoff_ns: 30_000_000,
        health_timeout_ns: 100_000_000,
        missed_probe_tolerance: 0,
    },
    RawService {
        kind: ServiceKind::InputServer,
        image: UserImageId::InputServer,
        launch_mode: ServiceLaunchMode::BindResident,
        shutdown_node: Some(ShutdownServiceNode::InputServer),
        restart_budget: 1,
        restart_backoff_ns: 30_000_000,
        health_timeout_ns: 100_000_000,
        missed_probe_tolerance: 1,
    },
    RawService {
        kind: ServiceKind::SurfaceServer,
        image: UserImageId::SurfaceServer,
        launch_mode: ServiceLaunchMode::BindResident,
        shutdown_node: Some(ShutdownServiceNode::SurfaceServer),
        restart_budget: 1,
        restart_backoff_ns: 30_000_000,
        health_timeout_ns: 100_000_000,
        missed_probe_tolerance: 1,
    },
];

const PRODUCT_DEPENDENCIES: [RawDependency; PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT] = [
    RawDependency {
        prerequisite: ServiceKind::StorageServer,
        dependent: ServiceKind::App,
        kind: DependencyKind::Hard,
    },
    RawDependency {
        prerequisite: ServiceKind::InputServer,
        dependent: ServiceKind::App,
        kind: DependencyKind::Soft,
    },
    RawDependency {
        prerequisite: ServiceKind::SurfaceServer,
        dependent: ServiceKind::InputServer,
        kind: DependencyKind::Hard,
    },
    RawDependency {
        prerequisite: ServiceKind::ServiceManager,
        dependent: ServiceKind::SurfaceServer,
        kind: DependencyKind::Hard,
    },
];

const fn encode_product_service_manifest(generation: u32) -> [u8; PRODUCT_SERVICE_MANIFEST_SIZE] {
    let mut wire = [0_u8; PRODUCT_SERVICE_MANIFEST_SIZE];
    wire[0] = SERVICE_MANIFEST_MAGIC[0];
    wire[1] = SERVICE_MANIFEST_MAGIC[1];
    wire[2] = SERVICE_MANIFEST_MAGIC[2];
    wire[3] = SERVICE_MANIFEST_MAGIC[3];
    wire[VERSION_OFFSET] = SERVICE_MANIFEST_VERSION;
    wire[HEADER_SIZE_OFFSET] = SERVICE_MANIFEST_HEADER_SIZE as u8;
    wire[SERVICE_RECORD_SIZE_OFFSET] = SERVICE_MANIFEST_SERVICE_RECORD_SIZE as u8;
    wire[DEPENDENCY_RECORD_SIZE_OFFSET] = SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE as u8;
    write_u32(
        &mut wire,
        TOTAL_SIZE_RANGE.start,
        PRODUCT_SERVICE_MANIFEST_SIZE as u32,
    );
    write_u32(&mut wire, GENERATION_RANGE.start, generation);
    wire[SERVICE_COUNT_OFFSET] = PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT as u8;
    wire[DEPENDENCY_COUNT_OFFSET] = PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT as u8;

    let mut service_index = 0;
    while service_index < PRODUCT_SERVICES.len() {
        let service = PRODUCT_SERVICES[service_index];
        let offset =
            SERVICE_MANIFEST_HEADER_SIZE + service_index * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
        wire[offset + SERVICE_KIND_OFFSET] = service.kind.raw();
        wire[offset + SERVICE_IMAGE_OFFSET] = service.image.raw() as u8;
        wire[offset + SERVICE_LAUNCH_MODE_OFFSET] = service.launch_mode.raw();
        wire[offset + SERVICE_NODE_OFFSET] = match service.shutdown_node {
            Some(node) => node.raw() as u8,
            None => NO_SHUTDOWN_NODE,
        };
        write_u32(
            &mut wire,
            offset + SERVICE_RESTART_BUDGET_RANGE.start,
            service.restart_budget,
        );
        write_u64(
            &mut wire,
            offset + SERVICE_RESTART_BACKOFF_RANGE.start,
            service.restart_backoff_ns,
        );
        write_u64(
            &mut wire,
            offset + SERVICE_HEALTH_TIMEOUT_RANGE.start,
            service.health_timeout_ns,
        );
        write_u32(
            &mut wire,
            offset + SERVICE_MISSED_TOLERANCE_RANGE.start,
            service.missed_probe_tolerance,
        );
        service_index += 1;
    }

    let dependency_start = SERVICE_MANIFEST_HEADER_SIZE
        + PRODUCT_SERVICES.len() * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
    let mut dependency_index = 0;
    while dependency_index < PRODUCT_DEPENDENCIES.len() {
        let dependency = PRODUCT_DEPENDENCIES[dependency_index];
        let offset = dependency_start + dependency_index * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
        wire[offset + DEPENDENCY_PREREQUISITE_OFFSET] = dependency.prerequisite.raw();
        wire[offset + DEPENDENCY_DEPENDENT_OFFSET] = dependency.dependent.raw();
        wire[offset + DEPENDENCY_KIND_OFFSET] = match dependency.kind {
            DependencyKind::Hard => 1,
            DependencyKind::Soft => 2,
        };
        dependency_index += 1;
    }

    let fingerprint = service_manifest_payload_fingerprint(&wire);
    write_u64(&mut wire, FINGERPRINT_RANGE.start, fingerprint);
    wire
}

const fn write_u32(wire: &mut [u8], offset: usize, value: u32) {
    let bytes = value.to_le_bytes();
    wire[offset] = bytes[0];
    wire[offset + 1] = bytes[1];
    wire[offset + 2] = bytes[2];
    wire[offset + 3] = bytes[3];
}

const fn write_u64(wire: &mut [u8], offset: usize, value: u64) {
    let bytes = value.to_le_bytes();
    wire[offset] = bytes[0];
    wire[offset + 1] = bytes[1];
    wire[offset + 2] = bytes[2];
    wire[offset + 3] = bytes[3];
    wire[offset + 4] = bytes[4];
    wire[offset + 5] = bytes[5];
    wire[offset + 6] = bytes[6];
    wire[offset + 7] = bytes[7];
}

#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use super::{
        DEPENDENCY_COUNT_OFFSET, DEPENDENCY_DEPENDENT_OFFSET, DEPENDENCY_KIND_OFFSET,
        DEPENDENCY_PREREQUISITE_OFFSET, FINGERPRINT_RANGE, GENERATION_RANGE, HEADER_RESERVED_RANGE,
        NO_SHUTDOWN_NODE, PRODUCT_DEPENDENCIES, PRODUCT_SERVICE_MANIFEST_BYTES,
        PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT, PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT,
        PRODUCT_SERVICE_MANIFEST_SIZE, PRODUCT_SERVICES, RawDependency, RawService,
        SERVICE_COUNT_OFFSET, SERVICE_HEALTH_TIMEOUT_RANGE, SERVICE_IMAGE_OFFSET,
        SERVICE_KIND_OFFSET, SERVICE_LAUNCH_MODE_OFFSET, SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE,
        SERVICE_MANIFEST_GENERATION, SERVICE_MANIFEST_HEADER_SIZE, SERVICE_MANIFEST_MAGIC,
        SERVICE_MANIFEST_MAX_DEPENDENCIES, SERVICE_MANIFEST_MAX_SERVICES,
        SERVICE_MANIFEST_SERVICE_RECORD_SIZE, SERVICE_MANIFEST_VERSION,
        SERVICE_MISSED_TOLERANCE_RANGE, SERVICE_NODE_OFFSET, SERVICE_RESERVED_RANGE,
        SERVICE_RESTART_BUDGET_RANGE, ServiceLaunchMode, ServiceManifest, TOTAL_SIZE_RANGE,
        VERIFIED_PRODUCT_SERVICE_MANIFEST_BYTES, VERIFIED_PRODUCT_SERVICE_MANIFEST_FINGERPRINT,
        VERIFIED_SERVICE_MANIFEST_GENERATION, VERSION_OFFSET, service_manifest_payload_fingerprint,
        write_u32, write_u64,
    };
    use crate::health::{DependencyKind, ServiceKind, ServicePolicyError};
    use bndr_abi::{ShutdownServiceNode, UserImageId};

    const MAX_TEST_WIRE_SIZE: usize = SERVICE_MANIFEST_HEADER_SIZE
        + SERVICE_MANIFEST_MAX_SERVICES * SERVICE_MANIFEST_SERVICE_RECORD_SIZE
        + SERVICE_MANIFEST_MAX_DEPENDENCIES * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;

    #[test]
    fn product_manifest_is_strict_and_uses_nonlegacy_service_order() {
        let manifest = ServiceManifest::decode(&PRODUCT_SERVICE_MANIFEST_BYTES).unwrap();
        assert_eq!(manifest.generation(), SERVICE_MANIFEST_GENERATION);
        assert_eq!(manifest.wire_size(), PRODUCT_SERVICE_MANIFEST_SIZE);
        assert_eq!(
            manifest.service_count(),
            PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
        );
        assert_eq!(
            manifest.dependency_count(),
            PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT
        );
        assert_eq!(
            manifest
                .services()
                .map(|service| service.kind())
                .collect::<Vec<_>>(),
            vec![
                ServiceKind::App,
                ServiceKind::ServiceManager,
                ServiceKind::StorageServer,
                ServiceKind::InputServer,
                ServiceKind::SurfaceServer,
            ]
        );
        let storage = manifest.service(ServiceKind::StorageServer).unwrap();
        assert_eq!(storage.image(), UserImageId::StorageServer);
        assert_eq!(storage.launch_mode(), ServiceLaunchMode::Spawn);
        assert_eq!(storage.shutdown_node(), None);
        assert_eq!(storage.policy().missed_probe_tolerance(), 0);
        assert_eq!(
            manifest
                .service(ServiceKind::SurfaceServer)
                .unwrap()
                .policy()
                .missed_probe_tolerance(),
            1
        );
        assert_eq!(
            manifest
                .dependencies()
                .map(|dependency| (
                    dependency.prerequisite(),
                    dependency.dependent(),
                    dependency.kind(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    ServiceKind::StorageServer,
                    ServiceKind::App,
                    DependencyKind::Hard,
                ),
                (
                    ServiceKind::InputServer,
                    ServiceKind::App,
                    DependencyKind::Soft,
                ),
                (
                    ServiceKind::SurfaceServer,
                    ServiceKind::InputServer,
                    DependencyKind::Hard,
                ),
                (
                    ServiceKind::ServiceManager,
                    ServiceKind::SurfaceServer,
                    DependencyKind::Hard,
                ),
            ]
        );
    }

    #[test]
    fn valid_bounded_subset_and_permutation_decode_without_fixed_product_array() {
        let services = [PRODUCT_SERVICES[4], PRODUCT_SERVICES[1]];
        let dependencies = [RawDependency {
            prerequisite: ServiceKind::ServiceManager,
            dependent: ServiceKind::SurfaceServer,
            kind: DependencyKind::Hard,
        }];
        let (wire, length) = encode_for_test(&services, &dependencies);
        let manifest = ServiceManifest::decode(&wire[..length]).unwrap();
        assert_eq!(manifest.service_count(), 2);
        assert_eq!(manifest.dependency_count(), 1);
        assert_eq!(
            manifest
                .services()
                .map(|service| service.kind())
                .collect::<Vec<_>>(),
            vec![ServiceKind::SurfaceServer, ServiceKind::ServiceManager]
        );
    }

    #[test]
    fn manifest_decode_rejects_header_and_fingerprint_mutations_transactionally() {
        let cases: &[(usize, u8)] = &[
            (0, b'X'),
            (VERSION_OFFSET, SERVICE_MANIFEST_VERSION + 1),
            (5, 0),
            (6, 0),
            (7, 0),
            (HEADER_RESERVED_RANGE.start, 1),
            (
                FINGERPRINT_RANGE.start,
                PRODUCT_SERVICE_MANIFEST_BYTES[FINGERPRINT_RANGE.start] ^ 1,
            ),
        ];
        for (offset, value) in cases.iter().copied() {
            let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
            wire[offset] = value;
            assert!(ServiceManifest::decode(&wire).is_err(), "offset {offset}");
        }
        assert!(ServiceManifest::decode(&PRODUCT_SERVICE_MANIFEST_BYTES[..31]).is_err());

        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        write_u32(&mut wire, TOTAL_SIZE_RANGE.start, 0);
        assert!(ServiceManifest::decode(&wire).is_err());
        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        write_u32(&mut wire, GENERATION_RANGE.start, 0);
        assert!(ServiceManifest::decode(&wire).is_err());
        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        wire[SERVICE_COUNT_OFFSET] = 0;
        assert!(ServiceManifest::decode(&wire).is_err());
        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        wire[SERVICE_COUNT_OFFSET] = SERVICE_MANIFEST_MAX_SERVICES as u8 + 1;
        assert!(ServiceManifest::decode(&wire).is_err());
        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        wire[DEPENDENCY_COUNT_OFFSET] = SERVICE_MANIFEST_MAX_DEPENDENCIES as u8 + 1;
        assert!(ServiceManifest::decode(&wire).is_err());
    }

    #[test]
    fn manifest_decode_rejects_service_identity_policy_and_padding_mutations() {
        let service_offset = SERVICE_MANIFEST_HEADER_SIZE;
        for (relative, value) in [
            (SERVICE_KIND_OFFSET, 0),
            (SERVICE_IMAGE_OFFSET, 0),
            (SERVICE_LAUNCH_MODE_OFFSET, 0),
            (SERVICE_NODE_OFFSET, NO_SHUTDOWN_NODE),
            (SERVICE_RESTART_BUDGET_RANGE.start, 0),
            (SERVICE_MISSED_TOLERANCE_RANGE.start, 9),
            (SERVICE_RESERVED_RANGE.start, 1),
        ] {
            let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
            wire[service_offset + relative] = value;
            reseal(&mut wire);
            assert!(
                ServiceManifest::decode(&wire).is_err(),
                "relative service offset {relative}"
            );
        }
        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        write_u64(
            &mut wire,
            service_offset + SERVICE_HEALTH_TIMEOUT_RANGE.start,
            0,
        );
        reseal(&mut wire);
        assert!(ServiceManifest::decode(&wire).is_err());

        let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
        let second = SERVICE_MANIFEST_HEADER_SIZE + SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
        wire[second + SERVICE_KIND_OFFSET] = ServiceKind::App.raw();
        wire[second + SERVICE_IMAGE_OFFSET] = UserImageId::App.raw() as u8;
        wire[second + SERVICE_LAUNCH_MODE_OFFSET] = ServiceLaunchMode::BindResident.raw();
        wire[second + SERVICE_NODE_OFFSET] = ShutdownServiceNode::App.raw() as u8;
        reseal(&mut wire);
        assert!(ServiceManifest::decode(&wire).is_err());
    }

    #[test]
    fn manifest_decode_rejects_dependency_mutations_and_cycles() {
        let dependency_start = SERVICE_MANIFEST_HEADER_SIZE
            + PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
        for (relative, value) in [
            (DEPENDENCY_PREREQUISITE_OFFSET, 0),
            (DEPENDENCY_DEPENDENT_OFFSET, 0),
            (DEPENDENCY_KIND_OFFSET, 0),
            (3, 1),
        ] {
            let mut wire = PRODUCT_SERVICE_MANIFEST_BYTES;
            wire[dependency_start + relative] = value;
            reseal(&mut wire);
            assert!(
                ServiceManifest::decode(&wire).is_err(),
                "relative dependency offset {relative}"
            );
        }

        let mut duplicate = PRODUCT_SERVICE_MANIFEST_BYTES;
        let second = dependency_start + SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
        duplicate[second..second + SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE].copy_from_slice(
            &PRODUCT_SERVICE_MANIFEST_BYTES
                [dependency_start..dependency_start + SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE],
        );
        reseal(&mut duplicate);
        assert!(ServiceManifest::decode(&duplicate).is_err());

        let services = [PRODUCT_SERVICES[1], PRODUCT_SERVICES[4]];
        let cycle = [
            RawDependency {
                prerequisite: ServiceKind::ServiceManager,
                dependent: ServiceKind::SurfaceServer,
                kind: DependencyKind::Hard,
            },
            RawDependency {
                prerequisite: ServiceKind::SurfaceServer,
                dependent: ServiceKind::ServiceManager,
                kind: DependencyKind::Hard,
            },
        ];
        let (wire, length) = encode_for_test(&services, &cycle);
        assert!(ServiceManifest::decode(&wire[..length]).is_err());

        let missing_services = [PRODUCT_SERVICES[1]];
        let (wire, length) = encode_for_test(&missing_services, &cycle[..1]);
        assert!(ServiceManifest::decode(&wire[..length]).is_err());
    }

    #[test]
    fn product_manifest_constant_matches_wire_contract() {
        assert_eq!(
            &PRODUCT_SERVICE_MANIFEST_BYTES[..4],
            &SERVICE_MANIFEST_MAGIC
        );
        assert_eq!(
            PRODUCT_SERVICE_MANIFEST_BYTES[VERSION_OFFSET],
            SERVICE_MANIFEST_VERSION
        );
        assert_eq!(
            service_manifest_payload_fingerprint(&PRODUCT_SERVICE_MANIFEST_BYTES),
            u64::from_le_bytes(
                PRODUCT_SERVICE_MANIFEST_BYTES[FINGERPRINT_RANGE]
                    .try_into()
                    .unwrap()
            )
        );
        assert_eq!(PRODUCT_DEPENDENCIES.len(), 4);
        assert_eq!(PRODUCT_SERVICES.len(), 5);
        assert_eq!(
            ServicePolicyError::MissedProbeToleranceTooLarge,
            ServicePolicyError::MissedProbeToleranceTooLarge
        );
        let verified = ServiceManifest::decode(&VERIFIED_PRODUCT_SERVICE_MANIFEST_BYTES).unwrap();
        assert_eq!(verified.generation(), VERIFIED_SERVICE_MANIFEST_GENERATION);
        assert_eq!(
            verified.fingerprint(),
            VERIFIED_PRODUCT_SERVICE_MANIFEST_FINGERPRINT
        );
        assert_eq!(
            verified.services().collect::<Vec<_>>(),
            ServiceManifest::decode(&PRODUCT_SERVICE_MANIFEST_BYTES)
                .unwrap()
                .services()
                .collect::<Vec<_>>()
        );
    }

    fn encode_for_test(
        services: &[RawService],
        dependencies: &[RawDependency],
    ) -> ([u8; MAX_TEST_WIRE_SIZE], usize) {
        assert!(!services.is_empty());
        assert!(services.len() <= SERVICE_MANIFEST_MAX_SERVICES);
        assert!(dependencies.len() <= SERVICE_MANIFEST_MAX_DEPENDENCIES);
        let length = SERVICE_MANIFEST_HEADER_SIZE
            + services.len() * SERVICE_MANIFEST_SERVICE_RECORD_SIZE
            + dependencies.len() * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
        let mut wire = [0_u8; MAX_TEST_WIRE_SIZE];
        wire[..4].copy_from_slice(&SERVICE_MANIFEST_MAGIC);
        wire[VERSION_OFFSET] = SERVICE_MANIFEST_VERSION;
        wire[5] = SERVICE_MANIFEST_HEADER_SIZE as u8;
        wire[6] = SERVICE_MANIFEST_SERVICE_RECORD_SIZE as u8;
        wire[7] = SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE as u8;
        write_u32(&mut wire, TOTAL_SIZE_RANGE.start, length as u32);
        write_u32(
            &mut wire,
            GENERATION_RANGE.start,
            SERVICE_MANIFEST_GENERATION,
        );
        wire[SERVICE_COUNT_OFFSET] = services.len() as u8;
        wire[DEPENDENCY_COUNT_OFFSET] = dependencies.len() as u8;
        for (index, service) in services.iter().copied().enumerate() {
            let offset =
                SERVICE_MANIFEST_HEADER_SIZE + index * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
            wire[offset + SERVICE_KIND_OFFSET] = service.kind.raw();
            wire[offset + SERVICE_IMAGE_OFFSET] = service.image.raw() as u8;
            wire[offset + SERVICE_LAUNCH_MODE_OFFSET] = service.launch_mode.raw();
            wire[offset + SERVICE_NODE_OFFSET] = service
                .shutdown_node
                .map_or(NO_SHUTDOWN_NODE, |node| node.raw() as u8);
            write_u32(
                &mut wire,
                offset + SERVICE_RESTART_BUDGET_RANGE.start,
                service.restart_budget,
            );
            write_u64(&mut wire, offset + 8, service.restart_backoff_ns);
            write_u64(
                &mut wire,
                offset + SERVICE_HEALTH_TIMEOUT_RANGE.start,
                service.health_timeout_ns,
            );
            write_u32(
                &mut wire,
                offset + SERVICE_MISSED_TOLERANCE_RANGE.start,
                service.missed_probe_tolerance,
            );
        }
        let dependency_start =
            SERVICE_MANIFEST_HEADER_SIZE + services.len() * SERVICE_MANIFEST_SERVICE_RECORD_SIZE;
        for (index, dependency) in dependencies.iter().copied().enumerate() {
            let offset = dependency_start + index * SERVICE_MANIFEST_DEPENDENCY_RECORD_SIZE;
            wire[offset + DEPENDENCY_PREREQUISITE_OFFSET] = dependency.prerequisite.raw();
            wire[offset + DEPENDENCY_DEPENDENT_OFFSET] = dependency.dependent.raw();
            wire[offset + DEPENDENCY_KIND_OFFSET] = match dependency.kind {
                DependencyKind::Hard => 1,
                DependencyKind::Soft => 2,
            };
        }
        let fingerprint = service_manifest_payload_fingerprint(&wire[..length]);
        write_u64(&mut wire, FINGERPRINT_RANGE.start, fingerprint);
        (wire, length)
    }

    fn reseal(wire: &mut [u8]) {
        let fingerprint = service_manifest_payload_fingerprint(wire);
        write_u64(wire, FINGERPRINT_RANGE.start, fingerprint);
    }
}
