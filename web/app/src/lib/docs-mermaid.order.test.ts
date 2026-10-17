/**
 * SPEC-195. The mutation run tries every mutant against the tests that cover it, in file order, and
 * stops at the first one that fails. The generated-population tests are the ones that kill nearly
 * every mutant of the generator and the reader, and the two docs-wide tests are the costliest to run, so the
 * population tests come first and the docs-wide tests after them. This guard holds that order and
 * the set of tests in `docs-mermaid.test.ts` (none dropped, none twice), and the populations those
 * tests judge.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { GRAMMAR, digestOf, fenceMembers } from './docs-mermaid-fences.js';
import { readMermaid } from './docs-mermaid-read.js';

const SOURCE = readFileSync(resolve(import.meta.dirname, 'docs-mermaid.test.ts'), 'utf8');

/** The generated-population tests, in the order they are written. */
const POPULATION = [
  'reads exactly the fences GitHub renders as diagrams, or refuses by name, in every generated container form',
  'refuses a generated container form it cannot write or read, and says which',
  'refuses every generated container form planted unparsable by name, and accepts it planted valid'
];

/** The two tests that read every block under docs. */
const DOCS_WIDE = ['reads every fenced block', 'parses every block'];

/** The tests that are neither, small and self-contained. */
const OTHERS = [
  'refuses a block whose node id is a reserved word',
  'accepts the same block once its node id is not a reserved word',
  'reads an indented fence and refuses one that does not parse',
  'accepts an indented valid block of each diagram type the docs use'
];

/** The titles of the `it(` calls in `text`, in order. */
function titlesOf(text: string): string[] {
  return [...text.matchAll(/^\s*it\('([^']+)'/gm)].map((match) => match[1]);
}

/** The order rule over a list of titles: the problems it finds, none when the order holds. */
function orderProblems(titles: string[]): string[] {
  const expected = [...POPULATION, ...DOCS_WIDE, ...OTHERS];
  const problems: string[] = [];
  for (const title of expected) {
    const at = titles.filter((found) => found === title).length;
    if (at !== 1) problems.push(`${at} tests are titled "${title}"`);
  }
  for (const title of titles) if (!expected.includes(title)) problems.push(`an unknown test "${title}"`);
  const last = Math.max(...POPULATION.map((title) => titles.indexOf(title)));
  for (const title of DOCS_WIDE) {
    if (titles.indexOf(title) < last) problems.push(`"${title}" comes before a generated-population test`);
  }
  return problems;
}

describe('the order of the Mermaid tests', () => {
  it('holds every test once, the generated-population tests before the docs-wide ones', () => {
    const titles = titlesOf(SOURCE);

    expect(titles.length, 'the file holds another number of tests').toBe(9);
    expect(orderProblems(titles)).toEqual([]);
  });

  it('sees a docs-wide test moved first, a title deleted and a title duplicated', () => {
    const titles = titlesOf(SOURCE);
    const [first, ...rest] = [...DOCS_WIDE, ...titles.filter((title) => !DOCS_WIDE.includes(title))];

    expect(orderProblems([first, ...rest]).length).toBeGreaterThan(0);
    expect(orderProblems(titles.slice(1)).length).toBeGreaterThan(0);
    expect(orderProblems([...titles, titles[0]]).length).toBeGreaterThan(0);
  });

  it('judges the same populations as the recorded ones', () => {
    const truth = JSON.parse(readFileSync(resolve(import.meta.dirname, 'docs-mermaid.fences.json'), 'utf8'));
    const pinned = SOURCE.match(/READ_DIGEST = '([0-9a-f]{64})'/)?.[1];
    const members = fenceMembers(GRAMMAR);
    const read = members.filter((member) => readMermaid(member.text).refused.length === 0);

    console.log(`examined ${members.length} generated members, read ${read.length}`);
    expect(members.length).toBe(8994);
    expect(members.map((member) => member.id)).toEqual(Object.keys(truth.renders));
    expect(digestOf(members)).toBe(truth.digest);
    expect(read.length).toBe(2934);
    expect(members.length - read.length).toBe(6060);
    expect(digestOf(read)).toBe(pinned);
  });
});
