# The prune pin's token groups and their Lean proof

Kind: data flow. SPEC-469, ADR-486, issue #474. Every `path:line` is read at
`36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

A source text enters a pin, loses one byte order mark for the shebang check only, and is refused or
lexed. The prune pin hands its one token list to two counts, which skip the same doc attribute
groups. The Lean entry `lean/AgentPinGroups` ports each step; its vectors are read back by one test
in each pin's file.

```mermaid
flowchart TD
  src["source text"]
  subgraph prune["prune pin: crates/agent/tests/runs.rs"]
    pp["prune_pin_problems, runs.rs:165-199"]
    sb["opens_with_a_shebang, runs.rs:50-53: one U+FEFF removed, then #! and not #!["]
    lex["tokens_of, runs.rs:38-43: the source as handed in"]
    toks["one token list, runs.rs:172"]
    doc["is_doc_attribute, runs.rs:85-93: a bracket group after # or #!, first token doc or r#doc"]
    run["statements_run_in, runs.rs:145-160: a doc group is 0, else descend"]
    word["code_texts, runs.rs:97-108: descend unless a doc group"]
    words["the word delete counted, runs.rs:185-192"]
    refused["refused: the shebang problem alone"]
    nolex["refused: the source does not lex"]
  end
  subgraph verdict["verdict pin: crates/agent/tests/verdict.rs"]
    vp["verdict_pin_problems, verdict.rs:156-185"]
    vsb["opens_with_a_shebang, verdict.rs:22-25: the same body"]
    decl["declarations, verdict.rs:45-59: every group, doc groups included"]
  end
  src --> pp --> sb
  sb -- "true" --> refused
  sb -- "false" --> lex
  lex -- "error" --> nolex
  lex -- "tokens" --> toks
  toks --> run
  toks --> word
  run -. "asks" .-> doc
  word -. "asks" .-> doc
  word --> words
  src --> vp --> vsb
  vsb -- "false" --> decl
```

## The Lean port, step by step

```mermaid
flowchart LR
  subgraph lean["formal/lean/Formal/AgentPinGroups.lean"]
    l_sb["opensWithAShebang"]
    l_pin["prunePinProblems: lex is any function"]
    l_doc["isDocAttribute, isPunct, nameOf, tokensIn"]
    l_run["statementsRunIn: count and groups"]
    l_word["codeTexts: texts and groups"]
    t1["theorem a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one"]
    t2["theorem the_run_count_and_the_word_count_descend_into_the_same_groups"]
    w1["witness a_check_that_reads_the_byte_order_mark_misses_a_shebang (S46901)"]
    w2["witness a_word_count_that_reads_doc_attributes_parts_from_the_run_count (S46902)"]
  end
  writer["formal/lean/Formal/AgentPinGroupsVectors.lean"]
  vec["formal/vectors/agent-pin-groups.jsonl: 96 shebang, 576 count vectors"]
  a1["A1: runs::the_prune_pin_answers_every_vector_of_its_lean_port"]
  a2["A2: verdict::the_verdict_pins_shebang_check_answers_every_vector_of_its_lean_port"]
  l_pin --> l_sb
  l_pin --> l_run
  l_pin --> l_word
  l_run -. "asks" .-> l_doc
  l_word -. "asks" .-> l_doc
  l_pin --> t1
  l_run --> t2
  l_word --> t2
  w1 -. "breaks" .-> t1
  w2 -. "breaks" .-> t2
  lean --> writer --> vec
  vec --> a1
  vec --> a2
```

| step | code | port | held by |
|---|---|---|---|
| one byte order mark removed, then `#!` and not `#![` refused | `runs.rs:50-53`, `verdict.rs:22-25` | `opensWithAShebang` | theorem 1; witness 1; shebang vectors; A1, A2; rows S46901, S46903 |
| refused before any lex | `runs.rs:166-171` | `prunePinProblems` | theorem 1 |
| one token list to both counts | `runs.rs:172-184` | `prunePinProblems` | the port's shape; A1 |
| a doc attribute group | `runs.rs:85-93`, `runs.rs:61-65`, `runs.rs:79-81` | `isDocAttribute`, `nameOf`, `isPunct` | theorem 2; count vectors |
| the run count's descent | `runs.rs:145-160` | `statementsRunIn` | theorem 2; A1; row S46904 |
| the word count's descent | `runs.rs:97-108` | `codeTexts` | theorem 2; witness 2; A1; row S46902 |
| a call to the query macro | `runs.rs:111-138` | `endsInTheQueryMacro`, `opensWithTheStatement` | count vectors; A1; row S46904 |
| a literal's cooked text | `runs.rs:69-76` | `textOf`, over the token's kind and cooked text | count vectors; A1 |

The lexer is not ported: the pin's port takes any function from text to an optional token list, so
both theorems hold whatever tokens a lexer returns. What rustc does after it lexes is not read
(#444). The verdict pin's `declarations` is not ported (#474 names the prune pin's two counts).
