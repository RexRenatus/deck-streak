### Added

- Bury and flag on iPhone and iPad (SPEC-358, part a, #633): the review gains Bury and Flag beside
  Replay, each acting on the shown card alone, with the bar's buttons held while the call runs. Bury
  shows the next card; Flag turns the card red or clears it, shown as an icon with a text label. The
  flag and bury rules move from the web engine into the shared core, so both clients run one copy,
  and the native adapter gains its bury and flag calls. The client's codec carries the queued
  card's flag, pinned by literal bytes. The Apple job's `harness` job gains the bury and flag
  tests on both simulators. ADR-369 records the choices.
