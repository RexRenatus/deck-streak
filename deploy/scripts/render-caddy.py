#!/usr/bin/env python3
"""render-caddy: DeckStreak's Caddy block, with its three placeholders filled (SPEC-062 R7;
ADR-061, ADR-007).

    python3 deploy/scripts/render-caddy.py --config FILE [--template FILE] [--out FILE]

The configuration is a JSON object with `host`, `web_root` and `api_upstream`, kept where the
deployment keeps its own values; the template is the release's `deploy/caddy/deck-streak.caddy`.
The output is the template with only its three placeholders replaced. The render refuses, writing
nothing: a missing key (named), a value that is not the shape its placeholder takes (a host name,
an absolute path, a loopback `host:port`), any value that could carry another placeholder or open
a directive, and a `{$` left in the output. Exit 0 rendered, 1 refused, 2 unreadable input.
"""

import argparse
import json
import re
import sys
from pathlib import Path

PLACEHOLDERS = {
    "host": "{$DECKSTREAK_HOST}",
    "web_root": "{$DECKSTREAK_WEB_ROOT}",
    "api_upstream": "{$DECKSTREAK_API_UPSTREAM}",
}
HOST = re.compile(r"^[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?(?:\.[A-Za-z0-9-]{1,63})*$")
WEB_ROOT = re.compile(r"^/[A-Za-z0-9._@+/-]*$")
UPSTREAM = re.compile(r"^(?:(\d{1,3}(?:\.\d{1,3}){3})|localhost|\[::1\]):(\d{1,5})$")


class Refused(Exception):
    """A value or a file the render will not write a block from."""


def check_host(value):
    if not HOST.match(value) or len(value) > 253:
        raise Refused("host is not a bare host name (no scheme, port, space or brace)")


def check_web_root(value):
    if not WEB_ROOT.match(value) or ".." in value.split("/"):
        raise Refused("web_root is not an absolute path of plain characters")


def check_upstream(value):
    found = UPSTREAM.match(value)
    if not found:
        raise Refused("api_upstream is not a loopback host:port")
    address, port = found.groups()
    if address is not None:
        octets = [int(part) for part in address.split(".")]
        if octets[0] != 127 or any(part > 255 for part in octets):
            raise Refused("api_upstream is not a loopback address")
    if not 0 < int(port) < 65536:
        raise Refused("api_upstream's port is out of range")


CHECKS = {"host": check_host, "web_root": check_web_root, "api_upstream": check_upstream}


def render(config, template):
    if not isinstance(config, dict):
        raise Refused("the configuration is not a JSON object")
    for key in PLACEHOLDERS:
        if key not in config:
            raise Refused(f"the configuration lacks {key}")
        if not isinstance(config[key], str) or not config[key]:
            raise Refused(f"{key} is not a non-empty string")
    for key in sorted(set(config) - set(PLACEHOLDERS)):
        raise Refused(f"the configuration holds {key}, which the block has no placeholder for")
    for key, check in CHECKS.items():
        check(config[key])
    text = template
    for key, placeholder in PLACEHOLDERS.items():
        if placeholder not in text:
            raise Refused(f"the template lacks {placeholder}")
        text = text.replace(placeholder, config[key])
    if "{$" in text:
        raise Refused("a placeholder is left unfilled in the output")
    return text


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--config", required=True, help="the JSON configuration")
    default = Path(__file__).resolve().parent.parent / "caddy" / "deck-streak.caddy"
    parser.add_argument("--template", default=str(default), help="the block's template")
    parser.add_argument("--out", help="write here; the default is standard output")
    args = parser.parse_args(argv)
    try:
        config = json.loads(Path(args.config).read_text(encoding="utf-8"))
        template = Path(args.template).read_text(encoding="utf-8")
    except (OSError, ValueError) as error:
        print(f"VOID: an input cannot be read: {error}", file=sys.stderr)
        return 2
    try:
        text = render(config, template)
    except Refused as error:
        print(f"REFUSE: {error}", file=sys.stderr)
        return 1
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
