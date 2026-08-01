use super::*;

fn request_roundtrip(request: Request) {
    let wire = request.encode();
    assert_eq!(Request::decode(&wire), Ok(request));
    assert_eq!(&wire[REQUEST_RESERVED_RANGE], &[0; 8]);
    assert_eq!(&wire[REQUEST_PADDING_RANGE], &[0; 16]);
}

fn response_roundtrip(response: Response) {
    let wire = response.encode();
    assert_eq!(Response::decode(&wire), Ok(response));
    assert_eq!(&wire[RESPONSE_RESERVED_RANGE], &[0; 3]);
    assert_eq!(&wire[RESPONSE_PADDING_RANGE], &[0; 16]);
}

#[test]
fn stable_constants_and_discriminants() {
    assert_eq!(FRAME_SIZE, 64);
    assert_eq!(ABI_VERSION, 1);
    assert_eq!(REQUEST_MAGIC, *b"BSQ1");
    assert_eq!(RESPONSE_MAGIC, *b"BSP1");
    assert_eq!(REQUEST_PAYLOAD_MAX_BYTES, 4160);
    assert_eq!(Opcode::Bind.raw(), 1);
    assert_eq!(Opcode::Unlink.raw(), 6);
    assert_eq!(Status::Ok.raw(), 0);
    assert_eq!(Status::NoSpace.raw(), 17);
    assert_eq!(Status::RequiresReset.raw(), 21);
    assert!(Status::OutcomeUnknown.is_session_fatal());
    assert!(Status::RequiresReset.is_session_fatal());
    for status in [
        Status::Ok,
        Status::Unavailable,
        Status::PeerClosed,
        Status::DataCorrupt,
        Status::ProtocolError,
    ] {
        assert!(!status.is_session_fatal());
    }
    assert_eq!(ResultKind::None.raw(), 0);
    assert_eq!(ResultKind::EndOfDirectory.raw(), 5);
}

#[test]
fn every_request_opcode_and_replace_mode_roundtrips() {
    let requests = [
        Request::bind(1).unwrap(),
        Request::create_directory(2, 3).unwrap(),
        Request::replace(3, 4, 5, ReplaceCondition::Any).unwrap(),
        Request::replace(4, 4, 0, ReplaceCondition::CreateOnly).unwrap(),
        Request::replace(5, 4, 4096, ReplaceCondition::Exact(17)).unwrap(),
        Request::read(6, 7).unwrap(),
        Request::list(7, 0, 0).unwrap(),
        Request::list(8, 3, DIRECTORY_MAX_ENTRIES).unwrap(),
        Request::unlink(9, 7).unwrap(),
    ];
    for request in requests {
        request_roundtrip(request);
    }
    assert_eq!(requests[2].replace_condition(), Some(ReplaceCondition::Any));
    assert_eq!(
        requests[3].replace_condition(),
        Some(ReplaceCondition::CreateOnly)
    );
    assert_eq!(
        requests[4].replace_condition(),
        Some(ReplaceCondition::Exact(17))
    );
    assert_eq!(requests[5].replace_condition(), None);
}

#[test]
fn request_vmo_splits_exact_path_then_value() {
    let request = Request::replace(7, 14, 5, ReplaceCondition::Exact(9)).unwrap();
    let payload = request.validate_payload(b"settings/themeamber").unwrap();
    assert_eq!(payload.path(), "settings/theme");
    assert_eq!(payload.value(), b"amber");
    assert_eq!(request.payload_length(), 19);
    assert_eq!(
        request.replace_condition(),
        Some(ReplaceCondition::Exact(9))
    );

    assert_eq!(
        request.validate_payload(b"settings/themeambe"),
        Err(PayloadError::LengthMismatch {
            expected: 19,
            actual: 18,
        })
    );
}

#[test]
fn canonical_path_matrix_is_strict_and_list_may_name_root() {
    assert!(
        Request::list(1, 0, 0)
            .unwrap()
            .validate_payload(b"")
            .is_ok()
    );
    assert!(
        Request::read(2, "设置/主题".len())
            .unwrap()
            .validate_payload("设置/主题".as_bytes())
            .is_ok()
    );

    let cases: &[(&[u8], PayloadError)] = &[
        (b"/a", PayloadError::AbsolutePath),
        (b"a/", PayloadError::TrailingSlash),
        (b"a//b", PayloadError::EmptyComponent),
        (b"a/./b", PayloadError::CurrentDirectoryComponent),
        (b"a/../b", PayloadError::ParentDirectoryComponent),
        (b"a/b/c/d/e", PayloadError::TooDeep),
        (b"a\0b", PayloadError::ContainsNul),
        (&[0xff], PayloadError::InvalidUtf8),
    ];
    for (index, (path, expected)) in cases.iter().enumerate() {
        let request = Request::read(10 + index as u64, path.len()).unwrap();
        assert_eq!(request.validate_payload(path), Err(*expected));
    }
}

#[test]
fn request_constructor_rejects_invalid_semantics() {
    assert_eq!(
        Request::bind(0),
        Err(RequestError::TransactionIdMustBeNonZero)
    );
    assert_eq!(
        Request::read(1, PATH_MAX_BYTES + 1),
        Err(RequestError::PathTooLong)
    );
    assert_eq!(Request::read(1, 0), Err(RequestError::PathRequired));
    assert_eq!(
        Request::replace(1, 1, VALUE_MAX_BYTES + 1, ReplaceCondition::Any),
        Err(RequestError::ValueTooLong)
    );
    assert_eq!(
        Request::replace(1, 1, 0, ReplaceCondition::Exact(0)),
        Err(RequestError::ExactGenerationMustBeNonZero)
    );
    assert_eq!(
        Request::list(1, 0, DIRECTORY_MAX_ENTRIES + 1),
        Err(RequestError::CursorOutOfRange)
    );
}

#[test]
fn malformed_request_header_matrix_is_rejected() {
    let valid = Request::replace(9, 4, 5, ReplaceCondition::Exact(7))
        .unwrap()
        .encode();

    let mut wire = valid;
    wire[0] ^= 1;
    assert_eq!(Request::decode(&wire), Err(RequestDecodeError::BadMagic));

    let mut wire = valid;
    write_u16(&mut wire, VERSION_RANGE, ABI_VERSION + 1);
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::UnsupportedVersion(2))
    );

    let mut wire = valid;
    wire[OPCODE_OFFSET] = 0xff;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::UnknownOpcode(0xff))
    );

    let mut wire = valid;
    wire[FLAGS_OFFSET] |= 0x80;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::UnknownFlags(
            REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_EXACT_GENERATION | 0x80
        ))
    );

    let mut wire = valid;
    wire[40] = 1;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::ReservedNonZero { offset: 40 })
    );

    let mut wire = valid;
    wire[63] = 1;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::PaddingNonZero { offset: 63 })
    );
}

#[test]
fn malformed_request_semantic_matrix_is_rejected() {
    let base = Request::read(8, 1).unwrap().encode();
    let cases: &[(usize, u64, RequestError)] = &[
        (8, 0, RequestError::TransactionIdMustBeNonZero),
        (16, 0, RequestError::PathRequired),
        (20, 1, RequestError::UnexpectedValue),
        (24, 1, RequestError::CasGenerationMustBeZero),
        (32, 1, RequestError::UnexpectedCursor),
    ];
    for (offset, value, expected) in cases {
        let mut wire = base;
        match *offset {
            8 | 24 | 32 => write_u64(&mut wire, *offset..*offset + 8, *value),
            16 | 20 => write_u32(&mut wire, *offset..*offset + 4, *value as u32),
            _ => unreachable!(),
        }
        assert_eq!(
            Request::decode(&wire),
            Err(RequestDecodeError::InvalidRequest(*expected))
        );
    }

    let mut wire = Request::replace(1, 1, 0, ReplaceCondition::Any)
        .unwrap()
        .encode();
    wire[FLAGS_OFFSET] =
        REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_CREATE_ONLY | REQUEST_FLAG_EXACT_GENERATION;
    write_u64(&mut wire, REQUEST_CAS_GENERATION_RANGE, 1);
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::InvalidRequest(
            RequestError::ReplaceFlagsConflict
        ))
    );

    let mut wire = Request::replace(1, 1, 0, ReplaceCondition::Any)
        .unwrap()
        .encode();
    wire[FLAGS_OFFSET] = REQUEST_FLAG_PAYLOAD_VMO | REQUEST_FLAG_EXACT_GENERATION;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::InvalidRequest(
            RequestError::ExactGenerationMustBeNonZero
        ))
    );

    let mut wire = Request::bind(1).unwrap().encode();
    wire[FLAGS_OFFSET] = 0;
    assert_eq!(
        Request::decode(&wire),
        Err(RequestDecodeError::InvalidRequest(
            RequestError::PayloadVmoRequired
        ))
    );
}

#[test]
fn successful_response_matrix_roundtrips() {
    let responses = [
        Response::bound(1).unwrap(),
        Response::mutation(Opcode::CreateDirectory, 2, 3, 0).unwrap(),
        Response::mutation(Opcode::Replace, 3, 4, 4096).unwrap(),
        Response::mutation(Opcode::Unlink, 4, 5, 0).unwrap(),
        Response::file(5, 6, 0).unwrap(),
        Response::file(6, 7, 4096).unwrap(),
        Response::directory_entry(7, 1, 3, EntryKind::Directory, 0, 0).unwrap(),
        Response::directory_entry(8, 2, 7, EntryKind::File, 9, 4096).unwrap(),
        Response::end_of_directory(9, 2).unwrap(),
    ];
    for response in responses {
        response_roundtrip(response);
    }
    assert!(responses[4].has_payload_vmo());
    assert_eq!(responses[4].payload_length(), 0);
    assert_eq!(responses[7].entry_kind(), Some(EntryKind::File));
}

#[test]
fn every_stable_error_status_roundtrips_without_result() {
    for raw in 1..=Status::ProtocolError.raw() {
        let status = Status::from_raw(raw).unwrap();
        let response = Response::error(Opcode::Read, raw as u64, status).unwrap();
        assert_eq!(response.result_kind(), ResultKind::None);
        assert!(!response.has_payload_vmo());
        response_roundtrip(response);
    }
    assert_eq!(
        Response::error(Opcode::Read, 1, Status::Ok),
        Err(ResponseError::SuccessMustHaveResult)
    );
}

#[test]
fn response_payload_validation_checks_exact_length_and_list_path() {
    let file = Response::file(1, 2, 5).unwrap();
    assert_eq!(file.validate_payload(b"amber"), Ok(()));
    assert_eq!(
        file.validate_payload(b"ambe"),
        Err(PayloadError::LengthMismatch {
            expected: 5,
            actual: 4,
        })
    );

    let entry = Response::directory_entry(2, 1, 3, EntryKind::Directory, 0, 0).unwrap();
    assert_eq!(entry.validate_payload(b"dir"), Ok(()));
    assert_eq!(entry.validate_payload(b"a/b"), Ok(()));

    let bad = Response::directory_entry(3, 1, 3, EntryKind::Directory, 0, 0).unwrap();
    assert_eq!(
        bad.validate_payload(b"a/."),
        Err(PayloadError::CurrentDirectoryComponent)
    );
}

#[test]
fn response_constructors_reject_cross_opcode_and_metadata_errors() {
    assert_eq!(
        Response::mutation(Opcode::Read, 1, 1, 0),
        Err(ResponseError::ResultDoesNotMatchOpcode)
    );
    assert_eq!(
        Response::mutation(Opcode::CreateDirectory, 1, 0, 0),
        Err(ResponseError::GenerationMustBeNonZero)
    );
    assert_eq!(
        Response::mutation(Opcode::Unlink, 1, 1, 1),
        Err(ResponseError::UnexpectedObjectSize)
    );
    assert_eq!(
        Response::file(1, 0, 0),
        Err(ResponseError::GenerationMustBeNonZero)
    );
    assert_eq!(
        Response::directory_entry(1, 0, 1, EntryKind::Directory, 0, 0),
        Err(ResponseError::CursorOutOfRange)
    );
    assert_eq!(
        Response::directory_entry(1, 1, 1, EntryKind::Directory, 1, 0),
        Err(ResponseError::DirectoryMetadataMustBeZero)
    );
    assert_eq!(
        Response::directory_entry(1, 1, 1, EntryKind::File, 0, 0),
        Err(ResponseError::FileGenerationMustBeNonZero)
    );
}

#[test]
fn malformed_response_header_and_semantic_matrix_is_rejected() {
    let valid = Response::directory_entry(9, 2, 3, EntryKind::File, 4, 5)
        .unwrap()
        .encode();

    let mut wire = valid;
    wire[0] ^= 1;
    assert_eq!(Response::decode(&wire), Err(ResponseDecodeError::BadMagic));

    let mut wire = valid;
    write_u16(&mut wire, VERSION_RANGE, 2);
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnsupportedVersion(2))
    );

    let mut wire = valid;
    wire[OPCODE_OFFSET] = 0xff;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnknownOpcode(0xff))
    );

    let mut wire = valid;
    wire[FLAGS_OFFSET] = 0x80;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnknownFlags(0x80))
    );

    let mut wire = valid;
    write_u16(&mut wire, RESPONSE_STATUS_RANGE, 0xffff);
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnknownStatus(0xffff))
    );

    let mut wire = valid;
    write_u16(&mut wire, RESPONSE_RESULT_RANGE, 0xffff);
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnknownResultKind(0xffff))
    );

    let mut wire = valid;
    wire[RESPONSE_ENTRY_KIND_OFFSET] = 0xff;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::UnknownEntryKind(0xff))
    );

    let mut wire = valid;
    wire[45] = 1;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::ReservedNonZero { offset: 45 })
    );

    let mut wire = valid;
    wire[63] = 1;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::PaddingNonZero { offset: 63 })
    );

    let mut wire = Response::file(1, 2, 3).unwrap().encode();
    wire[FLAGS_OFFSET] = 0;
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::InvalidResponse(
            ResponseError::PayloadVmoRequired
        ))
    );

    let mut wire = Response::error(Opcode::Read, 1, Status::NotFound)
        .unwrap()
        .encode();
    write_u64(&mut wire, RESPONSE_GENERATION_RANGE, 1);
    assert_eq!(
        Response::decode(&wire),
        Err(ResponseDecodeError::InvalidResponse(
            ResponseError::ErrorMustHaveNoResult
        ))
    );
}

#[test]
fn no_principal_or_session_field_can_hide_in_canonical_padding() {
    let request = Request::bind(1).unwrap().encode();
    assert!(
        request[REQUEST_RESERVED_RANGE]
            .iter()
            .all(|byte| *byte == 0)
    );
    assert!(request[REQUEST_PADDING_RANGE].iter().all(|byte| *byte == 0));

    for offset in 40..64 {
        let mut malformed = request;
        malformed[offset] = 1;
        assert!(Request::decode(&malformed).is_err());
    }
}
