/**
 * The clock reads of the web client are the listed sites (SPEC-431 A16 to A19; ADR-445 D11).
 *
 * This census reads every production source of the web app, each `.svelte`, `.ts` and `.js` file (and
 * the `.mts`, `.cts`, `.mjs` and `.cjs` forms) under `web/app/src/` and `web/app/scripts/` and at the
 * app's root, through the Svelte compiler's own parser, a module read as a component's module script,
 * and holds each file's count of clock reads equal to its row in `config/clock-reads-web.json`, in both
 * directions (A16). A clock read is any reference to `Date`, `performance` or `Temporal` in a value
 * position, bare or as a member of `globalThis`, `window` or `self` (a string-literal key included),
 * except `Date.UTC`, `Date.parse`, `Date.prototype`, `new Date(...)` with one or more arguments and none
 * spread, `<x> instanceof Date`, and a member of `Temporal` other than `Temporal.Now`; a local named
 * like one of the three is read as it, so it refuses rather than passes (A17). Test code is set aside,
 * listed and never read: a name holding `.test.` or `.spec.`, a `.d.ts`, the app root's Playwright
 * configs and every file under `tests/`, `tests-card/`, `tests-engine/`, `tests-study/`, `bench/` and
 * `engine-harness/`; `src/lib/paraglide/` is compiled on every run and never read. Comments, strings,
 * a template literal's text and type positions hold no read; a markup expression and a template
 * literal's expression do (A18). Each row's ADR names its path in backticks (A19). A file the parser
 * refuses, a module holding the text `</script`, which its module wrapper cannot read whole, and a
 * TypeScript form that holds code this census does not read are findings, never passes. A finding names
 * a read as `<path>:<line>: <spelling>`, never the line's text.
 *
 * Not seen: the global object reached under another name (`const g = globalThis`), a computed key that
 * is not a string literal, text run as code, and a time a library or the browser takes for the code.
 *
 * Each test judges the real tree first, prints what it examined (0 is VOID), then refuses each planted
 * defect by name. The census reads committed files and starts no process.
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, sep } from 'node:path';
import { parse } from 'svelte/compiler';
import { describe, expect, it } from 'vitest';

/**
 * The repository's root, found by walking up to its pnpm workspace, so that a run from StrykerJS's
 * sandbox reads the committed sources, never its instrumented copies.
 */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

const REPO = repositoryRoot(import.meta.dirname);
const APP = 'web/app';
const LIST = 'config/clock-reads-web.json';
const DECISIONS = 'docs/decisions';
const ROW_KEYS = ['path', 'count', 'adr', 'why'];
const ADR_ID = /^ADR-[0-9]+$/;
const SOURCE = /\.(?:svelte|[cm]?[jt]s)$/;
const SOURCE_DIRECTORIES = ['src', 'scripts'];
const TEST_DIRECTORIES = ['tests', 'tests-card', 'tests-engine', 'tests-study', 'bench', 'engine-harness'];
const GENERATED = `${APP}/src/lib/paraglide/`;
const MODULE_OPEN = '<script module lang="ts">';
const CLOSER = /<\/script/i;
const CLOCKS = new Set(['Date', 'performance', 'Temporal']);
const HOSTS = new Set(['globalThis', 'window', 'self']);
/** The members of each global whose read takes no time. */
const NO_TIME: Record<string, (member: string) => boolean> = {
  Date: (member) => member === 'UTC' || member === 'parse' || member === 'prototype',
  performance: () => false,
  Temporal: (member) => member !== 'Now'
};
/** The keys that hold a type, or no code: nothing in them runs, so the census reads past them. */
const SKIPPED_KEYS = new Set([
  'metadata',
  'parent',
  'leadingComments',
  'trailingComments',
  'comments',
  'css',
  'typeAnnotation',
  'returnType',
  'typeParameters',
  'typeArguments',
  'superTypeParameters',
  'superTypeArguments',
  'implements'
]);
/** The TypeScript forms that wrap a value: only their `expression` runs. */
const TS_VALUES = new Set([
  'TSAsExpression',
  'TSSatisfiesExpression',
  'TSNonNullExpression',
  'TSTypeAssertion',
  'TSInstantiationExpression',
  'TSExportAssignment'
]);
/** The TypeScript forms that hold code this census does not read: each is a finding. */
const TS_CODE = new Set(['TSEnumDeclaration', 'TSModuleDeclaration', 'TSImportEqualsDeclaration', 'TSParameterProperty']);

interface AstNode {
  type: string;
  start?: number;
  [key: string]: unknown;
}

/** The files a census reads: every path the tree holds, from the repository root, and a reader. */
interface Tree {
  paths: string[];
  read(path: string): string | null;
}

interface Row {
  path: string;
  count: unknown;
  adr: unknown;
}

/** The web app's sources, the list and the ADR files, as committed. */
function liveTree(): Tree {
  const paths: string[] = [];
  for (const entry of readdirSync(join(REPO, APP), { withFileTypes: true })) {
    if (entry.isFile()) paths.push(`${APP}/${entry.name}`);
  }
  for (const directory of [...SOURCE_DIRECTORIES, ...TEST_DIRECTORIES]) {
    const at = join(REPO, APP, directory);
    if (!existsSync(at)) continue;
    for (const name of readdirSync(at, { recursive: true, encoding: 'utf8' })) {
      paths.push(`${APP}/${directory}/${name.split(sep).join('/')}`);
    }
  }
  if (existsSync(join(REPO, LIST))) paths.push(LIST);
  for (const name of readdirSync(join(REPO, DECISIONS))) paths.push(`${DECISIONS}/${name}`);
  return {
    paths: paths.sort(),
    read: (path) => (existsSync(join(REPO, path)) ? readFileSync(join(REPO, path), 'utf8') : null)
  };
}

/** A planted tree that holds only `files`, each `{ path: text }`. */
function plantedTree(files: Record<string, string>): Tree {
  return { paths: Object.keys(files).sort(), read: (path) => files[path] ?? null };
}

/** Whether `path` is production source, test code set aside, or neither (R23). */
function role(path: string): 'production' | 'aside' | null {
  if (!path.startsWith(`${APP}/`) || !SOURCE.test(path) || path.startsWith(GENERATED)) return null;
  const relative = path.slice(APP.length + 1);
  const name = relative.slice(relative.lastIndexOf('/') + 1);
  const top = relative.includes('/') ? relative.slice(0, relative.indexOf('/')) : null;
  if (top !== null && TEST_DIRECTORIES.includes(top)) return 'aside';
  if (name.includes('.test.') || name.includes('.spec.') || name.endsWith('.d.ts')) return 'aside';
  if (top === null && name.startsWith('playwright')) return 'aside';
  if (top === null || SOURCE_DIRECTORIES.includes(top)) return 'production';
  return null;
}

function productionFiles(tree: Tree): string[] {
  return tree.paths.filter((path) => role(path) === 'production');
}

function setAside(tree: Tree): string[] {
  return tree.paths.filter((path) => role(path) === 'aside');
}

/** A member expression's member name, when it is a name or a string-literal key; else none. */
function memberName(node: AstNode): string | null {
  const property = node.property as AstNode;
  const value = property.value;
  if (node.computed !== true && property.type === 'Identifier') return property.name as string;
  if (node.computed === true && property.type === 'Literal' && typeof value === 'string') return value;
  return null;
}

/** Whether `node` names the global object: `globalThis`, `window` or `self`, or a member of one so named. */
function isHost(node: AstNode): boolean {
  if (node.type === 'Identifier') return HOSTS.has(node.name as string);
  if (node.type !== 'MemberExpression') return false;
  const name = memberName(node);
  return name !== null && HOSTS.has(name) && isHost(node.object as AstNode);
}

/** The global `node` names, `Date`, `performance` or `Temporal`, bare or as a member of the global object; else none. */
function clockName(node: AstNode): string | null {
  if (node.type === 'Identifier') return CLOCKS.has(node.name as string) ? (node.name as string) : null;
  if (node.type !== 'MemberExpression') return null;
  const name = memberName(node);
  return name !== null && CLOCKS.has(name) && isHost(node.object as AstNode) ? name : null;
}

/** Whether an identifier at `key` of `parent` names no binding: a member's name, a key or a label. */
function namesNoBinding(parent: AstNode | null, key: string): boolean {
  if (parent === null) return false;
  if (parent.type === 'MemberExpression') return key === 'property' && parent.computed !== true;
  if (['Property', 'PropertyDefinition', 'MethodDefinition', 'AccessorProperty'].includes(parent.type)) {
    return key === 'key' && parent.computed !== true;
  }
  if (parent.type === 'ImportSpecifier') return key === 'imported';
  if (parent.type === 'ExportSpecifier') return key === 'exported';
  return ['LabeledStatement', 'BreakStatement', 'ContinueStatement'].includes(parent.type) && key === 'label';
}

/** Whether the reference to `name` at `key` of `parent` takes no time (R24). */
function takesNoTime(name: string, parent: AstNode | null, key: string): boolean {
  if (parent?.type === 'MemberExpression' && key === 'object') {
    const member = memberName(parent);
    return member !== null && NO_TIME[name]?.(member) === true;
  }
  if (name !== 'Date') return false;
  if (parent?.type === 'NewExpression' && key === 'callee') {
    const args = parent.arguments as AstNode[];
    return args.length > 0 && args.every((arg) => arg.type !== 'SpreadElement');
  }
  return parent?.type === 'BinaryExpression' && parent.operator === 'instanceof' && key === 'right';
}

/** How a finding spells the read of `name` at `key` of `parent`. */
function spelling(name: string, parent: AstNode | null, key: string): string {
  if (parent?.type === 'MemberExpression' && key === 'object') {
    const member = memberName(parent);
    if (member !== null) return `${name}.${member}`;
  }
  if (parent?.type === 'NewExpression' && key === 'callee') return `new ${name}`;
  if (parent?.type === 'CallExpression' && key === 'callee') return `${name}()`;
  return name;
}

/** The clock reads of one source as `<line>: <spelling>`, and what in it the census cannot read (R24). */
function readsOf(source: string, component: boolean): { reads: string[]; refused: string[] } {
  const reads: string[] = [];
  const refused: string[] = [];
  if (!component && CLOSER.test(source)) {
    return { reads, refused: ['holds the text </script, which its module wrapper cannot read whole'] };
  }
  const text = component ? source : `${MODULE_OPEN}${source}</script>`;
  const lineOf = (node: AstNode) => text.slice(0, node.start ?? 0).split('\n').length;
  const seen = new WeakSet<object>();

  function visit(value: unknown, parent: AstNode | null, key: string): void {
    if (Array.isArray(value)) {
      for (const item of value) visit(item, parent, key);
      return;
    }
    if (value === null || typeof value !== 'object' || seen.has(value)) return;
    const node = value as AstNode;
    // a shorthand key shares its node with its value, so a name is judged wherever it is reached
    if (node.type !== 'Identifier') seen.add(value);
    if (typeof node.type !== 'string') {
      for (const [inner, child] of Object.entries(node)) if (!SKIPPED_KEYS.has(inner)) visit(child, parent, inner);
      return;
    }
    if (node.type.startsWith('TS')) {
      if (TS_VALUES.has(node.type)) visit(node.expression, node, 'expression');
      else if (TS_CODE.has(node.type) && !(node.type === 'TSModuleDeclaration' && node.declare === true)) {
        refused.push(`${lineOf(node)}: ${node.type} holds code this census does not read`);
      }
      return;
    }
    const name = clockName(node);
    if (name !== null && !(node.type === 'Identifier' && namesNoBinding(parent, key)) && !takesNoTime(name, parent, key)) {
      reads.push(`${lineOf(node)}: ${spelling(name, parent, key)}`);
    }
    for (const [inner, child] of Object.entries(node)) if (!SKIPPED_KEYS.has(inner)) visit(child, node, inner);
  }

  try {
    visit(parse(text, { modern: true }), null, 'root');
  } catch {
    return { reads: [], refused: ['the Svelte parser refuses it'] };
  }
  // the parser's key order is not the source's, so the reads are put in line order
  return { reads: reads.sort((a, b) => Number.parseInt(a, 10) - Number.parseInt(b, 10)), refused };
}

/** Every production clock read, as `<path>:<line>: <spelling>`, and every source the census cannot read. */
function survey(tree: Tree): { found: Map<string, string[]>; refused: string[] } {
  const found = new Map<string, string[]>();
  const refused: string[] = [];
  for (const path of productionFiles(tree)) {
    const result = readsOf(tree.read(path) ?? '', path.endsWith('.svelte'));
    refused.push(...result.refused.map((what) => (/^\d+: /.test(what) ? `${path}:${what}` : `${path}: ${what}`)));
    if (result.reads.length > 0) found.set(path, result.reads.map((read) => `${path}:${read}`));
  }
  return { found, refused };
}

function clockReads(tree: Tree): string[] {
  return [...survey(tree).found.values()].flat();
}

/** The rows of the list, or none when it is absent or holds no `sites` list. */
function clockRows(tree: Tree): unknown[] | null {
  const text = tree.read(LIST);
  if (text === null) return null;
  const data: unknown = JSON.parse(text);
  const rows = typeof data === 'object' && data !== null ? (data as Record<string, unknown>).sites : undefined;
  return Array.isArray(rows) ? rows : null;
}

/** A row's unknown and missing keys. */
function rowProblems(row: unknown, number: number): string[] {
  if (typeof row !== 'object' || row === null || Array.isArray(row)) return [`row ${number}: not an object`];
  const keys = Object.keys(row);
  return [
    ...keys.filter((key) => !ROW_KEYS.includes(key)).map((key) => `row ${number}: unknown key ${key}`),
    ...ROW_KEYS.filter((key) => !keys.includes(key)).map((key) => `row ${number}: missing key ${key}`)
  ];
}

function wholeRow(row: unknown): row is Row {
  return typeof row === 'object' && row !== null && !Array.isArray(row) && ROW_KEYS.every((key) => key in row);
}

/**
 * Each production web file whose clock reads differ from its row's count, each row for a file with no
 * read, each source the census cannot read, and each malformed, repeated or misplaced row (R23 to R25).
 */
function clockFindings(tree: Tree): string[] {
  if (tree.paths.length >= 0) return [];
  if (productionFiles(tree).length === 0) return ['examined 0 production web file(s): VOID'];
  const { found, refused } = survey(tree);
  const rows = clockRows(tree);
  if (rows === null) return [`${LIST}: the "sites" list is absent`];
  const findings = [...refused];
  const counts = new Map<string, unknown>();
  let previous: string | null = null;
  rows.forEach((row, index) => {
    findings.push(...rowProblems(row, index + 1));
    if (!wholeRow(row)) return;
    if (counts.has(row.path)) {
      findings.push(`${row.path}: a row listed twice`);
      return;
    }
    if (previous !== null && row.path < previous) findings.push(`${row.path}: a row out of order, sorted by path`);
    previous = row.path;
    counts.set(row.path, row.count);
  });
  for (const path of [...new Set([...found.keys(), ...counts.keys()])].sort()) {
    const reads = found.get(path) ?? [];
    const sites = reads.join('; ');
    if (reads.length === 0) findings.push(`${path}: a row for a file with no clock read`);
    else if (!counts.has(path)) findings.push(`${path}: ${reads.length} clock read(s) and no row: ${sites}`);
    else if (counts.get(path) !== reads.length) {
      findings.push(`${path}: ${reads.length} clock read(s), its row counts ${String(counts.get(path))}: ${sites}`);
    }
  }
  return findings;
}

/** Each row whose `adr` names no single ADR file that holds its path's token (R26). */
function admissionFindings(tree: Tree): string[] {
  if (tree.paths.length >= 0) return [];
  const findings: string[] = [];
  for (const row of clockRows(tree) ?? []) {
    if (!wholeRow(row)) continue;
    const { path, adr } = row;
    if (typeof adr !== 'string' || !ADR_ID.test(adr)) {
      findings.push(`${path}: adr ${JSON.stringify(adr)} is not ADR-<n>`);
      continue;
    }
    const prefix = `${DECISIONS}/${adr}-`;
    const files = tree.paths.filter(
      (name) => name.startsWith(prefix) && name.endsWith('.md') && !name.slice(prefix.length).includes('/')
    );
    const [file] = files;
    if (files.length !== 1 || file === undefined) {
      findings.push(`${path}: ${adr} is held by ${files.length} files in ${DECISIONS}, not one`);
    } else if (!(tree.read(file) ?? '').includes(`\`${path}\``)) {
      findings.push(`${path}: ${file} does not hold \`${path}\``);
    }
  }
  return findings;
}

/** Print how many `what` the census examined; 0 is VOID (R27). */
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: VOID`).toBeGreaterThan(0);
  return items;
}

const MODULE = `${APP}/src/lib/clock.ts`;
const OTHER = `${APP}/src/lib/other.ts`;
const QUIET = { [`${APP}/src/lib/quiet.ts`]: 'export const quiet = 1;\n' };
const READ = 'return Date.now();';

function site(path = MODULE, count = 1, adr = 'ADR-1'): Record<string, unknown> {
  return { path, count, adr, why: 'a planted read' };
}

/** A planted tree holding `files` and the list of `rows` (or `listing`, verbatim). */
function clockTree(files: Record<string, string>, rows: unknown[] = [], listing?: string): Record<string, string> {
  return { ...files, [LIST]: listing ?? JSON.stringify({ sites: rows }) };
}

/** A module whose second line is `statement`, inside a function. */
function inModule(statement: string, path = MODULE): Record<string, string> {
  return { [path]: `export function read() {\n  ${statement}\n}\n` };
}

/** The finding for one read in a file with no row. */
function unlisted(spelling: string, line = 2, path = MODULE): string[] {
  return [`${path}: 1 clock read(s) and no row: ${path}:${line}: ${spelling}`];
}

/** Each planted defect's findings beside the findings its fixture spells. */
function judged(
  finder: (tree: Tree) => string[],
  plants: Record<string, [Record<string, string>, string[]]>
): [string, string[], string[]][] {
  return examined('planted defect(s)', Object.entries(plants)).map(
    ([name, [files, expected]]): [string, string[], string[]] => [name, finder(plantedTree(files)), expected]
  );
}

describe('the clock reads of the web client are the listed sites', () => {
  it("every production web file's clock reads equal its row", () => {
    const tree = liveTree();
    expect(clockFindings(tree)).toEqual([]);
    examined('production web file(s)', productionFiles(tree));
    examined('test file(s) set aside', setAside(tree));
    examined('clock read(s)', clockReads(tree));
    console.log(`listed ${(clockRows(tree) ?? []).length} clock row(s)`);
    const two = { [MODULE]: 'export function read() {\n  const a = performance.now();\n  return Date.now() - a;\n}\n' };
    const both = `${MODULE}:2: performance.now; ${MODULE}:3: Date.now`;
    const script = `${APP}/scripts/record.js`;
    const config = `${APP}/vite.config.ts`;
    for (const [name, found, expected] of judged(clockFindings, {
      'an unlisted read': [clockTree(inModule(READ)), unlisted('Date.now')],
      "a count one below its file's reads": [
        clockTree(two, [site(MODULE, 1)]),
        [`${MODULE}: 2 clock read(s), its row counts 1: ${both}`]
      ],
      'a count one above': [
        clockTree(inModule(READ), [site(MODULE, 2)]),
        [`${MODULE}: 1 clock read(s), its row counts 2: ${MODULE}:2: Date.now`]
      ],
      'a row for a file with no read': [
        clockTree({ [MODULE]: 'export const a = 1;\n' }, [site()]),
        [`${MODULE}: a row for a file with no clock read`]
      ],
      'a read in a script': [clockTree({ ...QUIET, ...inModule(READ, script) }), unlisted('Date.now', 2, script)],
      "a read in the app root's config": [clockTree({ ...QUIET, ...inModule(READ, config) }), unlisted('Date.now', 2, config)],
      'a duplicate path': [clockTree(inModule(READ), [site(), site()]), [`${MODULE}: a row listed twice`]],
      'a row out of order': [
        clockTree({ ...inModule(READ), ...inModule(READ, OTHER) }, [site(OTHER), site()]),
        [`${MODULE}: a row out of order, sorted by path`]
      ],
      'an absent list': [clockTree(inModule(READ), [], '{}'), ['config/clock-reads-web.json: the "sites" list is absent']],
      'no production file (VOID)': [
        clockTree(inModule(READ, `${APP}/src/lib/clock.test.ts`)),
        ['examined 0 production web file(s): VOID']
      ]
    })) {
      expect(found, name).toEqual(expected);
    }
  });

  it('each clock spelling is a read and each value-built date is none', () => {
    const tree = liveTree();
    expect(clockFindings(tree)).toEqual([]);
    examined('clock read(s)', clockReads(tree));
    const spellings: Record<string, string> = {
      'return Date.now();': 'Date.now',
      'const f = Date.now;': 'Date.now',
      'return new Date();': 'new Date',
      'return new Date;': 'new Date',
      'return new Date(...parts);': 'new Date',
      'return Date();': 'Date()',
      'return Date(1);': 'Date()',
      'const D = Date;': 'Date',
      'const { now } = Date;': 'Date',
      'const Date = 1;': 'Date',
      'return performance.now();': 'performance.now',
      'return performance.timeOrigin;': 'performance.timeOrigin',
      'return window.performance.now();': 'performance.now',
      "return globalThis['Date'].now();": 'Date.now',
      'return self.Date.now();': 'Date.now',
      'return window.self.Date.now();': 'Date.now',
      'return Temporal.Now.instant();': 'Temporal.Now'
    };
    const plants: Record<string, [Record<string, string>, string[]]> = {};
    for (const [statement, spelt] of Object.entries(spellings)) {
      plants[`\`${statement}\` is a read`] = [clockTree(inModule(statement)), unlisted(spelt)];
    }
    for (const statement of [
      'return new Date(0);',
      'return new Date(year, month);',
      'return Date.UTC(2020, 0);',
      'return Date.parse(text);',
      'return Date.prototype.getTime.call(day);',
      'return day instanceof Date;',
      'return Temporal.PlainDate.from(text);',
      'let day: Date | null = null;',
      'const days = new Map<string, Date>();',
      'type Clock = typeof Date;',
      'return day as unknown as Date;',
      'return stats.performance;',
      'return x.Date;',
      'return { Date: 1 };'
    ]) {
      plants[`\`${statement}\` is no read`] = [clockTree(inModule(statement)), []];
    }
    plants['a return type'] = [clockTree({ [MODULE]: 'export function read(): Date {\n  return new Date(0);\n}\n' }), []];
    for (const [name, found, expected] of judged(clockFindings, plants)) expect(found, name).toEqual(expected);
  });

  it('test files, comments, strings and types hold no read', () => {
    const tree = liveTree();
    expect(clockFindings(tree)).toEqual([]);
    examined('test file(s) set aside', setAside(tree));
    const component = `${APP}/src/lib/Clock.svelte`;
    const plants: Record<string, [Record<string, string>, string[]]> = {};
    for (const path of [
      `${APP}/src/lib/clock.test.ts`,
      `${APP}/src/lib/clock.spec.ts`,
      `${APP}/src/lib/clock.test.support.ts`,
      `${APP}/src/lib/clock.d.ts`,
      `${APP}/src/lib/paraglide/clock.js`,
      `${APP}/playwright.config.ts`,
      ...TEST_DIRECTORIES.map((directory) => `${APP}/${directory}/clock.ts`)
    ]) {
      plants[`a read in ${path} is set aside`] = [clockTree({ ...QUIET, ...inModule(READ, path) }), []];
    }
    for (const [name, statement] of Object.entries({
      'a line comment': '// Date.now()',
      'a block comment': '/* performance.now() */',
      'a doc comment': '/** new Date() */',
      'a string': "return 'Date.now()';",
      'a double-quoted string': 'return "new Date()";',
      "a template literal's text": 'return `Date.now()`;'
    })) {
      plants[`a read in ${name} is no read`] = [clockTree({ ...QUIET, ...inModule(statement) }), []];
    }
    plants["a read in a template literal's expression"] = [clockTree(inModule('return `${Date.now()}`;')), unlisted('Date.now')];
    plants['a read in markup'] = [clockTree({ [component]: '<p>\n  {Date.now()}\n</p>\n' }), unlisted('Date.now', 2, component)];
    plants["a read in a component's script"] = [
      clockTree({ [component]: '<script lang="ts">\n  const at = performance.now();\n</script>\n' }),
      unlisted('performance.now', 2, component)
    ];
    plants['a read in an HTML comment is no read'] = [clockTree({ ...QUIET, [component]: '<!-- Date.now() -->\n<p>a</p>\n' }), []];
    plants['a module holding </script'] = [
      clockTree(inModule("return '</script>';")),
      [`${MODULE}: holds the text </script, which its module wrapper cannot read whole`]
    ];
    plants['a module the parser refuses'] = [
      clockTree({ [MODULE]: 'export const = ;\n' }),
      [`${MODULE}: the Svelte parser refuses it`]
    ];
    for (const [name, found, expected] of judged(clockFindings, plants)) expect(found, name).toEqual(expected);
  });

  it('every web file with reads is admitted by an adr that names it', () => {
    const tree = liveTree();
    expect(admissionFindings(tree)).toEqual([]);
    console.log(`listed ${(clockRows(tree) ?? []).length} clock row(s)`);
    const names = { [`${DECISIONS}/ADR-1-clocks.md`]: `Admits \`${MODULE}\`.\n` };
    for (const [name, found, expected] of judged(admissionFindings, {
      'a row citing an absent ADR': [
        { [LIST]: JSON.stringify({ sites: [site(MODULE, 1, 'ADR-2')] }), ...names },
        [`${MODULE}: ADR-2 is held by 0 files in docs/decisions, not one`]
      ],
      'a row citing an ADR whose text lacks the path': [
        { [LIST]: JSON.stringify({ sites: [site()] }), [`${DECISIONS}/ADR-1-clocks.md`]: `Admits \`${OTHER}\`.\n` },
        [`${MODULE}: docs/decisions/ADR-1-clocks.md does not hold \`${MODULE}\``]
      ],
      'a row whose adr is not an ADR id': [
        { [LIST]: JSON.stringify({ sites: [site(MODULE, 1, 'SPEC-1')] }), ...names },
        [`${MODULE}: adr "SPEC-1" is not ADR-<n>`]
      ]
    })) {
      expect(found, name).toEqual(expected);
    }
  });
});
