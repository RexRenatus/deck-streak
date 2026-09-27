---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/persona-core

The shared core of DeckStreak's teaching personas (SPEC-V2-2212 / ADR-V2-2212): the persona
TEMPLATE schema, the persona OUTPUT contract, how duty packs and persona packs compose, and the
checks every persona shares. The owner chose "4 packs: core, languages, law, LSAT (Recommended)";
this is the core, and the language, law and LSAT persona packs build to it.

The owner's persona decisions this pack encodes:

- "Named mentor per subject": a name, a teaching personality and a consistent, professional voice
  rooted in the language's culture or the subject's tradition, with no fictional drama.
- "Remembers weak spots": each persona reads the leeches, drill grades and lapse history of ITS
  subject, and never the journal.
- "Public templates, private roster": templates are public and neutral; names, bios, voices, the
  subjects run and every weak spot stay in private configuration and the database.
- "Architect drafts, you approve": this pack drafts no name, bio or roster. A template carries
  SLOTS for them.
- No dates or timelines, anywhere: a persona never states a deadline, an exam date or a countdown.
- Blocking for facts, privacy and safety; advisory for voice and heuristics.

`scripts/persona-core-probe.py` is the executable contract. It is standard-library Python and
vendorable, it judges any tree through `--root`, and every row below runs it. Which seats consume
this pack is its catalog row's `consumes`, the one record of that edge (ADR-V2-1990), so this body
names none.

```
phxd pack probe --pack persona-core --root PATH --format json
```

## The rows

Eight rows, all `tree`-scoped, one per class of `persona-core-probe.py`. Each runs
`python3 {skills}/../scripts/persona-core-probe.py --root {root} check <class>` under a 120-second
wall. `{skills}` is the skills directory the catalog was read from, so the script and its data
always come from this pack, whatever tree `--root` names.

The `schema` stage: 3 rows. They hold templates and outputs to the schema and the contract.

| row | severity | reason | refuses when |
|---|---|---|---|
| `template-schema` | block | `template-invalid` | a claimed file does not parse; a template has an unknown or missing key, a bad slug, subject or `lang`, empty or repeated `duties`, a malformed `sections` map, a missing or empty required section, a roster slot missing from its section (a filled name, bio, voice or personality), a disclosure that never says AI, a slot spelled with spaces, an unknown slot, or an unmarked `##` heading; two templates share an id; a rules file carries a stray key or no section |
| `output-contract` | block | `output-invalid` | an output has an unknown or missing key; its `persona` resolves to no template in the examined population; its `subject` or `lang` is not its template's; its `duty` is off the registry; a language output has no `cefr` or one outside A1 to C2, or another kind carries one; a `memory` token is not `<source>@<subject>`; a source id holds a space; a slot is left unfilled; a heading is unmarked |
| `duty-composition` | block | `duty-sections-missing` | a template lists a duty off the registry, maps sections for a duty it does not list, or never names a duty in its `duties` section; an output's duty is not one of its persona's, or a section its duty or its persona's `sections[duty]` requires is missing or empty |

The `safety` stage: 4 rows. They carry the owner's blocking rules for privacy and safety.

| row | severity | reason | refuses when |
|---|---|---|---|
| `no-dates` | block | `date-or-timeline` | any claimed file holds a calendar date or a timeline, by the 40 rows of `patterns.json` (below) |
| `scrubber` | block | `scrubber-denied` | a frontmatter key at any depth contains one of v9's `scrub_public()` key markers, or a line matches a public value shape, a private pattern or a private literal. A finding names the rule and the line, never the value |
| `memory-scope` | block | `memory-out-of-scope` | a template's `memory` names a source off the list or the journal; an output reads the journal, another subject's memory, or a source its persona does not declare; an output's body links, embeds or tags into the journal |
| `no-human-claim` | block | `human-claim` | any claimed file claims to be human or denies being an AI, in any of six languages |

The `voice` stage: 1 row. It reports and never refuses.

| row | severity | reason | refuses when |
|---|---|---|---|
| `voice` | advisory | `voice-drift` | an output carries a roleplay stage direction, an emoji, an invented personal backstory, or more exclamation marks than one per 40 words (three at least) |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
population it read. The script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: nothing was examined, or the pack's data or a named private deny-list could not be read,
which is never a pass. The card turns any non-zero exit of a `block` row red (exit 4), and prints
the advisory row's failure as `advisory` without reddening the card. The card shows each row's
exit; the finding lines are read by running the class directly.

On phoenix-v2's own tree the rows examine this pack's `rules.md` and its worked example in
`examples/`, and every persona template the sibling persona packs ship.

## The file format

Templates, outputs and the rules share one format.

- UTF-8 markdown. Line 1 is `---`, then the frontmatter, then a closing `---`, then the body.
- Each frontmatter line is `key: <single-line JSON value>`. YAML 1.2 is a superset of JSON, so
  Obsidian's Properties and any YAML reader read the same data, and the engine writes each value
  with a JSON serializer and needs no YAML library.
- Keys are `[a-z][a-z0-9_]*`, plus `x-<slug>` extension keys that take any JSON value. An unknown
  key that is not `x-` is refused, so a misspelt key never passes silently.
- No blank line, comment, duplicate key or multi-line value in the frontmatter.
- A file is CLAIMED when line 1 is `---` and a frontmatter line contains `phx.persona.`. A claimed
  file that fails to parse is a finding, never a skip. The walk skips `.git`, `node_modules`,
  `target` and every dot-directory.

Sections:

- A section is a level-2 heading that ends in a marker: `## <title in any language> <!-- section:<id> -->`,
  where the id is `[a-z0-9]+(-[a-z0-9]+)*`.
- The HTML comment is invisible when rendered (CommonMark raw HTML; hidden in Obsidian's Reading
  view), so a title can follow the CEFR language-of-instruction rule while its id stays machine
  readable.
- A section runs to the next `#` or `##` heading. A `###` heading stays inside it, and a fenced
  code block is never read for headings.
- An unmarked `##` heading is refused, and so is a repeated section id.
- Content before the first section, such as a `#` title or a greeting, is allowed.

## The persona template schema, v1

`schema: "phx.persona.template.v1"`. Name the file `templates/<id>.persona.md`, never `SKILL.md`.

| key | required | value |
|---|---|---|
| `schema` | yes | `"phx.persona.template.v1"` |
| `template` | yes | the template id, a slug, unique across the tree |
| `subject` | yes | `"<kind>/<area>"`; the kind is `language`, `law`, `test-prep` or `general`, as `contract.json` lists; for example `"language/zh"`, `"law/torts"`, `"test-prep/lsat"` |
| `lang` | when the kind is `language` | the BCP 47 tag of the target language, such as `zh-Hans`, `ko`, `ja`, `fr` or `es`; refused on any other kind |
| `duties` | yes | a non-empty, unique list of duty ids from the registry |
| `memory` | yes | a unique subset of `leeches`, `drill-grades` and `lapses`; the journal is never a source |
| `sections` | no | `{"<duty>": ["<section id>", ...]}`: the sections this persona ADDS to that duty's output; every key is one of its `duties` |
| `x-<slug>` | no | an extension |

The body holds seven required sections, each non-empty:

| section id | holds | slots it must carry |
|---|---|---|
| `identity` | who the persona is | `{{name}}`, `{{bio}}` |
| `voice` | how it speaks, and the template's public constraints on that | `{{voice}}` |
| `personality` | how it teaches | `{{personality}}` |
| `method` | the subject's teaching method, written by the template's author | none |
| `memory` | what it reads, in prose | none |
| `duties` | how it shapes each duty; it names every id in `duties` | none |
| `disclosure` | that the persona is an AI tutor; it says `AI` (EU AI Act Art. 50(1)) | `{{name}}` |

The roster slots are exactly `{{name}}`, `{{bio}}`, `{{voice}}` and `{{personality}}`: lowercase,
no inner spaces. The engine fills them from the private roster by plain-text substitution, not
HTML escaping, so a Mustache engine renders them unescaped. Any other `{{...}}` token is refused.
A public template stays neutral: a name, bio, voice or personality written into it removes the
placeholder and is refused.

## The persona output contract, v1

`schema: "phx.persona.output.v1"`. One file per generated text.

| key | required | value |
|---|---|---|
| `schema` | yes | `"phx.persona.output.v1"` |
| `persona` | yes | the template id that produced it; it resolves to a template in the examined population |
| `subject` | yes | the template's `subject` |
| `duty` | yes | one duty id, listed in the template's `duties` |
| `cefr` | when the kind is `language` | `A1`, `A2`, `B1`, `B2`, `C1` or `C2`: that language's live Road-to-C2 band; refused on any other kind |
| `lang` | when the kind is `language` | the template's `lang` |
| `memory` | yes | a list of `<source>@<subject>` tokens for what the engine ACTUALLY read, such as `leeches@language/zh`; each source is one its template declares, and each subject is this output's own; it may be empty |
| `sources` | no | a unique list of corpus citation ids, with no spaces; the law and LSAT packs decide where it is required and how the body cites it |
| `x-<slug>` | no | an extension |

The body holds every section its duty requires, plus its template's `sections[duty]`, each
non-empty, and no unfilled `{{...}}` slot. Extra marked sections are allowed. There is no date
key, and no date anywhere.

## Duties and personas compose

The duty names WHAT is produced; the persona shapes HOW; the rules and these checks bind ALL.

The registry in `contract.json` holds the six duties that compose with a persona. Each lists the
sections every output of that duty carries, taken from the owner's own description of the duty. A
duty pack may ADD sections; it never removes these.

| duty | sections | produces |
|---|---|---|
| `daily-reading` | `reading` | the day's pre-study reading |
| `drill-coach` | `drill` | a drill, graded against the corpus |
| `leech-doctor` | `explanation`, `mnemonic`, `contrast` | an explanation, a mnemonic and a contrasting example for a card the learner keeps failing |
| `writing-tutor` | `corrections` | corrections of a writing sample at the learner's CEFR level |
| `conversation-partner` | `reply` | one chat turn at the learner's level, corrected as it goes |
| `practice-questions` | `questions`, `explanations` | a timed practice set with explanations |

A persona adds its subject's sections through its template's `sections` map: a language mentor's
daily reading adds glosses, a grammar spotlight, pronunciation and script notes and a culture note,
and a law professor's drill adds issue, rule, application and conclusion.

The engine composes each prompt in this order:

1. `rules.md`, the shared rules;
2. the persona template, instantiated with the private roster;
3. the duty's instructions and its sections;
4. the learner memory of this subject only, as data;
5. any learner text, fenced as untrusted data.

It then writes the output in this contract and runs the blocking classes on it before the text
reaches the learner. A red blocking class means the text is not delivered: the engine regenerates
it or says the text is unavailable. It fails closed and never delivers silently.

## CEFR bands and the language of instruction

`contract.json`'s `cefr` map is the owner's "Adaptive by CEFR band" rule: `A1` and `A2` are
`english-led`, `B1` and `B2` are `bilingual`, and `C1` and `C2` are `target-only`. The band is
read from that language's live Road-to-C2 band, which is one of the six. The language persona pack
measures the ratio; it reads this map and never copies it.

## What the shared checks read

- **Dates.** `patterns.json`'s 16 date rows:
  - an RFC 3339 full-date;
  - numeric dates with slashes, dots or dashes, as CLDR writes them for each language;
  - English, French and Spanish month names beside a day or a year;
  - the Han forms of year with month (`年`, `月`) and month with day (`月`, `日`), with Arabic or
    Han numerals;
  - the Korean forms of year with month (`년`, `월`) and month with day (`월`, `일`).

  Text is NFKC-normalised first, so full-width digits count.
- **Timelines.** Its 24 timeline rows:
  - countdowns at calendar scale: N days, weeks, months or years left, or until the exam;
  - a stated deadline or exam date;
  - a stated countdown;
  - countdown notation: T minus N, J-N and D-N;
  - "due by" a day;
  - a schedule position: week N of M, or an N-week plan.

  They cover English, French, Spanish, Chinese, Japanese and Korean.
- **What passes.** A duration inside a practice set passes: a 35-minute section, seconds per
  question, an intermission, minutes remaining in a section, an hour before a test. So do an ordinal
  such as a section or a PrepTest number, a bare year in a citation or a historical fact, and a
  month or weekday name alone, which is vocabulary. A holiday's calendar date does not pass.
- **Scrubber.** `deny-list.json` holds the public half:
  - v9's `scrub_public()` key markers;
  - its value shapes: IPv4 addresses and secret-token shapes;
  - hostnames that encode an IP address;
  - GitHub, Google, AWS, Slack and Telegram bot-token shapes;
  - Telegram supergroup ids;
  - private-key headers;
  - email addresses outside the RFC 2606 and RFC 6761 example domains.

  Box paths, project ids, deck names and personal words are private. They live only in a private
  list with the same schema, named by `--deny-list FILE` or the `PERSONA_CORE_DENY_LIST`
  environment variable. That list is merged over the public one, and its literals are reported by
  index, never by value.
- **Memory.** The declared reads, and every wikilink, embed, markdown link and tag in an output's
  body whose path segment is a journal name: `journal` or `diary`, plus the private list's
  `journal_paths`. The word "journal" in prose is not a link, so French prose passes.
- **Human claims.** An explicit "I am human" or an explicit denial of being an AI, in six
  languages. In Chinese, Japanese and Korean only AI denials are read, because "I am Japanese"
  names a nationality with the same character.
- **Voice.** Stage directions such as an asterisk-wrapped smile, emoji, exclamation bursts, and
  invented childhood or family stories, in six languages.

A new language or a new pattern is a row in `patterns.json` or `deny-list.json`, never a code
branch. CJK rows carry no word boundary, because a CJK character is a word character to Python's
`re` and no boundary falls inside a CJK run.

## The Python API

A sibling persona pack parses through this API, never through a parser of its own. Load
`scripts/persona-core-probe.py` with `importlib.util.spec_from_file_location`; it works whether or
not the loader registers the module in `sys.modules`.

1. `parse_document(path: pathlib.Path | str) -> Document`. `Document` is a frozen dataclass:
   - `path: Path`;
   - `kind: str`: `"template"`, `"output"` or `"rules"`, from the `schema` value;
   - `frontmatter: dict[str, object]`: the JSON-decoded values, in file order;
   - `sections: tuple[Section, ...]`: in file order;
   - `body: str`: everything after the closing `---`;
   - `body_line: int`: the 1-based line number of the first body line;
   - `text: str`: the whole file;
   - `problems: tuple[str, ...]`: the non-fatal structural problems, such as an unmarked `##`
     heading or a repeated or malformed section id. `output-contract` and `template-schema` report
     them; `parse_document` does not raise on them.

   `Section` is a frozen dataclass:
   - `id: str`;
   - `title: str`: the visible heading text, with the marker removed;
   - `line: int`: the 1-based line of the heading;
   - `body: str`: the lines up to the next `#` or `##` heading, joined with `"\n"`.

   Every line number is 1-based. On a fatal failure `parse_document` RAISES `ContractError`, a
   `ValueError` subclass whose `str()` is the reason: unreadable or not UTF-8, no `---` on line 1,
   unclosed frontmatter, a line that is not `key: <JSON>`, invalid JSON, a duplicate key, or a
   missing or unknown `schema`. It never returns `None`.
2. `load_contract(pack_dir: pathlib.Path | None = None) -> dict` reads `<pack_dir>/contract.json`.
   When `pack_dir` is `None`, it resolves relative to the script, to
   `Path(__file__).resolve().parents[1] / "skills" / "packs" / "persona-core"`. That is
   `{skills}/packs/persona-core` when the script is loaded from `{skills}/../scripts/`.
   `load_patterns(pack_dir=None)` does the same for `patterns.json`. The command line takes
   `--pack-dir DIR` for a vendored layout. A missing or malformed file raises `ContractError`.
3. Also exported:
   - `claimed(path) -> bool`;
   - `discover(bases) -> list[Path]`;
   - `subject_kind(subject) -> str | None`, the part before `/` when it is a known kind shape.
4. `load_deny(pack_dir: pathlib.Path | None = None, private: pathlib.Path | None = None) -> dict`
   merges the public `deny-list.json` with a private list of the same schema. It returns four
   lists, and each entry is tagged with its origin, `"public"` or `"private"`:
   - `key_markers`: `(origin, marker)`, casefolded;
   - `patterns`: `(origin, id, compiled re.Pattern)`;
   - `literals`: `(origin, literal)`, casefolded;
   - `journal_paths`: `(origin, word)`, casefolded.

   `private=None` reads the public half alone. `load_deny` does not read the environment: the
   command line resolves `--deny-list`, then `PERSONA_CORE_DENY_LIST`, and a caller does the same.
   An unreadable or malformed file raises `ContractError`.
5. `scan(text: str, rows, first_line: int = 1) -> list[tuple[int, str, str]]` is the per-line NFKC
   scan every class shares. `rows` are `(id, compiled re.Pattern)` pairs, as `load_patterns()`
   returns each group. Each hit is `(line, id, matched text)`, with 1-based lines counted from
   `first_line`.
6. The frontmatter key is literally `schema`. Its values are `phx.persona.template.v1`,
   `phx.persona.output.v1` and `phx.persona.rules.v1`.

`persona-core-probe.py parse FILE` prints the same parse as JSON, for a tool in any language.

## How a sibling persona pack reuses the checks

It reuses them and never copies them. A row in its own `checks.json` runs a class of this script,
scoped to its subject kind:

```
["python3", "{skills}/../scripts/persona-core-probe.py", "--root", "{root}", "--kind", "language", "check", "no-dates"]
```

- `--kind` keeps only files whose subject is that kind, and `--subject DIR` replaces the walk base.
  Both repeat.
- A kind that ships templates and no example output has a non-empty population for:
  - `template-schema`;
  - `duty-composition`;
  - `no-dates`;
  - `scrubber`;
  - `memory-scope`;
  - `no-human-claim`.

  `output-contract` and `voice` read outputs only, so they are VOID for it.
- The pack reads the CEFR map, the duty registry and the kinds from `contract.json`.
- It puts its own section ids in each template's `sections` map.
- It avoids v9's key markers (`date`, `exam`, `note`, `deck`, `target`, `journal`, `schedule`) in
  any `x-` key.
- It builds its broken fixtures in temporary directories and never commits them. The walk from
  `--root` would find them and turn this pack red.

## How DeckStreak's engine uses this pack

1. **Vendor.** Copy `scripts/persona-core-probe.py` and this pack's `contract.json`,
   `patterns.json`, `deny-list.json` and `rules.md`, and the persona packs' templates. Or run the
   pinned pack runner against the repository with `--root`.
2. **Instantiate.** Fill a template's four roster slots from the private roster. The roster and
   the private deny-list stay outside the public repository.
3. **Generate.** Compose the prompt in the order above, and write each text in the output contract,
   declaring in `memory` exactly the reads it made. The engine's own test asserts that the declared
   reads equal the actual ones, because a static check can read only what is declared.
4. **Gate before delivery.** Run the blocking classes on each new text, with `--subject` naming the
   text and the templates, and `PERSONA_CORE_DENY_LIST` naming the private list. Deliver only a
   green text.
5. **Gate in CI.** Run every class on the public repository's templates and its synthetic golden
   outputs. The private list is absent there, so the public shapes judge.

What the gate refuses: a date or a countdown; a secret shape or a private word; a read outside the
persona's subject, or of the journal; a claim to be human; a template that carries a real name; a
text missing its duty's sections. Voice drift only advises.

## References

The dated research is SPEC-V2-2212's References section.

- Council of Europe, CEFR Companion Volume:
  https://rm.coe.int/common-european-framework-of-reference-for-languages-learning-teaching/16809ea0d4
- CommonMark 0.31.2: https://spec.commonmark.org/0.31.2/
- YAML 1.2.2: https://yaml.org/spec/1.2.2/
- Mustache: https://github.com/mustache/spec
- Obsidian: https://help.obsidian.md/properties · https://help.obsidian.md/syntax · https://obsidian.md/help/html
- RFC 3339: https://www.rfc-editor.org/rfc/rfc3339.html
- RFC 2606: https://www.rfc-editor.org/rfc/rfc2606.html
- Unicode CLDR dates (TR35): https://www.unicode.org/reports/tr35/tr35-dates.html
- EU AI Act, Article 50: https://artificialintelligenceact.eu/article/50/ ·
  https://digital-strategy.ec.europa.eu/en/faqs/transparency-obligations-under-article-50-ai-act
- GDPR, Articles 5 and 25: https://gdpr-info.eu/art-5-gdpr/ · https://gdpr-info.eu/art-25-gdpr/
- ICO, data minimisation:
  https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-protection-principles/a-guide-to-the-data-protection-principles/data-minimisation/
- OWASP Top 10 for LLM Applications: https://genai.owasp.org/llm-top-10/
- NIST AI 600-1: https://nvlpubs.nist.gov/nistpubs/ai/NIST.AI.600-1.pdf
- UNESCO guidance for generative AI in education:
  https://www.unesco.org/en/articles/guidance-generative-ai-education-and-research
- Anthropic's guardrail guides:
  https://platform.claude.com/docs/en/test-and-evaluate/strengthen-guardrails/increase-consistency ·
  https://platform.claude.com/docs/en/test-and-evaluate/strengthen-guardrails/reduce-hallucinations ·
  https://platform.claude.com/docs/en/test-and-evaluate/strengthen-guardrails/mitigate-jailbreaks
- Pedagogical agents and personas:
  - https://dl.acm.org/doi/10.1145/258549.258797 (the persona effect)
  - https://journals.sagepub.com/doi/10.2190/EC.49.1.a (a meta-analysis)
  - https://www.sciencedirect.com/science/article/abs/pii/S1071581907001267 (politeness)
  - https://www.sciencedirect.com/science/article/abs/pii/S0360131510000539 (stereotypes)
  - https://arxiv.org/abs/2402.10962 (persona drift)
- Learner models and privacy: https://doi.org/10.1007/s40593-015-0090-8 (open learner models) ·
  https://dl.acm.org/doi/10.1145/2883851.2883893 (DELICATE)
- Anki leeches: https://docs.ankiweb.net/leeches.html
- Rust YAML crates: https://rustsec.org/advisories/RUSTSEC-2025-0068.html ·
  https://github.com/dtolnay/serde-yaml
- Context7 ids:
  - `/websites/spec_commonmark_0_31_2`;
  - `/yaml/yaml-spec`;
  - `/mustache/spec`;
  - `/websites/obsidian_md_help`;
  - `/json-schema-org/json-schema-spec`;
  - `/ankitects/anki-manual`;
  - `/websites/platform_claude_en`;
  - `/python/cpython`.
