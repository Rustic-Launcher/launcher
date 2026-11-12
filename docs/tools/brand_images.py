"""Exports apps/web/public/icon-512.png and og-image.png from launcher/assets/brand.

Usage (from launcher/): python docs/tools/brand_images.py .  (needs Pillow; text in Inter, OFL)
"""
import sys
from PIL import Image, ImageDraw, ImageFont

root = sys.argv[1]
brand = f"{root}/assets/brand"
web = f"{root}/apps/web/public"
font_path = f"{root}/packages/ui/node_modules/@fontsource-variable/inter/files/inter-latin-wght-normal.woff2"

icon = Image.open(f"{brand}/rustic-app-icon-1024.png").convert("RGBA")
icon.resize((512, 512), Image.LANCZOS).save(f"{web}/icon-512.png", optimize=True)

W, H = 1200, 630
img = Image.new("RGBA", (W, H), (17, 18, 22, 255))  # #111216, the app-icon tile colour
# soft orange glow top-right
glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow)
for r in range(420, 0, -6):
    a = int(40 * (1 - r / 420) ** 1.6)
    gd.ellipse((W - 260 - r, -140 - r, W - 260 + r, -140 + r), fill=(255, 106, 0, a))
img = Image.alpha_composite(img, glow)

mark = Image.open(f"{brand}/rustic-mark-1024.png").convert("RGBA").resize((300, 300), Image.LANCZOS)
img.alpha_composite(mark, (110, 165))

def font(size, weight):
    f = ImageFont.truetype(font_path, size)
    f.set_variation_by_axes([weight])
    return f

d = ImageDraw.Draw(img)
d.text((460, 190), "Rustic", font=font(120, 800), fill=(255, 255, 255, 255))
d.text((464, 345), "A fast, social launcher for Horizon", font=font(40, 500), fill=(214, 216, 222, 255))
d.text((464, 400), "rustic-launcher.com", font=font(32, 600), fill=(255, 106, 0, 255))
d.text((110, 560), "Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.",
       font=font(20, 400), fill=(140, 143, 152, 255))
img.convert("RGB").save(f"{web}/og-image.png", optimize=True)
print("ok")
