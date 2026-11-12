"""Regenerates the companion's two images from Rustic's brand sources (run from this folder:
`python make_assets.py <launcher>/assets/brand <inter-latin-wght-normal.woff2>`):

- `rustic-wordmark.png`: the title screen's logo, the official mark (the polygons of
  `assets/brand/rustic-mark.svg`, copied as they are) beside "Rustic" set in Inter SemiBold
  (SIL OFL 1.1, the launcher's own UI font, `@fontsource-variable/inter`), white on transparent,
  256 px high. The game scales it down per GUI scale (`rusticmc_companion_core::title::logo`).
- `rustic-icon-256.png`: the window and task bar icon, `assets/brand/rustic-app-icon-1024.png`
  at 256 px.

Needs Pillow. Nothing of the game or of a third party's art is in either image.
"""
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

brand, font_path = Path(sys.argv[1]), sys.argv[2]
SS = 4  # supersampling
H = 256

svg = (brand / "rustic-mark.svg").read_text()
paths = re.findall(r'<path fill="(#[0-9A-Fa-f]{6})" d="([^"]+)"', svg)
polys = []
for fill, d in paths:
    pts = [tuple(map(float, p)) for p in re.findall(r"([\d.]+) ([\d.]+)", d)]
    polys.append((fill, pts))
xs = [x for _, pts in polys for x, _ in pts]
ys = [y for _, pts in polys for _, y in pts]
x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
scale = H * SS / (y1 - y0)
mark_w = int((x1 - x0) * scale) + 1

font = ImageFont.truetype(font_path, int(H * SS * 0.86))
font.set_variation_by_axes([600])
text = "Rustic"
bbox = font.getbbox(text)
gap = int(H * SS * 0.16)
width = mark_w + gap + (bbox[2] - bbox[0])
img = Image.new("RGBA", (width, H * SS), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)
for fill, pts in polys:
    rgb = tuple(int(fill[i : i + 2], 16) for i in (1, 3, 5))
    draw.polygon([((x - x0) * scale, (y - y0) * scale) for x, y in pts], fill=rgb + (255,))
# The text's baseline on the mark's bottom, its cap height a little under the mark's.
ascent, _ = font.getmetrics()
draw.text((mark_w + gap - bbox[0], H * SS - ascent), text, font=font, fill=(255, 255, 255, 255))
out = img.resize((width // SS, H), Image.LANCZOS)
out.save("rustic-wordmark.png", optimize=True)

icon = Image.open(brand / "rustic-app-icon-1024.png").convert("RGBA").resize((256, 256), Image.LANCZOS)
icon.save("rustic-icon-256.png", optimize=True)
print(out.size, icon.size)

# The menu's icons, Rustic's own simple shapes (no third party's set): white on transparent,
# 128 px each, side by side in ICONS order (rusticmc_companion_core::title::Glyph). The game
# tints and resamples them to the screen's pixel size.
ICONS = ["person", "people", "realms", "bag", "gear", "globe", "access", "power", "switch",
         "crown", "sparkle", "paw", "smile", "tag", "shirt"]
S = 512  # drawn at 4x, then reduced
W = (255, 255, 255, 255)


def stroke_circle(d, c, r, w):
    d.ellipse([c[0] - r, c[1] - r, c[0] + r, c[1] + r], outline=W, width=w)


def icon(name):
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    if name == "person":
        d.ellipse([186, 70, 326, 210], fill=W)
        d.chord([96, 250, 416, 570], 180, 360, fill=W)
    elif name == "people":
        d.ellipse([96, 96, 216, 216], fill=W)
        d.chord([36, 250, 276, 490], 180, 360, fill=W)
        d.ellipse([296, 96, 416, 216], fill=W)
        d.chord([236, 250, 476, 490], 180, 360, fill=W)
    elif name == "realms":
        # A tower with battlements.
        d.polygon([(96, 448), (96, 160), (156, 160), (156, 208), (226, 208), (226, 160),
                   (286, 160), (286, 208), (356, 208), (356, 160), (416, 160), (416, 448)], fill=W)
        d.rectangle([216, 330, 296, 448], fill=(0, 0, 0, 0))
        d.pieslice([216, 290, 296, 370], 180, 360, fill=(0, 0, 0, 0))
    elif name == "bag":
        d.rounded_rectangle([86, 170, 426, 456], radius=48, fill=W)
        d.arc([166, 50, 346, 290], 180, 360, fill=W, width=36)
        d.ellipse([168, 222, 200, 254], fill=(0, 0, 0, 0))
        d.ellipse([312, 222, 344, 254], fill=(0, 0, 0, 0))
    elif name == "gear":
        import math
        c = 256
        for k in range(8):
            a = k * math.pi / 4
            pts = []
            for dx, dy in [(-44, 120), (44, 120), (44, 205), (-44, 205)]:
                x = dx * math.cos(a) - dy * math.sin(a)
                y = dx * math.sin(a) + dy * math.cos(a)
                pts.append((c + x, c + y))
            d.polygon(pts, fill=W)
        d.ellipse([c - 150, c - 150, c + 150, c + 150], fill=W)
        d.ellipse([c - 62, c - 62, c + 62, c + 62], fill=(0, 0, 0, 0))
    elif name == "globe":
        stroke_circle(d, (256, 256), 196, 34)
        d.ellipse([176, 60, 336, 452], outline=W, width=30)
        d.line([(60, 256), (452, 256)], fill=W, width=30)
        d.line([(96, 160), (416, 160)], fill=W, width=26)
        d.line([(96, 352), (416, 352)], fill=W, width=26)
    elif name == "access":
        stroke_circle(d, (256, 256), 210, 30)
        d.ellipse([222, 96, 290, 164], fill=W)
        d.line([(130, 200), (382, 200)], fill=W, width=40)
        d.line([(256, 196), (256, 300)], fill=W, width=48)
        d.line([(256, 290), (196, 410)], fill=W, width=40)
        d.line([(256, 290), (316, 410)], fill=W, width=40)
    elif name == "power":
        d.arc([86, 96, 426, 436], 300, 240, fill=W, width=44)
        d.line([(256, 50), (256, 250)], fill=W, width=48)
        d.ellipse([232, 26, 280, 74], fill=W)
        d.ellipse([232, 226, 280, 274], fill=W)
    elif name == "switch":
        d.line([(100, 180), (380, 180)], fill=W, width=40)
        d.polygon([(360, 110), (450, 180), (360, 250)], fill=W)
        d.line([(132, 332), (412, 332)], fill=W, width=40)
        d.polygon([(152, 262), (62, 332), (152, 402)], fill=W)
    elif name == "crown":
        d.polygon([(76, 400), (60, 150), (176, 270), (256, 110), (336, 270), (452, 150),
                   (436, 400)], fill=W)
        d.rounded_rectangle([76, 410, 436, 456], radius=14, fill=W)
    elif name == "sparkle":
        def star(cx, cy, r, k):
            d.polygon([(cx, cy - r), (cx + r * k, cy - r * k), (cx + r, cy), (cx + r * k, cy + r * k),
                       (cx, cy + r), (cx - r * k, cy + r * k), (cx - r, cy), (cx - r * k, cy - r * k)],
                      fill=W)
        star(220, 290, 190, 0.22)
        star(396, 120, 80, 0.25)
    elif name == "paw":
        d.ellipse([146, 250, 366, 440], fill=W)
        for box in ([66, 170, 156, 280], [156, 70, 246, 190], [266, 70, 356, 190], [356, 170, 446, 280]):
            d.ellipse(box, fill=W)
    elif name == "smile":
        stroke_circle(d, (256, 256), 200, 34)
        d.ellipse([170, 170, 216, 230], fill=W)
        d.ellipse([296, 170, 342, 230], fill=W)
        d.arc([150, 190, 362, 380], 20, 160, fill=W, width=34)
    elif name == "tag":
        d.polygon([(60, 90), (260, 90), (450, 280), (280, 450), (60, 260)], fill=W)
        d.ellipse([130, 160, 190, 220], fill=(0, 0, 0, 0))
    elif name == "shirt":
        d.polygon([(176, 70), (210, 70), (256, 120), (302, 70), (336, 70), (456, 160), (400, 250),
                   (366, 226), (366, 450), (146, 450), (146, 226), (112, 250), (56, 160)], fill=W)
    return im.resize((128, 128), Image.LANCZOS)


strip = Image.new("RGBA", (128 * len(ICONS), 128), (0, 0, 0, 0))
for i, name in enumerate(ICONS):
    strip.alpha_composite(icon(name), (128 * i, 0))
strip.save("rustic-icons.png", optimize=True)
print(strip.size)
