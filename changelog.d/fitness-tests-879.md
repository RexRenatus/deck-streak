### Added

- The clock reads of the web client's TypeScript and of the native Swift are listed sites. Every
  production web file's reads of `Date`, `performance` and `Temporal` equal its row's count in
  `config/clock-reads-web.json`, read through the Svelte compiler's parser, and every production
  Swift file's clock reads equal its row's count in `config/clock-reads-swift.json`. Each row cites
  an ADR that names its path, and each census prints what it examined and refuses an empty
  population (SPEC-431).
