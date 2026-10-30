# ADR-531: A media file is known by its SHA-256, its names are bound once, and one grammar reads its references

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-513 |
| builds on | ADR-024, ADR-029 (none amended) |
| measured at | the delivery's base `554a223e`, and the pinned engine `a078820f` |

## Context

DeckStreak knows a media file only by the name a note writes, and reads those names only through the engine: a note's
text through the engine's own media expressions, a note type's CSS through the engine core's font pass, which knows
fonts alone. Nothing can tell two files of one name apart, say which stored file no note needs, or read a note's
names without the engine. The engine's reading and its media check have goldens (G-L5-1, G-L5-2, SPEC-498). This ADR
decides how a file is identified, how a name is formed and bound, which grammar reads names, where it diverges from
the engine, how an image the engine renders from LaTeX is named, and what a sweep may delete.

## D1. A file is known by the SHA-256 of its bytes

- **Chosen: `MediaHash`, the SHA-256 of the file's bytes, from the workspace's `sha2` (ADR-024), with a lowercase hex
  form and a blob path of the form's first two digits, `/` and the whole form.** SHA-256 is the one digest the
  workspace already locks and links in both clients' dependency trees; the blob path spreads a store's files over 256
  folders and names its hash and nothing else, so a path reads back to exactly one hash.
- **BLAKE3:** lost, because it is a new package in the lock with its own admission, for a speed no measured store
  needs, and because a hash a third party checks (a published deck's manifest, a browser's `SubtleCrypto`) is
  SHA-256.
- **The engine's SHA-1:** lost, because SHA-1 is broken for collisions, and a name that certifies its bytes must not
  be shared by two files.

## D2. Certified names, and imported names kept as written

- **Chosen: a file DeckStreak names gets `ds-`, its hash's 64 lowercase hex digits, and an optional `.` with an
  extension of 1 to 10 lowercase ASCII letters or digits; every other plain name is an imported one and is never
  renamed.** The certified form has one spelling, so a name reads back to itself; a file whose bytes hash to anything
  else is refused under it. An imported name is the deck's own: a note writes it, and renaming it would rewrite every
  note that does.
- **A name that starts like a certified one but is not is refused.** `ds-` and 64 hex digits in any case, with an
  uppercase digit, a bad extension or a tail, would look certified without certifying anything.
- **Renaming every file to its certified name at import:** lost, because every note that names the file would be
  rewritten, and the deck would no longer round-trip to the engine.

## D3. A name is bound to one hash, once

- **Chosen: `step` over one name's slot: an unbound name binds live; a bound name is admitted again only for its own
  hash (unchanged when live, revived when released); any other bind is a conflict; a certified name's claim of
  another hash is a mismatch; a refusal leaves the slot as it was. `release_step` releases a live binding and keeps
  its hash.** A name therefore never means two files, even after a sweep deleted its file: the name comes back only
  for the same bytes.
- **Forgetting a released name:** lost, because a later file could take the name, and a note written against the old
  file would show the new one.
- **Last write wins:** lost for the same reason, while the name is live.
- Lean `MediaBind` proves the six rules over `step` and `release_step`; its vectors tie the port to the Rust code.

## D4. One grammar, the engine's, and its divergences

- **Chosen: `scan` reads a note's text as the pinned engine's media check reads it** (`rslib/src/text.rs`'s tag and
  sound tag expressions and its remote check, and the check's use of them in `rslib/src/media/check.rs`): a `src` or
  `data` attribute of an `img`, `audio`, `video`, `object` or `source` tag, with the engine's case folding (the long s
  matches `s`), its word boundaries, its quoted, single-quoted and unquoted values, and a quoted `>` in the tag's run
  of attributes; a lower-case `[sound:…]` tag up to its first `]`; remote references dropped; entities decoded as the
  engine's decoder does, a name it cannot decode kept whole. It is written in Rust with no expression library: the
  core depends on `sha2` alone.
- **The divergences, each cited by its G-L5-1 case:**
  - **LaTeX (cases 64 to 68, 70 and 71):** the engine names an image `latex-` and the SHA-1 of the expression; the
    grammar renders nothing and names nothing for LaTeX. The index adds those names from `latex_images` (D8), so
    `plan` lists them, and G-L5-2's eight `latex-notes` cases are equal.
  - **The inline image (case 24):** the engine writes a `data:image/png;base64,` source to a file named `paste-` and
    the SHA-1 of its bytes and names that file; the grammar writes nothing and keeps the source as written.
  - **Not drawn by either golden:** NFC normalization of a name, the engine's named entities other than `&amp;`,
    `&lt;`, `&gt;`, `&quot;` and `&nbsp;` (numeric ones are decoded), and a word boundary at a combining mark or a
    connector other than `_`.
- **Taking the engine's expressions with an expression library:** lost, because the core would gain a dependency for
  two expressions, and the engine's lazy run of attributes is the one subtle part either way.
- **A grammar of DeckStreak's own, unlike the engine's:** lost, because a sweep must keep every file the engine's own
  check keeps, so the grammar must read at least what the engine reads.

## D5. A CSS is read as the font pass reads it, for every plain name

- **Chosen: `scan_css` reads every `url(`, in any ASCII case, up to the first `)`, trimmed, unquoted and
  `%XX`-decoded, and names it when it is a plain file name, whatever its type.** The reading is the engine core's font
  pass's, so every font case of the face suite names the fonts the pass inlines or empties; a scheme, a `data:` URL
  and a path hold a separator and name nothing. A CSS's image is named too, since a sweep must keep it.
- **Fonts alone:** lost, because a CSS background image would then be swept.

## D6. What a sweep keeps, and the port it reads

- **Chosen: `fate` keeps a file for the first reason that holds: live (a note, a template or a CSS names it), pinned
  (its name starts with `_`, which the engine's check also never reports unused, since a template may load it by a
  name no scan sees), in grace (added within the window, or after the sweep's instant) or imported; and deletes every
  other file. `plan_sweep` applies it to every stored file in name order and lists every name written that no stored
  file has.** The window is read through `i128`, so no instant or window overflows.
- **`MediaIndex` is the port a collection implements:** every note field and card template as HTML (`Source::Html`
  for a note type that renders its LaTeX as PNG, `Source::HtmlSvg` for one that renders SVG), every note type's CSS as
  CSS, and every stored file. The core does no I/O and has no workspace edge; the collection runs its own queries and
  hands the core what they return.
- **Sweeping imported files by reference alone:** lost, because an imported deck may load a file by a name no note
  writes (a template's script, a font), and the deck's files are the deck's.
- With no window, no pin and no import, the plan's deletions are G-L5-2's unused names and its missing names are
  G-L5-2's missing names, in all 64 cases, with no name set aside.
- Lean `MediaGc` proves that a live, pinned, young or imported file is never deleted and every other file is.

## D7. Kani harnesses beside the code, each with its witness

- **Chosen: `crates/media/src/kani_proofs.rs` under `cfg(kani)`, with K-blob-path, K-bind, K-sweep and K-scan over the
  shipped functions, and four witness features (`kani-witness-blob-path`, `-bind`, `-sweep`, `-scan`), each a mutant
  of the shipped code that no default list and no other manifest names.** `cfg(kani)` is declared to the
  `unexpected_cfgs` lint in the workspace's lint table, so `-D warnings` stays clean.
- No Kani run is pinned for this tree yet, so each harness is VOID: unratcheted, and nothing runs.

## D8. A LaTeX image is named as the engine names it

- **Chosen: `latex_images` reads a text's LaTeX as the pinned engine's `extract_latex` does
  (`rslib/src/latex.rs`), and `references` adds each image's name for every HTML text.** The three forms
  (`[latex]…[/latex]`; `[$]…[/$]`, the expression `$…$`; `[$$]…[/$$]`, the expression
  `\begin{displaymath}…\end{displaymath}`) match in any case, at least one character up to the first closing marker,
  tried in that order at each `[`. In the expression `<br>`, `<br/>` and `<div>` become newlines (the engine's
  expression for them ignores its own whitespace, so `<br />` is removed as a tag), every comment, `style` and
  `script` element and other tag is removed, and the entities are decoded by the grammar's decoder. The image is
  `latex-`, the expression's SHA-1 in lowercase hex, and `.png`, or `.svg` for a note type that renders SVG. Without
  these names a sweep would plan to delete every image the engine renders, and the plan would not reproduce G-L5-2.
- **SHA-1 written in the crate (FIPS 180-4), 82 lines of code in `sha1.rs`.** It names what the engine already names
  by it and certifies nothing, so D1's reason against it does not apply. Its test holds it to the FIPS 180 examples
  and to every padding boundary (55, 56, 63, 64, 65, 119 and 120 bytes), each digest computed by Python's `hashlib`.
- **The lock's `sha1` package:** lost. The lock holds `sha1` 0.11.0 for another package, but its first use here would
  be a new admission, with its licence and size measured in its own decision, and the core would gain a second
  dependency for one function.
- **Leaving LaTeX unnamed, a store pinning its images:** lost, because the plan would then miss 16 of G-L5-2's
  missing names, and a store that forgot the pin would delete images the engine keeps.
- **The divergences:**
  - **Case 14 of the LaTeX golden** (`anki.render.latex_images`, 838 cases, 875 images, 404 SVG cases; every other
    case is equal): its expression holds `&copy;` and `&eacute;`, which the engine decodes and the grammar's decoder
    (D4) keeps whole, so the image is named for the text as written.
  - **Not drawn by any golden:** the engine's media check reveals a field's clozes before it reads LaTeX, where its
    renderer, which the golden draws, does not; the core reads an expression as written. The check reads LaTeX in a
    note's fields alone, and the core reads it in every HTML text, a card template's included, so a sweep keeps an
    image a template renders where the engine's check would report it unused.

## Consequences

- One crate decides a media file's identity, its names, their bindings, the names a note and a CSS write, the images
  the engine renders from LaTeX, and what a sweep may delete; the store, the collection and the renderer depend on it
  rather than on the engine.
- A LaTeX image is live while a note writes its expression. An image the engine renders from a revealed cloze inside
  an expression is named otherwise, so a store must pin or import it to keep it through a sweep.
