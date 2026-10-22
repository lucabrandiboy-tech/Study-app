"""Copies a .kra, switching every layer group from pass-through to isolated (Krita's
default), for scripts/krita-interop.sh."""
import sys, zipfile

src, dst = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(src) as zin, zipfile.ZipFile(dst, "w") as zout:
    for info in zin.infolist():
        data = zin.read(info.filename)
        if info.filename == "maindoc.xml":
            data = data.replace(b'passthrough="1"', b'passthrough="0"')
        zout.writestr(info, data)
