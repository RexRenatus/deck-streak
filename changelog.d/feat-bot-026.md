### Added

- The Telegram bot, `deckstreakd bot` (SPEC-026, ADR-026): it long-polls the Bot API as the one
  poller of its token, removes any webhook, and drains the updates queued before it started instead
  of replaying them; each poll confirms the updates before its offset, and a failed poll waits the
  predecessor's backoff, 3 seconds doubling to a minute.
- The owner gate: a message is answered only when it comes from the owner in the owner's private
  chat, and a tap only when it is the owner's; anything else gets no reply, no answer and a log line
  that names its kind and one reason code. A text over 4096 characters is dropped, and a tap whose
  data is over 64 bytes is answered and not dispatched.
- One transport for everything the bot sends: HTML with every dynamic value escaped and link
  previews off, a long text split at a paragraph, a line or a word into chunks of at most 4096
  UTF-16 units that each parse on their own, a 429 waited out for its `retry_after` before the same
  request goes again, at most three attempts a chunk, and counts of the messages attempted and
  delivered.
- The owner's commands: `/start` says the coach is an AI and opens the Mini App; `/privacy` links
  the privacy policy; `/export` sends the owner's data as one JSON document; `/delete` erases only
  after the owner taps the latest question's button; `/sync` runs the owner's sync now and says what
  it did. The menu is shown in the owner's chat alone.
