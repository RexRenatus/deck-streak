"""SPEC-020's registrations: the digest hour, the redaction filter and the kernel's constants.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `digest_hour` drives `config.py:_env_digest_hour`, which reads its value from the process
  environment: the adapter sets the predecessor's variable to the case's raw text (or removes it
  when the case has none) for the one call, then restores the environment as it found it.
* `redaction` drives `logging_redact.py:SecretRedactingFilter`: the adapter constructs the filter
  with the case's secrets as its own values (never the module's process-wide registry), runs one
  `logging.LogRecord` carrying the case's text through `filter`, and returns the record's message.
* `kernel.constants` reads each constant SPEC-020 ports from the predecessor's module.

The case builders cover the classes a port gets wrong. For the digest hour: an unset or blank
value (the larger of the default and the rollover hour), an explicit value with a sign, a leading
zero or surrounding whitespace, one earlier than the rollover, one out of range, and the two kinds
DeckStreak refuses where the predecessor fell back or parsed on (ADR-020): an unparsable value and
a spelling only Python's `int` accepts. For the redaction: overlapping secrets (longest first), the
minimum length counted in characters rather than bytes, the token shape's boundaries, a token in a
URL, repeats, a secret that is part of the redaction marker, and every digit Python's `\\d` reads:
one token per run of ten decimal digits (Unicode `Nd`, as this interpreter's `unicodedata` lists
them), and seeded tokens of numeric characters that are not decimal digits. Every secret and token
here is synthetic, and no token has the shape the public scrub refuses.
"""

import logging
import os
import unicodedata

#: The variable `config.py:_env_digest_hour` reads.
DIGEST_VARIABLE = "ANKI_DIGEST_HOUR"
#: The shortest numeric id and secret the predecessor's token pattern matches.
TOKEN_ID_DIGITS = 6
TOKEN_SECRET_CHARS = 30
#: The characters a token's secret part may hold.
TOKEN_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-"


def with_digest_variable(env_digest_hour, predecessor, *, raw, rollover_hour):
    """Set the digest-hour variable to `raw` (or remove it when `raw` is None) for one call."""
    saved = os.environ.get(DIGEST_VARIABLE)
    try:
        if raw is None:
            os.environ.pop(DIGEST_VARIABLE, None)
        else:
            os.environ[DIGEST_VARIABLE] = raw
        return env_digest_hour(rollover_hour)
    finally:
        if saved is None:
            os.environ.pop(DIGEST_VARIABLE, None)
        else:
            os.environ[DIGEST_VARIABLE] = saved


def digest_case(raw, rollover_hour):
    return {"raw": raw, "rollover_hour": rollover_hour}


def digest_cases(rng):
    """24 unset, 3 blank, 6 explicit, 2 earlier, 2 out-of-range, 3 unparsable, 2 python-only."""
    drawn = [("unset", digest_case(None, hour)) for hour in range(24)]
    for raw in ("", "   ", "\t"):
        drawn.append(("blank", digest_case(raw, rng.randrange(24))))
    # An explicit hour at or after its rollover hour, so each is resolved rather than refused.
    for raw, hour in (("0", 0), ("9", 9), ("23", 23), (" 7 ", 7), ("+7", 7), ("07", 7)):
        drawn.append(("explicit", digest_case(raw, rng.randrange(hour + 1))))
    drawn.append(("earlier", digest_case("3", 4)))
    drawn.append(("earlier", digest_case("0", 23)))
    for raw in ("24", "-1"):
        drawn.append(("out-of-range", digest_case(raw, rng.randrange(24))))
    for raw in ("nine", "9.5", "0x9"):
        drawn.append(("unparsable", digest_case(raw, rng.randrange(24))))
    # Python's int reads a digit separator and digits outside ASCII; DeckStreak reads neither.
    for raw in ("1_0", "٧"):
        drawn.append(("python-only", digest_case(raw, rng.randrange(24))))
    return drawn


def through_the_filter(secret_redacting_filter, predecessor, *, secrets, text):
    """Run one record carrying `text` through a filter built with `secrets` as its own values."""
    record = logging.LogRecord(
        name="deckstreak.golden",
        level=logging.INFO,
        pathname="",
        lineno=0,
        msg=text,
        args=(),
        exc_info=None,
    )
    secret_redacting_filter(extra_values=tuple(secrets)).filter(record)
    return record.getMessage()


def redaction_case(secrets, text):
    return {"secrets": secrets, "text": text}


def token(rng, id_digits, secret_chars, digits="0123456789"):
    """A synthetic token of the predecessor's shape: a numeric id, a colon and a secret part."""
    identity = "".join(rng.choice(digits) for _ in range(id_digits))
    secret = "".join(rng.choice(TOKEN_ALPHABET) for _ in range(secret_chars))
    return f"{identity}:{secret}"


def word(rng, length):
    return "".join(rng.choice("abcdefghijklmnopqrstuvwxyz") for _ in range(length))


#: The first code point of each run of ten decimal digits (Unicode `Nd`), which `re` reads as \d.
DECIMAL_RUNS = sorted(
    {
        point - unicodedata.decimal(chr(point))
        for point in range(0x110000)
        if unicodedata.category(chr(point)) == "Nd"
    }
)
#: Every numeric character that is not a decimal digit (Unicode `Nl` and `No`), never a \d.
NOT_DECIMAL = [
    chr(point)
    for point in range(0x110000)
    if unicodedata.category(chr(point)) in ("Nl", "No")
]


def redaction_cases(rng):
    """30 classed cases, a decimal-run case per run of ten digits, 4 numeric ones, 10 ordinary."""
    drawn = []
    # A registered secret wherever it stands, and every time it stands there.
    drawn.append(
        ("secret", redaction_case(["kitten-stapler"], "sync as kitten-stapler failed"))
    )
    drawn.append(("secret", redaction_case(["kitten-stapler"], "kitten-stapler")))
    drawn.append(
        (
            "repeat",
            redaction_case(["loom-harbor"], "loom-harbor then loom-harbor again"),
        )
    )
    drawn.append(
        ("repeat", redaction_case(["abcabc"], "abcabcabcabcabc: overlapping repeats"))
    )
    # Longest first: a secret inside a longer one never leaves the longer one half-replaced.
    drawn.append(
        (
            "overlap",
            redaction_case(
                ["river", "riverstone-gate"], "via riverstone-gate and river"
            ),
        )
    )
    drawn.append(
        ("overlap", redaction_case(["stone-gate", "riverstone"], "riverstone-gate end"))
    )
    drawn.append(("overlap", redaction_case(["bbbb", "aaaa"], "aaaabbbb bbbbaaaa")))
    # The minimum length is four characters, counted as characters, never as bytes.
    drawn.append(("short", redaction_case(["abc"], "abc stays when it is three")))
    drawn.append(("short", redaction_case(["abcd"], "abcd goes when it is four")))
    drawn.append(("short", redaction_case(["été"], "été has six bytes")))
    drawn.append(("short", redaction_case(["étés"], "étés has four chars")))
    drawn.append(("short", redaction_case([""], "an empty secret registers nothing")))
    # A secret the marker contains is replaced inside a marker a longer secret left behind.
    drawn.append(
        (
            "marker",
            redaction_case(["REDA", "long-secret-value"], "long-secret-value and REDA"),
        )
    )
    drawn.append(
        (
            "marker",
            redaction_case(["REDACTED"], "an earlier ***REDACTED*** stays marked"),
        )
    )
    # The token shape: at least six digits, a colon, and at least thirty secret characters.
    shortest = token(rng, TOKEN_ID_DIGITS, TOKEN_SECRET_CHARS)
    drawn.append(("token", redaction_case([], f"polling {shortest} now")))
    drawn.append(("token", redaction_case([], f"polling {shortest}")))
    few_digits = token(rng, TOKEN_ID_DIGITS - 1, TOKEN_SECRET_CHARS)
    drawn.append(("token", redaction_case([], f"polling {few_digits} now")))
    few_chars = token(rng, TOKEN_ID_DIGITS, TOKEN_SECRET_CHARS - 1)
    drawn.append(("token", redaction_case([], f"polling {few_chars} now")))
    long_id = token(rng, 11, 33)
    drawn.append(("token", redaction_case([], f"{long_id}.")))
    drawn.append(
        ("token", redaction_case([], f"api/bot{token(rng, 7, 31)}/status?page=5"))
    )
    first, second = token(rng, 7, 30), token(rng, 6, 34)
    drawn.append(("token", redaction_case([], f"{first} and {second}")))
    drawn.append(("token", redaction_case([], f"{first}{second}")))
    drawn.append(("token", redaction_case([], f"x{token(rng, 7, 32)}:tail")))
    drawn.append(("token", redaction_case([], f"12345:{token(rng, 7, 30)}")))
    drawn.append(
        ("token", redaction_case([], f"{token(rng, 7, 30).replace(':', ' : ')}"))
    )
    # A registered secret and a token in one text: the secret goes first, then the token.
    drawn.append(
        (
            "token",
            redaction_case(["amber-lattice"], f"amber-lattice {token(rng, 7, 30)}"),
        )
    )
    # Every digit Python's \d reads: one token per run of ten decimal digits, its id the run's ten
    # digits in order, so a port that misses any digit of any run leaves a token unredacted.
    for start in DECIMAL_RUNS:
        digits = "".join(chr(start + value) for value in range(10))
        secret = "".join(rng.choice(TOKEN_ALPHABET) for _ in range(TOKEN_SECRET_CHARS))
        drawn.append(("decimal-run", redaction_case([], f"{digits}:{secret}")))
    # A numeric character that is not a decimal digit (a Roman numeral, a superscript, a
    # fraction) never makes a token's id.
    for _ in range(4):
        numerals = rng.sample(NOT_DECIMAL, 10)
        drawn.append(
            ("numeric-not-decimal", redaction_case([], token(rng, 7, 30, numerals)))
        )
    # Text the scrub leaves alone, and a secret that is not in the text.
    drawn.append(("none", redaction_case([], "nothing secret here")))
    drawn.append(("none", redaction_case(["quiet-meadow"], "a different line")))
    drawn.append(("none", redaction_case(["percent-sign-value"], "100% of %s and %d")))
    drawn.append(
        ("secret", redaction_case(["tab\tand space"], "held tab\tand space here"))
    )
    # Ordinary cases: seeded secrets in seeded text, sometimes with a token.
    for _ in range(10):
        secrets = [word(rng, rng.randrange(3, 12)) for _ in range(rng.randrange(0, 4))]
        parts = [word(rng, rng.randrange(2, 9)) for _ in range(rng.randrange(3, 8))]
        parts += secrets
        if rng.random() < 0.5:
            parts.append(token(rng, rng.randrange(6, 8), rng.randrange(30, 35)))
        rng.shuffle(parts)
        drawn.append((None, redaction_case(secrets, " ".join(parts))))
    return drawn


FUNCTIONS = {
    "digest_hour": {
        "kind": "adapter",
        "function": "config._env_digest_hour",
        "adapter": with_digest_variable,
        "note": (
            "Sets the predecessor's digest-hour variable to raw, or removes it when raw is "
            "null, for one call with rollover_hour, and restores the environment after it."
        ),
        "cases": digest_cases,
    },
    "redaction": {
        "kind": "adapter",
        "function": "logging_redact.SecretRedactingFilter",
        "adapter": through_the_filter,
        "note": (
            "Constructs the filter with secrets as its own values, runs one log record "
            "carrying text through it, and returns the record's message."
        ),
        "cases": redaction_cases,
    },
    "kernel.constants": {
        "kind": "constants",
        "names": [
            "constants.DEFAULT_ROLLOVER_HOUR",
            "constants.DEFAULT_DIGEST_HOUR",
            "logging_redact._REDACTED",
            "logging_redact._TELEGRAM_TOKEN_RE.pattern",
            "logging_redact._MIN_SECRET_LEN",
            "offload.OFFLOAD_MAX_WORKERS",
            "offload.SLOW_OFFLOAD_MS",
            "database.DB_BUSY_TIMEOUT_MS",
        ],
    },
}
