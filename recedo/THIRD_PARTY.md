# Licences

**Recedo** is licensed GPL-3.0-or-later (personal use; GPL so that code from Krita, which is
GPL, may be studied and ported). Any code ported from Krita will be listed below with the
Krita file it came from. As of Phase 0, no Krita code has been copied; the Wintab context
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
