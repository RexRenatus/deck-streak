"""SPEC-110's registrations: the law drill notes, the deferral reason's sanitiser, the queue and the
rollup, the answer's append, the graded parse and the callback token.

Every adapter writes SYNTHETIC notes into a temporary vault and CALLS the predecessor; none computes
a rule. A note is described by its stem and text. A calendar day inside a stem, a text or a result
is written as the token `{day:N}`, N its epoch day number: the adapter expands every token of the
case into the ISO date of that day before the call, and writes every ISO date of a result back as
its token. A stem whose value is `null` is a folder of that name, which no reader can open as a note.

* `drill_meta` runs `vault_bridge.py:list_active_drills`, `list_unanswered_drills` and
  `read_active_drill` over the notes of the Active folder for `today` (an epoch day number). It
  returns the two lists, the single view of each stem and of the ids the predecessor refuses before
  any read, and the folders the predecessor reads (`_ACTIVE_DRILLS_RELATIVE_DIR`,
  `_GRADED_DRILLS_RELATIVE_DIR`).
* `drill_defer_reason` calls `vault_bridge.py:_sanitise_defer_reason` on a string.
* `drill_queue` calls `pipeline_layers/nudges.py:NudgesLayer.drill_queue` on a layer holding only
  its settings and its day.
* `drill_rollup` calls `vault_bridge.py:_active_drill_rollup`.
* `drill_answer_append` calls `vault_bridge.py:append_drill_answer` once per step and returns each
  step's result (the title, the refusal, or nothing) with the note's text after it.
* `drill_graded_parse` calls `vault_bridge.py:_parse_graded_drill` on each note of the Graded folder.
* `drill_callback_token` calls `bot.py:CommandBot._encode_drill_token` and `_resolve_drill_token`.

The builders draw only from the `random.Random` the generator seeds. Every note, subject and answer
here is synthetic.
"""

import re
import tempfile
import types
from datetime import date
from pathlib import Path

EPOCH_ORDINAL = date(1970, 1, 1).toordinal()
DAY_TOKEN = re.compile(r"\{day:(-?[0-9]+)\}")
ISO_DAY = re.compile(r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}(?![0-9])")

UNSAFE_IDS = ("", "..", "a/b", "a\\b", "x..y", "missing-drill")
SUBJECTS = ("Torts", "Contracts", "Trusts & Estates", "Constitutional Law", "Evidence")
TYPES = ("irac", "rule-statement", "outline", "case-brief")
MARKER = "- [ ] **Ready for grading**"


def expand(text):
    """`text` with each `{day:N}` written as the ISO date of epoch day N, and `{badday}` as a date
    no calendar holds."""
    text = text.replace("{badday}", "2026-02-30")
    return DAY_TOKEN.sub(
        lambda m: date.fromordinal(EPOCH_ORDINAL + int(m.group(1))).isoformat(), text
    )


def contract(value):
    """`value` with each ISO date of a string written back as its token."""
    if isinstance(value, str):
        return ISO_DAY.sub(
            lambda m: "{day:%d}" % (date.fromisoformat(m.group(0)).toordinal() - EPOCH_ORDINAL),
            value,
        )
    if isinstance(value, dict):
        return {contract(k): contract(v) for k, v in value.items()}
    if isinstance(value, list | tuple):
        return [contract(v) for v in value]
    return value


def day_of(number):
    """The `date` of epoch day `number`."""
    return date.fromordinal(EPOCH_ORDINAL + number)


def settle(root, sub, notes):
    """Writes `notes` (stem -> text, or None for a folder) into `root/sub`."""
    folder = root / sub
    folder.mkdir(parents=True, exist_ok=True)
    for stem, text in notes.items():
        target = folder / f"{expand(stem)}.md"
        if text is None:
            target.mkdir()
        else:
            target.write_text(expand(text), encoding="utf-8", newline="")
    return folder


def note(kind="irac", subject="Torts", created=None, title="A synthetic drill", body="", extra="",
         ticked=" ", status="active"):
    """A synthetic drill note in the shape the predecessor reads."""
    head = ["---"]
    if kind is not None:
        head.append(f"type: drill-{kind}")
    if subject is not None:
        head.append(f'subject: "{subject}"')
    if created is not None:
        head.append(f"created: {created}")
    head.append(f"status: {status}")
    if extra:
        head.append(extra)
    head.append("---")
    return (
        "\n".join(head)
        + f"\n# {title}\n\nThe prompt of {title}.\n<!-- a comment\nover lines -->\n{body}\n"
        + "## Free Recall\n\n## Self-Check\n\n"
        + f"- [ ] the rule stated\n- [{ticked}] **Ready for grading**\n"
    )


def as_settings(root):
    return types.SimpleNamespace(vault_replica_path=root)


def drill_notes(rng):
    """A set of Active notes covering the classes a port gets wrong."""
    notes = {
        "irac-{day:100}": note(created="{day:100}"),
        "evidence-outline-{day:90}": note(kind=None, subject="Evidence", title="Hearsay"),
        "case-brief-{day:80}": note(kind="case-brief", created="{day:80}", ticked="x",
                                    subject="Contracts"),
        "rule-{day:70}": note(kind="rule-statement", created="{badday}", ticked="X",
                              extra="defer_reason: needs the source"),
        "untitled": "---\ntype: drill-outline\nstatus: active\n---\nno heading here\n"
                    + MARKER + "\n",
        "nofm": "# no frontmatter\n" + MARKER + "\n",
        "quoted-{day:60}": note(kind='"irac"', subject="Trusts & Estates", created="{day:60}"),
        "folded-{day:50}": note(subject="TRUSTS and ESTATES", created="{day:50}", ticked="x",
                                extra="defer_reason: >"),
        "spaced-{day:40}": note(subject="  Torts  ", created="{day:40}", ticked="x",
                                extra="defer_reason:   too\u200b hard\u0085 \u00a0now  "),
        "folder-note": None,
    }
    for index in range(rng.randint(2, 4)):
        stem = f"extra-{index}-{{day:{rng.randint(1, 20000)}}}"
        notes[stem] = note(
            kind=rng.choice(TYPES),
            subject=rng.choice(SUBJECTS),
            created="{day:%d}" % rng.randint(1, 20000),
            ticked=rng.choice(" xX"),
            extra=rng.choice(("", "defer_reason: later", "defer_reason: " + "long " * 40)),
        )
    return notes


def meta_cases(rng):
    cases = [(None, {"notes": {}, "today": 200}), ("empty", {"notes": {"a-note": ""}, "today": 200})]
    for index in range(3):
        cases.append(("mixed", {"notes": drill_notes(rng), "today": 100 + index * 137}))
    return cases


def meta(function, predecessor, notes, today):
    bridge = predecessor("vault_bridge")
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        settle(root, bridge._ACTIVE_DRILLS_RELATIVE_DIR, notes)
        settings, when = as_settings(root), day_of(today)
        views = {}
        for stem in [expand(s) for s in notes] + list(UNSAFE_IDS):
            views[stem] = bridge.read_active_drill(settings, stem, today=when)
        result = {
            "active": bridge.list_active_drills(settings, today=when),
            "unanswered": bridge.list_unanswered_drills(settings, today=when),
            "views": views,
            "folders": [
                list(bridge._ACTIVE_DRILLS_RELATIVE_DIR.parts),
                list(bridge._GRADED_DRILLS_RELATIVE_DIR.parts),
            ],
        }
    return contract(result)


def defer_cases(rng):
    fixed = [
        "", "   ", ">", "|-", ">+", "plain reason", "  two   spaces  ", "tab\there", "line\nbreak",
        "zero\u200dwidth joiner", "private\ue000use", "unassigned\u0378point", "c1\u0085control",
        "format\u202eoverride", "nbsp\u00a0here\u3000ideographic\u2003em", "sep\u2028line\u2029para",
        "\u0000\u001f\u007f\u009f", "e\u0301 combining", "a" * 119, "a" * 120, "a" * 121,
        "word " * 40, "x" * 118 + " yy", "\u2026" * 130, "\U0001f600" * 130, "\ufeffbom", "\u180emongolian",
        "\U000e0001tag\U000e007f", "\U0010fffd", "\U0001fffe",
    ]
    for _ in range(rng.randint(6, 10)):
        pool = "ab c\u200d\ue000\u0378\u0085\t\n\u00a0\u3000\u202e-|>"
        fixed.append("".join(rng.choice(pool) for _ in range(rng.randint(0, 40))))
    return [(None, {"raw": raw}) for raw in fixed]


def queue_layer(predecessor, root, today):
    layer_class = predecessor("pipeline_layers.nudges.NudgesLayer")
    layer = object.__new__(layer_class)
    layer._settings = as_settings(root)
    layer._today = lambda: day_of(today)
    return layer


def queue(function, predecessor, notes, today):
    import asyncio

    bridge = predecessor("vault_bridge")
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        settle(root, bridge._ACTIVE_DRILLS_RELATIVE_DIR, notes)
        result = asyncio.run(queue_layer(predecessor, root, today).drill_queue())
    return contract(result)


def queue_cases(rng):
    return [(None, {"notes": {}, "today": 300})] + [
        ("mixed", {"notes": drill_notes(rng), "today": 90 + index * 211}) for index in range(3)
    ]


def rollup(function, predecessor, notes, today, subjects):
    bridge = predecessor("vault_bridge")
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        settle(root, bridge._ACTIVE_DRILLS_RELATIVE_DIR, notes)
        result = bridge._active_drill_rollup(as_settings(root), set(subjects), day_of(today))
    return contract(result)


def rollup_cases(rng):
    subjects = list(SUBJECTS[:3]) + ["ConstitutionalLaw", "Trusts&Estates", "STRASSE"]
    cases = [(None, {"notes": {}, "today": 300, "subjects": []})]
    for index in range(4):
        picked = rng.sample(subjects, rng.randint(0, len(subjects)))
        cases.append(("mixed", {"notes": drill_notes(rng), "today": 90 + index * 173,
                                "subjects": picked}))
    return cases


def append(function, predecessor, notes, steps):
    from unittest import mock

    bridge = predecessor("vault_bridge")
    written = []
    real = Path.write_text

    def capture(self, data, *args, **kwargs):
        written.append(data)
        return real(self, data, *args, **kwargs)

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        settle(root, bridge._ACTIVE_DRILLS_RELATIVE_DIR, notes)
        settings, outcomes = as_settings(root), []
        for step in steps:
            drill = expand(step["id"])
            written.clear()
            with mock.patch.object(Path, "write_text", capture):
                got = bridge.append_drill_answer(settings, drill, step["answer"], step["when"])
            if got is None:
                shown = {"none": True}
            elif got is bridge.DRILL_ALREADY_ANSWERED:
                shown = {"already_answered": True}
            else:
                shown = {"title": got}
            outcomes.append({"result": shown, "text": written[0] if written else None})
    return contract(outcomes)


def append_cases(rng):
    base = {"one": note(title="First"), "two": note(kind=None, title="Second", ticked="x"),
            "crlf": note().replace("\n", "\r\n"), "bare": "no marker at all\n",
            "notitle": "---\nstatus: active\n---\n" + MARKER + "\n", "gap": note() + "\n\n\n"}
    cases = []
    for answer in ("An answer.", "  padded  \n", "line one\nline two\n\n", "unicode \u00e9\u4e2d",
                   "# a heading\n- [ ] **Ready for grading**"):
        cases.append(("append", {"notes": base, "steps": [
            {"id": "one", "answer": answer, "when": "morning"},
            {"id": "one", "answer": "again", "when": "evening"},
        ]}))
    cases.append(("refused", {"notes": base, "steps": [
        {"id": "two", "answer": "late", "when": "noon"},
        {"id": "nope", "answer": "x", "when": "noon"},
        {"id": "..", "answer": "x", "when": "noon"},
        {"id": "crlf", "answer": "a", "when": "night"},
        {"id": "bare", "answer": "b", "when": "night"},
        {"id": "notitle", "answer": "c", "when": "night"},
        {"id": "gap", "answer": "d", "when": "night"},
    ]}))
    return cases


def graded(function, predecessor, notes):
    bridge = predecessor("vault_bridge")
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        folder = settle(root, bridge._GRADED_DRILLS_RELATIVE_DIR, notes)
        result = {}
        for stem in notes:
            got = bridge._parse_graded_drill(folder / f"{expand(stem)}.md")
            result[stem] = None if got is None else [got[0], got[1]]
    return contract(result)


def graded_note(xp=None, status="graded", extra=""):
    lines = ["---", f"status: {status}", "type: drill-irac"]
    if xp is not None:
        lines.append(f"xp: {xp}")
    if extra:
        lines.append(extra)
    return "\n".join(lines) + "\n---\n# graded\n"


def graded_cases(rng):
    notes = {
        "plain": graded_note(),
        "no-frontmatter": "# just a heading\n",
        "empty-frontmatter": "---\n---\nbody\n",
        "unclosed": "---\nstatus: graded\nxp: 12\n",
        "other-status": graded_note(status="active"),
        "status-case": graded_note(status="Graded"),
        " ": graded_note(),
        "xp-9": graded_note(9), "xp-10": graded_note(10), "xp-24": graded_note(24),
        "xp-25": graded_note(25), "xp-26": graded_note(26), "xp-neg": graded_note(-4),
        "xp-zero": graded_note(0), "xp-huge": graded_note("9" * 40),
        "xp-space": graded_note("  17  "), "xp-tab": graded_note("\t18\t"),
        "xp-plus": graded_note("+20"), "xp-minus-plus": graded_note("-+3"),
        "xp-underscore": graded_note("1_2"), "xp-double-underscore": graded_note("1__2"),
        "xp-lead-underscore": graded_note("_12"), "xp-trail-underscore": graded_note("12_"),
        "xp-decimal": graded_note("12.5"), "xp-word": graded_note("high"),
        "xp-empty": graded_note(""), "xp-hex": graded_note("0x10"),
        "dup-key": graded_note(11, extra="xp: 22"),
        "indent-key": "---\nstatus: graded\n xp: 30\n---\n",
        "dash-key": "---\nstatus: graded\n- xp: 30\n---\n",
        "crlf": "---\r\nstatus: graded\r\nxp: 19\r\n---\r\n",
        "colons": "---\nstatus: graded\nnote: a: b: c\n---\n",
        "bare-rule-later": "---\nstatus: graded\n---\nxp: 30\n",
        "a-folder": None,
    }
    for index in range(rng.randint(2, 4)):
        notes[f"random-{index}"] = graded_note(rng.randint(-50, 80))
    return [(None, {"notes": notes})]


def token(function, predecessor, prefix, drill, offered):
    bot = predecessor("bot.CommandBot")
    listed = [{"drill_id": name} for name in offered]
    encoded = bot._encode_drill_token(prefix, drill)
    body = encoded[len(prefix):]
    resolved = bot._resolve_drill_token(body, listed)
    return {"token": encoded, "resolved": None if resolved is None else resolved["drill_id"]}


def token_cases(rng):
    ids = ["irac-one", "a" * 61, "a" * 62, "a" * 63, "\u00e9" * 30, "\u00e9" * 31, "\u4e2d" * 20,
           "x", "with space", "h.hashed", "a" * 128]
    cases = []
    for drill in ids:
        for prefix in ("dv:", "da:"):
            cases.append((None, {"prefix": prefix, "drill": drill,
                                 "offered": [drill, "other-drill"]}))
    cases.append(("absent", {"prefix": "dv:", "drill": "gone", "offered": ["kept"]}))
    cases.append(("twice", {"prefix": "dv:", "drill": "a" * 70, "offered": ["a" * 70, "a" * 70]}))
    cases.append(("unresolved", {"prefix": "dv:", "drill": "a" * 70, "offered": ["b" * 70]}))
    return cases


FUNCTIONS = {
    "drill_meta": {
        "kind": "adapter", "function": "vault_bridge.list_active_drills", "adapter": meta,
        "note": "Writes the notes into a temporary vault, and returns the predecessor's lists, "
                "each stem's single view and the folders it reads, with days as tokens.",
        "cases": meta_cases,
    },
    "drill_defer_reason": {
        "kind": "function", "function": "vault_bridge._sanitise_defer_reason",
        "cases": defer_cases,
    },
    "drill_queue": {
        "kind": "adapter", "function": "pipeline_layers.nudges.NudgesLayer.drill_queue",
        "adapter": queue,
        "note": "Calls the queue on a layer holding only its settings and its day.",
        "cases": queue_cases,
    },
    "drill_rollup": {
        "kind": "adapter", "function": "vault_bridge._active_drill_rollup", "adapter": rollup,
        "note": "Calls the rollup over the notes for the day and the subject set.",
        "cases": rollup_cases,
    },
    "drill_answer_append": {
        "kind": "adapter", "function": "vault_bridge.append_drill_answer", "adapter": append,
        "note": "Calls the append once per step, returning each result and the note's text after.",
        "cases": append_cases,
    },
    "drill_graded_parse": {
        "kind": "adapter", "function": "vault_bridge._parse_graded_drill", "adapter": graded,
        "note": "Calls the parse on each note of the Graded folder.",
        "cases": graded_cases,
    },
    "drill_callback_token": {
        "kind": "adapter", "function": "bot.CommandBot._encode_drill_token", "adapter": token,
        "note": "Encodes the drill's token, and resolves its body against the offered list.",
        "cases": token_cases,
    },
    "drills.constants": {
        "kind": "constants",
        "names": [
            "vault_bridge.DRILL_POSTBACK_XP", "vault_bridge.DRILL_XP_MIN",
            "vault_bridge.DRILL_XP_MAX", "vault_bridge.DEFER_REASON_MAX_LEN",
            "vault_bridge._READY_UNTICKED", "vault_bridge._READY_TICKED",
            "bot._MAX_CALLBACK_DATA", "bot._DRILL_TOKEN_HASH_LEN", "bot._DRILLS_KEYBOARD_LIMIT",
            "curriculum.LAW_DRILL_ALIASES",
        ],
    },
}
