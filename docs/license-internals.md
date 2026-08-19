# SOFTWARE ID and MBR Deep Dive

Detailed analysis of the SOFTWARE ID computation, MBR license structure, and the collision search principle. Based on reverse engineering of the RouterOS 7.23.2 keyman binary.

---

## 1. Three-Layer Licensing Architecture

```
+-------------------------------------------------------------------+
|                    License Verification Layer                      |
|                                                                    |
|   Disk params (serial, model, size) --+                            |
|                                       +--> SOFTWARE ID             |
|   MBR identity region (0x100-0x109) --+        |                   |
|                                                v                   |
|                                    KCDSA signature (0x110-0x14F)   |
|                                                |                   |
|                                                v                   |
|                                    RouterOS built-in public key    |
|                                    verifies signature at boot      |
+-------------------------------------------------------------------+
```

- **Identity layer**: Disk hardware parameters + MBR seed compute a unique SOFTWARE ID
- **Signature layer**: EC-KCDSA over Curve25519; proves this SOFTWARE ID holds an L6 license
- **Verification layer**: RouterOS embedded public key validates the signature on every boot

---

## 2. MBR License Region Structure

Within the first disk sector (512 bytes), offsets `0x100-0x14F` form the 80-byte license region:

```
Offset       Size   Field                   Role in Licensing
-----------  -----  ----------------------  ------------------------------------------
0x100-0x109  10B    Identity seed           Input to MBR mixing phase of SOFTWARE ID
0x10A-0x10B   2B    License marker          Must be BD E8; installer resets to FF FF
0x10C-0x10F   4B    Boot counter            Incremented by RouterOS each boot; no effect
0x110-0x14F  64B    KCDSA digital signature Core license proof
```

### Independence of Identity and Signature Regions

The two functional regions serve different purposes and are independent:

```
+-- Identity (0x100-0x10F) ---+     +-- Signature (0x110-0x14F) ---+
|                              |     |                               |
|  Affects SOFTWARE ID         |     |  Proves license validity      |
|  Variable per machine        |     |  Binds to SOFTWARE ID only    |
|  Auto-generated on key import|     |  Fixed for a given SOFTWARE ID|
+------------------------------+     +-------------------------------+
```

Experimental evidence (see [experiments.md](experiments.md)):

| Config | Identity (0x100-0x10F) | Signature (0x110-0x14F) | SOFTWARE ID |
|---|---|---|---|
| 6G | `00...BD E8...` | `E67A8F47...` | TI09-7WK3 |
| 42G | `F0A25846...` | `E67A8F47...` (same) | TI09-7WK3 |
| 8G | `48065595...` | `080342D3...` | 4MZF-SFTR |
| 16G (collision) | `00...BD E8...` | `080342D3...` (same) | 4MZF-SFTR |

The signature is reusable across any disk configuration that produces the same SOFTWARE ID.

---

## 3. SOFTWARE ID Computation

### 3.1 Input Preparation

| Input | Source | Size | Charset / Encoding |
|---|---|---|---|
| serial | ATA IDENTIFY or QEMU `serial=` | 20 bytes | `[0-9A-Za-z-]` |
| model | ATA IDENTIFY or QEMU `model=` | 16 bytes | `[0-9A-Za-z- ]`, space-padded right |
| sector_val | `total_sectors >> 11`, rounded | 4 bytes | uint32 LE |
| mbr_10 | MBR[0x100:0x10A] | 10 bytes | raw binary |

SHA-256 input buffer = `serial(20) || model(16) || LE32(sector_val)` = 40 bytes.

### 3.2 Four-Phase Computation

**Phase 1: Hardware fingerprint**

```
buf[40]  = serial[20] || model[16] || LE32(sector_val)
digest   = MikroTik_SHA256(buf)
hash_lo  = digest[0:4] as LE uint32
hash_hi  = digest[4] | 0x100
```

**Phase 2: MBR mixing**

```
sha_val  = MikroTik_SHA256(mbr_10)[0:2] as LE uint16
chksum   = bitwise_NOT(sum_of_5_LE_uint16(mbr_10)) & 0xFFFF
mbr_val  = (sha_val XOR chksum) & 0x7FF        // 11 bits effective
mix      = mbr_val * 0x3FF800F                  // expanded to 43 bits
```

**Phase 3: Combination**

```
final_lo = hash_lo XOR mix_lo
final_hi = hash_hi XOR mix_hi
final    = (final_hi << 32) | final_lo
```

**Phase 4: Encoding**

```
SOFTWARE_ID = Base35Encode(final)
Alphabet: TN0BYX18S5HZ4IA67DGF3LPCJQRUK9MW2VE
Output format: XXXX-XXXX (hyphen at position 4)
```

### 3.3 MikroTik SHA-256 Constants

Standard SHA-256 structure (64-round Merkle-Damgard) with custom IV and K:

```
IV = { 0x5B653932, 0x7B145F8F, 0x71FFB291, 0x38EF925F,
       0x03E1AAF9, 0x4A2057CC, 0x4CAF4DD9, 0x643CC9EA }

K[64] = { 0x0548D563, 0x98308EAB, 0x37AF7CCC, ... }
```

Full K table: identical to `MIKRO_SHA256_K` in MikroTikPatch `mikro.py`.

### 3.4 sector_val Rounding Rule

Disk size is lossily compressed to reduce sensitivity to exact sector counts:

```
raw        = total_sectors >> 11
bits       = highest_set_bit(raw)
if bits <= 4:
    sector_val = raw
else:
    shift = bits - 4
    sector_val = ceil(raw / 2^shift) * 2^shift
```

Examples:

| Disk Size | Total Sectors | raw (>>11) | sector_val | Hex |
|---|---|---|---|---|
| 6G | 0xC00000 | 0x1800 | 0x1800 | Already aligned |
| 8G (7.38 GiB) | 0xEBFD10 | 0x1D7F | 0x1E00 | Rounded up |
| 64G | 0x7740AB0 | 0xEE81 | 0xF000 | Rounded up |

---

## 4. Three Forms of a License

### Form 1: MBR Binary (80 bytes at disk offset 0x100)

```
0x100: 00 00 00 00 00 00 00 00 00 00 BD E8 00 00 00 00   <- fixed header
0x110: <64-byte KCDSA signature>                          <- looked up by SOFTWARE ID
```

Written via `dd` to the raw disk after installation.

### Form 2: Key Text File

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```

Imported via `/system license import file-name=license.key`. RouterOS decodes the key text, generates the identity region from current disk parameters, and writes the full 80 bytes to MBR.

### Form 3: RouterOS Display

```
/system license print
  software-id: XXXX-XXXX
       nlevel: 6
     features:
```

### Conversion Between Forms

```
                    MTBase64Decode
Key text  --------------------------->  MBR[0x110:0x150] (64-byte signature)

                    MTBase64Encode
MBR[0x110:0x150]  ------------------->  Key text
```

MTBase64 uses the standard Base64 alphabet but with LSB-first bit ordering, differing from standard Base64.

The key text encodes only the 64-byte signature. The identity region (`0x100-0x10F`) is **not** included in the key text; RouterOS generates it automatically on import.

---

## 5. License Verification Flow

RouterOS executes this check on every boot:

```
1. Read disk serial, model, size  (ATA IDENTIFY or custom ioctl 0x80044604)
2. Read MBR[0x100:0x10A]          (10-byte identity seed)
3. Compute SOFTWARE ID            = f(serial, model, sectors, mbr_seed)
4. Read MBR[0x110:0x150]          (64-byte KCDSA signature)
5. Verify signature               using built-in Curve25519 public key
6. Pass -> nlevel: 6              Fail -> 24-hour trial mode
```

### Installer Intervention

| Offset | Before Install | After Install | Impact |
|---|---|---|---|
| 0x10A-0x10B | `BD E8` | `FF FF` | Breaks signature verification (not SOFTWARE ID) |
| 0x10C | `00` | `01` or `05` | No impact (outside identity region) |

This is why license data must be written **after** RouterOS installation.

---

## 6. Collision Search Principle

### Why It Works

The SOFTWARE ID is a ~40-bit hash. With 20 bytes of serial (160 bits of entropy), the search space vastly exceeds the output space. Finding a serial that produces one of 4 known SOFTWARE IDs is a birthday-style search with favorable odds:

```
P(match per hash) = 4 / 2^40 ~ 3.6 * 10^-12
Trials to 50% success = ln(2) / P ~ 1.9 * 10^11
At 40M hashes/sec (16 cores): ~4750 sec ~ 80 min
```

### Why MBR-Only Modification Does Not Work

The MBR identity region contributes only 11 bits to the SOFTWARE ID via `mbr_val`. With serial/model/size fixed, only 2048 distinct SOFTWARE IDs are reachable. The probability of hitting one of 4 targets from 2048 options is `4/2048 ~ 0.2%` -- and you cannot iterate, because you need the MBR to remain valid.

### Fixed MBR Header in Collision Scheme

In our collision search, `MBR[0x100:0x10A]` is set to all zeros. This fixes `mbr_val = 0x0BD` and `mix = 0x0BD * 0x3FF800F`. The entire variation comes from iterating the serial field.

---

## 7. Security Analysis

The RouterOS licensing system has three distinct security layers with vastly different strengths:

| Layer | Mechanism | Bits | Breakable |
|---|---|---|---|
| SOFTWARE ID binding | Custom SHA-256 hash | ~40 | Yes (collision search) |
| License signature | EC-KCDSA / Curve25519 | ~252 | No |
| Public key trust | Embedded in firmware | 0 (if replaced) | Yes (firmware patch) |

Our approach exploits the weakest layer (SOFTWARE ID binding) without attacking the cryptographic signature. The signature remains valid because it covers only the SOFTWARE ID, not the full set of disk parameters. This is a protocol design limitation: the signature should ideally bind to the complete hardware identity, not just its hash.
