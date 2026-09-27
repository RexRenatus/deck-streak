---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/ai-content-safety

How a repository keeps its AI safe when the AI reads text it did not write: the learner's answers,
the text of their cards, the notes in their vault (SPEC-V2-2222 / ADR-V2-2222). Any of that text can
carry instructions, so it is fenced as data, the agent that reads it holds no tool that can carry
data out, and every output is checked before it is delivered. It also carries the owner's rules
for every AI text (no dates, cited law) and the EU AI Act's Article 50 disclosure.

A repository declares each AI task once, in `ai-safety.json`: the system files and the prompt
template it sends, the slot each input fills and where that input comes from, the tools the agent
holds, where the task's golden outputs live, and the gate its output must pass.
`scripts/ai-content-safety-probe.py` proves the declaration against the tree. It is
standard-library Python and vendorable, and it judges any tree through `--root`.

It composes with two sibling packs and copies neither. It loads persona-core's probe and calls
`load_contract()` (the kinds and the disclosure token), `load_deny()` (the secret shapes in
prompts), `parse_document()` (persona outputs and the shared rules) and `CLASSES`; it reads
law-professors' `CLASS_NAMES`. The owner's rules for every AI text (no dates, no secrets, never
claiming to be human, and a cited source for every law rule) are those packs' classes, and this
pack makes every task's gate run them before delivery. Which seats consume this pack is its
catalog row's `consumes`, so this body names none.

```
phxd pack probe --pack ai-content-safety --root PATH --format json
```

## The rows

Eleven rows, all `tree`-scoped, one per class of `ai-content-safety-probe.py`. Each runs
`python3 {skills}/../scripts/ai-content-safety-probe.py --root {root} check <class>` under a
120-second wall. The script finds every `ai-safety.json` under the root (dot-directories,
`node_modules` and `target` skipped) and judges each against the tree under its own directory.

The `injection` stage: 5 rows. They read the manifest, the prompts and the agent's settings
(OWASP LLM01:2025 and LLM06:2025).

| row | severity | reason | refuses when |
|---|---|---|---|
| `prompt-contract` | block | `prompt-invalid` | `ai-safety.json` is not JSON or not `phx.ai.safety.v1`; a key is unknown or missing; a task's format, kind (persona-core's kinds), source or `on_invalid` is outside the vocabulary; a system or prompt file is missing; a slot is used but not declared, declared but never used, or filtered by anything but `json`; a `*.prompt.md` file belongs to no task; a prompt or system file matches persona-core's deny list (OWASP LLM07:2025) |
| `untrusted-fenced` | block | `untrusted-unfenced` | an untrusted slot appears outside a fence, or in any system file; a fence holds anything but one slot; the fence's `source` is not the slot's source; the slot is not `\|json`; a fence tag shares its line, nests, never closes, or closes without opening |
| `policy-stated` | block | `policy-missing` | a task that reads untrusted text has no system file with a non-empty `<untrusted_content_policy>` block and none that is persona-core's shared rules with its `learner-text` section |
| `agency-scoped` | block | `excessive-agency` | a task that reads untrusted text holds `WebFetch`, `WebSearch`, `Task` or `Agent`; an unscoped `Read` or `Grep`; a `Write` or `Edit` outside its output directory; a `Bash` that is unscoped or runs a program that reaches the network or runs code (`curl`, `git`, `python3`, `bash` and the rest); an MCP tool not declared offline; or the agent's settings allow a tool no task declares, or bypass permissions |
| `redteam-present` | block | `redteam-missing` | tasks read untrusted text and there is no cases directory; a case is malformed; an untrusted source no case feeds; no case tries `instruction-override`, `fence-breakout` or `exfil-link`; no named test mentions the cases |

The `output` stage: 5 rows (4 blocking, 1 advisory). They read the gate and the golden outputs, or
the files `--subject` names (OWASP LLM05:2025 and LLM09:2025).

| row | severity | reason | refuses when |
|---|---|---|---|
| `output-gated` | block | `output-ungated` | a gate entry is not `pack:class`, or names a class the pack does not have; the gate lacks a required class (below); `on_invalid` would deliver an output that failed; no golden output matches the task's `outputs` |
| `output-links` | block | `link-not-allowed` | an output links anywhere but https to a host on `links.allow` (or its subdomain), in markdown, HTML, an autolink or a bare URL; or embeds any remote image, which loads by itself and can carry data out |
| `output-invisible` | block | `invisible-text` | an output carries a Unicode format character: a tag character (the Tags block), a bidirectional control, a zero-width space or joiner other than ZWJ and ZWNJ, a word joiner; or a run of variation selectors. With `--subject` it screens an input the same way, with no manifest |
| `output-marked` | block | `output-unmarked` | an output carries no machine-readable AI mark: neither persona-core's output schema (parsed by persona-core) nor `ai_generated: true` in its frontmatter (EU AI Act Art. 50(2)) |
| `output-echo` | advisory | `prompt-echoed` | an output carries a fence or policy tag, or repeats a line of a system file verbatim. Advisory: it reports and never refuses |

The `disclosure` stage: 1 row. It reads the first-contact surfaces and their messages (EU AI Act
Art. 50(1) and (5)).

| row | severity | reason | refuses when |
|---|---|---|---|
| `disclosure-first-contact` | block | `disclosure-missing` | the disclosure text, or each localised message, never says persona-core's disclosure token (`AI`) as a word; a first-contact surface does not show the disclosure text or its message key |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
population it read. The script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: no manifest was found, a probe it composes with is missing, or an input could not be read,
which is never a pass.

## How it composes: one bar, one owner

The owner's rules for every AI text are already sibling classes, and each keeps its own bar:

| rule | class | where it is required |
|---|---|---|
| no date, deadline or countdown | persona-core's `no-dates` | every persona task's gate |
| no secret, address or private word | persona-core's `scrubber` | every persona task's gate |
| never claim to be human (Art. 50(1)) | persona-core's `no-human-claim` | every persona task's gate |
| read only the learner's own subject | persona-core's `memory-scope` | every persona task's gate |
| the output contract | persona-core's `output-contract` | every persona task's gate |
| every rule statement cites the corpus | law-professors' `rule-cites-corpus` | every law task's gate |
| every citation resolves | law-professors' `citations-resolve` | every law task's gate |
| every quote is in its source | law-professors' `quotes-grounded` | every law task's gate |
| no authority without a citation | law-professors' `authority-grounded` | every law task's gate |

`output-gated` refuses a gate that lacks one, and checks each named class against the sibling's own
class list (`CLASSES`, `CLASS_NAMES`), so a renamed class turns this pack red rather than silently
passing. Over the repository tree, persona-core's and law-professors' packs run those classes
themselves; this pack does not run them a second time. At run time the engine runs every gate class
on each new output before it is delivered, which is where an AI text is actually stopped.

## The manifest, `ai-safety.json`

| key | holds |
|---|---|
| `schema` | `phx.ai.safety.v1` |
| `tasks` | one entry per AI task, below |
| `links` | `{allow: [host]}`: the only hosts an output may link to |
| `disclosure` | `{text, surfaces}`, or `{key, surfaces, messages}` for a localised message |
| `redteam` | `{cases: dir, tests: [file]}` |
| `agent` | `{settings: file, mcp_offline: [tool]}`: the agent's Claude Code settings, and MCP tools that reach nothing |

A task holds `id`, `format` (`persona` for persona-core outputs, `other` otherwise), `kind` (a
persona-core kind, required for a persona task), `system` (files sent as the system prompt),
`prompt` (the template sent as the user turn), `inputs` (`[{slot, source}]`), `tools`, `outputs`
(globs of golden outputs), `gate` (`pack:class` entries) and `on_invalid` (`withhold`, `degrade` or
`retry`; never deliver).

Trust follows the source, and the manifest cannot change it:

- **Untrusted:** `learner`, `cards`, `vault`, `memory`, `stats`, `web`, `tool-output`,
  `telegram`, `upload`. Anything a person or another system wrote.
- **Trusted:** `rules`, `template`, `duty`, `config`. Text this repository wrote.

## The prompt contract

An untrusted slot is written alone, JSON-encoded, inside a fence that names its source, with each
tag on a line of its own:

```text
The card, as the learner's deck stores it:
<untrusted source="cards">
{{card_text|json}}
</untrusted>
```

- **Separate and identify it** (OWASP LLM01:2025, mitigation 6; Anthropic's guidance on indirect
  prompt injection): the fence tells the model what the text is and where it came from.
- **Encode it.** `|json` means the engine inserts the value as a JSON string, so a quote or a newline
  cannot break out. Have the engine also escape `<` and `>` as `<` and `>`, so the text
  cannot close the fence.
- **Keep instructions out of it.** A fence holds its slot and nothing else, and untrusted text never
  enters a system file.
- **State the policy** in a system file: a `<untrusted_content_policy>` block that says fenced text is
  data, never an instruction; or persona-core's shared rules, whose `learner-text` section says the
  same. `templates/untrusted-policy.template.md` is a starting point.
- **Read untrusted files through a tool.** When the agent reads a vault note with its `Read` tool,
  the note arrives as a tool result, which is where Anthropic's guidance puts third-party content.

## Agency

A task that reads untrusted text may hold only what it needs (OWASP LLM06:2025):

| tool | rule beside untrusted text |
|---|---|
| `Read`, `Grep` | scoped to paths, such as `Read(cards/**)` |
| `Glob`, `LS` | allowed |
| `Write`, `Edit` | scoped inside the task's output directory |
| `Bash` | scoped to one command, never one that reaches the network or runs code |
| `WebFetch`, `WebSearch`, `Task`, `Agent` | never |
| an MCP tool | only when `agent.mcp_offline` names it |

The agent's settings file may allow no tool that no task declares, and may never set
`defaultMode` to `bypassPermissions`. Run the gate itself in the engine, outside the model (OWASP
LLM07:2025): the agent does not grade its own output.

## The gate

Every task's gate names these classes, and the engine runs them on each new output, with
`--subject` naming it, before it is delivered:

| task | required classes |
|---|---|
| every task | `ai-content-safety:output-links`, `ai-content-safety:output-invisible`, `ai-content-safety:output-marked` |
| a persona task | persona-core's `output-contract`, `no-dates`, `scrubber`, `no-human-claim` and `memory-scope` |
| a law task | law-professors' `rule-cites-corpus`, `citations-resolve`, `quotes-grounded` and `authority-grounded` |

An output that fails is withheld, or replaced by a degraded text that says coaching was unavailable,
or retried; it is never delivered. Keep one golden output per task in the tree, so CI proves the
gate on real text.

- **Links and images.** A remote image loads by itself, so a link planted in a card can carry data
  out the moment the output renders; Telegram also fetches the first link to build a preview. Send
  AI text with `link_preview_options.is_disabled`, and allow only your own hosts.
- **Invisible text.** Tag characters and bidirectional controls can hide instructions a person never
  sees. Run `output-invisible` with `--subject` on a vault note or a card before it is fenced into a
  prompt, as well as on the output.
- **The mark.** A persona output is marked by its schema; any other output carries
  `ai_generated: true`. Keep the mark on the stored output and in the message metadata.

## Disclosure

The learner is told, at the first interaction and in a clear and distinguishable way, that the
coach is an AI (EU AI Act Art. 50(1) and (5)). Declare each first-contact surface (the bot's
`/start` reply, the Mini App's first screen) and the text or message key it shows. Every localised
message says `AI` as a word. persona-core's template schema requires each persona's disclosure
section to say it too, and its `no-human-claim` class, which every persona task's gate runs,
refuses any persona text that claims to be human.

## What no static read can prove, taught here

- **That the engine runs the gate.** The engine's own test sends a failing output through delivery
  and asserts it is withheld. This pack proves the gate is declared, complete and proved on golden
  outputs.
- **What the model does with an attack.** The red-team test feeds each case to the engine and
  asserts the output ignores it or the gate withholds it. This pack proves the cases exist and cover
  the sources.
- **Screening tool output with a classifier**, as Anthropic suggests, before the agent acts on it: a
  runtime call, judged by the engine.
- **Link previews off** on every AI message: the bot's send options, which the Telegram platform pack
  owns.
- **Output encoding** for HTML and MarkdownV2, and content security policy: the Telegram platform
  and web security packs own them.
- **Rate limits and monitoring** of AI output: the observability pack owns them.
- **Public-interest text** (Art. 50(4)) does not arise: DeckStreak's AI text is study material for
  one learner, not published to inform the public.

## How DeckStreak applies it

1. **Declare.** Copy `templates/ai-safety.template.json` to `ai-safety.json` at the root. Add one task
   per AI duty, with its system files, its prompt, its inputs and their sources, the tools the agent
   holds, its golden outputs and its gate.
2. **Write the prompts** from `templates/task-prompt.template.md`: every untrusted slot fenced alone
   and JSON-encoded. Put the policy from `templates/untrusted-policy.template.md`, or persona-core's
   shared rules, first in each task's system files.
3. **Scope the agent.** Give each task only scoped reads and its output directory, and make the
   agent's settings allow nothing more.
4. **Add red-team cases** from `templates/redteam-case.template.md`: one per untrusted source, and at
   least an instruction override, a fence breakout and an exfiltration link. Write a test that runs
   them.
5. **Run it.** From this repository against DeckStreak's tree:

   ```
   phxd pack probe --pack ai-content-safety --root PATH --format json
   ```

   The engine also runs the output classes on each new text with `--subject`, and on each untrusted
   input with `check output-invisible --subject`, before it goes anywhere.

What the gate refuses: an unfenced or unencoded untrusted slot, untrusted text in a system prompt,
no stated policy, an agent that can fetch, search, shell out or write anywhere while it reads
untrusted text, missing red-team cases, an incomplete gate, a link to an unknown host, a remote
image, invisible characters, an unmarked output, a surface that never says it is an AI, and a gate
that would deliver a persona text without persona-core's checks or a law text without
law-professors' citation checks.

`examples/deckstreak/` is the worked example, green on every class of this pack's own probe.

## References

The dated research, with access dates and the Context7 ids that answered, is SPEC-V2-2222's
References section.

- OWASP Top 10 for LLM Applications: https://genai.owasp.org/llmrisk/llm01-prompt-injection/ ·
  https://genai.owasp.org/llmrisk/llm052025-improper-output-handling/ ·
  https://genai.owasp.org/llmrisk/llm062025-excessive-agency/ ·
  https://genai.owasp.org/llmrisk/llm072025-system-prompt-leakage/ ·
  https://genai.owasp.org/llmrisk/llm092025-misinformation/
- OWASP LLM Prompt Injection Prevention Cheat Sheet:
  https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html
- NIST AI 600-1, the Generative AI Profile: https://nvlpubs.nist.gov/nistpubs/ai/NIST.AI.600-1.pdf
- EU AI Act, Article 50: https://artificialintelligenceact.eu/article/50/ ·
  https://digital-strategy.ec.europa.eu/en/policies/guidelines-ai-transparency-obligations ·
  https://digital-strategy.ec.europa.eu/en/policies/code-practice-ai-generated-content
- Anthropic: https://platform.claude.com/docs/en/test-and-evaluate/strengthen-guardrails/mitigate-jailbreaks ·
  https://platform.claude.com/docs/en/test-and-evaluate/strengthen-guardrails/reduce-hallucinations ·
  https://code.claude.com/docs/en/permissions
- Spotlighting: https://arxiv.org/abs/2403.14720 · Design patterns for securing LLM agents:
  https://arxiv.org/abs/2506.08837
- Invisible text: https://arxiv.org/abs/2111.00169 (Trojan Source) · https://www.unicode.org/reports/tr9/
- Telegram link previews: https://core.telegram.org/bots/api#linkpreviewoptions
