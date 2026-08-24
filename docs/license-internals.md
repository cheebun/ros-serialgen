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
