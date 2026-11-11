### Added

- A census at warn reads every tracked shell script, workflow `run:` text and Python file for a
  process waiter that can match its own command line: a `pgrep -f` or `pkill -f` whose pattern is in
  the command line that runs it, and a `ps` piped to a `grep` that matches its own line. The bracket
  form, a lookup by process name, `pidof` and a pid file read are not sites (SPEC-657).
