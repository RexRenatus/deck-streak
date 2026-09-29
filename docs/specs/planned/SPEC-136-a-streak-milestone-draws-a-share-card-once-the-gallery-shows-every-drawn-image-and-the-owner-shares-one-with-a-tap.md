# SPEC-136: a streak milestone draws a share card once, the gallery shows every drawn image, and the owner shares one with a tap

- **Wave:** W7. **Issue:** #125 (streak milestone share cards) (epic #8); its art is the owner's
  decision, #169. **Context(s):** `deck-streak-progression` (the milestone set);
  `deck-streak-coordination` (the fold's share-card step and the share use case); `deck-streak-api`
  (the gallery and share routes); `deck-streak-daemon` (the api role joins the bot transport to its
  router); the Mini App (`web/app`: the gallery screen).
- **Decided by:** ADR-136 (this SPEC's: a share is a prepared inline message the owner sends from
  Telegram's share sheet), ADR-135 (the image pipeline), ADR-054 (no-AI mode is the default), ADR-041
  (the one router) and ADR-012 (the parity oracle).
- **Prerequisites:** SPEC-024 (the owner's session), SPEC-028 (the Mini App shell), SPEC-029 (the
  goldens), SPEC-071 (the fold), SPEC-076 (the language streak), SPEC-132 (the photo occasion and
  the prepared share) and SPEC-135 (the draw, the job and `agent_images`). SPEC-076, SPEC-132
  and SPEC-135 are unlanded. **Mutation band:** `S13600-S13699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-136.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's share card.** `ghost_race.py:GhostRaceLayer._milestone_share_card` (at
  `27ee2bc`) runs when a study day moves the streak's current length (`pipeline.py`'s streak step),
  and only at a length in `constants.SHARE_CARD_STREAKS` (7, 30, 50, 100, 180, 365, 500 and 1000). It
  is once per length, ever (`sharecard:streak:<n>` in its notified ledger), asks
  `_generate_art` for a medallion under the key `streak_<n>`, sends it with a one-line caption that
  names the day (the golden `share_card_prompt` holds it), and marks the length done whatever the art
  came to. Without the art key nothing is sent and nothing fails.
- **A recorded deviation.** The predecessor's caption carries a second, mascot sentence, and its
  prompt builds on the style sentence SPEC-135 R2 records. DeckStreak's caption is `Day <n>.`
  alone, the prompt uses SPEC-135's neutral emblem motif, and the golden's adapter applies the same
  replacements, so A5 compares like with like.
- **Nothing shows or shares an image.** #125 asks for a gallery in the Mini App where sharing the
  image is one tap. The Mini App has no gallery and no share control.
- **Sharing needs no public link.** Telegram's share sheet can send a message the bot prepared
  (Bot API 8.0), so an image never needs a public URL or a domain (#168, ADR-136).

## 2. Requirements

The milestone

R1. `progression::share_cards::SHARE_CARD_STREAKS` is the predecessor's set, equal to the golden
    `share_card.constants`.
R2. A fold step in phase 7 (awards) of SPEC-071's fold reads the streak step's report (SPEC-076): when
    the language streak's current length CHANGED on a study day and the new length is in the set, it
    enqueues one `agent_images` row keyed `share_card:streak:<n>`, kind `share_card`, with the
    predecessor's prompt for `n` and the caption `Day <n>.`, without its mascot sentence (a recorded
    deviation, §1), equal to the golden `share_card_prompt`. A length
    reached again after a lapse enqueues nothing: the key is present (SPEC-135 R5), as the
    predecessor's ledger is once ever. A length that did not change enqueues nothing, and so does the
    law streak.
R3. The draw, its cap (shared with the keepsake), its gate and the send are SPEC-135's (R3 to R9), and
    the photo is raised through the router (SPEC-132 R4).
R4. With no provider, the product sends nothing and raises no error: the row settles `no_provider`
    and no occasion is raised (#125's silent skip), and the milestone's own celebration, if any, is
    unchanged.

The gallery

R5. `GET /api/images` (`OwnerSession`, SPEC-024 R7) lists the `ready` and `sent` rows, newest first:
    key, kind, caption, drawn instant, and whether it can be shared (a file id is held). It lists no
    other state and never a prompt.
R6. `GET /api/images/{key}` (`OwnerSession`) answers the image's bytes with its content type,
    `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`; an unknown key or a row
    with no image is 404 `image_unknown`. No image is served outside the owner's session or from a
    public path.
R7. The Mini App's `/gallery` screen shows each image with its caption as its text alternative and
    the label "AI-generated art". With none it shows its empty state, which says that no art has been
    drawn. It works in both Telegram colour schemes, animates nothing under reduced motion, and keeps
    the contrast the accessibility pack names.

Sharing

R8. `POST /api/images/{key}/share` (`OwnerSession`, SPEC-024 R9's CSRF bound) runs coordination's
    `prepare_share(key)`: a row with no file id is 409 `share_unavailable`; otherwise the router's
    `prepare_share` (SPEC-132 R8) with the row's file id and caption. `Ready` answers the prepared
    message's id; `Unsupported` or `Failed` answer 503 `share_unavailable` by name. At most 10 shares
    a minute (a chosen bound), then 429.
R9. The share control appears only when the row can be shared and the client reports Bot API 8.0 or
    later (`isVersionAtLeast('8.0')`); a tap asks the route, then passes the id to
    `WebApp.shareMessage`. On any refusal the control reports that the image cannot be shared now,
    and nothing else changes.
R10. The api role joins the bot transport, built from the credentials it already reads (SPEC-024
     R3), to its router so that `prepare_share` has a transport.

Rules

R11. `agent_images` is SPEC-135's; this SPEC adds no table.
R12. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no AI by
     default (R4), owner-only gating (R5, R6, R8) and one router (R3, R8).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the set equals the golden: a card is enqueued at 7, 30, 50, 100, 180, 365, 500 and 1000, and at none of 6, 8, 29, 31 or 999 | `a_card_is_enqueued_exactly_at_the_milestones` |
| A2 | a length reached again after a lapse enqueues nothing | `a_milestone_is_carded_once_ever` |
| A3 | a recompute that does not change the length enqueues nothing | `an_unchanged_length_enqueues_nothing` |
| A4 | the law streak enqueues nothing | `the_law_streak_draws_no_card` |
| A5 | the prompt and caption equal the golden `share_card_prompt` | `the_share_card_prompt_matches_the_parity_golden` |
| A6 | with no provider nothing is sent, no occasion is raised and no error is returned | `with_no_provider_the_share_card_is_silent` |
| A7 | the gallery lists `ready` and `sent` rows only, newest first, and never a prompt | `the_gallery_lists_drawn_images_only` |
| A8 | an image is served to the owner with `private, no-store` and `nosniff`, and 401 without a session | `an_image_is_served_to_the_owner_only` |
| A9 | an unknown key is 404 `image_unknown` | `an_unknown_image_is_image_unknown` |
| A10 | a share answers the prepared message's id | `a_share_answers_the_prepared_id` |
| A11 | a row with no file id is 409 `share_unavailable` | `a_share_without_a_file_id_is_unavailable` |
| A12 | an `Unsupported` or `Failed` transport is 503 `share_unavailable` | `a_share_the_transport_refuses_is_unavailable` |
| A13 | the eleventh share in a minute is 429 | `shares_are_bounded_per_minute` |
| A14 | the api role's router holds the bot transport | `the_api_role_joins_the_bot_transport` |
| A15 | the gallery shows each image with its caption as its alternative and the AI label | `the gallery labels every image` |
| A16 | with no images the gallery shows its empty state | `the gallery shows its empty state` |
| A17 | the share control is absent below Bot API 8.0 or without a file id | `the share control needs a shareable image and 8.0` |
| A18 | a tap passes the prepared id to `shareMessage` | `a tap shares the prepared message` |
| A19 | the gallery moves nothing under reduced motion | `the gallery honours reduced motion` |
| A20 | the milestone set equals the golden `share_card.constants` | `the_milestones_equal_the_golden` |
| A21 | the gallery's read answers `ready` and `sent` rows only | `the_gallery_reads_drawn_images_only` |
| A22 | a share of a row with no file id answers `share_unavailable` without a call | `a_share_without_a_file_id_makes_no_call` |

```acceptance
A1: cargo test -p deck-streak-coordination --test share_cards -- --exact a_card_is_enqueued_exactly_at_the_milestones
A2: cargo test -p deck-streak-coordination --test share_cards -- --exact a_milestone_is_carded_once_ever
A3: cargo test -p deck-streak-coordination --test share_cards -- --exact an_unchanged_length_enqueues_nothing
A4: cargo test -p deck-streak-coordination --test share_cards -- --exact the_law_streak_draws_no_card
A5: cargo test -p deck-streak-coordination --test share_cards -- --exact the_share_card_prompt_matches_the_parity_golden
A6: cargo test -p deck-streak-coordination --test share_cards -- --exact with_no_provider_the_share_card_is_silent
A7: cargo test -p deck-streak-api --test gallery_routes -- --exact the_gallery_lists_drawn_images_only
A8: cargo test -p deck-streak-api --test gallery_routes -- --exact an_image_is_served_to_the_owner_only
A9: cargo test -p deck-streak-api --test gallery_routes -- --exact an_unknown_image_is_image_unknown
A10: cargo test -p deck-streak-api --test gallery_routes -- --exact a_share_answers_the_prepared_id
A11: cargo test -p deck-streak-api --test gallery_routes -- --exact a_share_without_a_file_id_is_unavailable
A12: cargo test -p deck-streak-api --test gallery_routes -- --exact a_share_the_transport_refuses_is_unavailable
A13: cargo test -p deck-streak-api --test gallery_routes -- --exact shares_are_bounded_per_minute
A14: cargo test -p deck-streak-daemon --test roles -- --exact the_api_role_joins_the_bot_transport
A15: pnpm exec vitest run web/app/src/routes/gallery.test.ts -t "the gallery labels every image"
A16: pnpm exec vitest run web/app/src/routes/gallery.test.ts -t "the gallery shows its empty state"
A17: pnpm exec vitest run web/app/src/routes/gallery.test.ts -t "the share control needs a shareable image and 8.0"
A18: pnpm exec vitest run web/app/src/routes/gallery.test.ts -t "a tap shares the prepared message"
A19: pnpm exec vitest run web/app/src/routes/gallery.test.ts -t "the gallery honours reduced motion"
A20: cargo test -p deck-streak-progression --test share_cards -- --exact the_milestones_equal_the_golden
A21: cargo test -p deck-streak-coordination --test share -- --exact the_gallery_reads_drawn_images_only
A22: cargo test -p deck-streak-coordination --test share -- --exact a_share_without_a_file_id_makes_no_call
```

## 3a. What the box run judges

The private-wiring change that enforces B1 to B3 is the ux-laws pack reading the gallery screen: the
delivery hands back the JSON diff that adds `web/app/src/routes/gallery/` to its population. The
accessibility and telegram-platform packs are already enforced and widen their populations.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/app/src/routes/gallery/`: text alternatives, native controls, contrast in both schemes, reduced motion | the accessibility pack |
| B2 | over `web/app/src/routes/gallery/` and `web/app/src/lib/telegram.svelte.ts`: the share call is version-gated and the screen follows Telegram's theme | the telegram-platform pack |
| B3 | over `web/app/src/routes/gallery/`: one primary action per image, and a named state for empty, loading and refused | the ux-laws pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/share_cards.rs` | `deck-streak-progression` | added: the milestone set |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the module |
| `crates/progression/tests/share_cards.rs` | `deck-streak-progression` | added: A20 |
| `crates/coordination/src/recompute/share_cards.rs` | `deck-streak-coordination` | added: the phase 7 step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: registers the step (SPEC-071 adds it) |
| `crates/coordination/src/share.rs` | `deck-streak-coordination` | added: `prepare_share` and the gallery's reads |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules |
| `crates/coordination/tests/share_cards.rs` | `deck-streak-coordination` | added: A1 to A6 |
| `crates/coordination/tests/share.rs` | `deck-streak-coordination` | added: A21, A22 |
| `crates/api/src/gallery_routes.rs` | `deck-streak-api` | added: the three routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/gallery_routes.rs` | `deck-streak-api` | added: A7 to A13 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the bot transport joined to the api role's router |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the api role's router built with the transport |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A14 |
| `web/app/src/routes/gallery/+page.svelte` | miniapp | added: the gallery |
| `web/app/src/routes/gallery.test.ts` | miniapp | added: A15 to A19 |
| `web/app/src/lib/telegram.svelte.ts` | miniapp | changed: `shareMessage` behind the version check |
| `web/app/src/lib/api.ts` | miniapp | changed: the three calls |
| `web/app/src/lib/routes.ts` | miniapp | changed: the route |
| `web/app/messages/*.json` | miniapp | changed: the gallery's strings, in each locale's catalog |
| `tools/parity-oracle/registry/spec_136.py` | parity oracle | added: the two goldens' adapters |
| `tools/parity-oracle/goldens/share_card.constants.json` | parity oracle | added |
| `tools/parity-oracle/goldens/share_card_prompt.json` | parity oracle | added |
| `docs/specs/SPEC-136-a-streak-milestone-draws-a-share-card-once-the-gallery-shows-every-drawn-image-and-the-owner-shares-one-with-a-tap.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w7-image-pipeline-and-its-no-provider-path.md` | docs | added by the W7 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-136.md` | docs | added |
| `scripts/mutation-rows.d/S13600-S13699.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It chooses no image provider: the owner decides, and until then no card is drawn (#169).
- It publishes no image and needs no domain (#168, #156).
- It draws no card for a landmark or a level (#127, #128).
- It shares nothing to a story, which needs a public image link (#168).

## 6. Risks

- **A card for a length the predecessor never carded.** R1 and R2 against the golden; detected by A1
  and A3.
- **A card sent twice.** The key; detected by A2.
- **An image shown to someone else.** R6's owner gate and no public path; detected by A8.
- **A share control that fails silently.** R8 and R9 answer by name; detected by A11, A12 and A17.

## 7. Parity goldens

`tools/parity-oracle/registry/spec_136.py` generates, at `27ee2bc`:

- `share_card.constants`: `constants.SHARE_CARD_STREAKS`.
- `share_card_prompt`: the prompt, the art key, the notified reference and the caption
  `ghost_race.py:GhostRaceLayer._milestone_share_card` builds (the adapter applies §1's
  replacements), over a recording double, for the
  lengths 6, 7, 8, 30, 999 and 1000 (6, 8 and 999 build nothing; 7 and 1000 are the set's ends).

## 8. Tables and the v9 import

None: the cards are rows of SPEC-135's `agent_images`.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13601-SET` | `crates/progression/src/share_cards.rs` | the eight lengths | `share_cards::the_milestones_equal_the_golden` |
| `S13602-CHANGED-ONLY` | `crates/coordination/src/recompute/share_cards.rs` | only a changed length enqueues | `share_cards::an_unchanged_length_enqueues_nothing` |
| `S13603-LANGUAGE-ONLY` | `crates/coordination/src/recompute/share_cards.rs` | the language streak alone | `share_cards::the_law_streak_draws_no_card` |
| `S13604-MEMBERSHIP` | `crates/coordination/src/recompute/share_cards.rs` | the set's membership test | `share_cards::a_card_is_enqueued_exactly_at_the_milestones` |
| `S13605-OWNER-ONLY` | `crates/api/src/gallery_routes.rs` | the image route's `OwnerSession` | `gallery_routes::an_image_is_served_to_the_owner_only` |
| `S13606-NO-STORE` | `crates/api/src/gallery_routes.rs` | `private, no-store` | `gallery_routes::an_image_is_served_to_the_owner_only` |
| `S13607-STATES-LISTED` | `crates/coordination/src/share.rs` | `ready` and `sent` only | `share::the_gallery_reads_drawn_images_only` |
| `S13608-NO-FILE-ID` | `crates/coordination/src/share.rs` | a row with no file id is unavailable | `share::a_share_without_a_file_id_makes_no_call` |
| `S13609-SHARE-RATE` | `crates/api/src/gallery_routes.rs` | 10 a minute; the test names the tenth and the eleventh | `gallery_routes::shares_are_bounded_per_minute` |
