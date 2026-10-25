# Licences

**Recedo** is licensed GPL-3.0-or-later (personal use; GPL so that code from Krita, which is
GPL, may be studied and ported). Any code ported from Krita will be listed below with the
Krita file it came from. As of Phase 1, no Krita code has been copied; the Wintab context
setup and the tilt formula follow the publicly documented behaviour of Qt and Krita and
were written from scratch.

## Bundled in the Windows build

All Rust dependencies are permissively licensed (MIT, Apache-2.0, Zlib, BSD-2-Clause, ISC,
Unicode-3.0, or a choice among these), all compatible with the GPL. Run
`cargo metadata --filter-platform x86_64-pc-windows-gnu` for the full list. The main ones:

| Component | Licence |
|---|---|
| wgpu, naga (GPU) | MIT OR Apache-2.0 |
| egui, epaint, emath, ecolor, egui-wgpu (UI) | MIT OR Apache-2.0 |
| windows-sys (Win32 bindings) | MIT OR Apache-2.0 |
| raw-window-handle, bytemuck, pollster, serde, serde_json, png | MIT / Apache-2.0 / Zlib |
| rayon, rayon-core, crossbeam-deque/-epoch/-utils, either (thread pool for painting and compositing) | MIT OR Apache-2.0 |
| zip, flate2, miniz_oxide, quick-xml (Krita `.kra` files) | MIT / Apache-2.0 / Zlib |
| zopfli (part of zip's deflate support) | Apache-2.0 |
| jpeg-encoder (JPG export) | (MIT OR Apache-2.0) AND IJG, see below |
| jpeg-decoder (opening JPG) | MIT OR Apache-2.0 |
| self_cell (used by the UI's text shaping) | Apache-2.0 OR GPL-2.0-only, used under Apache-2.0 |

JPG export: this software is based in part on the work of the Independent JPEG Group
(the `jpeg-encoder` crate is derived from the IJG's libjpeg).

`.kra` files name Krita's sRGB profile (`sRGB-elle-V2-srgbtrc.icc`) but do not contain it;
Krita ships that profile itself.

### Fonts (from `epaint_default_fonts`, embedded in the .exe)

| Font | Licence |
|---|---|
| Ubuntu Light | Ubuntu Font Licence 1.0 |
| Hack (monospace) | MIT; based on Bitstream Vera (Bitstream Vera Licence) and DejaVu (public domain) |
| Noto Emoji | SIL Open Font Licence 1.1 |
| emoji-icon-font / egui-icons | MIT (John Slegers) |

## Not bundled

* `wintab-mock` (crate in this repository) is a test-only stand-in for `Wintab32.dll`; it is
  never shipped.
* `Wintab32.dll` itself belongs to the tablet driver (XP-Pen) and is loaded from the
  system at run time.
