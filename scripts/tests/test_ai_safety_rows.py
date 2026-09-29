"""The agent's settings and ai-safety.json (SPEC-043 A4 and A5; ADR-069).

A4 is a census of the tree's settings documents: none names an `apiKeyHelper`. A5 checks the
manifest's structure in every environment, and runs the ai-content-safety probe over it only where
the box-run packs' scripts are present (the public tree holds no probe, ADR-069): the box run is
where every row is judged.
"""

import json
import os
import re
import subprocess
import unittest
from pathlib import Path

from _support import REPO, examined

MANIFEST = REPO / "ai-safety.json"
# The directory that holds the box-run packs' probe scripts, named by the environment when the
# tests run on the box.
PACKS_SCRIPTS_ENV = "DECKSTREAK_PACKS_SCRIPTS"
SKIPPED = {".git", "target", "node_modules", ".sqlx", "__pycache__"}
SETTINGS_SCHEMA = "json.schemastore.org/claude-code-settings.json"


def settings_documents():
    """Every JSON document that declares the Claude Code settings schema, or is named settings."""
    found = []
    for directory, subdirectories, files in os.walk(REPO):
        subdirectories[:] = [d for d in subdirectories if d not in SKIPPED]
        for name in files:
            path = Path(directory) / name
            if not name.endswith(".json"):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            if SETTINGS_SCHEMA in text or name in ("settings.json", "settings.local.json"):
                found.append(path)
    return found


class SettingsAreClean(unittest.TestCase):
    def test_no_settings_file_names_an_api_key_helper(self):
        documents = examined("settings documents", settings_documents())
        self.assertIn(REPO / "agent" / "settings.json", documents)
        for path in documents:
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("apiKeyHelper", text, f"{path} names an apiKeyHelper")
            data = json.loads(text)
            self.assertIn("$schema", data, f"{path} must declare the settings schema")


class ManifestIsGreen(unittest.TestCase):
    def test_every_ai_content_safety_row_is_green(self):
        data = json.loads(MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(data["schema"], "phx.ai.safety.v1")
        tasks = examined("tasks", data["tasks"])
        self.assertEqual(
            sorted(task["id"] for task in tasks),
            ["daily-reading-language", "daily-reading-law"],
        )
        for task in tasks:
            self.assertEqual(task["tools"], [], "the daily reading holds no tool (R8)")
            self.assertEqual(task["on_invalid"], "withhold")
            for name in task["system"] + [task["prompt"]]:
                self.assertTrue((REPO / name).is_file(), f"{name} is missing")
        self.assertTrue((REPO / data["agent"]["settings"]).is_file())
        cases = examined("red-team cases", (REPO / data["redteam"]["cases"]).glob("*.md"))
        sources = {re.search(r'^source: "(\w+)"', c.read_text(), re.M)[1] for c in cases}
        self.assertEqual(sources, {"cards", "memory"})
        scripts = os.environ.get(PACKS_SCRIPTS_ENV)
        if not scripts:
            print(f"the packs' scripts are not present ({PACKS_SCRIPTS_ENV} is unset): the box run judges the rows")
            return
        probe = Path(scripts) / "ai-content-safety-probe.py"
        classes = subprocess.run(
            ["python3", str(probe), "classes"], capture_output=True, text=True, check=True
        ).stdout.split("\n")
        names = examined("classes", [line.split()[0] for line in classes if line.strip()])
        for name in names:
            result = subprocess.run(
                ["python3", str(probe), "--root", str(REPO), "check", name],
                capture_output=True,
                text=True,
                timeout=300,
            )
            if name == "output-echo":  # advisory: it reports and never refuses
                continue
            self.assertEqual(result.returncode, 0, f"{name}: {result.stdout}{result.stderr}")


if __name__ == "__main__":
    unittest.main()
