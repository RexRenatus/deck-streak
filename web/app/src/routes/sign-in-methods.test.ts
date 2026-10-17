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

/** The question the confirm step asks. */
const QUESTION = 'Remove this passkey? You can still sign in with Telegram.';

/** The day a passkey row shows, built from its instant as the day test builds it. */
function day(createdAt: number): string {
  return `Feb ${new Date(createdAt).getDate()}, 2001`;
}

/** The name a passkey's Remove carries: its place among the passkey rows, and its day. */
function removeName(position: number, createdAt: number): string {
  return `Remove passkey ${position}, added ${day(createdAt)}`;
}

/** A keyboard user's press of a row's Remove: the focus is on it first (a click moves none). */
async function open(row: HTMLElement): Promise<void> {
  const remove = within(row).getAllByRole('button')[0];
  remove.focus();
  await fireEvent.click(remove);
  await settle();
}

/** Opens a row's confirm step, then presses the step's first button, Remove. */
async function confirm(row: HTMLElement): Promise<void> {
  await open(row);
  await fireEvent.click(within(row).getAllByRole('button')[0]);
  await settle();
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
      expect(within(row).getByRole('button', { name: removeName(listed.indexOf(row), day.getTime()) })).toBeTruthy();
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
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }));
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
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }));
    await settle();
    await fireEvent.click(within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }));
    await settle();
    expect(lines(sent)).toEqual(['DELETE /api/identities/4']);
    expect(mocks.identities).toHaveBeenCalledTimes(2);
  });
});

describe('the passkey list keeps the focus and names each remove', () => {
  it('pressing remove moves the focus to keep in the confirm step', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    render(Methods);
    await settle();

    await open(rows()[1]);
    expect(document.activeElement).toBe(within(rows()[1]).getByRole('button', { name: 'Keep' }));
  });

  it('the confirm step is an alert dialog named by its question', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    render(Methods);
    await settle();

    await open(rows()[1]);
    const dialog = within(rows()[1]).queryByRole('alertdialog', { name: QUESTION });
    expect(dialog).not.toBeNull();
    const named = document
      .getElementById(dialog?.getAttribute('aria-labelledby') ?? '')
      ?.textContent?.trim();
    expect(named).toBe(QUESTION);
    expect(dialog?.contains(document.activeElement)).toBe(true);
  });

  it("keep puts the question away and gives the focus back to the row's remove", async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    const sent = server({});
    render(Methods);
    await settle();

    await open(rows()[1]);
    const keep = within(rows()[1]).getByRole('button', { name: 'Keep' });
    keep.focus();
    await fireEvent.click(keep);
    await settle();
    expect(screen.queryByText(QUESTION)).toBeNull();
    expect(lines(sent)).toEqual([]);
    expect(document.activeElement).toBe(within(rows()[1]).getByRole('button'));
  });

  it('escape keeps the passkey as keep does, and changes nothing while the removal is pending', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    const fetched = vi.fn(() => new Promise<Response>(() => undefined));
    vi.stubGlobal('fetch', fetched);
    render(Methods);
    await settle();

    // Escape on an open step keeps the passkey, sends nothing, and gives the focus back
    await open(rows()[1]);
    await fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
    await settle();
    expect(screen.queryByText(QUESTION)).toBeNull();
    expect(fetched).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(within(rows()[1]).getByRole('button'));

    // while the removal is pending, Keep is held and Escape changes nothing
    await confirm(rows()[1]);
    await fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
    await settle();
    expect(screen.queryByText(QUESTION)).not.toBeNull();
    expect(
      within(rows()[1])
        .getAllByRole('button')
        .map((button) => (button as HTMLButtonElement).disabled)
    ).toEqual([true, true]);
    expect(fetched).toHaveBeenCalledOnce();
  });

  it('after a removal the focus moves to the next remove, the previous one, or the list', async () => {
    const FOUR = { id: 4, kind: 'passkey', createdAt: Date.UTC(2001, 1, 3, 12) };
    const FIVE = { id: 5, kind: 'passkey', createdAt: Date.UTC(2001, 1, 4, 12) };
    const SIX = { id: 6, kind: 'passkey', createdAt: Date.UTC(2001, 1, 5, 12) };
    mocks.identities
      .mockResolvedValueOnce({ kind: 'ok', value: [LISTED[0], FOUR, FIVE, SIX] })
      .mockResolvedValueOnce({ kind: 'ok', value: [LISTED[0], FOUR, SIX] })
      .mockResolvedValueOnce({ kind: 'ok', value: [LISTED[0], FOUR] })
      .mockResolvedValueOnce({ kind: 'ok', value: [LISTED[0]] });
    server({
      'DELETE /api/identities/5': () => new Response(null, { status: 204 }),
      'DELETE /api/identities/6': () => new Response(null, { status: 204 }),
      'DELETE /api/identities/4': () => new Response(null, { status: 204 })
    });
    render(Methods);
    await settle();
    let examined = 0;

    // the middle row: the focus moves to the next passkey's Remove
    await confirm(rows()[2]);
    expect(document.activeElement).toBe(within(rows()[2]).getByRole('button'));
    expect(rows()[2].textContent).toContain(day(SIX.createdAt));
    examined += 1;
    // the last row: the focus moves to the one before it
    await confirm(rows()[2]);
    expect(document.activeElement).toBe(within(rows()[1]).getByRole('button'));
    examined += 1;
    // the only row: the focus moves to the list
    await confirm(rows()[1]);
    expect(document.activeElement).toBe(screen.getByRole('list', { name: 'Sign-in methods' }));
    examined += 1;

    console.log(`examined ${examined} of 3 removals`);
    expect(examined).toBe(3);
  });

  it("a refused removal gives the focus back to the row's remove", async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => Response.json({ reason: 'too_many_ceremonies' }, { status: 429 }))
    );
    render(Methods);
    await settle();

    await confirm(rows()[1]);
    expect(screen.getByRole('alert').textContent).toBe(
      'Too many attempts. Wait a minute, then start again.'
    );
    expect(document.activeElement).toBe(within(rows()[1]).getByRole('button'));
  });

  it('each remove names its passkey by place and day, and no two names are alike', async () => {
    const FOUR = { id: 4, kind: 'passkey', createdAt: Date.UTC(2001, 1, 3, 12) };
    const SIX = { id: 6, kind: 'passkey', createdAt: Date.UTC(2001, 1, 3, 12) };
    const FIVE = { id: 5, kind: 'passkey', createdAt: Date.UTC(2001, 1, 4, 12) };
    mocks.identities.mockResolvedValue({ kind: 'ok', value: [LISTED[0], FOUR, SIX, FIVE] });
    render(Methods);
    await settle();

    const names: string[] = [];
    for (const row of rows().slice(1)) {
      within(row).getAllByRole('button', {
        name: (accessibleName) => {
          names.push(accessibleName);
          return true;
        }
      });
    }
    expect(names).toEqual([
      removeName(1, FOUR.createdAt),
      removeName(2, SIX.createdAt),
      removeName(3, FIVE.createdAt)
    ]);
    expect(new Set(names).size).toBe(3);
    console.log(`examined ${names.length} of 3 Remove buttons`);
    expect(names).toHaveLength(3);

    // the open step's Remove carries its row's name
    await open(rows()[2]);
    expect(within(rows()[2]).getByRole('button', { name: removeName(2, SIX.createdAt) })).toBe(
      within(rows()[2]).getAllByRole('button')[0]
    );
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
        within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) })
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
      await fireEvent.click(within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }));
      await settle();
      await fireEvent.click(within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }));
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
    const remove = within(rows()[1]).getByRole('button', { name: removeName(1, LISTED[1].createdAt ?? 0) }) as HTMLButtonElement;
    expect(remove.disabled).toBe(false);

    // a passkey already gone answers no wording: nothing is shown, and the list is read again
    await confirm();
    answer(Response.json({ reason: 'identity_unknown' }, { status: 404 }));
    await settle();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(mocks.identities).toHaveBeenCalledTimes(2);
  });

  it('a key other than escape leaves the open question and the focus where they are', async () => {
    mocks.identities.mockResolvedValue({ kind: 'ok', value: LISTED });
    render(Methods);
    await settle();

    await open(rows()[1]);
    const keep = within(rows()[1]).getByRole('button', { name: 'Keep' });
    await fireEvent.keyDown(keep, { key: 'a' });
    await settle();
    expect(screen.queryByText(QUESTION)).not.toBeNull();
    expect(document.activeElement).toBe(keep);
  });
});
