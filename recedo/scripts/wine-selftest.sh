#!/usr/bin/env bash
# Runs the Windows build's self-test under Wine on a virtual X display (no tablet needed).
# Covers the mouse path and the Wintab path (through the mock driver). Windows Ink cannot
# be injected under Wine; that part is covered by the Windows CI job.
#
#   scripts/wine-selftest.sh [output dir]
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-$here/target/wine-selftest}"
rel="$here/target/x86_64-pc-windows-gnu/release"
export WINEDEBUG=-all WINEPREFIX="${WINEPREFIX:-$HOME/.wine-recedo}"
export DISPLAY="${DISPLAY:-:99}"
if ! xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
  Xvfb "$DISPLAY" -screen 0 1920x1080x24 >/dev/null 2>&1 &
  sleep 1
fi
[ -d "$WINEPREFIX" ] || wineboot -i >/dev/null 2>&1
rm -rf "$out" && mkdir -p "$out"
cp "$rel/recedo-tablet-test.exe" "$rel/wintab_mock.dll" "$out/"
echo "testing $(sha256sum "$rel/recedo-tablet-test.exe")"
cd "$out"
# Wine's own GL driver path (llvmpipe) is the reliable GPU backend here.
set +e
RECEDO_GPU_BACKEND=gl RECEDO_WINTAB_DLL="Z:$out/wintab_mock.dll" \
  timeout 180 wine recedo-tablet-test.exe --selftest "Z:$out/results" > "$out/console.log" 2>&1
code=$?
set -e
cat "$out/results/selftest-results.txt" 2>/dev/null || { echo "no results file; console output:"; cat "$out/console.log"; }
echo "exit code $code"
exit $code
