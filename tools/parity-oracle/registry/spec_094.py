"""SPEC-094's registrations: the wire walk, the safe name, the template tokens, the Dark Fields
report, and the constants the port uses verbatim.

Each adapter builds what JSON cannot carry (bytes, tuple keys, a frozenset) and CALLS the
predecessor; none computes a rule.

* `walk_hex` drives `darkfields.py:_iter_fields` on the bytes the case's hex names. It writes each
  byte value back as hex and a refusal as its message, so an error is part of the golden.
* `tokens_of_hex` drives `darkfields.py:_extract_config_tokens_ex` on a template config, whose
  bytes it builds from the case's hex (or none), and returns the sorted tokens and the failure
  flag.
* `with_rows` drives `darkfields.py:build_dark_fields_report` on the rows and mappings the case's
  lists name, and returns the report as plain lists.
* `sorted_names` returns the predecessor's special field names, which JSON cannot carry as a set,
  as a sorted list.

The case builders draw only from the `random.Random` the generator seeds. Every name is synthetic.
"""

#: A wire type the walk refuses, a group start.
GROUP_START = 3


def varint(value):
    """The protobuf varint encoding of the non-negative `value`."""
    out = bytearray()
    while True:
        low = value & 0x7F
        value >>= 7
        if value:
            out.append(low | 0x80)
        else:
            out.append(low)
            return bytes(out)


def field(number, wire_type, payload):
    """One protobuf field: its key, then its payload as its wire type carries it."""
    key = varint((number << 3) | wire_type)
    if wire_type == 0:
        return key + varint(payload)
    if wire_type == 2:
        return key + varint(len(payload)) + payload
    return key + payload


def config(q=None, a=None):
    """A template config: the question format as field 1 and the answer format as field 2."""
    out = b""
    if q is not None:
        out += field(1, 2, q.encode("utf-8"))
    if a is not None:
        out += field(2, 2, a.encode("utf-8"))
    return out


def walk_hex(iter_fields, predecessor, *, hex):
    try:
        walked = iter_fields(bytes.fromhex(hex))
    except ValueError as refusal:
        return {"error": str(refusal)}
    return {
        "fields": [
            [number, wire, value if isinstance(value, int) else value.hex()]
            for number, wire, value in walked
        ]
    }


def wire_cases(rng):
    fixed = [
        ("empty", {"hex": ""}),
        ("varint", {"hex": field(1, 0, 150).hex()}),
        ("varint", {"hex": field(3, 0, 0).hex()}),
        ("varint", {"hex": field(2, 0, 2**63).hex()}),
        ("fixed64", {"hex": field(4, 1, bytes(range(8))).hex()}),
        ("fixed32", {"hex": field(5, 5, bytes(range(4))).hex()}),
        ("length", {"hex": field(1, 2, b"hello").hex()}),
        ("length", {"hex": field(2, 2, b"").hex()}),
        ("length", {"hex": (field(1, 2, b"a") + field(2, 2, b"b") + field(1, 0, 7)).hex()}),
        ("big_field_number", {"hex": field(2**20, 0, 1).hex()}),
        ("truncated_varint", {"hex": "08" + "80"}),
        ("truncated_key", {"hex": "80"}),
        ("varint_too_long", {"hex": "08" + "ff" * 10 + "01"}),
        ("truncated_fixed64", {"hex": "21" + "0102030405060708"[:-2]}),
        ("truncated_fixed32", {"hex": "2d" + "010203"}),
        ("truncated_length", {"hex": "0a05" + "616263"}),
        ("group_start", {"hex": field(1, GROUP_START, b"").hex()}),
        ("group_end", {"hex": field(1, 4, b"").hex()}),
        ("wire_type_six", {"hex": field(1, 6, b"").hex()}),
        ("wire_type_seven", {"hex": field(1, 7, b"").hex()}),
        ("trailing_after_field", {"hex": field(1, 0, 1).hex() + "80"}),
    ]
    drawn = []
    for _ in range(12):
        parts = b""
        for _ in range(rng.randrange(1, 5)):
            number = rng.randrange(1, 40)
            wire = rng.choice([0, 1, 2, 5])
            if wire == 0:
                parts += field(number, 0, rng.randrange(0, 2**40))
            elif wire == 1:
                parts += field(number, 1, bytes(rng.randrange(256) for _ in range(8)))
            elif wire == 2:
                size = rng.randrange(0, 200)
                parts += field(number, 2, bytes(rng.randrange(256) for _ in range(size)))
            else:
                parts += field(number, 5, bytes(rng.randrange(256) for _ in range(4)))
        drawn.append((None, {"hex": parts.hex()}))
    return fixed + drawn


def safe_name_cases(rng):
    fixed = [
        ("plain", {"name": "Example field"}),
        ("empty", {"name": ""}),
        ("html", {"name": "<b>Example</b> & \"quoted\" 'single'"}),
        ("control_low", {"name": "a\x00b\x01c\x08d"}),
        ("control_kept", {"name": "a\tb\nc\rd"}),
        ("control_vt_ff", {"name": "a\x0bb\x0cc"}),
        ("control_high", {"name": "a\x0eb\x1fc\x7fd"}),
        ("c1_kept", {"name": "a\x80b\x9fc"}),
        ("unicode", {"name": "Écrit 例   \U0001f311"}),
        ("replacement_then_html", {"name": "\x01<x>"}),
    ]
    alphabet = ["a", "Z", " ", "<", ">", "&", '"', "'", "\x00", "\x07", "\x0b", "\x1f", "\x7f", "é"]
    drawn = [
        (None, {"name": "".join(rng.choice(alphabet) for _ in range(rng.randrange(0, 12)))})
        for _ in range(12)
    ]
    return fixed + drawn


def tokens_of_hex(extract, predecessor, *, hex):
    data = None if hex is None else bytes.fromhex(hex)
    tokens, failed = extract(data)
    return {"tokens": sorted(tokens), "failed": failed}


def token_cases(rng):
    def case(kind, q=None, a=None):
        return (kind, {"hex": config(q, a).hex()})

    fixed = [
        ("none", {"hex": None}),
        ("empty", {"hex": ""}),
        case("plain", "{{Front}}", "{{Back}}"),
        case("filter", "{{text:Front}}", "{{furigana:cloze:Back}}"),
        case("section", "{{#Front}}x{{/Front}}", "{{^Back}}y{{/Back}}"),
        case("section_space", "{{# Front }}", "{{ ^ Back }}"),
        case("special", "{{FrontSide}}{{Tags}}{{Type}}{{Deck}}{{Subdeck}}{{Card}}", "{{Front}}"),
        case("special_after_filter", "{{text:FrontSide}}", "{{type:Card}}"),
        case("blank", "{{}} {{  }} {{#}} {{:}}", ""),
        case("nested", "{{{Front}}} {{{{Back}}}}", ""),
        case("unclosed", "{{Front} {{Back", "}}{{"),
        case("static_text", "no fields here", "still none"),
        case("q_only", "{{Front}}"),
        case("a_only", None, "{{Back}}"),
        case("unicode", "{{Écrit}}", "{{例:Field}}"),
        case("newline_inside", "{{Fr\nont}}", ""),
        case("trailing_colon", "{{text:}}", "{{a::b}}"),
        case("whitespace_name", "{{  Front  }}", "{{\x1cBack\x1f}}"),
        case("empty_string_config_q", "", ""),
        ("repeated_field", {"hex": (config("{{One}}") + config("{{Two}}")).hex()}),
        ("unknown_field", {"hex": (field(9, 2, b"{{Ignored}}") + config("{{Kept}}")).hex()}),
        ("varint_field_skipped", {"hex": (field(1, 0, 5) + config("{{Kept}}")).hex()}),
        ("fixed_field_skipped", {"hex": (field(3, 1, bytes(8)) + config("{{Kept}}")).hex()}),
        ("bad_utf8", {"hex": field(1, 2, b"{{A}}\xff").hex()}),
        ("bad_utf8_overlong", {"hex": field(2, 2, b"\xc0\x80").hex()}),
        ("bad_utf8_surrogate", {"hex": field(1, 2, b"\xed\xa0\x80").hex()}),
        ("truncated", {"hex": config("{{Front}}").hex()[:-4]}),
        ("group", {"hex": field(1, GROUP_START, b"").hex()}),
    ]
    names = ["Front", "Back", "Extra", "Note", "Audio", "Hint"]
    prefixes = ["", "text:", "hint:", "cloze:", "furigana:", "#", "/", "^"]
    drawn = []
    for _ in range(10):
        def side():
            parts = []
            for _ in range(rng.randrange(0, 4)):
                parts.append("{{" + rng.choice(prefixes) + rng.choice(names) + "}}")
            return " ".join(parts)

        drawn.append((None, {"hex": config(side(), side()).hex()}))
    return fixed + drawn


def with_rows(
    build,
    predecessor,
    *,
    template_names,
    template_configs,
    declared_fields,
    presence,
    reviewed_note_count,
    failed_reads,
):
    names = {(mid, ordinal): (nt, t) for mid, ordinal, nt, t in template_names}
    configs = [
        predecessor("darkfields.TemplateConfigRow")(
            ntid=row["ntid"],
            ord=row["ord"],
            config=None if row["hex"] is None else bytes.fromhex(row["hex"]),
        )
        for row in template_configs
    ]
    fields = [
        predecessor("darkfields.FieldRow")(ntid=row["ntid"], ord=row["ord"], name=row["name"])
        for row in declared_fields
    ]
    counts = {(ntid, ordinal): count for ntid, ordinal, count in presence}
    report = build(
        names,
        configs,
        fields,
        counts,
        reviewed_note_count=reviewed_note_count,
        failed_reads=tuple(failed_reads),
    )
    return {
        "dark_fields": [
            [d.notetype, d.field, d.reviewed_notes_with_content] for d in report.dark_fields
        ],
        "unparseable": [[u.notetype, u.notetype_id] for u in report.unparseable],
        "notetypes_checked": report.notetypes_checked,
        "reviewed_note_count": report.reviewed_note_count,
        "is_cold": report.is_cold,
        "failed_reads": list(report.failed_reads),
    }


def row(ntid, ordinal, hex_config):
    return {"ntid": ntid, "ord": ordinal, "hex": hex_config}


def fieldrow(ntid, ordinal, name):
    return {"ntid": ntid, "ord": ordinal, "name": name}


def dark_case(**overrides):
    base = {
        "template_names": [[10, 0, "Type A", "Card 1"]],
        "template_configs": [row(10, 0, config("{{One}}", "{{Two}}").hex())],
        "declared_fields": [fieldrow(10, 0, "One"), fieldrow(10, 1, "Two"), fieldrow(10, 2, "Dark")],
        "presence": [[10, 0, 9], [10, 1, 9], [10, 2, 5]],
        "reviewed_note_count": 9,
        "failed_reads": [],
    }
    base.update(overrides)
    return base


def dark_cases(rng):
    fixed = [
        ("one_dark_field", dark_case()),
        ("failed_reads", dark_case(failed_reads=["templates (ntid, ord, config)"])),
        (
            "failed_reads_two",
            dark_case(failed_reads=["fields (ntid, ord, name)", "notes (batched flds presence read)"]),
        ),
        ("cold_no_reviews", dark_case(reviewed_note_count=0)),
        ("cold_no_fields", dark_case(declared_fields=[])),
        ("failed_beats_cold", dark_case(reviewed_note_count=0, failed_reads=["x"])),
        ("below_floor", dark_case(presence=[[10, 0, 9], [10, 1, 9], [10, 2, 2]])),
        ("at_floor", dark_case(presence=[[10, 0, 9], [10, 1, 9], [10, 2, 3]])),
        ("no_presence_row", dark_case(presence=[])),
        (
            "no_tokens_is_unparseable",
            dark_case(template_configs=[row(10, 0, config("static", "static").hex())]),
        ),
        (
            "no_template_row_is_unparseable",
            dark_case(template_configs=[]),
        ),
        (
            "one_broken_sibling",
            dark_case(
                template_names=[[10, 0, "Type A", "Card 1"], [10, 1, "Type A", "Card 2"]],
                template_configs=[
                    row(10, 0, config("{{One}}").hex()),
                    row(10, 1, config("{{One}}")[:-1].hex()),
                ],
            ),
        ),
        (
            "tokens_union_across_templates",
            dark_case(
                template_configs=[
                    row(10, 0, config("{{One}}").hex()),
                    row(10, 1, config("{{text:Two}}", "{{#Dark}}{{/Dark}}").hex()),
                ]
            ),
        ),
        (
            "special_names_are_not_fields",
            dark_case(
                template_configs=[row(10, 0, config("{{FrontSide}}{{One}}").hex())],
                declared_fields=[fieldrow(10, 0, "One"), fieldrow(10, 1, "Tags")],
                presence=[[10, 0, 9], [10, 1, 9]],
            ),
        ),
        (
            "unknown_notetype_name",
            dark_case(template_names=[]),
        ),
        (
            "name_is_the_first_of_the_map",
            dark_case(
                template_names=[[10, 1, "Second", "c"], [10, 0, "First", "c"]],
            ),
        ),
        (
            "sorted_by_name_then_field",
            {
                "template_names": [[1, 0, "Zeta", "c"], [2, 0, "Alpha", "c"], [3, 0, "Alpha", "c"]],
                "template_configs": [
                    row(1, 0, config("{{X}}").hex()),
                    row(2, 0, config("{{X}}").hex()),
                    row(3, 0, config("static").hex()),
                ],
                "declared_fields": [
                    fieldrow(1, 0, "X"), fieldrow(1, 1, "b"), fieldrow(1, 2, "a"),
                    fieldrow(2, 0, "X"), fieldrow(2, 1, "z"), fieldrow(2, 2, "y"),
                    fieldrow(3, 0, "Q"),
                ],
                "presence": [[1, 1, 4], [1, 2, 4], [2, 1, 7], [2, 2, 3], [3, 0, 8]],
                "reviewed_note_count": 12,
                "failed_reads": [],
            },
        ),
        (
            "two_unparseable_sorted",
            {
                "template_names": [[7, 0, "Same", "c"], [5, 0, "Same", "c"], [6, 0, "Aaa", "c"]],
                "template_configs": [],
                "declared_fields": [fieldrow(7, 0, "f"), fieldrow(5, 0, "f"), fieldrow(6, 0, "f")],
                "presence": [],
                "reviewed_note_count": 1,
                "failed_reads": [],
            },
        ),
    ]
    names = ["Front", "Back", "Extra", "Note", "Audio", "Hint", "Gloss"]
    drawn = []
    for _ in range(10):
        types = rng.randrange(1, 4)
        template_names, template_configs, declared, presence = [], [], [], []
        for index in range(types):
            ntid = 100 + index * 3 + rng.randrange(0, 3)
            template_names.append([ntid, 0, f"Type {rng.choice('ABC')}", "c"])
            used = rng.sample(names, rng.randrange(0, len(names)))
            question = " ".join("{{" + rng.choice(["", "text:"]) + n + "}}" for n in used)
            template_configs.append(row(ntid, 0, config(question, "{{FrontSide}}").hex()))
            for ordinal, fname in enumerate(rng.sample(names, rng.randrange(1, len(names)))):
                declared.append(fieldrow(ntid, ordinal, fname))
                if rng.random() < 0.8:
                    presence.append([ntid, ordinal, rng.randrange(0, 12)])
        drawn.append(
            (
                None,
                {
                    "template_names": template_names,
                    "template_configs": template_configs,
                    "declared_fields": declared,
                    "presence": presence,
                    "reviewed_note_count": rng.randrange(0, 30),
                    "failed_reads": [],
                },
            )
        )
    return fixed + drawn


def sorted_names(special_field_names, predecessor):
    return {"names": sorted(special_field_names)}


def names_cases(rng):
    return [(None, {})]


FUNCTIONS = {
    "wire_walk": {
        "kind": "adapter",
        "function": "darkfields._iter_fields",
        "adapter": walk_hex,
        "note": "Walks the bytes the case's hex names and writes each byte value back as hex",
        "cases": wire_cases,
    },
    "safe_name": {
        "kind": "function",
        "function": "darkfields._safe_name",
        "cases": safe_name_cases,
    },
    "dark_fields_tokens": {
        "kind": "adapter",
        "function": "darkfields._extract_config_tokens_ex",
        "adapter": tokens_of_hex,
        "note": "Builds a template config's bytes from the case's hex and returns the sorted tokens",
        "cases": token_cases,
    },
    "dark_fields": {
        "kind": "adapter",
        "function": "darkfields.build_dark_fields_report",
        "adapter": with_rows,
        "note": "Builds the predecessor's rows and mappings from the case's lists, and returns the "
        "report as plain lists",
        "cases": dark_cases,
    },
    "dark_fields.special_names": {
        "kind": "adapter",
        "function": "darkfields.SPECIAL_FIELD_NAMES",
        "adapter": sorted_names,
        "note": "Returns the predecessor's special field names, a set, as a sorted list",
        "cases": names_cases,
    },
    "dark_fields.constants": {
        "kind": "constants",
        "names": [
            "darkfields.MIN_DARK_NOTES",
            "darkfields.MAX_DARK_FIELDS_SHOWN",
            "darkfields.MAX_UNPARSEABLE_SHOWN",
            "darkfields._SECTION_PREFIXES",
            "darkfields._Q_FORMAT_FIELD",
            "darkfields._A_FORMAT_FIELD",
            "transfer.NOTES_BATCH_SIZE",
        ],
    },
}
