#!/usr/bin/env python3
"""Regenerate every VibeCody app icon from the shared brand mark.

    make icons                 # everything
    make icons ARGS=--check    # fail if anything is out of date (CI)
    python3 scripts/brand/gen_icons.py --only coder,mobile

Run it after touching `brandkit.py`; the generated PNG/ICO/ICNS files are
committed, so the build never needs this script or `rsvg-convert`.

External tools: `rsvg-convert` (SVG rasteriser, `brew install librsvg`) and
`iconutil` (ships with macOS; needed only for `.icns`).
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import brandkit as bk  # noqa: E402
import pngkit  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
MASTERS = ROOT / "assets" / "brand"

#: Below this pixel size the cursor block turns to mush, so we drop it.
SIMPLIFY_AT = 40

_written: list[Path] = []
_stale: list[Path] = []


# --------------------------------------------------------------------------
# rasterising
# --------------------------------------------------------------------------

@dataclass
class Renderer:
    """Rasterises brand SVGs, and either writes or verifies the result."""

    tmp: Path
    check_only: bool = False

    def svg(self, brand: bk.Brand, variant: bk.Variant, size: int,
            simplified: bool | None = None) -> Path:
        """Write the SVG for one brand/variant/size to a scratch file."""
        if simplified is None:
            simplified = size < SIMPLIFY_AT
        source = bk.render(brand, variant, size, simplified=simplified)
        path = self.tmp / f"{brand.key}-{variant.value}-{size}-{int(simplified)}.svg"
        if not path.exists():
            path.write_text(source)
        return path

    def raster(self, brand: bk.Brand, variant: bk.Variant, size: int,
               simplified: bool | None = None) -> pngkit.Image:
        """Render to PNG bytes and decode to RGBA."""
        svg = self.svg(brand, variant, size, simplified)
        out = svg.with_suffix(".png")
        if not out.exists():
            subprocess.run(
                ["rsvg-convert", "-w", str(size), "-h", str(size),
                 "-o", str(out), str(svg)],
                check=True, capture_output=True,
            )
        return pngkit.decode(out.read_bytes())

    def emit(self, path: Path, data: bytes) -> None:
        """Write `data` to `path`, or record it as stale under --check."""
        if path.exists() and path.read_bytes() == data:
            return
        if self.check_only:
            _stale.append(path)
            return
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        _written.append(path)

    def png(self, path: Path, brand: bk.Brand, variant: bk.Variant, size: int,
            *, opaque: bool = False, simplified: bool | None = None) -> None:
        """Render one PNG. `opaque=True` strips the alpha channel (iOS)."""
        img = self.raster(brand, variant, size, simplified)
        if opaque:
            img = pngkit.flatten(img, bk.BG_SOLID)
        self.emit(path, pngkit.encode(img, rgb=opaque))

    def ico(self, path: Path, brand: bk.Brand,
            sizes: tuple[int, ...] = (16, 24, 32, 48, 64, 128, 256)) -> None:
        images = [self.raster(brand, bk.Variant.TILE, s) for s in sizes]
        self.emit(path, pngkit.build_ico(images))

    def icns(self, path: Path, brand: bk.Brand) -> None:
        """Build a .icns via iconutil, which needs a populated .iconset dir."""
        iconset = self.tmp / f"{brand.key}.iconset"
        iconset.mkdir(exist_ok=True)
        for base in (16, 32, 128, 256, 512):
            for scale, suffix in ((1, ""), (2, "@2x")):
                px = base * scale
                img = self.raster(brand, bk.Variant.MACOS, px)
                (iconset / f"icon_{base}x{base}{suffix}.png").write_bytes(
                    pngkit.encode(img))
        out = self.tmp / f"{brand.key}.icns"
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out)],
                       check=True, capture_output=True)
        self.emit(path, out.read_bytes())

    def text(self, path: Path, content: str) -> None:
        self.emit(path, content.encode())


# --------------------------------------------------------------------------
# per-client fan-out
# --------------------------------------------------------------------------

#: Windows Store tile assets that Tauri bundles alongside the .ico.
WINDOWS_TILES = {
    "Square30x30Logo.png": 30, "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71, "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107, "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150, "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310, "StoreLogo.png": 50,
}

#: Tauri desktop clients and the brand tint each one wears.
TAURI_APPS = {"vibecoder": "coder", "vibeapp": "app", "vibedesk": "desk"}


def tauri(r: Renderer, app_dir: str, brand: bk.Brand) -> None:
    icons = ROOT / app_dir / "src-tauri" / "icons"
    if not icons.is_dir():
        return
    for name, size in (("32x32.png", 32), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)):
        r.png(icons / name, brand, bk.Variant.TILE, size)
    for name, size in WINDOWS_TILES.items():
        r.png(icons / name, brand, bk.Variant.TILE, size)
    r.ico(icons / "icon.ico", brand)
    r.icns(icons / "icon.icns", brand)


#: iOS asset catalogue: filename -> pixel size, from Contents.json.
IOS_ICONS = {
    "Icon-App-20x20@1x.png": 20, "Icon-App-20x20@2x.png": 40,
    "Icon-App-20x20@3x.png": 60, "Icon-App-29x29@1x.png": 29,
    "Icon-App-29x29@2x.png": 58, "Icon-App-29x29@3x.png": 87,
    "Icon-App-40x40@1x.png": 40, "Icon-App-40x40@2x.png": 80,
    "Icon-App-40x40@3x.png": 120, "Icon-App-60x60@2x.png": 120,
    "Icon-App-60x60@3x.png": 180, "Icon-App-76x76@1x.png": 76,
    "Icon-App-76x76@2x.png": 152, "Icon-App-83.5x83.5@2x.png": 167,
    "Icon-App-1024x1024@1x.png": 1024,
}

#: Android density buckets: launcher px, then adaptive-layer px (108dp).
ANDROID_DENSITIES = {
    "mdpi": (48, 108), "hdpi": (72, 162), "xhdpi": (96, 216),
    "xxhdpi": (144, 324), "xxxhdpi": (192, 432),
}

ADAPTIVE_XML = """<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>
</adaptive-icon>
"""


def android(r: Renderer, res: Path, brand: bk.Brand) -> None:
    """Legacy launcher icons plus a full adaptive-icon set.

    Adaptive icons are what modern Android and Wear OS actually draw; without
    `mipmap-anydpi-v26` the launcher falls back to the legacy bitmap and
    cannot mask it to the device's icon shape.
    """
    for density, (launcher, layer) in ANDROID_DENSITIES.items():
        d = res / f"mipmap-{density}"
        r.png(d / "ic_launcher.png", brand, bk.Variant.TILE, launcher)
        r.png(d / "ic_launcher_round.png", brand, bk.Variant.ROUND, launcher)
        r.png(d / "ic_launcher_foreground.png", brand, bk.Variant.ADAPTIVE_FG, layer)
        r.png(d / "ic_launcher_background.png", brand, bk.Variant.ADAPTIVE_BG, layer)
        r.png(d / "ic_launcher_monochrome.png", brand, bk.Variant.MONO, layer)
    for name in ("ic_launcher.xml", "ic_launcher_round.xml"):
        r.text(res / "mipmap-anydpi-v26" / name, ADAPTIVE_XML)


def mobile(r: Renderer, brand: bk.Brand) -> None:
    """VibeMobile: iOS, Android, macOS, Windows and the PWA shell."""
    base = ROOT / "vibemobile"

    ios = base / "ios/Runner/Assets.xcassets/AppIcon.appiconset"
    for name, size in IOS_ICONS.items():
        # The App Store rejects iOS icons carrying an alpha channel.
        r.png(ios / name, brand, bk.Variant.SQUARE, size, opaque=True)

    android(r, base / "android/app/src/main/res", brand)

    mac = base / "macos/Runner/Assets.xcassets/AppIcon.appiconset"
    for size in (16, 32, 64, 128, 256, 512, 1024):
        r.png(mac / f"app_icon_{size}.png", brand, bk.Variant.MACOS, size)

    r.png(base / "web/favicon.png", brand, bk.Variant.TILE, 32)
    for size in (192, 512):
        r.png(base / f"web/icons/Icon-{size}.png", brand, bk.Variant.TILE, size)
        r.png(base / f"web/icons/Icon-maskable-{size}.png",
              brand, bk.Variant.MASKABLE, size)

    r.ico(base / "windows/runner/resources/app_icon.ico", brand)


def watches(r: Renderer) -> None:
    """watchOS marketing icon and the Wear OS launcher set."""
    brand = bk.BRANDS["watch"]

    # watchOS masks to a circle and, like iOS, forbids an alpha channel.
    watch_os = (ROOT / "vibewatch/VibeCodyWatch Watch App/Assets.xcassets"
                / "AppIcon.appiconset")
    if watch_os.is_dir():
        r.png(watch_os / "icon-1024.png", brand, bk.Variant.CIRCLE, 1024,
              opaque=True)

    wear = ROOT / "vibewatch/VibeCodyWear/app/src/main/res"
    if wear.is_dir():
        android(r, wear, brand)


def extras(r: Renderer) -> None:
    """Editor plugins, the desktop web shells and the docs site."""
    coder = bk.BRANDS["coder"]

    # VS Code marketplace listing icon (package.json gains "icon": "icon.png").
    r.png(ROOT / "vscode-extension/icon.png", coder, bk.Variant.TILE, 256)

    # JetBrains ships the mark as SVG; 16px is the tool-window size.
    jb = ROOT / "jetbrains-plugin/src/main/resources/icons/vibecli.svg"
    if jb.parent.is_dir():
        r.text(jb, bk.render(coder, bk.Variant.TILE, 16, simplified=True))

    # Tauri web shells: replace the leftover Vite default favicon. VibeApp and
    # VibeDesk had no public/ at all, so their /vite.svg reference was a 404.
    for app_dir, key in TAURI_APPS.items():
        if (ROOT / app_dir / "index.html").exists():
            r.text(ROOT / app_dir / "public" / "favicon.svg",
                   bk.render(bk.BRANDS[key], bk.Variant.TILE, 64))

    # Docs site (Jekyll).
    r.text(ROOT / "docs/favicon.svg", bk.render(coder, bk.Variant.TILE, 64))
    r.png(ROOT / "docs/apple-touch-icon.png", coder, bk.Variant.TILE, 180)


def masters(r: Renderer) -> None:
    """Committed SVG masters -- the human-readable source of every icon."""
    for brand in bk.BRANDS.values():
        r.text(MASTERS / f"vibecody-{brand.key}.svg",
               bk.render(brand, bk.Variant.TILE, 1024))
    r.text(MASTERS / "vibecody-mark.svg",
           bk.render(bk.BRANDS["coder"], bk.Variant.MARK, 1024))
    r.text(MASTERS / "vibecody-mono.svg",
           bk.render(bk.BRANDS["coder"], bk.Variant.MONO, 1024))


# --------------------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true",
                    help="exit non-zero if any icon is out of date")
    ap.add_argument("--only", default="",
                    help="comma-separated subset: coder,app,desk,mobile,watch,extras")
    args = ap.parse_args()

    if not shutil.which("rsvg-convert"):
        print("error: rsvg-convert not found (brew install librsvg)", file=sys.stderr)
        return 2

    only = {s.strip() for s in args.only.split(",") if s.strip()}
    want = lambda name: not only or name in only  # noqa: E731

    with tempfile.TemporaryDirectory(prefix="vibecody-icons-") as tmp:
        r = Renderer(Path(tmp), check_only=args.check)
        masters(r)
        for app_dir, key in TAURI_APPS.items():
            if want(key):
                tauri(r, app_dir, bk.BRANDS[key])
        if want("mobile"):
            mobile(r, bk.BRANDS["mobile"])
        if want("watch"):
            watches(r)
        if want("extras"):
            extras(r)

    if args.check:
        if _stale:
            print(f"{len(_stale)} icon(s) out of date; run `make icons`:",
                  file=sys.stderr)
            for p in _stale[:20]:
                print(f"  {p.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print("all icons up to date")
        return 0

    print(f"wrote {len(_written)} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
