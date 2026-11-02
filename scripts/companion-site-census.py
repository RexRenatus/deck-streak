#!/usr/bin/env python3
"""companion-site-census: the built site holds no file the owner route serves (SPEC-511 R12).

    python3 scripts/companion-site-census.py --root web/app/build

WHY. Every file of the built site is public: the reverse proxy serves the web root to anyone. The
owner's scheduling companion (its module, its weights and the manifest that names them) reaches the
owner's session alone, through the API's owner route, and never sits in the site (ADR-529 D1, D5).
This census reads every file of a built site and refuses, by path:

- a file named `module.wasm` or `weights.bin`;
- any file under `api/`, which the proxy hands to the API and a static file would shadow;
- a `manifest.json` that names `module.wasm` or `weights.bin`;
- a WebAssembly module outside `engine/`, where the public engine's module is staged.

A census is only a verdict over a site that holds something, so a missing site or one with no file
reads VOID: never a pass. It prints how many files it examined, one line per finding, then the
verdict. Exit 0 when nothing is found, 1 on a finding, 2 for VOID (argparse's usage error is 2 as
well, and is no pass either).
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True

EXIT_PASS, EXIT_FOUND, EXIT_VOID = 0, 1, 2
#: The names the owner route serves; a file of either name is never public.
SERVED_NAMES = ("module.wasm", "weights.bin")
#: The manifest's name; it is refused when it names a served file.
MANIFEST = "manifest.json"
#: The first four bytes of every WebAssembly module.
WASM_MAGIC = b"\0asm"
#: The site's folder of the public engine, the one place a module may sit.
ENGINE = "engine"
#: The API's prefix, which the proxy hands to the API.
API = "api"


class Void(Exception):
    """A reason the census judged nothing."""


def files_of(root: Path) -> list[str]:
    """Every file under `root`, as `/`-separated paths relative to it, sorted; a link to a folder
    is not followed, and a link to a file is read as that file."""
    if not root.is_dir():
        raise Void(f"the site {root} is missing")
    found = []
    for folder, _folders, names in os.walk(root):
        for name in names:
            found.append((Path(folder) / name).relative_to(root).as_posix())
    if not found:
        raise Void(f"the site {root} holds no file")
    return sorted(found)


def head(path: Path, size: int) -> bytes:
    """The first `size` bytes of `path`, or none when it does not read."""
    try:
        with path.open("rb") as handle:
            return handle.read(size)
    except OSError:
        return b""


def findings(root: Path, relative: str) -> list[str]:
    """Why the file at `relative` under `root` must not be in the site, each reason named."""
    parts = relative.split("/")
    name = parts[-1]
    path = root / relative
    found = []
    if name in SERVED_NAMES:
        found.append(f"a file named {name}, which the owner route serves")
    if parts[0] == API:
        found.append(f"a file under {API}/, which the proxy hands to the API")
    if name == MANIFEST:
        text = head(path, 1 << 20)
        named = [served for served in SERVED_NAMES if served.encode() in text]
        if named:
            found.append(f"a {MANIFEST} that names {' and '.join(named)}")
    if parts[0] != ENGINE and head(path, len(WASM_MAGIC)) == WASM_MAGIC:
        found.append(f"a WebAssembly module outside {ENGINE}/")
    return found


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="companion-site-census", description=__doc__.split("\n")[0]
    )
    parser.add_argument("--root", required=True, type=Path, help="the built site's root")
    args = parser.parse_args(argv)
    try:
        files = files_of(args.root)
    except Void as void:
        print(f"companion-site-census: VOID: {void}")
        return EXIT_VOID
    print(f"companion-site-census: examined {len(files)} file(s)")
    refused = 0
    for relative in files:
        reasons = findings(args.root, relative)
        if reasons:
            refused += 1
            print(f"companion-site-census: FINDING {relative}: {'; '.join(reasons)}")
    if refused:
        print(f"companion-site-census: FAIL: {refused} finding(s)")
        return EXIT_FOUND
    print("companion-site-census: PASS")
    return EXIT_PASS


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
