### Fixed

- The skip write's take tests no longer depend on the other client's change and the server's stamp falling in
  different whole seconds (#918). The other client's change in `crates/ingest/tests/support/synthetic.rs` now
  moves its collection's stamp to one past the stamp it found, so it can never equal the server's, and
  `support::sync_on_another_client` refuses by name a sync that left that stamp where it was. Only the two test
  helpers change: no assertion of either take test moves, and no test is skipped, filtered or retried.
