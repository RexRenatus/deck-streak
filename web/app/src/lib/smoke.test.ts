import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import { m } from '$lib/paraglide/messages.js';
import { baseLocale, locales } from '$lib/paraglide/runtime.js';
import Page from '../routes/+page.svelte';

describe('the Mini App shell', () => {
  it('renders the DeckStreak heading', () => {
    const { body } = render(Page);

    expect(body).toMatch(/<h1[^>]*>DeckStreak<\/h1>/);
  });

  it('compiles every locale, English first and Chinese named by its script', () => {
    expect(baseLocale).toBe('en');
    expect(locales).toEqual(['en', 'zh-Hans', 'zh-Hant', 'ja', 'ko', 'fr', 'es']);
  });

  it('has a tagline in every locale that names Anki', () => {
    for (const locale of locales) {
      expect(m.tagline({}, { locale })).toContain('Anki');
    }
  });
});
