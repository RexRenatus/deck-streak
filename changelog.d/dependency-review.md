### Security

- Every pull request into `dev` now runs GitHub's dependency review, which fails on a dependency
  the pull request adds or changes that carries a known advisory of any severity, in any scope, or
  a licence outside the list `deny.toml` allows, with no advisory allowed and no package exempt.
