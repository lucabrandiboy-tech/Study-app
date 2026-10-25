#!/usr/bin/env bash
# Runs a Windows build's self-test under Wine on a virtual X display (no tablet needed).
# Covers the mouse/keyboard path and the Wintab path (through the mock driver). Windows Ink
# cannot be injected under Wine; that part is covered by the Windows CI job.
#
#   scripts/wine-selftest.sh [recedo | tablet-test] [output dir]
#
# Build first: cargo build --release --target x86_64-pc-windows-gnu -p recedo-app -p recedo-tabletest -p wintab-mock
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
app="${1:-tablet-test}"
case "$app" in
  recedo) exe=recedo.exe ;;
  tablet-test) exe=recedo-tablet-test.exe ;;
  *) echo "unknown app: $app (recedo or tablet-test)" >&2; exit 2 ;;
esac
out="${2:-$here/target/wine-selftest-$app}"
rel="$here/target/x86_64-pc-windows-gnu/release"
export WINEDEBUG=-all WINEPREFIX="${WINEPREFIX:-$HOME/.wine-recedo}"
# In the C locale Wine cannot create files with non-English letters in their names.
export LC_ALL=C.UTF-8
export DISPLAY="${DISPLAY:-:99}"
if ! xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
  Xvfb "$DISPLAY" -screen 0 1920x1080x24 >/dev/null 2>&1 &
  sleep 1
fi
[ -d "$WINEPREFIX" ] || wineboot -i >/dev/null 2>&1
rm -rf "$out" && mkdir -p "$out"
cp "$rel/$exe" "$rel/wintab_mock.dll" "$out/"
echo "testing $(sha256sum "$rel/$exe")"
cd "$out"
# Wine's own GL driver path (llvmpipe) is the reliable GPU backend here.
set +e
RECEDO_GPU_BACKEND=gl RECEDO_WINTAB_DLL="Z:$out/wintab_mock.dll" \
  timeout 300 wine "$exe" --selftest "Z:$out/results" > "$out/console.log" 2>&1
code=$?
set -e
cat "$out/results/selftest-results.txt" 2>/dev/null || { echo "no results file; console output:"; cat "$out/console.log"; }
echo "exit code $code"
exit $code
