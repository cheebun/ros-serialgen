# RouterOS L6 Licensing Mechanism -- Technical Analysis

Analysis of the RouterOS x86 licensing system based on reverse engineering of the keyman binary (RouterOS 7.23.2) and verification on PVE hardware.

---

## 1. Evolution of the Approach

### Phase 1: Fixed-Parameter Verification

Starting from a forum post (chaohucity), five known parameter sets (6G/8G/32G/42G/64G) were collected and verified on PVE. Critical discovery: the installer overwrites MBR bytes `0x10A-0x10B`, so license data must be written **after** installation.

### Phase 2: Custom Disk Sizes

Testing 16G with 6G parameters produced a different SOFTWARE ID, proving that disk size participates in the computation. Arbitrary disk sizes require finding new collisions.

### Phase 3: Algorithm Recovery

The SOFTWARE ID algorithm was recovered through:

1. Analysis of `keygen_x86` from MikroTikPatch (Go-compiled keyman replacement)
2. Extraction of the real `keyman` binary from RouterOS squashfs (ELF 32-bit, 55 KB)
3. Ghidra-level disassembly of the computation flow
4. Differential analysis across configurations to verify correctness

### Phase 4: Collision Search

A multi-threaded brute-force searcher was built (initially in C, then rewritten in Rust with AVX-512 SIMD). Three C-version bugs were fixed before the first successful 16G collision was found and verified on real PVE hardware.

### Phase 5: Key Text Recovery

MTBase64 encode/decode was obtained from the MTLic project. Key text was found to be `MTBase64Encode(MBR[0x110:0x150])`. All four key texts were recovered, enabling both MBR write and key import activation methods.

---

## 2. Algorithm Details

### SOFTWARE ID Computation

```
Inputs:
  serial     = 20 bytes (disk serial number, from ATA IDENTIFY or QEMU serial=)
  model      = 16 bytes (disk model name, truncated or space-padded to 16)
  sector_val = 4 bytes  (total_sectors >> 11, rounded to 4-bit boundary)

Steps:
  1. buf[40] = serial[20] || model[16] || LE32(sector_val)
  2. digest  = MikroTik_SHA256(buf)        // non-standard IV and K constants
  3. hash_lo = digest[0:4] as LE uint32
     hash_hi = digest[4] | 0x100
  4. mbr_val = (SHA256(MBR[0x100:0x10A])[0:2] XOR checksum(MBR[0x100:0x10A])) & 0x7FF
     mix     = mbr_val * 0x3FF800F
  5. final   = (hash_hi XOR mix_hi) << 32 | (hash_lo XOR mix_lo)
  6. SOFTWARE_ID = Base35Encode(final)
     Alphabet: TN0BYX18S5HZ4IA67DGF3LPCJQRUK9MW2VE
     Format: XXXX-XXXX
```

### MikroTik SHA-256

Standard 64-round Merkle-Damgard construction with non-standard constants:

```
IV = { 0x5B653932, 0x7B145F8F, 0x71FFB291, 0x38EF925F,
       0x03E1AAF9, 0x4A2057CC, 0x4CAF4DD9, 0x643CC9EA }

K[64] = { 0x0548D563, 0x98308EAB, 0x37AF7CCC, ... }
```

Full K table matches `MIKRO_SHA256_K` in MikroTikPatch's `mikro.py`.

### sector_val Rounding

Disk size is lossily compressed to reduce the SOFTWARE ID's sensitivity to exact sector counts:

```
raw       = total_sectors >> 11
bits      = highest_set_bit(raw)
if bits <= 4: sector_val = raw
else: sector_val = ceil(raw / 2^(bits-4)) * 2^(bits-4)

Examples:
  6G:   0xC00000 sectors >> 11 = 0x1800  -> 0x1800  (already aligned)
  8G:   0xEBFD10 sectors >> 11 = 0x1D7F  -> 0x1E00  (rounded up)
  64G:  0x7740AB0 sectors >> 11 = 0xEE81 -> 0xF000  (rounded up)
```

---

## 3. MBR Data Structure

The first 512 bytes of the disk contain the license region at offsets `0x100-0x14F`:

```
Offset       Size   Purpose                          Notes
-----------  -----  -------------------------------  --------------------------
0x0B3-0x0FF  77B    MBR random bytes                 All zeros in our scheme
0x100-0x109  10B    License identity seed             Participates in SOFTWARE ID
0x10A-0x10B   2B    License marker                   Must be BD E8 (installer resets to FF FF)
0x10C-0x10F   4B    System counter                   Incremented each boot; no impact
0x110-0x14F  64B    KCDSA signature                  The actual license proof
```

The identity region (`0x100-0x10F`) and signature region (`0x110-0x14F`) are functionally independent. The signature binds to the SOFTWARE ID, not to specific disk parameters. See [license-internals.md](license-internals.md) for the full analysis.

---

## 4. Collision Search Mechanism

### Probability Analysis

```
SOFTWARE ID space:  ~40 bits
Serial space:       20 bytes = 160 bits (charset: [0-9A-Za-z-])
Known signatures:   4
Probability/hash:   4 / 2^40 ~ 3.6 * 10^-12
Search speed:       ~40M hashes/sec (16-core C), higher with AVX-512 Rust
Expected time:      2^40 / (4 * 40M) ~ 7000 sec ~ 2 hours
```

### Scaling with More Signatures

Search time is inversely proportional to the number of known signatures:

| Known Signatures | Probability/Hash | Estimated Time (16 cores) |
|---|---|---|
| 4 (current) | 4 / 2^40 | ~2 hours |
| 10 | 10 / 2^40 | ~48 minutes |
| 100 | 100 / 2^40 | ~5 minutes |
| 1000 | 1000 / 2^40 | ~30 seconds |

Each new RouterOS image or licensed disk yields a new SOFTWARE ID + signature pair, linearly improving search speed.

### Why MBR-Only Modification Fails

The MBR identity region contributes only 11 bits (`mbr_val`), giving 2048 possible SOFTWARE IDs per serial/model/size combination. Matching one of 4 targets from 2048 candidates would require ~270 million known signatures for 50% success -- impractical.

---

## 5. Why Private Key Recovery Is Impossible

RouterOS uses **EC-KCDSA over Curve25519** for license signing:

| Property | SOFTWARE ID Collision | Private Key Recovery |
|---|---|---|
| Target bits | ~40 | ~252 (Curve25519 ECDLP) |
| Search space | ~10^12 | ~10^75 |
| Time at 40M/s | ~2 hours | ~10^59 years |
| Feasibility | Fully feasible | Mathematically impossible |

Our method does not attack the signature scheme. It reuses existing valid signatures under different disk parameters that happen to produce the same SOFTWARE ID. This is a protocol-level weakness (the signature covers only the SOFTWARE ID, not the full disk identity), not a cryptographic break.

### Alternative Approaches

| Approach | Requires | Modifies System | Status |
|---|---|---|---|
| Collision search (this project) | No private key | No | In use |
| MikroTikPatch (replace public key) | Own key pair | Modifies kernel | Rejected |
| Private key recovery | -- | -- | Impossible |

---

## 6. Security Boundaries

| Attack Surface | Bits of Security | Feasible |
|---|---|---|
| SOFTWARE ID collision (vary serial) | ~40 | Yes (~2 hours) |
| SOFTWARE ID collision (vary MBR only) | ~11 vs ~40 | No (insufficient freedom) |
| KCDSA private key recovery | ~252 | No |
| Public key replacement | 0 (direct patch) | Yes (requires firmware mod) |

---

## 7. Future Optimization Directions

| Direction | Speedup | Difficulty |
|---|---|---|
| Collect more signatures (more collision targets) | Linear | Low |
| AVX-512 SIMD parallel SHA-256 (current Rust impl) | 4-16x | Done |
| GPU acceleration (needs discrete GPU) | 10-100x | High |
| FPGA | 100x+ | Very high |
| Precomputed tables for common sizes | Instant | One-time cost |

The most practical improvement is collecting more signatures from RouterOS images found online. At 100 known signatures, search time drops from ~2 hours to ~5 minutes.
