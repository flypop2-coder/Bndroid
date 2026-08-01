pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        let mut bit = 0;
        while bit < 8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            bit += 1;
        }
    }
    !crc
}

pub(crate) fn adler32(bytes: &[u8]) -> u32 {
    const MODULUS: u32 = 65_521;
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in bytes {
        a = (a + u32::from(byte)) % MODULUS;
        b = (b + a) % MODULUS;
    }
    (b << 16) | a
}

pub(crate) fn sha1(bytes: &[u8]) -> [u8; 20] {
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let full_blocks = bytes.len() / 64;
    let remainder = bytes.len() % 64;
    let tail_blocks = if remainder < 56 { 1 } else { 2 };

    let mut state = [
        0x6745_2301u32,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];

    let mut block_index = 0usize;
    while block_index < full_blocks {
        let start = block_index * 64;
        compress_sha1(&mut state, &bytes[start..start + 64]);
        block_index += 1;
    }

    let mut tail = [0u8; 128];
    tail[..remainder].copy_from_slice(&bytes[full_blocks * 64..]);
    tail[remainder] = 0x80;
    let tail_len = tail_blocks * 64;
    tail[tail_len - 8..tail_len].copy_from_slice(&bit_len.to_be_bytes());
    compress_sha1(&mut state, &tail[..64]);
    if tail_blocks == 2 {
        compress_sha1(&mut state, &tail[64..128]);
    }

    let mut output = [0u8; 20];
    for (index, word) in state.into_iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

fn compress_sha1(state: &mut [u32; 5], block: &[u8]) {
    let mut words = [0u32; 80];
    let mut index = 0usize;
    while index < 16 {
        let base = index * 4;
        words[index] = u32::from_be_bytes([
            block[base],
            block[base + 1],
            block[base + 2],
            block[base + 3],
        ]);
        index += 1;
    }
    while index < 80 {
        words[index] =
            (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                .rotate_left(1);
        index += 1;
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    index = 0;
    while index < 80 {
        let (function, constant) = match index {
            0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
            20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
            _ => (b ^ c ^ d, 0xca62_c1d6),
        };
        let temporary = a
            .rotate_left(5)
            .wrapping_add(function)
            .wrapping_add(e)
            .wrapping_add(constant)
            .wrapping_add(words[index]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temporary;
        index += 1;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32, sha1};

    #[test]
    fn standard_digest_vectors() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        assert_eq!(
            sha1(b"abc"),
            [
                0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
                0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d,
            ]
        );
    }
}
