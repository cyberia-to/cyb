//! GPT-2 byte-to-unicode mapping.
//!
//! Each of the 256 bytes maps deterministically to a printable unicode
//! code point. Visible ASCII + printable Latin-1 map to themselves;
//! control bytes and space (0x20) are remapped to u+0100..u+013F range.
//!
//! Spec: reference/runtime/tokenizer.md

/// Build the byte → unicode char mapping. This is constant but we construct
/// it at runtime (once) rather than hardcoding.
pub fn build_byte_encoder() -> [char; 256] {
    let mut table = ['\0'; 256];
    let mut extra: u32 = 0;

    // Start with "visible" printable chars: ! through ~, ¡ through ¬, ® through ÿ
    let visible_ranges: [std::ops::RangeInclusive<u8>; 3] = [
        b'!'..=b'~',
        0xA1..=0xAC,
        0xAE..=0xFF,
    ];
    let mut visible = vec![false; 256];
    for r in &visible_ranges {
        for b in r.clone() {
            visible[b as usize] = true;
            table[b as usize] = char::from_u32(b as u32).unwrap();
        }
    }
    // Remaining bytes → 0x100, 0x101, ...
    for b in 0..256 {
        if !visible[b] {
            table[b] = char::from_u32(0x100 + extra).unwrap();
            extra += 1;
        }
    }
    table
}

/// Reverse: char → byte.
pub fn build_byte_decoder(encoder: &[char; 256]) -> std::collections::HashMap<char, u8> {
    encoder
        .iter()
        .enumerate()
        .map(|(b, &c)| (c, b as u8))
        .collect()
}

/// Encode a UTF-8 string into the byte-level representation: each input byte
/// becomes one character in the output.
pub fn encode_bytes(encoder: &[char; 256], input: &str) -> String {
    input.bytes().map(|b| encoder[b as usize]).collect()
}

/// Decode a byte-level string back to the original UTF-8 bytes.
pub fn decode_bytes(decoder: &std::collections::HashMap<char, u8>, encoded: &str) -> Vec<u8> {
    encoded
        .chars()
        .filter_map(|c| decoder.get(&c).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ascii() {
        let enc = build_byte_encoder();
        let dec = build_byte_decoder(&enc);
        let s = "Hello, world!";
        let encoded = encode_bytes(&enc, s);
        let decoded = decode_bytes(&dec, &encoded);
        assert_eq!(decoded, s.as_bytes());
    }

    #[test]
    fn space_remapped() {
        let enc = build_byte_encoder();
        // Space (0x20) is not in visible ranges — gets remapped.
        let space = enc[0x20];
        assert!(space as u32 >= 0x100);
    }

    #[test]
    fn visible_is_identity() {
        let enc = build_byte_encoder();
        // 'A' = 0x41 is in visible range
        assert_eq!(enc[b'A' as usize], 'A');
    }

    #[test]
    fn unicode_input() {
        let enc = build_byte_encoder();
        let dec = build_byte_decoder(&enc);
        // UTF-8 emoji byte sequence
        let s = "héllo 🌍";
        let encoded = encode_bytes(&enc, s);
        let decoded = decode_bytes(&dec, &encoded);
        assert_eq!(decoded, s.as_bytes());
    }
}
