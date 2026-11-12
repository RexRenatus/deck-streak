### Security

- A scheduled workflow audits `dev` again once a day, and on dispatch: it runs the `audit-rust` and
  `audit-web` stages over `dev` with a read-only token, so an advisory published after the last push
  reads red on a run of its own before a push or a pull request meets it. It runs on no pull request
  and no push, adds no required check, ignores no advisory and keeps each job's stage logs.
