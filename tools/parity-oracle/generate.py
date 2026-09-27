#!/usr/bin/env python3
"""Write the parity oracle's goldens by calling the predecessor's own functions (ADR-012, ADR-029).

Run it on the owner's private checkout of the predecessor, never in this public repository's CI:

    python3 tools/parity-oracle/generate.py --source-checkout PATH --package NAME \\
        --out tools/parity-oracle/goldens

`--package` names the predecessor's top-level package, and every registry path is relative to it
(`analytics.study_day`), so no committed file names the package.

The registry is one module per SPEC, `registry/spec_NNN.py`, loaded in path order. Each exports
`FUNCTIONS`, a golden's name mapped to one registration of three kinds (README.md):

    {"kind": "function", "function": "module.name", "cases": build}
    {"kind": "adapter", "function": "module.name", "adapter": glue, "note": "...", "cases": build}
    {"kind": "constants", "names": ["module.NAME", ...]}

A `function` is called with each case's input as keyword arguments. An `adapter` is glue in the
registry module, called as `glue(function, predecessor, **input)`: it receives the predecessor's
function already resolved, builds the arguments JSON cannot carry (`predecessor(path)` resolves any
other object of the predecessor), and CALLS the function; it never computes the rule itself. A
`constants` golden reads each attribute from the predecessor, never typed. A case builder,
`build(rng)`, receives only a `random.Random` seeded with SEED and returns `(class or None, input)`
pairs.

Each golden records the predecessor's commit, this file's sha256 and the sha256 of the registry
module that built it, so a golden reads stale when either changes and editing one registry module
stales only its own goldens (test_goldens.py). No golden holds a calendar date: a returned `date`
is written as its epoch day number and an aware `datetime` as epoch milliseconds.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib
import importlib.util
import json
import platform
import random
import re
import subprocess
import sys
from collections.abc import Callable
from functools import partial
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve()
REGISTRY = HERE.parent / "registry"
SEED = 20
SCHEMA = "phx.parity-golden.v1"
GENERATOR = "tools/parity-oracle/generate.py"
#: Where a registry module is published, whichever directory the generator loaded it from.
PUBLISHED_REGISTRY = "tools/parity-oracle/registry"
MODULE_NAME = re.compile(r"spec_\d{3}\.py")
GOLDEN_NAME = re.compile(r"[a-z0-9]+(?:[_.][a-z0-9]+)*")
#: The keys each kind of registration takes, all of them required.
KINDS = {
    "function": {"kind", "function", "cases"},
    "adapter": {"kind", "function", "adapter", "note", "cases"},
    "constants": {"kind", "names"},
}
EPOCH = dt.datetime(1970, 1, 1, tzinfo=dt.timezone.utc)
EPOCH_ORDINAL = EPOCH.date().toordinal()
MILLISECOND = dt.timedelta(milliseconds=1)

Case = tuple[str | None, dict]


class RegistryError(Exception):
    """A registry the generator refuses before it calls anything (exit 2)."""


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_registry(directory: Path) -> dict[str, tuple[Path, dict]]:
    """Every registration in `directory`, in path order: golden name -> (module, registration)."""
    registered: dict[str, tuple[Path, dict]] = {}
    for path in sorted(directory.glob("*.py")):
        where = f"registry/{path.name}"
        if not MODULE_NAME.fullmatch(path.name):
            raise RegistryError(
                f"{where} is not named spec_NNN.py: one module per SPEC"
            )
        functions = getattr(load_module(path), "FUNCTIONS", None)
        if not isinstance(functions, dict):
            raise RegistryError(f"{where} exports no FUNCTIONS mapping")
        for name, registration in functions.items():
            if name in registered:
                first = f"registry/{registered[name][0].name}"
                raise RegistryError(
                    f"golden {name!r} is registered by both {first} and {where}"
                )
            check(where, name, registration)
            registered[name] = (path, registration)
    if not registered:
        raise RegistryError(f"{directory} registers no golden")
    return registered


def load_module(path: Path) -> ModuleType:
    name = f"parity_registry_{path.stem}"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    # A dataclass in the module resolves its own annotations through sys.modules.
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def check(where: str, name: object, registration: object) -> None:
    """Refuse a registration that is not exactly one of the three kinds."""
    if not isinstance(name, str) or not GOLDEN_NAME.fullmatch(name):
        raise RegistryError(
            f"{where}: {name!r} is not a golden name (a-z, 0-9, '_' and '.')"
        )
    kind = registration.get("kind") if isinstance(registration, dict) else None
    if kind not in KINDS:
        raise RegistryError(
            f"{where}: {name} has kind {kind!r}, not one of {', '.join(KINDS)}"
        )
    if set(registration) != KINDS[kind]:
        raise RegistryError(
            f"{where}: a {kind} registration takes {', '.join(sorted(KINDS[kind]))}; "
            f"{name} gives {', '.join(sorted(registration))}"
        )
    if kind == "constants":
        names = registration["names"]
        if (
            not isinstance(names, list)
            or not names
            or not all(isinstance(n, str) for n in names)
        ):
            raise RegistryError(f"{where}: {name} names no attribute")
        return
    if not isinstance(registration["function"], str) or not callable(
        registration["cases"]
    ):
        raise RegistryError(
            f"{where}: {name} needs a predecessor function and a case builder"
        )
    if kind == "adapter" and not (
        callable(registration["adapter"])
        and isinstance(registration["note"], str)
        and registration["note"].strip()
    ):
        raise RegistryError(f"{where}: {name} needs a callable adapter and a note")


def resolve(package: str, path: str) -> object:
    """The predecessor's object at `path`, a dotted path relative to its package."""
    parts = path.split(".")
    for split in range(len(parts), 0, -1):
        name = ".".join([package, *parts[:split]])
        try:
            target = importlib.import_module(name)
        except ModuleNotFoundError as missing:
            # Only the path itself may be missing; a dependency the predecessor lacks is raised.
            if missing.name is None or not (
                name == missing.name or name.startswith(f"{missing.name}.")
            ):
                raise
            continue
        for attribute in parts[split:]:
            target = getattr(target, attribute)
        return target
    raise ModuleNotFoundError(f"no module of {package!r} holds {path!r}")


def plain(value: object) -> object:
    """`value` as JSON with no calendar string in it: a `date` becomes its epoch day number and
    an aware `datetime` its epoch milliseconds, both floored."""
    if isinstance(value, dt.datetime):
        if value.utcoffset() is None:
            raise ValueError(f"{value!r} is a naive datetime, which names no instant")
        return (value - EPOCH) // MILLISECOND
    if isinstance(value, dt.date):
        return value.toordinal() - EPOCH_ORDINAL
    if isinstance(value, dict):
        return {key: plain(item) for key, item in value.items()}
    if isinstance(value, list | tuple):
        return [plain(item) for item in value]
    return value


def draw(name: str, build: Callable[[random.Random], list[Case]]) -> list[Case]:
    """A builder's cases, drawn twice from `random.Random(SEED)`. A builder that draws from any
    other source would write a golden nobody could regenerate, so a second draw that differs is
    refused, and so is a draw of nothing."""
    first, second = build(random.Random(SEED)), build(random.Random(SEED))
    if first != second:
        raise ValueError(
            f"{name}: its case builder draws from more than random.Random(SEED)"
        )
    if not first:
        raise ValueError(f"{name}: its case builder drew no case")
    for edge, arguments in first:
        if edge is not None and not (isinstance(edge, str) and edge):
            raise ValueError(f"{name}: a case's class is {edge!r}, not a name or None")
        if not isinstance(arguments, dict):
            raise ValueError(f"{name}: a case's input is {arguments!r}, not an object")
    return first


def cases_of(
    name: str, registration: dict, predecessor: Callable[[str], object]
) -> list[dict]:
    """The golden's cases: each input with the output the predecessor's own code returned."""
    kind = registration["kind"]
    if kind == "constants":
        return [
            {"input": {"name": path}, "output": plain(predecessor(path))}
            for path in registration["names"]
        ]
    function = predecessor(registration["function"])
    cases = []
    for edge, arguments in draw(name, registration["cases"]):
        recorded = json.loads(json.dumps(arguments, allow_nan=False))
        if kind == "adapter":
            output = registration["adapter"](function, predecessor, **arguments)
        else:
            output = function(**arguments)
        case = {"input": recorded, "output": plain(output)}
        if edge is not None:
            case["class"] = edge
        cases.append(case)
    return cases


def golden_for(
    name: str,
    module: Path,
    registration: dict,
    predecessor: Callable[[str], object],
    commit: str,
    generator_digest: str,
) -> dict:
    kind = registration["kind"]
    golden = {
        "schema": SCHEMA,
        "kind": kind,
        # A constants golden drives no one function, so its name stands for one.
        "function": name if kind == "constants" else registration["function"],
        "source_commit": commit,
        "generator": GENERATOR,
        "generator_sha256": generator_digest,
        "registry": f"{PUBLISHED_REGISTRY}/{module.name}",
        "registry_sha256": sha256(module),
        "inputs": "synthetic",
        "seed": SEED,
        "cases": cases_of(name, registration, predecessor),
    }
    if kind == "adapter":
        golden["adapter"] = registration["adapter"].__name__
        # The interpreter drew the cases: a seeded sequence may change between versions.
        interpreter = f"{platform.python_implementation()} {platform.python_version()}"
        golden["note"] = (
            f"{registration['note'].rstrip('.')}; cases drawn under {interpreter}."
        )
    return golden


def render(golden: dict) -> str:
    """Strict, canonical JSON: sorted keys, a two-space indent, and no NaN or Infinity."""
    return json.dumps(golden, indent=2, sort_keys=True, allow_nan=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--source-checkout", required=True, help="the predecessor's private checkout"
    )
    parser.add_argument(
        "--package",
        required=True,
        help="the predecessor's top-level package, which registry paths are relative to",
    )
    parser.add_argument(
        "--registry",
        default=str(REGISTRY),
        help="the registry directory (default: registry/)",
    )
    parser.add_argument("--out", required=True, help="the goldens directory")
    args = parser.parse_args(argv)
    try:
        registered = load_registry(Path(args.registry))
    except RegistryError as refusal:
        print(f"generate.py: {refusal}", file=sys.stderr)
        return 2
    checkout = Path(args.source_checkout).resolve()
    commit = subprocess.run(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    generator_digest = sha256(HERE)
    predecessor = partial(resolve, args.package)
    # The generator reads the predecessor's checkout and never writes to it, not even bytecode.
    writes_bytecode, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    sys.path.insert(0, str(checkout))
    try:
        texts = {
            name: render(
                golden_for(
                    name, module, registration, predecessor, commit, generator_digest
                )
            )
            for name, (module, registration) in registered.items()
        }
    finally:
        sys.path.remove(str(checkout))
        sys.dont_write_bytecode = writes_bytecode
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for name, text in texts.items():
        path = out / f"{name}.json"
        path.write_text(text, encoding="utf-8")
        print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
