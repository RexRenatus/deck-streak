# SPEC-670: every declared dependency is used by its own package

Path: `docs/specs/SPEC-670-every-declared-dependency-is-used-by-its-own-package.md`.

- **Issue:** #1388.
- **Context(s):** one census module under `scripts/tests/`, its planted fixtures, the rows band and
  the documents. No manifest, lock file, product source, workflow, existing test or tool
  configuration changes, and no dependency is added or removed.
- **Decided by:** ADR-693 (unused dependencies are one text census over every manifest, and it
  starts at warn). It cites SPEC-436 and ADR-450 D6 (the promotion rule and kill criteria a new
  census follows) and SPEC-431 (the census shape over `Cargo.toml` text), and amends none of them.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-670.md`.

## 1. The problem, measured

No check reads whether a declared dependency is used. A dependency no file of its package uses
still builds, still enters the lock files and still reaches every audit, and nothing in the tree
says why it stays. A reading of the tree before this change, by the forms of section 2, read the
root `Cargo.toml` and 39 member manifests, three `package.json` files (the root package, the app
and a vendored package reached by a `file:` override), three `Package.swift` files, two XcodeGen
specs and one requirement file, and at least 490 declared entries. It found no use for 38 of
them: 30 are listed in the census with their reasons, one is undecided by R9, and seven are left to
the census's first CI run (four workspace keys a member manifest that reading did not read may
inherit, and three entries of a crate holding files that reading did not read). That reading did
not read every file of the population, so this SPEC claims no exact count over the whole
population: the census's first CI run reads it all, and a site that run prints beyond the listing
stays at warn until its own listing commit.

## 2. Requirements

R1. The population, per ecosystem: Rust, the root `Cargo.toml` and each directory a `[workspace]
members` pattern matches, less `exclude`, that holds a `Cargo.toml`; web, the root `package.json`,
each folder a `packages:` entry of `pnpm-workspace.yaml` names and each folder a `file:` value under
its `overrides:` names, each holding a `package.json`; Swift, each `ios/*/Package.swift` and each
`ios/*.yml` with a top-level `targets:`; Python, every file whose name matches
`*requirements*.txt` in any folder of the tree, so every tracked one, less `node_modules`,
`.svelte-kit` and `.git`, where no file is tracked. The census prints
`examined N <ecosystem> manifest(s)` and refuses 0 as VOID, and prints
`read N <ecosystem> source file(s)`; a package with no source file makes each of its entries a
site, never a pass.

R2. A Rust entry's source name is its key with `-` as `_` when its entry, or for `workspace = true`
the `[workspace.dependencies]` entry of its key, names a `package` other than the key; else, for a
workspace member, that member's `[lib] name`, or its package name with `-` as `_`; else its package
name with `-` as `_`.

R3. A Rust use of a source name `N` is `use N` (a leading `::` allowed), `extern crate N`, or `N::`
not preceded by an identifier character or by `::` after one; a macro path, an attribute path and a
derive path are `N::` forms. Plain comments are blanked first; doc comments and strings are kept.

R4. An entry of `[dependencies]` or `[dev-dependencies]`, at the top level or under a
`[target.<cfg>]` table, is judged against every `.rs` file of its member but the build script; a
`[build-dependencies]` entry is judged against the build script alone (`package.build`, else
`build.rs`). Each group also holds every file a `#[path = "..."]` attribute or an `include!` in it
names, followed transitively, inside or outside the member and never above the repository root.

R5. A `[workspace.dependencies]` key that no member entry inherits by `workspace = true` is a site.

R6. A web package's files are, for the root package, its `*.config.*` files at the root, and for
any other package every `.ts`, `.js`, `.mjs`, `.cjs`, `.svelte`, `.html`, `.css` and `.json` file
under its folder, with `package.json`, `node_modules` and `.svelte-kit` left out. An entry of
`dependencies`, `devDependencies` or `optionalDependencies` is used when an import specifier in
those files names it (`from`, `import`, `import(`, `require(`, `vi.mock` and its siblings,
`/// <reference types>`, and CSS `@import`, `@plugin` and `@reference`; the package is the first
path part, or the first two for a scoped name, with a query dropped; `$app/`, `$env/` and
`$service-worker` name `@sveltejs/kit`; `node:` and a Node builtin name the Node builtins), when a
`@vitest-environment <name>` docblock names it, when a word of a `package.json` script equals it
(each segment's first word after its environment assignments, or the word after `pnpm`, `npm` or
`npx` and their `exec`, `dlx` or `x`), when a configuration file (the package folder's own files
by those suffixes and every `.json` file of the package) quotes it alone or after a `/` and before
`@`, `/` or the quote, or when it is `@types/<x>` and `<x>` is named, or listed in a
`tsconfig*.json`'s `compilerOptions.types` at the package root, or is `node` with a builtin named.

R7. A Swift target dependency (a bare string, `.product(name:)`, `.target(name:)` or
`.byName(name:)` in a `Package.swift` target's `dependencies:`; an XcodeGen `- package:` with its
`product:`, or `- target:`) is used when an `import`, `@testable import` or `@_exported import` of
its module (the product, the package name when no product is given, or the target) sits in the
target's sources (its `path:`, else `Sources/<target>` or `Tests/<target>`; a spec's `sources:`).

R8. A Python requirement's module, its name lowercased with `-` as `_`, is used when an `import M`
or `from M` line sits in a `.py` file under the requirement file's folder.

R9. Three forms are undecided: a Rust entry with no use found whose key a `[features]` value names
(`dep:K`, `K/f`, `K?/f` or `K`); a `peerDependencies` entry with no use found; a Swift dependency
of a target with no source file. Each prints `UNDECIDED unused-dependency <key> <form>`, the form
`feature`, `peer` or `sourceless`, and never warns or fails.

R10. A site's key is `<manifest> <table> <name>`: the manifest's path from the root; the table as
the manifest spells it (`dependencies`, `dev-dependencies`, `build-dependencies`,
`target.<cfg>.<table>`, `workspace.dependencies`, `devDependencies`, `optionalDependencies`,
`targets.<target>.dependencies`, `requirements`); the name as declared, `<package>/<product>` for
an XcodeGen package product.

R11. Each unlisted site prints `WARN unused-dependency <key>`, which passes at `Warn` and fails at
`Fail`; each listed key that is no current site prints `STALE unused-dependency <key>`, which fails
at both levels.

R12. `LEVEL` is `Warn`. `LISTED` maps each of today's sites to a non-empty reason. A site the first
CI run prints beyond the listing is listed by its own commit with that run's job id in the record, unless its key names a path the listing may not name; such a site stays unlisted and prints at warn.

R13. `LEVEL` moves to `Fail` only by its own delivery, under ADR-450 D6's promotion rule.

## 3. Acceptance criteria of SPEC-670

| id | criterion | decided by |
|---|---|---|
| A1 | R2, R3, R4, R9: over the planted Rust tree, an unused entry, a name only in a plain comment, a lookalike path, a normal entry named only by the build script, a build entry named only by the library, an unused dev entry and an unused target entry are the exact sites; a `use`, an `extern crate`, a path, macro, attribute and derive use, a rename, a workspace-inherited rename, a hyphen, a member's `[lib] name`, a doc test, a `#[path]` file outside the crate and an `include!` file are clean; a feature-only entry is undecided | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_a_rust_entry_is_used_by_a_use_form_of_its_source_name` |
| A2 | R5: over the planted Rust tree, the one workspace key no member inherits is the only workspace site | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_a_workspace_entry_no_member_inherits_is_a_site` |
| A3 | R1, R6, R9: over the planted web tree (a root package, a `packages:` member and a `file:` override), each specifier form, a scoped subpath, a SvelteKit module, a Node builtin, a script word after an assignment and a runner, a configuration string, the docblock and an `@types` package are clean; an unused entry in each package and a bin whose name differs from its package are the exact sites; a peer is undecided | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_a_web_entry_is_used_by_a_specifier_a_script_a_configuration_or_its_types` |
| A4 | R7, R9: over the planted Swift tree, a `Package.swift` and an XcodeGen spec, an imported target, product and `@testable` host are clean; an unimported product and an unimported package are the exact sites; a dependency of a target with no source is undecided | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_a_swift_target_dependency_is_used_by_an_import` |
| A5 | R1, R8: over the planted Python tree, a requirement file under `tools/*/` and one in a folder outside the root and `tools/*/` are both read; in each an imported requirement is clean and an unimported one is a site, the exact sites | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_a_python_requirement_is_used_by_an_import` |
| A6 | R11, R12: `LEVEL` is `Warn`; a planted unlisted site passes with its exact `WARN` line at `Warn` and fails at `Fail`; a listed site prints nothing; a planted stale entry fails with its exact `STALE` line at both levels | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails` |
| A7 | R1: a planted tree with no manifest is refused as VOID for each ecosystem, naming `examined 0 <ecosystem> manifest(s)` | `test_unused_dependencies.EachDeclaredDependencyIsUsedByItsPackage.test_an_empty_population_is_refused_per_ecosystem` |
| A8 | R1, R11, R12: over the repository, every ecosystem's manifest count prints above zero, `LEVEL` is `Warn`, every listed reason is non-empty and no listed key is stale | `test_unused_dependencies.TheRepositoryListsTodaysSites.test_the_repository_lists_exactly_todays_sites` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_a_rust_entry_is_used_by_a_use_form_of_its_source_name
A2: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_a_workspace_entry_no_member_inherits_is_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_a_web_entry_is_used_by_a_specifier_a_script_a_configuration_or_its_types
A4: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_a_swift_target_dependency_is_used_by_an_import
A5: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_a_python_requirement_is_used_by_an_import
A6: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails
A7: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_an_empty_population_is_refused_per_ecosystem
A8: python3 -m unittest discover -s scripts/tests -p test_unused_dependencies.py -k test_the_repository_lists_exactly_todays_sites
```

Every planted case is a file under `scripts/tests/fixtures/unused-dependencies/` whose name ends in
`.fixture`, written under its name less that suffix into a scratch root and read there as text; no
runner collects it and nothing runs, compiles or imports it. Every expected key and line is a
literal the test writes, never a value the census computes.

## 4. File manifest

| file | change |
|---|---|
| `scripts/tests/test_unused_dependencies.py` | added: the census, its listing and its tests |
| `scripts/tests/fixtures/unused-dependencies/**/*.fixture` | added: the planted Rust, web, Swift, Python and empty trees |
| `scripts/mutation-rows.d/S67000-S67099.json` | added: section 9's rows |
| `docs/specs/SPEC-670-every-declared-dependency-is-used-by-its-own-package.md` | added: this file |
| `docs/decisions/ADR-693-unused-dependencies-are-one-text-census-over-every-manifest-that-starts-at-warn.md` | added |
| `docs/red-first/SPEC-670.md` | added |
| `changelog.d/unused-dependencies.md` | added |

## 5. What this does NOT cover

- No dependency is removed. Each listed site keeps its entry; a removal is its own change.
- No use through another crate's re-export, a macro's generated code, a generated source the tree
  does not hold, or a command-line tool run outside a package script is found from text; each such
  entry is a listed site with its reason.
- `[patch]` entries and the lock files are not read: a patch entry replaces a source and declares
  no use.
- XcodeGen `- framework:` and `- sdk:` dependencies are not read; the tree holds none.
- No existing tool for unused dependencies is installed, run or configured.

## 6. Risks

- **A sibling adds or drops a dependency first.** The census then prints a new `WARN` line or a
  `STALE` failure on this branch; the listing is re-derived at the build's own base.
- **A form the census does not read.** A use spelt in a way R3, R6, R7 or R8 does not name is a
  false site at warn, never a silent pass; its cure is an amendment naming the form.
- **A name in a string.** Strings are kept, so a quoted crate name counts as a use; the reading
  before this change found no verdict that turns on a comment or a string.

## 7. What only CI proves

- The repository test reads every manifest and every source file of the population, and its first
  run prints each site beyond the listing; its job id is recorded in `docs/red-first/SPEC-670.md`.

## 8. Formal model

No state, actor or write path is added, and no covered span is edited. The census is a pure reader
of committed text; its classification is closed by the planted trees, whose expected keys are spelt
literally, and by section 9's rows. No model checker runs.

## 9. Mutation rows (S67000-S67099)

Rows S67000 to S67029, in `scripts/mutation-rows.d/S67000-S67099.json`, table
`SCRIPT_MUTATIONS`, each on `scripts/tests/test_unused_dependencies.py`. Each find is taken from the
green commit's text and occurs exactly once in the file; each is killed by a planted test, never by
the repository test.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S67000-S67002 | the `use`, `extern crate` and path forms | A1 |
| S67003-S67005 | the rename, member library and hyphen arms of the source name | A1 |
| S67006-S67007 | the build script's judgement and the dev table | A1 |
| S67008-S67009 | the `#[path]` and `include!` follow | A1 |
| S67010 | the plain comment blank | A1 |
| S67011 | the workspace arm | A2 |
| S67012-S67017 | the specifier, script, configuration, docblock, types and scoped arms | A3 |
| S67018-S67019 | the Swift import and the XcodeGen package reader | A4 |
| S67020 | the Python import | A5 |
| S67021-S67023 | the feature, peer and sourceless undecided arms | A1, A3, A4 |
| S67024-S67026 | the warn pass, the fail refusal and the stale refusal | A6 |
| S67027 | `LEVEL`'s `Warn` | A6 |
| S67028 | the empty-population refusal | A7 |
| S67029 | the Python population's rule, every `*requirements*.txt` in any folder | A5 |
