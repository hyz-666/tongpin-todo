"""Generate the Tauri app icons (solid brand-purple) without external deps.

Produces the icon set referenced by apps/windows/src-tauri/tauri.conf.json:
  32x32.png, 128x128.png, 128x128@2x.png (256), icon.ico
"""
import os
import struct
import zlib


def png_chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def solid_png(path: str, size: int, rgba: tuple) -> None:
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8-bit RGBA
    row = bytes([0]) + bytes(rgba) * size
    idat = zlib.compress(row * size)
    data = sig + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", idat) + png_chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(data)


def make_ico(path: str, png_path: str) -> None:
    with open(png_path, "rb") as f:
        png = f.read()
    header = struct.pack("<HHH", 0, 1, 1)  # reserved, type=icon, count=1
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 6 + 16)
    with open(path, "wb") as f:
        f.write(header + entry + png)


def main() -> None:
    icons = os.path.join("apps", "windows", "src-tauri", "icons")
    os.makedirs(icons, exist_ok=True)
    purple = (0x53, 0x4A, 0xB7, 0xFF)  # brand purple
    solid_png(os.path.join(icons, "32x32.png"), 32, purple)
    solid_png(os.path.join(icons, "128x128.png"), 128, purple)
    solid_png(os.path.join(icons, "128x128@2x.png"), 256, purple)
    make_ico(os.path.join(icons, "icon.ico"), os.path.join(icons, "128x128@2x.png"))
    print("icons generated in", icons)


if __name__ == "__main__":
    main()
