### Added

- The sync screen lists the browser's backups of the collection and the server copies it counted,
  by kind and age, newest first, and exports one as a file the owner keeps (SPEC-377 part c2,
  ADR-388; refs #631). Only a backup the list names can leave the browser.
- The browser keeps the three newest backups of each kind and never removes the newest of a kind.
  It removes older ones only while no full-sync choice is held, and the screen names each one it
  removed.
- The sync screen says whether the browser keeps this site's storage, that it does not, or that it
  cannot tell, and that a collection the browser cleared is restored by a download the owner taps.
- Before an upload, the service is asked whether a sealed snapshot of the server's copy exists and
  how old the newest is. Until the service can list its archive, it answers that it does not know,
  and the upload stays refused.
