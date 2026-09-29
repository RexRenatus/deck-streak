# Red-first record: SPEC-066

The SPEC, in `docs/specs/planned/`, ADR-067, ADR-038's note and the two schematics were committed
alone (9fc4e73). Then came the tests of A1 to A6 (00e0bd0), with a stub: `CredentialError::Empty`
declared with its message, which the loader did not yet return, so every test compiled and each
red failed by assertion. The alert script's refusal (6e68804) turned A5 green, and the loader's
refusal (2d3435c) turned A1, A3 and A6 green. A1, A3 and A5 were read red on 00e0bd0's tree. A6 was
read red at 6e68804, whose `crates/` tree is 00e0bd0's byte for byte: the one commit between them
changes only the alert script. A2 and A4 pin what the base already did, so each is disclosed.
R6 and R3's exit came after, in three commits of their own, R3's restart after those, in three
more, an exit-status reader with R2's condition and R3's pins after those, in four more, and round
4's refusals, which replaced that reader, last; each is recorded below the commands.

```red-first
A1: red at 00e0bd0: "" was not refused as an empty sync-login: Ok(Secret(..)) (a credential of zero bytes loaded as an empty value)
A1: green at 2d3435c
A2: not red: it pins what the refusal must leave as it was, a missing credential refused as Missing, an unreadable one as Unreadable, and a value of one character or more loaded less one trailing newline, all of which the base already did; row S06604 is its killing case, a refusal that reaches past an empty value
A3: red at 00e0bd0: assertion `left == right` failed: left: Ok(()) right: Err(("the credential sync-login is empty in the credentials directory", "Empty { id: \"sync-login\" }"))
A3: green at 2d3435c
A4: not red: the three templates that load a credential beside the alert template already name OnFailure= the alert template and carry no ExecCondition=, no [Unit] condition or assertion, no `-` ExecStart= prefix, no SuccessExitStatus= and no RestartMode=direct, and the alert template already counts no refusal a success, restarts none and is never unloaded while failed (examined 4 service units); no template holds a line the reader refuses, an exit-status word the census does not read, or an empty or unknown Restart=, RestartMode= or CollectMode=; its planted templates are its killing cases, 29 that load a credential, one refused for each condition and each refusal and alert-shaped ones, naming no OnFailure=, that the alert template's checks refuse each for what it breaks, 24 that the reader refuses by file and line, and a cross-check corpus of 22,113 planted exit-status words, of which it admits none
A5: red at 00e0bd0: AssertionError: 0 != 1 : owner-user-id holding '' (the script exited 0: it went on to the journal and the request)
A5: green at 6e68804
A6: red at 00e0bd0: assertion `left == right` failed: anki-sync-username "" left: Ok(()) right: Err(MissingCredentials) (the scripted engine was asked to sync, and the run was recorded as a success)
A6: green at 2d3435c
A7: red at ed12601: AssertionError: {'no restart, no budget': [], 'restart and [208 chars]: []} != {'restart and delay, no start limit': ['dep[1246 chars]: []}
A7: green at 2b778de
```

## The commands

Each criterion's fence line. A1 to A3 were read at c916846, and A4 and A5 at 27499ab, the last
commit that changes a test. A6 builds the sync engine, so it was read at 2d3435c, and its reading
at the delivery's head is CI's `test` job:

| id | command | result |
|---|---|---|
| A1 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_credential_refuses_start_by_its_id` | ok |
| A2 | `cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged` | ok |
| A3 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_refusal_names_the_id_and_never_a_value` | ok |
| A4 | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` | OK, `examined 4 service unit(s) that load a credential`, `examined 22113 cross-check plant(s)`, `examined 24 planted template(s) the reader refuses` |
| A5 | `python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_credential_fails_the_alert_unit_before_any_request` | OK, `examined 4 empty credential case(s)`, then 9, 4, 3 and 7 planted alert templates |
| A6 | `cargo test -p deck-streak-ingest --test retry -- --exact an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine` | ok |

## R6 and R3's exit

Three commits carry them, the SPEC first: 519153e amends SPEC-066 (§1's fourth reader, R3's exit,
R6, the criteria's text and §4), e099e2d adds the tests, and c916846 changes the probe.

- **R6 takes no red.** c916846 moves the engine probe's read of the sync's two credentials onto the
  loader. The probe is an example a person runs by hand, with no test of its own, and the refusal
  it now takes is the loader's, which A1 to A3 hold: each is green at c916846, as at 2d3435c.
  `cargo check -p deck-streak-ingest --example engine_probe` compiles it at c916846 with no warning.
- **R3's exit is disclosed not red.** e099e2d holds the alert template to R3's two exit conditions
  in A4 and in A5, and adds an alert-shaped planted template to A4. Each addition passed when it was
  committed, since the alert template declares no `-` prefix and no `SuccessExitStatus=`. A4 stays
  disclosed not red in the fence; A5 keeps the red and green lines of its route, and this addition
  is disclosed here. The killing cases were measured on a `git archive` export of e099e2d, one
  plant at a time in `deploy/systemd/deck-streak-alert@.service`:

| plant | A4 | A5 |
|---|---|---|
| `SuccessExitStatus=1` after `ExecStart=` | fails: `deck-streak-alert@.service: SuccessExitStatus=1 counts the refusal a success` | fails: `Items in the first set but not the second: '1'` |
| a `-` prefix on `ExecStart=` | fails: `deck-streak-alert@.service: ExecStart=-… counts a failure as a success` | fails: `'-' unexpectedly found in '-'` |

## R3's restart

Three commits carry it, the SPEC first: 7d9e2e3 amends SPEC-066 (R3's third exit condition,
`RestartMode=direct`, and its restart, the criteria's text, §3 and §7), 572bc1d adds the plants and
A5's reads, and 3f6dd77 tells the alert template's conditions apart by name and writes its restart
check.

- **The plants were committed red.** At 572bc1d, A4's alert check still dropped every refusal whose
  text named `OnFailure=`, and the restart check was a stub that refused nothing. A4 failed by
  assertion where it reads its three alert-shaped plants: `alert-restarts.service` came back refused
  for nothing where its `RestartMode=direct skips OnFailure=` line was expected, since that line
  names `OnFailure=` too, and neither it nor `alert-forced.service` had its restart refused. 3f6dd77
  turned A4 green, and it reads green there with its fence command.
- **A4's body changed at 3f6dd77 in two places.** Besides its comments, and its expected refusals
  given a name of their own, the test's own filter, which kept a refusal of the alert template
  unless its text named `OnFailure=`, gave way to the census's `alert_exit_refusals`, which tells
  the conditions apart by the directive each refusal reads; and the `RestartMode=direct` refusal's
  line took its present wording, `skips the failed state and OnFailure=`, for `skips OnFailure=`.
  No expected refusal was removed. The red at 572bc1d depends on neither: the restart check was a
  stub, so `alert-restarts.service` and `alert-forced.service` came back with no restart refused,
  where A4 expects one for each.
- **The additions are disclosed not red.** Each pins what the alert template already declares: no
  `RestartMode=`, no `Restart=` and no `RestartForceExitStatus=`. A4 stays disclosed not red in the
  fence, and A5's three reads passed when they were committed; A5 keeps the red and green lines of
  its route. The killing cases were measured on `git archive` exports of 3f6dd77 and of 916c7dd, the
  head before these commits, one plant at a time after `ExecStart=` in
  `deploy/systemd/deck-streak-alert@.service`, the template restored by its sha256 after each. At
  3f6dd77 the two plants of the table above fail A4 and A5 with the same lines.

| plant | A4 at 3f6dd77 | A5 at 3f6dd77 | 916c7dd |
|---|---|---|---|
| `Restart=on-failure`, `RestartSec=30` and `RestartMode=direct` | fails: `deck-streak-alert@.service: RestartMode=direct skips the failed state and OnFailure=` | fails: `'direct' unexpectedly found in ['direct']` | A4 and A5 pass |
| `RestartMode=direct` alone | fails: the same line | fails: the same | A4 and A5 pass |
| `Restart=on-failure` and `RestartSec=30` | fails: `deck-streak-alert@.service: Restart=on-failure restarts the refusal` | fails: `Lists differ: ['on-failure'] != []` | A4 and A5 pass |
| `RestartForceExitStatus=1` alone | fails: `deck-streak-alert@.service: RestartForceExitStatus=1 restarts the refusal` | fails: `Items in the first set but not the second: '1'` | A4 and A5 pass |

A5 is the killer of rows S06605 to S06608, so each was proved again at 3f6dd77 with `python3
scripts/mutation_rows.py prove --row <id>`: each KILLED, its killer passing without the mutant and
failing with it, and the script restored byte for byte.

## An exit-status reader, R2's condition and R3's pins

Four commits carry them, the SPEC first: abe449f amends SPEC-066 (R2's census refuses an
`ExecCondition=` and reads its exit status in spellings beside `1`, a reading round 4 replaced with
a refusal, below; R3's four exit conditions, its restart and its collection; the criteria's text, §4
and §7) and ADR-067's bullets and references, 667ef04 adds the plants, A4's table of readings and
A5's reads, on stubs, 7e6cf6d writes the reader in `_units.py` and the census's checks, and 02e4d83
strengthens A4's table.

- **The plants were committed red.** At 667ef04 `_units.exit_status` was a stub that read a word
  as its text (the two names, `0` and `1` alone), `status_words` split on whitespace alone, the
  census refused no `ExecCondition=`, and the collection check was a stub that refused nothing.
  Over both whole files only A4 failed, by assertion where it reads its planted templates, which
  lacked four expected lines: `ExecCondition=/bin/true can skip the start, which neither fails the
  unit nor starts OnFailure=` for `condition.service` and for `alert-condition.service`,
  `SuccessExitStatus=01 counts the refusal a success` for `alert-spelled.service`, and
  `SuccessExitStatus=0x1 counts the refusal a success` for `spelled.service`. `test_alert_unit.py`
  passed its 7 tests. 7e6cf6d turned A4 green, and both whole files pass there (11 and 7 tests), as
  they do at 75360a9.
- **The additions are disclosed not red.** Each pins what the templates already declare: no
  paging template names an `ExecCondition=` or a `SuccessExitStatus=`, and the alert template
  names no `ExecCondition=`, `SuccessExitStatus=`, `RestartForceExitStatus=` or `CollectMode=`.
  A4 stays disclosed not red in the fence, and A5's new reads passed when they were committed; A5
  keeps the red and green lines of its route. The killing cases were measured on `git archive`
  exports of 7e6cf6d and of e660152, the head before these commits, one plant at a time after
  `ExecStart=` (`CollectMode=` in `[Unit]`), the template restored by its sha256 after each. Each
  fails A4 at 7e6cf6d with the line below, and each in the alert template fails A5 there too; at
  e660152 each passes A4, A5 and both whole files. An export of 02e4d83 reads as 7e6cf6d's.

| plant | A4 at 7e6cf6d | A5 at 7e6cf6d |
|---|---|---|
| `SuccessExitStatus=01` | fails: `deck-streak-alert@.service: SuccessExitStatus=01 counts the refusal a success` | fails: `Lists differ: ['01'] != []` |
| `SuccessExitStatus=0x1` | fails: the same line, for `0x1` | fails: `Lists differ: ['0x1'] != []` |
| `SuccessExitStatus=+1` | fails: the same line, for `+1` | fails: `Lists differ: ['+1'] != []` |
| `SuccessExitStatus=0b1` | fails: the same line, for `0b1` | fails: `Lists differ: ['0b1'] != []` |
| `SuccessExitStatus=\1` | fails: the same line, for `\1` | fails: `Lists differ: ['1'] != []` |
| `SuccessExitStatus=2` | fails: `deck-streak-alert@.service: SuccessExitStatus=2 is named, and the alert template names none` | fails: `Lists differ: ['2'] != []` |
| `RestartForceExitStatus=01` | fails: `deck-streak-alert@.service: RestartForceExitStatus=01 makes the service manager refuse the Type=oneshot unit outright` | fails: `Lists differ: ['01'] != []` |
| `RestartForceExitStatus=2` | fails: the same line, for `2` | fails: `Lists differ: ['2'] != []` |
| `CollectMode=inactive-or-failed` | fails: `deck-streak-alert@.service: CollectMode=inactive-or-failed can unload the failed instance, which systemctl --failed then no longer lists` | fails: `Lists differ: ['inactive-or-failed'] != []` |
| `ExecCondition=/bin/true` | fails: `deck-streak-alert@.service: ExecCondition=/bin/true can skip the start, which neither fails the unit nor starts OnFailure=` | fails: `Lists differ: ['/bin/true'] != []` |
| `SuccessExitStatus=01` in `deck-streak-bot.service` | fails: `deck-streak-bot.service: SuccessExitStatus=01 counts the refusal a success` | passes: A5 reads the alert template alone |
| `ExecCondition=/bin/true` in `deck-streak-api.service` | fails: `deck-streak-api.service: ExecCondition=/bin/true can skip the start, which neither fails the unit nor starts OnFailure=` | passes: the same |

The six plants of the two tables above still fail A4 and A5 at both exports. At 7e6cf6d
`SuccessExitStatus=1` and `RestartForceExitStatus=1` fail A5 with `Lists differ: ['1'] != []`, and
`RestartForceExitStatus=1` fails A4 with `RestartForceExitStatus=1 makes the service manager
refuse the Type=oneshot unit outright`, R3's reason for it.

- **A4's body changed by additions only.** Besides its comments, 667ef04 added the plants, their
  expected refusals, the collection check and the table of readings, and the expected pair of each
  earlier alert-shaped plant, its exit conditions' refusals and its restart's, became a triple
  whose third list, the collection's, is empty; 02e4d83 added words to the table, and reads the
  status of each word the split gives, where 667ef04 read that of `0 1` alone. No expected refusal
  of an earlier plant was removed or weakened. A5's reads of a literal `1` or `FAILURE` in
  `SuccessExitStatus=` and `RestartForceExitStatus=` became a reading of every word as the reader
  then read it, beside a check that the template names neither directive at all.
- **The reader.** At 02e4d83 `_units.exit_status` modelled how systemd reads an exit-status word,
  held to a table of 922 generated words, and A4 killed each of 25 hand mutants of it. Round 4
  (below) replaced the model with a refusal: the census reads a word only as a decimal of at most
  255 or a status name, and refuses every other word, so the claim is what the census refuses, not
  an agreement with systemd.
- **The rows.** A5 is the killer of rows S06605 to S06608, so each was proved again at 75360a9
  with `python3 scripts/mutation_rows.py prove --row <id>`: each KILLED, its killer selecting one
  test, passing without the mutant and failing with it, and the script restored byte for byte.

## Round 4: the readers refuse what they do not read

Round 4 takes a refusal wherever the census had read a unit file or an exit status by modelling
systemd's reader. dev was merged first, at 98ea4af, with no conflict. 67aa3b9 moves A5's reads of
the alert template into one check. Four rules follow, each in a red commit and a green one, with
the red commit's plants in A4 and, for the alert template, in A5. Both whole files ran at every
red commit, and only the criterion's own test failed: A4 in `test_deploy_templates.py` (11 tests,
1 failure), and A5 in `test_alert_unit.py` (7 tests, 1 failure) where the rule reaches A5. Both
pass at every green commit (11 and 7 tests). A red commit's A4 stops at its first failing
assertion, so its later lists, the cross-check corpus among them, were first read green. e4b4c20
adds three reader plants that hand mutants showed missing, and 27499ab renames three plants and
rewords comments, with no change to what any check refuses. The SPEC, ADR-067 and this record were
amended last, in one commit. No green commit changes a test's body: each changes the checks the
tests call, in `_units.py`, or beside the test in its file.

- **67aa3b9, A5's body.** A5's separate reads of the alert template, no `-` prefix, no
  `ExecCondition=`, no `SuccessExitStatus=`, no `RestartMode=direct`, no `Restart=` other than
  `no`, no `RestartForceExitStatus=` and no `CollectMode=` other than `inactive`, became one check,
  `alert_template_refusals`, which refuses each and also a second `ExecStart=` and an
  `OnFailure=`. A5 asserts it refuses nothing in the template, and refuses nine planted templates,
  each for what it breaks. No read was removed: each became a line of the check. Both whole files
  pass there.
- **Lines: 9872eb3 red, e40af64 green.** The readers split lines at a newline alone, and refuse a
  line that ends in a backslash, a control character other than a tab, whitespace outside ASCII,
  and a line that is neither blank, a comment, a section header nor an assignment in a section.
  At 9872eb3 both readers were round 3's, which refuse no such line; only the `Refused` class the
  tests name was declared. A4 failed where it reads its planted templates the reader refuses, each
  read as if it held nothing wrong (`{..., 'alert-continued.service': None, ...} != {...}`). A5
  failed at its first plant, a comment ending in a backslash before `ExecCondition=/bin/true`,
  refused for its condition where A5 expects the backslash. e40af64 writes one reader in
  `_units.py` (`logical_lines`, `assignments`), and A5's `unit_file` reads through it. A4's plants:
  a continued comment, a continued line after an escaped backslash, an alert-shaped continued line,
  a spaced section header, a key with no `=`, a bare key, a spaced key, a line before any section,
  and fifteen control or non-ASCII whitespace characters in a value, and one more on an
  alert-shaped template; a tab and a `§` in a comment are read. A5's: two continued lines and five
  characters.
- **Restart and collect values: a6c9d96 red, 27b1ef5 green.** `Restart=`, `RestartMode=` and
  `CollectMode=` are read at every assignment, with no reset applied, and a value that is empty or
  not one their manuals list is refused, on every template that loads a credential. At a6c9d96 the
  checks were round 3's. A4 failed where it reads its planted templates, the first missing line
  `alert-reset-collect.service`'s empty `CollectMode=`; A5 failed at its first plant,
  `Restart=on-failure` with `RestartSec=1d` and then an empty `Restart=`, refused for nothing where
  A5 expects the restart and the empty value. 27b1ef5 adds `Unit.every` and the known values. A4's
  plants: `RestartMode=direct` then an empty one, `Restart=On-Failure`, and alert-shaped a
  `Restart=` and a `CollectMode=` each set and then emptied, and `RestartMode=Direct`. A5's: the
  same three resets and `Restart=On-Failure`.
- **Exit-status words: e641821 red, 49c269f green.** A word is read only as a decimal of at most
  255, with no sign and no leading zero, or as a status name `systemd-analyze exit-status` lists,
  in a value split at spaces and tabs, and the census refuses every other word in a
  `SuccessExitStatus=` or a `RestartForceExitStatus=`. At e641821 the reader was round 3's model.
  A4 failed where it reads its planted templates, first at `alert-spelled.service`, whose
  `SuccessExitStatus=01` the model read as 1 where A4 now expects it refused as a word the census
  does not read. A5 passed its 7 tests: it refuses every `SuccessExitStatus=` and
  `RestartForceExitStatus=` on the alert template, so no A5 plant can be red for this rule.
  49c269f writes the refusal and removes the model. A4's plants: a large octal and a large binary
  magnitude after a prefix, on a paging template, the octal again on an alert-shaped one, and
  `RestartForceExitStatus=0x1` on an alert-shaped one; the table of readings, now a decimal, a
  name or none; and the cross-check corpus, 7,371 words, small and large magnitudes in every base
  with prefixes, signs and whitespace around them, each planted as a paging template's
  `SuccessExitStatus=` and as an alert-shaped one's `SuccessExitStatus=` and
  `RestartForceExitStatus=`: 22,113 plants, 0 admitted. Two earlier plants' expected lines change,
  and the red depends on one of them: `spelled.service`'s `0x1` and `alert-spelled.service`'s `01`
  are now refused as words the census does not read, not as the refusal counted a success.
- **`[Unit]` conditions and assertions: d4d81e3 red, f52bb52 green.** Every `Condition*=` and
  `Assert*=` in `[Unit]`, an empty one included, is refused on every template that loads a
  credential. At d4d81e3 the census refused none. A4 failed where it reads its planted templates,
  the first missing line `unit-assert.service`'s assertion; A5 failed at its first plant,
  `ConditionPathExists=/nonexistent`, refused for nothing. f52bb52 adds the refusal. A4's plants:
  a condition, a condition and then an empty one, an assertion, and alert-shaped a condition and an
  assertion. A5's: the same three.
- **Measured on the templates.** Each rule's plants were also planted in the templates themselves,
  one at a time in its section, on full-tree `git archive` exports, the template restored by its
  sha256 after each. Each passes A4 on the export before its rule's green commit and fails A4 on
  the green commit's, and each fails A4 at e4b4c20; each on the alert template fails A5 at e4b4c20.
  On an export before a green commit, a plant on the alert template can fail A5 for a reason the
  rule does not change, another line the plant carries or A5's own planted templates, which copy
  the template, so A4 is the rule's measure there.
- **Hand mutants.** 40 hand mutants of the round's refusals, in `_units.py` (18),
  `test_deploy_templates.py` (14) and `test_alert_unit.py` (8), each run against both whole files
  on an export of e4b4c20 with the file restored by its sha256 after each: 39
  killed, and one equivalent, which strips a section header's name that the header's pattern
  already admits only without spaces. At f52bb52 three survived, a key with no `=`, a key name that
  is not a plain name, and a digit outside ASCII in a decimal; e4b4c20 adds the plants that kill
  each. The checks are test code and take no hand-proved row.
- **The rows.** A5 is the killer of rows S06605 to S06608, so each was proved again with `python3
  scripts/mutation_rows.py prove --row <id>` at f52bb52, whose A5 and alert script are the head's:
  each KILLED, its killer selecting one test, passing without the mutant and failing with it, and
  the script restored byte for byte. No refusal the round adds is in a row's target, and no row is
  added.

## Round 5: the readers hold each unit to a list of keys

Each addition pins what the templates already declare, so each is disclosed not red on the tree,
and each was committed red first, with a stub that compiles: both whole modules ran at each red
commit, and only the tests named failed, by assertion.

| item | red commit | what failed at the red | green commit |
|---|---|---|---|
| a literal list of keys per kind of unit, checked in A4 and A5 | e826281 | `test_a_key_off_its_units_list_is_refused_and_the_trees_units_hold_only_listed_keys` (A4) and `test_a_key_off_the_alert_templates_list_is_refused_by_name` (A5), each on its first plant the stub `off_list` did not flag | a76a59e |
| the credential lines read through the unit reader | 7006ff0 | `test_a_credential_key_with_blanks_before_its_equals_sign_is_read_and_refused` (A4's module), a key with a space or a tab before `=` not read as a credential key | 4162a83 |
| a `*.d/` directory of no unit refused | 997fcf9 | `test_only_a_units_own_dropin_directory_is_shipped_under_deploy`, the stub refusing none | 8000f00 |

The plants of the list: a `Requisite=`, a `Requires=` and a `BindsTo=` on the alert template, a
`Requisite=` on a unit that loads a credential and pages, an `X-` key on each kind, a key of the other section on each kind,
a key of a section no list holds, and a drop-in's `Requires=`. The alert template's existing
plants, which add a line the list also refuses, expect that line beside the refusal they had. A
follow-up commit (3018669) adds the plants that kill the hand mutants below: a section off the
list, an empty credential value, and a drop-in's credential line.

**Hand mutants.** 13 hand mutants of the round's checks, in `_units.py` (3), `test_alert_unit.py`
(2) and `test_deploy_templates.py` (8), each run against the three whole modules, the file restored
by its sha256 after each: 3 survived the first commits (a fallback of an unlisted section to the
service list, the credential lines' empty value, and the `.conf` files' suffix), and 3018669's
plants kill each; one anchor did not occur and was dropped. The checks are test code and take no
hand-proved row, and rows S06605 to S06608, whose killer is A5, were proved again at the head.

## Round 6: the lists are the keys, and a value and a target are bounded

Each addition pins what the templates already declare, so each is disclosed not red on the tree. The
plants of the value table and of the target were committed red first, beside the unchanged
`_units.py`: both whole modules ran at each red commit, and only the tests named failed, by
assertion.

| item | red commit | what failed at the red | green commit |
|---|---|---|---|
| each list equal to the keys its units hold, a key extending a listed key planted, each drop-in directory case the only refusal | not red: the lists are the keys the units hold, and the added cases pin what the check already refuses | none | 4bb2078 |
| a table of the admitted `Restart=` value | 593ee63 | `test_a_paging_units_restart_holds_only_the_admitted_value`: the plants `Restart=always` and `Restart=on-success` on a `Type=oneshot` unit came back with no refusal, the table being empty | 5de252b |
| every `OnFailure=` of a unit that loads a credential and pages is the alert template alone | 593ee63 | `test_a_paging_unit_names_no_failure_target_but_the_alert`: the plants for a target beside the alert's, in its place, after it and reset came back with no refusal, the stub checker refusing none | 5de252b |

The plants of the value table: `Restart=always` and `Restart=on-success` on a `Type=oneshot` unit
that pages, `Restart=always` in a drop-in, `Restart=on-failure-extra` (a value extending the admitted
one, added at 3e176bf), and the admitted control `Restart=on-failure`. The plants of the target: a
second target in one assignment, a second `OnFailure=`, a replaced target, an empty reset after the
alert's, a drop-in's `OnFailure=`, and the admitted control.

**Hand mutants.** 192 hand mutants of the round's checks and of round 5's, in
`_units.py`, `test_alert_unit.py` and `test_deploy_templates.py`, each run against both whole
modules on a worker copy of the head's full-tree export, the file restored by its sha256 after each:
192 killed, 0 survived at 3e176bf. By class: a listed key dropped 88 of 88; an extra key admitted to
either list 70 of 70 (the round-5 head killed 19 of them, the equality assertion kills each);
`off_list` implementations 4 of 4; the drop-in exemption, own-directory match and walk 11 of 11; the
spaced-key read 3 of 3; A4 and A5 wiring 5 of 5; the value table and its check 6 of 6; the target
check 5 of 5. One value-check mutant survived the first plants and 3e176bf's plant kills it.
The box's `service.oneshot-restart` row, which refused the restart plants before, still does.

**The rows.** The value table and the target check are guards no existing row's killer reaches, so
S06609 (`scripts/tests/_units.py`, the table admitting a second restart value) and S06610
(`scripts/tests/test_deploy_templates.py`, the check admitting a target beside the alert's) are
added, each with the round's own test as its killer. Rows S06605 to S06610 were each proved with
`prove --row <id>` on the committed head, KILLED, `PYTHONDONTWRITEBYTECODE=1`, each target restored
by its sha256.

## Round 7: the restart budget and the ordering are bounded

The table gains the keys a crash loop's paging depends on and the keys that order a unit: the start
limit interval (300), the start limit burst (5), the restart delay (15), and `After=` and `Wants=`
(`network-online.target`), each value the one the shipped units hold. It pins what the templates
already declare, so it is disclosed not red on the tree. The plants were committed red first, beside
the unchanged `_units.py` (95acbbb): the three modules ran whole, and only
`test_a_paging_units_restart_holds_only_the_admitted_value` failed, by assertion, the new plants
coming back with no refusal. The table followed at 44b52f0, all three modules green at 16, 8 and 14
tests.

| item | red commit | what failed at the red | green commit |
|---|---|---|---|
| a table of the restart budget and the ordering | 95acbbb | `test_a_paging_units_restart_holds_only_the_admitted_value`: the plants for a start limit interval of 0, a burst of 1000, a restart delay of 0, and an `After=` and a `Wants=` naming another unit came back with no refusal | 44b52f0 |

The plants: `StartLimitIntervalSec=0`, `StartLimitBurst=1000`, `RestartSec=0`,
`After=other.service` and `Wants=other.service` on a planted unit that loads a credential and pages,
and the admitted control holding all five admitted values. Row S06611 (6b48194) drops the interval
entry of the table, killed by the plants, and S06609's anchor moved with the table's layout.

**Hand mutants.** 210 hand mutants of the round's checks and of round 5's and 6's, in `_units.py`,
`test_alert_unit.py` and `test_deploy_templates.py`, each run against both whole modules on a worker
copy of the head's full-tree export, the file restored by its sha256 after each: 210 killed, 0
survived. Beside round 6's 192, the round adds 18: for each of the six bounded keys, the entry
dropped, a second value admitted, and the value check blind to the key.

## Round 9: a unit that restarts holds its whole restart budget

A7's red and green lines are in the fence above. The plants were committed alone (ed12601), with a
helper that refused nothing: a unit that loads a credential and pages, holds `Restart=on-failure`
and `RestartSec=15` and no start limit, and four more lacking one key of the budget at a time,
beside three controls that must not be refused. The three modules ran whole, and only
`test_a_restarting_paging_unit_holds_the_whole_restart_budget` failed, by assertion, the plants
coming back with no refusal; `test_alert_unit` (8) and the rail module (14) passed. The rule
followed at 2b778de (`restart_budget_refusals`, over `_units.RESTART_BUDGET`), with the modules
green at 17, 8 and 14 tests. Every earlier plant is refused for its own reason as before, and the
tree's units hold all three keys. Row S06612 makes the check `if False`, killed by A7's test.

**Hand mutants.** 215 hand mutants, each run against both whole modules on a worker copy of the
head's full-tree export, the file restored by its sha256 after each: 215 killed, 0 survived. Beside
the earlier 210, the round adds 5: the presence check dropped, the restart trigger dropped, and each
of the three keys left out of `RESTART_BUDGET` in turn.

## Mutants of the changed code

**The rows.** `python3 scripts/mutation_rows.py prove --band S06600-S06699` at e0cdf9f, on the
committed tree: `rows: examined 8: killed 8, survived 0, void 0` (`examined 11: killed 11` with rows
S06609 to S06611, at the round-7 head; S06612 proved with `--row`, KILLED). Each row's killer passed without
its mutant and failed with it, and the tool restored each target byte for byte before the next. The
four script mutants were also checked to parse (`sh -n`), since the tool parses only Python. It
read the same at c916846, after A5, the killer of S06605 to S06608, gained its exit check.

| row | target | mutant | killer |
|---|---|---|---|
| S06601 | `crates/kernel/src/credentials.rs` | the refusal's `if value.is_empty()` made `if false` | A1 |
| S06602 | `crates/kernel/src/credentials.rs` | the refusal returns `Missing` in place of `Empty` | A1 |
| S06603 | `crates/kernel/src/error.rs` | the message drops the id | A3 |
| S06604 | `crates/kernel/src/credentials.rs` | the refusal takes a value of one character too | A2 |
| S06605 | `deploy/scripts/alert-telegram.sh` | the empty pattern never matches | A5 |
| S06606 | `deploy/scripts/alert-telegram.sh` | the refusal exits 0 | A5 |
| S06607 | `deploy/scripts/alert-telegram.sh` | the token's check removed | A5 |
| S06608 | `deploy/scripts/alert-telegram.sh` | the owner id's check removed | A5 |
| S06609 | `scripts/tests/_units.py` | the restart entry of the value table admits a second value | A4's value test |
| S06610 | `scripts/tests/test_deploy_templates.py` | the target check admits a target beside the alert's | A4's target test |
| S06611 | `scripts/tests/_units.py` | the value table drops its `StartLimitIntervalSec=` entry | A4's value test |
| S06612 | `scripts/tests/test_deploy_templates.py` | the restart budget's presence check made `if False` | A7 |

**cargo-mutants on the loader's file.** `cargo mutants --package deck-streak-kernel --file
crates/kernel/src/credentials.rs -j 1` (cargo-mutants 27.1.0), on the committed tree:

- at 5e67962: 12 mutants, 9 caught, 1 unviable (`load`'s body as `Ok(Default::default())`, since a
  `Secret` has no default) and 2 missed, neither on a line this delivery changes: `Secret`'s `Debug`
  as `Ok(Default::default())`, which survived an assertion of absence alone, and the `NotFound`
  guard as `true`, which no test reached with an unreadable credential;
- eb0b28a adds what kills both: SPEC-020's test asserts the `Debug` a secret shows,
  `Ok(Secret(..))`, and A2 plants a directory at a credential's path, refused as `Unreadable`;
- at eb0b28a: 12 mutants, 11 caught, 1 unviable (the same body), 0 missed.
- dev's own tests now kill the same two (`a_secret_shows_no_value_in_debug` and
  `a_credential_that_exists_and_cannot_be_read_is_unreadable_and_not_missing`), so this delivery's
  assertions restate them and claim no kill of their own.

cargo-mutants lists no mutant of `crates/kernel/src/error.rs`, and none of the refusal's `if` or
its variant (the same listing at the base shows none of the trim's `if`): rows S06601 to S06604
carry them, and each counts as examined for the changed line its anchor overlaps (SPEC-039 R10).

**The engine probe.** The verdict's `rust` class reads `crates/<crate>/src/` only, so the plan at
c916846 reads the probe as `other` (17 changed paths; `rust` 2, `credentials.rs` and `error.rs`; 8
rows selected), and cargo-mutants mutates no example target: `cargo mutants --list -f
crates/ingest/examples/engine_probe.rs` lists no mutant, where the same listing of
`crates/ingest/src/settings.rs` lists 47. The diff's listing at c916846 holds the one mutant it held
before, `load`'s body, unviable.
