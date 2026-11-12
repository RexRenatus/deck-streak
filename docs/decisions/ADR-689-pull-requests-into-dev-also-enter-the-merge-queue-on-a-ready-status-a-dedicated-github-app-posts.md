# ADR-689: pull requests into dev also enter the merge queue on a READY status that a dedicated GitHub App posts, chosen over the label alone, the maintainer's own account, a status inside the queue conditions and a Dependabot rule

- **Status:** accepted with the SPEC-604 revision that names it.
- **Decides:** SPEC-604 R2 (revised). ADR-625 stands.

## Context

ADR-625 sends a pull request into the Mergify merge queue when it carries the label `queue`. The
maintainer verifies each pull request before it may merge, and until now that verification reached
the queue only as that label, added by hand on each pull request.

Mergify's and GitHub's documentation set the shape this decision works within:

- A queue action in `pull_request_rules` may sit on any conditions; the pull request must also
  satisfy the queue's `queue_conditions` to enter (Queue action).
- Any status check reported to GitHub can be read through `check-success`. A commit status
  carries no app identity, so Mergify attributes it to the account that posted it: a status posted
  by a GitHub App also matches `@<slug>/<name>`, while one posted by a person is reachable only
  through the bare form (Conditions, qualifying checks by GitHub App).
- Only the latest status posted in a context counts for a commit; GitHub keeps the earlier ones (Commit statuses).
- If a pull request no longer meets the `queue_conditions`, it is removed from the queue
  (Merge Queue Lifecycle).
- In case of a draft pull request, the merge conditions for checks are evaluated against the
  temporary pull request instead of the original one (Configuration File Format, queue rules).
- A GitHub App has no permissions by default, and its installation access token can be narrowed
  to one repository and one permission and expires after one hour (GitHub Apps documentation).

## Decision

- A GitHub App, `deckstreak-ready`, holds the repository permission Commit statuses (write) and
  nothing beyond the read-only metadata every app holds. It has no webhook and is installed on this
  repository alone.
- When the maintainer has verified a pull request at one commit, the App posts the status
  `queue-ready` with the state `success` on that commit. A second `pull_request_rules` entry, with
  the `queue` action alone, queues a pull request into dev that carries it, read in the qualified
  form `@deckstreak-ready/queue-ready`.
- The label rule stays, for pull requests the maintainer queues by hand and for re-queues.
- The queue conditions drop the label, since the two rules now decide entry, and gain
  `-check-failure = @deckstreak-ready/queue-ready`: when the App turns its status on a queued pull
  request to `failure`, the pull request leaves the queue. The merge conditions repeat the queue
  conditions, so the queue stays single-step; the App never posts on a queue batch pull request,
  so the line holds there.
- `commands_restrictions` keeps a queue command, and the queue checkbox, to what the two rules
  admit, and keeps the default write permission.
- A status binds to one commit, so a push leaves the new head without the status and takes a queued
  pull request out of the queue.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| The label alone | each pull request waits for a hand step after it is verified |
| The status posted by the maintainer's own account, read in the bare form | the bare form matches a status of that name from any account or app that can write statuses on the repository |
| `check-success` of the status inside `queue_conditions` and `merge_conditions` (an `or` with the label) | the merge conditions for checks are read on the batch's temporary pull request, which never carries the status, so a batch holding such a member would wait until its checks timeout |
| A check run in place of a commit status | it needs the Checks permission, and Mergify already reads a commit status through `check-success` |
| A rule that queues dependency updates by author | out of this change's scope: each such update is verified like any other pull request and then carries the status |
| `auto_merge_conditions` under `merge_protections_settings` | ADR-625 rejected it, and it stands |

## Consequences

- A verified pull request into dev enters the queue when the App posts the status, with no label.
- The status is public: it names the context, its state and a short description, and links nowhere.
- The App's key is a credential outside the tree; whoever holds it can queue a pull request whose
  three checks are green, or hold one, until the key is deleted or the App is suspended.
- A queue command on a pull request with neither the label nor the status is refused.
- ADR-625's bullet that lists the label among the queue conditions no longer describes the file:
  this decision replaces it, and the rest of ADR-625 stands.
- Removing the label from a queued pull request no longer takes it out of the queue; the
  `dequeue` command does, and so does a `failure` status from the App on its head.

## What would make this wrong

- A Mergify change that stops reading commit statuses in the qualified form, or stops removing a
  queued pull request whose queue conditions no longer hold.
- Batches that wait on the new line in the merge conditions, which would show the temporary pull
  request read differently from the documentation.
