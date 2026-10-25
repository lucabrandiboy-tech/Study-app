# Recedo: feature status

Every feature in the spec, with one status:

* **WORKING**: run and verified (how is stated).
* **PARTIAL**: some of it works; what is missing is stated.
* **BROKEN**: known to fail.
* **NEEDS HARDWARE CHECK**: done and tested without the real tablet; needs the user's pen.
* **NOT STARTED**: nothing yet. Never shown in the app as if it worked.

Current phase: **1 (the painting app), finished; waiting for the user's pen check.** The
Phase 0 pen checklist is also still open (the pen has not been tried on either program yet).

* "Test app" means the Phase 0 tablet test program (`recedo-tablet-test.exe`).
* "Self-test" means `recedo.exe --selftest`: it drives the real Recedo window with injected
  input (mouse, keyboard, a Windows Ink pen with pressure, the stand-in Wintab driver) and
  checks the painting, the screen and the files. On real Windows (CI): 31 passed, 0 failed,
  0 skipped. Under Wine: 26 passed, 3 skipped (no pen injection or screen read-back there, and
  Wine drops keys held with SendInput when the mouse is pressed).
* "UI tests" render the whole interface off-screen on a GPU and click through it.
* "Editor tests" drive the app's editor (everything between the input and the screen) with
  synthetic pen samples.

## Phase 1 deliverables

| Item | Status | Notes / how verified |
|---|---|---|
| GPU canvas (wgpu, DirectX 12 / Vulkan / OpenGL) | WORKING | Self-test on Windows reads the window's image back: painted spots dark, empty ones white. GPU tests compare drawn pixels with the view math. |
| Tiled storage (64x64 tiles, copy on write) | WORKING | Engine tests; 15 painted 4K layers use 469 MB. |
| Pan, zoom, rotate, mirror | WORKING | Space+drag, middle-button drag, wheel zoom about the pointer, "2" fit: self-test on Windows. Shift+Space rotate, Ctrl+Space zoom, 4/6/5, M, 1, +/-: editor and UI tests (same input path as the Space key, which the self-test runs). |
| Round brush with pressure (size, opacity, flow), hardness, spacing, Wash / Build-up | NEEDS HARDWARE CHECK | Injected Windows Ink stroke on Windows: line 3 px wide at light pressure, 19 px at full; mock Wintab: 2 to 21 px; every touched point painted. The real pen: checklist part 2. |
| Krita stabilizer (distance, delay, finish line, smoothed pressure, zoom-scaled) | NEEDS HARDWARE CHECK | Engine tests (15) and editor tests (keeps catching up while the pen is held still). How it feels needs the pen. |
| Eraser tool, eraser end of the pen | Tool WORKING; eraser end NEEDS HARDWARE CHECK | Editor tests with synthetic eraser samples. |
| Eyedropper (tool, Ctrl+click, all layers or one, averaging radius) | WORKING | Editor tests. |
| Layers: add, delete, duplicate, move, merge down, clear, rename, opacity, visibility, lock, 16 blend modes | WORKING | Editor tests; self-test on Windows: Insert adds a layer, paint goes only to the selected one. Locked/hidden layer: message instead of painting (editor tests). |
| Undo 50 steps, history panel (click to jump) | WORKING | Editor tests; Ctrl+Z / Ctrl+Shift+Z in the self-test on Windows. Slider drags are one step. |
| New canvas dialog (presets incl. 3840x2160 and 1280x720, custom size/resolution/background) | WORKING | UI tests. 8-bit only. |
| `.kra` open/save (basic level) | WORKING | Real Krita 5.2.2 both ways (CI); self-test on Windows saves a 3-layer 4K painting under a non-English file name and reads it back identical. |
| PNG / JPEG export | WORKING | Self-test on Windows: PNG equals the flattened painting. JPEG: editor tests (decoded again). |
| Autosave and crash recovery | WORKING | Self-test on Windows: a recovery copy is written, a normal close removes it. Under Wine: a copy left by a killed Recedo is offered at the next start ("Recover your work?"); opening it is covered by editor tests. A copy is also written at once when Windows shuts down. Not offered while another Recedo is running. |
| Docking shell: panels dock, tab, float, close, reopen, resize | PARTIAL | UI tests: floating, closing, reopening from the Window menu, layout kept in the settings. Dragging tabs between groups by hand has not been run in a test (it is in the checklist). |
| Settings (tablet interface, interface size, autosave interval), window position kept | WORKING | Settings tests (damaged file kept aside, values clamped); switching Windows Ink / Wintab in the self-test. |
| Benchmark | WORKING | `recedo.exe --benchmark`; numbers at the end of this file. |
| Interface at 125 % / 150 % display scaling | NEEDS HARDWARE CHECK | Per-monitor DPI aware; rendered at 1080p and 150 % in a UI test (panels scroll, sections fold). Only seen at 100 % on a screen. |

## Phase 0 deliverables

| Item | Status | Notes / how verified |
|---|---|---|
| Stack evaluation, `DECISIONS.md` | WORKING | Written; includes rejected options and scope warnings. |
| Test app: draws with pressure and tilt through **Wintab** | NEEDS HARDWARE CHECK | Self-test under Wine with a stand-in `Wintab32.dll`: every packet arrives, a deliberately dropped packet is detected, positions within 0.03 px, pressure within 0.0001, tilt within 0.1°. Real XP-Pen driver: user's checklist. |
| Test app: draws with pressure and tilt through **Windows Ink** | NEEDS HARDWARE CHECK | Self-test on real Windows (CI) with injected pen strokes: every frame arrived, pressure and tilt exact, positions within 0.024 px, line width follows pressure. Real pen: user's checklist. |
| Test app: raw event rate display | WORKING | Rates, peak, device interval rate, samples per message, losses; checked by unit tests and the self-test. The Artist 12's actual numbers come from the checklist. |
| Every tablet event used, including coalesced ones | NEEDS HARDWARE CHECK | Wintab: whole queue drained per message (self-test on Windows: 220/220 packets, up to 47 in one message). Ink: full pointer history read, repeats dropped (Windows: 121/121 frames, up to 3 in one message). |
| Sub-pixel positions | NEEDS HARDWARE CHECK | Wintab: from tablet units (self-test). Ink: HIMETRIC location, falls back to whole pixels if inconsistent (counted in the panel). |
| Wintab screen mapping on a multi-monitor PC | NEEDS HARDWARE CHECK | Automatic choice by comparison with the cursor; unit-tested for two monitors, flipped Y and sub-area mappings. |
| Display scaling 125% / 150%, crisp UI | NEEDS HARDWARE CHECK | Per-monitor DPI awareness v2; panel rendered at the monitor's real pixel density; layout unit-tested at 150%. Only seen at 100% so far. |
| Manual checklist for the user | WORKING | `PHASE0_CHECKLIST.md` (also in the download as `READ ME FIRST.txt`). |
| Portable .exe | NEEDS HARDWARE CHECK | Self-contained, needs only Windows' own DLLs. Runs under Wine and on Windows CI. |
| Test app extra: stabilizer preview | NEEDS HARDWARE CHECK | Krita's stabilizer (engine version) on the test brush, with Krita's delay leash. Self-test stroke through the live app (Wine and Windows CI); GPU test: a shaky line drawn ≥10 rows thick raw, ≤6 stabilized. |
| Test app extra: Save drawing (.kra for Krita + .png) | WORKING | Self-test saves the test strokes and reads the `.kra` back identical to the canvas (Wine, and real Windows on CI). Krita 5.2.2 exports that file with no pixel difference from the PNG. |

## Tablet and input

| Feature | Status | Notes |
|---|---|---|
| Pressure controls size and opacity, per brush | NEEDS HARDWARE CHECK | Recedo's brush and eraser: pressure→size (with a smallest size), →opacity, →flow switches in Tool options (linear curves); injected pen pressure on Windows changes the line width 3→19 px. Curve shapes: Phase 2. |
| Tilt used by brushes | NEEDS HARDWARE CHECK | "Tilt leans the brush" in Tool options (engine and editor tests with synthetic tilt). The Artist 12's pen may not report tilt. More tilt options: Phase 2. |
| Pressure curve editor, global + per brush | PARTIAL | Engine only: Krita-style curves (control points, smooth, never overshoot; `curve.rs`, tested). No editor window yet (Phase 2). |
| Switch between Wintab and Windows Ink | NEEDS HARDWARE CHECK | Recedo: Edit > Settings > Tablet, live, kept in the settings; falls back to Windows Ink with a message if Wintab cannot open. Both paths painted in the self-test (mock Wintab, injected Ink). While a pen is in range the mouse never paints (no duplicate strokes from the driver's mouse messages: checked). |
| Pen side buttons customisable | NOT STARTED | Phase 2. The test app shows what the buttons report. |
| Eraser end switches to the eraser | NEEDS HARDWARE CHECK | Recedo and the test app erase with the eraser end (editor and GPU tests with synthetic eraser samples). The Artist 12's pen may have no eraser end. |
| Every action bindable to any key combination (for express keys) | NOT STARTED | Phase 7. |

## Line smoothing

| Feature | Status | Notes |
|---|---|---|
| Krita-style stabilizer: distance, delay, finish line, smooth pressure | NEEDS HARDWARE CHECK | Re-implemented from Krita's algorithm (`stabilizer.rs`: per-millisecond resampling, moving average, delay dead zone, finish line, stabilized sensors, zoom-scaled distance; 15 tests). In Recedo's Tool options, with Krita's delay leash drawn on the canvas; the app keeps updating while the pen is held still (editor test). How it feels needs the pen. |
| Off setting with shortcut and tool-option toggle | PARTIAL | Tool options checkbox (and the section title says on/off). No shortcut yet (shortcut editor: Phase 7). |
| Basic and weighted modes | NOT STARTED | Phase 1 (extras). |

## Brushes (Phase 2 unless noted)

| Feature | Status |
|---|---|
| Built-in presets: pencil, inking pens, oil, acrylic, gouache, airbrush, soft round, watercolour, markers, erasers | NOT STARTED |
| Full brush editor (tip shape, size, opacity, flow, hardness, spacing, angle, roundness, rotation, scatter, texture, colour dynamics, blend mode, sensor curves for pressure/tilt/speed/random) | NOT STARTED |
| Colour smudge engine (smearing, dulling, smudge length, colour rate) | NOT STARTED |
| Built-in paper and canvas textures | NOT STARTED |
| Brush size up to 1000 px | NEEDS HARDWARE CHECK: Recedo's size slider goes to 1000 px ([ and ] step it). Benchmark on this 4-core container, 15-layer 4K painting: a fast 1000 px stroke costs 7-9 ms of painting + 5-7 ms of picture update per frame (median), within a 60 Hz frame in 56-87 % of frames. The real PC's numbers: `--benchmark` |
| Brush outline cursor at true size and shape, follows zoom and rotation | PARTIAL: circle at the true size, follows zoom; not the shape of tilted/leaned dabs yet |
| Each preset remembers its size, opacity and settings | NOT STARTED |
| Brush library with tags and favourites; save, rename, duplicate, delete | NOT STARTED |
| Radial pop-up palette | NOT STARTED |
| Import `.kpp`, bundles, `.abr` (with a report of what did not translate) | NOT STARTED (Phase 8) |

## Colour (Phase 3 unless noted)

| Feature | Status |
|---|---|
| Colour wheel with triangle/square, HSV and RGB sliders, hex input | PARTIAL: Recedo's Colour panel has a saturation/value square with a hue bar, RGB sliders and hex input (UI tests). No wheel or triangle yet |
| Krita-style Advanced Colour Selector | NOT STARTED |
| Eyedropper, current layer / all layers, Ctrl quick access | WORKING: tool (P) and Ctrl+click with the brush; visible picture or selected layer; averaging radius (editor tests) |
| Palettes: create, edit, save, `.kpl` and `.gpl` import/export | NOT STARTED |
| Recent colours | NOT STARTED |
| Harmony helpers (complementary, analogous, triadic, split-complementary, tetradic) | NOT STARTED |
| 8-bit default, 16-bit per document | PARTIAL: Krita's 16-bit RGB layers open (converted to 8-bit, with a warning; within 1/255 of Krita's own 16-bit export). Painting and saving in 16-bit: not started |
| Colour management: sRGB working space, ICC profiles, embedded on export, display profile | NOT STARTED |

## Fill and gradient (Phase 5)

| Feature | Status |
|---|---|
| Bucket fill: tolerance, grow/shrink, feathering, anti-aliasing | NOT STARTED |
| Gap closing | NOT STARTED |
| Fill using a reference layer | NOT STARTED |
| Gradient: linear, radial, FG→BG, FG→transparent, repeat options | NOT STARTED |

## Layers (basic: Phase 1, rest: Phase 4)

| Feature | Status |
|---|---|
| 15 layers at 4K comfortably | NEEDS HARDWARE CHECK: app benchmark below (15 fully painted 4K layers: 20 and 200 px strokes well within a 60 Hz frame on the CPU, 1000 px borderline on this 4-core container; undo ~0.1 s; save 1.4-1.8 s, open 0.7-2.3 s); the real PC decides |
| Paint, group, filter and text layers | NOT STARTED |
| Opacity, visibility, lock, alpha lock, rename, drag to reorder, duplicate, merge down, merge selected, flatten | PARTIAL: in Recedo's Layers panel and menu: opacity, visibility, lock, rename (double-click), add, delete, move up/down (buttons and Ctrl+Page Up/Down), duplicate, merge down, clear (editor and self-tests). No alpha lock, drag to reorder, merge selected or flatten command yet |
| Full Krita blend mode set | PARTIAL: 16 of Krita's modes, chosen in the Layers panel (Normal, Multiply, Screen, Overlay, Hard Light, Soft Light (Photoshop), Color Dodge, Color Burn, Linear Dodge, Linear Burn, Darken, Lighten, Difference, Exclusion, Subtract, Divide) with Krita's 8-bit arithmetic; read from and written to `.kra`. Real Krita 5.2.2 flattens a two-layer file for each mode with **0** difference from Recedo (`scripts/krita-interop.sh`, on CI). Krita has about 100 modes; the rest (HSL modes, etc.) are not started |
| Clipping masks, transparency masks, selection masks | NOT STARTED |
| Layer panel: thumbnails, colour labels, multi-select | NOT STARTED |

## Filters (Phase 4)

| Feature | Status |
|---|---|
| Gaussian blur, motion blur, sharpen/unsharp mask, levels, curves, HSV/HSL, colour balance, desaturate, add noise, pixelate | NOT STARTED |
| Live on-canvas preview | NOT STARTED |
| Each filter as destructive filter and as filter layer | NOT STARTED |

## Selections (Phase 5)

| Feature | Status |
|---|---|
| Rectangle, ellipse, freehand lasso, polygon lasso, magic wand, select by colour | NOT STARTED |
| Replace / add / subtract / intersect with Krita's modifier keys | NOT STARTED |
| Feather, grow, shrink, invert, select all, deselect, marching ants | NOT STARTED |

## Transform (Phase 5)

| Feature | Status |
|---|---|
| Move, scale, rotate, flip | NOT STARTED |
| Perspective and free distort | NOT STARTED |
| Warp and mesh | NOT STARTED |
| Liquify (push, grow, shrink, swirl, undo brush) | NOT STARTED |
| On layers, groups and selections | NOT STARTED |

## Shapes and drawing aids (Phase 6)

| Feature | Status |
|---|---|
| Line, rectangle, ellipse, polygon, polyline, Bézier tools with the current brush | NOT STARTED |
| Hold-to-snap shapes (QuickShape-style) | NOT STARTED |
| Assistants: perspective 1/2/3-point, vanishing point, ruler, parallel ruler, ellipse, concentric ellipse, snapping strength | NOT STARTED |
| Symmetry: horizontal, vertical, radial with N segments | NOT STARTED |
| Grid, guides from rulers, snapping | NOT STARTED |

## Text, comics, reference images (Phase 6)

| Feature | Status |
|---|---|
| Editable text layers (font, size, colour, alignment, line spacing), convert to paint layer | NOT STARTED |
| Comic panel/frame tool with gutters and border thickness | NOT STARTED |
| Reference board: from file or clipboard, move, scale, rotate, opacity, saved with the document | NOT STARTED |

## Canvas and navigation

| Feature | Status | Phase |
|---|---|---|
| Space+drag pan, wheel/middle-button zoom, remappable | WORKING (not remappable yet): Space+drag and middle-button drag pan, wheel zooms about the pointer, Ctrl+Space+drag zooms, +/- and 1/2 (self-test on Windows, editor tests) | 1 (remapping: 7) |
| Free rotation, reset rotation | WORKING: Shift+Space+drag, 4/6 by 15°, 5 resets; painting stays under the pen when rotated (editor tests) | 1 |
| Mirror view with M | WORKING (editor tests: painting stays under the pen when mirrored) | 1 |
| Navigator / minimap | NOT STARTED | 7 |
| Second view of the same canvas | NOT STARTED | 7 |
| Smooth zoom, pixel grid at high zoom | PARTIAL: mipmaps when zoomed out (fine detail averages instead of shimmering), sharp pixels from 200 % (GPU-tested). No pixel grid yet | 1 |
| New canvas dialog: presets (3840×2160, 1280×720, …), custom size/DPI, background, depth, custom presets | PARTIAL: File > New with the built-in presets (your two sizes first), custom size and resolution, background, memory estimate (UI tests). 8-bit only; saving your own presets is in the engine but not in the dialog yet | 1 |
| Canvas resize, image resize, crop | NOT STARTED | not assigned by the spec; proposed for 5 |

## Undo and history (Phase 1)

| Feature | Status |
|---|---|
| 50 undo steps (configurable), tiled so undo only stores changed tiles | PARTIAL: 50 steps, only changed tiles stored (1.1 ms per step for 15 4K layers); undoing a 1000 px stroke across a 15-layer 4K painting: 3-8 ms, then about 0.1 s to update the screen (benchmark). The number of steps is not a setting yet |
| History panel, click to jump | WORKING (UI and editor tests) |

## Files

| Feature | Status | Phase |
|---|---|---|
| Native format compatible with Krita `.kra` (opens in Krita; Krita files open in Recedo, with warnings for unsupported parts) | PARTIAL (basic level done): 8-bit RGBA paint layers with name, visibility, opacity, lock, position, blend mode (16-bit layers open as 8-bit). Checked against the real Krita 5.2.2 both ways (`scripts/krita-interop.sh`, also on CI): Krita's flattening of a Recedo file differs by 0; Krita re-saves it and Recedo reads the same layers and pixels; Krita-made files open with pixels identical to Krita's own. Groups (Recedo has none yet): pass-through groups open as their layers with the group's opacity applied to each, as Krita does; isolated groups open as their layers when that keeps the picture, otherwise merged into one layer. 42 group and blend-mode cases flattened by the real Krita are reproduced exactly (one, an isolated group of semi-transparent Normal layers opened as separate layers, differs by 1/255 on some pixels: 8-bit rounding; kept so the layers stay editable); vector/filter/fill/file/clone layers, masks, blend modes Recedo lacks and other colour depths are listed in plain words and left out (shown as Normal for unknown blend modes) | 1 basic, 4 full, 9 animation |
| Save (Ctrl+S), Save As | WORKING: Windows' own dialogs, saving in the background while you keep painting, safe save (`.part` file renamed into place); self-test on Windows | 1 |
| Autosave to a separate recovery file, crash recovery, interval setting | WORKING: every 1-60 min (Settings) to `%LOCALAPPDATA%\Recedo\recovery`, offered after a crash, removed on a normal close (self-test on Windows; offer at start-up seen under Wine) | 1 |
| Open PNG, JPG, `.kra` | PARTIAL: File > Open, Open recent, dropping a file on the window, or "Open with" Recedo; recognised by content (JPG grey/RGB/CMYK). JPG photo rotation (EXIF) is not applied yet | 1 |
| Open WebP; paste clipboard image as a new layer | NOT STARTED | 8 |
| Export PNG (transparency), JPG (quality) | WORKING: PNG with transparency (self-test on Windows: equals the painting); JPG with a quality setting, transparent areas filled with white (like Krita) | 1 |
| Export PDF | NOT STARTED | 8 |
| Timelapse recording to MP4 (bundled ffmpeg) | NOT STARTED | 8 |

## Animation (Phase 9)

| Feature | Status |
|---|---|
| Timeline: frames per layer, keyframes, hold frames, frame rate, playback range | NOT STARTED |
| Onion skinning (frames before/after, tint, opacity) | NOT STARTED |
| Looping playback | NOT STARTED |
| Opacity and transform tweening | NOT STARTED |
| Export GIF, MP4, image sequences | NOT STARTED |
| Saved in `.kra` so Krita can open it | NOT STARTED |

## Interface

| Feature | Status | Phase |
|---|---|---|
| Dockable panels (dock, float, tab, close, reopen), named workspaces | PARTIAL: Recedo's own dock (UI tests); dragging tabs by hand not yet run in a test; no named workspaces (Phase 7) | 1 shell, 7 |
| Dark theme | WORKING | Recedo and the test app are dark. |
| Light theme, accent colour | NOT STARTED | 7 |
| Adjustable UI scale | WORKING | Edit > Settings > Interface size (75-200 %), on top of Windows' display scaling. |
| Krita default shortcuts, remappable, conflict warnings, import/export | NOT STARTED | 7 |
| Tab: canvas-only focus mode | NOT STARTED | 7 |
| Tool options panel | WORKING | Brush/eraser size, opacity, flow, hardness, spacing, mode, pen and stabilizer options; eyedropper options. More with each phase. |
| Settings window (tablet API, pressure curve, pen buttons, autosave, undo steps, theme, UI scale, shortcuts, performance) | PARTIAL | Edit > Settings: tablet interface, interface size, autosave interval. The rest: Phase 7. |

## Network

| Feature | Status | Notes |
|---|---|---|
| Fully offline, no telemetry, no accounts | WORKING | There is no networking code; neither program sends anything anywhere. |
| Optional update check, off by default | NOT STARTED | Lowest priority. |

## Packaging (Phase 10)

| Feature | Status |
|---|---|
| Windows installer | NOT STARTED (a portable .exe in a zip until then) |
| `USER_GUIDE.md` | NOT STARTED |

## Benchmarks

`recedo.exe --benchmark` (or `cargo run --release -p recedo-app -- --benchmark <dir>` on any
system) times the spec's targets through the app's own code: a 3840×2160 painting with 15
layers painted everywhere (the worst case), fast strokes (pen at 200 samples/s, screen at 60
frames/s), undo, saving and opening. Times per frame are split into *painting* (the brush),
*picture* (the changed tiles recombined from all 15 layers; CPU) and *display* (upload and
drawing until the GPU is done). The numbers below come from this build container: 4 shared
CPU cores and **no real GPU** (Mesa llvmpipe draws in software, so "display" here is 7-25 ms
and says nothing about a graphics card). The real PC's report is the one that counts.

| Target | Measured here (Linux build / Windows build under Wine) |
|---|---|
| Fast 20 px stroke, CPU per frame (painting + picture, median) | 1.5 / 1.8 ms; 100 % of frames within 16.7 ms |
| Fast 200 px stroke, CPU per frame | 3.3 / 4.0 ms; 100 % |
| Fast 1000 px stroke, CPU per frame | 11 / 16 ms; 87 % / 56 % of frames within 16.7 ms. **Borderline on this machine** |
| Undo / redo a 1000 px stroke across the painting | 3-8 ms, then 75-100 ms to recombine the picture (+ the display) |
| Save / open the 15-layer `.kra` (155 MB) | 1.4-1.8 s / 0.7-2.3 s |
| Memory for the 15 layers | 469 MB |
| Display redraw with nothing changed | Needs a real GPU (software GPU here: 7-10 ms) |
| Tablet sample rate on the Artist 12 (both APIs) | Waiting for the user's report |
| Pen-to-screen time | Waiting for the user's report (test app panel) |

Engine-only measurements (Phase 1 groundwork, `kra_tool bench`): flatten of 15 4K layers
0.26-0.57 s; one soft dab 10 px 2.6 µs, 100 px 0.14-0.25 ms, 500 px 2.4-2.8 ms on 4 cores.
This container is shared, so timings vary by up to 2× between runs.
