# Schematic: one box-pack run, from a commit to one line per pack

Kind: data flow, with the verdict each pack reaches. Read at DeckStreak `dev` cb66427
(`scripts/box-packs.sh`, `.packs/wiring.json`, `.packs/VENDORED.json`, ADR-004), and at the
vendored phoenix-v2 commit for the verbs it drives (`phxd pack list`, `phxd pack probe`,
`phxd pack run`, `phxd verify seo-pipeline`, `phxd init`, `phxd project register`, and
`scripts/proxy-client-scan.py`). Decided by ADR-030; built by SPEC-030.

```mermaid
flowchart LR
  rev[the commit at --rev] -->|git archive| export[(scratch: the exported tree)]
  export -->|reads| wiring[.packs/wiring.json: box]
  export -->|reads| vendored[.packs/VENDORED.json]
  vendored -->|vendored_from equals the checkout's HEAD| pin{pinned?}
  pin -->|no: exit 2| stop([refused])
  export -->|removes .packs/ and every path VENDORED.json lists| judged[(the judged tree)]
  catalog[(phoenix-v2 skills/catalog.json)] -->|phxd pack list| verbs[each pack's schema]
  wiring -->|the packs to run| verbs
  verbs -->|phxd.pack.probe.v1| probe[phxd pack probe --skills-root]
  verbs -->|phxd.pack.run.v1| run[phxd --ledger scratch pack run --project]
  verbs -->|phxd.seo-pipeline.v1, once the site is built| seo[phxd verify seo-pipeline]
  ledger[(scratch ledger: phxd init, phxd project register)] --> run
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
  judge --> lines[one line per pack; exit 0, 1 or 2]
```

The scratch directory holds the exported tree and the ledger. It is created with the process's
temporary directory, outside both repositories (the run refuses a scratch directory inside either),
and removed when the run ends, whatever its verdict. The cards are kept under `$BOX_PACKS_OUT`, to
be posted on the pull request.

## The verdict of one pack

| the card says | the wiring says | the pack reads | fails the run |
|---|---|---|---|
| no card: phxd refused, or the card's schema is not the verb's | anything | `FAIL` with phxd's last line | yes |
| examined 0 (a declarative walk, or no built site) | `pending #N` | `pending #N` | no |
| examined 0 | nothing | `FAIL` VOID | yes |
| examined 1 or more | `pending #N` | `FAIL`, the pending issue stale | yes |
| a blocking row red | the row under `expected_red` | counted expected | no |
| a blocking row red | not named | `FAIL`, an unexpected red by name | yes |
| a row named under `expected_red` is not red | named | `FAIL`, a stale expectation by name | yes |
| an advisory row failed | anything | counted as neither | no |

The proxy scan is judged by its row lines, never by its exit, because `check all` exits VOID over
RED. Any RED line fails the run. With no RED line, it reads `pending #N` while it examines no
settings document and the wiring names that issue; a pending issue it has outgrown (a settings
document examined) is stale; and a blocking VOID row with a settings document present is VOID.

| step | what crosses | guard |
|---|---|---|
| repository to scratch | the committed tree at `--rev`, never the working copy | `git archive` of a resolved commit |
| scratch to judged tree | DeckStreak's own files | `.packs/` and every `VENDORED.json` path removed before any pack runs |
| catalog to verb | the schema a pack's row declares | a schema no verb admits, or a pack the catalog lacks, fails that pack by name |
| card to verdict | each row's id and state | the wiring's expectations; a card that is not the verb's schema is no card |
| run to pull request | one line per pack and a summary | exit 0 only when every pack examined a row or reads pending, and none failed |
