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
| **Electron / web** | No Wintab. Excluded by the spec. |
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
| Mouse end to end | Self-test under Wine and on Windows CI | pass under Wine |
| Windows Ink end to end | Self-test on Windows CI: pen strokes injected with `InjectSyntheticPointerInput`, several frames per message; checks every frame arrives with exact pressure and tilt | see CI run |
| Drawing | GPU tests: pressure ramp gives thin→thick, no gaps on fast sparse samples, eraser end erases | pass |
| **Real XP-Pen Artist 12, both APIs** | **The user's checklist** | **waiting** |

Phase 1 does not start until the user confirms pressure and tilt through both APIs.

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

## 5. Licences

Recedo is GPL-3.0-or-later, so code ported from Krita (GPL) can be used. Third-party
components and fonts are listed in [`THIRD_PARTY.md`](THIRD_PARTY.md).
