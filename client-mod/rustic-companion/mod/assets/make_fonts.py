"""Static Inter weights for the title menu's text (`rustic-companion` draws it anti-aliased with
ab_glyph): run `python make_fonts.py <@fontsource-variable/inter folder>`. Inter is SIL OFL 1.1
(The Inter Project Authors); its licence ships beside the fonts (`Inter-LICENSE.txt`). Needs
fontTools and brotli. Latin and Latin Extended only (the menu's French and English).
"""
import sys
from pathlib import Path

from fontTools.merge import Merger
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

src = Path(sys.argv[1]) / "files"
for weight, name in [(500, "Inter-Medium.ttf"), (600, "Inter-SemiBold.ttf")]:
    parts = []
    for subset in ["latin", "latin-ext"]:
        font = TTFont(src / f"inter-{subset}-wght-normal.woff2")
        static = instancer.instantiateVariableFont(font, {"wght": weight})
        static.flavor = None
        out = Path(f"_{subset}.ttf")
        static.save(out)
        parts.append(str(out))
    merged = Merger().merge(parts)
    merged.save(name)
    for p in parts:
        Path(p).unlink()
    print(name, Path(name).stat().st_size)
