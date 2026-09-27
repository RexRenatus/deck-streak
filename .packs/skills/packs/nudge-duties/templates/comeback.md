# The comeback: one reading per lapse, re-offered with an invitation

After two or more days without study, the daily readings pause and the lapse gets ONE comeback
reading. The comeback message offers it. The notifications-policy pack decides WHEN a comeback may
be sent (at most three per lapse, spaced apart, never in quiet hours, subject to the holdout); this
template decides WHAT it says.

## 1. The rules every variant keeps

- It invites and never scolds. It does not count the missed days, name a broken or lost streak,
  or say that anything is at risk. Highlighting a broken streak lowers the return (Silverman and
  Barasch), and naming a lapse as a total failure is how a lapse becomes a relapse (Marlatt and
  Gordon's abstinence violation effect).
- It tells the truth that helps: one missed day does not undo a habit (Lally and colleagues), so
  there is nothing to catch up on.
- It makes ONE small, concrete ask: the reading, and a few cards after it if the owner likes.
  A small if-then step is easier to start than a general intention (Gollwitzer).
- It offers, never commands: "if it suits you", "whenever you are ready"; never should, must or
  have to.
- It carries no date, deadline, clock time or countdown, and no urgency.
- It carries exactly ONE button, which opens the reading by url or web_app. There is no decline
  button, so no decline can be worded to shame.

## 2. Three variants

The engine picks the variant by how many comebacks this lapse has already sent: the first send uses
variant 1, the second variant 2, and any later send variant 3.

**Variant 1**

```
<b>Welcome back.</b>
A short reading is ready whenever you are: one topic, a few minutes, and nothing to catch up on.
If it suits you, three cards after it make a gentle start.
```

**Variant 2**

```
Your reading is still here for you.
It picks up exactly where your cards left off. Open it when you have a quiet moment.
```

**Variant 3**

```
One short reading, whenever you like.
There is no backlog to clear first: the reading and a few cards are a complete session on their own.
```

The button, on every variant:

```json
{"text": "Open the reading", "url": "<the reading's deep link>"}
```

## 3. The envelope

```json
{
  "schema": "phx.duty.message.v1",
  "duty": "comeback",
  "kind": "comeback",
  "tier": "T2",
  "budget_key": "comeback",
  "dedupe_key": "comeback:<opaque token for this send>",
  "lapse_id": "<opaque token for this lapse>",
  "reading_id": "<the lapse's one comeback reading>",
  "parts": [{"role": "invite", "text": "<the variant>"}],
  "send": {
    "text": "<the variant>",
    "parse_mode": "HTML",
    "reply_markup": {"inline_keyboard": [[{"text": "Open the reading", "url": "<deep link>"}]]}
  }
}
```

Every comeback of one lapse names the same `reading_id`. The deep link names the reading by an
opaque id, never by a date.
