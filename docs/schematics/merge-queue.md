# Schematic: the merge queue into dev (SPEC-604, ADR-625)

Kind: data flow. It draws the path a pull request into dev takes from its label to its merge
commit, as `.mergify.yml` and `.github/workflows/changelog.yml` define it once SPEC-604 lands.

```mermaid
flowchart TD
    pr["pull request into dev"] --> label["label queue"]
    label --> rule["pull_request_rules: queue action"]
    rule --> qc{"queue_conditions: base dev, label queue, not draft, no unresolved review thread, ci, fragment, dependency-review"}
    qc -- "refused" --> wait["waits, unqueued"]
    qc -- "admitted" --> lone{"alone at the front of the queue and holds the tip of dev"}
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
