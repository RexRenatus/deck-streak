"""Every agent document states the maintainer's fork rule (SPEC-034 A8)."""

import unittest

from _support import REPO, examined

DOCUMENTS = ["AGENTS.md", "CLAUDE.md", "CONTRIBUTING.md"]
RULE = (
    "never approve a workflow run from a fork",
    "never merge a pull request whose head repository is not `RexRenatus/deck-streak`",
)


class TheForkRuleIsWrittenDown(unittest.TestCase):
    def test_every_agent_document_states_the_fork_rule(self):
        for name in examined("agent document(s)", DOCUMENTS):
            text = " ".join((REPO / name).read_text(encoding="utf-8").split())
            for clause in RULE:
                self.assertIn(clause, text, name)


if __name__ == "__main__":
    unittest.main()
