# PVE RouterOS L6 VM Deployment Guide

> Step-by-step guide for creating L6-licensed RouterOS x86 VMs on Proxmox VE.

---

## Prerequisites

- Proxmox VE 8.x / 9.x (verified on PVE 9.2.4)
- RouterOS official ISO ([download](https://mikrotik.com/download))
- ISO uploaded to PVE storage: Datacenter → Storage → ISO Images → Upload

---

## Quick Path

If your disk size is in the [collision database](collision-database.md), grab the Serial/Model and skip to [Step 2](#step-2-create-vm).

---

## Full Walkthrough

### Step 1: Search for Collision (new sizes only)

```bash
# Build ros-serialgen (first time only)
cd tools/rust && cargo build --release

# Search (roughly 2 hours on 16 cores)
./target/release/ros-serialgen search -s 200 -t 16
```

Record the **Serial**, **Model**, and **SOFTWARE ID** printed in the output.

### Step 2: Create VM

PVE Web UI → Create VM:

| Setting | Value |
|---|---|
| VM ID | as needed (this guide uses `100`) |
| Name | e.g. `RouterOS-16G` |
| OS Type | Linux, 6.x - 2.6 Kernel |
| ISO Image | the uploaded RouterOS ISO |
| BIOS | **OVMF (UEFI)** |
| EFI Storage | `local-lvm`, check **Add EFI Disk** |
| Machine | i440fx (default) |
| CPU | 1 core (adjust as needed) |
| Memory | 256 MB (adjust as needed) |
| Disk | any placeholder disk (replaced in Step 3) |
| Network | virtio, bridged to the desired vmbr |

**Do not start the VM after creation.**

CLI equivalent (VM ID 100, 16G example) — do NOT use the `args:` raw QEMU command line method; attach the disk with `qm set --ide0` as shown in Step 3.

### Step 3: Disk Attachment — Storage Backends

**Directory storage (e.g. `local`):**

```bash
qm set 100 --delete scsi0
mkdir -p /var/lib/vz/images/100
qemu-img create -f qcow2 /var/lib/vz/images/100/vm-100-disk.qcow2 17179869184
qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=ROS16G,serial=00000000202155543391
```

**LVM storage (e.g. `local-lvm`):**

```bash
# Thin-provisioned (recommended)
lvcreate -T pve/data -V 17179869184B -n vm-100-disk-0

# Thick-provisioned (fallback — extent rounding may prevent exact byte-size collisions)
lvcreate -L 17179869184B -n vm-100-disk-0 pve
```

> Thick LVM rounds volume size up to the nearest extent boundary (default 4 MiB). If the collision search targeted an exact byte count, the resulting disk size may not match, causing SOFTWARE ID verification to fail. Prefer thin-provisioned LVM, or verify the actual byte size with `lvs --units b` after creation.

Then attach it the same way:

```bash
qm set 100 --delete scsi0
qm set 100 --ide0 local-lvm:vm-100-disk-0,model=ROS16G,serial=00000000202155543391
```

### Model Names Containing Spaces

PVE's `qm set` property-string parser splits on commas and requires **URL encoding** (`%20`) for spaces inside the `model=` value:

```bash
qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=VMware%20Virtual%20IDE%20Hard%20Drive,serial=00000000000000000001
```

Shell quoting (`model="VMware Virtual IDE Hard Drive"`) **fails** with `invalid format - format error` — the PVE config parser does not perform shell-style word splitting, so quotes are taken literally as part of the value.

**Verification:**

```bash
qm config 100 | grep ide0
# Shows the %20-encoded value as stored, e.g.:
# ide0: local:100/vm-100-disk.qcow2,model=VMware%20Virtual%20IDE%20Hard%20Drive,serial=00000000000000000001

qm monitor 100
# Then at the (qemu) prompt:
info qtree
# Confirms QEMU decoded %20 to actual spaces in the model string
```

Note: this `%20` encoding is specific to the `qm set --ideN` property-string method. It is different from the older `args:` raw QEMU command-line method, which required shell-style single quotes instead. That method is no longer used in this guide; see [experiments.md](experiments.md) Experiment 4 for the historical comparison.

### Step 4: Install RouterOS

1. PVE Web UI → start the VM → open Console.
2. Press `a` to select all packages, `i` to install, `y` to confirm formatting the disk.
3. After installation completes, **stop the VM** (do not reboot into RouterOS yet).

### Step 5: Activate License

Two methods: MBR write (fastest) or Key import (works without host disk access).

#### MBR write — directory/qcow2 storage

```bash
modprobe nbd max_part=8
qemu-nbd --connect=/dev/nbd0 /var/lib/vz/images/100/vm-100-disk.qcow2
sleep 1
echo -n '00000000000000000000BDE800000000F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07' | xxd -r -p | dd of=/dev/nbd0 bs=1 seek=256 count=80 conv=notrunc
hexdump -C -s 0x100 -n 80 /dev/nbd0
qemu-nbd --disconnect /dev/nbd0
```

#### MBR write — LVM storage (no qemu-nbd needed)

```bash
echo -n '00000000000000000000BDE800000000F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07' | xxd -r -p | dd of=/dev/pve/vm-100-disk-0 bs=1 seek=256 count=80 conv=notrunc
hexdump -C -s 0x100 -n 80 /dev/pve/vm-100-disk-0
```

#### Key import

Start the VM (no need to stop it first). On the PVE host, serve the key file over HTTP:

```bash
mkdir -p /tmp/serve
cat > /tmp/serve/license.key << 'EOF'
<paste the output of ros-serialgen sig2key for the corresponding SOFTWARE ID>
EOF
cd /tmp/serve && python3 -m http.server 8080 &
ip addr add 10.255.255.1/24 dev vmbr0 2>/dev/null
```

In the RouterOS Console:

```
/ip address add address=10.255.255.2/24 interface=ether1
/tool fetch url="http://10.255.255.1:8080/license.key" dst-path=license.key
/system license import file-name=license.key
```

When prompted `Reboot? [y/N]:`, type `y`.

> The file must have a `.key` extension. A `.txt` extension causes the import to fail with `no new key found`.

### Step 6: Boot and Verify

MBR-write installs need the boot CD removed first:

```bash
qm set 100 --delete ide2
qm set 100 --boot ''
qm start 100
```

Key-import installs already rebooted automatically and do not need this step.

Verify in the RouterOS Console:

```
/system license print
```

Expected output:

```
  software-id: XXXX-XXXX
       nlevel: 6
     features:
```

`nlevel: 6` confirms the L6 license is active.

---

## Signature Table

| SOFTWARE ID | Signature hex (64 bytes) |
|---|---|
| TI09-7WK3 | `E67A8F47AE86672FAE6D91DF19221453B34FE40E23F19E917107C449DDCB1D2061521816AD7730671B4CB226F1B0DB7448923C6297C49BDB3CCBF40AECBBCF0B` |
| 4MZF-SFTR | `080342D34683448A1C8E3952E5A5D315F1C5FB4E4EB419C94FB88170DF0290EE3F4DFB796ECA3034D93E934B3FC27169D6506C88F23FE508B26F83546C335A05` |
| HHJH-UFWL | `B08F6DA0CE6D8A13357403F0146B1DD227C5DEBFBD1B8260BE38DB0016D8B0BD110B34457997C8AC956FB7551081C1CB8DA79C0E6160A8DFE79F6FC38E543905` |
| C7CU-PGT9 | `F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07` |

## Key Text

<details>
<summary>TI09-7WK3</summary>

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```
</details>

<details>
<summary>4MZF-SFTR</summary>

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```
</details>

<details>
<summary>HHJH-UFWL</summary>

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```
</details>

<details>
<summary>C7CU-PGT9</summary>

```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```
</details>

---

## FAQ

### VM boots into the installer instead of RouterOS

The boot CD is still in the boot order. Remove it:

```bash
qm set 100 --delete ide2
qm set 100 --boot ''
```

### `ROUTER HAS NO SOFTWARE KEY`

License data was not written, or does not match the disk. Check:

1. Serial and Model match the collision database entry exactly.
2. MBR bytes at offset 0x10A-0x10B read `BD E8` (not `FF FF`).
3. The MBR was written **after** installing RouterOS — the installer overwrites 0x10A-0x10B.

### SOFTWARE ID does not match expected value

- A model name containing spaces was not `%20`-encoded — see [Model Names Containing Spaces](#model-names-containing-spaces).
- The disk size is not exact — recreate it with `qemu-img create` or `lvcreate` specifying the precise byte count.
- Check the stored `model=`/`serial=` values with `qm config 100 | grep ide0`.

### Key import fails with `no new key found`

- Confirm the file extension is `.key`, not `.txt`.
- Confirm the SOFTWARE ID printed by `/system license print` matches the SOFTWARE ID the key was generated for.
- Fall back to the MBR write method instead.

### Migrating the VM to another PVE node

- The `ide0` disk line (including `model=` and `serial=`) migrates automatically with `qm migrate`.
- If the disk file path changes on the target node, verify the `ide0` line in `qm config` still points to a valid storage location.
- Do not convert the disk from qcow2 to raw format — this can change on-disk layout assumptions relied on by the collision search.
