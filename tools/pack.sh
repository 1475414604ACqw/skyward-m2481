#!/bin/bash
# Pack the recovery image: the TWRP-16 ramdisk as the base, with Skyward, its
# init program, the phone's configuration and the extra files laid over it.
#
#   STOCK=/path/to/TWRP-16.img tools/pack.sh devices/meizu21pro.toml out/skyward.img
set -e
SRC="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG="${1:?usage: pack.sh <devices/name.toml> <out.img>}"
OUT="${2:?usage: pack.sh <devices/name.toml> <out.img>}"
STOCK="${STOCK:?set STOCK to the TWRP-16 recovery image of the phone}"
EXTRA="${EXTRA:-$SRC/work/stage2}"
PK="$SRC/target/release/skyward-pack"
REL="$SRC/target/aarch64-linux-android/release"

"$PK" recovery \
    --stock "$STOCK" \
    --binary "$REL/skyward" \
    --system-init "$REL/system-init" \
    --config "$SRC/$CONFIG" \
    --font "$SRC/resources/fonts/MiSansVF.ttf" \
    --mono "$SRC/resources/fonts/KirkMono-Regular.ttf" \
    --mono-license "$SRC/resources/fonts/KirkMono-OFL.txt" \
    --extra "$EXTRA" \
    --allow-custom \
    --out "$OUT"
