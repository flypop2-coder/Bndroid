//! Bionic short-value property ABI. The mutable key set is supplied by Init;
//! updates cannot insert keys, change read-only boot facts or resize the area.
#[cfg(test)]
use alloc::vec::Vec;
pub const AREA_BYTES: usize = 128 * 1024;
pub fn find(bytes: &[u8], name: &str) -> Result<usize, i64> {
    fn word(bytes: &[u8], at: usize) -> Result<u32, i64> {
        Ok(u32::from_le_bytes(
            bytes.get(at..at + 4).ok_or(22)?.try_into().unwrap(),
        ))
    }
    if bytes.len() != AREA_BYTES || word(bytes, 8)? != 0x504f5250 || word(bytes, 12)? != 0xfc6ed0ab
    {
        return Err(22);
    }
    let used = word(bytes, 0)? as usize;
    let data = bytes.get(128..128 + used).ok_or(22)?;
    let mut node = 0;
    let mut steps = 0;
    for token in name.split('.') {
        node = word(data, node + 16)? as usize;
        loop {
            steps += 1;
            if node == 0 {
                return Err(2);
            }
            if steps > 4096 || !node.is_multiple_of(4) {
                return Err(22);
            }
            let length = word(data, node)? as usize;
            let key = data.get(node + 20..node + 20 + length).ok_or(22)?;
            match token
                .len()
                .cmp(&length)
                .then_with(|| token.as_bytes().cmp(key))
            {
                core::cmp::Ordering::Equal => break,
                core::cmp::Ordering::Less => node = word(data, node + 8)? as usize,
                core::cmp::Ordering::Greater => node = word(data, node + 12)? as usize,
            }
        }
    }
    let prop = word(data, node + 4)? as usize;
    if prop == 0 || !prop.is_multiple_of(4) {
        return Err(2);
    }
    if data.get(prop + 96..prop + 96 + name.len()) != Some(name.as_bytes())
        || data.get(prop + 96 + name.len()) != Some(&0)
    {
        return Err(22);
    }
    Ok(128 + prop)
}
/// Parse one fragmented protocol-v2 request. A complete packet has no trailing
/// bytes; oversized lengths fail before allocation or a property mutation.
pub fn request(bytes: &[u8]) -> Result<Option<(&str, &str)>, i64> {
    if bytes.len() < 8 {
        return Ok(None);
    }
    if u32::from_le_bytes(bytes[..4].try_into().unwrap()) != 0x0002_0001 {
        return Err(22);
    }
    let n = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if n == 0 || n > 96 {
        return Err(22);
    }
    if bytes.len() < 12 + n {
        return Ok(None);
    }
    let v = u32::from_le_bytes(bytes[8 + n..12 + n].try_into().unwrap()) as usize;
    if v >= 92 {
        return Err(22);
    }
    if bytes.len() < 12 + n + v {
        return Ok(None);
    }
    if bytes.len() != 12 + n + v {
        return Err(22);
    }
    let name = core::str::from_utf8(&bytes[8..8 + n]).map_err(|_| 22)?;
    let value = core::str::from_utf8(&bytes[12 + n..]).map_err(|_| 22)?;
    if !bndr_sm::service_authority::property_name(name) || value.as_bytes().contains(&0) {
        return Err(22);
    }
    Ok(Some((name, value)))
}
pub fn update(bytes: &mut [u8], name: &str, value: &str) -> Result<usize, i64> {
    if name.starts_with("ro.") || value.len() >= 92 || value.as_bytes().contains(&0) {
        return Err(13);
    }
    let at = find(bytes, name)?;
    let serial = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    if serial & 1 != 0 || serial >> 24 >= 92 {
        return Err(22);
    }
    // This staging buffer is published under the serialized kernel owner; the
    // actual shared area uses the same dirty/clean publication order below.
    let mut old = [0; 92];
    old.copy_from_slice(&bytes[at + 4..at + 96]);
    bytes[148..240].copy_from_slice(&old);
    bytes[at + 4..at + 96].fill(0);
    bytes[at + 4..at + 4 + value.len()].copy_from_slice(value.as_bytes());
    let next = ((value.len() as u32) << 24) | (serial.wrapping_add(2) & 0xfffffe);
    bytes[at..at + 4].copy_from_slice(&next.to_le_bytes());
    let sequence = u32::from_le_bytes(bytes[4..8].try_into().unwrap()).wrapping_add(1);
    bytes[4..8].copy_from_slice(&sequence.to_le_bytes());
    Ok(at)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_requests_reject_excess_and_oversized_fields_before_mutation() {
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x20001u32.to_le_bytes());
        packet.extend_from_slice(&1u32.to_le_bytes());
        packet.push(b'x');
        packet.extend_from_slice(&1u32.to_le_bytes());
        packet.push(b'y');
        for n in 0..packet.len() {
            assert_eq!(request(&packet[..n]), Ok(None));
        }
        assert_eq!(request(&packet), Ok(Some(("x", "y"))));
        packet.push(0);
        assert_eq!(request(&packet), Err(22));
        packet[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(request(&packet), Err(22));
    }
    fn area() -> Vec<u8> {
        let mut bytes = alloc::vec![0;AREA_BYTES];
        for (at, n) in [
            (0, 256u32),
            (8, 0x504f5250),
            (12, 0xfc6ed0ab),
            (144, 112),
            (240, 1),
            (244, 136),
            (272, 0),
        ] {
            bytes[at..at + 4].copy_from_slice(&n.to_le_bytes());
        }
        bytes[260] = b'x';
        bytes[360] = b'x';
        bytes
    }
    #[test]
    fn updates_preserve_boot_area_and_unknown_keys_without_growth() {
        let mut bytes = area();
        assert_eq!(find(&bytes, "x"), Ok(264));
        assert_eq!(update(&mut bytes, "x", "ready"), Ok(264));
        assert_eq!(&bytes[268..274], b"ready\0");
        assert_eq!(
            u32::from_le_bytes(bytes[264..268].try_into().unwrap()),
            (5 << 24) | 2
        );
        assert_eq!(bytes[4], 1);
        let before = bytes.clone();
        assert_eq!(update(&mut bytes, "y", "value"), Err(2));
        assert_eq!(update(&mut bytes, "ro.x", "value"), Err(13));
        assert_eq!(bytes, before);
        bytes[144..148].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(find(&bytes, "x"), Err(22));
    }
}
