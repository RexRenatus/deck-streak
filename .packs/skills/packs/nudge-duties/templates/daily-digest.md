# The daily digest: layout, envelope and the degraded form

The daily digest is ONE Telegram message a study day: deterministic numbers first, then a short
coaching paragraph the agent writes. The engine renders it, stores it as a `*.msg.json` envelope
(`phx.duty.message.v1`), runs this pack's rows over it, and sends `send` only when every blocking
row is green.

## 1. The stats part (deterministic; the engine writes it, never the model)

One line per stat, in the form `<label>: <number>`. The label holds no digit, and the number is the
FIRST number after the label on that line. Every number in this part must be a declared stat.

```
<b>Your study day</b>
Reviews: <today.reviews>
New cards: <today.new>
Accuracy: <today.accuracy_pct>%
Study time: <today.minutes> min
Streak: <streak_days> days
XP: <xp>
Level: <level>
```

The stats input is the JSON the stats step writes. Its values are display-ready: if the digest
shows `92.5%`, the input holds `92.5`, never `0.925`. A derived value is computed by the stats
step, never by the model. Numbers are written with `.` as the decimal mark, and `,` only between
groups of three digits.

A near-miss line such as `Next chest: <n> reviews away` belongs here and only here, because only
here is the gap a fact the input backs.

## 2. The coaching part (the model writes it)

Two to four sentences. The prompt carries `coaching-rules.md`, the stats input as data, and
nothing from the journal. The coaching may quote a number only if the stats input holds it.

## 3. The degraded form

When the coaching step fails closed (the model route is unreachable, a turn or time bound is hit,
or the output fails a blocking row), the ENGINE sends the digest anyway, with `coaching` set to
`unavailable`, the coaching part removed, and a notice part in plain view. The notice is written by
the engine from this sentence, never by the failed step:

```
Coaching was unavailable today, so this digest carries your numbers only.
```

The notice never names the cause in technical terms: no host, port, status code, error body or
exception name. The cause goes to the logs, not to Telegram.

## 4. During a lapse

While a lapse is open (the envelope carries `lapse_id`), the digest carries no `readings` part:
the daily readings pause, and the comeback offers the one reading of the lapse.

## 5. The envelope

```json
{
  "schema": "phx.duty.message.v1",
  "duty": "daily-digest",
  "kind": "digest",
  "tier": "T2",
  "dedupe_key": "digest:<opaque study-day token>",
  "coaching": "ok",
  "stats_source": "digest.stats.json",
  "stats": [{"key": "today.reviews", "label": "Reviews"}],
  "parts": [
    {"role": "stats", "text": "<the stats part>"},
    {"role": "coaching", "text": "<the coaching part>"}
  ],
  "send": {"text": "<the parts joined by a blank line>", "parse_mode": "HTML"}
}
```

The digest carries no `budget_key`: the notifications-policy pack exempts it from the budgets,
because it is sent once a study day and deduplicated by `dedupe_key`. The dedupe token is opaque:
it names the study day without spelling a calendar date.
