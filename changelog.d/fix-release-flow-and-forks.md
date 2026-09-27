### Fixed

- A second release can no longer deadlock. `main` does not require an up-to-date head, because only
  this repository's `dev` reaches it, and nothing is merged back into `dev` (ADR-034).

### Security

- `ci` and `fragment` count only when GitHub Actions produced them.
- `base-is-dev` refuses a fork's branch named `dev`.
- The contributor and agent documents record the maintainer's fork rule (ADR-035).
