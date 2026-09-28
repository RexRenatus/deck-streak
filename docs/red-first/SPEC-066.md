# Red-first record: SPEC-066

The SPEC, in `docs/specs/planned/`, ADR-067, ADR-038's note and the two schematics were committed
alone (9fc4e73). Then came the tests of A1 to A6 (00e0bd0), with a stub: `CredentialError::Empty`
declared with its message, which the loader did not yet return, so every test compiled and each
red failed by assertion. The alert script's refusal (6e68804) turned A5 green, and the loader's
refusal (2d3435c) turned A1, A3 and A6 green. A1, A3 and A5 were read red on 00e0bd0's tree. A6 was
read red at 6e68804, whose `crates/` tree is 00e0bd0's byte for byte: the one commit between them
changes only the alert script. A2 and A4 pin what the base already did, so each is disclosed.

```red-first
A1: red at 00e0bd0: "" was not refused as an empty sync-login: Ok(Secret(..)) (a credential of zero bytes loaded as an empty value)
A1: green at 2d3435c
A2: not red: it pins what the refusal must leave as it was, a missing credential refused as Missing, an unreadable one as Unreadable, and a value of one character or more loaded less one trailing newline, all of which the base already did; row S06604 is its killing case, a refusal that reaches past an empty value
A3: red at 00e0bd0: assertion `left == right` failed: left: Ok(()) right: Err(("the credential sync-login is empty in the credentials directory", "Empty { id: \"sync-login\" }"))
A3: green at 2d3435c
A4: not red: the three templates that load a credential beside the alert template already name OnFailure= the alert template and carry no `-` ExecStart= prefix, no SuccessExitStatus= naming 1 and no RestartMode=direct (examined 4 service units); its five planted templates are its killing cases, one refused for each condition and one admitted
A5: red at 00e0bd0: AssertionError: 0 != 1 : owner-user-id holding '' (the script exited 0: it went on to the journal and the request)
A5: green at 6e68804
A6: red at 00e0bd0: assertion `left == right` failed: anki-sync-username "" left: Ok(()) right: Err(MissingCredentials) (the scripted engine was asked to sync, and the run was recorded as a success)
A6: green at 2d3435c
```

## The commands

Each criterion's fence line, run at the delivery's head:

| id | command | result |
|---|---|---|
| A1 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_credential_refuses_start_by_its_id` | ok |
| A2 | `cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged` | ok |
| A3 | `cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_refusal_names_the_id_and_never_a_value` | ok |
| A4 | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` | OK, `examined 4 service unit(s) that load a credential` |
| A5 | `python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_credential_fails_the_alert_unit_before_any_request` | OK, `examined 4 empty credential case(s)` |
| A6 | `cargo test -p deck-streak-ingest --test retry -- --exact an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine` | ok |

## Mutants of the changed code

**The rows.** `python3 scripts/mutation_rows.py prove --band S06600-S06699` at e0cdf9f, on the
committed tree: `rows: examined 8: killed 8, survived 0, void 0`. Each row's killer passed without
its mutant and failed with it, and the tool restored each target byte for byte before the next. The
four script mutants were also checked to parse (`sh -n`), since the tool parses only Python.

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
