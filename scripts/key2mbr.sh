#!/usr/bin/env bash
# key2mbr.sh -- convert a MikroTik License Key text file to a full 80-byte MBR hex string.
#
# Usage: key2mbr.sh <key-file> [identity-hex]
#   key-file      Path to a .key file containing the MIKROTIK SOFTWARE KEY block.
#   identity-hex  Optional 20-hex-char MBR identity seed (0x100-0x109). Defaults to the
#                 standard all-zero identity used by this project's collision search.
#
# Pure bash + coreutils, no python/perl dependency. Implements MTBase64 decode
# (same LSB-first bit-order alphabet as src/convert.rs::mt_base64_decode) to recover the
# 64-byte signature, then prepends the identity + standard marker/reserved
# (00...BDE800000000) to form the full MBR[0x100:0x150] hex block.
#
# NOTE: BDE800000000 (marker+reserved) is only correct for the standard all-zero identity
# or for a real device whose marker happens to reduce to BDE8 -- see
# docs/license-internals.md §3.6 for why this isn't a universal constant.

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <key-file> [identity-hex]" >&2
    exit 1
fi

key_file="$1"
identity_hex="${2:-00000000000000000000}"

if [[ ! -f "$key_file" ]]; then
    echo "Error: file not found: $key_file" >&2
    exit 1
fi

if [[ ! "$identity_hex" =~ ^[0-9A-Fa-f]{20}$ ]]; then
    echo "Error: identity-hex must be exactly 20 hex characters" >&2
    exit 1
fi

alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

# Look up a character's index in the MTBase64 alphabet.
# Portable to bash 3.2 (macOS default) -- no associative arrays required.
b64_index() {
    local c="$1"
    if [[ "$c" == "/" ]]; then
        echo $(( ${#alphabet} - 1 ))  # '/' is the last alphabet char; skip the glob below
        return
    fi
    local stripped="${alphabet%%"$c"*}"
    if [[ "$stripped" == "$alphabet" ]]; then
        echo -1
    else
        echo "${#stripped}"
    fi
}

# Extract the base64 payload between BEGIN/END markers, stripping whitespace and '=' padding
b64=""
in_key=0
while IFS= read -r line; do
    trimmed="$(echo -n "$line" | tr -d '[:space:]')"
    if [[ "$trimmed" == *"BEGINMIKROTIK"* ]]; then
        in_key=1
        continue
    fi
    if [[ "$trimmed" == *"ENDMIKROTIK"* ]]; then
        break
    fi
    if [[ $in_key -eq 1 ]]; then
        b64+="$trimmed"
    fi
done < "$key_file"

b64="${b64//=/}"

if [[ -z "$b64" ]]; then
    echo "Error: no key data found between BEGIN/END markers" >&2
    exit 1
fi

# MTBase64 decode (LSB-first bit order, mirrors convert.rs::mt_base64_decode exactly)
pending_bits=0
prev_pos=0
hex_sig=""
len=${#b64}

for ((i = 0; i < len; i++)); do
    c="${b64:$i:1}"
    pos="$(b64_index "$c")"
    if [[ "$pos" -lt 0 ]]; then
        echo "Error: invalid base64 char '$c' at position $i" >&2
        exit 1
    fi
    if [[ $pending_bits -eq 0 ]]; then
        pending_bits=6
    else
        value1=$(( prev_pos >> (6 - pending_bits) ))
        mask=$(( (1 << (8 - pending_bits)) - 1 ))
        value2=$(( pos & mask ))
        value=$(( (value1 | (value2 << pending_bits)) & 0xFF ))
        hex_sig+=$(printf '%02X' "$value")
        pending_bits=$(( pending_bits - 2 ))
    fi
    prev_pos=$pos
done

sig_len=$(( ${#hex_sig} / 2 ))
if [[ $sig_len -ne 64 ]]; then
    echo "Error: decoded signature is $sig_len bytes, expected 64" >&2
    exit 1
fi

mbr_hex="$(echo -n "$identity_hex" | tr '[:lower:]' '[:upper:]')BDE800000000${hex_sig}"

echo "Signature (64 bytes): $hex_sig"
echo "MBR (80 bytes):       $mbr_hex"
