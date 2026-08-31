# hap_ac2-7.16.1.bin — Analysis Notes

Analysis of a full flash dump from a real hAP ac² device (2026-08-31).

## File identification

- Size: 16,777,216 bytes (16 MiB) exactly
- `file`: ELF 32-bit LSB executable, ARM, EABI5, statically linked, no section header
- Contains Qualcomm DAL/bootloader strings (`SOC_HW_VERSION`, `FOUNDRY`, `DALGLBCTXT`, etc.) — consistent with hAP ac²'s Qualcomm IPQ40xx SoC
- This is a combined bootloader + dual-firmware-partition image, not a single plain ELF binary

## Structure

Three regions identified by scanning for squashfs (`hsqs`) magic and manually mapping the gap between them:

| Region | Offset (hex) | Size | Content |
|---|---|---|---|
| Bootloader / header | `0x000000` – `0x990820` | ~10.0 MiB | ELF bootloader stub + padding |
| squashfs #1 | `0x990820` – `0xb6f85c` | 1,962,044 bytes | Read-only rootfs (major.minor 4.0, XZ compression, 67 inodes) |
| **Writable config gap** | `0xb6f85c` – `0xc81450` | 1,121,268 bytes | **Live device config — see below** |
| squashfs #2 | `0xc81450` – ??? | superblock claims 9,567,756 bytes, which exceeds the file's remaining size — extraction not yet successful | Likely second (failsafe/backup) rootfs partition |

### squashfs #1 extraction status

Superblock parses cleanly and completely (all fields sane: `bytes_used`, `inode_table_start`, `directory_table_start`, `fragment_table_start`, `lookup_table_start` all fall within bounds; `xattr_id_table_start` is the correct `0xFFFFFFFFFFFFFFFF` "no xattrs" sentinel). However, `unsquashfs` (squashfs-tools 4.7.5) fails with `FATAL ERROR: File system corruption detected` — the 8-byte pointer at `id_table_start` (the very last 8 bytes of the declared filesystem) does not decode to a plausible metadata block reference; surrounding bytes look like a continuation of compressed data rather than a clean table entry. Not resolved — possibly a vendor-specific squashfs variant, or the true id-table position differs from the declared `id_table_start` for this build.

### squashfs #2

The `hsqs` magic at `0xc81450` parses as a structurally valid squashfs 4.0 superblock (same compression/major/minor as #1), but its declared `bytes_used` (9,567,756 bytes) would require the filesystem to extend to file offset `0x15a125c` — beyond the 16 MiB dump's end. Either this capture is truncated, this offset isn't actually a genuine squashfs start (coincidental `hsqs` bytes inside other data), or there's a field-layout difference not yet accounted for. Not investigated further.

## Confirmed real-device data (writable config gap)

The gap between the two squashfs regions contains **live, device-specific configuration** in RouterOS's internal binary config format (length-prefixed ASCII strings), not template/firmware content — confirmed by real, sequential factory-assigned MAC addresses (not zeroed/placeholder values):

| Interface | MAC address |
|---|---|
| ether1 | `48:8f:5a:3d:91:dc` |
| ether2 | `48:8f:5a:3d:91:dd` |
| ether3 | `48:8f:5a:3d:91:de` |
| ether4 | `48:8f:5a:3d:91:df` |
| ether5 | `48:8f:5a:3d:91:e0` |
| wlan1 (2.4GHz) | `48:8f:5a:3d:91:e1` |
| wlan2 (5GHz) | `48:8f:5a:3d:91:e2` |

Also present:
- Default SSID `MikroTik-3D91E1` (matches RouterOS's `ssid = "MikroTik-" + wlanMac[9:11]+[12:14]+[15:17]` default-config generation rule, using wlan1's MAC)
- Default hostname `router.lan`
- Full stock `defconf` script content (default bridge/DHCP/firewall setup — standard RouterOS out-of-box config, not evidence of custom configuration)
- A handful of **other people's device hostnames** that had associated to this device's WiFi at some point (`WLZ-AN00`, `nova_5i-fac86bcfcb`, `NCO-AL00`, each with a partial MAC) — third-party data, not this device's own identity; not investigated further for privacy reasons

