# Verification Experiments

Key experiments that validated (or disproved) assumptions about the RouterOS licensing mechanism.

## Test Environment

- PVE 9.2.4
- RouterOS 7.23.2 / 7.24 (both verified)
- BIOS: OVMF (UEFI)
- Date: 2026-07-20

---

## Experiment 1: Five Known Configurations Verified

**Hypothesis**: All five parameter sets from the forum post produce valid L6 licenses on PVE.

**Method**: Created VMs for 6G, 8G, 32G, 42G, 64G with exact serial/model/size, wrote MBR, booted.

| VM | Size | SOFTWARE ID | Result |
|---|---|---|---|
| 931 | 6G | TI09-7WK3 | nlevel: 6 |
| 932 | 8G | 4MZF-SFTR | nlevel: 6 |
| -- | 32G | HHJH-UFWL | nlevel: 6 |
| 933 | 42G | TI09-7WK3 | nlevel: 6 |
| 920 | 64G | C7CU-PGT9 | nlevel: 6 |

**Conclusion**: All five configurations produce valid L6 licenses.

---

## Experiment 2: Installer Overwrites 0x10A-0x10B

**Hypothesis**: MBR license data can be written before installation.

**Method**: Compared VM 920 (MBR rewritten after install) with VM 921 (MBR left as-is after install).

| | VM 920 (rewritten) | VM 921 (as-is) |
|---|---|---|
| 0x10A-0x10B | `BD E8` | `FF FF` (set by installer) |
| SOFTWARE ID | C7CU-PGT9 | C7CU-PGT9 (same) |
| License | nlevel: 6 | ROUTER HAS NO SOFTWARE KEY |

**Conclusion**: The installer resets `0x10A-0x10B` to `FF FF`. These bytes do not affect the SOFTWARE ID but are required for license validation. MBR must be written **after** installation.

---

## Experiment 3: MBR 0x100-0x109 Affects SOFTWARE ID

**Hypothesis**: Bytes `0x100-0x109` are inert metadata.

**Method**: Changed `0x100-0x109` from original values (`47 11 02 26 ...`) to all zeros on the 32G configuration.

| | Original | Modified |
|---|---|---|
| 0x100-0x109 | `47 11 02 26 68 89 83 99 16 18` | `00 00 00 00 00 00 00 00 00 00` |
| SOFTWARE ID | HHJH-UFWL | Different value |
| License | Valid | Invalid |

**Conclusion**: `0x100-0x109` participates in SOFTWARE ID computation via the MBR mixing phase. Changing it changes the ID and breaks the signature match. In our collision scheme, these bytes are set to all zeros, producing a fixed `mbr_val` of `0x0BD`.

---

## Experiment 4: Model Name Space Handling in PVE

**Hypothesis**: URL encoding (`%20`) works for spaces in model names.

**Method**: Tested the 6G configuration (`VMware Virtual IDE Hard Drive`) via a raw `args:` QEMU command line, and separately via `qm set --ide0`'s property-string parser.

| Path | Encoding | QEMU Receives | SOFTWARE ID |
|---|---|---|---|
| Raw `args:` line | `model=VMware%20Virtual%20IDE%20Hard%20Drive` | Literal `%20` characters (not decoded) | LGMJ-R75F (wrong) |
| Raw `args:` line | `-device 'ide-hd,...,model=VMware Virtual IDE Hard Drive'` (shell-quoted) | Correct spaces | TI09-7WK3 (correct) |
| `qm set --ide0` property-string | `model=VMware%20Virtual%20IDE%20Hard%20Drive` | Correct spaces (PVE decodes `%20` before building the QEMU arg) | TI09-7WK3 (correct) |
| `qm set --ide0` property-string | `model="VMware Virtual IDE Hard Drive"` (shell-quoted) | Rejected: `invalid format - format error` | -- |

**Conclusion**: The two paths handle spaces differently. `args:` is passed to QEMU as a raw command line, so `%20` is never decoded -- only shell single-quoting works there. `qm set --ide0` goes through PVE's own property-string parser, which decodes `%20` but rejects literal spaces or shell quotes outright. Since `qm set --ide0` is the simpler and now-standard attachment method (see [deployment-guide.md](../guides/x86-install.md#model-names-containing-spaces)), use `%20` encoding for models with spaces.

---

## Experiment 5: Different Parameters Can Produce the Same SOFTWARE ID

**Hypothesis**: Each SOFTWARE ID maps to a unique parameter set.

**Method**: Compared the 6G and 42G configurations, both producing TI09-7WK3.

| | 6G | 42G |
|---|---|---|
| Model | `VMware Virtual IDE Hard Drive` | `n4X7W6eSOxyxUhOd` |
| Serial | `00000000000000000001` | `G4HQT594JN8VLY0FGN9` |
| Size | 6,442,450,944 | 45,097,156,608 |
| SOFTWARE ID | TI09-7WK3 | TI09-7WK3 |

The MBR signatures (`0x110-0x14F`) are identical for both, confirming the signature binds to the SOFTWARE ID alone.

**Conclusion**: The SOFTWARE ID is a ~40-bit hash; collisions across different parameter sets are expected and exploitable. The signature is reusable for any parameter set that produces the same SOFTWARE ID.

---

## Experiment 6: Disk Size Affects SOFTWARE ID

**Hypothesis**: Only serial and model determine the SOFTWARE ID.

**Method**: Used the 6G serial/model/MBR on a 16G disk (VM 916).

| | 6G Original | 16G Test |
|---|---|---|
| Disk size | 6,442,450,944 | 17,179,869,184 |
| Serial + Model | Identical | Identical |
| MBR 0x100-0x14F | Identical | Identical |
| SOFTWARE ID | TI09-7WK3 | 6ZXK-WCJL |
| License | Valid | ROUTER HAS NO SOFTWARE KEY |

**Conclusion**: Disk size (via `sector_val`) is a mandatory input to the SOFTWARE ID computation. Custom disk sizes require a new collision search.

---

## Experiment 7: RouterOS Modifies 0x10C at Boot

**Hypothesis**: RouterOS does not touch the MBR after initial installation.

**Method**: Observed MBR changes across boots.

**Result**: Byte `0x10C` changes from `00` to `01` on first boot. This is a system counter outside the `0x100-0x109` range used for SOFTWARE ID computation. No impact on licensing.

---

## Experiment 8: Key Import Requires .key Extension

**Hypothesis**: Any file extension works for key import.

**Method**: Imported the same key text as `license.txt` and `license.key`.

| Extension | Result |
|---|---|
| `.txt` | `no new key found` |
| `.key` | License imported, reboot prompt |

**Conclusion**: RouterOS filters by file extension. Only `.key` is accepted.
