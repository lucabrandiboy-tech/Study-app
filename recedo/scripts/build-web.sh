#!/usr/bin/env bash
# Builds Recedo for the browser into target/web/ (index.html, the WebAssembly app, the
# install manifest, icons and the offline worker). Needs: rustup target wasm32-unknown-unknown,
# and wasm-bindgen-cli of the same version as the wasm-bindgen crate in Cargo.lock.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
cargo build --release --target wasm32-unknown-unknown -p recedo-web
out="$here/target/web"
rm -rf "$out" && mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" target/wasm32-unknown-unknown/release/recedo_web.wasm
cp web/manifest.webmanifest web/icon-192.png web/icon-512.png "$out/"
version="$(git describe --always --dirty --abbrev=10)-$(date +%s)"
sed "s/__VERSION__/recedo-$version/" web/sw.js > "$out/sw.js"
sed "s/__VERSION__/$version/g" web/index.html > "$out/index.html"
ls -l "$out"
