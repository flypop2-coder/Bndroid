//! Signed, one-use maintenance authorization for the product supervisor.
//!
//! `BMA1` deliberately keeps the same 256-byte signed region and 256-byte
//! RSA-2048 signature boundary as `BMS1`, but it has a separate wire namespace
//! and trust anchor. A valid signature is only the first gate: the signed
//! region must also bind the exact verified product manifest, keyring policy,
//! device namespace, maintenance policy, operation set, and bounded use count.
//! Persistent replay protection is enforced by the kernel's audit transaction,
//! not by this allocation-free decoder.

use crate::verified_manifest::{Rsa2048PublicKey, rsa2048_pkcs1_v15_sha256_verify, sha256};

pub const MAINTENANCE_AUTHORIZATION_MAGIC: [u8; 4] = *b"BMA1";
pub const MAINTENANCE_AUTHORIZATION_VERSION: u8 = 1;
pub const MAINTENANCE_AUTHORIZATION_ALGORITHM_RSA2048_PKCS1_V15_SHA256: u8 = 1;
pub const MAINTENANCE_AUTHORIZATION_KEY_ID: u8 = 1;
pub const MAINTENANCE_AUTHORIZATION_HEADER_SIZE: usize = 64;
pub const MAINTENANCE_AUTHORIZATION_PAYLOAD_SIZE: usize = 192;
pub const MAINTENANCE_AUTHORIZATION_SIGNED_SIZE: usize =
    MAINTENANCE_AUTHORIZATION_HEADER_SIZE + MAINTENANCE_AUTHORIZATION_PAYLOAD_SIZE;
pub const MAINTENANCE_AUTHORIZATION_SIGNATURE_SIZE: usize = 256;
pub const MAINTENANCE_AUTHORIZATION_SIZE: usize =
    MAINTENANCE_AUTHORIZATION_SIGNED_SIZE + MAINTENANCE_AUTHORIZATION_SIGNATURE_SIZE;
pub const MAINTENANCE_OPERATION_STORAGE_ROTATION: u64 = 1;
pub const MAINTENANCE_AUTHORIZATION_MAX_USES: u32 = 2;
pub const MAINTENANCE_AUDIT_STATE_VERSION: u32 = 1;

pub const MAINTENANCE_PRODUCT_MANIFEST_SHA256: [u8; 32] = [
    0x5a, 0x8c, 0xcd, 0x0a, 0xe6, 0x01, 0xda, 0xd4, 0x33, 0x56, 0x78, 0x2e, 0xd0, 0xcc, 0x80, 0x3f,
    0xc3, 0xfe, 0x12, 0x32, 0xd3, 0xf4, 0x2b, 0xe9, 0x81, 0xa7, 0x1d, 0x94, 0xca, 0x33, 0xe9, 0x88,
];
pub const MAINTENANCE_PRODUCT_KEY_POLICY_SHA256: [u8; 32] = [
    0x30, 0x67, 0x6f, 0x35, 0x76, 0x83, 0xb6, 0xc8, 0xd2, 0xc5, 0xd6, 0x77, 0x55, 0xe4, 0x8b, 0xeb,
    0x93, 0xbd, 0xfe, 0x88, 0x64, 0xec, 0xcc, 0x67, 0xc7, 0x16, 0xa1, 0x78, 0x5e, 0x93, 0x5e, 0x01,
];
pub const MAINTENANCE_DEVICE_BINDING_SHA256: [u8; 32] = [
    0x04, 0xed, 0x2c, 0x7c, 0x65, 0x7e, 0xbe, 0x51, 0x47, 0xd3, 0xda, 0x71, 0x03, 0xea, 0x7a, 0x7e,
    0x0d, 0x4d, 0x9b, 0xbe, 0x80, 0x89, 0x15, 0x89, 0x89, 0x67, 0x25, 0x9e, 0x1b, 0x04, 0xa5, 0x55,
];
pub const MAINTENANCE_POLICY_SHA256: [u8; 32] = [
    0xd5, 0x1f, 0xfa, 0x50, 0xe8, 0xae, 0x40, 0xf2, 0x78, 0xb5, 0x24, 0x3f, 0x9d, 0x86, 0xa9, 0x85,
    0xb6, 0xd2, 0x83, 0x5b, 0xe5, 0x9f, 0xf4, 0xc2, 0x62, 0xd6, 0xc6, 0x43, 0x6e, 0xfe, 0x82, 0x00,
];
pub const MAINTENANCE_AUTHORIZATION_ROOT_SHA256: [u8; 32] = [
    0x68, 0x3e, 0xaf, 0xa4, 0x12, 0x77, 0xe7, 0x5a, 0xe8, 0xcd, 0x13, 0x32, 0x49, 0x08, 0x39, 0xa9,
    0xca, 0xb2, 0x04, 0xc8, 0xe8, 0xb8, 0x08, 0xe2, 0xeb, 0xdf, 0x77, 0xf3, 0x72, 0x59, 0xb0, 0x05,
];

const MAINTENANCE_AUTHORIZATION_RSA2048_MODULUS_HEX: &[u8; 512] =
    b"d444712ced368992a4221470281839d6aa7b76c49d60a4463b47be23f922869414b94649aef5fce59b03ac1401d07b73952a2868a1ee2a58b27cbd22317c5ea248e7dc0376695958fad47b3f7ea6685288a3693e211c418894206c02ac0ff6bbe00190e59495510972229d4227840d8362ca12ff09f8a30460fdb0c94ca5528c707cf3540471fd5885f6055d376879bad30b5f9e06c32f28eafe43f9f2498ae32c90c86402fb65cf3ef7b81b2bcdccdc6d1c0a251eee1d6bef82b4da0194906af334f3f58795ebe56e4aa94079aa16e904a7cdbbcd2944042a19cb8808e62b38578477259f39b6da383fd3a2af61c1dc47e4586699eb253480e9bc1e2e86bf83";

pub const MAINTENANCE_AUTHORIZATION_RSA2048_KEY1: Rsa2048PublicKey = Rsa2048PublicKey::new(
    MAINTENANCE_AUTHORIZATION_KEY_ID,
    decode_modulus_hex(MAINTENANCE_AUTHORIZATION_RSA2048_MODULUS_HEX),
);

const VERSION_OFFSET: usize = 4;
const ALGORITHM_OFFSET: usize = 5;
const KEY_ID_OFFSET: usize = 6;
const FLAGS_OFFSET: usize = 7;
const TOTAL_SIZE_OFFSET: usize = 8;
const SIGNED_SIZE_OFFSET: usize = 12;
const PAYLOAD_OFFSET_OFFSET: usize = 16;
const PAYLOAD_SIZE_OFFSET: usize = 20;
const SEQUENCE_OFFSET: usize = 24;
const OPERATIONS_OFFSET: usize = 32;
const MAX_USES_OFFSET: usize = 40;
const MANIFEST_GENERATION_OFFSET: usize = 44;
const ACTIVE_KEY_EPOCH_OFFSET: usize = 48;
const AUDIT_VERSION_OFFSET: usize = 52;
const HEADER_RESERVED_OFFSET: usize = 56;
const MANIFEST_DIGEST_OFFSET: usize = 64;
const KEY_POLICY_DIGEST_OFFSET: usize = 96;
const DEVICE_BINDING_OFFSET: usize = 128;
const MAINTENANCE_POLICY_OFFSET: usize = 160;
const AUTHORIZATION_ID_OFFSET: usize = 192;
const PAYLOAD_RESERVED_OFFSET: usize = 224;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuthorizationBinding {
    pub manifest_sha256: [u8; 32],
    pub key_policy_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub maintenance_policy_sha256: [u8; 32],
    pub manifest_generation: u32,
    pub active_key_epoch: u32,
}

pub const FIXTURE_MAINTENANCE_BINDING: MaintenanceAuthorizationBinding =
    MaintenanceAuthorizationBinding {
        manifest_sha256: MAINTENANCE_PRODUCT_MANIFEST_SHA256,
        key_policy_sha256: MAINTENANCE_PRODUCT_KEY_POLICY_SHA256,
        device_binding_sha256: MAINTENANCE_DEVICE_BINDING_SHA256,
        maintenance_policy_sha256: MAINTENANCE_POLICY_SHA256,
        manifest_generation: 5,
        active_key_epoch: 4,
    };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedMaintenanceAuthorization {
    sequence: u64,
    operations: u64,
    max_uses: u32,
    manifest_generation: u32,
    active_key_epoch: u32,
    authorization_id: [u8; 32],
    signed_region_sha256: [u8; 32],
}

impl VerifiedMaintenanceAuthorization {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn operations(&self) -> u64 {
        self.operations
    }

    pub const fn max_uses(&self) -> u32 {
        self.max_uses
    }

    pub const fn manifest_generation(&self) -> u32 {
        self.manifest_generation
    }

    pub const fn active_key_epoch(&self) -> u32 {
        self.active_key_epoch
    }

    pub const fn authorization_id(&self) -> &[u8; 32] {
        &self.authorization_id
    }

    pub const fn signed_region_sha256(&self) -> &[u8; 32] {
        &self.signed_region_sha256
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceAuthorizationError {
    Length { actual: usize },
    BadMagic,
    UnsupportedVersion(u8),
    UnsupportedAlgorithm(u8),
    UnknownKey { actual: u8, expected: u8 },
    NonZeroFlags(u8),
    Layout,
    NonZeroReserved,
    ZeroSequence,
    UnsupportedOperations(u64),
    InvalidUseLimit(u32),
    InvalidManifestGeneration(u32),
    InvalidActiveKeyEpoch(u32),
    InvalidAuditVersion(u32),
    Signature,
    Binding,
    AuthorizationId,
}

/// Verifies one complete BMA1 artifact before persistent replay enforcement.
///
/// Cryptographic verification intentionally precedes semantic binding checks,
/// so unsigned input cannot select a device, manifest, or policy namespace.
pub fn verify_maintenance_authorization(
    artifact: &[u8],
    key: &Rsa2048PublicKey,
    expected: MaintenanceAuthorizationBinding,
) -> Result<VerifiedMaintenanceAuthorization, MaintenanceAuthorizationError> {
    if artifact.len() != MAINTENANCE_AUTHORIZATION_SIZE {
        return Err(MaintenanceAuthorizationError::Length {
            actual: artifact.len(),
        });
    }
    if artifact[..4] != MAINTENANCE_AUTHORIZATION_MAGIC {
        return Err(MaintenanceAuthorizationError::BadMagic);
    }
    if artifact[VERSION_OFFSET] != MAINTENANCE_AUTHORIZATION_VERSION {
        return Err(MaintenanceAuthorizationError::UnsupportedVersion(
            artifact[VERSION_OFFSET],
        ));
    }
    if artifact[ALGORITHM_OFFSET] != MAINTENANCE_AUTHORIZATION_ALGORITHM_RSA2048_PKCS1_V15_SHA256 {
        return Err(MaintenanceAuthorizationError::UnsupportedAlgorithm(
            artifact[ALGORITHM_OFFSET],
        ));
    }
    if artifact[KEY_ID_OFFSET] != key.key_id() {
        return Err(MaintenanceAuthorizationError::UnknownKey {
            actual: artifact[KEY_ID_OFFSET],
            expected: key.key_id(),
        });
    }
    if artifact[FLAGS_OFFSET] != 0 {
        return Err(MaintenanceAuthorizationError::NonZeroFlags(
            artifact[FLAGS_OFFSET],
        ));
    }
    if read_u32(artifact, TOTAL_SIZE_OFFSET) as usize != MAINTENANCE_AUTHORIZATION_SIZE
        || read_u32(artifact, SIGNED_SIZE_OFFSET) as usize != MAINTENANCE_AUTHORIZATION_SIGNED_SIZE
        || read_u32(artifact, PAYLOAD_OFFSET_OFFSET) as usize
            != MAINTENANCE_AUTHORIZATION_HEADER_SIZE
        || read_u32(artifact, PAYLOAD_SIZE_OFFSET) as usize
            != MAINTENANCE_AUTHORIZATION_PAYLOAD_SIZE
    {
        return Err(MaintenanceAuthorizationError::Layout);
    }
    if artifact[HEADER_RESERVED_OFFSET..MAINTENANCE_AUTHORIZATION_HEADER_SIZE]
        .iter()
        .chain(artifact[PAYLOAD_RESERVED_OFFSET..MAINTENANCE_AUTHORIZATION_SIGNED_SIZE].iter())
        .any(|byte| *byte != 0)
    {
        return Err(MaintenanceAuthorizationError::NonZeroReserved);
    }

    let sequence = read_u64(artifact, SEQUENCE_OFFSET);
    if sequence == 0 {
        return Err(MaintenanceAuthorizationError::ZeroSequence);
    }
    let operations = read_u64(artifact, OPERATIONS_OFFSET);
    if operations != MAINTENANCE_OPERATION_STORAGE_ROTATION {
        return Err(MaintenanceAuthorizationError::UnsupportedOperations(
            operations,
        ));
    }
    let max_uses = read_u32(artifact, MAX_USES_OFFSET);
    if max_uses != MAINTENANCE_AUTHORIZATION_MAX_USES {
        return Err(MaintenanceAuthorizationError::InvalidUseLimit(max_uses));
    }
    let manifest_generation = read_u32(artifact, MANIFEST_GENERATION_OFFSET);
    if manifest_generation == 0 {
        return Err(MaintenanceAuthorizationError::InvalidManifestGeneration(
            manifest_generation,
        ));
    }
    let active_key_epoch = read_u32(artifact, ACTIVE_KEY_EPOCH_OFFSET);
    if active_key_epoch == 0 {
        return Err(MaintenanceAuthorizationError::InvalidActiveKeyEpoch(
            active_key_epoch,
        ));
    }
    let audit_version = read_u32(artifact, AUDIT_VERSION_OFFSET);
    if audit_version != MAINTENANCE_AUDIT_STATE_VERSION {
        return Err(MaintenanceAuthorizationError::InvalidAuditVersion(
            audit_version,
        ));
    }

    let signed_region = &artifact[..MAINTENANCE_AUTHORIZATION_SIGNED_SIZE];
    let signed_region_sha256 = sha256(signed_region);
    if !rsa2048_pkcs1_v15_sha256_verify(
        &artifact[MAINTENANCE_AUTHORIZATION_SIGNED_SIZE..],
        &signed_region_sha256,
        key.modulus(),
    ) {
        return Err(MaintenanceAuthorizationError::Signature);
    }

    if artifact[MANIFEST_DIGEST_OFFSET..KEY_POLICY_DIGEST_OFFSET] != expected.manifest_sha256
        || artifact[KEY_POLICY_DIGEST_OFFSET..DEVICE_BINDING_OFFSET] != expected.key_policy_sha256
        || artifact[DEVICE_BINDING_OFFSET..MAINTENANCE_POLICY_OFFSET]
            != expected.device_binding_sha256
        || artifact[MAINTENANCE_POLICY_OFFSET..AUTHORIZATION_ID_OFFSET]
            != expected.maintenance_policy_sha256
        || manifest_generation != expected.manifest_generation
        || active_key_epoch != expected.active_key_epoch
    {
        return Err(MaintenanceAuthorizationError::Binding);
    }

    let authorization_id = authorization_id(
        sequence,
        operations,
        max_uses,
        manifest_generation,
        active_key_epoch,
        expected,
    );
    if artifact[AUTHORIZATION_ID_OFFSET..PAYLOAD_RESERVED_OFFSET] != authorization_id {
        return Err(MaintenanceAuthorizationError::AuthorizationId);
    }

    Ok(VerifiedMaintenanceAuthorization {
        sequence,
        operations,
        max_uses,
        manifest_generation,
        active_key_epoch,
        authorization_id,
        signed_region_sha256,
    })
}

pub fn authorization_id(
    sequence: u64,
    operations: u64,
    max_uses: u32,
    manifest_generation: u32,
    active_key_epoch: u32,
    binding: MaintenanceAuthorizationBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 132];
    material[..8].copy_from_slice(b"BMA1-ID\0");
    material[8..16].copy_from_slice(&sequence.to_le_bytes());
    material[16..24].copy_from_slice(&operations.to_le_bytes());
    material[24..28].copy_from_slice(&max_uses.to_le_bytes());
    material[28..32].copy_from_slice(&manifest_generation.to_le_bytes());
    material[32..36].copy_from_slice(&active_key_epoch.to_le_bytes());
    material[36..68].copy_from_slice(&binding.manifest_sha256);
    material[68..100].copy_from_slice(&binding.device_binding_sha256);
    material[100..132].copy_from_slice(&binding.maintenance_policy_sha256);
    sha256(&material)
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
        _ => panic!("invalid hexadecimal maintenance trust anchor"),
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
        FIXTURE_MAINTENANCE_BINDING, MAINTENANCE_AUTHORIZATION_ROOT_SHA256,
        MAINTENANCE_AUTHORIZATION_RSA2048_KEY1, MAINTENANCE_AUTHORIZATION_SIGNED_SIZE,
        MAINTENANCE_OPERATION_STORAGE_ROTATION, MaintenanceAuthorizationError,
        verify_maintenance_authorization,
    };
    use crate::verified_manifest::sha256;

    const SEQUENCE1: &str =
        include_str!("../../../boot/maintenance-authorization-sequence1.bma1.hex");
    const SEQUENCE2: &str =
        include_str!("../../../boot/maintenance-authorization-sequence2.bma1.hex");
    const SEQUENCE1_REQUEST: &str =
        include_str!("../../../boot/maintenance/maintenance-authorization-sequence1.request.hex");
    const BAD_SIGNATURE: &str = include_str!(
        "../../../boot/test-fixtures/maintenance-authorization-sequence2-bad-signature.bma1.hex"
    );
    const WRONG_BINDING: &str = include_str!(
        "../../../boot/test-fixtures/maintenance-authorization-sequence3-wrong-binding.bma1.hex"
    );

    #[test]
    fn fixture_authorizations_have_real_signatures_and_ordered_sequences() {
        let first = decode_hex(SEQUENCE1);
        let second = decode_hex(SEQUENCE2);
        let first_verified = verify_maintenance_authorization(
            &first,
            &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
            FIXTURE_MAINTENANCE_BINDING,
        )
        .unwrap();
        let second_verified = verify_maintenance_authorization(
            &second,
            &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
            FIXTURE_MAINTENANCE_BINDING,
        )
        .unwrap();
        assert_eq!(first_verified.sequence(), 1);
        assert_eq!(second_verified.sequence(), 2);
        assert_eq!(
            first_verified.operations(),
            MAINTENANCE_OPERATION_STORAGE_ROTATION
        );
        assert_eq!(first_verified.max_uses(), 2);
        assert_ne!(
            first_verified.signed_region_sha256(),
            second_verified.signed_region_sha256()
        );
        assert_ne!(
            first_verified.authorization_id(),
            second_verified.authorization_id()
        );
        assert_eq!(
            sha256(MAINTENANCE_AUTHORIZATION_RSA2048_KEY1.modulus()),
            MAINTENANCE_AUTHORIZATION_ROOT_SHA256
        );
    }

    #[test]
    fn offline_request_is_exactly_the_signed_region() {
        let artifact = decode_hex(SEQUENCE1);
        let request = decode_hex(SEQUENCE1_REQUEST);
        assert_eq!(request, artifact[..MAINTENANCE_AUTHORIZATION_SIGNED_SIZE]);
    }

    #[test]
    fn signature_and_signed_binding_fail_closed_independently() {
        let bad_signature = decode_hex(BAD_SIGNATURE);
        assert_eq!(
            verify_maintenance_authorization(
                &bad_signature,
                &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
                FIXTURE_MAINTENANCE_BINDING,
            ),
            Err(MaintenanceAuthorizationError::Signature)
        );

        let wrong_binding = decode_hex(WRONG_BINDING);
        assert_eq!(
            verify_maintenance_authorization(
                &wrong_binding,
                &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
                FIXTURE_MAINTENANCE_BINDING,
            ),
            Err(MaintenanceAuthorizationError::Binding)
        );
    }

    #[test]
    fn envelope_and_authorization_id_mutations_are_rejected() {
        let artifact = decode_hex(SEQUENCE1);
        let mut bad_magic = artifact.clone();
        bad_magic[0] ^= 1;
        assert_eq!(
            verify_maintenance_authorization(
                &bad_magic,
                &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
                FIXTURE_MAINTENANCE_BINDING,
            ),
            Err(MaintenanceAuthorizationError::BadMagic)
        );

        let mut bad_layout = artifact.clone();
        bad_layout[8] ^= 1;
        assert_eq!(
            verify_maintenance_authorization(
                &bad_layout,
                &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
                FIXTURE_MAINTENANCE_BINDING,
            ),
            Err(MaintenanceAuthorizationError::Layout)
        );

        let mut bad_id = artifact;
        bad_id[192] ^= 1;
        assert_eq!(
            verify_maintenance_authorization(
                &bad_id,
                &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
                FIXTURE_MAINTENANCE_BINDING,
            ),
            Err(MaintenanceAuthorizationError::Signature)
        );
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
