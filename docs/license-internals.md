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
- **Signature layer**: EC-KCDSA over Curve25519; proves this SOFTWARE ID holds a valid license
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

The MBR identity region contributes only 11 bits to the SOFTWARE ID via `mbr_val`. With serial/model/size fixed, only 2048 distinct SOFTWARE IDs are reachable -- but those 2048 are a fixed subset of the full ~2^40 SOFTWARE ID space, not independently drawn from it. The probability that any of them equals one of N known targets is `2048 * N / 2^40`, not `N/2048`: for this project's 10 known signatures, that's `2048 * 10 / 2^40 ~ 1.9 * 10^-8` per (serial, MBR) combination -- varying only the MBR while holding serial/model/size fixed is exactly as hard as brute-forcing the full serial space (§ Collision Search Mechanism below), not a shortcut. (An earlier version of this section stated a much more optimistic `4/2048 ~ 0.2%`, which conflated "how many values are reachable" with "probability of matching a specific external target" -- corrected here.)

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

---

## 8. ARM32 keyman on virtio-scsi: a platform-specific investigation

**Status: in progress, not yet fully resolved.** This section documents an unexplained discrepancy on a different hardware/platform combination and the disassembly evidence gathered so far. It does not change any conclusion in sections 1-7, which remain fully verified on x86 IDE.

### 8.1 The discrepancy

On a separate ARM64-hardware PVE host, a RouterOS ARM64 VM (`scsihw: virtio-scsi-pci`) was configured with a known-good x86/IDE collision-search combo:

```
serial = 00000000717959548436
model  = SSD1G (via -set device.scsi0.product=SSD1G)
size   = 1 GiB
```

On x86/IDE this combo is expected to produce `4MZF-SFTR` (per `docs/collision-database.md`). On the ARM64 VM it instead produces `3X8K-8K32`.

### 8.2 What was ruled out

- **Not an MBR/signature issue**: the test disk's MBR license region was found completely blank (identity/marker/reserved/signature all zero or garbage) before any test -- simply never written. Writing the standard MBR (`00...BDE8...` + a known signature) did not change the computed SOFTWARE ID.
- **Not the `vendor` SCSI field**: with `serial`/`size` fixed, `vendor=""` vs. the QEMU default `"QEMU"` produced an identical SOFTWARE ID.
- **`product` does participate**: changing `product` from `SSD1G` to `ZZZZZZZZZZZZZZZ` (15 chars) changed the SOFTWARE ID (to `ABAH-C0JJ`), confirming the field is hashed -- but no encoding hypothesis (space/NUL padding, left/right justify, field reordering, big-endian `sector_val`, size-as-raw-MB integer) predicted both `(product, resulting id)` pairs simultaneously from pure computation.
- **Not the SHA-256 core**: the IV and round-constant tables in the ARM32 binary are byte-identical to the x86 binary (see 8.3).

### 8.3 ARM32/Thumb-2, not true ARM64

The RouterOS "arm64" install package's `keyman` binary is actually **32-bit ARMv7-A EABI5 (Thumb-2)** code (`file`: "ELF 32-bit LSB executable, ARM, EABI5 version 1"; `readelf -A`: `Tag_CPU_arch: v7`, `Tag_THUMB_ISA_use: Thumb-2`) -- not native AArch64 -- despite running on an aarch64 kernel/host. Extracted from the guest disk via host-side loop mount (no in-guest shell needed):

```bash
qemu-nbd --read-only --connect=/dev/nbd1 vm-100-disk-1.qcow2
mount -o ro,noload /dev/nbd1p2 /mnt/ros-arm64
dd if=/mnt/ros-arm64/var/pdb/system/image of=/tmp/system.squashfs bs=4096 skip=1
mount -o ro,loop -t squashfs /tmp/system.squashfs /mnt/ros-arm64-sysimg
cp /mnt/ros-arm64-sysimg/nova/bin/keyman /tmp/keyman_arm32
```

Same SHA-256 IV `{0x5B653932, 0x7B145F8F, ...}` located at file offset via `struct.pack('<I', 0x5B653932)` byte search, confirming the hash core is unchanged from x86.

### 8.4 Located the SOFTWARE-ID hash call site

Using the same technique that worked on x86 (search for the `length=40` immediate right before the call to the hash wrapper), found in ARM mode:

```asm
192cc: mov r1, r4
192d0: add r0, sp, #272    @ 0x110    ; buffer pointer
192d4: mov r2, #40         @ 0x28     ; length = 40, confirms this is the SOFTWARE-ID hash
192d8: bl 16ff8                       ; hash wrapper, itself calls the compress fn at 0x16e74
```

### 8.5 Buffer population: two candidate data paths

Immediately before the sanitization loops that fill the 40-byte hash buffer, the code branches on the result of a low-level ioctl:

```asm
19054: add r2, sp, #20
1905c: movw r1, #0x5386        ; ioctl request code 0x5386
19060: bl ioctl@plt            ; attempt: ask the block device directly
19064: cmp r0, #0
19068: bne 19098               ; failure -> fall back

; fallback path (ioctl failed):
1906c: ldr r3, [sp, #20]
19074: ldr r2, [pc, ...]       ; format string
1907c: bl snprintf             ; build a path string
19084: bl fopen                ; open a file
...                             ; then fgets + sscanf, line by line, into string objects
```

Whichever path succeeds, the resulting data is later copied (via length-prefixed `memcpy`) into the same two fixed regions and sanitized:

```asm
1923c: add r3, sp, #24
19244: mov r2, #20             ; 20-byte region: serial
1924c: ldrb r0, [r3], #1
19250: cmp r0, #0
19254: strbeq r1, [r3, #-1]    ; NUL -> ' ' (0x20)
...
19260: add r3, sp, #44         ; 0x2c
19264: mov r1, #16             ; 16-byte region: model
...                             ; identical NUL -> ' ' sanitization
```

**This confirms the buffer layout and NUL-padding convention are identical to x86** (`serial[20] || model[16]`, embedded NULs replaced with spaces) -- the field-encoding hypotheses from 8.2 were correctly ruled out; the encoding scheme itself is not the source of the discrepancy.

### 8.6 The two candidate fallback paths, both dead ends for virtio-scsi

Decoding the literal pool referenced by this code resolved what the `0x5386` ioctl and its two "fallback" branches actually are:

**Path A -- legacy USB-storage `/proc` parsing.** If `ioctl(fd, 0x5386, ...)` succeeds, the code builds the path `/proc/scsi/usb-storage/%u` (literal format string, not a generic per-driver template) via `snprintf`, `fopen`s it, and `fgets`/`sscanf`s each line for `"Serial Number: %19s"`. This is **only** ever going to exist for USB-attached storage -- for a `virtio-scsi-pci` disk this file does not exist, `fopen` returns `NULL`, and the whole primary path is abandoned.

**Path B -- NVMe passthrough.** The fallback function (`0x17764`) turned out to be **NVMe-specific**, not generic SCSI: it `sscanf`s the device's basename against `"nvme%dn%d"`, and issues `ioctl(fd, NVME_IOCTL_ADMIN_CMD, &admin_cmd)` (request code `0xC0484E41` = `_IOWR('N', 0x41, struct nvme_admin_cmd)`, confirmed by decoding the ioctl direction/size/type/nr bitfields) to run an NVMe Identify Controller command, then extracts the 20-byte Serial Number and 40-byte Model Number fields from the response (matching the NVMe spec's SN/MN field widths exactly -- not a coincidence). For a `virtio-scsi` device named e.g. `sda`, the `"nvme%dn%d"` sscanf never matches, so this path returns failure immediately without ever calling the ioctl.

**When both fail**, the code path we traced (`main.rs`-equivalent function around `0x18c00-0x18cc0`) prints `"getHardwareID: could not get disk %s info\n"` and **returns early with an error code** -- it does not fall through to compute a hash over zeroed/default buffers.

`0x5386` was decoded precisely: it is the legacy `SCSI_IOCTL_GET_BUS_NUMBER` request code (from `<scsi/scsi_ioctl.h>`; not a modern `_IOC`-encoded number, just a plain legacy constant). It returns an integer bus number into the buffer at `sp+20`, which is then substituted into Path A's `%u` in `/proc/scsi/usb-storage/%u` -- i.e. Path A's file path is not hardcoded to bus 0, it uses whatever bus number this ioctl reports for the actual device.

An exhaustive string search of the whole binary for other plausible generic-SCSI identification strings (`/proc/scsi/scsi`, `Vendor:`, `/sys/block`, `/sys/class/scsi`, `scsi_generic`, `/dev/sg`, `INQUIRY`) found **zero matches** -- `/proc/scsi/usb-storage` (Path A, section 8.6) is the *only* `/proc`-based identification string anywhere in the binary. Combined with `SCSI_IOCTL_GET_BUS_NUMBER` succeeding for essentially any SCSI-registered block device (not just literal USB storage), this makes it likely that Path A is in fact the live path for `virtio-scsi-pci` on this custom embedded kernel -- i.e. RouterOS's virtio-scsi driver registers itself under the legacy `/proc/scsi/usb-storage` procfs tree (an unusual but plausible code-reuse choice in a heavily customized kernel), and the "no other path exists" evidence outweighs the earlier assumption that Path A's naming implies it's USB-only.

If that is correct, the real unknown is no longer *which code path runs*, but **what the kernel driver itself writes into `/proc/scsi/usb-storage/<bus>`'s `Serial Number:` line** -- that string is synthesized by the kernel's block/SCSI driver, not by `keyman`, and may not be a verbatim copy of the QEMU-level `serial=` SCSI INQUIRY property (VPD page 0x80). This would fully explain the observed behavior: `product` (read via a different, still-unlocated field/mechanism) visibly affects the result, while the expected `serial` does not, because the actual hashed "serial" bytes come from whatever the kernel driver formats into that procfs line, not from what QEMU was told to report.

### 8.7 Verification blocked without dynamic tracing

RouterOS ships no interactive Linux shell in the guest (confirmed earlier in this investigation), so `cat /proc/scsi/usb-storage/<bus>` cannot simply be run from inside the VM to check what the kernel driver actually wrote there. Confirming the working hypothesis in 8.6 would require either running `keyman`/`nova` under user-mode ARM emulation (`qemu-arm-static` + `strace -f`) against a representative block device outside the guest, or finding another way to read that specific `/proc` file's contents from inside a running RouterOS ARM64 VM (e.g. a custom RouterOS package with shell access, if one exists) -- neither has been attempted yet.

### 8.8 Dynamic-tracing attempt (qemu-arm + strace)

An attempt was made to resolve 8.6/8.7 dynamically rather than statically, using `qemu-arm` (user-mode ARM emulation, available on the ARM64 PVE host) plus `strace -f` against the real `keyman` binary and its runtime dependencies extracted from the guest image.

**What worked:**

- `qemu-arm -L <sysroot>` successfully loads and runs the ARM32 `keyman`/`loader` binaries against the extracted `/lib/*.so` set, with `strace -f` transparently observing every real syscall the guest program issues (since user-mode QEMU translates each guest syscall into a real host syscall).
- `keyman` first tries to connect to a Unix-domain control socket (`/ram/novasock`) belonging to "the loader" -- RouterOS's `nova`-framework process supervisor (binary at `nova/bin/loader`, also present in the extracted image). Running the *real* `loader` binary (also under `qemu-arm`) makes this socket genuinely live, rather than needing to fake it.
- `loader` itself required a working `/dev/mtdblock0` (physical RouterBOARD flash) satisfying an ATA `HDIO_DRIVE_CMD`/`HDIO_GET_IDENTITY` probe before it would proceed past its own board-identity check -- confirmed via `strace -e inject=ioctl:retval=0` (forcing these ioctls to report success) that this check is **non-fatal** when faked: `loader` printed `STRONG FAIL: this is not equal to that` (a checksum mismatch, expected since the injected data is garbage) but continued to `scheduling service startup...` and successfully bound `/ram/novasock`.
- With the real `loader` alive and `/ram/run` created, `keyman` connects, exchanges an initial handshake (`sendmsg` of a 27-byte `nv::message`-framework packet), and blocks in `ppoll()` waiting for `loader`'s reply.

**Where it stopped:** `loader` never sends a reply to this specific request (still waiting for it to reach full readiness, or the request type isn't handled given the earlier faked/failed board-identity check). Attempting to force progress by fault-injecting `ppoll`/`recvmsg` to report a fabricated "response ready" (via `strace -e inject`) crashes `qemu-arm` itself with an internal `SIGSEGV` -- the guest code computes jump targets/offsets from the (fabricated, garbage) message content, and without knowing the real `nv::message` wire format (a typed binary RPC protocol built on C++ template methods `message::insert<u32_array_id>`/`append<>`/`extract<>`, disassembled far enough in `libumsg.so`'s `nv::Looper::connectLoader` to confirm its shape but not its exact byte layout), no fabricated response is safe to inject.

Fully resolving 8.6/8.7 dynamically would require decoding this RPC protocol precisely enough to author a real (not fabricated) `loader`-side reply -- a substantially larger reverse-engineering effort than the static disassembly in 8.1-8.7, and was not completed.

### 8.9 Not ARM-specific: root cause confirmed via the x86 binary too

Testing on x86_64 PVE hosts confirms the same failure with `scsi0` (virtio-scsi/LSI) and `sata0` bus types -- collision-search combos verified on `ide0` do **not** reproduce there either. Disassembling `tools/bin/keyman_7.23.2` (the x86 binary already used for sections 1-7) around its own `getHardwareID`-equivalent function resolves this completely, and turns 8.1-8.8's ARM findings from "likely" into confirmed:

```asm
; try the ATA-specific path first
pushl  $0x31f              ; HDIO_DRIVE_CMD -- standard Linux ATA passthrough ioctl
pushl  <fd>
calll  ioctl@plt
testl  %eax, %eax
je     <skip SCSI, use ATA IDENTIFY data>   ; ioctl succeeded -> real ATA/IDE device

; ATA ioctl FAILED (not a real ATA device) -- fall back to the SCSI-generic path:
pushl  $0x5386              ; SCSI_IOCTL_GET_BUS_NUMBER -- identical to the ARM32 code in 8.6
pushl  <fd>
calll  ioctl@plt
...
pushl  $"/proc/scsi/usb-storage/%u"   ; identical string, identical snprintf/fopen/fgets/sscanf loop
...
pushl  $"Serial Number: %19s"         ; identical format string
```

This is **byte-for-byte the same logic** as the ARM32 disassembly in 8.5-8.6 -- same `0x5386` ioctl, same `/proc/scsi/usb-storage/%u` path, same `Serial Number: %19s` format, and the same `0x80041272` (`BLKGETSIZE64`) constant elsewhere in the function. It is shared source code compiled for both architectures, not an ARM-specific quirk.

The dispatch logic is now fully explained: `HDIO_DRIVE_CMD` (`0x31f`) only succeeds against a real ATA/IDE device (`/dev/hd*`, or a QEMU `ide0`-attached disk). Any disk exposed through the Linux SCSI subsystem instead (`/dev/sd*` -- `scsi0`, `sata0`/AHCI, and `virtio-scsi-pci` all present this way to the guest kernel) fails the ATA ioctl and falls through to the narrow `GET_BUS_NUMBER` + `/proc/scsi/usb-storage` text-parsing path from 8.6 -- which, as established there, is only ever populated for literal USB-attached storage and is not guaranteed to reflect QEMU's `serial=` property verbatim for other SCSI transports. This is a **disk-bus-type** issue, not a CPU-architecture one: it reproduces identically on x86_64/`scsi0`+`sata0` and ARM64/`virtio-scsi-pci`, and does not reproduce on `ide0` on either architecture.

### 8.10 Practical implication

**Collision-search results in this project (`docs/collision-database.md`) are only verified for `ide0`-attached disks**, and that is now known to be a hard requirement rather than an incidental detail of how the reference hardware happened to be captured: `HDIO_DRIVE_CMD` must succeed, which requires a real ATA/IDE-presented disk. `scsi0`, `sata0`, and `virtio-scsi-pci` are all confirmed **not** interchangeable with `ide0` for this purpose, on x86_64 or ARM64. **Always attach the target disk as `ide0` when applying a collision-search result.**

### 8.11 `serial=` is controllable on `scsi0` too -- confirmed empirically

The one open question from 8.6-8.9 -- whether the SCSI fallback path's `Serial Number:` value reflects QEMU's `serial=` property at all, or is synthesized by the kernel independent of it -- is resolved. Tested on the ARM64 host (`192.168.2.1`, VM 100, `scsi0`, `product=ZZZZZZZZZZZZZZZ` fixed):

| `serial=` | Resulting SOFTWARE ID |
|---|---|
| `00000000717959548436` | `ABAH-C0JJ` (reproduced across two separate boots) |
| `AAAAAAAAAAAAAAAAAAAA` | `BSW5-9EGM` |

Changing only `serial=` deterministically changes the SOFTWARE ID, and reverting it reproduces the original result exactly. **`serial=` is read and does participate in the hash on `scsi0`** -- it is not ignored or kernel-synthesized. The earlier mismatch (8.1: `product=SSD1G` expected `4MZF-SFTR`, got `3X8K-8K32`) is therefore not "serial is uncontrollable on SCSI" -- it is that the SCSI path's byte-level encoding of `serial=`/`product=` into the 40-byte hash input differs from `ide0`'s, in a way not yet reverse-engineered (padding, truncation via `%19s`, or a different field order than `serial[20] || model[16]`).

This means a **dedicated `scsi0`-targeted collision search is plausible in principle** -- unlike a scenario where the kernel discards/regenerates the identity, here the mapping is deterministic and (based on this one data point) appears to still depend on both `serial=` and `product=`. What's missing is the exact encoding rule for the SCSI path, which would need to be derived either by further disassembly of the `/proc/scsi/usb-storage` parsing/hash-input-assembly code (8.5-8.6 traced the sanitization loops but not the final byte layout used for this specific path) or by black-box probing (vary `serial=` systematically, observe the resulting SOFTWARE IDs, and infer the transform) -- neither has been done yet.

### 8.12 Found the real mechanism: `SG_IO` + standard INQUIRY + VPD page 0x80

Continuing the disassembly of `tools/bin/keyman_7.23.2` around the same function (just before the `GET_BUS_NUMBER`/`/proc/scsi/usb-storage` code from 8.9) turned up a second, more legitimate SCSI-identification path that had not been located before -- and it is almost certainly the one actually responsible for the empirical result in 8.11, not the `/proc/scsi/usb-storage` text parse.

A helper function at `0x804fa51` builds a standard Linux `sg_io_hdr_t` on the stack and calls `ioctl(fd, 0x2285, &sg_io_hdr)`:

```asm
movl   $0x53, -0x58(%ebp)        ; interface_id = 'S'  (sg_io_hdr_t.interface_id)
movl   $0xfffffffd, -0x54(%ebp)  ; dxfer_direction = SG_DXFER_FROM_DEV (-3)
movb   %al, -0x50(%ebp)          ; cmd[0] = CDB opcode byte, taken from the caller's request
...
movl   $0x3e8, -0x3c(%ebp)       ; timeout = 1000 ms
pushl  $0x2285                   ; SG_IO
calll  ioctl@plt
```

`0x2285` is the standard Linux `SG_IO` ioctl (`_IOWR('S', 0x85, sg_io_hdr_t)`) -- this is genuine SCSI-generic passthrough, not the ARM32 `NVME_IOCTL_ADMIN_CMD` (`0xc0484e41`) that was initially (and incorrectly) suspected to be the same thing in early ARM32 analysis.

The caller invokes this wrapper twice with different CDB values:

```asm
movl   $0x12, -0x2a0(%ebp)       ; CDB = 0x00000012 -> opcode 0x12 (INQUIRY), EVPD=0  (standard inquiry)
...
calll  0x804fa51                 ; -> vendor/product/revision (this is where "product" comes from)
...
movl   $0x800112, -0x2a0(%ebp)   ; CDB bytes (LE) = 12 01 80 00 -> opcode 0x12, EVPD=1, page=0x80
...
calll  0x804fa51                 ; -> Unit Serial Number VPD page (this is where "serial" comes from)
```

`INQUIRY` with `EVPD=1, page=0x80` is the standard SCSI "Unit Serial Number" VPD page -- exactly the mechanism QEMU's `scsi-hd`/`virtio-scsi-pci` backend uses to expose the `serial=` device property to the guest. This is consistent, deterministic, and guest-kernel-independent (unlike the `/proc/scsi/usb-storage` text file, which depends on which kernel subsystem happens to register the device there) -- it directly explains why 8.11's black-box test found `serial=` to be reliably controllable.

**The VPD-80 extraction logic was fully traced and matches the standard SCSI VPD-80 wire format exactly:**

```asm
movb   -0x21d(%ebp), %al   ; al = response[3]  -- VPD page's "page length" byte (offset 3)
cmpb   $-6, %al             ; clamp to 0xFA (250) as a safety cap
jbe    ...
movzbl %al, %esi            ; esi = clamped length
xorl   %ebx, %ebx           ; ebx = index, starts at 0
; loop:
movzbl -0x21c(%ebp,%ebx), %eax   ; response[4+i]  -- first byte of VPD-80's ASCII serial payload
pushl  %eax
calll  isprint@plt
testl  %eax, %eax
je     <break>               ; stop at the first non-printable byte
incl   %ebx
jmp    <loop>
; after loop: ebx = count of leading printable bytes (<= page-length byte, <= 250)
```

This is byte-for-byte the standard T10 VPD page 0x80 layout (`peripheral qualifier/type` (1) + `page code=0x80` (1) + reserved (1) + `page length N` (1) + N bytes of ASCII serial number starting at offset 4) -- the code takes the ASCII payload starting right after the 4-byte VPD header, and copies out the printable-prefix run (bounded by both the page-length byte and a 250-byte safety cap). For QEMU's `serial=` property (which populates this exact VPD-80 payload for `scsi-hd`/`virtio-scsi-pci`), an all-printable-ASCII value like a 20-character serial should therefore be captured in full and unmodified -- consistent with 8.11's clean, deterministic `serial=` -> SOFTWARE-ID mapping.

**The standard-INQUIRY (non-EVPD) result feeds "model"** via a *fixed*-length copy (`0x10` = 16 bytes, no `isprint()` trimming) from response offset 16 -- exactly the standard SCSI INQUIRY "Product Identification" field (bytes 16-31 of the standard 36-byte INQUIRY response). QEMU's `product=` property populates this field, space-padded per the T10 spec, so this is also expected to be a clean, direct copy.

**Precedence resolved:** `GET_BUS_NUMBER` + `/proc/scsi/usb-storage` (8.9) *is* attempted unconditionally, immediately after the two `SG_IO` calls, regardless of whether they succeeded -- but its result is only *used* as a last resort:

```asm
movl   %esi, %eax     ; esi was set by the SG_IO VPD-80 call: 1 = ioctl succeeded, 0 = failed
testb  %al, %al
jne    0x80509fd       ; VPD-80 succeeded -> jump straight to finalization with the SG_IO-derived
                        ; serial/product, discarding whatever /proc/scsi/usb-storage parsed
; only reached if VPD-80 FAILED:
calll  0x804fac1        ; a third, distinct identification routine (not yet traced) -- its result
                         ; is what actually gets used when SG_IO is unavailable
```

So the real priority order is: **`SG_IO` (standard INQUIRY + VPD-80 Unit Serial Number) wins whenever it succeeds; `/proc/scsi/usb-storage` text parsing and a third fallback routine at `0x804fac1` are only consulted if `SG_IO` fails outright** (e.g. permission denied on the device node, or the backend doesn't implement SCSI-generic passthrough). For `virtio-scsi-pci`/`scsi-hd` under QEMU -- both of which do support `SG_IO` -- this means 8.12's clean INQUIRY+VPD-80 mechanism is expected to be the one actually in effect, matching 8.11's clean empirical result. This resolves the open precedence question and gives high confidence that the "20-byte printable-ASCII prefix of the VPD-80 payload" and "16-byte raw copy of the standard INQUIRY Product ID field" rules in this section are the real `scsi0` encoding -- a dedicated `scsi0` collision-search implementation is the natural next step, not further disassembly.

### 8.13 `0x804fac1` traced: it's the same NVMe fallback as ARM32, confirmed identical

`0x804fac1` -- the "third fallback", only reached when the `SG_IO` VPD-80 call fails outright -- turns out to be **the exact same NVMe-specific mechanism already documented for ARM32** (§8.1-8.4's `keyman_arm32` analysis), not a new, distinct code path. Confirmed byte-for-byte:

```asm
movb   $0x6, -0x1060(%ebp)       ; cmd_len = 6 (NVMe admin-command CDB-style length)
pushl  $0xc0484e41                ; NVME_IOCTL_ADMIN_CMD -- identical constant to the ARM32 binary
movl   $0x1000, -0x103c(%ebp)     ; 4096-byte response buffer (NVMe Identify Controller response size)
calll  ioctl@plt
je     <process NVMe Identify response>   ; ioctl succeeded -> use it directly

; only reached if the ioctl on the ORIGINAL fd fails:
calll  basename@plt
pushl  $"nvme%dn%d"                ; sscanf the device basename against this pattern
calll  sscanf@plt
...                                 ; if it matches, open "/dev/nvme<N>" and retry the same
                                     ; NVME_IOCTL_ADMIN_CMD ioctl against that controller node
```

And the response parsing confirms the same field widths as ARM32's NVMe path:

```asm
strnlen(buf_at_offset_20, 0x28)   ; 0x28 = 40 -- NVMe "Model Number" field width
strnlen(buf_at_offset_0,  0x14)   ; 0x14 = 20 -- NVMe "Serial Number" field width
```

This is genuinely shared, cross-architecture source code (as expected, given 8.9's confirmation for the `SG_IO`/`GET_BUS_NUMBER` paths) -- there is no separate, not-yet-found x86-specific fallback. **The complete, now fully-traced hardware-identification priority chain for `keyman`/`nova`, across both architectures:**

| Priority | Mechanism | Bus types it actually works for | Source of `serial`/`model` |
|---|---|---|---|
| 1 | `ioctl(HDIO_DRIVE_CMD)` (`0x31f`) | `ide0` (real ATA/IDE) | Real ATA IDENTIFY data -- ground truth for this project's collision database (§1-7) |
| 2 | `ioctl(SG_IO)` (`0x2285`): standard INQUIRY + EVPD page 0x80 | `scsi0`, `sata0`/AHCI, `virtio-scsi-pci` -- anything with working SCSI-generic passthrough | Product ID field (16B, raw) + VPD-80 printable-ASCII prefix (up to 20B) -- §8.12, confirmed to track QEMU's `serial=`/`product=` |
| 3a | `SCSI_IOCTL_GET_BUS_NUMBER` (`0x5386`) + `/proc/scsi/usb-storage/<bus>` text parse | Literal USB-attached storage only | `Serial Number: %19s` line -- §8.6, only used if priority 2 fails |
| 3b | `NVME_IOCTL_ADMIN_CMD` (`0xc0484e41`) via basename match `nvme%dn%d` | NVMe devices (`/dev/nvme0n1` etc.) | NVMe Identify Controller SN (20B)/MN (40B) fields -- this section, only used if priority 2 fails |

Priorities 3a and 3b are tried in an unspecified order relative to each other when priority 2 fails (not yet determined which is attempted first), but neither applies to `scsi0`/`sata0`/`virtio-scsi-pci` disks in practice, since those support `SG_IO` and priority 2 wins before either is reached.

### 8.14 SCSI collision search works for SOFTWARE ID -- but the license still doesn't validate

The `--bus scsi` encoding from 8.11-8.13 (`sector_val=0`, standard `serial[20]+model[16]` layout) was implemented in `ros-serialgen` and run as a real search (see `docs/command-reference.md` for the `-b`/`--bus` flag). It found genuine hits -- e.g. `serial=00000000430480281048`, `model=SSD1G`, `size=1G`, `--bus scsi` computes `C7CU-PGT9`, matching this project's known signature exactly. Booting this combo on a real `scsi0` VM confirmed `software-id: C7CU-PGT9` on `/system license print`, proving the SOFTWARE ID side of 8.11-8.13 is correct and the SCSI-specific search is genuinely usable for finding *matching SOFTWARE IDs*.

**However, writing `C7CU-PGT9`'s known-good MBR (`00...BDE800000000` + its signature) to the standard file offset `0x100` and rebooting did not activate the license** -- `/system license print` kept showing the SOFTWARE ID correctly but stayed in 24-hour trial mode (`expires-in`) instead of `nlevel: 6`. Ruled out:

- **Not the boot-counter (`reserved`, `0x10C-0x10F`) incrementing.** RouterOS bumped it from `00000000` to `01000000` after the first boot, matching documented behavior (§5's installer-intervention table) and consistent with §3.6's finding that `reserved`/`marker` don't affect the SOFTWARE ID computation. Explicitly resetting `reserved` back to `00000000` and rebooting again made no difference -- still trial mode.
- **Not ARM64-specific.** The same failure to activate (correct SOFTWARE ID, signature doesn't validate) was independently observed on an x86_64 host with a `scsi0`-attached disk as well.

Since this reproduces identically across architectures and is unaffected by `reserved`, the most likely explanation -- not yet confirmed by disassembly -- is that **license *signature validation* (reading the MBR license region at boot, independent of the `serial`/`model` identification code traced in 8.1-8.13) may also be bus-type-dependent**, e.g. not reading from the standard file offset `0x100` at all for SCSI-attached disks, mirroring the same pattern already found for `serial`/`model` reads. This has not been investigated -- 8.1-8.13 only trace how `serial`/`model`/`sector_val` become the SOFTWARE ID input; the boot-time MBR-read/signature-verify code path (§5's "License Verification Flow") is a distinct, not-yet-disassembled part of `keyman`/`nova`.

**Practical implication:** a `--bus scsi` search can currently find a serial whose *computed SOFTWARE ID* matches a known signature (useful for research, and confirmed accurate), but **does not yet result in an activatable license** on `scsi0`/`sata0`/`virtio-scsi-pci` -- full activation on those bus types remains unsolved pending disassembly of the MBR-read path used during boot-time signature verification.

### 8.15 Found it: on QEMU/KVM, `readMBR` doesn't touch the disk at all -- it uses `/dev/hvckvm0`

Disassembling `readMBR`'s internals resolves 8.14. `readMBR` first calls a cached predicate (`0x804f902`) that is **byte-for-byte the same `getenv("board")` + `strstr(..., "qemu")` check already found on ARM32** (§8.9's virtualization-detection function) -- confirming this is shared, cross-architecture logic, not something new.

When that predicate is true (i.e. running under QEMU/KVM, which covers essentially every PVE VM), `readMBR` skips the physical-flash path (`/dev/flash` + custom `0x4601`/`0x90004602` ioctls, for real RouterBOARD hardware) entirely and instead:

```asm
calll  0x804f76f            ; a second, more specific gate check
testb  %al, %al
je     <fail>                ; bail out if false

pushl  $2                     ; O_RDWR
pushl  $"/dev/hvckvm0"        ; MikroTik-private hypervisor virtual console device
calll  open@plt
...
calll  tcgetattr@plt          ; configure it as a raw terminal (no echo, no canonical mode, etc.)
calll  tcsetattr@plt
...
; later, via 0x804f9a0(fd, cmd=6, len=0x208):
write(fd, {len=8, cmd=6}, 8)  ; send an 8-byte request: "read command 6"
read(fd, buf, 0x208)           ; read back a 520-byte response (512-byte sector + 8-byte header?)
```

`/dev/hvckvm0` ("hvc" = hypervisor virtual console, the standard Linux paravirtualized-console naming convention used by Xen/KVM `virtio-console`) is a **MikroTik-private device node, not a standard block device**. This means: **on any QEMU/KVM-detected VM, `readMBR` never reads the guest disk file at all** -- it sends a small request over this console channel and expects a companion process (presumably MikroTik's own CHR-specific QEMU integration, or a host-side helper backing this virtio-console) to respond with sector data. Writing directly to the disk image via `qemu-nbd` (as done throughout this project, including 8.14's failed activation attempt) **never touches whatever `/dev/hvckvm0` actually returns** -- these are two completely independent data paths.

This fully explains 8.14's mystery without needing any further bus-type-dependent hypothesis: the MBR write itself was never going to be read back by a standard PVE VM, because standard PVE VMs almost certainly don't provide a working `/dev/hvckvm0` backend (this is CHR/MikroTik-cloud-image-specific QEMU integration that a plain `qm create`-built VM has no reason to implement). Whether `open("/dev/hvckvm0")` succeeds or fails inside our test VMs, and if it fails, what `readMBR` falls back to (if anything), has not yet been checked -- this is the next concrete step, and does not require any more static disassembly to investigate (it's a runtime/environment question: does `/dev/hvckvm0` exist in the guest, and if not, does `readMBR` have any further fallback beyond this one already-traced branch).

### 8.16 `/dev/hvckvm0` traced further: it's the standard Linux `hvc0` virtio-console driver, self-provisioned, with no fallback on failure

The gate function `0x804f76f` was disassembled and resolves the remaining questions in 8.15:

```asm
cmpb   $0x0, <cached_flag>
je     <compute>
retl                               ; return cached result on repeat calls

; first call:
pushl  <stat_buf>
pushl  $"/dev/hvckvm0"
calll  stat@plt
testl  %eax, %eax
jne    <not_found>                 ; stat failed -> device node doesn't exist yet
movb   $1, <cached_ok_flag>        ; already exists -> success, done
...
<not_found>:
pushl  $"r"
pushl  $"/sys/class/tty/hvc0/dev"
calll  fopen@plt
testl  %eax, %eax
je     <fail>                       ; sysfs entry doesn't exist either -> give up (cached_ok_flag stays 0)
...
fscanf(fp, "%u:%u", &major, &minor)
fclose(fp); unlink("/dev/hvckvm0")
mknod("/dev/hvckvm0", S_IFCHR|0777, makedev(major, minor))
movb   $1, <cached_ok_flag>
```

`hvc0` ("Hypervisor Virtual Console") is a **standard upstream Linux kernel driver** (`CONFIG_HVC_DRIVER`), used for Xen PV consoles and `virtio-console` devices -- it is not a MikroTik invention. `/sys/class/tty/hvc0/dev` is the normal sysfs path any Linux kernel exposes for a registered `tty` device's `major:minor`, used here purely to self-provision the `/dev/hvckvm0` node (since a CHR image's minimal `/dev` may not have it pre-populated by udev). The logic reduces to: **"does this VM have a `virtio-console` (or Xen console) device attached to the guest?"** -- if yes, use it as the MBR data channel; if no, `readMBR` returns failure with **no further fallback** on this code path.

Standard PVE VMs created via `qm create`/`qm set` (including every VM used in this session, and almost certainly the historical `ide0` VMs behind this project's verified collision-database entries) **do not attach a `virtio-console` device by default** -- PVE's `serial0: socket` option (used throughout this project for console access) provisions an **isolated PC-style UART** (`ttyS0`/`COM1`), which is a completely different QEMU device (`isa-serial`/`pci-serial`) from `virtio-console` (`virtconsole`/`virtio-serial-bus`). Having `serial0: socket` configured does **not** make `/sys/class/tty/hvc0` appear.

This raises a real, testable question this section does not yet answer: **the project's own collision-database entries were verified as fully activated (`nlevel: 6`, no `expires-in`) on plain PVE `ide0` VMs** -- if `board` containing `"qemu"` unconditionally forces this `hvc0`-only path with no fallback, how did those succeed without a `virtio-console` device? Two explanations are consistent with the evidence so far, neither yet confirmed:

1. **The `board` environment variable's actual value differs by VM configuration** (machine type, `smbios1` overrides, BIOS vs. OVMF, etc.), and the historically-successful `ide0` VMs' `board` value happened not to contain the substring `"qemu"` -- in which case `0x804f902` returns false immediately and `readMBR` takes the `/dev/flash`-then-generic-`fopen()` path from 8.15 instead (which, being a plain file read, would work identically on `ide0` regardless of bus type).
2. **`ide0` disks are read through an entirely different, not-yet-traced license-verification code path** that doesn't share this `readMBR` function's `board=qemu` branch at all.

**Next step (empirical, no further disassembly needed):** compare `getenv("board")`'s actual value between a VM known to activate successfully on `ide0` and the `scsi0` VMs used in 8.11-8.15 -- if the `ide0` VM's value doesn't contain `"qemu"`, explanation 1 is confirmed, and the practical fix for `scsi0`/`sata0` activation becomes straightforward: override the VM's reported `board`/SMBIOS values so `"qemu"` isn't a substring, steering `readMBR` back onto the `/dev/flash`-or-generic-file-read path instead of the `hvc0`-only one.

### 8.17 Tried overriding SMBIOS to dodge `board=qemu` -- landed in a third, different detection branch (`MetaROUTER`)

To test 8.16's hypothesis directly, PVE's `smbios1` `product`/`manufacturer` fields were overridden away from the QEMU defaults (`qm set <vmid> --smbios1 uuid=<existing>,product=<base64>,manufacturer=<base64>,base64=1`, non-default values chosen to avoid the substring `"qemu"`), keeping everything else (disk, `serial=`/`product=` SCSI properties, `-b scsi` search target) identical to the already-confirmed `C7CU-PGT9` combo from 8.14.

**Result: the boot behavior changed completely, but not to the expected `/dev/flash`-fallback path.** Before the override, boot showed the normal 24-hour CHR trial banner (`software-id: ..., expires-in: 23hXXm`). After the override:

- The CLI prompt changed from `[admin@arm64]` to **`[admin@MetaROUTER]`**.
- The boot banner changed to `ROUTER HAS NO SOFTWARE KEY` with an unusual **~136-year** countdown (`1193046h27m`) instead of the normal 24-hour trial.
- `/system license print` now shows **only** `software-id: C7CU-PGT9` -- no `expires-in` line at all (neither the trial state nor a fully-activated `nlevel: 6` state).

This is RouterOS's **`MetaROUTER`** mode -- MikroTik's nested-virtualization feature where a guest router is normally expected to receive its license from a *parent* RouterOS instance rather than validating its own disk MBR, which plausibly explains why the normal trial/license flow is bypassed entirely once this mode is detected.

The trigger was located via a debug string in `lib/libumsg.so` (shared across the whole `nova` framework, not `keyman`-specific):

```
"open /dev/rb failed, probably metarouter"
```

i.e. **a third hardware-presence check**, independent of both `readMBR`'s `board`-string check (8.15-8.16) and the `/dev/hvckvm0` virtio-console path (8.15-8.16): the framework tries to `open("/dev/rb")` (a "RouterBOARD" device -- distinct from `/dev/flash` and `/dev/hvckvm0`), and if that fails, concludes "probably MetaROUTER" and apparently short-circuits the normal boot-time license flow well before `readMBR`'s own `board=qemu` branching is reached.

**Net result: changing `smbios1` did successfully steer the platform-detection logic away from `board=qemu`, but toward a different special-cased mode rather than the plain-hardware `/dev/flash`/generic-file-read path 8.16 hypothesized.** This is not yet a working activation path, and not yet a dead end either -- the open questions are:

- What exact condition triggers the `/dev/rb`-absence -> "MetaROUTER" conclusion, and is it purely `open()` failing, or does it also depend on the same `board` string (e.g. some other substring match, not just absence of `"qemu"`)?
- Does the *original* `board=qemu` value (unmodified SMBIOS) also fail `open("/dev/rb")` -- i.e. is `/dev/rb` open failing on *every* QEMU VM regardless of `board`, with `board=qemu` normally taking priority and being checked *first* (explaining why the un-modified VMs never showed `MetaROUTER` -- the `board=qemu` branch intercepted the check before `/dev/rb` was ever tried)? If so, the real fix may require a `board`/SMBIOS value that is simultaneously **not** `"qemu"`-like *and* somehow satisfies (or avoids) the `/dev/rb`-presence check -- which likely means creating an actual `/dev/rb` device node (analogous to 8.15's `/dev/flash`) rather than relying on `smbios1` alone.
- Whether `flash.ko` (a real, present kernel module in this image that also references `MetaROUTER` per the string search) is involved in provisioning `/dev/rb`, the way `hvc0`/sysfs was used to self-provision `/dev/hvckvm0` in 8.16 -- not yet disassembled.

This has not been resolved -- continuing requires disassembling `libumsg.so`'s `MetaROUTER`-detection function (the `open("/dev/rb")` caller) and `flash.ko` to determine what, if anything, would make `/dev/rb` present and change this outcome.

### 8.18 Resolved on x86_64: `scsi0` activates cleanly out of the box, no SMBIOS tricks needed -- this was an ARM64/`virt`-machine-type quirk, not a SCSI limitation

A fresh, minimal-config test settles 8.9-8.17's remaining open question. A brand-new x86_64 PVE VM was built from scratch (cloned from a working RouterOS template, default `smbios1` -- **no product/manufacturer override at all**):

- `scsihw: virtio-scsi-pci`, `scsi0` disk, 1G, `serial=00000000430480281048`, `-set device.scsi0.product=SSD1G` (the exact `--bus scsi` search hit from 8.14/`C7CU-PGT9`)
- Fresh RouterOS 7.24 install directly onto the `scsi0` disk (installer run via `qm sendkey`, `a`/`i`/`y`)
- Shut down (not rebooted) per the standard MBR-write procedure, then the standard `00...BDE800000000` + `C7CU-PGT9` signature MBR written via `qemu-nbd`
- Booted once

Result: `/system license print` shows

```
software-id: C7CU-PGT9
nlevel: 6
features:
```

**Fully activated -- no `expires-in`, `nlevel: 6`, on the first boot, with zero SMBIOS manipulation.** This is the cleanest possible confirmation that the `--bus scsi` SOFTWARE-ID encoding (8.11-8.13) and the standard MBR-write activation procedure both work correctly and completely on a real `scsi0`/`virtio-scsi-pci` disk -- there is no `scsi0`-specific activation problem on x86_64.

**This reframes 8.14-8.17 entirely.** The activation failures documented there were specific to the **ARM64 test VM's `virt` QEMU machine type**, not to `scsi0`/SCSI as a bus type in general.

One assumption from 8.16-8.17 needs correcting, though: it is **not** simply "x86's DMI doesn't say QEMU." Checked directly via `/system resource print` on the working x86 VM: `board-name: x86 QEMU Standard PC (Q35 + ICH9, 2009)` -- this **does** contain `"QEMU"`, just like the ARM64 VM's `board-name: arm64 QEMU KVM Virtual Machine` did. Both platforms' *displayed* `board-name` contain the substring, yet only ARM64 hit the problem branch. This means **`keyman`'s internal `getenv("board")` value is not simply identical to the `board-name` string shown by `/system resource print`** -- they likely come from related but distinct sources (or different case-normalization), and `strstr(getenv("board"), "qemu")` is a case-sensitive C string search, so the exact casing of whatever `board` actually contains matters and has not been directly captured (no way to read `keyman`'s process environment from the RouterOS CLI). The `board=qemu` / `MetaROUTER` / `/dev/hvckvm0` detection maze in 8.15-8.17 is still confirmed real via disassembly, but *why* it triggers on this specific ARM64/`virt` setup and not on x86_64/`q35` remains an open, unexplained platform difference -- not the simple "x86 board name lacks qemu" explanation originally assumed here.

**Practical implication -- superseding 8.10's blanket warning:** `--bus scsi` search results **are** activatable, at least on x86_64 with a standard PVE-default `smbios1` (no override required). The remaining open question is narrower than previously stated: does `scsi0` activation also work on **ARM64** with a `board`/SMBIOS value that avoids `"qemu"` *and* avoids triggering the `MetaROUTER` fallback from 8.17 (e.g. a value resembling real RouterBOARD ARM64 hardware) -- this was not retested after 8.18's x86 result and remains open specifically for ARM64/`virt`, not for `scsi0` in general.

### 8.19 `sector_val=0` confirmed size-independent -- tested at 2GiB, not just 1GiB

8.11's `sector_val=0` finding was originally validated against 7 real boot tests, all on a single 1GiB disk -- leaving open whether `sector_val=0` was a genuine, size-independent property of the `scsi0` path, or coincidentally zero only for that one disk size.

Retested with a **fresh install** on a **2GiB** `scsi0` disk (same host as 8.18, same `serial=00000000430480281048`/`product=SSD1G` -- only the disk size changed): full activation succeeded identically -- `software-id: C7CU-PGT9`, `nlevel: 6`, no `expires-in`, same as the 1GiB case. Since `--bus scsi` computes the same SOFTWARE ID at 1GiB and 2GiB for the same `serial=`/`product=` (both force `sector_val=0` regardless of the actual disk size passed to `ros-serialgen search -s`), and both independently activate against the same signature, this confirms `sector_val=0` is **not** a 1GiB-specific coincidence -- it holds across at least two different disk sizes on `scsi0`. The size caveat in 8.11-8.13's wording can be considered resolved for x86_64/`virtio-scsi-pci`.

**Practical implication: on `scsi0`, the actual disk size is irrelevant to which `serial=`/`product=` combo you need.** This is a real, useful difference from `ide0`: `ide0`'s SOFTWARE ID depends on `sector_val`, which is derived from the disk's exact byte count, so an `ide0` collision result is only valid for a disk of that *exact* size (§6, §3.4). On `scsi0`, since `sector_val` is always `0` regardless of the disk's real size, **a single `serial=`/`product=` combo found via `ros-serialgen search -b scsi -s <any size>` will activate on a `scsi0` disk of *any* size** -- there is no need to match the search size to the deployed disk size, and no need to maintain size-specific tables the way `docs/collision-database.md` §2 does for `ide0`. The `-s`/`-u` flags still need *some* value when running `search -b scsi` (they're required CLI arguments), but the resulting `serial=`/`product=` pair is size-agnostic in practice for `scsi0` deployments.

### 8.20 `sata0` is NOT like `scsi0` -- it uses the exact same encoding as `ide0`

Every prior section in §8 treats `scsi0` and `sata0` as a pair (both non-`ide0`, both assumed to share the SCSI-generic code path from §8.9/8.12). This assumption was never actually tested for `sata0` specifically -- it turns out to be wrong.

QEMU's `sata0` (AHCI) disks are backed by the **same `ide-hd` qdev device model as `ide0`**, just attached to an AHCI controller instead of a legacy PIIX/ISA IDE controller -- confirmed directly: attempting `-set device.sata0.product=<x>` (the SCSI-specific property used throughout §8.11-8.19) fails at QEMU startup with `Property 'ide-hd.product' not found`. `ide-hd` only exposes a `model=` property (the same one `ide0` uses), not `vendor=`/`product=` (which are `scsi-hd`-only). This alone strongly suggests `sata0` disks respond to ATA IDENTIFY like real IDE drives, taking `readMBR`'s `HDIO_DRIVE_CMD` success path (§8.9) rather than the `SG_IO` path.

Confirmed both algorithmically and empirically:

- **Algorithmic**: booting a `sata0` disk with `serial=00000000430480281048`/`model=SSD1G` (the SCSI-verified `C7CU-PGT9` combo from §8.14) at 2GiB showed `Current installation "software ID": EJSX-HUUP` -- a **different** ID than the `scsi0` result for the identical `serial=`/`model=` pair. Running `ros-serialgen check --serial 00000000430480281048 -s 2 -u g -m SSD1G -b ide` (note: `-b ide`, not `scsi`) computes the **exact same** `EJSX-HUUP` -- confirming `sata0` uses `ide0`'s real-sector_val encoding, not `scsi0`'s `sector_val=0` encoding.
- **Empirical, with an existing `ide0` collision-database entry**: a fresh 1GiB `sata0` install using the *unmodified* `ide0` table entry (`serial=00000000251582663387`, `model=SSD1G`, `1,073,741,824` bytes -> `TI09-7WK3`, no new search needed) with the standard MBR write (`00...BDE800000000` + `TI09-7WK3`'s signature) **fully activated** on first boot -- confirming this isn't just a matching-SOFTWARE-ID coincidence, the *existing* `ide0` collision database works directly on `sata0`.

**Practical implication:** `docs/collision-database.md`'s `ide0` table (§1-2) applies directly to `sata0` disks of the same size, with no `-b scsi`/`-b ide` distinction needed and no new search required -- treat `sata0` as an alias for `ide0` for collision-search purposes, not as part of the `scsi0` SCSI-generic family. This also means `--bus ide`'s existing wording ("verified against real hardware") extends to `sata0` without qualification, while `--bus scsi` remains specific to `scsi0`/`virtio-scsi-pci` only. `docs/deployment-guide.md` and `docs/command-reference.md`'s bus-type framing (currently grouping `scsi0`+`sata0` together against `ide0`) should be corrected to reflect this.

### 8.21 Signature metadata decryption (`MT_Transform`)

Separately from the bus-type investigation above, the [MTLic project](https://github.com/Ygnecz/MTLic) (`MTTools.py`, `ParseLic.py` -- already listed in `docs/toolchain.md`) documents the internal structure of the 64-byte signature stored in `.key` files and at MBR `0x110-0x14F`:

```
signature[0:16]  -- MT_Transform-encrypted: SOFTWARE_ID(6B LE) || reserved(1B) || level(1B) || zero-padding(8B)
signature[16:32] -- XORed into a hash of the decrypted [0:16] block, used for EC-KCDSA-style verification
signature[32:64] -- the actual Curve25519 signature integer
```

`MT_Transform` is a 16-round ARX block cipher operating on the 16-byte block as four 32-bit words, using round constants -- **which turn out to be exactly this project's existing `ROUND_CONSTANTS`** (`src/sha256_constants.rs`, the MikroTik custom SHA-256 K-table): confirmed byte-for-byte identical against `MTTools.py`'s `SHA256_K`. The decrypted SOFTWARE ID's Base-35 encoding table (`MT_SWSNToSWID`'s `SWIDTab`) is likewise identical to this project's existing `software_id::encode` alphabet. No new reverse-engineered constants were needed -- both pieces were already in this codebase, just not previously connected to this use.

**What this enables:** given any known-valid signature (from `docs/collision-database.md`'s Signature Table, or extracted from a `.key` file via `key2sig`), decrypting `signature[0:16]` reveals which SOFTWARE ID and license level that signature was actually issued for -- useful for auditing/labeling signatures, independent of booting a VM. `ros-serialgen sig2key`/`key2sig` now print this (`SOFTWARE-ID`/`VERSION`/`LEVEL`) to stderr alongside their normal output (`src/convert.rs`'s `decode_metadata`). Verified against `VI8Q-E90F`'s known signature hex (`docs/collision-database.md`): decrypts to SOFTWARE ID `VI8Q-E90F`, level `1` -- matching the real-hardware-confirmed `nlevel: 1` from §1.

**What this does NOT enable:** signing a *new* license for an arbitrary SOFTWARE ID/level still requires the Curve25519 private key corresponding to the public key used in `ParseLic.py`'s verification (`Y = signature*PubKey + hash*G`, checked via `MT_Hash(Y) == signature[16:32]`) -- an ECDLP problem, same ~252-bit hardness already established in `architecture.md` §5. This finding only explains the *structure* of an existing signature; it does not provide a way to forge one for parameters not already covered by a known-valid signature. The project's approach remains unchanged: reuse existing valid signatures via SOFTWARE ID collision search (§1-7), not signature forgery.

The `version_byte` field (byte 6 of the decrypted block) is printed as-is but its meaning is **unconfirmed** -- `ParseLic.py` doesn't label or use it, so treat it as informational only until independently verified (e.g. against RouterOS version numbers across several known signatures).
