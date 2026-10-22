#!/usr/bin/env bash
# Checks Recedo's .kra files against the real Krita (tested with 5.2.2):
#  1. Recedo writes a painting, Krita flattens it to PNG: it must match Recedo's flattening.
#  2. Krita re-saves Recedo's file as .kra: Recedo must read back the same layers
#     (names, visibility, opacity, lock) and the same pixels.
#  4. Layer groups (faded, with a multiply layer inside, plain): Krita builds the .kra,
#     Recedo opens it and must flatten it as Krita does.
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
echo "4. Layer groups: Krita writes a .kra with groups; Recedo opens it"
python3 -I scripts/make-group-ora.py "$OUT/groups.ora"
krita_export "$OUT/groups.ora" "$OUT/groups.kra"
krita_export "$OUT/groups.kra" "$OUT/groups-krita.png"
"$TOOL" flatten "$OUT/groups.kra" "$OUT/groups-recedo.png"
"$TOOL" diff "$OUT/groups-recedo.png" "$OUT/groups-krita.png" 1
echo "   the same groups isolated (Krita's default for new groups)"
python3 -I scripts/kra-set-isolated.py "$OUT/groups.kra" "$OUT/groups-isolated.kra"
krita_export "$OUT/groups-isolated.kra" "$OUT/groups-isolated-krita.png"
"$TOOL" flatten "$OUT/groups-isolated.kra" "$OUT/groups-isolated-recedo.png"
"$TOOL" diff "$OUT/groups-isolated-recedo.png" "$OUT/groups-isolated-krita.png" 1
echo "Krita interop OK ($(krita --version 2>/dev/null | tail -n 1))"
