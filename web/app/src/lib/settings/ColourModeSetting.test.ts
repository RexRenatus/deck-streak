/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ColourModeSetting from './ColourModeSetting.svelte';

// SPEC-593 R8; ADR-616 D1. Mutation coverage: the settings control names the colour mode and its
// three choices in the page's language, shows the stored choice once it has mounted, and keeps a
// new choice on the device and applies it to the document's root element. The storage key is
// spelled out here, never read from the module.
const KEY = 'deckstreak.colour-mode';

afterEach(() => {
  localStorage.clear();
  document.documentElement.className = '';
});

/** Each choice the control shows, in order, as `<label> <value> <checked or not>`. */
function choices(): string[] {
  return screen.getAllByRole<HTMLInputElement>('radio').map((input) => {
    const label = input.closest('label')?.textContent?.trim();
    return `${label} ${input.value} ${input.checked ? 'checked' : 'not checked'}`;
  });
}

describe('the colour mode setting', () => {
  it('names the setting and each of its three choices', async () => {
    const { container } = render(ColourModeSetting);
    await tick();

    expect(container.querySelector('legend')?.textContent).toBe('Colour mode');
    expect(choices()).toEqual(['System system checked', 'Light light not checked', 'Dark dark not checked']);
  });

  it('shows the stored choice once it has mounted', async () => {
    localStorage.setItem(KEY, 'light');
    render(ColourModeSetting);
    await tick();

    expect(choices()).toEqual(['System system not checked', 'Light light checked', 'Dark dark not checked']);
  });

  it('a new choice is kept on the device and applied to the page', async () => {
    render(ColourModeSetting);
    await tick();

    await fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(localStorage.getItem(KEY)).toBe('dark');
    expect([...document.documentElement.classList]).toEqual(['dark']);
    expect(choices()).toEqual(['System system not checked', 'Light light not checked', 'Dark dark checked']);

    await fireEvent.click(screen.getByRole('radio', { name: 'System' }));
    expect(localStorage.getItem(KEY)).toBe('system');
    expect([...document.documentElement.classList]).toEqual([]);
  });
});
