#!/usr/bin/env python3
"""The harness's app icon, written at build time (SPEC-352 R19, ADR-363).

`python3 scripts/ios_icon.py write <path>` writes a 1024 by 1024 PNG of one opaque colour with no
text, and any other command line is refused with the usage line and exit 1. An upload is refused
when the app has no 1024-pixel icon or its icon has an alpha channel, and the image is never
committed (the scrubber refuses a committed binary), so the harness job and each lane's `app` job
run this into the asset catalog's icon set, which git ignores, before the project is generated.

The image is 8-bit RGB (colour type 2, so it has no alpha channel), every row filtered with filter
type 0 (none), and it holds no chunk but IHDR, IDAT and IEND, so no text and no transparency.
Standard library only.
"""

import sys
import zlib
from pathlib import Path

# The icon's side in pixels, and its one colour as 8-bit red, green and blue.
SIDE = 1024
COLOUR = bytes((35, 87, 137))
SIGNATURE = b"\x89PNG\r\n\x1a\n"


def chunk(kind, body):
    """One PNG chunk: the body's length, the type, the body, and the CRC-32 of the type and body."""
    return len(body).to_bytes(4, "big") + kind + body + zlib.crc32(kind + body).to_bytes(4, "big")


def icon():
    """The icon's bytes: the signature, the header, the rows (each a filter byte of 0 and the
    colour once per pixel) compressed, and the end."""
    header = SIDE.to_bytes(4, "big") * 2 + bytes((8, 2, 0, 0, 0))
    rows = (b"\x00" + COLOUR * SIDE) * SIDE
    return (
        SIGNATURE
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


def main(argv):
    """Write the icon to the path of `write <path>`; refuse any other command line."""
    if len(argv) != 2 or argv[0] != "write":
        sys.exit("usage: ios_icon.py write <path>")
    Path(argv[1]).write_bytes(icon())


if __name__ == "__main__":
    main(sys.argv[1:])
