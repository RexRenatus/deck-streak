---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: cac3ede067874a89b831ed4e544829291364ddd9
body_status: seeded
---

# packs/ddd

Domain-driven design as a boundary a probe can hold: bounded contexts with a binding context map,
one ubiquitous language per context, an ownership register, a dependency graph equal to the map,
and the composition root as the one place contexts are joined. Its probe judges any repository
root (SPEC-V2-2186, ADR-V2-2186). Which seats consume this pack is its catalog row's `consumes`
(ADR-V2-1990), so this body names none.

```
phxd pack probe --pack ddd --root PATH --format json
python3 scripts/ddd-probe.py --root PATH check all
```

`pack probe` keeps only each row's exit; run the probe itself to read the findings.

## Bounded contexts, and the map as binding

A bounded context owns one part of the domain and speaks its own language inside it. Derive the
contexts from what the system does, not from where code happens to sit, and write for each one
what it owns, its aggregates, its language, and what it does NOT own.

The context map is the one record of which context may depend on which. It is binding: code
follows the map, and a change that needs an edge the map lacks is a design question, answered by
an ADR that amends the map in the same change. Never add an edge to make code compile. The edge
is the design.

The map is a fenced block, one context per line:

    ```context-map
    deckstreak-kernel                             depends on: nothing
    deckstreak-cards                              depends on: kernel
    deckstreak-review                             depends on: kernel, cards
    deckstreak-bot      (Telegram bot adapter)    depends on: kernel, cards, review
    deckstreak-server   (composition root)        depends on: kernel, cards, review, bot
    miniapp-ui          miniapp/src/ui            depends on: miniapp-api
    miniapp-api         miniapp/src/api           depends on: nothing internal
    ```

- A context with **no path** is a Cargo package of that name in the root workspace; its edges are
  its manifest's `[dependencies]` and `[build-dependencies]`, never `[dev-dependencies]`.
- A context with **paths** owns the code under them: Python, TypeScript and JavaScript files, and
  a crate if a path holds a `Cargo.toml`. Test files are not its code (`exclude` names them).
- `(note)` is prose. `depends on: nothing` and `nothing internal` both mean none; a trailing
  `only` (`kernel only`) is ignored.
- `dependency_prefix` lets `kernel` name `deckstreak-kernel`. phoenix-v2's crate graph is written
  that way, under a `## Crate graph` heading with an untagged fence, which `map_heading` reads.

## The dependency graph equals the map

In both directions:

- **An undeclared edge** is code in one context that imports another context the map does not
  let it depend on. It is refused with the first site (`file:line`, or the manifest).
- **An unused edge** is a dependency the map declares and no code uses. It is refused too, because
  a permitted edge nobody uses is where the next violation enters without a review. Draw an edge
  in the change that first uses it.
- **Code no context owns.** An import that resolves to first-party code outside every context's
  paths is refused. In a Cargo workspace, a member no context names is refused.
- **A planned context**, one with no code yet, is counted and its edges are not judged.

Enforce the graph with the build where the language allows. In Rust, one crate per context makes
the map the Cargo dependency graph: a type from an undeclared context is a compile error, and the
probe's work is only to hold the manifests and the map equal. In Python and TypeScript nothing
stops an import, so the probe reads every import statement: `ast` for Python, relative imports
resolved against their package; a scanner for TypeScript and JavaScript that follows
`tsconfig.json` `paths` and `baseUrl`, `require()` and dynamic `import()`.

## One name per concept

The ubiquitous language is per context: one concept has one name in the code, the schema and the
prose of the context that owns it. Declare it in a lexicon, `docs/LEXICON.md`:

    ```lexicon
    card: flashcard, flash card
    review in deckstreak-review: study session, drill
    streak in deckstreak-review, deckstreak-bot: combo
    ```

Each entry is `<the one name> [in <context>, ...]: <the words it replaces>`. The probe splits every
identifier the scoped contexts declare into snake and camel segments (`load_flashcards` is `load`,
`flashcards`; `StudySession` is `study`, `session`), and refuses one that says a replaced word, in
the singular or with an `s` or `es`. Comments and string contents are not declarations. A term
cannot hold the word `in`.

The other direction, one word naming one concept, is a stronger lock phoenix-v2 holds with
per-word guards that register every file a word may be declared in: the Overloaded-tokens table
of its context map, and `crates/phxd/tests/hold_vocabulary.rs` and
`crates/phxd/tests/speculation_vocabulary.rs`. Write one when a word starts meaning two things.

## The ownership register

Every table, and every aggregate's store, is owned by the context that WRITES it. Reads say
nothing: many contexts read a table and none may change what it means. Record the owner of each
table beside the map. A table with two writing contexts is a design defect. A second writer is
either the owner calling through a port, or evidence that the two contexts are one. phoenix-v2
derives its register from every write in production source and refuses a table with no owner;
its context map's `## Table ownership` section is the worked example.

## Wiring and anti-corruption at the composition root

- **One composition root.** The binary that assembles the contexts depends on all of them and
  nothing depends on it. A trait one context declares and another implements is joined there,
  because in Rust the orphan rule makes that the only crate that can hold the impl.
- **An adapter translates at the edge.** A foreign model (a Telegram update, an Anki note, a
  third-party payload) is translated into the context's own types in an adapter, never passed
  inward. The adapter owns the foreign vocabulary so the context never learns it.
- **The shared kernel admits little.** Ids, the event type, the verdict type and the storage
  policy. A type enters only when two or more contexts that may not depend on each other both
  need it. Domain logic placed in the kernel for convenience is a context hiding in shared code.

## Check table

| id | scope | green when | on phoenix-v2 |
|---|---|---|---|
| `context-map-parses` | tree | every line parses; names unique; every dependency a declared context; no cycle; no context's path inside another's | binding: 13 contexts, the crate graph |
| `declared-edges-match-imports` | tree | each context's manifest and imports depend on exactly its declared contexts; no unowned first-party import; no unnamed workspace member | binding: 13 manifests equal the crate graph |
| `lexicon-locks` | tree | no declared identifier in a scoped context says a replaced word | binding over `docs/LEXICON.md`; the vocabulary guards above hold the other direction |

## Configuration

| key (`ddd`) | default |
|---|---|
| `context_map` | `docs/CONTEXT-MAP.md` |
| `map_heading` | `""`: read every ```context-map fence; set it to read the first fence under that `## ` heading |
| `dependency_prefix` | `""` |
| `lexicon` | `docs/LEXICON.md` |
| `python_roots` | `[]`: a context's module is found by walking up its `__init__.py` files |
| `include_dev_dependencies` | `false` |
| `exclude` | test files: `**/test_*.py`, `**/*_test.py`, `**/conftest.py`, `**/tests/**`, `**/__tests__/**`, `**/*.test.*`, `**/*.spec.*`, `**/*.d.ts` |
| `advisory` | `{}` |

## Adopt in another repository

1. Copy `scripts/methodology_probe.py`, `scripts/sdd-probe.py`, `scripts/ddd-probe.py` and
   `scripts/tdd-probe.py` from a phoenix-v2 checkout into your `scripts/`, side by side.
2. For a Rust workspace with one crate per context (`crates/<context>/Cargo.toml`, each package
   named `deckstreak-<context>`), write the map above into `docs/CONTEXT-MAP.md` and:

       {
         "vendored_from": "<the phoenix-v2 commit the four files came from>",
         "ddd": {"dependency_prefix": "deckstreak-"}
       }

   A TypeScript front end joins the same map as path contexts (`miniapp-ui miniapp/src/ui`), and
   its `tsconfig.json` aliases resolve without configuration. A Rust to WASM front end is one more
   crate, and one more line.
3. Add `python3 scripts/ddd-probe.py --root . check all` to CI.

## What this pack does not do

- It reads no `use` statement in Rust: the compiler already refuses an import the manifest lacks.
- It follows no import built at run time from a string, no `sys.path` edit, and no package that
  `package.json` workspaces resolve by name. An edge it cannot see reads as unused, which refuses
  rather than passes.
- It derives no ownership register: the register is authored beside the map, and a repository
  that wants it held writes a test over its own writes.
