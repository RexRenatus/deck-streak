### Fixed

- The agent documents now name the CI jobs, gate stages and weekly dispatch scopes that the
  workflows and the gate hold. There are eight mutation jobs: `mutation-web-shard` runs
  StrykerJS, and `mutation-python-release` runs a release run's later Python legs. The gate has
  twelve stages, no job is named `packs`, and the weekly dispatch also takes `python`. A new guard
  test keeps the documents in step with those files.
