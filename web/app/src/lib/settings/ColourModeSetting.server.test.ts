import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import ColourModeSetting from './ColourModeSetting.svelte';

// SPEC-593 R8; ADR-616 D1. Mutation coverage: the server renders no stored choice, so before the
// control mounts it shows the system choice checked, and only that one. The test run resolves
// Svelte's browser build, whose onMount refuses a server render; Svelte's server build makes
// onMount do nothing, so the test gives the control that server onMount.
vi.mock('svelte', async (original) => ({ ...(await original<typeof import('svelte')>()), onMount: () => undefined }));
describe('the colour mode setting before it mounts', () => {
  it('the server renders the system choice checked, and only it', () => {
    const { body } = render(ColourModeSetting);
    const radios = [...body.matchAll(/<input\b[^>]*>/g)].map(([tag]) => {
      const value = /\bvalue="([^"]*)"/.exec(tag)?.[1];
      return `${value} ${/\schecked\b/.test(tag) ? 'checked' : 'not checked'}`;
    });

    expect(radios).toEqual(['system checked', 'light not checked', 'dark not checked']);
    expect(body).toMatch(/<legend[^>]*>Colour mode<\/legend>/);
  });
});
