# Red-first record: SPEC-066

The SPEC, in `docs/specs/planned/`, ADR-067, ADR-038's note and the two schematics were committed
alone (9fc4e73). Then came the tests of A1 to A6 (00e0bd0), with a stub: `CredentialError::Empty`
declared with its message, which the loader did not yet return, so every test compiled and each
red failed by assertion. The alert script's refusal (6e68804) turned A5 green, and the loader's
refusal (2d3435c) turned A1, A3 and A6 green. A1, A3 and A5 were read red on 00e0bd0's tree. A6 was
read red at 6e68804, whose `crates/` tree is 00e0bd0's byte for byte: the one commit between them
changes only the alert script. A2 and A4 pin what the base already did, so each is disclosed.
R6 and R3's exit came after, in three commits of their own, recorded below the commands.

```red-first
A1: red at 00e0bd0: "" was not refused as an empty sync-login: Ok(Secret(..)) (a credential of zero bytes loaded as an empty value)
A1: green at 2d3435c
A2: not red: it pins what the refusal must leave as it was, a missing credential refused as Missing, an unreadable one as Unreadable, and a value of one character or more loaded less one trailing newline, all of which the base already did; row S06604 is its killing case, a refusal that reaches past an empty value
A3: red at 00e0bd0: assertion `left == right` failed: left: Ok(()) right: Err(("the credential sync-login is empty in the credentials directory", "Empty { id: \"sync-login\" }"))
A3: green at 2d3435c
A4: not red: the three templates that load a credential beside the alert template already name OnFailure= the alert template and carry no `-` ExecStart= prefix, no SuccessExitStatus= naming 1 and no RestartMode=direct, and the alert template already counts no refusal a success (examined 4 service units); its six planted templates are its killing cases, one refused for each condition, one admitted, and one alert-shaped, naming no OnFailure=, that the alert template's check refuses for its SuccessExitStatus=1 alone
A5: red at 00e0bd0: AssertionError: 0 != 1 : owner-user-id holding '' (the script exited 0: it went on to the journal and the request)
A5: green at 6e68804
A6: red at 00e0bd0: assertion `left == right` failed: anki-sync-username "" left: Ok(()) right: Err(MissingCredentials) (the scripted engine was asked to sync, and the run was recorded as a success)
A6: green at 2d3435c
```

## The commands

Each criterion's fence line. A1 to A5 were read at c916846. A6 builds the sync engine, so it was
read at 2d3435c, and its reading at the delivery's head is CI's `test` job:

| id | command | result |
|---|---|---|
| A1 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_credential_refuses_start_by_its_id` | ok |
| A2 | `cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged` | ok |
| A3 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_refusal_names_the_id_and_never_a_value` | ok |
| A4 | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` | OK, `examined 4 service unit(s) that load a credential` |
| A5 | `python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_credential_fails_the_alert_unit_before_any_request` | OK, `examined 4 empty credential case(s)` |
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

## Mutants of the changed code

**The rows.** `python3 scripts/mutation_rows.py prove --band S06600-S06699` at e0cdf9f, on the
committed tree: `rows: examined 8: killed 8, survived 0, void 0`. Each row's killer passed without
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

**cargo-mutants on the loader's file.** `cargo mutants --package deck-streak-kernel --file
crates/kernel/src/credentials.rs -j 1` (cargo-mutants 27.1.0), on the committed tree:

- at 5e67962: 12 mutants, 9 caught, 1 unviable (`load`'s body as `Ok(Default::default())`, since a
  `Secret` has no default) and 2 missed, neither on a line this delivery changes: `Secret`'s `Debug`
  as `Ok(Default::default())`, which survived an assertion of absence alone, and the `NotFound`
  guard as `true`, which no test reached with an unreadable credential;
- eb0b28a adds what kills both: SPEC-020's test asserts the `Debug` a secret shows,
  `Ok(Secret(..))`, and A2 plants a directory at a credential's path, refused as `Unreadable`;
- at eb0b28a: 12 mutants, 11 caught, 1 unviable (the same body), 0 missed.

cargo-mutants lists no mutant of `crates/kernel/src/error.rs`, and none of the refusal's `if` or
its variant (the same listing at the base shows none of the trim's `if`): rows S06601 to S06604
carry them, and each counts as examined for the changed line its anchor overlaps (SPEC-039 R10).

**The engine probe.** The verdict's `rust` class reads `crates/<crate>/src/` only, so the plan at
c916846 reads the probe as `other` (17 changed paths; `rust` 2, `credentials.rs` and `error.rs`; 8
rows selected), and cargo-mutants mutates no example target: `cargo mutants --list -f
crates/ingest/examples/engine_probe.rs` lists no mutant, where the same listing of
`crates/ingest/src/settings.rs` lists 47. The diff's listing at c916846 holds the one mutant it held
before, `load`'s body, unviable.
