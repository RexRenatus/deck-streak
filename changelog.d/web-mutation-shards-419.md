### Changed

- A pull request's mutation sweep of the Mini App runs in three shards. Each shard asks
  StrykerJS's own mutation server how many mutants each planned file holds, takes its share of the
  files dealt whole by that count, largest first onto the lightest shard, and keeps the leg's time
  limit and settings. The `mutation-web` check reads every shard's record and report by name,
  reads VOID when one is missing, cut, partial, foreign or misdealt, and judges their union with
  the unchanged thresholds (SPEC-419).
