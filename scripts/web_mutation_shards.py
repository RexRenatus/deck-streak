#!/usr/bin/env python3
"""The pull request's Mini App mutation sweep in three shards (SPEC-419, ADR-433).

A stub: it holds the names its tests call and does none of the work, so each test of A6 to A13
fails by its own assertion.
"""

import sys

SHARDS = 3
PREFIX = "mutation-web-shard-"
RECORD = "shard.json"
REPORT = "mutation.json"
EXITED = "stryker.exit"
SCHEMA = "deckstreak-web-shard/1"
STRYKER_BIN = "node_modules/@stryker-mutator/core/bin/stryker.js"
EXIT_OK, EXIT_VOID = 0, 3


class Refused(Exception):
    """A count or a record this script cannot read."""


def deal(counts, shards):
    """Deals nothing."""
    return [[] for _ in range(shards)]


def main(argv=None):
    """Parses nothing."""
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
