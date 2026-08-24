//! signature_hex ↔ Key text conversion
//!
//! Key text = MTBase64Encode(the 64 bytes of signature_hex)
//! signature_hex = hex representation of MTBase64Decode(Key text)

use crate::sha256_constants::ROUND_CONSTANTS;

/// MTBase64 character table (same alphabet as standard Base64, but LSB-first bit order)
const BASE64_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Convert a 64-byte signature (hex string) to Key text format
pub fn signature_to_key_text(signature_hex: &str) -> Result<String, String> {
    let sig_bytes = hex_decode(signature_hex)?;
    if sig_bytes.len() != 64 {
        return Err(format!("signature must be 64 bytes, got {}", sig_bytes.len()));
    }

    let encoded = mt_base64_encode(&sig_bytes);

    // Split into two lines (around the middle)
    let mid = encoded.len() / 2;

    Ok(format!(
        "-----BEGIN MIKROTIK SOFTWARE KEY------------\n{}\n{}\n-----END MIKROTIK SOFTWARE KEY--------------",
        &encoded[..mid],
        &encoded[mid..]
    ))
}

/// Convert Key text to the hex string of a 64-byte signature
pub fn key_text_to_signature(key_text: &str) -> Result<String, String> {
    // Extract the content between BEGIN/END
    let lines: Vec<&str> = key_text.lines().collect();
    let mut b64_data = String::new();

    let mut in_key = false;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.contains("BEGIN MIKROTIK") {
            in_key = true;
            continue;
        }
        if trimmed.contains("END MIKROTIK") {
            break;
        }
        if in_key {
            b64_data.push_str(trimmed);
        }
    }

    if b64_data.is_empty() {
        return Err("no key data found between BEGIN/END markers".to_string());
    }

    let decoded = mt_base64_decode(&b64_data)?;
    Ok(hex_encode(&decoded))
}

/// Metadata embedded in a signature's first 16 bytes: SOFTWARE ID, a version byte, and license level.
///
/// See `decode_metadata` for how this is extracted.
pub struct LicenseMetadata {
    pub software_id: String,
    /// Byte 6 of the decrypted block. Not labeled by the reference MTLic `ParseLic.py` --
    /// meaning unconfirmed, printed as-is.
    pub version_byte: u8,
    pub level: u8,
    /// Whether bytes 8..16 of the decrypted block are all zero, as expected for a well-formed
    /// signature. `false` means either this isn't a real signature or the decode is wrong.
    pub padding_ok: bool,
}

/// Decrypt the SOFTWARE ID / level metadata embedded in a signature's first 16 bytes.
///
/// Confirmed against the reference implementation (`MT_Transform` in MTLic's `MTTools.py`,
/// https://github.com/Ygnecz/MTLic): a signature's first 16 bytes, run through this transform,
/// decode to `SOFTWARE_ID(6B LE) || reserved(1B) || level(1B) || zero-padding(8B)`.
pub fn decode_metadata(signature_hex: &str) -> Result<LicenseMetadata, String> {
    let sig_bytes = hex_decode(signature_hex)?;
    if sig_bytes.len() != 64 {
        return Err(format!("signature must be 64 bytes, got {}", sig_bytes.len()));
    }

    let mut block = [0u8; 16];
    block.copy_from_slice(&sig_bytes[0..16]);
    mt_transform(&mut block);

    let software_id_val = u64::from_le_bytes(block[0..8].try_into().unwrap()) & 0x0000_FFFF_FFFF_FFFF;

    Ok(LicenseMetadata {
        software_id: crate::software_id::encode(software_id_val),
        version_byte: block[6],
        level: block[7],
        padding_ok: block[8..].iter().all(|&b| b == 0),
    })
}

/// MikroTik's proprietary ARX block cipher, called `MT_Transform` in the MTLic reference
/// implementation. Decrypts (not encrypts) a signature's first 16 bytes into SOFTWARE ID/level
/// metadata -- see `decode_metadata`. Reuses this project's MikroTik SHA-256 round constants,
/// which are the same table `MT_Transform` uses (confirmed against MTLic's `MTTools.py`).
fn mt_transform(block: &mut [u8; 16]) {
    let mut s = [0u32; 4];
    for (w, chunk) in s.iter_mut().zip(block.chunks_exact(4)) {
        *w = u32::from_be_bytes(chunk.try_into().unwrap());
    }

    for i in 0..16 {
        let (p, q, r, t) = (i % 4, (i + 1) % 4, (i + 2) % 4, (i + 3) % 4);
        let k0 = ROUND_CONSTANTS[i * 4];
        let k1 = ROUND_CONSTANTS[i * 4 + 1];
        let k2 = ROUND_CONSTANTS[i * 4 + 2];
        let k3 = ROUND_CONSTANTS[i * 4 + 3];

        // `k & 0x0F` is always 0..=15, so `rotate_left` can never overflow.
        s[r] = s[r].wrapping_sub(s[p]).wrapping_sub(k0);
        s[t] = (s[p].rotate_left(k0 & 0x0F) ^ s[t]).wrapping_add(s[p]);

        s[q] = s[q].wrapping_sub(s[t]).wrapping_sub(k1);
        s[r] = (s[q].rotate_left(k1 & 0x0F) ^ s[r]).wrapping_add(s[q]);

        s[p] = s[p].wrapping_sub(s[r]).wrapping_sub(k2);
        s[q] = (s[r].rotate_left(k2 & 0x0F) ^ s[q]).wrapping_add(s[r]);

        s[t] = s[t].wrapping_sub(s[q]).wrapping_sub(k3);
        s[p] = (s[t].rotate_left(k3 & 0x0F) ^ s[p]).wrapping_add(s[t]);
    }

    for (chunk, w) in block.chunks_exact_mut(4).zip(s.iter()) {
        chunk.copy_from_slice(&w.to_be_bytes());
    }
}

/// MikroTik Base64 encoding (LSB-first bit order)
fn mt_base64_encode(data: &[u8]) -> String {
    let mut encoded = String::new();
    let mut pending_bits = 0u32;

    for (i, &byte) in data.iter().enumerate() {
        if pending_bits == 0 {
            encoded.push(BASE64_TABLE[(byte & 0x3F) as usize] as char);
            pending_bits = 2;
        } else if pending_bits == 6 {
            encoded.push(BASE64_TABLE[(data[i - 1] >> 2) as usize] as char);
            encoded.push(BASE64_TABLE[(byte & 0x3F) as usize] as char);
            pending_bits = 2;
        } else {
            let index1 = data[i - 1] >> (8 - pending_bits);
            let index2 = (byte as u32) << pending_bits;
            encoded.push(BASE64_TABLE[((index1 as u32 | index2) & 0x3F) as usize] as char);
            pending_bits += 2;
        }
    }

    if pending_bits != 0 {
        encoded.push(
            BASE64_TABLE[(data[data.len() - 1] >> (8 - pending_bits)) as usize] as char,
        );
    }

    // Padding
    while encoded.len() % 4 != 0 {
        encoded.push('=');
    }

    encoded
}

/// MikroTik Base64 decoding (LSB-first bit order)
fn mt_base64_decode(data: &str) -> Result<Vec<u8>, String> {
    let bytes: Vec<u8> = data
        .bytes()
        .filter(|&b| b != b'=')
        .collect();

    let mut result = Vec::new();
    let mut pending_bits = 0u32;

    for (i, &byte) in bytes.iter().enumerate() {
        if pending_bits == 0 {
            pending_bits = 6;
        } else {
            let pos_prev = BASE64_TABLE
                .iter()
                .position(|&c| c == bytes[i - 1])
                .ok_or_else(|| format!("invalid base64 char: {}", bytes[i - 1] as char))?;
            let pos_curr = BASE64_TABLE
                .iter()
                .position(|&c| c == byte)
                .ok_or_else(|| format!("invalid base64 char: {}", byte as char))?;

            let value1 = pos_prev >> (6 - pending_bits);
            let value2 = pos_curr & ((1 << (8 - pending_bits)) - 1);
            let value = (value1 | (value2 << pending_bits)) as u8;
            result.push(value);
            pending_bits -= 2;
        }
    }

    Ok(result)
}

/// hex string → byte array
fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return Err("hex string must have even length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at position {}: {}", i, e))
        })
        .collect()
}

/// byte array → uppercase hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_synthetic() {
        // Synthetic 64-byte signature; verify sig→key→sig round-trips exactly.
        let sig: String = (0..64).map(|i| format!("{:02X}", (i * 7 + 3) as u8)).collect();

        // sig → key
        let key = signature_to_key_text(&sig).unwrap();
        assert!(key.starts_with("-----BEGIN"), "key header missing");
        assert!(key.trim_end().ends_with("-----"), "key footer missing");

        // key → sig
        let back = key_text_to_signature(&key).unwrap();
        assert_eq!(back, sig, "sig↔key roundtrip mismatch");
    }

    #[test]
    fn test_decode_metadata_vi8q_e90f() {
        // VI8Q-E90F's signature hex, from docs/collision-database.md -- confirmed L1
        // (nlevel: 1) on real hardware. Also independently confirmed by decoding this
        // project's key-text form of the same signature against the reference MT_Transform.
        let sig = "FAF308BA3FFD4185308A8784244749EFFE7E4E65C14C01CD55D946506B47F636757F62106D114329104012DE7B44543F3444F0E724080873E3A20E11F5EF450E";
        let meta = decode_metadata(sig).unwrap();
        assert_eq!(meta.software_id, "VI8Q-E90F");
        assert_eq!(meta.level, 1);
        assert!(meta.padding_ok);
    }
}
