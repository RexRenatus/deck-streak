"""SPEC-022's registrations: the sync's retry schedule, and the constants the port uses verbatim.

The adapter drives the predecessor's `pipeline.py:GamifyPipeline._sync_attempts` on a stand-in
pipeline that holds only a stub syncer. The stub fails the case's number of attempts, each by the
case's kind (a returned failure, or a `TimeoutError` as `asyncio.wait_for` raises one), and
succeeds after them. The loop's waits go to a recording sleep instead of the clock, and its jitter
draws are the case's fixed values instead of `random.random`. The adapter computes nothing: the
waits, the attempts and the outcome are what the predecessor's loop produced.

The case builder draws only from the `random.Random` the generator seeds, and covers a success at
each attempt, a failure of every attempt by each kind, and jitter draws at both ends of `[0, 1)`.
"""

import asyncio
import types
from unittest import mock

#: How a failed attempt fails: a returned failure, or the timeout `asyncio.wait_for` raises.
KINDS = ("error", "timeout")


class StubSyncer:
    """Fails its first `failures` calls by `kind`, then succeeds."""

    def __init__(self, sync_outcome, failures, kind):
        self.sync_outcome = sync_outcome
        self.failures = failures
        self.kind = kind
        self.calls = 0

    async def sync_now(self):
        self.calls += 1
        if self.calls > self.failures:
            return self.sync_outcome(ok=True, detail="normal")
        if self.kind == "timeout":
            raise TimeoutError
        return self.sync_outcome(ok=False, error="server_error")


def with_stub_syncer(sync_attempts, predecessor, *, failures, kind, jitter_draws):
    """Run the retry loop over a stub syncer, recording its waits and drawing the case's jitter."""
    pipeline = predecessor("pipeline")
    syncer = StubSyncer(predecessor("sync.SyncOutcome"), failures, kind)
    waits = []

    async def recording_sleep(seconds):
        waits.append(seconds)

    draws = iter(jitter_draws)
    with (
        mock.patch.object(pipeline.asyncio, "sleep", recording_sleep),
        mock.patch.object(pipeline.random, "random", lambda: next(draws)),
    ):
        outcome = asyncio.run(sync_attempts(types.SimpleNamespace(_syncer=syncer)))
    return {
        "attempts": syncer.calls,
        "ok": outcome.ok,
        "error": outcome.error,
        "waits_seconds": waits,
    }


def case(failures, kind, jitter_draws):
    return {"failures": failures, "kind": kind, "jitter_draws": jitter_draws}


def cases(rng):
    """A success at each attempt, every attempt failing by each kind, and jitter at its ends."""
    drawn = []
    # The loop draws one jitter per wait, and waits at most twice: two draws cover every case.
    for kind in KINDS:
        for failures in range(4):
            drawn.append(
                ("attempts", case(failures, kind, [rng.random(), rng.random()]))
            )
    # The jitter's ends: no jitter, and the largest draw `random.random` can return.
    for draw in (0.0, 0.9999999999999999):
        drawn.append(("jitter", case(3, "error", [draw, draw])))
    for _ in range(6):
        drawn.append(
            (
                None,
                case(
                    rng.randrange(0, 4), rng.choice(KINDS), [rng.random(), rng.random()]
                ),
            )
        )
    return drawn


FUNCTIONS = {
    "sync_retry": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._sync_attempts",
        "adapter": with_stub_syncer,
        "note": (
            "Runs the loop on a stand-in pipeline holding only a stub syncer that fails the "
            "case's number of attempts by the case's kind; records each wait instead of "
            "sleeping, and draws the jitter from the case's fixed values."
        ),
        "cases": cases,
    },
    "sync.constants": {
        "kind": "constants",
        "names": [
            "constants.SYNC_RETRY_ATTEMPTS",
            "constants.SYNC_RETRY_BASE_SECS",
            "constants.SYNC_RETRY_JITTER_FRAC",
            "constants.SYNC_TIMEOUT_SECS",
            "constants.COLLECTION_OPEN_RETRIES",
            "constants.COLLECTION_OPEN_RETRY_BASE_SECS",
        ],
    },
}
