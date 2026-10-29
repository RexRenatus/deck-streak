/**
 * @vitest-environment jsdom
 */
import { readdirSync, readFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { parse } from 'svelte/compiler';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { studyEngine } from '$lib/engine/credential-stand-in.test.support';
import type { CardHealth, CardView, Head, HealthReadout, UndoOffer } from '$lib/engine/protocol';
import { Session, type EngineModule } from '$lib/engine/session';
import en from '../../../messages/en.json';
import { VERDICT } from './card-health';
import CardHealthPanel from './CardHealthPanel.svelte';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

// SPEC-507 A3 to A5; ADR-525. The card health panel shows the shown card's own record: its counters,
// its answers' capped times, its newest rows, the scheduler's labelled estimate only when the
// scheduler is Ready and the state's line otherwise, and where it came from. It judges no card, it
// reaches the record only through the review's read over the Worker, and the answer clock stops
// while it is open.
const APP = join(import.meta.dirname, '..', '..', '..');

/** Each state but Ready, and the line its recall rows read. */
const STATE_LINES: Record<Exclude<HealthReadout['state'], 'ready'>, string> = {
  absent: 'Not available in this build.',
  fixture: 'Not available: the scheduler is a test fixture.',
  unready: 'Not available: the scheduler has not read this card yet.',
  stale: "Not available: the scheduler's state is older than this card's last review."
};

/** A fixture card's record: 4 reviews, 1 lapse, the core's capped figures, and `scheduler`. */
function record(scheduler: HealthReadout, extra: Partial<CardHealth> = {}): CardHealth {
  return {
    card: 1_700_000_000_000n,
    reps: 4,
    lapses: 1,
    logged: { answers: 4, lapses: 1 },
    rows: 5,
    reviews: [
      { at: 1_700_000_005_000, grade: 3, kind: 2, interval: 1, ms: 20_000 },
      { at: 1_700_000_004_000, grade: 0, kind: 4, interval: 0, ms: 0 },
      { at: 1_700_000_003_000, grade: 1, kind: 1, interval: -600, ms: 12_000 },
      { at: 1_700_000_002_000, grade: 3, kind: 0, interval: 1, ms: 60_000 },
      { at: 1_700_000_001_000, grade: 3, kind: 0, interval: -600, ms: 8_000 }
    ],
    totalMs: 100_000,
    meanMs: 25_000,
    medianMs: 16_000,
    scheduler,
    provenance: { source: 'collection', createdMs: 1_700_000_000_000 },
    ...extra
  };
}

/** The instant of the core's fixture card, 2024-04-05 19:34:38.901 UTC, in epoch milliseconds: its id
 * and its created instant. */
const CORE_CARD = 1_712_345_678_901;

/** A record as the engine core's JSON spells it (`CardHealth::to_json`): snake-case keys, integers, the
 * card as a number, and `scheduler` as its readout part. */
function coreJson(scheduler: object): string {
  return JSON.stringify({
    card: CORE_CARD,
    reps: 7,
    lapses: 2,
    logged: { answers: 6, lapses: 1 },
    rows: 207,
    reviews: [
      { at: CORE_CARD + 7_000, grade: 3, kind: 1, interval: 4, ms: 29_998 },
      { at: CORE_CARD + 6_000, grade: 1, kind: 1, interval: -600, ms: 12_000 }
    ],
    total_ms: 129_999,
    mean_ms: 21_667,
    median_ms: 16_000,
    scheduler,
    provenance: { source: 'collection', created_ms: CORE_CARD }
  });
}

/** The record the Worker answers for `json`: a session opened over a module whose `card_health`
 * answers `json`, asked for one card's health as the client asks (SPEC-507 R1, R10). */
async function throughWorker(json: string): Promise<CardHealth | null> {
  const engine: EngineModule = { ...studyEngine(), card_health: () => json };
  const session = new Session({ lock: async () => 'held', storage: async () => null, load: async () => engine });
  expect(await session.handle({ id: 1, op: 'open' })).toMatchObject({ id: 1, ok: true });
  const reply = await session.handle({ id: 2, op: 'card-health', card: BigInt(CORE_CARD) });
  expect(reply).toMatchObject({ id: 2, ok: true });
  return (reply as { value: CardHealth | null }).value;
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The text of the element, its whitespace collapsed. */
function text(element: Element): string {
  return (element.textContent ?? '').replace(/\s+/g, ' ').trim();
}

function panel(health: CardHealth | null): void {
  render(CardHealthPanel, { health, loading: false, onclose: () => undefined });
}

// ---- the panel's reach, default-deny (SPEC-507 R10) ----

/** What the panel's two modules may reach. Each list is closed: the census refuses whatever is not on
 * it, so a new import, an undeclared name, a property, an indexed value, a kind of syntax, an element or
 * an attribute fails the census until it is reviewed and listed here. */
const ALLOWED = {
  /** the modules they import */
  imports: new Set(['$lib/engine/protocol', '$lib/paraglide/messages.js', './card-health']),
  /** the names they read that no scope of theirs declares: the language's, Svelte's runes, the document */
  globals: new Set(['$derived', '$effect', '$props', 'Date', 'String', 'document']),
  /** the properties they read by name; the message catalogue's keys are read through `m` too */
  members: new Set([
    'activeElement',
    'answers',
    'at',
    'createdMs',
    'daysTo90',
    'focus',
    'grade',
    'indexOf',
    'label',
    'lapses',
    'length',
    'lines',
    'logged',
    'meanMs',
    'medianMs',
    'ms',
    'preventDefault',
    'provenance',
    'querySelectorAll',
    'recallPpm',
    'reps',
    'reviews',
    'rows',
    'scheduler',
    'shiftKey',
    'slice',
    'state',
    'toFixed',
    'toISOString',
    'totalMs'
  ]),
  /** the values they index by a computed key, as their source writes them */
  indexed: new Set(['STATE_LINES', 'controls', 'names', 'recall.lines']),
  /** the kinds of syntax they are written in */
  kinds: new Set([
    'ArrayExpression',
    'ArrowFunctionExpression',
    'Attribute',
    'BinaryExpression',
    'BindDirective',
    'Block',
    'BlockStatement',
    'CallExpression',
    'ChainExpression',
    'ConditionalExpression',
    'EachBlock',
    'ExportNamedDeclaration',
    'ExpressionStatement',
    'ExpressionTag',
    'Fragment',
    'FunctionDeclaration',
    'Identifier',
    'IfBlock',
    'IfStatement',
    'ImportDeclaration',
    'ImportSpecifier',
    'Line',
    'Literal',
    'LogicalExpression',
    'MemberExpression',
    'NewExpression',
    'ObjectExpression',
    'ObjectPattern',
    'Program',
    'Property',
    'RegularElement',
    'ReturnStatement',
    'Root',
    'Script',
    'SpreadElement',
    'TSAsExpression',
    'Text',
    'UnaryExpression',
    'VariableDeclaration',
    'VariableDeclarator'
  ]),
  /** the elements of the dialog's markup */
  elements: new Set(['button', 'dd', 'div', 'dl', 'dt', 'h2', 'h3', 'p', 'section', 'table', 'tbody', 'td', 'th', 'thead', 'tr']),
  /** the attributes of the dialog's markup and of its scripts' tags */
  attributes: new Set([
    'aria-labelledby',
    'aria-modal',
    'bind:this',
    'class',
    'id',
    'lang',
    'module',
    'onclick',
    'role',
    'scope',
    'type'
  ])
};

/** The kinds of syntax that only state a type: nothing in them runs, so the census reads past them.
 * Any other kind is read, and must be on ALLOWED's list of kinds. */
const TYPE_ONLY = new Set([
  'TSAnyKeyword',
  'TSArrayType',
  'TSBooleanKeyword',
  'TSFunctionType',
  'TSIndexedAccessType',
  'TSInterfaceDeclaration',
  'TSLiteralType',
  'TSNullKeyword',
  'TSNumberKeyword',
  'TSPropertySignature',
  'TSQualifiedName',
  'TSStringKeyword',
  'TSTypeAliasDeclaration',
  'TSTypeAnnotation',
  'TSTypeLiteral',
  'TSTypeParameterInstantiation',
  'TSTypeQuery',
  'TSTypeReference',
  'TSUndefinedKeyword',
  'TSUnionType',
  'TSUnknownKeyword',
  'TSVoidKeyword'
]);

/** The base's route files under src/routes, each a SvelteKit route (`+` first), frozen. SPEC-516 R1
 * adds the data screen's, `data/+page.svelte`. */
const ROUTES = [
  '+layout.svelte',
  '+layout.ts',
  '+page.svelte',
  'about/+page.svelte',
  'apple-touch-icon.png/+server.ts',
  'badges/+page.svelte',
  'capture/+page.svelte',
  'data/+page.svelte',
  'icon-192.png/+server.ts',
  'icon-512.png/+server.ts',
  'insights/+page.svelte',
  'law/+page.svelte',
  'level/+page.svelte',
  'link/+page.svelte',
  'progress/+page.svelte',
  'records/+page.svelte',
  'remote/+page.svelte',
  'score/+page.svelte',
  'sign-in-methods/+page.svelte',
  'signin/+page.svelte',
  'streak/+page.svelte',
  'study/+page.svelte',
  'study/ai-decks/+page.svelte',
  'study/mapping/+page.svelte',
  'study/review/+page.svelte',
  'sync/+page.svelte',
  'wallet/+page.svelte'
];

/** Reaches the census must refuse: a source, whether it is a component, and a refusal it must read. */
const PLANTS: { source: string; component: boolean; refused: string }[] = [
  {
    source: "<script lang=\"ts\">\n  function send(): void {\n    void fetch('/api/card-health');\n  }\n</script>\n",
    component: true,
    refused: 'global fetch'
  },
  {
    source:
      "<script lang=\"ts\">\n  function send(): void {\n    const get = globalThis.fetch;\n    void get('/api/card-health');\n  }\n</script>\n",
    component: true,
    refused: 'global globalThis'
  },
  {
    source: "export function leave(): void {\n  location.assign('/card-info');\n}\n",
    component: false,
    refused: 'global location'
  },
  {
    source:
      "export function own(location: string): string {\n  return location;\n}\nexport function leave(): void {\n  location.assign('/card-info');\n}\n",
    component: false,
    refused: 'global location'
  },
  {
    source:
      "<script lang=\"ts\">\n  let dialog: HTMLElement;\n  function send(): void {\n    void dialog.ownerDocument.defaultView?.fetch('/api/card');\n  }\n</script>\n<div bind:this={dialog}></div>\n",
    component: true,
    refused: 'member ownerDocument'
  },
  {
    source: "export async function later(): Promise<unknown> {\n  return import('./elsewhere');\n}\n",
    component: false,
    refused: 'kind ImportExpression'
  },
  {
    source: "import { goto } from '$app/navigation';\nexport function leave(): void {\n  void goto('/card-info');\n}\n",
    component: false,
    refused: 'import $app/navigation'
  },
  {
    source: '<a href="/card-info">Card info</a>\n',
    component: true,
    refused: 'attribute href'
  },
  {
    source: 'export function pick(record: Record<string, unknown>, key: string): unknown {\n  return record[key];\n}\n',
    component: false,
    refused: 'indexed record'
  },
  {
    source: 'export function hand(): unknown {\n  return { fetch };\n}\n',
    component: false,
    refused: 'global fetch'
  }
];

/** A node of the Svelte parser's tree. */
interface AstNode {
  type: string;
  start?: number;
  end?: number;
  [key: string]: unknown;
}

/** What one module's source reaches. */
interface Reach {
  imports: Set<string>;
  free: Set<string>;
  members: Set<string>;
  indexed: Set<string>;
  kinds: Set<string>;
  elements: Set<string>;
  attributes: Set<string>;
}

/** The names one scope declares, and the scope around it. */
interface Scope {
  names: Set<string>;
  up: Scope | null;
}

function resolves(name: string, scope: Scope | null): boolean {
  for (let at = scope; at !== null; at = at.up) {
    if (at.names.has(name)) return true;
  }
  return false;
}

/** Adds the names `pattern` binds to `names`. */
function bind(pattern: unknown, names: Set<string>): void {
  if (pattern === null || typeof pattern !== 'object') return;
  const node = pattern as AstNode;
  if (node.type === 'Identifier') names.add(node.name as string);
  if (node.type === 'ObjectPattern') {
    for (const property of node.properties as AstNode[]) {
      bind(property.type === 'RestElement' ? property.argument : property.value, names);
    }
  }
  if (node.type === 'ArrayPattern') for (const element of node.elements as unknown[]) bind(element, names);
  if (node.type === 'AssignmentPattern') bind(node.left, names);
  if (node.type === 'RestElement') bind(node.argument, names);
}

/** The names `statements` declare in their own scope: imports, variables, functions and classes. */
function declared(statements: unknown): Set<string> {
  const names = new Set<string>();
  for (const statement of (statements ?? []) as AstNode[]) {
    const node = statement.type === 'ExportNamedDeclaration' ? (statement.declaration as AstNode | null) : statement;
    if (node === null) continue;
    if (node.type === 'ImportDeclaration') for (const specifier of node.specifiers as AstNode[]) bind(specifier.local, names);
    if (node.type === 'VariableDeclaration') for (const declarator of node.declarations as AstNode[]) bind(declarator.id, names);
    if (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') bind(node.id, names);
  }
  return names;
}

/** The body of a component's script, or none. */
function scriptBody(script: unknown): unknown {
  return ((script as AstNode | null)?.content as AstNode | undefined)?.body;
}

/** What `source` reaches: a component's scripts and markup, or a TypeScript module read as a component's
 * module script. Names resolve through the scopes that declare them: a module's, a block's, a function's
 * parameters and an each block's item; a scope the census does not model declares nothing, so a name read
 * in it reads as undeclared and is refused unless listed. */
function reach(source: string, component: boolean): Reach {
  const text = component ? source : `<script module lang="ts">${source}</script>`;
  const found: Reach = {
    imports: new Set(),
    free: new Set(),
    members: new Set(),
    indexed: new Set(),
    kinds: new Set(),
    elements: new Set(),
    attributes: new Set()
  };
  const seen = new WeakSet<object>();

  function children(node: AstNode, scope: Scope | null, skip: string[] = []): void {
    for (const [key, value] of Object.entries(node)) {
      if (key !== 'metadata' && key !== 'parent' && !skip.includes(key)) visit(value, scope, node, key);
    }
  }

  function visit(value: unknown, scope: Scope | null, parent: AstNode | null, key: string): void {
    if (Array.isArray(value)) {
      for (const item of value) visit(item, scope, parent, key);
      return;
    }
    if (value === null || typeof value !== 'object' || seen.has(value)) return;
    const node = value as AstNode;
    // a name is judged wherever it is reached, so a shorthand key that shares its node with its value
    // is still read as the value; any other node is read once
    if (node.type !== 'Identifier') seen.add(value);
    if (typeof node.type !== 'string') {
      for (const [inner, child] of Object.entries(node)) visit(child, scope, parent, inner);
      return;
    }
    if (TYPE_ONLY.has(node.type)) return;
    found.kinds.add(node.type);
    switch (node.type) {
      case 'Root': {
        const outer: Scope = { names: declared(scriptBody(node.module)), up: null };
        const own: Scope = { names: declared(scriptBody(node.instance)), up: outer };
        visit(node.module, null, node, 'module');
        visit(node.instance, outer, node, 'instance');
        visit(node.fragment, own, node, 'fragment');
        children(node, null, ['module', 'instance', 'fragment']);
        return;
      }
      case 'Program':
      case 'BlockStatement':
        children(node, { names: declared(node.body), up: scope });
        return;
      case 'FunctionDeclaration':
      case 'FunctionExpression':
      case 'ArrowFunctionExpression': {
        const names = new Set<string>();
        if (node.type === 'FunctionExpression') bind(node.id, names);
        for (const parameter of node.params as unknown[]) bind(parameter, names);
        children(node, { names, up: scope });
        return;
      }
      case 'EachBlock': {
        const names = new Set<string>();
        bind(node.context, names);
        if (typeof node.index === 'string') names.add(node.index);
        visit(node.expression, scope, node, 'expression');
        children(node, { names, up: scope }, ['expression']);
        return;
      }
      case 'Identifier': {
        // a property read by name and an object literal's key name no binding; every other name does
        const plain = parent !== null && parent.computed !== true;
        const property = plain && parent?.type === 'MemberExpression' && key === 'property';
        const label = plain && parent?.type === 'Property' && key === 'key';
        if (!property && !label && !resolves(node.name as string, scope)) found.free.add(node.name as string);
        break;
      }
      case 'MemberExpression': {
        const object = node.object as AstNode;
        if (node.computed) found.indexed.add(text.slice(object.start, object.end));
        else found.members.add((node.property as AstNode).name as string);
        break;
      }
      case 'ImportDeclaration':
        found.imports.add(String((node.source as AstNode).value));
        break;
      case 'RegularElement':
        found.elements.add(node.name as string);
        break;
      case 'Attribute':
        found.attributes.add(node.name as string);
        break;
      case 'BindDirective':
        found.attributes.add(`bind:${node.name as string}`);
        break;
    }
    children(node, scope);
  }

  visit(parse(text, { modern: true }), null, null, 'root');
  return found;
}

/** Each part of `found` that ALLOWED does not list, named by its part. */
function refusals(found: Reach): string[] {
  const listed = (part: keyof typeof ALLOWED, name: string) =>
    ALLOWED[part].has(name) || (part === 'members' && Object.hasOwn(en, name));
  const refused: string[] = [];
  const parts: [keyof typeof ALLOWED, keyof Reach, string][] = [
    ['imports', 'imports', 'import'],
    ['globals', 'free', 'global'],
    ['members', 'members', 'member'],
    ['indexed', 'indexed', 'indexed'],
    ['kinds', 'kinds', 'kind'],
    ['elements', 'elements', 'element'],
    ['attributes', 'attributes', 'attribute']
  ];
  for (const [part, seen, word] of parts) {
    for (const name of [...found[seen]].sort()) if (!listed(part, name)) refused.push(`${word} ${name}`);
  }
  return refused;
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('the card health panel', () => {
  it("the recall rows read the state's line in every state but Ready", async () => {
    for (const [state, line] of examined('state(s) but Ready', Object.entries(STATE_LINES))) {
      // a stray stored stability rides along on the record; the panel never shows it
      panel({ ...record({ state } as HealthReadout), stability: 12.34 } as CardHealth);
      const recall = screen.getByRole('region', { name: 'Recall' });
      expect(text(recall)).toBe(`Recall ${line}`);
      expect(text(recall)).not.toMatch(/\d/);
      expect(text(screen.getByRole('dialog'))).not.toContain('12.34');
      cleanup();
    }
    // Ready: the scheduler's two figures, under its label
    panel(record({ state: 'ready', recallPpm: 912_345, daysTo90: 12 }));
    expect(text(screen.getByRole('region', { name: 'Recall' }))).toBe(
      'Recall The scheduler estimates Chance of recall now: 91.2% Days until recall falls to 90%: 12'
    );
    cleanup();

    // the core's own records through the Worker: each state keeps its word and reads its line
    for (const [state, line] of examined('core record(s) in a state but Ready', Object.entries(STATE_LINES))) {
      const health = await throughWorker(coreJson({ state }));
      expect(health?.scheduler, state).toEqual({ state });
      panel(health);
      expect(text(screen.getByRole('region', { name: 'Recall' }))).toBe(`Recall ${line}`);
      cleanup();
    }
    // a Ready record with its two integer figures carries them, and every other field, across
    const ready = await throughWorker(coreJson({ state: 'ready', recall_ppm: 912_345, days_to_90: 12 }));
    expect(ready).toEqual({
      card: BigInt(CORE_CARD),
      reps: 7,
      lapses: 2,
      logged: { answers: 6, lapses: 1 },
      rows: 207,
      reviews: [
        { at: CORE_CARD + 7_000, grade: 3, kind: 1, interval: 4, ms: 29_998 },
        { at: CORE_CARD + 6_000, grade: 1, kind: 1, interval: -600, ms: 12_000 }
      ],
      totalMs: 129_999,
      meanMs: 21_667,
      medianMs: 16_000,
      scheduler: { state: 'ready', recallPpm: 912_345, daysTo90: 12 },
      provenance: { source: 'collection', createdMs: CORE_CARD }
    });
    panel(ready);
    expect(text(screen.getByRole('region', { name: 'Recall' }))).toBe(
      'Recall The scheduler estimates Chance of recall now: 91.2% Days until recall falls to 90%: 12'
    );
    cleanup();
    // a Ready part that lacks a figure or holds a fraction, and a word no state has, read as absent
    const malformed = examined('malformed scheduler part(s)', [
      { state: 'ready', recall_ppm: 912_345 },
      { state: 'ready', days_to_90: 12 },
      { state: 'ready', recall_ppm: 912_345.5, days_to_90: 12 },
      { state: 'ready', recall_ppm: 912_345, days_to_90: 1.5 },
      { state: 'certain' }
    ]);
    for (const part of malformed) {
      const health = await throughWorker(coreJson(part));
      expect(health?.scheduler, JSON.stringify(part)).toEqual({ state: 'absent' });
      panel(health);
      expect(text(screen.getByRole('region', { name: 'Recall' }))).toBe(`Recall ${STATE_LINES.absent}`);
      cleanup();
    }
  });

  it('the panel shows the counts, the capped times and the newest reviews first', () => {
    panel(record({ state: 'absent' }));
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    expect(text(dialog)).toContain('Reviews 4 Lapses 1');
    expect(text(dialog)).toContain('Time answering, each answer at most 60 s 100.0 s');
    expect(text(dialog)).toContain('Mean answer time 25.0 s Median answer time 16.0 s');
    // each row's cells, in order: a table holds no whitespace between them
    const rows = screen
      .getAllByRole('row')
      .slice(1)
      .map((row) => [...row.querySelectorAll('td')].map(text).join(' '));
    expect(rows).toEqual([
      '2023-11-14 Good 20.0 s',
      '2023-11-14 Set by hand 0.0 s',
      '2023-11-14 Again 12.0 s',
      '2023-11-14 Good 60.0 s',
      '2023-11-14 Good 8.0 s'
    ]);
    // the counters agree with the log and every row is shown: no line about the log, none about rows left out
    expect(text(dialog)).not.toContain('The review log holds');
    expect(text(dialog)).not.toContain('The newest');
    cleanup();
    // counters that differ from the log one count at a time: the panel shows the card's own counters,
    // and says what the log holds, as a fact
    const differing = examined('counter(s) apart from the log', [
      { extra: { reps: 6 }, shown: 'Reviews 6 Lapses 1' },
      { extra: { lapses: 3 }, shown: 'Reviews 4 Lapses 3' }
    ]);
    for (const { extra, shown } of differing) {
      panel(record({ state: 'absent' }, extra));
      expect(text(screen.getByRole('dialog')), shown).toContain(shown);
      expect(text(screen.getByRole('dialog')), shown).toContain('The review log holds 4 answers and 1 lapses.');
      cleanup();
    }
    // a card with more rows than the record carries: the panel says it shows the newest of them
    panel(record({ state: 'absent' }, { rows: 207 }));
    expect(text(screen.getByRole('dialog'))).toContain('The newest 5 of 207 reviews.');
  });

  it("the panel's strings label the estimate and name no verdict", () => {
    const strings = examined(
      'panel string(s)',
      Object.entries(en as Record<string, string>).filter(([key]) => key.startsWith('card_health_'))
    );
    expect((en as Record<string, string>).card_health_estimates).toBe('The scheduler estimates');
    expect(strings.filter(([, line]) => VERDICT.test(line))).toEqual([]);
    // each state's rendered panel holds no verdict either
    for (const state of [...Object.keys(STATE_LINES), 'ready']) {
      const readout = (state === 'ready' ? { state, recallPpm: 500_000, daysTo90: 3 } : { state }) as HealthReadout;
      panel(record(readout));
      const shown = text(screen.getByRole('dialog'))
        // the answer buttons' own names record what was pressed (SPEC-507 R8)
        .replace(/\b(Again|Hard|Good|Easy)\b/g, '');
      expect(VERDICT.test(shown), shown).toBe(false);
      cleanup();
    }
    // the census sees a planted verdict
    expect(['This card looks hard for you', 'a leech', 'at risk'].map((line) => VERDICT.test(line))).toEqual([
      true,
      true,
      true
    ]);
  });

  it('the panel says where the card came from', async () => {
    panel(record({ state: 'absent' }));
    expect(text(screen.getByRole('region', { name: 'Where it came from' }))).toBe(
      'Where it came from Came through your collection, created 2023-11-14.'
    );
    cleanup();
    // the Worker carries the core's created instant in epoch milliseconds, and the panel dates it
    const health = await throughWorker(coreJson({ state: 'absent' }));
    expect(health?.provenance).toEqual({ source: 'collection', createdMs: 1_712_345_678_901 });
    panel(health);
    expect(text(screen.getByRole('region', { name: 'Where it came from' }))).toBe(
      'Where it came from Came through your collection, created 2024-04-05.'
    );
  });

  it("the panel's module opens no fetch and adds no route", () => {
    // default-deny: each of the panel's two modules may import, read undeclared, read by name, index,
    // write in and mark up only what ALLOWED lists, so a fetch or a navigation by any name is refused
    const sources = examined('panel source(s)', [
      'src/lib/study/CardHealthPanel.svelte',
      'src/lib/study/card-health.ts'
    ]);
    const used = new Set<string>();
    for (const source of sources) {
      const found = reach(readFileSync(join(APP, source), 'utf8'), source.endsWith('.svelte'));
      console.log(
        `examined ${source}: ${found.imports.size} import(s), ${found.free.size} undeclared name(s), ` +
          `${found.members.size} member(s), ${found.indexed.size} indexed value(s), ${found.kinds.size} kind(s), ` +
          `${found.elements.size} element(s), ${found.attributes.size} attribute(s)`
      );
      expect(examined(`${source} kind(s)`, [...found.kinds]).length).toBeGreaterThan(0);
      expect(refusals(found), source).toEqual([]);
      for (const [part, names] of Object.entries(found)) for (const name of names) used.add(`${part} ${name}`);
    }
    // every entry of ALLOWED is one the two modules use, so the lists hold no slack
    const lists: [keyof typeof ALLOWED, keyof Reach][] = [
      ['imports', 'imports'],
      ['globals', 'free'],
      ['members', 'members'],
      ['indexed', 'indexed'],
      ['kinds', 'kinds'],
      ['elements', 'elements'],
      ['attributes', 'attributes']
    ];
    const unused = lists.flatMap(([part, seen]) => [...ALLOWED[part]].filter((name) => !used.has(`${seen} ${name}`)));
    expect(unused, 'entries of ALLOWED no source uses').toEqual([]);
    // the census refuses each planted reach, the aliased and the shadowed included
    const planted = examined('planted reach(es)', PLANTS);
    for (const { source, component, refused } of planted) {
      expect(refusals(reach(source, component)), source).toContain(refused);
    }
    // the record comes only through the review's read, which the client sends as one Worker operation
    const screenSource = readFileSync(join(APP, 'src/lib/study/ReviewScreen.svelte'), 'utf8');
    expect(screenSource).toMatch(/<CardHealthPanel\b/);
    expect(screenSource).toMatch(/review\.cardHealth\(\)/);
    expect(readFileSync(join(APP, 'src/lib/engine/client.ts'), 'utf8')).toMatch(/op: 'card-health'/);
    // the base's route files, frozen: any route added, under any name, is refused
    const routes = readdirSync(join(APP, 'src/routes'), { recursive: true, encoding: 'utf8' })
      .filter((path) => basename(path).startsWith('+'))
      .map((path) => path.split('\\').join('/'))
      .sort();
    expect(examined('route file(s)', routes)).toEqual(ROUTES);
  });
});

// ---- the panel on the review screen ----

function view(id: number): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false
  };
}

/** The engine as the screen sees it, with the card health read. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  async card(): Promise<Head> {
    this.calls.push('card');
    return this.heads.shift() as Head;
  }

  async rate(card: bigint, rating: number, ms: number): Promise<null> {
    this.calls.push(`rate ${card} ${rating} ${ms}`);
    return null;
  }

  async bury(): Promise<null> {
    return null;
  }

  async flag(): Promise<number> {
    return 0;
  }

  async undo(): Promise<null> {
    return null;
  }

  async undoOffer(): Promise<UndoOffer> {
    return { offer: null, why: 'none' };
  }

  async cardHealth(card: bigint): Promise<CardHealth | null> {
    this.calls.push(`card-health ${card}`);
    return record({ state: 'absent' }, { card });
  }
}

let now = 0;

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

describe('the card health panel on the review', () => {
  beforeEach(() => {
    now = 1000;
    vi.spyOn(performance, 'now').mockImplementation(() => now);
    Object.defineProperty(navigator, 'wakeLock', {
      configurable: true,
      value: { request: () => new Promise(() => undefined) }
    });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'visible' });
    localStorage.clear();
  });

  it('the card info panel pauses the answer clock', async () => {
    const counts = { new: 1, learning: 0, review: 0 };
    const client = new FakeClient([
      { counts, card: view(1) },
      { counts, card: null }
    ]);
    render(ReviewScreen, { client: async () => client });
    await settle();
    const region = screen.getByRole('region', { name: 'Review' });

    // 5 s on the question, then the panel opens over it for 30 s
    now += 5_000;
    await fireEvent.click(screen.getByRole('button', { name: 'Card info' }));
    await settle();
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    expect(dialog.contains(document.activeElement)).toBe(true);
    expect(screen.getByRole('region', { name: 'Recall' }).textContent).toContain('Not available in this build.');
    now += 30_000;
    // a key the review would take does nothing while the panel is open
    await fireEvent.keyDown(window, { key: ' ' });
    expect(screen.queryByRole('button', { name: 'Show answer' })).not.toBeNull();

    // Escape closes it, and the focus returns to the review
    await fireEvent.keyDown(window, { key: 'Escape' });
    await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(region);

    // 2 s more on the card: the grade sends 7 s, not 37 s, and names the card shown
    now += 2_000;
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect(client.calls).toEqual(['card', 'card-health 1', 'rate 1 3 7000', 'card']);
  });

  it('Tab and Shift+Tab stay inside the open panel', async () => {
    const client = new FakeClient([{ counts: { new: 1, learning: 0, review: 0 }, card: view(1) }]);
    render(ReviewScreen, { client: async () => client });
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Card info' }));
    await settle();
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    const close = screen.getByRole('button', { name: 'Close' });
    expect(document.activeElement).toBe(close);
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(dialog.contains(document.activeElement)).toBe(true);
    await fireEvent.keyDown(window, { key: 'Tab', shiftKey: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});
