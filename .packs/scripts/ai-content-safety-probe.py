#!/usr/bin/env python3
"""ai-content-safety-probe.py -- judges how a repository feeds untrusted text to its AI and gates its output.

SPEC-V2-2222. A repository declares every AI task in `ai-safety.json` (schema `phx.ai.safety.v1`):
the system files and the prompt template it sends, the slot each input fills and where that input
comes from, the agent tools it holds, where its golden outputs live, and the checks its output must
pass before it is delivered. This script proves that declaration against the tree:

    prompt-contract           block     the manifest parses, every slot is declared and used, no
                                        prompt file is undeclared, no secret shape is in a prompt
    untrusted-fenced          block     every untrusted slot sits alone, JSON-encoded, in an
                                        <untrusted source="..."> fence that names its source, and
                                        never in a system file (OWASP LLM01:2025)
    policy-stated             block     a task that reads untrusted text states the untrusted-content
                                        policy in its system files
    agency-scoped             block     a task that reads untrusted text holds no open-ended or
                                        network tool, and its reads and writes are path-scoped;
                                        the agent's settings allow no more (OWASP LLM06:2025)
    redteam-present           block     injection cases cover every untrusted source and the three
                                        core attacks, and a test runs them
    output-gated              block     every task's gate names the classes its output must pass,
                                        persona-core's and law-professors' included, and a golden
                                        output exists to prove it on
    output-links              block     an output links only to allowed hosts over https, and embeds
                                        no remote image (exfiltration)
    output-invisible          block     an output (or an input, with --subject) carries no Unicode tag
                                        character, bidirectional control, zero-width or run of
                                        variation selectors
    output-marked             block     an output carries a machine-readable AI mark (EU AI Act
                                        Art. 50(2))
    output-echo               advisory  an output echoes no fence and no line of a system prompt
    disclosure-first-contact  block     every first-contact surface tells the learner they are talking
                                        to an AI (EU AI Act Art. 50(1) and (5))

It composes and never copies: persona-core's probe (`load_contract`, `load_deny`, `parse_document`,
`CLASSES`) and law-professors' probe (`CLASS_NAMES`) are loaded with importlib from beside this
script, or from `--persona-core` and `--law-professors`. It is VENDORABLE: standard library only
(Python >= 3.10), no import from phoenix-v2. The vocabulary lives in the pack's `vocabulary.json`.

`ai-safety.json` is found anywhere under `--root` (dot-directories, node_modules and target
skipped), or named by `--manifest`; each manifest is judged against the tree under its directory.
`--subject PATH` judges the named outputs (or inputs) instead of the golden outputs. Every class
prints one line per finding, `<class>: <finding>`, and ends with `examined N`. Exit 0 is green, 1 a
finding, 2 a usage error, and 3 VOID: nothing was examined or an input could not be read.
"""

import argparse
import importlib.util
import json
import os
import re
import sys
import unicodedata
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import urlsplit

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

MANIFEST_NAME = "ai-safety.json"
SCHEMA = "phx.ai.safety.v1"
VOCABULARY_SCHEMA = "phx.ai.vocabulary.v1"
REDTEAM_SCHEMA = "phx.ai.redteam.v1"
PERSONA_OUTPUT = "phx.persona.output.v1"
PACK_SLUG = "ai-content-safety"
HERE = Path(__file__).resolve().parent
DEFAULT_PACK_DIR = HERE.parent / "skills" / "packs" / PACK_SLUG
DEFAULT_PERSONA_CORE = HERE / "persona-core-probe.py"
DEFAULT_LAW_PROFESSORS = HERE / "law-professors-probe.py"
DENY_ENV = "PERSONA_CORE_DENY_LIST"
SKIPPED_DIRECTORIES = frozenset({"node_modules", "target"})
MAX_SCAN_BYTES = 5_000_000

CLASSES = (
    ("prompt-contract", "injection", "block"),
    ("untrusted-fenced", "injection", "block"),
    ("policy-stated", "injection", "block"),
    ("agency-scoped", "injection", "block"),
    ("redteam-present", "injection", "block"),
    ("output-gated", "output", "block"),
    ("output-links", "output", "block"),
    ("output-invisible", "output", "block"),
    ("output-marked", "output", "block"),
    ("output-echo", "output", "advisory"),
    ("disclosure-first-contact", "disclosure", "block"),
)
CLASS_NAMES = tuple(name for name, _stage, _severity in CLASSES)
# Classes that judge a population of outputs, which --subject may name without any manifest.
SUBJECT_ONLY = frozenset({"output-invisible", "output-marked"})

TOP_KEYS = frozenset({"schema", "tasks", "links", "disclosure", "redteam", "agent"})
TOP_REQUIRED = ("schema", "tasks", "links", "disclosure")
TASK_KEYS = frozenset(
    {
        "id",
        "format",
        "kind",
        "system",
        "prompt",
        "inputs",
        "tools",
        "outputs",
        "gate",
        "on_invalid",
    }
)
TASK_REQUIRED = (
    "id",
    "format",
    "system",
    "prompt",
    "inputs",
    "tools",
    "outputs",
    "gate",
    "on_invalid",
)
AGENT_KEYS = frozenset({"settings", "mcp_offline"})

SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
SLOT_NAME = re.compile(r"^[a-z][a-z0-9_]*$")
SLOT = re.compile(
    r"\{\{\s*(?P<name>[a-z][a-z0-9_]*)\s*(?:\|\s*(?P<filter>[a-z]+)\s*)?\}\}"
)
ANY_BRACES = re.compile(r"\{\{(?P<inner>[^{}\n]*)\}\}")
FENCE_OPEN = re.compile(r'^\s*<untrusted\s+source="(?P<source>[a-z0-9-]+)"\s*>\s*$')
FENCE_CLOSE = re.compile(r"^\s*</untrusted>\s*$")
FENCE_ANY = re.compile(r"</?untrusted\b")
POLICY_BLOCK = re.compile(
    r"<untrusted_content_policy>(?P<body>.*?)</untrusted_content_policy>", re.DOTALL
)
TOOL = re.compile(r"^(?P<name>[A-Za-z_][A-Za-z0-9_]*)(?:\((?P<spec>.*)\))?$")
GATE = re.compile(
    r"^(?P<pack>[a-z0-9]+(?:-[a-z0-9]+)*):(?P<cls>[a-z0-9]+(?:-[a-z0-9]+)*)$"
)
KEY_LINE = re.compile(r"^(?P<key>[a-z][a-z0-9_]*|x-[a-z0-9-]+):[ ]+(?P<value>\S.*)$")
MD_LINK = re.compile(r"(?P<bang>!?)\[[^\]\n]*\]\(\s*<?(?P<url>[^)\s>]+)>?")
HTML_LINK = re.compile(
    r"<(?P<tag>[a-zA-Z]+)\b[^>]*?\b(?P<attr>href|src)\s*=\s*[\"'](?P<url>[^\"']+)[\"']"
)
AUTOLINK = re.compile(r"<(?P<url>[a-zA-Z][a-zA-Z0-9+.-]*:[^>\s]+)>")
BARE_URL = re.compile(
    r"(?<![\w@/])(?P<url>(?:https?|ftp|tg|javascript|file)://[^\s<>()\[\]\"'`]+)",
    re.IGNORECASE,
)
SCRIPT_URL = re.compile(
    r"(?<![\w-])(?P<url>(?:javascript|vbscript|data):[^\s<>()\"'`]+)", re.IGNORECASE
)
TOKEN_AI = "AI"


class Void(Exception):
    """An input the class needs could not be read: the class is VOID, never green."""


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass
class Manifest:
    path: Path
    base: Path
    data: "dict | None" = None
    error: "str | None" = None

    def usable(self) -> bool:
        return isinstance(self.data, dict) and self.data.get("schema") == SCHEMA

    def tasks(self) -> list:
        value = self.data.get("tasks") if isinstance(self.data, dict) else None
        return (
            [t for t in value if isinstance(t, dict)] if isinstance(value, list) else []
        )

    def block(self, key: str) -> dict:
        value = self.data.get(key) if isinstance(self.data, dict) else None
        return value if isinstance(value, dict) else {}


@dataclass
class Context:
    root: Path
    manifests: list
    subjects: list
    vocabulary: dict
    pack_dir: Path
    persona_core: Path
    law_professors: Path
    deny_list: "Path | None"
    _modules: dict = field(default_factory=dict)
    _texts: dict = field(default_factory=dict)

    def usable(self) -> list:
        return [m for m in self.manifests if m.usable()]

    def show(self, path: Path) -> str:
        try:
            return path.resolve().relative_to(self.root.resolve()).as_posix()
        except ValueError:
            return str(path)

    def text(self, path: Path) -> str:
        key = path.resolve()
        if key not in self._texts:
            try:
                self._texts[key] = path.read_text(encoding="utf-8", errors="replace")
            except OSError as error:
                raise Void(
                    f"{self.show(path)}: unreadable ({error.strerror or error})"
                ) from error
        return self._texts[key]

    def module(self, name: str, path: Path) -> object:
        if name not in self._modules:
            self._modules[name] = load_module(name, path)
        return self._modules[name]

    def core(self) -> object:
        return self.module("persona_core_probe", self.persona_core)

    def untrusted(self) -> set:
        return set(self.vocabulary["untrusted_sources"])


# --- reading ----------------------------------------------------------------------------------------


def load_module(name: str, path: Path) -> object:
    """A sibling pack's probe, loaded and registered so its dataclasses resolve their module."""
    if not path.is_file():
        raise Void(f"{path} is not there; this class composes with it")
    spec = importlib.util.spec_from_file_location(f"{name}_for_ai_content_safety", path)
    if spec is None or spec.loader is None:
        raise Void(f"{path} cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except (OSError, SyntaxError, ImportError) as error:
        raise Void(f"{path} cannot be loaded ({error})") from error
    return module


def load_vocabulary(pack_dir: Path) -> dict:
    path = pack_dir / "vocabulary.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"{path}: the vocabulary cannot be read ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != VOCABULARY_SCHEMA:
        raise Void(f"{path}: schema is not {VOCABULARY_SCHEMA}")
    return data


def discover(root: Path) -> list:
    found = []
    for directory, subdirectories, files in os.walk(root):
        subdirectories[:] = sorted(
            name
            for name in subdirectories
            if not name.startswith(".") and name not in SKIPPED_DIRECTORIES
        )
        if MANIFEST_NAME in files:
            found.append(Path(directory) / MANIFEST_NAME)
    return sorted(found)


def load_manifest(path: Path) -> Manifest:
    manifest = Manifest(path=path, base=path.parent)
    try:
        manifest.data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        manifest.error = f"unreadable ({error.strerror or error})"
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        manifest.error = f"not JSON ({error})"
    return manifest


def relative_file(manifest: Manifest, value: object) -> "Path | None":
    if not isinstance(value, str) or not value or Path(value).is_absolute():
        return None
    if ".." in Path(value).parts:
        return None
    return manifest.base / value


def globbed(manifest: Manifest, patterns: object) -> list:
    found: dict = {}
    for pattern in patterns if isinstance(patterns, list) else []:
        if not isinstance(pattern, str) or not pattern or Path(pattern).is_absolute():
            continue
        if ".." in Path(pattern).parts:
            continue
        for path in manifest.base.glob(pattern):
            if path.is_file():
                found[path.resolve()] = path
    return [found[key] for key in sorted(found)]


def text_files(bases: Iterable) -> list:
    found: dict = {}
    for base in bases:
        base = Path(base)
        if base.is_file():
            found[base.resolve()] = base
            continue
        for directory, subdirectories, files in os.walk(base):
            subdirectories[:] = sorted(
                n
                for n in subdirectories
                if not n.startswith(".") and n not in SKIPPED_DIRECTORIES
            )
            for name in sorted(files):
                path = Path(directory) / name
                found[path.resolve()] = path
    return [found[key] for key in sorted(found)]


def frontmatter(text: str) -> "dict | None":
    """The single-line-JSON frontmatter (persona-core's format), or None when there is none."""
    lines = text.removeprefix("﻿").split("\n")
    if not lines or lines[0].rstrip("\r") != "---":
        return None
    found: dict = {}
    for line in lines[1:]:
        line = line.rstrip("\r")
        if line == "---":
            return found
        match = KEY_LINE.match(line)
        if not match:
            return None
        try:
            found[match["key"]] = json.loads(match["value"])
        except json.JSONDecodeError:
            return None
    return None


def slot_sources(task: dict) -> dict:
    inputs = task.get("inputs")
    found = {}
    for item in inputs if isinstance(inputs, list) else []:
        if isinstance(item, dict) and isinstance(item.get("slot"), str):
            found[item["slot"]] = item.get("source")
    return found


def task_files(manifest: Manifest, task: dict) -> tuple:
    """(system files, prompt file) of a task, as paths (None when a name is unusable)."""
    system = task.get("system")
    systems = (
        [relative_file(manifest, s) for s in system] if isinstance(system, list) else []
    )
    return systems, relative_file(manifest, task.get("prompt"))


def outputs_of(ctx: Context) -> list:
    """[(manifest or None, path)] of the population an output class judges."""
    if ctx.subjects:
        manifest = ctx.usable()[0] if ctx.usable() else None
        return [(manifest, path) for path in text_files(ctx.subjects)]
    found = []
    for manifest in ctx.usable():
        seen = set()
        for task in manifest.tasks():
            for path in globbed(manifest, task.get("outputs")):
                if path.resolve() not in seen:
                    seen.add(path.resolve())
                    found.append((manifest, path))
    return found


def readable_text(ctx: Context, path: Path) -> "str | None":
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise Void(
            f"{ctx.show(path)}: unreadable ({error.strerror or error})"
        ) from error
    if len(raw) > MAX_SCAN_BYTES or b"\0" in raw[:8192]:
        return None
    return raw.decode("utf-8", errors="replace")


# --- the classes ------------------------------------------------------------------------------------


def check_prompt_contract(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    sources = set(vocabulary["untrusted_sources"]) | set(vocabulary["trusted_sources"])
    filters = set(vocabulary["filters"])
    findings = []
    examined = 0
    for manifest in ctx.manifests:
        where = ctx.show(manifest.path)
        examined += 1
        if manifest.error:
            findings.append(f"{where}: {manifest.error}")
            continue
        data = manifest.data
        if not isinstance(data, dict) or data.get("schema") != SCHEMA:
            findings.append(f"{where}: schema is not {SCHEMA}")
            continue
        for key in sorted(set(data) - TOP_KEYS):
            findings.append(f"{where}: unknown key {key}")
        for key in TOP_REQUIRED:
            if key not in data:
                findings.append(f"{where}: {key} is missing")
        links = data.get("links")
        if "links" in data and not (
            isinstance(links, dict)
            and isinstance(links.get("allow"), list)
            and all(isinstance(h, str) and h and "/" not in h for h in links["allow"])
        ):
            findings.append(f"{where}: links is not {{allow: [host, ...]}}")
        agent = data.get("agent")
        if agent is not None and not (
            isinstance(agent, dict) and set(agent) <= AGENT_KEYS
        ):
            findings.append(f"{where}: agent is not {{settings, mcp_offline}}")
        tasks = data.get("tasks")
        if not isinstance(tasks, list) or not tasks:
            findings.append(f"{where}: tasks is not a non-empty list")
            continue
        kinds = None
        declared_prompts = set()
        seen_ids = set()
        for index, task in enumerate(tasks):
            if not isinstance(task, dict):
                findings.append(f"{where}: task {index} is not an object")
                continue
            label = task.get("id", index)
            for key in sorted(set(task) - TASK_KEYS):
                findings.append(f"{where}: task {label}: unknown key {key}")
            for key in TASK_REQUIRED:
                if key not in task:
                    findings.append(f"{where}: task {label}: {key} is missing")
            if not isinstance(task.get("id"), str) or not SLUG.match(task["id"]):
                findings.append(f"{where}: task {label}: id is not a slug")
            elif task["id"] in seen_ids:
                findings.append(f"{where}: duplicate task id {task['id']}")
            else:
                seen_ids.add(task["id"])
            form = task.get("format")
            if form not in vocabulary["formats"]:
                findings.append(
                    f"{where}: task {label}: format {form} is not persona or other"
                )
            if form == "persona" or "kind" in task:
                if kinds is None:
                    kinds = list(ctx.core().load_contract()["kinds"])
                if task.get("kind") not in kinds:
                    findings.append(
                        f"{where}: task {label}: kind {task.get('kind')} is not one of persona-core's "
                        f"kinds ({', '.join(kinds)})"
                    )
            for key in ("tools", "outputs", "gate"):
                value = task.get(key)
                if key in task and not (
                    isinstance(value, list)
                    and all(isinstance(v, str) and v for v in value)
                ):
                    findings.append(
                        f"{where}: task {label}: {key} is not a list of strings"
                    )
            if task.get("on_invalid") not in vocabulary["on_invalid"]:
                findings.append(
                    f"{where}: task {label}: on_invalid {task.get('on_invalid')} is not "
                    f"{', '.join(vocabulary['on_invalid'])}"
                )
            inputs = task.get("inputs")
            slots = slot_sources(task)
            if not isinstance(inputs, list) or len(slots) != len(inputs):
                findings.append(
                    f"{where}: task {label}: inputs are not a list of {{slot, source}}"
                )
            names = (
                [i.get("slot") for i in inputs if isinstance(i, dict)]
                if isinstance(inputs, list)
                else []
            )
            for name in sorted({n for n in names if names.count(n) > 1}):
                findings.append(f"{where}: task {label}: slot {name} is declared twice")
            for name, source in slots.items():
                if not SLOT_NAME.match(name):
                    findings.append(
                        f"{where}: task {label}: slot {name} is not a slot name"
                    )
                if source not in sources:
                    findings.append(
                        f"{where}: task {label}: slot {name} has source {source}, which is not a known source"
                    )
            systems, prompt = task_files(manifest, task)
            files = [*systems, prompt]
            if not isinstance(task.get("system"), list) or not task.get("system"):
                findings.append(
                    f"{where}: task {label}: system is not a non-empty list of files"
                )
            used: set = set()
            system_names = (
                task.get("system") if isinstance(task.get("system"), list) else []
            )
            for name, path in zip([*system_names, task.get("prompt")], files):
                if path is None or not path.is_file():
                    findings.append(f"{where}: task {label}: {name} does not exist")
                    continue
                declared_prompts.add(path.resolve())
                text = ctx.text(path)
                for number, line in enumerate(text.split("\n"), 1):
                    for match in ANY_BRACES.finditer(line):
                        slot = SLOT.fullmatch(match.group(0))
                        at = f"{ctx.show(path)}:{number}"
                        if slot is None:
                            findings.append(f"{at}: {match.group(0)} is not a slot")
                            continue
                        used.add(slot["name"])
                        if slot["name"] not in slots:
                            findings.append(
                                f"{at}: slot {slot['name']} is not declared in task {label}"
                            )
                        if slot["filter"] and slot["filter"] not in filters:
                            findings.append(
                                f"{at}: filter {slot['filter']} is not json"
                            )
            for name in sorted(set(slots) - used):
                findings.append(
                    f"{where}: task {label}: slot {name} is declared but never used"
                )
            findings += secret_shapes(
                ctx, [p for p in files if p is not None and p.is_file()]
            )
        for path in sorted(manifest.base.rglob("*.prompt.md")):
            if any(
                part.startswith(".") or part in SKIPPED_DIRECTORIES
                for part in path.relative_to(manifest.base).parts[:-1]
            ):
                continue
            if path.resolve() not in declared_prompts:
                findings.append(f"{ctx.show(path)}: a prompt file no task declares")
    return Outcome(examined, tuple(findings))


def secret_shapes(ctx: Context, paths: list) -> list:
    """persona-core's deny shapes (public, plus private when named) in prompt and system files."""
    core = ctx.core()
    try:
        deny = core.load_deny(None, ctx.deny_list)
    except ValueError as error:
        raise Void(f"a deny list cannot be read ({error})") from error
    findings = []
    for path in paths:
        for number, line in enumerate(ctx.text(path).split("\n"), 1):
            folded = unicodedata.normalize("NFKC", line)
            for origin, row_id, pattern in deny["patterns"]:
                if pattern.search(folded):
                    findings.append(
                        f"{ctx.show(path)}:{number}: {origin} pattern {row_id} in a prompt "
                        "(OWASP LLM07:2025: keep secrets out of prompts)"
                    )
            lowered = folded.casefold()
            for index, (_origin, literal) in enumerate(deny["literals"]):
                if literal in lowered:
                    findings.append(
                        f"{ctx.show(path)}:{number}: private literal {index} in a prompt"
                    )
    return findings


def check_untrusted_fenced(ctx: Context) -> Outcome:
    untrusted = ctx.untrusted()
    findings = []
    examined = 0
    for manifest in ctx.usable():
        for task in manifest.tasks():
            examined += 1
            label = task.get("id")
            slots = slot_sources(task)
            systems, prompt = task_files(manifest, task)
            for path in systems:
                if path is None or not path.is_file():
                    continue
                for number, line in enumerate(ctx.text(path).split("\n"), 1):
                    for match in SLOT.finditer(line):
                        if slots.get(match["name"]) in untrusted:
                            findings.append(
                                f"{ctx.show(path)}:{number}: untrusted slot {match['name']} is in a "
                                "system file; untrusted text never enters the system prompt"
                            )
            if prompt is None or not prompt.is_file():
                continue
            where = ctx.show(prompt)
            fence = None
            body: list = []
            for number, line in enumerate(ctx.text(prompt).split("\n"), 1):
                opened = FENCE_OPEN.match(line)
                closed = FENCE_CLOSE.match(line)
                if opened:
                    if fence is not None:
                        findings.append(
                            f"{where}:{number}: a fence opens inside another fence"
                        )
                    fence, body = (number, opened["source"]), []
                    continue
                if closed:
                    if fence is None:
                        findings.append(
                            f"{where}:{number}: a fence closes that never opened"
                        )
                    else:
                        findings += judge_fence(
                            where, fence, body, slots, untrusted, label
                        )
                    fence = None
                    continue
                if FENCE_ANY.search(line):
                    findings.append(
                        f"{where}:{number}: a fence tag that is not on a line of its own"
                    )
                    continue
                if fence is not None:
                    body.append((number, line))
                    continue
                for match in SLOT.finditer(line):
                    if slots.get(match["name"]) in untrusted:
                        findings.append(
                            f"{where}:{number}: untrusted slot {match['name']} is outside a fence "
                            "(OWASP LLM01:2025: segregate and identify external content)"
                        )
            if fence is not None:
                findings.append(f"{where}:{fence[0]}: a fence is never closed")
    return Outcome(examined, tuple(findings))


def judge_fence(
    where: str, fence: tuple, body: list, slots: dict, untrusted: set, label: object
) -> list:
    number, source = fence
    lines = [(n, line) for n, line in body if line.strip()]
    content = " ".join(line.strip() for _n, line in lines)
    match = SLOT.fullmatch(content) if content else None
    if match is None:
        return [
            f"{where}:{number}: the fence must hold only its slot, JSON-encoded; instructions stay "
            "outside untrusted text"
        ]
    found = []
    declared = slots.get(match["name"])
    if declared not in untrusted:
        found.append(
            f"{where}:{number}: slot {match['name']} is not an untrusted input of task {label}"
        )
    elif declared != source:
        found.append(
            f"{where}:{number}: the fence says source {source}, but slot {match['name']} comes from "
            f"{declared}"
        )
    if match["filter"] != "json":
        found.append(
            f"{where}:{number}: slot {match['name']} is not JSON-encoded ({{{{{match['name']}|json}}}}), "
            "so its text could close the fence"
        )
    return found


def check_policy_stated(ctx: Context) -> Outcome:
    untrusted = ctx.untrusted()
    findings = []
    examined = 0
    for manifest in ctx.usable():
        where = ctx.show(manifest.path)
        for task in manifest.tasks():
            examined += 1
            if not set(slot_sources(task).values()) & untrusted:
                continue
            systems, _prompt = task_files(manifest, task)
            if not any(
                states_policy(ctx, path)
                for path in systems
                if path is not None and path.is_file()
            ):
                findings.append(
                    f"{where}: task {task.get('id')} reads untrusted text, and no system file states "
                    "the untrusted-content policy (an <untrusted_content_policy> block or "
                    "persona-core's learner-text rule)"
                )
    return Outcome(examined, tuple(findings))


def states_policy(ctx: Context, path: Path) -> bool:
    text = ctx.text(path)
    if any(m["body"].strip() for m in POLICY_BLOCK.finditer(text)):
        return True
    if "phx.persona.rules" not in text:
        return False
    core = ctx.core()
    try:
        document = core.parse_document(path)
    except ValueError:
        return False
    return document.kind == "rules" and any(
        section.id == "learner-text" and section.body.strip()
        for section in document.sections
    )


def output_dirs(task: dict) -> list:
    dirs = []
    for pattern in task.get("outputs") if isinstance(task.get("outputs"), list) else []:
        if not isinstance(pattern, str):
            continue
        parts = []
        for part in Path(pattern).parts:
            if any(ch in part for ch in "*?["):
                break
            parts.append(part)
        if parts and parts != list(Path(pattern).parts):
            dirs.append("/".join(parts))
        elif parts:
            dirs.append("/".join(parts[:-1]))
    return [d for d in dirs if d]


def judge_tool(tool: str, task: dict, vocabulary: dict, offline: set) -> "str | None":
    """Why a tool is too wide for a task that reads untrusted text, or None when it is scoped."""
    match = TOOL.match(tool.strip())
    if match is None:
        return f"{tool} is not a tool name"
    name, spec = match["name"], match["spec"]
    spec = spec.strip() if isinstance(spec, str) else None
    unscoped = set(vocabulary["unscoped_paths"])
    if name.startswith("mcp__"):
        return (
            None
            if tool in offline
            else f"{tool} is an MCP tool not declared in agent.mcp_offline"
        )
    if name in vocabulary["open_tools"]:
        return f"{tool} is an open-ended tool (it fetches, searches or delegates)"
    if name in vocabulary["list_tools"]:
        return None
    if name in vocabulary["read_tools"]:
        if spec is None or spec in unscoped:
            return f"{tool} reads any file; scope it to the paths the task needs"
        return None
    if name in vocabulary["write_tools"]:
        if spec is None or spec in unscoped:
            return f"{tool} writes anywhere; scope it to the task's output directory"
        if not any(
            spec == d or spec.startswith(d.rstrip("/") + "/") for d in output_dirs(task)
        ):
            return f"{tool} writes outside the task's outputs"
        return None
    if name in vocabulary["shell_tools"]:
        if spec is None or spec in {"", "*", ":*"}:
            return f"{tool} runs any command"
        command = re.split(r"[\s:]", spec, maxsplit=1)[0]
        program = command.rsplit("/", 1)[-1]
        if program in vocabulary["egress_programs"]:
            return f"{tool} runs {program}, which can reach the network or run code"
        return None
    return f"{tool} is not a tool this pack knows is safe beside untrusted text"


def check_agency_scoped(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    untrusted = ctx.untrusted()
    findings = []
    examined = 0
    for manifest in ctx.usable():
        where = ctx.show(manifest.path)
        agent = manifest.block("agent")
        offline = (
            {t for t in agent.get("mcp_offline", []) if isinstance(t, str)}
            if isinstance(agent.get("mcp_offline"), list)
            else set()
        )
        granted: set = set()
        for task in manifest.tasks():
            examined += 1
            tools = (
                [t for t in task.get("tools", []) if isinstance(t, str)]
                if isinstance(task.get("tools"), list)
                else []
            )
            granted |= set(tools)
            reads = sorted(set(slot_sources(task).values()) & untrusted)
            if not reads:
                continue
            for tool in tools:
                why = judge_tool(tool, task, vocabulary, offline)
                if why:
                    findings.append(
                        f"{where}: task {task.get('id')} reads untrusted {', '.join(reads)}; {why} "
                        "(OWASP LLM06:2025)"
                    )
        settings = (
            relative_file(manifest, agent.get("settings"))
            if agent.get("settings")
            else None
        )
        if settings is None:
            continue
        examined += 1
        shown = ctx.show(settings)
        try:
            data = json.loads(ctx.text(settings))
        except json.JSONDecodeError as error:
            findings.append(f"{shown}: the agent's settings are not JSON ({error})")
            continue
        permissions = data.get("permissions") if isinstance(data, dict) else None
        permissions = permissions if isinstance(permissions, dict) else {}
        for tool in (
            permissions.get("allow", [])
            if isinstance(permissions.get("allow"), list)
            else []
        ):
            if tool not in granted:
                findings.append(
                    f"{shown}: the settings allow {tool}, which no task in {where} declares; the "
                    "agent must not hold more than its tasks"
                )
        if permissions.get("defaultMode") == "bypassPermissions":
            findings.append(
                f"{shown}: defaultMode bypassPermissions lets every tool run unchecked"
            )
    return Outcome(examined, tuple(findings))


def check_redteam_present(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    untrusted = ctx.untrusted()
    findings = []
    examined = 0
    for manifest in ctx.usable():
        where = ctx.show(manifest.path)
        sources = sorted(
            {s for task in manifest.tasks() for s in slot_sources(task).values()}
            & untrusted
        )
        examined += 1
        if not sources:
            continue
        redteam = manifest.block("redteam")
        cases_dir = relative_file(manifest, redteam.get("cases"))
        if not redteam or cases_dir is None or not cases_dir.is_dir():
            findings.append(
                f"{where}: tasks read untrusted {', '.join(sources)}, and no redteam cases directory "
                "exists (OWASP LLM01:2025: adversarial testing)"
            )
            continue
        covered: set = set()
        attacks: set = set()
        for path in sorted(cases_dir.rglob("*.md")):
            examined += 1
            text = ctx.text(path)
            meta = frontmatter(text)
            shown = ctx.show(path)
            if meta is None or meta.get("schema") != REDTEAM_SCHEMA:
                findings.append(
                    f"{shown}: a redteam case needs {REDTEAM_SCHEMA} frontmatter"
                )
                continue
            body = (
                text.split("\n---", 1)[-1].split("\n", 1)[-1] if "\n---" in text else ""
            )
            problems = []
            if meta.get("source") not in untrusted:
                problems.append(
                    f"source {meta.get('source')} is not an untrusted source"
                )
            if meta.get("attack") not in vocabulary["attacks"]:
                problems.append(
                    f"attack {meta.get('attack')} is not one of {', '.join(vocabulary['attacks'])}"
                )
            if meta.get("expect") not in vocabulary["expects"]:
                problems.append(
                    f"expect {meta.get('expect')} is not {' or '.join(vocabulary['expects'])}"
                )
            if not body.strip():
                problems.append("the case has no adversarial text")
            if problems:
                findings += [f"{shown}: {problem}" for problem in problems]
                continue
            covered.add(meta["source"])
            attacks.add(meta["attack"])
        for source in sources:
            if source not in covered:
                findings.append(f"{where}: no redteam case feeds untrusted {source}")
        for attack in vocabulary["required_attacks"]:
            if attack not in attacks:
                findings.append(f"{where}: no redteam case tries {attack}")
        tests = redteam.get("tests")
        if not isinstance(tests, list) or not tests:
            findings.append(f"{where}: redteam.tests names no test that runs the cases")
            continue
        needles = {str(redteam.get("cases")).rstrip("/"), cases_dir.name}
        for name in tests:
            path = relative_file(manifest, name)
            examined += 1
            if path is None or not path.is_file():
                findings.append(f"{where}: redteam test {name} does not exist")
            elif not any(n and n in ctx.text(path) for n in needles):
                findings.append(
                    f"{ctx.show(path)}: the redteam test never names the cases directory"
                )
    return Outcome(examined, tuple(findings))


def check_output_gated(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    required = vocabulary["required_gates"]
    findings = []
    examined = 0
    known: dict = {PACK_SLUG: set(CLASS_NAMES)}
    for manifest in ctx.usable():
        where = ctx.show(manifest.path)
        for task in manifest.tasks():
            examined += 1
            label = task.get("id")
            gate = (
                [g for g in task.get("gate", []) if isinstance(g, str)]
                if isinstance(task.get("gate"), list)
                else []
            )
            for entry in gate:
                match = GATE.match(entry)
                if match is None:
                    findings.append(
                        f"{where}: task {label}: gate entry {entry} is not pack:class"
                    )
                    continue
                classes = gate_classes(ctx, match["pack"], known)
                if classes is not None and match["cls"] not in classes:
                    findings.append(
                        f"{where}: task {label}: {entry} is not a class of {match['pack']}"
                    )
            needed = list(required["all"])
            if task.get("format") == "persona":
                needed += required["persona"]
            if task.get("kind") == "law":
                needed += required["law"]
            for entry in needed:
                if entry not in gate:
                    findings.append(
                        f"{where}: task {label}: the gate lacks {entry}, so an output that fails it "
                        "could be delivered (OWASP LLM05:2025)"
                    )
            if task.get("on_invalid") not in vocabulary["on_invalid"]:
                findings.append(
                    f"{where}: task {label}: on_invalid {task.get('on_invalid')} would deliver an "
                    f"output that failed its gate; use {', '.join(vocabulary['on_invalid'])}"
                )
            if not globbed(manifest, task.get("outputs")):
                findings.append(
                    f"{where}: task {label}: no golden output matches {task.get('outputs')}, so the "
                    "gate is proved on nothing"
                )
    return Outcome(examined, tuple(findings))


def gate_classes(ctx: Context, pack: str, known: dict) -> "set | None":
    if pack not in known:
        if pack == "persona-core":
            known[pack] = {
                c if isinstance(c, str) else c[0] for c in ctx.core().CLASSES
            }
        elif pack == "law-professors":
            law = ctx.module("law_professors_probe", ctx.law_professors)
            known[pack] = set(getattr(law, "CLASS_NAMES", ()))
        else:
            known[pack] = None
    return known[pack]


def urls(text: str) -> list:
    """[(line, url, is_image)] for every link, image and bare URL in an output."""
    found = []
    for number, line in enumerate(text.split("\n"), 1):
        spans = []
        for match in MD_LINK.finditer(line):
            found.append((number, match["url"], bool(match["bang"])))
            spans.append(match.span("url"))
        for match in HTML_LINK.finditer(line):
            found.append((number, match["url"], match["tag"].lower() == "img"))
            spans.append(match.span("url"))
        for match in AUTOLINK.finditer(line):
            found.append((number, match["url"], False))
            spans.append(match.span("url"))
        for pattern in (BARE_URL, SCRIPT_URL):
            for match in pattern.finditer(line):
                start, end = match.span("url")
                if any(s <= start < e for s, e in spans):
                    continue
                found.append((number, match["url"].rstrip(".,;:!?"), False))
    return found


def check_output_links(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    allow = sorted(
        {
            h.lower()
            for m in ctx.usable()
            for h in m.block("links").get("allow", [])
            if isinstance(h, str)
        }
    )
    for _manifest, path in outputs_of(ctx):
        text = readable_text(ctx, path)
        if text is None:
            continue
        examined += 1
        shown = ctx.show(path)
        for number, url, image in urls(text):
            parts = urlsplit(url)
            scheme = parts.scheme.lower()
            host = (parts.hostname or "").lower()
            at = f"{shown}:{number}"
            if not scheme:
                continue
            if image and scheme in {"http", "https", "ftp"}:
                findings.append(
                    f"{at}: a remote image loads by itself and can carry data out; embed none"
                )
            elif scheme != "https":
                findings.append(
                    f"{at}: a {scheme} link is not https to an allowed host"
                )
            elif not any(host == h or host.endswith("." + h) for h in allow):
                findings.append(f"{at}: a link to {host} is not on links.allow")
    return Outcome(examined, tuple(findings))


def in_ranges(point: int, ranges: list) -> bool:
    return any(int(low, 16) <= point <= int(high, 16) for low, high in ranges)


def check_output_invisible(ctx: Context) -> Outcome:
    rules = ctx.vocabulary["invisible"]
    allowed = {int(code, 16) for code in rules["allowed"]}
    selectors = rules["variation_selectors"]
    longest = int(rules["max_variation_run"])
    findings = []
    examined = 0
    for _manifest, path in outputs_of(ctx):
        text = readable_text(ctx, path)
        if text is None:
            continue
        examined += 1
        shown = ctx.show(path)
        for number, line in enumerate(text.split("\n"), 1):
            reported: set = set()
            run = 0
            for index, char in enumerate(line):
                point = ord(char)
                if in_ranges(point, selectors):
                    run += 1
                    if run == longest + 1:
                        findings.append(
                            f"{shown}:{number}: a run of variation selectors hides data"
                        )
                    continue
                run = 0
                if unicodedata.category(char) != "Cf" or point in allowed:
                    continue
                if point == 0xFEFF and number == 1 and index == 0:
                    continue
                if point not in reported:
                    reported.add(point)
                    name = unicodedata.name(char, "unnamed")
                    findings.append(
                        f"{shown}:{number}: invisible U+{point:04X} ({name}) can hide instructions "
                        "(OWASP LLM01:2025)"
                    )
    return Outcome(examined, tuple(findings))


def check_output_marked(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    for _manifest, path in outputs_of(ctx):
        text = readable_text(ctx, path)
        if text is None:
            continue
        examined += 1
        shown = ctx.show(path)
        if f'"{PERSONA_OUTPUT}"' in text.split("\n---", 1)[0]:
            try:
                document = ctx.core().parse_document(path)
            except ValueError as error:
                findings.append(
                    f"{shown}: a persona output that does not parse ({error})"
                )
                continue
            if document.kind != "output":
                findings.append(
                    f"{shown}: schema names a persona {document.kind}, not an output"
                )
            continue
        meta = frontmatter(text)
        if meta is None or meta.get("ai_generated") is not True:
            findings.append(
                f"{shown}: no machine-readable AI mark: persona-core's output schema or "
                "ai_generated: true (EU AI Act Art. 50(2))"
            )
    return Outcome(examined, tuple(findings))


def check_output_echo(ctx: Context) -> Outcome:
    markers = ctx.vocabulary["fence_markers"]
    shortest = int(ctx.vocabulary["echo_min_chars"])
    system_lines: dict = {}
    for manifest in ctx.usable():
        for task in manifest.tasks():
            systems, _prompt = task_files(manifest, task)
            for path in systems:
                if path is not None and path.is_file():
                    for line in ctx.text(path).split("\n"):
                        if len(line.strip()) >= shortest:
                            system_lines.setdefault(line.strip(), ctx.show(path))
    findings = []
    examined = 0
    for _manifest, path in outputs_of(ctx):
        text = readable_text(ctx, path)
        if text is None:
            continue
        examined += 1
        shown = ctx.show(path)
        for number, line in enumerate(text.split("\n"), 1):
            for marker in markers:
                if marker in line:
                    findings.append(
                        f"{shown}:{number}: the output carries {marker}, a prompt's fence"
                    )
                    break
            source = system_lines.get(line.strip())
            if source:
                findings.append(
                    f"{shown}:{number}: the output repeats a line of {source} (OWASP LLM07:2025)"
                )
    return Outcome(examined, tuple(findings))


def says_ai(text: str, token: str) -> bool:
    return re.search(rf"(?<![A-Za-z]){re.escape(token)}(?![A-Za-z])", text) is not None


def check_disclosure_first_contact(ctx: Context) -> Outcome:
    token = str(ctx.core().load_contract().get("disclosure_token", TOKEN_AI))
    findings = []
    examined = 0
    for manifest in ctx.usable():
        where = ctx.show(manifest.path)
        disclosure = manifest.block("disclosure")
        surfaces = disclosure.get("surfaces")
        if not isinstance(surfaces, list) or not surfaces:
            findings.append(
                f"{where}: disclosure.surfaces names no first-contact surface"
            )
            continue
        text = disclosure.get("text")
        key = disclosure.get("key")
        if isinstance(text, str) and text.strip():
            if not says_ai(text, token):
                findings.append(
                    f"{where}: the disclosure never says {token} (EU AI Act Art. 50(1))"
                )
            needle = " ".join(text.split())
        elif isinstance(key, str) and key:
            needle = key
            messages = globbed(manifest, disclosure.get("messages"))
            if not messages:
                findings.append(
                    f"{where}: disclosure.messages matches no messages file"
                )
            for path in messages:
                examined += 1
                try:
                    data = json.loads(ctx.text(path))
                except json.JSONDecodeError as error:
                    findings.append(f"{ctx.show(path)}: not JSON ({error})")
                    continue
                value = data
                for part in key.split("."):
                    value = value.get(part) if isinstance(value, dict) else None
                if not isinstance(value, str) or not says_ai(value, token):
                    findings.append(
                        f"{ctx.show(path)}: {key} is missing or never says {token} (EU AI Act Art. 50(1))"
                    )
        else:
            findings.append(f"{where}: disclosure names neither text nor key")
            continue
        for name in surfaces:
            path = relative_file(manifest, name)
            examined += 1
            if path is None or not path.is_file():
                findings.append(f"{where}: disclosure surface {name} does not exist")
            elif needle not in " ".join(ctx.text(path).split()):
                findings.append(
                    f"{ctx.show(path)}: the first-contact surface never shows the disclosure, so the "
                    "learner is not told at the first interaction (EU AI Act Art. 50(5))"
                )
    return Outcome(examined, tuple(findings))


CHECKS = {
    "prompt-contract": check_prompt_contract,
    "untrusted-fenced": check_untrusted_fenced,
    "policy-stated": check_policy_stated,
    "agency-scoped": check_agency_scoped,
    "redteam-present": check_redteam_present,
    "output-gated": check_output_gated,
    "output-links": check_output_links,
    "output-invisible": check_output_invisible,
    "output-marked": check_output_marked,
    "output-echo": check_output_echo,
    "disclosure-first-contact": check_disclosure_first_contact,
}


# --- the command line -------------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ai-content-safety-probe.py",
        description="Judge how a repository feeds and gates its AI (SPEC-V2-2222).",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--manifest",
        action="append",
        default=[],
        help="an ai-safety.json to judge; repeatable",
    )
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="an output (or input) file or directory to judge instead of the golden outputs",
    )
    parser.add_argument(
        "--persona-core", help="persona-core's probe (default: beside this script)"
    )
    parser.add_argument(
        "--law-professors", help="law-professors' probe (default: beside this script)"
    )
    parser.add_argument(
        "--deny-list",
        help=f"persona-core's private deny list (default: ${DENY_ENV}, if set)",
    )
    parser.add_argument(
        "--pack-dir",
        help=f"this pack's data directory (default: skills/packs/{PACK_SLUG})",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASS_NAMES)
    commands.add_parser(
        "classes", help="list the classes with their stage and severity"
    )
    return parser


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        for name, stage, severity in CLASSES:
            print(f"{name} {stage} {severity}")
        return EXIT_GREEN
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"ai-content-safety-probe: --root is not a directory: {args.root}",
            file=sys.stderr,
        )
        return EXIT_USAGE
    for value, label in [(m, "--manifest") for m in args.manifest] + [
        (s, "--subject") for s in args.subject
    ]:
        if not Path(value).exists():
            print(
                f"ai-content-safety-probe: {label} does not exist: {value}",
                file=sys.stderr,
            )
            return EXIT_USAGE
    private = args.deny_list or os.environ.get(DENY_ENV) or None
    name = args.name
    try:
        pack_dir = Path(args.pack_dir) if args.pack_dir else DEFAULT_PACK_DIR
        ctx = Context(
            root=root,
            manifests=[
                load_manifest(Path(p)) for p in (args.manifest or discover(root))
            ],
            subjects=[Path(s) for s in args.subject],
            vocabulary=load_vocabulary(pack_dir),
            pack_dir=pack_dir,
            persona_core=Path(args.persona_core)
            if args.persona_core
            else DEFAULT_PERSONA_CORE,
            law_professors=Path(args.law_professors)
            if args.law_professors
            else DEFAULT_LAW_PROFESSORS,
            deny_list=Path(private) if private else None,
        )
        subject_only = name in SUBJECT_ONLY and ctx.subjects
        if not ctx.manifests and not subject_only:
            raise Void(f"no {MANIFEST_NAME} under {root}")
        if name != "prompt-contract" and not ctx.usable() and not subject_only:
            raise Void(f"no {MANIFEST_NAME} under {root} parses as {SCHEMA}")
        outcome = CHECKS[name](ctx)
    except Void as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    if outcome.examined == 0:
        print(f"{name}: VOID: examined nothing")
        print("examined 0")
        return EXIT_VOID
    print(f"examined {outcome.examined}")
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())
