---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/vault-duties

The checks for DeckStreak's three VAULT duties, run on their OUTPUT before it reaches the owner's
Obsidian vault (SPEC-V2-2218 / ADR-V2-2218):

- the **daily note**: tomorrow's note, linking the day's readings, drills and filed captures;
- the **weekly synthesis**: themes and connections across the week's notes, every claim linked to
  a note it synthesises, and questions left for the owner;
- the **inbox curator**: filing `90-Inbox/` captures into their folders.

The owner chose three duty packs: study, nudges and vault. A duty names WHAT is produced, and a
persona shapes HOW. The owner's decisions this pack encodes:

- Obsidian's own conventions: properties, links and embeds that resolve, and names that fit the
  vault's folder rules.
- Writes stay in the agent's folders: `12-Readings/`, `11-Drills/`, the daily-notes folder, and
  filing out of `90-Inbox/`.
- Agent-written notes are static: no Dataview JavaScript, no Templater, no executable block.
- The weekly synthesis cites the notes it synthesises, with every claim linked.
- The inbox curator never deletes, and never moves outside its target folders.
- The journal is never quoted into any persona or public text, and nothing personal leaves the
  vault.
- No dates and no timelines. A daily note is NAMED in the vault's daily format, and that name is
  the one place a date may stand: its properties and its body carry none, and a link to a
  date-named note carries a date-free alias.
- Blocking for facts, privacy and safety; advisory for heuristics.

`scripts/vault-duties-probe.py` is the executable contract. It is standard-library Python and
vendorable, it judges any tree through `--root`, and every row below runs it. Which seats consume
this pack is its catalog row's `consumes`, the one record of that edge (ADR-V2-1990), so this body
names none.

```
phxd pack probe --pack vault-duties --root PATH --format json
```

## The rows

Fifteen rows, all `tree`-scoped, one per class of `vault-duties-probe.py`. Each runs
`python3 {skills}/../scripts/vault-duties-probe.py --root {root} check <class>` under a
120-second wall. `{skills}` is the skills directory the catalog was read from, so the script and
its data always come from this pack, whatever tree `--root` names.

Stage `obsidian`: 3 rows (3 blocking, 0 advisory). They hold each note to Obsidian's own
conventions.

| row | severity | reason | refuses when |
|---|---|---|---|
| `note-properties` | block | `properties-invalid` | a written note does not parse; it carries no agent schema; a key is unknown, or is the deprecated `tag`, `alias` or `cssclass`; a value is nested; `tags` is not a list of bare tags (no `#`, no space, not all digits); `aliases` or `cssclasses` holds a non-string; `duty` is not a vault duty or not the run's; a synthesis has no `sources`, or a source is not a `[[wikilink]]`; a `##` heading is unmarked or repeated; a template variable is left unfilled |
| `note-links` | block | `link-unresolved` | a wikilink, embed, markdown link or property link resolves to no note or file after the run; a bare name is ambiguous because two notes share it; a `#heading` or `#^block` names one its note does not have, where the note is readable |
| `note-names` | block | `name-off-rule` | a written name holds a reserved character or one that breaks an Obsidian link, is a reserved device name, ends in a period or a space, starts with a period, or is longer than 255 bytes; a note is not `.md`; a periodic note does not fit the vault's daily or weekly format, or is not a calendar day; a name breaks its folder's rule; a filed capture is renamed |

Stage `rails`: 3 rows (3 blocking, 0 advisory). They hold each run to what the agent may do.

| row | severity | reason | refuses when |
|---|---|---|---|
| `write-confinement` | block | `write-outside-agent-folders` | a record does not parse or is malformed; the layout is unsound (it names the inbox as a target, or a folder that overlaps the journal); the duty has no folders; a write lands outside the duty's folders, in a dot-folder, in the journal, or escapes the vault's paths; a move leaves its `moves_from` or enters anything but its `moves_to`, or goes back into the inbox; a duty that files nothing moves; a staged file has no op, or an op has no staged file |
| `no-executable` | block | `executable-content` | a note holds a fence off the allow-list (DataviewJS, Dataview, an embedded search, a Bases view, Tasks, JS Engine, Buttons, Execute Code's `run-` blocks, any programming language), a Templater command anywhere, a Dataview inline query, HTML off the allow-list or an attribute off it, a `javascript:`, `vbscript:`, `data:` or `obsidian:` link, a MathJax `\href` or `\url`, or an embedded `.base` view |
| `never-deletes` | block | `destructive-op` | an op is outside the grammar `create`, `update`, `move`; a create or a move would overwrite an existing note, compared case-insensitively; two ops write one path; an update names a note that is not an agent note, that has changed since the agent wrote it, or that no readable vault can vouch for; a filed capture's bytes change, or its hash is not the inbox snapshot's or the vault's; a capture in the inbox is missing from the snapshot, disappears, or is moved twice |

Stage `duty`: 2 rows (2 blocking, 0 advisory). They hold each duty to its shape.

| row | severity | reason | refuses when |
|---|---|---|---|
| `daily-note` | block | `daily-note-incomplete` | the daily note lacks its `readings`, `drills` or `inbox` section, or one is empty; a note the engine prepared for the day is not linked in the section the run names for it |
| `synthesis-cites` | block | `claim-uncited` | the synthesis declares no sources; a source resolves to no note, or is not one of the run's inputs; a `themes`, `connections` or `questions` section is missing or empty; a claim (a list item or a paragraph of `themes` or `connections`) cites no source, or cites a note that is not a source; a connection joins fewer than two notes; a source is never cited |

Stage `privacy`: 3 rows (3 blocking, 0 advisory). They carry the owner's privacy rules.

| row | severity | reason | refuses when |
|---|---|---|---|
| `journal-never-leaks` | block | `journal-leak` | a journal note was one of the run's inputs; a text links, embeds or tags into the journal; a text repeats eight consecutive words of a journal note; the journal holds notes and no vault is readable to prove none is quoted |
| `nothing-leaves` | block | `vault-egress` | a note sets `publish`, which sends it to Obsidian Publish over any folder filter; an embed or an HTML source is fetched from the web when the note renders; a `file:` link names a path on a machine |
| `no-dates` | block | `date-or-timeline` | a vault-duty note's text, properties included, holds a calendar date or a timeline by persona-core's patterns; a link to a date-named note shows the date because it has no alias |

Stage `practice`: 4 rows (0 blocking, 4 advisory). They report and never refuse.

| row | severity | reason | refuses when |
|---|---|---|---|
| `claim-sentences` | advisory | `sentence-uncited` | a sentence of a synthesis claim carries no link of its own; a trailing run of links belongs to the sentence before it |
| `synthesis-own-words` | advisory | `synthesis-copies-sources` | the synthesis repeats eight consecutive words of one of its sources |
| `orphan-notes` | advisory | `note-orphaned` | a note a run creates or files, other than a periodic note, is linked from no note in the runs judged |
| `inbox-zero` | advisory | `inbox-left` | the curator left captures in the inbox |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
population it read. The script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: nothing was examined, or the pack's data or persona-core's probe could not be read, which is
never a pass. The card turns any non-zero exit of a `block` row red, and prints an advisory row's
failure as `advisory` without reddening the card. The card shows each row's exit; the finding lines
are read by running the class directly.

On phoenix-v2's own tree the rows examine this pack's worked example in `examples/`: a small vault
and one staged run per duty.

## What the engine stages

A vault duty never writes the vault directly. The engine runs it against a STAGING directory, one
per run, and applies the run only when every blocking class is green:

```
<run>/
  duty-run.json        the record: phx.duty.vault.run.v1
  <vault path>         every file the run creates, updates or files, at its vault path
```

`--root` walks for `duty-run.json` and skips `.git`, `node_modules`, `target` and every
dot-directory. Inside a run every file counts, dot-folders included, so a staged `.obsidian/` file
is seen. `--subject` names a run directory, a record, or a directory of texts, and repeats.

### The run record, `phx.duty.vault.run.v1`

| key | required | value |
|---|---|---|
| `schema` | yes | `"phx.duty.vault.run.v1"` |
| `duty` | yes | the duty that ran: `daily-note`, `weekly-synthesis`, `inbox-curator`, or a persona duty that writes the vault, such as `daily-reading` |
| `vault` | yes | every vault path that existed before the run: names only, the index links resolve against |
| `ops` | yes | the operations, in the grammar below |
| `inputs` | for the daily note and the synthesis | what the engine gave the duty: a vault path, or `{"path": ..., "section": ...}` naming the daily-note section that must link it |
| `inbox` | for the curator | the inbox snapshot: `{"path": ..., "sha256": ...}` for every capture before the run |
| `layout` | no | the layout in force, as an object or a path beside the record; the pack's public `layout.json` otherwise |
| `vault_root` | no | the vault itself, as a path from the record, for headings, blocks, updates and the journal |

The op grammar has three verbs and no fourth:

- `{"op": "create", "path": P}`: a new `.md` note at `P`, where nothing exists yet;
- `{"op": "update", "path": P, "sha256_before": H}`: a rewrite of the agent's own note, which
  still hashes to `H`, the bytes the agent last wrote;
- `{"op": "move", "from": F, "to": T, "sha256": H}`: a capture filed byte for byte, under its own
  name.

There is no delete. The engine's executor knows only these three, so a delete cannot be staged,
and a record that names one is refused. That is OWASP's "complete mediation": the model proposes,
and code the model cannot change decides.

### The layout, `phx.duty.vault.layout.v1`

[layout.json](layout.json) is the PUBLIC default. It names only the documented agent folders and
Obsidian's own defaults: new notes at the vault root, and daily notes named `YYYY-MM-DD`. The
owner's layout, with the real daily-notes folder, the filing targets and the journal, is PRIVATE.
The engine writes it into each record's `layout`, or passes `--layout FILE` or
`VAULT_DUTIES_LAYOUT`, and it never enters a public repository.

| key | value |
|---|---|
| `periodic` | `{"daily": {"folder", "format"}, "weekly": {"folder", "format"}}`; a format is Moment.js, as Obsidian's Daily notes plugin writes it, where `[text]` is literal and `/` makes folders |
| `inbox` | the inbox folder: `90-Inbox` |
| `folders` | each agent folder's name rule: `"title"` (the portable-name rules alone), `"any"`, or `{"regex": R}` over the name without `.md` |
| `duties` | per duty, `writes` (folders, or `@daily` and `@weekly`), or `moves_from` and `moves_to` |
| `journal` | the journal's folders, besides persona-core's journal words |

The worked example's own [layout](examples/layout.json) uses date-free periodic formats, so this
pack carries no date.

### The note, `phx.duty.vault.note.v1`

The daily note and the synthesis use persona-core's file format: line 1 `---`, one
`key: <one-line JSON>` per property (valid YAML, so Obsidian reads it as Properties), a closing
`---`, and `## Title <!-- section:<id> -->` sections.

| key | required | value |
|---|---|---|
| `schema` | yes | `"phx.duty.vault.note.v1"` |
| `duty` | yes | `"daily-note"` or `"weekly-synthesis"`, and the run's own duty |
| `tags` | no | a list of bare tags: letters, digits, `_`, `-`, `/` for nesting, and not all digits |
| `aliases` | no | a list of strings |
| `cssclasses` | no | a list of strings |
| `sources` | on a synthesis | a list of quoted wikilinks: the notes it synthesises |
| `x-<slug>` | no | an extension; never nested |

| duty | sections | claims |
|---|---|---|
| `daily-note` | `readings`, `drills`, `inbox` | none |
| `weekly-synthesis` | `themes`, `connections`, `questions` | `themes` and `connections`: each list item or paragraph is one claim |

A persona output staged into the vault, such as a reading, keeps persona-core's own format and
contract. These classes judge where it lands, what it links and what it runs, and leave its
properties to persona-core.

[contract.json](contract.json) holds these vocabularies, [rails.json](rails.json) holds the
rails, and [templates/](templates/) holds the shape of each duty's output.

## The rails, and why each one

An Obsidian vault runs code. Obsidian starts in Restricted mode, and once the owner turns community
plugins on they inherit Obsidian's own access: they read files, reach the network and install
programs. So an agent-written note is static text, and `no-executable` refuses whatever a plugin
would run or render live:

- **DataviewJS** fences and inline `$=` queries run JavaScript with the plugin's file and network
  access.
- **Dataview**, **Tasks**, embedded **search** and **Bases** views are sandboxed, but they render
  other notes live, so no static check can say what the owner will read; a Bases view also edits
  other notes' properties.
- **Templater** runs `<%* %>` JavaScript and system commands, and its "trigger on new file
  creation" setting runs the commands in every NEW file, which is exactly what a note the agent
  writes is. So a `<%` is refused anywhere, fences included.
- **Execute Code** puts a run button on fences in some thirty languages, and its `run-` fences run
  in the preview. So the fence allow-list is plain text only.
- **HTML** is sanitised, but `<iframe>` is allowed and loads a page, so only a short list of
  inline tags passes, with only harmless attributes.
- **`obsidian:` links** act: `new` can overwrite a note, `open` can append to one, and `path` names
  the machine. **`javascript:`**, **`vbscript:`** and **`data:`** links run or carry code.
- **MathJax's `\href`** makes a link inside math, and filtering `javascript:` there needs an
  extension that is opt-in.

A new rail is a row in `rails.json`, never a code branch.

## Privacy

- **Where the journal is.** persona-core's `load_deny()` names it: the public words `journal` and
  `diary`, plus the private list's `journal_paths` (`--deny-list FILE` or
  `PERSONA_CORE_DENY_LIST`), so the owner configures it once for every pack. The private layout's
  `journal` folders add to it. A path segment equal to one of those words, or starting with the
  word and a separator, is the journal.
- **What the journal says.** To prove that no text quotes the journal, the checker reads the
  journal notes LOCALLY, on the box where the vault lives, and keeps only eight-token shingles
  (NFKC, casefolded, one token per CJK character). It prints no journal text and no journal path,
  only the line of the text that repeats it. When the journal holds notes and no vault is readable,
  the class fails closed.
- **Nothing leaves.** The vault syncs to the owner's devices; it is never published. So a note
  carries no `publish` property, which Obsidian Publish honours over any folder filter, and no
  image or page that the note fetches from the web when it renders. An ordinary web link is fine:
  it is fetched only when the owner follows it.

## Composing with the persona and duty packs

- **Dates** are persona-core's. `no-dates` loads persona-core's probe through importlib and calls
  its public `load_patterns()` and `scan()`, the same date and timeline rows every persona text
  answers to. It never copies them.
- **The journal** is persona-core's `load_deny()`, as above.
- **Any text** can be judged for journal leaks: a sibling pack runs `journal-never-leaks` as a row
  with `--subject DIR --vault DIR`, and a directory holding no `duty-run.json` is judged as bare
  texts (`.md`, `.txt`, `.json`, `.html`). Without `--vault` it checks the shapes only (links,
  embeds and tags), which is what a public repository's CI can prove; with `--vault`, on the box,
  it also proves that no text repeats the journal.

## The Python API

Load `scripts/vault-duties-probe.py` with `importlib.util.spec_from_file_location`; it works
whether or not the loader registers the module.

- `parse_note(path) -> Note`, a frozen dataclass: `frontmatter`, `sections` (each with `id`,
  `title`, `line`, `body`), `links` (each with `form`, `embed`, `target`, `subpath`, `alias`,
  `line`, `text`, `scheme`), `tags`, `problems` and `fatal`. It never raises: a fatal defect is
  `fatal`, and the text is still read. Every line number is 1-based.
- `load_run(record, *, layout_override=None, vault_override=None, pack_dir=None) -> Run`: the
  record, the layout in force, the staged files and the notes they hold. It never raises; a record
  that does not parse is `fatal`.
- `load_journal(vault, *, layout=None, deny_list=None) -> Journal`, built once, and
  `journal_leaks(text, journal, *, first_line=1) -> list[str]`: every way a plain text reaches
  into the journal, as `line N: <what>`, never echoing the journal.
- `load_contract()`, `load_rails()`, `load_layout(path)`, `layout_problems(layout)`,
  `compile_format(format)` and `Resolver(paths).resolve(target, source)`.

`vault-duties-probe.py parse FILE` prints a note's or a record's parse as JSON, for a tool in any
language.

## How DeckStreak's engine uses this pack

1. **Confine the agent before it runs.** The engine is Claude Code through the owner's proxy.
   Give the vault duty a permission set that matches the layout: `Edit(path)` allow rules for its
   folders only, a `Read` deny rule on the journal (a Read deny also blocks writing there), and a
   sandbox whose writable paths are the staging directory. Claude Code consults `Edit(path)` and
   `Read(path)` rules for its file tools, and never a `Write(path)` rule.
2. **Stage.** Run the duty against a staging directory. Write `duty-run.json` from what the duty
   did, with the vault index, the inputs, the inbox snapshot and the private layout.
3. **Gate.** Run every blocking class on the run directory, with `--vault` naming the vault and
   `PERSONA_CORE_DENY_LIST` naming the private list. Apply the run only when all are green:
   create, update and move through an executor that knows only those three verbs, each write
   atomic. A red class means the run is discarded and the vault is untouched: it fails closed.
4. **Report.** Show the advisory findings to the owner as suggestions; they never block.
5. **Gate in CI.** Run every class over the public repository's synthetic runs. The journal and
   the private layout are absent there, so the public shapes judge.

What the gate refuses: a note outside the agent's folders or over an existing one; a delete, a
rename or a changed capture; code, a live view or a link that acts; a link that resolves to
nothing; an uncited claim; a link, tag or verbatim run from the journal; a Publish flag or a remote
fetch; a date or a timeline. Heuristics only advise.

## What the engine owns, and this pack teaches

These bind the engine rather than the text, so no output check can prove them:

- **the permission set** that keeps the agent out of the journal and out of every folder but its
  own;
- **atomic writes** into a synced vault, a temporary file and a rename, so a sync never carries a
  half-written note;
- **scheduling**: when each duty runs, and in which order (the curator before the daily note, so
  the daily note can link what was filed);
- **surfacing** the advisory findings, and a capture the curator left behind;
- **liveness**: noticing when the nightly run did not happen.

## References

The dated research is SPEC-V2-2218's References section.

- Obsidian Help: https://obsidian.md/help/properties · https://obsidian.md/help/links ·
  https://obsidian.md/help/embeds · https://obsidian.md/help/tags · https://obsidian.md/help/html ·
  https://obsidian.md/help/plugins/daily-notes · https://obsidian.md/help/uri ·
  https://obsidian.md/help/plugin-security · https://obsidian.md/help/data-storage ·
  https://obsidian.md/help/file-formats · https://obsidian.md/help/publish/publish
- Dataview: https://github.com/blacksmithgu/obsidian-dataview
- Templater: https://github.com/silentvoid13/templater
- Execute Code: https://github.com/twibiral/obsidian-execute-code
- MathJax safe extension: https://docs.mathjax.org/en/latest/options/safe.html
- Moment.js format: https://momentjs.com/docs/#/displaying/format/
- Microsoft, naming files: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
- OWASP LLM06 Excessive Agency: https://genai.owasp.org/llmrisk/llm062025-excessive-agency/
- ALCE, citation recall: https://arxiv.org/abs/2305.14627
- Broder, resemblance and containment:
  https://www.cs.princeton.edu/courses/archive/spr05/cos598E/bib/broder97resemblance.pdf
- Claude Code permissions: https://code.claude.com/docs/en/permissions
- GTD Weekly Review: https://gettingthingsdone.com/wp-content/uploads/2014/10/Weekly_Review_Checklist.pdf
- Zettelkasten: https://zettelkasten.de/introduction/ · Evergreen notes:
  https://notes.andymatuschak.org/Evergreen_notes
- Context7 ids: `/websites/obsidian_md_help`, `/blacksmithgu/obsidian-dataview`,
  `/silentvoid13/templater`, `/obsidianmd/obsidian-api`, `/websites/code_claude`.
