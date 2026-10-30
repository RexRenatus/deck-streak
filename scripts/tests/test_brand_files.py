"""The DeckStreak logo's files are the brand source's own, byte for byte (SPEC-579 A1 to A12; ADR-598
D3 to D5).

`scripts/brand-files.json` lists each file the DeckStreak brand source gave the tree, by its path,
sha256, byte count, kind and pixel sizes. This census holds every listed file to its entry: present,
the same bytes, the kind its own header says, the pixel sizes its own header gives, and no text,
time or comment metadata (A1 to A6). It holds the README's header to its order, the title, then the
hero from a listed file, then the description paragraph (A7 to A9), and the app's project spec to
the catalog and the icon set it names (A10 to A12). The public scrub admits a binary only at a
listed path with the listed sha256 and size (SPEC-033 as SPEC-579 amends it); that half is
`test_public_scrub.py`'s.

The readers are written here from the formats' own rules, with the standard library alone: PNG
chunks by their length fields and CRCs, an icon file's directory and the PNG each entry holds, a
GIF's blocks to its trailer, an SVG's element and a JSON document. Each refusal is planted over a
complete set whose bytes this file builds, and that set reads green as the control.
"""

import hashlib
import json
import re
import struct
import tempfile
import unittest
import zlib
from pathlib import Path

from _support import REPO, examined

LIST = Path("scripts") / "brand-files.json"
README = Path("README.md")
PROJECT = Path("ios/app.yml")
CATALOG = Path("ios/App/Assets.xcassets")
ICON_SET = CATALOG / "AppIcon.appiconset"
APP_TARGET = "DeckStreak"
ICON_SETTING = "ASSETCATALOG_COMPILER_APPICON_NAME"
ICON_NAME = "AppIcon"
TITLE = "# DeckStreak"
HERO_ALT = "DeckStreak"
# SPEC-579 R1: every file the brand source gives the tree, in the list's order (sorted by path).
BRAND_PATHS = (
    "docs/assets/deckstreak-hero-tide-light.gif",
    "ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png",
    "ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-dark-1024.png",
    "ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-tinted-1024.png",
    "ios/App/Assets.xcassets/AppIcon.appiconset/Contents.json",
    "web/app/static/apple-touch-icon.png",
    "web/app/static/favicon.ico",
    "web/app/static/favicon.svg",
    "web/app/static/icon-192.png",
    "web/app/static/icon-512.png",
    "web/app/static/icon-maskable-512.png",
)
# SPEC-579 R5: the icon set's three icons with their appearances, as the brand source's icon set
# names them.
ICON_IMAGES = [
    {
        "filename": "AppIcon-1024.png",
        "idiom": "universal",
        "platform": "ios",
        "size": "1024x1024",
    },
    {
        "appearances": [{"appearance": "luminosity", "value": "dark"}],
        "filename": "AppIcon-dark-1024.png",
        "idiom": "universal",
        "platform": "ios",
        "size": "1024x1024",
    },
    {
        "appearances": [{"appearance": "luminosity", "value": "tinted"}],
        "filename": "AppIcon-tinted-1024.png",
        "idiom": "universal",
        "platform": "ios",
        "size": "1024x1024",
    },
]
SIGNATURE = b"\x89PNG\r\n\x1a\n"
# The PNG chunks that carry text or a time: refused in a brand file.
PNG_METADATA = (b"tEXt", b"zTXt", b"iTXt", b"tIME", b"eXIf")
# The one GIF application extension a looping animation needs.
GIF_LOOP = b"NETSCAPE2.0"
IMAGE_LINE = re.compile(r"!\[([^\]]*)\]\(([^)\s]*)\)")
TARGETS = re.compile(r"(?ms)^targets:\n(.*?)(?=^\S|\Z)")


def target(text, name):
    """The lines of the target `name` in an XcodeGen spec, read under its `targets:` key by their
    indentation, or "" when the spec has no such target (as `test_ios_icon.py` reads one)."""
    targets = TARGETS.search(text)
    found = targets and re.search(
        rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  \S|\Z)", targets.group(1)
    )
    return found.group(1) if found else ""


def section(lines, key):
    """The lines under a target's own key `key`, or "" when the target has none."""
    found = re.search(rf"(?ms)^    {re.escape(key)}:\n(.*?)(?=^    \S|\Z)", lines)
    return found.group(1) if found else ""


def png_chunks(data):
    """(type, body) for each chunk after the signature, each read by its length field and refused
    when its CRC does not hold or it runs past the end."""
    if not data.startswith(SIGNATURE):
        raise ValueError("no PNG signature")
    found, at = [], len(SIGNATURE)
    while at < len(data):
        if at + 12 > len(data):
            raise ValueError("a chunk runs past the end")
        length = int.from_bytes(data[at : at + 4], "big")
        end = at + 12 + length
        if end > len(data):
            raise ValueError("a chunk runs past the end")
        kind, body = data[at + 4 : at + 8], data[at + 8 : end - 4]
        if zlib.crc32(kind + body) != int.from_bytes(data[end - 4 : end], "big"):
            raise ValueError(f"the {kind.decode('latin-1')} chunk's CRC does not hold")
        found.append((kind, body))
        at = end
    return found


def read_png(data):
    """("png", [[width, height]], the metadata chunks found)."""
    chunks = png_chunks(data)
    kinds = [kind for kind, _body in chunks]
    if not chunks or kinds[0] != b"IHDR" or len(chunks[0][1]) != 13:
        raise ValueError("the first chunk is not a 13-byte IHDR")
    if kinds[-1] != b"IEND" or chunks[-1][1]:
        raise ValueError("the last chunk is not an empty IEND")
    if b"IDAT" not in kinds:
        raise ValueError("no image data chunk")
    header = chunks[0][1]
    width, height = int.from_bytes(header[0:4], "big"), int.from_bytes(header[4:8], "big")
    return (
        "png",
        [[width, height]],
        [kind.decode("ascii") for kind in kinds if kind in PNG_METADATA],
    )


def read_ico(data):
    """("ico", each entry's [width, height], the metadata found): the icon directory, and each
    entry a PNG the PNG reader reads, of the size its directory entry states."""
    if data[:4] != b"\x00\x00\x01\x00":
        raise ValueError("no icon file header")
    count = int.from_bytes(data[4:6], "little")
    if count == 0 or 6 + 16 * count > len(data):
        raise ValueError(f"an icon directory of {count} entries does not fit the file")
    pixels, metadata = [], []
    for index in range(count):
        entry = data[6 + 16 * index : 22 + 16 * index]
        size = int.from_bytes(entry[8:12], "little")
        offset = int.from_bytes(entry[12:16], "little")
        if offset + size > len(data):
            raise ValueError(f"entry {index} runs past the end")
        _kind, sizes, found = read_png(data[offset : offset + size])
        stated = [entry[0] or 256, entry[1] or 256]
        if stated != sizes[0]:
            raise ValueError(f"entry {index} states {stated} and holds {sizes[0]}")
        pixels += sizes
        metadata += found
    return "ico", pixels, metadata


def gif_blocks(data, at):
    """The data sub-blocks that start at `at`, and the offset after their terminator."""
    blocks = []
    while True:
        if at >= len(data):
            raise ValueError("a data block runs past the end")
        size = data[at]
        at += 1
        if size == 0:
            return blocks, at
        if at + size > len(data):
            raise ValueError("a data block runs past the end")
        blocks.append(data[at : at + size])
        at += size


def read_gif(data):
    """("gif", [[width, height]] of the logical screen, the metadata found): every block read to
    the trailer, which ends the file."""
    if data[:6] not in (b"GIF87a", b"GIF89a") or len(data) < 13:
        raise ValueError("no GIF signature and logical screen")
    width, height = int.from_bytes(data[6:8], "little"), int.from_bytes(data[8:10], "little")
    at = 13 + (3 * (2 << (data[10] & 7)) if data[10] & 0x80 else 0)
    metadata = []
    while True:
        if at >= len(data):
            raise ValueError("no trailer")
        block = data[at]
        if block == 0x3B:
            if at + 1 != len(data):
                raise ValueError("bytes follow the trailer")
            return "gif", [[width, height]], metadata
        if block == 0x21:
            if at + 2 > len(data):
                raise ValueError("an extension runs past the end")
            label = data[at + 1]
            blocks, at = gif_blocks(data, at + 2)
            if label == 0xFE:
                metadata.append("comment")
            elif label == 0x01:
                metadata.append("plain-text")
            elif label == 0xFF and (not blocks or blocks[0] != GIF_LOOP):
                name = blocks[0].decode("latin-1") if blocks else ""
                metadata.append(f"application {name}")
        elif block == 0x2C:
            if at + 11 > len(data):
                raise ValueError("an image descriptor runs past the end")
            local = data[at + 9]
            at += 10 + (3 * (2 << (local & 7)) if local & 0x80 else 0) + 1
            _blocks, at = gif_blocks(data, at)
        else:
            raise ValueError(f"an unknown block 0x{block:02x}")


def read_svg(data):
    """("svg", None, []) for UTF-8 text that holds an `<svg` element."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        raise ValueError("not UTF-8 text") from None
    if re.search(r"<svg[\s/>]", text) is None:
        raise ValueError("no <svg element")
    return "svg", None, []


def read_json(data):
    """("json", None, []) for a JSON document."""
    try:
        json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, ValueError):
        raise ValueError("not a JSON document") from None
    return "json", None, []


READERS = {"png": read_png, "ico": read_ico, "gif": read_gif, "svg": read_svg, "json": read_json}


def read_list(root):
    """(the list's `files`, None) at `root`, or (None, a finding) when it is absent or unreadable."""
    path = Path(root) / LIST
    if not path.is_file():
        return None, f"missing: {LIST.as_posix()}: no list at the root"
    try:
        files = json.loads(path.read_text(encoding="utf-8"))["files"]
        if not isinstance(files, list) or not all(isinstance(entry, dict) for entry in files):
            raise TypeError("its files are not a list of entries")
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as error:
        return None, f"kind: {LIST.as_posix()}: the list cannot be read: {error!r}"
    return files, None


def file_findings(root):
    """Every way a listed file differs from its entry, as "<reason>: <path>: <detail>"."""
    files, problem = read_list(root)
    if problem:
        return [problem]
    findings = []
    for entry in files:
        rel = entry.get("path", "")
        path = Path(root) / rel
        if not rel or not path.is_file():
            findings.append(f"missing: {rel}: not in the tree")
            continue
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != entry.get("sha256"):
            findings.append(f"sha256: {rel}: the file's sha256 is not the list's")
        if len(data) != entry.get("bytes"):
            findings.append(
                f"bytes: {rel}: {len(data)} bytes where the list says {entry.get('bytes')}"
            )
        reader = READERS.get(entry.get("kind"))
        if reader is None:
            findings.append(f"kind: {rel}: no reader for the kind {entry.get('kind')!r}")
            continue
        try:
            kind, pixels, metadata = reader(data)
        except ValueError as error:
            findings.append(f"kind: {rel}: not a {entry['kind']}: {error}")
            continue
        if kind != entry["kind"]:
            findings.append(f"kind: {rel}: read as {kind}, listed as {entry['kind']}")
        if pixels != entry.get("pixels"):
            findings.append(f"pixels: {rel}: {pixels} where the list says {entry.get('pixels')}")
        if metadata:
            findings.append(f"metadata: {rel}: {', '.join(metadata)}")
    return findings


def readme_findings(root):
    """Every way the README's header differs from the title, a blank line, the hero from a listed
    file, a blank line and the description paragraph, as "<reason>: README.md: <detail>"."""
    lines = (Path(root) / README).read_text(encoding="utf-8").split("\n") + [""] * 5
    findings = []
    if lines[0] != TITLE:
        findings.append(f"title-not-first: README.md: line 1 is {lines[0][:60]!r}")
    image = IMAGE_LINE.fullmatch(lines[2])
    if lines[1] or image is None or lines[3]:
        findings.append(
            f"hero-not-below-title: README.md: lines 2 to 4 are not a blank line, the hero and a"
            f" blank line; line 3 is {lines[2][:60]!r}"
        )
    else:
        alt, path = image.groups()
        if alt != HERO_ALT:
            findings.append(f"hero-alt: README.md: the hero's alt text is {alt!r}")
        files, _problem = read_list(root)
        gifs = {entry.get("path") for entry in files or [] if entry.get("kind") == "gif"}
        if path not in gifs or not (Path(root) / path).is_file():
            findings.append(f"hero-not-listed: README.md: {path} is not a listed gif in the tree")
    if not lines[4] or lines[4][0] in "#!<>":
        findings.append(f"paragraph-missing: README.md: line 5 is {lines[4][:60]!r}")
    return findings


def app_target(root):
    """(the sources, the base settings) of the app target in the project spec at `root`."""
    app = target((Path(root) / PROJECT).read_text(encoding="utf-8"), APP_TARGET)
    sources = re.findall(r"(?m)^      - path: (\S+)\s*$", section(app, "sources"))
    settings = re.findall(r"(?m)^        ([A-Z][A-Z0-9_]*): (.*?)\s*$", section(app, "settings"))
    return sources, dict(settings)


def app_findings(root):
    """Every way the app target misses its catalog or its icon set, as "<reason>: <path>: <detail>"."""
    root = Path(root)
    sources, settings = app_target(root)
    findings = []
    source = CATALOG.relative_to("ios").as_posix()
    if source not in sources:
        findings.append(f"no-catalog-source: {PROJECT}: the target {APP_TARGET} lists no {source}")
    if settings.get(ICON_SETTING) != ICON_NAME:
        findings.append(
            f"no-icon-setting: {PROJECT}: {ICON_SETTING} is {settings.get(ICON_SETTING)!r}"
        )
    if not (root / CATALOG / "Contents.json").is_file():
        findings.append(f"no-catalog: {CATALOG / 'Contents.json'}: no asset catalog")
    contents = ICON_SET / "Contents.json"
    try:
        images = json.loads((root / contents).read_text(encoding="utf-8"))["images"]
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError):
        images = None
    if images != ICON_IMAGES:
        findings.append(f"icon-set-differs: {contents}: its images are not the three icons")
    files, _problem = read_list(root)
    listed = {entry.get("path") for entry in files or []}
    for image in images if isinstance(images, list) else []:
        name = image.get("filename", "") if isinstance(image, dict) else ""
        path = (ICON_SET / name).as_posix()
        if not name or path not in listed:
            findings.append(f"icon-not-listed: {path}: not a listed brand file")
    return findings


def reasons(findings):
    """(reason, path) of each finding."""
    return [tuple(finding.split(": ", 2)[:2]) for finding in findings]


def chunk(kind, body):
    """One PNG chunk: its length, type, body and CRC."""
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))


def planted_png(width, height, extra=()):
    """An 8-bit red, green and blue PNG of one colour, with the chunks `extra` before its data."""
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    rows = b"".join(b"\x00" + b"\x10\x20\x30" * width for _row in range(height))
    middle = b"".join(chunk(kind, body) for kind, body in extra)
    return (
        SIGNATURE
        + chunk(b"IHDR", header)
        + middle
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


def planted_ico():
    """An icon file of two entries, a 2 by 2 and a 4 by 4 PNG."""
    images = [(2, planted_png(2, 2)), (4, planted_png(4, 4))]
    directory, offset = b"", 6 + 16 * len(images)
    for side, image in images:
        directory += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(image), offset)
        offset += len(image)
    return struct.pack("<HHH", 0, 1, len(images)) + directory + b"".join(i for _s, i in images)


def planted_gif():
    """A 1 by 1 GIF89a that loops: a two-colour table, the NETSCAPE2.0 loop block, one image."""
    return (
        b"GIF89a"
        + struct.pack("<HHBBB", 1, 1, 0x80, 0, 0)
        + b"\x00\x00\x00\xff\xff\xff"
        + b"\x21\xff\x0b"
        + GIF_LOOP
        + b"\x03\x01\x00\x00\x00"
        + b"\x2c"
        + struct.pack("<HHHHB", 0, 0, 1, 1, 0)
        + b"\x02\x02\x44\x01\x00"
        + b"\x3b"
    )


def planted_files():
    """The complete planted set, path to (bytes, kind, pixels): one file of each kind, and the
    icon set's three icons and its contents."""
    square = planted_png(2, 2)
    contents = json.dumps({"images": ICON_IMAGES, "info": {"author": "xcode", "version": 1}})
    return {
        "docs/assets/hero.gif": (planted_gif(), "gif", [[1, 1]]),
        (ICON_SET / "AppIcon-1024.png").as_posix(): (square, "png", [[2, 2]]),
        (ICON_SET / "AppIcon-dark-1024.png").as_posix(): (square, "png", [[2, 2]]),
        (ICON_SET / "AppIcon-tinted-1024.png").as_posix(): (square, "png", [[2, 2]]),
        (ICON_SET / "Contents.json").as_posix(): (contents.encode("utf-8"), "json", None),
        "web/app/static/favicon.ico": (planted_ico(), "ico", [[2, 2], [4, 4]]),
        "web/app/static/favicon.svg": (b'<svg width="2" height="2"></svg>\n', "svg", None),
        "web/app/static/icon.png": (square, "png", [[2, 2]]),
    }


def write_list(root, files):
    """The list for `files`, each entry's sha256 and size computed from its bytes."""
    entries = [
        {
            "path": path,
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
            "kind": kind,
            "pixels": pixels,
        }
        for path, (data, kind, pixels) in sorted(files.items())
    ]
    (root / LIST).parent.mkdir(parents=True, exist_ok=True)
    (root / LIST).write_text(json.dumps({"files": entries}, indent=2) + "\n", encoding="utf-8")


PLANTED_README = "# DeckStreak\n\n![DeckStreak](docs/assets/hero.gif)\n\nA planted paragraph.\n"
PLANTED_SPEC = """targets:
  DeckStreak:
    type: application
    sources:
      - path: App/Sources
      - path: App/Assets.xcassets
    settings:
      base:
        GENERATE_INFOPLIST_FILE: NO
        ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon
"""


def plant(directory, files=None):
    """A complete planted tree under `directory`: the files, their list, a README with its hero, a
    project spec and the catalog."""
    root = Path(directory)
    files = planted_files() if files is None else files
    for path, (data, _kind, _pixels) in files.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
    write_list(root, files)
    (root / README).write_text(PLANTED_README, encoding="utf-8")
    (root / PROJECT).parent.mkdir(parents=True, exist_ok=True)
    (root / PROJECT).write_text(PLANTED_SPEC, encoding="utf-8")
    (root / CATALOG / "Contents.json").write_text(
        json.dumps({"info": {"author": "xcode", "version": 1}}), encoding="utf-8"
    )
    return root


class TheBrandFilesAreTheExports(unittest.TestCase):
    def test_every_listed_file_matches_its_entry(self):
        # A1 (R1, R2): each of R1's files is in the tree as its entry says.
        absent = [path for path in BRAND_PATHS if not (REPO / path).is_file()]
        self.assertEqual(file_findings(REPO), [], f"R1's files absent from the tree: {absent}")
        files, _problem = read_list(REPO)
        listed = examined("listed brand files", [entry["path"] for entry in files])
        self.assertEqual(listed, list(BRAND_PATHS))

    def test_a_missing_file_is_refused(self):
        # A2 (R2).
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            (root / "web/app/static/icon.png").unlink()
            self.assertEqual(reasons(file_findings(root)), [("missing", "web/app/static/icon.png")])

    def test_a_changed_byte_is_refused(self):
        # A3 (R2): one byte of the vector icon changed, its size and its kind kept.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            path = root / "web/app/static/favicon.svg"
            data = bytearray(path.read_bytes())
            data[data.index(b'width="2"') + len('width="')] = ord("3")
            path.write_bytes(bytes(data))
            found = file_findings(root)
            self.assertEqual(reasons(found), [("sha256", "web/app/static/favicon.svg")], found)

    def test_a_pixel_size_other_than_the_lists_is_refused(self):
        # A4 (R2): the file is right, and its entry says 193 by 193.
        with tempfile.TemporaryDirectory() as tmp:
            files = planted_files()
            root = plant(tmp, files)
            data, kind, _pixels = files["web/app/static/icon.png"]
            files["web/app/static/icon.png"] = (data, kind, [[193, 193]])
            write_list(root, files)
            found = file_findings(root)
            self.assertEqual(reasons(found), [("pixels", "web/app/static/icon.png")], found)

    def test_a_text_or_time_chunk_is_refused(self):
        # A5 (R2): a PNG carrying a text chunk, then one carrying a time chunk, each listed as it is.
        planted = {
            "tEXt": (b"tEXt", b"Comment\x00planted"),
            "tIME": (b"tIME", struct.pack(">HBBBBB", 2026, 1, 1, 0, 0, 0)),
        }
        for name, extra in examined("planted metadata chunks", list(planted.items())):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                files = planted_files()
                files["web/app/static/icon.png"] = (planted_png(2, 2, [extra]), "png", [[2, 2]])
                found = file_findings(plant(tmp, files))
                self.assertEqual(reasons(found), [("metadata", "web/app/static/icon.png")], found)
                self.assertTrue(found[0].endswith(f": {name}"), found)

    def test_a_complete_planted_set_reads_green(self):
        # A6 (R2): the control for A2 to A5.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            files, _problem = read_list(root)
            examined("planted brand files", files)
            self.assertEqual(len(files), len(planted_files()))
            self.assertEqual(file_findings(root), [])


class TheReadmeHeaderShowsTheHero(unittest.TestCase):
    def test_the_readme_opens_with_the_title_then_the_hero(self):
        # A7 (R4).
        self.assertEqual(readme_findings(REPO), [])

    def test_an_image_above_the_title_is_refused(self):
        # A8 (R4).
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            self.assertEqual(readme_findings(root), [], "the planted README is the control")
            (root / README).write_text(
                "![DeckStreak](docs/assets/hero.gif)\n\n# DeckStreak\n\nA planted paragraph.\n",
                encoding="utf-8",
            )
            self.assertEqual(
                reasons(readme_findings(root)),
                [("title-not-first", "README.md"), ("hero-not-below-title", "README.md")],
            )

    def test_an_image_that_is_not_a_listed_file_is_refused(self):
        # A9 (R4): an image from outside the repository, a committed gif the list does not name,
        # and a listed file that is not a gif.
        planted = {
            "an https image": "https://example.com/deckstreak-hero.gif",
            "an unlisted gif": "docs/assets/other.gif",
            "a listed file that is not a gif": "web/app/static/icon.png",
        }
        for name, path in examined("planted hero paths", list(planted.items())):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                root = plant(tmp)
                self.assertEqual(readme_findings(root), [], "the planted README is the control")
                (root / "docs/assets/other.gif").write_bytes(planted_gif())
                text = PLANTED_README.replace("docs/assets/hero.gif", path)
                (root / README).write_text(text, encoding="utf-8")
                self.assertEqual(reasons(readme_findings(root)), [("hero-not-listed", "README.md")])


class TheAppCarriesItsIconSet(unittest.TestCase):
    def test_the_app_target_names_the_catalog_and_its_icon_set(self):
        # A10 (R5).
        sources, settings = app_target(REPO)
        examined(f"sources of the target {APP_TARGET}", sources)
        examined(f"base settings of the target {APP_TARGET}", list(settings.items()))
        self.assertEqual(app_findings(REPO), [])

    def test_an_icon_set_missing_an_icon_is_refused(self):
        # A11 (R5): the tinted icon left out of the icon set.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            self.assertEqual(app_findings(root), [], "the planted tree is the control")
            images = [image for image in ICON_IMAGES if "tinted" not in image["filename"]]
            (root / ICON_SET / "Contents.json").write_text(
                json.dumps({"images": images}), encoding="utf-8"
            )
            contents = (ICON_SET / "Contents.json").as_posix()
            self.assertEqual(reasons(app_findings(root)), [("icon-set-differs", contents)])

    def test_a_target_without_the_icon_setting_is_refused(self):
        # A12 (R5).
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            self.assertEqual(app_findings(root), [], "the planted tree is the control")
            spec = PLANTED_SPEC.replace("        ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon\n", "")
            self.assertNotEqual(spec, PLANTED_SPEC)
            (root / PROJECT).write_text(spec, encoding="utf-8")
            self.assertEqual(reasons(app_findings(root)), [("no-icon-setting", PROJECT.as_posix())])


if __name__ == "__main__":
    unittest.main()
