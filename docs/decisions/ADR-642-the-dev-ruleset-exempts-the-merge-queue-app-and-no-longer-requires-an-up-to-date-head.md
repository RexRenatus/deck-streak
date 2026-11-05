# ADR-642: the dev ruleset exempts the merge queue app and no longer requires an up-to-date head

- **Status:** accepted with SPEC-620.
- **Decides:** SPEC-620.

## Context

Pull requests into dev merge through the Mergify merge queue (ADR-625). The queue merges a tested
batch of pull requests, and a ruleset on dev lists the queue's app as a bypass actor in the
`exempt` mode so that it can. The owner set that bypass on the live dev ruleset, and set the up-to-date
rule off. The committed file `.github/rulesets/dev.json` still held no bypass actor and an up-to-date
head, and `scripts/github-setup.sh` sends each ruleset file whole, so applying the rulesets would have
undone the live ruleset and stopped the queue. Mergify reads its configuration from the default
branch, and the queue file did not say so.

## Decision

The dev ruleset file records the live dev ruleset, field by field:

- `bypass_actors` holds exactly the merge queue app (actor 10562, type `Integration`, mode `exempt`);
  the files of `main` and of the release tags keep no bypass actor;
- `strict_required_status_checks_policy` is `false`, because the queue tests each batch on the tip of
  dev before it merges, so the batch's checks already ran on the code that is about to land;
- the pull request rule carries the two fields the live ruleset carries,
  `require_extra_approval_for_unattributed_changes: true` and `required_reviewers: []`.

The tests pin exactly that: one test compares dev's bypass actors whole, one holds every other
ruleset file to none, and one holds dev's strict policy `false`. The queue file gains one comment line,
line 2, naming `dev` as the default branch it is read from. `RELEASING.md` says dev does not require
an up-to-date head.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| Leaving the file as it was and the live ruleset set by hand | the setup script sends the file whole, so applying the rulesets would remove the queue's bypass and turn the up-to-date rule back on, and the queue could not merge |
| Keeping the up-to-date rule on dev beside the queue | every merge of a batch moves dev, so every other waiting pull request would fall behind and need an update before it could merge, which is the cost the queue removes |
| A bypass for a person or a team in place of the queue's app | it widens who can skip the checks beyond the one app that merges a batch the checks passed on |

## Consequences

- Applying the rulesets from the file keeps the live dev ruleset as it is.
- A pull request into dev can merge on checks that ran against an older tip when it is merged by a
  person; the queue never does, since it tests each batch on the tip of dev.
- `.mergify.yml` carries one added comment line, line 2, saying it is read from dev, the default
  branch; its queue rules are unchanged, and its merge into dev is the first change that touches the
  file the queue reads.
- The two tests that held `bypass_actors == []` for every file and a strict policy of `true` for dev
  are replaced by three that hold the new values, the removal of the old values being the owner's
  ruling, recorded in the merge queue plan.
- ADR-625 stays as it is: it left the ruleset file alone, and this decision records it afterwards.
  ADR-034 still holds for `main`, which keeps no up-to-date rule and no bypass.

## What would make this wrong

- A second app or person added to the bypass list without a decision of its own would make A1 red;
  a bypass that skips checks the queue never ran would make the exemption unsafe.
- A queue that stops testing each batch on the tip of dev would leave dev with no up-to-date check at
  all.
