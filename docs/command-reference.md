# Command Reference

Every `ros-serialgen` subcommand and flag, explained. Use this when you need to know exactly what a parameter does before running it.

For PVE/QEMU deployment commands (`qm`, `qemu-img`, `qemu-nbd`, `dd`, `lvcreate`, etc.), see the inline explanations in [deployment-guide.md](deployment-guide.md).

---

## `ros-serialgen search`

```bash
ros-serialgen search -s 100 -u g -t 16 -c 0 -k keys.toml
ros-serialgen search -s 128 -u m -t 16 -c 0 -k keys.toml
```

| Flag | Long form | Meaning |
|---|---|---|
| `-s <N>` | `--disk-size <N>` | Disk size magnitude, paired with `-u`/`--unit`. Determines `sector_val` -- must match the disk you'll actually create. **Required.** |
| `-u <unit>` | `--unit <unit>` | Unit for `-s`: `g` (gigabytes, default), `m` (megabytes), `k` (kilobytes), or `b` (raw bytes). Case-insensitive. |
| `-t <N>` | `--threads <N>` | Number of search threads. Defaults to all available CPU cores if omitted. |
| `-m <name>` | `--model <name>` | Disk model string to search under. Defaults to `ROS<N><unit>` (e.g. `ROS100G`, `ROS128M`) if omitted. |
| `-k <path>` | `--keys <path>` | Path to `keys.toml`. Defaults to `./keys.toml` if omitted. |
| `-c <N>` | `--count <N>` | Number of collisions to find before stopping. `1` (default) stops at the first hit; `0` runs until interrupted (Ctrl+C), collecting every hit. |
| `-f <N>` | `--from <N>` | Resume the search from N million hashes in, matching the `M` value printed in progress output. Defaults to `0` (start from the beginning). |

### Minimum disk size per unit

Each unit has a separate minimum, all equivalent to 64 MB, enforced at startup (the process exits with an error if violated):

| Unit | Minimum `-s` value |
|---|---|
| `g` | `1` (1 GB) |
| `m` | `64` (64 MB) |
| `k` | `65536` (64 MB in KB) |
| `b` | `67108864` (64 MB in bytes) |

Decimal sizes are not supported (`-s` is an integer) -- fractional GB values must be expressed in a smaller unit instead, e.g. `-s 1536 -u m` for 1.5 GB. This avoids floating-point rounding errors in the byte-exact `sector_val` calculation.

Progress is logged every 10,000M (10 billion) hashes, e.g. `10000M hashes, 5s, 0 found`. At ~2000M hash/s (AVX-512) that's roughly every 5 seconds; at ~100M hash/s (scalar) roughly every 100 seconds.

## `ros-serialgen check`

```bash
ros-serialgen check --serial 00000000090681934458 -s 24 -u g -m cheerlon
```

| Flag | Long form | Meaning |
|---|---|---|
| `--serial <value>` | -- | The 20-character serial number to verify. **Required.** No short form (`-s` is reserved for disk size). |
| `-s <N>` | `--disk-size <N>` | Disk size magnitude, paired with `-u`/`--unit`. **Required.** |
| `-u <unit>` | `--unit <unit>` | Unit for `-s`: `g` (gigabytes, default), `m` (megabytes), `k` (kilobytes), or `b` (raw bytes). Same minimums as `search` above. |
| `-m <name>` | `--model <name>` | Disk model string. Defaults to `ROS<N><unit>` if omitted. |
| `-k <path>` | `--keys <path>` | Path to `keys.toml`. Defaults to `./keys.toml` if omitted. |

Prints the computed SOFTWARE ID, and if it matches a known signature, the License Key and MBR hex.

## `ros-serialgen sig2key <signature_hex>`

Positional argument: a 128-character hex string (64 bytes) -- the signature from the [Signature Table](collision-database.md#signature-table). Outputs the corresponding `-----BEGIN MIKROTIK SOFTWARE KEY-----...` block.

## `ros-serialgen key2sig <key_file>`

Positional argument: path to a `.key` file containing MikroTik key text. Outputs the 128-character signature hex.

## `ros-serialgen verify`

```bash
ros-serialgen verify
```

No arguments, no `keys.toml` needed -- this is a self-contained sanity check, unrelated to real signatures or collision search.

Runs the full SOFTWARE ID pipeline (custom SHA-256 -> MBR mix XOR -> Base-35 encode) against two fixed, hardcoded (serial, model, sector_val) test vectors, then checks that the result round-trips correctly through `encode -> decode -> re-encode`. Also prints which hash engine is active (`AVX-512 x16` or `scalar`), so you can confirm SIMD acceleration is being used on the current machine.

Run this once after building, or after moving to a different machine/CPU, before trusting `search` output.
