# Toolchain Reference

Reference documentation for external projects, in-house tools, and PVE operations used in this project.

## External Projects

| Project | Link | Purpose |
|---|---|---|
| MikroTikPatch | https://github.com/elseif/MikroTikPatch | Custom SHA-256, Base64, and KCDSA implementations from `mikro.py`; `keygen_x86` binary analysis |
| MTLic | https://github.com/Ygnecz/MTLic | License file parser, MT_Transform/MT_TransformRev encryption/decryption, MTBase64 encoding/decoding |

## In-house Tools

| Tool | Path | Purpose |
|---|---|---|
| `ros-serialgen` | `tools/rust/` | Primary tool (Rust, AVX-512 SIMD): given a disk size in GB, performs a collision search for a serial matching a known SOFTWARE ID |
| `keyman_x86_7.23.2` | `tools/bin/` | Reference binary extracted from the RouterOS 7.23.2 squashfs |
| Archive tools | `archive/old-tools/` | Historical C/Python/GPU implementations, superseded by `ros-serialgen`; not maintained |

## PVE Operations Reference

### Basic Operations
- `qm create <VMID> ...` — create a virtual machine
- `qm clone <VMID> <NEWID>` — clone a VM
- `qm set <VMID> --delete <option>` — remove a disk/CD-ROM device
- `qm config <VMID>` — inspect configuration
- `qm start/stop/destroy <VMID>` — lifecycle management

### Disk Operations
- `qemu-img create -f qcow2 <path> <size_bytes>` — create a qcow2 disk of an exact size
- `qemu-nbd --connect=/dev/nbd0 <qcow2>` — mount a qcow2 image as a block device
- `dd of=/dev/nbd0 bs=1 seek=256 count=80` — write MBR license data
- `hexdump -C -s 0x100 -n 80 /dev/nbd0` — verify MBR contents
- `qemu-nbd --disconnect /dev/nbd0` — disconnect the NBD device

### Disk Attachment

`qm set <VMID> --ide0 <path>,serial=<serial>,model=<model>` is the current method for attaching a disk with a custom serial/model.

The `args:` method below is superseded and kept only for historical/reference purposes:

```conf
# model without spaces
args: -drive file=<path>,format=qcow2,if=none,id=drive0 -device ide-hd,drive=drive0,serial=<serial>,model=<model>

# model with spaces (wrapped in single quotes)
args: -drive file=<path>,format=qcow2,if=none,id=drive0 -device 'ide-hd,drive=drive0,serial=<serial>,model=<model>'
```

### Key Import (simpler than writing the MBR directly)

```bash
# On the PVE host:
mkdir -p /tmp/serve
cat > /tmp/serve/license.key << 'EOF'
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
EOF
cd /tmp/serve && python3 -m http.server 8080 &
ip addr add 10.255.255.1/24 dev vmbr0

# In the RouterOS console:
/ip address add address=10.255.255.2/24 interface=ether1
/tool fetch url="http://10.255.255.1:8080/license.key" dst-path=license.key
/system license import file-name=license.key
# reboot -> L6
```

> The file must use the `.key` extension; a `.txt` extension will fail to import.

## Reverse Engineering Operations

### keyman Extraction
```bash
# Extract the squashfs from the RouterOS image (NPK format)
dd if=/rw/pdb/system/image of=/tmp/routeros.squashfs bs=1 skip=4096
unsquashfs -d /tmp/squashfs_edit /tmp/routeros.squashfs
cp /tmp/squashfs_edit/nova/bin/keyman /tmp/keyman
```

### keyman Analysis (strace + chroot)
```bash
# Run keyman under chroot on the PVE host
mount --bind /dev /tmp/ros_chroot/dev
ln -s /dev/sda /tmp/ros_chroot/dev/root-disk
chroot /tmp/ros_chroot /bin/qemu-i386-static -strace /nova/bin/keyman --software-id
```

### Key Findings
- `keyman` reads the disk serial/model via `ioctl(HDIO_DRIVE_CMD, ATA_CMD_ID_ATA)`
- RouterOS's custom ioctl `0x80044604` takes precedence over HDIO
- `keyman` resides inside the read-only squashfs and cannot be replaced directly
- RouterOS 7.23.2 closes off the devel backdoor

## C Brute-Force Bug Fix History

These fixes apply to the historical C brute-force tool, which is superseded by the Rust tool `ros-serialgen`.

| Version | Bug | Fix |
|---|---|---|
| bf3 (bruteforce.c) | `snprintf` null terminator overwrote the first byte of the model field | Used a temporary buffer + `memcpy` |
| bf4 | SHA-256 output byte order was wrong (stored LE but should be read BE→LE) | Corrected to output BE, then read LE |
| bf5 | MBR mix value was hardcoded incorrectly (0x1EEF was the default value, not the actual value) | Computed the actual value: sha_val=0x1742 -> mbr_val=0x0BD |
| bf6 | Search space did not match the MBR header | Discovered that the key import method bypasses the MBR header mismatch |

## Key Experiments Summary

| Experiment | Finding |
|---|---|
| VM 920 vs 921 | The installer overwrites 0x10A-0x10B (BD E8 -> FF FF) |
| VM 916 (16G) | Disk size affects the SOFTWARE ID |
| T1-T4 differential analysis | Both serial and model participate in the computation; not a simple ATA byte swap |
| 6G vs 42G signature comparison | The signature binds only to the SOFTWARE ID, not to disk parameters |
| Key import (.txt vs .key) | Only the `.key` extension imports successfully |
| MBR write vs key import | Both methods activate the license; key import is simpler |
