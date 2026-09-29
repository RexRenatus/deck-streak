# SPEC-137: the public achievement page is opt-in, allow-listed, served by DeckStreak, and withdrawn by switching off or erasing

- **Wave:** W7. **Issues:** #156 (public achievement publishing) and #348 (where the page is
  hosted) (epic #8).
  **Context(s):** `deck-streak-publishing` (the typed pages, the composition, the badges, the
  scrubber, the canonical bytes and the content hash, the page's rendering, the site writer, the
  switch, the tables `publishing_state` and `published_files`); `deck-streak-coordination` (the
  job `public_page`, the erase's withdrawal, the switch in the settings census); `deck-streak-api` (the
  status and publish-now routes); `deck-streak-bot` (/delete withdraws first); `deck-streak-daemon`
  (the api, bot, data and job roles hand over the site); the Mini App (`web/app`, the publishing
  section of the settings screen); deploy (the Caddy handle and the job's timer).
- **Decided by:** ADR-137 (this SPEC's: DeckStreak serves the page itself, so an unpublish and an
  erase withdraw it; the public repository is rejected), ADR-130 (the settings screen writes stored
  runtime settings only), ADR-012 (the parity oracle proves every number), ADR-027 (each job on its
  own minute).
- **Prerequisites:** SPEC-020 (migrations), SPEC-021 (the data-rights registry), SPEC-024 (the
  owner's session and the CSRF bound), SPEC-027 (the job table and its exit codes), SPEC-029 (the
  parity oracle), SPEC-032 (the Caddy block), SPEC-040 (the XP ledger), SPEC-071 (the day's rollup,
  score and grade) and SPEC-130 (the settings census and screen). SPEC-130 is unlanded. The job `public_page` needs
  SPEC-100 R28's INSTANCE widening (the effective-config check refuses a job name it does not list);
  it holds through SPEC-130. The sections
  whose sources are SPEC-072, SPEC-073, SPEC-076, SPEC-077, SPEC-078, SPEC-079 and SPEC-083 join
  when their contexts land (R6), so none of those is a prerequisite. **Mutation band:**
  `S13700-S13799`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-137.md` (ADR-016).

## 1. The problem, measured

- **The publishing context is empty.** `crates/publishing/src/lib.rs` holds a module comment and no
  code, and depends on the kernel only (CONTEXT-MAP). Nothing on dev composes, scrubs or serves a
  public page, and no table records one.
- **The predecessor's product.** A daily job (`publish.py:run`) composed a language page
  (`publish.py:compose_stats`) and a law page (`publish.py:compose_law_stats`), each with
  shields.io endpoint badges (`publish.py:build_badges`, `publish.py:build_law_badges`), passed
  both through a fail-closed deny-list (`publish.py:scrub_public`), skipped a payload whose hash had
  not moved (`publish.py:content_hash` over `publish.py:canonical_stats_bytes`), and pushed the
  files to a git repository (`publish.py:publish_to_repo`). It also spliced a region into a hub
  page between two markers (`publish.py:splice_hub_region`) and refused a push when the remote's
  history had been rewritten (`publish.py:_verify_remote_history`: `root_changed`,
  `count_decreased`, `head_not_ancestor`), at `27ee2bc`.
- **A pushed history cannot be withdrawn.** A page pushed to a git repository stays in its history
  and in every clone, so neither an unpublish nor an erase could withdraw it. Binding decision 4 of
  the W7 plan asks that an unpublish withdraw what was published and that export and erase cover
  it. #348 asks the owner where the page is hosted; this SPEC plans the self-served answer
  (ADR-137), and the two criteria of #156 that only a repository has depart with it (§5).
- **The predecessor's page passed some fields through.** `compose_stats` copies a language's
  display name and unit, the habits' and focus's accumulated maps, the focus subjects, the worst
  leech strand, the goals and the last skip day. A name, a unit and a subject come from the owner's
  private files or decks, and a passed-through map is not an allow-list. The law page already drops
  the strand for that reason (`publish.py:law_leech_summary`).
- **Unmeasured retention.** `publish.py:_retention_or_absent` answers `None` for a measured `0.0`
  whose day answered no card, and `build_badges` renders it as a neutral `n/a`, never `0%`.
- **Python's JSON bytes.** `canonical_stats_bytes` is `json.dumps(sort_keys=True,
  ensure_ascii=False)`, whose separators are `", "` and `": "`. A port that writes compact JSON
  hashes different bytes.

## 2. Requirements

The pages, typed (the allow-list)

R1. Publishing owns two typed pages, `LanguagePage` and `LawPage`. A page is built only from typed
    fields: integers, fixed-point numbers rounded as the predecessor rounds them, study days, and
    strings drawn from a closed catalog. No field is a free string, and no field is a map passed
    through. The fields are the allow-list:
    - language page: the study day; the score and its grade (analytics' `grade_band`); true
      retention (R5); the level, its title and emoji, total XP, XP into the level and XP for the
      next; the streak's current, longest and freezes; the totals (reviews, study hours and study
      days, with mature and total cards when the languages section is wired); each language's ISO
      639 code, current band, mastery percentage, mature and total cards, and each band's total,
      mature, percentage and achieved flag; the badges' count and each earned badge's key, tier,
      name, emoji and earned day; the reading streak in weeks, reading minutes and the writing
      streak; the leech counts (total, active, holding, regressed); focus minutes today and this
      week, the focus streak and focus hours all time; the skip days this month and all time and
      the cards moved all time;
    - law page: the study day, the track `law`, the law streak's current and longest, the law
      level, title, emoji and total XP, the law study days, and the law leech counts
      (`publish.py:law_leech_summary`).
R2. These predecessor fields are not published, each by name: a language's `name` and
    `current_unit`, `habits.week_label`, `habits.reading`, `habits.writing`,
    `habits.accumulated`, `habits.reading_goal_total_min`, `leeches.worst_strand`,
    `focus.daily_goal_min`, `focus.weekly_goal_min`, `focus.by_subject`, `focus.accumulated`
    (except focus hours all time, typed), and `skip_days.last_day`. No page carries a free-text
    note, a deck name, a vault path, a goal or a personal setting value.
R3. A string field's value is a member of its catalog: a level title and emoji of progression's
    level table, a grade of analytics' `grade_band`, a badge key, name and emoji of the badge
    catalog, a CEFR band from A1 to C2, and a language code of two or three lowercase letters.
    Composing a value outside its catalog is refused `not_in_catalog`.
R4. Every study day on a page (the page's day and each badge's earned day) is on or before the
    page's own study day, by the kernel's 04:00 rollover. A later day is refused `future_date`.
R5. `publishing::compose::retention_or_absent` equals `publish.py:_retention_or_absent`: a
    measured value other than `0.0` is kept; a measured `0.0` is kept when the day's rollup
    answered at least one card, and is absent otherwise.
R6. Each section is read through a port the composition root supplies. A section whose source has
    not landed in the build is omitted and named in the page's `omitted` list, never filled with
    zero, and the page says which sections it does not show. A page with no wired section is not
    written. At this plan, the language page's score, grade, retention, totals and total XP are
    wired (SPEC-040, SPEC-071); the level (SPEC-072), badges (SPEC-073), streak (SPEC-076),
    languages (SPEC-077), habits (SPEC-078), focus (SPEC-079) and skip days (SPEC-083) join as
    their contexts land. The law page is written once its streak and level are wired.
R7. The law page is built only from law-scoped sources: `LawPage` takes a law-track input with no
    language field, so no language-only value can be passed to it.

The checks after the allow-list

R8. `publishing::scrub::scrub_public` is the second line: a port of `publish.py:scrub_public` over
    the page's JSON value, with its key markers, its address pattern and its secret shapes, failing
    closed with the predecessor's violation text. The predecessor's literal value list is not
    ported (it names one deployment); the typed pages make it moot. A page the scrubber refuses is
    refused `scrub_refused`.
R9. `publishing::badges::build_badges` and `build_law_badges` equal their goldens, retention's
    neutral `n/a` included, and each badge is written as a shields.io endpoint file. A badge whose
    section is omitted is not written.

What is written

R10. `publishing::canonical::canonical_stats_bytes` reproduces the predecessor's bytes: the
     generation instant removed, keys sorted, non-ASCII kept, and the separators `", "` and
     `": "`. `publishing::canonical::content_hash` is SHA-256 over those bytes, then each file's
     name and bytes in sorted name order, then the page's text when present.
R11. A page is its directory's `stats.json`, `badges/<stem>.json`, `page.css` and `index.html`:
     the language page at the public directory's root and the law page under `law/`. The HTML is
     rendered from the page's typed fields alone, escaped, with `lang="en"`, a title and one
     heading, a table of numbers, the omitted sections named in words, and the study day it shows.
     It carries no script, no inline style, no form and no resource from another origin.
R12. `publishing::site::Site` writes a page in one `BEGIN IMMEDIATE` transaction on the page's
     records. It writes nothing when the page's content hash equals the recorded one. Otherwise it
     writes each file to a temporary name in the same directory and renames it, removes each
     recorded file of that page that the new page does not produce, and records each file (page,
     directory, path, SHA-256, length, content, content hash, published instant). Files are
     written with mode `0644` in directories of mode `0755`, whatever the unit's umask, so the web
     server reads them. A failed write rolls the records back, so the next run writes the page
     again.
R13. A page recorded under another directory than the configured one is withdrawn from it before
     the page is written under the configured one.

The switch and the job

R14. Publishing declares the stored runtime setting `public_achievements`, a switch, default off
     (SPEC-130 R1, R5), held in `publishing_state`. The census routes its write to
     `publishing::switch::set`, which the api role hands the site. Switching on writes the switch
     and nothing else, and is refused `publishing_unconfigured` while `DECKSTREAK_PUBLIC_DIR` is
     unset. Switching off withdraws every recorded file (a missing file counts as withdrawn),
     deletes the records and writes the switch in one transaction; with nothing published it
     writes the switch only.
R15. `DECKSTREAK_PUBLIC_DIR` is configuration, empty in `.env.example`; unset means publishing is
     off. The api and job roles read it.
R16. The job `public_page` is `DailyAtRollover { minute: 36 }`, `catch_up: true`, on the plain job
     template (it sends nothing): a minute SPEC-027 R2 admits and no other job holds. With the
     switch off it writes nothing and exits 0. With the switch on and the directory unset it exits
     with the page code (SPEC-027 R7). Otherwise it composes both pages and writes each through
     R12, so a second run on the same study day writes nothing.
R17. A page refused by R3, R4 or R8, or a failed write, leaves that page's last written files and
     records as they were, and the job exits with the page code. The other page is still written.
R18. `POST /api/publishing/run` runs R16's composition and write at once and answers each page's
     outcome (`published`, `unchanged`, `not_written`, or the refusal's name); it answers 409
     `publishing_off` while the switch is off and 503 `publishing_unconfigured` while the directory
     is unset. `GET /api/publishing` answers the switch, whether the directory is set, each
     published page's path under `/public/`, its published instant and file count, and each page's
     omitted sections. Both sit behind SPEC-024 R7's `OwnerSession`, and the run behind R9's CSRF
     bound.

Export, erase and the unpublish

R19. Publishing owns `publishing_state` (a single row: the switch and `created_at`) and
     `published_files` (page, directory, path, SHA-256, length, content, content hash, published
     instant, `created_at`), created `STRICT` by `migrations/013701_publishing_state_and_files.sql`,
     `published_files` with `UNIQUE (page, path)` and a `CHECK` that the page is `language` or
     `law`. Each owes SPEC-021's six files (§4). An export holds each published file's bytes as
     they were written, with its path and instant, and the switch. `publishing_state` is reset in
     place to off (the kernel's `Disposition::ResetInPlace`), and `published_files` is exported and
     erased.
R20. Both erase paths, the bot's /delete and the data role's erase, withdraw every recorded file
     first, through coordination's `publishing::withdraw_published`, then run the erase below it.
     A file that cannot be removed refuses the erase with `withdraw_failed`, before anything is
     erased, so the records still name what is public and the erase can be run again. When SPEC-131
     has landed, the order is the withdrawal, then SPEC-131's revocation step, then the engine;
     whichever of the two lands second chains them.
R21. Only publishing names `publishing_state` and `published_files` in a query or a migration.

Serving

R22. The Caddy block gains `handle_path /public/*` with `root * {$DECKSTREAK_PUBLIC_DIR}` and
     `file_server`, with no fallback: a withdrawn file answers 404. That handle sends
     `Cache-Control: no-cache` and its own policy, `default-src 'none'; style-src 'self';
     frame-ancestors 'none'; base-uri 'none'`, and the site-wide `Content-Security-Policy` moves
     into the SPA's and the API's handles so it cannot replace this one. The robots rule and
     `X-Robots-Tag: noindex` still cover `/public/`.

The screen

R23. The settings screen's publishing section (SPEC-130 R7) shows the switch. Switching on first
     shows, in words, what becomes public (R1's sections that are wired) and what never does (R2),
     and writes only on the owner's confirmation. The section shows each published page's path,
     its published instant and its omitted sections, and a publish-now control. A write shows the
     stored value the route answered. No bot command publishes or unpublishes.
R24. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no secret,
     address or forward-looking date on anything public (R2, R4, R8), no dishonest copy (R6's
     omitted sections, R5's `n/a`), and no opened inbound port (the page is one more handle of the
     existing Caddy site, R22).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `retention_or_absent` equals its golden: `0.0` with none answered, `0.0` with one answered, and `0.1` | `retention_or_absent_matches_the_parity_golden` |
| A2 | the language page equals `compose_stats`'s golden on every allow-listed field | `compose_stats_matches_the_parity_golden_on_the_allow_list` |
| A3 | each field R2 names is absent from the page's JSON and HTML, with a synthetic value planted in each source | `a_dropped_field_never_reaches_the_page` |
| A4 | the law page equals `compose_law_stats`'s golden | `compose_law_stats_matches_the_parity_golden` |
| A5 | the law leech counts equal `law_leech_summary`'s golden | `law_leech_summary_matches_the_parity_golden` |
| A6 | the law page carries no language-only field | `the_law_page_carries_no_language_field` |
| A7 | a planted earned day one study day after the page's day is refused `future_date`, and the page's own day passes | `a_planted_future_date_fails_closed` |
| A8 | a title, grade, band, badge or code outside its catalog is refused `not_in_catalog` | `a_string_outside_its_catalog_is_refused` |
| A9 | a secret-shaped badge name from a catalog double passes R3 and is refused `scrub_refused` | `a_planted_secret_fails_closed_at_the_scrubber` |
| A10 | an unwired section is omitted and named, and a page with no wired section is not written | `an_unwired_section_is_omitted_and_named` |
| A11 | `build_badges` equals its golden | `build_badges_matches_the_parity_golden` |
| A12 | `build_law_badges` equals its golden | `build_law_badges_matches_the_parity_golden` |
| A13 | an absent retention renders `n/a` in the neutral colour, and a measured `0.0` renders `0%` | `unmeasured_retention_renders_n_a_never_zero` |
| A14 | `scrub_public` equals its golden, each key marker and each boundary case included | `scrub_public_matches_the_parity_golden` |
| A15 | `canonical_stats_bytes` equals its golden | `canonical_stats_bytes_matches_the_parity_golden` |
| A16 | `content_hash` equals its golden | `content_hash_matches_the_parity_golden` |
| A17 | the colours and key markers equal `publish.constants` | `the_constants_match_the_parity_golden` |
| A18 | the page's HTML holds only the page's fields, escaped, with no script, inline style, form or foreign resource | `the_page_is_rendered_from_the_snapshot_alone` |
| A19 | an unchanged page writes nothing and leaves every file's modification time | `an_unchanged_page_writes_nothing` |
| A20 | a changed page is written, recorded, `0644` in `0755` | `a_changed_page_is_written_and_recorded` |
| A21 | a recorded file the new page does not produce is removed | `a_file_no_longer_produced_is_removed` |
| A22 | a page recorded under another directory is withdrawn from it first | `a_page_under_a_moved_directory_is_withdrawn_first` |
| A23 | switching off withdraws every file and record, and a second switch-off writes the switch only | `switching_off_withdraws_every_published_file` |
| A24 | the switch is declared, off by default, and refuses to switch on while the directory is unset | `the_switch_is_off_by_default` |
| A25 | `publishing_state` and `published_files` export and erase are symmetric | `publishing_export_and_erase_are_symmetric` |
| A26 | only publishing names its two tables | `only_publishing_names_its_tables` |
| A27 | a page other than `language` or `law` is refused by the schema | `an_unknown_page_is_refused_by_the_schema` |
| A28 | the job writes nothing while the switch is off | `the_job_writes_nothing_while_the_switch_is_off` |
| A29 | the job publishes both pages, and a second run on the same study day writes nothing | `the_job_publishes_once_and_a_rerun_writes_nothing` |
| A30 | a refused page keeps its last written files, the other page is written, and the job fails | `a_refused_page_keeps_the_last_clean_page_and_fails_the_job` |
| A31 | the erase withdraws every published file before the engine runs | `the_erase_withdraws_every_published_file_first` |
| A32 | a file that cannot be removed refuses the erase with `withdraw_failed` and erases nothing | `a_file_that_cannot_be_withdrawn_refuses_the_erase` |
| A33 | the job table holds `public_page` daily at minute 36, a minute SPEC-027 R2 admits | `the_publish_job_is_daily_at_a_free_minute` |
| A34 | the publishing routes answer 401 without the owner's session, and the run 403 across sites | `the_publishing_routes_are_owner_only` |
| A35 | the status names each published page, its instant, its file count and its omitted sections | `the_status_names_every_published_page_and_every_omitted_section` |
| A36 | publish-now answers 409 `publishing_off` and 503 `publishing_unconfigured`, and each page's outcome otherwise | `publish_now_refuses_while_off_or_unconfigured` |
| A37 | the job role runs `public_page` with the switch off and the directory unset, and exits 0 | `the_publish_job_runs_with_publishing_off` |
| A38 | /delete withdraws the public page before it erases, only after the owner confirms | `delete_withdraws_the_public_page_before_it_erases` |
| A39 | no bot command publishes or unpublishes | `no_bot_command_publishes` |
| A40 | the job's timer holds the table's calendar | `test_the_publish_timer_holds_the_tables_minute` |
| A41 | the Caddy block serves `/public/` with no fallback, no-cache and its own policy | `test_the_caddy_block_serves_the_public_page_without_a_fallback` |
| A42 | the section asks before it switches on and names what becomes public and what never does | `publishing asks before it switches on and names what becomes public` |
| A43 | the section shows each published page and each omitted section | `publishing shows each published page and each omitted section` |
| A44 | the data role's erase withdraws the public page before it erases, only with the confirmation word | `the_data_role_withdraws_the_public_page_before_it_erases` |

```acceptance
A1: cargo test -p deck-streak-publishing --test compose -- --exact retention_or_absent_matches_the_parity_golden
A2: cargo test -p deck-streak-publishing --test compose -- --exact compose_stats_matches_the_parity_golden_on_the_allow_list
A3: cargo test -p deck-streak-publishing --test compose -- --exact a_dropped_field_never_reaches_the_page
A4: cargo test -p deck-streak-publishing --test compose -- --exact compose_law_stats_matches_the_parity_golden
A5: cargo test -p deck-streak-publishing --test compose -- --exact law_leech_summary_matches_the_parity_golden
A6: cargo test -p deck-streak-publishing --test compose -- --exact the_law_page_carries_no_language_field
A7: cargo test -p deck-streak-publishing --test compose -- --exact a_planted_future_date_fails_closed
A8: cargo test -p deck-streak-publishing --test compose -- --exact a_string_outside_its_catalog_is_refused
A9: cargo test -p deck-streak-publishing --test compose -- --exact a_planted_secret_fails_closed_at_the_scrubber
A10: cargo test -p deck-streak-publishing --test compose -- --exact an_unwired_section_is_omitted_and_named
A11: cargo test -p deck-streak-publishing --test badges -- --exact build_badges_matches_the_parity_golden
A12: cargo test -p deck-streak-publishing --test badges -- --exact build_law_badges_matches_the_parity_golden
A13: cargo test -p deck-streak-publishing --test badges -- --exact unmeasured_retention_renders_n_a_never_zero
A14: cargo test -p deck-streak-publishing --test scrub -- --exact scrub_public_matches_the_parity_golden
A15: cargo test -p deck-streak-publishing --test canonical -- --exact canonical_stats_bytes_matches_the_parity_golden
A16: cargo test -p deck-streak-publishing --test canonical -- --exact content_hash_matches_the_parity_golden
A17: cargo test -p deck-streak-publishing --test constants -- --exact the_constants_match_the_parity_golden
A18: cargo test -p deck-streak-publishing --test render -- --exact the_page_is_rendered_from_the_snapshot_alone
A19: cargo test -p deck-streak-publishing --test site -- --exact an_unchanged_page_writes_nothing
A20: cargo test -p deck-streak-publishing --test site -- --exact a_changed_page_is_written_and_recorded
A21: cargo test -p deck-streak-publishing --test site -- --exact a_file_no_longer_produced_is_removed
A22: cargo test -p deck-streak-publishing --test site -- --exact a_page_under_a_moved_directory_is_withdrawn_first
A23: cargo test -p deck-streak-publishing --test site -- --exact switching_off_withdraws_every_published_file
A24: cargo test -p deck-streak-publishing --test switch -- --exact the_switch_is_off_by_default
A25: cargo test -p deck-streak-publishing --test rights -- --exact publishing_export_and_erase_are_symmetric
A26: cargo test -p deck-streak-publishing --test tables -- --exact only_publishing_names_its_tables
A27: cargo test -p deck-streak-publishing --test tables -- --exact an_unknown_page_is_refused_by_the_schema
A28: cargo test -p deck-streak-coordination --test publishing -- --exact the_job_writes_nothing_while_the_switch_is_off
A29: cargo test -p deck-streak-coordination --test publishing -- --exact the_job_publishes_once_and_a_rerun_writes_nothing
A30: cargo test -p deck-streak-coordination --test publishing -- --exact a_refused_page_keeps_the_last_clean_page_and_fails_the_job
A31: cargo test -p deck-streak-coordination --test publishing -- --exact the_erase_withdraws_every_published_file_first
A32: cargo test -p deck-streak-coordination --test publishing -- --exact a_file_that_cannot_be_withdrawn_refuses_the_erase
A33: cargo test -p deck-streak-coordination --test job_table -- --exact the_publish_job_is_daily_at_a_free_minute
A34: cargo test -p deck-streak-api --test publishing_routes -- --exact the_publishing_routes_are_owner_only
A35: cargo test -p deck-streak-api --test publishing_routes -- --exact the_status_names_every_published_page_and_every_omitted_section
A36: cargo test -p deck-streak-api --test publishing_routes -- --exact publish_now_refuses_while_off_or_unconfigured
A37: cargo test -p deck-streak-daemon --test roles -- --exact the_publish_job_runs_with_publishing_off
A38: cargo test -p deck-streak-bot --test commands -- --exact delete_withdraws_the_public_page_before_it_erases
A39: cargo test -p deck-streak-bot --test commands -- --exact no_bot_command_publishes
A40: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_publish_timer_holds_the_tables_minute
A41: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_caddy_block_serves_the_public_page_without_a_fallback
A42: pnpm exec vitest run web/app/src/lib/settings/publishing.test.ts -t "publishing asks before it switches on and names what becomes public"
A43: pnpm exec vitest run web/app/src/lib/settings/publishing.test.ts -t "publishing shows each published page and each omitted section"
A44: cargo test -p deck-streak-daemon --test roles -- --exact the_data_role_withdraws_the_public_page_before_it_erases
```

## 3a. What the box run judges

The private-wiring change that enforces B1 and B2 is the ux-laws pack reading the publishing
section and the privacy-gdpr pack reading the publishing context: the delivery hands back the JSON
diff that adds `web/app/src/lib/settings/Publishing.svelte` to the ux-laws population and
`crates/publishing/src/` to the privacy-gdpr population.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md`, `crates/publishing/src/data_rights.rs` and `migrations/013701_publishing_state_and_files.sql`: both tables are declared with purpose, basis and retention, the public page is a disclosure the owner switches on, and export and erase cover what was published | the privacy-gdpr pack |
| B2 | over `web/app/src/lib/settings/Publishing.svelte`: one primary action, a confirmation before the switch goes on, and a named state for off, unconfigured, published and refused | the ux-laws pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/publishing/src/compose.rs` | `deck-streak-publishing` | added: the two pages' composition, retention, the catalogs and the day check |
| `crates/publishing/src/pages.rs` | `deck-streak-publishing` | added: `LanguagePage`, `LawPage`, their sections and the source ports |
| `crates/publishing/src/badges.rs` | `deck-streak-publishing` | added: the badges |
| `crates/publishing/src/scrub.rs` | `deck-streak-publishing` | added: the scrubber |
| `crates/publishing/src/canonical.rs` | `deck-streak-publishing` | added: the canonical bytes and the content hash |
| `crates/publishing/src/render.rs` | `deck-streak-publishing` | added: `index.html` and `page.css` |
| `crates/publishing/src/site.rs` | `deck-streak-publishing` | added: the writer, the records and the withdrawal |
| `crates/publishing/src/switch.rs` | `deck-streak-publishing` | added: the setting's declaration and setter |
| `crates/publishing/src/data_rights.rs` | `deck-streak-publishing` | added: the context's data-rights port |
| `crates/publishing/src/lib.rs` | `deck-streak-publishing` | changed: the modules, and the module comment names no history check |
| `crates/publishing/Cargo.toml` | `deck-streak-publishing` | changed: sqlx, serde, serde_json and sha2 from the workspace, tempfile for tests |
| `crates/publishing/tests/compose.rs` | `deck-streak-publishing` | added: A1 to A10 |
| `crates/publishing/tests/badges.rs` | `deck-streak-publishing` | added: A11 to A13 |
| `crates/publishing/tests/scrub.rs` | `deck-streak-publishing` | added: A14 |
| `crates/publishing/tests/canonical.rs` | `deck-streak-publishing` | added: A15, A16 |
| `crates/publishing/tests/constants.rs` | `deck-streak-publishing` | added: A17 |
| `crates/publishing/tests/render.rs` | `deck-streak-publishing` | added: A18 |
| `crates/publishing/tests/site.rs` | `deck-streak-publishing` | added: A19 to A23 |
| `crates/publishing/tests/switch.rs` | `deck-streak-publishing` | added: A24 |
| `crates/publishing/tests/rights.rs` | `deck-streak-publishing` | added: A25 |
| `crates/publishing/tests/tables.rs` | `deck-streak-publishing` | added: A26, A27 |
| `migrations/013701_publishing_state_and_files.sql` | `deck-streak-publishing` | added: the two tables, the unique key and the page check |
| `crates/coordination/src/publishing.rs` | `deck-streak-coordination` | added: the job's use case over the source ports, and `withdraw_published` |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `public_page`, daily at minute 36 |
| `crates/coordination/src/settings_census.rs` | `deck-streak-coordination` | changed: the switch joins the census |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/publishing.rs` | `deck-streak-coordination` | added: A28 to A32 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A33 |
| `crates/api/src/publishing_routes.rs` | `deck-streak-api` | added: the status and publish-now routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/publishing_routes.rs` | `deck-streak-api` | added: A34 to A36 |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: /delete withdraws before it erases |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: A38, A39 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role reads the directory and hands the site and the source ports to the routes and the census |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role hands the withdrawal to /delete |
| `crates/daemon/src/role_data.rs` | `deck-streak-daemon` | changed: the data role's erase withdraws first |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the job `public_page`, with the site and the source ports |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A37, A44 |
| `deploy/caddy/deck-streak.caddy` | deploy | changed: the /public/* handle, and the site-wide policy moved into the two other handles |
| `deploy/systemd/deck-streak-job@public_page.timer` | deploy | added: daily at the rollover hour, minute 36 |
| `deploy/README.md` | deploy | changed: the new placeholder and the timer |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A40, A41; `WAIVED` gains the durable lint's departures for `deck-streak-job@public_page.timer`; TheCaddyBlock's two tests read the new `handle_path /public/*` child and the policy in each handle |
| `deploy/rail-contract.json` | deploy | changed: `public_page`'s calendar |
| `deploy/scripts/render-caddy.py` | deploy | changed: `{$DECKSTREAK_PUBLIC_DIR}` joins `PLACEHOLDERS` (an absolute path) |
| `scripts/tests/test_caddy_render.py` | repo | changed: the fourth placeholder |
| `scripts/tests/test_deploy_scripts.py` | repo | changed: the Caddy install tests fill the fourth placeholder |
| `.env.example`, `deploy/deck-streak.env.example` | repo | changed: `DECKSTREAK_PUBLIC_DIR`, empty |
| `settings.defaults.json` | repo | changed: `public_achievements` off |
| `web/app/src/lib/settings/publishing.ts` | miniapp | added: the section's client |
| `web/app/src/lib/settings/Publishing.svelte` | miniapp | added: the section |
| `web/app/src/lib/settings/publishing.test.ts` | miniapp | added: A42, A43 |
| `web/app/src/routes/settings/+page.svelte` | miniapp | changed: the section joined |
| `web/app/messages/*.json` | miniapp | changed: the section's strings, in each locale's catalog |
| `tools/parity-oracle/registry/spec_137.py` | repo | added: the goldens' registrations |
| `tools/parity-oracle/goldens/publish.constants.json` | repo | added |
| `tools/parity-oracle/goldens/publish_retention_or_absent.json` | repo | added |
| `tools/parity-oracle/goldens/publish_compose_stats.json` | repo | added |
| `tools/parity-oracle/goldens/publish_compose_law_stats.json` | repo | added |
| `tools/parity-oracle/goldens/publish_law_leech_summary.json` | repo | added |
| `tools/parity-oracle/goldens/publish_build_badges.json` | repo | added |
| `tools/parity-oracle/goldens/publish_build_law_badges.json` | repo | added |
| `tools/parity-oracle/goldens/publish_scrub_public.json` | repo | added |
| `tools/parity-oracle/goldens/publish_canonical_stats_bytes.json` | repo | added |
| `tools/parity-oracle/goldens/publish_content_hash.json` | repo | added |
| `docs/schematics/w7-publishing-and-unpublishing.md` | docs | added by the W7 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables rows for `publishing_state` and `published_files` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: publishing's port registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for both tables |
| `privacy.json` | repo | changed: both tables' purpose, basis and retention, and the public page as a disclosure |
| `PRIVACY.md` | docs | changed: one line for the public page |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-137-the-public-achievement-page-is-opt-in-allow-listed-served-by-deckstreak-and-withdrawn-by-switching-off-or-erasing.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-137.md` | docs | added |
| `scripts/mutation-rows.d/S13700-S13799.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It pushes to no git repository, so the remote-history anomalies of #156 (`root_changed`,
  `count_decreased`, `head_not_ancestor`) have nothing to guard; they return only if the owner
  chooses a repository (#348).
- It splices no hub region into another page, so #156's marker criterion departs with the
  repository (#348).
- It publishes no chart image; the charts are #152's, and a chart joins the page only through a
  change to its allow-list (#152).
- It wires no section whose source has not landed at its build; each is omitted and named until it
  does (#156), and the law page's leech counts come with the law-track summary (#134).
- It asks no search engine to index the page, and chooses no domain for it (#168).
- It adds no bot command to publish or unpublish (#156).
- It imports no predecessor publishing state; the v9 import leaves the switch off (#61).

## 6. Risks

- **A field that leaks.** R1's typed pages and R2's named drops; detected by A3, A6 and A8, with
  R8's scrubber behind them (A9, A14).
- **A page that outlives an unpublish or an erase.** R12's records, R14 and R20; detected by A23,
  A31 and A32. A copy a viewer already saved cannot be withdrawn; the confirmation says so (R23).
- **A forward-looking date.** R4; detected by A7.
- **JSON bytes that drift from the predecessor's.** R10's separators and float text; detected by
  A15 and A16.
- **A policy header replaced by the site's.** R22 moves the site-wide policy; detected by A41.
- **A job that pages daily for a refusal.** R17 keeps the last clean page; the page code names the
  refusal, so the owner sees it once a day until the source is fixed.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_137.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `publish.constants` | `publish.py:_GOLD`, `publish.py:_GREEN`, `publish.py:_NEUTRAL`, `publish.py:_DENY_KEY_MARKERS` | constants | none |
| `publish_retention_or_absent` | `publish.py:_retention_or_absent` | function | none; cases `0.0` with no row, `0.0` with `answered` 0, `0.0` with `answered` 1 (the boundary), and `0.1` |
| `publish_compose_stats` | `publish.py:compose_stats` | adapter | a synthetic snapshot, badges, two language progress rows, habits, focus, skip, leech and three rollups |
| `publish_compose_law_stats` | `publish.py:compose_law_stats` | adapter | a synthetic law-track input |
| `publish_law_leech_summary` | `publish.py:law_leech_summary` | adapter | synthetic leech rows, one of each status |
| `publish_build_badges` | `publish.py:build_badges` | function | none; cases with retention absent, `0.0` and `87.5` |
| `publish_build_law_badges` | `publish.py:build_law_badges` | function | none |
| `publish_scrub_public` | `publish.py:scrub_public` | function | none; one case per key marker, `255.255.255.255` and `256.1.1.1` (the boundary), a `ghp_` token of 20 characters and of 19 (the boundary), a private-key header, a denied key whose value is also denied, and a clean nested value |
| `publish_canonical_stats_bytes` | `publish.py:canonical_stats_bytes` | function | none; nested keys out of order, non-ASCII text, `1.0`, `12.3` and `null` |
| `publish_content_hash` | `publish.py:content_hash` | function | none; stats alone, two files given out of order, and a page text that alone changes the hash |

## 8. Tables and the v9 import

`publishing_state` and `published_files` (publishing,
`migrations/013701_publishing_state_and_files.sql`). Neither maps from a predecessor table: the
predecessor kept its content hash in the pushed repository and its history baseline in
`settings_kv` (`publish.py:_REMOTE_HISTORY_BASELINE_KEY`), and both depart with the repository
(#348). W8's import writes `publishing_state` with the switch off, so the owner switches it on anew.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13701-RETENTION-ABSENT` | `crates/publishing/src/compose.rs` | a `0.0` with none answered is absent; the golden names one answered | `compose::retention_or_absent_matches_the_parity_golden` |
| `S13702-NA-BADGE` | `crates/publishing/src/badges.rs` | an absent retention renders `n/a`, neutral | `badges::unmeasured_retention_renders_n_a_never_zero` |
| `S13703-DROPPED-FIELD` | `crates/publishing/src/compose.rs` | a language's name is not copied | `compose::a_dropped_field_never_reaches_the_page` |
| `S13704-FUTURE-DATE` | `crates/publishing/src/compose.rs` | a day after the page's is refused; the test also passes the page's own day | `compose::a_planted_future_date_fails_closed` |
| `S13705-CATALOG` | `crates/publishing/src/compose.rs` | a title outside the catalog is refused | `compose::a_string_outside_its_catalog_is_refused` |
| `S13706-LAW-ONLY` | `crates/publishing/src/compose.rs` | the law page takes the law streak, not the language streak | `compose::the_law_page_carries_no_language_field` |
| `S13707-OMITTED` | `crates/publishing/src/compose.rs` | an unwired section is named, not zero-filled | `compose::an_unwired_section_is_omitted_and_named` |
| `S13708-KEY-MARKER` | `crates/publishing/src/scrub.rs` | the key markers match as lowercase substrings | `scrub::scrub_public_matches_the_parity_golden` |
| `S13709-SECRET-SHAPE` | `crates/publishing/src/scrub.rs` | a `ghp_` token needs 20 characters; the golden names 19 and 20 | `scrub::scrub_public_matches_the_parity_golden` |
| `S13710-SEPARATORS` | `crates/publishing/src/canonical.rs` | the separators are `", "` and `": "` | `canonical::canonical_stats_bytes_matches_the_parity_golden` |
| `S13711-HASH-ORDER` | `crates/publishing/src/canonical.rs` | the files are hashed in sorted name order | `canonical::content_hash_matches_the_parity_golden` |
| `S13712-UNCHANGED` | `crates/publishing/src/site.rs` | an equal content hash writes nothing | `site::an_unchanged_page_writes_nothing` |
| `S13713-STALE-FILE` | `crates/publishing/src/site.rs` | a file no longer produced is removed | `site::a_file_no_longer_produced_is_removed` |
| `S13714-WITHDRAW` | `crates/publishing/src/site.rs` | switching off removes every recorded file | `site::switching_off_withdraws_every_published_file` |
| `S13715-DEFAULT-OFF` | `crates/publishing/src/switch.rs` | the switch's default is off | `switch::the_switch_is_off_by_default` |
| `S13716-PAGE-CHECK` | `migrations/013701_publishing_state_and_files.sql` | the page check (a script-mutation row whose cargo killer is in `deck-streak-publishing`) | `tables::an_unknown_page_is_refused_by_the_schema` |
| `S13717-OFF-WRITES-NOTHING` | `crates/coordination/src/publishing.rs` | the job reads the switch before it composes | `publishing::the_job_writes_nothing_while_the_switch_is_off` |
| `S13718-WITHDRAW-FIRST` | `crates/coordination/src/publishing.rs` | the withdrawal runs before the engine | `publishing::the_erase_withdraws_every_published_file_first` |
| `S13719-WITHDRAW-FAILED` | `crates/coordination/src/publishing.rs` | a file that cannot be removed refuses the erase | `publishing::a_file_that_cannot_be_withdrawn_refuses_the_erase` |
| `S13720-OWNER-ONLY` | `crates/api/src/publishing_routes.rs` | the routes take `OwnerSession` | `publishing_routes::the_publishing_routes_are_owner_only` |
