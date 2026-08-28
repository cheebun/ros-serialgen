# Verification Experiment Log

## Test Environment

- PVE 9.2.4
- RouterOS 7.23.2 (also verified on 7.24)
- BIOS: OVMF (UEFI)
- Date: 2026-07-20

---

## Experiment 1: All Five Schemes Verified

| VM | Capacity | SOFTWARE ID | Result |
|---|---|---|---|
| 931 | 6G | `TI09-7WK3` | ✅ nlevel: 6 |
| 932 | 8G | `4MZF-SFTR` | ✅ nlevel: 6 |
| — | 32G | `HHJH-UFWL` | ✅ nlevel: 6 |
| 933 | 42G | `TI09-7WK3` | ✅ nlevel: 6 |
| 920 | 64G | `C7CU-PGT9` | ✅ nlevel: 6 |

---

## Experiment 2: Installer Overwrites 0x10A-0x10B (VM 920 vs 921)

| | VM 920 | VM 921 |
|---|---|---|
| Operation | Install → shut down → rewrite MBR | Install → leave `FF FF` |
| 0x10A-0x10B | `BD E8` (written back manually) | `FF FF` (set by installer) |
| SOFTWARE ID | `C7CU-PGT9` | `C7CU-PGT9` (identical) |
| Authorization | ✅ nlevel: 6 | ❌ ROUTER HAS NO SOFTWARE KEY |

**Conclusion**: 0x10A-0x10B does not affect the SOFTWARE ID, but it does affect authorization validation. The correct value must be written after installation.

---

## Experiment 3: Effect of Modifying 0x100-0x109 on SOFTWARE ID (VM 932)

32G scheme, changing 0x100-0x109 from its original value to all zeros:

| | Original | Modified |
|---|---|---|
| 0x100-0x109 | `47 11 02 26 68 89 83 99 16 18` | `00 00 00 00 00 00 00 00 00 00` |
| SOFTWARE ID | `HHJH-UFWL` | **changed** (different value) |
| Authorization | ✅ | ❌ |

**Conclusion**: 0x100-0x109 participates in the SOFTWARE ID computation. Modifying it changes the ID and breaks authorization matching.

---

## Experiment 4: Model Names with Spaces — args: vs qm set --ide0

Using the 6G scheme with model `VMware Virtual IDE Hard Drive`:

| Method | Encoding | QEMU receives | SOFTWARE ID |
|---|---|---|---|
| `args:` with `%20` | URL-encoded | Literal `%20` string | `LGMJ-R75F` ❌ (wrong) |
| `args:` with shell single-quotes | `'ide-hd,...,model=VMware Virtual IDE Hard Drive'` | Correct spaces | `TI09-7WK3` ✅ (correct) |
| `qm set --ide0` with `%20` | `model=VMware%20Virtual%20IDE%20Hard%20Drive` | Correct spaces (PVE decodes) | `TI09-7WK3` ✅ (correct) |
| `qm set --ide0` with shell quotes | `model="VMware Virtual IDE Hard Drive"` | Parse error | N/A (fails) |

**Conclusion**: The two paths have opposite encoding requirements. The `args:` raw QEMU command-line method passes strings directly to QEMU — QEMU does not URL-decode, so `%20` fails and shell single-quotes are required for spaces. The `qm set --ide0` property-string method uses PVE's property-string parser, which does URL-decode `%20`; shell quotes instead produce an "invalid format - format error". The `qm set --ide0` method is recommended for all new deployments.

---

## Experiment 5: 6G and 42G Produce the Same SOFTWARE ID

Two entirely different parameter sets produce the same SOFTWARE ID `TI09-7WK3`:

| | 6G | 42G |
|---|---|---|
| Model | `VMware Virtual IDE Hard Drive` | `n4X7W6eSOxyxUhOd` |
| Serial | `00000000000000000001` | `G4HQT594JN8VLY0FGN9` |
| Size | 6,442,450,944 | 45,097,156,608 |
| 0xB3-0xFF | all zero | all zero |
| SOFTWARE ID | `TI09-7WK3` | `TI09-7WK3` |

However, the MBR signatures (0x110-0x14F) of the two are completely different and cannot be interchanged.

---

## Experiment 6: Base64 Key Decoding vs MBR Binary

Attempted to write the Base64-decoded Key text into 0x110-0x14F:

| | Base64 decoded | Extracted from raw img |
|---|---|---|
| 6G 0x110-0x14F | 64 bytes (decoded result) | 64 bytes (from ROS_6G_clean.img) |
| Content | **identical** ✅ | — |
| 8G 0x110-0x14F | 64 bytes (decoded result) | 64 bytes (from SSD08G-noconfig.img) |
| Content | **different** ❌ | — |

**Conclusion**: The Base64 Key and the MBR binary are not a uniform mapping. The 6G case happened to match; the 8G case did not. Data extracted from the raw img is the most reliable source.

---

## Experiment 7: RouterOS Modifies 0x10C at Boot

On every RouterOS boot, 0x10C changes from `00` to `01`. This does not affect an authorization that has already been written and applied.

---

## Experiment 8: Effect of Disk Size on SOFTWARE ID (VM 916)

16G disk, using the 6G scheme's serial + model + the full 80-byte MBR:

| | 6G original | 16G test |
|---|---|---|
| Disk size | 6,442,450,944 | 17,179,869,184 |
| Serial | `00000000000000000001` | `00000000000000000001` |
| Model | `VMware Virtual IDE Hard Drive` | `VMware Virtual IDE Hard Drive` |
| MBR 0x100-0x14F | identical (6G's 80 bytes) | identical |
| SOFTWARE ID | `TI09-7WK3` | **`6ZXK-WCJL`** ← different |
| Authorization | ✅ nlevel: 6 | ❌ ROUTER HAS NO SOFTWARE KEY |

![16G experiment](../images/vm916-16g-different-id.png)

**Conclusion**: Disk size participates in the SOFTWARE ID computation. With identical serial + model + MBR data, a different disk size yields a different SOFTWARE ID. Custom disk sizes require reverse-engineering the `f()` algorithm in `keygen_x86`.
