# Toolchain Reference

Tools, dependencies, and operational commands used in this project.

---

## External Projects

| Project | URL | Used For |
|---|---|---|
| MikroTikPatch | https://github.com/elseif/MikroTikPatch | Custom SHA-256 constants (IV, K), MTBase64, KCDSA implementation in `mikro.py`; `keygen_x86` binary analysis |
| MTLic | https://github.com/Ygnecz/MTLic | License file parser, `MT_Transform`/`MT_TransformRev` encrypt/decrypt, MTBase64 encode/decode |

---

## ros-serialgen (This Project)

Rust-based collision search tool with AVX-512 SIMD acceleration.

### Build

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Search

```bash
# Search for a collision at a given disk size
ros-serialgen search -s <GB> -t <threads> -c 0 -k keys.toml

# Resume from checkpoint
ros-serialgen search -s <GB> -t <threads> -c 0 -f <progress_M> -k keys.toml

# Background execution
nohup ros-serialgen search -s <GB> -t <threads> -c 0 -k keys.toml \
  > /tmp/results.txt 2> /tmp/progress.txt &
```

### Verify

```bash
ros-serialgen check --serial <Serial> -s <GB>
```

### Test

```bash
cargo test
```

---

## PVE Operations

For a flag-by-flag explanation of every `qm`, `qemu-img`, `qemu-nbd`, `dd`, and `hexdump` command used in this project, see [command-reference.md](command-reference.md).

---

## Reverse Engineering

### keyman Extraction from RouterOS

```bash
# Extract squashfs from RouterOS image (NPK format)
dd if=/rw/pdb/system/image of=/tmp/routeros.squashfs bs=1 skip=4096
unsquashfs -d /tmp/squashfs_edit /tmp/routeros.squashfs
cp /tmp/squashfs_edit/nova/bin/keyman /tmp/keyman
```

### keyman Analysis

```bash
# Run keyman via chroot on PVE
mount --bind /dev /tmp/ros_chroot/dev
ln -s /dev/sda /tmp/ros_chroot/dev/root-disk
chroot /tmp/ros_chroot /bin/qemu-i386-static -strace /nova/bin/keyman --software-id
```

Key findings:
- keyman reads disk serial/model via `ioctl(HDIO_DRIVE_CMD, ATA_CMD_ID_ATA)`
- RouterOS custom ioctl `0x80044604` takes priority over HDIO
- keyman resides in squashfs (read-only); cannot be replaced without firmware modification
- RouterOS 7.23.2 has closed the devel backdoor

---

## C Brute-Force Bug Fix History

| Version | Bug | Fix |
|---|---|---|
| bf3 | `snprintf` null terminator overwrote first byte of model | Used tmp buffer + `memcpy` |
| bf4 | SHA-256 output byte order wrong (stored LE, should be read BE then LE) | Corrected to BE output, LE read |
| bf5 | MBR mix value hardcoded wrong (`0x1EEF` default, not actual) | Computed actual `sha_val=0x1742`, `mbr_val=0x0BD` |
| bf6 | Search space assumed wrong MBR header | Discovered key import bypasses the MBR header issue |

The C tool has been superseded by ros-serialgen (Rust, AVX-512).

---

## Key Experimental Findings

| Experiment | Finding |
|---|---|
| VM 920 vs 921 | Installer overwrites `0x10A-0x10B` (`BD E8` -> `FF FF`) |
| VM 916 (16G) | Disk size affects SOFTWARE ID |
| Differential analysis | Both serial and model participate; no ATA byte swap |
| 6G vs 42G signatures | Signature binds to SOFTWARE ID, not disk parameters |
| `.txt` vs `.key` import | Only `.key` extension is accepted |
| MBR write vs key import | Both activate L6; key import is simpler |

See [experiments.md](experiments.md) for full details.
