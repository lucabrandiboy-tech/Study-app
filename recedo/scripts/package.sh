#!/usr/bin/env bash
# Builds a release .exe and packs the zip handed to the user:
#
#   scripts/package.sh recedo        dist/Recedo-Phase1.zip
#     recedo.exe
#     READ ME FIRST.txt        (PHASE1_CHECKLIST.md: setup and what to try)
#     LICENCES.txt
#
#   scripts/package.sh tablet-test   dist/Recedo-TabletTest-Phase0.zip
#     recedo-tablet-test.exe
#     READ ME FIRST.txt        (PHASE0_CHECKLIST.md)
#     LICENCES.txt
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
app="${1:-recedo}"
case "$app" in
  recedo) exe_name=recedo.exe; guide=PHASE1_CHECKLIST.md; zip_name=Recedo-Phase1.zip ;;
  tablet-test) exe_name=recedo-tablet-test.exe; guide=PHASE0_CHECKLIST.md; zip_name=Recedo-TabletTest-Phase0.zip ;;
  *) echo "unknown app: $app (recedo or tablet-test)" >&2; exit 2 ;;
esac
# Same command as CI and the Wine self-test instructions use, so the zipped .exe is byte for
# byte the one the self-tests ran (building one package alone unifies dependency features
# differently and gives a slightly different binary).
cargo build --release --target x86_64-pc-windows-gnu -p recedo-app -p recedo-tabletest -p wintab-mock
exe="target/x86_64-pc-windows-gnu/release/$exe_name"
# The .exe must need nothing beyond Windows' own DLLs.
allowed='^(kernel32|user32|gdi32|advapi32|shell32|comdlg32|ole32|oleaut32|combase|rpcrt4|ntdll|msvcrt|userenv|ws2_32|bcryptprimitives|dxgi|opengl32|setupapi|api-ms-win-[a-z0-9-]+)\.dll$'
bad=$(x86_64-w64-mingw32-objdump -p "$exe" | awk '/DLL Name:/ {print tolower($3)}' | sort -u | grep -Ev "$allowed" || true)
if [ -n "$bad" ]; then
  echo "unexpected DLL dependencies: $bad" >&2
  exit 1
fi
stage="$here/target/package-$app"
rm -rf "$stage" && mkdir -p "$stage" "$here/dist"
cp "$exe" "$stage/"
# Windows Notepad copes with LF, but CRLF is friendlier to older editors.
sed 's/$/\r/' "$guide" > "$stage/READ ME FIRST.txt"
sed 's/$/\r/' THIRD_PARTY.md > "$stage/LICENCES.txt"
zip_path="$here/dist/$zip_name"
rm -f "$zip_path"
(cd "$stage" && python3 - "$zip_path" <<'EOF'
import sys, zipfile, os
with zipfile.ZipFile(sys.argv[1], "w", zipfile.ZIP_DEFLATED) as z:
    for name in sorted(os.listdir(".")):
        z.write(name, name)
EOF
)
ls -l "$zip_path"
python3 -c "import zipfile,sys; [print(f'  {i.file_size:>10}  {i.filename}') for i in zipfile.ZipFile(sys.argv[1]).infolist()]" "$zip_path"
sha256sum "$exe"
