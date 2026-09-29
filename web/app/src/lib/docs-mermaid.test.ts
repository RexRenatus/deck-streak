/**
 * SPEC-195, ADR-195. Every fenced mermaid block under docs/ must parse with Mermaid's own parser,
 * so a diagram that would render as an error box is refused before merge. The parser sanitises
 * labels through the document, hence jsdom.
 *
 * @vitest-environment jsdom
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import mermaid from 'mermaid';
import { describe, expect, it } from 'vitest';
import { GRAMMAR, digestOf, fenceMembers } from '../../scripts/docs-mermaid-fences.js';

const DOCS = resolve(import.meta.dirname, '../../../../docs');

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

/** A `mermaid` opener: indentation and blockquote markers, three backticks, blanks, `mermaid`. */
const OPENER = '^((?:[ \\t]*>[ \\t]?)*[ \\t]*)```[ \\t]*mermaid';
const CLOSER = '```[ \\t]*$';

interface Block {
  name: string;
  source: string;
}

/**
 * The fenced `mermaid` blocks of one file, each named `<file> block <n>` counted from 1. The opener's
 * prefix (its indentation and any blockquote markers) is captured, the closer must carry the same,
 * and the prefix is stripped from each body line; a quoted blank line (the prefix without its
 * trailing blanks) becomes an empty line.
 */
function blocksOf(name: string, text: string): Block[] {
  const found = text.matchAll(new RegExp(`${OPENER}[^\\n]*\\n([\\s\\S]*?)^\\1${CLOSER}`, 'gm'));
  return [...found].map((match, index) => {
    const prefix = match[1];
    const bare = prefix.trimEnd();
    const strip = (line: string) =>
      line.startsWith(prefix) ? line.slice(prefix.length) : line.trimEnd() === bare ? '' : line;
    return { name: `${name} block ${index + 1}`, source: match[2].split('\n').map(strip).join('\n') };
  });
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
    const opened = markdownFiles(DOCS)
      .map((file) => (readFileSync(file, 'utf8').match(new RegExp(OPENER, 'gm')) ?? []).length)
      .reduce((sum, count) => sum + count, 0);

    expect(BLOCKS.length).toBeGreaterThan(100);
    expect(BLOCKS.length).toBe(opened);
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

    expect(escaped.slice(0, 3), `${escaped.length} of ${members.length} members read otherwise`).toEqual([]);
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

    expect(wrong.slice(0, 3), `${wrong.length} of ${planted.length} members judged otherwise`).toEqual([]);
  });
});
