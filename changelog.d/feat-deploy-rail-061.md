### Added

- The private deploy rail's public contract (SPEC-061, ADR-061). `deploy/rail-contract.json` lists
  the neutral values the rail overrides, by unit and key, and names its one drop-in per unit and the
  credential socket. `deploy/scripts/credential-pairs.py` prints every (unit, credential id) pair
  the templates declare, a template named as the template it is, and refuses any credential that
  does not come from the socket; the rail refuses to install when its map differs from that list.
  `deploy/scripts/effective-check.py` reads `systemctl cat` and refuses a unit that still carries a
  neutral value, a credential not from the socket, a secret-named `Environment=` variable, or a
  drop-in other than the rail's own; its `--census` holds the contract equal to the neutral values
  the templates carry. `deploy/scripts/guards-check.py` refuses a pinned guard that is missing,
  changed, of another mode, or writable by anyone but root, and a manifest that names no file.
  Both checks read a unit file line by line as systemd reads it, measured with `systemd-analyze`,
  and refuse a construct systemd would read otherwise; the effective check also refuses a neutral
  value in another spelling, a calendar that fires at UTC's instants or names no zone, and the other
  routes a value can take into a unit (a secret-named `PassEnvironment=`, standard input written in
  the file or read from one, a second `EnvironmentFile=`).

### Changed

- The private rail's map answers the sync login to the `sync` job alone, the one job that reads it
  (SPEC-061 §8).

- ADR-061 is accepted: a unit's host values reach it as a drop-in beside the template installed
  byte for byte, and every timer's calendar, not only a job's, takes the deployment's zone.
