# Recedo: feature status

Every feature in the spec, with one status:

* **WORKING**: run and verified (how is stated).
* **PARTIAL**: some of it works; what is missing is stated.
* **BROKEN**: known to fail.
* **NEEDS HARDWARE CHECK**: done and tested without the real tablet; needs the user's pen.
* **NOT STARTED**: nothing yet. Never shown in the app as if it worked.

Current phase: **1 (the painting app), finished; waiting for the user's pen check.** The
Phase 0 pen checklist is also still open (the pen has not been tried on either program yet).

**Browser version.** Smart App Control on the user's PC blocks unsigned `.exe` files, and the
user cannot turn it off without an administrator. So the same editor, interface and canvas
display also run in the browser (`crates/recedo-web`: WebAssembly, WebGL2), installable from
Edge or Chrome as an app with its own window:
https://lucabrandiboy-tech.github.io/Study-app/recedo/. Pen input there comes from the
browser (Windows Ink underneath); **Wintab is not available in a browser**. Statuses below
are for `recedo.exe` unless a row says otherwise.

* "Test app" means the Phase 0 tablet test program (`recedo-tablet-test.exe`).
* "Self-test" means `recedo.exe --selftest`: it drives the real Recedo window with injected
  input (mouse, keyboard, a Windows Ink pen with pressure, the stand-in Wintab driver) and
  checks the painting, the screen and the files. On real Windows (CI, 954025382d): 38
  passed, 0 failed, 0 skipped, including the seven checks of the new tools. Under Wine: 33 passed, 3 skipped (no pen injection or screen read-back there, and Wine
  drops keys held with SendInput when the mouse is pressed).
* "UI tests" render the whole interface off-screen on a GPU and click through it.
* "Editor tests" drive the app's editor (everything between the input and the screen) with
  synthetic pen samples.
* "Browser test" means the browser version in headless Chrome (software GPU, SwiftShader),
  driven with injected pen, mouse, touch and key events, checked on screenshots. Not the
  user's PC, browser or graphics card.

## Phase 1 deliverables

| Item | Status | Notes / how verified |
|---|---|---|
| GPU canvas (wgpu, DirectX 12 / Vulkan / OpenGL) | WORKING | Self-test on Windows reads the window's image back: painted spots dark, empty ones white. GPU tests compare drawn pixels with the view math. |
| Tiled storage (64x64 tiles, copy on write) | WORKING | Engine tests; 15 painted 4K layers use 469 MB. |
| Pan, zoom, rotate, mirror | WORKING | Space+drag, middle-button drag, wheel zoom about the pointer, "2" fit: self-test on Windows. Shift+Space rotate, Ctrl+Space zoom, 4/6/5, M, 1, +/-: editor and UI tests (same input path as the Space key, which the self-test runs). When the canvas area changes size (window, panels, the reference panel, G) a painting still fitted to the window is fitted again, and otherwise what was in the middle stays in the middle at the same zoom (editor test; browser test with the reference panel and G). |
| Round brush with pressure (size, opacity, flow), hardness, spacing, Wash / Build-up | NEEDS HARDWARE CHECK | Injected Windows Ink stroke on Windows: line 3 px wide at light pressure, 19 px at full; mock Wintab: 2 to 21 px; every touched point painted. The real pen: checklist part 2. |
| Krita stabilizer (distance, delay, finish line, smoothed pressure, zoom-scaled) | NEEDS HARDWARE CHECK | Engine tests (15) and editor tests (keeps catching up while the pen is held still). How it feels needs the pen. |
| Stability slider (0-100, like other apps' "Stabilization") with adaptive smoothing | WORKING; the feel NEEDS HARDWARE CHECK | On the toolbar next to the brush size and under Size in Tool property; 0 is off. The slider drives a speed-adaptive filter (a two-stage "One Euro" filter on the screen-space path, resampled every millisecond with the readings joined by straight lines): strong while the pen is slow, where a shaky hand shows, lighter as it speeds up, where trailing would show. The line always ends where the pen lifts and smooths the same at any zoom. Compared with the plain average it replaced, on simulated hands (200 readings/s; `cargo test -p recedo-core smoothing_comparison -- --ignored --nocapture`): at 50, wobble 0.49 vs 0.77 px and trailing a 1500 px/s stroke 15 vs 22 px; at 80, 0.10 vs 0.19 px, 43 vs 109 px, circles shrink 2.6 vs 3.3 px; at 100, 0.03 vs 0.08 px, 87 vs 292 px, corners 16 vs 20 px off, circles 9 vs 20 px. Honest catch: from 50 to 80 it rounds a sharp corner slightly more (2.6 vs 1.8 px at 50); low settings trail a little more than before (about 9 ms at 10, still too little to feel). Test `adaptive_smoothing_beats_the_plain_average` keeps these. Browser test with realistic pen timing: the same shaky line at 0, 30, 60, 90 goes from jagged to nearly straight. Settings from the first slider version move over at the same number (settings test). "Stabilizer details" offers Krita's average instead, a dead zone (string) and the finish line. |
| Stabilizer string | WORKING | While the line trails the pen, a thin blue line joins the pen to where the line is being drawn, with a dot there (as in Lazy Nezumi or Procreate); a circle shows the dead zone when one is set. UI test renders it mid-stroke and checks it is gone after lifting (fails without it). |
| Hold still to straighten | WORKING; the feel NEEDS HARDWARE CHECK | Keep the pen still (within 3 px) for half a second at the end of a stroke at least 30 px long that strays from straight by no more than 6 px or 8 % of its length: it becomes a straight line from where it began, with the pressure the hand used along the way; moving the pen before lifting moves its end; one undo step. Curves, short strokes and strokes lifted at once stay as drawn; a checkbox under Stability turns it off. Editor test (each case; fails with it disabled); browser test: a shaky diagonal held 0.8 s became a straight line, one "Brush stroke" step. |
| Eraser tool, eraser end of the pen | Tool WORKING; eraser end NEEDS HARDWARE CHECK | Editor tests with synthetic eraser samples. |
| Eyedropper (tool, Ctrl+click, all layers or one, averaging radius) | WORKING | Editor tests. |
| Layers: add, delete, duplicate, move, merge down, clear, rename, opacity, visibility, lock, 16 blend modes | WORKING | Editor tests; self-test on Windows: Insert adds a layer, paint goes only to the selected one. Locked/hidden layer: message instead of painting (editor tests). Layers panel as in Clip Studio: blend mode and an Opacity slider (how see-through the selected layer is) on top, icon buttons, and rows with a small picture of the layer, its name, opacity and blend mode. Browser test: the slider at 30 % turns a black line light grey (178) and the row says 30 %. The pictures are averaged from every pixel (every second one on paintings over 2 megapixels), so thin lines show (engine test; a 3×3 sample missed them), and redone within 0.4 s of painting, also when nothing else happens (UI test; it stayed out of date before that fix). |
| Undo 50 steps, history panel (click to jump) | WORKING | Editor tests; Ctrl+Z / Ctrl+Shift+Z in the self-test on Windows. Slider drags are one step. F undoes too (asked for; the fill tool moved to Shift+F); an F typed into a text stays a letter (UI test `f_undoes_and_shift_f_fills`, fails without the change; browser test: F, F, Shift+F, redo). |
| New canvas dialog (presets incl. 3840x2160 and 1280x720, custom size/resolution/background) | WORKING | UI tests. 8-bit only. |
| `.kra` open/save (basic level) | WORKING | Real Krita 5.2.2 both ways (CI); self-test on Windows saves a 3-layer 4K painting under a non-English file name and reads it back identical. |
| PNG / JPEG export | WORKING | Self-test on Windows: PNG equals the flattened painting. JPEG: editor tests (decoded again). |
| Autosave and crash recovery | WORKING | Self-test on Windows: a recovery copy is written, a normal close removes it. Under Wine: a copy left by a killed Recedo is offered at the next start ("Recover your work?"); opening it is covered by editor tests. A copy is also written at once when Windows shuts down. Not offered while another Recedo is running. |
| Docking shell: panels dock, tab, float, close, reopen, resize | PARTIAL | UI tests: floating, closing, reopening from the Window menu, layout kept in the settings. Dragging tabs between groups by hand has not been run in a test (it is in the checklist). |
| Settings (tablet interface, interface size, autosave interval), window position kept | WORKING | Settings tests (damaged file kept aside, values clamped); switching Windows Ink / Wintab in the self-test. |
| Benchmark | WORKING | `recedo.exe --benchmark`; numbers at the end of this file. |
| Interface at 125 % / 150 % display scaling | NEEDS HARDWARE CHECK | Per-monitor DPI aware; rendered at 1080p and 150 % in a UI test (panels scroll, sections fold). Only seen at 100 % on a screen. |
| Browser version, installable as an app (for PCs that block `.exe` files) | NEEDS HARDWARE CHECK | Browser tests: loads with no errors; pen strokes with pressure (thin to thick); mouse strokes; every tool below; pinch zoom; autosave into the browser's storage and restore after a reload; display scaling 125 / 150 / 200 % (strokes where the pen went, no stray copies when zoomed in or out). The offline worker registers. Not yet seen on the user's PC with the pen. |
| Clip Studio-like layout (asked for after 1.0) | WORKING | Icon toolbar (new, open, save, undo, redo, brush size, zoom, fit, and Reference in the browser), a narrow strip of tool icons on the far left with the two colours under it, the Colour, Sub tool and Tool property panels on the left, Layers and History on the right, Clip Studio's greys. All 43 icons are drawn by Recedo (UI test: each paints, no two alike; looked at on a sheet at 20, 28 and 64 px). Panel tabs name what they show ("Sub tool [Brush]", "Tool property [Pencil]"). A layout saved by an older version is replaced by this one once (dock test); Window > Reset panels too. The strip's icons shrink to fit a short screen (UI test at 1080p, 150 %). Browser test at 1600×900 and screenshots at 1366×768. |
| Side bar of your choice, and an "All tools" window with 32 tools | WORKING | The tool strip on the left shows the tools you picked as icons, in your order (point at one for its name and key; kept in the settings; the standard set has 16; a bar saved as the old standard set of 15 gets the new one). The grid button at its top lists every tool in six groups: click one to use it, tick "On bar" to keep it on the strip, Up / Down / Remove to order it, "Back to the standard tools". A tool that is not on the bar shows at its end while you use it. UI tests (every tool on the strip and on screen, a click on an icon picks its tool, every tool listed without scrolling at 1600×900, ticking and ordering, the window keeps its size, nothing on the canvas draws over it); browser test. |
| Sub tool and Tool property panels | WORKING | Sub tool lists the brush presets (10) and eraser presets (4), each with a stroke painted by the real brush engine, or for other tools the tools of the same kind with their icons and keys. Tool property starts with the current brush's stroke in the colour, at its real size when it fits, then its settings (engine tests of the stroke samples: size, taper, opacity; browser test: picking Pencil changes Tool property). |
| Retouch tools: smudge, blur, sharpen, dodge, burn, sponge, recolour, clone, push, spray | WORKING; pressure NEEDS HARDWARE CHECK | One soft round brush for all of them (size, strength, hardness; smudge length; spray dots and dot size); pen pressure sets the strength. Engine tests (12): each changes only what is under the brush and is stronger with more strength; blur reaches further with a bigger brush (its reach is an eighth of the size, so it shows on a 4K canvas); sharpen leaves no halo at the edge of paint; smudge drags paint along; clone copies exactly (Ctrl+click sets where from); push moves pixels the way the brush goes; spray stays inside the brush. Editor tests: one undo step each, nothing on a locked layer, the eraser end still erases. Self-test on real Windows and under Wine: S picks smudge, a mouse stroke drags paint, one undo step. Browser test: every one of them on screenshots. |
| Selections: rectangle, ellipse, lasso, magic wand; add, take away, keep the overlap; select all, deselect, invert; marching ants | WORKING | Shift when starting a selection adds to it, Alt takes away, both keep the overlap (or the buttons in Tool options). Painting, fills, shapes, retouching, text, Delete and Backspace stay inside the selection; copy or cut it to a new layer; crop to it. Selections are undo steps. Engine and editor tests; self-test on real Windows and under Wine: R, a drag, Delete clears inside it only, Ctrl+Z, Ctrl+D; browser test (rectangle with a brush stroke across it, wand + Backspace, ellipse + fill, lasso + Delete). Shift/Alt held at a real mouse press: editor tests only (Wine drops held keys there). Invert: Ctrl+I or Ctrl+Shift+I (UI test; browser test: both invert a box selection in headless Chromium). With nothing selected it now says so instead of doing nothing silently. If a browser opens its developer tools on Ctrl+Shift+I (headless Chromium has none, so this is untested), Ctrl+I does the same job. |
| Move, crop, text | WORKING | Move: drag, or the arrow keys (Shift: 10 px); moves the selected pixels when there is a selection, else the whole layer. Crop: drag, then Enter or Crop; Escape cancels. Text: click, type, then Ctrl+Enter or "Place text"; Sans or Mono (the interface's own fonts), size, bold, line spacing, in the colour; a click elsewhere moves it; picking another tool places it; Escape cancels. Self-test on real Windows and under Wine: C and Enter crop, Ctrl+Z undoes; Right / Shift+Right move the layer 1 / 10 px; T, a click, typing "hi", Ctrl+Enter paints the text. UI tests: after a click that moves the text, typing still goes into it, and letters are never shortcuts while writing. Limits: text is painted into the layer (not editable afterwards); its preview does not turn with a rotated view. |
| Reference panel (browser version): search pictures, videos and articles, watch video links, beside the canvas | WORKING with stand-in answers; the live sites NEED A CHECK on the user's PC | "Reference" in the toolbar (or Window menu) opens a panel on the right; × closes it, its left edge drags to resize, the canvas shrinks to make room. Pictures from Wikipedia, Wikimedia Commons, Openverse, the Art Institute of Chicago and NASA; videos from Wikimedia Commons and the Internet Archive; Wikipedia articles, read inside the panel; "Watch a link" plays YouTube (no-cookie player, `t=` start times, playlists), Vimeo, Internet Archive, Commons and plain .mp4/.webm links. All keyless. Private: nothing about searches is stored (no history, address bar or storage), answers are fetched with `no-store` and without a referrer, the offline worker never keeps other sites' answers, the search box is outside any form. No word filter (removed at the user's request: it hid most results); Openverse is still asked for its own non-mature results. Browser test with stand-in answers in each site's real format: every kind of result, the picture viewer, a Commons and a YouTube player, an article; letters typed in the box are not shortcuts, a press on the canvas gives the keys back, painting lands under the pen with the panel open; closing removes the player. "Full screen" in its title bar fills the screen with the panel (Escape, the button again or × comes back; browser test). This container cannot reach those sites, so the real answers are untested here. `recedo.exe` has no panel (no web view). |
| Lasso fill, magic eraser, zoom tool, rotate-view tool, measure | WORKING | Editor tests; browser test (lasso fill, zoom clicks, rotating by dragging and 5 to straighten, a measured line of 1374 px at -14°). |
| Speed of the new tools at 3840×2160 | NEEDS HARDWARE CHECK | Browser test, software graphics: the longest pause is 0.14 s for the magic wand on most of the canvas, 0.15 s to fill that selection, 0.2-0.3 s to clear it, 0.4 s to move everything at once, 0.1 s to crop and 0.3 s to undo it, under 0.1 s per frame for a 300 px blur stroke, 0.04 s to place 400 px text. |
| Fill (paint bucket), line, rectangle, ellipse and gradient tools | WORKING | Editor tests: each is one undo step and paints where it was dragged; fill stays inside a drawn outline with no gap at the line's soft edge; Shift gives 15° angles, squares and circles; locked layers refuse with a message; the pen's eraser end still erases with these tools. Browser test: rectangle + fill, Shift line, ellipse, gradient on a new layer and its undo, checked on screenshots. Options: see "Fill and gradient" and "Shapes". Speed (browser test, software graphics, 3840×2160): filling the whole canvas holds the app for 0.6 s, its undo 0.5 s, a gradient 0.4 s (they were 1.6 / 1.3 / 0.7 s before the fill, picture-update and upload code was sped up). |
| Brush presets (10) and flat brush tips (roundness, angle) | WORKING | Paint brush, Pencil, Ink pen, Fine liner, Felt-tip, Marker, Highlighter, Calligraphy, Watercolour, Airbrush set the round brush's settings (Tool options). Roundness/angle: engine tests (dab shape; a 5 %-round tip leaves no gap or stripe going along, across or diagonally, because dabs are spaced by the tip's width in the direction of travel, as in Krita); browser test draws with the highlighter and calligraphy presets. |
| Browser version keeps working after you leave it | WORKING (browser test) | The browser can take the graphics (WebGL) away from a page: a hidden tab, sleep, a graphics driver restart. Recedo stayed blank grey for good before; it now asks for them back and sets the drawing side up again, with the painting, letters and previews as they were. Browser test: the context lost and given back twice, a stroke painted after each. Leaving the tab writes the autosave at once (browser test: 189 KB copy right after hiding); losing focus mid-stroke ends the stroke there, and nothing paints by itself on coming back (browser test). |
| No crash when an image is replaced while drawn | WORKING | egui's freed images were destroyed before the frame that still drew them was sent, which wgpu rejects (a crash; the Wine self-test at 1024x768 hit it every time). They are freed after the frame now. UI test `an_image_dropped_while_drawn_does_not_crash` fails without the fix. |
| Interface look: filled sliders, flat tabs, tool icons everywhere | WORKING | Sliders are bars filled in blue up to the value; panel tabs are text with a blue line under the open one; check boxes and fields have an edge; the painting has a dark edge and a soft shadow; the All tools window shows each tool's icon; dialogs have room round their contents, the main button in blue. Escape closes an open menu (it did not: the app's own Escape took the key first; UI test). Looked at on screenshots at 1920×1080, 1600×900 and 1366×768. |
| Lasso select (and every tool) next to a dock | WORKING | A press on the canvas right at a dock's edge also grabbed the dock's resize handle: the dock followed the pen and the painting moved under a lasso being drawn (a lasso round 650 px came out 304 px wide). Presses that close to a dock are now for the handle only, and a dock no longer grows with what its panels show (it crept 1-2 px on some tool changes, once 155 px); only dragging its edge changes it. The left dock's standard width is 232 (Tool property's rows fit). UI tests `a_press_at_the_dock_edge_resizes_the_dock_and_nothing_else` (fails without the fix) and `the_docks_keep_their_width_whatever_the_tool` (every tool, with and without a selection, at 1600×900, 1366×768 and 1008×681); browser test: lassos with the mouse and pen, partly outside the painting, starting off it, 400-point slow loops and figure-eights, each followed by a brush stroke that stays inside, run three times in a row with no failure. A tool key pressed just before a press now takes effect first (a safeguard: the browser test could not show it failing before). |

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
| Known issue: an injected Windows Ink stroke once split into many | Not reproduced since | On the Windows CI machine, the run for f5038eb turned one injected Ink stroke into 24 undo steps (90 % of the line painted). Nothing in that change touched pen input, and later runs pass (639a255: 31 of 31). The self-test now prints a trace of every pen message and of where strokes start and end if it happens again. |
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
| Built-in presets: pencil, inking pens, oil, acrylic, gouache, airbrush, soft round, watercolour, markers, erasers | PARTIAL: 10 presets of the round brush (Paint brush, Pencil, Ink pen, Fine liner, Felt-tip, Marker, Highlighter, Calligraphy, Watercolour, Airbrush) in Tool options. Oil, acrylic and gouache need the smudge and texture engines (not started); erasers have no presets yet |
| Full brush editor (tip shape, size, opacity, flow, hardness, spacing, angle, roundness, rotation, scatter, texture, colour dynamics, blend mode, sensor curves for pressure/tilt/speed/random) | PARTIAL: size, opacity, flow, hardness, spacing, roundness and tip angle in Tool options. The rest NOT STARTED |
| Colour smudge engine (smearing, dulling, smudge length, colour rate) | PARTIAL: the smudge tool smears with Krita's smudge length, strength and hardness. It is a tool of its own, not a brush engine; no dulling or colour rate |
| Built-in paper and canvas textures | NOT STARTED |
| Brush size up to 1000 px | NEEDS HARDWARE CHECK: Recedo's size slider goes to 1000 px ([ and ] step it; in `recedo.exe` those two keys did nothing until now, the self-test checks them). Benchmark on this 4-core container, 15-layer 4K painting: a fast 1000 px stroke costs 7-9 ms of painting + 5-7 ms of picture update per frame (median), within a 60 Hz frame in 56-87 % of frames. The real PC's numbers: `--benchmark` |
| Brush outline cursor at true size and shape, follows zoom and rotation | PARTIAL: circle at the true size, follows zoom; not the shape of tilted/leaned dabs yet |
| Each preset remembers its size, opacity and settings | NOT STARTED |
| Brush library with tags and favourites; save, rename, duplicate, delete | NOT STARTED |
| Radial pop-up palette | NOT STARTED |
| Import `.kpp`, bundles, `.abr` (with a report of what did not translate) | NOT STARTED (Phase 8) |

## Colour (Phase 3 unless noted)

| Feature | Status |
|---|---|
| Colour wheel with triangle/square, HSV and RGB sliders, hex input | PARTIAL: a hue ring around a saturation/value square (Clip Studio's), a second colour (X swaps them; kept in the settings), RGB numbers, sliders and hex input (UI tests; browser test of X). No triangle, no HSV sliders |
| Krita-style Advanced Colour Selector | NOT STARTED |
| Eyedropper, current layer / all layers, Ctrl quick access | WORKING: tool (P) and Ctrl+click with the brush; visible picture or selected layer; averaging radius (editor tests) |
| Palettes: create, edit, save, `.kpl` and `.gpl` import/export | NOT STARTED |
| Recent colours | WORKING: the last 16 colours painted with (any tool), as swatches in the Colour panel, kept in the settings (editor tests) |
| Harmony helpers (complementary, analogous, triadic, split-complementary, tetradic) | NOT STARTED |
| 8-bit default, 16-bit per document | PARTIAL: Krita's 16-bit RGB layers open (converted to 8-bit, with a warning; within 1/255 of Krita's own 16-bit export). Painting and saving in 16-bit: not started |
| Colour management: sRGB working space, ICC profiles, embedded on export, display profile | NOT STARTED |

## Fill and gradient (Phase 5)

| Feature | Status |
|---|---|
| Bucket fill: tolerance, grow/shrink, feathering, anti-aliasing | PARTIAL (done early): tolerance slider; the fill always grows 1 px under the edge of the lines around it so no light gap shows (engine and editor tests). No grow/shrink setting, feathering or anti-aliasing |
| Gap closing | NOT STARTED |
| Fill using a reference layer | PARTIAL: finds the area in the visible picture (default, so lines on another layer hold the fill) or in the selected layer only. Not an arbitrary reference layer |
| Gradient: linear, radial, FG→BG, FG→transparent, repeat options | PARTIAL (done early): linear, colour to transparent, over the whole layer (solid before the start, untouched after the end), Shift for 15° steps (engine and editor tests). No radial, FG→BG or repeat |

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
| Gaussian blur, motion blur, sharpen/unsharp mask, levels, curves, HSV/HSL, colour balance, desaturate, add noise, pixelate | NOT STARTED as filters. Blur, sharpen and desaturate exist as brushes (retouch tools) |
| Live on-canvas preview | NOT STARTED |
| Each filter as destructive filter and as filter layer | NOT STARTED |

## Selections (Phase 5)

| Feature | Status |
|---|---|
| Rectangle, ellipse, freehand lasso, polygon lasso, magic wand, select by colour | PARTIAL (done early): rectangle, ellipse, freehand lasso, magic wand (one connected area, in the visible picture or the layer, with a tolerance). No polygon lasso or select by colour |
| Replace / add / subtract / intersect with Krita's modifier keys | WORKING (done early): Shift adds, Alt takes away, Shift+Alt keeps the overlap, or the buttons (editor tests) |
| Feather, grow, shrink, invert, select all, deselect, marching ants | PARTIAL (done early): invert, select all, deselect, marching ants. No feather, grow or shrink |

## Transform (Phase 5)

| Feature | Status |
|---|---|
| Move, scale, rotate, flip | PARTIAL (done early): the move tool moves a layer or the selected pixels (drag, arrow keys); the lasso transform (next row) scales and moves a part drawn round. No rotate or flip |
| Lasso transform (asked for after 1.0) | WORKING | Draw round part of the selected layer; a box with eight handles appears: a corner scales (Shift keeps the shape), the middle of a side stretches one way, inside moves; Tool property has a Size % slider and Put down / Cancel. Only the selected layer changes. Enter, another tool or any command (undo, save, another layer…) puts it down as one undo step; Escape puts it back exactly. Enlarging blends neighbouring pixels, shrinking averages them (thin lines stay). Locked or hidden layers and empty areas are refused with a message. Editor tests (scale, move, Shift, undo, Escape, refusals, the box maths), engine tests (exact at the same size, colours, averaging, half-covered edge pixels, thin lines), UI test (box and handles on screen, a tool change puts it down), browser test with a pen (drawn round, corner dragged, Enter, Ctrl+Z). Speed (this machine, release build): a 1500×1000 part redone at 1.5× in 91 ms, at half size in 24 ms, moved in 35 ms; redone once per screen frame while dragging, not per pen sample (editor test). A part that big is slow to drag in the browser; small parts are quick. |
| Perspective and free distort | NOT STARTED |
| Warp and mesh | NOT STARTED |
| Liquify (push, grow, shrink, swirl, undo brush) | PARTIAL: the push brush (like liquify's move). No grow, shrink, swirl or undo brush |
| On layers, groups and selections | NOT STARTED |

## Shapes and drawing aids (Phase 6)

| Feature | Status |
|---|---|
| Line, rectangle, ellipse, polygon, polyline, Bézier tools with the current brush | PARTIAL (done early): line, rectangle and ellipse, outlined with the current brush (full pressure) and optionally filled with the colour; a preview while dragging; Shift for 15° angles, squares and circles (editor and browser tests). No polygon, polyline or Bézier |
| Hold-to-snap shapes (QuickShape-style) | NOT STARTED |
| Assistants: perspective 1/2/3-point, vanishing point, ruler, parallel ruler, ellipse, concentric ellipse, snapping strength | NOT STARTED |
| Symmetry: horizontal, vertical, radial with N segments | NOT STARTED |
| Grid, guides from rulers, snapping | NOT STARTED |

## Text, comics, reference images (Phase 6)

| Feature | Status |
|---|---|
| Editable text layers (font, size, colour, alignment, line spacing), convert to paint layer | PARTIAL (done early): the text tool paints text into the layer: two fonts, size, bold, colour, line spacing. The text being written is shown turned and mirrored with the view, where it will be painted (UI test: the preview's pixels match the painted text's on a view turned 40°, also mirrored; the old upright preview fails it). Not editable afterwards (no text layers), no alignment, no other fonts |
| Comic panel/frame tool with gutters and border thickness | NOT STARTED |
| Reference board: from file or clipboard, move, scale, rotate, opacity, saved with the document | NOT STARTED |

## Canvas and navigation

| Feature | Status | Phase |
|---|---|---|
| Space+drag pan, wheel/middle-button zoom, remappable | WORKING (not remappable yet): Space+drag and middle-button drag pan, wheel zooms about the pointer, Ctrl+Space+drag zooms, +/- and 1/2 (self-test on Windows, editor tests) | 1 (remapping: 7) |
| Free rotation, reset rotation | WORKING: Shift+Space+drag or the rotate-view tool, 4/6 by 15°, 5 resets; painting stays under the pen when rotated (editor tests, browser test) | 1 |
| Mirror view with M | WORKING (editor tests: painting stays under the pen when mirrored) | 1 |
| Navigator / minimap | NOT STARTED | 7 |
| Second view of the same canvas | NOT STARTED | 7 |
| Smooth zoom, pixel grid at high zoom | PARTIAL: mipmaps when zoomed out (fine detail averages instead of shimmering), computed on the CPU only where tiles changed (a GPU test checks every level against a reference after full and partial updates); sharp pixels from 200 % (GPU-tested). No pixel grid yet | 1 |
| New canvas dialog: presets (3840×2160, 1280×720, …), custom size/DPI, background, depth, custom presets | PARTIAL: File > New with the built-in presets (your two sizes first), custom size and resolution, background, memory estimate (UI tests). 8-bit only; saving your own presets is in the engine but not in the dialog yet | 1 |
| Canvas resize, image resize, crop | PARTIAL (done early): crop, with the crop tool or to a selection (an undo step). No resizing | not assigned by the spec; proposed for 5 |

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
| Dark theme | WORKING | Recedo and the test app are dark; Recedo's greys follow Clip Studio's. |
| Light theme, accent colour | NOT STARTED | 7 |
| Adjustable UI scale | WORKING | Edit > Settings > Interface size (75-200 %), on top of Windows' display scaling. |
| Krita default shortcuts, remappable, conflict warnings, import/export | PARTIAL: fixed one-key shortcuts for 13 tools and the selection, crop and text keys (Help > Keyboard shortcuts), Krita's where Krita has one. Not remappable | 7 |
| Canvas-only focus mode | WORKING (on G, asked for by the user, instead of Krita's Tab) | G (or View > Only the canvas) hides the menus, toolbar, panels, status bar and the browser's reference panel; G again or Escape brings everything back, a video in the reference panel keeps playing. Gradient moved to Shift+G. G typed into a text box (the reference search, a layer name, the colour's hex) is a letter, not the shortcut. UI test: the painting gets the whole window, G and Escape restore the exact layout, G while renaming a layer types a g, Shift+G gives the gradient (each fails with the shortcut removed). Browser test: G on the canvas hides the reference panel and gives the canvas the full width, G again shows it as it was; "dog" typed in the search box changes nothing. |
| Select menu; Help > Getting started; dialogs that fit small screens | WORKING | Select menu: select all, deselect, invert, fill, clear, copy / cut to a new layer, crop to the selection. Getting started: the essentials in one page. The shortcut list and Getting started scroll on a 1366×768 screen (UI test; the shortcut list used to run off the screen). Version 1.0.0 shown in About, with the build's commit in the browser version too. Renaming a layer ends with Enter (it used to keep the keyboard for good, so no shortcut worked until a reload; found by the canvas-only UI test). The "All tools" buttons have a light outline. The starting colour is black (settings saved with the old dark-blue default turn black). |
| Tool options panel | WORKING | Options for every tool (UI test draws each one): brush/eraser presets, size, opacity, flow, hardness, spacing, roundness, tip angle, mode, pen and stabilizer options; eyedropper, fill, shape, gradient, retouch, selection, crop, text, measure. |
| Fingers on a touch screen (browser version) | NEEDS HARDWARE CHECK | One finger pans, two pan and pinch-zoom, fingers never paint (browser test with injected touches: pinch 4× zooms 4× with no paint). Not tried on a real touch screen. |
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
| Display redraw with nothing changed | Needs a real GPU (software GPU here: 7-24 ms, depending on the day) |
| Tablet sample rate on the Artist 12 (both APIs) | Waiting for the user's report |
| Pen-to-screen time | Waiting for the user's report (test app panel) |

Engine-only measurements (Phase 1 groundwork, `kra_tool bench`): flatten of 15 4K layers
0.26-0.57 s; one soft dab 10 px 2.6 µs, 100 px 0.14-0.25 ms, 500 px 2.4-2.8 ms on 4 cores.
This container is shared, so timings vary by up to 2× between runs.
