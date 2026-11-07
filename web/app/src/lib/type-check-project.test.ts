import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { expect, test } from 'vitest';

// SPEC-628 R3, A1 and A2; ADR-650. svelte-check diagnoses only the files the include list of
// web/app/tsconfig.json names, and Playwright strips a browser suite's types without checking
// them, so a suite, harness page or configuration at the top of web/app that the list leaves out
// is never type-checked. This census reads the list by TypeScript's own configuration parser and
// names, sorted, each top-level member the list omits.
const APP = fileURLToPath(new URL('../..', import.meta.url));

/** A top-level entry of web/app: its name, and whether it is a directory. */
interface Entry {
  name: string;
  directory: boolean;
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(
    items.length,
    `examined 0 ${what}: the population is empty, so nothing was judged`
  ).toBeGreaterThan(0);
  return items;
}

/** The members R3 selects: a source, suite or harness directory, or a top-level script file. */
function members(entries: Entry[]): Entry[] {
  return entries.filter((entry) =>
    entry.directory
      ? /^(src|tests|tests-[a-z][a-z0-9-]*|[a-z][a-z0-9-]*-harness)$/.test(entry.name)
      : /^[a-z][a-z.-]*\.(ts|js)$/.test(entry.name) && !entry.name.endsWith('.d.ts')
  );
}

// Each member the list does not name, sorted: a directory is named bare, as `<dir>/**/*.ts` or
// as `<dir>/**/*`, and a file by its exact name.
function omitted(entries: Entry[], list: string[]): string[] {
  const named = new Set(list);
  const forms = (entry: Entry): string[] =>
    entry.directory ? [entry.name, `${entry.name}/**/*.ts`, `${entry.name}/**/*`] : [entry.name];
  return entries
    .filter((entry) => !forms(entry).some((form) => named.has(form)))
    .map((entry) => entry.name)
    .sort();
}

/** The include list of web/app/tsconfig.json, read by TypeScript's parser (comments allowed). */
function include(): string[] {
  const path = join(APP, 'tsconfig.json');
  const { config, error } = ts.parseConfigFileTextToJson(path, readFileSync(path, 'utf8'));
  expect(error, 'tsconfig.json does not parse').toBeUndefined();
  const list: unknown = config?.include;
  expect(Array.isArray(list), 'tsconfig.json names no include list').toBe(true);
  const entries: unknown[] = Array.isArray(list) ? list : [];
  const names = entries.filter((item): item is string => typeof item === 'string');
  expect(names.length, 'an include entry is not a string').toBe(entries.length);
  return names;
}

test('the type-check project names every browser suite, harness and configuration', () => {
  const top = readdirSync(APP, { withFileTypes: true }).map((entry) => ({
    name: entry.name,
    directory: entry.isDirectory()
  }));
  const selected = examined('top-level members', members(top));
  const list = include();
  const missing = omitted(selected, list);
  expect(missing, `the project omits ${missing.join(', ')}`).toEqual([]);
  const planted = omitted([...selected, { name: 'tests-planted', directory: true }], list);
  expect(planted, `a planted suite reads as ${planted.join(', ')}`).toEqual(['tests-planted']);
});

test('the census names a member the project omits', () => {
  const listing: Entry[] = [
    { name: 'tests-planted', directory: true },
    { name: 'planted-harness', directory: true },
    { name: 'vite.planted.config.ts', directory: false },
    { name: 'playwright.planted.config.ts', directory: false },
    { name: 'src', directory: true },
    { name: 'scripts', directory: true },
    { name: 'messages', directory: true },
    { name: 'tsconfig.json', directory: false },
    { name: 'stryker.config.json', directory: false },
    { name: 'app.d.ts', directory: false },
    { name: 'vite.config.ts.timestamp-1.mjs', directory: false }
  ];
  const selected = examined('planted members', members(listing));
  expect(selected).toEqual(listing.slice(0, 5));
  expect(omitted(selected, ['src'])).toEqual([
    'planted-harness',
    'playwright.planted.config.ts',
    'tests-planted',
    'vite.planted.config.ts'
  ]);
  expect(
    omitted(selected, [
      'src',
      'tests-planted/**/*.ts',
      'planted-harness/**/*',
      'vite.planted.config.ts',
      'playwright.planted.config.ts'
    ])
  ).toEqual([]);
  expect(
    omitted(selected, [
      'src',
      'tests-planted',
      'planted-harness',
      'vite.planted.config.ts',
      'playwright.planted.config.ts'
    ])
  ).toEqual([]);
  expect(
    omitted(selected, [
      'src',
      'tests-planted/*.ts',
      'planted-harness/**/*.ts',
      'vite.planted.config.ts'
    ])
  ).toEqual(['playwright.planted.config.ts', 'tests-planted']);
});
