#!/usr/bin/env python3
"""WCAG 2.2 contrast checker for the Rustic Launcher palettes.

Reads docs/tools/palettes.json, computes the contrast ratio of every pair the UI relies on and
prints a Markdown report. With --write it replaces the block between
`<!-- contrast:begin -->` and `<!-- contrast:end -->` in docs/PALETTES.md.
Exit code 1 when a required pair is below its minimum.

Translucent colours (rgba) are composited over the background they sit on before measuring.
Standard library only: `python docs/tools/contrast.py [--write]`.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PALETTES = HERE / "palettes.json"
DOC = HERE.parent / "PALETTES.md"

RGB = tuple[float, float, float]


def parse(color: str) -> tuple[RGB, float]:
    color = color.strip()
    if color.startswith("#"):
        h = color[1:]
        return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)), 1.0
    m = re.fullmatch(r"rgba\((\d+),(\d+),(\d+),([0-9.]+)\)", color.replace(" ", ""))
    if not m:
        raise ValueError(f"unsupported colour {color!r}")
    r, g, b, a = m.groups()
    return (int(r), int(g), int(b)), float(a)


def over(fg: str, bg: RGB) -> RGB:
    """Composite `fg` (maybe translucent) over the opaque `bg`."""
    (r, g, b), a = parse(fg)
    return (r * a + bg[0] * (1 - a), g * a + bg[1] * (1 - a), b * a + bg[2] * (1 - a))


def luminance(c: RGB) -> float:
    def ch(v: float) -> float:
        s = v / 255
        return s / 12.92 if s <= 0.04045 else ((s + 0.055) / 1.055) ** 2.4

    return 0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])


def ratio(a: RGB, b: RGB) -> float:
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def solid(tokens: dict[str, str], key: str, base: str = "bg.2") -> RGB:
    """A token as an opaque colour: translucent ones are composited over `base`."""
    base_rgb, _ = parse(tokens[base])
    return over(tokens[key], base_rgb)


def pairs(tokens: dict[str, str]) -> list[tuple[str, str, float, str]]:
    """(foreground, background, minimum, use). Backgrounds may be 'glass@bg.2' style composites."""
    bgs = ["bg.0", "bg.1", "bg.2", "bg.3", "bg.4", "surface.glass", "surface.glass-strong"]
    out: list[tuple[str, str, float, str]] = []
    for fg in ["text.primary", "text.secondary", "text.tertiary"]:
        for bg in bgs:
            out.append((fg, bg, 4.5, "body text"))
    for bg in ["bg.0", "bg.1", "bg.2", "bg.3", "surface.glass"]:
        out.append(("accent.text", bg, 4.5, "links, accent text"))
    for bg in ["accent", "accent.hover", "accent.active"]:
        out.append(("text.on-accent", bg, 4.5, "label on accent button"))
    for fg in ["success", "warning", "danger", "info"]:
        for bg in ["bg.2", "bg.3", "surface.glass"]:
            out.append((fg, bg, 4.5, "status text and icons"))
    # The ring is drawn with a 2 px offset, so it always sits on the page background, never on
    # the control itself (an accent button included).
    for bg in ["bg.0", "bg.1", "bg.2", "bg.3", "bg.4"]:
        out.append(("focus.ring", bg, 3.0, "focus indicator (1.4.11)"))
    for bg in ["bg.2", "bg.3"]:
        out.append(("border.strong", bg, 3.0, "input / control boundary (1.4.11)"))
    for bg in ["bg.0", "bg.2", "bg.3"]:
        out.append(("accent", bg, 3.0, "accent shapes, PLAY button (1.4.11)"))
    out.append(("text.disabled", "bg.2", 0.0, "disabled (exempt, informative)"))
    out.append(("border.default", "bg.2", 0.0, "decorative divider (informative)"))
    return out


def report() -> tuple[str, int]:
    data = json.loads(PALETTES.read_text(encoding="utf-8"))
    lines: list[str] = []
    failures = 0
    for pal in data["palettes"]:
        t = pal["tokens"]
        rows = []
        fails_here = 0
        worst = 99.0
        for fg, bg, minimum, use in pairs(t):
            bg_rgb = solid(t, bg)
            fg_rgb = over(t[fg], bg_rgb)
            r = ratio(fg_rgb, bg_rgb)
            ok = r >= minimum
            if minimum > 0:
                worst = min(worst, r / minimum)
            if not ok:
                fails_here += 1
            verdict = "info" if minimum == 0 else ("pass" if ok else "**FAIL**")
            rows.append(f"| `{fg}` | `{bg}` | {r:.2f}:1 | {minimum:g} | {verdict} | {use} |")
        failures += fails_here
        lines.append(f"#### {pal['name']} (`{pal['id']}`){' (default)' if pal.get('default') else ''}")
        lines.append("")
        lines.append(
            f"{len(rows)} pairs, {fails_here} failing. Translucent surfaces are composited over `bg.2`."
        )
        lines.append("")
        lines.append("| Foreground | Background | Ratio | Min | Result | Use |")
        lines.append("|---|---|---|---|---|---|")
        lines.extend(rows)
        lines.append("")
    summary = "All required pairs pass WCAG 2.2 AA." if failures == 0 else f"{failures} required pair(s) FAIL."
    lines.insert(0, f"Generated by `python docs/tools/contrast.py --write`. {summary}\n")
    return "\n".join(lines), failures


def main() -> int:
    text, failures = report()
    if "--write" in sys.argv:
        doc = DOC.read_text(encoding="utf-8")
        new = re.sub(
            r"(<!-- contrast:begin -->)(.*?)(<!-- contrast:end -->)",
            lambda m: f"{m.group(1)}\n{text}\n{m.group(3)}",
            doc,
            flags=re.S,
        )
        DOC.write_text(new, encoding="utf-8")
    else:
        print(text)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
