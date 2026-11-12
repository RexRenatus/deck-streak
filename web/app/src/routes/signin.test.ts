/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SignIn from './signin/+page.svelte';

// SPEC-385 R5, R8, R11, A2, A10, A12 (SPEC-359 A38); ADR-399 D2, D3. Outside Telegram the
// sign-in page offers a passkey where the browser offers WebAuthn, posts one finish per start, and
// after a sign-in opens Today at its fixed path; a refusal leaves the page where it is. SvelteKit's
// navigation is replaced, and the server and the authenticator are stand-ins that record what
// they were sent.
const mocks = vi.hoisted(() => ({ goto: vi.fn(), replaceState: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto: mocks.goto, replaceState: mocks.replaceState }));

const HOST = location.hostname;
const OFFER = 'Sign in with a passkey';

interface Sent {
  readonly url: string;
  readonly method: string;
  readonly body: unknown;
}

/** A server answering `METHOD path` with its answer (599 for any other), recording every request. */
function server(answers: Record<string, () => Response>): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = String(input);
      const method = init.method ?? 'GET';
      sent.push({ url, method, body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined });
      return answers[`${method} ${url}`]?.() ?? new Response(null, { status: 599 });
    })
  );
  return sent;
}

function lines(sent: readonly Sent[]): string[] {
  return sent.map((request) => `${request.method} ${request.url}`);
}

/** A sign-in's start, answering the owner's one credential; its finish answers `finish`. */
function signingIn(finish: () => Response): Record<string, () => Response> {
  return {
    'POST /api/passkeys/sign-in/start': () =>
      Response.json({
        publicKey: {
          rpId: HOST,
          challenge: '____',
          allowCredentials: [{ type: 'public-key', id: '----' }],
          userVerification: 'required'
        }
      }),
    'POST /api/passkeys/sign-in/finish': finish
  };
}

/** The browser's authenticator, answering a get with an assertion. */
function authenticator() {
  const bytes = (...values: number[]) => Uint8Array.from(values).buffer;
  const get = vi.fn(async () => {
    return {
      id: '____',
      rawId: bytes(0xff, 0xff, 0xff),
      type: 'public-key',
      response: {
        authenticatorData: bytes(0x00),
        clientDataJSON: bytes(0xfb, 0xef, 0xbe),
        signature: bytes(0xff, 0xff, 0xff),
        userHandle: bytes(0x00)
      }
    } as unknown as Credential;
  });
  Object.defineProperty(navigator, 'credentials', { configurable: true, value: { get } });
  vi.stubGlobal('PublicKeyCredential', class {});
  return { get };
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

afterEach(() => {
  vi.unstubAllGlobals();
  mocks.goto.mockReset();
  delete (navigator as { credentials?: unknown }).credentials;
  history.replaceState(null, '', '/');
});

describe('the sign-in page', () => {
  it('outside telegram the shell offers sign-in', async () => {
    server({});
    authenticator();
    const offered = render(SignIn);
    expect(screen.getByRole('heading', { name: 'Sign in' })).toBeTruthy();
    expect(screen.getByRole('button', { name: OFFER })).toBeTruthy();
    offered.unmount();
    vi.unstubAllGlobals();
    delete (navigator as { credentials?: unknown }).credentials;

    // a browser with no WebAuthn is offered no passkey, and is told to open one that can
    render(SignIn);
    expect(screen.queryByRole('button', { name: OFFER })).toBeNull();
    expect(
      screen.getByText(
        "This browser can't use passkeys. Open DeckStreak in a browser that can, or reopen it from Telegram."
      )
    ).toBeTruthy();
  });

  it('a passkey sign-in opens today', async () => {
    history.replaceState(null, '', '/signin?next=/wallet');
    const sent = server(
      signingIn(() => Response.json({ credential_ids: ['____'], user_handle: 'AA' }))
    );
    authenticator();
    mocks.goto.mockResolvedValue(undefined);
    render(SignIn);
    await fireEvent.click(screen.getByRole('button', { name: OFFER }));
    await settle();

    expect(lines(sent)).toEqual([
      'POST /api/passkeys/sign-in/start',
      'POST /api/passkeys/sign-in/finish'
    ]);
    // Today, at its fixed path, whatever the address asked for
    expect(mocks.goto.mock.calls).toEqual([['/']]);
    // after one finish, in the server's shape
    expect(sent[1].body).toEqual({
      id: '____',
      rawId: '____',
      type: 'public-key',
      response: { authenticatorData: 'AA', clientDataJSON: '----', signature: '____', userHandle: 'AA' },
      extensions: {}
    });
  });

  it('a refused finish is never posted again', async () => {
    const sent = server(
      signingIn(() => Response.json({ reason: 'challenge_expired' }, { status: 401 }))
    );
    const { get } = authenticator();
    render(SignIn);
    await fireEvent.click(screen.getByRole('button', { name: OFFER }));
    await settle();

    // the refused finish was posted once, and the page went nowhere
    expect(lines(sent)).toEqual([
      'POST /api/passkeys/sign-in/start',
      'POST /api/passkeys/sign-in/finish'
    ]);
    expect(mocks.goto).not.toHaveBeenCalled();
    // it says why, and offers a new start
    expect(
      screen.getByText('That attempt expired or was already used. Start again.')
    ).toBeTruthy();
    const again = screen.getByRole('button', { name: OFFER });
    expect((again as HTMLButtonElement).disabled).toBe(false);
    await fireEvent.click(again);
    await settle();
    expect(lines(sent)).toEqual([
      'POST /api/passkeys/sign-in/start',
      'POST /api/passkeys/sign-in/finish',
      'POST /api/passkeys/sign-in/start',
      'POST /api/passkeys/sign-in/finish'
    ]);
    expect(get).toHaveBeenCalledTimes(2);
  });
});

// MUTATION COVERAGE (SPEC-385 §7): the button's pending state, green when its test was written; it
// is no criterion of §3.
describe('the sign-in page, while it works', () => {
  it('the button waits while a sign-in is pending, and returns after a refusal', async () => {
    let answer: (response: Response) => void = () => undefined;
    const fetched = vi.fn(() => new Promise<Response>((resolve) => (answer = resolve)));
    vi.stubGlobal('fetch', fetched);
    authenticator();
    render(SignIn);
    await settle();
    const button = () => screen.getByRole('button', { name: OFFER }) as HTMLButtonElement;

    expect(button().disabled).toBe(false);
    await fireEvent.click(button());
    await settle();
    expect(button().disabled).toBe(true);
    answer(Response.json({ reason: 'not_linked' }, { status: 401 }));
    await settle();
    expect(button().disabled).toBe(false);
    expect(screen.getByRole('alert').textContent).toBe(
      'No passkey is linked yet. Link one from DeckStreak in Telegram.'
    );
    expect(fetched).toHaveBeenCalledOnce();
    expect(mocks.goto).not.toHaveBeenCalled();
  });
});
