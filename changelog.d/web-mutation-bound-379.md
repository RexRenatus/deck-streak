### Changed
- Both web mutation legs, the weekly battery's `web` job and `mutation-web`, are bounded at 100 minutes, the worst measured cost a mutant times the larger measured count, one and a half times, plus set-up and tail, rounded up to the five, held inside 100 to 120 by a band test; no StrykerJS setting changes and no mutant is dropped (SPEC-379, ADR-390; refs #693).
