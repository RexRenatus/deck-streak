/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Today from './+page.svelte';
import Methods from './sign-in-methods/+page.svelte';

// SPEC-385 R6, R7, R10, R12, A3, A4, A16, A17 (SPEC-359 A39, A40); ADR-399 D2, D3. The sign-in
// methods screen lists the owner's ways in, Telegram first and never removable; inside Telegram,
// "Link a passkey" mints a code and opens the link page in the browser through the Mini App's link
// opener, with no ceremony in the frame; a refused mint posts no launch data; a removal asks first
// and sends one delete. The API client and Telegram's wrapper are replaced before the screen's
// imports run; the mint and the removal reach a stand-in server that records what it was sent.
const mocks = vi.hoisted(() => ({
  identities: vi.fn(),
  me: vi.fn(),
  telegram: { launchData: 'auth_date=1&hash=synthetic' as string | null, openLink: vi.fn() }
}));
vi.mock('$lib/api', () => ({ api: { identities: mocks.identities, me: mocks.me } }));
vi.mock('$lib/telegram.svelte', () => ({ telegram: mocks.telegram }));

/** Telegram, then two passkeys, as the client lists them. */
const LISTED = [
  { id: 0, kind: 'telegram' },
  { id: 4, kind: 'passkey', createdAt: Date.UTC(2001, 1, 3, 12) },
  { id: 5, kind: 'passkey', createdAt: Date.UTC(2001, 1, 4, 12) }
];

interface Sent {
  readonly url: string;
  readonly method: string;
}

/** A server answering `METHOD path` with its answer (599 for any other), recording every request. */
function server(answers: Record<string, () => Response>): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = String(input);
      const method = init.method ?? 'GET';
      sent.push({ url, method });
      return answers[`${method} ${url}`]?.() ?? new Response(null, { status: 599 });
    })
  );
  return sent;
}

function lines(sent: readonly Sent[]): string[] {
  return sent.map((request) => `${request.method} ${request.url}`);
}

/** The browser's authenticator, which no step of this screen may call. */
function authenticator() {
  const create = vi.fn();
  const get = vi.fn();
  Object.defineProperty(navigator, 'credentials', { configurable: true, value: { create, get } });
  vi.stubGlobal('PublicKeyCredential', class {});
  return { create, get };
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** Each row of the methods list. */
function rows(): HTMLElement[] {
  return screen.getAllByRole('listitem');
}

afterEach(() => {
  vi.unstubAllGlobals();
  mocks.identities.mockReset();
  mocks.me.mockReset();
  mocks.telegram.openLink.mockReset();
  mocks.telegram.launchData = 'auth_date=1&hash=synthetic';
  delete (navigator as { credentials?: unknown }).credentials;
});

describe('the sign-in methods screen', () => {
  it('the sign-in section offers no unlink for telegram', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[1], LISTED[0], LISTED[2]] });
    render(Methods);
    await settle();

    expect(screen.getByRole('heading', { level: 1, name: 'Sign-in methods' })).toBeTruthy();
    const listed = rows();
    expect(listed).toHaveLength(3);
    // Telegram first, with no removal
    expect(listed[0].textContent).toContain('Telegram');
    expect(within(listed[0]).queryAllByRole('button')).toEqual([]);
    // each passkey with its date and a removal
    for (const [row, day] of [
      [listed[1], new Date(LISTED[1].createdAt ?? 0)],
      [listed[2], new Date(LISTED[2].createdAt ?? 0)]
    ] as const) {
      expect(row.textContent).toContain('Passkey');
      expect(row.textContent).toContain(`Added `);
      expect(row.textContent).toContain(String(day.getDate()));
      expect(within(row).getByRole('button', { name: 'Remove' })).toBeTruthy();
    }

    // and Today links to the screen
    mocks.me.mockResolvedValue({ kind: 'unavailable' });
    render(Today);
    const link = screen.getByRole('link', { name: 'Sign-in methods' });
    expect(link.getAttribute('href')).toBe('/sign-in-methods');
  });

  it('link a passkey opens the link page in the browser', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0]] });
    const sent = server({ 'POST /api/link/code': () => Response.json({ code: 'c0de' }) });
    const { create, get } = authenticator();
    render(Methods);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Link a passkey' }));
    await settle();

    // no ceremony runs in Telegram's frame
    expect([create.mock.calls.length, get.mock.calls.length]).toEqual([0, 0]);
    // one mint, then the link page at this origin, the code in its fragment, through the opener
    expect(lines(sent)).toEqual(['POST /api/link/code']);
    expect(mocks.telegram.openLink.mock.calls).toEqual([[`${location.origin}/link#c0de`]]);
  });

  it('a stale telegram session asks for a reopen before a link', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0]] });
    const sent = server({
      'POST /api/link/code': () => Response.json({ reason: 'reauth_required' }, { status: 401 }),
      'POST /api/session': () => new Response(null, { status: 200 })
    });
    render(Methods);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Link a passkey' }));
    await settle();

    // no launch data is posted again, the mint is sent once, and no link opens
    expect(lines(sent)).toEqual(['POST /api/link/code']);
    expect(mocks.telegram.openLink).not.toHaveBeenCalled();
    // the owner is asked to reopen DeckStreak from Telegram
    expect(screen.getByText('Reopen DeckStreak from Telegram, then try again.')).toBeTruthy();
  });

  it('removing a passkey asks first and sends one delete', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0], LISTED[1]] });
    const sent = server({ 'DELETE /api/identities/4': () => new Response(null, { status: 204 }) });
    render(Methods);
    await settle();

    // the first tap asks, and sends nothing
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: 'Remove' }));
    await settle();
    expect(lines(sent)).toEqual([]);
    expect(
      screen.getByText('Remove this passkey? You can still sign in with Telegram.')
    ).toBeTruthy();
    // Keep sends nothing, and puts the question away
    await fireEvent.click(screen.getByRole('button', { name: 'Keep' }));
    await settle();
    expect(lines(sent)).toEqual([]);
    expect(screen.queryByText('Remove this passkey? You can still sign in with Telegram.')).toBeNull();
    // Remove, then Remove again, sends one delete, and the list is read again
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: 'Remove' }));
    await settle();
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: 'Remove' }));
    await settle();
    expect(lines(sent)).toEqual(['DELETE /api/identities/4']);
    expect(mocks.identities).toHaveBeenCalledTimes(2);
  });
});

// MUTATION COVERAGE (SPEC-385 §7): every other state the screen shows, each green when its test
// was written; none is a criterion of §3.
describe('the sign-in methods screen, each state', () => {
  it('the screen says it is loading until the list is read', async () => {
    let answer: (listed: unknown) => void = () => undefined;
    mocks.identities.mockReturnValue(new Promise((resolve) => (answer = resolve)));
    render(Methods);
    await settle();

    expect(screen.getByRole('status').textContent).toBe('Loading…');
    expect(screen.queryAllByRole('listitem')).toEqual([]);
    answer({ kind: 'ok', value: [LISTED[0]] });
    await settle();
    expect(screen.queryByRole('status')).toBeNull();
    expect(rows()).toHaveLength(1);
    // and the screen leads back to Today
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
  });

  it('a list the screen cannot read says why, and shows no row', async () => {
    const ANSWERS = [
      ['reopen', 'Reopen DeckStreak from Telegram to sign in again.'],
      ['unavailable', 'DeckStreak could not reach its server. Try again in a moment.']
    ] as const;
    let judged = 0;
    for (const [kind, text] of ANSWERS) {
      mocks.identities.mockResolvedValue({ kind });
      const shown = render(Methods);
      await settle();
      expect([kind, screen.getByRole('alert').textContent]).toEqual([kind, text]);
      expect(screen.queryAllByRole('listitem')).toEqual([]);
      shown.unmount();
      judged += 1;
    }
    console.log(`examined ${judged} of ${ANSWERS.length} answers`);
    expect(judged).toBe(ANSWERS.length);
  });

  it('a passkey row shows the day it was added', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0], LISTED[1]] });
    render(Methods);
    await settle();

    const day = new Date(LISTED[1].createdAt ?? 0).getDate();
    expect(rows()[1].textContent).toContain(`Added Feb ${day}, 2001`);
  });

  it('outside telegram the screen offers no link', async () => {
    mocks.telegram.launchData = null;
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0]] });
    render(Methods);
    await settle();

    expect(rows()).toHaveLength(1);
    expect(screen.queryByRole('button', { name: 'Link a passkey' })).toBeNull();
  });

  it('a mint holds every button until it answers, and a refused one shows why', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0], LISTED[1]] });
    let answer: (response: Response) => void = () => undefined;
    const fetched = vi.fn(() => new Promise<Response>((resolve) => (answer = resolve)));
    vi.stubGlobal('fetch', fetched);
    render(Methods);
    await settle();
    const held = () =>
      [
        screen.getByRole('button', { name: 'Link a passkey' }),
        within(rows()[1]).getByRole('button', { name: 'Remove' })
      ].map((button) => (button as HTMLButtonElement).disabled);

    expect(held()).toEqual([false, false]);
    await fireEvent.click(screen.getByRole('button', { name: 'Link a passkey' }));
    await settle();
    expect(held()).toEqual([true, true]);
    answer(Response.json({ reason: 'too_many_ceremonies' }, { status: 429 }));
    await settle();
    expect(held()).toEqual([false, false]);
    expect(screen.getByRole('alert').textContent).toBe(
      'Too many attempts. Wait a minute, then start again.'
    );
    expect(fetched).toHaveBeenCalledOnce();
    expect(mocks.telegram.openLink).not.toHaveBeenCalled();
  });

  it('a removal holds its buttons until it answers; a refusal shows why, and a gone one is read away', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0], LISTED[1]] });
    let answer: (response: Response) => void = () => undefined;
    vi.stubGlobal(
      'fetch',
      vi.fn(() => new Promise<Response>((resolve) => (answer = resolve)))
    );
    render(Methods);
    await settle();
    const confirm = async () => {
      await fireEvent.click(within(rows()[1]).getByRole('button', { name: 'Remove' }));
      await settle();
      await fireEvent.click(within(rows()[1]).getByRole('button', { name: 'Remove' }));
      await settle();
    };

    // while the delete is pending, Remove and Keep are both held
    await confirm();
    const pending = within(rows()[1])
      .getAllByRole('button')
      .map((button) => [button.textContent?.trim(), (button as HTMLButtonElement).disabled]);
    expect(pending).toEqual([
      ['Remove', true],
      ['Keep', true]
    ]);
    // a refusal with its wording is shown, the list is not read again, and Remove returns
    answer(Response.json({ reason: 'too_many_ceremonies' }, { status: 429 }));
    await settle();
    expect(screen.getByRole('alert').textContent).toBe(
      'Too many attempts. Wait a minute, then start again.'
    );
    expect(mocks.identities).toHaveBeenCalledOnce();
    const remove = within(rows()[1]).getByRole('button', { name: 'Remove' }) as HTMLButtonElement;
    expect(remove.disabled).toBe(false);

    // a passkey already gone answers no wording: nothing is shown, and the list is read again
    await confirm();
    answer(Response.json({ reason: 'identity_unknown' }, { status: 404 }));
    await settle();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(mocks.identities).toHaveBeenCalledTimes(2);
  });
});
