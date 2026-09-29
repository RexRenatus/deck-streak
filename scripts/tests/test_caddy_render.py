"""The Caddy block is rendered for the host from private configuration (SPEC-062 R7; ADR-061,
ADR-007). The render fills the block's three placeholders, refuses one left unfilled, a value that
carries another placeholder or could open a directive, and an upstream that is not loopback, and
its output keeps every header SPEC-032 R6 requires (A7).

    python3 deploy/scripts/render-caddy.py --config FILE [--template FILE] [--out FILE]

The configuration is a JSON object with `host`, `web_root` and `api_upstream`."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "deploy" / "scripts" / "render-caddy.py"
TEMPLATE = REPO / "deploy" / "caddy" / "deck-streak.caddy"
# Addresses built from parts, so no committed line holds one.
PRIVATE_V4 = ".".join(["10", "0", "0", "5"])
PUBLIC_V4 = ".".join(["203", "0", "113", "9"])
GOOD = {
    "host": "app.example.org",
    "web_root": "/usr/local/lib/deck-streak/current/web",
    "api_upstream": "127.0.0.1:8080",
}
HEADERS = (
    'Strict-Transport-Security "max-age=31536000; includeSubDomains"',
    'X-Content-Type-Options "nosniff"',
    'Referrer-Policy "same-origin"',
    'X-Robots-Tag "noindex"',
    "frame-ancestors https://web.telegram.org; object-src 'none'; base-uri 'self'",
    "-Server",
)


def render(config, template=TEMPLATE):
    assert SCRIPT.is_file(), f"{SCRIPT.relative_to(REPO)} does not exist"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "config.json"
        path.write_text(json.dumps(config), encoding="utf-8")
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--config", str(path), "--template", str(template)],
            capture_output=True,
            text=True,
            check=False,
        )


class TheCaddyRender(unittest.TestCase):
    def test_the_render_refuses_placeholders_and_keeps_the_headers(self):
        done = render(GOOD)
        self.assertEqual(done.returncode, 0, done.stderr)
        text = done.stdout
        self.assertNotIn("{$", text, "a placeholder is left in the output")
        for value in GOOD.values():
            self.assertIn(value, text)
        for header in examined("required headers", HEADERS):
            self.assertIn(header, text, f"the render dropped {header}")
        template = TEMPLATE.read_text(encoding="utf-8")
        expected = (
            template.replace("{$DECKSTREAK_HOST}", GOOD["host"])
            .replace("{$DECKSTREAK_WEB_ROOT}", GOOD["web_root"])
            .replace("{$DECKSTREAK_API_UPSTREAM}", GOOD["api_upstream"])
        )
        self.assertEqual(text, expected, "the render changes only the three placeholders")
        for missing in ("host", "web_root", "api_upstream"):
            config = {k: v for k, v in GOOD.items() if k != missing}
            done = render(config)
            self.assertNotEqual(done.returncode, 0, f"a config without {missing}")
            self.assertEqual(done.stdout, "", "a refusal writes no block")
            self.assertIn(missing, done.stderr)
        for upstream in (
            f"{PRIVATE_V4}:8080",
            "0.0.0.0:8080",
            f"{PUBLIC_V4}:8080",
            "example.org:8080",
            "127.0.0.1.example.org:8080",
            "http://127.0.0.1:8080",
            "127.0.0.1",
            f"127.0.0.1:8080 {PRIVATE_V4}:8080",
        ):
            done = render({**GOOD, "api_upstream": upstream})
            self.assertNotEqual(done.returncode, 0, f"the upstream {upstream} is not loopback")
            self.assertEqual(done.stdout, "")
        for upstream in ("127.0.0.1:8080", "localhost:8080", "[::1]:8080", "127.0.1.1:9000"):
            done = render({**GOOD, "api_upstream": upstream})
            self.assertEqual(done.returncode, 0, f"{upstream}: {done.stderr}")
        for key, value in (
            ("host", "{$DECKSTREAK_API_UPSTREAM}"),
            ("host", "app.example.org {\n\trespond 200\n}\nx"),
            ("host", "https://app.example.org"),
            ("web_root", "relative/web"),
            ("web_root", "/srv/web\n\trespond 200"),
            ("web_root", "/srv/{$HOME}"),
        ):
            done = render({**GOOD, key: value})
            self.assertNotEqual(done.returncode, 0, f"{key}={value!r}")
            self.assertEqual(done.stdout, "")
        with tempfile.TemporaryDirectory() as tmp:
            odd = Path(tmp) / "odd.caddy"
            odd.write_text(template + "\nimport {$DECKSTREAK_SECRET}\n", encoding="utf-8")
            done = render(GOOD, odd)
            self.assertNotEqual(done.returncode, 0, "an unknown placeholder is left in the output")
            self.assertEqual(done.stdout, "")


if __name__ == "__main__":
    unittest.main()
