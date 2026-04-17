"""Generate a monochrome-ready tray icon (orb silhouette with halo).

The macOS menu bar renders tray icons in template mode — it ignores the
RGB channels and uses alpha as a mask. So this image is all-white RGB
with a carefully shaped alpha: a solid disc in the middle, wrapped in a
visible alpha halo so it reads as a glowing orb even in monochrome.

Run: python3 scripts/make_tray_icon.py
"""
from PIL import Image, ImageFilter
import math
import os

SIZE = 88  # @2x of a ~22pt tray icon
OUT = "src-tauri/icons/tray.png"


def build():
    img = Image.new("RGBA", (SIZE, SIZE), (255, 255, 255, 0))
    px = img.load()
    cx = cy = SIZE / 2

    # Layout (fractions of icon size):
    CORE_R = SIZE * 0.22       # solid orb disc
    RING_CENTER = SIZE * 0.40  # ripple ring centerline
    RING_HALF = SIZE * 0.06    # ring half-width; a cosine bump around centerline
    RING_ALPHA = 200           # peak ring alpha

    for y in range(SIZE):
        for x in range(SIZE):
            dx = x - cx
            dy = y - cy
            r = math.sqrt(dx * dx + dy * dy)

            if r <= CORE_R:
                a = 255
            else:
                d = abs(r - RING_CENTER)
                if d <= RING_HALF:
                    t = d / RING_HALF
                    a = int(RING_ALPHA * (0.5 + 0.5 * math.cos(math.pi * t)))
                else:
                    a = 0

            px[x, y] = (255, 255, 255, a)

    # Light blur to soften both edges.
    img = img.filter(ImageFilter.GaussianBlur(1.0))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    img.save(OUT, "PNG")
    print(f"wrote {OUT}  ({SIZE}x{SIZE})")


if __name__ == "__main__":
    build()
