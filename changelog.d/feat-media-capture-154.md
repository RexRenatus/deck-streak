### Added

- A photo, voice note or document the owner sends the bot lands in the vault inbox once, as the
  predecessor saved it: the last photo size Telegram lists, a document's plain extension of 1 to
  10 characters or `.bin`, nothing declared over 20 MB fetched, and a download that streams past
  the cap discarded. The bot answers with what became of the capture. Only the owner's media is admitted,
  and the file's URL never reaches a log line.

### Fixed

- A staged duty run now guards each folder, move and note against the journal folders when it is
  applied: an operation that would reach a journal folder through a link the vault gained after
  the run was checked stops the run there, and the operations before it stay applied.
- With no vault configured, the daemon's log says no capture is saved, which holds for the bot's
  media as well as the Mini App's quick capture.
