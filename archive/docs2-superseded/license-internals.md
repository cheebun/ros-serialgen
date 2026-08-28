# RouterOS x86 License Internals — Deep Dive

> Based on reverse engineering of the RouterOS 7.23.2 keyman binary and real-machine verification on PVE.

---

## 1. License Architecture Overview

The RouterOS x86 licensing system is built from three layers:

```
┌─────────────────────────────────────────────────────┐
│                 Verification Layer                   │
│                                                      │
│   Disk hardware params ─┐                            │
│                          ├──→ SOFTWARE ID ←──→ Signature check │
│   MBR authorization area ─┘        ↑               ↑          │
│                           │               │          │
│                     Identity            MikroTik public key    │
└─────────────────────────────────────────────────────┘
```

- **Identity layer**: disk hardware parameters (serial / model / size) + MBR data → compute a unique SOFTWARE ID
- **Authorization layer**: a KCDSA over Curve25519 digital signature attests that this SOFTWARE ID is granted L6-level authorization
- **Verification layer**: RouterOS's built-in public key verifies the signature at boot

All three layers are required: identity determines the ID, the signature proves authorization, and the public key guarantees the signature cannot be forged.

---

## 2. MBR Authorization Area Structure

Within the disk's first sector (512 bytes), the 80-byte range 0x100-0x14F is reserved for authorization:

```
Offset         Size    Purpose                   Role
────────────────────────────────────────────────────────
0x100-0x109    10B     Authorization random seed  → input to SOFTWARE ID computation
0x10A-0x10B     2B     Authorization marker (BD E8) → reset to FF FF by the installer
0x10C-0x10F     4B     System counter            → incremented by RouterOS on every boot
────────────────────────────────────────────────────────
0x110-0x14F    64B     KCDSA digital signature   → the core of authorization verification
```

### Independence of the Two Functional Zones

```
┌── Identity Zone (0x100-0x10F) ──┐    ┌── Signature Zone (0x110-0x14F) ──┐
│                                  │    │                                    │
│  Affects the SOFTWARE ID value  │    │  Verifies whether authorization   │
│  Different disk params →        │    │  is valid                         │
│    different value              │    │  Bound only to the SOFTWARE ID    │
│  Auto-generated on Key import   │    │  = binary form of the Key text    │
│                                  │    │                                    │
│  Variable, unique per machine   │    │  Fixed, identical for the same ID │
└──────────────────────────────────┘    └────────────────────────────────────┘
```

**Experimental evidence:**

| Scheme | Identity zone (0x100-0x10F) | Signature zone (0x110-0x14F) | SOFTWARE ID |
|---|---|---|---|
| 6G | `00...BD E8...` | `E67A8F47...` | TI09-7WK3 |
| 42G | `F0A25846...` | `E67A8F47...` (identical) | TI09-7WK3 |
| 8G | `48065595...` | `080342D3...` | 4MZF-SFTR |
| 16G | `00...BD E8...` | `080342D3...` (identical) | 4MZF-SFTR |

**Conclusion: the signature is bound only to the SOFTWARE ID, independent of the identity zone and disk parameters.**

---

## 3. SOFTWARE ID Computation Algorithm

### 3.1 Inputs

| Input | Source | Size |
|---|---|---|
| serial | disk firmware / QEMU `serial=` | 20 bytes |
| model | disk firmware / QEMU `model=` | 16 bytes (truncated) |
| sector_val | `total_sectors >> 11`, rounded up to a 4-bit boundary | 4 bytes LE |
| mbr_10 | MBR[0x100:0x10A] | 10 bytes |

### 3.2 Computation Flow

```
Phase 1: Hardware Fingerprint
───────────────────────────────────
  buf[40] = serial[20] ∥ model[16] ∥ LE32(sector_val)
  digest  = MikroTik_SHA256(buf)
  hash_lo = digest[0:4] as LE uint32
  hash_hi = digest[4] | 0x100

Phase 2: MBR Mixing
───────────────────────────────────
  sha_val = MikroTik_SHA256(mbr_10)[0:2] as LE uint16
  chksum  = bitwise_NOT(sum_of_5_LE_uint16(mbr_10)) & 0xFFFF
  mbr_val = (sha_val ⊕ chksum) & 0x7FF          ← only 11 bits significant
  mix     = mbr_val × 0x3FF800F                   ← expanded to 43 bits

Phase 3: Synthesis
───────────────────────────────────
  final_lo = hash_lo ⊕ mix_lo
  final_hi = hash_hi ⊕ mix_hi
  final    = (final_hi << 32) | final_lo

Phase 4: Encoding
───────────────────────────────────
  SOFTWARE ID = Base35Encode(final)
  Alphabet: TN0BYX18S5HZ4IA67DGF3LPCJQRUK9MW2VE
  Format: XXXX-XXXX (hyphen inserted at position 4)
```

### 3.3 MikroTik SHA-256

A standard SHA-256 structure (64-round Merkle-Damgård construction), but with non-standard constants:

```
IV = { 0x5B653932, 0x7B145F8F, 0x71FFB291, 0x38EF925F,
       0x03E1AAF9, 0x4A2057CC, 0x4CAF4DD9, 0x643CC9EA }

K[64] = { 0x0548D563, 0x98308EAB, 0x37AF7CCC, ... }
```

### 3.4 sector_val Rounding Rules

Disk size is encoded via sector count, using lossy compression to reduce the ID's sensitivity to exact capacity:

```
raw       = total_sectors >> 11
bits      = highest_set_bit(raw)
sector_val = round up to a (bits - 4)-bit boundary

Examples:
  6G:   0xC00000 sectors → >>11 = 0x1800   → 0x1800 (already aligned)
  8G:   0xEBFD10 sectors → >>11 = 0x1D7F   → 0x1E00 (rounded up)
  64G:  0x7740AB0 sectors → >>11 = 0xEE81  → 0xF000 (rounded up)
```

---

## 4. License Forms

### 4.1 MBR Binary

```
Disk offset 0x100-0x14F, 80 bytes total
Write method: dd of=/dev/nbd0 bs=1 seek=256 count=80
```

### 4.2 Key Text File

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
<MTBase64-encoded 64-byte signature, split across two lines>
-----END MIKROTIK SOFTWARE KEY--------------

Import method: /system license import file-name=license.key
```

### 4.3 RouterOS Internal Display

```
/system license print
  software-id: XXXX-XXXX
       nlevel: 6
     features:
```

### 4.4 Conversion Relationships Between the Three Forms

```
         MTBase64Decode(Key text)
Key text ─────────────────────────→ MBR[0x110:0x150] (64-byte signature)
         MTBase64Encode(MBR signature)
MBR signature ─────────────────────────→ Key text

On Key import, RouterOS automatically performs:
  1. Decode Key text → 64-byte signature
  2. Generate the identity zone (0x100-0x10F) from current disk parameters
  3. Write the full 80 bytes to the MBR
  4. Verification takes effect after reboot
```

**The Key text does not contain the identity zone — RouterOS generates it automatically on import.**

---

## 5. Authorization Verification Flow

RouterOS performs the following verification on every boot:

```
Step 1  Read disk serial / model / size (via ATA IDENTIFY or a RouterOS ioctl)
Step 2  Read MBR[0x100:0x10A] (the 10-byte authorization seed)
Step 3  Compute SOFTWARE ID = f(serial, model, sectors, mbr_seed)
Step 4  Read MBR[0x110:0x150] (the 64-byte KCDSA signature)
Step 5  Verify the signature with the built-in public key: does this signature attest that this SOFTWARE ID holds L6 authorization?
Step 6  Verification passes → nlevel: 6 | fails → 24-hour trial
```

### Installer Intervention

The RouterOS installer modifies the MBR:

| Offset | Before install | After install | Impact |
|---|---|---|---|
| 0x10A-0x10B | BD E8 | **FF FF** | Does not affect SOFTWARE ID, but breaks signature verification |
| 0x10C | 00 | **01 or 05** | No effect (outside the 0x100-0x109 range) |

Therefore, **authorization data must be rewritten after installation**.

---

## 6. Collision Search Theory

### 6.1 Why It Is Feasible

```
SOFTWARE ID space   ≈ 40 bits
serial              = 20 bytes = 160 bits of freedom
probability/hash    = 4 / 2^40 ≈ 4 × 10⁻¹² (4 known signatures)
search speed        ≈ 40M hash/sec (16-core CPU)
expected time        ≈ 2^40 / (4 × 40M) ≈ 7000 seconds ≈ 2 hours
```

### 6.2 Why Pure MBR Modification Does Not Work

```
mbr_val       = 11 bits → only 2048 possible SOFTWARE ID values
match probability = 2048 × 4 / 2^40 ≈ 10⁻⁸
Conclusion: ~270 million known signatures would be needed for a 50% success rate
```

### 6.3 Relationship Between Search Speed and Number of Known Signatures

Search time is inversely proportional to the number of known signatures:

```
 4 signatures    ───→  ~2 hours
10 signatures    ───→  ~48 minutes
100 signatures   ──→  ~5 minutes
1000 signatures  ─→  ~30 seconds
```

---

## 7. Security Boundaries

| Attack surface | Security strength | Feasibility |
|---|---|---|
| SOFTWARE ID collision (modify serial) | ~40 bits | Yes — ~2 hours |
| SOFTWARE ID collision (modify MBR only) | ~11 bits vs 40 bits | No — insufficient freedom |
| KCDSA private key recovery | ~252 bits (Curve25519) | No — mathematically infeasible |
| Public key replacement (MikroTikPatch) | 0 bits (direct replacement) | Yes — but requires firmware modification |

The true security of the RouterOS licensing system rests on the KCDSA signature. Our method does not attack the signature itself; instead, it exploits SOFTWARE ID collisions — reusing an existing legitimate signature under different disk parameters. Cryptographically, this is a normal use of a signature (a signature is valid for a message, not for a context), and it represents a protocol-level weakness rather than a cryptographic weakness.
