"""The committed rulesets are the enforced dev-to-main release workflow (SPEC-033 A9 to A12), and
a release never deadlocks and takes checks only from GitHub Actions (SPEC-034 A1 to A4)."""

import json
import re
import unittest

from _support import REPO, examined

RULESETS = REPO / ".github" / "rulesets"
# The rule types the plan enforces; GitHub refuses `tag_name_pattern` on it (HTTP 422).
ENFORCEABLE = {"deletion", "non_fast_forward", "update", "pull_request", "required_status_checks"}
# The GitHub Actions app: measured as the app id of every ci, fragment and base-is-dev run on dev.
GITHUB_ACTIONS = 15368


def load(name):
    return json.loads((RULESETS / f"{name}.json").read_text(encoding="utf-8"))


def rules(ruleset):
    return {rule["type"]: rule.get("parameters", {}) for rule in ruleset["rules"]}


def checks(ruleset):
    found = rules(ruleset)["required_status_checks"]
    return sorted(check["context"] for check in found["required_status_checks"])


class TheCommittedRulesetsAreTheReleaseWorkflow(unittest.TestCase):
    def test_main_merges_only_by_merge_commit_after_ci_and_fragment(self):
        main = load("main")
        self.assertEqual((main["target"], main["enforcement"]), ("branch", "active"))
        self.assertEqual(main["conditions"]["ref_name"]["include"], ["refs/heads/main"])
        found = rules(main)
        self.assertEqual(found["pull_request"]["allowed_merge_methods"], ["merge"])
        self.assertEqual(checks(main), ["ci", "fragment"])
        self.assertLessEqual({"deletion", "non_fast_forward"}, set(found))

    def test_dev_takes_pull_requests_only_after_ci_and_fragment(self):
        dev = load("dev")
        self.assertEqual((dev["target"], dev["enforcement"]), ("branch", "active"))
        self.assertEqual(dev["conditions"]["ref_name"]["include"], ["refs/heads/dev"])
        found = rules(dev)
        self.assertIn("pull_request", found)
        self.assertEqual(checks(dev), ["ci", "fragment"])
        self.assertLessEqual({"deletion", "non_fast_forward"}, set(found))

    def test_release_tags_block_deletion_update_and_force(self):
        tags = load("release-tags")
        self.assertEqual((tags["target"], tags["enforcement"]), ("tag", "active"))
        self.assertEqual(tags["conditions"]["ref_name"]["include"], ["refs/tags/v*"])
        self.assertEqual(set(rules(tags)), {"deletion", "update", "non_fast_forward"})

    def test_no_ruleset_has_a_bypass_actor_or_a_refused_rule(self):
        for path in examined("ruleset file(s)", sorted(RULESETS.glob("*.json"))):
            ruleset = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(ruleset["bypass_actors"], [], path.name)
            self.assertLessEqual(set(rules(ruleset)), ENFORCEABLE, path.name)


    def test_main_does_not_require_an_up_to_date_head(self):
        # Only this repository's dev reaches main, and one pull request per head and base can be
        # open, so main cannot move under a release pull request; strictness would only deadlock
        # the next release (ADR-034).
        found = rules(load("main"))["required_status_checks"]
        self.assertIs(found["strict_required_status_checks_policy"], False)

    def test_dev_requires_an_up_to_date_head(self):
        found = rules(load("dev"))["required_status_checks"]
        self.assertIs(found["strict_required_status_checks_policy"], True)

    def test_every_required_check_comes_from_github_actions(self):
        required = [
            (name, check)
            for name in ("main", "dev")
            for check in rules(load(name))["required_status_checks"]["required_status_checks"]
        ]
        for name, check in examined("required check(s)", required):
            where = f"{name}: {check['context']}"
            self.assertEqual(check.get("integration_id"), GITHUB_ACTIONS, where)

    def test_the_release_runbook_merges_nothing_back_into_dev(self):
        runbook = " ".join((REPO / "RELEASING.md").read_text(encoding="utf-8").split())
        self.assertIn("Nothing is merged back into dev", runbook)
        self.assertNotRegex(runbook, re.compile(r"git merge [^`]*origin/main"))

if __name__ == "__main__":
    unittest.main()
