"""The VibeCody brand mark, as SVG.

One mark for the whole product family: a bold tapered **V** with a gold
cursor block resting on its baseline. Each client gets the same silhouette in
its own accent hue, so the apps read as a family but stay apart in a dock or
app switcher.

Everything here is a pure function of a `Brand` plus a `Variant` -- no I/O.
`gen_icons.py` owns the rendering and the file writing.

Geometry lives in a 0..100 unit box that is scaled to whatever pixel size is
asked for, so the mark is resolution-independent by construction.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

# --------------------------------------------------------------------------
# palette -- mirrors vibecoder/design-system/tokens.css (dark theme)
# --------------------------------------------------------------------------

BG_TOP = "#151a26"
BG_BOTTOM = "#0a0c11"
#: Flat stand-in for the tile gradient, used when compositing away alpha.
BG_SOLID = (0x10, 0x14, 0x1D)

GOLD = "#f5c542"
GOLD_DEEP = "#f0b429"
GOLD_LIGHT = "#ffe9a3"
BLUE = "#6c8cff"


@dataclass(frozen=True)
class Brand:
    """One client's tint of the shared mark."""

    key: str
    label: str
    v_from: str      # top of the V gradient
    v_to: str        # bottom of the V gradient
    cursor: str      # the cursor block
    cursor_hi: str   # its highlight


def _tint(key: str, label: str, v_from: str, v_to: str) -> Brand:
    """Standard tint: coloured V, gold cursor block."""
    return Brand(key, label, v_from, v_to, GOLD_DEEP, GOLD_LIGHT)


#: The mark is identical everywhere; only the accent hue moves.
BRANDS: dict[str, Brand] = {
    b.key: b
    for b in (
        _tint("coder", "VibeCoder", "#7d99ff", "#8b5cf6"),   # blue -> violet
        _tint("app", "VibeCLI App", "#a78bfa", "#6366f1"),   # purple -> indigo
        _tint("desk", "VibeDesk", "#22d3ee", "#3b82f6"),     # cyan -> blue
        _tint("mobile", "VibeMobile", "#34d399", "#06b6d4"), # green -> cyan
        # The watch inverts the pairing: a gold V needs a cool cursor to keep
        # the two-colour signature legible.
        Brand("watch", "VibeCody Watch", "#f5c542", "#f97b22", BLUE, "#a9bcff"),
    )
}


class Variant(Enum):
    """What shape the artwork needs to take for a given platform."""

    TILE = "tile"          # full-bleed rounded square (Tauri, Windows, web)
    MACOS = "macos"        # 824/1024 rounded square + shadow, per Apple's grid
    SQUARE = "square"      # full-bleed, no rounding, opaque (iOS -- OS masks it)
    CIRCLE = "circle"      # full-bleed, content inside the circular safe zone
    ROUND = "round"        # circular artwork (Wear OS, Android round launcher)
    MASKABLE = "maskable"  # full-bleed, content inside the PWA maskable safe zone
    MARK = "mark"          # mark only, transparent background
    ADAPTIVE_FG = "adaptive_fg"  # Android adaptive foreground (66/108 safe zone)
    ADAPTIVE_BG = "adaptive_bg"  # Android adaptive background
    MONO = "mono"          # single-colour silhouette (Android themed icons)


# --------------------------------------------------------------------------
# geometry (0..100 box)
# --------------------------------------------------------------------------

_TOP = 19.0        # y of the two upper limb ends
_OUTER_X = 10.0    # x of the outer top corners
_INNER_X = 27.0    # x of the inner top corners
_VY_IN = 58.0      # y where the inner edges meet
_VY_OUT = 75.0     # y of the outer vertex -- the point of the V
_JOIN = 4.0        # stroke that rounds the corners and fattens the limbs

_CURSOR = (68.5, 59.0, 16.0, 16.0, 4.6)  # x, y, w, h, rx
_MARK_DY = 1.5     # nudge down: a top-heavy V optically sits high

#: Apple's macOS icon grid -- the rounded square is 824pt inside a 1024pt canvas.
_MACOS_SPAN = 824.0 / 1024.0
_MACOS_RADIUS = 185.4 / 824.0
#: Full-bleed tile corner radius, matching the iOS/macOS squircle proportion.
_TILE_RADIUS = 0.2197
#: Android adaptive icons show only the central 66 of 108 dp.
_ADAPTIVE_SAFE = 66.0 / 108.0
#: watchOS masks to a circle; keep the mark comfortably inside it.
_CIRCLE_SAFE = 0.82


def _v_path() -> str:
    """Outline of the V, traced outer-top-left -> inner -> outer vertex."""
    return (
        f"M {_OUTER_X:g},{_TOP:g} L {_INNER_X:g},{_TOP:g} L 50,{_VY_IN:g} "
        f"L {100 - _INNER_X:g},{_TOP:g} L {100 - _OUTER_X:g},{_TOP:g} "
        f"L 50,{_VY_OUT:g} Z"
    )


def _mark(brand: Brand, *, simplified: bool, mono: str | None = None) -> str:
    """The V (and, unless simplified, the cursor block) in the 0..100 box.

    Below ~32px the cursor block collapses into an indistinct smudge, so the
    small-size artwork drops it and lets the V carry the identity alone.
    """
    v_fill = mono or "url(#v)"
    cursor_fill = mono or "url(#c)"
    # With the block gone the V can grow into the space it was occupying.
    scale = 1.12 if simplified else 1.0
    origin = 50 * (1 - scale)

    parts = [
        f'<path d="{_v_path()}" fill="{v_fill}" stroke="{v_fill}" '
        f'stroke-width="{_JOIN:g}" stroke-linejoin="round" stroke-linecap="round"/>'
    ]
    if not simplified:
        x, y, w, h, rx = _CURSOR
        parts.append(
            f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}" '
            f'rx="{rx:g}" fill="{cursor_fill}"/>'
        )
    body = "\n      ".join(parts)
    return (
        f'<g transform="translate({origin:g},{origin + _MARK_DY * scale:g}) '
        f'scale({scale:g})">\n      {body}\n    </g>'
    )


def _defs(brand: Brand, *, shadow: bool) -> str:
    """Gradient (and optional shadow) definitions."""
    blocks = [
        f'<linearGradient id="v" x1="0.05" y1="0" x2="0.95" y2="1">'
        f'<stop offset="0" stop-color="{brand.v_from}"/>'
        f'<stop offset="0.55" stop-color="{brand.v_from}"/>'
        f'<stop offset="1" stop-color="{brand.v_to}"/></linearGradient>',
        f'<radialGradient id="c" cx="0.35" cy="0.28" r="0.85">'
        f'<stop offset="0" stop-color="{brand.cursor_hi}"/>'
        f'<stop offset="1" stop-color="{brand.cursor}"/></radialGradient>',
        f'<linearGradient id="bg" x1="0.15" y1="0" x2="0.85" y2="1">'
        f'<stop offset="0" stop-color="{BG_TOP}"/>'
        f'<stop offset="1" stop-color="{BG_BOTTOM}"/></linearGradient>',
    ]
    if shadow:
        # Blur a separate shape *behind* the icon rather than filtering the
        # artwork itself -- running the tile through a filter resamples its
        # gradient and leaves a visible diagonal seam.
        blocks.append(
            '<filter id="sh" x="-25%" y="-25%" width="150%" height="150%">'
            '<feGaussianBlur stdDeviation="10"/></filter>'
        )
    return "\n    ".join(blocks)


def render(brand: Brand, variant: Variant, size: int = 1024, *,
           simplified: bool = False) -> str:
    """Return the SVG source for one brand in one variant."""
    unit = size / 100.0
    defs = _defs(brand, shadow=variant is Variant.MACOS)

    def doc(body: str) -> str:
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" '
            f'height="{size}" viewBox="0 0 {size} {size}">\n  <defs>\n    '
            f"{defs}\n  </defs>\n  {body}\n</svg>\n"
        )

    def scaled_mark(factor: float = 1.0, mono: str | None = None) -> str:
        """Mark centred in the canvas, occupying `factor` of it."""
        s = unit * factor
        off = size * (1 - factor) / 2
        return (f'<g transform="translate({off:g},{off:g}) scale({s:g})">'
                f"{_mark(brand, simplified=simplified, mono=mono)}</g>")

    if variant is Variant.TILE:
        r = size * _TILE_RADIUS
        return doc(f'<rect width="{size}" height="{size}" rx="{r:g}" '
                   f'fill="url(#bg)"/>\n  {scaled_mark()}')

    if variant is Variant.SQUARE:
        return doc(f'<rect width="{size}" height="{size}" fill="url(#bg)"/>\n  '
                   f"{scaled_mark()}")

    if variant is Variant.CIRCLE:
        return doc(f'<rect width="{size}" height="{size}" fill="url(#bg)"/>\n  '
                   f"{scaled_mark(_CIRCLE_SAFE)}")

    if variant is Variant.ROUND:
        c = size / 2.0
        return doc(f'<circle cx="{c:g}" cy="{c:g}" r="{c:g}" fill="url(#bg)"/>\n  '
                   f"{scaled_mark(_CIRCLE_SAFE)}")

    if variant is Variant.MASKABLE:
        return doc(f'<rect width="{size}" height="{size}" fill="url(#bg)"/>\n  '
                   f"{scaled_mark(_ADAPTIVE_SAFE)}")

    if variant is Variant.MACOS:
        span = size * _MACOS_SPAN
        off = (size - span) / 2
        r = span * _MACOS_RADIUS
        drop = size * 0.014  # how far the shadow sits below the tile
        shadow = (f'<rect x="{off:g}" y="{off + drop:g}" width="{span:g}" '
                  f'height="{span:g}" rx="{r:g}" fill="#000" fill-opacity="0.5" '
                  f'filter="url(#sh)"/>')
        tile = (f'<g transform="translate({off:g},{off:g})">'
                f'<rect width="{span:g}" height="{span:g}" rx="{r:g}" fill="url(#bg)"/>'
                f'<g transform="scale({span / 100.0:g})">'
                f"{_mark(brand, simplified=simplified)}</g></g>")
        return doc(shadow + "\n  " + tile)

    if variant is Variant.MARK:
        return doc(scaled_mark())

    if variant is Variant.ADAPTIVE_FG:
        return doc(scaled_mark(_ADAPTIVE_SAFE))

    if variant is Variant.ADAPTIVE_BG:
        return doc(f'<rect width="{size}" height="{size}" fill="url(#bg)"/>')

    if variant is Variant.MONO:
        # Android tints this itself, so it must be a flat white silhouette.
        return doc(scaled_mark(_ADAPTIVE_SAFE, mono="#ffffff"))

    raise ValueError(f"unhandled variant {variant}")
