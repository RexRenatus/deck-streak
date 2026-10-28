#!/usr/bin/env python3
"""Write the second old system's goldens: the Anki engine's own outputs over a seeded synthetic
corpus, from one run of a pinned harness in a sandbox (SPEC-498, ADR-516).

The run is the operator's, in the sandbox the runbook builds (RUNBOOK.md), never public CI:

    python3 tools/parity-oracle/anki/generate.py --harness PATH/TO/BUILT/HARNESS \\
        --harness-source PATH/TO/HARNESS/SOURCE --engine-source PATH/TO/ENGINE/CHECKOUT \\
        --out PATH/TO/OUTPUT

The registry is one module per SPEC, `registry/spec_NNN.py`, loaded in path order. Each exports
`FUNCTIONS`, a golden's name mapped to one registration:

    {"id": "F4", "classes": ["rollover", ...], "note": "...", "cases": build}

`build(rng)` receives only a `random.Random` seeded with the corpus's SEED and returns
`(class or None, input)` pairs. Every declared class must have a case, every case's class must
be declared, and no class may hold one input twice, or the registry is refused before anything
runs (exit 2).

The harness is a small program on the engine's API, kept outside this repository and outside
its workspace. This driver pins the sha256 of the harness's source, so a golden's
`generator_sha256`, the digest of this file, covers the harness too. The driver talks to the
harness in JSON lines over stdin and stdout:

    driver:  {"op": "hello", "protocol": 1, "work": DIR}
    harness: {"engine_commit": "<40 hex>", "stock_texts": ["...", ...]}
    driver:  {"op": "call", "function": NAME, "input": {...}}
    harness: {"output": <JSON>}  or  {"refused": "<label>"}
    driver:  {"op": "bye"}

`stock_texts` are the texts of the note types the engine adds by itself, which the harness then
deletes before it builds anything. The driver keeps only the sha256 of each of their lines and
refuses to write any golden that holds one of those lines (the stock-text census). A refusal
carries a short label, never the engine's words.

Exit 0 when every golden and the census file are written; 2 when the registry, the harness pin
or a source is refused before the harness starts; 3 when the run fails, which writes nothing.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import platform
import random
import re
import subprocess
import sys
import tempfile
from collections.abc import Callable, Iterable
from fnmatch import fnmatch
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve()
REGISTRY = HERE.parent / "registry"
SCHEMA = "phx.parity-golden.v1"
GENERATOR = "tools/parity-oracle/anki/generate.py"
#: Where a registry module is published, whichever directory the driver loaded it from.
PUBLISHED_REGISTRY = "tools/parity-oracle/anki/registry"
#: The engine commit DeckStreak's root manifest pins: the engine every golden records.
ENGINE_COMMIT = "65e053b829825f534803b2b772c507a73ba936a3"
#: The harness's package name. Its manifest has an empty `[workspace]` table, so no lock but its
#: own ever names it.
HARNESS_PACKAGE = "parity-oracle-anki-harness"
#: Paths the harness and the render generator may be built from. No workflow names one.
HARNESS_PATHS = ("tools/parity-oracle/anki/harness", "tools/render-oracle")
#: The sha256 of the harness's source tree (`tree_digest`). The run that adds the goldens pins
#: it here, in the same commit, so every golden's `generator_sha256` covers the harness. Until
#: then the driver refuses to start (exit 2).
HARNESS_SOURCE_SHA256: str | None = None
PROTOCOL = 1
MODULE_NAME = re.compile(r"spec_\d{3}\.py")
GOLDEN_NAME = re.compile(r"anki(?:[_.][a-z0-9]+)+")
CLASS_NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
REFUSAL_LABEL = re.compile(r"[a-z][a-z0-9_]{0,47}")
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
#: A registration's keys, all required.
REGISTRATION_KEYS = {"id", "classes", "note", "cases"}
#: A golden's keys, closed: no other key may appear.
GOLDEN_KEYS = {
    "schema",
    "kind",
    "function",
    "source_commit",
    "generator",
    "generator_sha256",
    "registry",
    "registry_sha256",
    "inputs",
    "seed",
    "note",
    "cases",
}
CASE_KEYS = {"input", "output", "class"}
#: A stock line shorter than this is too common to name the engine's text (a brace, a field
#: name), so the census counts only stripped lines of at least this many characters.
MIN_STOCK_LINE = 12
CENSUS_FILE = "stock-lines.sha256"

Case = tuple[str | None, dict]


class Refused(Exception):
    """Refused before the harness starts (exit 2)."""


class RunFailed(Exception):
    """The run failed after the harness started; nothing is written (exit 3)."""


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_digest(root: Path) -> str:
    """The sha256 of a source tree: each regular file's relative path and sha256, in path order."""
    lines = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise Refused(f"{path} is a symbolic link: a pinned source holds regular files only")
        if path.is_file():
            relative = path.relative_to(root).as_posix()
            lines.append(f"{relative}\0{sha256(path)}\n")
    if not lines:
        raise Refused(f"{root} holds no file to pin")
    return hashlib.sha256("".join(lines).encode("utf-8")).hexdigest()


def load_module(path: Path) -> ModuleType:
    name = f"anki_registry_{path.stem}"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_registry(directory: Path) -> dict[str, tuple[Path, dict]]:
    """Every registration in `directory`, in path order: golden name -> (module, registration)."""
    registered: dict[str, tuple[Path, dict]] = {}
    ids: dict[str, str] = {}
    for path in sorted(directory.glob("*.py")):
        where = f"registry/{path.name}"
        if not MODULE_NAME.fullmatch(path.name):
            raise Refused(f"{where} is not named spec_NNN.py: one module per SPEC")
        module = load_module(path)
        functions = getattr(module, "FUNCTIONS", None)
        seed = getattr(module, "SEED", None)
        if not isinstance(functions, dict):
            raise Refused(f"{where} exports no FUNCTIONS mapping")
        if not isinstance(seed, int) or isinstance(seed, bool):
            raise Refused(f"{where} exports no integer SEED")
        corpus = getattr(module, "CORPUS", None)
        corpus_path = Path(getattr(corpus, "__file__", "") or "")
        if not corpus_path.is_file():
            raise Refused(f"{where} exports no CORPUS module")
        if sha256(corpus_path) != getattr(module, "CORPUS_SHA256", None):
            raise Refused(
                f"{where}: CORPUS_SHA256 is not the digest of {corpus_path.name}; a corpus edit "
                "edits the pin too, which stales the goldens the module registered"
            )
        for name, registration in functions.items():
            if name in registered:
                first = f"registry/{registered[name][0].name}"
                raise Refused(f"golden {name!r} is registered by both {first} and {where}")
            check(where, name, registration)
            if registration["id"] in ids:
                raise Refused(
                    f"{where}: {name} takes id {registration['id']}, which "
                    f"{ids[registration['id']]} already holds"
                )
            ids[registration["id"]] = name
            covered(name, registration, seed)
            registered[name] = (path, registration)
    if not registered:
        raise Refused(f"{directory} registers no golden")
    return registered


def check(where: str, name: object, registration: object) -> None:
    """Refuse a registration that is not exactly the four keys, each of its type."""
    if not isinstance(name, str) or not GOLDEN_NAME.fullmatch(name):
        raise Refused(f"{where}: {name!r} is not a golden name (anki. then a-z, 0-9, '_', '.')")
    if not isinstance(registration, dict) or set(registration) != REGISTRATION_KEYS:
        given = sorted(registration) if isinstance(registration, dict) else registration
        raise Refused(
            f"{where}: a registration takes {', '.join(sorted(REGISTRATION_KEYS))}; "
            f"{name} gives {given}"
        )
    classes = registration["classes"]
    if not isinstance(classes, list) or not all(
        isinstance(c, str) and CLASS_NAME.fullmatch(c) for c in classes
    ):
        raise Refused(f"{where}: {name} declares classes that are not a list of class names")
    if len(set(classes)) != len(classes):
        raise Refused(f"{where}: {name} declares a class twice")
    if not (isinstance(registration["id"], str) and registration["id"].strip()):
        raise Refused(f"{where}: {name} has no id")
    if not (isinstance(registration["note"], str) and registration["note"].strip()):
        raise Refused(f"{where}: {name} has no note on what the harness calls")
    if not callable(registration["cases"]):
        raise Refused(f"{where}: {name} needs a case builder")


def draw(name: str, build: Callable[[random.Random], list[Case]], seed: int) -> list[Case]:
    """A builder's cases, drawn twice from `random.Random(seed)`. A second draw that differs is
    refused, because a builder that draws from anything else writes a golden nobody could
    regenerate, and so is a draw of nothing or an input that is not strict JSON."""
    first, second = build(random.Random(seed)), build(random.Random(seed))
    if first != second:
        raise Refused(f"{name}: its case builder draws from more than random.Random(SEED)")
    if not first:
        raise Refused(f"{name}: its case builder drew no case")
    for edge, arguments in first:
        if edge is not None and not (isinstance(edge, str) and CLASS_NAME.fullmatch(edge)):
            raise Refused(f"{name}: a case's class is {edge!r}, not a class name or None")
        if not isinstance(arguments, dict):
            raise Refused(f"{name}: a case's input is {arguments!r}, not an object")
        try:
            json.dumps(arguments, allow_nan=False)
        except (TypeError, ValueError) as error:
            raise Refused(f"{name}: a case's input is not strict JSON: {error}") from error
    return first


def covered(name: str, registration: dict, seed: int) -> list[Case]:
    """The registration's cases, refused unless every declared class has a case and every
    case's class is declared (edge-class coverage), and refused if a class holds one hand case
    twice, which would count a copy as a case."""
    cases = draw(name, registration["cases"], seed)
    declared = registration["classes"]
    drawn = {edge for edge, _ in cases if edge is not None}
    missing = [c for c in declared if c not in drawn]
    if missing:
        raise Refused(f"{name}: declared class(es) with no case: {', '.join(missing)}")
    stray = sorted(drawn - set(declared))
    if stray:
        raise Refused(f"{name}: case class(es) it does not declare: {', '.join(stray)}")
    seen = set()
    for edge, arguments in cases:
        if edge is None:
            continue
        key = (edge, json.dumps(arguments, sort_keys=True, ensure_ascii=False))
        if key in seen:
            raise Refused(
                f"{name}: class {edge} holds one input twice; a repeated case adds nothing"
            )
        seen.add(key)
    return cases


def writable_refusals(
    paths: Iterable[Path], writable: Callable[[Path], bool] | None = None
) -> list[str]:
    """Each source the run could write to. The sandbox mounts every source read-only, so a
    writable one means the run is not in the sandbox."""
    probe = writable or (lambda path: os.access(path, os.W_OK))
    refusals = []
    for root in paths:
        if not root.is_dir():
            refusals.append(f"{root} is not a directory")
            continue
        for path in [root, *sorted(root.rglob("*"))]:
            if not path.is_symlink() and probe(path):
                refusals.append(f"{path} is writable: the sandbox mounts every source read-only")
                break
    return refusals


def stock_hashes(texts: Iterable[str]) -> set[str]:
    """The sha256 of each stripped line of at least MIN_STOCK_LINE characters."""
    hashes = set()
    for text in texts:
        for line in text.splitlines():
            line = line.strip()
            if len(line) >= MIN_STOCK_LINE:
                hashes.add(hashlib.sha256(line.encode("utf-8")).hexdigest())
    return hashes


def strings(value: object) -> Iterable[str]:
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def census_refusals(name: str, golden: dict, hashes: set[str]) -> list[str]:
    """Each place in the golden that holds a stock line: the stock-text census."""
    refusals = []
    for index, case in enumerate(golden.get("cases", [])):
        for text in strings(case):
            for line in text.splitlines():
                line = line.strip()
                digest = hashlib.sha256(line.encode("utf-8")).hexdigest()
                if len(line) >= MIN_STOCK_LINE and digest in hashes:
                    refusals.append(f"{name}: case {index} holds stock line {digest[:16]}")
    return refusals


def strict_refusals(name: str, golden: object) -> list[str]:
    """Each way the golden falls short of strict `phx.parity-golden.v1` as this driver writes it:
    closed keys, the engine commit, digests, an integer seed, synthetic inputs and cases."""
    if not isinstance(golden, dict):
        return [f"{name}: is not an object"]
    refusals = []
    if set(golden) != GOLDEN_KEYS:
        refusals.append(
            f"{name}: keys {sorted(set(golden) ^ GOLDEN_KEYS)} differ from the schema's"
        )
    expected = {
        "schema": SCHEMA,
        "kind": "engine",
        "function": name,
        "generator": GENERATOR,
        "inputs": "synthetic",
    }
    for key, value in expected.items():
        if golden.get(key) != value:
            refusals.append(f"{name}: {key} is {golden.get(key)!r}, not {value!r}")
    if not (
        isinstance(golden.get("source_commit"), str) and HEX40.fullmatch(golden["source_commit"])
    ):
        refusals.append(f"{name}: source_commit is not a 40-hex commit")
    for key in ("generator_sha256", "registry_sha256"):
        if not (isinstance(golden.get(key), str) and HEX64.fullmatch(golden[key])):
            refusals.append(f"{name}: {key} is not a sha256")
    seed = golden.get("seed")
    if not isinstance(seed, int) or isinstance(seed, bool):
        refusals.append(f"{name}: seed is not an integer")
    cases = golden.get("cases")
    if not isinstance(cases, list) or not cases:
        refusals.append(f"{name}: holds no case")
        return refusals
    for index, case in enumerate(cases):
        if not isinstance(case, dict) or not {"input", "output"} <= set(case) <= CASE_KEYS:
            refusals.append(f"{name}: case {index} is not input, output and an optional class")
    return refusals


def provenance_refusals(name: str, golden: dict, root: Path) -> list[str]:
    """Each digest of the golden that differs from the committed file it names under `root`:
    the generator, which pins the harness, and the registry module."""
    refusals = []
    for path_key, digest_key in (
        ("generator", "generator_sha256"),
        ("registry", "registry_sha256"),
    ):
        path = root / str(golden.get(path_key, ""))
        if not path.is_file():
            refusals.append(f"{name}: {path_key} {golden.get(path_key)!r} is not a committed file")
        elif sha256(path) != golden.get(digest_key):
            refusals.append(f"{name}: {digest_key} differs from the committed {golden[path_key]}")
    return refusals


def workspace_refusals(root: Path) -> list[str]:
    """Each way the harness reaches DeckStreak's workspace or a workflow: a workspace member
    that covers a harness path, a root lock that names the harness's package, or a workflow
    that names either."""
    import tomllib

    refusals = []
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    members = manifest.get("workspace", {}).get("members", [])
    for member in members:
        for path in HARNESS_PATHS:
            parts = path.split("/")
            prefixes = ["/".join(parts[: i + 1]) for i in range(len(parts))]
            if any(
                fnmatch(prefix, member) or fnmatch(f"{prefix}/x", member) for prefix in prefixes
            ):
                refusals.append(f"Cargo.toml: workspace member {member!r} covers {path}")
    lock = (root / "Cargo.lock").read_text(encoding="utf-8")
    if re.search(rf'^name = "{re.escape(HARNESS_PACKAGE)}"$', lock, re.MULTILINE):
        refusals.append(f"Cargo.lock: names {HARNESS_PACKAGE}, the harness's own package")
    workflows = root / ".github" / "workflows"
    names = (*HARNESS_PATHS, HARNESS_PACKAGE, GENERATOR)
    for workflow in sorted(workflows.glob("*.y*ml")):
        text = workflow.read_text(encoding="utf-8")
        for needle in names:
            if needle in text:
                refusals.append(f".github/workflows/{workflow.name}: names {needle}")
    return refusals


class Harness:
    """The harness process, one JSON line each way."""

    def __init__(self, argv: list[str], work: Path) -> None:
        try:
            self.process = subprocess.Popen(
                argv,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                env={"PATH": os.environ.get("PATH", ""), "LC_ALL": "C.UTF-8"},
            )
        except OSError as error:
            raise RunFailed(f"the harness cannot start: {error.strerror}") from error
        self.work = work

    def ask(self, message: dict) -> dict:
        self.process.stdin.write(json.dumps(message, sort_keys=True) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise RunFailed(f"the harness closed its output after {message['op']}")
        try:
            answer = json.loads(line)
        except json.JSONDecodeError as error:
            raise RunFailed(
                f"the harness answered {message['op']} with a line that is not JSON"
            ) from error
        if not isinstance(answer, dict):
            raise RunFailed(f"the harness answered {message['op']} with {type(answer).__name__}")
        return answer

    def hello(self) -> tuple[str, list[str]]:
        answer = self.ask({"op": "hello", "protocol": PROTOCOL, "work": str(self.work)})
        if set(answer) != {"engine_commit", "stock_texts"}:
            raise RunFailed(f"the harness's hello carries {sorted(answer)}")
        texts = answer["stock_texts"]
        if not isinstance(texts, list) or not all(isinstance(t, str) for t in texts):
            raise RunFailed("the harness's stock_texts is not a list of texts")
        return answer["engine_commit"], texts

    def call(self, name: str, arguments: dict) -> object:
        answer = self.ask({"op": "call", "function": name, "input": arguments})
        if set(answer) == {"output"}:
            return answer["output"]
        if set(answer) == {"refused"}:
            label = answer["refused"]
            if not (isinstance(label, str) and REFUSAL_LABEL.fullmatch(label)):
                raise RunFailed(
                    f"{name}: a refusal must be a short label, never the engine's words"
                )
            return {"refused": label}
        raise RunFailed(f"{name}: the harness answered with {sorted(answer)}")

    def close(self) -> int:
        if self.process.poll() is None:
            try:
                self.process.stdin.write(json.dumps({"op": "bye"}) + "\n")
                self.process.stdin.close()
            except BrokenPipeError:
                pass
        try:
            return self.process.wait(timeout=60)
        finally:
            self.process.stdout.close()


def run(
    harness: list[str],
    registered: dict[str, tuple[Path, dict]],
    seed_of: Callable[[Path], int],
    generator_digest: str,
) -> tuple[dict[str, dict], set[str]]:
    """Every golden, and the stock-line hashes, from one harness session; nothing is written."""
    with tempfile.TemporaryDirectory(prefix="anki-oracle-work-") as work:
        session = Harness(harness, Path(work))
        try:
            commit, texts = session.hello()
            if commit != ENGINE_COMMIT:
                raise RunFailed(
                    f"the harness links engine {commit!r}, not the pinned {ENGINE_COMMIT}"
                )
            hashes = stock_hashes(texts)
            goldens = {}
            interpreter = f"{platform.python_implementation()} {platform.python_version()}"
            for name, (module, registration) in registered.items():
                seed = seed_of(module)
                cases = []
                for edge, arguments in covered(name, registration, seed):
                    recorded = json.loads(json.dumps(arguments, allow_nan=False))
                    output = session.call(name, recorded)
                    case = {
                        "input": recorded,
                        "output": json.loads(json.dumps(output, allow_nan=False)),
                    }
                    if edge is not None:
                        case["class"] = edge
                    cases.append(case)
                goldens[name] = {
                    "schema": SCHEMA,
                    "kind": "engine",
                    "function": name,
                    "source_commit": commit,
                    "generator": GENERATOR,
                    "generator_sha256": generator_digest,
                    "registry": f"{PUBLISHED_REGISTRY}/{module.name}",
                    "registry_sha256": sha256(module),
                    "inputs": "synthetic",
                    "seed": seed,
                    "note": (
                        f"{registration['note'].rstrip('.')} ({registration['id']}); "
                        f"cases drawn under {interpreter}."
                    ),
                    "cases": cases,
                }
        finally:
            rc = session.close()
        if rc != 0:
            raise RunFailed(f"the harness exited {rc}")
    refusals = [
        r for name, golden in goldens.items() for r in census_refusals(name, golden, hashes)
    ]
    if refusals:
        raise RunFailed("the stock-text census refuses: " + "; ".join(refusals[:20]))
    return goldens, hashes


def render(golden: dict) -> str:
    """Strict, canonical JSON: sorted keys, a two-space indent, and no NaN or Infinity."""
    return json.dumps(golden, indent=2, sort_keys=True, allow_nan=False, ensure_ascii=False) + "\n"


def write(out: Path, goldens: dict[str, dict], hashes: set[str]) -> list[Path]:
    """The goldens under `out/goldens/` and the census under `out/`: the run's only writes."""
    folder = out / "goldens"
    folder.mkdir(parents=True, exist_ok=True)
    written = []
    for name, golden in goldens.items():
        path = folder / f"{name}.json"
        path.write_text(render(golden), encoding="utf-8")
        written.append(path)
    census = out / CENSUS_FILE
    census.write_text("".join(f"{h}\n" for h in sorted(hashes)), encoding="utf-8")
    written.append(census)
    return written


def main(argv: list[str] | None = None, pin: str | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--harness", help="the harness binary, built in the sandbox from the pinned source"
    )
    parser.add_argument("--harness-source", help="the harness's source tree, mounted read-only")
    parser.add_argument(
        "--engine-source", help="the engine's checkout at the pin, mounted read-only"
    )
    parser.add_argument("--registry", default=str(REGISTRY), help="the registry directory")
    parser.add_argument("--out", help="the run's one writable output directory")
    parser.add_argument(
        "--digest-harness", metavar="DIR", help="print a harness source tree's digest and exit"
    )
    args = parser.parse_args(argv)
    # The run reads its own tree and the sources and never writes to them, not even bytecode.
    sys.dont_write_bytecode = True
    pinned = HARNESS_SOURCE_SHA256 if pin is None else pin
    try:
        if args.digest_harness:
            print(tree_digest(Path(args.digest_harness)))
            return 0
        if not (args.harness and args.harness_source and args.engine_source and args.out):
            raise Refused("--harness, --harness-source, --engine-source and --out are required")
        registered = load_registry(Path(args.registry))
        if pinned is None:
            raise Refused(
                "the harness is not pinned: HARNESS_SOURCE_SHA256 is set by the run that adds the goldens"
            )
        source = Path(args.harness_source)
        refusals = writable_refusals([source, Path(args.engine_source)])
        if refusals:
            raise Refused("; ".join(refusals))
        digest = tree_digest(source)
        if digest != pinned:
            raise Refused(f"the harness source's digest {digest} is not the pinned {pinned}")
    except Refused as refusal:
        print(f"generate.py: {refusal}", file=sys.stderr)
        return 2
    seeds = {module: load_module(module).SEED for module, _ in registered.values()}
    try:
        goldens, hashes = run([args.harness], registered, seeds.__getitem__, sha256(HERE))
    except (RunFailed, Refused) as failure:
        print(f"generate.py: {failure}", file=sys.stderr)
        return 3
    for path in write(Path(args.out), goldens, hashes):
        print(path)
    print(
        f"examined {sum(len(g['cases']) for g in goldens.values())} case(s) of {len(goldens)} function(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
