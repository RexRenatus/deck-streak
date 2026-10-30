The owner rules on the internal TestFlight lane's environment, in the owner's own words: "Sign an amendment (Recommended)" (owner, 2026-10-10, answering whether the recorded decision that names dispatches only is amended to the lane's trigger, #772): the internal environment admits deployments from the branch `dev` alone, for a push to `dev` or a dispatch on `dev`, and has no required reviewer.

# OWNER RULING 2026-10-10: the internal lane's environment admits a push or a dispatch on `dev`

It amends one line of `docs/rulings/OWNER-RULING-2026-10-05-testflight-lane-secrets.md`: the internal environment's
bullet under "Where the credential lives" (its lines 22-23). Everything else that ruling says stands, and its file keeps
the text it was signed with.

## What was held

- The 2026-10-05 ruling says "the internal environment admits dispatches on the branch `dev` only, and has no required
  reviewer, because the dispatch is already the ask (ADR-344)". It was written before the lane had a push trigger.
- #694 widened the lane's trigger to a push to `dev` that changes one of the app's inputs
  (`.github/workflows/testflight-internal.yml:4`, `:12`). Under the recorded line, the lane's ordinary run is one the
  record does not name.
- The environment's deployment rule matches the ref, not the event. No setting can admit a dispatch on `dev` and refuse
  a push to `dev`, so the dispatch-only wording could not be held by the setting either.

## What replaces it

- The internal environment admits deployments from the branch `dev` alone, for a push to `dev` or a dispatch on `dev`,
  and has no required reviewer, because a push run or a dispatch is itself the ask. Its deployment rule selects
  branches and tags with one branch rule, `dev`, no tag rule and no wildcard. This is a repository setting, which no
  branch's files can change.
- Nothing wider is admitted. A run on any other branch, a dispatch on a branch whose own workflow file was edited
  included, is refused at the environment before its first step. A pull request's run carries a `refs/pull/` ref that
  the branch rule does not match. A fork's runs hold none of this repository's environments.
- Which job may name the environment, and which events the workflow starts on, are the tree's to hold, by the delivery
  that answers #772. This ruling does not decide them.
- The release environment's line is unchanged: tags matching `v*` only, the owner as its required reviewer, and admin
  bypass off.

## Why it is admitted

The set of refs that can reach the internal credential does not change: it is still `dev` alone. What changes is the
set of events named on that ref, which now includes a push. A push to `dev` comes only from a merged pull request,
because `dev` refuses deletion and a non-fast-forward push and has no bypass actor (`.github/rulesets/dev.json`).

The rejected alternatives:

- **Keep dispatches only, and drop the push trigger.** Every internal build would wait for a person to dispatch it,
  which is the wait #694 removed.
- **An environment rule that tells a push from a dispatch.** No such rule exists, because the rule reads the ref and not
  the event. A dispatch-only line can only be held in the tree, and a branch's own workflow file can drop any guard the
  tree writes.
- **A required reviewer on the internal environment.** Every push run would wait for an approval.
- **Editing the 2026-10-05 ruling's line in place.** A signed record keeps its signed text. This amendment replaces that
  one line, and leaves the signed file as it was signed.
