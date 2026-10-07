# Schematic: the context map as components

Kind: component. Read at DeckStreak `main` 769ee62 (`docs/CONTEXT-MAP.md`, `crates/*/Cargo.toml`).
The binding record is the fence in docs/CONTEXT-MAP.md; this diagram draws it by layer. An arrow
is a Cargo dependency, and it points down only.

```mermaid
flowchart TB
  subgraph root[composition root]
    daemon[daemon: deckstreakd]
  end
  subgraph adapters[adapters]
    api[api: axum HTTPS]
    bot[bot: Telegram Bot API]
  end
  subgraph app[application]
    coordination[coordination: use cases and jobs]
  end
  subgraph domain[domain contexts]
    analytics
    progression
    streaks
    curriculum
    economy
    quests
    habits
    focus
    discipline
    markets
    notifications
    readings
    insights
    publishing
    privacy
  end
  subgraph acl[anti-corruption layers]
    ingest[ingest: Anki]
    vault[vault: second brain]
    identity[identity: Telegram initData]
    agent[agent: Claude Code runner]
  end
  kernel[kernel: shared kernel]
  xp[xp: the per-review XP rule, no I/O]

  daemon --> api & bot & coordination
  api --> coordination
  bot --> coordination
  api --> notifications
  bot --> notifications
  api --> identity
  bot --> identity
  coordination --> domain
  coordination --> ingest & vault & identity & agent
  analytics & progression & curriculum & quests & discipline & readings & insights --> ingest
  domain --> kernel
  acl --> kernel
  progression --> xp
```

The XP crate was drawn by SPEC-360; the clients' edges to it are #639's.

The Mini App (`web/app`) is outside the crate graph: it reaches `api` over HTTPS only.
