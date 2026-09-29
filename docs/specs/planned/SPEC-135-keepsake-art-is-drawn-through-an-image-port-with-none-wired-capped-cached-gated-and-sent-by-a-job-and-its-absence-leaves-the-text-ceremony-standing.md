# SPEC-135: keepsake art is drawn through an image port with none wired, capped, cached, gated and sent by a job, and its absence leaves the text ceremony standing

- **Wave:** W7. **Issues:** #126 (AI keepsake art for chapter ceremonies) and the pipeline #125
  shares (epic #8); the provider is the owner's decision, #169. **Context(s):**
  `deck-streak-agent` (the image port, the draw, its cap, its cache, its gate and `agent_images`);
  `deck-streak-coordination` (the keepsake's request after the ceremony, and the sending job
  `image_art`); `deck-streak-daemon` (the job's wiring); `ai-safety.json`.
- **Decided by:** ADR-135 (this SPEC's: the image port with no provider wired, drawn by a sending
  job, capped and cached), ADR-054 (the AI route is optional and no-AI mode is the default), ADR-124
  (a job that sends runs under its own template, planned in #343), ADR-041 (the one router) and
  ADR-012 (the parity oracle).
- **Prerequisites:** SPEC-021 (data rights), SPEC-027 (the job table), SPEC-029 (the goldens),
  SPEC-041 (the router), SPEC-043 (the agent core and its output gate), SPEC-066 (the credential
  loader), SPEC-071 (the fold), SPEC-074 (the chapter ceremony), SPEC-100 (the sending template) and
  SPEC-132 (the photo occasion). SPEC-043, SPEC-074, SPEC-100 and SPEC-132 are unlanded.
  **Mutation band:** `S13500-S13599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-135.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's keepsake.** `showcase.py:ShowcaseLayer._month_ceremony` (at `27ee2bc`) sends the
  chapter's text ceremony, then, outside quiet hours, asks `_generate_art` for a keepsake and sends it
  as a photo captioned with the chapter title. `_generate_art` answers nothing without its key, the
  cached image for the key when there is one, nothing when the day's count has reached
  `art.ART_DAILY_CAP` (2), and otherwise counts the attempt BEFORE calling `art.py:generate`.
  `generate` answers nothing on a transport error or a timeout (`_TIMEOUT_S`, 90 seconds), on an
  HTTP status of 400 or more, and on a body it cannot read (`data[0].b64_json`); otherwise the image,
  cached by its key for good. Every nothing leaves the ceremony as text.
- **DeckStreak has no image route.** ADR-054 makes every AI route optional and off by default, and
  the owner has not chosen an image provider (#169). The design must hold with none.
- **Where it lives.** SPEC-001 files the feature under progression; the draw lives in the agent
  context because the output gate every AI output passes is the agent's (SPEC-043 R11), and
  progression has no edge to it (ADR-135). The ceremony stays progression's (SPEC-074).
- **The fold must not wait on a provider.** The ceremony is raised by the fold (SPEC-074 R4). A
  90-second call inside it would hold the sync.

## 2. Requirements

The port

R1. `agent::images::ImageProvider` is a port with one call, `draw(&ImageRequest) -> DrawFuture`,
    answering a PNG or JPEG's bytes or a closed `DrawFailure`: `refused` (the provider answered a
    refusal), `unreachable` (no answer), or `unparseable` (an answer with no image). The daemon wires
    `NoImageProvider`, which is off. No provider is wired until #169's delivery chooses one; its
    credential, `image-provider-key`, is named by no unit until that delivery binds it, is read
    through the kernel's `CredentialLoader` (SPEC-066) as optional (`Missing` is off, `Empty` refuses
    start by its id), and is never in the repository or the environment.
R2. The prompt is the request's own sentence followed by the predecessor's style sentence
    (`art.STYLE`), equal to the golden `art.constants`.

The draw

R3. `agent::images::draw_once(key)` takes one `pending` row of `agent_images` and settles it with
    exactly one outcome, in this order, equal to the goldens `art_generate_art` and `art_generate`:
    1. the provider is off: `no_provider`, and no call;
    2. a `ready` or `sent` row holds an image for the key: `cached`, and no call (a cached image
       counts nothing);
    3. `ART_DAILY_CAP` draws were already counted on the study day: `capped`, and no call;
    4. otherwise the draw is COUNTED on the study day, in its own write, before the call; then the
       call, bounded at 90 seconds: `timeout`, `refused`, `unreachable`, `unparseable`, or an image.
    The study day is the kernel's (the 04:00 rollover), and the cap is shared by every kind of
    request.
R4. An image passes the output gate (SPEC-043 R11) before it is stored as `ready`: `ai-safety.json`
    declares the task `keepsake-art` (and, for SPEC-136, `share-card-art`) with `on_invalid: withhold`
    and a gate naming every class that applies to an image's prompt and caption; a failing class
    settles the row `withheld` with the class, and the image is discarded, never stored and never
    shown.
R5. Every outcome of R3 and R4 is final: the row is drawn once, as the predecessor marks the
    ceremony done whatever its art came to. A row is never drawn twice, and a key already present
    enqueues nothing.

The keepsake

R6. When SPEC-074 R4's ceremony step raises the chapter's occasion and the router answers anything
    but `already_recorded`, the same fold step enqueues one `pending` row keyed
    `keepsake:chapter:<number>`, kind `keepsake`, with the predecessor's prompt sentence for the
    chapter's title, season XP and crown days and its caption (the title after the book emoji), both
    equal to the golden `keepsake_prompt`. The fold makes no call to a provider.
R7. With no provider, the product shows the text ceremony alone (SPEC-074 R4), and nothing else: no
    photo, no error, no placeholder. The row settles `no_provider` at the job's next run.

The job

R8. The job `image_art` (coordination's job table, marked sending under SPEC-100 R28) runs hourly at
    minute 53 (SPEC-027 R1's hourly kind; a minute R2 admits) on the sending template (ADR-124). Each
    run draws the `pending` rows oldest first (R3), then raises each `ready` row through
    `Router::route_photo` (SPEC-132 R4) as a `celebration` occasion keyed by the row's key.
R9. `Sent` settles the row `sent` with the Bot API's file id; `photo_unsupported` or another final
    withhold settles it by that reason; `NotNow` leaves it `ready` for the next run, and a `ready` row
    older than 720 minutes (SPEC-041 R7's abandonment age) settles `abandoned`. A failure of one row
    never stops the run's other rows.
R10. A provider's failure, timeout or absence never blocks, delays or changes a ceremony, a streak,
     a grant or the fold: none of them reads `agent_images`.

Rights and rules

R11. The agent owns `agent_images` (key, kind, prompt, caption, state, the reason, the counted study
     day, the image, its SHA-256, the file id, `created_at`, `settled_at`), `STRICT`, `UNIQUE (key)`,
     created by `migrations/013501_agent_images.sql`. It owes SPEC-021's six files (§4): the export
     writes each row with its image as base64, and the erase empties the table.
R12. Nothing of a request's prompt, caption or image reaches a log line or a span field; a draw logs
     its key and outcome only.
R13. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no AI by
     default (R1, R7), no unbounded work (R3's cap and bound, R9's age) and one router (R8).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with no provider wired a pending row settles `no_provider` and no call is made | `a_draw_without_a_provider_settles_no_provider` |
| A2 | a key with a stored image settles `cached`, makes no call and counts nothing | `a_cached_key_makes_no_call` |
| A3 | a third draw on one study day settles `capped` with no call; the second is drawn | `a_third_draw_on_one_study_day_is_capped` |
| A4 | the draw is counted before the call, so a draw that fails still counts | `a_failed_draw_still_counts` |
| A5 | the count starts again on the next study day, at the 04:00 rollover | `the_cap_starts_again_at_the_rollover` |
| A6 | a call still running at 90 seconds settles `timeout`; the test runs on a paused clock and bounds its own wait at 91 seconds | `a_draw_is_bounded_at_ninety_seconds` |
| A7 | a refusal, no answer and an answer with no image settle `refused`, `unreachable` and `unparseable` | `every_draw_failure_is_named` |
| A8 | an image the output gate fails settles `withheld` with its class and is never stored | `a_withheld_image_is_never_stored` |
| A9 | a row is drawn once, and enqueuing a present key writes nothing | `a_row_is_drawn_once` |
| A10 | the draw's outcomes equal the goldens `art_generate_art` and `art_generate` | `the_draw_matches_the_parity_goldens` |
| A11 | the prompt's style and the constants equal the golden `art.constants` | `the_style_and_constants_match_the_parity_golden` |
| A12 | the ceremony's step enqueues one keepsake row with the golden's prompt and caption, and makes no call | `the_ceremony_enqueues_one_keepsake` |
| A13 | with no provider the ceremony is the text alone, and no photo occasion is raised | `with_no_provider_the_ceremony_is_text_alone` |
| A14 | a `ready` row is raised as a photo and settles `sent` with its file id | `a_ready_keepsake_is_sent_as_a_photo` |
| A15 | a `NotNow` row stays `ready`, and at 720 minutes settles `abandoned` | `a_keepsake_not_sent_in_time_is_abandoned` |
| A16 | an `Unsupported` transport settles the row `photo_unsupported` | `an_unsupported_transport_settles_the_keepsake_by_name` |
| A17 | a provider failure leaves the ceremony, the streak and the grants of the day unchanged | `a_provider_failure_changes_no_ceremony_or_grant` |
| A18 | the job table holds `image_art` hourly at minute 53, sending, at a minute SPEC-027 R2 admits | `the_image_art_job_is_hourly_and_sending` |
| A19 | the job role builds `NoImageProvider` and reads the optional credential, `Missing` as off | `the_image_art_job_starts_with_no_provider` |
| A20 | an empty `image-provider-key` refuses start by its id | `an_empty_image_provider_key_refuses_start` |
| A21 | `agent_images`' export and erase are symmetric, the image exported as base64 | `agent_images_export_and_erase_are_symmetric` |
| A22 | no prompt, caption or image reaches a log line | `no_prompt_or_image_reaches_a_log` |
| A23 | the job's timer is on the sending template and holds the table's calendar | `test_the_image_art_timer_is_on_the_sending_template` |

```acceptance
A1: cargo test -p deck-streak-agent --test images -- --exact a_draw_without_a_provider_settles_no_provider
A2: cargo test -p deck-streak-agent --test images -- --exact a_cached_key_makes_no_call
A3: cargo test -p deck-streak-agent --test images -- --exact a_third_draw_on_one_study_day_is_capped
A4: cargo test -p deck-streak-agent --test images -- --exact a_failed_draw_still_counts
A5: cargo test -p deck-streak-agent --test images -- --exact the_cap_starts_again_at_the_rollover
A6: cargo test -p deck-streak-agent --test images -- --exact a_draw_is_bounded_at_ninety_seconds
A7: cargo test -p deck-streak-agent --test images -- --exact every_draw_failure_is_named
A8: cargo test -p deck-streak-agent --test images -- --exact a_withheld_image_is_never_stored
A9: cargo test -p deck-streak-agent --test images -- --exact a_row_is_drawn_once
A10: cargo test -p deck-streak-agent --test images_parity -- --exact the_draw_matches_the_parity_goldens
A11: cargo test -p deck-streak-agent --test images_parity -- --exact the_style_and_constants_match_the_parity_golden
A12: cargo test -p deck-streak-coordination --test keepsake -- --exact the_ceremony_enqueues_one_keepsake
A13: cargo test -p deck-streak-coordination --test keepsake -- --exact with_no_provider_the_ceremony_is_text_alone
A14: cargo test -p deck-streak-coordination --test image_art -- --exact a_ready_keepsake_is_sent_as_a_photo
A15: cargo test -p deck-streak-coordination --test image_art -- --exact a_keepsake_not_sent_in_time_is_abandoned
A16: cargo test -p deck-streak-coordination --test image_art -- --exact an_unsupported_transport_settles_the_keepsake_by_name
A17: cargo test -p deck-streak-coordination --test image_art -- --exact a_provider_failure_changes_no_ceremony_or_grant
A18: cargo test -p deck-streak-coordination --test job_table -- --exact the_image_art_job_is_hourly_and_sending
A19: cargo test -p deck-streak-daemon --test roles -- --exact the_image_art_job_starts_with_no_provider
A20: cargo test -p deck-streak-daemon --test roles -- --exact an_empty_image_provider_key_refuses_start
A21: cargo test -p deck-streak-agent --test rights -- --exact agent_images_export_and_erase_are_symmetric
A22: cargo test -p deck-streak-agent --test images -- --exact no_prompt_or_image_reaches_a_log
A23: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_image_art_timer_is_on_the_sending_template
```

## 3a. What the box run judges

The private-wiring change that enforces B1 and B2 is none beyond what the packs already judge: the
privacy-gdpr and durable-services packs are enforced for DeckStreak, and this delivery adds a table
and a job to their populations. The delivery hands back an empty JSON diff and says so.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/agent/src/rights.rs`: `agent_images` is declared with purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `deploy/systemd/deck-streak-job-send@image_art.timer`, `crates/coordination/src/image_art.rs` and `crates/agent/src/images.rs`: every outbound call is bounded and every row is settled once | the durable-services pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/images.rs` | `deck-streak-agent` | added: the port, `NoImageProvider`, the draw, its cap, its bound and its outcomes |
| `crates/agent/src/rights.rs` | `deck-streak-agent` | changed: the port exports and erases `agent_images` (SPEC-043 adds it) |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module |
| `crates/agent/tests/images.rs` | `deck-streak-agent` | added: A1 to A9, A22 |
| `crates/agent/tests/images_parity.rs` | `deck-streak-agent` | added: A10, A11 |
| `crates/agent/tests/rights.rs` | `deck-streak-agent` | changed: A21 |
| `migrations/013501_agent_images.sql` | `deck-streak-agent` | added: the table |
| `ai-safety.json` | repo | changed: the tasks `keepsake-art` and `share-card-art` |
| `crates/coordination/src/keepsake.rs` | `deck-streak-coordination` | added: the ceremony step's enqueue |
| `crates/coordination/src/image_art.rs` | `deck-streak-coordination` | added: the job's run, its raises and its settles |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `image_art` |
| `crates/coordination/src/recompute/seasons.rs` | `deck-streak-coordination` | changed: the ceremony step calls the enqueue (SPEC-074 adds it) |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules |
| `crates/coordination/tests/keepsake.rs` | `deck-streak-coordination` | added: A12, A13 |
| `crates/coordination/tests/image_art.rs` | `deck-streak-coordination` | added: A14 to A17 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A18 |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the agent's port registered for `agent_images` |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `agent_images` row |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: `image_art` joins `NoImageProvider`, the router and the optional credential |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the image port's wiring, `NoImageProvider` |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A19, A20 |
| `deploy/systemd/deck-streak-job-send@image_art.timer` | deploy | added: hourly at minute 53 |
| `deploy/rail-contract.json` | deploy | changed: the timer's calendar |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A23 |
| `tools/parity-oracle/registry/spec_135.py` | parity oracle | added: the four goldens' adapters |
| `tools/parity-oracle/goldens/art.constants.json` | parity oracle | added |
| `tools/parity-oracle/goldens/art_generate.json` | parity oracle | added |
| `tools/parity-oracle/goldens/art_generate_art.json` | parity oracle | added |
| `tools/parity-oracle/goldens/keepsake_prompt.json` | parity oracle | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `agent_images` |
| `privacy.json` | repo | changed: `agent_images` |
| `PRIVACY.md` | docs | changed: one line for the drawn images |
| `docs/specs/SPEC-135-keepsake-art-is-drawn-through-an-image-port-with-none-wired-capped-cached-gated-and-sent-by-a-job-and-its-absence-leaves-the-text-ceremony-standing.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w7-image-pipeline-and-its-no-provider-path.md` | docs | added by the W7 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-135.md` | docs | added |
| `scripts/mutation-rows.d/S13500-S13599.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It chooses no image provider and wires none: the owner decides (#169).
- It draws no share card and shows no gallery; the share card does (#125).
- It draws no art for a CEFR band-up, which the predecessor also reserved; that celebration is its
  own item (#85).
- It writes no image file to disk and serves no image publicly (#126).

## 6. Risks

- **A provider that bills per call.** R3's cap is counted before the call; detected by A3 and A4.
- **A draw that hangs a job.** R3's 90-second bound; detected by A6.
- **An unsafe image shown.** R4's gate before storing; detected by A8. The ai-content-safety pack's
  classes judge text today, so the image's own content is judged through its prompt and caption
  until the pack gains an image class (#29).
- **A keepsake that repeats.** R5 and the key; detected by A9.

## 7. Parity goldens

`tools/parity-oracle/registry/spec_135.py` generates, at `27ee2bc`:

- `art.constants`: `ART_DAILY_CAP` (2), `_TIMEOUT_S` (90.0) and `STYLE`, from `art.py`.
- `art_generate`: `art.py:generate` over a recording HTTP double: a transport error, a timeout, the
  statuses 399 and 400 (the boundary), a body without `data`, without `b64_json` and not base64, and
  a success; with the outcome of each.
- `art_generate_art`: `showcase.py:ShowcaseLayer._generate_art` over a recording store and double: no
  key, a cache hit, counts 0, 1 and 2 (2 is the on-boundary case that `capped` tells apart from a
  draw), a new day after a count of 2, and a failed draw that still counts.
- `keepsake_prompt`: the prompt sentence and the caption `_month_ceremony` builds for three titles,
  season XP values and crown-day counts.

## 8. Tables and the v9 import

`agent_images` (agent, `migrations/013501_agent_images.sql`). The predecessor cached its art as files
under its data directory and kept its count in two settings rows (`art_gen_day`, `art_gen_count`); it
had no table of images, so W8's import maps no predecessor table into it, and a keepsake the
predecessor drew is not imported (#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13501-OFF-FIRST` | `crates/agent/src/images.rs` | no provider settles before any call | `images::a_draw_without_a_provider_settles_no_provider` |
| `S13502-CACHE-FREE` | `crates/agent/src/images.rs` | a cached key counts nothing | `images::a_cached_key_makes_no_call` |
| `S13503-CAP` | `crates/agent/src/images.rs` | `ART_DAILY_CAP` compared with `>=`; the test names counts 1 and 2 | `images::a_third_draw_on_one_study_day_is_capped` |
| `S13504-COUNT-BEFORE-CALL` | `crates/agent/src/images.rs` | the count's write precedes the call | `images::a_failed_draw_still_counts` |
| `S13505-STUDY-DAY` | `crates/agent/src/images.rs` | the count keys on the kernel's study day | `images::the_cap_starts_again_at_the_rollover` |
| `S13506-BOUND` | `crates/agent/src/images.rs` | 90 seconds; the test runs on a paused clock and bounds its own wait at 91 seconds, so a mutant that waits longer fails rather than hangs | `images::a_draw_is_bounded_at_ninety_seconds` |
| `S13507-GATE` | `crates/agent/src/images.rs` | a failing class withholds before the store | `images::a_withheld_image_is_never_stored` |
| `S13508-ONCE` | `migrations/013501_agent_images.sql` | `UNIQUE (key)` (a script-mutation row whose cargo killer is in `deck-streak-agent`) | `images::a_row_is_drawn_once` |
| `S13509-ENQUEUE-NO-CALL` | `crates/coordination/src/keepsake.rs` | the ceremony step only enqueues | `keepsake::the_ceremony_enqueues_one_keepsake` |
| `S13510-ABANDON-AGE` | `crates/coordination/src/image_art.rs` | 720 minutes; the test names 719 and 720 | `image_art::a_keepsake_not_sent_in_time_is_abandoned` |
| `S13511-UNSUPPORTED-SETTLES` | `crates/coordination/src/image_art.rs` | `photo_unsupported` settles the row | `image_art::an_unsupported_transport_settles_the_keepsake_by_name` |
