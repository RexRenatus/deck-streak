#!/usr/bin/env python3
"""web-route-size: each web route's JavaScript against its recorded budget (SPEC-652, ADR-674).

    python3 scripts/web-route-size.py --app web/app --budgets config/web-route-budgets.json

WHY. The web engine's module and the owner's companion files are held to a size budget
(`scripts/web-engine-size.py`), but the Mini App's own JavaScript had none, so a route could grow
unseen. After the web build, this check reads the client build the app holds, gives each route the
JavaScript it needs, and compares each figure with the route's entry in the budget file.

A route's figure (ADR-674 D2). The client build makes one node for each page and each layout, an
entry of Vite's manifest. A route's files are its page's node and the node of every layout between
it and the root layout, each with the chunks it imports statically, transitively. A dynamic import
is not followed: the client app imports every route's node dynamically, so following one would
count every route in each. The shell (every other entry: the client's start and app, the root
layout and the error page, each with its static imports) loads before any route, so it is one
entry of its own, `(shell)`, and a route's figure leaves the shell's files out. A chunk's CSS is
not its JavaScript, so no stylesheet is counted. A route is named by its page's folder under
`src/routes`, a group's folder included, so no two pages share a name.

The unit is the neighbour's: each file compressed by `gzip -9 -n`, measured by gzip itself, summed.

Four findings: a route over its entry, a route with no entry, an entry for a route that is gone,
and a route more than RATCHET_MARGIN bytes under its entry, whose entry the ratchet asks lowered
to the figure. At LEVEL "warn" each prints a WARN line and the check passes; at "fail" each prints
a FAIL line and the check fails. LEVEL moves to "fail" only by its own delivery (ADR-674 D6).

VOID, never a pass: a manifest or budget file that does not read, no route examined, a page under
`src/routes` with no node in the manifest, a node or chunk the manifest names and does not hold, a
file outside the client folder, a missing or empty file, a gzip that is absent or fails, or a
budget entry that is not whole bytes above zero with the job that measured them.

One line per route, then each finding, then the verdict on the last line. Exit 0 at a pass or at
warn, 1 at fail with a finding, and 2 for VOID (argparse's usage error is 2 as well, and is no pass
either).
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True

#: The level findings are reported at (ADR-674 D5): "warn" passes them, "fail" fails on them.
LEVEL = "warn"
#: How far, in bytes `gzip -9`, a route may sit under its entry before the ratchet asks it lowered.
RATCHET_MARGIN = 1024
EXIT_PASS, EXIT_FAIL, EXIT_VOID = 0, 1, 2
#: Where the client build leaves its files and Vite's manifest, each relative to the app.
CLIENT = Path(".svelte-kit") / "output" / "client"
MANIFEST = CLIENT / ".vite" / "manifest.json"
#: A node's source as the manifest names it, and the route file a node's source re-exports.
NODE = re.compile(r"^\.svelte-kit/generated/build/client-optimized/nodes/[0-9]+\.js$")
ROUTE_FILE = re.compile(r"src/routes/((?:[^\"'/\s]+/)*)\+(page|layout)\.svelte")
#: The entry of the files every route loads first.
SHELL = "(shell)"


class Void(Exception):
    """A reason the check measured nothing it could judge."""


def read_json(path: Path, what: str) -> dict:
    """The JSON object at `path`, or VOID naming `what`."""
    try:
        loaded = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Void(f"the {what} {path} cannot be read: {error}") from None
    if not isinstance(loaded, dict):
        raise Void(f"the {what} {path} is not a JSON object")
    return loaded


def route_id(folder: str) -> str:
    """The name of the route whose page sits in `folder` under `src/routes`, which is empty or
    ends in a slash: a slash, then the folder without its last slash."""
    return "/" + folder[:-1]


def page_files(app: Path) -> dict[str, str]:
    """Every page under the app's `src/routes`, by the route it serves."""
    routes = app / "src" / "routes"
    found = {}
    for page in sorted(routes.rglob("+page.svelte")):
        folder = page.relative_to(routes).as_posix().removesuffix("+page.svelte")
        found[route_id(folder)] = page.relative_to(app).as_posix()
    return found


def nodes(app: Path, manifest: dict) -> tuple[dict, dict, list]:
    """The manifest's entries: the pages, {route: (key, folder)}; the layouts below the root,
    {folder: key}; and the shell's keys, every other entry."""
    pages: dict[str, tuple[str, str]] = {}
    layouts: dict[str, str] = {}
    shell: list[str] = []
    for key, chunk in sorted(manifest.items()):
        if not isinstance(chunk, dict):
            raise Void(f"the manifest's entry {key} is not an object")
        if not chunk.get("isEntry"):
            continue
        source = chunk.get("src")
        if not isinstance(source, str) or not NODE.match(source):
            shell.append(key)
            continue
        try:
            text = (app / source).read_text(encoding="utf-8")
        except OSError as error:
            raise Void(f"the node {source} cannot be read: {error}") from None
        named = ROUTE_FILE.search(text)
        if named is None or (named.group(2) == "layout" and not named.group(1)):
            shell.append(key)
        elif named.group(2) == "layout":
            layouts[named.group(1)] = key
        elif route_id(named.group(1)) in pages:
            raise Void(f"two nodes serve the route {route_id(named.group(1))}")
        else:
            pages[route_id(named.group(1))] = (key, named.group(1))
    return pages, layouts, shell


def closure(manifest: dict, keys: list[str]) -> set[str]:
    """The files of `keys` and of every chunk they import statically, transitively. A dynamic
    import is not followed, and a chunk's CSS is not its JavaScript."""
    files: set[str] = set()
    seen: set[str] = set()
    stack = list(keys)
    while stack:
        key = stack.pop()
        if key in seen:
            continue
        seen.add(key)
        chunk = manifest.get(key)
        if not isinstance(chunk, dict) or not isinstance(chunk.get("file"), str):
            raise Void(f"the manifest names {key} and holds no file for it")
        files.add(chunk["file"])
        stack.extend(chunk.get("imports", []))
    return files


def gzip_size(gzip: str, path: Path) -> int:
    """The file's size after `gzip -9 -n`, measured by gzip itself."""
    out = subprocess.run([gzip, "-9", "-n", "-c", str(path)], capture_output=True, check=False)
    if out.returncode != 0 or not out.stdout:
        raise Void(f"gzip exited {out.returncode} on {path}")
    return len(out.stdout)


def total(client: Path, files: set[str], gzip: str) -> int:
    """The `gzip -9 -n` sum of `files`, each under the client folder."""
    summed = 0
    for name in sorted(files):
        if Path(name).is_absolute() or ".." in Path(name).parts:
            raise Void(f"the manifest names {name}, outside the client folder")
        path = client / name
        if not path.is_file():
            raise Void(f"the file {path} is missing")
        if path.stat().st_size == 0:
            raise Void(f"the file {path} is empty")
        summed += gzip_size(gzip, path)
    return summed


def measure(app: Path) -> tuple[dict[str, int], dict[str, int]]:
    """Each route's figure and file count, and the shell's, from the app's client build."""
    manifest = read_json(app / MANIFEST, "manifest")
    pages, layouts, shell = nodes(app, manifest)
    on_disk = page_files(app)
    missing = sorted(set(on_disk) - set(pages))
    if missing:
        raise Void(f"the page {on_disk[missing[0]]} has no node in the manifest")
    if not pages:
        raise Void("examined 0 route(s)")
    gzip = shutil.which("gzip")
    if gzip is None:
        raise Void("gzip is not on PATH, so no figure is measured")
    shell_files = closure(manifest, shell)
    figures = {SHELL: total(app / CLIENT, shell_files, gzip)}
    counts = {SHELL: len(shell_files)}
    for route, (key, folder) in sorted(pages.items()):
        keys = [key] + [layouts[at] for at in sorted(layouts) if folder.startswith(at)]
        own = closure(manifest, keys) - shell_files
        figures[route] = total(app / CLIENT, own, gzip)
        counts[route] = len(own)
    return figures, counts


def budgets(path: Path) -> dict[str, int]:
    """Each entry's bytes, by route. Every entry is `{bytes, job}`, whole bytes above zero and the
    job whose run measured them, so an entry that is not, or a key that names no route, is VOID."""
    table = read_json(path, "budget file").get("routes")
    if not isinstance(table, dict):
        raise Void(f"the budget file {path} holds no routes object")
    found: dict[str, int] = {}
    for route, entry in sorted(table.items()):
        if not (route == SHELL or route.startswith("/")):
            raise Void(f"the budget file's entry {route!r} names no route")
        if not isinstance(entry, dict) or set(entry) != {"bytes", "job"}:
            raise Void(f"the entry for {route} is not an object of bytes and job")
        figure, job = entry["bytes"], entry["job"]
        if type(figure) is not int or figure <= 0 or type(job) is not int or job <= 0:
            raise Void(f"the entry for {route} needs whole bytes and the job that measured them")
        found[route] = figure
    return found


def findings(figures: dict[str, int], table: dict[str, int]) -> list[tuple[str, str, str]]:
    """Each finding as (kind, route, what): over its entry, no entry, the ratchet's request to
    lower an entry the route sits more than RATCHET_MARGIN under, and an entry for no route."""
    found = []
    for route, figure in sorted(figures.items()):
        budget = table.get(route)
        if budget is None:
            found.append(("NO BUDGET", route, f"gzip-9 {figure}; add its entry"))
        elif figure > budget:
            found.append(("OVER BUDGET", route, f"gzip-9 {figure} > {budget} by {figure - budget}"))
        elif budget - figure > RATCHET_MARGIN:
            lower = f"more than {RATCHET_MARGIN} under it; lower its entry to {figure}"
            found.append(("RATCHET", route, f"gzip-9 {figure}, budget {budget}, {lower}"))
    for route in sorted(set(table) - set(figures)):
        found.append(("GONE", route, "an entry for no route; remove it"))
    return found


def report(figures: dict[str, int], counts: dict[str, int], table: dict[str, int], level: str):
    """Print each route's line, each finding at `level` and the verdict. Returns the exit."""
    for route in sorted(figures):
        shown = table.get(route)
        print(
            f"web-route-size: route {route}: {counts[route]} file(s), gzip-9 {figures[route]}, "
            f"budget {'none' if shown is None else shown}"
        )
    print(f"web-route-size: examined {len(figures) - 1} route(s) and the shell")
    found = findings(figures, table)
    tag = "FAIL" if level == "fail" else "WARN"
    for kind, route, what in found:
        print(f"web-route-size: {tag} {kind} {route}: {what}")
    if not found:
        print("web-route-size: PASS: every route within its budget")
    elif level == "fail":
        print(f"web-route-size: FAIL: {len(found)} finding(s)")
    else:
        print(f"web-route-size: WARN: {len(found)} finding(s), reported at warn")
    return EXIT_FAIL if found and level == "fail" else EXIT_PASS


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="web-route-size", description=__doc__.split("\n")[0])
    parser.add_argument("--app", required=True, type=Path, help="the web app's folder, built")
    parser.add_argument(
        "--budgets", required=True, type=Path, help="the budget file, one entry per route"
    )
    args = parser.parse_args(argv)
    try:
        figures, counts = measure(args.app)
        table = budgets(args.budgets)
    except Void as void:
        print(f"web-route-size: VOID: {void}")
        return EXIT_VOID
    return report(figures, counts, table, LEVEL)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
