### Changed

- Every pack is judged on the maintainer's box, and public CI judges none. `scripts/box-packs.sh`
  reads its wiring and pin from a private file (`PACKS_WIRING`), with the packs' checkout
  (`PACKS_CHECKOUT`) and their runner (`PACKS_RUNNER`), and refuses VOID by name without them. It
  keeps the removed row runner's judgment of enforced, pending and deferred packs, runs the sdd, ddd
  and tdd probes and the proxy-client and apiKeyHelper scans from the checkout, and fails when a data
  file DeckStreak keeps drifts from the pack it came from. `--post-status` posts one verdict-only
  `box/packs` commit status on the judged commit; it is not a required check (ADR-069).
- The box run holds every advisory departure the durable lint reports on the deploy templates to a
  waiver in its unit, with a why of more than five words, or to an open issue the private file
  names; an unwaived departure fails the durable-services pack by name, and a waiver that matches
  nothing is stale. A public test pins the templates' waivers.
- The gate and CI drop the `packs` stage and job; the scrub stage runs the public scrub alone.
  `scripts/check.sh` names its log directory only on stderr, and never when a caller names one.
- The vault adapter compiles in its own rails, default layout and gate classes
  (`crates/vault/data/`), and the public scrub reads its own shapes (`scripts/scrub-rules/`), each
  reduced to the fields its parser reads. Three hand-proved mutation rows hold the adapter to them.
- A criterion of a delivered SPEC whose test this removal takes away is retired insert-only: its id
  is struck, and its command and red-first lines sit in a `retired` fence (SPEC-056 R14).

### Removed

- `.packs/` (the vendored skills, rows, probes, `VENDORED.json` and `wiring.json`),
  `scripts/pack-rows.py`, `scripts/vendor-packs.py`, the four methodology probes and
  `scripts/no-apikeyhelper-scan.py`, with the four tests that tested only them.
- `methodology.json`'s `vendored_from`, and `ruff.toml`'s vendored excludes.
