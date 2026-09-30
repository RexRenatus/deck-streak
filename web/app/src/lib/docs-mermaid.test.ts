/**
 * SPEC-195, ADR-195. Every fenced mermaid block under docs/ must parse with Mermaid's own parser,
 * so a diagram that would render as an error box is refused before merge. The parser sanitises
 * labels through the document, hence jsdom.
 *
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { type Node, Parser } from 'commonmark';
import mermaid from 'mermaid';
import { describe, expect, it } from 'vitest';
import { GRAMMAR, digestOf, fenceMembers } from './docs-mermaid-fences.js';

/**
 * The repository's `docs/`, found by walking up to the workspace root. A fixed `../../../../` would
 * miss it when StrykerJS runs this file from its sandbox, which sits below `web/app/.stryker-tmp`.
 */
function docsDir(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return join(dir, 'docs');
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

const DOCS = docsDir(import.meta.dirname);

/** Every Markdown file under `dir`, in path order. */
function markdownFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) return markdownFiles(path);
      return entry.name.endsWith('.md') ? [path] : [];
    });
}

/**
 * A line that opens a `mermaid` fence in any container: blanks, `>` and list markers, three or more
 * backticks or tildes, blanks, `mermaid`. It is not the reader; it cross-checks the reader on the
 * documents.
 */
const OPENER_LINE = /^[ \t>*+\-0-9.)]*(?:\x60{3,}|~{3,})[ \t]*mermaid(?![^ \t])/;

interface Block {
  name: string;
  line: number;
  source: string;
}

/**
 * commonmark.js 0.31.2's state while it opens a block, which the fence offset below reads: its block
 * starts (a block quote, an ATX heading, then a fenced code block, ...), the block just opened, and
 * the offsets into the line.
 */
interface ParserState {
  blockStarts: ((parser: ParserState, container: Node) => number)[];
  tip: Node & { _fenceOffset: number };
  offset: number;
  nextNonspace: number;
}

const FENCED_CODE = 2;

/**
 * A CommonMark parser that opens a fenced code block as GitHub's cmark-gfm does. CommonMark counts a
 * fence's indentation in columns and cmark-gfm in characters, so when a container prefix consumes
 * part of a tab, GitHub's block keeps the tab's remaining columns on each line. The wrapper runs the
 * fenced code start and then sets the offset cmark-gfm would.
 */
function gfmParser(): Parser {
  const parser = new Parser();
  const state = parser as unknown as ParserState;
  const starts = [...state.blockStarts];
  const fenced = starts[FENCED_CODE];
  starts[FENCED_CODE] = (current, container) => {
    const { offset, nextNonspace } = current;
    const started = fenced(current, container);
    if (started === 2) current.tip._fenceOffset = nextNonspace - offset;
    return started;
  };
  state.blockStarts = starts;
  return parser;
}

/** The word GitHub keys a diagram on: the info string up to its first ASCII blank. */
function language(info: string | null): string {
  return (info ?? '').split(/[ \t\n\v\f\r]/)[0];
}

/**
 * The `mermaid` blocks of one file, each named `<file> block <n>` counted from 1: every fenced code
 * block, in any container, whose language word is `mermaid` and whose text is not blank, with the
 * text GitHub renders as the diagram. A blank one GitHub shows as code, so it is not a block.
 */
function blocksOf(name: string, text: string): Block[] {
  const blocks: Block[] = [];
  const walker = gfmParser().parse(text).walker();
  for (let step = walker.next(); step; step = walker.next()) {
    const { node } = step;
    const source = node.literal ?? '';
    if (step.entering && node.type === 'code_block' && language(node.info) === 'mermaid' && /[^ \t\n\v\f\r]/.test(source)) {
      blocks.push({ name: `${name} block ${blocks.length + 1}`, line: node.sourcepos[0][0], source });
    }
  }
  return blocks;
}

/** Whether Mermaid's own parser accepts the diagram. */
async function parses(source: string): Promise<boolean> {
  return (await mermaid.parse(source, { suppressErrors: true })) !== false;
}

/** Reports how many of a population were examined, and refuses an empty one. */
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: nothing was judged`).toBeGreaterThan(0);
  return items;
}

/**
 * GitHub's rendering of every generated fence member, recorded by
 * `web/app/scripts/record-docs-mermaid-fences.js` from the grammar in `docs-mermaid-fences.js`: the
 * diagram sources of each member (none when GitHub shows it as code), as indices into `sources`.
 */
interface Truth {
  grammar: unknown;
  members: number;
  digest: string;
  sources: string[];
  renders: Record<string, number[]>;
}

const TRUTH: Truth = JSON.parse(readFileSync(resolve(import.meta.dirname, 'docs-mermaid.fences.json'), 'utf8'));

/** The size of the generated fence population, so a shrunken grammar is visible in the diff. */
const FENCE_MEMBERS = 3241;

/** Every generated member GitHub renders as a diagram, with the diagram sources GitHub renders. */
function renderedMembers() {
  const members = fenceMembers(GRAMMAR);
  expect(TRUTH.grammar, 'the grammar is not the one recorded: run the refresh script').toEqual(GRAMMAR);
  expect(members.map((member) => member.id)).toEqual(Object.keys(TRUTH.renders));
  expect(digestOf(members), 'the members are not the ones recorded: run the refresh script').toBe(TRUTH.digest);
  expect(members.length).toBe(FENCE_MEMBERS);
  expect(TRUTH.members).toBe(FENCE_MEMBERS);
  return members.map((member) => ({ ...member, rendered: TRUTH.renders[member.id].map((at) => TRUTH.sources[at]) }));
}

const BLOCKS = markdownFiles(DOCS).flatMap((file) =>
  blocksOf(relative(DOCS, file), readFileSync(file, 'utf8'))
);

describe('the Mermaid diagrams under docs', () => {
  it('reads every fenced block', () => {
    examined('mermaid blocks', BLOCKS);
    const read = new Set(BLOCKS.map((block) => `${block.name.replace(/ block \d+$/, '')}:${block.line}`));
    const unread = markdownFiles(DOCS).flatMap((file) =>
      readFileSync(file, 'utf8')
        .split('\n')
        .flatMap((line, at) => (OPENER_LINE.test(line) ? [`${relative(DOCS, file)}:${at + 1}`] : []))
        .filter((opener) => !read.has(opener))
    );

    expect(BLOCKS.length).toBeGreaterThan(100);
    expect(unread).toEqual([]);
  });

  it('parses every block', async () => {
    const refused: string[] = [];
    for (const block of examined('mermaid blocks', BLOCKS)) {
      if (!(await parses(block.source))) refused.push(block.name);
    }

    expect(BLOCKS.length).toBeGreaterThan(100);
    expect(refused).toEqual([]);
  }, 60_000);

  it('refuses a block whose node id is a reserved word', async () => {
    const planted = blocksOf('planted.md', '```mermaid\nflowchart TD\n  call --> done\n```\n');

    expect(planted.map((block) => block.name)).toEqual(['planted.md block 1']);
    expect(await parses(planted[0].source)).toBe(false);
  });

  it('accepts the same block once its node id is not a reserved word', async () => {
    const planted = blocksOf('planted.md', '```mermaid\nflowchart TD\n  caller --> done\n```\n');

    expect(await parses(planted[0].source)).toBe(true);
  });

  it('reads an indented fence and refuses one that does not parse', async () => {
    const inList = blocksOf('planted.md', '- item\n\n  ```mermaid\n  flowchart TD\n    call --> done\n  ```\n');
    const indented = blocksOf('planted.md', '   ```mermaid\n   flowchart TD\n     call --> done\n   ```\n');

    expect(inList.map((block) => block.name)).toEqual(['planted.md block 1']);
    expect(await parses(inList[0].source)).toBe(false);
    expect(indented.map((block) => block.name)).toEqual(['planted.md block 1']);
    expect(await parses(indented[0].source)).toBe(false);
  });

  it('accepts an indented valid block of each diagram type the docs use', async () => {
    const sources = [
      'sequenceDiagram\n  Alice->>Bob: hello\n',
      'flowchart TD\n  caller --> done\n',
      'stateDiagram-v2\n  [*] --> Idle\n  Idle --> [*]\n'
    ];
    for (const source of sources) {
      const text = `1. step\n\n   \`\`\`mermaid\n${source.replace(/^(.)/gm, '   $1')}   \`\`\`\n`;
      const planted = blocksOf('planted.md', text);

      expect(planted.map((block) => block.name)).toEqual(['planted.md block 1']);
      expect(await parses(planted[0].source)).toBe(true);
    }
  });

  it('reads exactly the fences GitHub renders as diagrams, in every generated container form', () => {
    const members = renderedMembers();
    console.log(`examined ${members.length} generated container forms`);
    const escaped = members.flatMap((member) => {
      const read = blocksOf('planted.md', member.text).map((block) => block.source);
      return JSON.stringify(read) === JSON.stringify(member.rendered)
        ? []
        : [`${member.id} ${JSON.stringify(member.text)}: read ${JSON.stringify(read)}, GitHub renders ${JSON.stringify(member.rendered)}`];
    });

    expect(members.length).toBe(FENCE_MEMBERS);
    expect(escaped.slice(0, 3), `${escaped.length} of ${members.length} members read otherwise`).toEqual([]);
  });

  it('refuses a grammar whose body names no row, and says which', () => {
    expect(() => fenceMembers({ ...GRAMMAR, body: ['no-such-row'] })).toThrow('no body named no-such-row');
    expect(fenceMembers().length).toBe(FENCE_MEMBERS);
  });

  it('refuses every generated container form planted unparsable by name, and accepts it planted valid', async () => {
    const verdicts = new Map<string, boolean>();
    const verdict = async (source: string) => {
      if (!verdicts.has(source)) verdicts.set(source, await parses(source));
      return verdicts.get(source);
    };
    const planted = renderedMembers().filter((member) => member.rendered.length > 0 && member.body !== 'lazy');
    const wrong: string[] = [];
    for (const member of examined('generated container forms planted', planted)) {
      const refused: string[] = [];
      for (const block of blocksOf('planted.md', member.text)) {
        if (!(await verdict(block.source))) refused.push(block.name);
      }
      const expected = member.body === 'broken' ? ['planted.md block 1'] : [];
      if (JSON.stringify(refused) !== JSON.stringify(expected)) wrong.push(`${member.id}: refused ${JSON.stringify(refused)}`);
    }

    expect(planted.length).toBeGreaterThan(0);
    expect(wrong.slice(0, 3), `${wrong.length} of ${planted.length} members judged otherwise`).toEqual([]);
  });
});
