/**
 * SPEC-195, ADR-195. Every fenced mermaid block under docs/ must parse with Mermaid's own parser,
 * so a diagram that would render as an error box is refused before merge. The parser sanitises
 * labels through the document, hence jsdom.
 *
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import mermaid from 'mermaid';
import { describe, expect, it } from 'vitest';
import { GRAMMAR, digestOf, fenceMembers } from './docs-mermaid-fences.js';
import { readMermaid } from './docs-mermaid-read.js';

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

interface Block {
  name: string;
  line: number;
  source: string;
}

/**
 * The `mermaid` blocks of one file, each named `<file> block <n>` counted from 1: the blocks GitHub
 * renders as diagrams, as `docs-mermaid-read.js` reads them, with the text of each.
 */
function blocksOf(name: string, text: string): Block[] {
  return readMermaid(text).blocks.map((block, at) => ({ name: `${name} block ${at + 1}`, ...block }));
}

/** Each form of one file the reader refuses, named `<file>:<line> <form>`. */
function refusedOf(name: string, text: string): string[] {
  return readMermaid(text).refused.map((refusal) => `${name}:${refusal.line} ${refusal.form}`);
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
const FENCE_MEMBERS = 8994;

/**
 * The generated members the reader reads without refusing any form, so a reader that refuses more or
 * fewer of them is visible in the diff.
 */
const READ_MEMBERS = 2934;

/**
 * Which members those are, by `digestOf`, so a reader that reads one member more and another one fewer,
 * leaving the count as it was, fails too.
 */
const READ_DIGEST = '14cd449a4ec4d03cfafbe1e1ca12d6937c9721b98bd901fe32589f95023313da';

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

/**
 * Every `mermaid` block under `docs/`. It is read inside the test that asks for it, not when the file
 * loads, so a reader that throws fails that test by name rather than the file's import, which the
 * mutation run would count as no test failing.
 */
function docsBlocks(): Block[] {
  return markdownFiles(DOCS).flatMap((file) => blocksOf(relative(DOCS, file), readFileSync(file, 'utf8')));
}

describe('the Mermaid diagrams under docs', () => {
  it('reads exactly the fences GitHub renders as diagrams, or refuses by name, in every generated container form', () => {
    const members = renderedMembers();
    console.log(`examined ${members.length} generated container forms`);
    const read = members.filter((member) => refusedOf('planted.md', member.text).length === 0);
    console.log(`read ${read.length} generated container forms, refused ${members.length - read.length} by name`);
    const escaped = read.flatMap((member) => {
      const sources = blocksOf('planted.md', member.text).map((block) => block.source);
      return JSON.stringify(sources) === JSON.stringify(member.rendered)
        ? []
        : [`${member.id} ${JSON.stringify(member.text)}: read ${JSON.stringify(sources)}, GitHub renders ${JSON.stringify(member.rendered)}`];
    });

    expect(members.length).toBe(FENCE_MEMBERS);
    expect(escaped.slice(0, 3), `${escaped.length} of ${read.length} members read otherwise`).toEqual([]);
    expect(read.length, 'the reader refuses another set of members').toBe(READ_MEMBERS);
    expect(digestOf(read), 'the reader reads another set of members').toBe(READ_DIGEST);
  });

  it('refuses a generated container form it cannot write or read, and says which', () => {
    const members = new Map(fenceMembers().map((member) => [member.id, member.text]));
    const refusals = (id: string) => refusedOf('planted.md', members.get(id) ?? '');
    const opener = 'a line that may open a `mermaid` fence and is not read as one';
    const raw = 'a line that may open raw HTML';
    const hidden = 'raw HTML a CommonMark reading may hide';
    const deep = 'a block in more than 99 block quotes and list items';

    expect(() => fenceMembers({ ...GRAMMAR, body: ['no-such-row'] })).toThrow('no body named no-such-row');
    expect(fenceMembers().length).toBe(FENCE_MEMBERS);
    expect(refusals('body.b.top')).toEqual([
      'planted.md:1 a `mermaid` block that holds no diagram',
      `planted.md:1 ${opener}`,
      'planted.md:2 a character GitHub reads otherwise'
    ]);
    expect(refusals('html.start.0.opening.top')).toEqual([`planted.md:1 ${raw}`, `planted.md:2 ${opener}`]);
    expect(refusals('html.source.3.opening.top')).toEqual([`planted.md:1 ${raw}`]);
    expect(refusals('inline.select.0')).toEqual([`planted.md:3 ${hidden}`]);
    expect(refusals('inline.select.2')).toEqual([`planted.md:3 ${hidden}`]);
    expect(refusals('top:f030:b0')).toEqual([
      'planted.md:1 a fence whose info string is not `mermaid` alone',
      `planted.md:1 ${opener}`
    ]);
    expect(refusals('depth.100')).toEqual([`planted.md:1 ${deep}`]);
    expect(refusals('bound.qi.100')).toEqual([`planted.md:1 ${deep}`]);
    expect(refusals('bound.qi.100.after')).toEqual([`planted.md:3 ${deep}`]);
    expect(refusals('carry.code1.SOURCE')).toEqual([`planted.md:2 ${raw}`]);
    expect(refusals('page.121')).toEqual(['planted.md:1 a block GitHub may nest more than 240 elements deep']);
    expect(refusals('end.d')).toEqual([`planted.md:2 ${raw}`, `planted.md:3 ${opener}`]);
    expect(refusals('prefix.2d.9')).toEqual([`planted.md:1 ${raw}`]);
    expect(refusals('trust.parens.33')).toEqual([`planted.md:1 ${hidden}`]);
  });

  it('refuses every generated container form planted unparsable by name, and accepts it planted valid', async () => {
    const verdicts = new Map<string, boolean>();
    const verdict = async (source: string) => {
      if (!verdicts.has(source)) verdicts.set(source, await parses(source));
      return verdicts.get(source);
    };
    const planted = renderedMembers().filter(
      (member) => member.rendered.length > 0 && member.body !== 'lazy' && refusedOf('planted.md', member.text).length === 0
    );
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

  it('reads every fenced block', () => {
    const blocks = examined('mermaid blocks', docsBlocks());
    const refused = markdownFiles(DOCS).flatMap((file) => refusedOf(relative(DOCS, file), readFileSync(file, 'utf8')));

    expect(blocks.length).toBeGreaterThan(100);
    expect(refused).toEqual([]);
  });

  it('parses every block', async () => {
    const blocks = examined('mermaid blocks', docsBlocks());
    const refused: string[] = [];
    for (const block of blocks) {
      if (!(await parses(block.source))) refused.push(block.name);
    }

    expect(blocks.length).toBeGreaterThan(100);
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
});
