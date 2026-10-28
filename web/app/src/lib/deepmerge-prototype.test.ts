/**
 * SPEC-499, ADR-517. deepmerge 4.3.1 (GHSA-gcqx-w48m-w2h4) lets an own `__proto__` key in a merged
 * source, the shape `JSON.parse` makes, set the merged object's prototype. SvelteKit's Vite plugin
 * merges its options with deepmerge, so this file reaches deepmerge the way the plugin does: from
 * the plugin's own installed directory, never by a direct import that web/app does not have.
 *
 * T1, T2 and T6 are the advisory's behaviour and the shape of the fix (upstream's open pull request
 * TehShrike/deepmerge#277 keeps the key as an own data property). T3 to T5 are controls that any
 * fix must keep.
 */
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

type Options = { arrayMerge?: (target: unknown[], source: unknown[]) => unknown[] };
type Merge = (target: object, source: object, options?: Options) => Record<string, unknown>;

/** deepmerge as `@sveltejs/vite-plugin-svelte` resolves it, from that package's own directory. */
function deepmergeOfThePlugin(): Merge {
  const plugin = createRequire(import.meta.url).resolve('@sveltejs/vite-plugin-svelte');
  return createRequire(plugin)('deepmerge') as Merge;
}

const deepmerge = deepmergeOfThePlugin();

/** A value `JSON.parse` returns, whose `__proto__` key is an own property of the object. */
function parsed(text: string): Record<string, unknown> {
  return JSON.parse(text) as Record<string, unknown>;
}

describe('deepmerge as the SvelteKit Vite plugin resolves it (GHSA-gcqx-w48m-w2h4)', () => {
  it('T1 a nested own __proto__ key merged over a primitive leaves the prototype alone', () => {
    const merged = deepmerge({ a: 1 }, parsed('{"a": {"__proto__": {"polluted": "yes"}}}'));
    const inner = merged.a as Record<string, unknown>;
    expect(inner.polluted).toBeUndefined();
    expect(Object.getPrototypeOf(inner)).toBe(Object.prototype);
  });

  it('T2 a top-level own __proto__ key merged over a null prototype leaves the prototype alone', () => {
    const source = parsed('{"__proto__": {"polluted": "yes"}}');
    const merged = deepmerge(Object.create(null) as object, source);
    expect(merged.polluted).toBeUndefined();
    expect(Object.getPrototypeOf(merged)).toBe(Object.prototype);
  });

  it("T3 the plugin's array option replaces arrays and objects still merge deeply", () => {
    const merged = deepmerge(
      { x: [1], o: { b: 1 } },
      { x: [2], o: { c: 2 } },
      { arrayMerge: (target, source) => source ?? target }
    );
    expect(merged).toEqual({ x: [2], o: { b: 1, c: 2 } });
  });

  it('T4 a deep merge leaves both of its inputs unchanged', () => {
    const target = { k: { m: 1 } };
    const source = { k: { n: 2 }, z: 3 };
    expect(deepmerge(target, source)).toEqual({ k: { m: 1, n: 2 }, z: 3 });
    expect(target).toEqual({ k: { m: 1 } });
    expect(source).toEqual({ k: { n: 2 }, z: 3 });
  });

  it('T5 Object.prototype gains no property from the merges', () => {
    deepmerge({ a: 1 }, parsed('{"a": {"__proto__": {"polluted": "yes"}}}'));
    deepmerge(Object.create(null) as object, parsed('{"__proto__": {"polluted": "yes"}}'));
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'polluted')).toBe(false);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it('T6 an own __proto__ key stays an own data property, cloned, and the prototype unchanged', () => {
    const source = parsed('{"__proto__": {"admin": true}}');
    const merged = deepmerge(Object.create(null) as object, source);
    const own = Object.getOwnPropertyDescriptor(merged, '__proto__');
    expect(own).toBeDefined();
    expect(own?.value).toEqual({ admin: true });
    expect(own?.value).not.toBe(Object.getOwnPropertyDescriptor(source, '__proto__')?.value);
    expect(Object.getPrototypeOf(merged)).toBe(Object.prototype);
  });
});
