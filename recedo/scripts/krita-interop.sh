#!/usr/bin/env bash
# Checks Recedo's .kra files against the real Krita (tested with 5.2.2):
#  1. Recedo writes a painting, Krita flattens it to PNG: it must match Recedo's flattening.
#  2. Krita re-saves Recedo's file as .kra: Recedo must read back the same layers
#     (names, visibility, opacity, lock) and the same pixels.
#  3. Every blend mode: Krita's flattening of a two-layer file using it must match
#     Recedo's within 1/255 per channel (Krita 5.2.2: identical).
# Needs `krita` on PATH (Linux: apt install krita). Uses Xvfb if there is no display.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=$(realpath -m "${1:-target/krita-interop}")
mkdir -p "$OUT"
cargo build -q --release -p recedo-core --example kra_tool
TOOL=$PWD/target/release/examples/kra_tool
export QT_QPA_PLATFORM=offscreen
if [ -z "${DISPLAY:-}" ]; then
  Xvfb :97 -screen 0 1280x1024x24 >/dev/null 2>&1 &
  XPID=$!
  trap 'kill $XPID' EXIT
  export DISPLAY=:97
  sleep 2
fi
krita_export() { # in out
  rm -f "$2"
  timeout 300 krita "$1" --export --export-filename "$2" >"$OUT/krita.log" 2>&1 || { cat "$OUT/krita.log"; exit 1; }
  [ -s "$2" ] || { echo "Krita did not write $2"; cat "$OUT/krita.log"; exit 1; }
}
echo "1. Recedo writes recedo.kra; Krita flattens it"
"$TOOL" sample "$OUT/recedo.kra"
"$TOOL" flatten "$OUT/recedo.kra" "$OUT/recedo.png"
krita_export "$OUT/recedo.kra" "$OUT/krita.png"
"$TOOL" diff "$OUT/recedo.png" "$OUT/krita.png" 1
echo "2. Krita re-saves recedo.kra; Recedo reads it back"
krita_export "$OUT/recedo.kra" "$OUT/krita-resaved.kra"
"$TOOL" same "$OUT/recedo.kra" "$OUT/krita-resaved.kra"
echo "3. Blend modes: Recedo writes one file per mode; Krita flattens each"
for mode in $("$TOOL" modes); do
  "$TOOL" blend-sample "$mode" "$OUT/blend-$mode.kra"
  "$TOOL" flatten "$OUT/blend-$mode.kra" "$OUT/blend-$mode-recedo.png" 2>/dev/null
  krita_export "$OUT/blend-$mode.kra" "$OUT/blend-$mode-krita.png"
  printf '%-14s' "$mode"
  "$TOOL" diff "$OUT/blend-$mode-recedo.png" "$OUT/blend-$mode-krita.png" 1
done
echo "Krita interop OK ($(krita --version 2>/dev/null | tail -n 1))"
