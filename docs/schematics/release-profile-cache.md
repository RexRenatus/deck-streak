# The release job's cache, and the two keys with one writer each

Kind: component and data flow. Read at the base of this pull request (`dev` when it was cut;
`.github/workflows/ci.yml:81-92` the `rust` job's clean and save, `:156-209` the `release` job).
The cache of SPEC-631 (ADR-654): which job writes which key, which jobs restore it, and the clean
before each save.

## Component and data flow

```mermaid
flowchart LR
  subgraph writers["writers (a push to dev or main that missed its key)"]
    rustjob["rust job<br/>cargo clean --workspace<br/>then save"]
    warmer["rust-cache.yml<br/>scheduled warmer<br/>cargo clean --workspace, then save"]
    relsave["release job<br/>cargo clean --workspace --release<br/>then save"]
  end
  subgraph keys["cache keys"]
    debugkey["debug key<br/>rust-OS-toolchain-lockfile<br/>holds target/debug"]
    relkey["release key<br/>rust-release-OS-toolchain-lockfile<br/>holds target/release"]
  end
  subgraph readers["restorers (every run)"]
    dbgreaders["rust, engine, hygiene,<br/>web-engine, mutation legs,<br/>mutation-weekly.yml"]
    relreader["release job<br/>restore, test-release stage,<br/>df -h / before and after"]
  end
  rustjob --> debugkey
  warmer --> debugkey
  relsave --> relkey
  debugkey --> dbgreaders
  relkey --> relreader
  relreader --> relsave
```

## What each part holds

- The debug key has two writers, the `rust` job and the scheduled warmer; its fallback prefix is
  `rust-<os>-<toolchain hash>-`, which is no prefix of the release key.
- The release key has one writer, the `release` job. Its fallback prefix is
  `rust-release-<os>-<toolchain hash>-`, which matches no debug key.
- Each writer cleans before it saves: `cargo clean --workspace` leaves the dependencies' debug
  artifacts, `cargo clean --workspace --release` leaves the dependencies' release artifacts, so the
  workspace's own crates, rebuilt from any fresh checkout, are not cached.
- The save runs only on a push to `dev` or `main` whose restore missed its exact key. A pull request
  restores and never saves, so it misses the release key until a saving push has written it.
- The `release` job prints `df -h /` before its restore and after its stage.
