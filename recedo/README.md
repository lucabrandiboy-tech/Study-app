# Recedo

A Krita-style painting app for Windows, built for an XP-Pen Artist 12 pen display.
The spec is in the conversation that started it; decisions are in
[`DECISIONS.md`](DECISIONS.md), the status of every feature in [`FEATURES.md`](FEATURES.md).

**Current state: Phase 1 finished, waiting for the pen check.** `recedo.exe` is the painting
app (round brush with pressure, stabilizer, eraser, eyedropper, layers, undo, `.kra`, PNG/JPEG,
autosave, docking panels); the user's guide and checklist is
[`PHASE1_CHECKLIST.md`](PHASE1_CHECKLIST.md). The Phase 0 pen test app
(`recedo-tablet-test.exe`, [`PHASE0_CHECKLIST.md`](PHASE0_CHECKLIST.md)) is still there for
measuring the tablet itself.

## Layout

| Path | What |
|---|---|
| `crates/recedo-app` | Recedo itself (`recedo.exe`). Portable and tested on Linux: `editor.rs` (tools, pen handling, view, layers, undo, files, autosave), `ui.rs` (menus, panels, dialogs, shortcuts), `dock.rs` (docking shell), `widgets.rs`, `session.rs` (editor + interface + GPU renderer), `settings.rs`, `bench.rs` (`--benchmark`). Windows only: `win/` (window, message loop, pen/mouse/keyboard routing, file dialogs, self-test). |
| `crates/recedo-win` | Win32 helpers shared by both programs: window messages to egui input, clipboard, folders, monitors. |
| `crates/recedo-tablet` | Pen input: sample model, Wintab (`wintab.rs`, `wintab_abi.rs`), Windows Ink (`ink.rs`), tilt math, screen-mapping calibration, statistics, CSV recordings. Everything except the two Win32 modules is portable and unit-tested on Linux. |
| `crates/recedo-tabletest` | The Phase 0 test app: Win32 window and message loop (`src/win/`), wgpu canvas (`gpu.rs`, `shaders.wgsl`), egui panel (`ui.rs`), brush and strokes, report, self-test (`src/win/selftest.rs`). |
| `crates/recedo-core` | The painting engine, portable and unit-tested: tiles (`tiles.rs`), documents and Krita's compositing (`doc.rs`), brush and dabs (`brush.rs`, `dab.rs`, `curve.rs`), stabilizer, undo (`history.rs`), layer operations and eyedropper (`ops.rs`), view math (`view.rs`), projection, `.kra` files (`kra.rs`, `lzf.rs`), PNG/JPG (`image.rs`), autosave. `examples/kra_tool.rs`: command-line checks and benchmarks. |
| `crates/recedo-render` | GPU display of the canvas (wgpu): tile uploads, view transform, mipmaps; graphics adapter selection (`gpu.rs`). |
| `crates/wintab-mock` | Stand-in `Wintab32.dll` for tests. Never shipped. |
| `scripts/wine-selftest.sh` | Runs a Windows build's self-test (`recedo` or `tablet-test`) under Wine on a virtual screen. |
| `scripts/package.sh` | Builds the release and the zip handed to the user (`recedo` or `tablet-test`). |
| `scripts/krita-interop.sh` | Checks `.kra` files against the real Krita (needs `krita` installed): Recedo writes, Krita reads; Krita re-saves, Recedo reads. |
| `../.github/workflows/recedo.yml` | CI: lint, tests and cross-compile on Linux; both self-tests and the benchmark on real Windows; `.kra` against the real Krita. |

## Build and test (Linux, cross-compiling to Windows)

Needs: Rust (the version in `rust-toolchain.toml` installs itself), `gcc-mingw-w64-x86-64`,
and for the GPU tests a software Vulkan driver (`mesa-vulkan-drivers`). For the Wine
self-test: `wine64`, `xvfb`, `x11-utils`.

```sh
cargo clippy --workspace --all-targets -- -D warnings                       # Linux lints
cargo clippy --workspace --target x86_64-pc-windows-gnu -- -D warnings      # Windows lints
RECEDO_REQUIRE_GPU=1 cargo test --workspace                                 # unit + GPU tests
cargo build --release --target x86_64-pc-windows-gnu -p recedo-app -p recedo-tabletest -p wintab-mock
scripts/wine-selftest.sh recedo                                              # Recedo end-to-end under Wine
scripts/wine-selftest.sh tablet-test                                         # test app end-to-end under Wine
scripts/package.sh recedo                                                    # dist/Recedo-Phase1.zip
scripts/package.sh tablet-test                                               # dist/Recedo-TabletTest-Phase0.zip
scripts/krita-interop.sh                                                     # .kra against real Krita
cargo run --release -p recedo-app -- --benchmark /tmp                        # app benchmark (any OS)
cargo run --release -p recedo-core --example kra_tool -- bench /tmp 14       # engine benchmark, 15 4K layers
```

`RECEDO_REQUIRE_GPU=1` turns "no GPU available" in the rendering tests from a skip into a
failure; CI and the scripts always set it.

## Recedo command line

```
recedo.exe [file]                         open a painting (.kra, .png, .jpg) or start with a new one
recedo.exe --benchmark [folder]           time painting, undo and files on a 15-layer 4K painting;
                                          without a folder it asks first and opens the report
recedo.exe --selftest <folder>            automated end-to-end test (SendInput mouse and keys,
                                          injected Windows Ink pen, mock Wintab if
                                          RECEDO_WINTAB_DLL points at wintab_mock.dll)
```

Files Recedo writes: settings in `%APPDATA%\Recedo\recedo.json`; autosaved copies of unsaved
paintings in `%LOCALAPPDATA%\Recedo\recovery`; crash reports and benchmark reports in
`Documents\Recedo`.

## Test app command line

```
recedo-tablet-test.exe                              the test window
recedo-tablet-test.exe --selftest <dir>             automated end-to-end test (injected mouse,
                                                    Windows Ink pen, and mock Wintab if
                                                    RECEDO_WINTAB_DLL points at wintab_mock.dll)
recedo-tablet-test.exe --replay <in.csv> <out.png>  draw a saved pen recording (any OS)
```

Environment variables: `RECEDO_GPU_BACKEND=dx12|vulkan|gl` forces a GPU backend,
`RECEDO_WINTAB_DLL=<path>` loads a different Wintab DLL.

Files the test app writes: settings in `%APPDATA%\Recedo\tablet-test.json`; recordings,
reports and saved drawings (`.kra` + `.png`, only when asked) in `Documents\Recedo`.
