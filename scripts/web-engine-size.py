#!/usr/bin/env python3
"""web-engine-size: the web engine's size gate (SPEC-338 R8, ADR-349, ADR-336's budget).

    python3 scripts/web-engine-size.py \\
        --module target/web-engine/deck_streak_web_engine_bg.wasm \\
        --bindings target/web-engine/deck_streak_web_engine.js

WHY. ADR-336 accepted the browser engine on a budget: the module a browser loads plus its JS
bindings, each compressed by `gzip -9`, summed, at most 8000000 bytes. These are the bytes a
browser downloads, so the gate measures exactly them, one file at a time, and sums. Brotli at
quality 11 is the figure a server that serves brotli would see; it is printed beside each file
and never gated, because the budget is in `gzip -9`.

A size is only a measurement when its compressor ran on a file that holds something, so a missing
or empty file, or an absent compressor, reads VOID: never a pass, and never a figure printed as if
measured. gzip runs with `-n`, so the file's name does not count toward its size.

One line per file, then the verdict on the last line. Exit 0 when the total is at most the budget,
1 above it, and 2 for VOID (argparse's usage error is 2 as well, and is no pass either).
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True

#: ADR-336's budget, in bytes `gzip -9`, for the module plus its bindings.
BUDGET = 8_000_000
EXIT_PASS, EXIT_OVER, EXIT_VOID = 0, 1, 2
#: Brotli at quality 11 with a 2^24-byte window, the brotli CLI's own defaults, by node's zlib.
BROTLI = (
    "const z = require('zlib'), fs = require('fs');"
    "const data = fs.readFileSync(process.argv[1]);"
    "const out = z.brotliCompressSync(data, {params: {"
    "[z.constants.BROTLI_PARAM_QUALITY]: 11, [z.constants.BROTLI_PARAM_LGWIN]: 24,"
    "[z.constants.BROTLI_PARAM_SIZE_HINT]: data.length}});"
    "process.stdout.write(String(out.length));"
)


class Void(Exception):
    """A reason the gate measured nothing it could judge."""


def judge(total: int) -> int:
    """The exit a `gzip -9` total earns: a pass at or under the budget, a failure above it."""
    return EXIT_PASS if total <= BUDGET else EXIT_OVER


def gzip_size(gzip: str, path: Path) -> int:
    """The file's size after `gzip -9`, measured by gzip itself."""
    out = subprocess.run([gzip, "-9", "-n", "-c", str(path)], capture_output=True, check=False)
    if out.returncode != 0 or not out.stdout:
        raise Void(f"gzip exited {out.returncode} on {path}")
    return len(out.stdout)


def brotli_size(node: str, path: Path) -> int:
    """The file's size after brotli at quality 11, measured by node's zlib."""
    out = subprocess.run(
        [node, "-e", BROTLI, str(path)], capture_output=True, check=False, text=True
    )
    if out.returncode != 0 or not out.stdout.strip().isdigit():
        raise Void(f"node's brotli exited {out.returncode} on {path}")
    return int(out.stdout)


def measure(files: list[tuple[str, Path]]) -> tuple[int, int]:
    """Measure each file and print its line. Returns the `gzip -9` and brotli totals."""
    for name, path in files:
        if not path.is_file():
            raise Void(f"the {name} {path} is missing")
        if path.stat().st_size == 0:
            raise Void(f"the {name} {path} is empty")
    tools = {}
    for tool in ("gzip", "node"):
        found = shutil.which(tool)
        if found is None:
            raise Void(f"{tool} is not on PATH, so no {tool} figure is measured")
        tools[tool] = found
    total_gzip = total_brotli = 0
    for name, path in files:
        raw = path.stat().st_size
        gzipped = gzip_size(tools["gzip"], path)
        brotli = brotli_size(tools["node"], path)
        print(
            f"web-engine-size: {name} {path.name}: raw {raw}, gzip-9 {gzipped}, brotli-11 {brotli}"
        )
        total_gzip += gzipped
        total_brotli += brotli
    print(f"web-engine-size: examined {len(files)} file(s)")
    return total_gzip, total_brotli


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="web-engine-size", description=__doc__.split("\n")[0])
    parser.add_argument("--module", required=True, type=Path, help="the module a browser loads")
    parser.add_argument("--bindings", required=True, type=Path, help="its JS bindings")
    args = parser.parse_args(argv)
    try:
        total, brotli = measure([("module", args.module), ("bindings", args.bindings)])
    except Void as void:
        print(f"web-engine-size: VOID: {void}")
        return EXIT_VOID
    verdict = judge(total)
    if verdict == EXIT_PASS:
        print(
            f"web-engine-size: PASS: total gzip-9 {total} <= {BUDGET}, "
            f"headroom {BUDGET - total}; total brotli-11 {brotli}"
        )
    else:
        print(
            f"web-engine-size: OVER BUDGET: total gzip-9 {total} > {BUDGET} by {total - BUDGET}; "
            f"total brotli-11 {brotli}"
        )
    return verdict


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
