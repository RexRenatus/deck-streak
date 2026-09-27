### Security

- Dependabot's automated security fixes stay off, because they would open pull requests against
  `main`, which only a release pull request from `dev` may change. Its alerts stay on, and each
  fixable alert becomes a pull request into `dev` (ADR-036).
