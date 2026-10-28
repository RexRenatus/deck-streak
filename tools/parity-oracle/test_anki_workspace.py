"""The harness stays outside DeckStreak's workspace and no workflow builds it (SPEC-498 R4,
ADR-516): the root manifest's members cover no harness path, the root lock names no package the
harness adds, and no workflow under .github/workflows names the harness or the driver.
"""

import importlib.util
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

spec = importlib.util.spec_from_file_location(
    "anki_generate_workspace", HERE / "anki" / "generate.py"
)
generate = importlib.util.module_from_spec(spec)
sys.modules["anki_generate_workspace"] = generate
spec.loader.exec_module(generate)


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def copy_of_root(scratch):
    """The three inputs of the census, copied: the manifest, the lock and the workflows."""
    copy = Path(scratch)
    shutil.copyfile(ROOT / "Cargo.toml", copy / "Cargo.toml")
    shutil.copyfile(ROOT / "Cargo.lock", copy / "Cargo.lock")
    shutil.copytree(ROOT / ".github" / "workflows", copy / ".github" / "workflows")
    return copy


class OutsideTheWorkspace(unittest.TestCase):
    def test_generator_outside_workspace(self):
        """T-G15: today's tree holds no member, lock package or workflow that reaches the
        harness, and a temporary workflow copy that names it is caught."""
        examined("workflows", (ROOT / ".github" / "workflows").glob("*.y*ml"))
        self.assertEqual(generate.workspace_refusals(ROOT), [])
        with tempfile.TemporaryDirectory() as scratch:
            copy = copy_of_root(scratch)
            planted = copy / ".github" / "workflows" / "planted.yml"
            planted.write_text("jobs: {build: {steps: [{run: cd tools/render-oracle}]}}\n")
            self.assertEqual(
                generate.workspace_refusals(copy),
                [".github/workflows/planted.yml: names tools/render-oracle"],
            )

    def test_a_workflow_that_names_the_harness_is_caught(self):
        """T-G15's control: a temporary workflow copy that names the harness is caught."""
        for needle in examined("names", [*generate.HARNESS_PATHS, generate.HARNESS_PACKAGE]):
            with self.subTest(needle=needle), tempfile.TemporaryDirectory() as scratch:
                copy = copy_of_root(scratch)
                planted = copy / ".github" / "workflows" / "planted.yml"
                planted.write_text(
                    f"jobs:\n  build:\n    steps:\n      - run: cargo build --manifest-path {needle}/Cargo.toml\n",
                    encoding="utf-8",
                )
                refusals = generate.workspace_refusals(copy)
                self.assertEqual(len(refusals), 1, refusals)
                self.assertIn("planted.yml", refusals[0])

    def test_a_lock_that_names_the_harness_package_is_caught(self):
        with tempfile.TemporaryDirectory() as scratch:
            copy = copy_of_root(scratch)
            lock = copy / "Cargo.lock"
            entry = f'\n[[package]]\nname = "{generate.HARNESS_PACKAGE}"\nversion = "0.1.0"\n'
            lock.write_text(lock.read_text(encoding="utf-8") + entry, encoding="utf-8")
            self.assertEqual(
                generate.workspace_refusals(copy),
                [f"Cargo.lock: names {generate.HARNESS_PACKAGE}, the harness's own package"],
            )

    def test_a_workspace_member_that_covers_the_harness_is_caught(self):
        for member in examined(
            "members", ["tools/*", "tools/parity-oracle/anki/harness", "tools/render-oracle/*"]
        ):
            with self.subTest(member=member), tempfile.TemporaryDirectory() as scratch:
                copy = copy_of_root(scratch)
                manifest = copy / "Cargo.toml"
                text = manifest.read_text(encoding="utf-8")
                planted = text.replace("members = [", f'members = [\n    "{member}",', 1)
                self.assertNotEqual(planted, text)
                manifest.write_text(planted, encoding="utf-8")
                refusals = generate.workspace_refusals(copy)
                self.assertTrue(refusals, member)
                self.assertTrue(all(member in r for r in refusals), refusals)


if __name__ == "__main__":
    unittest.main()
