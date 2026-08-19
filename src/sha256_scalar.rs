//! MikroTik custom SHA-256 implementation
//!
//! Standard SHA-256 algorithm structure (64-round Merkle-Damgård compression),
//! but using MikroTik's proprietary initial vector (IV) and round constants (K).
//!
//! Source: reverse-engineered from the RouterOS `/nova/bin/keyman` binary.
//! Verified: matches `MikroSHA256` in the MikroTikPatch project's `mikro.py` exactly.

use crate::sha256_constants::{INITIAL_HASH_VALUES, ROUND_CONSTANTS};

/// Perform MikroTik custom SHA-256 on exactly 40 bytes of input.
///
/// Returns `(sid_lo, sid_hi)`:
/// - `sid_lo`: first u32 of the SHA-256 output (big-endian output → little-endian read), source of the SOFTWARE ID low 32 bits
/// - `sid_hi`: the 5th byte of the SHA-256 output (the most significant big-endian byte of the second u32), source of the SOFTWARE ID high byte
///
/// These two values are used in the SOFTWARE ID computation:
/// ```text
/// final_hi = (sid_hi | 0x100) XOR mix_hi  (ensures bit 8 is always set)
/// final_lo = sid_lo XOR mix_lo
/// SOFTWARE_ID = base35_encode(final_hi << 32 | final_lo)
/// ```
///
/// # Byte order note
///
/// SHA-256 operates on u32 internally, with output in big-endian order (standard behavior).
/// However, RouterOS keyman reads the first 4 bytes as sid_lo in little-endian,
/// so we convert to a big-endian byte array first, then interpret as little-endian — equivalent to a byte reversal.
pub fn hash_40(data: &[u8; 40]) -> (u32, u8) {
    // ---- Padding ----
    // 40 bytes + 0x80 + 21 zero bytes + 8-byte length = 64 bytes (one block)
    let mut padded = [0u8; 64];
    padded[..40].copy_from_slice(data);
    padded[40] = 0x80; // padding start bit
                       // message length = 40 × 8 = 320 bits = 0x0140, written into the last 8 bytes (big-endian)
    padded[62] = 0x01;
    padded[63] = 0x40;

    // ---- Parse into 16 big-endian u32 words (message schedule initial values) ----
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            padded[i * 4],
            padded[i * 4 + 1],
            padded[i * 4 + 2],
            padded[i * 4 + 3],
        ]);
    }

    // ---- Message schedule expansion ----
    // Expand from w[0..15] to w[16..63]
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }

    // ---- Compression function ----
    // Initialize working variables with the custom IV
    let (mut a, mut b, mut c, mut d) = (
        INITIAL_HASH_VALUES[0],
        INITIAL_HASH_VALUES[1],
        INITIAL_HASH_VALUES[2],
        INITIAL_HASH_VALUES[3],
    );
    let (mut e, mut f, mut g, mut h) = (
        INITIAL_HASH_VALUES[4],
        INITIAL_HASH_VALUES[5],
        INITIAL_HASH_VALUES[6],
        INITIAL_HASH_VALUES[7],
    );

    // 64 compression rounds
    for i in 0..64 {
        // Σ1(e) = ROTR(e,6) ⊕ ROTR(e,11) ⊕ ROTR(e,25)
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        // Ch(e,f,g) = (e ∧ f) ⊕ (¬e ∧ g)
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(ROUND_CONSTANTS[i])
            .wrapping_add(w[i]);

        // Σ0(a) = ROTR(a,2) ⊕ ROTR(a,13) ⊕ ROTR(a,22)
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        // Maj(a,b,c) = (a ∧ b) ⊕ (a ∧ c) ⊕ (b ∧ c)
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);

        // State rotation
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    // Add the initial vector
    a = a.wrapping_add(INITIAL_HASH_VALUES[0]);
    b = b.wrapping_add(INITIAL_HASH_VALUES[1]);

    // ---- Output ----
    // SHA-256 standard output is big-endian: a's byte order is [MSB, ..., LSB]
    // RouterOS reads the first 4 bytes little-endian → equivalent to a byte reversal
    let a_be = a.to_be_bytes();
    let sid_lo = u32::from_le_bytes(a_be);

    // sid_hi = the most significant big-endian byte of the second u32 (b)
    let sid_hi = b.to_be_bytes()[0];

    (sid_lo, sid_hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the known correct value for the 6G VMware scheme
    #[test]
    fn test_6g_known_hash() {
        let mut buf = [0x20u8; 40];
        buf[..20].copy_from_slice(b"00000000000000000001");
        buf[20..36].copy_from_slice(b"VMware Virtual I");
        buf[36..40].copy_from_slice(&0x1800u32.to_le_bytes());

        let (sid_lo, sid_hi) = hash_40(&buf);
        assert_eq!(sid_lo, 0x0B49EC2E, "sid_lo mismatch");
        assert_eq!(sid_hi, 0x35, "sid_hi mismatch");
    }
}
