# SPEC-130: one settings screen shows and sets every stored runtime setting, and names the rest

- **Wave:** W7. **Issue:** #57 (the Mini App's settings screen for every runtime setting) (epic
  #8). **Context(s):** `deck-streak-coordination` (the settings census and its write route);
  `deck-streak-notifications` (its settings' declarations and setter); `deck-streak-quests` (the
  chest settings' declarations and setter); `deck-streak-ingest` (the skip day's declaration);
  `deck-streak-discipline` (the rail's scope and switch, declared and set); `deck-streak-api` (the
  settings routes); `deck-streak-daemon` (the wiring); the Mini App (`web/app`, the screen).
- **Decided by:** ADR-130 (this SPEC's: the screen writes stored runtime settings only, through each
  owning context's setter, and configuration read at start stays configuration), ADR-087 and
  ADR-096 (the owner's private files are configuration), ADR-024 (the owner's session), ADR-041 (the
  router's policy declares each kind's setting).
- **Prerequisites:** SPEC-021 (data rights and the default-settings row), SPEC-024 (the owner's
  session and the CSRF bound), SPEC-028 (the Mini App shell), SPEC-041 (`notification_settings`),
  SPEC-081 (`chest_settings`), SPEC-084 (the intensity), SPEC-100 (the nudge switches and the
  holdout), SPEC-102 (the widget's switches), SPEC-104 (the rail's scope and switch), SPEC-106 (the
  discipline notices' switch), SPEC-109 (the skip day's switch and setter) and SPEC-117 (the last
  vault pass). SPEC-081, SPEC-084, SPEC-100, SPEC-102, SPEC-104, SPEC-106, SPEC-109 and SPEC-117 are
  unlanded. **Mutation band:** `S13000-S13099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-130.md` (ADR-016).

## 1. The problem, measured

- **Twenty-nine documents defer a setting to this screen.** `git grep -n '#57'` over dev's SPECs and
  ADRs and W5's plan (PR #343, read by `git show`) finds them. §2a is the census: each deferred
  setting, the document that deferred it, and where it lands.
- **Two kinds of setting exist.** A stored runtime setting is a row a context owns, whose writer
  bumps the kernel's settings generation (the LEXICON's rule; SPEC-109 R2 is the worked example).
  Configuration is read at start: environment settings, the owner's private files (ADR-087,
  ADR-096) and credentials. The first can change while the service runs; the second cannot.
- **No surface writes a stored setting.** `notification_settings` (SPEC-041) has a reader and no
  writer on dev; SPEC-109 builds the skip day's setter and says that no surface calls it until this
  screen; SPEC-081 and SPEC-104 keep the chests' and the rail's settings at their defaults for want
  of it.
- **No default settings file exists.** SPEC-021 R9 left the privacy pack's default-settings row
  deferred until this screen, and `privacy.json` has no `config` entry.
- **The shell stores nothing on the device** (SPEC-028), and this screen keeps that.

## 2. Requirements

The census

R1. Each context that owns a stored runtime setting declares it once, beside its setter, as a
    `SettingDecl` (the kernel's type: key, section, shape (a switch, a choice from a closed set, an
    integer range, or a minute of the day), and default). The contexts and their settings are:
    - notifications (`notification_settings`): every kind's setting that `notifications-policy.json`
      names, the discipline notices' switch (SPEC-106), the nudge switches SPEC-100 seeds, the
      widget's two switches (SPEC-102), the quiet window's start and end minutes
      (`quiet_start_min`, `quiet_end_min`, SPEC-041 R9), the celebration intensity (a choice from
      the policy's `celebration_budgets.intensities`, SPEC-084 R2) and the holdout's percentage
      (0 to 50, SPEC-100);
    - quests (`chest_settings`, SPEC-081): the day's chest cap, 0 to 3 (0 turns chests off; the
      economy's faucet bound keeps the predecessor's default as the ceiling), and the vault hour,
      0 to 23;
    - ingest (`skip_settings`, SPEC-109): the skip day's switch;
    - discipline (`tripwire_state`, SPEC-104): the rail's scope and its switch.
R2. `coordination::settings_census` gathers every declaration and answers, for each, its value now
    and its default. A key declared twice, by one context or two, refuses the census at start by
    name.
R3. A write names a key and a value. The census routes it to the owning context's setter, which
    checks the value against the declaration and writes it in the kernel's `BEGIN IMMEDIATE`
    write, bumping the settings generation in that same write (`Db::bump_settings_generation`). A
    value equal to the stored one writes nothing and bumps nothing. An undeclared key is refused
    `unknown_setting`, and a value outside its shape `value_invalid`; neither writes. The census
    mints both reasons: a setter answers the kernel's typed refusal, and the census names it.
R4. Every notifications write checks the key against the policy's declared settings, so a key the
    policy does not name is never written into `notification_settings`.

The defaults

R5. `settings.defaults.json` at the repository root holds every declared setting's default, and a
    test holds it equal to the declarations. `privacy.json` gains `"config":
    ["settings.defaults.json"]`. No setting under a sharing, public or visibility key is on by
    default (SPEC-137's publishing switch is off).

The screen

R6. `GET /api/settings` answers the census in section order; `PUT /api/settings/{key}` takes
    `{"value": ...}` and answers the stored value or R3's refusal with 422. Both sit behind SPEC-024
    R7's `OwnerSession`, and the write behind R9's CSRF bound.
R7. The Mini App's `/settings` route shows one section per context in R1's order, then the last
    vault pass (SPEC-117 R10, read-only), then the sections later SPECs add (SPEC-131, SPEC-137,
    SPEC-138). A switch is a native checkbox with its label; a choice a native radio group; a
    range or a minute a native input with its bounds. A write shows the stored value the route
    answered, never an optimistic one.
R8. The screen stores nothing on the device (no CloudStorage, DeviceStorage or SecureStorage),
    renders on the client, honours reduced motion, reads in both Telegram colour schemes, and is in
    the accessibility audit's routes (SPEC-028).
R9. Telegram's settings button opens `/settings` (the Mini App's `SettingsButton`, shown on start).
R10. The discipline engine's switch and hard mode keep their own card (SPEC-105); the screen links to
     it and writes neither.

What stays configuration (ADR-130)

R11. The screen writes nothing read at start, and shows none of it: the owner's private files, the
     environment's caps and switches, and credentials. §2a names each with its reason.
R12. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no
     unbounded faucet or notification volume (R1's bounds) and no dishonest copy (R7 shows what was
     stored).

## 2a. The census of deferred settings

| setting | deferred by | where it lands |
|---|---|---|
| each kind's switch; the quiet window | SPEC-041, SPEC-084 | notifications section |
| the celebration intensity | SPEC-084 | notifications section |
| the readings' switch (`reading_ready_enabled`) | SPEC-051 | notifications section |
| the focus nudge's switch | SPEC-079 | notifications section (`focus_enabled`, SPEC-100's seed) |
| the nudge switches and the holdout | SPEC-100 | notifications section |
| `widget_enabled`, `widget_mood`, the celebrations' switch | SPEC-102 | notifications section |
| the discipline notices' switch | SPEC-106 | notifications section |
| the day's chest cap and the vault hour | SPEC-081 | quests section |
| the skip day's switch | SPEC-109 | ingest section |
| the rail's scope and switch | SPEC-104 | discipline section; the rail's source waits for its binding (#170) |
| the discipline engine's switch and hard mode | SPEC-105 | their own card, linked (R10) |
| the last vault pass | SPEC-117 | a read-only row (R7) |
| the default settings file | SPEC-021 | `settings.defaults.json` (R5) |
| nothing stored on the device | SPEC-028 | kept (R8) |
| the courses and the focus subjects | ADR-087, SPEC-071, SPEC-079, SPEC-092 | not on the screen: the owner's private configuration file (ADR-130) |
| the note conventions | ADR-096, SPEC-094, SPEC-096, SPEC-099 | not on the screen: the owner's private configuration file (ADR-130) |
| the leech threshold and its scope | SPEC-071, SPEC-093, SPEC-098 | not on the screen: configuration read at start |
| a drill setting; the drill cadence, archive age and grade cap | SPEC-110, SPEC-111 | not on the screen: configuration read at start |
| the remedies cap; the tutor's caps and turn window; the practice set's time multiplier and XP; the coaching; the note formats | SPEC-112, SPEC-113, SPEC-114, SPEC-115, SPEC-116 | not on the screen: a duty's caps are configuration (SPEC-043 R6) |
| the drill flag and the tokens | SPEC-119 | not on the screen: the flag is configuration and the tokens are credentials |
| a drill type's time target | SPEC-121 | not on the screen: no table holds one, and where it is read from is the owner's question (#335) |
| a drill's worth | ADR-111 | not on the screen: no such setting exists; ADR-111 names it as what would make it wrong |
| a chart setting | SPEC-085 | not on the screen: none exists |
| the desired retention | SPEC-091 | not on the screen: it is the collection's own, changed in Anki |
| the reintroduction seed | SPEC-080 | not on the screen: minted once, and a change would re-deal draws already made |
| an edited measurement | SPEC-092 | not on the screen: it reads the courses file |

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the census holds every declaration of R1, each kind's setting the policy names among them, and no key twice | `the_census_holds_every_declared_setting_once` |
| A2 | a key declared twice refuses the census by name | `a_key_declared_twice_refuses_the_census` |
| A3 | a write reaches the owning context's setter, and a changed value bumps the settings generation once | `a_write_reaches_its_owner_and_bumps_the_generation_once` |
| A4 | a write of the stored value writes nothing and bumps nothing | `an_unchanged_write_bumps_nothing` |
| A5 | an undeclared key and a value outside its shape are refused by name and write nothing | `an_unknown_key_or_invalid_value_is_refused` |
| A6 | the notifications setter refuses a key the policy does not declare | `the_notifications_setter_refuses_an_undeclared_key` |
| A7 | the chest cap is refused above 3 and below 0, and the vault hour outside 0 to 23 | `the_chest_settings_are_bounded` |
| A8 | the rail's scope and switch are set through discipline's setter with the bump | `the_rails_scope_and_switch_are_set_with_the_bump` |
| A9 | `settings.defaults.json` equals every declared default, and `privacy.json` names it | `test_the_defaults_file_equals_the_declarations` |
| A10 | the settings routes answer 401 without the owner's session and 403 across sites | `the_settings_routes_are_owner_only` |
| A11 | a write answers the stored value, and a refusal answers 422 with its reason | `a_settings_write_answers_the_stored_value` |
| A12 | the screen renders one control per census row, and shows the value the route answered | `the settings screen renders every census row` |
| A13 | the screen touches no device storage | `the settings screen stores nothing on the device` |
| A14 | the accessibility audit covers `/settings` in both colour schemes | `the accessibility audit covers every route in both colour schemes` |
| A15 | the settings button opens `/settings` | `the settings button opens the settings screen` |

```acceptance
A1: cargo test -p deck-streak-coordination --test settings_census -- --exact the_census_holds_every_declared_setting_once
A2: cargo test -p deck-streak-coordination --test settings_census -- --exact a_key_declared_twice_refuses_the_census
A3: cargo test -p deck-streak-coordination --test settings_census -- --exact a_write_reaches_its_owner_and_bumps_the_generation_once
A4: cargo test -p deck-streak-coordination --test settings_census -- --exact an_unchanged_write_bumps_nothing
A5: cargo test -p deck-streak-coordination --test settings_census -- --exact an_unknown_key_or_invalid_value_is_refused
A6: cargo test -p deck-streak-notifications --test settings -- --exact the_notifications_setter_refuses_an_undeclared_key
A7: cargo test -p deck-streak-quests --test chest_settings -- --exact the_chest_settings_are_bounded
A8: cargo test -p deck-streak-discipline --test rail_settings -- --exact the_rails_scope_and_switch_are_set_with_the_bump
A9: python3 -m unittest discover -s scripts/tests -p test_settings_defaults.py -k test_the_defaults_file_equals_the_declarations
A10: cargo test -p deck-streak-api --test settings_routes -- --exact the_settings_routes_are_owner_only
A11: cargo test -p deck-streak-api --test settings_routes -- --exact a_settings_write_answers_the_stored_value
A12: pnpm exec vitest run web/app/src/routes/settings.test.ts -t "the settings screen renders every census row"
A13: pnpm exec vitest run web/app/src/routes/settings.test.ts -t "the settings screen stores nothing on the device"
A14: pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts -t "the accessibility audit covers every route in both colour schemes"
A15: pnpm exec vitest run web/app/src/lib/telegram.test.ts -t "the settings button opens the settings screen"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree. They have no
line in the acceptance fence, because no public test can run a pack's row. The private-wiring change
that enforces B2 is the privacy pack's default-settings row, deferred on #57 since SPEC-021 R9: this
delivery lifts that deferral, and hands the change back as its JSON diff.

| id | criterion | decided by |
|---|---|---|
| B1 | over `notifications-policy.json` (its nine kinds): every kind but the alert names an owner-facing setting, and each is on the screen | the notifications-policy pack |
| B2 | over `settings.defaults.json` (every declared setting) and `privacy.json`: the default file is named, and no sharing, public or visibility setting is on by default | the privacy-gdpr pack |
| B3 | over `web/app/src/routes/settings/` (the route's components): labels in names, native controls, contrast pairs and reduced motion | the accessibility pack |
| B4 | over `web/app/src/lib/telegram.svelte.ts`: the settings button is shown only where the client's version has it | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/settings_decl.rs` | `deck-streak-kernel` | added: `SettingDecl`, its shapes and its check |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the module |
| `crates/coordination/src/settings_census.rs` | `deck-streak-coordination` | added: the census, its write route and its refusals |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/settings_census.rs` | `deck-streak-coordination` | added: A1 to A5 |
| `crates/notifications/src/settings.rs` | `deck-streak-notifications` | added: the declarations from the policy and the setter |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the module |
| `crates/notifications/tests/settings.rs` | `deck-streak-notifications` | added: A6 |
| `crates/quests/src/chest_store.rs` | `deck-streak-quests` | changed: the chest settings' declarations and setter |
| `crates/quests/tests/chest_settings.rs` | `deck-streak-quests` | added: A7 |
| `crates/ingest/src/skip.rs` | `deck-streak-ingest` | changed: the skip day's declaration beside its setter |
| `crates/discipline/src/rail.rs` | `deck-streak-discipline` | changed: the scope's and the switch's declarations and setter |
| `crates/discipline/tests/rail_settings.rs` | `deck-streak-discipline` | added: A8 |
| `crates/api/src/settings_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/settings_routes.rs` | `deck-streak-api` | added: A10, A11 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role builds the census over each context's settings port |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: each context's settings port joined to the census |
| `web/app/src/routes/settings/+page.svelte` | miniapp | added: the screen |
| `web/app/src/routes/settings/+page.ts` | miniapp | added: the census load |
| `web/app/src/lib/settings/SettingControl.svelte` | miniapp | added: one row's native control |
| `web/app/src/routes/settings.test.ts` | miniapp | added: A12, A13 |
| `web/app/src/lib/a11y-coverage.test.ts` | miniapp | changed: A14's routes gain `/settings` |
| `web/app/src/lib/telegram.svelte.ts` | miniapp | changed: the settings button |
| `web/app/src/lib/telegram.test.ts` | miniapp | changed: A15 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the route |
| `web/app/messages/*.json` | miniapp | changed: the screen's strings, in each locale's catalog |
| `settings.defaults.json` | repo | added: every declared default |
| `privacy.json` | repo | changed: `config` names the defaults file |
| `scripts/tests/test_settings_defaults.py` | repo | added: A9 |
| `docs/specs/SPEC-130-one-settings-screen-shows-and-sets-every-stored-runtime-setting-and-names-the-rest.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-130.md` | docs | added |
| `scripts/mutation-rows.d/S13000-S13099.json` | repo | added: §9's rows |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It moves no configuration into a table and writes no private file: the courses and conventions
  files stay the owner's (ADR-130, #57).
- It binds no source for the rail; the owner binds one (#170).
- It sets no drill type's time target, which waits for the owner's answer (#335).
- It re-rolls no quest seed (#100).
- It adds no bot command that changes a setting; settings change on this screen only (#19).
- It builds none of the later sections: linked sign-in (#58), publishing (#156) and prices (#281)
  add their own.

## 6. Risks

- **A setting that changes and nothing notices.** R3's bump in the same write; detected by A3 and
  A4.
- **A key written that nothing reads.** R3 and R4 refuse an undeclared key; detected by A5 and A6.
- **A default that drifts from its seed.** R5's equality; detected by A9.
- **A switch turned on before the cutover sends a nudge twice.** The switch is the owner's; the
  cutover decides when the seeded switches turn on (#62).

## 7. Parity goldens

None. The screen is new: the predecessor kept these values as scattered runtime settings with no
screen, and each context's own SPEC holds the goldens of what its setting controls.

## 8. Tables and the v9 import

This SPEC adds no table. The settings it writes live in the tables of their own SPECs, and W8's
import maps the predecessor's runtime settings into them (#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13001-BUMP-ON-CHANGE` | `crates/coordination/src/settings_census.rs` | a changed value bumps the generation | `settings_census::a_write_reaches_its_owner_and_bumps_the_generation_once` |
| `S13002-NO-BUMP-UNCHANGED` | `crates/coordination/src/settings_census.rs` | an unchanged value bumps nothing | `settings_census::an_unchanged_write_bumps_nothing` |
| `S13003-UNKNOWN-REFUSED` | `crates/coordination/src/settings_census.rs` | an undeclared key is refused | `settings_census::an_unknown_key_or_invalid_value_is_refused` |
| `S13004-DUPLICATE-REFUSED` | `crates/coordination/src/settings_census.rs` | a key declared twice refuses start | `settings_census::a_key_declared_twice_refuses_the_census` |
| `S13005-SHAPE-CHECKED` | `crates/kernel/src/settings_decl.rs` | a range's upper bound is inclusive; the test names 3 and 4 for the chest cap | `chest_settings::the_chest_settings_are_bounded` |
| `S13006-POLICY-KEYS` | `crates/notifications/src/settings.rs` | only a policy-declared key is written | `settings::the_notifications_setter_refuses_an_undeclared_key` |
| `S13007-OWNER-ONLY` | `crates/api/src/settings_routes.rs` | the routes take `OwnerSession` | `settings_routes::the_settings_routes_are_owner_only` |
