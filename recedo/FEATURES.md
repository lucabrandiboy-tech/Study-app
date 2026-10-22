# Recedo: feature status

Every feature in the spec, with one status:

* **WORKING**: run and verified (how is stated).
* **PARTIAL**: some of it works; what is missing is stated.
* **BROKEN**: known to fail.
* **NEEDS HARDWARE CHECK**: done and tested without the real tablet; needs the user's pen.
* **NOT STARTED**: nothing yet. Never shown in the app as if it worked.

Current phase: **0 (foundation and proof), waiting for the user's pen check.** At the user's
request ("get as much done as possible") work on the Phase 1 *engine* started early: the
parts that do not depend on the pen results (tiles, brush, layers, undo, `.kra`, display).
They live in `crates/recedo-core` and `crates/recedo-render` and are tested there, but the
Recedo app itself (its window, panels and tools) is **not built yet**, so they are marked
PARTIAL ("engine only") until they are usable in the app.

"Test app" means the Phase 0 tablet test program (`recedo-tablet-test.exe`), not Recedo itself.
"Engine only" means: implemented and tested in the engine, not yet reachable from an app.

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
| Pressure controls size and opacity, per brush | PARTIAL | Test app brush: pressure→size, pressure→opacity switches. Engine only: each brush has pressure→size, →opacity and →flow curves (`brush.rs`, tested). |
| Tilt used by brushes | PARTIAL | Test app brush stretches with tilt; engine brush has the same option (tested). Real brush engine options: Phase 2. |
| Pressure curve editor, global + per brush | PARTIAL | Engine only: Krita-style curves (control points, smooth, never overshoot; `curve.rs`, tested). No editor window yet (Phase 2). |
| Switch between Wintab and Windows Ink | NEEDS HARDWARE CHECK | Test app switches live. Recedo settings: Phase 1/7. |
| Pen side buttons customisable | NOT STARTED | Phase 2. The test app shows what the buttons report. |
| Eraser end switches to the eraser | NEEDS HARDWARE CHECK | Test app erases with the eraser end (GPU test with synthetic eraser samples). The Artist 12's pen may have no eraser end. |
| Every action bindable to any key combination (for express keys) | NOT STARTED | Phase 7. |

## Line smoothing

| Feature | Status | Notes |
|---|---|---|
| Krita-style stabilizer: distance, delay, finish line, smooth pressure | NEEDS HARDWARE CHECK | Re-implemented from Krita's algorithm (`stabilizer.rs`: per-millisecond resampling, moving average, delay dead zone, finish line, stabilized sensors, zoom-scaled distance; 15 tests). Usable now in the test app ("Stabilizer (Phase 1 preview)"); how it feels needs the pen. |
| Off setting with shortcut and tool-option toggle | PARTIAL | Test app checkbox. Shortcut and tool options: with the Recedo app. |
| Basic and weighted modes | NOT STARTED | Phase 1 (extras). |

## Brushes (Phase 2 unless noted)

| Feature | Status |
|---|---|
| Built-in presets: pencil, inking pens, oil, acrylic, gouache, airbrush, soft round, watercolour, markers, erasers | NOT STARTED |
| Full brush editor (tip shape, size, opacity, flow, hardness, spacing, angle, roundness, rotation, scatter, texture, colour dynamics, blend mode, sensor curves for pressure/tilt/speed/random) | NOT STARTED |
| Colour smudge engine (smearing, dulling, smudge length, colour rate) | NOT STARTED |
| Built-in paper and canvas textures | NOT STARTED |
| Brush size up to 1000 px | PARTIAL: engine dabs up to 20000 px; a 500 px soft dab takes 3 ms on one core, so big brushes need the multi-core painter planned for Phase 2 |
| Brush outline cursor at true size and shape, follows zoom and rotation | NOT STARTED |
| Each preset remembers its size, opacity and settings | NOT STARTED |
| Brush library with tags and favourites; save, rename, duplicate, delete | NOT STARTED |
| Radial pop-up palette | NOT STARTED |
| Import `.kpp`, bundles, `.abr` (with a report of what did not translate) | NOT STARTED (Phase 8) |

## Colour (Phase 3 unless noted)

| Feature | Status |
|---|---|
| Colour wheel with triangle/square, HSV and RGB sliders, hex input | NOT STARTED |
| Krita-style Advanced Colour Selector | NOT STARTED |
| Eyedropper, current layer / all layers, Ctrl quick access | PARTIAL: engine only (`ops::pick_color`, all layers or one, radius average, tested); no tool yet |
| Palettes: create, edit, save, `.kpl` and `.gpl` import/export | NOT STARTED |
| Recent colours | NOT STARTED |
| Harmony helpers (complementary, analogous, triadic, split-complementary, tetradic) | NOT STARTED |
| 8-bit default, 16-bit per document | NOT STARTED |
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
| 15 layers at 4K comfortably | PARTIAL: engine benchmark below (15 fully painted 4K layers: flatten 0.26 s, undo snapshot 1.1 ms, save 1.7 s, open 1.4 s, about 500 MB of tiles); not yet in an app |
| Paint, group, filter and text layers | NOT STARTED |
| Opacity, visibility, lock, alpha lock, rename, drag to reorder, duplicate, merge down, merge selected, flatten | PARTIAL: engine only: opacity, visibility, lock, rename, add, delete, move, duplicate, merge down (picture unchanged, tested), clear, flatten. No alpha lock or merge selected yet; no panel |
| Full Krita blend mode set | PARTIAL: engine only: 16 of Krita's modes (Normal, Multiply, Screen, Overlay, Hard Light, Soft Light (Photoshop), Color Dodge, Color Burn, Linear Dodge, Linear Burn, Darken, Lighten, Difference, Exclusion, Subtract, Divide) with Krita's 8-bit arithmetic; read from and written to `.kra`. Real Krita 5.2.2 flattens a two-layer file for each mode with **0** difference from Recedo (`scripts/krita-interop.sh`, on CI). Krita has about 100 modes; the rest (HSL modes, etc.) are not started |
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
| Space+drag pan, wheel/middle-button zoom, remappable | PARTIAL: engine only (`view.rs`: pan, zoom about the cursor, Krita's zoom steps; `recedo-render` draws it, GPU-tested). No input bindings yet | 1 (remapping: 7) |
| Free rotation, reset rotation | PARTIAL: engine only (rotate about the cursor, reset; GPU-tested) | 1 |
| Mirror view with M | PARTIAL: engine only (mirror about the cursor; GPU-tested) | 1 |
| Navigator / minimap | NOT STARTED | 7 |
| Second view of the same canvas | NOT STARTED | 7 |
| Smooth zoom, pixel grid at high zoom | PARTIAL: mipmaps when zoomed out (fine detail averages instead of shimmering), sharp pixels from 200 % (GPU-tested). No pixel grid yet | 1 |
| New canvas dialog: presets (3840×2160, 1280×720, …), custom size/DPI, background, depth, custom presets | NOT STARTED | 1 |
| Canvas resize, image resize, crop | NOT STARTED | not assigned by the spec; proposed for 5 |

## Undo and history (Phase 1)

| Feature | Status |
|---|---|
| 50 undo steps (configurable), tiled so undo only stores changed tiles | PARTIAL: engine only (`history.rs`: tiles shared between steps, a stroke copies only the tiles it touches; 1.1 ms per step for 15 4K layers; tested) |
| History panel, click to jump | PARTIAL: engine only (list of steps, jump to any; tested). No panel |

## Files

| Feature | Status | Phase |
|---|---|---|
| Native format compatible with Krita `.kra` (opens in Krita; Krita files open in Recedo, with warnings for unsupported parts) | PARTIAL (basic level done): 8-bit RGBA paint layers with name, visibility, opacity, lock, position. Checked against the real Krita 5.2.2 both ways (`scripts/krita-interop.sh`, also on CI): Krita's flattening of a Recedo file differs by 0; Krita re-saves it and Recedo reads the same layers and pixels; Krita-made files open with pixels identical to Krita's own. Groups: opened as plain layers when that keeps the picture, otherwise merged into one layer exactly as Krita composites the group (Recedo has no groups yet); vector/filter/fill/file/clone layers, masks, blend modes Recedo lacks and other colour depths are listed in plain words and left out (shown as Normal for unknown blend modes) | 1 basic, 4 full, 9 animation |
| Save (Ctrl+S), Save As | PARTIAL: engine only: safe save (written to a `.part` file, renamed over the old file only when complete) | 1 |
| Autosave to a separate recovery file, crash recovery, interval setting | PARTIAL: engine only (`autosave.rs`: timing, background save from a snapshot, recovery files found at start-up; changes made while an autosave runs are kept for the next one; an autosave never marks the painting as saved; tested). No app yet | 1 |
| Open PNG, JPG, `.kra` | PARTIAL: engine only: opens all three, recognised by content (`import.rs`; JPG grey/RGB/CMYK, tested). JPG photo rotation (EXIF) is not applied yet | 1 |
| Open WebP; paste clipboard image as a new layer | NOT STARTED | 8 |
| Export PNG (transparency), JPG (quality) | PARTIAL: engine only: PNG with transparency; JPG with quality, transparent areas filled with white (like Krita), full colour resolution (decoded again in a test) | 1 |
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
| Dockable panels (dock, float, tab, close, reopen), named workspaces | NOT STARTED | 1 shell, 7 |
| Dark theme | PARTIAL | Test app panel is dark; Recedo theme: 1 |
| Light theme, accent colour | NOT STARTED | 7 |
| Adjustable UI scale | PARTIAL | Test app has a panel size slider; Recedo: 7 |
| Krita default shortcuts, remappable, conflict warnings, import/export | NOT STARTED | 7 |
| Tab: canvas-only focus mode | NOT STARTED | 7 |
| Tool options panel | NOT STARTED | 1 onward |
| Settings window (tablet API, pressure curve, pen buttons, autosave, undo steps, theme, UI scale, shortcuts, performance) | NOT STARTED | 7 |

## Network

| Feature | Status | Notes |
|---|---|---|
| Fully offline, no telemetry, no accounts | WORKING | There is no networking code; the test app sends nothing anywhere. |
| Optional update check, off by default | NOT STARTED | Lowest priority. |

## Packaging (Phase 10)

| Feature | Status |
|---|---|
| Windows installer | NOT STARTED (Phase 0 ships a portable .exe in a zip) |
| `USER_GUIDE.md` | NOT STARTED |

## Benchmarks

The spec's performance targets (4K, 15 layers, 16-bit, 1000 px brush; refresh-rate canvas;
gap-free fast strokes; instant undo; fast file opening) are about the real canvas engine,
which Phase 1 builds. The benchmark script is written in Phase 1 and its numbers recorded
here after every phase.

| Target | Measured |
|---|---|
| All performance targets in the app | Not measured yet: the Recedo app does not exist yet |
| Engine, 3840×2160, 15 fully painted layers (worst case: noise everywhere), 4-core container | Flatten 0.26 s; undo snapshot 1.1 ms; save 1.7 s (155 MB file); open 1.4 s. `cargo run --release -p recedo-core --example kra_tool -- bench <dir> 14` |
| Engine, 3840×2160, 6 layers | Flatten 0.18–0.22 s; save 0.85 s (62 MB); open 0.5 s |
| Engine, one soft dab (CPU) | 10 px: 2.6 µs; 100 px: 0.14 ms; 500 px: 3.1 ms |
| Tablet sample rate on the Artist 12 (both APIs) | Waiting for the user's report |
| Pen-to-screen time in the test app | Waiting for the user's report (shown in the panel) |
