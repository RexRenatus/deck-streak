# Schematic: which committed messages the notification-policy check judges

Kind: data flow. Read at DeckStreak dev `02758d40b`. Decided by ADR-323; built by SPEC-323.

```mermaid
flowchart TD
  goldens[every *.msg.json in the tree] --> probe[message-metadata, notification-policy pack]
  policy[notifications-policy.json] -->|replies list| probe
  probe --> kind{carries a kind key?}
  kind -->|yes| judged[judged as a notification, whatever its duty]
  kind -->|no| duty{duty is in replies, and every policy class is clean?}
  duty -->|yes| skipped[skipped, counted as examined, named on one line]
  duty -->|no| finding[a finding: kind None is not a kind the policy declares]
  goldens --> payload[telegram-platform payload rows: length, markup, keyboard]
  goldens --> botmsgs[test_bot_messages.py: parses as the Bot API would]
  policy -->|read once at start| loader[Policy::parse]
  loader -->|replies names a declared kind| refuse[start refused: Malformed, key replies]
  loader -->|replies absent| missing[start refused: Missing, key replies]
  goldens --> test[A1: every golden is a notification or a declared reply]
  policy --> test
```

The payload rows and `test_bot_messages.py` read all 26 command replies whether or not the metadata
row skips them. At this head the metadata row judges 0 notification envelopes.
