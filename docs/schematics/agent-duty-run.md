# Schematic: one agent duty run, from composition to verdict

Kind: sequence. Read at DeckStreak `main` ce3683d (ADR-010, ADR-015, `.packs/VENDORED.json`'s
exclusions, `scripts/check.sh`'s scrub stage), and at the packs vendored from `19bb0f3`
(persona-core's prompt order, ai-content-safety's fence and gate, and the subscription-proxy pack's
client rules as the maintainer's box runs them). Added by SPEC-043.

```mermaid
sequenceDiagram
  participant C as coordination (a duty's use case)
  participant A as agent context
  participant G as the gate (vendored probes, python3)
  participant R as agent/run-headless.sh
  participant P as subscription proxy (loopback, reverse tunnel)
  C->>A: duty request: topic, persona binding, seed, memory reader for one subject
  A->>G: output-invisible on each untrusted input
  G-->>A: green, or the input is refused
  A->>A: compose: rules and policy (system), persona, duty, memory as data, fenced JSON-encoded inputs
  A->>R: prompt file on disk (private run directory), caps in the environment
  R->>R: key read with gcloud into a variable; competitors unset; loopback URL checked
  R->>P: /phx/capacity, the key on curl's stdin
  P-->>R: ready, exhausted, 401 or no word
  R->>P: claude, prompt on stdin, max turns, max budget, json output, dontAsk, strict MCP, settings, under timeout
  P-->>R: result JSON
  R-->>A: exit 0 to 6, and the result on stdout
  A->>A: read subtype and is_error first; engine-written frontmatter
  A->>G: the task's gate classes with --subject naming the output and its template
  G-->>A: every class green, or the first red class and its findings
  A->>A: record agent_runs: duty, persona, verdict, cause or class, turns, tokens, cost estimate, duration
  A-->>C: delivered (output), withheld (class), or unavailable (closed cause)
```

| exit | cause in the verdict |
|---|---|
| 1 | `key_missing` |
| 2 | `refused_shape` |
| 3 | `key_rejected` |
| 4 | `capacity_exhausted` |
| 5 | `proxy_unreachable` |
| 6 | `run_failed`, `turn_cap`, `time_cap` or `budget_cap`, read from the result |

A withheld or unavailable verdict delivers nothing to the owner or the vault and raises one alert
through the router. The daily-reading task holds no tool, so the only thing a run can produce is its
reply.
