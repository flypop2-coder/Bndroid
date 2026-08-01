//! Strict, allocation-free APK Signature Scheme v2 verification.
//!
//! This is deliberately not a general Android APK verifier. It admits one
//! signer, one X.509 certificate, one RSA-2048/65537 public key, and exactly
//! signature algorithm `0x0103` (RSA PKCS#1 v1.5 with SHA-256). Unknown APK
//! Signing Block pairs and signed attributes fail closed; the zero-filled
//! `0x42726577` alignment padding pair is the only exception.

use bndr_sm::verified_manifest::{Sha256, rsa2048_pkcs1_v15_sha256_verify, sha256};

use crate::MAX_APK_BYTES;

pub const APK_SIGNATURE_SCHEME_V2_BLOCK_ID: u32 = 0x7109_871a;
pub const APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID: u32 = 0x0103;

const VERITY_PADDING_BLOCK_ID: u32 = 0x4272_6577;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const EOCD_FIXED_SIZE: usize = 22;
const MAX_COMMENT_BYTES: usize = u16::MAX as usize;
const CENTRAL_DIRECTORY_HEADER_SIZE: usize = 46;
const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
const MAX_APK_ENTRIES: u16 = 32;
const APK_SIGNING_BLOCK_FOOTER_SIZE: usize = 24;
const APK_SIGNING_BLOCK_MIN_SIZE_FIELD: u64 = APK_SIGNING_BLOCK_FOOTER_SIZE as u64;
const APK_SIGNING_BLOCK_MAGIC: [u8; 16] = *b"APK Sig Block 42";
const CONTENT_DIGEST_CHUNK_BYTES: usize = 1024 * 1024;
const RSA_SIGNATURE_BYTES: usize = 256;
const SHA256_BYTES: usize = 32;
const RSA_ENCRYPTION_OID: [u8; 9] = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

/// Cryptographic identity and content binding from one accepted APK signer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApkSignerInfo {
    /// SHA-256 of the exact DER leaf certificate stored in the v2 block.
    pub certificate_sha256: [u8; SHA256_BYTES],
    /// SHA-256 of the exact DER SubjectPublicKeyInfo signer record.
    pub public_key_sha256: [u8; SHA256_BYTES],
    /// APK v2 chunked SHA-256 digest authenticated by the signer.
    pub content_digest_sha256: [u8; SHA256_BYTES],
}

/// Failure from the deliberately bounded APK v2 signature profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApkSignatureError {
    ApkTooLarge,
    ZipEndRecordMissing,
    ZipEndRecordInvalid,
    Zip64Unsupported,
    CentralDirectoryInvalid,
    V1SignatureUnsupported,
    SigningBlockMissing,
    SigningBlockMalformed,
    SigningBlockSizeMismatch,
    DuplicateV2Block,
    DuplicatePaddingBlock,
    UnknownSigningBlockPair(u32),
    NonZeroPadding,
    SignerContainerMalformed,
    SignerCount,
    SignerMalformed,
    SignedDataMalformed,
    SignedDataExtensionUnsupported,
    AdditionalAttributesUnsupported,
    DigestRecordCount,
    DigestRecordMalformed,
    SignatureRecordCount,
    SignatureRecordMalformed,
    CertificateCount,
    CertificateRecordMalformed,
    UnsupportedAlgorithm(u32),
    DigestLength,
    SignatureLength,
    PublicKeyDer,
    CertificateDer,
    PublicKeyMismatch,
    SignatureMismatch,
    ContentDigestMismatch,
}

#[derive(Clone, Copy)]
struct ApkLayout {
    eocd: usize,
    central_directory: usize,
    signing_block: usize,
}

#[derive(Clone, Copy)]
struct SignerRecord<'a> {
    signed_data: &'a [u8],
    expected_content_digest: &'a [u8],
    signature: &'a [u8],
    certificate: &'a [u8],
    public_key: &'a [u8],
}

/// Verifies one complete APK against AndroidBox's bounded v2-only profile.
///
/// A successful result proves the v2 signer and the standard three-section APK
/// content digest. It does not install the APK, authorize permissions, validate
/// certificate time or trust chains, or make the APK generally executable.
pub fn verify_apk_v2(apk: &[u8]) -> Result<ApkSignerInfo, ApkSignatureError> {
    let layout = locate_apk_layout(apk)?;
    let v2_block = find_v2_block(apk, layout)?;
    let signer = parse_signer(v2_block)?;
    let modulus = parse_rsa2048_subject_public_key_info(signer.public_key)?;
    let certificate_public_key = certificate_subject_public_key_info(signer.certificate)?;
    if certificate_public_key != signer.public_key {
        return Err(ApkSignatureError::PublicKeyMismatch);
    }

    let signed_data_digest = sha256(signer.signed_data);
    if !rsa2048_pkcs1_v15_sha256_verify(signer.signature, &signed_data_digest, &modulus) {
        return Err(ApkSignatureError::SignatureMismatch);
    }

    let actual_content_digest = apk_content_digest(apk, layout);
    if signer.expected_content_digest != actual_content_digest {
        return Err(ApkSignatureError::ContentDigestMismatch);
    }

    Ok(ApkSignerInfo {
        certificate_sha256: sha256(signer.certificate),
        public_key_sha256: sha256(signer.public_key),
        content_digest_sha256: actual_content_digest,
    })
}

fn locate_apk_layout(apk: &[u8]) -> Result<ApkLayout, ApkSignatureError> {
    if apk.len() > MAX_APK_BYTES {
        return Err(ApkSignatureError::ApkTooLarge);
    }
    let eocd = find_eocd(apk)?;
    if eocd >= 20
        && read_u32(apk, eocd - 20).ok_or(ApkSignatureError::ZipEndRecordInvalid)?
            == ZIP64_LOCATOR_SIGNATURE
    {
        return Err(ApkSignatureError::Zip64Unsupported);
    }

    let disk = read_u16(apk, eocd + 4).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    let central_disk = read_u16(apk, eocd + 6).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    let entries_on_disk = read_u16(apk, eocd + 8).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    let entries_total = read_u16(apk, eocd + 10).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    let central_size = read_u32(apk, eocd + 12).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    let central_offset = read_u32(apk, eocd + 16).ok_or(ApkSignatureError::ZipEndRecordInvalid)?;
    if disk != 0
        || central_disk != 0
        || entries_on_disk != entries_total
        || entries_total == 0
        || entries_total > MAX_APK_ENTRIES
    {
        return Err(ApkSignatureError::ZipEndRecordInvalid);
    }
    if entries_total == u16::MAX || central_size == u32::MAX || central_offset == u32::MAX {
        return Err(ApkSignatureError::Zip64Unsupported);
    }
    let central_directory =
        usize::try_from(central_offset).map_err(|_| ApkSignatureError::CentralDirectoryInvalid)?;
    let central_size =
        usize::try_from(central_size).map_err(|_| ApkSignatureError::CentralDirectoryInvalid)?;
    if central_directory
        .checked_add(central_size)
        .ok_or(ApkSignatureError::CentralDirectoryInvalid)?
        != eocd
    {
        return Err(ApkSignatureError::CentralDirectoryInvalid);
    }
    validate_central_directory(apk, central_directory, eocd, entries_total)?;
    if central_directory < APK_SIGNING_BLOCK_FOOTER_SIZE {
        return Err(ApkSignatureError::SigningBlockMissing);
    }

    let footer = central_directory - APK_SIGNING_BLOCK_FOOTER_SIZE;
    if apk.get(footer + 8..central_directory) != Some(APK_SIGNING_BLOCK_MAGIC.as_slice()) {
        return Err(ApkSignatureError::SigningBlockMissing);
    }
    let size = read_u64(apk, footer).ok_or(ApkSignatureError::SigningBlockMalformed)?;
    if size < APK_SIGNING_BLOCK_MIN_SIZE_FIELD {
        return Err(ApkSignatureError::SigningBlockMalformed);
    }
    let total_size = usize::try_from(
        size.checked_add(8)
            .ok_or(ApkSignatureError::SigningBlockMalformed)?,
    )
    .map_err(|_| ApkSignatureError::SigningBlockMalformed)?;
    let signing_block = central_directory
        .checked_sub(total_size)
        .ok_or(ApkSignatureError::SigningBlockMalformed)?;
    if read_u64(apk, signing_block).ok_or(ApkSignatureError::SigningBlockMalformed)? != size {
        return Err(ApkSignatureError::SigningBlockSizeMismatch);
    }

    Ok(ApkLayout {
        eocd,
        central_directory,
        signing_block,
    })
}

fn validate_central_directory(
    apk: &[u8],
    start: usize,
    end: usize,
    entry_count: u16,
) -> Result<(), ApkSignatureError> {
    let mut cursor = start;
    for _ in 0..entry_count {
        let fixed_end = cursor
            .checked_add(CENTRAL_DIRECTORY_HEADER_SIZE)
            .ok_or(ApkSignatureError::CentralDirectoryInvalid)?;
        if fixed_end > end
            || read_u32(apk, cursor) != Some(CENTRAL_DIRECTORY_SIGNATURE)
            || read_u32(apk, cursor + 20) == Some(u32::MAX)
            || read_u32(apk, cursor + 24) == Some(u32::MAX)
            || read_u32(apk, cursor + 42) == Some(u32::MAX)
            || read_u16(apk, cursor + 34) != Some(0)
        {
            return Err(ApkSignatureError::CentralDirectoryInvalid);
        }
        let name_len = usize::from(
            read_u16(apk, cursor + 28).ok_or(ApkSignatureError::CentralDirectoryInvalid)?,
        );
        let extra_len = usize::from(
            read_u16(apk, cursor + 30).ok_or(ApkSignatureError::CentralDirectoryInvalid)?,
        );
        let comment_len = usize::from(
            read_u16(apk, cursor + 32).ok_or(ApkSignatureError::CentralDirectoryInvalid)?,
        );
        if name_len == 0 {
            return Err(ApkSignatureError::CentralDirectoryInvalid);
        }
        let name_end = fixed_end
            .checked_add(name_len)
            .ok_or(ApkSignatureError::CentralDirectoryInvalid)?;
        let entry_end = name_end
            .checked_add(extra_len)
            .and_then(|offset| offset.checked_add(comment_len))
            .ok_or(ApkSignatureError::CentralDirectoryInvalid)?;
        if entry_end > end {
            return Err(ApkSignatureError::CentralDirectoryInvalid);
        }
        let name = &apk[fixed_end..name_end];
        if name.len() >= b"META-INF/".len()
            && name[..b"META-INF/".len()].eq_ignore_ascii_case(b"META-INF/")
        {
            return Err(ApkSignatureError::V1SignatureUnsupported);
        }
        cursor = entry_end;
    }
    if cursor != end {
        return Err(ApkSignatureError::CentralDirectoryInvalid);
    }
    Ok(())
}

fn find_eocd(bytes: &[u8]) -> Result<usize, ApkSignatureError> {
    if bytes.len() < EOCD_FIXED_SIZE {
        return Err(ApkSignatureError::ZipEndRecordMissing);
    }
    let lower = bytes
        .len()
        .saturating_sub(EOCD_FIXED_SIZE + MAX_COMMENT_BYTES);
    let mut cursor = bytes.len() - EOCD_FIXED_SIZE;
    loop {
        if read_u32(bytes, cursor) == Some(EOCD_SIGNATURE) {
            let comment_len = usize::from(
                read_u16(bytes, cursor + 20).ok_or(ApkSignatureError::ZipEndRecordInvalid)?,
            );
            if cursor
                .checked_add(EOCD_FIXED_SIZE)
                .and_then(|end| end.checked_add(comment_len))
                == Some(bytes.len())
            {
                return Ok(cursor);
            }
        }
        if cursor == lower {
            break;
        }
        cursor -= 1;
    }
    Err(ApkSignatureError::ZipEndRecordMissing)
}

fn find_v2_block(apk: &[u8], layout: ApkLayout) -> Result<&[u8], ApkSignatureError> {
    let pairs_start = layout
        .signing_block
        .checked_add(8)
        .ok_or(ApkSignatureError::SigningBlockMalformed)?;
    let pairs_end = layout
        .central_directory
        .checked_sub(APK_SIGNING_BLOCK_FOOTER_SIZE)
        .ok_or(ApkSignatureError::SigningBlockMalformed)?;
    if pairs_start > pairs_end {
        return Err(ApkSignatureError::SigningBlockMalformed);
    }

    let mut cursor = pairs_start;
    let mut v2 = None;
    let mut padding_seen = false;
    while cursor < pairs_end {
        let pair_size = read_u64(apk, cursor).ok_or(ApkSignatureError::SigningBlockMalformed)?;
        if pair_size < 4 {
            return Err(ApkSignatureError::SigningBlockMalformed);
        }
        let pair_size =
            usize::try_from(pair_size).map_err(|_| ApkSignatureError::SigningBlockMalformed)?;
        let value_start = cursor
            .checked_add(12)
            .ok_or(ApkSignatureError::SigningBlockMalformed)?;
        let pair_end = cursor
            .checked_add(8)
            .and_then(|offset| offset.checked_add(pair_size))
            .ok_or(ApkSignatureError::SigningBlockMalformed)?;
        if value_start > pair_end || pair_end > pairs_end {
            return Err(ApkSignatureError::SigningBlockMalformed);
        }
        let id = read_u32(apk, cursor + 8).ok_or(ApkSignatureError::SigningBlockMalformed)?;
        let value = &apk[value_start..pair_end];
        match id {
            APK_SIGNATURE_SCHEME_V2_BLOCK_ID => {
                if v2.replace(value).is_some() {
                    return Err(ApkSignatureError::DuplicateV2Block);
                }
            }
            VERITY_PADDING_BLOCK_ID => {
                if padding_seen {
                    return Err(ApkSignatureError::DuplicatePaddingBlock);
                }
                if value.iter().any(|byte| *byte != 0) {
                    return Err(ApkSignatureError::NonZeroPadding);
                }
                padding_seen = true;
            }
            other => return Err(ApkSignatureError::UnknownSigningBlockPair(other)),
        }
        cursor = pair_end;
    }
    if cursor != pairs_end {
        return Err(ApkSignatureError::SigningBlockMalformed);
    }
    v2.ok_or(ApkSignatureError::SigningBlockMissing)
}

fn parse_signer(v2_block: &[u8]) -> Result<SignerRecord<'_>, ApkSignatureError> {
    let mut v2_cursor = 0;
    let signers = take_length_prefixed(v2_block, &mut v2_cursor)
        .ok_or(ApkSignatureError::SignerContainerMalformed)?;
    if v2_cursor != v2_block.len() {
        return Err(ApkSignatureError::SignerContainerMalformed);
    }

    let mut signers_cursor = 0;
    let signer =
        take_length_prefixed(signers, &mut signers_cursor).ok_or(ApkSignatureError::SignerCount)?;
    if signer.is_empty() || signers_cursor != signers.len() {
        return Err(ApkSignatureError::SignerCount);
    }

    let mut signer_cursor = 0;
    let signed_data = take_length_prefixed(signer, &mut signer_cursor)
        .ok_or(ApkSignatureError::SignerMalformed)?;
    let signatures = take_length_prefixed(signer, &mut signer_cursor)
        .ok_or(ApkSignatureError::SignerMalformed)?;
    let public_key = take_length_prefixed(signer, &mut signer_cursor)
        .ok_or(ApkSignatureError::SignerMalformed)?;
    if signer_cursor != signer.len() || signed_data.is_empty() || public_key.is_empty() {
        return Err(ApkSignatureError::SignerMalformed);
    }

    let (expected_content_digest, certificate) = parse_signed_data(signed_data)?;
    let signature = parse_signature_record(signatures)?;
    Ok(SignerRecord {
        signed_data,
        expected_content_digest,
        signature,
        certificate,
        public_key,
    })
}

fn parse_signed_data(signed_data: &[u8]) -> Result<(&[u8], &[u8]), ApkSignatureError> {
    let mut cursor = 0;
    let digests = take_length_prefixed(signed_data, &mut cursor)
        .ok_or(ApkSignatureError::SignedDataMalformed)?;
    let certificates = take_length_prefixed(signed_data, &mut cursor)
        .ok_or(ApkSignatureError::SignedDataMalformed)?;
    let additional_attributes = take_length_prefixed(signed_data, &mut cursor)
        .ok_or(ApkSignatureError::SignedDataMalformed)?;
    if !additional_attributes.is_empty() {
        return Err(ApkSignatureError::AdditionalAttributesUnsupported);
    }
    if cursor < signed_data.len() {
        let reserved = take_length_prefixed(signed_data, &mut cursor)
            .ok_or(ApkSignatureError::SignedDataExtensionUnsupported)?;
        if !reserved.is_empty() || cursor != signed_data.len() {
            return Err(ApkSignatureError::SignedDataExtensionUnsupported);
        }
    }
    if cursor != signed_data.len() {
        return Err(ApkSignatureError::SignedDataMalformed);
    }

    Ok((
        parse_digest_record(digests)?,
        parse_certificate_record(certificates)?,
    ))
}

fn parse_digest_record(digests: &[u8]) -> Result<&[u8], ApkSignatureError> {
    let mut cursor = 0;
    let record =
        take_length_prefixed(digests, &mut cursor).ok_or(ApkSignatureError::DigestRecordCount)?;
    if record.is_empty() || cursor != digests.len() {
        return Err(ApkSignatureError::DigestRecordCount);
    }
    let algorithm = read_u32(record, 0).ok_or(ApkSignatureError::DigestRecordMalformed)?;
    if algorithm != APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID {
        return Err(ApkSignatureError::UnsupportedAlgorithm(algorithm));
    }
    let mut record_cursor = 4;
    let digest = take_length_prefixed(record, &mut record_cursor)
        .ok_or(ApkSignatureError::DigestRecordMalformed)?;
    if record_cursor != record.len() {
        return Err(ApkSignatureError::DigestRecordMalformed);
    }
    if digest.len() != SHA256_BYTES {
        return Err(ApkSignatureError::DigestLength);
    }
    Ok(digest)
}

fn parse_signature_record(signatures: &[u8]) -> Result<&[u8], ApkSignatureError> {
    let mut cursor = 0;
    let record = take_length_prefixed(signatures, &mut cursor)
        .ok_or(ApkSignatureError::SignatureRecordCount)?;
    if record.is_empty() || cursor != signatures.len() {
        return Err(ApkSignatureError::SignatureRecordCount);
    }
    let algorithm = read_u32(record, 0).ok_or(ApkSignatureError::SignatureRecordMalformed)?;
    if algorithm != APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID {
        return Err(ApkSignatureError::UnsupportedAlgorithm(algorithm));
    }
    let mut record_cursor = 4;
    let signature = take_length_prefixed(record, &mut record_cursor)
        .ok_or(ApkSignatureError::SignatureRecordMalformed)?;
    if record_cursor != record.len() {
        return Err(ApkSignatureError::SignatureRecordMalformed);
    }
    if signature.len() != RSA_SIGNATURE_BYTES {
        return Err(ApkSignatureError::SignatureLength);
    }
    Ok(signature)
}

fn parse_certificate_record(certificates: &[u8]) -> Result<&[u8], ApkSignatureError> {
    let mut cursor = 0;
    let certificate = take_length_prefixed(certificates, &mut cursor)
        .ok_or(ApkSignatureError::CertificateCount)?;
    if certificate.is_empty() || cursor != certificates.len() {
        return Err(ApkSignatureError::CertificateCount);
    }
    Ok(certificate)
}

fn take_length_prefixed<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let length = usize::try_from(read_u32(bytes, *cursor)?).ok()?;
    let start = cursor.checked_add(4)?;
    let end = start.checked_add(length)?;
    let value = bytes.get(start..end)?;
    *cursor = end;
    Some(value)
}

fn apk_content_digest(apk: &[u8], layout: ApkLayout) -> [u8; SHA256_BYTES] {
    let first = &apk[..layout.signing_block];
    let second = &apk[layout.central_directory..layout.eocd];
    let third = &apk[layout.eocd..];
    let chunk_count = section_chunk_count(first.len())
        + section_chunk_count(second.len())
        + section_chunk_count(third.len());

    let mut digest = Sha256::new();
    digest.update(&[0x5a]);
    digest.update(&(chunk_count as u32).to_le_bytes());
    append_section_chunk_digests(&mut digest, first);
    append_section_chunk_digests(&mut digest, second);
    append_patched_eocd_digest(&mut digest, third, layout.signing_block);
    digest.finalize()
}

fn section_chunk_count(length: usize) -> usize {
    length.div_ceil(CONTENT_DIGEST_CHUNK_BYTES)
}

fn append_section_chunk_digests(output: &mut Sha256, section: &[u8]) {
    for chunk in section.chunks(CONTENT_DIGEST_CHUNK_BYTES) {
        let mut digest = Sha256::new();
        digest.update(&[0xa5]);
        digest.update(&(chunk.len() as u32).to_le_bytes());
        digest.update(chunk);
        output.update(&digest.finalize());
    }
}

fn append_patched_eocd_digest(output: &mut Sha256, eocd: &[u8], signing_block: usize) {
    debug_assert!(eocd.len() >= EOCD_FIXED_SIZE);
    debug_assert!(eocd.len() <= CONTENT_DIGEST_CHUNK_BYTES);
    let mut digest = Sha256::new();
    digest.update(&[0xa5]);
    digest.update(&(eocd.len() as u32).to_le_bytes());
    digest.update(&eocd[..16]);
    digest.update(&(signing_block as u32).to_le_bytes());
    digest.update(&eocd[20..]);
    output.update(&digest.finalize());
}

#[derive(Clone, Copy)]
struct DerValue<'a> {
    tag: u8,
    full: &'a [u8],
    value: &'a [u8],
}

fn certificate_subject_public_key_info(certificate: &[u8]) -> Result<&[u8], ApkSignatureError> {
    let mut certificate_cursor = 0;
    let outer =
        take_der(certificate, &mut certificate_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    if outer.tag != 0x30 || certificate_cursor != certificate.len() {
        return Err(ApkSignatureError::CertificateDer);
    }

    let mut outer_cursor = 0;
    let tbs = take_der(outer.value, &mut outer_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let outer_algorithm =
        take_der(outer.value, &mut outer_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let certificate_signature =
        take_der(outer.value, &mut outer_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    if tbs.tag != 0x30
        || outer_algorithm.tag != 0x30
        || certificate_signature.tag != 0x03
        || certificate_signature.value.len() < 2
        || certificate_signature.value[0] != 0
        || outer_cursor != outer.value.len()
    {
        return Err(ApkSignatureError::CertificateDer);
    }

    let mut tbs_cursor = 0;
    if tbs.value.first() == Some(&0xa0) {
        let version =
            take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
        let mut version_cursor = 0;
        let version_number = take_der(version.value, &mut version_cursor)
            .ok_or(ApkSignatureError::CertificateDer)?;
        if version_number.tag != 0x02
            || version_number.value.len() != 1
            || version_number.value[0] > 2
            || version_cursor != version.value.len()
        {
            return Err(ApkSignatureError::CertificateDer);
        }
    }
    let serial = take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let tbs_algorithm =
        take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let issuer = take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let validity = take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let subject = take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    let subject_public_key =
        take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
    if serial.tag != 0x02
        || !is_canonical_positive_integer(serial.value)
        || tbs_algorithm.tag != 0x30
        || issuer.tag != 0x30
        || validity.tag != 0x30
        || subject.tag != 0x30
        || subject_public_key.tag != 0x30
        || tbs_algorithm.full != outer_algorithm.full
    {
        return Err(ApkSignatureError::CertificateDer);
    }

    let mut last_optional_rank = 0;
    while tbs_cursor < tbs.value.len() {
        let optional =
            take_der(tbs.value, &mut tbs_cursor).ok_or(ApkSignatureError::CertificateDer)?;
        let rank = match optional.tag {
            0x81 => 1,
            0x82 => 2,
            0xa3 => 3,
            _ => return Err(ApkSignatureError::CertificateDer),
        };
        if rank <= last_optional_rank {
            return Err(ApkSignatureError::CertificateDer);
        }
        last_optional_rank = rank;
    }
    Ok(subject_public_key.full)
}

fn parse_rsa2048_subject_public_key_info(
    public_key: &[u8],
) -> Result<[u8; RSA_SIGNATURE_BYTES], ApkSignatureError> {
    let mut cursor = 0;
    let sequence = take_der(public_key, &mut cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    if sequence.tag != 0x30 || cursor != public_key.len() {
        return Err(ApkSignatureError::PublicKeyDer);
    }

    let mut sequence_cursor = 0;
    let algorithm =
        take_der(sequence.value, &mut sequence_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    let key_bits =
        take_der(sequence.value, &mut sequence_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    if algorithm.tag != 0x30
        || key_bits.tag != 0x03
        || key_bits.value.first() != Some(&0)
        || sequence_cursor != sequence.value.len()
    {
        return Err(ApkSignatureError::PublicKeyDer);
    }

    let mut algorithm_cursor = 0;
    let oid =
        take_der(algorithm.value, &mut algorithm_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    let parameters =
        take_der(algorithm.value, &mut algorithm_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    if oid.tag != 0x06
        || oid.value != RSA_ENCRYPTION_OID
        || parameters.tag != 0x05
        || !parameters.value.is_empty()
        || algorithm_cursor != algorithm.value.len()
    {
        return Err(ApkSignatureError::PublicKeyDer);
    }

    let mut bits_cursor = 1;
    let rsa = take_der(key_bits.value, &mut bits_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    if rsa.tag != 0x30 || bits_cursor != key_bits.value.len() {
        return Err(ApkSignatureError::PublicKeyDer);
    }
    let mut rsa_cursor = 0;
    let modulus = take_der(rsa.value, &mut rsa_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    let exponent = take_der(rsa.value, &mut rsa_cursor).ok_or(ApkSignatureError::PublicKeyDer)?;
    if modulus.tag != 0x02
        || modulus.value.len() != RSA_SIGNATURE_BYTES + 1
        || modulus.value[0] != 0
        || modulus.value[1] & 0x80 == 0
        || exponent.tag != 0x02
        || exponent.value != [0x01, 0x00, 0x01]
        || rsa_cursor != rsa.value.len()
    {
        return Err(ApkSignatureError::PublicKeyDer);
    }
    let mut value = [0; RSA_SIGNATURE_BYTES];
    value.copy_from_slice(&modulus.value[1..]);
    if value[RSA_SIGNATURE_BYTES - 1] & 1 == 0 {
        return Err(ApkSignatureError::PublicKeyDer);
    }
    Ok(value)
}

fn is_canonical_positive_integer(value: &[u8]) -> bool {
    !value.is_empty()
        && value[0] & 0x80 == 0
        && (value.len() == 1 || value[0] != 0 || value[1] & 0x80 != 0)
}

fn take_der<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<DerValue<'a>> {
    let start = *cursor;
    let tag = *bytes.get(start)?;
    if tag & 0x1f == 0x1f {
        return None;
    }
    let first_length = *bytes.get(start.checked_add(1)?)?;
    let mut header_len = 2usize;
    let value_len = if first_length & 0x80 == 0 {
        usize::from(first_length)
    } else {
        let length_bytes = usize::from(first_length & 0x7f);
        if length_bytes == 0 || length_bytes > 4 {
            return None;
        }
        let length_start = start.checked_add(2)?;
        let length_end = length_start.checked_add(length_bytes)?;
        let encoded = bytes.get(length_start..length_end)?;
        if encoded.first() == Some(&0) {
            return None;
        }
        let mut length = 0usize;
        for byte in encoded {
            length = length.checked_mul(256)?.checked_add(usize::from(*byte))?;
        }
        if length < 128 {
            return None;
        }
        header_len = header_len.checked_add(length_bytes)?;
        length
    };
    let value_start = start.checked_add(header_len)?;
    let end = value_start.checked_add(value_len)?;
    let full = bytes.get(start..end)?;
    let value = bytes.get(value_start..end)?;
    *cursor = end;
    Some(DerValue { tag, full, value })
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let value = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let value = bytes.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use super::{
        APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID, APK_SIGNATURE_SCHEME_V2_BLOCK_ID, ApkSignatureError,
        find_v2_block, locate_apk_layout, parse_signed_data, parse_signer, verify_apk_v2,
    };

    const SIGNED_RESOURCE_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-resource-demo/androidbox-resource-demo.apk");
    const UNSIGNED_ACTIVITY_APK: &[u8] =
        include_bytes!("../../../fixtures/androidbox-demo/androidbox-demo.apk");

    #[test]
    fn repository_resource_fixture_has_one_real_v2_signer() {
        let signer = verify_apk_v2(SIGNED_RESOURCE_APK).expect("signed resource fixture");
        assert_eq!(
            signer.certificate_sha256,
            [
                0xe7, 0x41, 0x2e, 0x1c, 0xc0, 0xff, 0xbd, 0x21, 0x00, 0x0e, 0xce, 0x51, 0x76, 0xd0,
                0x08, 0x37, 0xce, 0x27, 0x6e, 0x42, 0xe6, 0xb5, 0x2b, 0xed, 0x99, 0xcb, 0x5e, 0x62,
                0x85, 0x7b, 0x77, 0xbf,
            ]
        );
        assert_eq!(
            signer.public_key_sha256,
            [
                0xc0, 0x4d, 0xfd, 0x1d, 0x94, 0x65, 0x4a, 0xa6, 0xdc, 0x5c, 0x86, 0x1c, 0x2f, 0x15,
                0xf2, 0x24, 0xd9, 0x72, 0x1c, 0x07, 0xf6, 0xc6, 0xc2, 0xf5, 0x44, 0x7c, 0x7e, 0xfd,
                0xba, 0x05, 0xc2, 0x36,
            ]
        );
        assert_eq!(
            signer.content_digest_sha256,
            [
                0xf9, 0xd6, 0xce, 0x14, 0x3b, 0x11, 0x21, 0x43, 0xae, 0x60, 0xae, 0x6e, 0x26, 0xac,
                0x8e, 0xbb, 0xc9, 0x94, 0x5d, 0xb1, 0x5c, 0x28, 0xb0, 0x0b, 0xaa, 0xdd, 0x14, 0xf6,
                0x37, 0x7e, 0x93, 0xa5,
            ]
        );
    }

    #[test]
    fn unsigned_activity_fixture_remains_unsigned_and_loadable_elsewhere() {
        assert_eq!(
            verify_apk_v2(UNSIGNED_ACTIVITY_APK),
            Err(ApkSignatureError::SigningBlockMissing)
        );
    }

    #[test]
    fn jar_v1_metadata_is_rejected_even_when_a_v2_block_is_present() {
        let layout = locate_apk_layout(SIGNED_RESOURCE_APK).expect("layout");
        let mut apk = SIGNED_RESOURCE_APK.to_vec();
        let first_name = layout.central_directory + super::CENTRAL_DIRECTORY_HEADER_SIZE;
        apk[first_name..first_name + b"META-INF/".len()].copy_from_slice(b"META-INF/");
        assert_eq!(
            verify_apk_v2(&apk),
            Err(ApkSignatureError::V1SignatureUnsupported)
        );
    }

    #[test]
    fn content_signature_and_public_key_mutations_fail_independently() {
        let layout = locate_apk_layout(SIGNED_RESOURCE_APK).expect("layout");
        let v2 = find_v2_block(SIGNED_RESOURCE_APK, layout).expect("v2");
        let signer = parse_signer(v2).expect("signer");

        let mut content = SIGNED_RESOURCE_APK.to_vec();
        content[128] ^= 1;
        assert_eq!(
            verify_apk_v2(&content),
            Err(ApkSignatureError::ContentDigestMismatch)
        );

        let mut signature = SIGNED_RESOURCE_APK.to_vec();
        let signature_offset = subslice_offset(SIGNED_RESOURCE_APK, signer.signature);
        signature[signature_offset] ^= 1;
        assert_eq!(
            verify_apk_v2(&signature),
            Err(ApkSignatureError::SignatureMismatch)
        );

        let mut public_key = SIGNED_RESOURCE_APK.to_vec();
        let public_key_offset = subslice_offset(SIGNED_RESOURCE_APK, signer.public_key);
        let modulus_byte = public_key_offset + signer.public_key.len() - 8;
        public_key[modulus_byte] ^= 2;
        assert_eq!(
            verify_apk_v2(&public_key),
            Err(ApkSignatureError::PublicKeyMismatch)
        );
    }

    #[test]
    fn unknown_pairs_and_nonzero_padding_fail_closed() {
        let layout = locate_apk_layout(SIGNED_RESOURCE_APK).expect("layout");
        let mut unknown = SIGNED_RESOURCE_APK.to_vec();
        let pairs_start = layout.signing_block + 8;
        assert_eq!(
            u32::from_le_bytes(
                unknown[pairs_start + 8..pairs_start + 12]
                    .try_into()
                    .unwrap()
            ),
            APK_SIGNATURE_SCHEME_V2_BLOCK_ID
        );
        let first_pair_size = usize::try_from(u64::from_le_bytes(
            unknown[pairs_start..pairs_start + 8].try_into().unwrap(),
        ))
        .unwrap();
        let padding = pairs_start + 8 + first_pair_size;
        unknown[padding + 8..padding + 12].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
        assert_eq!(
            verify_apk_v2(&unknown),
            Err(ApkSignatureError::UnknownSigningBlockPair(0x1234_5678))
        );

        let mut nonzero = SIGNED_RESOURCE_APK.to_vec();
        nonzero[padding + 12] = 1;
        assert_eq!(
            verify_apk_v2(&nonzero),
            Err(ApkSignatureError::NonZeroPadding)
        );
    }

    #[test]
    fn algorithm_and_sdk36_reserved_field_are_strict() {
        let layout = locate_apk_layout(SIGNED_RESOURCE_APK).expect("layout");
        let v2 = find_v2_block(SIGNED_RESOURCE_APK, layout).expect("v2");
        let signer = parse_signer(v2).expect("signer");

        let mut algorithm = SIGNED_RESOURCE_APK.to_vec();
        let digest_algorithm =
            subslice_offset(SIGNED_RESOURCE_APK, signer.expected_content_digest) - 8;
        assert_eq!(
            u32::from_le_bytes(
                algorithm[digest_algorithm..digest_algorithm + 4]
                    .try_into()
                    .unwrap()
            ),
            APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID
        );
        algorithm[digest_algorithm..digest_algorithm + 4]
            .copy_from_slice(&0x0104_u32.to_le_bytes());
        assert_eq!(
            verify_apk_v2(&algorithm),
            Err(ApkSignatureError::UnsupportedAlgorithm(0x0104))
        );

        let mut extension = SIGNED_RESOURCE_APK.to_vec();
        let signed_data_offset = subslice_offset(SIGNED_RESOURCE_APK, signer.signed_data);
        let extension_length = signed_data_offset + signer.signed_data.len() - 4;
        extension[extension_length] = 1;
        assert_eq!(
            verify_apk_v2(&extension),
            Err(ApkSignatureError::SignedDataExtensionUnsupported)
        );
    }

    #[test]
    fn signed_data_accepts_only_three_fields_or_one_extra_empty_field() {
        let field = |bytes: &[u8]| {
            let mut encoded = Vec::new();
            encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            encoded.extend_from_slice(bytes);
            encoded
        };
        let digest_record = {
            let mut record = Vec::new();
            record.extend_from_slice(&APK_SIGNATURE_RSA_PKCS1_V15_SHA256_ID.to_le_bytes());
            record.extend_from_slice(&field(&[7; 32]));
            field(&record)
        };
        let certificates = field(&[0x30, 0]);
        let mut three = Vec::new();
        three.extend_from_slice(&field(&digest_record));
        three.extend_from_slice(&field(&certificates));
        three.extend_from_slice(&field(&[]));
        assert!(parse_signed_data(&three).is_ok());

        let mut four = three.clone();
        four.extend_from_slice(&field(&[]));
        assert!(parse_signed_data(&four).is_ok());

        let mut nonempty = three;
        nonempty.extend_from_slice(&field(&[1]));
        assert_eq!(
            parse_signed_data(&nonempty),
            Err(ApkSignatureError::SignedDataExtensionUnsupported)
        );
    }

    #[test]
    fn multi_signer_container_is_rejected_before_crypto() {
        let layout = locate_apk_layout(SIGNED_RESOURCE_APK).expect("layout");
        let v2 = find_v2_block(SIGNED_RESOURCE_APK, layout).expect("v2");
        let mut cursor = 0;
        let signers = super::take_length_prefixed(v2, &mut cursor).expect("signers");
        let mut signer_cursor = 0;
        let signer =
            super::take_length_prefixed(signers, &mut signer_cursor).expect("first signer");
        let mut two_signers = Vec::new();
        for _ in 0..2 {
            two_signers.extend_from_slice(&(signer.len() as u32).to_le_bytes());
            two_signers.extend_from_slice(signer);
        }
        let mut block = Vec::new();
        block.extend_from_slice(&(two_signers.len() as u32).to_le_bytes());
        block.extend_from_slice(&two_signers);
        assert!(matches!(
            parse_signer(&block),
            Err(ApkSignatureError::SignerCount)
        ));
    }

    fn subslice_offset(outer: &[u8], inner: &[u8]) -> usize {
        inner.as_ptr() as usize - outer.as_ptr() as usize
    }
}
