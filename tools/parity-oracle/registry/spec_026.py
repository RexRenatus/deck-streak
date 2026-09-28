"""SPEC-026's registrations: the poll's error backoff, the send's retry, and the bot's constants.

`poll_backoff` drives the predecessor's `bot.py:CommandBot._note_getupdates_failure` through the
case's number of consecutive failed rounds, on a stand-in bot that holds only the failure counter
and the predecessor's own `_backoff_delay`, and returns the wait the last of them asked for. The
glue counts nothing itself: each failure goes through the predecessor's own bookkeeping.

`send_retry` drives the predecessor's `telegram.py:TelegramNotifier.send_html`, the method whose
default gives a chunk its attempts, on a notifier built with a synthetic token and chat id whose
HTTP client is a stub answering the case's sequence of responses (a 429 with or without its
`retry_after`, a 5xx, a 400, a network error, a 200). The waits go to a recording sleep instead of
the clock. The output is what the predecessor did: whether it delivered, how many requests it made,
each wait, and its monotonic (attempted, delivered) marker afterwards.

`bot.constants` reads the constants the port uses verbatim: the text cap, the inbound caps, the
default wait of a 429 without `retry_after`, and `send_html`'s default attempts.

`bot.timeouts` reads the long-poll timeout and the HTTP client's timeouts from a bot the
predecessor's own constructor built with a synthetic token and chat id. They are `CommandBot`'s
keyword defaults, not module constants, so a constants registration cannot read them: its
`__init__.__kwdefaults__` also holds the injected clock and sleep, which are functions.

The case builders draw only from the `random.Random` the generator seeds.
"""

import asyncio
import types
from unittest import mock

#: A token and chat id with no real shape: the predecessor builds URLs from them and sends nothing.
SYNTHETIC_TOKEN = "synthetic-token"
SYNTHETIC_CHAT = 1

# --------------------------------------------------------------------------- poll_backoff


def after_consecutive_failures(backoff_delay, predecessor, *, failures):
    """The wait the last of `failures` consecutive failed rounds asks for."""
    bot = predecessor("bot")
    note_failure = predecessor("bot.CommandBot._note_getupdates_failure")
    stand_in = types.SimpleNamespace(_poll_failures=0)
    stand_in._backoff_delay = lambda: backoff_delay(stand_in)
    delay = None
    with mock.patch.object(bot.logger, "warning"):
        for _ in range(failures):
            delay = note_failure(stand_in, ValueError("a synthetic failed round"))
    return {"delay_seconds": delay}


def backoff_cases(rng):
    """Each count up to the cap and past it, the exponent's clamp, and the overflow counts."""
    drawn = [("schedule", {"failures": failures}) for failures in range(1, 9)]
    drawn += [("clamp", {"failures": failures}) for failures in (11, 12)]
    drawn += [("overflow", {"failures": failures}) for failures in (1024, 1025, 1026)]
    drawn += [(None, {"failures": rng.randrange(1, 64)}) for _ in range(4)]
    return drawn


# --------------------------------------------------------------------------- send_retry


class StubResponse:
    """The three things the predecessor reads of an HTTP response."""

    def __init__(self, status, body):
        self.status_code = status
        self._body = body
        self.text = str(body)

    def json(self):
        return self._body


class StubClient:
    """Answers each POST with the next response of the case, and counts the requests."""

    def __init__(self, responses, network_error):
        self.responses = list(responses)
        self.network_error = network_error
        self.requests = 0

    async def post(self, url, json):
        self.requests += 1
        answer = self.responses.pop(0)
        if answer.get("network_error"):
            raise self.network_error("a synthetic network error")
        status = answer["status"]
        if status == 200:
            body = {
                "ok": True,
                "result": {"message_id": self.requests, "date": 0, "chat": {"id": 1}},
            }
        else:
            body = {"ok": False, "error_code": status, "description": "synthetic"}
            if "retry_after" in answer:
                body["parameters"] = {"retry_after": answer["retry_after"]}
        return StubResponse(status, body)


def through_stub_responses(send_html, predecessor, *, responses):
    """Send one short message through the predecessor's notifier over a stub client."""
    telegram = predecessor("telegram")
    notifier = predecessor("telegram.TelegramNotifier")(SYNTHETIC_TOKEN, SYNTHETIC_CHAT)
    stub = StubClient(responses, predecessor("telegram.httpx.ConnectError"))
    notifier._client = stub
    waits = []

    async def recording_sleep(seconds):
        waits.append(seconds)

    with (
        mock.patch.object(telegram.asyncio, "sleep", recording_sleep),
        mock.patch.object(telegram.logger, "warning"),
        mock.patch.object(telegram.logger, "error"),
    ):
        delivered = asyncio.run(send_html(notifier, "a synthetic message"))
    return {
        "delivered": delivered,
        "requests": stub.requests,
        "waits_seconds": waits,
        "send_marker": list(notifier.send_marker),
    }


#: One response of each kind the predecessor tells apart.
KINDS = ("ok", "rate_limited", "rate_limited_bare", "server_error", "client_error", "network")


def answer(kind, rng):
    if kind == "ok":
        return {"status": 200}
    if kind == "rate_limited":
        return {"status": 429, "retry_after": rng.randrange(1, 31)}
    if kind == "rate_limited_bare":
        return {"status": 429}
    if kind == "server_error":
        return {"status": rng.choice([500, 502, 503, 504])}
    if kind == "client_error":
        return {"status": 400}
    return {"network_error": True}


def retry_cases(rng):
    """A delivery at each attempt after each kind of failure, three failures of each kind, a
    zero-second `retry_after`, and random sequences, each long enough for every attempt."""
    drawn = [("delivered", {"responses": [{"status": 200}]})]
    for kind in KINDS[1:]:
        drawn.append(("then-delivered", {"responses": [answer(kind, rng), {"status": 200}]}))
        drawn.append(("exhausted", {"responses": [answer(kind, rng) for _ in range(3)]}))
    drawn.append(
        (
            "third-attempt",
            {
                "responses": [
                    answer("rate_limited", rng),
                    answer("server_error", rng),
                    {"status": 200},
                ]
            },
        )
    )
    drawn.append(("zero-wait", {"responses": [{"status": 429, "retry_after": 0}, {"status": 200}]}))
    for _ in range(6):
        drawn.append((None, {"responses": [answer(rng.choice(KINDS), rng) for _ in range(3)]}))
    return drawn


# --------------------------------------------------------------------------- bot.timeouts


def from_a_constructed_bot(command_bot, predecessor):
    """The long-poll timeout and the HTTP client's timeouts of a bot built with the defaults."""
    bot = command_bot(token=SYNTHETIC_TOKEN, chat_id=SYNTHETIC_CHAT, pipeline=None)
    return {
        "long_poll_seconds": bot._longpoll,
        "http_timeout_seconds": bot._client.timeout.as_dict(),
    }


def timeout_cases(rng):
    """One bot, built with every default."""
    return [(None, {})]


FUNCTIONS = {
    "poll_backoff": {
        "kind": "adapter",
        "function": "bot.CommandBot._backoff_delay",
        "adapter": after_consecutive_failures,
        "note": (
            "Records each consecutive failed round through the predecessor's "
            "_note_getupdates_failure on a stand-in holding only the failure counter and "
            "_backoff_delay, with its warning log silenced; returns the wait of the last round."
        ),
        "cases": backoff_cases,
    },
    "send_retry": {
        "kind": "adapter",
        "function": "telegram.TelegramNotifier.send_html",
        "adapter": through_stub_responses,
        "note": (
            "Sends one short message through a notifier built with a synthetic token and chat id "
            "whose HTTP client is a stub answering the case's responses; records each wait "
            "instead of sleeping, with the notifier's logs silenced."
        ),
        "cases": retry_cases,
    },
    "bot.constants": {
        "kind": "constants",
        "names": [
            "constants.TELEGRAM_MAX_LEN",
            "bot._MAX_INBOUND_TEXT",
            "bot._MAX_CALLBACK_DATA",
            "telegram._DEFAULT_RETRY_AFTER",
            "telegram.TelegramNotifier.send_html.__kwdefaults__",
        ],
    },
    "bot.timeouts": {
        "kind": "adapter",
        "function": "bot.CommandBot",
        "adapter": from_a_constructed_bot,
        "note": (
            "Builds the predecessor's CommandBot with a synthetic token and chat id and no "
            "pipeline, and reads the long-poll timeout and the HTTP client's timeouts it holds."
        ),
        "cases": timeout_cases,
    },
}
