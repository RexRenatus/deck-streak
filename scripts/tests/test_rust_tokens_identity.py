"""SPEC-480 R2 (A6; ADR-497 D2): the progress guard in `rust_tokens` is the identity.

The census reads every path the verdict's own `classify` reads as `rust`, whole, and compares the
guarded function's tokens with a verbatim copy of the base function's, byte for byte. It sits
outside `scripts/mutation-python.json`'s map, so no mutant run executes it. A local run may add
exclude pathspecs at its invocation, one per line in `RUST_CENSUS_PATHSPECS`; under `CI` it
refuses any, so CI reads the population whole.
"""

import json
import os
import subprocess
import unittest

from _support import REPO, examined
from test_mutation_verdict import verdict_module

V = verdict_module()


def base_rust_tokens(text: str) -> list[tuple[int, int]]:
    """(start, end) of each token of Rust code in `text`, read as `c_code_lines` reads Rust: a
    word, a literal whole, or one other character. Whitespace and comments hold no token, so a
    brace or an attribute inside a string, a raw string, a character or a comment is no token.
    The base's function (36c4783a), verbatim but for its own name and the two it calls on V."""
    tokens: list[tuple[int, int]] = []
    at = 0
    while at < len(text):
        char = text[at]
        if char.isspace():
            at += 1
            continue
        if text.startswith("//", at):
            stop = text.find("\n", at)
            at = len(text) if stop < 0 else stop
            continue
        if text.startswith("/*", at):
            at = V.block_end(text, at, nested=True)
            continue
        stop = V.rust_literal(text, at)
        if stop is None and (char.isalnum() or char == "_"):
            stop = at
            while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):
                stop += 1
        stop = at + 1 if stop is None else stop
        tokens.append((at, stop))
        at = stop
    return tokens


def differing(texts, guarded):
    """For a `{path: text}` mapping: the sorted paths whose `guarded` tokens, as JSON text encoded
    as UTF-8, differ from the base copy's, and the total count of the base's tokens."""
    differ, total = [], 0
    for path, text in texts.items():
        base = base_rust_tokens(text)
        total += len(base)
        if json.dumps(guarded(text)).encode("utf-8") != json.dumps(base).encode("utf-8"):
            differ.append(path)
    return sorted(differ), total


def release_population(root, extra):
    """Every path `git ls-files` lists at `root`, past the `extra` exclude pathspecs, that the
    verdict's own `classify` reads as `rust`, sorted."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", ".", *extra],
        check=True,
        capture_output=True,
    ).stdout.decode("utf-8")
    return sorted(path for path in listed.split("\0") if path and V.classify(path) == "rust")


class TheGuardIsTheIdentityOnTheReleasePopulation(unittest.TestCase):
    def test_every_rust_source_reads_the_same_tokens_with_the_guard(self):
        """A6: a positive control first, a guarded side that drops a token read as differing,
        then every Rust source of the release population, read whole, reads byte-equal tokens
        with the guard and with the base copy."""
        given = os.environ.get("RUST_CENSUS_PATHSPECS", "")
        extra = [line for line in given.splitlines() if line.strip()]
        if os.environ.get("CI") and extra:
            self.fail(
                f"RUST_CENSUS_PATHSPECS adds {len(extra)} pathspecs under CI, "
                "where the census reads the population whole"
            )
        print(f"added pathspecs {len(extra)}")
        self.assertEqual(
            differing({"crates/a/src/lib.rs": "fn a() {}"}, lambda text: V.rust_tokens(text)[:-1]),
            (["crates/a/src/lib.rs"], 6),
        )
        paths = examined("rust sources", release_population(REPO, extra))
        texts = {path: (REPO / path).read_text(encoding="utf-8") for path in paths}
        differ, total = differing(texts, V.rust_tokens)
        print(f"compared {total} tokens over {len(paths)} sources")
        self.assertEqual((differ, total > 0), ([], True))


if __name__ == "__main__":
    unittest.main()
