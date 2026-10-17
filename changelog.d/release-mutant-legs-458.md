### Changed

- Every mutation-testing leg of every workflow fetches the locked dependency graph before its baseline, the scheduled Rust cache warm-up fetches it before it saves, and a pull request's Rust mutation leg fails its job when cargo-mutants leaves no whole report.
