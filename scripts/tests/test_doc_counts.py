"""The counts that living documents state are read, and each bound count is derived again
(SPEC-676, ADR-699).

A living document is every tracked Markdown file that no record rule leaves out (R1, R2). Its
prose lines are read by the forms of R3 to R6: a number, in digits or as a word from two up, then
at most three words, then a noun that names something the tree holds (a job, a stage, a row, a
crate, a file, a test, a locale and the like). A count inside a fenced or indented code block, an
inline code span, a block quote, front matter, an HTML comment or a table row is not read, and nor
is a line that names its run, job or attempt id, a figure measured once (R5, R6).

A claim BOUND to a derivation has its count derived again from the tree, and a count that differs
is a site (R7, R8). Every other claim is a site unless the listing names it with its reason (R9).
At `warn` each unlisted site prints a `WARN` line and passes; at `fail` it fails; a listing or a
binding entry that names no current claim fails at both levels (R10). Moving `LEVEL` to `fail` is
its own delivery, under ADR-450 D6 (R11).
"""

import collections
import fnmatch
import re
import subprocess
import unittest

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "doc-counts"
LEVEL = "warn"

# R2: the records whose counts are history by their own rule, each left out by path; the rule
# that makes each one a record is named in SPEC-676 section 1.
RECORDS = re.compile(
    r"^(?:docs/specs/SPEC-[^/]+\.md|docs/decisions/ADR-[^/]+\.md|docs/red-first/[^/]+\.md"
    r"|changelog\.d/(?!README\.md$)[^/]+\.md)$"
)
# R2: a ruling whose authority is the signed commit that adds it, and a released changelog section.
SIGNED = re.compile(r"sign(?:ing|s) the commit that adds this file")
RELEASED = re.compile(r"^## \[\d+\.\d+\.\d+\]")

WORDS = {
    "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7, "eight": 8, "nine": 9,
    "ten": 10, "eleven": 11, "twelve": 12, "thirteen": 13, "fourteen": 14, "fifteen": 15,
    "sixteen": 16, "seventeen": 17, "eighteen": 18, "nineteen": 19, "twenty": 20, "thirty": 30,
    "forty": 40, "fifty": 50, "sixty": 60, "seventy": 70, "eighty": 80, "ninety": 90,
    "hundred": 100,
}  # fmt: skip
NOUNS = (
    "jobs?|stages?|steps?|rows?|crates?|members?|packages?|workflows?|tests?|modules?|files?|"
    "rules?|locales?|catalogs?|migrations?|tables?|routes?|checks?|census(?:es)?|shards?|"
    "lints?|specs?|adrs?|schematics?|personas?|duty|duties|packs?|screens?|fixtures?|keys?|"
    "entry|entries|criterion|criteria|requirements?|sections?|columns?|targets?|binary|"
    "binaries|scripts?|legs?|mutants?|records?|lanes?|suites?|commands?|sources?|projects?|"
    "contexts?|endpoints?|events?|templates?|languages?|apps?|services?|units?|timers?|"
    "triggers?|fields?|platforms?|simulators?|devices?|benches|benchmarks?|probes?|guards?|"
    "tools?|actions?|plants?|cases?|properties|property|obligations?|models?|proofs?|"
    "theorems?|counts?|readers?|registers?|pins?|ids?|bands?"
)
STOP = (
    "a|an|the|of|in|on|at|to|by|for|from|with|and|or|nor|but|is|are|was|were|be|been|when|"
    "then|than|that|which|who|its|it|their|his|her|your|our|my|as|if|per|each|every|all|any|"
    "no|not|into|onto|over|under|apart|more|less|before|after|so|do|does|did|has|have|had|"
    "can|may|must|will|would|should|could|this|these|those|there|here|out|up|down|off|about|"
    "between|through|via|while|where|until|since|times"
)
# R3: a number no identifier, version, date, path or line citation holds, then at most three
# words that are not function words, then the noun. "\x01" stands for an inline code span.
CLAIM = re.compile(
    r"(?<![\w.\-#/:@$`])(?P<count>\d{1,6}|" + "|".join(WORDS) + r")(?![\w.\-/:%]|\.\d)"
    r"(?:\s+(?:(?!(?:" + STOP + r")\b)[A-Za-z][\w'\-]*|\x01)){0,3}?\s+(?P<noun>" + NOUNS + r")\b",
    re.I,
)
FENCE = re.compile(r"^(`{3,}|~{3,})(.*)$")
DELIMITER = re.compile(r"^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)+\|?\s*$")
ITEM = re.compile(r"^\s*(?:[-*+]|\d{1,9}[.)])\s+")
ONCE = re.compile(r"\b(?:run|job|attempt)s?\s+`?\d{6,}", re.I)
SPAN = re.compile(r"(`+)(.+?)\1")
TARGET = re.compile(r"\]\([^)]*\)")
TAG = re.compile(r"<[^>]+>")

Claim = collections.namedtuple("Claim", "document line count noun words ordinal")

# R7: each bound claim, as (document, the claim's words, derivation, argument, argument).
BOUND = (
    (
        "docs/TESTING.md",
        "five jobs",
        "jobs-running",
        ".github/workflows/ci.yml",
        "bash scripts/check.sh",
    ),
    (
        "docs/schematics/the-web-apps-seven-message-catalogs-and-the-census-that-holds-them-whole.md",
        "seven message catalogs",
        "tracked",
        "web/app/messages/*.json",
        "",
    ),
)
# R9: today's sites, each as (key, reason); measured by the hygiene job of the first push.
LISTED = (
    ("PRIVACY.md 2 commands 'two commands' #1", "unbound at hygiene job 130649809572"),
    ("RELEASING.md 3 mutants 'three named mutants' #1", "unbound at hygiene job 130649809572"),
    ("deploy/README.md 2 steps 'two steps' #1", "unbound at hygiene job 130649809572"),
    ("deploy/README.md 3 scripts 'three scripts' #1", "unbound at hygiene job 130649809572"),
    ("docs/BUILDER-BRIEF.md 6 jobs 'Six CI jobs' #1", "unbound at hygiene job 130649809572"),
    ("docs/BUILDER-BRIEF.md 2 mutants 'two mutants' #1", "unbound at hygiene job 130649809572"),
    ("docs/CONTEXT-MAP.md 14 tables 'fourteen tables' #1", "unbound at hygiene job 130649809572"),
    ("docs/CONTEXT-MAP.md 2 contexts 'two contexts' #1", "unbound at hygiene job 130649809572"),
    ("docs/CONTEXT-MAP.md 4 contexts 'Four contexts' #1", "unbound at hygiene job 130649809572"),
    (
        "docs/CONTEXT-MAP.md 3 contexts 'Three domain contexts' #1",
        "unbound at hygiene job 130649809572",
    ),
    ("docs/CONTEXT-MAP.md 2 contexts 'two contexts' #2", "unbound at hygiene job 130649809572"),
    ("docs/CONTEXT-MAP.md 64 tables '64 tables' #1", "unbound at hygiene job 130649809572"),
    (
        "docs/clean-room/card-renderer.md 4 files '4 files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md 4 packages '4 packages' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md 2 tests 'Two settle-census tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md 2 workflows 'two public workflows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md 2 lane 'two lane' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md 2 lane 'two lane' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md 2 files 'two files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md 2 files 'two files' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-10-29-web-and-ios-and-rex.md 2 apps 'two apps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/rulings/OWNER-RULING-2026-11-03-tide-palette.md 2 sources 'two sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    ("docs/runbooks/host-scrub.md 3 tools 'three tools' #1", "unbound at hygiene job 130649809572"),
    (
        "docs/runbooks/sync-server-cutover.md 2 entries 'two entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/runbooks/sync-server-cutover.md 2 triggers 'two triggers' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/agent-pin-token-groups-proof.md 2 counts 'two counts' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/agent-pin-token-groups-proof.md 2 counts 'two counts' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/app-clients-engine-and-sync.md 9 records '9 records' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/apple-build-on-change-and-on-tag.md 14 jobs 'fourteen jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/apple-build-on-change-and-on-tag.md 2 workflows 'two workflows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/async-robustness-checks.md 3 checks 'Three checks' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/async-robustness-checks.md 2 censuses 'two censuses' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/async-robustness-checks.md 3 test 'three test' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/async-robustness-checks.md 2 test 'two test' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/async-robustness-checks.md 4 crates 'four crates' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/badges-records-and-the-next-milestone.md 2 steps 'two steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/card-frame-channels.md 5 tests 'five tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/card-frame-channels.md 20 tests '20 pair tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 5 jobs 'five gate jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 2 stages 'two stages' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 4 jobs 'four gate jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 4 checks 'four checks' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 15 files '15 files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 4 checks 'four checks' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-jobs-and-caches.md 3 reader 'Three reader' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-tool-pins-and-job-timeouts.md 3 checks 'three checks' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-tool-pins-and-job-timeouts.md 25 entries '25 entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-tool-pins-and-job-timeouts.md 6 workflow '6 workflow' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-tool-pins-and-job-timeouts.md 55 jobs '55 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ci-tool-pins-and-job-timeouts.md 4 jobs '4 call jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/codspeed-benches.md 4 jobs 'four bench jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/codspeed-benches.md 17 jobs '17 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/coverage-report.md 3 suites 'three suites' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 3 census 'three census' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 33 files '33 files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 32 files '32 files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 6 crates 'six crates' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 57 sources '57 sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 349 sources '349 sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 11 rows '11 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 341 sources '341 sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 13 tests 'thirteen tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 10 fixture 'ten fixture' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 3 tests 'three tree tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/data-integrity-ii-checks.md 6 crates 'six crates' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/digest-and-weekly-report.md 5 schematics 'five accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/digest-and-weekly-report.md 2 jobs 'two jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/discipline-tick-and-standby-notice.md 4 schematics 'four accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/docs-mermaid-reader-and-its-lean-statement.md 4 columns 'four columns' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/domain-integrity-checks.md 3 source 'Three source' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/domain-integrity-checks.md 5 rules 'five rules' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/domain-integrity-checks.md 3 contexts 'three contexts' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        'docs/schematics/due-card-scores-reorder.md 2 sources "two client adapters\' sources" #1',
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/ffi-adapter-xcframework-and-swift-package.md 2 checks 'two output checks' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/fine-verdict-and-refund.md 3 schematics 'three accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/first-deploy-and-gates.md 6 units 'six units' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/fitness-tests-crate-edges-formal-vectors-and-clock-reads.md 3 tests 'three fitness tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/formal-bounded-settings.md 2 readers 'Two readers' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/formal-check-settings.md 2 readers 'Two readers' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md 2 ids 'two ids' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md 2 steps 'two schedule steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/host-scrub.md 3 tools 'three tools' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/kani-run-budget.md 2 readers 'Two readers' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/landmarks-milestones-and-the-pinned-widget.md 5 schematics 'five accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/landmarks-milestones-and-the-pinned-widget.md 2 steps 'two new steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/market-position-lifecycle.md 4 schematics 'four accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mcp-guard-refusals.md 2 tools 'two withdrawn tools' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/merge-queue.md 2 rules 'two pull request rules' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-rows-population-shape.md 3 spec 'three SPEC' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 20 mutants '20 listed mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 272 legs '272 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 32 jobs '32 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 256 jobs '256 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 15 jobs '15 other jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 225 legs '225 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 256 jobs '256 jobs' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 9 files 'Nine listed files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 2 jobs 'two matrix jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 256 legs '256 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 2 legs 'two legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 256 legs '256 legs' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 2 tests 'two `code` tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 24 legs '24 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 19 legs '19 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 6 mutants 'six mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing-python.md 6 mutants 'six mutants' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2 tests 'two tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 32 legs '32 whole-tree legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2 properties 'two properties' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2512 rows '2512 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 331 rows '331 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 158 rows '158 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2512 rows '2512 rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 15 legs '15 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2370 rows '2370 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 16 legs '16 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 157 rows '157 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2686 rows '2686 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2687 rows '2687 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 2539 rows '2539 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 16 legs '16 legs' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 17 legs '17 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 150 rows '150 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 47 jobs '47 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 3 shards 'three shards' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 3 shards 'three shards' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 256 legs '256 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 190 legs '190 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 11 legs '11 draws how many legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/mutation-testing.md 8 jobs '8 mutation jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/notification-policy-check-population.md 26 command '26 command' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/notification-router.md 3 guards 'Three guards' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/nudge-coordinator-and-holdout.md 7 schematics 'seven accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/owner-press-to-recorded-grade.md 2 entry 'two entry' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/owner-press-to-recorded-grade.md 4 fixture 'four fixture' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/owner-press-to-recorded-grade.md 2 rows 'two rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/owner-press-to-recorded-grade.md 3 rows 'three rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/path-gate-634.md 2 theorems 'two theorems' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/perf-census.md 2 files 'two files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/perf-census.md 2 files 'two files' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/push-senders-and-remote-harness.md 5 events 'five events' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/python-stage-module-pool.md 3 suites 'three python suites' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/release-mutant-legs.md 2 jobs 'Two jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/release-mutant-legs.md 143 legs '143 completed legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/release-mutant-legs.md 256 jobs '256 jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/release-mutant-legs.md 17 legs '17 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/release-profile-cache.md 2 keys 'two keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/scheduler-tests-take-their-time-from-one-injected-clock.md 4 tests 'four tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/scheduler-tests-take-their-time-from-one-injected-clock.md 2 modules 'two clock modules' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/score-pillar-descriptions.md 7 locales 'seven locales' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/smoke-bomb-spend-and-the-chest-lock.md 2 schematics 'two schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/tagline.md 7 catalogs 'seven catalogs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 18 obligations '18 obligations' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 4 ids '4 ids' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 3 models 'three sync models' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 11 properties '11 properties' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 52 properties '52 properties' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 27 entries '27 entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-formal-obligations-ledger-and-its-checker.md 43 rows '43 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-op-log-model-and-its-four-proofs.md 4 proofs 'four proofs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/the-skip-write-tests-other-client-change-always-reaches-the-server.md 5 jobs '5 cancelled `code` jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/tide-tokens.md 2 entries 'two entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/umbrella-ffi-crate-and-one-static-library.md 2 steps 'two new steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/umbrella-ffi-crate-and-one-static-library.md 2 steps 'Two steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/wager-and-contract-lifecycle.md 4 schematics 'four accepted schematics' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/wager-and-contract-lifecycle.md 2 steps 'two steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/web-sink-census.md 111 files '111 tracked files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/web-sink-census.md 112 files '112 tracked files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/web-sink-census.md 2 tests 'Two new tests' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/schematics/web-sync-credential.md 2 key 'two key' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/sealed-sync/device-census.md 2 tables 'two tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/sealed-sync/device-census.md 9 contexts 'nine contexts' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/sealed-sync/device-census.md 2 rows 'two rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    ("docs/specs/_TEMPLATE.md 2 commands 'two commands' #1", "unbound at hygiene job 130649809572"),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 32 shards '32 shards' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 16 shards '16 shards' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 154 mutants '154 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 316 mutants '316 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 310 mutants '310 missed mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 33 files '33 files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 274 mutants '274 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 8 mutants '8 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 27 rows '27 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 33 files '33 Rust files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 8 app '8 Mini App' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 48 rows '48 selected rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 207 mutants '207 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 27 shards '27 shards' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 67 mutants '67 new mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 207 mutants '207 mutants' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 188 keys '188 shared keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 3 files 'three files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 2 mutants 'two mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 0 records '0 records' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 4 fixtures 'four fixtures' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 27 shards '27 shards' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 67 mutants '67 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 297 mutants '297 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 17 files '17 missed across seven files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 24 mutants '24 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 4 files 'four files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 146 mutants '146 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 57 mutants '57 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 29 mutants '29 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 28 mutants '28 uncovered mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 207 mutants '207 mutants' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 27 shards '27 shards' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 8 files '8 changed files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 266 mutants '266 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 2 crates 'two crates' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 410 mutants '410 mutants' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 2 crates 'two crates' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 2 rows 'two rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 2 crates 'two crates' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 3 modules 'three `code` modules' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 3 test 'three test' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 11 records '11 records' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md 32 legs '32 legs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-079-a-focus-block-runs-on-the-services-clock-and-deep-work-earns-once.md 3 sources 'three sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-079-a-focus-block-runs-on-the-services-clock-and-deep-work-earns-once.md 3 sources 'three sources' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-080-daily-quests-the-weekly-quest-and-the-ghost-race-pay-once-and-settle-once.md 1 key '1 selectable key' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-080-daily-quests-the-weekly-quest-and-the-ghost-race-pay-once-and-settle-once.md 7 tables 'seven tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-090-each-course-is-forecast-the-daily-goal-adapts-and-the-balance-names-a-neglected-course.md 5 checks 'five structural checks' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-093-a-leech-is-snapshotted-each-sync-and-its-remediation-pays-once-and-can-be-undone.md 2 actions 'two owner actions' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-093-a-leech-is-snapshotted-each-sync-and-its-remediation-pays-once-and-can-be-undone.md 2 counts 'two counts' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-093-a-leech-is-snapshotted-each-sync-and-its-remediation-pays-once-and-can-be-undone.md 3 counts 'three counts' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-095-the-echo-test-the-price-of-a-day-off-bench-ii-and-the-docket-run-weekly-on-the-frame.md 4 sections 'four sections' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-095-the-echo-test-the-price-of-a-day-off-bench-ii-and-the-docket-run-weekly-on-the-frame.md 3 bands 'three bands' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-096-the-illusion-ledger-and-the-hanzi-dividend-read-the-owners-note-conventions.md 2 sections 'two sections' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-096-the-illusion-ledger-and-the-hanzi-dividend-read-the-owners-note-conventions.md 2 bands '2 bands' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-097-dead-air-the-fluency-trap-the-tilt-test-and-the-other-hand-read-how-the-owner-studies.md 4 sections 'four sections' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-097-dead-air-the-fluency-trap-the-tilt-test-and-the-other-hand-read-how-the-owner-studies.md 0 rows '000 manual rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-098-divestment-day-the-hand-the-sabbatical-clock-and-the-runway-price-the-owners-time.md 8 rows '8 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-098-divestment-day-the-hand-the-sabbatical-clock-and-the-runway-price-the-owners-time.md 8 rows '8 rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-098-divestment-day-the-hand-the-sabbatical-clock-and-the-runway-price-the-owners-time.md 8 rows '8 rows' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-098-divestment-day-the-hand-the-sabbatical-clock-and-the-runway-price-the-owners-time.md 8 rows '8 rows' #4",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-099-the-can-do-ladder-unlocks-a-rung-in-the-owners-own-words-after-each-sync.md 5 rows '5 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md 3 jobs 'three nudge jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md 6 files 'six files' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md 3 jobs 'three jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md 2 ids 'two ids' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md 2 jobs 'two jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md 8 rows '8 most recent rollup rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md 6 files 'six files' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md 2 jobs 'two jobs' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-103-a-fine-is-recorded-once-beside-its-coins-and-refunded-once-when-reversed.md 2 sources 'two refund sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-104-the-doomscroll-rail-judges-each-ping-and-fines-only-what-the-evidence-still-holds.md 7 step '7 clean days step' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-104-the-doomscroll-rail-judges-each-ping-and-fines-only-what-the-evidence-still-holds.md 4 tables 'four tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-105-committed-windows-and-hard-mode-pay-on-evidence-and-a-quarter-hourly-tick-reminds.md 3 jobs 'three jobs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-105-committed-windows-and-hard-mode-pay-on-evidence-and-a-quarter-hourly-tick-reminds.md 2 duties 'Two duties' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-105-committed-windows-and-hard-mode-pay-on-evidence-and-a-quarter-hourly-tick-reminds.md 4 tables 'four tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-106-wagers-and-contracts-stake-only-coins-settle-on-evidence-and-a-panic-waits-for-the-rollover.md 5 tables 'five tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-106-wagers-and-contracts-stake-only-coins-settle-on-evidence-and-a-panic-waits-for-the-rollover.md 2 fields 'two new fields' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-106-wagers-and-contracts-stake-only-coins-settle-on-evidence-and-a-panic-waits-for-the-rollover.md 5 tables 'five tables' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md 6 rows '6 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md 14 rows '14 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md 14 rows '14 rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md 14 rows '14 rows' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md 3 sources 'three sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-108-a-smoke-bomb-cancels-one-days-pending-coin-fines-and-a-chest-lock-makes-the-days-first-chest-common.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-109-the-skip-day-has-a-switch-and-only-the-first-three-skips-of-a-month-bridge-the-streaks.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-109-the-skip-day-has-a-switch-and-only-the-first-three-skips-of-a-month-bridge-the-streaks.md 6 files 'six files' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-112-the-leech-doctor-prepares-a-remedy-for-each-failing-card-before-its-next-review.md 3 sections 'three sections' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-112-the-leech-doctor-prepares-a-remedy-for-each-failing-card-before-its-next-review.md 10 rows '10 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-116-the-inbox-curator-files-captures-byte-for-byte-the-daily-note-links-the-day-and-the-weekly-synthesis-cites-every-claim.md 3 sections 'three sections' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-116-the-inbox-curator-files-captures-byte-for-byte-the-daily-note-links-the-day-and-the-weekly-synthesis-cites-every-claim.md 3 sections 'three sections' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-116-the-inbox-curator-files-captures-byte-for-byte-the-daily-note-links-the-day-and-the-weekly-synthesis-cites-every-claim.md 3 duties 'three duties' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-117-the-vault-pass-runs-nightly-and-on-vaultops-one-at-a-time-on-the-host-bounded-at-600-seconds.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-120-the-collection-atlas-is-a-series-refreshed-from-changed-cards-after-each-sync-and-the-agent-reads-it-in-pages.md 8 keys 'eight keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-120-the-collection-atlas-is-a-series-refreshed-from-changed-cards-after-each-sync-and-the-agent-reads-it-in-pages.md 2 spec 'two beside SPEC' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-131-passkeys-google-and-apple-link-to-the-owners-telegram-account-and-every-doubtful-assertion-is-refused.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 8 packs 'Eight packs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 2 packs 'two packs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 5 rows 'Five rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 2 rows 'Two rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 6 packs 'Six box-run packs' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 5 rows 'five blocking rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md 5 rows 'five blocking rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-135-keepsake-art-is-drawn-through-an-image-port-with-none-wired-capped-cached-gated-and-sent-by-a-job-and-its-absence-leaves-the-text-ceremony-standing.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-135-keepsake-art-is-drawn-through-an-image-port-with-none-wired-capped-cached-gated-and-sent-by-a-job-and-its-absence-leaves-the-text-ceremony-standing.md 2 rows 'two settings rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-137-the-public-achievement-page-is-opt-in-allow-listed-served-by-deckstreak-and-withdrawn-by-switching-off-or-erasing.md 2 criteria 'two criteria' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-137-the-public-achievement-page-is-opt-in-allow-listed-served-by-deckstreak-and-withdrawn-by-switching-off-or-erasing.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-138-future-grants-are-repriced-per-source-from-the-next-study-day.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-139-the-public-landing-page-is-a-static-astro-site-that-states-only-what-deckstreak-does-on-an-origin-the-owner-configures.md 7 locales 'seven locales' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 64 tables '64 tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 4 censuses 'Four censuses' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 8 rule '8 rule' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 8 rows '8 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 8 rows '8 rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md 8 rule '8 rule' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 38 tables '38 tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 8 rows '8 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 8 rows '8 rows' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 3 tables 'Three split tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 8 rows '8 rows' #3",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 8 rows '8 rows' #4",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 4 keys 'four money-rung keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 8 rule '8 rule' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md 3 tables 'three split tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 2 records 'Two accepted records' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 64 source '64 source' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 3 test 'three test' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 7 row '7 row' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 4 keys 'four keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 1 files '1 naming both files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 103 tables '103 DeckStreak tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 8 rows '8 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-142-the-v9-import-reads-a-verified-copy-runs-a-dry-run-first-and-applies-once-over-a-verified-backup-with-a-rollback.md 2 tables 'two tables' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-143-the-cutover-checklist-moves-each-contract-one-at-a-time-and-no-contract-has-two-writers-at-any-step.md 2 steps 'two steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-143-the-cutover-checklist-moves-each-contract-one-at-a-time-and-no-contract-has-two-writers-at-any-step.md 4 commands 'four commands' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-144-the-predecessor-retires-only-after-the-owners-go-and-a-day-of-deckstreak-alone-is-judged-by-every-slo.md 2 commands 'two commands' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-144-the-predecessor-retires-only-after-the-owners-go-and-a-day-of-deckstreak-alone-is-judged-by-every-slo.md 2 steps 'two steps' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-146-the-nightly-stats-file-keeps-the-predecessors-contract-and-is-written-after-the-days-one-sync-by-one-writer.md 19 keys '19 top-level keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-146-the-nightly-stats-file-keeps-the-predecessors-contract-and-is-written-after-the-days-one-sync-by-one-writer.md 64 rows '64 rows' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-146-the-nightly-stats-file-keeps-the-predecessors-contract-and-is-written-after-the-days-one-sync-by-one-writer.md 7 keys 'seven keys' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-150-a-tagged-notes-flashcards-become-candidates-that-only-the-owners-tap-approves.md 0 entries '000 entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-150-a-tagged-notes-flashcards-become-candidates-that-only-the-owners-tap-approves.md 0 units '000 UTF-16 code units' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-150-a-tagged-notes-flashcards-become-candidates-that-only-the-owners-tap-approves.md 6 files 'six files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-151-the-package-holds-only-approved-cards-keeps-each-guid-and-reaches-only-the-owners-chat.md 2 files 'two temporary files' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-152-the-mini-apps-vault-card-screen-lets-the-owner-approve-edit-or-reject-each-pending-card.md 3 screens 'three screens' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-152-the-mini-apps-vault-card-screen-lets-the-owner-approve-edit-or-reject-each-pending-card.md 7 locales 'seven locales' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-152-the-mini-apps-vault-card-screen-lets-the-owner-approve-edit-or-reject-each-pending-card.md 2 fields 'two text fields' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-152-the-mini-apps-vault-card-screen-lets-the-owner-approve-edit-or-reject-each-pending-card.md 7 locales 'seven locales' #2",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-607-focus-blocks-in-the-study-clients-run-on-one-timer-that-carries-the-focus-parity-rows.md 3 sources 'three sources' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-619-an-ai-duty-runs-on-the-device-first-then-on-the-deployments-route-and-reaches-a-third-party-only-with-consent.md 20 entries '20 gate entries' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "docs/specs/planned/SPEC-619-an-ai-duty-runs-on-the-device-first-then-on-the-deployments-route-and-reaches-a-third-party-only-with-consent.md 3 routes 'three routes' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "tools/parity-oracle/README.md 2 modules 'two modules' #1",
        "unbound at hygiene job 130649809572",
    ),
    (
        "vendor/deepmerge/PATCHED.md 2 files 'two built files' #1",
        "unbound at hygiene job 130649809572",
    ),
    ("vendor/deepmerge/PATCHED.md 3 files 'three files' #1", "unbound at hygiene job 130649809572"),
)


def line_kinds(text):
    """Each line's kind (R4, R5): prose, table, code, front, quote or comment."""
    lines = text.split("\n")
    kinds = ["prose"] * len(lines)
    if lines[0].rstrip() == "---":
        ends = [at for at in range(1, len(lines)) if lines[at].rstrip() in ("---", "...")]
        for at in range(ends[0] + 1 if ends else 0):
            kinds[at] = "front"
    fence, comment, blank, listed, table = "", False, True, False, False
    for at, line in enumerate(lines):
        bare = line.lstrip(" ")
        if kinds[at] == "front":
            continue
        if fence:
            kinds[at] = "code"
            if re.fullmatch(re.escape(fence[0]) + "{" + str(len(fence)) + r",}\s*", bare):
                fence = ""
            continue
        opened = FENCE.match(bare)
        if opened and not (opened.group(1)[0] == "`" and "`" in opened.group(2)):
            fence, kinds[at] = opened.group(1), "code"
            continue
        if comment or bare.startswith("<!--"):
            kinds[at], comment = "comment", "-->" not in line
            continue
        if bare.startswith(">"):
            kinds[at], blank = "quote", False
            continue
        if not line.strip():
            blank, table = True, False
            continue
        indent = len(line) - len(bare)
        listed = bool(ITEM.match(line)) or (listed and indent > 0)
        if indent >= 4 and blank and not listed:
            kinds[at] = "code"
            continue
        if table or ("|" in line and at + 1 < len(lines) and DELIMITER.match(lines[at + 1])):
            kinds[at], table = "table", True
        blank = False
    return kinds


def living_text(name, text):
    """The text of `name` the census reads, or None for a record (R1, R2)."""
    if not name.endswith(".md") or RECORDS.match(name):
        return None
    if name.startswith("docs/rulings/") and SIGNED.search(text):
        return None
    if name == "CHANGELOG.md":
        lines = text.split("\n")
        first = next((at for at, line in enumerate(lines) if RELEASED.match(line)), len(lines))
        return "\n".join(lines[:first])
    return text


def claims(name, text):
    """Every count claim on a prose line of `name` (R3 to R6), with its ordinal among equal words."""
    found, seen = [], collections.Counter()
    for number, (line, kind) in enumerate(zip(text.split("\n"), line_kinds(text)), 1):
        if kind != "prose" or ONCE.search(line):
            continue
        bare = SPAN.sub("\x01", TAG.sub(" ", TARGET.sub("]", line)))
        bare = ITEM.sub(lambda item: " " * len(item.group(0)), bare, count=1)
        for match in CLAIM.finditer(bare):
            words = " ".join(match.group(0).replace("\x01", "`code`").split())
            spelt = match.group("count").lower()
            seen[words] += 1
            count = WORDS[spelt] if spelt in WORDS else int(spelt)
            found.append(
                Claim(name, number, count, match.group("noun").lower(), words, seen[words])
            )
    return found


def key(claim):
    return f"{claim.document} {claim.count} {claim.noun} {claim.words!r} #{claim.ordinal}"


def jobs_running(tree, workflow, command):
    """How many jobs of `workflow` hold a line that runs `command`."""
    jobs, current = collections.Counter(), None
    for line in tree[workflow].split("\n"):
        heading = re.fullmatch(r"  ([A-Za-z0-9_-]+):\s*", line)
        if heading:
            current = heading.group(1)
        elif current and line.startswith("    ") and command in line:
            jobs[current] += 1
    return len(jobs)


def tracked(tree, pattern, _unused=""):
    """How many tracked paths match `pattern`."""
    return len([name for name in tree if fnmatch.fnmatchcase(name, pattern)])


DERIVATIONS = {"jobs-running": jobs_running, "tracked": tracked}


def sites(tree, documents, bound, listed):
    """(sites, stale, bound claims seen): each site as (key, form) (R7 to R10)."""
    found, stale, seen = [], [], 0
    texts = {name: living_text(name, tree[name]) for name in documents}
    every = [
        claim for name, text in texts.items() if text is not None for claim in claims(name, text)
    ]
    binding = {(document, words): rest for document, words, *rest in bound}
    for document, words in binding:
        if not any((claim.document, claim.words) == (document, words) for claim in every):
            stale.append(f"bound {document} {words!r}")
    for claim in every:
        rest = binding.get((claim.document, claim.words))
        if rest is None:
            found.append((key(claim), "unbound"))
            continue
        seen += 1
        derivation, first, second = rest
        if DERIVATIONS[derivation](tree, first, second) != claim.count:
            found.append((key(claim), "mismatch"))
    keys = {site for site, _form in found}
    stale += [f"listed {entry}" for entry, _reason in listed if entry not in keys]
    return found, stale, seen


def verdict(found, stale, listed, level):
    """(printed lines, failures): an unlisted site warns at `warn` and fails at `fail`; a stale
    entry fails at both (R10)."""
    names = {entry for entry, _reason in listed}
    unlisted = [f"WARN doc-count {form} {site}" for site, form in found if site not in names]
    stales = [f"STALE doc-count {entry}" for entry in stale]
    return unlisted + stales, stales + (unlisted if level == "fail" else [])


def planted_tree():
    """The planted documents and the planted workflow, each read as text, never run."""
    tree = {}
    for path in examined("planted file(s)", sorted(FIXTURES.glob("*.fixture"))):
        tree[path.name.removesuffix(".fixture")] = path.read_text(encoding="utf-8")
    return tree


PLANTED_BOUND = (
    ("bound-match.md", "three jobs", "jobs-running", "planted-ci.yml", "planted-check"),
    (
        "bound-differs.md",
        "four jobs",
        "jobs-running",
        "planted-ci.yml",
        "planted-check",
    ),
)


class TheCountsDocumentsStateAreDerived(unittest.TestCase):
    def test_the_repository_holds_todays_listing(self):
        listed = subprocess.run(
            ["git", "-C", str(REPO), "ls-files", "-z", "--", "*.md", "*.json"],
            capture_output=True,
            check=True,
        ).stdout.decode("utf-8")
        names = [name for name in listed.split("\0") if name]
        markdown = [name for name in names if name.endswith(".md")]
        tree = {name: (REPO / name).read_text(encoding="utf-8") for name in markdown}
        tree.update({name: "" for name in names if name not in tree})
        tree[".github/workflows/ci.yml"] = (REPO / ".github/workflows/ci.yml").read_text("utf-8")
        living = [name for name in markdown if living_text(name, tree[name]) is not None]
        examined("living document(s)", living)
        found, stale, seen = sites(tree, living, BOUND, LISTED)
        examined("bound claim(s)", range(seen))
        printed, failures = verdict(found, stale, LISTED, LEVEL)
        print("\n".join(printed))
        print(f"sites {len(found)}, listed {len(LISTED)}, level {LEVEL}")
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(failures, [])

    def test_a_planted_bound_claim_whose_count_matches_is_clean(self):
        tree = planted_tree()
        found, stale, seen = sites(tree, ["bound-match.md"], PLANTED_BOUND[:1], ())
        self.assertEqual((found, stale, seen), ([], [], 1))
        listing = {"a/x.json": "", "a/y.json": "", "a/z.md": ""}
        self.assertEqual(DERIVATIONS["tracked"](listing, "a/*.json", ""), 2)

    def test_a_planted_bound_claim_whose_count_differs_is_a_site(self):
        tree = planted_tree()
        found, stale, seen = sites(tree, ["bound-differs.md"], PLANTED_BOUND[1:], ())
        self.assertEqual(found, [("bound-differs.md 4 jobs 'four jobs' #1", "mismatch")])
        self.assertEqual((stale, seen), ([], 1))

    def test_a_planted_unbound_claim_is_a_site_at_warn_and_fails_at_fail(self):
        tree = planted_tree()
        found, stale, _seen = sites(tree, ["unbound.md"], (), ())
        site = "unbound.md 5 crates 'five crates' #1"
        self.assertEqual(found, [(site, "unbound")])
        warned = verdict(found, stale, (), "warn")
        self.assertEqual(warned, ([f"WARN doc-count unbound {site}"], []))
        failed = verdict(found, stale, (), "fail")
        self.assertEqual(failed[1], [f"WARN doc-count unbound {site}"])
        self.assertEqual(verdict(found, stale, ((site, "planted"),), "fail"), ([], []))

    def test_a_count_in_code_a_quote_front_matter_a_table_or_a_measured_run_is_not_read(
        self,
    ):
        text = planted_tree()["code-block.md"]
        kinds = collections.Counter(line_kinds(text))
        self.assertEqual(
            (
                kinds["code"],
                kinds["quote"],
                kinds["front"],
                kinds["table"],
                kinds["comment"],
            ),
            (7, 1, 3, 3, 1),
        )
        self.assertEqual(
            [claim.words for claim in claims("code-block.md", text)],
            ["two prose files"],
        )

    def test_a_dated_record_leaves_the_population_by_its_rule(self):
        text = planted_tree()["dated-record.md"]
        signed = "The owner signs the commit that adds this file.\n" + text
        for name, record in (
            ("docs/red-first/SPEC-001.md", text),
            ("docs/specs/SPEC-001-a.md", text),
            ("docs/decisions/ADR-001-a.md", text),
            ("changelog.d/a.md", text),
            ("docs/rulings/a.md", signed),
        ):
            self.assertIsNone(living_text(name, record), name)
        for name, kept in (
            ("docs/specs/planned/SPEC-001-a.md", text),
            ("changelog.d/README.md", text),
            ("docs/rulings/a.md", text),
            ("docs/a.md", signed),
        ):
            self.assertEqual(living_text(name, kept), kept, name)
        released = "# Changelog\n\n## [1.0.0]\n\n" + text
        self.assertEqual(living_text("CHANGELOG.md", released), "# Changelog\n")
        found, _stale, _seen = sites({"docs/a.md": text}, ["docs/a.md"], (), ())
        self.assertEqual(found, [("docs/a.md 3 rows 'three rows' #1", "unbound")])

    def test_an_empty_population_is_void(self):
        with self.assertRaises(AssertionError):
            examined("living document(s)", [])

    def test_a_listing_entry_that_names_no_site_fails_at_both_levels(self):
        tree = planted_tree()
        entry = ("bound-match.md 9 jobs 'nine jobs' #1", "planted")
        found, stale, _seen = sites(tree, ["bound-match.md"], PLANTED_BOUND[:1], (entry,))
        line = f"STALE doc-count listed {entry[0]}"
        for level in ("warn", "fail"):
            self.assertEqual(verdict(found, stale, (entry,), level), ([line], [line]))
        gone = (("bound-match.md", "nine jobs", "tracked", "*.yml", ""),)
        _found, stale, _seen = sites(tree, ["bound-match.md"], gone, ())
        self.assertEqual(stale, ["bound bound-match.md 'nine jobs'"])


if __name__ == "__main__":
    unittest.main()
