//! Signed, bounded maintenance-plan program for the product supervisor.
//!
//! `BMP1` uses a trust anchor separate from both the verified product manifest
//! and the one-use `BMA1` authorization.  Its 256-byte signed region binds the
//! exact authorization, device namespace, legacy M80 plan identity, and a
//! fixed-capacity operation table.  The decoder is allocation-free and
//! intentionally verifies the RSA signature before interpreting signed
//! bindings or program semantics.

use crate::verified_manifest::{Rsa2048PublicKey, rsa2048_pkcs1_v15_sha256_verify, sha256};

pub const MAINTENANCE_PLAN_MAGIC: [u8; 4] = *b"BMP1";
pub const MAINTENANCE_PLAN_VERSION: u8 = 1;
pub const MAINTENANCE_PLAN_ALGORITHM_RSA2048_PKCS1_V15_SHA256: u8 = 1;
pub const MAINTENANCE_PLAN_KEY_ID: u8 = 1;
pub const MAINTENANCE_PLAN_HEADER_SIZE: usize = 64;
pub const MAINTENANCE_PLAN_PAYLOAD_SIZE: usize = 192;
pub const MAINTENANCE_PLAN_SIGNED_SIZE: usize =
    MAINTENANCE_PLAN_HEADER_SIZE + MAINTENANCE_PLAN_PAYLOAD_SIZE;
pub const MAINTENANCE_PLAN_SIGNATURE_SIZE: usize = 256;
pub const MAINTENANCE_PLAN_SIZE: usize =
    MAINTENANCE_PLAN_SIGNED_SIZE + MAINTENANCE_PLAN_SIGNATURE_SIZE;
pub const MAINTENANCE_PLAN_OPERATION_COUNT: u32 = 3;
pub const MAINTENANCE_PLAN_TRANSITION_COUNT: u32 = 9;
pub const MAINTENANCE_PLAN_PROGRAM_VERSION: u32 = 1;
pub const MAINTENANCE_PLAN_DESCRIPTOR_SIZE: u32 = 16;
pub const MAINTENANCE_PLAN_DESCRIPTOR_CAPACITY: usize = 4;

pub const MAINTENANCE_PLAN_OPERATION_STORAGE_ROTATION: u32 = 1;
pub const MAINTENANCE_PLAN_OPERATION_RESIDENT_DRAIN: u32 = 2;
pub const MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT: u32 = 1 << 0;
pub const MAINTENANCE_PLAN_OPERATION_FLAG_PREAPPLY_CANCELLABLE: u32 = 1 << 1;
pub const MAINTENANCE_PLAN_SUPPORTED_OPERATION_FLAGS: u32 =
    MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT
        | MAINTENANCE_PLAN_OPERATION_FLAG_PREAPPLY_CANCELLABLE;

pub const MAINTENANCE_PLAN_ROOT_SHA256: [u8; 32] = [
    0x30, 0xf1, 0x39, 0xbf, 0x41, 0xdf, 0x65, 0x93, 0xa0, 0xdb, 0x7c, 0x80, 0xbe, 0x1b, 0x80, 0x6f,
    0x56, 0x95, 0x49, 0x77, 0xc9, 0x22, 0x76, 0x76, 0x5e, 0x41, 0x56, 0x68, 0x5b, 0x10, 0xa2, 0x4e,
];

const MAINTENANCE_PLAN_RSA2048_MODULUS_HEX: &[u8; 512] =
    b"cffa1681835bdb06cf401321a1d0751ccaad983e4bb353517082f45d831d6a7fe0b4f382a7251f671da10a9716ac22eca9898d9c942c484c240202a073044e2cbc5c0c54a24d529f8e2bdc43b3f06befe2ab5bddf72dbad2a6992be8dfd06900ba6b3516609d34411ecc666849c149320a464e5f42749fa84224fa26e8bf9538b9c38ff78223a36859de68d82933d4c75ba36e7417ea736ee66ac41a4a11b5db8907da73d00ca703caa2901e0f68e28a74a606c6037f757fb754f50e1e0f15c161653de36bcd1ee8bc2282978f08a69524d1c4b1fad7352d27456c9ed070581c03b34c8c0264d5d0722c30201c3c17dcb35a1e774c374788340d303cd0479f39";

pub const MAINTENANCE_PLAN_RSA2048_KEY1: Rsa2048PublicKey = Rsa2048PublicKey::new(
    MAINTENANCE_PLAN_KEY_ID,
    decode_modulus_hex(MAINTENANCE_PLAN_RSA2048_MODULUS_HEX),
);

const VERSION_OFFSET: usize = 4;
const ALGORITHM_OFFSET: usize = 5;
const KEY_ID_OFFSET: usize = 6;
const FLAGS_OFFSET: usize = 7;
const TOTAL_SIZE_OFFSET: usize = 8;
const SIGNED_SIZE_OFFSET: usize = 12;
const PAYLOAD_OFFSET_OFFSET: usize = 16;
const PAYLOAD_SIZE_OFFSET: usize = 20;
const AUTHORIZATION_SEQUENCE_OFFSET: usize = 24;
const OPERATION_COUNT_OFFSET: usize = 32;
const TRANSITION_COUNT_OFFSET: usize = 36;
const PROGRAM_VERSION_OFFSET: usize = 40;
const DESCRIPTOR_SIZE_OFFSET: usize = 44;
const PLAN_GENERATION_OFFSET: usize = 48;
const HEADER_RESERVED_OFFSET: usize = 56;
const AUTHORIZATION_DIGEST_OFFSET: usize = 64;
const AUTHORIZATION_ID_OFFSET: usize = 96;
const DEVICE_BINDING_OFFSET: usize = 128;
const PLAN_ID_OFFSET: usize = 160;
const DESCRIPTORS_OFFSET: usize = 192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanBinding {
    pub authorization_sequence: u64,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub plan_id_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanOperation {
    kind: u32,
    flags: u32,
    argument: u32,
    token: u32,
}

impl MaintenancePlanOperation {
    pub const fn kind(&self) -> u32 {
        self.kind
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn argument(&self) -> u32 {
        self.argument
    }

    pub const fn token(&self) -> u32 {
        self.token
    }

    pub const fn is_idempotent(&self) -> bool {
        self.flags & MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT != 0
    }

    pub const fn is_preapply_cancellable(&self) -> bool {
        self.flags & MAINTENANCE_PLAN_OPERATION_FLAG_PREAPPLY_CANCELLABLE != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedMaintenancePlan {
    authorization_sequence: u64,
    operation_count: u32,
    transition_count: u32,
    program_version: u32,
    plan_generation: u64,
    authorization_sha256: [u8; 32],
    authorization_id: [u8; 32],
    device_binding_sha256: [u8; 32],
    plan_id_sha256: [u8; 32],
    descriptor_sha256: [u8; 32],
    signed_region_sha256: [u8; 32],
    operations: [MaintenancePlanOperation; MAINTENANCE_PLAN_OPERATION_COUNT as usize],
}

impl VerifiedMaintenancePlan {
    pub const fn authorization_sequence(&self) -> u64 {
        self.authorization_sequence
    }

    pub const fn operation_count(&self) -> u32 {
        self.operation_count
    }

    pub const fn transition_count(&self) -> u32 {
        self.transition_count
    }

    pub const fn program_version(&self) -> u32 {
        self.program_version
    }

    pub const fn plan_generation(&self) -> u64 {
        self.plan_generation
    }

    pub const fn authorization_sha256(&self) -> &[u8; 32] {
        &self.authorization_sha256
    }

    pub const fn authorization_id(&self) -> &[u8; 32] {
        &self.authorization_id
    }

    pub const fn device_binding_sha256(&self) -> &[u8; 32] {
        &self.device_binding_sha256
    }

    pub const fn plan_id_sha256(&self) -> &[u8; 32] {
        &self.plan_id_sha256
    }

    pub const fn descriptor_sha256(&self) -> &[u8; 32] {
        &self.descriptor_sha256
    }

    pub const fn signed_region_sha256(&self) -> &[u8; 32] {
        &self.signed_region_sha256
    }

    pub const fn operation(&self, ordinal: u32) -> Option<&MaintenancePlanOperation> {
        if ordinal == 0 || ordinal > self.operation_count {
            return None;
        }
        Some(&self.operations[ordinal as usize - 1])
    }

    pub const fn preapply_cancel_mask(&self) -> u32 {
        let mut mask = 0_u32;
        let mut index = 0;
        while index < self.operations.len() {
            if self.operations[index].is_preapply_cancellable() {
                mask |= 1 << index;
            }
            index += 1;
        }
        mask
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenancePlanError {
    Length { actual: usize },
    BadMagic,
    UnsupportedVersion(u8),
    UnsupportedAlgorithm(u8),
    UnknownKey { actual: u8, expected: u8 },
    NonZeroFlags(u8),
    Layout,
    NonZeroReserved,
    Signature,
    ZeroAuthorizationSequence,
    InvalidOperationCount(u32),
    InvalidTransitionCount(u32),
    InvalidProgramVersion(u32),
    InvalidPlanGeneration(u64),
    Binding,
    PlanId,
    InvalidProgram,
}

/// Verifies one complete BMP1 artifact before any persistent state mutation.
///
/// Only the fixed envelope and reserved-byte rules run before RSA
/// verification. Signed bindings and operation descriptors are interpreted
/// only after the signature succeeds.
pub fn verify_maintenance_plan(
    artifact: &[u8],
    key: &Rsa2048PublicKey,
    expected: MaintenancePlanBinding,
) -> Result<VerifiedMaintenancePlan, MaintenancePlanError> {
    if artifact.len() != MAINTENANCE_PLAN_SIZE {
        return Err(MaintenancePlanError::Length {
            actual: artifact.len(),
        });
    }
    if artifact[..4] != MAINTENANCE_PLAN_MAGIC {
        return Err(MaintenancePlanError::BadMagic);
    }
    if artifact[VERSION_OFFSET] != MAINTENANCE_PLAN_VERSION {
        return Err(MaintenancePlanError::UnsupportedVersion(
            artifact[VERSION_OFFSET],
        ));
    }
    if artifact[ALGORITHM_OFFSET] != MAINTENANCE_PLAN_ALGORITHM_RSA2048_PKCS1_V15_SHA256 {
        return Err(MaintenancePlanError::UnsupportedAlgorithm(
            artifact[ALGORITHM_OFFSET],
        ));
    }
    if artifact[KEY_ID_OFFSET] != key.key_id() {
        return Err(MaintenancePlanError::UnknownKey {
            actual: artifact[KEY_ID_OFFSET],
            expected: key.key_id(),
        });
    }
    if artifact[FLAGS_OFFSET] != 0 {
        return Err(MaintenancePlanError::NonZeroFlags(artifact[FLAGS_OFFSET]));
    }
    if read_u32(artifact, TOTAL_SIZE_OFFSET) as usize != MAINTENANCE_PLAN_SIZE
        || read_u32(artifact, SIGNED_SIZE_OFFSET) as usize != MAINTENANCE_PLAN_SIGNED_SIZE
        || read_u32(artifact, PAYLOAD_OFFSET_OFFSET) as usize != MAINTENANCE_PLAN_HEADER_SIZE
        || read_u32(artifact, PAYLOAD_SIZE_OFFSET) as usize != MAINTENANCE_PLAN_PAYLOAD_SIZE
        || read_u32(artifact, DESCRIPTOR_SIZE_OFFSET) != MAINTENANCE_PLAN_DESCRIPTOR_SIZE
    {
        return Err(MaintenancePlanError::Layout);
    }
    if artifact[HEADER_RESERVED_OFFSET..MAINTENANCE_PLAN_HEADER_SIZE]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(MaintenancePlanError::NonZeroReserved);
    }

    let signed_region = &artifact[..MAINTENANCE_PLAN_SIGNED_SIZE];
    let signed_region_sha256 = sha256(signed_region);
    if !rsa2048_pkcs1_v15_sha256_verify(
        &artifact[MAINTENANCE_PLAN_SIGNED_SIZE..],
        &signed_region_sha256,
        key.modulus(),
    ) {
        return Err(MaintenancePlanError::Signature);
    }

    let authorization_sequence = read_u64(artifact, AUTHORIZATION_SEQUENCE_OFFSET);
    if authorization_sequence == 0 {
        return Err(MaintenancePlanError::ZeroAuthorizationSequence);
    }
    let operation_count = read_u32(artifact, OPERATION_COUNT_OFFSET);
    if operation_count != MAINTENANCE_PLAN_OPERATION_COUNT {
        return Err(MaintenancePlanError::InvalidOperationCount(operation_count));
    }
    let transition_count = read_u32(artifact, TRANSITION_COUNT_OFFSET);
    if transition_count != MAINTENANCE_PLAN_TRANSITION_COUNT {
        return Err(MaintenancePlanError::InvalidTransitionCount(
            transition_count,
        ));
    }
    let program_version = read_u32(artifact, PROGRAM_VERSION_OFFSET);
    if program_version != MAINTENANCE_PLAN_PROGRAM_VERSION {
        return Err(MaintenancePlanError::InvalidProgramVersion(program_version));
    }
    let plan_generation = read_u64(artifact, PLAN_GENERATION_OFFSET);
    if plan_generation != authorization_sequence {
        return Err(MaintenancePlanError::InvalidPlanGeneration(plan_generation));
    }

    if authorization_sequence != expected.authorization_sequence
        || artifact[AUTHORIZATION_DIGEST_OFFSET..AUTHORIZATION_ID_OFFSET]
            != expected.authorization_sha256
        || artifact[AUTHORIZATION_ID_OFFSET..DEVICE_BINDING_OFFSET] != expected.authorization_id
        || artifact[DEVICE_BINDING_OFFSET..PLAN_ID_OFFSET] != expected.device_binding_sha256
    {
        return Err(MaintenancePlanError::Binding);
    }
    if artifact[PLAN_ID_OFFSET..DESCRIPTORS_OFFSET] != expected.plan_id_sha256 {
        return Err(MaintenancePlanError::PlanId);
    }

    let descriptor_bytes = &artifact[DESCRIPTORS_OFFSET..MAINTENANCE_PLAN_SIGNED_SIZE];
    let operations = [
        decode_operation(&descriptor_bytes[0..16]),
        decode_operation(&descriptor_bytes[16..32]),
        decode_operation(&descriptor_bytes[32..48]),
    ];
    if descriptor_bytes[48..64].iter().any(|byte| *byte != 0) || !valid_product_program(operations)
    {
        return Err(MaintenancePlanError::InvalidProgram);
    }

    Ok(VerifiedMaintenancePlan {
        authorization_sequence,
        operation_count,
        transition_count,
        program_version,
        plan_generation,
        authorization_sha256: expected.authorization_sha256,
        authorization_id: expected.authorization_id,
        device_binding_sha256: expected.device_binding_sha256,
        plan_id_sha256: expected.plan_id_sha256,
        descriptor_sha256: sha256(descriptor_bytes),
        signed_region_sha256,
        operations,
    })
}

const fn valid_product_program(
    operations: [MaintenancePlanOperation; MAINTENANCE_PLAN_OPERATION_COUNT as usize],
) -> bool {
    let first = operations[0];
    let second = operations[1];
    let third = operations[2];
    valid_operation_envelope(first)
        && valid_operation_envelope(second)
        && valid_operation_envelope(third)
        && first.kind == MAINTENANCE_PLAN_OPERATION_STORAGE_ROTATION
        && first.flags
            == MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT
                | MAINTENANCE_PLAN_OPERATION_FLAG_PREAPPLY_CANCELLABLE
        && first.argument == 1
        && second.kind == MAINTENANCE_PLAN_OPERATION_STORAGE_ROTATION
        && second.flags == MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT
        && second.argument == 2
        && third.kind == MAINTENANCE_PLAN_OPERATION_RESIDENT_DRAIN
        && third.flags == MAINTENANCE_PLAN_OPERATION_FLAG_IDEMPOTENT
        && third.argument == 2
        && first.token != second.token
        && first.token != third.token
        && second.token != third.token
}

const fn valid_operation_envelope(operation: MaintenancePlanOperation) -> bool {
    operation.token != 0
        && operation.flags & !MAINTENANCE_PLAN_SUPPORTED_OPERATION_FLAGS == 0
        && operation.is_idempotent()
}

fn decode_operation(bytes: &[u8]) -> MaintenancePlanOperation {
    MaintenancePlanOperation {
        kind: read_u32(bytes, 0),
        flags: read_u32(bytes, 4),
        argument: read_u32(bytes, 8),
        token: read_u32(bytes, 12),
    }
}

const fn decode_modulus_hex(hex: &[u8; 512]) -> [u8; 256] {
    let mut decoded = [0_u8; 256];
    let mut index = 0;
    while index < decoded.len() {
        decoded[index] = (hex_nibble(hex[index * 2]) << 4) | hex_nibble(hex[index * 2 + 1]);
        index += 1;
    }
    decoded
}

const fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid hexadecimal maintenance-plan trust anchor"),
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::{
        MAINTENANCE_PLAN_OPERATION_RESIDENT_DRAIN, MAINTENANCE_PLAN_OPERATION_STORAGE_ROTATION,
        MAINTENANCE_PLAN_ROOT_SHA256, MAINTENANCE_PLAN_RSA2048_KEY1, MAINTENANCE_PLAN_SIGNED_SIZE,
        MaintenancePlanBinding, MaintenancePlanError, verify_maintenance_plan,
    };
    use crate::maintenance_authorization::{
        FIXTURE_MAINTENANCE_BINDING, MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
        verify_maintenance_authorization,
    };
    use crate::verified_manifest::sha256;

    const AUTHORIZATION: &str =
        include_str!("../../../boot/maintenance-authorization-sequence2.bma1.hex");
    const PLAN: &str = include_str!("../../../boot/maintenance-plan-sequence2.bmp1.hex");
    const REQUEST: &str =
        include_str!("../../../boot/maintenance/maintenance-plan-sequence2.request.hex");
    const BAD_SIGNATURE: &str = include_str!(
        "../../../boot/test-fixtures/maintenance-plan-sequence2-bad-signature.bmp1.hex"
    );
    const WRONG_BINDING: &str = include_str!(
        "../../../boot/test-fixtures/maintenance-plan-sequence2-wrong-binding.bmp1.hex"
    );
    const INVALID_PROGRAM: &str = include_str!(
        "../../../boot/test-fixtures/maintenance-plan-sequence2-invalid-program.bmp1.hex"
    );

    #[test]
    fn fixture_plan_has_real_signature_and_bounded_program() {
        let authorization = fixture_authorization();
        let plan = decode_hex(PLAN);
        let verified = verify_maintenance_plan(
            &plan,
            &MAINTENANCE_PLAN_RSA2048_KEY1,
            fixture_binding(&authorization),
        )
        .unwrap();
        assert_eq!(verified.authorization_sequence(), 2);
        assert_eq!(verified.operation_count(), 3);
        assert_eq!(verified.transition_count(), 9);
        assert_eq!(verified.program_version(), 1);
        assert_eq!(verified.plan_generation(), 2);
        assert_eq!(verified.preapply_cancel_mask(), 1);
        assert_eq!(
            verified.operation(1).unwrap().kind(),
            MAINTENANCE_PLAN_OPERATION_STORAGE_ROTATION
        );
        assert_eq!(verified.operation(1).unwrap().argument(), 1);
        assert_eq!(verified.operation(2).unwrap().argument(), 2);
        assert_eq!(
            verified.operation(3).unwrap().kind(),
            MAINTENANCE_PLAN_OPERATION_RESIDENT_DRAIN
        );
        assert_eq!(verified.operation(3).unwrap().argument(), 2);
        assert!(verified.operation(4).is_none());
        assert_eq!(
            sha256(MAINTENANCE_PLAN_RSA2048_KEY1.modulus()),
            MAINTENANCE_PLAN_ROOT_SHA256
        );
    }

    #[test]
    fn offline_request_is_exactly_the_signed_region() {
        let artifact = decode_hex(PLAN);
        let request = decode_hex(REQUEST);
        assert_eq!(request, artifact[..MAINTENANCE_PLAN_SIGNED_SIZE]);
    }

    #[test]
    fn signature_binding_and_program_fail_closed_independently() {
        let authorization = fixture_authorization();
        let binding = fixture_binding(&authorization);
        assert_eq!(
            verify_maintenance_plan(
                &decode_hex(BAD_SIGNATURE),
                &MAINTENANCE_PLAN_RSA2048_KEY1,
                binding,
            ),
            Err(MaintenancePlanError::Signature)
        );
        assert_eq!(
            verify_maintenance_plan(
                &decode_hex(WRONG_BINDING),
                &MAINTENANCE_PLAN_RSA2048_KEY1,
                binding,
            ),
            Err(MaintenancePlanError::Binding)
        );
        assert_eq!(
            verify_maintenance_plan(
                &decode_hex(INVALID_PROGRAM),
                &MAINTENANCE_PLAN_RSA2048_KEY1,
                binding,
            ),
            Err(MaintenancePlanError::InvalidProgram)
        );
    }

    #[test]
    fn signed_region_mutation_is_a_signature_failure() {
        let authorization = fixture_authorization();
        let mut artifact = decode_hex(PLAN);
        artifact[196] ^= 1;
        assert_eq!(
            verify_maintenance_plan(
                &artifact,
                &MAINTENANCE_PLAN_RSA2048_KEY1,
                fixture_binding(&authorization),
            ),
            Err(MaintenancePlanError::Signature)
        );
    }

    fn fixture_authorization() -> crate::maintenance_authorization::VerifiedMaintenanceAuthorization
    {
        verify_maintenance_authorization(
            &decode_hex(AUTHORIZATION),
            &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
            FIXTURE_MAINTENANCE_BINDING,
        )
        .unwrap()
    }

    fn fixture_binding(
        authorization: &crate::maintenance_authorization::VerifiedMaintenanceAuthorization,
    ) -> MaintenancePlanBinding {
        MaintenancePlanBinding {
            authorization_sequence: authorization.sequence(),
            authorization_sha256: *authorization.signed_region_sha256(),
            authorization_id: *authorization.authorization_id(),
            device_binding_sha256: FIXTURE_MAINTENANCE_BINDING.device_binding_sha256,
            plan_id_sha256: [
                0x8e, 0x5d, 0x48, 0x22, 0xf6, 0x51, 0xf5, 0x04, 0xca, 0x58, 0x82, 0x5b, 0xda, 0xc4,
                0x08, 0xcc, 0xf8, 0x6b, 0xb5, 0xb2, 0xd5, 0x2a, 0xc5, 0x8c, 0xa9, 0xa4, 0x44, 0x33,
                0x79, 0x6f, 0xbf, 0xca,
            ],
        }
    }

    fn decode_hex(encoded: &str) -> Vec<u8> {
        let digits: Vec<u8> = encoded
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        assert_eq!(digits.len() % 2, 0);
        digits
            .chunks_exact(2)
            .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
            .collect()
    }

    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("fixture contains non-lowercase hexadecimal"),
        }
    }
}
