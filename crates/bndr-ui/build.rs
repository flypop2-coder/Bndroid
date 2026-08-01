use std::{env, error::Error, fs, io, path::PathBuf};

const EXPECTED_FONT_BYTES: usize = 126_023;
const EXPECTED_FONT_FNV1A64: u64 = 0x7794_4e20_f75a_453f;

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, io::Error> {
    let mut output = Vec::with_capacity(EXPECTED_FONT_BYTES);
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;

    for byte in input.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            break;
        }
        let value = base64_value(byte).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "mobile font atlas contains an invalid base64 byte",
            )
        })?;
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(
                u8::try_from((accumulator >> bits) & 0xff)
                    .expect("masked base64 output always fits in one byte"),
            );
        }
    }
    if output.len() != EXPECTED_FONT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mobile font atlas decoded to an unexpected length",
        ));
    }
    let digest = output.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    if digest != EXPECTED_FONT_FNV1A64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mobile font atlas checksum did not match its generated metadata",
        ));
    }
    Ok(output)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/mobile_font_alpha4.b64");
    let decoded = decode_base64(include_str!("src/mobile_font_alpha4.b64"))?;
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Cargo did not provide OUT_DIR")
        })?)
        .join("mobile_font_alpha4.bin");
    fs::write(output_path, decoded)?;
    Ok(())
}
