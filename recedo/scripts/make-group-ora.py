"""Writes groups.ora: layer groups for scripts/krita-interop.sh (Krita turns it into a .kra).

Under (green, opaque) at the bottom; group "Faded" at 50 % holding two overlapping opaque
squares; group "Shading" holding a multiply layer (only shades the layer inside its group,
since Krita groups are isolated); a plain group at full opacity.
"""
import struct, sys, zipfile, zlib

def png(w, h, px):
    raw = b"".join(b"\0" + bytes(c for x in range(w) for c in px(x, y)) for y in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")

W = H = 64
layers = {
    "under": png(W, H, lambda x, y: (0, 120 + y, 60 + x, 255)),
    "red": png(W, H, lambda x, y: (255, 0, 0, 255) if 4 <= x < 36 and 4 <= y < 36 else (0, 0, 0, 0)),
    "blue": png(W, H, lambda x, y: (0, 0, 255, 255) if 20 <= x < 52 and 20 <= y < 52 else (0, 0, 0, 0)),
    "base": png(W, H, lambda x, y: (240, 200, 40, 255) if y >= 40 else (0, 0, 0, 0)),
    "shade": png(W, H, lambda x, y: (100, 100, 200, 255) if x >= 32 else (0, 0, 0, 0)),
    "plain": png(W, H, lambda x, y: (250, 250, 250, 255) if 56 <= x and y < 8 else (0, 0, 0, 0)),
}
stack = f"""<?xml version="1.0" encoding="UTF-8"?>
<image w="{W}" h="{H}"><stack>
 <stack name="Plain" opacity="1.0" visibility="visible">
  <layer name="Plain layer" src="data/plain.png" x="0" y="0" opacity="1.0" visibility="visible"/>
 </stack>
 <stack name="Shading" opacity="1.0" visibility="visible">
  <layer name="Shade" src="data/shade.png" x="0" y="0" opacity="1.0" visibility="visible" composite-op="svg:multiply"/>
  <layer name="Base" src="data/base.png" x="0" y="0" opacity="1.0" visibility="visible"/>
 </stack>
 <stack name="Faded" opacity="0.5" visibility="visible">
  <layer name="Blue" src="data/blue.png" x="0" y="0" opacity="1.0" visibility="visible"/>
  <layer name="Red" src="data/red.png" x="0" y="0" opacity="1.0" visibility="visible"/>
 </stack>
 <layer name="Under" src="data/under.png" x="0" y="0" opacity="1.0" visibility="visible"/>
</stack></image>"""
with zipfile.ZipFile(sys.argv[1], "w") as z:
    z.writestr(zipfile.ZipInfo("mimetype"), "image/openraster")
    z.writestr("stack.xml", stack)
    for name, data in layers.items():
        z.writestr(f"data/{name}.png", data)
