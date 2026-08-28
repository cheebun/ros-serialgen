# RouterOS L6 Authorization Mechanism — Complete Analysis

## 1. Overall Approach Evolution

### Starting Point: Forum Post
Starting from a post by chaohucity, learned that RouterOS licensing binds to disk parameters (name, serial number, size).
Initial approach: mass-produce USB drives with fixed parameters → clone/install → write authorization data to MBR.

### Stage 1: Fixed-Scheme Verification
Collected 5 sets of known parameters (6G/8G/32G/42G/64G), all verified successfully on PVE.
Discovered a key operational constraint: write MBR after install, never before (the installer overwrites 0x10A-0x10B).

### Stage 2: Exploring Custom Disk Sizes
Question: can arbitrary disk sizes (e.g. 16G/100G/500G) be used?
Verification: 16G + 6G parameters → SOFTWARE ID changed → disk size participates in the computation.

### Stage 3: Reverse Engineering the SOFTWARE ID Algorithm
- Analyzed keygen_x86 (Go-compiled, replacement keyman) from the MikroTikPatch project
- Extracted the real keyman (ELF 32-bit, 55KB) from the RouterOS squashfs
- Reconstructed the full algorithm via Ghidra-level disassembly
- Verified algorithm correctness via differential analysis (same disk, different serial/model)

### Stage 4: Collision Search
- Wrote a multi-threaded brute-force searcher (initial C implementation)
- Fixed 3 bugs along the way (snprintf null terminator, byte order, hardcoded mbr_val)
- 32 cores × ~1 hour found a 16G collision
- Verified on real PVE hardware
- Later rewritten in Rust as **ros-serialgen**, with AVX-512 SIMD acceleration for production use

### Stage 5: Key Text Recovery
- Obtained MTBase64 encode/decode from the MTLic project
- Discovered Key text = MTBase64Encode(MBR[0x110:0x150])
- Successfully recovered Key text from all MBR data
- Both activation methods now work: MBR write / Key import

---

## 2. Algorithm Details

### SOFTWARE ID Computation

```
Input:
  serial     (20 bytes, disk serial number)
  model      (16 bytes, disk model, truncated/space-padded)
  sector_val (4 bytes, total_sectors >> 11, rounded up to 4 significant bits)

Steps:
  1. buf[40] = serial[20] + model[16] + LE32(sector_val)
  2. digest = custom_sha256(buf)    // non-standard IV + K
  3. hash_lo = digest[0:4] as LE uint32
     hash_hi = digest[4] | 0x100
  4. mbr_val = (sha256(MBR[0x100:0x10A])[0:2] ^ checksum(MBR[0x100:0x10A])) & 0x7FF
     mix = mbr_val × 0x3FF800F
  5. final = (hash_hi ^ mix_hi) << 32 | (hash_lo ^ mix_lo)
  6. SOFTWARE_ID = base35_encode(final)    // table: TN0BYX18S5HZ4IA67DGF3LPCJQRUK9MW2VE
```

### Custom SHA-256 Parameters

IV:
```
0x5B653932, 0x7B145F8F, 0x71FFB291, 0x38EF925F
0x03E1AAF9, 0x4A2057CC, 0x4CAF4DD9, 0x643CC9EA
```

K[64]: identical to `MIKRO_SHA256_K` in MikroTikPatch's `mikro.py`.

### sector_val Rounding Rule

```
raw = total_sectors >> 11
bits = highest_set_bit(raw)
if bits <= 4: return raw
shift = bits - 4
rounded = (raw >> shift + (remainder ? 1 : 0)) << shift
```

### MBR Data Structure

```
Offset      Purpose
0xB3-0xFF   MBR random bytes (participate in SOFTWARE ID, but all-zero in experiments)
0x100-0x10F Authorization header (participates in MBR mixing + authorization validation)
0x110-0x14F Authorization signature (64 bytes, KCDSA over Curve25519)
```

### Key Text Format

```
Key text = MTBase64Encode(MBR[0x110:0x150])

MTBase64: standard alphabet, but LSB-first bit order (differs from standard Base64)
```

---

## 3. Collision Search Mechanism

### Principle

```
hash(serial + model + sector_val) is fixed → ~40 bit output
MBR mix is fixed when MBR[0x100-0x109] is all-zero → mbr_val = 0x0BD
Final SOFTWARE ID is determined by hash XOR mix

Search: iterate over serial values, find a hash collision against one of the known SOFTWARE IDs
Probability: N / 2^40 per try (N = number of known signatures)
Speed: ~2000M hash/s (Rust AVX-512 SIMD, 8-core) — historically ~40M hash/s (16-core scalar C)
Time: seconds to minutes with ros-serialgen, vs. hours with the old scalar approach
```

### Collision Count vs. Search Speed

**More known signatures = faster search.**

| Known Key Count | Collision Probability/hash | Est. Time (~2000M hash/s, 8-core AVX-512) |
|---|---|---|
| 4 (current) | 4/2^40 | seconds |
| 10 | 10/2^40 | sub-second to seconds |
| 100 | 100/2^40 | near-instant |
| 1000 | 1000/2^40 | near-instant |

*(Historical reference: with the original 16-core scalar C implementation at ~40M hash/s, 4 known keys took ~2 hours, 1000 keys took ~30 seconds.)*

Ways to obtain more Keys:
- Collect more MBR data from additional RouterOS images/posts online
- Every new (serial, model, disk_size) + MBR combination = a new SOFTWARE ID + signature pair
- Recover Key text via `Key = MTBase64Encode(MBR[0x110:0x150])`

**Each additional signature linearly increases search speed.**

---

## 4. Can the Public/Private Key Be Collided?

### Answer: No.

RouterOS uses **EC-KCDSA over Curve25519** to sign authorizations:

```
Signature verification:
  RouterOS built-in public key → verify signature → authorization is valid if it passes

Signature generation:
  requires the private key → held only by MikroTik
```

### Why It Cannot Be Collided

| Comparison | SOFTWARE ID Collision | Private Key Collision |
|---|---|---|
| Target | ~40 bit hash | ~252 bit discrete log (Curve25519) |
| Search space | 2^40 ≈ 10^12 | 2^252 ≈ 10^75 |
| Speed (8-core AVX-512) | ~2000M/s | ~2000M/s |
| Time required | seconds to minutes | **~10^59 years** (10^49 times the age of the universe) |
| Feasibility | Fully feasible | Mathematically impossible |

Curve25519's security rests on the elliptic curve discrete logarithm problem (ECDLP); no known polynomial-time algorithm can break it. Even with a quantum computer (Shor's algorithm), thousands of logical qubits would be required — far beyond current quantum computing capability.

### Alternatives

| Approach | Requires Private Key? | Modifies the System? | Our Choice |
|---|---|---|---|
| **Collision search** (current) | No | No | Yes |
| MikroTikPatch (replace public key) | Uses own private key | Modifies kernel | No — undesired approach |
| Break the private key | — | — | No — infeasible |

---

## 5. Complete Results

### Verified Schemes

| Disk Size | Serial | Model | SOFTWARE ID | Key | MBR | Status |
|---|---|---|---|---|---|---|
| 6G | `00000000000000000001` | `VMware Virtual IDE Hard Drive` | TI09-7WK3 | Yes | Yes | Verified |
| 8G | `HKHYPO14032703B0778` | `SSD08G` | 4MZF-SFTR | Yes | Yes | Verified |
| 16G | `00000000202155543391` | `ROS16G` | 4MZF-SFTR | Yes | Yes | **Collision search** |
| 32G | `SZHYPO14090903D0164` | `SSD32G` | HHJH-UFWL | Yes | Yes | Verified |
| 42G | `G4HQT594JN8VLY0FGN9` | `n4X7W6eSOxyxUhOd` | TI09-7WK3 | Yes | Yes | Verified |
| 64G | `HYSSD-20160419B79028` | `SSD64G2016` | C7CU-PGT9 | Yes | Yes | Verified |

### Standard MBR Write Format

```
0x100: 00 00 00 00 00 00 00 00 00 00 BD E8 00 00 00 00  ← fixed header
0x110: <64-byte signature, looked up from signature table>  ← keyed by SOFTWARE ID
```

### 4 Key Texts

All recovered from MBR via `MTBase64Encode(MBR[0x110:0x150])` and verified.

### Tool: ros-serialgen

```bash
ros-serialgen search -s <disk_GB> -t <threads>
```

Rust implementation with AVX-512 SIMD, achieving ~2000M hash/s on an 8-core machine.

---

## 6. Future Optimization Directions

| Direction | Effect | Difficulty |
|---|---|---|
| Collect more Keys (increase collision targets) | Linear speedup | Low |
| AVX-512 SIMD parallel SHA-256 | ~50x over scalar (already implemented) | Medium |
| GPU (requires discrete GPU, integrated GPUs too weak) | 10-100x | High |
| FPGA | Very fast | Very high |
| Precomputed lookup tables for common sizes | Instant | One-time cost |

### Most Practical Speedup: Collect More Keys

Every new RouterOS image/licensed disk contains an MBR from which a new SOFTWARE ID + signature can be extracted.
Growing from 4 to 100 known Keys reduces search time by roughly 25x.

---

## 7. Key Lessons

1. **Reverse-engineering results must be verified end-to-end** — never blindly trust a sub-agent's analysis.
2. **C code must be cross-validated against Python** — byte order, encoding, and boundary conditions all matter.
3. **Every assumption must be verified experimentally** — the assumption "write MBR before installing the system" was disproven by experiment.
4. **The simplest method is often best** — Key text import is simpler than writing the MBR directly.
5. **Public-key cryptography cannot be broken** — don't waste time pursuing infeasible directions.
