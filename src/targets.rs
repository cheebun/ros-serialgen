//! Collision search target management
//!
//! Loads known L6 signatures from an external `keys.toml`, allowing new keys to be added without recompilation.
//!
//! # Configuration format
//! ```toml
//! [[key]]
//! software_id = "XXXX-XXXX"
//! signature_hex = "..."
//! ```

use crate::software_id;
use std::fs;
use std::path::Path;

/// Collision search target
pub struct Target {
    /// SOFTWARE ID (e.g. "XXXX-XXXX")
    pub name: String,
    /// Required sid_lo to match (= target_lo ⊕ mix_lo)
    pub need_lo: u32,
    /// Required sid_hi to match (= (target_hi ⊕ mix_hi) & 0xFF)
    pub need_hi: u8,
    /// MBR signature hex (64 bytes, printed on a hit)
    pub signature_hex: String,
}

/// Fixed mix value for an all-zero MBR: mbr_val=0x0BD, mix=0x0BD × 0x3FF800F
const MBR_MIX: u64 = 0x0BD_u64 * 0x3FF800F;

/// Load targets from keys.toml; exits with error if not found or empty.
///
/// `mix` must be the *same* mix (lo, hi) that the caller will use to compute candidate
/// SOFTWARE IDs (`targets::mbr_mix()` for the standard identity, or
/// `targets::mix_from_identity(...)` for a custom one) -- `need_lo`/`need_hi` are only
/// meaningful relative to that specific mix. Passing a mismatched mix silently makes
/// every match check fail (or match the wrong candidates).
pub fn load_targets(config_path: Option<&str>, mix: (u32, u32)) -> Vec<Target> {
    let entries = config_path
        .and_then(load_from_file)
        .or_else(|| load_from_file("keys.toml"))
        .unwrap_or_default();

    if entries.is_empty() {
        eprintln!("Error: keys.toml not found or empty. Copy keys.example.toml to keys.toml and add your signatures.");
        std::process::exit(1);
    }

    eprintln!("Loaded {} keys from config", entries.len());
    entries_to_targets(&entries, mix)
}

/// Get the lo/hi components of the MBR mix
pub fn mbr_mix() -> (u32, u32) {
    (MBR_MIX as u32, (MBR_MIX >> 32) as u32)
}

/// Derive the mix (lo, hi) from a real, non-standard 10-byte MBR identity seed
/// (`0x100-0x109`), instead of assuming the standard all-zero identity.
///
/// Formula (see `docs/license-internals.md` §3.2, §3.6, reverse-engineered from and
/// cross-checked against the `keyman` binary):
/// ```text
/// sha_val  = MikroTik_SHA256(identity)[0:2] as LE u16
/// chksum   = NOT(sum of 5 LE u16 words of identity) & 0xFFFF
/// mbr_val  = (sha_val XOR chksum) & 0x7FF
/// mix      = mbr_val * 0x3FF800F
/// ```
pub fn mix_from_identity(identity: &[u8; 10]) -> (u32, u32) {
    let sha_val = crate::sha256::hash_10(identity);
    let mut sum: u16 = 0;
    for chunk in identity.chunks_exact(2) {
        sum = sum.wrapping_add(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    let chksum = !sum;
    let mbr_val = ((sha_val ^ chksum) as u64) & 0x7FF;
    let mix = mbr_val * 0x3FF800F;
    (mix as u32, (mix >> 32) as u32)
}

// ---- Internal implementation ----

struct KeyEntry {
    software_id: String,
    signature_hex: String,
}

fn load_from_file(path: &str) -> Option<Vec<KeyEntry>> {
    if !Path::new(path).exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let mut entries = Vec::new();
    let mut current_sid = String::new();
    let mut current_sig = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[[key]]" {
            if !current_sid.is_empty() {
                entries.push(KeyEntry {
                    software_id: current_sid.clone(),
                    signature_hex: current_sig.clone(),
                });
            }
            current_sid.clear();
            current_sig.clear();
        } else if let Some(rest) = trimmed.strip_prefix("software_id") {
            current_sid = rest
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .to_string();
        } else if let Some(rest) = trimmed.strip_prefix("signature_hex") {
            current_sig = rest
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .to_string();
        }
    }

    if !current_sid.is_empty() {
        entries.push(KeyEntry {
            software_id: current_sid,
            signature_hex: current_sig,
        });
    }

    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_from_identity_matches_standard_all_zero() {
        // The standard all-zero identity used by collision search must reduce to
        // the same fixed mix as mbr_mix()'s hardcoded MBR_MIX constant.
        let (lo, hi) = mix_from_identity(&[0u8; 10]);
        let (std_lo, std_hi) = mbr_mix();
        assert_eq!((lo, hi), (std_lo, std_hi));
    }

    #[test]
    fn test_mix_from_identity_deterministic() {
        let identity = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];
        let a = mix_from_identity(&identity);
        let b = mix_from_identity(&identity);
        assert_eq!(a, b, "same identity must produce same mix");
    }

    #[test]
    fn test_mix_from_identity_differs_from_standard() {
        // A non-zero identity should (overwhelmingly likely) produce a different mix
        // than the standard all-zero one.
        let identity = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];
        assert_ne!(mix_from_identity(&identity), mbr_mix());
    }
}

fn entries_to_targets(entries: &[KeyEntry], mix: (u32, u32)) -> Vec<Target> {
    let (mix_lo, mix_hi) = mix;

    entries
        .iter()
        .map(|e| {
            let tv = software_id::decode(&e.software_id).unwrap_or_else(|e| panic!("invalid SOFTWARE ID in config: {}", e));
            Target {
                name: e.software_id.clone(),
                need_lo: (tv as u32) ^ mix_lo,
                need_hi: (((tv >> 32) as u32 ^ mix_hi) & 0xFF) as u8,
                signature_hex: e.signature_hex.clone(),
            }
        })
        .collect()
}

