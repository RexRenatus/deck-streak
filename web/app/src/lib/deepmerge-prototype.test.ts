/**
 * SPEC-499, ADR-517. deepmerge 4.3.1 (GHSA-gcqx-w48m-w2h4) lets an own `__proto__` key in a merged
 * source, the shape `JSON.parse` makes, set the merged object's prototype. SvelteKit's Vite plugin
 * merges its options with deepmerge, so this file reaches deepmerge the way the plugin does: from
 * the plugin's own installed directory, never by a direct import that web/app does not have.
 *
 * T1, T2 and T6 are the advisory's behaviour and the shape of the fix (upstream's open pull request
 * TehShrike/deepmerge#277 keeps the key as an own data property). T3 to T5 are controls that any
 * fix must keep. T7 to T9 pin the rest of that shape: the key on the target's side, its property
 * flags, and the key on both sides.
 */
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

type Options = {
  arrayMerge?: (target: unknown[], source: unknown[]) => unknown[];
  isMergeableObject?: (value: unknown) => boolean;
};
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

/**
 * A null-prototype object with an own enumerable `__proto__` data property, built the way
 * upstream's tests build one.
 */
function withOwnProtoKey(value: object): Record<string, unknown> {
  const holder = Object.create(null) as Record<string, unknown>;
  Object.defineProperty(holder, '__proto__', {
    value,
    enumerable: true,
    writable: true,
    configurable: true
  });
  return holder;
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

  it('T7 an own __proto__ key of a null-prototype target stays an own data property, cloned', () => {
    // Upstream pull request 277's case 'preserving an own __proto__ data property from a
    // null-prototype target', reading the kept key through its own descriptor.
    const proto = { admin: true };
    const merged = deepmerge(withOwnProtoKey(proto), {});
    expect(Object.getPrototypeOf(merged)).toBe(Object.prototype);
    const own = Object.getOwnPropertyDescriptor(merged, '__proto__');
    expect(own?.value).toEqual({ admin: true });
    expect(own?.value).not.toBe(proto);
    expect(merged.admin).toBeUndefined();
  });

  it('T8 the kept own __proto__ key is enumerable, writable and configurable', () => {
    const merged = deepmerge(Object.create(null) as object, parsed('{"__proto__": {"admin": true}}'));
    expect(Object.getOwnPropertyDescriptor(merged, '__proto__')).toEqual({
      value: { admin: true },
      enumerable: true,
      writable: true,
      configurable: true
    });
  });

  it('T9 an own __proto__ key on both target and source merges into one own data property', () => {
    const merged = deepmerge(parsed('{"__proto__": {"a": 1}}'), parsed('{"__proto__": {"b": 2}}'));
    expect(Object.getPrototypeOf(merged)).toBe(Object.prototype);
    expect(Object.getOwnPropertyDescriptor(merged, '__proto__')?.value).toEqual({ a: 1, b: 2 });

    // deepmerge's isMergeableObject option declines the null-prototype target here, so the target's
    // own keys are not copied first and the deep-merge branch alone writes the key.
    const plainOnly = (value: unknown) =>
      value !== null && typeof value === 'object' && Object.getPrototypeOf(value) === Object.prototype;
    const declined = deepmerge(withOwnProtoKey({ a: 1 }), parsed('{"__proto__": {"b": 2}}'), {
      isMergeableObject: plainOnly
    });
    expect(Object.getPrototypeOf(declined)).toBe(Object.prototype);
    expect(Object.getOwnPropertyDescriptor(declined, '__proto__')?.value).toEqual({ a: 1, b: 2 });
  });
});
