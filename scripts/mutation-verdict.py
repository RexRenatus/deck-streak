#!/usr/bin/env python3
"""mutation-verdict: a pull request's mutation plan and verdict, and the weekly battery's
survivors as issue drafts (SPEC-039 R2 to R4, R12; ADR-057).

    python3 scripts/mutation-verdict.py plan --base REF [--head REF] [--root DIR] [--out DIR]
    python3 scripts/mutation-verdict.py judge --plan FILE --class rust|web
                                             [--outcomes FILE] [--tool-exit N]
                                             [--stryker FILE] [--rows FILE]
    python3 scripts/mutation-verdict.py survivors --reports DIR --out DIR [--open-titles FILE]
    python3 scripts/mutation-verdict.py exclusions [--root DIR]

Red stub: every entry point is in place and every class reads green.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

sys.dont_write_bytecode = True


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["plan", "judge", "survivors", "exclusions"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--base")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--out")
    parser.add_argument("--plan")
    parser.add_argument("--class", dest="klass", choices=["rust", "web"])
    parser.add_argument("--outcomes")
    parser.add_argument("--tool-exit", type=int)
    parser.add_argument("--stryker")
    parser.add_argument("--rows")
    parser.add_argument("--reports")
    parser.add_argument("--open-titles")
    args = parser.parse_args(argv)
    if args.verb == "plan" and args.out:
        out = pathlib.Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        plan = {"files": [], "classes": {}, "rows": [], "stryker_mutate": []}
        (out / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    print("mutation: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
