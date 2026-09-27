#!/usr/bin/env python3
"""nudge-duties-probe.py -- the executable output contract for DeckStreak's nudge duties.

SPEC-V2-2217. Two agent duties write a Telegram message: the DAILY DIGEST (deterministic stats plus
AI coaching) and the ONE-PER-LAPSE COMEBACK. The engine stores each message it is about to send as
a `*.msg.json` envelope, `phx.duty.message.v1`: the literal Bot API sendMessage payload under
`send`, the policy metadata the notifications-policy pack reads (duty, kind, tier, budget_key,
dedupe_key, lapse_id, reading_id), and the provenance of every part of the text under `parts`.
This script judges those envelopes. What it reads is what Telegram would receive.

    message-contract   block     the envelope: keys, vocabularies, parts, the payload they join to
    telegram-length    block     1-4096 UTF-16 units of visible text after entity parsing
    telegram-entities  block     every markup problem telegram-platform's parse_text names
    telegram-keyboard  block     every keyboard problem telegram-platform's check_keyboard names
    digest-numbers     block     every number in the stats part equals its source; none is unsourced
    digest-degraded    block     a failed AI step says so plainly; coaching is never silently missing
    no-internals       block     no host, port, path, stack trace, error body or credential name
    coaching-numbers   advisory  a number in the coaching that no stat in the source holds
    comeback-shape     block     one comeback reading per lapse; one url or web_app button
    readings-paused    block     no readings part in a digest while a lapse is open
    no-dates-or-countdowns  block  no date, deadline or countdown; no Bot API date_time entity
    no-shame-framing   block     no guilt, shame, loss framing or confirmshaming
    no-journal-in-messages  block   no link, embed, tag or verbatim run from the journal
    autonomy-language  advisory  controlling language: should, must, have to
    shouting           advisory  block capitals and exclamation runs
    near-miss-copy     advisory  near-miss copy outside the deterministic stats part

It is VENDORABLE on purpose: standard library only (Python >= 3.10), no import from phoenix-v2,
and every check reads any tree through `--root`, or the paths `--subject` names. The vocabularies
live in the pack's contract.json and the phrase tables in its patterns.json, so a new duty, tag,
button field or phrase is a data row, never a code branch.

REUSE, never copy: the calendar-date and timeline tables, the public scrubber shapes and the
per-line NFKC scan are persona-core's (`scripts/persona-core-probe.py`, its public
`load_patterns`, `load_deny` and `scan`); the journal rule is vault-duties' (`load_journal`,
`journal_leaks`); and Telegram message validation, visible text included, is telegram-platform's
(`parse_text`, `check_message`, `check_keyboard`, `utf16_len`). A class that needs a sibling and cannot
load it is VOID.

A file is CLAIMED when its name ends `.msg.json`. A claimed file that does not parse is a finding in
every class, never a skip. Every class prints one line per finding, `<class>: <finding>`, and ends
with `examined N`. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: nothing was examined
or an input could not be read, which is never a pass.
"""

import argparse
import importlib.util
import json
import os
import re
import sys
import unicodedata
from collections.abc import Sequence
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

MESSAGE_SCHEMA = "phx.duty.message.v1"
CONTRACT_SCHEMA = "phx.duty.nudge.contract.v1"
PATTERNS_SCHEMA = "phx.duty.nudge.patterns.v1"
SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_PACK_DIR = SCRIPT_DIR.parent / "skills" / "packs" / "nudge-duties"
DEFAULT_PERSONA_CORE = SCRIPT_DIR / "persona-core-probe.py"
DEFAULT_VAULT_DUTIES = SCRIPT_DIR / "vault-duties-probe.py"
DEFAULT_TELEGRAM_PLATFORM = SCRIPT_DIR / "telegram-platform-probe.py"
SKIPPED_DIRECTORIES = frozenset({"node_modules", "target"})
PATTERN_GROUPS = (
    "countdowns",
    "shame",
    "internals",
    "degraded_notice",
    "controlling",
    "near_miss",
)
CONTRACT_KEYS = (
    "message_schema",
    "claim_suffix",
    "part_separator",
    "token",
    "stat_key",
    "keys",
    "duties",
    "tiers",
    "coaching",
    "send",
    "keyboard",
)
EXTENSION_KEY = re.compile(r"^x-[a-z0-9]+(?:-[a-z0-9]+)*$")
NUMBER = re.compile(
    r"(?<![0-9A-Za-z_.,])[+-]?(?:[0-9]{1,3}(?:,[0-9]{3})+|[0-9]+)(?:\.[0-9]+)?(?![0-9A-Za-z_])"
)
WORD = re.compile(r"[A-Za-z]+")
PERSONA_CORE_API = ("load_patterns", "load_deny", "scan")
VAULT_DUTIES_API = ("load_journal", "journal_leaks")
TELEGRAM_PLATFORM_API = ("parse_text", "check_message", "check_keyboard", "utf16_len")
#: The entities that hide their text until the reader acts: a notice inside one is not plain.
HIDING = {"spoiler": "a spoiler", "expandable_blockquote": "an expandable quote"}


class ContractError(ValueError):
    """A fatal defect: a file cannot be read as a message envelope, or the pack data is unusable."""


@dataclass(frozen=True)
class Part:
    role: str
    text: str


@dataclass(frozen=True)
class Message:
    """One parsed envelope. `envelope` is the decoded JSON object, in file order."""

    path: Path
    envelope: dict
    duty: str
    parts: tuple
    send: dict
    text: str
    parse_mode: str | None

    def roles(self) -> list:
        return [part.role for part in self.parts]

    def part(self, role: str) -> "Part | None":
        for part in self.parts:
            if part.role == role:
                return part
        return None


@dataclass(frozen=True)
class Broken:
    path: Path
    reason: str


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass(frozen=True)
class Reading:
    """A text as its reader sees it, parsed by telegram-platform's `parse_text`."""

    visible: str
    unhidden: str
    problems: tuple
    hiders: tuple
    date_times: tuple
    links: tuple


def _load_json(path: Path, schema: str) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ContractError(
            f"{path}: unreadable ({error.strerror or error})"
        ) from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ContractError(f"{path}: not JSON ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != schema:
        raise ContractError(f"{path}: schema is not {schema}")
    return data


def load_contract(pack_dir: "Path | None" = None) -> dict:
    """The pack's contract.json: the envelope's vocabularies and the Telegram grammar it relies on."""
    contract = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "contract.json", CONTRACT_SCHEMA
    )
    for key in CONTRACT_KEYS:
        if key not in contract:
            raise ContractError(f"contract.json: {key} is missing")
    return contract


def _compiled(row: object, where: str) -> tuple:
    if not isinstance(row, dict) or not isinstance(row.get("regex"), str):
        raise ContractError(f"{where}: a row has no regex")
    flags = re.IGNORECASE if "i" in str(row.get("flags", "")) else 0
    try:
        return (str(row.get("id", "?")), re.compile(row["regex"], flags))
    except re.error as error:
        raise ContractError(f"{where}: {row.get('id')}: {error}") from error


def load_patterns(pack_dir: "Path | None" = None) -> dict:
    """The pack's patterns.json, every regex compiled into persona-core's (id, pattern) row shape."""
    raw = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "patterns.json", PATTERNS_SCHEMA
    )
    patterns: dict = {}
    for group in PATTERN_GROUPS:
        rows = raw.get(group)
        if not isinstance(rows, list) or not rows:
            raise ContractError(f"patterns.json: {group} is missing or empty")
        patterns[group] = [_compiled(row, f"patterns.json {group}") for row in rows]
    acronyms = raw.get("acronyms")
    if not isinstance(acronyms, list):
        raise ContractError("patterns.json: acronyms is missing")
    patterns["acronyms"] = frozenset(str(word) for word in acronyms)
    return patterns


def _load_sibling(path: Path, label: str, why: str, api: tuple):
    """A sibling pack's probe, loaded the way its contract says (registered in sys.modules)."""
    if not path.is_file():
        raise ContractError(f"the {label} probe is not at {path}; {why}")
    name = f"{label.replace('-', '_')}_probe_for_nudge_duties_{abs(hash(str(path.resolve())))}"
    if name in sys.modules:
        return sys.modules[name]
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ContractError(f"the {label} probe at {path} cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as error:
        del sys.modules[name]
        raise ContractError(
            f"the {label} probe at {path} failed to load ({error})"
        ) from error
    for attribute in api:
        if not callable(getattr(module, attribute, None)):
            del sys.modules[name]
            raise ContractError(
                f"the {label} probe at {path} has no public {attribute}()"
            )
    return module


def load_persona_core(path: "Path | str | None" = None):
    """persona-core's probe: its date and timeline rows, public scrubber shapes and scan()."""
    return _load_sibling(
        Path(path or DEFAULT_PERSONA_CORE),
        "persona-core",
        "its date tables are required",
        PERSONA_CORE_API,
    )


def load_vault_duties(path: "Path | str | None" = None):
    """vault-duties' probe: load_journal() and journal_leaks(), the one home of the journal rule."""
    return _load_sibling(
        Path(path or DEFAULT_VAULT_DUTIES),
        "vault-duties",
        "its journal rule is required",
        VAULT_DUTIES_API,
    )


def load_telegram_platform(path: "Path | str | None" = None):
    """telegram-platform's probe: `parse_text`, `check_message`, `check_keyboard` and `utf16_len`,
    the one home of Telegram message validation."""
    return _load_sibling(
        Path(path or DEFAULT_TELEGRAM_PLATFORM),
        "telegram-platform",
        "its payload validator is required",
        TELEGRAM_PLATFORM_API,
    )


def claimed(path: "Path | str", suffix: str = ".msg.json") -> bool:
    """Whether `path` claims to be a message envelope: its name ends `.msg.json`."""
    return Path(path).name.endswith(suffix)


def discover(bases: Sequence, suffix: str = ".msg.json") -> list:
    """Every claimed file under `bases`, sorted; dot-directories, node_modules and target skipped."""
    found: dict = {}
    for base in bases:
        base = Path(base)
        if base.is_file():
            if claimed(base, suffix):
                found[base.resolve()] = base
            continue
        for directory, subdirectories, files in os.walk(base):
            subdirectories[:] = sorted(
                name
                for name in subdirectories
                if not name.startswith(".") and name not in SKIPPED_DIRECTORIES
            )
            for name in sorted(files):
                if claimed(name, suffix):
                    candidate = Path(directory) / name
                    found[candidate.resolve()] = candidate
    return [found[key] for key in sorted(found)]


def _reject_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON")


def load_message(path: "Path | str") -> Message:
    """Parse one envelope. Raises ContractError, with the reason, when it cannot be read at all."""
    path = Path(path)
    try:
        text = path.read_bytes().decode("utf-8")
    except OSError as error:
        raise ContractError(f"unreadable ({error.strerror or error})") from error
    except UnicodeDecodeError as error:
        raise ContractError(f"not UTF-8 ({error.reason})") from error
    try:
        envelope = json.loads(text, parse_constant=_reject_constant)
    except (json.JSONDecodeError, ValueError) as error:
        raise ContractError(f"not JSON ({error})") from error
    if not isinstance(envelope, dict):
        raise ContractError("not a JSON object")
    if envelope.get("schema") != MESSAGE_SCHEMA:
        raise ContractError(f"schema is not {MESSAGE_SCHEMA}")
    parts = []
    for entry in (
        envelope.get("parts") if isinstance(envelope.get("parts"), list) else []
    ):
        if isinstance(entry, dict) and isinstance(entry.get("role"), str):
            text_value = entry.get("text")
            parts.append(
                Part(entry["role"], text_value if isinstance(text_value, str) else "")
            )
    send = envelope.get("send") if isinstance(envelope.get("send"), dict) else {}
    body = send.get("text") if isinstance(send.get("text"), str) else ""
    mode = send.get("parse_mode") if isinstance(send.get("parse_mode"), str) else None
    duty = envelope.get("duty") if isinstance(envelope.get("duty"), str) else ""
    return Message(path, envelope, duty, tuple(parts), send, body, mode)


def _other_duty(path: Path, duties: "frozenset | None") -> bool:
    """Whether `path` is a readable envelope of a duty this pack does not own. The envelope is
    every DeckStreak outbound message's (a celebration too), and nudge-duties judges only the
    daily digest and the comeback; a file nothing can read is still judged, never skipped."""
    if duties is None:
        return False
    try:
        envelope = json.loads(path.read_bytes().decode("utf-8"))
    except (OSError, UnicodeDecodeError, ValueError):
        return False
    if not isinstance(envelope, dict):
        return False
    duty = envelope.get("duty")
    return isinstance(duty, str) and duty != "" and duty not in duties


def load_population(
    bases: Sequence, suffix: str = ".msg.json", duties: "frozenset | None" = None
) -> tuple:
    messages, broken = [], []
    for path in discover(bases, suffix):
        if _other_duty(path, duties):
            continue
        try:
            messages.append(load_message(path))
        except ContractError as error:
            broken.append(Broken(path, str(error)))
    return messages, broken


def _unhidden(plain: str, hidden: list) -> str:
    """`plain` without the characters inside `hidden`, a list of UTF-16 [start, end) ranges."""
    kept = []
    unit = 0
    for character in plain:
        if not any(start <= unit < end for start, end in hidden):
            kept.append(character)
        unit += 2 if ord(character) > 0xFFFF else 1
    return "".join(kept)


def read_text(telegram_platform, text: str, parse_mode: "str | None") -> Reading:
    """What the reader sees, and every problem Telegram would answer `can't parse entities` for,
    from telegram-platform's `parse_text`; this probe keeps no Telegram parser of its own."""
    parsed = telegram_platform.parse_text(text, parse_mode, None)
    hidden = [
        (entity.offset, entity.offset + entity.length)
        for entity in parsed.entities
        if entity.type in HIDING
    ]
    hiders = tuple(
        dict.fromkeys(
            HIDING[entity.type] for entity in parsed.entities if entity.type in HIDING
        )
    )
    date_times = tuple(
        (entity.offset, entity.date_time_format)
        for entity in parsed.entities
        if entity.type == "date_time"
    )
    links = tuple(
        entity.url
        for entity in parsed.entities
        if entity.type == "text_link" and entity.url
    )
    return Reading(
        parsed.plain,
        _unhidden(parsed.plain, hidden),
        tuple(parsed.problems),
        hiders,
        date_times,
        links,
    )


def visible_text(text: str, parse_mode: "str | None", telegram_platform=None) -> str:
    """What the reader sees: telegram-platform's `parse_text`, plain text for no parse mode."""
    return read_text(
        telegram_platform or load_telegram_platform(), text, parse_mode
    ).visible


def numbers(text: str) -> list:
    """(line, start, end, token, value) for every number on the NFKC-normalised lines of `text`."""
    found = []
    for line_number, line in enumerate(text.split("\n"), start=1):
        folded = unicodedata.normalize("NFKC", line)
        for match in NUMBER.finditer(folded):
            token = match.group(0)
            try:
                value = Decimal(token.replace(",", ""))
            except InvalidOperation:
                continue
            found.append((line_number, match.start(), match.end(), token, value))
    return found


def _number(value: object) -> "Decimal | None":
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    try:
        return Decimal(str(value))
    except InvalidOperation:
        return None


def _resolve(source: object, key: str) -> object:
    node = source
    for segment in key.split("."):
        if not isinstance(node, dict) or segment not in node:
            return None
        node = node[segment]
    return node


def _leaves(node: object) -> list:
    if isinstance(node, dict):
        return [leaf for value in node.values() for leaf in _leaves(value)]
    if isinstance(node, list):
        return [leaf for value in node for leaf in _leaves(value)]
    number = _number(node)
    return [number] if number is not None else []


def _article(word: str) -> str:
    return "an" if word[:1] in "aeiou" else "a"


class Context:
    """One run: the population, the pack data, and persona-core, loaded only by the classes that
    need it, so the classes that do not are never VOID for its absence."""

    def __init__(
        self,
        root,
        messages,
        broken,
        contract,
        patterns,
        persona_core_path=None,
        vault_duties_path=None,
        vault=None,
        telegram_platform_path=None,
    ):
        self.root = root
        self.messages = messages
        self.broken = broken
        self.contract = contract
        self.patterns = patterns
        self.persona_core_path = persona_core_path
        self.vault_duties_path = vault_duties_path
        self.vault = vault
        self._persona_core = None
        self._vault_duties = None
        self.telegram_platform_path = telegram_platform_path
        self._telegram_platform = None
        self._reading: dict = {}

    def show(self, path: Path) -> str:
        try:
            return str(Path(path).resolve().relative_to(Path(self.root).resolve()))
        except ValueError:
            return str(path)

    def persona_core(self):
        if self._persona_core is None:
            self._persona_core = load_persona_core(self.persona_core_path)
        return self._persona_core

    def vault_duties(self):
        if self._vault_duties is None:
            self._vault_duties = load_vault_duties(self.vault_duties_path)
        return self._vault_duties

    @staticmethod
    def _sibling_call(label: str, what: str, call):
        """A sibling raises ITS OWN error class when its pack data is missing (a vendored copy
        without its data): that is VOID here, named, never a traceback."""
        try:
            return call()
        except ContractError:
            raise
        except Exception as error:
            raise ContractError(
                f"the {label} probe's {what}() failed: {error}"
            ) from error

    def shared_patterns(self) -> dict:
        return self._sibling_call(
            "persona-core", "load_patterns", lambda: self.persona_core().load_patterns()
        )

    def public_deny(self) -> dict:
        return self._sibling_call(
            "persona-core",
            "load_deny",
            lambda: self.persona_core().load_deny(None, None),
        )

    def journal(self):
        return self._sibling_call(
            "vault-duties",
            "load_journal",
            lambda: self.vault_duties().load_journal(
                self.vault,
                persona_core=Path(self.persona_core_path or DEFAULT_PERSONA_CORE),
            ),
        )

    def telegram_platform(self):
        if self._telegram_platform is None:
            self._telegram_platform = load_telegram_platform(
                self.telegram_platform_path
            )
        return self._telegram_platform

    def reading(self, message: Message) -> Reading:
        key = str(message.path)
        if key not in self._reading:
            self._reading[key] = read_text(
                self.telegram_platform(), message.text, message.parse_mode
            )
        return self._reading[key]

    def part_reading(self, message: Message, part: Part) -> Reading:
        return read_text(self.telegram_platform(), part.text, message.parse_mode)

    def duty(self, duty: str) -> list:
        return [message for message in self.messages if message.duty == duty]

    def broken_findings(self) -> list:
        return [f"{self.show(broken.path)}: {broken.reason}" for broken in self.broken]


def _buttons(message: Message) -> list:
    """(label, button) for every button of an inline keyboard; nothing for any other markup."""
    markup = message.send.get("reply_markup")
    if not isinstance(markup, dict) or not isinstance(
        markup.get("inline_keyboard"), list
    ):
        return []
    found = []
    for row_number, row in enumerate(markup["inline_keyboard"], start=1):
        if not isinstance(row, list):
            continue
        for column, button in enumerate(row, start=1):
            if isinstance(button, dict):
                found.append((f"button {row_number}.{column}", button))
    return found


def _button_strings(message: Message) -> list:
    """(label, text) for every button text and URL a reader can see."""
    strings = []
    for label, button in _buttons(message):
        if isinstance(button.get("text"), str):
            strings.append((label, button["text"]))
        if isinstance(button.get("url"), str):
            strings.append((f"{label} url", button["url"]))
        web_app = button.get("web_app")
        if isinstance(web_app, dict) and isinstance(web_app.get("url"), str):
            strings.append((f"{label} url", web_app["url"]))
    return strings


def _contract_problems(message: Message, contract: dict) -> list:
    envelope = message.envelope
    token = re.compile(contract["token"])
    keys = contract["keys"]
    duties = {duty["id"]: duty for duty in contract["duties"]}
    known = set(keys["always"]) | set(keys["optional"])
    for duty in duties.values():
        known |= set(duty["required"])
    problems = []
    for key in envelope:
        if key not in known and not EXTENSION_KEY.match(key):
            problems.append(f"unknown key {key}")
    for key in keys["always"]:
        if key not in envelope:
            problems.append(f"missing key {key}")
    duty_id = envelope.get("duty")
    spec = duties.get(duty_id) if isinstance(duty_id, str) else None
    if spec is None:
        if "duty" in envelope:
            problems.append(f"duty {duty_id} is not one of {', '.join(duties)}")
        return problems
    if "kind" in envelope and envelope["kind"] != spec["kind"]:
        problems.append(
            f"kind {envelope['kind']} is not the kind of {duty_id} ({spec['kind']})"
        )
    if "tier" in envelope and envelope["tier"] not in contract["tiers"]:
        problems.append(
            f"tier {envelope['tier']} is not one of {', '.join(contract['tiers'])}"
        )
    for key in keys["tokens"]:
        if key in envelope and not (
            isinstance(envelope[key], str) and token.match(envelope[key])
        ):
            problems.append(f"{key} is not a token ({contract['token']})")
    for key in spec["required"]:
        if key not in envelope:
            problems.append(f"missing key {key}")
    for other in duties.values():
        if other is spec:
            continue
        for key in other["required"]:
            if (
                key in envelope
                and key not in spec["required"]
                and key not in keys["optional"]
            ):
                problems.append(
                    f"{key} belongs to {_article(other['id'])} {other['id']}"
                )
    if "coaching" in spec["required"] and "coaching" in envelope:
        if envelope["coaching"] not in contract["coaching"]:
            problems.append(
                f"coaching {envelope['coaching']} is not one of {', '.join(contract['coaching'])}"
            )
    if "stats_source" in spec["required"] and "stats_source" in envelope:
        problems += _stats_source_problems(envelope["stats_source"])
    if "stats" in spec["required"] and "stats" in envelope:
        problems += _stats_problems(envelope["stats"], contract)
    problems += _parts_problems(envelope.get("parts"), duty_id, spec)
    problems += _send_problems(envelope.get("send"), envelope.get("parts"), contract)
    return problems


def _stats_source_problems(source: object) -> list:
    if (
        not isinstance(source, str)
        or not source
        or source.startswith("/")
        or "\\" in source
        or ".." in source.split("/")
    ):
        return ["stats_source must be a relative path inside the tree"]
    return []


def _stats_problems(stats: object, contract: dict) -> list:
    if not isinstance(stats, list) or not stats:
        return ["stats must be a non-empty list"]
    stat_key = re.compile(contract["stat_key"])
    problems, keys, labels = [], set(), set()
    for index, entry in enumerate(stats, start=1):
        if (
            not isinstance(entry, dict)
            or set(entry) != {"key", "label"}
            or not isinstance(entry.get("key"), str)
            or not stat_key.match(entry["key"])
            or not isinstance(entry.get("label"), str)
            or not entry["label"].strip()
        ):
            problems.append(
                f"stats entry {index} needs a key and a label, and nothing else"
            )
            continue
        key, label = entry["key"], entry["label"]
        if key in keys:
            problems.append(f"stats key {key} repeats")
        if label in labels:
            problems.append(f"stats label {label} repeats")
        if any(character.isdigit() for character in label):
            problems.append(f"stats label {label} holds a digit")
        keys.add(key)
        labels.add(label)
    return problems


def _parts_problems(parts: object, duty_id: str, spec: dict) -> list:
    if not isinstance(parts, list) or not parts:
        return ["parts must be a non-empty list"]
    problems, seen = [], []
    for index, entry in enumerate(parts, start=1):
        if not isinstance(entry, dict) or set(entry) != {"role", "text"}:
            problems.append(f"part {index} needs a role and a text, and nothing else")
            continue
        role, text = entry["role"], entry["text"]
        if role not in spec["roles"]:
            problems.append(f"part {index} role {role} is not a role of {duty_id}")
        elif role in seen:
            problems.append(f"part role {role} repeats")
        if not isinstance(text, str) or not text.strip():
            problems.append(f"part {index} text is empty")
        seen.append(role)
    for role in spec["required_roles"]:
        if role not in seen:
            problems.append(
                f"{_article(duty_id)} {duty_id} needs {_article(role)} {role} part"
            )
    first = spec.get("first_role")
    if first and seen and first in seen and seen[0] != first:
        problems.append(f"the {first} part comes first")
    return problems


def _send_problems(send: object, parts: object, contract: dict) -> list:
    if not isinstance(send, dict):
        return ["send must be an object"]
    rules = contract["send"]
    problems = []
    for key in send:
        if key in rules["refused"]:
            problems.append(f"send.{key}: {rules['refused'][key]}")
        elif key not in rules["keys"]:
            problems.append(f"send key {key} is not admitted")
    text = send.get("text")
    if not isinstance(text, str) or not text:
        problems.append("send.text must be a non-empty string")
    mode = send.get("parse_mode")
    if mode is not None and mode not in rules["parse_modes"]:
        problems.append(
            f"send.parse_mode {mode}: the contract admits "
            f"{' or '.join(rules['parse_modes'])} or plain text"
        )
    markup = send.get("reply_markup")
    if markup is not None and not isinstance(markup, dict):
        problems.append("send.reply_markup must be an object")
    if isinstance(parts, list) and all(
        isinstance(entry, dict) and isinstance(entry.get("text"), str)
        for entry in parts
    ):
        joined = contract["part_separator"].join(entry["text"] for entry in parts)
        if isinstance(text, str) and text != joined:
            problems.append("send.text is not the parts joined by a blank line")
    return problems


def check_message_contract(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    for message in ctx.messages:
        for problem in _contract_problems(message, ctx.contract):
            findings.append(f"{ctx.show(message.path)}: {problem}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_telegram_length(ctx: Context) -> Outcome:
    """telegram-platform's `check_message` on the text alone: its length and emptiness rules. The
    markup problems it also names are telegram-entities' findings, so they are not repeated here."""
    findings = ctx.broken_findings()
    for message in ctx.messages:
        payload = {"text": message.text, "parse_mode": message.parse_mode}
        markup = set(ctx.reading(message).problems)
        for problem in ctx.telegram_platform().check_message(payload):
            if problem not in markup:
                findings.append(f"{ctx.show(message.path)}: {problem}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_telegram_entities(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    for message in ctx.messages:
        for problem in ctx.reading(message).problems:
            findings.append(f"{ctx.show(message.path)}: {problem}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_telegram_keyboard(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    for message in ctx.messages:
        markup = message.send.get("reply_markup")
        if markup is None:
            continue
        for problem in ctx.telegram_platform().check_keyboard(markup):
            findings.append(f"{ctx.show(message.path)}: {problem}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def _stats_source(message: Message) -> "tuple[object, str | None]":
    relative = message.envelope.get("stats_source")
    if not isinstance(relative, str) or _stats_source_problems(relative):
        return (
            None,
            "the digest names no usable stats_source; message-contract says why",
        )
    try:
        source = json.loads(
            (message.path.parent / relative).read_text(encoding="utf-8")
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError):
        return None, f"stats source {relative} is unreadable"
    if not isinstance(source, dict):
        return None, f"stats source {relative} is unreadable: not a JSON object"
    return source, None


def _stat_findings(ctx: Context, message: Message) -> list:
    where = ctx.show(message.path)
    declared = message.envelope.get("stats")
    if not isinstance(declared, list) or _stats_problems(declared, ctx.contract):
        return [
            f"{where}: the digest declares no usable stats; message-contract says why"
        ]
    source, problem = _stats_source(message)
    if problem:
        return [f"{where}: {problem}"]
    part = message.part("stats")
    if part is None:
        return [f"{where}: the digest has no stats part"]
    lines = [
        unicodedata.normalize("NFKC", line)
        for line in ctx.part_reading(message, part).visible.split("\n")
    ]
    tokens = numbers("\n".join(lines))
    claimed_tokens: set = set()
    findings = []
    for entry in declared:
        key, label = entry["key"], unicodedata.normalize("NFKC", entry["label"])
        value = _number(_resolve(source, key))
        if value is None:
            findings.append(f"{where}: stat {key}: the source holds no number at {key}")
        hits = [number for number, line in enumerate(lines, start=1) if label in line]
        if not hits:
            findings.append(
                f"{where}: stat {key}: its label {label} is not in the stats part"
            )
            continue
        if len(hits) > 1:
            findings.append(
                f"{where}: stat {key}: its label {label} appears on {len(hits)} lines"
            )
            continue
        line_number = hits[0]
        after = lines[line_number - 1].find(label) + len(label)
        rendered = next(
            (
                token
                for token in tokens
                if token[0] == line_number and token[1] >= after
            ),
            None,
        )
        if rendered is None:
            findings.append(f"{where}: stat {key}: no number follows its label")
            continue
        claimed_tokens.add(rendered[:3])
        if value is not None and rendered[4] != value:
            findings.append(
                f"{where}: stat {key}: renders {rendered[3]} but its source says "
                f"{_resolve(source, key)}"
            )
    for token in tokens:
        if token[:3] not in claimed_tokens:
            findings.append(
                f"{where}: an unsourced number {token[3]} in the stats part"
            )
    return findings


def check_digest_numbers(ctx: Context) -> Outcome:
    digests = ctx.duty("daily-digest")
    findings = ctx.broken_findings()
    for message in digests:
        findings += _stat_findings(ctx, message)
    return Outcome(len(digests) + len(ctx.broken), tuple(findings))


def _says(persona_core, text: str, rows: list) -> bool:
    return bool(persona_core.scan(text, rows))


def check_digest_degraded(ctx: Context) -> Outcome:
    digests = ctx.duty("daily-digest")
    findings = ctx.broken_findings()
    if not digests:
        return Outcome(len(ctx.broken), tuple(findings))
    persona_core = ctx.persona_core()
    rows = ctx.patterns["degraded_notice"]
    for message in digests:
        where = ctx.show(message.path)
        coaching = message.envelope.get("coaching")
        roles = message.roles()
        if coaching == "unavailable":
            if "coaching" in roles:
                findings.append(
                    f"{where}: coaching is unavailable but a coaching part is present"
                )
            notice = message.part("notice")
            if notice is None:
                findings.append(
                    f"{where}: coaching is unavailable and no notice part says so"
                )
                continue
            reading = ctx.part_reading(message, notice)
            if _says(persona_core, reading.unhidden, rows):
                continue
            if reading.hiders and _says(persona_core, reading.visible, rows):
                findings.append(
                    f"{where}: the notice is hidden inside {reading.hiders[0]}"
                )
            else:
                findings.append(
                    f"{where}: the notice does not say plainly that coaching was unavailable"
                )
        elif coaching == "ok":
            if "notice" in roles:
                findings.append(f"{where}: coaching is ok but a notice part is present")
            part = message.part("coaching")
            if part is None or not ctx.part_reading(message, part).visible.strip():
                findings.append(
                    f"{where}: coaching is ok but there is no coaching part"
                )
        else:
            findings.append(
                f"{where}: coaching {coaching!r} is neither ok nor unavailable"
            )
    return Outcome(len(digests) + len(ctx.broken), tuple(findings))


def check_no_internals(ctx: Context) -> Outcome:
    """A finding names the rule and where, never the value: the value is what must not travel."""
    findings = ctx.broken_findings()
    if ctx.messages:
        persona_core = ctx.persona_core()
        public = ctx.public_deny()
        rows = [(row_id, pattern) for _origin, row_id, pattern in public["patterns"]]
        rows += ctx.patterns["internals"]
        for message in ctx.messages:
            where = ctx.show(message.path)
            for line, row_id, _found in persona_core.scan(
                ctx.reading(message).visible, rows
            ):
                findings.append(f"{where}:{line}: {row_id}")
            for label, text in _button_strings(message):
                for _line, row_id, _found in persona_core.scan(text, rows):
                    findings.append(f"{where}: {label}: {row_id}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_coaching_numbers(ctx: Context) -> Outcome:
    digests = ctx.duty("daily-digest")
    findings = ctx.broken_findings()
    for message in digests:
        part = message.part("coaching")
        if part is None:
            continue
        source, problem = _stats_source(message)
        if problem:
            continue
        held = set(_leaves(source))
        for token in numbers(ctx.part_reading(message, part).visible):
            if token[4] not in held:
                findings.append(
                    f"{ctx.show(message.path)}: the coaching quotes {token[3]}, "
                    "which no stat in its source holds"
                )
    return Outcome(len(digests) + len(ctx.broken), tuple(findings))


def check_comeback_shape(ctx: Context) -> Outcome:
    comebacks = ctx.duty("comeback")
    allowed = ctx.contract["keyboard"]["comeback_types"]
    findings = ctx.broken_findings()
    readings: dict = {}
    for message in comebacks:
        lapse = message.envelope.get("lapse_id")
        reading = message.envelope.get("reading_id")
        if isinstance(lapse, str) and isinstance(reading, str):
            readings.setdefault(lapse, set()).add(reading)
        where = ctx.show(message.path)
        buttons = _buttons(message)
        if len(buttons) != 1:
            findings.append(
                f"{where}: a comeback carries exactly one button; this one carries {len(buttons)}"
            )
        for label, button in buttons:
            if "callback_data" in button:
                findings.append(
                    f"{where}: {label}: a comeback button opens the reading by "
                    f"{' or '.join(allowed)}, never callback_data"
                )
            elif not any(kind in button for kind in allowed):
                findings.append(
                    f"{where}: {label}: a comeback button opens the reading by "
                    f"{' or '.join(allowed)}"
                )
    for lapse in sorted(readings):
        offered = sorted(readings[lapse])
        if len(offered) > 1:
            findings.append(
                f"lapse {lapse}: its comeback messages offer {len(offered)} readings "
                f"({', '.join(offered)}); one comeback reading per lapse"
            )
    return Outcome(len(comebacks) + len(ctx.broken), tuple(findings))


def check_readings_paused(ctx: Context) -> Outcome:
    digests = ctx.duty("daily-digest")
    findings = ctx.broken_findings()
    for message in digests:
        lapse = message.envelope.get("lapse_id")
        if lapse and "readings" in message.roles():
            findings.append(
                f"{ctx.show(message.path)}: a readings part while lapse {lapse} is open; "
                "readings pause during a lapse, and the comeback offers the one reading"
            )
    return Outcome(len(digests) + len(ctx.broken), tuple(findings))


def _scan_message(
    ctx: Context, persona_core, message: Message, rows: list, echo: bool
) -> list:
    where = ctx.show(message.path)
    findings = []
    for line, row_id, found in persona_core.scan(ctx.reading(message).visible, rows):
        findings.append(
            f"{where}:{line}: {row_id} {found!r}"
            if echo
            else f"{where}:{line}: {row_id}"
        )
    for label, text in _button_strings(message):
        for _line, row_id, found in persona_core.scan(text, rows):
            findings.append(
                f"{where}: {label}: {row_id} {found!r}"
                if echo
                else f"{where}: {label}: {row_id}"
            )
    return findings


def check_no_dates(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    if ctx.messages:
        persona_core = ctx.persona_core()
        shared = ctx.shared_patterns()
        rows = (
            list(shared["dates"])
            + list(shared["timelines"])
            + ctx.patterns["countdowns"]
        )
        for message in ctx.messages:
            where = ctx.show(message.path)
            findings += _scan_message(ctx, persona_core, message, rows, echo=True)
            reading = ctx.reading(message)
            for offset, date_format in reading.date_times:
                findings.append(
                    f"{where}: offset {offset}: a date-time entity (Bot API 9.5) with format "
                    f"{date_format or 'none'}: the client renders a date or a countdown, "
                    "whatever its text says"
                )
            for url in reading.links:
                if "tg://time" in url.lower():
                    findings.append(f"{where}: a tg://time link renders a date")
            for label, text in _button_strings(message):
                if "tg://time" in text.lower():
                    findings.append(
                        f"{where}: {label}: a tg://time link renders a date"
                    )
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_no_shame_framing(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    if ctx.messages:
        persona_core = ctx.persona_core()
        for message in ctx.messages:
            findings += _scan_message(
                ctx, persona_core, message, ctx.patterns["shame"], echo=True
            )
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_journal_never_leaks(ctx: Context) -> Outcome:
    """vault-duties' rule on every message: no link, embed or tag into the journal and, when a
    vault is named, no verbatim run of a journal note. A finding names a line, never the text."""
    findings = ctx.broken_findings()
    if ctx.messages:
        vault_duties = ctx.vault_duties()
        journal = ctx.journal()
        for message in ctx.messages:
            where = ctx.show(message.path)
            for finding in vault_duties.journal_leaks(
                ctx.reading(message).visible, journal
            ):
                findings.append(f"{where}: {finding}")
            for label, text in _button_strings(message):
                if label.endswith("url"):
                    continue
                for finding in vault_duties.journal_leaks(text, journal):
                    findings.append(f"{where}: {label}: {finding}")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_autonomy_language(ctx: Context) -> Outcome:
    findings = ctx.broken_findings()
    if ctx.messages:
        persona_core = ctx.persona_core()
        rows = ctx.patterns["controlling"]
        for message in ctx.messages:
            findings += _scan_message(ctx, persona_core, message, rows, echo=True)
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def _capital_runs(line: str, acronyms: frozenset) -> list:
    runs, current = [], []
    for match in WORD.finditer(line):
        word = match.group(0)
        if len(word) >= 2 and word.isupper() and word not in acronyms:
            current.append(match)
            continue
        if current:
            runs.append(current)
        current = []
    if current:
        runs.append(current)
    return [
        line[run[0].start() : run[-1].end()]
        for run in runs
        if sum(len(match.group(0)) for match in run) >= 3
    ]


def check_shouting(ctx: Context) -> Outcome:
    acronyms = ctx.patterns["acronyms"]
    findings = ctx.broken_findings()
    for message in ctx.messages:
        where = ctx.show(message.path)
        texts = [ctx.reading(message).visible] + [
            text
            for label, text in _button_strings(message)
            if not label.endswith("url")
        ]
        marks = 0
        for text in texts:
            folded = unicodedata.normalize("NFKC", text)
            marks += folded.count("!")
            for line_number, line in enumerate(folded.split("\n"), start=1):
                for run in _capital_runs(line, acronyms):
                    findings.append(f"{where}:{line_number}: block capitals {run!r}")
        if marks >= 2:
            findings.append(f"{where}: {marks} exclamation marks")
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


def check_near_miss_copy(ctx: Context) -> Outcome:
    """Near-miss copy belongs only where the gap is a fact: the deterministic stats part."""
    findings = ctx.broken_findings()
    if ctx.messages:
        persona_core = ctx.persona_core()
        rows = ctx.patterns["near_miss"]
        for message in ctx.messages:
            where = ctx.show(message.path)
            for part in message.parts:
                if part.role == "stats":
                    continue
                visible = ctx.part_reading(message, part).visible
                for line, row_id, found in persona_core.scan(visible, rows):
                    findings.append(
                        f"{where}: {part.role} part line {line}: {row_id} {found!r} "
                        "outside the stats part"
                    )
            for label, text in _button_strings(message):
                if label.endswith("url"):
                    continue
                for _line, row_id, found in persona_core.scan(text, rows):
                    findings.append(
                        f"{where}: {label}: {row_id} {found!r} outside the stats part"
                    )
    return Outcome(len(ctx.messages) + len(ctx.broken), tuple(findings))


CHECKS = {
    "message-contract": check_message_contract,
    "telegram-length": check_telegram_length,
    "telegram-entities": check_telegram_entities,
    "telegram-keyboard": check_telegram_keyboard,
    "digest-numbers": check_digest_numbers,
    "digest-degraded": check_digest_degraded,
    "no-internals": check_no_internals,
    "coaching-numbers": check_coaching_numbers,
    "comeback-shape": check_comeback_shape,
    "readings-paused": check_readings_paused,
    "no-dates-or-countdowns": check_no_dates,
    "no-shame-framing": check_no_shame_framing,
    "no-journal-in-messages": check_journal_never_leaks,
    "autonomy-language": check_autonomy_language,
    "shouting": check_shouting,
    "near-miss-copy": check_near_miss_copy,
}
CLASSES = tuple(CHECKS)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="nudge-duties-probe.py",
        description="Judge DeckStreak's digest and comeback messages against the envelope v1.",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="a directory or file to judge instead of --root; repeatable",
    )
    parser.add_argument(
        "--pack-dir",
        help="the nudge-duties pack directory (default: beside this script)",
    )
    parser.add_argument(
        "--persona-core",
        help="persona-core's probe, for its date tables (default: beside this script)",
    )
    parser.add_argument(
        "--vault-duties",
        help="vault-duties' probe, for its journal rule (default: beside this script)",
    )
    parser.add_argument(
        "--telegram-platform",
        help="telegram-platform's probe, for its payload validator (default: beside this script)",
    )
    parser.add_argument(
        "--vault",
        help="the vault, so no-journal-in-messages also proves verbatim quotes (live)",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    parse = commands.add_parser("parse", help="print one message envelope as JSON")
    parse.add_argument("file")
    commands.add_parser("classes", help="list the classes")
    return parser


def _usage(message: str) -> int:
    print(f"nudge-duties-probe: {message}", file=sys.stderr)
    return EXIT_USAGE


def _parse_command(path: str, pack_dir: "Path | None", telegram_platform_path) -> int:
    try:
        contract = load_contract(pack_dir)
    except ContractError as error:
        print(f"nudge-duties-probe: VOID: {error}", file=sys.stderr)
        return EXIT_VOID
    try:
        message = load_message(path)
    except ContractError as error:
        print(f"nudge-duties-probe: {path}: {error}", file=sys.stderr)
        return EXIT_FINDING
    try:
        telegram_platform = load_telegram_platform(telegram_platform_path)
    except ContractError as error:
        print(f"nudge-duties-probe: VOID: {error}", file=sys.stderr)
        return EXIT_VOID
    reading = read_text(telegram_platform, message.text, message.parse_mode)
    print(
        json.dumps(
            {
                "path": str(message.path),
                "duty": message.duty,
                "parts": [
                    {"role": part.role, "text": part.text} for part in message.parts
                ],
                "visible_text": reading.visible,
                "utf16_length": telegram_platform.utf16_len(reading.visible),
                "markup_problems": list(reading.problems),
                "contract_problems": _contract_problems(message, contract),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return EXIT_GREEN


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    pack_dir = Path(args.pack_dir) if args.pack_dir else None
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    if args.command == "parse":
        return _parse_command(args.file, pack_dir, args.telegram_platform)
    name = args.name
    root = Path(args.root)
    if not root.is_dir():
        return _usage(f"--root is not a directory: {args.root}")
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            return _usage(f"--subject does not exist: {base}")
    try:
        contract = load_contract(pack_dir)
        patterns = load_patterns(pack_dir)
    except ContractError as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    messages, broken = load_population(
        bases,
        contract["claim_suffix"],
        frozenset(duty["id"] for duty in contract["duties"]),
    )
    ctx = Context(
        root,
        messages,
        broken,
        contract,
        patterns,
        args.persona_core,
        args.vault_duties,
        Path(args.vault) if args.vault else None,
        args.telegram_platform,
    )
    try:
        outcome = CHECKS[name](ctx)
    except ContractError as error:
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
