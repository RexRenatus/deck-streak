"""SPEC-425 R5, A3: the Mini App's type check refuses a switch case that falls through.

The flag lives in the app's own config, the type check reads that config, and the web stage runs the
type check. Each link is read from the tree, so a change that drops any one of them fails here
(ADR-439 D4)."""

import json
import re
import unittest

from _support import REPO, examined

APP = REPO / "web" / "app"
#: The app's `check` script's type check, on the app's own config.
TYPE_CHECK = "svelte-check --tsconfig ./tsconfig.json"
#: `scripts/check.sh`'s web stage, from its opening line to its closing brace.
STAGE_WEB = re.compile(r"^stage_web\(\) \{\n(.*?)^\}", re.MULTILINE | re.DOTALL)


def config_options(text):
    """A TypeScript config's `compilerOptions`, its whole-line comments set aside. Any other comment
    or a trailing comma fails the parse, which fails the test rather than reading a flag as set."""
    kept = [line for line in text.splitlines() if not line.lstrip().startswith("//")]
    return json.loads("\n".join(kept))["compilerOptions"]


class TheTypeCheckRefusesAFallThrough(unittest.TestCase):
    def test_the_type_check_reads_no_fallthrough_from_the_apps_own_config(self):
        options = config_options((APP / "tsconfig.json").read_text(encoding="utf-8"))
        self.assertIs(
            options.get("noFallthroughCasesInSwitch"),
            True,
            "web/app/tsconfig.json lets a switch case fall through",
        )
        check = json.loads((APP / "package.json").read_text(encoding="utf-8"))["scripts"]["check"]
        self.assertIn(
            TYPE_CHECK, check, "the app's check script does not read the app's own config"
        )
        stage = STAGE_WEB.search((REPO / "scripts" / "check.sh").read_text(encoding="utf-8"))
        self.assertIsNotNone(stage, "scripts/check.sh holds no stage_web")
        self.assertIn(
            "pnpm -r check &&", stage.group(1), "the web stage does not run the type check"
        )
        examined("links of the type check's chain", ["tsconfig.json", "package.json", "stage_web"])


if __name__ == "__main__":
    unittest.main()
