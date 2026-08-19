//! signature_hex ↔ Key text conversion
//!
//! Key text = MTBase64Encode(the 64 bytes of signature_hex)
//! signature_hex = hex representation of MTBase64Decode(Key text)

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
}
