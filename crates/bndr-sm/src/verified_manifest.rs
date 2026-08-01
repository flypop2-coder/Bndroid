//! Signed service-manifest envelope and allocation-free verification.
//!
//! `BMS1` signs one exact 256-byte region: a strict 32-byte envelope header
//! followed by one 224-byte BMF1 payload. The detached 256-byte signature uses
//! RSA-2048 PKCS#1 v1.5 with SHA-256. Verification accepts only the kernel's
//! pinned key id and modulus, then enforces a trusted rollback floor before
//! exposing the decoded [`ServiceManifest`].
//!
//! RSA verification deliberately uses a small fixed-width implementation
//! instead of a dependency or firmware service. It handles only public
//! exponent 65537 and is not a general-purpose cryptographic API.

use crate::manifest::{ManifestDecodeError, ServiceManifest};

pub const SIGNED_SERVICE_MANIFEST_MAGIC: [u8; 4] = *b"BMS1";
pub const SIGNED_SERVICE_MANIFEST_VERSION: u8 = 1;
pub const SIGNED_SERVICE_MANIFEST_ALGORITHM_RSA2048_PKCS1_V15_SHA256: u8 = 1;
pub const SIGNED_SERVICE_MANIFEST_HEADER_SIZE: usize = 32;
pub const SIGNED_SERVICE_MANIFEST_PAYLOAD_SIZE: usize = 224;
pub const SIGNED_SERVICE_MANIFEST_SIGNED_SIZE: usize =
    SIGNED_SERVICE_MANIFEST_HEADER_SIZE + SIGNED_SERVICE_MANIFEST_PAYLOAD_SIZE;
pub const SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE: usize = 256;
pub const SIGNED_SERVICE_MANIFEST_SIZE: usize =
    SIGNED_SERVICE_MANIFEST_SIGNED_SIZE + SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE;
pub const PRODUCT_MANIFEST_TRUSTED_KEY_ID: u8 = 1;
pub const PRODUCT_MANIFEST_ROLLBACK_FLOOR: u32 = 2;
pub const PERSISTENT_MANIFEST_TRUSTED_KEY_ID: u8 = 2;
pub const PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR: u32 = 2;
pub const KEY_ROTATION_TRANSITION_KEY_ID: u8 = 3;
pub const KEY_ROTATION_ACTIVE_KEY_ID: u8 = 4;
pub const KEY_ROTATION_TRANSITION_KEY_EPOCH: u32 = 3;
pub const KEY_ROTATION_ACTIVE_KEY_EPOCH: u32 = 4;
pub const KEY_ROTATION_POLICY_SHA256: [u8; 32] = [
    0x30, 0x67, 0x6f, 0x35, 0x76, 0x83, 0xb6, 0xc8, 0xd2, 0xc5, 0xd6, 0x77, 0x55, 0xe4, 0x8b, 0xeb,
    0x93, 0xbd, 0xfe, 0x88, 0x64, 0xec, 0xcc, 0x67, 0xc7, 0x16, 0xa1, 0x78, 0x5e, 0x93, 0x5e, 0x01,
];
pub const MAX_MANIFEST_TRUST_ANCHORS: usize = 8;

const VERSION_OFFSET: usize = 4;
const ALGORITHM_OFFSET: usize = 5;
const KEY_ID_OFFSET: usize = 6;
const FLAGS_OFFSET: usize = 7;
const TOTAL_SIZE_OFFSET: usize = 8;
const SIGNED_SIZE_OFFSET: usize = 12;
const PAYLOAD_OFFSET_OFFSET: usize = 16;
const PAYLOAD_SIZE_OFFSET: usize = 20;
const ROLLBACK_INDEX_OFFSET: usize = 24;
const RESERVED_OFFSET: usize = 28;

const RSA_LIMBS: usize = SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE / 4;
type RsaValue = [u32; RSA_LIMBS];

const PRODUCT_MANIFEST_RSA2048_MODULUS_HEX: &[u8; 512] =
    b"ac4b9f4890228584eeee8333c84221db1a3273366c4b6e9783f7c279a77d804d76555f4a57f0fe69974da419d99fccaf1f72ab4aae797464914a0601dea42c83b33d4ffe020880a83b2418e0795380654b8a9567310f1531ba78c0cbbeae93e367d3825ebc88d63f593d89fb69b819ee0eac23273ba14054c5114cb87d5c3d33529ec7b331e1ab401617a8be9966721ffdaf1486b384b68b080c84f5038e76e190fdba1ee136803157756021a9b791e4ae27938d507b09b774e63dc5fb8bc13a2ba9dc0913977490c84170c004ec2a94ba62e49974154f41d08284bbc7399fe4aefab4baab7349a8a6e2c696ff0d1e3729dd2f815f6ba4b30d59104820f5286b";
const PERSISTENT_MANIFEST_RSA2048_MODULUS_HEX: &[u8; 512] =
    b"d7cc4e2922920a3feafda46ac10a2bcc23bdb7efebcbba4ae5461a1da11a90c4f40c47515dba303da54070838738fbd0d1d992c29fddb00addb729b52e793af5d9d542344c93f01c2d574634b58f560cd517cc020fb0d4207c3f761cc7cc7604c30b9797e44f3c1ce2182134b4b55ff751b71c895f068addedc14a8473d45c2ef53a068d87d3aa33b004ff4584b2fc48755cbdb3d1dface05c3d2f7fbd6d8e8c64bbc7caec1123bbb520a901560d489972913ad19adc7bec863958814d70bb519c9bac7af42fa7746cf5ae3b66166fdbbfdeb11eee7b748155ac915863bfb967032bad3b08bedce2b09ae317cacbb69d1284278cc1855be37244aabf6c7df697";
const KEY_ROTATION_MANIFEST_RSA2048_KEY3_MODULUS_HEX: &[u8; 512] =
    b"caa12eeee9786123c8bde4fdab27a7200e328d5c709da83d0c953d205524779105ad5c1cefb5fe06606c5104e58d8b561feda5ad22cdc38ce6fdbae2ec533e0f1a6380119340f93e72c10adfa1778de320e27339914e5a7cb4fea3e2ae82544c66cc96e374e60ecb40790449b1c4be9dc16038520ed2c7433728e4477fb9d1c85b059998c3c3c85531454733046bba62126d4d6c0ef5b1285ef7e49f36d9719f0107b504875c9649c443df475d00149cb96c4060fdd63ad7257148df1a7ddc19b11ff614cfc58a2b97e17f32f02c27ac5cf7996df64aab57926edd75ac5459bfd6949f12846085cd38db7614bf19012915aef5f1332c8f86b0886bae0d8458cb";
const KEY_ROTATION_MANIFEST_RSA2048_KEY4_MODULUS_HEX: &[u8; 512] =
    b"bd77fae2b4818869cd6eb329d58550cccb7fae97a1267b26e194ca8cae5cb7f90f1718f95392f7cd328ed6dcca18e4fccf4ad5e437a34c0fc6fd6bbdad73ccb9537cca48af9a474d25d6895d2254785a59deef5a4d0140bb70160865e2bcc1c48cfff78de009bee16cf5f75be4450155c7a3446a128406e66e07cdabbbbf0e6abac9ea26423384b99343ad38c7e5b709ed977aefc87e201badd68f6bad19e7c1253b996412caaec22e8edf0e4c49d343bf35c7ad263766bea9cab0c1a53687a66e8443b556c4cd98a3e38032bf60ddabeb42bd4cb9e11dece25a7c6e1f343d9373202a9f3b3b10f938088be94af9ff34d4e303a083970b6e2241c1d38bdd5907";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rsa2048PublicKey {
    key_id: u8,
    modulus: [u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE],
}

impl Rsa2048PublicKey {
    pub const fn new(key_id: u8, modulus: [u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE]) -> Self {
        Self { key_id, modulus }
    }

    pub const fn key_id(&self) -> u8 {
        self.key_id
    }

    pub const fn modulus(&self) -> &[u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE] {
        &self.modulus
    }
}

pub const PRODUCT_MANIFEST_RSA2048_KEY1: Rsa2048PublicKey = Rsa2048PublicKey::new(
    PRODUCT_MANIFEST_TRUSTED_KEY_ID,
    decode_modulus_hex(PRODUCT_MANIFEST_RSA2048_MODULUS_HEX),
);
pub const PERSISTENT_MANIFEST_RSA2048_KEY2: Rsa2048PublicKey = Rsa2048PublicKey::new(
    PERSISTENT_MANIFEST_TRUSTED_KEY_ID,
    decode_modulus_hex(PERSISTENT_MANIFEST_RSA2048_MODULUS_HEX),
);
pub const KEY_ROTATION_MANIFEST_RSA2048_KEY3: Rsa2048PublicKey = Rsa2048PublicKey::new(
    KEY_ROTATION_TRANSITION_KEY_ID,
    decode_modulus_hex(KEY_ROTATION_MANIFEST_RSA2048_KEY3_MODULUS_HEX),
);
pub const KEY_ROTATION_MANIFEST_RSA2048_KEY4: Rsa2048PublicKey = Rsa2048PublicKey::new(
    KEY_ROTATION_ACTIVE_KEY_ID,
    decode_modulus_hex(KEY_ROTATION_MANIFEST_RSA2048_KEY4_MODULUS_HEX),
);
pub const KEY_ROTATION_MANIFEST_KEYRING: [Rsa2048PublicKey; 3] = [
    PERSISTENT_MANIFEST_RSA2048_KEY2,
    KEY_ROTATION_MANIFEST_RSA2048_KEY3,
    KEY_ROTATION_MANIFEST_RSA2048_KEY4,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedServiceManifest {
    manifest: ServiceManifest,
    rollback_index: u32,
    key_id: u8,
    signed_region_sha256: [u8; 32],
}

impl VerifiedServiceManifest {
    pub const fn manifest(&self) -> &ServiceManifest {
        &self.manifest
    }

    pub const fn rollback_index(&self) -> u32 {
        self.rollback_index
    }

    pub const fn key_id(&self) -> u8 {
        self.key_id
    }

    pub const fn signed_region_sha256(&self) -> &[u8; 32] {
        &self.signed_region_sha256
    }

    pub const fn into_manifest(self) -> ServiceManifest {
        self.manifest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignedManifestError {
    Length { actual: usize },
    BadMagic,
    UnsupportedVersion(u8),
    UnsupportedAlgorithm(u8),
    UnknownKey { actual: u8, expected: u8 },
    NonZeroFlags(u8),
    NonZeroReserved,
    Layout,
    RollbackIndexZero,
    InvalidTrustAnchor,
    EmptyTrustAnchorSet,
    TooManyTrustAnchors { actual: usize },
    DuplicateTrustAnchor(u8),
    UnknownKeyringKey { actual: u8 },
    Signature,
    Manifest(ManifestDecodeError),
    GenerationMismatch { manifest: u32, rollback_index: u32 },
    Rollback { actual: u32, minimum: u32 },
}

/// Verifies a BMS1 artifact against one bounded, duplicate-free public keyring.
///
/// The envelope key id selects exactly one anchor. All keys remain public;
/// accepting a signature here does not authorize a retired key. The caller
/// must enforce the persistent key epoch after cryptographic verification.
pub fn verify_signed_service_manifest_with_keyring(
    artifact: &[u8],
    keys: &[Rsa2048PublicKey],
    minimum_rollback_index: u32,
) -> Result<VerifiedServiceManifest, SignedManifestError> {
    let _ = rsa2048_keyring_sha256(keys)?;
    if artifact.len() != SIGNED_SERVICE_MANIFEST_SIZE {
        return Err(SignedManifestError::Length {
            actual: artifact.len(),
        });
    }
    let key_id = artifact[KEY_ID_OFFSET];
    let key = keys
        .iter()
        .find(|key| key.key_id == key_id)
        .ok_or(SignedManifestError::UnknownKeyringKey { actual: key_id })?;
    verify_signed_service_manifest(artifact, key, minimum_rollback_index)
}

/// Hashes the ordered public-key policy without allocation.
///
/// Each digest entry is `key_id || SHA256(modulus)`. Order is policy-significant
/// so an accidental epoch reorder produces a different persistent namespace.
pub fn rsa2048_keyring_sha256(keys: &[Rsa2048PublicKey]) -> Result<[u8; 32], SignedManifestError> {
    if keys.is_empty() {
        return Err(SignedManifestError::EmptyTrustAnchorSet);
    }
    if keys.len() > MAX_MANIFEST_TRUST_ANCHORS {
        return Err(SignedManifestError::TooManyTrustAnchors { actual: keys.len() });
    }
    let mut material = [0_u8; MAX_MANIFEST_TRUST_ANCHORS * 33];
    for (index, key) in keys.iter().enumerate() {
        if key.key_id == 0
            || key.modulus[0] & 0x80 == 0
            || key.modulus[SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE - 1] & 1 == 0
        {
            return Err(SignedManifestError::InvalidTrustAnchor);
        }
        if keys[..index]
            .iter()
            .any(|earlier| earlier.key_id == key.key_id)
        {
            return Err(SignedManifestError::DuplicateTrustAnchor(key.key_id));
        }
        let offset = index * 33;
        material[offset] = key.key_id;
        material[offset + 1..offset + 33].copy_from_slice(&sha256(&key.modulus));
    }
    Ok(sha256(&material[..keys.len() * 33]))
}

/// Verifies one complete BMS1 artifact before exposing its BMF1 payload.
pub fn verify_signed_service_manifest(
    artifact: &[u8],
    key: &Rsa2048PublicKey,
    minimum_rollback_index: u32,
) -> Result<VerifiedServiceManifest, SignedManifestError> {
    if artifact.len() != SIGNED_SERVICE_MANIFEST_SIZE {
        return Err(SignedManifestError::Length {
            actual: artifact.len(),
        });
    }
    if artifact[..SIGNED_SERVICE_MANIFEST_MAGIC.len()] != SIGNED_SERVICE_MANIFEST_MAGIC {
        return Err(SignedManifestError::BadMagic);
    }
    if artifact[VERSION_OFFSET] != SIGNED_SERVICE_MANIFEST_VERSION {
        return Err(SignedManifestError::UnsupportedVersion(
            artifact[VERSION_OFFSET],
        ));
    }
    if artifact[ALGORITHM_OFFSET] != SIGNED_SERVICE_MANIFEST_ALGORITHM_RSA2048_PKCS1_V15_SHA256 {
        return Err(SignedManifestError::UnsupportedAlgorithm(
            artifact[ALGORITHM_OFFSET],
        ));
    }
    if artifact[KEY_ID_OFFSET] != key.key_id {
        return Err(SignedManifestError::UnknownKey {
            actual: artifact[KEY_ID_OFFSET],
            expected: key.key_id,
        });
    }
    if artifact[FLAGS_OFFSET] != 0 {
        return Err(SignedManifestError::NonZeroFlags(artifact[FLAGS_OFFSET]));
    }
    if artifact[RESERVED_OFFSET..SIGNED_SERVICE_MANIFEST_HEADER_SIZE]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(SignedManifestError::NonZeroReserved);
    }
    if read_u32(artifact, TOTAL_SIZE_OFFSET) as usize != SIGNED_SERVICE_MANIFEST_SIZE
        || read_u32(artifact, SIGNED_SIZE_OFFSET) as usize != SIGNED_SERVICE_MANIFEST_SIGNED_SIZE
        || read_u32(artifact, PAYLOAD_OFFSET_OFFSET) as usize != SIGNED_SERVICE_MANIFEST_HEADER_SIZE
        || read_u32(artifact, PAYLOAD_SIZE_OFFSET) as usize != SIGNED_SERVICE_MANIFEST_PAYLOAD_SIZE
    {
        return Err(SignedManifestError::Layout);
    }
    let rollback_index = read_u32(artifact, ROLLBACK_INDEX_OFFSET);
    if rollback_index == 0 {
        return Err(SignedManifestError::RollbackIndexZero);
    }
    if key.modulus[0] & 0x80 == 0
        || key.modulus[SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE - 1] & 1 == 0
    {
        return Err(SignedManifestError::InvalidTrustAnchor);
    }

    let signed_region = &artifact[..SIGNED_SERVICE_MANIFEST_SIGNED_SIZE];
    let signature = &artifact[SIGNED_SERVICE_MANIFEST_SIGNED_SIZE..];
    let signed_region_sha256 = sha256(signed_region);
    if !rsa2048_pkcs1_v15_sha256_verify(signature, &signed_region_sha256, &key.modulus) {
        return Err(SignedManifestError::Signature);
    }

    let payload =
        &artifact[SIGNED_SERVICE_MANIFEST_HEADER_SIZE..SIGNED_SERVICE_MANIFEST_SIGNED_SIZE];
    let manifest = ServiceManifest::decode(payload).map_err(SignedManifestError::Manifest)?;
    if manifest.generation() != rollback_index {
        return Err(SignedManifestError::GenerationMismatch {
            manifest: manifest.generation(),
            rollback_index,
        });
    }
    if rollback_index < minimum_rollback_index {
        return Err(SignedManifestError::Rollback {
            actual: rollback_index,
            minimum: minimum_rollback_index,
        });
    }

    Ok(VerifiedServiceManifest {
        manifest,
        rollback_index,
        key_id: key.key_id,
        signed_region_sha256,
    })
}

/// Allocation-free incremental SHA-256 state.
///
/// This is intentionally a small hashing primitive for bounded system
/// formats. It does not allocate and may be fed discontiguous byte ranges.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    block_len: u8,
    input_len: u64,
}

impl Sha256 {
    pub const fn new() -> Self {
        Self {
            state: [
                0x6a09_e667_u32,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            block: [0; 64],
            block_len: 0,
            input_len: 0,
        }
    }

    pub fn update(&mut self, mut input: &[u8]) {
        self.input_len = self.input_len.wrapping_add(input.len() as u64);

        if self.block_len != 0 {
            let occupied = usize::from(self.block_len);
            let copied = core::cmp::min(64 - occupied, input.len());
            self.block[occupied..occupied + copied].copy_from_slice(&input[..copied]);
            self.block_len += copied as u8;
            input = &input[copied..];
            if self.block_len == 64 {
                sha256_compress(&mut self.state, &self.block);
                self.block_len = 0;
            } else {
                return;
            }
        }

        let mut chunks = input.chunks_exact(64);
        for block in &mut chunks {
            sha256_compress(&mut self.state, block);
        }
        let remainder = chunks.remainder();
        self.block[..remainder.len()].copy_from_slice(remainder);
        self.block_len = remainder.len() as u8;
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let mut occupied = usize::from(self.block_len);
        self.block[occupied] = 0x80;
        occupied += 1;
        if occupied > 56 {
            self.block[occupied..].fill(0);
            sha256_compress(&mut self.state, &self.block);
            self.block.fill(0);
            occupied = 0;
        }
        self.block[occupied..56].fill(0);
        self.block[56..].copy_from_slice(&self.input_len.wrapping_mul(8).to_be_bytes());
        sha256_compress(&mut self.state, &self.block);

        let mut digest = [0_u8; 32];
        for (index, word) in self.state.into_iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sha256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize()
}

fn sha256_compress(state: &mut [u32; 8], block: &[u8]) {
    debug_assert_eq!(block.len(), 64);
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut words = [0_u32; 64];
    for (index, word) in words.iter_mut().take(16).enumerate() {
        let offset = index * 4;
        *word = u32::from_be_bytes([
            block[offset],
            block[offset + 1],
            block[offset + 2],
            block[offset + 3],
        ]);
    }
    for index in 16..64 {
        let s0 = words[index - 15].rotate_right(7)
            ^ words[index - 15].rotate_right(18)
            ^ (words[index - 15] >> 3);
        let s1 = words[index - 2].rotate_right(17)
            ^ words[index - 2].rotate_right(19)
            ^ (words[index - 2] >> 10);
        words[index] = words[index - 16]
            .wrapping_add(s0)
            .wrapping_add(words[index - 7])
            .wrapping_add(s1);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];
    for index in 0..64 {
        let choice = (e & f) ^ ((!e) & g);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let temporary1 = h
            .wrapping_add(sum1)
            .wrapping_add(choice)
            .wrapping_add(K[index])
            .wrapping_add(words[index]);
        let temporary2 = sum0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temporary1);
        d = c;
        c = b;
        b = a;
        a = temporary1.wrapping_add(temporary2);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

/// Verifies one RSA-2048/65537 PKCS#1 v1.5 SHA-256 signature.
///
/// The modulus is public and must be exactly 2048 bits. This deliberately
/// supports no private-key operation and no other exponent or padding mode.
pub fn rsa2048_pkcs1_v15_sha256_verify(
    signature: &[u8],
    digest: &[u8; 32],
    modulus_bytes: &[u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE],
) -> bool {
    if signature.len() != SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE
        || modulus_bytes[0] & 0x80 == 0
        || modulus_bytes[SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE - 1] & 1 == 0
    {
        return false;
    }
    let modulus = rsa_value_from_be(modulus_bytes);
    let signature_value = rsa_value_from_be(signature);
    if rsa_is_zero(&signature_value)
        || rsa_compare(&signature_value, &modulus) != core::cmp::Ordering::Less
    {
        return false;
    }

    // 65537 = 2^16 + 1.
    let mut decoded_value = signature_value;
    for _ in 0..16 {
        decoded_value = rsa_multiply_mod(&decoded_value, &decoded_value, &modulus);
    }
    decoded_value = rsa_multiply_mod(&decoded_value, &signature_value, &modulus);
    let decoded = rsa_value_to_be(&decoded_value);

    const SHA256_DIGEST_INFO_PREFIX: [u8; 19] = [
        0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
        0x05, 0x00, 0x04, 0x20,
    ];
    const SEPARATOR: usize = 204;
    decoded[0] == 0
        && decoded[1] == 1
        && decoded[2..SEPARATOR].iter().all(|byte| *byte == 0xff)
        && decoded[SEPARATOR] == 0
        && decoded[SEPARATOR + 1..SEPARATOR + 1 + SHA256_DIGEST_INFO_PREFIX.len()]
            == SHA256_DIGEST_INFO_PREFIX
        && decoded[SEPARATOR + 1 + SHA256_DIGEST_INFO_PREFIX.len()..] == *digest
}

fn rsa_value_from_be(bytes: &[u8]) -> RsaValue {
    let mut value = [0_u32; RSA_LIMBS];
    for (index, limb) in value.iter_mut().enumerate() {
        let end = bytes.len() - index * 4;
        *limb = u32::from_be_bytes([
            bytes[end - 4],
            bytes[end - 3],
            bytes[end - 2],
            bytes[end - 1],
        ]);
    }
    value
}

fn rsa_value_to_be(value: &RsaValue) -> [u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE] {
    let mut bytes = [0_u8; SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE];
    for (index, limb) in value.iter().copied().enumerate() {
        let end = bytes.len() - index * 4;
        bytes[end - 4..end].copy_from_slice(&limb.to_be_bytes());
    }
    bytes
}

fn rsa_compare(left: &RsaValue, right: &RsaValue) -> core::cmp::Ordering {
    for index in (0..RSA_LIMBS).rev() {
        match left[index].cmp(&right[index]) {
            core::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    core::cmp::Ordering::Equal
}

fn rsa_is_zero(value: &RsaValue) -> bool {
    value.iter().all(|limb| *limb == 0)
}

fn rsa_subtract(left: &RsaValue, right: &RsaValue) -> RsaValue {
    let mut result = [0_u32; RSA_LIMBS];
    let mut borrow = 0_u32;
    for index in 0..RSA_LIMBS {
        let (partial, first_borrow) = left[index].overflowing_sub(right[index]);
        let (difference, second_borrow) = partial.overflowing_sub(borrow);
        result[index] = difference;
        borrow = u32::from(first_borrow || second_borrow);
    }
    debug_assert_eq!(borrow, 0);
    result
}

fn rsa_add_without_overflow(left: &RsaValue, right: &RsaValue) -> RsaValue {
    let mut result = [0_u32; RSA_LIMBS];
    let mut carry = 0_u64;
    for index in 0..RSA_LIMBS {
        let sum = u64::from(left[index]) + u64::from(right[index]) + carry;
        result[index] = sum as u32;
        carry = sum >> 32;
    }
    debug_assert_eq!(carry, 0);
    result
}

fn rsa_add_mod(left: &RsaValue, right: &RsaValue, modulus: &RsaValue) -> RsaValue {
    let distance_to_modulus = rsa_subtract(modulus, right);
    if rsa_compare(left, &distance_to_modulus) != core::cmp::Ordering::Less {
        rsa_subtract(left, &distance_to_modulus)
    } else {
        rsa_add_without_overflow(left, right)
    }
}

fn rsa_multiply_mod(left: &RsaValue, right: &RsaValue, modulus: &RsaValue) -> RsaValue {
    let mut result = [0_u32; RSA_LIMBS];
    let mut addend = *left;
    for bit_index in 0..SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE * 8 {
        if right[bit_index / 32] & (1_u32 << (bit_index % 32)) != 0 {
            result = rsa_add_mod(&result, &addend, modulus);
        }
        if bit_index + 1 != SIGNED_SERVICE_MANIFEST_SIGNATURE_SIZE * 8 {
            addend = rsa_add_mod(&addend, &addend, modulus);
        }
    }
    result
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
        _ => panic!("invalid hexadecimal trust anchor"),
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

#[cfg(test)]
mod tests {
    use std::string::String;
    use std::vec::Vec;

    use super::{
        KEY_ID_OFFSET, KEY_ROTATION_ACTIVE_KEY_ID, KEY_ROTATION_MANIFEST_KEYRING,
        KEY_ROTATION_MANIFEST_RSA2048_KEY3, KEY_ROTATION_MANIFEST_RSA2048_KEY4,
        KEY_ROTATION_POLICY_SHA256, KEY_ROTATION_TRANSITION_KEY_ID,
        PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR, PERSISTENT_MANIFEST_RSA2048_KEY2,
        PRODUCT_MANIFEST_ROLLBACK_FLOOR, PRODUCT_MANIFEST_RSA2048_KEY1, Rsa2048PublicKey,
        SIGNED_SERVICE_MANIFEST_SIGNED_SIZE, Sha256, SignedManifestError, rsa2048_keyring_sha256,
        rsa2048_pkcs1_v15_sha256_verify, sha256, verify_signed_service_manifest,
        verify_signed_service_manifest_with_keyring,
    };

    const PRODUCT: &str = include_str!("../../../boot/product-service-manifest-v2.bms1.hex");
    const PERSISTENT_PRODUCT: &str =
        include_str!("../../../boot/product-service-manifest-v3.bms1.hex");
    const PERSISTENT_OLD: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v2-persistent-rollback.bms1.hex"
    );
    const PERSISTENT_BAD_SIGNATURE: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v3-persistent-bad-signature.bms1.hex"
    );
    const KEY3_TRANSITION: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex"
    );
    const KEY3_RETIRED_HIGH_INDEX: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v6-retired-key3.bms1.hex"
    );
    const KEY4_PRODUCT: &str =
        include_str!("../../../boot/product-service-manifest-v5-key4.bms1.hex");
    const KEY4_BAD_SIGNATURE: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v5-key4-bad-signature.bms1.hex"
    );
    const ROLLBACK: &str =
        include_str!("../../../boot/test-fixtures/product-service-manifest-v1-rollback.bms1.hex");
    const BAD_SIGNATURE: &str = include_str!(
        "../../../boot/test-fixtures/product-service-manifest-v2-bad-signature.bms1.hex"
    );

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn streaming_sha256_matches_one_shot_across_block_boundaries() {
        let input = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\
            discontiguous APK sections and one final partial SHA-256 block";
        let expected = sha256(input);
        for width in [1, 2, 3, 7, 55, 56, 63, 64, 65, 100] {
            let mut streaming = Sha256::new();
            for chunk in input.chunks(width) {
                streaming.update(chunk);
            }
            assert_eq!(streaming.finalize(), expected, "chunk width {width}");
        }
    }

    #[test]
    fn public_rsa_primitive_rejects_wrong_lengths_and_invalid_moduli() {
        let digest = [0; 32];
        let signature = [1; 256];
        let zero_modulus = [0; 256];
        assert!(!rsa2048_pkcs1_v15_sha256_verify(
            &signature,
            &digest,
            &zero_modulus
        ));

        let mut even_modulus = [0; 256];
        even_modulus[0] = 0x80;
        even_modulus[255] = 2;
        assert!(!rsa2048_pkcs1_v15_sha256_verify(
            &signature,
            &digest,
            &even_modulus
        ));
        assert!(!rsa2048_pkcs1_v15_sha256_verify(
            &signature[..255],
            &digest,
            PRODUCT_MANIFEST_RSA2048_KEY1.modulus()
        ));
    }

    #[test]
    fn product_artifact_has_a_real_signature_and_meets_the_floor() {
        let artifact = decode_hex(PRODUCT);
        let verified = verify_signed_service_manifest(
            &artifact,
            &PRODUCT_MANIFEST_RSA2048_KEY1,
            PRODUCT_MANIFEST_ROLLBACK_FLOOR,
        )
        .unwrap();
        assert_eq!(verified.key_id(), 1);
        assert_eq!(verified.rollback_index(), 2);
        assert_eq!(verified.manifest().generation(), 2);
        assert_eq!(verified.manifest().service_count(), 5);
        assert_eq!(verified.manifest().dependency_count(), 4);
        assert_eq!(
            hex(verified.signed_region_sha256()),
            "99c3865eb64c5314c96ee87318924c9dd29a4bf55fdb571499d36db9cb2f7bc3"
        );
        assert_eq!(
            hex(&sha256(PRODUCT_MANIFEST_RSA2048_KEY1.modulus())),
            "6249c4c758ecd9117cd3799d705a371fac64a210fa464c0e64007fccc30c7d6c"
        );
    }

    #[test]
    fn persistent_floor_artifacts_share_the_second_fixture_anchor() {
        let current = decode_hex(PERSISTENT_PRODUCT);
        let verified = verify_signed_service_manifest(
            &current,
            &PERSISTENT_MANIFEST_RSA2048_KEY2,
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        )
        .unwrap();
        assert_eq!(verified.key_id(), 2);
        assert_eq!(verified.rollback_index(), 3);
        assert_eq!(verified.manifest().generation(), 3);
        assert_eq!(
            hex(verified.signed_region_sha256()),
            "76970b66ada0eab016c7d04af345c0b0a99d0bd9c9dc4e50e6df596cb795143a"
        );
        assert_eq!(
            hex(&sha256(PERSISTENT_MANIFEST_RSA2048_KEY2.modulus())),
            "a050397ce65d2a9f46bb65d9220b56c02984b10dcecd39c530282190ca676c37"
        );

        let old = decode_hex(PERSISTENT_OLD);
        let old_verified = verify_signed_service_manifest(
            &old,
            &PERSISTENT_MANIFEST_RSA2048_KEY2,
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        )
        .unwrap();
        assert_eq!(old_verified.key_id(), 2);
        assert_eq!(old_verified.rollback_index(), 2);
        assert_eq!(old_verified.manifest().generation(), 2);
        assert_eq!(
            hex(old_verified.signed_region_sha256()),
            "30701a3a6281872c489a9569b77a32dcebafa4c64dc8f19ebb5dc0c0b3d8d3df"
        );
    }

    #[test]
    fn rotation_keyring_selects_both_successor_keys_and_has_a_stable_policy_digest() {
        assert_eq!(
            rsa2048_keyring_sha256(&KEY_ROTATION_MANIFEST_KEYRING),
            Ok(KEY_ROTATION_POLICY_SHA256)
        );
        assert_eq!(
            hex(&sha256(KEY_ROTATION_MANIFEST_RSA2048_KEY3.modulus())),
            "c36a7d09c6eabeef6d307827ca0d66a91ffb50396904f7a1f677ae41720c3c9e"
        );
        assert_eq!(
            hex(&sha256(KEY_ROTATION_MANIFEST_RSA2048_KEY4.modulus())),
            "36b7c88dc40ba32c77729d04ac7ceb90a21691d39abe94b17b12b1242f33c52f"
        );

        for (source, key_id, generation) in [
            (KEY3_TRANSITION, KEY_ROTATION_TRANSITION_KEY_ID, 4),
            (KEY4_PRODUCT, KEY_ROTATION_ACTIVE_KEY_ID, 5),
            (KEY3_RETIRED_HIGH_INDEX, KEY_ROTATION_TRANSITION_KEY_ID, 6),
        ] {
            let artifact = decode_hex(source);
            let verified = verify_signed_service_manifest_with_keyring(
                &artifact,
                &KEY_ROTATION_MANIFEST_KEYRING,
                PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
            )
            .unwrap();
            assert_eq!(verified.key_id(), key_id);
            assert_eq!(verified.rollback_index(), generation);
            assert_eq!(verified.manifest().generation(), generation);
        }
    }

    #[test]
    fn rotation_keyring_rejects_bad_signatures_unknown_ids_and_invalid_policy_sets() {
        let bad = decode_hex(KEY4_BAD_SIGNATURE);
        assert_eq!(
            verify_signed_service_manifest_with_keyring(
                &bad,
                &KEY_ROTATION_MANIFEST_KEYRING,
                PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Signature)
        );

        let mut unknown = decode_hex(KEY4_PRODUCT);
        unknown[KEY_ID_OFFSET] = 9;
        assert_eq!(
            verify_signed_service_manifest_with_keyring(
                &unknown,
                &KEY_ROTATION_MANIFEST_KEYRING,
                PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::UnknownKeyringKey { actual: 9 })
        );
        assert_eq!(
            rsa2048_keyring_sha256(&[]),
            Err(SignedManifestError::EmptyTrustAnchorSet)
        );
        let duplicate: [Rsa2048PublicKey; 2] = [
            KEY_ROTATION_MANIFEST_RSA2048_KEY3,
            KEY_ROTATION_MANIFEST_RSA2048_KEY3,
        ];
        assert_eq!(
            rsa2048_keyring_sha256(&duplicate),
            Err(SignedManifestError::DuplicateTrustAnchor(
                KEY_ROTATION_TRANSITION_KEY_ID
            ))
        );
    }

    #[test]
    fn signature_mutations_fail_closed() {
        let artifact = decode_hex(BAD_SIGNATURE);
        assert_eq!(
            verify_signed_service_manifest(
                &artifact,
                &PRODUCT_MANIFEST_RSA2048_KEY1,
                PRODUCT_MANIFEST_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Signature)
        );

        let mut signed_mutation = decode_hex(PRODUCT);
        signed_mutation[64] ^= 1;
        assert_eq!(
            verify_signed_service_manifest(
                &signed_mutation,
                &PRODUCT_MANIFEST_RSA2048_KEY1,
                PRODUCT_MANIFEST_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Signature)
        );
        let mut signature_mutation = decode_hex(PRODUCT);
        signature_mutation[SIGNED_SERVICE_MANIFEST_SIGNED_SIZE] ^= 1;
        assert_eq!(
            verify_signed_service_manifest(
                &signature_mutation,
                &PRODUCT_MANIFEST_RSA2048_KEY1,
                PRODUCT_MANIFEST_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Signature)
        );

        let persistent_bad_signature = decode_hex(PERSISTENT_BAD_SIGNATURE);
        assert_eq!(
            verify_signed_service_manifest(
                &persistent_bad_signature,
                &PERSISTENT_MANIFEST_RSA2048_KEY2,
                PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Signature)
        );
    }

    #[test]
    fn valid_old_signature_is_rejected_by_the_rollback_floor() {
        let artifact = decode_hex(ROLLBACK);
        assert_eq!(
            verify_signed_service_manifest(
                &artifact,
                &PRODUCT_MANIFEST_RSA2048_KEY1,
                PRODUCT_MANIFEST_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Rollback {
                actual: 1,
                minimum: 2,
            })
        );
    }

    #[test]
    fn strict_envelope_fields_are_rejected_before_use() {
        let base = decode_hex(PRODUCT);
        for (offset, value, expected) in [
            (0, b'X', SignedManifestError::BadMagic),
            (4, 2, SignedManifestError::UnsupportedVersion(2)),
            (5, 2, SignedManifestError::UnsupportedAlgorithm(2)),
            (
                6,
                2,
                SignedManifestError::UnknownKey {
                    actual: 2,
                    expected: 1,
                },
            ),
            (7, 1, SignedManifestError::NonZeroFlags(1)),
            (28, 1, SignedManifestError::NonZeroReserved),
        ] {
            let mut artifact = base.clone();
            artifact[offset] = value;
            assert_eq!(
                verify_signed_service_manifest(
                    &artifact,
                    &PRODUCT_MANIFEST_RSA2048_KEY1,
                    PRODUCT_MANIFEST_ROLLBACK_FLOOR,
                ),
                Err(expected),
                "offset {offset}"
            );
        }
        assert!(matches!(
            verify_signed_service_manifest(
                &base[..base.len() - 1],
                &PRODUCT_MANIFEST_RSA2048_KEY1,
                PRODUCT_MANIFEST_ROLLBACK_FLOOR,
            ),
            Err(SignedManifestError::Length { .. })
        ));
    }

    fn decode_hex(source: &str) -> Vec<u8> {
        let digits: Vec<_> = source
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
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("invalid fixture hex"),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        use core::fmt::Write;

        let mut output = String::new();
        for byte in bytes {
            write!(&mut output, "{byte:02x}").unwrap();
        }
        output
    }
}
