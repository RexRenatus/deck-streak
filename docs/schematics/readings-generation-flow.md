# Schematic: the nightly readings generation, topic by topic

Kind: data flow. Read at DeckStreak `main` ce3683d (ADR-010, ADR-011, ADR-019,
docs/schematics/data-flow.md), at the predecessor's `27ee2bc`
(`pipeline_layers/preread.py:PreReadLayer.run_preread_generation`, `preread.py:check_coverage`,
`reading_notes.py:roll_forward`, `scheduler.py:run_startup_catchup`), and at the packs vendored from
`19bb0f3` (study-duties, learning-science, law-professors, language-mentors). Added by SPEC-046;
SPEC-048 and SPEC-053 act on it. The job never syncs, it reads the study day's sync outcome
(ADR-037), and it checks the AI route before anything else (ADR-054).

```mermaid
flowchart TD
  timer["readings-generate at the rollover hour plus 40 minutes, or caught up within 360 minutes"] --> claim{"fire claimed for this study day?"}
  claim -- "already" --> stop["nothing"]
  claim -- "claimed" --> route{"AI route configured?"}
  route -- "absent, the default" --> absent["every topic ai_route_absent: nothing resolved, attempted, stored or alerted"]
  route -- "configured" --> sync{"the study day's sync succeeded? read under the shared collection lock, never run"}
  sync -- "not succeeded" --> cnt["every topic could_not_tell, sync_failed"]
  sync -- "succeeded" --> pause{"no review on the two study days before?"}
  pause -- "yes" --> paused["every topic paused"]
  pause -- "no" --> resolve["resolve the day set per root, attribute by original deck"]
  resolve --> rollfwd["once a night, when the archive switch is on: roll the carried notes forward, archive the rest"]
  rollfwd --> topics["for each topic, one after another, no daily cap"]
  topics --> lock{"topic lock free, or free within 620 seconds?"}
  lock -- "held" --> holder["left to its holder"]
  lock -- "taken" --> kind{"state of the day set"}
  kind -- "empty" --> nnc["no_new_cards"]
  kind -- "same digest as the last ready reading" --> carried["ready, carried"]
  kind -- "new" --> seed["seed, persona, word target T(n), fenced cards"]
  seed --> screen{"a law seed with every anchor unusable, or empty?"}
  screen -- "yes" --> failed["failed: closed reason, nothing written"]
  screen -- "no" --> run["agent run and pack gate (SPEC-043)"]
  run --> cov{"coverage gates: complete, roster, anchors, band, list markers, contract"}
  cov -- "red, first time" --> repair["one repair, naming the gate"] --> run
  cov -- "red again, or unavailable on a configured route" --> failed
  cov -- "green" --> store["store the reading"]
  store --> archive{"vault archive switch on?"}
  archive -- "off" --> off["recorded vault_archive_off"]
  archive -- "on" --> vault["date tree: create the new note, or replace the body of the same reading"]
  vault -- "write failed" --> vfail["reading kept, vault_write_failed recorded"]
  cnt --> carryall["when the archive switch is on: roll every topic's note forward, rolls plus one"]
  paused --> carryall
  carryall --> health["health check: verdict and pages"]
  absent --> health
  holder --> health
  nnc --> health
  carried --> health
  failed --> health
  off --> health
  vault --> health
  vfail --> health
```

Card text leaves the host only inside the fenced prompt, through a configured AI route (the owner's
subscription proxy), and only for a topic that reached the agent run. Nights with no route, paused
nights and nights whose sync did not succeed make no collection copy and no call; with no route, the
health check reports the readings not enabled and pages nothing (SPEC-050).
