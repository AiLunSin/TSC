import struct
from pathlib import Path

SIZE = 32
OUT = Path(__file__).resolve().parents[1] / "app" / "src-tauri" / "icons" / "icon.ico"


def rgba_row(y: int) -> bytes:
    row = bytearray()
    for x in range(SIZE):
        t = (x + y) / (SIZE * 2 - 2)
        r = int(31 + (53 - 31) * t)
        g = int(94 + (194 - 94) * t)
        b = 255
        a = 255
        row += struct.pack("BBBB", b, g, r, a)
    return bytes(row)


xor_bitmap = b"".join(rgba_row(y) for y in range(SIZE - 1, -1, -1))
and_mask = b"\x00" * (4 * SIZE)

bitmap_info = struct.pack(
    "<IiiHHIIiiII",
    40,
    SIZE,
    SIZE,
    1,
    32,
    0,
    len(xor_bitmap) + len(and_mask),
    0,
    0,
    0,
    0,
)
image = bitmap_info + xor_bitmap + and_mask

icondir = struct.pack("<HHH", 0, 1, 1)
entry = struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, len(image), 6 + 16)

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_bytes(icondir + entry + image)
print(f"wrote {OUT} ({OUT.stat().st_size} bytes)")