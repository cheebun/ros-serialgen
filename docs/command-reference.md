# Command Reference

Every `ros-serialgen` subcommand and flag, explained. Use this when you need to know exactly what a parameter does before running it.

For PVE/QEMU deployment commands (`qm`, `qemu-img`, `qemu-nbd`, `dd`, `lvcreate`, etc.), see the inline explanations in [deployment-guide.md](deployment-guide.md).

---

## `ros-serialgen search`

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

## `ros-serialgen check`

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

## `ros-serialgen sig2key <signature_hex>`

Positional argument: a 128-character hex string (64 bytes) -- the signature from the [Signature Table](collision-database.md#signature-table). Outputs the corresponding `-----BEGIN MIKROTIK SOFTWARE KEY-----...` block.

## `ros-serialgen key2sig <key_file>`

Positional argument: path to a `.key` file containing MikroTik key text. Outputs the 128-character signature hex.

## `ros-serialgen verify`

No arguments. Runs the algorithm's built-in self-check against known test vectors and prints pass/fail.
