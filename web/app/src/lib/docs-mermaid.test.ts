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

interface Block {
  name: string;
  source: string;
}

/** The fenced `mermaid` blocks of one file, each named `<file> block <n>` counted from 1. */
function blocksOf(name: string, text: string): Block[] {
  const found = text.matchAll(/^([ \t]*)```mermaid[^\n]*\n([\s\S]*?)^\1```[ \t]*$/gm);
  return [...found].map((match, index) => ({
    name: `${name} block ${index + 1}`,
    source: match[2].replace(new RegExp(`^${match[1]}`, 'gm'), '')
  }));
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

const BLOCKS = markdownFiles(DOCS).flatMap((file) =>
  blocksOf(relative(DOCS, file), readFileSync(file, 'utf8'))
);

describe('the Mermaid diagrams under docs', () => {
  it('reads every fenced block', () => {
    examined('mermaid blocks', BLOCKS);
    const opened = markdownFiles(DOCS)
      .map((file) => (readFileSync(file, 'utf8').match(/^[ \t]*```mermaid/gm) ?? []).length)
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

  it('reads a quoted fence and one spaced before its info string, and refuses one that does not parse', async () => {
    const plants = [
      '> ```mermaid\n> flowchart TD\n>   call --> done\n> ```\n',
      '> - item\n>\n>   ```mermaid\n>   flowchart TD\n>     call --> done\n>   ```\n',
      '``` mermaid\nflowchart TD\n  call --> done\n```\n'
    ];
    for (const text of plants) {
      const planted = blocksOf('planted.md', text);

      expect(planted.map((block) => block.name)).toEqual(['planted.md block 1']);
      expect(await parses(planted[0].source)).toBe(false);
    }
  });

  it('accepts a quoted valid block with a quoted blank line in it', async () => {
    const planted = blocksOf('planted.md', '> ```mermaid\n> flowchart TD\n>\n>   caller --> done\n> ```\n');

    expect(planted.map((block) => block.name)).toEqual(['planted.md block 1']);
    expect(await parses(planted[0].source)).toBe(true);
  });
});
