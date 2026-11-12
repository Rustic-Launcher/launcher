#!/usr/bin/env python3
"""Generates the low-fidelity wireframes in docs/wireframes/*.svg (UX_SPEC.md references them).

Neutral greys, original layout, labels only: no third-party assets, logos or screenshots.
Standard library only: `python docs/tools/wireframes.py`.
"""
from __future__ import annotations

from html import escape
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "wireframes"
W, H = 1280, 800
# Step 1b (ADR 0028): frameless 44 px title bar, 64 px rail, no bottom bar, no docked friends panel.
RAIL, TOP, BOTTOM, FRIENDS = 64, 44, 0, 0
CX, CY = RAIL + 24, TOP + 20  # content origin (inside the rounded content panel)
CW = W - RAIL - 56  # content width
CH = H - TOP - 40

BG, PANEL, CARD, LINE, TEXT, MUTED, ACCENT = "#1b1b1b", "#232323", "#2e2e2e", "#4a4a4a", "#e6e6e6", "#9a9a9a", "#bdbdbd"


def rect(x, y, w, h, fill=CARD, stroke=LINE, r=12, dash=False):
    d = ' stroke-dasharray="6 5"' if dash else ""
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}"{d}/>'


def text(x, y, s, size=13, fill=TEXT, weight=400, anchor="start"):
    return (
        f'<text x="{x}" y="{y}" font-family="Inter, Segoe UI, sans-serif" font-size="{size}" '
        f'font-weight="{weight}" fill="{fill}" text-anchor="{anchor}">{escape(s)}</text>'
    )


def box(x, y, w, h, label="", sub="", fill=CARD, dash=False, r=12):
    out = [rect(x, y, w, h, fill=fill, r=r, dash=dash)]
    if label:
        out.append(text(x + 12, y + 22, label, 13, TEXT, 600))
    if sub:
        out.append(text(x + 12, y + 40, sub, 11, MUTED))
    return out


def btn(x, y, w, h, label, primary=False):
    fill = ACCENT if primary else CARD
    col = "#111" if primary else TEXT
    return [rect(x, y, w, h, fill=fill, r=12), text(x + w / 2, y + h / 2 + 5, label, 13, col, 600, "middle")]


def chip(x, y, label, w=None):
    w = w or (len(label) * 7 + 20)
    return [rect(x, y, w, 24, fill=PANEL, r=12), text(x + w / 2, y + 16, label, 11, MUTED, 500, "middle")], w


def chips(x, y, labels, gap=8):
    out = []
    for lab in labels:
        c, w = chip(x, y, lab)
        out += c
        x += w + gap
    return out


def tabs(x, y, labels, active=0):
    out = []
    for i, lab in enumerate(labels):
        w = len(lab) * 8 + 24
        out.append(text(x + w / 2, y + 18, lab, 13, TEXT if i == active else MUTED, 600 if i == active else 400, "middle"))
        if i == active:
            out.append(f'<rect x="{x + 6}" y="{y + 26}" width="{w - 12}" height="3" rx="1.5" fill="{ACCENT}"/>')
        x += w + 4
    out.append(f'<line x1="{CX}" y1="{y + 30}" x2="{CX + CW}" y2="{y + 30}" stroke="{LINE}"/>')
    return out


def shell(active: str, title: str, friends=True, play_label="PLAY", state="Ready"):
    """Step 1b chrome: title bar (product switcher, history, breadcrumb, search, bell, social, account pill,
    running pill, window controls), outline-icon rail with + and Settings, one rounded content panel.
    `friends`, `play_label` and `state` are kept for call compatibility (no bottom bar / docked panel)."""
    items = ["Launchpad", "Versions", "Explore", "Servers", "Locker", "Store"]
    out = [f'<rect width="{W}" height="{H}" fill="{BG}"/>']
    # title bar
    out.append(rect(14, 12, 20, 20, fill=CARD, r=6))
    out.append(text(24, 27, "R", 11, TEXT, 700, "middle"))
    out.append(text(42, 27, "Rustic ▾", 12, TEXT, 600))
    out.append(text(110, 27, "‹  ›", 12, MUTED))
    out.append(text(150, 27, title, 12, TEXT, 500))
    x = W - 470
    for lab in ("⌕", "!", "☺"):
        out.append(rect(x, 10, 26, 24, fill=CARD, r=6))
        out.append(text(x + 13, 27, lab, 11, MUTED, 600, "middle"))
        x += 32
    out.append(rect(x + 4, 10, 112, 24, fill=CARD, r=6))
    out.append(f'<circle cx="{x + 18}" cy="22" r="7" fill="{LINE}"/>')
    out.append(text(x + 30, 27, "Alex ▾", 11, TEXT, 600))
    x += 124
    out.append(rect(x, 10, 132, 24, fill=CARD, r=6))
    out.append(text(x + 66, 27, "0 instances running", 10, MUTED, 600, "middle"))
    out.append(text(W - 20, 27, "–  ▢  ✕", 12, MUTED, 600, "end"))
    # rail: outline icons, accent bar on the left edge, separator, + and Settings
    for i, it in enumerate(items):
        y = TOP + 18 + i * 52
        if it == active or (active == "Home" and it == "Launchpad") or (active == "Play" and it == "Versions") or (active == "Content" and it == "Explore"):
            out.append(f'<rect x="0" y="{y - 2}" width="3" height="28" rx="1.5" fill="{ACCENT}"/>')
        out.append(rect(RAIL / 2 - 10, y + 2, 20, 20, fill="none", r=5))
    sy = H - 128
    out.append(f'<line x1="{RAIL / 2 - 14}" y1="{sy}" x2="{RAIL / 2 + 14}" y2="{sy}" stroke="{LINE}"/>')
    out.append(text(RAIL / 2, sy + 40, "+", 18, MUTED, 500, "middle"))
    if active == "Settings":
        out.append(f'<rect x="0" y="{sy + 66}" width="3" height="28" rx="1.5" fill="{ACCENT}"/>')
    out.append(f'<circle cx="{RAIL / 2}" cy="{sy + 80}" r="9" fill="none" stroke="{MUTED}" stroke-width="2"/>')
    # content panel
    out.append(rect(RAIL, TOP, W - RAIL - 8, H - TOP - 8, fill=PANEL, r=16))
    return out


def page(name, elements, note=""):
    body = "\n".join(elements)
    if note:
        body += "\n" + text(16, H - 6, note, 10, MUTED)
    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
        f'role="img" aria-label="{escape(name)} wireframe">\n<title>{escape(name)}</title>\n{body}\n</svg>\n'
    )
    (OUT / f"{name}.svg").write_text(svg, encoding="utf-8")


def grid_cards(x, y, cols, rows, w, h, gap, labels, subs=None):
    out = []
    k = 0
    for r in range(rows):
        for c in range(cols):
            lab = labels[k % len(labels)]
            sub = subs[k % len(subs)] if subs else ""
            out += box(x + c * (w + gap), y + r * (h + gap), w, h, "", fill=CARD)
            out.append(rect(x + c * (w + gap) + 10, y + r * (h + gap) + 10, w - 20, h - 64, fill=PANEL, r=8))
            out.append(text(x + c * (w + gap) + 12, y + r * (h + gap) + h - 34, lab, 12, TEXT, 600))
            if sub:
                out.append(text(x + c * (w + gap) + 12, y + r * (h + gap) + h - 16, sub, 10, MUTED))
            k += 1
    return out


def home():
    e = shell("Home", "Launchpad")
    col = CW - 340
    e.append(text(CX, CY + 20, "Welcome back,  Alex ▾", 16, TEXT, 600))
    e.append(text(CX + col, CY + 20, "● 395 online", 12, MUTED, 500, "end"))
    e.append(rect(CX, CY + 36, col, 300, fill=CARD, r=16))
    e.append(rect(CX + 24, CY + 70, 150, 240, fill=PANEL, r=10, dash=True))
    e.append(text(CX + 99, CY + 195, "3D player", 11, MUTED, 500, "middle"))
    e += btn(CX + col / 2 - 150, CY + 150, 300, 64, "LAUNCH GAME ▾", primary=True)
    e.append(text(CX + col / 2, CY + 232, "Horizon 0.2.15 (Rustic edition) · caret = switch instance", 11, MUTED, 500, "middle"))
    e.append(rect(CX + col - 44, CY + 292, 32, 32, fill=PANEL, r=8))
    e.append(text(CX + col - 28, CY + 313, "⚙", 12, MUTED, 600, "middle"))
    y = CY + 360
    e.append(text(CX, y + 14, "Latest news (4 unread)", 15, TEXT, 600))
    e.append(text(CX + col, y + 14, "Explore sources ▾", 12, MUTED, 500, "end"))
    for i in range(2):
        x = CX + i * (col / 2 + 8)
        e += box(x, y + 28, col / 2 - 8, 200, "", fill=CARD)
        e.append(f'<circle cx="{x + 4}" cy="{y + 32}" r="6" fill="{ACCENT}"/>')
        e.append(text(x + 16, y + 206, ["Rustic Launcher 0.1 beta", "Horizon 0.2.15"][i], 14, TEXT, 700))
    ax = CX + col + 24
    e += box(ax, CY + 4, 316, 56, "", fill=CARD)
    for i in range(4):
        e.append(rect(ax + 20 + i * 74, CY + 14, 36, 36, fill=PANEL, r=8))
    e.append(text(ax, CY + 92, "Friends (5 online)", 14, TEXT, 600))
    e.append(text(ax + 316, CY + 92, "Open social ↗", 12, MUTED, 500, "end"))
    for i in range(6):
        e.append(rect(ax + i * 40, CY + 106, 32, 32, fill=CARD, r=8))
    for i, (n, st) in enumerate([("Léa", "Playing on Les Copains"), ("Noah", "Playing on Arena Nord"), ("Tom", "Hosting Tom's Modded")]):
        yy = CY + 160 + i * 48
        e.append(f'<circle cx="{ax + 16}" cy="{yy + 16}" r="14" fill="{LINE}"/>')
        e.append(text(ax + 40, yy + 12, n, 13, TEXT, 600))
        e.append(text(ax + 40, yy + 28, st, 11, MUTED))
        e += btn(ax + 284, yy + 2, 30, 28, "→")
    page("01-home", e, "Launchpad (1b): greeting + online pill, hero with centred two-line LAUNCH, news grid; friends column (home only).")

def instances():
    e = shell("Play", "Versions / Horizon")
    col = CW - 320
    e += box(CX, CY, col - 330, 40, "", r=12)
    e.append(text(CX + 14, CY + 25, "Horizon ▾  |  Search your Horizon profiles…", 12, MUTED))
    e += box(CX + col - 318, CY, 160, 40, "", r=12)
    e.append(text(CX + col - 304, CY + 25, "Last played ▾", 12, TEXT))
    e += btn(CX + col - 146, CY, 146, 40, "+ New profile")
    w = (col - 3 * 16) / 4
    names = ["Survie entre amis", "PvP Practice", "Horizon Essentials", "Creative Builds"]
    for r in range(2):
        for c in range(4):
            x = CX + c * (w + 16)
            y = CY + 60 + r * 300
            e.append(rect(x, y, w, w * 4 / 3, fill=CARD, r=12))
            e.append(text(x + w / 2, y + 28, "HORIZON 0.2.15", 10, MUTED, 700, "middle"))
            e.append(text(x, y + w * 4 / 3 + 22, names[(r * 4 + c) % 4], 13, TEXT, 600))
            e.append(text(x + w, y + w * 4 / 3 + 22, "▶", 12, ACCENT, 600, "end"))
    ax = CX + col + 20
    e.append(rect(ax, CY, 300, 168, fill=CARD, r=12))
    e.append(text(ax + 12, CY + 156, "Media carousel", 11, MUTED))
    e.append(text(ax, CY + 196, "Survie entre amis", 18, TEXT, 700))
    e.append(text(ax, CY + 218, "Horizon 0.2.14 (Rustic edition) with 14 mods…", 11, MUTED))
    e.append(text(ax, CY + 238, "Read more", 12, TEXT, 600))
    for i, (k, v) in enumerate([("Version", "Horizon 0.2.14 ▾"), ("Build", "Release"), ("Edition", "Rustic edition"), ("Content", "14 mods · 2 packs")]):
        yy = CY + 256 + i * 40
        e.append(rect(ax, yy, 300, 36, fill=CARD, r=8))
        e.append(text(ax + 12, yy + 23, k, 12, MUTED))
        e.append(text(ax + 288, yy + 23, v, 12, TEXT, 500, "end"))
    e += btn(ax, CY + 430, 300, 44, "LAUNCH GAME", primary=True)
    page("02-instances", e, "Versions (1b): fused source/search, sort, New profile; portrait covers with play icon; right detail panel. No loader concept.")

def content_browse():
    e = shell("Content", "Explore / Mods")
    col = CW - 320
    e += box(CX, CY, col - 180, 40, "", r=12)
    e.append(text(CX + 14, CY + 25, "Mods ▾  |  Search mods on Horizon, Modrinth, Rustic…", 12, MUTED))
    e += box(CX + col - 168, CY, 168, 40, "", r=12)
    e.append(text(CX + col - 154, CY + 25, "Relevance ▾", 12, TEXT))
    e.append(text(CX, CY + 70, "Installing into: Survie entre amis ▾", 12, MUTED))
    for i, (n, b) in enumerate([("Lumen Renderer", "Builds on your Horizon"), ("Atlas Minimap", "Builds on your Horizon"),
                                ("Clean HUD", "Builds on your Horizon"), ("Recipe Lens", "Compatible"), ("Tech Mod (Java)", "Incompatible: Java only")]):
        yy = CY + 90 + i * 100
        e.append(f'<line x1="{CX}" y1="{yy + 92}" x2="{CX + col}" y2="{yy + 92}" stroke="{LINE}"/>')
        e.append(rect(CX, yy + 12, 64, 64, fill=CARD, r=12))
        e.append(text(CX + 80, yy + 32, n, 15, TEXT, 600))
        e.append(text(CX + 80, yy + 52, "One-line description", 12, MUTED))
        c, _ = chip(CX + 80, yy + 60, "author · " + b)
        e += c
        e.append(text(CX + col, yy + 30, "↓ 1.8M · ◷ 3 days ago", 11, MUTED, 500, "end"))
        e += btn(CX + col - 196, yy + 42, 84, 32, "View")
        e += btn(CX + col - 104, yy + 42, 104, 32, "Install", primary=True)
    ax = CX + col + 20
    for i, lab in enumerate(["Horizon", "Modrinth", "Rustic"]):
        e += btn(ax + i * 102, CY, 96, 40, lab, primary=False)
    e.append(text(ax, CY + 72, "Categories", 13, TEXT, 600))
    for i in range(6):
        e.append(rect(ax + (i % 2) * 154, CY + 84 + (i // 2) * 40, 146, 32, fill=CARD, r=8))
    e.append(text(ax + 150, CY + 222, "▾ View 5 more", 11, MUTED, 500, "middle"))
    e.append(text(ax, CY + 256, "Compatibility", 13, TEXT, 600))
    e += box(ax, CY + 268, 300, 40, "Compatible with this instance  ●", r=8)
    e.append(text(ax, CY + 340, "Horizon versions", 13, TEXT, 600))
    e += chips(ax, CY + 352, ["0.2.15", "0.2.14", "0.2.13", "Nightly"])
    page("03-content-browse", e, "Explore (1b): type dropdown fused with search, rows with View/Install; right column: sources, categories, compatibility, versions.")

def content_detail():
    e = shell("Content", "Content › Minimap")
    e += box(CX, CY, CW, 140, "", fill=PANEL)
    e.append(rect(CX + 20, CY + 20, 100, 100, fill=CARD, r=14))
    e.append(text(CX + 140, CY + 52, "Minimap", 24, TEXT, 700))
    e.append(text(CX + 140, CY + 76, "by horizon-mods · MIT · Horizon registry", 12, MUTED))
    e += chips(CX + 140, CY + 90, ["Works", "Client only", "Hooks: hud", "v1.4.0"])
    e += btn(CX + CW - 180, CY + 48, 160, 44, "Install", primary=True)
    e += tabs(CX, CY + 156, ["Description", "Gallery", "Versions", "Compatibility", "Permissions"], 0)
    e += box(CX, CY + 200, CW * 2 // 3 - 8, 380, "Description (sanitised Markdown)")
    e += box(CX + CW * 2 // 3 + 8, CY + 200, CW // 3 - 8, 180, "Compatibility", "Rustic edition nightly-11-08: Works")
    e += box(CX + CW * 2 // 3 + 8, CY + 396, CW // 3 - 8, 184, "Details", "Downloads · Updated · Source · Report")
    page("04-content-detail", e, "Detail: header with badges and install target, tabs, compatibility explained, report link.")


def servers_list():
    e = shell("Servers", "Servers / Featured")
    e += box(CX, CY, CW - 520, 40, "", r=12)
    e.append(text(CX + 14, CY + 25, "Featured ▾  |  Search featured servers…", 12, MUTED))
    e += box(CX + CW - 508, CY, 160, 40, "", r=12)
    e.append(text(CX + CW - 494, CY + 25, "Recommended ▾", 12, TEXT))
    e += btn(CX + CW - 340, CY, 40, 40, "⟳")
    e += btn(CX + CW - 292, CY, 40, 40, "▤")
    e += box(CX + CW - 244, CY, 244, 40, "+ Create server", "", r=12)
    w = (CW - 32) / 3
    names = ["Les Copains SMP", "Arena Nord", "Builders Guild", "Skyreach", "Tom's Modded", "Minigames"]
    for k in range(6):
        x = CX + (k % 3) * (w + 16)
        y = CY + 60 + (k // 3) * 300
        e.append(rect(x, y, w, w * 9 / 16, fill=CARD, r=12))
        e.append(text(x + w / 2, y + w * 9 / 32 + 6, "banner art", 12, MUTED, 500, "middle"))
        e.append(rect(x, y + w * 9 / 16 + 14, 32, 32, fill=CARD, r=8))
        e.append(text(x + 44, y + w * 9 / 16 + 28, names[k], 14, TEXT, 600))
        e.append(text(x + 44, y + w * 9 / 16 + 44, "38 online", 11, MUTED))
        e.append(text(x + w, y + w * 9 / 16 + 36, "👁  ▶", 12, MUTED, 600, "end"))
    page("05-servers-list", e, "Servers (1b): view dropdown (Featured / My servers / Multiplayer list) fused with search, sort, ping, layout, Create server.")

def wizard_frame(step, title):
    e = shell("Servers", "Create a server", friends=False)
    e.append(f'<rect x="{RAIL}" y="{TOP}" width="{W - RAIL}" height="{H - TOP - BOTTOM}" fill="#000" opacity="0.55"/>')
    mx, my, mw, mh = 240, 90, 800, 600
    e.append(rect(mx, my, mw, mh, fill=PANEL, r=16))
    labels = ["1 Basics", "2 Where to host", "3 Access"]
    for i, lab in enumerate(labels):
        x = mx + 32 + i * 250
        on = i + 1 == step
        e.append(rect(x, my + 24, 230, 32, fill=ACCENT if on else CARD, r=16))
        e.append(text(x + 115, my + 45, lab, 12, "#111" if on else MUTED, 600, "middle"))
    e.append(text(mx + 32, my + 92, title, 20, TEXT, 700))
    e += btn(mx + mw - 300, my + mh - 64, 120, 40, "Back")
    e += btn(mx + mw - 168, my + mh - 64, 136, 40, "Create" if step == 3 else "Next", primary=True)
    return e, mx, my, mw, mh


def wizard1():
    e, mx, my, mw, mh = wizard_frame(1, "Basics")
    e += box(mx + 32, my + 112, 360, 56, "Server name", "Lou's SMP")
    e += box(mx + 32, my + 180, 175, 90, "Rustic Server", "Plugins (.rplugin) ✓")
    e += box(mx + 217, my + 180, 175, 90, "Vanilla Horizon", "No plugins")
    e += box(mx + 32, my + 282, 360, 56, "World", "New · seed (optional) · or import")
    e += box(mx + 32, my + 350, 360, 56, "Game mode · Difficulty", "Survival · Normal")
    e += box(mx + 408, my + 112, 360, 294, "Starter plugins", "essentials-lite · perms · chat-format (capabilities shown)")
    e += box(mx + 32, my + 418, 736, 56, "Version", "Rustic Server 0.1 on Horizon nightly-11-08 (same as your client)")
    page("06-wizard-1-basics", e, "Wizard step 1: name, type, world, mode, starter plugins with their declared capabilities.")


def wizard2():
    e, mx, my, mw, mh = wizard_frame(2, "Where to host")
    cards = [
        ("This PC (Local)", "Free · your PC must stay on", "RAM 4 GB ▾ · Start with launcher"),
        ("Your VPS (SSH)", "Your Linux server · we install it", "Host · user · key → preflight"),
        ("Rustic Cloud", "Coming soon", "Join the waitlist"),
    ]
    for i, (a, b, c) in enumerate(cards):
        x = mx + 32 + i * 248
        e += box(x, my + 120, 232, 250, a, b, dash=i == 2)
        e.append(text(x + 12, my + 340, c, 11, MUTED))
        if i == 0:
            e.append(f'<rect x="{x}" y="{my + 120}" width="232" height="250" rx="12" fill="none" stroke="{ACCENT}" stroke-width="2"/>')
    e += box(mx + 32, my + 390, 736, 80, "Requirements check", "Disk 3.1 GB free ✓ · RAM ✓ · Game files prepared ✓")
    page("07-wizard-2-where-to-host", e, "Wizard step 2: hosting mode cards; Rustic Cloud is a disabled 'coming soon' card with waitlist.")


def wizard3():
    e, mx, my, mw, mh = wizard_frame(3, "Access: how friends join")
    opts = [
        ("Friends on my network (LAN)", "192.168.1.20:25565 · always on"),
        ("Temporary address", "brave-otter-4821.play.rustic.example (while running)"),
        ("Invite code", "RST-K7QM-3XPD · paste in Rustic Launcher"),
        ("My own domain", "Connect Porkbun · Buy at Porkbun · Use any registrar"),
    ]
    for i, (a, b) in enumerate(opts):
        y = my + 112 + i * 76
        e += box(mx + 32, y, 736, 64, a, b)
        e.append(rect(mx + 736, y + 22, 20, 20, fill=ACCENT if i in (0, 2) else CARD, r=6))
    e += box(mx + 32, my + 420, 736, 44, "Online mode: on (only real owners can join) · Whitelist: on", "")
    page("08-wizard-3-access", e, "Wizard step 3: reachability options (several can be on); domain opens the domain flow (16-18).")


def server_manage():
    e = shell("Servers", "Servers › Lou's SMP")
    e += box(CX, CY, CW, 84, "Lou's SMP · Running · 4/20 players", "play.loucraft.net · Rustic Server 0.1 · Local · up 2 h")
    e += btn(CX + CW - 340, CY + 22, 100, 40, "Stop")
    e += btn(CX + CW - 228, CY + 22, 100, 40, "Restart")
    e += btn(CX + CW - 116, CY + 22, 100, 40, "Join", primary=True)
    e += tabs(CX, CY + 96, ["Overview", "Console", "Players", "Plugins", "Config", "Backups", "Access", "Danger zone"], 1)
    e.append(rect(CX, CY + 140, CW, 360, fill="#141414", r=12))
    for i in range(10):
        e.append(text(CX + 16, CY + 166 + i * 30, f"[12:0{i}] INFO server log line {i}", 12, MUTED))
    e += box(CX, CY + 512, CW, 44, "", r=10)
    e.append(text(CX + 12, CY + 539, "> type a command (tab completes)", 12, MUTED))
    page("09-server-manage", e, "Server page: status header, tabs; console streams /api/v1/events; plugins tab installs .rplugin.")


def locker():
    e = shell("Locker", "Locker")
    e.append(rect(CX, CY, 300, 560, fill=PANEL, r=16))
    e.append(text(CX + 150, CY + 280, "3D player (drag to turn)", 12, MUTED, 500, "middle"))
    e += btn(CX + 20, CY + 500, 120, 36, "Reset view")
    e += btn(CX + 160, CY + 500, 120, 36, "Emote ▶")
    x = CX + 316
    e += chips(x, CY, ["All", "Hats", "Back", "Torso", "Shoulder", "Name tags", "Emotes", "Companions", "Trails", "Auras"])
    e += grid_cards(x, CY + 40, 4, 2, (CW - 316 - 48) // 4, 150, 16, ["Copper Visor", "Lantern Pack", "Moth Pet", "Ember Trail", "Nothing", "Leaf Tag", "Wave", "Forge Aura"], ["Equipped", "Owned", "Owned", "Owned", "", "Owned", "Emote", "Owned"])
    e += box(x, CY + 380, CW - 316, 180, "Outfits", "Save current look · 3 saved looks (one click to wear) · Show my cosmetics ☑")
    page("10-locker", e, "Locker: 3D model left, category chips, owned items grid (hover = try on), outfits; no capes.")


def store():
    e = shell("Store", "Store")
    e += box(CX, CY, CW, 200, "", fill=PANEL)
    e.append(rect(CX + 16, CY + 16, 360, 168, fill=CARD, r=12, dash=True))
    e.append(text(CX + 196, CY + 105, "Featured set (rendered items)", 12, MUTED, 500, "middle"))
    e.append(text(CX + 400, CY + 60, "Forge Set · 4 items", 20, TEXT, 700))
    e.append(text(CX + 400, CY + 86, "€7.99 incl. VAT", 14, TEXT, 600))
    e += btn(CX + 400, CY + 112, 120, 40, "Buy", primary=True)
    e += btn(CX + 532, CY + 112, 120, 40, "Try on")
    e += btn(CX + 664, CY + 112, 120, 40, "Gift")
    e.append(text(CX, CY + 236, "Items", 15, TEXT, 600))
    e += grid_cards(CX, CY + 248, 5, 2, (CW - 64) // 5, 150, 16, ["Copper Visor", "Moth Pet", "Lantern Pack", "Ember Trail", "Leaf Tag"], ["€2.99", "€3.99", "€3.99", "Owned", "€1.49"])
    page("11-store", e, "Store: real prices incl. VAT, no timers, no 'limited', no loot boxes; Buy opens checkout in the browser.")


def settings():
    e = shell("Settings", "Settings")
    cats = ["Account & security", "Game & instances", "Appearance", "Accessibility", "Notifications", "Privacy & data", "Parental controls", "Connections (Porkbun, VPS)", "Downloads & storage", "About & legal"]
    e.append(rect(CX, CY, 220, 560, fill=PANEL, r=12))
    for i, c in enumerate(cats):
        if i == 5:
            e.append(rect(CX + 8, CY + 10 + i * 40, 204, 32, fill=CARD, r=8))
        e.append(text(CX + 20, CY + 31 + i * 40, c, 12, TEXT if i == 5 else MUTED, 600 if i == 5 else 400))
    x, w = CX + 236, CW - 236
    rows = [
        ("Usage analytics", "Off · helps us improve the launcher"),
        ("Performance reports", "Off"),
        ("Crash reports", "Off · asks each time when off"),
        ("Presence visibility", "Friends only ▾"),
        ("Let friends join my game", "On"),
        ("Export my data", "Download a JSON archive"),
        ("Delete my account", "30-day grace, then permanent"),
    ]
    for i, (a, b) in enumerate(rows):
        e += box(x, CY + i * 72, w, 60, a, b)
        e.append(rect(x + w - 60, CY + i * 72 + 18, 40, 24, fill=CARD, r=12))
    page("12-settings", e, "Settings › Privacy & data (shown); all telemetry off by default; parental controls in their own section.")


def first_launch():
    e = [f'<rect width="{W}" height="{H}" fill="{BG}"/>']
    e.append(rect(390, 120, 500, 560, fill=PANEL, r=16))
    e.append(text(640, 180, "Welcome to Rustic", 26, TEXT, 700, "middle"))
    e.append(text(640, 208, "Step 1 of 4 · Sign in with Microsoft", 13, MUTED, 500, "middle"))
    e += btn(450, 250, 380, 48, "Sign in with Microsoft (browser)", primary=True)
    e += btn(450, 310, 380, 40, "Use a code on another device")
    e += box(450, 370, 380, 90, "Why?", "You need to own Minecraft: Java Edition. Your password never touches Rustic.")
    e.append(text(640, 500, "Next: Rustic account (optional) · Privacy choices · Get the game ready", 11, MUTED, 500, "middle"))
    for i in range(4):
        e.append(f'<circle cx="{604 + i * 24}" cy="640" r="5" fill="{ACCENT if i == 0 else LINE}"/>')
    page("13-first-launch", e, "First launch: 4 steps (Microsoft sign-in, Rustic account or skip, privacy toggles all off, game preparation).")


def vps():
    e = shell("Servers", "Your VPS › Install", friends=False)
    e += box(CX, CY, 520, 300, "Connect your VPS", "")
    for i, (a, b) in enumerate([("Host", "203.0.113.10"), ("Port", "22"), ("User", "root or sudo user"), ("Key", "Generate a key for me (recommended) ▾")]):
        e += box(CX + 16, CY + 44 + i * 60, 488, 48, a, b)
    e += box(CX + 536, CY, CW + FRIENDS - 536, 300, "Host key fingerprint", "SHA256:9fT…Qa · first connection: compare with your provider · Trust")
    steps = ["Preflight: Ubuntu 24.04 x86-64 ✓ · 2 vCPU · 4 GB ✓", "Create user 'rustic' ✓", "Upload signed server files ✓", "Upload your game files (from this PC) 62%", "Install service + firewall (port 25565)", "Start and check"]
    e += box(CX, CY + 316, CW + FRIENDS - 32, 240, "Install progress", "")
    for i, s in enumerate(steps):
        e.append(text(CX + 16, CY + 370 + i * 30, ("✓ " if i < 3 else ("… " if i == 3 else "○ ")) + s, 12, TEXT if i <= 3 else MUTED))
    page("14-vps-install", e, "VPS journey: connect, verify host key (TOFU), preflight, install as unprivileged user, manage via SSH tunnel.")


def domain_porkbun():
    e = shell("Servers", "Access › Connect Porkbun", friends=False)
    e += box(CX, CY, 560, 560, "Connect your Porkbun account", "")
    e.append(text(CX + 16, CY + 56, "1. porkbun.com › Account › API Access: create a key", 12, TEXT))
    e.append(text(CX + 16, CY + 80, "2. Turn on 'API Access' for your domain", 12, TEXT))
    e += box(CX + 16, CY + 100, 528, 48, "API key", "pk1_••••")
    e += box(CX + 16, CY + 160, 528, 48, "Secret API key", "sk1_•••• (stored in your OS keychain)")
    e += btn(CX + 16, CY + 222, 160, 40, "Connect", primary=True)
    e += box(CX + 16, CY + 280, 528, 120, "Domain", "loucraft.net ▾ · subdomain: play")
    e += box(CX + 16, CY + 412, 528, 130, "Changes we will make (preview)", "CNAME play → r-7f3a.relay… · TXT _rustic-challenge.play")
    e += box(CX + 576, CY, CW + FRIENDS - 608, 560, "Other paths", "Buy a domain at Porkbun · Use a domain from any registrar")
    page("15-domain-connect-porkbun", e, "Connect Porkbun: keys go to the OS keychain; dry-run preview of records before applying.")


def domain_buy():
    e = shell("Servers", "Access › Buy a domain", friends=False)
    e += box(CX, CY, CW + FRIENDS - 32, 56, "", r=10)
    e.append(text(CX + 16, CY + 34, "loucraft", 16, TEXT, 600))
    rows = [("loucraft.net", "$11.08 first year · renews $12.52", "Available"), ("loucraft.com", "", "Taken"), ("loucraft.gg", "$54.20 · renews $54.20", "Available")]
    for i, (a, b, c) in enumerate(rows):
        e += box(CX, CY + 72 + i * 72, CW + FRIENDS - 32, 60, a, b)
        e += btn(CX + CW + FRIENDS - 180, CY + 82 + i * 72, 132, 40, "Select" if c == "Available" else "Taken", primary=i == 0)
    e += box(CX, CY + 300, CW + FRIENDS - 32, 250, "Confirm purchase at Porkbun", "")
    for i, s in enumerate([
        "Seller: Porkbun LLC, not Rustic. Their terms, prices, renewals and refunds apply.",
        "Paid with your Porkbun account credit or saved card: $11.08 now; renews yearly at $12.52 unless you turn off auto-renew.",
        "☐ I agree to Porkbun's registration agreement and terms (link)",
        "☐ I understand Rustic only sends this order to Porkbun on my behalf",
    ]):
        e.append(text(CX + 16, CY + 352 + i * 30, s, 12, TEXT if i > 1 else MUTED))
    e += btn(CX + CW + FRIENDS - 252, CY + 490, 204, 44, "Buy at Porkbun · $11.08", primary=True)
    page("16-domain-buy", e, "Buy: availability + first-year and renewal price, explicit consent, Porkbun as seller, then auto-configuration.")


def domain_byo():
    e = shell("Servers", "Access › Use my own domain", friends=False)
    e += box(CX, CY, CW + FRIENDS - 32, 56, "Your domain: play.loucraft.net", "Registrar: OVH ▾ (shows a short guide)")
    hdr = ["Type", "Name", "Value", "Status"]
    xs = [16, 110, 420, 820]
    e.append(rect(CX, CY + 72, CW + FRIENDS - 32, 240, fill=CARD, r=12))
    for h, x in zip(hdr, xs):
        e.append(text(CX + x, CY + 98, h, 11, MUTED, 600))
    recs = [("CNAME", "play", "r-7f3a.relay.rustic.example", "OK ✓"), ("TXT", "_rustic-challenge.play", "rustic-verify=Zt8…", "Propagating (1/3)"), ("SRV", "_minecraft._tcp.play (optional)", "0 5 25565 r-7f3a.relay…", "Not needed")]
    for i, r in enumerate(recs):
        y = CY + 130 + i * 56
        for v, x in zip(r, xs):
            e.append(text(CX + x, y, v, 12, TEXT))
        e += btn(CX + 740, y - 20, 64, 28, "Copy")
    e += box(CX, CY + 328, CW + FRIENDS - 32, 110, "Checking automatically every 10 s", "Missing / wrong value / propagating / OK per record · last check 3 s ago · Check now")
    e += box(CX, CY + 452, CW + FRIENDS - 32, 100, "When all are OK", "Rustic Cloud verifies the TXT record, then your address starts working. You can close this; we'll notify you.")
    page("17-domain-byo-verify", e, "Bring your own domain: exact records with copy buttons, per-registrar guide, live per-record status.")


def social_window():
    e = shell("Servers", "Servers / Featured")
    e.append(f'<rect x="0" y="0" width="{W}" height="{H}" fill="#000" opacity="0.45"/>')
    mx, my, mw, mh = 180, 70, 920, 640
    e.append(rect(mx, my, mw, mh, fill=PANEL, r=16))
    e.append(text(mx + 20, my + 28, "R  Social", 13, TEXT, 600))
    e.append(text(mx + mw - 20, my + 28, "☺ 5   –  ▢  ✕", 12, MUTED, 600, "end"))
    e += btn(mx + 20, my + 48, 136, 36, "Friends", primary=False)
    e += btn(mx + 160, my + 48, 136, 36, "Chats · 3")
    e += box(mx + 20, my + 96, 236, 36, "", r=10)
    e.append(text(mx + 32, my + 119, "Search friends…", 12, MUTED))
    e += btn(mx + 262, my + 96, 34, 36, "+")
    e += box(mx + 20, my + 144, 276, 56, "Party 2/8", "Leave")
    for i in range(6):
        yy = my + 216 + i * 48
        e.append(rect(mx + 20, yy, 32, 32, fill=CARD, r=8))
        e.append(rect(mx + 62, yy + 4, 120, 10, fill=CARD, r=5))
        e.append(rect(mx + 62, yy + 20, 80, 8, fill=CARD, r=4))
    e.append(rect(mx + 316, my + 48, mw - 336, mh - 68, fill=CARD, r=12))
    e.append(f'<circle cx="{mx + 316 + (mw - 336) / 2}" cy="{my + 280}" r="34" fill="{PANEL}"/>')
    e.append(text(mx + 316 + (mw - 336) / 2, my + 352, "Select a conversation", 18, TEXT, 600, "middle"))
    e.append(text(mx + 316 + (mw - 336) / 2, my + 378, "Choose a friend to start chatting.", 12, MUTED, 400, "middle"))
    page("18-social-window", e, "Social (1b): detachable window (overlay in the browser build): Friends / Chats tabs, search + add, party, conversation pane.")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for f in (home, instances, content_browse, content_detail, servers_list, wizard1, wizard2, wizard3,
              server_manage, locker, store, settings, first_launch, vps, domain_porkbun, domain_buy, domain_byo,
              social_window):
        f()
    print("wrote", len(list(OUT.glob("*.svg"))), "wireframes to", OUT)


if __name__ == "__main__":
    main()
