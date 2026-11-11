# Schematic: the merge queue into dev (SPEC-604, ADR-625, ADR-689)

Kind: data flow. It draws the paths a pull request into dev takes to its merge commit, by its
label or by the READY status the `deckstreak-ready` GitHub App posts on its head, as `.mergify.yml`
and `.github/workflows/changelog.yml` define them.

```mermaid
flowchart TD
    pr["pull request into dev"] --> label["label queue"]
    pr --> status["status queue-ready at success on its head, posted by the deckstreak-ready GitHub App"]
    pr --> command["queue command or queue checkbox"]
    label --> rule["pull_request_rules: the label rule, its queue action"]
    status --> statusRule["pull_request_rules: the READY rule, its queue action, on the queue conditions and that status in its qualified form"]
    command --> restriction{"commands_restrictions: write permission, and the label or that status"}
    restriction -- "refused" --> wait["waits, unqueued"]
    restriction -- "admitted" --> qc
    rule --> qc{"queue_conditions: base dev, not draft, no unresolved review thread, ci, fragment, dependency-review, no failure in that status"}
    statusRule --> qc
    qc -- "refused" --> wait
    qc -- "admitted" --> lone{"alone at the front of the queue and holds the tip of dev"}
    qc -- "while queued, that status on the head turns to failure" --> dequeued["dequeued: the queue conditions no longer match"]
    lone -- "yes: Direct Merge, no batch pull request" --> merge
    lone -- "no" --> batch["batch draft pull request by mergify[bot] from mergify/merge-queue/, up to six members, one batch checked at a time"]
    batch --> ci["ci"]
    batch --> fragment["fragment"]
    batch --> review["dependency-review"]
    fragment --> arm{"author mergify[bot] and head ref under mergify/merge-queue/"}
    arm -- "yes" --> deps["dependency-file rule of the Dependabot arm"]
    arm -- "no" --> plain["fragment rule as before"]
    ci --> mc{"merge_conditions: the queue_conditions list, its checks read on the batch"}
    deps --> mc
    plain --> mc
    review --> mc
    mc -- "red" --> split["the queue splits the batch and retests"]
    mc -- "pending at checks_timeout" --> timeout["dequeued: checks-timeout"]
    mc -- "green" --> merge["each member merges into dev by a merge commit"]
    merge --> push["push runs on dev: ci (push)"]
```

The `fragment` check passes a batch whose members each add a fragment by its existing rule; the arm
above matters only for a batch that adds none, which is a batch of dependency updates alone.

The two pull request rules decide entry, so the queue conditions name neither the label nor the
status; they refuse a pull request whose status from that App reads `failure`, whichever way it came.
The App never posts on a batch pull request, so the merge conditions, which repeat the queue
conditions, read no failure there.
