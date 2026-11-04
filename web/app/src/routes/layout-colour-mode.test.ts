/**
 * @vitest-environment jsdom
 */
import { render } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Layout from './+layout.svelte';

// SPEC-593 R8; ADR-616 D1. Mutation coverage: once the root layout has mounted, the stored colour
// mode is set on the document's root element. The feed's client, the wallet's read, Telegram's
// wrapper and SvelteKit's page state are replaced before the layout's imports run, as the root
// layout's own tests replace them. The storage key is spelled out here, never read from the module.
const mocks = vi.hoisted(() => ({ ready: vi.fn(), feed: vi.fn(), wallet: vi.fn() }));
vi.mock('#lib/telegram.svelte.js', () => ({ telegram: { ready: mocks.ready } }));
vi.mock('#lib/api.js', () => ({ api: { feed: mocks.feed, wallet: mocks.wallet } }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/') } }));

const KEY = 'deckstreak.colour-mode';

afterEach(() => {
  localStorage.clear();
  document.documentElement.className = '';
});

/** The layout around a synthetic screen. */
function mount() {
  mocks.feed.mockResolvedValue({ kind: 'unavailable' });
  mocks.wallet.mockResolvedValue({ kind: 'unavailable' });
  const children = createRawSnippet(() => ({ render: () => '<main>A synthetic screen</main>' }));
  return render(Layout, { props: { children } });
}

describe('the root layout and the colour mode', () => {
  it('sets the stored colour mode on the root element once it has mounted', () => {
    for (const [stored, want] of [
      ['dark', ['dark']],
      ['light', ['light']]
    ] as const) {
      localStorage.setItem(KEY, stored);
      document.documentElement.className = '';
      const { unmount } = mount();
      expect([stored, [...document.documentElement.classList]]).toEqual([stored, want]);
      unmount();
    }
  });

  it('clears both classes for the system choice', () => {
    localStorage.setItem(KEY, 'system');
    document.documentElement.className = 'dark light';
    mount();
    expect([...document.documentElement.classList]).toEqual([]);
  });
});
