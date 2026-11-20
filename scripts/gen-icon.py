import struct
import zlib
from pathlib import Path

SIZE_ICO = 32
SIZE_PNG = 128
OUT_DIR = Path(__file__).resolve().parents[1] / "app" / "src-tauri" / "icons"


def pixel(x: int, y: int, size: int) -> tuple[int, int, int, int]:
    t = (x + y) / (size * 2 - 2)
    r = int(31 + (53 - 31) * t)
    g = int(94 + (194 - 94) * t)
    b = 255
    return r, g, b, 255


def write_ico(path: Path) -> None:
    xor_bitmap = bytearray()
    for y in range(SIZE_ICO - 1, -1, -1):
        for x in range(SIZE_ICO):
            r, g, b, a = pixel(x, y, SIZE_ICO)
            xor_bitmap += struct.pack("BBBB", b, g, r, a)

    and_mask = b"\x00" * (4 * SIZE_ICO)
    bitmap_info = struct.pack(
        "<IiiHHIIiiII",
        40,
        SIZE_ICO,
        SIZE_ICO,
        1,
        32,
        0,
        len(xor_bitmap) + len(and_mask),
        0,
        0,
        0,
        0,
    )
    image = bitmap_info + bytes(xor_bitmap) + and_mask
    icondir = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack(
        "<BBBBHHII", SIZE_ICO, SIZE_ICO, 0, 0, 1, 32, len(image), 6 + 16
    )
    path.write_bytes(icondir + entry + image)


def write_png(path: Path, size: int) -> None:
    raw = bytearray()
    for y in range(size):
        raw.append(0)
        for x in range(size):
            r, g, b, a = pixel(x, y, size)
            raw += bytes((r, g, b, a))

    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(png)


OUT_DIR.mkdir(parents=True, exist_ok=True)
write_ico(OUT_DIR / "icon.ico")
write_png(OUT_DIR / "icon.png", SIZE_PNG)
print(f"wrote {OUT_DIR / 'icon.ico'} and {OUT_DIR / 'icon.png'}")