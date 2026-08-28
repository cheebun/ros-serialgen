# ros-serialgen — RouterOS Serial Generator & License Key Converter

A CLI tool that computes a valid RouterOS serial number from an existing license. For any disk size, it searches for a collision serial whose SOFTWARE ID matches a known signature, so that the existing license activates without mass production. The technique and tool are not L6-specific -- any RouterOS license level (L1-L6) works the same way, since the SOFTWARE ID computation and collision-search process are identical regardless of `nlevel`. A custom disk model string can also be specified.

## Features

- **AVX-512 SIMD acceleration**: computes 16 SHA-256 hashes per batch (auto-detected at runtime, falls back to scalar when unsupported)
- **Hand-implemented MikroTik crypto primitives**: SHA-256 and MTBase64 have no MikroTik-compatible library equivalent, so both are hand-implemented; standard Curve25519 EC-KCDSA verification uses the audited `curve25519-dalek` crate instead of hand-rolled field/point arithmetic (see `docs/investigation/license-internals.md` §8.32 for why)
- **External key configuration**: add new signatures via `keys.toml` without recompiling
- **Resume search**: `--from` parameter resumes from a saved progress point
- **Shell completion**: `completions` subcommand generates bash/zsh/fish/powershell/elvish scripts

## Build

```bash
# Recommended: enable native CPU instructions (AVX-512, etc.)
RUSTFLAGS='-C target-cpu=native' cargo build --release

# Generic build
cargo build --release
```

## Usage

### Search for collisions

`-s` takes a magnitude, `-u` sets its unit (`g` gigabytes/default, `m` megabytes, `k` kilobytes, `b` bytes). Minimum size is 64 MB regardless of unit (`-s 1 -u g`, `-s 64 -u m`, `-s 65536 -u k`, `-s 67108864 -u b`).

```bash
# Find 1 collision (default), 100 GB
ros-serialgen search -s 100 -t 16

# Sub-1GB sizes: 128 / 256 / 512 MB
ros-serialgen search -s 128 -u m -t 16
ros-serialgen search -s 256 -u m -t 16
ros-serialgen search -s 512 -u m -t 16

# Find 4 collisions (one per SOFTWARE ID)
ros-serialgen search -s 6 -t 16 -c 4

# Unlimited collection (Ctrl+C to exit)
ros-serialgen search -s 6 -t 16 -c 0

# Custom Model
ros-serialgen search -s 200 -t 16 -m MyDisk

# Resume from the 50000M progress point
ros-serialgen search -s 42 -t 8 -f 50000

# Specify keys.toml
ros-serialgen search -s 100 -t 16 -k /path/to/keys.toml
```

Output:
```
FOUND [1] serial=00000000418756277141 target=TEST-0001 verified=TEST-0001
```

### Verify a known serial

```bash
ros-serialgen check --serial 00000000090681934458 -s 24 -m cheerlon
```

`check` accepts the same `-s`/`-u` disk size flags as `search`.

On a match, prints the SOFTWARE ID, License Key, and MBR HEX.

Pass `-l/--license` with a `.key` file (or a raw 128-char signature_hex file) to compare its
embedded SOFTWARE ID against the one computed from serial/model/disk-size/identity/bus:

```bash
ros-serialgen check --serial 00000000090681934458 -s 24 -m cheerlon -l license.key
```

### Conversion

```bash
# signature hex → Key text
ros-serialgen sig2key <128-char-hex>

# Key text → signature hex
ros-serialgen key2sig license.key
```

### Algorithm self-check

```bash
ros-serialgen verify
```

### Shell completion

Generates a completion script for the given shell and prints it to stdout.

```bash
# bash
ros-serialgen completions bash > /etc/bash_completion.d/ros-serialgen

# zsh
ros-serialgen completions zsh > "${fpath[1]}/_ros-serialgen"

# fish
ros-serialgen completions fish > ~/.config/fish/completions/ros-serialgen.fish

# powershell / elvish also supported
ros-serialgen completions powershell
ros-serialgen completions elvish
```

## Adding new keys

Edit `keys.toml` and append an entry; no recompilation needed:

```toml
[[key]]
software_id = "XXXX-XXXX"
signature_hex = "..."
```

More keys = faster search (linear speedup).

## Project structure

```
├── Cargo.toml
├── keys.toml                External key configuration
├── README.md
├── CLAUDE.md / AGENTS.md    AI tool instructions
└── src/
    ├── main.rs              CLI entry + multi-threaded search engine + unit tests
    ├── sha256_constants.rs  MikroTik SHA-256 shared constants (IV + K)
    ├── sha256.rs            MikroTik custom SHA-256 (scalar, production)
    ├── sha256_simd.rs       AVX-512 SIMD 16-way parallel SHA-256
    ├── sha256_scalar.rs     Scalar SHA-256 backup (for test cross-validation)
    ├── software_id.rs       Base-35 encode/decode + sector_val rounding
    ├── targets.rs           Load collision targets from keys.toml
    ├── convert.rs           signature_hex ↔ Key text conversion (MTBase64) + metadata decode
    └── curve25519.rs        EC-KCDSA local license verification (curve25519-dalek-based)
```

## Performance

| Environment | Engine | Speed | Search time (4 targets) |
|---|---|---|---|
| 8-core AVX-512 | SIMD x16 | ~2000M hash/s | ~1-2 hours |
| 8-core, no AVX-512 | Scalar | ~100M hash/s | ~20+ hours |

SIMD optimization highlights:
- `_mm512_i32gather_epi32` replaces scalar gathering
- Circular buffer W[16] fuses message schedule with compression (4KB→1KB stack)
- `_mm512_ternarylogic_epi32` single-instruction Ch/Maj
- BCD incremental counter avoids heap allocation
- sid_hi lookup pre-filter skips 99.99% of non-matching batches

## Testing

```bash
cargo test          # 66+ unit tests (count grows with new features -- see `cargo test` output for the exact number)
cargo clippy        # a handful of pre-existing lints (too-many-arguments on CLI-plumbing functions, etc.), no new categories from recent changes
cargo fmt --check   # format check
```

## Dependencies

- `clap` 4.x — CLI framework (derive mode)
- `clap_complete` 4.x — shell completion script generation (`completions` subcommand)
- `curve25519-dalek` 4.x — audited Curve25519 field/point arithmetic, used only for EC-KCDSA
  local license verification (`LICENSE-VALID` output field); see `docs/investigation/license-internals.md`
  §8.32 for why this isn't hand-implemented like SHA-256/MTBase64
- SHA-256 and MTBase64 are hand-implemented (MikroTik-proprietary variants with no library
  equivalent to depend on)

## Documentation

See [docs/README.md](docs/README.md) for the full documentation index. Highlights:

- [Quick Start](docs/quick-start.md) — deploy a licensed VM in 10 minutes
- [Deployment Guide](docs/guides/x86-install.md) — complete PVE setup reference
- [Command Reference](docs/reference/command-reference.md) — every ros-serialgen subcommand and flag explained
- [Collision Database](docs/database/collision-database.md) — verified serial/model combinations
- [Architecture](docs/reference/architecture.md) — algorithm and security analysis
- [License Internals](docs/investigation/license-internals.md) — SOFTWARE ID / MBR deep dive
- [Experiments](docs/investigation/experiments.md) — verification experiment log
- [Toolchain](docs/reference/toolchain.md) — tools and reverse-engineering notes
