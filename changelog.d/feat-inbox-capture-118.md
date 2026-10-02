### Added

- A capture lands in the vault's inbox once. The vault writes a capture's attachment and its
  same-stem `.md` stub as `<UTC date>-<kind>-<unique>`, never creates a missing inbox, never
  overwrites an earlier capture and never writes into a journal folder; a capture sent again under
  the same key is written once.
- The Mini App gains a quick capture: a text or a journal line, sent with its own retry key, is
  written by `POST /api/inbox/captures` as an inbox stub marked `source: miniapp`, and a capture id
  or a kind that is not its own form is answered 422 with its reason named. A journal line lands
  in the inbox, never in a journal section.
- The owner's layout file, named by `DECKSTREAK_VAULT_LAYOUT`, names the inbox and the journal
  folders. Unset, the vendored layout is in force; a file that cannot be read or is not a layout
  serves no capture and never refuses the start.
