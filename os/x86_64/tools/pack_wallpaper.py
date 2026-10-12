"""Pack unchanged source pixels into the framebuffer's bounded RGB565 asset."""
import pathlib
import struct
from PIL import Image

root = pathlib.Path(__file__).resolve().parents[1]
image = Image.open(root / "assets/ocean.png").convert("RGB")
assert 0 < image.width <= 1920 and 0 < image.height <= 1200
packed = bytearray()
for r, g, b in image.getdata():
    packed.extend(struct.pack("<H", ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)))
(root / "assets/ocean.rgb565").write_bytes(packed)
(root / "src/wallpaper_size.h").write_text(f"#define WALL_W {image.width}\n#define WALL_H {image.height}\n")
