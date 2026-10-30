"""SPEC-119's registrations: the bearer guard's constants, its bucket and its limiter (#158), and
the server's tool roster (#157).

Each adapter builds what JSON cannot carry and CALLS the predecessor; it computes no rule.

* `mcp_auth.constants` reads the limiter's window, failure count, bucket cap and history cap, the
  one denial word and the law-track scope's name from `mcp_auth.py`.
* `mcp_auth_bucket` builds a `DrillAuth` over no grants and calls its `_bucket_id` on each value: a
  synthetic token, a malformed header's whole value, a multibyte value and the empty value.
* `mcp_auth_limiter` builds a `DrillAuth` over synthetic grants, each a token joined from the
  case's parts with the case's scope names, and a clock that answers the instant the case set for
  the call, so every read inside one call sees one instant; the case's whole milliseconds are
  divided by 1000 into the predecessor's seconds. It makes the case's calls in order: `require`
  with a token and a scope name, or `_record_failure` on a token's bucket directly. Each call's
  outcome is read from the predecessor's own warning on its logger (`denied` or `rate limit
  tripped`), and a call that raised nothing and warned nothing is `allowed`. After the last call
  it reads the failure map: each bucket in order with its failure times, multiplied back into
  whole milliseconds.

* `mcp_roster` builds the predecessor's settings from an environment holding none of its own
  variables, so every setting is its default, and builds its server over a pipeline that is never
  called and a guard over no grants (#157). It lists the served tools in the order the server
  lists them and records, for each, its name, its annotations, its input schema with every
  `description` key removed, and its output fields in the order its output schema declares them.
  It records no description text: DeckStreak writes its own, and they are no parity surface.

A token is a list of parts, each a string or a `[string, count]` pair repeated that many times, so
no committed case holds a credential-shaped literal. The case builders draw only from the
`random.Random` the generator seeds.
"""

import asyncio
import logging
import os

#: The scope every grant holds, and the law track's, by the predecessor's own spelling.
CORE = "core"
LAW_TRACK = "law_track"

#: The synthetic grants: a core token and a law-track token, each joined from parts.
CORE_TOKEN = ["synthetic-core-", ["c", 24]]
LAW_TOKEN = ["synthetic-law-", ["l", 24]]
GRANTS = [
    {"token": CORE_TOKEN, "scopes": [CORE]},
    {"token": LAW_TOKEN, "scopes": [CORE, LAW_TRACK]},
]

#: Tokens no grant holds, and the empty token, which presents nothing.
WRONG = ["synthetic-wrong-", ["w", 24]]
OTHER = ["synthetic-other-", ["o", 24]]
EMPTY = []


def joined(parts):
    """The token the parts spell: each string as it is, each `[string, count]` repeated."""
    return "".join(part if isinstance(part, str) else part[0] * part[1] for part in parts)


def require(at, token, scope):
    return {"op": "require", "at": at, "token": token, "scope": scope}


def record(at, token):
    return {"op": "record", "at": at, "token": token}


def bucket_cases(rng):
    del rng
    return [
        ("token", {"value": "synthetic-bucket-token"}),
        ("token", {"value": joined(["synthetic-long-", ["t", 40]])}),
        ("malformed", {"value": "Bearer"}),
        ("malformed", {"value": "Bearer "}),
        ("malformed", {"value": "Bearer  two-spaces"}),
        ("malformed", {"value": "Bearer one two"}),
        ("malformed", {"value": "Token synthetic-scheme"}),
        ("multibyte", {"value": "Bearer café-ünïcode"}),
        ("multibyte", {"value": "漢字"}),
        ("empty", {"value": ""}),
    ]


def limiter_cases(rng):
    first = 1_000
    fixed = [
        (
            "sixth-failure",
            {
                "grants": GRANTS,
                "calls": [require(first + 1_000 * n, WRONG, CORE) for n in range(6)],
            },
        ),
        (
            "window-edge",
            {
                "grants": GRANTS,
                "calls": [require(first + 250 * n, WRONG, CORE) for n in range(5)]
                + [require(first + 59_999, WRONG, CORE), require(first + 60_000, WRONG, CORE)],
            },
        ),
        (
            "limited-unrecorded",
            {
                "grants": GRANTS,
                "calls": [require(1_000 * n, WRONG, CORE) for n in range(5)]
                + [require(5_000, OTHER, CORE), require(6_000, WRONG, CORE)],
            },
        ),
        (
            "history-cap",
            {"grants": GRANTS, "calls": [record(1_000 * n, WRONG) for n in range(11)]},
        ),
        (
            "eviction",
            {
                "grants": GRANTS,
                "calls": [require(10 * n, ["spray-", str(n)], CORE) for n in range(513)],
            },
        ),
        (
            "granted-never-limited",
            {
                "grants": GRANTS,
                "calls": [require(1_000 * n, CORE_TOKEN, LAW_TRACK) for n in range(10)]
                + [require(10_000, CORE_TOKEN, CORE), require(10_500, LAW_TOKEN, LAW_TRACK)],
            },
        ),
        (
            "clock-back",
            {
                "grants": GRANTS,
                "calls": [require(10_000 + n, WRONG, CORE) for n in range(5)]
                + [require(4_000, WRONG, CORE), require(4_000, EMPTY, CORE)],
            },
        ),
    ]
    pool = [CORE_TOKEN, LAW_TOKEN, WRONG, WRONG, WRONG, OTHER, EMPTY]
    drawn = []
    for _ in range(8):
        at = 0
        calls = []
        for _ in range(40):
            at += rng.choice([0, 1, 250, 500, 1_000, 2_000, 5_000, 30_000, -500])
            calls.append(require(at, rng.choice(pool), rng.choice([CORE, LAW_TRACK])))
        drawn.append((None, {"grants": GRANTS, "calls": calls}))
    return fixed + drawn


def over_no_grants(bucket_id, predecessor, *, value):
    auth = predecessor("mcp_auth.DrillAuth")(())
    return bucket_id(auth, value)


class Outcomes(logging.Handler):
    """Each warning the predecessor's logger writes during one call, by its message."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())


def with_a_set_clock(require_fn, predecessor, *, grants, calls):
    token_grant = predecessor("mcp_auth.TokenGrant")
    denied = predecessor("mcp_auth.AuthDeniedError")
    instant = [0.0]
    auth = predecessor("mcp_auth.DrillAuth")(
        tuple(
            token_grant(token=joined(grant["token"]), scopes=frozenset(grant["scopes"]))
            for grant in grants
        ),
        clock=lambda: instant[0],
    )
    logger = predecessor("mcp_auth.logger")
    handler = Outcomes()
    logger.addHandler(handler)
    outcomes = []
    try:
        for call in calls:
            instant[0] = call["at"] / 1000
            handler.messages.clear()
            if call["op"] == "record":
                auth._record_failure(auth._bucket_id(joined(call["token"])))
                outcomes.append("recorded")
                continue
            raised = False
            try:
                require_fn(auth, joined(call["token"]), call["scope"])
            except denied:
                raised = True
            if any("rate limit tripped" in message for message in handler.messages):
                outcomes.append("rate_limited")
            elif any("denied" in message for message in handler.messages):
                outcomes.append("denied")
            elif not raised and not handler.messages:
                outcomes.append("allowed")
            else:
                outcomes.append("unclassified")
    finally:
        logger.removeHandler(handler)
    buckets = [
        [bucket, [round(at * 1000) for at in times]] for bucket, times in auth._failures.items()
    ]
    return {"outcomes": outcomes, "buckets": buckets}


def roster_cases(rng):
    del rng
    return [(None, {})]


def without_descriptions(value):
    """`value` with every `description` key removed, at every depth."""
    if isinstance(value, dict):
        return {
            key: without_descriptions(item) for key, item in value.items() if key != "description"
        }
    if isinstance(value, list):
        return [without_descriptions(item) for item in value]
    return value


def at_defaults(create_server, predecessor):
    """The tools the predecessor's server lists with every setting at its default.

    The predecessor's own variables are removed from the environment while its settings are
    built, and restored after, so no variable of the machine that runs the generator can reach the
    roster.
    """
    own = {
        name: value
        for name, value in os.environ.items()
        if name.startswith("ANKI_") or name == "GCP_PROJECT_ID"
    }
    for name in own:
        del os.environ[name]
    try:
        settings = predecessor("config.Settings").from_env()
    finally:
        os.environ.update(own)
    guard = predecessor("mcp_auth.DrillAuth")(())
    server = create_server(object(), settings, drill_auth=guard)
    tools = asyncio.run(server.list_tools())
    listed = []
    for tool in tools:
        annotations = tool.annotations.model_dump(exclude_none=True) if tool.annotations else {}
        output = tool.outputSchema or {}
        listed.append(
            {
                "name": tool.name,
                "annotations": annotations,
                "input_schema": without_descriptions(tool.inputSchema),
                "output_fields": list(output.get("properties", {})),
            }
        )
    return {"tools": listed}


FUNCTIONS = {
    "mcp_auth.constants": {
        "kind": "constants",
        "names": [
            "mcp_auth._RATE_LIMIT_WINDOW_SECS",
            "mcp_auth._RATE_LIMIT_MAX_FAILURES",
            "mcp_auth._RATE_LIMIT_MAX_BUCKETS",
            "mcp_auth._RATE_LIMIT_BUCKET_HISTORY_CAP",
            "mcp_auth.DENIED_MESSAGE",
            "mcp_auth.SCOPE_LAW_TRACK",
        ],
    },
    "mcp_auth_bucket": {
        "kind": "adapter",
        "function": "mcp_auth.DrillAuth._bucket_id",
        "adapter": over_no_grants,
        "note": (
            "Builds a DrillAuth over no grants and calls the function on value: a synthetic "
            "token, a malformed header's whole value, a multibyte value or the empty value; "
            "returns the bucket it answered."
        ),
        "cases": bucket_cases,
    },
    "mcp_auth_limiter": {
        "kind": "adapter",
        "function": "mcp_auth.DrillAuth.require",
        "adapter": with_a_set_clock,
        "note": (
            "Builds a DrillAuth over grants, each token joined from its parts with its scope "
            "names, whose clock answers each call's at divided by 1000, one instant for every "
            "read inside the call; makes the calls in order, require with a token and a scope "
            "or _record_failure on a token's bucket; reads each call's outcome from the "
            "predecessor's own warning, denied or rate limit tripped, and allowed when it "
            "raised and warned nothing; returns the outcomes and the failure map's buckets in "
            "order, each failure time multiplied back into whole milliseconds."
        ),
        "cases": limiter_cases,
    },
    "mcp_roster": {
        "kind": "adapter",
        "function": "server.create_server",
        "adapter": at_defaults,
        "note": (
            "Builds the settings with none of the predecessor's own variables in the "
            "environment, so each is its default, and the server over a pipeline that is never "
            "called and a guard over no grants; lists the tools in the server's order and "
            "returns each one's name, its annotations without unset ones, its input schema with "
            "every description key removed, and its output fields in its output schema's order."
        ),
        "cases": roster_cases,
    },
}
