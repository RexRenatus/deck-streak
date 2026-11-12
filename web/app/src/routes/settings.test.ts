import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import Today from './+page.svelte';
import Settings from './settings/+page.svelte';

// SPEC-593 R8; ADR-616 D1. Mutation coverage: the settings screen is named in the page's language,
// holds the colour mode control and links back to Today, and Today links to it by name. The test
// run resolves Svelte's browser build, whose onMount refuses a server render; Svelte's server build
// makes onMount do nothing, so the colour mode control is given that server onMount.
vi.mock('svelte', async (original) => ({ ...(await original<typeof import('svelte')>()), onMount: () => undefined }));
describe('the settings screen', () => {
  it('the settings screen is named, holds the colour mode and links back to Today', () => {
    const { body } = render(Settings);

    expect(body).toMatch(/<h1[^>]*>Settings<\/h1>/);
    expect(body).toMatch(/<legend[^>]*>Colour mode<\/legend>/);
    expect([...body.matchAll(/<a href="([^"]*)"[^>]*>([^<]*)<\/a>/g)].map(([, href, text]) => `${href} ${text}`)).toEqual([
      '/ Back to Today'
    ]);
  });

  it('Today links to the settings screen by name', () => {
    const { body } = render(Today);

    expect([...body.matchAll(/<a href="\/settings"[^>]*>([^<]*)<\/a>/g)].map(([, text]) => text)).toEqual(['Settings']);
  });
});
