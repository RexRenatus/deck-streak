### Added

- The engine core decides a full sync's choice in one rule: it offers only the directions the
  server's answer allows, counts what each would lose, and writes only after a backup that holds
  every replaced row, a found snapshot and a re-check before an upload, and a re-read of the device
  before a download.
