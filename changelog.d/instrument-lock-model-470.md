### Fixed

- At most one instrument reads at a time across the job, api and bot processes again. An
  instrument run now hands its lock and its whole run to a task of its own, which holds the lock
  until the read ends and the report is stored, so a request the api's time limit ends, a client
  that goes away or a stopping process no longer frees the lock while the read runs on, and the
  weekly step can no longer read beside it. A run that panics still releases the lock, and its
  caller sees the panic. A TLA+ model of the lock across the three roles checks the property and a
  due weekly run's eventual turn (SPEC-470, ADR-487, #476).
