---
name: notifications-policy
description: >-
  Holds a product's notification policy as one checkable config, notifications-policy.json: the
  celebration ladder T0-T5, weekly budgets, the streak-break cap, dedupe, quiet-hours deferral, the
  failed-send hold, nudge budgets, the comeback cap, the holdout, the withhold ledger, and one
  router for the bot and the Mini App. Use when designing, changing or reviewing how an app
  celebrates, nudges or stays quiet, through scripts/notifications-policy-probe.py.
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/notifications-policy

v9's notification POLICY as ONE checkable config, and the check that holds a repository to it
(SPEC-V2-2219 / ADR-V2-2219). The policy covers:

- every celebration and every nudge: when it may speak, how loudly and how often;
- what it does in quiet hours, during a lapse, and when a send fails;
- the one router that decides for the bot and the Mini App alike, "or celebrations double".

The config is `notifications-policy.json` at the repository root (`phx.notifications.policy.v1`).
`baseline.json` in this pack holds v9's decided values. A repository may differ from the baseline
only by recording each difference with an ADR.

`scripts/notifications-policy-probe.py` is the check. It is standard-library Python and
vendorable, and it judges any tree through `--root`. Every row below runs it. Which seats consume
this pack is its catalog row's `consumes`, the one record of that edge (ADR-V2-1990), so this body
names none.

```
phxd pack probe --pack notifications-policy --root PATH --format json
```

## What this pack composes, and never copies

| practice | the pack that owns the check |
|---|---|
| the TEXT and metadata of a digest, a nudge or a comeback: no guilt, shame or loss framing, no dates or countdowns, numbers that match their stats | `packs/nudge-duties` |
| Telegram's own limits on the message a policy decision sends: length, markup, keyboard | `packs/telegram-platform` (its payload API) |
| deceptive and pressuring interface patterns in the product's screens | `packs/ux-laws` (its deceptive rows) |
| that a reminder has an opt-out, asks permission in context, and has some cap and some quiet time at all | `packs/ux-laws` (`notifications.opt-out`, `notifications.permission-in-context`, `notifications.frequency-cap`, `notifications.quiet-hours`) |
| the game's economy behind a celebration: XP, chests, coins, streak rules | `packs/game-economy` |

The policy decides WHETHER and HOW LOUDLY a message may go. nudge-duties judges WHAT it says. The
two meet at the envelope's metadata, the contract below. ux-laws advises that a cap and quiet hours
exist; this pack holds the decided values.

## The rows

Fifteen rows, all `tree`-scoped, one per class of `notifications-policy-probe.py`. Each runs
`python3 {skills}/../scripts/notifications-policy-probe.py --root {root} --baseline {skills}/packs/notifications-policy/baseline.json check <class>`
under a 60-second wall.

The `policy` stage: 13 rows. They read `notifications-policy.json`, and they are VOID when there is
none.

| row | severity | reason | refuses when |
|---|---|---|---|
| `policy-declared` | block | `policy-undeclared` | the policy is not JSON, names another schema, carries an unknown key, lacks a section, lists a surface other than bot and mini-app, declares no kinds, or a kind has an unknown key, a class other than celebration, nudge, digest or alert, or tiers outside T0-T5; or a kind other than an alert names no owner-facing setting |
| `policy-deviation-has-adr` | block | `deviation-without-adr` | a value differs from the baseline and no deviation records it; or a deviation cites an ADR that does not exist, lies outside the tree, or never names the key; or a deviation is stale |
| `ladder-tiers` | block | `ladder-malformed` | a tier T0-T5 is missing or its render is unknown or out of order; a rarity has no tier or rarities fall as they rise; the epic or legendary floor is below T2; an event's tier is not T0-T5; the reaction age is not positive; celebrations are not deduplicated once-ever |
| `celebration-budgets` | block | `budget-malformed` | the window is not a week; an intensity lacks non-negative T4 and T5 budgets, allows more T5 than T4, or allows less than a quieter intensity; the default intensity is undeclared; an over-budget tier does not drop to a lower tier; an exempt event is not a ladder event |
| `streak-break-cap` | block | `streak-break-uncapped` | a streak-break day is not capped at T1 or below, fanfare is not deferred, a deferred celebration is not re-capped at flush, or the near-miss rule allows gaps that are not real |
| `dedupe` | block | `dedupe-missing` | a kind declares no dedupe scope or an unknown one, or a celebration kind is not deduplicated once-ever |
| `quiet-hours` | block | `quiet-hours-violated` | the window is not a clock time; celebrations do not defer; nudges do not suppress; a class other than alert is exempt; the digest fires inside the window or before the rollover |
| `deferral-bounds` | block | `deferral-unbounded` | the deferral has no positive maximum age, flush count or queue bound; the queue is smaller than a flush; overflow is not collapsed into one rollup line; drops are not named |
| `send-failure-hold` | block | `send-failure-burns` | a failed send burns the celebration rather than holding it, retries or the outage cooldown are not positive, or a re-latched row does not keep its first timestamp |
| `nudge-budgets` | block | `nudge-budget-malformed` | a daily budget is not positive; an exempt consumer is not decided before the budget; a kind names an undeclared budget, or a budget is named by no kind; a lapse does not suppress nudges; a lapse escalates; the habit backoff is not positive |
| `comeback-cap` | block | `comeback-uncapped` | more than 3 comeback messages are allowed per lapse, the gap is not positive, the episode key is empty, the cap is not followed by silence, the landmark windows are negative, or the comeback kind is not a nudge on the comeback budget |
| `holdout` | block | `holdout-unsafe` | the holdout exceeds its ceiling or 50%, the draw is not a deterministic hash, the seed is not persisted, an eligible kind is not a nudge (alerts, celebrations and digests are never held), a held occasion is not recorded or spends budget, or the readout has no minimum sample |
| `withhold-ledger` | block | `withhold-unrecorded` | withheld occasions are not recorded, a withhold row can take the send's own key, a reason is listed twice, or a reason the policy withholds for is missing (quiet_hours, budget_spent, lapse, ablation_hold, nudges_disabled) |

The `router` stage: 1 row. It reads the tree's shipped source, with comments blanked and tests
left out.

| row | severity | reason | refuses when |
|---|---|---|---|
| `one-router` | block | `router-bypassed` | the router module does not exist or does not define its symbol; either surface, bot or mini-app, is undeclared or has no delivery call; or a delivery call named in `router.transport` is made anywhere but the router module |

The `messages` stage: 1 row. It reads committed envelopes, every `*.msg.json` carrying a `kind`,
test directories included.

| row | severity | reason | refuses when |
|---|---|---|---|
| `message-metadata` | block | `metadata-nonconforming` | an envelope's kind is undeclared; its tier is not one of the kind's; its `budget_key` does not name the kind's budget, or is set on a budget-exempt kind; a key is not an opaque token or carries a calendar date; a `dedupe_key` repeats; a comeback has no `lapse_id`; one lapse holds more comebacks than the cap |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`. The
script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when VOID: no policy, or
nothing examined. `message-metadata` over `--root` counts the files it walked, and over
`--subject DIR` it counts the envelopes, so a subject with none is VOID. The pack's card turns any
non-zero exit red.

## The config, section by section

Each value is v9's, carried unchanged, and the reason is recorded beside it. Change a value only
through a deviation with an ADR, as described in "Changing the policy".

- **`kinds`.** Every kind the product sends is declared, with its `class`, allowed `tiers`,
  `budget`, `dedupe` scope and the owner-facing `setting` that switches it off. An alert has no
  setting, and nothing silences it.
  - celebration: class celebration, tiers T0-T5, dedupe once-ever;
  - digest: class digest, tier T2, dedupe once per study day;
  - morning, streak_risk, last_chance and habit: class nudge, tier T2, dedupe per study day;
  - comeback: class nudge, tier T2, dedupe per lapse-episode day;
  - alert: class alert, tier T2, dedupe per incident.

  Every setting is shown on the Mini App's settings screen: people manage their notifications
  inside the app.
- **`ladder`.**
  - The tiers are T0 silent, T1 a reaction on the owner's last message, T2 a line, T3 a two-beat
    reveal, T4 dice, and T5 dice and a pin.
  - The rarities map common, rare, epic and legendary to T2, T3, T4 and T5, and epic and legendary
    never fall below T2.
  - Each event has a tier, and an unknown event is T2.
  - A T1 reaction is skipped when its target is more than 24 hours old.
  - A celebration fires once-ever per event key.
- **`celebration_budgets`.**
  - Weekly T4 and T5 budgets, by the owner's intensity: quiet 1 and 0, standard 2 and 1, loud 3
    and 2. Standard is the default.
  - Over budget, T5 drops to T4 and T4 drops to T3.
  - A CEFR band-up is exempt.
- **`streak_break` and `near_miss`.**
  - On a streak-break day everything caps at T1 with calm copy, and fanfare waits. A win on a
    losing day is a loss disguised as a win.
  - Near-miss copy only for a real gap: at most 5 units or 10%. Fabricated near-misses drive
    persistence the owner did not choose.
- **`quiet_hours`.**
  - The window is 23:00 to 07:30, wrapping midnight.
  - A celebration defers: it is never dropped and never fired inside the window.
  - A nudge is suppressed.
  - Only alerts ignore the window.
- **`digest`.** The digest fires at 09:05, outside the window and after the 04:00 rollover, so it
  reports a day that has closed.
- **`deferral`.**
  - A deferred celebration older than 720 minutes is abandoned, by name.
  - At most 2 flush in full when the window ends, and the rest collapse into one rollup line.
  - The queue holds at most 20.
  - A stored tier is re-capped at flush.
- **`send_failure`.**
  - A failed send holds the celebration; it does not burn it.
  - It retries up to 2 times, behind a 60-second outage breaker.
  - A re-latched row keeps its first deferral time, so the age cap stays reachable.
- **`nudge_budgets` and `lapse`.**
  - The evening budget is 2 a day, shared by streak_risk and last_chance.
  - A user-requested re-send is exempt, and the exemption is decided before the budget is read, so
    it never spends the budget.
  - After 3 zero-review days the owner is in lapse. Every nudge and stake is suppressed.
  - Nothing escalates: escalating on a lapsed person is associated with more abandonment, not less.
  - The habit check-in backs off to one send every 3 days after 5 lapse days.
- **`comeback`.**
  - At most 3 comeback messages per lapse episode, at least 3 days apart, keyed by the lapse id.
    Then silence for the rest of the episode.
  - The second message hangs on a fresh-start landmark: a Monday within 3 days, or a month start
    within 7.
  - The owner switches it off with the setting value `"0"`.
- **`holdout`.**
  - 10% of advisory nudge occasions (morning, streak_risk, last_chance) are held out, and 50% is
    the ceiling.
  - The arm is a deterministic hash of a persisted seed, the kind and the reference, so every past
    decision can be re-derived.
  - A held occasion is recorded and spends no budget.
  - The readout needs 10 occasions in each arm before it reports.
  - Alerts, celebrations and digests are never held.
- **`withhold`.**
  - Every occasion that was not sent is recorded with a reason, under the kind with `:withheld`
    appended. A withhold row can then never silence a later genuine send of the same key.
  - The reasons are lapse, skip_day, quiet_hours, budget_spent, below_streak_min, already_studied,
    ablation_hold, no_notifier, nudges_disabled, no_payload and already_recorded.
- **`surfaces` and `router`.**
  - ONE router decides for the bot and the Mini App.
  - When the Mini App is open, a celebration renders in the app. Otherwise it goes to the bot, as
    its tier.
  - Budgets and dedupe live on the server, in the router, so the two surfaces can never both
    celebrate one event.
  - `router.transport` names each surface's delivery calls, and only the router module calls them.

## The decision ledger

The router records every decision in the policy's ledger, one row per occasion. That is where the
holdout arm lives: a held or withheld occasion has a decision and no message. The record is
`phx.notifications.decision.v1`:

| field | meaning |
|---|---|
| `dedupe_key` | the occasion's opaque key, the same as its envelope's when one is sent |
| `kind` | a kind the policy declares |
| `surface` | `bot` or `mini-app` |
| `arm` | `send`, `hold` (the holdout), `defer` (quiet hours or a failed send) or `withhold` |
| `reason` | a withhold reason, or `quiet` / `send` for a deferral |
| `tier_requested`, `tier_rendered` | the tier asked for and the tier after budgets and caps |

A "never spoken" channel and one that "spoke N days ago" are different values. Neither renders as
zero.

## The envelope contract with nudge-duties

nudge-duties writes each outbound message as `phx.duty.message.v1`, `*.msg.json`: the literal Bot
API send payload under `send`, plus this metadata, which `message-metadata` reads.

- `duty`: the duty that wrote it, such as `daily-digest` or `comeback`.
- `kind`: a kind the policy declares. nudge-duties writes `digest` and `comeback`.
- `tier`: `T2` for both. T3-T5 are celebration renders.
- `budget_key`: the NAME of the kind's budget, such as `comeback`. It is absent on a budget-exempt
  kind such as the digest.
- `dedupe_key`: an opaque token, `[a-z0-9][a-z0-9:._-]*`, with no calendar date, unique per
  occasion.
- `lapse_id`: required on a comeback, and the same opaque shape. The policy caps comebacks per
  lapse id.
- `reading_id`: the comeback's one reading. nudge-duties checks it.

## The evidence the policy rests on

- **Frequency and urgency.**
  - Apple's guidelines say to avoid multiple notifications for the same thing, to represent
    urgency accurately, never to use a time-sensitive level for marketing, and to let people manage
    notifications in the app.
  - The policy's dedupe, its tiers and its per-kind settings follow these.
  - For a passive message, send with Telegram's `disable_notification`: a T2 line need not sound.
- **Batching.** Batching notifications a few times a day reduced stress and raised well-being in a
  randomized field trial (Fitz et al.). That is the quiet window and the flush.
- **Nagging.**
  - The FTC names nagging a dark pattern: asking repeatedly, or offering no way to permanently
    decline.
  - The EU Digital Services Act, Art. 25(3)(b), names repeatedly requesting a choice already made.
  - Hence: the comeback cap and its silence, the lapse suppression, no escalation, and a setting
    that stays off once the owner turns it off.
- **Honest celebration.** Losses disguised as wins (Dixon et al.) and near-misses (Clark et al.)
  drive persistence without progress. Hence the streak-break cap and the real-gap near-miss rule.
- **Streaks and fresh starts.**
  - A broken streak shown as broken lowers the next day's engagement (Silverman and Barasch). So
    the comeback invites without scolding, which is nudge-duties' check.
  - Temporal landmarks raise aspiration (Dai, Milkman and Riis). That is the comeback's landmark.
- **Measurement.** A nudge's effect is measured by randomizing each occasion, as micro-randomized
  trials do (Klasnja et al.; Bidargaddi et al.). That is the holdout, and the ledger that makes its
  readout auditable.

## Changing the policy

The baseline is v9's decided policy. To differ from it, the repository:

1. changes the value in `notifications-policy.json`;
2. writes an ADR that names the key, for example `quiet_hours.start`, says why, and names what it
   was chosen against;
3. adds `{"key": "quiet_hours.start", "adr": "docs/decisions/ADR-….md"}` to `deviations`.

A deviation key covers every value under it, so `kinds.reading_ready` records a whole new kind. A
deviation whose value no longer differs is stale and refused, so the record stays true. A change to
the baseline itself is a delivery of this pack, made on the owner's decision.

## How DeckStreak adopts this

1. **Declare the policy.** Copy `templates/notifications-policy.json` to the repository root. It is
   the baseline, plus DeckStreak's router: the `notifications` crate's `router.rs`, whose `route`
   is the one entry point, with the bot's `push_*` calls and the Mini App's `push_in_app` as its
   transport.
2. **Route everything.** Every celebration and nudge goes through `route`:
   - it reads the policy;
   - it checks the budgets and dedupe on the server;
   - it defers in quiet hours;
   - it draws the holdout arm;
   - it records the decision.

   The Mini App asks the server for its in-app celebrations and never decides one itself.
3. **Commit golden envelopes.** The engine's tests write every rendered message as `*.msg.json`
   under `tests/messages/`. `message-metadata` judges them against the policy, telegram-platform
   judges the payload, and nudge-duties judges the text.
4. **Run it.** From this repository:

   ```
   phxd pack probe --pack notifications-policy --root PATH --format json
   ```

   Or vendor `scripts/notifications-policy-probe.py` and `baseline.json`, and run each class with
   `--baseline`.
5. **Read what refuses.**
   - A value off the baseline with no ADR.
   - A celebration that fires in quiet hours.
   - A streak-break day louder than T1.
   - A comeback cap above 3.
   - A holdout on an alert.
   - A budget nobody names.
   - A `push_dice` call outside the router.
   - A comeback envelope without a lapse id.

## References

Primary sources; access dates are recorded in SPEC-V2-2219.

- Apple Human Interface Guidelines, Notifications and Managing notifications:
  https://developer.apple.com/design/human-interface-guidelines/notifications ·
  https://developer.apple.com/design/human-interface-guidelines/managing-notifications
- Android, notification runtime permission:
  https://developer.android.com/develop/ui/views/notifications/notification-permission
- FTC staff report, Bringing Dark Patterns to Light:
  https://www.ftc.gov/system/files/ftc_gov/pdf/P214800%20Dark%20Patterns%20Report%209.14.2022%20-%20FINAL.pdf
- Digital Services Act, Art. 25: https://eur-lex.europa.eu/eli/reg/2022/2065/oj · EDPB Guidelines
  03/2022: https://www.edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-032022-deceptive-design-patterns-social-media_en
- Fitz et al., Batching smartphone notifications can improve well-being: https://doi.org/10.1016/j.chb.2019.07.016
- Klasnja et al., Microrandomized trials: https://doi.org/10.1037/hea0000305 · Bidargaddi et al.,
  To prompt or not to prompt?: https://doi.org/10.2196/10123
- Silverman and Barasch, On or Off Track: https://doi.org/10.1093/jcr/ucac029 · Dai, Milkman and
  Riis, The Fresh Start Effect: https://doi.org/10.1287/mnsc.2014.1901
- Dixon et al., Losses disguised as wins: https://doi.org/10.1111/j.1360-0443.2010.03050.x · Clark
  et al., Gambling near-misses: https://doi.org/10.1016/j.neuron.2008.12.031
- Telegram, `disable_notification`: https://core.telegram.org/bots/api#sendmessage
