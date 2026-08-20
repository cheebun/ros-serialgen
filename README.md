# ros-serialgen — RouterOS L6 Serial Generator & License Key Converter

A CLI tool that computes a valid RouterOS serial number from an existing L6 license. For any disk size, it searches for a collision serial whose SOFTWARE ID matches a known L6 signature, so that the existing license activates without mass production. A custom disk model string can also be specified.

## Features

- **AVX-512 SIMD acceleration**: computes 16 SHA-256 hashes per batch (auto-detected at runtime, falls back to scalar when unsupported)
- **Zero runtime dependencies**: SHA-256 and MTBase64 are hand-implemented; only depends on the clap CLI framework
- **External key configuration**: add new signatures via `keys.toml` without recompiling
- **Resume search**: `--from` parameter resumes from a saved progress point

## Build

```bash
# Recommended: enable native CPU instructions (AVX-512, etc.)
RUSTFLAGS='-C target-cpu=native' cargo build --release

# Generic build
cargo build --release
```

## Usage

### Search for collisions

```bash
# Find 1 collision (default)
ros-serialgen search -s 100 -t 16

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

On a match, prints the SOFTWARE ID, License Key, and MBR HEX.

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
    ├── main.rs              CLI entry + multi-threaded search engine + 31 unit tests
    ├── sha256_constants.rs  MikroTik SHA-256 shared constants (IV + K)
    ├── sha256.rs            MikroTik custom SHA-256 (scalar, production)
    ├── sha256_simd.rs       AVX-512 SIMD 16-way parallel SHA-256
    ├── sha256_scalar.rs     Scalar SHA-256 backup (for test cross-validation)
    ├── software_id.rs       Base-35 encode/decode + sector_val rounding
    ├── targets.rs           Load collision targets from keys.toml
    └── convert.rs           signature_hex ↔ Key text conversion (MTBase64)
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
cargo test          # 31 unit tests
cargo clippy        # zero warnings
cargo fmt --check   # format check
```

## Dependencies

- `clap` 4.x — CLI framework (derive mode)
- Zero runtime dependencies (SHA-256 and MTBase64 are hand-implemented)

## Documentation

- [Quick Start](docs/quick-start.md) — deploy a licensed VM in 10 minutes
- [Deployment Guide](docs/deployment-guide.md) — complete PVE setup reference
- [Command Reference](docs/command-reference.md) — every command and flag explained
- [Collision Database](docs/collision-database.md) — verified serial/model combinations
- [Architecture](docs/architecture.md) — algorithm and security analysis
- [License Internals](docs/license-internals.md) — SOFTWARE ID / MBR deep dive
- [Experiments](docs/experiments.md) — verification experiment log
- [Toolchain](docs/toolchain.md) — tools and reverse-engineering notes
