# Red-first record: SPEC-034

The tests were committed (c79f14e) before the rulesets, the `ci` workflow and the documents changed.
Each criterion was run there for its own reason. A2, A6 and A7 pin behaviour that was already right,
so they are disclosed as not red. The same commit removed SPEC-033 A9's assertion that `main`
requires an up-to-date head, because ADR-034 reverses it; A9's criterion does not name strictness,
and A9 stays green.

```red-first
A1: red at c79f14e: AssertionError: True is not False
A1: green at d1f503f
A2: not red: dev already required an up-to-date head; the test pins it beside A1's reversal for main
A3: red at c79f14e: AssertionError: None != 15368 : main: ci
A3: green at d1f503f
A4: red at c79f14e: AssertionError: 'Nothing is merged back into dev' not found in the release runbook, which still merged main back into dev
A4: green at b848062
A5: red at c79f14e: AssertionError: 0 != 1 : base-is-dev: ok (pull_request into 'main')
A5: green at d1f503f
A6: not red: base-is-dev already admitted this repository's dev into main; the test is A5's positive control
A7: not red: base-is-dev already refused this repository's other branches; the test pins it through the new condition
A8: red at c79f14e: AssertionError: 'never approve a workflow run from a fork' not found in AGENTS.md
A8: green at b848062
A9: not red: the workflows already comply (they read no secret, pass none on, and check out only this repository), so the planted workflows prove the checker red instead (A10, A12)
A10: red at eeaa4e7: AssertionError: Lists differ: [] != ['checkout-of-another-repository.yml:jobs.[1729 chars]lls']; the stub admitted all eight planted workflows
A10: green at 705d506
A11: not red: the stub admitted everything, so the admitted workflows passed at eeaa4e7; the test is A10's positive control
A12: red at eeaa4e7: AssertionError: AssertionError not raised; the stub judged a directory that holds no workflow file
A12: green at 705d506
```

At c79f14e, A5 showed a fork's branch named `dev` passing `base-is-dev` into `main` (exit 0).

The amendment of 2026-09-28 (the SPEC's section 7, #216) committed the planted workflows and A9 to
A12 at eeaa4e7, against a checker stub that read every workflow and refused none, and the checker at
705d506. A hand sweep then deleted or inverted each branch of the checker in a scratch copy, 28
mutants. 26 were killed at once. The two that survived, the two boundaries of the word `secrets`,
are killed by the admitted workflow that 09eca13 and af4c5b7 add to A11, and the checker itself did
not change after 705d506 until the fix round below.

## Fix round, 2026-09-28

The review found that the checker's own workflow reader did not fail closed. The fix round made it
fail closed on quoting, escapes, anchors, aliases and tags, flow forms and keys, read a checkout
named in any case and git's scp-like form, held a `.yaml` workflow to the hardening tests (A13),
and pinned each rule with a planted workflow. `dev` was merged in first, at 9c84e61, with no
conflict. Each step was committed red, then green.

```red-first
A13: red at 403d9ce: AssertionError: AssertionError not raised, in each of three subtests; the hardening tests took .yml files only, so none saw the planted .yaml workflow
A13: green at 1b5f253
```

A10 and A11 are recorded above, so their fix-round runs are listed here, outside the fence:

- A10 red at 6de90e8: `AssertionError: Lists differ`; four planted workflows that hold a secret or a
  run step in quoted, escaped, aliased, tagged or flow forms, or under a key that is not a plain
  name, passed the reader. Green at 2d6731c, where the reader fails closed.
- A10 red at 8041679: `AssertionError: Lists differ`; flow-list items that hold a key, and a matrix
  item keyed by more than a plain name, were read as text. A11 red at 8041679:
  `AssertionError: line 27 was not read`; a key that begins with a dash was taken for a list item.
  Both green at a3f3ae6.
- A10 red at db630d1: `AssertionError: Lists differ`; a checkout named in another case and a fetch
  in git's scp-like form were admitted. Green at ac103f6.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7. Its expected list grew from 16 to 42
findings in four commits: 6de90e8 (13), 8041679 (3), db630d1 (7) and b0e917c (3). The first three
were committed red, as listed above. b0e917c's three pin reader rules that a hand mutant showed no
test held; the checker already met them, so they were green at once, and each is red against its
mutant. No finding that was in the list at eeaa4e7 was changed or removed. A9, A11, A12 and the
three hardening tests keep their bodies; their setUp takes its files from `workflow_files`.

At 7f9e423 a hand sweep ran 62 mutants over the checker and its reader, one at a time, on a scratch
copy of the committed tree, restoring the file byte for byte after each: the 28 of the first sweep,
re-expressed against the fix round's code, 11 from the review (V1, V3, V6 to V9, X06, and X12 in
four forms) and 23 of the reader's branches. 62 were killed and none survived. The first run, at
ac103f6, left five survivors: b0e917c's planted workflows kill four, and 7f9e423 removed the fifth's
branch, which read a bare dash as an empty item and which no test could tell apart.

## Fix round 2, 2026-09-28

The second review found forms the checker still admitted: white space and controls that YAML, or
GitHub's parser, reads as text, a checkout's input named in another case, a checkout from another
server, and actions/checkout written as the runner reads it apart. The checker failed on an empty
block and on a step that is not a mapping, and three hand mutants survived. `dev` had not moved
since 07322ae merged it. SPEC-034's inserted text was corrected first, at 0bec186. No criterion was
added, so the fences above are unchanged, and this round's runs are listed here. Each fix was
committed red, then green.

- A10 red at 34e669a: 48 of its subtests failed. 36 read `AssertionError: AssertionError not
  raised`: a planted line the reader cannot place was read. Four read `AssertionError: Lists
  differ` and eight `line 15 is not a mapping entry`: no line holding one of twelve characters
  outside printable ASCII was refused by its line, and for eight of them the reader split the line
  at the character. Green at 6ddbfb9.
- A10 red at d068a61: `AttributeError: 'str' object has no attribute 'get'`: the checker judged a
  step the reader had refused. A11 red at d068a61: `ValueError: min() iterable argument is empty`:
  the reader failed on an empty block. Both green at 36f9402.
- A10 red at e3b6f7a: `AssertionError: Lists differ`: two checkouts of another repository, their
  input named in another case, were admitted. Green at ce3cf5f.
- A10 red at 9e0828f: `AssertionError: Lists differ`: two checkouts from another server were
  admitted. Green at 18e3eac.
- A10 red at 9ee09d1: `AssertionError: Lists differ`: four checkouts of another repository, their
  action written as the runner reads actions/checkout, were admitted. A13 red at 9ee09d1:
  `AssertionError: AssertionError not raised`, in two of its three new subtests: the SHA-pin test
  admitted an action written with an empty part or a trailing slash. Both green at 5c12427.
- 94edf64 kills three hand mutants and was green at once, because the checker already met each.
  A13 runs each hardening test through its own setUp: Y0, whose setUp read `.yml` files only,
  survived until then. A10 plants a block the reader does not read before a clone, and a plain value
  over two lines (Y1, the final unplaced-line check deleted), and a secret after a `#` that no space
  precedes (Y5, a bare `#` read as a comment). 34e669a's planted characters already killed Y1 and
  Y5; with those subtests switched off, 94edf64's plants kill each.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7, again in this round. Its committed
list grew from 42 to 53 findings: d068a61 (2), e3b6f7a (2), 9e0828f (2), 94edf64 (1) and 9ee09d1
(4). 34e669a added its planted characters and lines (48 subtests), and 94edf64 its two unplaced
forms. No finding that was in the list before this round changed or left. A11's body did not
change; its admitted workflows gained an empty block (d068a61) and two checkouts from this server
(9e0828f). A13's body changed after its red commit, 403d9ce: at 94edf64 its three subtests run
through their own setUp rather than being handed their files, and at 9ee09d1 it gained three
subtests for the SHA-pin test's forms. Its criterion now also says that an action is pinned only in
its plain form.

At 5c12427 a hand sweep ran 102 mutants over the checker, its reader and the SHA-pin pattern, one at
a time, on a scratch copy of the committed tree, restoring the file byte for byte after each: the 62
of the first fix round, re-expressed against this round's code, the second review's six (Y0 to Y3,
Y5 and Y6), and 34 of this round's: the character check and the line ends, each white-space site,
the empty block, the job, step and input guards, the reading of `uses`, inputs in any case, the
server, the SHA-pin pattern, and the quoted and flow-list dispatches. 100 were killed there. W12
and W13, which read a block's trailing lines and its indent with white space other than a space or
a tab, survived, and fix round 3 kills both (below). Y2, which the second review recorded as
equivalent, is killed: each planted line asserts the reason its refusal gives.

Rider, 2026-09-28: the hardening tests read keys the way the checker does. `dev` had not moved
since 07322ae merged it. No criterion was added, so the fences above are unchanged.

- A13 red at 216aa16: `AssertionError: AssertionError not raised`, in nine of its ten new
  subtests: the hardening tests did not yet read keys the way the checker does. Green at 3c3acaa,
  where each reads a workflow through the checker's reader and fails closed on a form it does not
  read. The tenth, a trigger, was green at once: it pins the trigger check, whose deletion no test
  killed before, and it is red against that mutant.
- On the four live workflows the reader finds what the earlier reading found: the same 70 action
  references and 19 runners, in the same order, and each default token as `contents: read`.

DISCLOSURE: A13's body changed after its red commit, 403d9ce, again: at 216aa16 it gained ten
subtests that plant keys in the control and judge it beside the live workflows. Its criterion, and
R7, now also say that the hardening tests read keys the way the checker does: SPEC-034's inserted
text says so at dd3ba90, after the green commit, because it states how the tests read.

At 3c3acaa a hand sweep ran 24 mutants over the hardening tests and the two functions they read
through, one at a time, on a scratch copy of the committed tree, restoring the file byte for byte
after each: each new refusal and condition deleted or inverted, each test given back its earlier
reading, and each test made to skip a workflow the reader refuses. 23 were killed, each by A13, and
nine of them by a hardening test on the live workflows as well: the five inversions and the walk's
four. The 24th, F5, which stops the walk under a matching key, survived, and fix round 3 kills it
(below).

## Fix round 3, 2026-09-28

Fix round 3 judges every step inside a `parallel` block and refuses a checkout whose inputs are not
a mapping. It also kills the three hand mutants that survived above and pins two more. `dev` had not
moved since 07322ae merged it. SPEC-034's inserted text was corrected first, at 6533809, and its
note of A10's block plants was made exact at c31ae47, after the plants. No criterion was added, so
the fences above are unchanged, and this round's runs are listed here. Each fix was committed red,
then green.

- A10 red at 0b7f5b6: `AssertionError: Lists differ`: the checker walked a job's steps only, so
  three steps planted inside a `parallel` block, one of them two blocks deep, were not judged.
  Green at aef8139, where the checker takes every step from `steps_in`, which descends into a
  `parallel` step's list at any depth.
- A10 red at 1a125fd: `AssertionError: Lists differ`: three checkouts whose inputs are one `${{ }}`
  expression were read as having no inputs. Green at 5a6eccd, where the checker refuses a checkout
  whose `with` is set and is not a mapping. A11 was green at both: its checkout workflow gained a
  checkout whose `with` is empty, which is still no inputs.
- f7221b3 kills W12, W13 and F5 and was green at once, because the reader and the walk already met
  each. A10 plants, at test time, a `|` block that holds a line of each character the reader
  refuses, at the block's end and inside it, beside a secret the block names over two lines: the
  finding names the secret with the indent that line sets, which W12 and W13 change. A13 reads a
  `uses` nested in a `uses` through `entries`, whose second value F5 drops.
- 2d7cb8f pins two more hand mutants and was green at once. A10's clone workflow gains a step whose
  empty `env` comes first, which a reader that took a child block at its key's indent would admit.
  A13 refuses the control's action with its SHA cut short, which a pattern that took a short SHA
  would admit. Each is red against that mutant.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7, again in this round. Its committed
list grew from 53 to 60 findings: 0b7f5b6 (3), 1a125fd (3) and 2d7cb8f (1). f7221b3 added its
block plants (24 subtests). No finding that was in the list before this round changed or left.
A11's body did not change; its admitted workflows gained a checkout whose `with` is empty
(1a125fd). A13's body changed after its red commit, 403d9ce, again: at f7221b3 it reads a nested
`uses` through `entries`, and at 2d7cb8f its SHA-pin forms are written as whole references and gain
a short SHA.

At 2d7cb8f the hand sweeps ran again, one mutant at a time, on a scratch copy of the committed tree,
restoring the file byte for byte after each. The 102 of fix round 2: 102 of 102 killed, W12 and W13
by A10. The rider's 24: 24 of 24 killed, F5 by A13. And 11 of this round's, the walk into
`parallel` blocks and the refusal of inputs that are not a mapping, each deleted, inverted or given
back its earlier reading: 11 of 11 killed. The test file did not change after 2d7cb8f.

## Fix round 4, 2026-09-28

Fix round 4 refuses a `shell` that is not one of GitHub's built-in keywords, reads every string of
a workflow for a clone or a fetch, and refuses git configured from the environment: a key or a
string that names a variable whose name begins with `GIT_`. `dev` was
merged first, at 38e6e6d, with no conflict. No criterion was added, so the fences above are
unchanged, and this round's runs are listed here. Each refusal was committed red, then green.
SPEC-034's inserted text says so at 9fa2e40, after the green commits, because it states what the
checker reads (R7, section 3, and the manifest's rows of planted workflows).

- A10 red at 237176d: `AssertionError: Lists differ`: six planted shells were not yet refused: a
  custom shell that clones another repository in the workflow's `defaults.run`, a job's, a step's
  and a step's inside a `parallel` block, a keyword in another case, and a job's `defaults.run`
  that is one `${{ }}` expression. Green at a1403a5, where the checker refuses a shell that is not
  one of GitHub's built-in keywords, as written, and defaults it cannot read. A11 was green at
  both: its admitted workflows gained each keyword at each of those places.
- A10 red at 5447e5b: `AssertionError: Lists differ`: fourteen commands outside a run step's
  script were not yet read: `BASH_ENV` in the workflow's, a job's and two steps' `env`, the four
  custom shells' clones, three step names, and three values under keys the reader refuses. Each of
  the twelve planted-character subtests expects the value under its refused key as well; they
  follow the list's assertion, which stops the test at red, and each is red at 5447e5b run alone.
  Green at 89f86ec, where the checker reads every string the workflow holds for those commands.
- A10 red at 0994866: `AssertionError: Lists differ`: fourteen findings were missing: a variable whose name begins with `GIT_`, set as an
  `env` key of the workflow, a job, a job's container and a step, in any
  case, or named in a container's options and in a script (eight); an `env` of the workflow, a
  job, a job's container and a step, and a container, each one `${{ }}` expression (five); and an
  `env` the reader refuses as a flow mapping (one). Green at e7fb4f5. A11 was green at both: its
  admitted workflows gained names that hold the letters `GIT_` only inside a longer word, an empty
  `env`, a container named by its image alone, which has no `env` to read, and a container whose
  `env` is a mapping.
- 8274f9f pins four hand mutants and was green at once, because the checker already met each: a
  job's `defaults` that is one expression (S7), an empty `defaults.run` and an empty `shell` (S8
  and S4), and a script that names one variable twice (E7). Each is red against its mutant.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7, again in this round. Its committed
list grew from 60 to 95 findings: 237176d (6), 5447e5b (14), 0994866 (14) and 8274f9f (1). It
gained a local that holds the planted custom shell, and each of its twelve planted-character
subtests gained one finding at 5447e5b. No finding that was in the list before this round changed
or left: each new one follows its file's earlier ones, in the order the checker reads. A11's body
did not change; its admitted workflows grew from six to eight, `built-in-shells.yml` (237176d, its
empty forms at 8274f9f) and `environment-the-checker-reads.yml` (0994866). On the live workflows
the checker reads 636 strings, 570 of them outside a run step's script: none names a clone, a
fetch of a URL or a variable whose name begins with `GIT_`, and none of the four workflows sets a `shell`, `defaults`,
a container or an `env` that is not a mapping.

At 8274f9f a hand sweep ran 41 mutants over this round's code, one at a time, on a scratch copy of
the committed tree, restoring the file byte for byte after each: each built-in keyword dropped
(six), the shell and defaults checks deleted or inverted (twelve), the reading of every string
(four), and the variable pattern, the walk of keys and strings and the environment and container
checks (nineteen). 41 of 41 were killed. At e7fb4f5, before the pins, the same 41 left S4, S7, S8
and E7 alive.

## Fix round 5, 2026-09-28

Fix round 5 refuses a job's container whose `options`, `image`, `ports` or `volumes` is or holds a
`${{ }}` expression, which GitHub evaluates when the job runs. `dev` was merged first, at aac4e70,
with no conflict. No criterion was added, so the fences above are unchanged, and this round's runs
are listed here. Each refusal was committed red, then green. SPEC-034's inserted text says so at
b38cc14, after the green commits and the pin, because it states what the checker reads (R7 and
section 3).

Every property of a job's container in GitHub's workflow schema was read first, with the runner's
use of each, and each has its verdict:

| property | verdict |
|---|---|
| `image` | refused when it is or holds a `${{ }}` expression |
| `options` | refused when it is or holds a `${{ }}` expression |
| `ports` | refused when it is or holds a `${{ }}` expression |
| `volumes` | refused when it is or holds a `${{ }}` expression |
| `env` | refused when it is not a mapping or names a variable whose name begins with `GIT_` (fix round 4) |
| `credentials` | not read by steps |

- A10 red at ec13211: `AssertionError: Lists differ`: two planted containers' `options`, one
  `${{ }}` expression and a literal option beside one, were not yet refused. Green at a27e789,
  where the checker refuses a container's `options` that is or holds a `${{ }}` expression. A11 was
  green at both: its admitted workflows gained a container whose options are literal, `--cpus 1`.
- A10 red at 6a31256: `AssertionError: Lists differ`: six planted containers' `image`, `ports` and
  `volumes`, each one `${{ }}` expression and each holding one beside a literal value, were not yet
  refused. Green at 1ae727b, where the checker refuses each of a container's `image`, `options`,
  `ports` and `volumes` that is or holds one. A11 was green at both: its admitted workflows gained a
  container with literal `ports` and `volumes` and registry `credentials` given by an expression.
- 9f19b06 pins two hand mutants and was green at once, because the checker already met each: a
  container given every property by an expression, which a refusal that stopped at the first
  property it found (C14) or at the `env`'s finding (C19) would name only in part. Each is red
  against its mutant.

In each red run the whole test file ran and A10 alone failed.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7, again in this round. Its committed
list grew from 95 to 108 findings: ec13211 (2), 6a31256 (6) and 9f19b06 (5). No finding that was in
the list before this round changed or left: each new one follows its file's earlier ones, in the
order the checker reads. A11's body did not change; its admitted workflows are still eight, and
`environment-the-checker-reads.yml` gained two containers (ec13211 and 6a31256). The live workflows
set no job's container. On them the checker reads 668 strings, 599 of them outside a run step's
script, and none names a clone, a fetch of a URL or a variable whose name begins with `GIT_`.

At 9f19b06 the hand sweeps ran again, one mutant at a time, on a scratch copy of the committed
tree, restoring the file byte for byte after each. The 102 of fix round 2: 102 of 102 killed. The
rider's 24: 24 of 24. Fix round 3's 11: 11 of 11. Fix round 4's 41: 41 of 41, with E13, the
container's `env` refusal dropped, anchored on the line this round rewrote. And 19 over this
round's code: each property dropped from the refusal, and all four, `credentials` or `env` added to
it, its condition deleted, inverted, widened or narrowed, a list read by its first item or not
read, the walk cut short three ways, the finding's place changed, and the `env` refusal dropped or
made the only one: 19 of 19 killed. At 1ae727b, before the pin, the same 19 left C14 and C19 alive.
The test file did not change after 9f19b06.

## Second amendment (section 8): A14 and A15

The tests were committed (58574a9) before the workflow changed. In that red run the whole test file
ran and only A14 and A15 failed.

```red-first
A14: red at 58574a9: AssertionError: None != "${{ github.event_name == 'pull_request' && 'ci' || 'ci (push)' }}" : the aggregate job's name
A14: green at 3de33c5
A15: red at 58574a9: AssertionError: 'ci' unexpectedly found in {'ci', 'fragment'} : ci.yml: push reports ci
A15: green at 3de33c5
```

Rows S03401 to S03403 (`scripts/mutation-rows.d/S03400-S03499.json`) were proved with
`python3 scripts/mutation_rows.py prove --band S03400-S03499`, the file restored by its digest:
3 examined, 3 killed, 0 survived, 0 VOID. Each row's killer is A14. `census` and `ids` are clean.

DISCLOSURE: A14's and A15's bodies changed after their red commit (58574a9), in three commits after the green commit, each red against 58574a9's workflow. 39f81ac added the positive assertion that the push name `ci (push)` is judged. d24c0b6 made both arms of the evaluated form non-empty, added the empty-arm form to A14's refusals, and read a one-line `on:` trigger in either form, so A15 judges every job's name under every event that is not `pull_request`. Under each, A14 and A15 fail at 58574a9's workflow and pass at the head. The fix round after them added the second-arm empty form (`'ci' || ''`) to A14's refusals, which kills the mutant that lets the second arm be empty, and factored A15's events reading into `judged_events` with its own self-check over the scalar, list and mapping forms and a `pull_request`-only trigger, which kills the mutant that reverts the scalar branch.

The third amendment (section 9 of the SPEC, issue #360) adds A16.

```red-first
A16: red at 71ad257: AssertionError: 'Release model: no-back-merge (ADR-034)' not found in the lines of RELEASING.md
A16: green at 5c5dc26
```

DISCLOSURE: A16's body changed after its red commit (71ad257). It now reads the declaration as the only line of the runbook that begins `Release model:`, and as the first non-blank line under the heading of section 5. It fails at 71ad257's runbook by the assertion quoted above, and at dev's runbook by `Lists differ: [] != ['Release model: no-back-merge (ADR-034)']`; it passes at the head.
