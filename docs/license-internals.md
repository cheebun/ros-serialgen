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

### 3.5 Binary-Level Verification

Sections 3.2-3.4 were re-verified directly against `tools/bin/keyman_7.23.2` (ELF 32-bit LSB, Intel 80386, stripped) via raw byte search + `objdump -d` disassembly, rather than relying solely on repeated real-hardware test results. Every value below cites the actual file offset / instruction.

**SHA-256 constant tables -- exact byte match**

Searching the binary for the IV and round-constant words (as little-endian byte sequences) finds them contiguous in `.rodata`:

| Constant | File offset | Runtime VMA |
|---|---|---|
| `IV[0] = 0x5B653932` | `0xc7e0` | `0x080547E0` |
| `IV[1] = 0x7B145F8F` | `0xc7e4` | `0x080547E4` |
| `K[0] = 0x0548D563` | `0xc800` | `0x08054800` |
| `K[1] = 0x98308EAB` | `0xc804` | `0x08054804` |

(`VMA = file_offset + 0x08048000`, confirmed by cross-checking against the `.rodata` section's own VMA range from `objdump -h`.) The disassembly loads these addresses directly: `movl $0x80547e0, %esi` / `movl $0x8054800, %edi` at the entry of the compression-round function.

**Round function -- rotate amounts match `Σ0/Σ1/σ0/σ1`**

The compression loop uses `rorl $0x6` / `rorl $0xb` / `roll $0x7` (`roll 7` == `rotr 25`) for `Σ1(e)`, and `roll $0xe` (`rotr 18`) / `rorl $0x7` / `shrl $0x3` for `σ0`, matching `sha256.rs`'s `rotate_right(6/11/25)` and `(7/18/3)` exactly.

**`mbr_val` formula -- confirmed instruction-by-instruction**

A dedicated function calls the SHA-256 wrapper with `ecx=0xa` (10 bytes -- the `mbr_10` identity seed), then:

```asm
movzwl -0x28(%ebp), %esi   ; esi = digest[0:2] as u16  (sha_val)
testl  %esi, %esi
jne    skip
movl   $0x1eef, %esi       ; if sha_val == 0, substitute 0x1EEF (undocumented edge case)
skip:
movl   %ebx, %eax
calll  <checksum_fn>       ; 0x804bf80
xorl   %esi, %eax          ; mbr_val_16 = sha_val XOR chksum
```

The checksum function at `0x804bf80` reads five little-endian `u16` words, sums them, and returns `~sum & 0xFFFF` (short-circuited to `0xFFFF` directly if every word is zero -- numerically a no-op, since `~0 & 0xFFFF` is `0xFFFF` anyway). This is exactly `chksum = NOT(sum_of_5_LE_uint16(mbr_10)) & 0xFFFF` from 3.2, confirmed opcode-by-opcode rather than inferred.

**SOFTWARE ID input buffer -- confirmed 40-byte layout**

At `0x8050a9b`: `movl $0x28, %ecx` (0x28 = 40) immediately precedes a call to the SHA-256 wrapper -- this is the SOFTWARE ID hash from 3.1/3.2. Walking backward from this call, two NUL-to-space sanitization loops bound the buffer:

```
offset 0x00-0x13 (20 bytes): first loop bound  -- the serial field
offset 0x14-0x23 (16 bytes): second loop bound -- the model field
```

i.e. the buffer is laid out as `serial[20] || model[16]` starting at offset 0, immediately followed by `sector_val` to reach 40 bytes total -- an exact match for `main.rs`'s `SERIAL_LEN=20`, `MODEL_LEN=16`, `INPUT_LEN=40`, and field order. The model truncation documented in 3.1 is not a limitation of this project's tooling; it is the fixed size of keyman's own on-stack buffer, visible directly in its machine code.

A related debug format string found in the binary confirms the same field widths independently of the disassembly: `"%s: hdd-model='%.16s' s='%.20s' sz=%d MB"` -- `%.16s` for model, `%.20s` for serial.

**Empirically confirmed on a real VM**: PVE VM running the ER1G-WVEL MBR (2G, `serial=00000000000000000001`) was booted twice -- once with `model=VMware Virtual IDE Hard Drive` (29 chars) and once with `model=VMware Virtual I` (the 16-char truncation) -- both produced an identical `/system license print` result. This matches the disassembly's prediction exactly: bytes past the 16th are read from the disk's IDENTIFY response but never copied into keyman's hash input buffer, so they cannot affect the SOFTWARE ID.

### 3.6 marker and reserved: generated from identity, not just checked

Sections 3.1-3.5 establish that `mbr_val` (used in the SOFTWARE ID hash) is computed from the 10-byte identity alone (`0x100-0x109`) -- marker (`0x10A-0x10B`) and reserved (`0x10C-0x10F`) are never fed into that hash. Two real-VM tests confirmed marker/reserved still matter for validation even though they don't affect the SOFTWARE ID: taking a known-good MBR (HCC0-4FJR's real identity + real signature) and changing only marker (to `FFFF`) or only reserved (to `DEADBEEF`) both broke validation -- `/system license print` still showed the *correct* `software-id: HCC0-4FJR` (proving the hash truly doesn't use these bytes), but fell back to a 24-hour trial (`expires-in`) in both cases.

The disassembly explains why. The "key import" code path (`RouterOS decodes the key text ... generates the identity region`, §4 below) contains this sequence, found by tracing callers of the `mbr_val` function from 3.5:

```asm
movl  $0x0, 0xc(%eax)      ; reserved (0x10C, 4 bytes) := 0
calll 0x804f490            ; compute (sha_val XOR chksum) from the 10-byte identity -- same
                            ; computation as mbr_val, but the caller has NOT yet applied &0x7FF
movw  %ax, 0x10a(%esi)     ; write the raw *unmasked* 16-bit result directly to marker (0x10A)
```

So marker isn't an independent value or a fixed magic constant -- **it's the very same identity-derived hash used for `mbr_val`, just written out in full (16 bits) instead of masked down to 11 bits (`& 0x7FF`) for the mix.** `mbr_val = marker & 0x7FF`. reserved is unconditionally zeroed by this same code path, which is why it's `00000000` in every example seen so far.

This can be verified independently of the disassembly, using only the (already public) mbr_val formula from 3.2, computing the full unmasked value and reading it back as a little-endian 16-bit word:

```python
raw_value = sha_val ^ chksum   # same as mbr_val's inputs, but skip the "& 0x7FF"
marker    = raw_value.to_bytes(2, 'little')
```

Checked against every known real-device identity in this project:

| Device | identity-derived raw value | predicted marker (LE) | actual marker | match |
|---|---|---|---|---|
| standard (all-zero) | `0xE8BD` | `BDE8` | `BDE8` | yes |
| WUB2-EYCK | `0x89A3` | `A389` | `A389` | yes |
| HHJH-UFWL | `0x7EFA` | `FA7E` | `FA7E` | yes |
| TI09-7WK3 | `0x9864` | `6498` | `6498` | yes |
| ZJ3M-ESHW | `0x0875` | `7508` | `7508` | yes |
| ER1G-WVEL | `0x53D3` | `D353` | `D353` | yes |
| HCC0-4FJR | `0x2033` | `3320` | `BDE8` | **no** |
| 4MZF-SFTR | `0x42A4` | `A442` | `A442` | yes |

7 of 8 match exactly (a full 16-bit exact match by chance has probability 1/65536 per device -- seven independent exact matches rules out coincidence). 4MZF-SFTR's row was corrected after the original recorded data (`identity=...055A`, `marker=4442`) turned out to compute a completely different SOFTWARE ID (`1EGG-HMKR`, not `4MZF-SFTR`) when boot-tested on a real VM -- a transcription error from this project's earliest phase (Experiment 1's "five parameter sets from a forum post", see `experiments.md`), not a formula exception. Brute-forcing the 2048 possible `mbr_val` values against the target SOFTWARE ID (keeping serial/model/size fixed) found the one value that works, then searching single-hex-digit edits of the recorded identity found the fix (`A` misread as `5` in one position, and separately as `4` in the recorded marker) -- confirmed on a real VM after correction.

HCC0-4FJR remains the sole confirmed exception -- from this project's earliest phase as well, but (unlike 4MZF-SFTR) independently boot-verified multiple times as a real, working license with marker `BDE8`, so it isn't a transcription error. The most plausible explanation is that it was licensed by an older RouterOS/keyman version whose key-import routine derived marker differently, while the SOFTWARE ID hash itself (which every license, old or new, must still satisfy to keep working) stayed stable across versions. This isn't confirmed, just the most plausible explanation given the available evidence -- flagged here rather than asserted.

Practical implication: when writing an MBR for a real-device signature (§6 below / `docs/automated-install.md`), write `marker`/`reserved` exactly as captured from that device -- `BDE800000000` is simply what the formula produces for a large share of real captures (and for the standard all-zero collision-search identity), not a fixed constant every device is guaranteed to share. One of eight known real devices in this project proves the exception.

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
