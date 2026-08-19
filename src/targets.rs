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

/// Load targets from keys.toml; exits with error if not found or empty
pub fn load_targets(config_path: Option<&str>) -> Vec<Target> {
    let entries = config_path
        .and_then(load_from_file)
        .or_else(|| load_from_file("keys.toml"))
        .unwrap_or_default();

    if entries.is_empty() {
        eprintln!("Error: keys.toml not found or empty. Copy keys.example.toml to keys.toml and add your signatures.");
        std::process::exit(1);
    }

    eprintln!("Loaded {} keys from config", entries.len());
    entries_to_targets(&entries)
}

/// Get the lo/hi components of the MBR mix
pub fn mbr_mix() -> (u32, u32) {
    (MBR_MIX as u32, (MBR_MIX >> 32) as u32)
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

fn entries_to_targets(entries: &[KeyEntry]) -> Vec<Target> {
    let (mix_lo, mix_hi) = mbr_mix();

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

