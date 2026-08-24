# AGENTS.md

Machine-executable rules for all AI tools working on this Rust project.

## Project

`ros-serialgen` — RouterOS serial generator + key conversion CLI tool. Computes serials from an existing license via SOFTWARE ID collision search; custom model strings supported.

## Architecture

```
src/
├── main.rs              CLI entry (clap subcommands) + multi-threaded search logic + 43 tests
├── sha256_constants.rs  Shared constants (ROUND_CONSTANTS + INITIAL_HASH_VALUES)
├── sha256.rs            MikroTik custom SHA-256 (scalar, production)
├── sha256_scalar.rs     Scalar SHA-256 backup (#[cfg(test)], for cross-validation)
├── sha256_simd.rs       AVX-512 SIMD 16-way parallel SHA-256
├── software_id.rs       Base-35 encode/decode + sector_val rounding
├── targets.rs           Load collision targets from keys.toml
└── convert.rs           signature_hex ↔ Key text conversion (MTBase64)

keys.toml                External key configuration (loaded at runtime, no recompile needed)
```

## Commands

```bash
# Search for collisions
ros-serialgen search -s <N> -u <g|m|k|b> -t <threads> [-c <count>] [-f <from_M>] [-m <model>] [-k <keys.toml>] [-i <identity_hex>]
  -s  Disk size magnitude, paired with -u
  -u  Unit: g (gigabytes, default), m (megabytes), k (kilobytes), b (bytes) -- min size is 64M in any unit
  -t  Thread count
  -c  Collision count (default 1, 0 = unlimited collection)
  -f  Resume from N million hashes (matches the M value in progress output)
  -m  Custom Model (default ROS<N><unit>, e.g. ROS100G, ROS128M)
  -k  Specify keys.toml path
  -i  Non-standard 20-hex-char MBR identity (0x100-0x109); default is the standard all-zero identity

# Verify a serial
ros-serialgen check --serial <20-digit> -s <N> -u <g|m|k|b> [-m <model>] [-k <keys.toml>] [-i <identity_hex>]

# Conversion
ros-serialgen sig2key <128-char-hex>     # signature → Key text
ros-serialgen key2sig <file.key>         # Key text → signature

# Algorithm self-check
ros-serialgen verify
```

## Build

```bash
RUSTFLAGS='-C target-cpu=native' cargo build --release   # AVX-512 optimal
cargo test          # 58 unit tests
cargo clippy        # zero warnings
cargo fmt --check   # format check
```

## Code Rules

- Single source of constants: `sha256_constants.rs`, shared by all three SHA-256 implementations
- Consistent naming: `sid_lo`/`sid_hi` (not hash_lo/d4), `max_collisions` (not target_count)
- All public functions must have `///` doc comments
- SHA-256 implementations must annotate the reason for byte-order conversions
- New collision targets go into keys.toml configuration, never hardcoded in source
- No built-in default targets -- `load_targets` exits with an error if keys.toml is missing or empty
- Search results must self-verify (recompute the full SOFTWARE ID and print it)
- `decode()` returns `Result`, errors on invalid characters
- Production code must not use `assert!` (use `eprintln!` + `process::exit` instead)
- `cargo clippy` zero warnings (except `dead_code`)
- `cargo fmt` unified formatting

## Key Constants

```
MBR mix (10-zeros): mbr_val = 0x0BD, mix = 0x0BD × 0x3FF800F
SHA-256 IV: [0x5B653932, 0x7B145F8F, 0x71FFB291, 0x38EF925F, ...]
Base-35 table: "TN0BYX18S5HZ4IA67DGF3LPCJQRUK9MW2VE"
```

## SIMD Optimizations

- `_mm512_i32gather_epi32` replaces 16 scalar gathers (W[0..4] loading)
- Circular buffer W[16] fuses message schedule with compression (4KB→1KB stack)
- `_mm512_ternarylogic_epi32` single-instruction Ch(0xCA)/Maj(0xE8)
- `_mm512_shuffle_epi8` SIMD byte-order conversion
- bswap mask hoisted to function top for reuse
- BCD incremental counter + W[5..9] precomputation
- sid_hi lookup pre-filter (256-byte lookup table)

## Testing

- `sha256::tests::test_6g_known_hash` — 6G VMware known hash value
- `sha256_simd::tests::test_simd_matches_scalar` — SIMD vs scalar cross-validation
- `sha256_simd::tests::test_simd_6g_known` — SIMD 6G known value
- `software_id::tests::test_encode_decode_roundtrip` — encode/decode roundtrip
- `software_id::tests::test_decode_invalid_char` — invalid character error
- `software_id::tests::test_round_sectors` — 5 rounding verification cases
- `convert::tests::test_roundtrip_synthetic` — sig ↔ key conversion verification
- `main::tests` — 43 tests covering disk size parsing/validation, write_serial, BCD, software_id, model, input_buf, check_match, E2E, identity parsing/mix resolution
- `targets::tests` — 3 tests covering `mix_from_identity` (matches standard for all-zero, deterministic, differs for non-zero)

## Dependencies

- `clap` 4.x — CLI framework (derive mode)
- Zero runtime dependencies (SHA-256 and MTBase64 are hand-implemented)
