/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Link from './link/+page.svelte';

// SPEC-385 R3, R6, A1, A8, A9, A18 (SPEC-359 A37, A44); ADR-399 D2, D3. The link page, opened in
// the browser with a link code in its fragment, redeems the code in a JSON body on the owner's tap,
// only where the browser offers WebAuthn, clears the fragment once the redeem has answered, and
// registers a passkey inside the link session. SvelteKit's navigation is replaced, and the server
// and the authenticator are stand-ins that record what they were sent.
const mocks = vi.hoisted(() => ({ replaceState: vi.fn(), goto: vi.fn() }));
vi.mock('$app/navigation', () => ({ replaceState: mocks.replaceState, goto: mocks.goto }));

const HOST = location.hostname;

interface Sent {
  readonly url: string;
  readonly method: string;
  readonly body: unknown;
}

/** A server answering `METHOD path` with its answer (599 for any other), recording every request. */
function server(answers: Record<string, () => Response | Promise<Response>>): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = String(input);
      const method = init.method ?? 'GET';
      sent.push({ url, method, body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined });
      return (await answers[`${method} ${url}`]?.()) ?? new Response(null, { status: 599 });
    })
  );
  return sent;
}

function lines(sent: readonly Sent[]): string[] {
  return sent.map((request) => `${request.method} ${request.url}`);
}

/** Every route of a whole registration, each answering as it succeeds. */
const REGISTERS: Record<string, () => Response> = {
  'POST /api/link/redeem': () => new Response(null, { status: 204 }),
  'POST /api/passkeys/register/start': () =>
    Response.json({
      publicKey: {
        rp: { id: HOST, name: 'DeckStreak' },
        user: { id: 'AA', name: 'owner', displayName: 'owner' },
        challenge: '----',
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }]
      }
    }),
  'POST /api/passkeys/register/finish': () => Response.json({ id: 3 }, { status: 201 })
};

/** The browser's authenticator, answering a create with a new credential. */
function authenticator() {
  const bytes = (...values: number[]) => Uint8Array.from(values).buffer;
  const create = vi.fn(async (options: CredentialCreationOptions) => {
    void options;
    return {
      id: '____',
      rawId: bytes(0xff, 0xff, 0xff),
      type: 'public-key',
      response: { attestationObject: bytes(0xfb, 0xef, 0xbe), clientDataJSON: bytes(0x00) }
    } as unknown as Credential;
  });
  Object.defineProperty(navigator, 'credentials', { configurable: true, value: { create } });
  vi.stubGlobal('PublicKeyCredential', class {});
  return { create };
}

/** Opens the page's address at `path`, its fragment and all. */
function at(path: string): void {
  history.replaceState(null, '', path);
}

/** Lets every pending request and render finish. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  mocks.replaceState.mockReset();
  delete (navigator as { credentials?: unknown }).credentials;
  sessionStorage.clear();
  localStorage.clear();
  at('/');
});

describe('the link page', () => {
  it('the link page posts the code from the fragment', async () => {
    at('/link#c0de-C0DE_');
    let answer: (response: Response) => void = () => undefined;
    const sent = server({
      ...REGISTERS,
      'POST /api/link/redeem': () => new Promise<Response>((resolve) => (answer = resolve))
    });
    authenticator();
    render(Link);
    await settle();

    // nothing is sent before the owner's tap
    expect(lines(sent)).toEqual([]);
    await fireEvent.click(screen.getByRole('button', { name: 'Create a passkey' }));
    await settle();
    // the code travels in the body of a request whose URL names no code
    expect(sent).toEqual([{ url: '/api/link/redeem', method: 'POST', body: { code: 'c0de-C0DE_' } }]);
    // the fragment stays until the redeem has answered
    expect(mocks.replaceState).not.toHaveBeenCalled();
    answer(new Response(null, { status: 204 }));
    await settle();
    expect(mocks.replaceState.mock.calls.map(([url]) => url)).toEqual(['/link']);
    expect(sent.every((request) => !request.url.includes('c0de'))).toBe(true);
  });

  it('without webauthn the link page keeps the code unspent', async () => {
    at('/link#c0de');
    const sent = server(REGISTERS);
    render(Link);
    await settle();

    // nothing is redeemed, so the code stays unspent and in the address
    expect(lines(sent)).toEqual([]);
    expect(mocks.replaceState).not.toHaveBeenCalled();
    expect(location.hash).toBe('#c0de');
    expect(screen.queryByRole('button', { name: 'Create a passkey' })).toBeNull();
    // and the page asks for the browser
    expect(screen.getByText('Open this link in your browser to create a passkey.')).toBeTruthy();
  });

  it('the link page registers a passkey after the code is redeemed', async () => {
    at('/link#c0de');
    const sent = server(REGISTERS);
    const { create } = authenticator();
    render(Link);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Create a passkey' }));
    await settle();

    expect(lines(sent)).toEqual([
      'POST /api/link/redeem',
      'POST /api/passkeys/register/start',
      'POST /api/passkeys/register/finish'
    ]);
    // the authenticator was asked once, with the options decoded to bytes
    expect(create).toHaveBeenCalledOnce();
    const publicKey = create.mock.calls[0][0].publicKey as unknown as Record<string, never>;
    expect([...(publicKey.challenge as Uint8Array)]).toEqual([0xfb, 0xef, 0xbe]);
    // one finish, in the server's shape
    expect(sent[2].body).toEqual({
      id: '____',
      rawId: '____',
      type: 'public-key',
      response: { attestationObject: '----', clientDataJSON: 'AA' },
      extensions: {}
    });
    expect(
      screen.getByText('Passkey created. You can now sign in to DeckStreak in this browser.')
    ).toBeTruthy();
  });

  it('a registration writes nothing to browser storage', async () => {
    at('/link#c0de');
    const stored = vi.spyOn(Storage.prototype, 'setItem');
    const opened = vi.fn();
    vi.stubGlobal('indexedDB', { open: opened });
    const sent = server(REGISTERS);
    authenticator();
    render(Link);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Create a passkey' }));
    await settle();

    // no storage write and no stored key, before or after the registration
    expect(stored).not.toHaveBeenCalled();
    expect(opened).not.toHaveBeenCalled();
    expect([sessionStorage.length, localStorage.length]).toEqual([0, 0]);
    // and the registration ran to its one posted finish
    expect(lines(sent).filter((line) => line.endsWith('/register/finish'))).toEqual([
      'POST /api/passkeys/register/finish'
    ]);
  });
});

// MUTATION COVERAGE (SPEC-385 §7): every other stage the page shows, each green when its test was
// written; none is a criterion of §3.
describe('the link page, each stage', () => {
  it('a link with no code asks for a new one and offers nothing', async () => {
    at('/link');
    const sent = server(REGISTERS);
    authenticator();
    render(Link);
    await settle();

    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Link a passkey');
    expect(screen.getByRole('alert').textContent).toBe(
      'This link has no code. Get a new one from DeckStreak in Telegram.'
    );
    expect(screen.queryByRole('button')).toBeNull();
    expect(lines(sent)).toEqual([]);
  });

  it('a spent code says so, registers nothing and offers no second try', async () => {
    at('/link#c0de');
    const sent = server({
      ...REGISTERS,
      'POST /api/link/redeem': () => Response.json({ reason: 'link_code_invalid' }, { status: 401 })
    });
    const { create } = authenticator();
    render(Link);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Create a passkey' }));
    await settle();

    expect(lines(sent)).toEqual(['POST /api/link/redeem']);
    expect(create).not.toHaveBeenCalled();
    expect(screen.getByRole('alert').textContent).toBe(
      'This link expired or was already used. Get a new one from DeckStreak in Telegram.'
    );
    expect(screen.queryByRole('button')).toBeNull();
    // the address no longer carries the code
    expect(mocks.replaceState.mock.calls.map(([url]) => url)).toEqual(['/link']);
  });

  it('a refused registration offers a new start without redeeming the code again', async () => {
    at('/link#c0de');
    const sent = server(REGISTERS);
    const { create } = authenticator();
    create.mockRejectedValueOnce(new DOMException('the owner cancelled', 'NotAllowedError'));
    render(Link);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Create a passkey' }));
    await settle();

    // the cancel is shown, nothing is finished, and the page offers a new start
    expect(screen.getByRole('alert').textContent).toBe('No passkey was used. You can start again.');
    expect(lines(sent)).toEqual(['POST /api/link/redeem', 'POST /api/passkeys/register/start']);
    expect(screen.queryByRole('status')).toBeNull();
    const again = screen.getByRole('button', { name: 'Create a passkey' }) as HTMLButtonElement;
    expect(again.disabled).toBe(false);

    // the new start registers inside the same link session, with no second redeem
    await fireEvent.click(again);
    await settle();
    expect(lines(sent)).toEqual([
      'POST /api/link/redeem',
      'POST /api/passkeys/register/start',
      'POST /api/passkeys/register/start',
      'POST /api/passkeys/register/finish'
    ]);
    expect(screen.getByRole('status').textContent).toBe(
      'Passkey created. You can now sign in to DeckStreak in this browser.'
    );
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('the button waits while a step is pending, and returns after a refusal', async () => {
    at('/link#c0de');
    let answer: (response: Response) => void = () => undefined;
    server({
      ...REGISTERS,
      'POST /api/link/redeem': () => new Promise<Response>((resolve) => (answer = resolve))
    });
    authenticator();
    render(Link);
    await settle();
    const button = () => screen.getByRole('button', { name: 'Create a passkey' }) as HTMLButtonElement;

    expect(button().disabled).toBe(false);
    await fireEvent.click(button());
    await settle();
    expect(button().disabled).toBe(true);
    answer(Response.json({ reason: 'too_many_ceremonies' }, { status: 429 }));
    await settle();
    expect(button().disabled).toBe(false);
    expect(screen.getByRole('alert').textContent).toBe(
      'Too many attempts. Wait a minute, then start again.'
    );
  });
});
