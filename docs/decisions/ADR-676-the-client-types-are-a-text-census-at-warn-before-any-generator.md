# ADR-676: the client types are a text census at warn before any generator

- **Status:** accepted with SPEC-654.
- **Cites:** ADR-450 D6 (a census's promotion from warn to fail, and its demotion).

## Context

The web client's answer types are written by hand beside the handlers' answers, and the iPhone app
has no API layer yet. Most handler answers are `json!` literals with no Rust type, so a generator
that derives client types from Rust types reaches only a few of them. Section 1 of SPEC-654 holds
the measurement.

## Decisions

- **D1.** The client types are compared by a Python census in `scripts/tests` that reads the
  handlers, the web client and the Swift files as text. It adds no dependency and runs no build.
- **D2.** It reads the named forms of its docstring and nothing else. Its false negatives are named:
  a key built outside the forms (a key a handler puts in a map among them), a type another crate
  serializes, a request body posted by hand, a value's type, and a `read(` call whose path or
  parser is outside WEB-BINDING's form. Each pair they touch reads drifted, unmeasured or
  unmatched and is listed with that reason.
- **D3.** A route and a client type pair by method and path, and a pair is equal, drifted,
  unmeasured or unmatched. A drifted pair names every differing key on both sides.
- **D4.** The census starts at warn, with today's findings listed. A listed entry no finding
  matches fails at both levels, so the listing never outlives what it lists.
- **D5.** Promotion to fail is its own delivery, under ADR-450 D6's rule: three iterations of the
  rule review at a precision of 0.8 or more with at least five hits. Demotion is a signed ruling. An
  empty population is void at both levels.
- **D6.** A generator of the client types from the handlers' own types is a later delivery. It
  needs typed answers at the handlers first, and its dependency passes the repository's dependency
  review on its own.

## Alternatives considered

- **Generate the client types now.** Refused here: the `json!` answers have no type to derive from,
  and a new dependency is its own review.
- **An OpenAPI document as the middle step.** Refused here: it adds a second tool between the
  handlers and each client.
- **A Rust test in place of the Python census.** Refused: the census reads TypeScript and Swift as
  well, and the guard tests in `scripts/tests` already read text across the tree.

## Consequences

A drift between a handler's answer and its client's reader is printed on every run of the guard
tests, and a fixed drift must leave the listing in the same pull request.
