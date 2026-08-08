"""Minimal, dependency-free PNG/ICO helpers for the VibeCody icon pipeline.

`rsvg-convert` renders SVG to 8-bit RGBA PNG, and `iconutil` turns an
`.iconset` into `.icns`. Neither can do the two remaining jobs:

  * strip the alpha channel  -- the App Store rejects iOS app icons that
    carry one, even when every pixel is fully opaque;
  * write a Windows `.ico`   -- needed by every Tauri bundle.

This module covers exactly those, so the pipeline needs no Pillow and no
ImageMagick. It deliberately supports only what `rsvg-convert` emits:
8-bit, non-interlaced, colour type 2 (RGB) or 6 (RGBA).
"""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass

PNG_SIG = b"\x89PNG\r\n\x1a\n"


class PngError(ValueError):
    """Raised when a PNG is outside the narrow subset this module handles."""


@dataclass(frozen=True)
class Image:
    """A decoded image: `pixels` is row-major RGBA, 4 bytes per pixel."""

    width: int
    height: int
    pixels: bytes

    def __post_init__(self) -> None:
        expected = self.width * self.height * 4
        if len(self.pixels) != expected:
            raise PngError(f"expected {expected} bytes of RGBA, got {len(self.pixels)}")

    @property
    def has_transparency(self) -> bool:
        """True if any pixel is not fully opaque."""
        return any(a != 0xFF for a in self.pixels[3::4])


# --------------------------------------------------------------------------
# decode
# --------------------------------------------------------------------------

def _chunks(data: bytes):
    if data[:8] != PNG_SIG:
        raise PngError("not a PNG")
    pos = 8
    while pos + 8 <= len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        yield kind, body
        pos += 12 + length  # length + type + body + crc


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def _unfilter(raw: bytes, width: int, height: int, channels: int) -> bytearray:
    """Reverse the per-scanline PNG filters, returning packed samples."""
    stride = width * channels
    out = bytearray(stride * height)
    prev = bytearray(stride)
    pos = 0
    for y in range(height):
        ftype = raw[pos]
        pos += 1
        line = bytearray(raw[pos : pos + stride])
        pos += stride
        if ftype == 1:  # Sub
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 0xFF
        elif ftype == 2:  # Up
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif ftype == 3:  # Average
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 0xFF
        elif ftype == 4:  # Paeth
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                upleft = prev[i - channels] if i >= channels else 0
                line[i] = (line[i] + _paeth(left, prev[i], upleft)) & 0xFF
        elif ftype != 0:
            raise PngError(f"unknown scanline filter {ftype}")
        out[y * stride : (y + 1) * stride] = line
        prev = line
    return out


def decode(data: bytes) -> Image:
    """Decode a PNG into RGBA. Only 8-bit non-interlaced RGB/RGBA."""
    header, idat = None, bytearray()
    for kind, body in _chunks(data):
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body[:13])
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    if header is None:
        raise PngError("missing IHDR")

    width, height, depth, colour, _compress, _filter, interlace = header
    if depth != 8:
        raise PngError(f"only 8-bit supported, got {depth}-bit")
    if interlace != 0:
        raise PngError("interlaced PNG not supported")
    if colour not in (2, 6):
        raise PngError(f"only colour type 2/6 supported, got {colour}")

    channels = 3 if colour == 2 else 4
    samples = _unfilter(zlib.decompress(bytes(idat)), width, height, channels)

    if channels == 4:
        return Image(width, height, bytes(samples))
    rgba = bytearray(width * height * 4)
    rgba[0::4] = samples[0::3]
    rgba[1::4] = samples[1::3]
    rgba[2::4] = samples[2::3]
    rgba[3::4] = b"\xff" * (width * height)
    return Image(width, height, bytes(rgba))


# --------------------------------------------------------------------------
# encode
# --------------------------------------------------------------------------

def _chunk(kind: bytes, body: bytes) -> bytes:
    return (
        struct.pack(">I", len(body))
        + kind
        + body
        + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
    )


def encode(img: Image, *, rgb: bool = False) -> bytes:
    """Encode to PNG. `rgb=True` drops the alpha channel entirely."""
    channels = 3 if rgb else 4
    stride = img.width * channels
    raw = bytearray()
    for y in range(img.height):
        row = img.pixels[y * img.width * 4 : (y + 1) * img.width * 4]
        raw.append(0)  # filter: None -- these are small, flat, gradient images
        if rgb:
            row = b"".join(row[i : i + 3] for i in range(0, len(row), 4))
        raw += row

    ihdr = struct.pack(">IIBBBBB", img.width, img.height, 8, 2 if rgb else 6, 0, 0, 0)
    return (
        PNG_SIG
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + _chunk(b"IEND", b"")
    )


def flatten(img: Image, background: tuple[int, int, int]) -> Image:
    """Composite over an opaque background, then force alpha to 255.

    iOS icons must not carry an alpha channel. Our square variant is already
    opaque, but compositing first means a stray transparent pixel becomes the
    brand background rather than black.
    """
    px = bytearray(img.pixels)
    br, bg, bb = background
    for i in range(0, len(px), 4):
        a = px[i + 3]
        if a != 0xFF:
            f = a / 255.0
            px[i] = round(px[i] * f + br * (1 - f))
            px[i + 1] = round(px[i + 1] * f + bg * (1 - f))
            px[i + 2] = round(px[i + 2] * f + bb * (1 - f))
            px[i + 3] = 0xFF
    return Image(img.width, img.height, bytes(px))


# --------------------------------------------------------------------------
# Windows .ico
# --------------------------------------------------------------------------

def _dib(img: Image) -> bytes:
    """A 32-bit BITMAPINFOHEADER DIB: bottom-up BGRA plus an AND mask.

    The AND mask is obsolete for 32-bit icons but Windows still expects the
    bytes to be there, so we emit a zeroed (fully opaque) one.
    """
    header = struct.pack(
        "<IiiHHIIiiII",
        40,               # biSize
        img.width,
        img.height * 2,   # biHeight covers XOR + AND mask
        1,                # biPlanes
        32,               # biBitCount
        0,                # biCompression = BI_RGB
        0,                # biSizeImage (may be 0 for BI_RGB)
        0, 0, 0, 0,       # resolution + palette fields
    )
    rows = []
    for y in range(img.height - 1, -1, -1):  # bottom-up
        row = img.pixels[y * img.width * 4 : (y + 1) * img.width * 4]
        rows.append(b"".join(
            bytes((row[i + 2], row[i + 1], row[i], row[i + 3]))
            for i in range(0, len(row), 4)
        ))
    mask_stride = ((img.width + 31) // 32) * 4  # 1bpp, padded to 4 bytes
    return header + b"".join(rows) + bytes(mask_stride * img.height)


def build_ico(images: list[Image]) -> bytes:
    """Pack images into an .ico. 256px entries use PNG, smaller ones use DIB.

    That split is what Windows itself does: PNG compression is only defined
    for the 256 entry, while older shell code paths still want a real DIB.
    """
    payloads = [
        encode(img) if img.width >= 256 else _dib(img)
        for img in images
    ]
    offset = 6 + 16 * len(images)
    out = [struct.pack("<HHH", 0, 1, len(images))]
    for img, payload in zip(images, payloads):
        out.append(struct.pack(
            "<BBBBHHII",
            img.width % 256,   # 256 is encoded as 0
            img.height % 256,
            0, 0, 1, 32,
            len(payload),
            offset,
        ))
        offset += len(payload)
    return b"".join(out + payloads)
