# Schematic: one box-pack run, from a commit to one line per pack

Kind: data flow, with the verdict each pack reaches. Read at DeckStreak `dev` cb66427
(`scripts/box-packs.sh`, `.packs/wiring.json`, `.packs/VENDORED.json`, ADR-004), and at the
vendored packs commit for the verbs it drives (`<binary> pack list`, `<binary> pack probe`,
`<binary> pack run`, `<binary> verify seo-pipeline`, `<binary> init`, `<binary> project register`, and
`scripts/proxy-client-scan.py`). Decided by ADR-030; built by SPEC-030. The issue-state step was
added by SPEC-054 R4, read at `dev` c0dbf2a with `gh issue view <n> --json state`.

```mermaid
flowchart LR
  rev[the commit at --rev] -->|git archive| export[(scratch: the exported tree)]
  export -->|reads| wiring[.packs/wiring.json: box]
  export -->|reads| vendored[.packs/VENDORED.json]
  vendored -->|vendored_from equals the checkout's HEAD| pin{pinned?}
  pin -->|no: exit 2| stop([refused])
  export -->|removes .packs/ and every path VENDORED.json lists| judged[(the judged tree)]
  wiring -->|every issue it names, once each| gh[gh issue view N --json state, run in ROOT]
  gh -->|no gh, not logged in, offline, or no state: exit 2| void([VOID: no pack runs])
  gh -->|OPEN or CLOSED| states[each issue's state]
  catalog[(packs checkout skills/catalog.json)] -->|binary pack list| verbs[each pack's schema]
  wiring -->|the packs to run| verbs
  verbs -->|binary.pack.probe.v1| probe[binary pack probe --skills-root]
  verbs -->|binary.pack.run.v1| run[binary --ledger scratch pack run --project]
  verbs -->|binary.seo-pipeline.v1, once the site is built| seo[binary verify seo-pipeline]
  ledger[(scratch ledger: binary init, binary project register)] --> run
  judged --> probe
  judged --> run
  judged --> seo
  judged --> scan[proxy-client-scan.py check all]
  probe --> cards[(one card per pack)]
  run --> cards
  seo --> cards
  scan --> rows[its row lines and summary]
  cards --> judge{each row against the wiring}
  rows --> judge
  wiring -->|expected_red, pending| judge
  states -->|an expectation whose issue is CLOSED is stale| judge
  judge --> lines[one line per pack; exit 0, 1 or 2]
```

The scratch directory holds the exported tree and the ledger. It is created with the process's
temporary directory, outside both repositories (the run refuses a scratch directory inside either),
and removed when the run ends, whatever its verdict. The cards are kept under `$BOX_PACKS_OUT`, to
be posted on the pull request.

## The verdict of one pack

| the card says | the wiring says | the pack reads | fails the run |
|---|---|---|---|
| no card: the binary refused, or the card's schema is not the verb's | anything | `FAIL` with the binary's last line | yes |
| examined 0 (a declarative walk, or no built site) | `pending #N` | `pending #N` | no |
| examined 0 | nothing | `FAIL` VOID | yes |
| examined 1 or more | `pending #N` | `FAIL`, the pending issue stale | yes |
| a blocking row red | the row under `expected_red` | counted expected | no |
| a blocking row red | not named | `FAIL`, an unexpected red by name | yes |
| a row named under `expected_red` is not red | named | `FAIL`, a stale expectation by name | yes |
| an advisory row failed | anything | counted as neither | no |
| anything | a row under `expected_red`, or `pending`, whose issue is CLOSED | `FAIL`, the expectation stale by row and issue (`<row> (#N is closed)`) | yes |

The proxy scan is judged by its row lines, never by its exit, because `check all` exits VOID over
RED. Any RED line fails the run. With no RED line, it reads `pending #N` while it examines no
settings document and the wiring names that issue; a pending issue it has outgrown (a settings
document examined) is stale; and a blocking VOID row with a settings document present is VOID.

| step | what crosses | guard |
|---|---|---|
| repository to scratch | the committed tree at `--rev`, never the working copy | `git archive` of a resolved commit |
| scratch to judged tree | DeckStreak's own files | `.packs/` and every `VENDORED.json` path removed before any pack runs |
| wiring to GitHub | each issue the `box` section names, once, and its state | read with `gh` in ROOT before any pack runs; an issue `gh` cannot answer for (no `gh`, not logged in, offline, no state) makes the run VOID, exit 2 |
| catalog to verb | the schema a pack's row declares | a schema no verb admits, or a pack the catalog lacks, fails that pack by name |
| card to verdict | each row's id and state | the wiring's expectations; a card that is not the verb's schema is no card |
| run to pull request | one line per pack and a summary | exit 0 only when every pack examined a row or reads pending, and none failed |

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).

## The cards directory's lifetime (SPEC-476, ADR-493)

Kind: data flow, read at DeckStreak `dev` 36c4783a (`scripts/box-packs.sh`,
`scripts/tests/test_box_packs.py`). Since #531 the cards are kept under `$BOX_PACKS_OUT` only when
that variable is set and not empty. Otherwise a run writes them to a directory it makes itself, and
removes that directory when it ends unless `BOX_PACKS_KEEP` is set and not empty. For that case this
section qualifies the sentence on the cards above.

```mermaid
flowchart TD
  reads["read BOX_PACKS_OUT and BOX_PACKS_KEEP into made and keep"] --> chosen{"BOX_PACKS_OUT set and not empty?"}
  chosen -->|yes| given[("the caller's directory, made if missing, never removed")]
  chosen -->|no, unset or empty| fresh[("deckstreak-box-cards.RANDOM, made by mkdtemp in TMPDIR")]
  given --> work["try: issue states, the exported tree, every pack, probe, scan and owned file, each card written under out"]
  fresh --> work
  work -->|returns| fin{"finally: made and not keep?"}
  work -->|a refusal or any other exception| fin
  work -->|SIGTERM, SIGHUP or SIGINT| fin
  fin -->|yes| removal["shutil.rmtree of exactly out"]
  fin -->|no| named["stderr: cards: out"]
  parent[("TMPDIR, the parent: survives")] --- fresh
  decoy[("deckstreak-box-cards.decoy, a sibling: survives byte for byte")] --- parent
```

Both readings come before the make, so the make is the last statement before the `try`: at dev
`36c4783a` the two readings sat between them (`scripts/box-packs.sh:243-248`), where a signal would
have left the made directory behind. The removal names exactly the path the make returned, never its
parent and never a pattern, so the parent and every sibling in it survive. A signal reaches the
`finally` because the runner turns SIGTERM and SIGHUP into `sys.exit` and SIGINT raises
`KeyboardInterrupt`; no test sends one (SPEC-476 section 6).

| step | the test that holds it | the planted copy it reads red |
|---|---|---|
| both readings precede the make, which precedes the `try` | `test_the_cards_directory_is_made_just_before_its_try` (A6) | S47607 |
| an empty `BOX_PACKS_OUT` reads as unset | `test_an_empty_out_is_read_as_unset` (A2) | S47603, S47604 |
| `BOX_PACKS_KEEP=0` keeps the made directory | `test_a_keep_of_0_keeps_the_made_directory` (A3) | S47605 |
| an empty `BOX_PACKS_KEEP` keeps nothing | `test_an_empty_keep_removes_the_made_directory` (A4) | S47606 |
| the removal spares the parent and a sibling, on every end | `test_a_decoy_sibling_and_the_parent_survive_every_end` (A1) | S47601, S47602 |
| the made directory is removed on a pass, a failure and an error | `test_the_created_cards_directory_is_gone_after_every_end` | S12313 |
| a given directory survives, and `BOX_PACKS_KEEP=1` keeps and names the made one | `test_a_given_or_kept_cards_directory_survives_the_run` | none |
| every line of the runner fits 100 columns | `test_every_runner_line_fits_100_columns` (A5) | S47608, S47609, S47610 |

| step | what crosses | guard |
|---|---|---|
| test to runner | a synthetic repository, a fake runner, a fake issue-state reader and a private file, all in the test's own temporary directory | `TMPDIR` and the working directory are set inside that directory, so a copy that removes the parent or resolves an empty path reaches nothing outside the test |
| make to removal | the one path `cards_directory` returned | the `finally` removes that path alone, and only when the run made it and nothing keeps it |
| removal to the temporary directory | nothing but the made directory's removal | the decoy sibling's bytes and the parent are read after every end |
