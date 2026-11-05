### Fixed

- The card view's probe suite now waits on each page ask under one bound, and an ask that gets no
  answer within it fails its test by name and records the probe it stopped on, so a page that stops
  answering no longer holds the Apple job until it is cancelled. Every card, wait and assertion is
  unchanged.
