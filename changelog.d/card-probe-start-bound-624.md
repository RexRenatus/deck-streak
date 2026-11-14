### Fixed

- The card view's probe suite now waits on its listeners, its lookup witness, its rule list
  compiles and its planted cookie under the same bound as a page ask, so a listener that never
  gets ready fails its test by name and records the probe it stopped on, instead of holding the
  Apple job until it is cancelled. Every card, timing window and assertion is unchanged.
