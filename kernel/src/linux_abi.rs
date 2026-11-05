//! Linux arm64 entry/return ABI primitives, independent of kernel mechanisms.
//! This is a compatibility personality, not a Linux kernel implementation.

pub const STACK_PAGE_BYTES: usize = 4096;
pub const MAX_ARGUMENTS: usize = 32;
pub const MAX_ENVIRONMENT: usize = 32;
pub const MAX_AUXILIARY: usize = 32;
pub const EBADF: i64 = 9;
pub const EFAULT: i64 = 14;
pub const EINVAL: i64 = 22;
pub const ENOSYS: i64 = 38;

pub const fn errno(number: i64) -> u64 {
    (-number) as u64
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StackError {
    Misaligned,
    AddressOverflow,
    TooManyEntries,
    InteriorNul,
    NoSpace,
    InvalidAuxiliary,
}

/// Writes argc/argv/envp/auxv and strings into one already-mapped stack page.
/// SP points to the beginning of this page; every pointer is a guest VA.
/// On failure the caller must discard the unpublished page.
pub fn initial_stack(
    output: &mut [u8; STACK_PAGE_BYTES],
    address: u64,
    arguments: &[&[u8]],
    environment: &[&[u8]],
    auxiliary: &[(u64, u64)],
) -> Result<(), StackError> {
    initial_stack_with_random(output, address, arguments, environment, auxiliary, None)
}

pub fn initial_stack_with_random(
    output: &mut [u8; STACK_PAGE_BYTES],
    address: u64,
    arguments: &[&[u8]],
    environment: &[&[u8]],
    auxiliary: &[(u64, u64)],
    random: Option<&[u8; 16]>,
) -> Result<(), StackError> {
    if !address.is_multiple_of(16) {
        return Err(StackError::Misaligned);
    }
    address
        .checked_add(STACK_PAGE_BYTES as u64)
        .ok_or(StackError::AddressOverflow)?;
    if arguments.len() > MAX_ARGUMENTS
        || environment.len() > MAX_ENVIRONMENT
        || auxiliary.len() > MAX_AUXILIARY
    {
        return Err(StackError::TooManyEntries);
    }
    for (index, (kind, _)) in auxiliary.iter().enumerate() {
        if *kind == 0
            || *kind == 31
            || (*kind == 25 && random.is_some())
            || auxiliary[..index].iter().any(|(other, _)| other == kind)
        {
            return Err(StackError::InvalidAuxiliary);
        }
    }
    output.fill(0);
    let words = 1
        + arguments.len()
        + 1
        + environment.len()
        + 1
        + (auxiliary.len() + 2 + usize::from(random.is_some())) * 2;
    let mut strings = words * 8;
    let mut cursor = 0;
    put_word(output, &mut cursor, arguments.len() as u64);
    let mut executable_name = 0;
    for (group_index, group) in [arguments, environment].iter().enumerate() {
        for (index, string) in group.iter().enumerate() {
            if string.contains(&0) {
                return Err(StackError::InteriorNul);
            }
            let end = strings
                .checked_add(string.len())
                .and_then(|end| end.checked_add(1))
                .ok_or(StackError::NoSpace)?;
            if end > output.len() {
                return Err(StackError::NoSpace);
            }
            let pointer = address + strings as u64;
            if group_index == 0 && index == 0 {
                executable_name = pointer;
            }
            put_word(output, &mut cursor, pointer);
            output[strings..end - 1].copy_from_slice(string);
            strings = end;
        }
        put_word(output, &mut cursor, 0);
    }
    for &(kind, value) in auxiliary {
        put_word(output, &mut cursor, kind);
        put_word(output, &mut cursor, value);
    }
    put_word(output, &mut cursor, 31); // AT_EXECFN
    put_word(output, &mut cursor, executable_name);
    if let Some(random) = random {
        let start = (strings + 15) & !15;
        if start + random.len() > output.len() {
            return Err(StackError::NoSpace);
        }
        output[start..start + random.len()].copy_from_slice(random);
        put_word(output, &mut cursor, 25); // AT_RANDOM
        put_word(output, &mut cursor, address + start as u64);
    }
    put_word(output, &mut cursor, 0); // AT_NULL
    put_word(output, &mut cursor, 0);
    Ok(())
}

fn put_word(output: &mut [u8], cursor: &mut usize, value: u64) {
    output[*cursor..*cursor + 8].copy_from_slice(&value.to_le_bytes());
    *cursor += 8;
}

/// Linux timespec is two signed 64-bit values on arm64.
pub fn monotonic_timespec(counter: u64, frequency: u64) -> Option<[u8; 16]> {
    if frequency == 0 {
        return None;
    }
    let seconds = i64::try_from(counter / frequency).ok()?;
    let nanoseconds =
        ((u128::from(counter % frequency) * 1_000_000_000) / u128::from(frequency)) as i64;
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&seconds.to_le_bytes());
    bytes[8..].copy_from_slice(&nanoseconds.to_le_bytes());
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_contains_guest_pointers_terminators_and_auxv() {
        let base = 0x2001fe000;
        let mut page = [0xff; STACK_PAGE_BYTES];
        initial_stack(
            &mut page,
            base,
            &[b"/system/bin/probe", b"--verify"],
            &[b"LANG=C"],
            &[(6, 4096), (9, 0x200001000)],
        )
        .unwrap();
        let word =
            |index: usize| u64::from_le_bytes(page[index * 8..index * 8 + 8].try_into().unwrap());
        assert_eq!(word(0), 2);
        assert_eq!(word(3), 0);
        assert_eq!(word(5), 0);
        assert_eq!((word(6), word(7)), (6, 4096));
        assert_eq!((word(8), word(9)), (9, 0x200001000));
        assert_eq!((word(10), word(11)), (31, word(1)));
        assert_eq!((word(12), word(13)), (0, 0));
        for (index, text) in [
            (1, b"/system/bin/probe\0".as_slice()),
            (2, b"--verify\0"),
            (4, b"LANG=C\0"),
        ] {
            let offset = (word(index) - base) as usize;
            assert_eq!(&page[offset..offset + text.len()], text);
        }
    }

    #[test]
    fn stack_rejects_overflow_nul_and_ambiguous_auxv() {
        let mut page = [0; STACK_PAGE_BYTES];
        assert_eq!(
            initial_stack(&mut page, 1, &[], &[], &[]),
            Err(StackError::Misaligned)
        );
        assert_eq!(
            initial_stack(&mut page, u64::MAX - 15, &[], &[], &[]),
            Err(StackError::AddressOverflow)
        );
        assert_eq!(
            initial_stack(&mut page, 0, &[b"x\0y"], &[], &[]),
            Err(StackError::InteriorNul)
        );
        assert_eq!(
            initial_stack(&mut page, 0, &[&[b'x'; STACK_PAGE_BYTES]], &[], &[]),
            Err(StackError::NoSpace)
        );
        for aux in [&[(0, 0)][..], &[(31, 42)], &[(6, 4096), (6, 16384)]] {
            assert_eq!(
                initial_stack(&mut page, 0, &[], &[], aux),
                Err(StackError::InvalidAuxiliary)
            );
        }
        assert_eq!(
            initial_stack(
                &mut page,
                0,
                &[b"x".as_slice(); MAX_ARGUMENTS + 1],
                &[],
                &[]
            ),
            Err(StackError::TooManyEntries)
        );
        assert_eq!(
            initial_stack(
                &mut page,
                0,
                &[],
                &[b"x".as_slice(); MAX_ENVIRONMENT + 1],
                &[]
            ),
            Err(StackError::TooManyEntries)
        );
        assert_eq!(
            initial_stack(&mut page, 0, &[], &[], &[(6, 4096); MAX_AUXILIARY + 1]),
            Err(StackError::TooManyEntries)
        );
    }

    #[test]
    fn errno_is_negative_in_x0_not_a_native_status_tuple() {
        assert_eq!(errno(ENOSYS) as i64, -38);
        assert_eq!(errno(EFAULT) as i64, -14);
    }

    #[test]
    fn random_auxiliary_points_at_supplied_guest_bytes() {
        let mut page = [0; STACK_PAGE_BYTES];
        let bytes = [0xa5; 16];
        initial_stack_with_random(&mut page, 0x200000000, &[b"probe"], &[], &[], Some(&bytes))
            .unwrap();
        let word =
            |index: usize| u64::from_le_bytes(page[index * 8..index * 8 + 8].try_into().unwrap());
        assert_eq!(word(6), 25);
        let offset = (word(7) - 0x200000000) as usize;
        assert_eq!(offset % 16, 0);
        assert_eq!(&page[offset..offset + 16], &bytes);
        assert_eq!(word(8), 0);
        assert_eq!(
            initial_stack_with_random(&mut page, 0, &[], &[], &[(25, 8)], Some(&bytes)),
            Err(StackError::InvalidAuxiliary)
        );
    }

    #[test]
    fn timespec_uses_checked_seconds_and_subsecond_precision() {
        let wire = monotonic_timespec(75_000_001, 50_000_000).unwrap();
        assert_eq!(i64::from_le_bytes(wire[..8].try_into().unwrap()), 1);
        assert_eq!(
            i64::from_le_bytes(wire[8..].try_into().unwrap()),
            500_000_020
        );
        assert!(monotonic_timespec(1, 0).is_none());
        assert!(monotonic_timespec(u64::MAX, 1).is_none());
    }
}
