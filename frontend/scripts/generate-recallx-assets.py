#!/usr/bin/env python3
from __future__ import annotations

import os
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
PUBLIC = ROOT / "public"
APP = ROOT / "src" / "app"
TMP = ROOT / ".asset-build" / "recallx"

ICON_SVG = ICONS / "recallx-icon.svg"
LOGO_SVG = PUBLIC / "recallx-logo.svg"


PNG_TARGETS = {
    "icon.png": 1024,
    "icon_16x16.png": 16,
    "icon_16x16@2x.png": 32,
    "icon_32x32.png": 32,
    "icon_32x32@2x.png": 64,
    "icon_128x128.png": 128,
    "icon_128x128@2x.png": 256,
    "icon_256x256.png": 256,
    "icon_256x256@2x.png": 512,
    "icon_512x512.png": 512,
    "icon_512x512@2x.png": 1024,
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}

PUBLIC_ICON_TARGETS = {
    "icon_128x128.png": 128,
    "icon_32x32@2x.png": 64,
    "logo-collapsed.png": 500,
}

ICONSET_TARGETS = {
    "icon_16x16.png": 16,
    "icon_16x16@2x.png": 32,
    "icon_32x32.png": 32,
    "icon_32x32@2x.png": 64,
    "icon_128x128.png": 128,
    "icon_128x128@2x.png": 256,
    "icon_256x256.png": 256,
    "icon_256x256@2x.png": 512,
    "icon_512x512.png": 512,
    "icon_512x512@2x.png": 1024,
}


def run(args: list[str]) -> None:
    subprocess.run(args, check=True)


def render_png(svg: Path, out: Path, width: int, height: int | None = None) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    run([
        "rsvg-convert",
        "-w",
        str(width),
        "-h",
        str(height or width),
        "-f",
        "png",
        "-o",
        str(out),
        str(svg),
    ])


def make_iconset(icns_path: Path) -> None:
    iconset = TMP / f"{icns_path.stem}.iconset"
    iconset.mkdir(parents=True, exist_ok=True)
    for name, size in ICONSET_TARGETS.items():
        render_png(ICON_SVG, iconset / name, size)
    run(["iconutil", "-c", "icns", str(iconset), "-o", str(icns_path)])


def make_ico(ico_path: Path, names: list[str]) -> None:
    entries: list[tuple[int, int, bytes]] = []
    for name in names:
        data = (TMP / name).read_bytes()
        size = int(name.split("x")[0])
        entries.append((size, size, data))

    header = struct.pack("<HHH", 0, 1, len(entries))
    directory = bytearray()
    offset = 6 + 16 * len(entries)
    payload = bytearray()
    for width, height, data in entries:
        directory += struct.pack(
            "<BBBBHHII",
            0 if width >= 256 else width,
            0 if height >= 256 else height,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        payload += data
        offset += len(data)
    ico_path.write_bytes(header + directory + payload)


def main() -> None:
    TMP.mkdir(parents=True, exist_ok=True)

    for filename, size in PNG_TARGETS.items():
        render_png(ICON_SVG, ICONS / filename, size)
        render_png(ICON_SVG, TMP / f"{size}x{size}.png", size)

    for filename, size in PUBLIC_ICON_TARGETS.items():
        render_png(ICON_SVG, PUBLIC / filename, size)

    render_png(LOGO_SVG, PUBLIC / "logo.png", 845, 295)

    make_iconset(ICONS / "app_icon.icns")
    make_iconset(ICONS / "icon.icns")
    make_ico(ICONS / "app_icon.ico", ["16x16.png", "32x32.png", "64x64.png", "128x128.png", "256x256.png"])
    make_ico(ICONS / "icon.ico", ["16x16.png", "32x32.png", "64x64.png", "128x128.png", "256x256.png"])
    make_ico(APP / "favicon.ico", ["16x16.png", "32x32.png", "64x64.png", "128x128.png"])

    # Leave no generated scratch output in the committed tree.
    for child in sorted(TMP.rglob("*"), reverse=True):
        if child.is_file():
            child.unlink()
        elif child.is_dir():
            child.rmdir()
    TMP.rmdir()
    if not any((ROOT / ".asset-build").iterdir()):
        (ROOT / ".asset-build").rmdir()

    print("RecallX assets regenerated.")


if __name__ == "__main__":
    main()
