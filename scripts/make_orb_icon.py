"""Generate a glowing-orb app icon at 1024x1024 (transparent PNG).

Run: python3 scripts/make_orb_icon.py
Then: npm run tauri icon src-tauri/icons/icon.png
"""
from PIL import Image, ImageFilter
import math
import os

SIZE = 1024
CENTER = SIZE / 2
OUT = "src-tauri/icons/icon.png"

# Palette — violet → cyan through a bright core.
CORE = (235, 230, 255)       # near-white warm violet
MID = (150, 120, 255)        # violet
RIM = (90, 60, 220)          # deep indigo rim
HALO = (150, 120, 255)       # halo tint


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def draw_radial(size, colors, alpha_of_t, center=(0.5, 0.5), max_r_frac=0.5):
    """colors: list of (t, rgb) stops. alpha_of_t: function t->alpha 0..255.
    t is 0 at the named center, 1 at distance max_r_frac*size from center."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    px = img.load()
    cx = size * center[0]
    cy = size * center[1]
    max_r = size * max_r_frac
    for y in range(size):
        for x in range(size):
            dx = x - cx
            dy = y - cy
            r = math.sqrt(dx * dx + dy * dy)
            t = min(r / max_r, 1.0)
            for i in range(len(colors) - 1):
                t0, c0 = colors[i]
                t1, c1 = colors[i + 1]
                if t0 <= t <= t1:
                    local = (t - t0) / (t1 - t0) if t1 > t0 else 0
                    rgb = lerp(c0, c1, local)
                    break
            else:
                rgb = colors[-1][1]
            a = alpha_of_t(t)
            px[x, y] = (rgb[0], rgb[1], rgb[2], a)
    return img


def draw_sphere(size, body_colors, alpha_shape, light_center=(0.5, 0.5), light_r=0.5):
    """A sphere with gradient centered at `light_center` for the color,
    but alpha uses a circular mask centered at the geometric middle."""
    # color layer: radial from light_center
    color_img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    cpx = color_img.load()
    lx = size * light_center[0]
    ly = size * light_center[1]
    lr = size * light_r
    for y in range(size):
        for x in range(size):
            dx = x - lx
            dy = y - ly
            r = math.sqrt(dx * dx + dy * dy)
            t = min(r / lr, 1.0)
            for i in range(len(body_colors) - 1):
                t0, c0 = body_colors[i]
                t1, c1 = body_colors[i + 1]
                if t0 <= t <= t1:
                    local = (t - t0) / (t1 - t0) if t1 > t0 else 0
                    rgb = lerp(c0, c1, local)
                    break
            else:
                rgb = body_colors[-1][1]
            cpx[x, y] = (rgb[0], rgb[1], rgb[2], 255)

    # alpha shape: circular around geometric center
    alpha_img = draw_radial(
        size,
        [(0.0, (255, 255, 255)), (1.0, (255, 255, 255))],
        alpha_shape,
    )
    color_img.putalpha(alpha_img.split()[-1])
    return color_img


def smooth_falloff(start, end):
    """Cosine-ish soft falloff between two t values."""
    def f(t):
        if t <= start:
            return 255
        if t >= end:
            return 0
        x = (t - start) / (end - start)
        return int(255 * (0.5 + 0.5 * math.cos(math.pi * x)))
    return f


def build():
    # Outer halo: large, soft, only outside the sphere.
    halo_colors = [
        (0.0, HALO),
        (1.0, HALO),
    ]
    halo_alpha = smooth_falloff(0.30, 0.95)
    halo = draw_radial(SIZE, halo_colors, halo_alpha)
    halo = halo.filter(ImageFilter.GaussianBlur(18))

    # Core sphere: light source at upper-left. Indigo rim opposite.
    core = draw_sphere(
        SIZE,
        body_colors=[
            (0.00, CORE),
            (0.35, MID),
            (0.95, RIM),
        ],
        alpha_shape=smooth_falloff(0.38, 0.42),
        light_center=(0.40, 0.38),
        light_r=0.62,
    )

    # Composite halo + sphere. The off-center gradient on `core` already
    # acts as the highlight — no separate glint is needed.
    base = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    base = Image.alpha_composite(base, halo)
    base = Image.alpha_composite(base, core)

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    base.save(OUT, "PNG")
    print(f"wrote {OUT}  ({SIZE}x{SIZE})")


if __name__ == "__main__":
    build()
