#!/usr/bin/env python3
"""A stand-in for the harness, for the driver's tests: it speaks the harness's protocol and
computes nothing of the engine's. Each answer is a digest of its input, so a test can tell that
the driver recorded what it sent.

    stand_in_harness.py [--commit HEX] [--stock TEXT]... [--refuse NAME=LABEL]... [--fault KIND]

`--fault` plants one protocol fault: `extra-key` (an answer with a second key), `not-json`,
`words` (a refusal in sentences, not a label), `exit` (exit 1 after the last call) or `close`
(close after the hello).
"""

import argparse
import hashlib
import json
import sys

PINNED = "65e053b829825f534803b2b772c507a73ba936a3"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--commit", default=PINNED)
    parser.add_argument("--stock", action="append", default=[])
    parser.add_argument("--refuse", action="append", default=[])
    parser.add_argument("--fault", default="")
    args = parser.parse_args()
    refusals = dict(item.split("=", 1) for item in args.refuse)
    for line in sys.stdin:
        message = json.loads(line)
        if message["op"] == "hello":
            answer = {"engine_commit": args.commit, "stock_texts": args.stock}
        elif message["op"] == "bye":
            return 1 if args.fault == "exit" else 0
        elif args.fault == "close":
            return 0
        elif message["function"] in refusals:
            label = refusals[message["function"]]
            answer = {"refused": "The engine said: no." if args.fault == "words" else label}
        else:
            canonical = json.dumps(message["input"], sort_keys=True).encode("utf-8")
            answer = {"output": {"stand_in": hashlib.sha256(canonical).hexdigest()[:16]}}
            if args.fault == "extra-key":
                answer["note"] = "extra"
        if args.fault == "not-json" and message["op"] == "call":
            sys.stdout.write("not json\n")
        else:
            sys.stdout.write(json.dumps(answer) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
