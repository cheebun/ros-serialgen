# Command Reference

Every command used in [quick-start.md](quick-start.md) and [deployment-guide.md](deployment-guide.md), broken down flag by flag. Use this when you need to know exactly what a parameter does before running it.

---

## ros-serialgen

The collision search and key conversion CLI built by this project.

### `ros-serialgen search`

```bash
ros-serialgen search -s 100 -t 16 -c 0 -k keys.toml
```

| Flag | Long form | Meaning |
|---|---|---|
| `-s <GB>` | `--disk-gb <GB>` | Disk size in gigabytes. Determines `sector_val` -- must match the disk you'll actually create. **Required.** |
| `-t <N>` | `--threads <N>` | Number of search threads. Defaults to all available CPU cores if omitted. |
| `-m <name>` | `--model <name>` | Disk model string to search under. Defaults to `ROS<GB>G` (e.g. `ROS100G`) if omitted. |
| `-k <path>` | `--keys <path>` | Path to `keys.toml`. Defaults to `./keys.toml` if omitted. |
| `-c <N>` | `--count <N>` | Number of collisions to find before stopping. `1` (default) stops at the first hit; `0` runs until interrupted (Ctrl+C), collecting every hit. |
| `-f <N>` | `--from <N>` | Resume the search from N million hashes in, matching the `M` value printed in progress output. Defaults to `0` (start from the beginning). |

### `ros-serialgen check`

```bash
ros-serialgen check --serial 00000000090681934458 -s 24 -m cheerlon
```

| Flag | Long form | Meaning |
|---|---|---|
| `--serial <value>` | -- | The 20-character serial number to verify. **Required.** No short form (`-s` is reserved for disk size). |
| `-s <GB>` | `--disk-gb <GB>` | Disk size in gigabytes used to compute `sector_val`. **Required.** |
| `-m <name>` | `--model <name>` | Disk model string. Defaults to `ROS<GB>G` if omitted. |
| `-k <path>` | `--keys <path>` | Path to `keys.toml`. Defaults to `./keys.toml` if omitted. |

Prints the computed SOFTWARE ID, and if it matches a known signature, the License Key and MBR hex.

### `ros-serialgen sig2key <signature_hex>`

Positional argument: a 128-character hex string (64 bytes) -- the signature from the [Signature Table](collision-database.md#signature-table). Outputs the corresponding `-----BEGIN MIKROTIK SOFTWARE KEY-----...` block.

### `ros-serialgen key2sig <key_file>`

Positional argument: path to a `.key` file containing MikroTik key text. Outputs the 128-character signature hex.

### `ros-serialgen verify`

No arguments. Runs the algorithm's built-in self-check against known test vectors and prints pass/fail.

---

## Proxmox VE (`qm`)

### `qm create`

```bash
qm create 100 \
  --name ros100 \
  --machine q35 \
  --bios ovmf \
  --efidisk0 local:0,efitype=4m,pre-enrolled-keys=0,size=4M \
  --vga std \
  --cpu host \
  --cores 2 \
  --memory 2048 \
  --net0 virtio,bridge=vmbr0 \
  --scsihw virtio-scsi-single \
  --ostype l26 \
  --agent 0
```

| Argument | Meaning |
|---|---|
| `100` (positional) | The VM ID to create. Must be unused -- check with `qm list`. |
| `--name ros100` | Display name shown in the PVE UI. Cosmetic only. |
| `--machine q35` | QEMU machine type. `q35` is the modern chipset (PCIe support); `i440fx` is the legacy alternative. Either works for RouterOS. |
| `--bios ovmf` | Use UEFI firmware (OVMF) instead of legacy SeaBIOS. **Required** for RouterOS x86 to boot correctly on current ISOs. |
| `--efidisk0 local:0,efitype=4m,pre-enrolled-keys=0,size=4M` | Creates the small EFI variable-storage disk UEFI needs. `local:0` = auto-allocate on the `local` storage; `efitype=4m` = 4 MiB EFI disk layout; `pre-enrolled-keys=0` = do not pre-enroll Secure Boot keys (RouterOS isn't signed for Secure Boot); `size=4M` = disk size. |
| `--vga std` | Standard VGA display adapter for console access. |
| `--cpu host` | Pass the host CPU model through to the VM (best performance, matches host instruction set). |
| `--cores 2` | Number of virtual CPU cores. RouterOS runs fine on 1; adjust to taste. |
| `--memory 2048` | RAM in MiB. RouterOS needs very little (256 MB minimum); higher values help if you'll run heavier configs/packages. |
| `--net0 virtio,bridge=vmbr0` | One network interface, `virtio` model (paravirtualized, fastest), bridged to `vmbr0`. |
| `--scsihw virtio-scsi-single` | SCSI controller type for any SCSI disks. Not used by the IDE-attached license disk in this workflow, but required by `qm create` for VM initialization. |
| `--ostype l26` | Guest OS type hint (Linux 2.6+ kernel). Tunes some QEMU defaults; RouterOS is Linux-based so this is accurate. |
| `--agent 0` | Disable the QEMU guest agent channel. RouterOS doesn't run `qemu-guest-agent`, so leaving this enabled just adds a non-functional device. |

### `qm set` -- disk removal and attachment

```bash
qm set 100 --delete scsi0
qm set 100 --ide0 local:100/vm-100-disk-1.qcow2,model=ROS6G,serial=00000000401012206606
```

| Argument | Meaning |
|---|---|
| `100` (positional) | Target VM ID. |
| `--delete scsi0` | Detach and remove the auto-generated default disk (`qm create` sometimes adds one depending on storage defaults). Safe to run even if `scsi0` doesn't exist. |
| `--ide0 <storage>:<path>,model=X,serial=Y` | Attach a disk to the IDE bus, slot 0. `local:100/vm-100-disk-1.qcow2` is `<storage-id>:<vmid>/<filename>` for directory storage, or `local-lvm:vm-100-disk-0` (`<storage-id>:<volume-name>`) for LVM storage. `model=` and `serial=` are QEMU IDE drive properties -- these are what the RouterOS SOFTWARE ID algorithm reads to identify the disk. |

### `qm set` -- ISO and boot order

```bash
qm set 100 --ide2 local:iso/mikrotik-7.24.iso,media=cdrom --boot order=ide2
qm set 100 --delete ide2
qm set 100 --boot order=ide0
```

| Argument | Meaning |
|---|---|
| `--ide2 <storage>:iso/<file>,media=cdrom` | Attach an ISO image as a virtual CD-ROM on IDE slot 2. `media=cdrom` tells QEMU to present it as removable optical media, not a hard disk. |
| `--boot order=ide2` | Boot priority list. `order=ide2` boots from the CD-ROM first (for installation). |
| `--delete ide2` | Detach the CD-ROM once installation is done -- otherwise the VM may boot back into the installer. |
| `--boot order=ide0` | Switch boot priority to the license disk (`ide0`) for normal operation. |

### `qm config`

```bash
qm config 100 | grep ide0
```

Prints the VM's full configuration (the contents of `/etc/pve/qemu-server/100.conf`). Piping to `grep ide0` isolates the disk attachment line, useful for confirming the `model=`/`serial=` values were stored as you intended (including `%20`-encoded spaces).

### `qm monitor`

```bash
qm monitor 100
(qemu) info qtree
```

Opens an interactive QEMU monitor session attached to the running VM. `info qtree` (typed at the `(qemu)` prompt) dumps the live device tree, including the actual `model` string QEMU is using for the IDE drive -- useful for confirming PVE decoded a `%20`-encoded value back into a real space. Exit with `Ctrl+A` then `X` (in a terminal) or `q` typed at the prompt, depending on your access method.

### `qm start` / `qm stop` / `qm list`

| Command | Meaning |
|---|---|
| `qm start 100` | Power on the VM. |
| `qm stop 100` | Force power-off (like pulling the plug) -- used before writing the MBR offline, since the disk file must not be in use. |
| `qm list` | List all VMs and their IDs, to find a free VM ID before creating a new one. |

---

## Proxmox VE storage inspection

| Command | Meaning |
|---|---|
| `pvesm status` | Lists all configured storage pools and their type (`dir`, `lvmthin`, `lvm`, `zfspool`, etc.) and free space. Use this to determine whether you're on directory or LVM storage before choosing a disk-creation method. |
| `vgs` | Lists LVM volume groups (e.g. `pve`) and their size/free space. |
| `lvs pve` | Lists logical volumes inside the `pve` volume group, including thin pool names (e.g. `data`). |
| `blockdev --getsize64 /dev/pve/vm-100-disk-0` | Prints the exact size in bytes of a block device -- used to confirm an LVM volume landed on the precise byte count a collision requires. |

---

## LVM volume creation

```bash
lvcreate -T pve/data -V 6442450944B -n vm-100-disk-0
lvcreate -L 6442450944B -n vm-100-disk-0 pve
```

| Flag | Meaning |
|---|---|
| `-T pve/data` | Create a **thin** volume backed by the thin pool `data` inside volume group `pve`. Thin pools allocate space on demand and don't round to the VG's physical extent size, which is why they can hit exact byte counts. |
| `-V 6442450944B` | Thin volume size, in bytes (the trailing `B` means "bytes", not the default MiB/GiB unit). |
| `-L 6442450944B` | Thick volume size, in bytes. Used without `-T` for a plain (non-thin) volume group. Thick volumes are rounded to the VG's physical extent size (commonly 4 MiB), so this may not hit an exact byte count for all collisions. |
| `-n vm-100-disk-0` | Logical volume name. Follows PVE's naming convention (`vm-<vmid>-disk-<N>`) so PVE recognizes it as belonging to VM 100. |
| `pve` (trailing positional, `-L` only) | The volume group to create the thick volume in. |

---

## Disk image and block device tools

### `qemu-img create`

```bash
qemu-img create -f qcow2 /var/lib/vz/images/100/vm-100-disk-1.qcow2 6442450944
```

| Argument | Meaning |
|---|---|
| `-f qcow2` | Output format: QEMU Copy-On-Write v2, the standard PVE directory-storage disk format. |
| `/var/lib/vz/images/100/vm-100-disk-1.qcow2` | Output file path. `local` directory storage keeps disk images under `/var/lib/vz/images/<vmid>/`. |
| `6442450944` (trailing positional) | Exact disk size in bytes. |

### `qemu-nbd`

```bash
modprobe nbd max_part=8
qemu-nbd --connect=/dev/nbd0 /var/lib/vz/images/100/vm-100-disk-1.qcow2
qemu-nbd --disconnect /dev/nbd0
```

| Command | Meaning |
|---|---|
| `modprobe nbd max_part=8` | Load the kernel's Network Block Device driver, allowing up to 8 partitions per NBD device. Required once per boot before `qemu-nbd` can be used. |
| `qemu-nbd --connect=/dev/nbd0 <qcow2>` | Mount a qcow2 file as a raw block device at `/dev/nbd0`, so tools like `dd` can operate on it directly (qcow2 is a container format, not raw bytes on disk). |
| `qemu-nbd --disconnect /dev/nbd0` | Unmount the NBD device once you're done writing/reading. |

### `dd` (MBR write)

```bash
echo -n "<80-byte-hex>" | xxd -r -p | dd of=/dev/nbd0 bs=1 seek=256 count=80 conv=notrunc
```

| Part | Meaning |
|---|---|
| `echo -n "<hex>"` | Print the 80-byte hex string (header + signature) with no trailing newline. |
| `xxd -r -p` | Reverse hex dump (`-r`) in plain/postscript mode (`-p`) -- converts the ASCII hex string back into raw binary bytes. |
| `dd of=/dev/nbd0` | Write to the block device (or LVM volume path directly, e.g. `/dev/pve/vm-100-disk-0`). |
| `bs=1` | Block size of 1 byte -- required for byte-precise seeking (larger block sizes would round the seek offset). |
| `seek=256` | Skip 256 bytes into the target before writing -- `0x100` in decimal, the start of the RouterOS license region. |
| `count=80` | Write exactly 80 bytes (the full license region, `0x100`-`0x14F`). |
| `conv=notrunc` | Do not truncate the target file/device -- without this, `dd` would shrink the disk to 80 bytes. |

### `hexdump` (MBR verification)

```bash
hexdump -C -s 0x100 -n 80 /dev/nbd0
```

| Flag | Meaning |
|---|---|
| `-C` | Canonical output: hex bytes plus an ASCII sidebar, easiest format to eyeball. |
| `-s 0x100` | Start reading at offset `0x100` (256 decimal) -- skips straight to the license region. |
| `-n 80` | Read exactly 80 bytes. |

---

## Key import (HTTP method)

```bash
mkdir -p /tmp/serve
cat > /tmp/serve/license.key << 'EOF'
...
EOF
cd /tmp/serve && python3 -m http.server 8080 &
ip addr add 10.255.255.1/24 dev vmbr0
```

| Command | Meaning |
|---|---|
| `mkdir -p /tmp/serve` | Scratch directory to serve the key file from. |
| `cat > /tmp/serve/license.key << 'EOF' ... EOF` | Heredoc: writes everything between the `EOF` markers into `license.key`. The quoted `'EOF'` prevents shell variable expansion inside the pasted key text. |
| `python3 -m http.server 8080 &` | Serve the current directory over HTTP on port 8080, backgrounded (`&`) so the shell stays usable. |
| `ip addr add 10.255.255.1/24 dev vmbr0` | Add a temporary IP address to the PVE bridge, so the RouterOS VM (which shares that bridge) can reach the HTTP server. `/24` is the subnet mask; `10.255.255.1` is an arbitrary address unlikely to collide with your real network -- change it if it does. |

## RouterOS console commands

```
/ip address add address=10.255.255.2/24 interface=ether1
/tool fetch url="http://10.255.255.1:8080/license.key" dst-path=license.key
/system license import file-name=license.key
/system license print
```

| Command | Meaning |
|---|---|
| `/ip address add address=10.255.255.2/24 interface=ether1` | Give the RouterOS VM's first interface an address on the same temporary subnet as the PVE host, so it can reach the HTTP server. |
| `/tool fetch url="..." dst-path=license.key` | Download the key file from the PVE host's HTTP server. `dst-path` sets the local filename -- **must** end in `.key`, or import will silently fail to recognize it. |
| `/system license import file-name=license.key` | Import the downloaded key. Prompts `Reboot? [y/N]:` -- answer `y` to activate. |
| `/system license print` | Show the current license state (`software-id`, `nlevel`, `features`). `nlevel: 6` confirms L6 is active. |

Console direct import skips the file entirely:

```
/system/license/import "-----BEGIN MIKROTIK SOFTWARE KEY-----...-----END...-----"
```

The full key text (including BEGIN/END markers) is passed as a single quoted string argument to `/system/license/import`.
