# Recedo: feature status

Every feature in the spec, with one status:

* **WORKING**: run and verified (how is stated).
* **PARTIAL**: some of it works; what is missing is stated.
* **BROKEN**: known to fail.
* **NEEDS HARDWARE CHECK**: done and tested without the real tablet; needs the user's pen.
* **NOT STARTED**: nothing yet. Never shown in the app as if it worked.

Current phase: **0 (foundation and proof)**. Last updated at the end of the Phase 0 work.
"Test app" means the Phase 0 tablet test program (`recedo-tablet-test.exe`), not Recedo itself.

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

## Tablet and input

| Feature | Status | Notes |
|---|---|---|
| Pressure controls size and opacity, per brush | PARTIAL | Test app brush only (pressure→size, pressure→opacity switches). Per-brush settings: Phase 2. |
| Tilt used by brushes | PARTIAL | Test app brush stretches with tilt. Real brushes: Phase 2. |
| Pressure curve editor, global + per brush | NOT STARTED | Phase 2. |
| Switch between Wintab and Windows Ink | NEEDS HARDWARE CHECK | Test app switches live. Recedo settings: Phase 1/7. |
| Pen side buttons customisable | NOT STARTED | Phase 2. The test app shows what the buttons report. |
| Eraser end switches to the eraser | NEEDS HARDWARE CHECK | Test app erases with the eraser end (GPU test with synthetic eraser samples). The Artist 12's pen may have no eraser end. |
| Every action bindable to any key combination (for express keys) | NOT STARTED | Phase 7. |

## Line smoothing

| Feature | Status | Notes |
|---|---|---|
| Krita-style stabilizer: distance, delay, finish line, smooth pressure | NOT STARTED | Phase 1. |
| Off setting with shortcut and tool-option toggle | NOT STARTED | Phase 1. |
| Basic and weighted modes | NOT STARTED | Phase 1 (extras). |

## Brushes (Phase 2 unless noted)

| Feature | Status |
|---|---|
| Built-in presets: pencil, inking pens, oil, acrylic, gouache, airbrush, soft round, watercolour, markers, erasers | NOT STARTED |
| Full brush editor (tip shape, size, opacity, flow, hardness, spacing, angle, roundness, rotation, scatter, texture, colour dynamics, blend mode, sensor curves for pressure/tilt/speed/random) | NOT STARTED |
| Colour smudge engine (smearing, dulling, smudge length, colour rate) | NOT STARTED |
| Built-in paper and canvas textures | NOT STARTED |
| Brush size up to 1000 px | NOT STARTED |
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
| Eyedropper, current layer / all layers, Ctrl quick access | NOT STARTED (basic: Phase 1) |
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
| 15 layers at 4K comfortably | NOT STARTED |
| Paint, group, filter and text layers | NOT STARTED |
| Opacity, visibility, lock, alpha lock, rename, drag to reorder, duplicate, merge down, merge selected, flatten | NOT STARTED |
| Full Krita blend mode set | NOT STARTED |
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
| Space+drag pan, wheel/middle-button zoom, remappable | NOT STARTED | 1 (remapping: 7) |
| Free rotation, reset rotation | NOT STARTED | 1 |
| Mirror view with M | NOT STARTED | 1 |
| Navigator / minimap | NOT STARTED | 7 |
| Second view of the same canvas | NOT STARTED | 7 |
| Smooth zoom, pixel grid at high zoom | NOT STARTED | 1 |
| New canvas dialog: presets (3840×2160, 1280×720, …), custom size/DPI, background, depth, custom presets | NOT STARTED | 1 |
| Canvas resize, image resize, crop | NOT STARTED | not assigned by the spec; proposed for 5 |

## Undo and history (Phase 1)

| Feature | Status |
|---|---|
| 50 undo steps (configurable), tiled so undo only stores changed tiles | NOT STARTED |
| History panel, click to jump | NOT STARTED |

## Files

| Feature | Status | Phase |
|---|---|---|
| Native format compatible with Krita `.kra` (opens in Krita; Krita files open in Recedo, with warnings for unsupported parts) | NOT STARTED | 1 basic, 4 full, 9 animation |
| Save (Ctrl+S), Save As | NOT STARTED | 1 |
| Autosave to a separate recovery file, crash recovery, interval setting | NOT STARTED | 1 |
| Open PNG, JPG, `.kra` | NOT STARTED | 1 |
| Open WebP; paste clipboard image as a new layer | NOT STARTED | 8 |
| Export PNG (transparency), JPG (quality) | NOT STARTED | 1 |
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
| All performance targets | Not measured yet: no canvas engine in Phase 0 |
| Tablet sample rate on the Artist 12 (both APIs) | Waiting for the user's report |
| Pen-to-screen time in the test app | Waiting for the user's report (shown in the panel) |
