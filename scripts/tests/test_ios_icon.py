"""What an upload needs that a simulator build does not (SPEC-352 R19, A23 and A24; ADR-363).

An upload is refused when the app's property list has no launch screen or, for a build that runs on
an iPad, not all four orientations, and when the app has no 1024-pixel icon or its icon has an alpha
channel. The property list already holds both (A23's first half is MUTATION COVERAGE). The project
names the asset catalog among the target's sources and its icon set as the app icon, and a committed
standard-library script, `scripts/ios_icon.py write <path>`, writes the icon at build time, so no
image is ever committed (A24).

`scripts/ios_icon.py` is never imported: the test runs it with `sys.executable` as a child process,
writing into a temporary directory, and decodes what it wrote. The fields are read with
`int.from_bytes` and the image data with `zlib.decompress`, and the pixels are compared with the
colour the icon is known to have, never the compressed bytes.
"""

import json
import plistlib
import re
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

from _support import REPO, examined

ICON_SCRIPT = REPO / "scripts" / "ios_icon.py"
INFO_PLIST = Path("ios/Harness/Info.plist")
PROJECT = Path("ios/project.yml")
CATALOG = Path("ios/Harness/Assets.xcassets")
ICON_SET = CATALOG / "AppIcon.appiconset"
# The icon the build writes into the icon set, which git ignores and the scrubber refuses.
ICON = ICON_SET / "AppIcon.png"
IPAD_ORIENTATIONS = (
    "UIInterfaceOrientationPortrait",
    "UIInterfaceOrientationPortraitUpsideDown",
    "UIInterfaceOrientationLandscapeLeft",
    "UIInterfaceOrientationLandscapeRight",
)
# The icon's one colour, which the script writes and this test knows: 8-bit red, green and blue.
COLOUR = bytes((35, 87, 137))
SIDE = 1024
SIGNATURE = b"\x89PNG\r\n\x1a\n"
USAGE = "usage: ios_icon.py write <path>\n"
TARGETS = re.compile(r"(?ms)^targets:\n(.*?)(?=^\S|\Z)")


def target(text, name):
    """The lines of the target `name` in an XcodeGen spec, read under its `targets:` key by their
    indentation, or "" when the spec has no such target."""
    targets = TARGETS.search(text)
    found = targets and re.search(
        rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  \S|\Z)", targets.group(1)
    )
    return found.group(1) if found else ""


def section(lines, key):
    """The lines under a target's own key `key`, or "" when the target has none."""
    found = re.search(rf"(?ms)^    {re.escape(key)}:\n(.*?)(?=^    \S|\Z)", lines)
    return found.group(1) if found else ""


def crc32(data):
    """The CRC-32 a PNG chunk carries (ISO 3309, as the PNG specification defines it), computed bit
    by bit, so the test needs no second decoder to judge the script's."""
    crc = 0xFFFFFFFF
    for byte in data:
        crc ^= byte
        for _bit in range(8):
            crc = (crc >> 1) ^ (0xEDB88320 if crc & 1 else 0)
    return crc ^ 0xFFFFFFFF


def chunks(data):
    """(type, body, stored CRC) for each chunk after the signature, each read by its length field,
    so a chunk framed wrongly reads as a chunk of the wrong type."""
    found, at = [], len(SIGNATURE)
    while at < len(data):
        length = int.from_bytes(data[at : at + 4], "big")
        body = data[at + 8 : at + 8 + length]
        stored = int.from_bytes(data[at + 8 + length : at + 12 + length], "big")
        found.append((data[at + 4 : at + 8], body, stored))
        at += 12 + length
    return found


def write(scratch, *argv):
    """Run the icon script with `argv` in `scratch`, and its completed process."""
    return subprocess.run(
        [sys.executable, str(ICON_SCRIPT), *argv],
        cwd=scratch,
        capture_output=True,
        text=True,
        check=False,
    )


class WhatAnUploadNeeds(unittest.TestCase):
    def test_the_app_declares_what_an_upload_requires(self):
        # SPEC-352 A23 (R19). The property list's half is MUTATION COVERAGE: the plist already
        # holds the launch screen and the four orientations, and keeps them.
        plist = plistlib.loads((REPO / INFO_PLIST).read_bytes())
        self.assertIsInstance(plist.get("UILaunchScreen"), dict, "no launch screen")
        orientations = examined(
            "iPad orientations", plist.get("UISupportedInterfaceOrientations~ipad") or []
        )
        self.assertEqual(sorted(orientations), sorted(IPAD_ORIENTATIONS))
        # The project names the catalog among the target's sources and its icon set as the icon.
        harness = target((REPO / PROJECT).read_text(encoding="utf-8"), "Harness")
        settings = dict(
            examined(
                "base settings of the target Harness",
                re.findall(
                    r"(?m)^        ([A-Z][A-Z0-9_]*): (.*?)\s*$", section(harness, "settings")
                ),
            )
        )
        self.assertEqual(settings.get("ASSETCATALOG_COMPILER_APPICON_NAME"), "AppIcon")
        sources = examined(
            "sources of the target Harness",
            re.findall(r"(?m)^      - path: (\S+)\s*$", section(harness, "sources")),
        )
        self.assertIn(CATALOG.relative_to("ios").as_posix(), sources)
        # The icon set lists one 1024-pixel image, the one the build writes and git ignores.
        self.assertTrue((REPO / ICON_SET / "Contents.json").is_file(), "no icon set")
        self.assertTrue((REPO / CATALOG / "Contents.json").is_file(), "no asset catalog")
        images = examined(
            "icon set images",
            json.loads((REPO / ICON_SET / "Contents.json").read_text(encoding="utf-8"))["images"],
        )
        self.assertEqual(
            images,
            [{"filename": ICON.name, "idiom": "universal", "platform": "ios", "size": "1024x1024"}],
        )
        ignored = (REPO / ".gitignore").read_text(encoding="utf-8").splitlines()
        self.assertIn(ICON.as_posix(), ignored, "the generated icon is not ignored")

    def test_the_generated_icon_is_an_opaque_square_with_no_text(self):
        # SPEC-352 A24 (R19): the icon the script writes, decoded.
        with tempfile.TemporaryDirectory() as scratch:
            icon = Path(scratch) / "icon.png"
            run = write(scratch, "write", str(icon))
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertTrue(icon.is_file(), "the script wrote no icon")
            data = icon.read_bytes()
            # Any other command line is refused with the usage line, and writes nothing.
            refused = Path(scratch) / "refused.png"
            for argv in examined(
                "refused command lines",
                [(), ("write",), ("write", str(refused), str(refused)), ("draw", str(refused))],
            ):
                with self.subTest(argv=argv):
                    run = write(scratch, *argv)
                    self.assertEqual((run.returncode, run.stderr), (1, USAGE))
                    self.assertFalse(refused.exists(), "a refused command line wrote a file")
        self.assertEqual(data[: len(SIGNATURE)], SIGNATURE)
        parsed = examined("chunks", chunks(data))
        kinds = [kind for kind, _body, _stored in parsed]
        # No chunk but the header, the image data and the end: no text, and no transparency.
        self.assertEqual(kinds, [b"IHDR", *[b"IDAT"] * (len(kinds) - 2), b"IEND"])
        self.assertGreater(len(kinds), 2, "no image data")
        for kind, body, stored in parsed:
            self.assertEqual(stored, crc32(kind + body), f"{kind} carries a wrong CRC")
        self.assertEqual(parsed[-1][1], b"", "the end chunk carries a body")
        header = parsed[0][1]
        self.assertEqual(len(header), 13)
        self.assertEqual(int.from_bytes(header[0:4], "big"), SIDE, "width")
        self.assertEqual(int.from_bytes(header[4:8], "big"), SIDE, "height")
        # Bit depth 8; colour type 2, RGB with no alpha channel; deflate; adaptive; not interlaced.
        self.assertEqual(list(header[8:13]), [8, 2, 0, 0, 0])
        idat = b"".join(body for kind, body, _stored in parsed if kind == b"IDAT")
        raw = zlib.decompress(idat)
        stride = 1 + len(COLOUR) * SIDE
        self.assertEqual(len(raw), stride * SIDE, "the image data is not 1024 rows of 1024 pixels")
        rows = examined("rows", [raw[at : at + stride] for at in range(0, len(raw), stride)])
        self.assertEqual({row[0] for row in rows}, {0}, "a row is filtered")
        off = sum(row[1:] != COLOUR * SIDE for row in rows)
        self.assertEqual(off, 0, "a row holds a pixel that is not the icon's one opaque colour")


if __name__ == "__main__":
    unittest.main()
