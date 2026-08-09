# VibeCody brand assets

Every app icon in this repo is generated from one mark. Do not hand-edit the
PNG, ICO or ICNS files under `*/src-tauri/icons/`, `vibemobile/`, or
`vibewatch/` — they are build products and the next `make icons` overwrites
them.

```bash
make icons          # regenerate everything
make icons-check    # fail if any committed icon is stale (CI gate)

python3 scripts/brand/gen_icons.py --only coder,mobile   # a subset
```

Generated icons are **committed**, so an ordinary build needs neither this
pipeline nor `librsvg`. You only need the tools below to change the mark.

## The mark

A bold tapered **V** with a gold cursor block resting on its baseline —
the letter, plus the caret that says "this is where code happens".

One silhouette across all 13 clients; each wears its own accent hue so the
family reads as a set but the apps stay apart in a dock or app switcher.

| Client | Tint | Cursor |
|---|---|---|
| VibeCoder | blue → violet | gold |
| VibeCLI App | purple → indigo | gold |
| VibeDesk | cyan → blue | gold |
| VibeMobile | green → cyan | gold |
| VibeCody Watch / Wear | gold → orange | blue |

Hues come from `vibecoder/design-system/tokens.css`; change them there first,
then mirror them in `scripts/brand/brandkit.py`.

Below **40px** the artwork automatically drops the cursor block and grows the
V to fill the space — at 16px the block is an indistinct smudge, and a
smudge next to a letterform reads as a rendering bug.

## Files here

| File | What it is |
|---|---|
| `vibecody-<client>.svg` | 1024px full-bleed tile, one per client |
| `vibecody-mark.svg` | The mark alone, transparent background |
| `vibecody-mono.svg` | Flat white silhouette (Android themed icons) |

These are the human-readable masters. The generator re-derives them too, so
editing them by hand has no effect — change `brandkit.py`.

## Source

| File | Role |
|---|---|
| `scripts/brand/brandkit.py` | The mark itself: geometry, palette, per-platform variants. Pure functions, no I/O. |
| `scripts/brand/gen_icons.py` | Rasterises and fans out to every client. |
| `scripts/brand/pngkit.py` | Dependency-free PNG/ICO codec. |

Requires `rsvg-convert` (`brew install librsvg`) and, for `.icns`, macOS's
`iconutil`. No Pillow, no ImageMagick.

## Why the variants exist

Each platform masks and validates icons differently, so one PNG cannot serve
them all:

- **`SQUARE`** (iOS, watchOS) is full-bleed with **no alpha channel** — the
  App Store rejects icons that carry one, even when every pixel is opaque.
  The OS applies the rounded mask itself.
- **`MACOS`** follows Apple's grid: an 824pt rounded square inside a 1024pt
  canvas, with a drop shadow. The shadow is a separate blurred shape *behind*
  the tile; running the tile through a filter resamples its gradient and
  leaves a visible diagonal seam.
- **`ADAPTIVE_FG` / `ADAPTIVE_BG` / `MONO`** are Android's three icon layers.
  The launcher only ever shows the central 66 of 108dp, so the foreground is
  scaled to that safe zone. Without `mipmap-anydpi-v26/ic_launcher.xml` the
  launcher falls back to the legacy bitmap and cannot mask it to the device's
  icon shape.
- **`CIRCLE` / `ROUND`** keep content inside the circular mask that watchOS
  and Wear OS apply.
- **`MASKABLE`** is the PWA equivalent for `vibemobile/web/`.

## Adding a client

Add a `Brand` to `BRANDS` in `brandkit.py`, then a fan-out call in
`gen_icons.py`. Pick the variant that matches how the platform masks icons —
guessing here produces artwork that looks right in the repo and wrong on the
device.
