# RouterOS L6 Quick Start (Any Disk Size)

> L6 authorization for any disk size, achieved through SOFTWARE ID collision search.

---

## How It Works

1. ros-serialgen searches for a serial that produces a known SOFTWARE ID for the given disk size
2. Known SOFTWARE IDs have valid L6 signatures (64 bytes)
3. Fixed header (`00...BD E8`) + signature = 80-byte MBR authorization data
4. Write to MBR or import Key text to activate

---

## Step 1: Search for a Collision Serial

```bash
# Build
cd tools/rust
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Search (4 threads, unlimited mode)
./target/release/ros-serialgen search -s 6 -t 4 -c 0 -k keys.toml

# Resume from progress checkpoint
./target/release/ros-serialgen search -s 42 -t 4 -c 0 -f 90000 -k keys.toml

# Background run
nohup ./ros-serialgen search -s 48 -t 2 -c 0 -k keys.toml \
  > /tmp/48g_results.txt 2> /tmp/48g_progress.txt &
```

Output:
```
FOUND [1] serial=00000000XXXXXXXXXXXX target=4MZF-SFTR verified=4MZF-SFTR
```

Note the **serial**, **model** (default `ROS<GB>G`), and **target** (SOFTWARE ID).

---

## Step 2: Create VM

Example with a 6G disk and VM ID 100:

```bash
qm create 100 \
  --name RouterOS \
  --ostype l26 \
  --bios ovmf \
  --efidisk0 local-lvm:1,efitype=4m,format=raw \
  --cores 1 --memory 256 \
  --ide2 local:iso/mikrotik-7.23.2.iso,media=cdrom \
  --net0 virtio,bridge=vmbr0 \
  --boot "order=ide2" \
  --scsihw virtio-scsi-single

qm set 100 --delete scsi0
mkdir -p /var/lib/vz/images/100
qemu-img create -f qcow2 /var/lib/vz/images/100/vm-100-disk.qcow2 6442450944
qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=ROS6G,serial=00000000401012206606
```

For model names with spaces, use `%20` URL encoding in `qm set`:

```bash
qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=VMware%20Virtual%20IDE%20Hard%20Drive,serial=00000000000000000001
```

> For details on space handling and storage backends, see [deployment-guide.md](deployment-guide.md#model-names-containing-spaces).

---

## Step 3: Install RouterOS

1. Start the VM and open the console
2. Press `a` to select all packages, `i` to install, `y` to confirm formatting
3. When installation finishes, **stop the VM** (do not reboot)

---

## Step 4: Write MBR

Based on the **target** (SOFTWARE ID) found in Step 1, select the matching signature below.

### Signature Table

| SOFTWARE ID | Signature hex (64 bytes) |
|---|---|
| TI09-7WK3 | `E67A8F47AE86672FAE6D91DF19221453B34FE40E23F19E917107C449DDCB1D2061521816AD7730671B4CB226F1B0DB7448923C6297C49BDB3CCBF40AECBBCF0B` |
| 4MZF-SFTR | `080342D34683448A1C8E3952E5A5D315F1C5FB4E4EB419C94FB88170DF0290EE3F4DFB796ECA3034D93E934B3FC27169D6506C88F23FE508B26F83546C335A05` |
| HHJH-UFWL | `B08F6DA0CE6D8A13357403F0146B1DD227C5DEBFBD1B8260BE38DB0016D8B0BD110B34457997C8AC956FB7551081C1CB8DA79C0E6160A8DFE79F6FC38E543905` |
| C7CU-PGT9 | `F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07` |

Example below uses the C7CU-PGT9 signature:

```bash
modprobe nbd max_part=8
qemu-nbd --connect=/dev/nbd0 /var/lib/vz/images/100/vm-100-disk.qcow2
sleep 1

echo -n '00000000000000000000BDE800000000F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07' | xxd -r -p | dd of=/dev/nbd0 bs=1 seek=256 count=80 conv=notrunc

hexdump -C -s 0x100 -n 80 /dev/nbd0
qemu-nbd --disconnect /dev/nbd0
```

Verify the hexdump output:
- `0x10A-0x10B` is `bd e8`
- Bytes from `0x110` onward match the selected signature

---

## Alternative: Key Import

If you prefer not to write the MBR directly, you can import a Key text file instead (no shutdown required after install).

### Key Text Table

**TI09-7WK3**:
```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```

**4MZF-SFTR**:
```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```

**HHJH-UFWL**:
```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```

**C7CU-PGT9**:
```
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
```

### Import Steps

On the PVE host:

```bash
mkdir -p /tmp/serve
cat > /tmp/serve/license.key << 'EOF'
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
EOF
cd /tmp/serve && python3 -m http.server 8080 &
ip addr add 10.255.255.1/24 dev vmbr0 2>/dev/null
```

In the RouterOS console:

```
/ip address add address=10.255.255.2/24 interface=ether1
/tool fetch url="http://10.255.255.1:8080/license.key" dst-path=license.key
/system license import file-name=license.key
```

Enter `y` to reboot when prompted.

> The file must have a `.key` extension (not `.txt`).

---

## Step 5: Boot

```bash
qm set 100 --delete ide2
qm set 100 --boot ''
qm start 100
```

---

## Step 6: Verify

```
/system license print
```

Expected output:

```
  software-id: XXXX-XXXX
       nlevel: 6
     features:
```

---

## Collision Table

| Disk Size | Serial | Model | SOFTWARE ID |
|---|---|---|---|
| 6G | `00000000000000000001` | `VMware Virtual IDE Hard Drive` | TI09-7WK3 |
| 6G | `00000000401012206606` | `ROS6G` | C7CU-PGT9 |
| 8G | `HKHYPO14032703B0778` | `SSD08G` | 4MZF-SFTR |
| 16G | `00000000202155543391` | `ROS16G` | 4MZF-SFTR |
| 32G | `SZHYPO14090903D0164` | `SSD32G` | HHJH-UFWL |
| 42G | `G4HQT594JN8VLY0FGN9` | `n4X7W6eSOxyxUhOd` | TI09-7WK3 |
| 42G | `00000001855210443015` | `ROS42G` | 4MZF-SFTR |
| 42G | `00000002074332007468` | `ROS42G` | C7CU-PGT9 |
| 42G | `00000002201539438409` | `ROS42G` | TI09-7WK3 |
| 42G | `00000002448898419424` | `ROS42G` | HHJH-UFWL |
| 48G | `00000000398318370243` | `ROS48G` | TI09-7WK3 |
| 48G | `00000000470627909740` | `ROS48G` | HHJH-UFWL |
| 48G | `00000000580121167237` | `ROS48G` | 4MZF-SFTR |
| 48G | `00000000621828037033` | `ROS48G` | C7CU-PGT9 |
| 64G | `HYSSD-20160419B79028` | `SSD64G2016` | C7CU-PGT9 |
| 100G | `00000000418756277141` | `ROS100G` | 4MZF-SFTR |
| 128G | `00000000311309782924` | `ROS128G` | 4MZF-SFTR |
| 250G | `00000000146612334244` | `ROS250G` | 4MZF-SFTR |
| 256G | `00000000031811615027` | `ROS256G` | 4MZF-SFTR |
| 500G | `00000000082620520955` | `ROS500G` | TI09-7WK3 |
| 512G | `00000000037935077152` | `ROS512G` | TI09-7WK3 |
| 900G | `00000000254781369303` | `ROS900G` | 4MZF-SFTR |
| 960G | `00000000078119970871` | `ROS960G` | 4MZF-SFTR |
| 1000G | `00000000166128957477` | `ROS1000G` | C7CU-PGT9 |

> Full collision database with 190+ entries: [collision-database.md](collision-database.md)
>
> Search command: `ros-serialgen search -s <GB> -t <threads> -c 0 -k keys.toml`
> Resume search: `ros-serialgen search -s <GB> -t <threads> -c 0 -f <progress-M> -k keys.toml`
