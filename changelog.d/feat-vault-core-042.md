### Added

- The vault adapter's core: every file it writes lands through a temporary name the sync bridge
  ignores, is synced, renamed over its target and its folder synced; passes the content rails the
  vault-duties pack reads from its `rails.json`, plus a rail against control characters; stays inside
  its folder after `..` and symbolic links; and never overwrites or deletes a note. The vault
  features refuse to start, creating nothing, when the readings folder is missing or not writable.
- The readings date tree: a reading's note in the predecessor's eight keys with `ai_generated: true`
  and two box lines; the roll-forward that carries a note into the next study day patching exactly
  three keys, proved byte for byte against the predecessor's golden, and archives the rest by year,
  month and ISO week; the Studied stamp and the owner's read tick, each on its own line and never
  unticked; and a body replacement that refuses a note the owner edited.
- Staged duty runs: an executor of three verbs only, create, update and move, that checks a run
  itself and applies it only when the vault-duties pack's blocking classes are green on it.
