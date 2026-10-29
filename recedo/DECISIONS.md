# Recedo: decisions

This file records what was decided, why, and what was rejected. Status of every feature is
in [`FEATURES.md`](FEATURES.md).

## 1. Tech stack (decided in Phase 0)

**Rust + raw Win32 + wgpu (Direct3D 12, Vulkan fallback) + egui.**

| Part | Choice |
|---|---|
| Language | Rust 2024, pinned to 1.97.0 (`rust-toolchain.toml`) |
| Window, input, DPI | Own Win32 layer (`windows-sys`), per-monitor DPI aware v2 |
| Tablet | Own code for both APIs (crate `recedo-tablet`): Wintab via the driver's `Wintab32.dll` loaded at run time; Windows Ink via `WM_POINTER*` |
| GPU | `wgpu` 30: Direct3D 12 first, then Vulkan, then OpenGL |
| UI | `egui` 0.37 drawn on the GPU; `egui_dock` planned for the dockable panels (Phase 1/7) |
| Build | Cross-compiled from Linux to a self-contained `x86_64-pc-windows-gnu` .exe (needs only DLLs that ship with Windows 10/11) |

### Why

1. **Tablet input is the one thing that cannot be faked**, and no UI toolkit does it fully.
   Krita itself keeps its own Wintab code and patches Qt's Windows Ink handling. Writing the
   tablet layer ourselves gives exactly what the spec asks for:
   * **Wintab**: the context is opened the way Qt and Krita open theirs (a copy of the
     driver's default *system* context, messages on, absolute packets, Y flipped). On every
     `WT_PACKET` the whole queue is drained with `WTPacketsGet`, so no packet is dropped
     between frames, and packet serial numbers detect any the driver loses.
   * **Windows Ink**: every coalesced sample of every `WM_POINTER*` message is read with
     `GetPointerPenInfoHistory`; positions are sub-pixel (HIMETRIC location mapped through
     `GetPointerDeviceRects`); repeated frames are dropped by frame id.
   * Switching between them is a setting (live in the test app).
2. **GPU canvas with testable shaders.** wgpu runs the same shaders on the user's DirectX 12
   GPU and on software Vulkan/OpenGL in Linux, so brush and compositing output is checked by
   automated tests that compare pixels, without a GPU.
3. **I can build, run and test everything myself.** The whole project builds on Linux; the
   Windows .exe runs under Wine on a virtual screen (mouse, fake Wintab driver), and on a
   real Windows machine in CI (GitHub Actions), where Windows Ink pen input can be injected.
4. **One memory-safe language** for a very large codebase.
5. **Krita-compatible files are practical in Rust**: `.kra` is a zip of XML plus
   LZF-compressed tiles (crates `zip`, `quick-xml`); colour management through LittleCMS
   (`lcms2`); image codecs through `image`.

### Alternatives rejected

| Option | Why not |
|---|---|
| **C++ / Qt 6** (Krita's stack) | The strongest widget toolkit (docks, dialogs), and Krita's code could be ported more literally. But Qt's tablet code is not enough on its own (Krita maintains its own Wintab layer and Qt patches), so the custom tablet layer is needed anyway. Building Qt for Windows from this Linux environment is slow and fragile, which would slow down the run-it-and-check loop the spec requires. C++ memory bugs are a real risk at this size. |
| **Electron / web** | No Wintab. Excluded by the spec as the app's stack. (A browser build of the same Rust code was added later, only because the user's PC cannot run unsigned programs: section 7.) |
| **C# / WPF or WinUI 3** | Ink is built in, Wintab works through P/Invoke, but a 4K multi-layer GPU canvas needs Direct3D interop anyway, and WinUI apps cannot be built or run from Linux. |
| **C++ + Dear ImGui + D3D11** | Close second: excellent docking, including panels floating as separate OS windows. Rejected mainly for memory safety; Rust + egui is the same immediate-mode approach. |
| **Rust + winit** (the usual Rust window library) | It turns pen input into touch events, drops tilt and has no Wintab. Hence the own Win32 layer. |

### Costs of this choice (being honest)

* **The UI is custom-drawn, not native Windows controls.** It will look like Krita's dark
  theme, not like Windows. Typing text (IME) and screen-reader support are basic.
* **Floating panels float inside the main window**, not as separate OS windows (an
  `egui_dock` limitation). On a single 1920×1080 pen display this is a small difference.
  Marked as an approximation for Phase 7.
* **Windows Ink pressure has 1025 levels at most** (0…1024): Windows normalises it, whatever
  the pen's own resolution. Wintab gives the pen's full resolution (8192 on the Artist 12).
  Krita has the same limit in Ink mode.
* The real GPU path (Direct3D 12 on the user's card) and the real XP-Pen driver are
  verified only on the user's PC, through the checklist.

## 2. How pen input flows (Phase 0 test app)

```
XP-Pen driver ──Wintab packets──▶ WintabInput::process ─┐
              ──WM_POINTER*────▶ InkInput::handle  ─────┼─▶ PenSample (window px, sub-pixel,
mouse ─────────WM_MOUSE*───────▶ on_mouse (mouse mode) ─┘   pressure 0..1, tilt X/Y, buttons)
                                                             │
                     statistics (rate, gaps, ranges) ◀───────┤
                     recording (CSV) ◀───────────────────────┤
                     StrokeTracker ─▶ dabs ─▶ GPU canvas ◀───┘
```

* The panel gets ordinary mouse messages, which Windows makes from the pen in both modes;
  the canvas reads the tablet API directly.
* Each frame first waits for a free swap-chain image, then reads the input that arrived
  meanwhile, then draws, so the newest samples make it onto the screen.
* Wintab positions: the driver always keeps the system cursor under the pen, so the app
  compares candidate screen mappings (tablet-provided area, whole desktop, monitor under the
  pen, each also with Y flipped) against the cursor while the pen moves slowly, and uses
  the one that matches. It can also fit a mapping from those comparisons, which covers any
  sub-area a driver uses. Positions still come from the packets (sub-pixel, every packet).

## 3. Phase 0 test results

What has been verified, and how:

| Check | How | Result |
|---|---|---|
| Wintab structures and packet layout | Unit tests against offsets computed by hand from `pktdef.h` | pass |
| Tilt math (azimuth/altitude → tilt X/Y) | Unit tests, including a flat pen and the inverse | pass |
| Screen mapping auto-selection | Unit tests: two monitors, Y flip, sub-area fit, fast moves ignored | pass |
| Wintab end to end | Self-test under Wine with a stand-in `Wintab32.dll` that plays a stroke: every packet arrives, the one deliberately dropped is detected, positions within 0.03 px, pressure within 0.0001, tilt within 0.1° | pass |
| Mouse end to end | Self-test under Wine and on Windows CI | pass (both) |
| Windows Ink end to end | Self-test on real Windows (GitHub Actions, Windows build 26100 = 24H2, Direct3D 12): pen strokes injected with `InjectSyntheticPointerInput`, several frames per message. 121 of 121 touching frames arrived, up to 3 unpacked from one message, positions within 0.024 px, pressure and tilt exact, line width follows pressure (4 px → 22 px) | pass |
| Wintab on real Windows | Same run, mock driver: 220 of 220 packets (up to 47 drained from one message), dropped packet detected, positions within 0.016 px | pass |
| App's own work per frame | Self-test: must stay under 8 ms (half a 60 Hz frame). Under Wine: 3.2 ms. The overall frame rate on CI is not meaningful (no GPU there: Windows' software renderer gave 12 fps) | pass under Wine |
| Drawing | GPU tests: pressure ramp gives thin→thick, no gaps on fast sparse samples, eraser end erases | pass |
| **Real XP-Pen Artist 12, both APIs** | **The user's checklist** | **waiting** |

Phase 1 (the Recedo app) does not start until the user confirms pressure and tilt through
both APIs. At the user's request, engine work that does not depend on those results was done
early; see section 5.

## 4. Scope: what is unrealistic or much bigger than it looks

The spec is roughly Krita plus parts of Clip Studio Paint and Procreate. Krita is the work
of many people over two decades. Each phase below is real work, and some will take far
longer than their one line in the spec suggests. Proposals:

| Item | Assessment | Proposal |
|---|---|---|
| Full `.kra` round-trip of everything | Layers, groups, blend modes, opacity, masks: realistic. Filter layers need Krita's exact filter settings XML per filter. Animation and text are large. | Do it, verified by opening Recedo's files in the real Krita (Krita can run headless here: `krita --export`). Text layers may only round-trip approximately (Krita stores text as SVG). |
| Full Krita blend mode set (~100 modes) | The formulas are public in Krita's source; the work is volume. | Implement all, in batches, each checked against reference values. |
| Color Smudge engine "like Krita" and stabilizer "matching Krita's feel" | The algorithms can be ported closely from Krita's source; *feel* can only be judged by the user. | Port the algorithms; tune with the user's pen recordings. |
| 1000 px colour-smudge brush at 4K, 16-bit, no lag | Heavy: Krita itself lags here. | Measure in Phase 2; if needed, sample smudge colour at reduced resolution (Krita does similar). |
| `.kpp` / `.abr` import | Partial by nature (different engines), as the spec says. | Import tips and the settings that map; list what did not. |
| Warp, mesh, liquify | Substantial but well understood. | Phase 5, possibly simplified liquify first. |
| Text layers | Editable text with fonts, alignment and spacing is a mini word processor. | Single-style text boxes (font, size, colour, alignment, line spacing per layer); no per-character styling. |
| Animation + tweening + GIF/MP4 + `.kra` animation | A large phase on its own. | Phase 9 as planned. |
| Timelapse MP4 | Needs ffmpeg bundled (~80–120 MB). | Fine for personal use. |
| Dock panels as separate OS windows | See above. | Panels float inside the main window. |
| Update check | Lowest priority. | Last, off by default. |

None of this is cut yet; these are the places where approximations are most likely. Any
cut will be listed in `FEATURES.md` and agreed first.

## 5. Engine decisions (Phase 1 groundwork, done early)

| Decision | Why | Verified by |
|---|---|---|
| Pixels live on the CPU in 64×64 tiles (8-bit RGBA, straight alpha); the GPU only shows them | Krita's design. Undo, saving, file I/O and filters work on plain memory; a tile is the unit of change, so a stroke touches a few tiles. GPU painting would make undo and `.kra` saving need read-backs. | Engine tests; benchmark: 10 px dab 2.6 µs, 100 px 0.14 ms |
| Tiles shared through `Arc`, copied on first write | An undo step is the whole document "as it was", which costs only pointers; a stroke copies just the tiles it paints. One mechanism undoes everything, so there is no per-action undo code to get wrong. | History tests; 1.1 ms per step for 15 4K layers |
| Compositing uses Krita's own 8-bit arithmetic (rounding included) | So a picture looks the same in Recedo and Krita, and `.kra` files flatten identically. | Krita-made files flatten with 0 difference from Krita's own `mergedimage.png` |
| Wash painting mode by default (stroke coverage mask, redrawn from the pre-stroke tiles) | A stroke at 50 % stays at 50 % where it crosses itself, which is what Krita's Wash mode does and what artists expect from opacity; Build-up is the other option. | Brush tests (crossings exactly at the opacity) |
| A projection (flattened picture) kept up to date per changed tile; the GPU draws it with the view transform, mipmaps when zoomed out, sharp pixels from 200 % | Krita's approach; the display cost does not grow with the number of layers. | `recedo-render` GPU tests compare rendered pixels with the view math |
| `.kra` written in Krita 5.2's own layout (LZF tiles, planar BGRA), mimetype stored first, saved through a `.part` file renamed into place | Krita compatibility; a crash or full disk can never leave a half-written painting. | `scripts/krita-interop.sh` with the real Krita 5.2.2 (CI job) |
| Things Recedo cannot open yet are listed in plain words, never silently dropped; if nothing can be read, the flattened picture opens as one layer | The spec's "warnings for unsupported parts". | Tests with vector, filter, group, mask, 16-bit and CMYK cases |

## 6. App decisions (Phase 1)

| Decision | Why | Verified by |
|---|---|---|
| Our own docking shell instead of `egui_dock` / `egui_tiles` | Both require an older egui (0.36) than the one the app and the test app use (0.37); two egui versions cannot share one window. The shell is small: left and right stacks of tabbed panels, drag a tab to another stack or a new group, drag separators, float a panel as a window, close/reopen from the Window menu, layout saved in the settings. | UI tests (off-screen GPU rendering, screenshots) |
| The app is split into a portable part (editor, interface, renderer: `editor.rs`, `ui.rs`, `dock.rs`, `session.rs`) and a thin Windows shell (`win/`) | Everything except window messages runs in Linux tests: the editor with synthetic pen samples, the whole interface rendered off-screen and clicked through. The shell only turns messages into pen samples and egui input. | 32 app tests; the self-test runs the shell itself |
| Pen input paints only from the chosen tablet interface (Windows Ink or Wintab); the mouse messages Windows makes from the pen drive the interface only | Ink marks its mouse messages, so they are dropped for painting. Wintab drivers move the cursor with ordinary-looking mouse messages: the mouse is ignored for painting the whole time the driver says the pen is in range (as Krita does), so a driver click arriving before the pen's own packet can never start a stroke without pressure. | Self-test: "no duplicate stroke from the driver's mouse messages" |
| Space is tracked from its own key messages, not `GetKeyState` | Space typed into a text box must not pan, and Space must never press the focused button. (Under Wine `GetKeyState` also lost a held Space, which is how this came up.) | Self-test on Windows: Space+drag pans and does not paint |
| Vsync (Fifo) with one frame queued; each frame waits for its swap-chain image *before* reading input | Lowest latency without tearing or burning power: Mailbox would redraw as fast as the GPU allows during a stroke for no visible gain. Same choice as the Phase 0 test app (measured there). | Self-test: no frames while idle, frames paced while painting |
| File dialogs run outside the message handler | Windows' Open/Save dialogs run their own message loop and send messages back to the window; doing it inside the handler would re-enter the app. | Self-test saves/opens without dialogs; dialogs themselves are in the user checklist |
| Autosave to `%LOCALAPPDATA%\Recedo\recovery`, offered at the next start; not offered while another Recedo runs (a named mutex says so); written at once when Windows shuts down | Local (not roaming) because copies can be hundreds of MB. Another running Recedo's copies are in use, not left over from a crash. | Self-test (written, removed on a normal close); start-up offer seen under Wine; emergency copy: editor test |
| Escape (or a click outside) closes every dialog: Cancel for "Save changes?", "decide later" for recovery | What Windows users expect; nothing is lost by either. | Self-test on Windows |
| The projection keeps the layers below a stroke combined, per tile, while the stroke lasts | Only the stroke's layer changes; recombining 15 layers per tile per frame was most of the cost of big brushes. Each cached tile holds (not just points at) the tiles and settings it was made from, so it can never be used after any of them changed; exactness is tested against a full flatten after every kind of change, and each check was removed in turn to see a test fail. | Benchmark: 1000 px stroke picture update 10.2 → 5.2 ms per frame |
| A shared thread pool (`rayon`) for big dabs and picture updates | Starting threads for every dab cost more than the painting on Windows (thread creation is slow there). | Benchmark (Windows build under Wine): 200 px painting 8.1 → 2.6 ms per frame |
| The interface fits a 1080p screen at 150 %: always-visible scroll bars, fold-out sections (Pen, Stabilizer, colour numbers) | The Artist 12 is 1920×1080 on 11.6", which Windows often scales to 150 %, leaving 1280×720 points. | UI test renders that case |
| The fill tool finds its area in the visible picture by default (option: the selected layer only) and grows it by 1 px | Line art on one layer and colour on another is the usual way to work; the area must be bounded by lines the fill layer does not hold. Without the 1 px the soft edge of a line leaves a pale ring between line and fill. | Engine tests (exact area inside an outline, tolerance steps); editor test (fill inside a rectangle drawn with the tool) |
| Line, rectangle and ellipse outlines are painted by the normal brush engine at full pressure, as one undo step | They look exactly like the brush the user picked (presets, flat tips, opacity) and need no second drawing path. | Editor tests: painted where dragged, one step, Shift constraints |
| The gradient covers the whole layer: solid before its start, the layer untouched after its end | What Krita does without a selection (there are no selections yet). | Engine and editor tests |
| Dabs are spaced by the tip's width through its centre in the direction of travel (Krita's anisotropic spacing), not by its size | A flat tip spaced by its full size leaves a comb of separate marks when it moves along its thin side. Spaced this way a round tip is exactly as before. | Engine test: a 5 %-round tip in four directions, no gaps or stripes; the check fails with simpler formulas |
| Normal layers at full opacity copy their opaque pixels instead of blending them | Krita's "over" gives exactly the layer's pixel there, so the arithmetic was wasted; the picture update for a whole canvas took half the time afterwards (157-188 → 75-80 ms on one core here). | The `.kra` tests that compare with Krita's own pixels, and the projection-vs-flatten tests |
| Neighbouring changed tiles go to the GPU as one rectangle, and mip levels in runs of blocks | Thousands of small copies cost more than the pixels in them, above all in the browser. A whole-canvas change: about 200 → 105 ms of upload work on one core here. | GPU test: every mip level after full, partial and scattered updates (removing the last column of each run makes it fail) |
| Brush presets are settings of the one round brush, not a preset library | A real library (saved presets, tags, `.kpp`) is Phase 2; ten useful starting points cost nothing now and are easy to replace. | Browser test draws with them |
| Mipmaps of the canvas are computed on the CPU (whole rows at a time, blocks spread over the cores), only for the 64 px blocks under changed tiles, and uploaded level by level | The earlier GPU version drew each level from the level above inside the same texture. On the user's PC (browser version, WebGL) a lighter half-size copy of the strokes appeared, which is what a level drawn into the wrong place looks like; it could not be reproduced here (software GPUs). Plain uploads to each level avoid that GPU feature altogether. | GPU test: every level equals a CPU reference after a full and a partial update. Benchmark against the GPU version on this machine: big updates (undo of a 1000 px stroke) as fast, frames during a stroke faster. The user's PC: not yet confirmed |

## 7. The browser version (after Phase 1)

| Decision | Why | Verified by |
|---|---|---|
| A browser build of Recedo exists at all | Smart App Control on the user's PC blocks unsigned `.exe` files and cannot be turned off without an administrator; signing costs money and a reputation period. A web page is allowed. `recedo.exe` stays the main program: only it has Wintab. | Browser tests (see `FEATURES.md`) |
| The same modules compiled in (`#[path]` in `crates/recedo-web`), with a small web shell | No fork of the editor, interface or renderer: every fix and feature lands in both. Code only the desktop needs is marked as such. | Both builds lint and test from one source; CI lints the browser build |
| WebGL2 (wgpu's GL backend), not WebGPU | WebGL2 runs in every current browser on Windows; WebGPU is still missing on some PCs and browsers. | Browser tests (SwiftShader) |
| Pen input from pointer events, every coalesced sample; "touching" from pointerdown to pointerup, not from the pressure or buttons of each move | Some pen drivers report pressure 0 or no buttons on single samples in the middle of a stroke, which would break the line into dots; a sample without pressure takes the last pressure of the stroke. | Browser test with an injected pen |
| Fingers never paint: one pans, two pan and pinch-zoom | Palm and finger touches on a pen display must not leave marks. | Browser test with injected touches |
| Files open from a file picker and save as downloads; settings in the browser's local storage; the autosave copy in its IndexedDB, offered again after a crash or a closed tab | A web page cannot write files where it likes. | Browser test: autosave written while idle, restored after a reload |
| Hosted on the repository's GitHub Pages site under `/recedo/`, with its own offline worker; the Study app's worker leaves `/recedo/` alone | The repository has one Pages site, published from the Study app's branch. | Pages deploy runs; worker registers in the browser test |

## 8. More tools (asked for after Phase 1)

The user asked for many more real tools and a way to choose which ones sit on the side bar.
31 tools now; these are the choices behind them.

| Decision | Why | Verified by |
|---|---|---|
| The side bar is a list of tool ids in the settings (`hotbar`); unknown ids are dropped, repeats removed, an empty list falls back to the standard 15 | A settings file from another version, or a renamed tool, can never break the bar or the app. | Settings test |
| The "All tools" window takes no width from the window itself: the tool list is as wide as its grid, the bar list as wide as its rows | Content as wide as the space it is given makes a resizable egui window a little wider every frame (the first version filled the screen within seconds). | UI test: the window keeps its size over 20 frames and after a change (the old layout fails it) |
| The canvas overlay (brush outline, marching ants, previews) is painted on the panels' own layer, after them | On a layer of its own it was drawn over the tool window and floating panels. Nothing else paints the canvas area, so the overlay still shows over the painting. | UI test: a selection's outline changes no pixel inside the tool window |
| The selection is part of the document (a coverage mask, 0-255, shared like tiles) | It is undone and redone with everything else for free, and a crop or a new painting can never leave a selection that no longer fits. | Editor tests (selections undone and redone) |
| Every tool that changes pixels keeps its change inside the selection by putting back what was outside it afterwards (from the layer as it was), per changed tile | One rule for brush strokes, fills, shapes, gradients, text and retouching, instead of a mask in each of them. Soft selection edges blend. | Engine test (only inside changes), editor test (a stroke across a selection) |
| One retouch engine for ten tools: a soft round dab spaced at 10 % of the size, colours mixed with premultiplied alpha, the strength per dab scaled so one pass at full strength does about 75 % of the effect whatever the size | The tools then feel alike and respond to pressure the same way; premultiplied mixing keeps a smudge or blur into empty areas from turning dark. | Engine tests (12) |
| Blur and sharpen reach in proportion to the brush (a blur radius of size / 8, sharpen size / 32), using box blurs whose cost does not depend on the radius | The first version used a fixed 5×5 blur: on a 4K painting at 30 % zoom it did nothing visible. Sharpen compares with the colour around it weighted by opacity, so the edge of paint gets no bright halo. | Engine tests: a 120 px blur softens an edge 10 px out where a 30 px one does not; no halo on a grey square (both fail with the old code) |
| Text uses the interface's own two fonts (Ubuntu Light, Hack), read with `skrifa`, outlines filled with the same polygon filler as the lasso | No font files to ship or find, and the browser version draws exactly the same letters. Painted into the layer: text layers that stay editable are Phase 6. | Engine tests (letters where placed, holes kept, bold, both fonts, odd input) |
| While text is being written, the keyboard stays with its text box and letters are never shortcuts; picking another tool places the text | A click on the canvas (to move the text) took the keys away, and the next letter switched tools (B to the brush, and so on). Text left unplaced would stay on screen without being in the painting. | UI tests (each fails with the old code); self-test types "hi" and places it |
| Both shells tell the interface which of Ctrl, Shift and Alt are held (egui's `ModifiersChanged`) | egui 0.37 learns the held keys only from that event; without it Ctrl+Enter could not place text, in the browser or `recedo.exe`. | Self-test under Wine (Ctrl+Enter places text); UI test |
| G hides everything but the painting (Krita uses Tab); the gradient moved to Shift+G | The user asked for G. Tab moves the keyboard between fields in a browser page, so it would be a poor key there anyway. | UI test; browser test |
| Invert the selection is Ctrl+I, Ctrl+Shift+I kept as well; with nothing selected it says so | The user found Ctrl+Shift+I not working. In headless Chromium it did invert, so the likely causes were inverting with nothing selected (it did nothing and said nothing) or the browser opening its developer tools; Ctrl+I avoids the second. | UI test; editor test; browser test |
| A new canvas area size re-fits a painting that is still fitted to the window (the view unchanged since the fit), and otherwise keeps the middle in the middle | Opening the reference panel used to leave the right part of the painting under it; G left the painting off to one side. A painting the user zoomed or panned keeps its zoom. | Editor test; browser test |
| The text preview is laid out upright by egui, then its triangles are moved with the view's turn and mirror | egui draws text upright only; moving the finished triangles gives the exact view transform, mirror included, with no second text renderer. | UI test against the painted text |

## 9. The reference panel (browser version)

| Decision | Why | Verified by |
|---|---|---|
| Ordinary page code (`web/reference.js`) beside the canvas, not part of the egui interface | Pictures, articles and above all video players (YouTube's own) need the browser's page; egui cannot show a web page. The canvas element is narrowed while the panel is open, and the app follows the canvas's size every frame. | Browser test |
| Keyless public sources only, called straight from the browser (CORS), the same ones the Study app uses, plus the Internet Archive and Commons for videos | No account, key or server; nothing to pay or leak. There is no keyless way to search YouTube, so YouTube is by pasted link (the no-cookie player). | Unit checks of every source's answer format; browser test with stand-in answers |
| Nothing about a search is kept: no address-bar or storage entry, `cache: no-store`, no referrer, the offline worker ignores other sites, no form around the search box | The user asked that searches are not saved. | Browser test reads the address and storage afterwards |
| Keys typed in the panel never reach the painting's shortcuts; a press on the canvas takes the keyboard back | Otherwise B, E, S... in a search would change tools, and after watching a video the shortcuts would stay dead. | Browser test |
| No word filter on results (only Openverse's own `mature=false`) | The first version used the Study app's filter and also dropped Commons files whose category list had not arrived (Commons spreads those lists over several answers), which hid most results. Removed at the user's request. | Unit checks |

## 10. The Clip Studio-like interface (asked for after 1.0)

| Decision | Why | Verified by |
|---|---|---|
| Icons drawn with shapes on a 24 × 24 grid, in code | egui's fonts have no tool symbols, and shipping an icon font or images means licences and files to keep in step. Drawn icons follow the theme's text colour and stay sharp at any size. | UI test (each paints, none alike); sheet looked at |
| The tool strip is part of the window, not a dock panel; the old Tools panel became Sub tool | Clip Studio keeps its tool strip fixed at the edge. Keeping the panel's saved name means older settings still load. | UI tests |
| Layouts carry a version; an older one becomes the new default once | Otherwise everyone who used Recedo before would keep the old layout and never see this one. A layout changed by the user after that is kept. | Dock test |
| Stroke samples are painted by the real brush engine and kept as textures until the brush or colour changes | A drawn picture of a stroke would not show what the brush does (taper, softness, build-up). | Engine tests of the samples |
| Layer thumbnails average every painted pixel (every second one past 2 megapixels), only from the tiles that exist, at most every 0.4 s while painting, and ask for a redraw when they wait | Sampling a few points missed thin lines; redoing every frame while painting is too slow at 4K; without the redraw request the last update could be skipped until the mouse moved. | Engine test; UI test (fails without the request) |
| A second colour, swapped with X | Clip Studio shows two colours; X is Krita's key for the same. | UI and browser tests |
| The lasso transform's part floats until something else happens: Enter, another tool, any command (undo, save, another layer) puts it down as one undo step | Clip Studio and Krita work this way; tying it to the editor's "finish what the pen is doing" step means no command can act on a half-done transform, and Ctrl+Z right after scaling undoes the scaling. | Editor and UI tests |
| Scaling resamples in two passes (across, then down) with each new column's and row's weights worked once: bilinear when enlarging, an average of up to 16 samples per axis when shrinking, edge pixels continued past the copy's edge and partly covered pixels weighted | Bilinear alone drops thin lines when shrinking; the first version, sampling in 2-D for every new pixel and writing through the tile map pixel by pixel, took 398 ms for a 1500×1000 part at 1.5×; this one 91 ms. | Engine tests; timing |
| While dragging, the part's pixels are redone once per screen frame, not per pen sample | A tablet sends up to 200 samples a second; the box itself follows every one. | Editor test |

## 11. Speed and robustness work (after the interface)

| Decision | Why |
|---|---|
| Free egui's textures after `queue.submit`, not before | A texture dropped in the frame that still draws it was destroyed first and the submission failed (a crash). Found by the Wine self-test in a 1024x768 screen. |
| Browser: `preventDefault` on `webglcontextlost`, rebuild instance, device, renderer and egui's letter atlas on `webglcontextrestored` | Without it the browser never gives the context back. The painting lives on the CPU, so only GPU objects are rebuilt. The atlas is rebuilt by `set_font_providers(vec![])`, which only makes egui reload its fonts (the app adds no providers). |
| Mip levels: opaque rows read in place | Over a white background the picture is opaque, and premultiplying changes nothing there. 4K picture, level 0 + all mip levels, one core: 56.7 → 33.2 ms; the browser's layer-opacity drag 19.2 → 13.6 ms per frame (median). Exactness: `mip_blocks_equal_the_plain_formula`. A two-step (vertical, then horizontal) averaging loop was tried and was no faster, so it was not kept. |
| Self-test measures at whole screen pixels, empty spot 3 screen pixels from paint and edges | At 12.8 % zoom half a pixel is 4 canvas pixels; the old checks measured between pixels and at the painting's corner. The CI report (screen centre white, painted spot dark) showed the app drew correctly. |

## 12. The Stability slider (asked for after the interface)

| Decision | Why |
|---|---|
| A speed-adaptive filter (One Euro, Casiez et al. 2012, two stages) instead of the plain time average for the slider | The average smooths every speed alike, so a setting that steadies a slow line drags far behind a quick one. Measured on the same simulated hands, at equal or better wobble the adaptive filter trails a quick stroke a half to a third as far and keeps circles rounder; one stage trailed less but rounded corners more, so two. |
| Resample every millisecond and join readings with straight lines | Readings arrive in bursts when the app is busy; spreading them evenly in time (as Krita does) keeps the filter's timing right, and straight joins avoid steps. |
| Speeds measured on the screen | So the smoothing feels the same at any zoom. |
| Old settings move to the adaptive filter at the same number; Krita's average stays under "Stabilizer details" | The slider's number keeps its meaning; nothing the user chose is lost. |
| Hold to straighten only for nearly straight strokes | Snapping curves to a line would destroy work; the tolerance (6 px or 8 % of the length) lets a wobbly line through but not a curve. Pressure along the line follows the hand's, so the line keeps its taper. |
| Stability goes to 200: above 100 the cutoff halves by 200 (twice the smoothing time), 0-100 unchanged | Asked for: twice as stabilizing. Continuing the 0-100 curve would have made 200 sixty times as strong, not two; halving the cutoff doubles the filter's time constant at every speed, which halves the remaining shake (measured) and doubles the trailing, the honest cost. Saved settings keep their number. |
| A browser test with readings queued 5 ms apart, not waited for one by one | Waiting for each reading made the test pen send one every 123 ms, which no smoothing over time can work with; real pens send 200+ a second. |

## 13. Navigator and symmetry (asked for after the Stability slider)

| Decision | Why |
|---|---|
| The minimap is made from the projection (the combined picture already kept for the screen), shrunk by averaging | No second combining of the layers; thin lines still show. It is redone only when the picture changed, at most every 0.4 s while painting, so a stroke does not pay for it each frame. |
| The minimap follows `projection_rev`, the revision the projection was brought up to, not `pixels_rev` | The UI runs before the frame's refresh: keyed on `pixels_rev`, the map copied the old picture under the new number and never updated. The UI test caught it. |
| The clock is the system's local time: the browser's `Date` in the browser, `GetLocalTime` on Windows | "The time where you are" without asking for a time zone; it redraws on the minute, not every frame. |
| One brush painter paints every symmetry copy (one spacer per copy, one coverage and one copy of the tiles from before) | First each copy had its own painter. In Wash mode a painter redraws a dab's area from the layer as it was before the stroke; where copies met, one redrew from a tile taken before another copy had painted there, leaving white bands (seen in a browser test, then caught by a core test). Sharing them makes the copies one stroke: one undo step, and overlaps no darker than the stroke's opacity. |
| Radial symmetry: n copies turned round a centre; with X or Y, each mirrored too | The usual set in painting apps (Krita's multibrush, Procreate's drawing guides): spokes for mandalas, the kaleidoscope for snowflakes. The centre is stored as a share of the painting's size, so it stays in place when the painting is resized. |
| Flip in the Navigator is the existing mirror view (M), not a change to the pixels | Artists flip to check a drawing with fresh eyes; changing the pixels would be a different command (and is on the lasso transform already). |

## 14. The radial quick menu (R)

| Decision | Why |
|---|---|
| R opens it; rectangle select moves to Shift+R | The user asked for R. Selections are used less often than switching tools and colours. |
| Tap to keep it open, or hold, point and let go; letting go picks only if the pointer moved onto a slice | Both ways are standard (Blender's pie menus, Krita's pop-up palette): tapping suits a pen, holding is one motion. It first went by time (a quarter second); on the Windows CI's slow software renderer a frame takes about 200 ms, so a quick tap arrived as a "hold" and closed the menu (the self-test caught it). Movement cannot be fooled by a busy machine. |
| The tool bar's first eight tools, not a separate list | The bar is already the user's own choice and order (tool library); a second list to keep up would be clutter. Eight slices stay big enough to hit with a pen. |
| Recent colours, then basic ones | A ring that is never empty; the recent ones are what a painter goes back to. |
| A window-sized area on top while it is open | Every press goes to the menu, so a press outside closes it and never starts a stroke; the canvas needs no extra rule. |

## 15. Licences

Recedo is GPL-3.0-or-later, so code ported from Krita (GPL) can be used. Third-party
components and fonts are listed in [`THIRD_PARTY.md`](THIRD_PARTY.md).
