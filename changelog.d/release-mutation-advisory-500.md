### Changed

- On the release pull request, `dev` into `main`, the `ci` check reads every mutation job's result as advisory: it prints each one in its log and its step summary and passes when every other job succeeded, so a red mutation leg no longer holds a release whose changes were each judged on their own pull request into `dev`. `base-is-dev` names the release run by the rule the mutation verdict already uses, and every other run, each pull request into `dev` included, keeps today's rule (SPEC-500, ADR-518).
