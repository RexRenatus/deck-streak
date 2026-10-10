/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  assertionJSON,
  creationOptions,
  decode,
  encode,
  mint,
  redeem,
  register,
  registrationJSON,
  remove,
  requestOptions,
  signIn,
  type Wording
} from './passkeys';

// SPEC-385 R1 to R5, R8, R11, A6, A7, A11, A13, A14; ADR-399 D1. The ceremony client calls the
// browser's credential API through a codec of its own: every byte field travels as unpadded
// base64url, the authenticator's answer goes back in the JSON shape the server's own tests post,
// and every call is one same-origin JSON post. The server and the authenticator are stand-ins
// that record what they were sent, so a request the client should not make fails the test.
const HOST = location.hostname;

/** Bytes, as the authenticator hands them over. */
function bytes(...values: number[]): ArrayBuffer {
  return Uint8Array.from(values).buffer;
}

/** The bytes of a decoded field, as plain numbers. */
function numbers(field: unknown): number[] {
  return [...(field as Uint8Array)];
}

interface Sent {
  readonly url: string;
  readonly method: string;
  readonly credentials: RequestCredentials | undefined;
  readonly headers: Record<string, string>;
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
      sent.push({
        url,
        method,
        credentials: init.credentials,
        headers: Object.fromEntries(new Headers(init.headers).entries()),
        body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined
      });
      return answers[`${method} ${url}`]?.() ?? new Response(null, { status: 599 });
    })
  );
  return sent;
}

function lines(sent: readonly Sent[]): string[] {
  return sent.map((request) => `${request.method} ${request.url}`);
}

/** The registration options a start answers, every byte field in base64url. */
const CREATION = {
  publicKey: {
    rp: { id: HOST, name: 'DeckStreak' },
    user: { id: 'AA', name: 'owner', displayName: 'owner' },
    challenge: '----',
    pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
    excludeCredentials: [{ type: 'public-key', id: '____' }]
  }
};

/** The sign-in options a start answers, over the owner's one credential. */
const REQUEST = {
  publicKey: {
    rpId: HOST,
    challenge: '____',
    allowCredentials: [{ type: 'public-key', id: '----' }],
    userVerification: 'required'
  }
};

/** A new credential, as the authenticator answers a create. */
const ATTESTED = {
  id: '____',
  rawId: bytes(0xff, 0xff, 0xff),
  type: 'public-key',
  response: { attestationObject: bytes(0xfb, 0xef, 0xbe), clientDataJSON: bytes(0x00) }
};

/** An assertion, as the authenticator answers a get. */
const ASSERTED = {
  id: '____',
  rawId: bytes(0xff, 0xff, 0xff),
  type: 'public-key',
  response: {
    authenticatorData: bytes(0x00),
    clientDataJSON: bytes(0xfb, 0xef, 0xbe),
    signature: bytes(0xff, 0xff, 0xff),
    userHandle: bytes(0x00) as ArrayBuffer | null
  }
};

/** What the registration finish posts for ATTESTED: literal, never the codec's own output. */
const ATTESTED_JSON = {
  id: '____',
  rawId: '____',
  type: 'public-key',
  response: { attestationObject: '----', clientDataJSON: 'AA' },
  extensions: {}
};

/** What the sign-in finish posts for ASSERTED. */
const ASSERTED_JSON = {
  id: '____',
  rawId: '____',
  type: 'public-key',
  response: { authenticatorData: 'AA', clientDataJSON: '----', signature: '____', userHandle: 'AA' },
  extensions: {}
};

/** The browser's authenticator, answering each create and get, and recording both. */
function authenticator(signals: Record<string, unknown> = {}) {
  const create = vi.fn(async (options: CredentialCreationOptions) => {
    void options;
    return ATTESTED as unknown as Credential;
  });
  const get = vi.fn(async (options: CredentialRequestOptions) => {
    void options;
    return ASSERTED as unknown as Credential;
  });
  Object.defineProperty(navigator, 'credentials', { configurable: true, value: { create, get } });
  vi.stubGlobal('PublicKeyCredential', Object.assign(class {}, signals));
  return { create, get };
}

/** Every route a whole registration and a whole sign-in meet, each answering as it succeeds. */
const SUCCEEDS: Record<string, () => Response> = {
  'POST /api/link/redeem': () => new Response(null, { status: 204 }),
  'POST /api/passkeys/register/start': () => Response.json(CREATION),
  'POST /api/passkeys/register/finish': () => Response.json({ id: 3 }, { status: 201 }),
  'POST /api/passkeys/sign-in/start': () => Response.json(REQUEST),
  'POST /api/passkeys/sign-in/finish': () =>
    Response.json({ credential_ids: ['____'], user_handle: 'AA' }),
  'POST /api/link/code': () => Response.json({ code: 'c0de' }),
  'DELETE /api/identities/7': () => new Response(null, { status: 204 })
};

afterEach(() => {
  vi.unstubAllGlobals();
  delete (navigator as { credentials?: unknown }).credentials;
});

describe('the passkey ceremony client', () => {
  it('ceremony options and responses round-trip through base64url', () => {
    // the goldens: `----` is fb ef be, `____` is ff ff ff, `AA` is 00, with no padding
    expect(numbers(decode('----'))).toEqual([0xfb, 0xef, 0xbe]);
    expect(numbers(decode('____'))).toEqual([0xff, 0xff, 0xff]);
    expect(numbers(decode('AA'))).toEqual([0x00]);
    expect(encode(bytes(0xfb, 0xef, 0xbe))).toBe('----');
    expect(encode(bytes(0xff, 0xff, 0xff))).toBe('____');
    expect(encode(bytes(0x00))).toBe('AA');

    // the server's options reach the browser as bytes, every other member as it came
    const creation = creationOptions(CREATION, HOST) as unknown as Record<string, never>;
    expect(numbers(creation.challenge)).toEqual([0xfb, 0xef, 0xbe]);
    expect(numbers((creation.user as Record<string, unknown>).id)).toEqual([0x00]);
    expect((creation.user as Record<string, unknown>).name).toBe('owner');
    expect(numbers((creation.excludeCredentials as Record<string, unknown>[])[0].id)).toEqual([
      0xff, 0xff, 0xff
    ]);
    expect(creation.rp).toEqual({ id: HOST, name: 'DeckStreak' });
    expect(creation.pubKeyCredParams).toEqual([{ type: 'public-key', alg: -7 }]);
    const request = requestOptions(REQUEST, HOST) as unknown as Record<string, never>;
    expect(numbers(request.challenge)).toEqual([0xff, 0xff, 0xff]);
    expect(numbers((request.allowCredentials as Record<string, unknown>[])[0].id)).toEqual([
      0xfb, 0xef, 0xbe
    ]);
    expect(request.userVerification).toBe('required');

    // the authenticator's answers reach the server in its own shape
    expect(registrationJSON(ATTESTED as unknown as PublicKeyCredential)).toEqual(ATTESTED_JSON);
    expect(assertionJSON(ASSERTED as unknown as PublicKeyCredential)).toEqual(ASSERTED_JSON);
    const anonymous = { ...ASSERTED, response: { ...ASSERTED.response, userHandle: null } };
    expect(assertionJSON(anonymous as unknown as PublicKeyCredential)).toEqual({
      ...ASSERTED_JSON,
      response: { ...ASSERTED_JSON.response, userHandle: null }
    });
  });

  it('a sign-in signals the accepted credentials only where the browser offers it', async () => {
    server(SUCCEEDS);
    const signalled = vi.fn(async (options: unknown) => {
      void options;
    });
    authenticator({ signalAllAcceptedCredentials: signalled });

    expect(await signIn()).toEqual({ kind: 'ok', value: true });
    // the accepted credentials, with the request's relying party, once
    expect(signalled.mock.calls).toEqual([
      [{ rpId: HOST, userId: 'AA', allAcceptedCredentialIds: ['____'] }]
    ]);

    // a browser that offers no signal is asked nothing, and the sign-in still ends well
    authenticator();
    expect(await signIn()).toEqual({ kind: 'ok', value: true });
    expect(signalled).toHaveBeenCalledOnce();
  });

  it('options for another relying party never reach the authenticator', async () => {
    const elsewhere = [
      { publicKey: { ...CREATION.publicKey, rp: { id: 'another-party', name: 'DeckStreak' } } },
      { publicKey: { ...CREATION.publicKey, rp: { name: 'DeckStreak' } } },
      { publicKey: { ...CREATION.publicKey, rp: undefined } },
      {}
    ];
    const refusedRequests = [{ publicKey: { ...REQUEST.publicKey, rpId: 'another-party' } }, {}];
    const { create, get } = authenticator();
    for (const options of elsewhere) {
      const sent = server({ 'POST /api/passkeys/register/start': () => Response.json(options) });
      const outcome = await register();
      // the authenticator was asked nothing, and nothing followed the start
      expect(create).not.toHaveBeenCalled();
      expect(lines(sent)).toEqual(['POST /api/passkeys/register/start']);
      expect(outcome).toEqual({ kind: 'refused', wording: 'passkey_refused' });
    }
    for (const options of refusedRequests) {
      const sent = server({ 'POST /api/passkeys/sign-in/start': () => Response.json(options) });
      const outcome = await signIn();
      expect(get).not.toHaveBeenCalled();
      expect(lines(sent)).toEqual(['POST /api/passkeys/sign-in/start']);
      expect(outcome).toEqual({ kind: 'refused', wording: 'passkey_refused' });
    }
    expect([create.mock.calls.length, get.mock.calls.length]).toEqual([0, 0]);

    // the page's own host reaches it, once each
    server(SUCCEEDS);
    expect(await register()).toEqual({ kind: 'ok', value: true });
    expect(await signIn()).toEqual({ kind: 'ok', value: true });
    expect([create.mock.calls.length, get.mock.calls.length]).toEqual([1, 1]);
  });

  it('every ceremony refusal maps to its message', async () => {
    // SPEC-385 §2a, row by row; a code no row names meets the refusal wording, and an answer
    // that names no code, or a server that failed, the screen's unavailable wording
    const ANSWERS: readonly (readonly [number, string | null, Wording | null])[] = [
      [401, 'not_linked', 'signin_not_linked'],
      [401, 'challenge_invalid', 'passkey_start_again'],
      [401, 'challenge_expired', 'passkey_start_again'],
      [401, 'origin_mismatch', 'passkey_refused'],
      [401, 'uv_required', 'passkey_refused'],
      [401, 'passkey_invalid', 'passkey_refused'],
      [401, 'counter_regressed', 'passkey_refused'],
      [403, 'not_owner', 'passkey_refused'],
      [429, 'too_many_ceremonies', 'passkey_too_many'],
      [404, 'linking_off', 'passkey_off'],
      [403, 'cross_site_request', 'passkey_refused'],
      [403, 'not_json', 'passkey_refused'],
      [401, 'link_code_invalid', 'link_code_spent'],
      [401, 'link_code_expired', 'link_code_spent'],
      [401, 'no_session', 'link_code_spent'],
      [409, 'already_linked', 'link_already'],
      [401, 'reauth_required', 'methods_reopen'],
      [404, 'identity_unknown', null],
      [401, 'a_code_no_row_names', 'passkey_refused'],
      [500, null, 'server_unavailable'],
      [503, 'database_not_open', 'server_unavailable']
    ];
    // the authenticator's own refusals: the owner's cancel, and any other
    const THROWN: readonly (readonly [string, Wording])[] = [
      ['NotAllowedError', 'passkey_cancelled'],
      ['InvalidStateError', 'passkey_refused']
    ];
    const total = ANSWERS.length + THROWN.length;
    let judged = 0;
    for (const [status, reason, wording] of ANSWERS) {
      server({
        'POST /api/link/redeem': () =>
          reason === null ? new Response(null, { status }) : Response.json({ reason }, { status })
      });
      expect([status, reason, await redeem('c0de')]).toEqual([
        status,
        reason,
        { kind: 'refused', wording }
      ]);
      judged += 1;
    }
    for (const [name, wording] of THROWN) {
      server(SUCCEEDS);
      const { create } = authenticator();
      create.mockRejectedValueOnce(new DOMException('the authenticator refused', name));
      expect([name, await register()]).toEqual([name, { kind: 'refused', wording }]);
      judged += 1;
    }
    console.log(`examined ${judged} of ${total} refusals`);
    expect(judged).toBe(total);
  });

  it('every ceremony call is a same-origin json post', async () => {
    const stored = vi.spyOn(Storage.prototype, 'setItem');
    const sent = server(SUCCEEDS);
    authenticator();

    expect(await redeem('c0de')).toEqual({ kind: 'ok', value: true });
    expect(await register()).toEqual({ kind: 'ok', value: true });
    expect(await signIn()).toEqual({ kind: 'ok', value: true });
    expect(await mint()).toEqual({ kind: 'ok', value: 'c0de' });
    expect(await remove(7)).toEqual({ kind: 'ok', value: true });

    expect(lines(sent)).toEqual([
      'POST /api/link/redeem',
      'POST /api/passkeys/register/start',
      'POST /api/passkeys/register/finish',
      'POST /api/passkeys/sign-in/start',
      'POST /api/passkeys/sign-in/finish',
      'POST /api/link/code',
      'DELETE /api/identities/7'
    ]);
    // each to a relative path of this origin, as JSON, with this origin's cookies
    for (const request of sent) {
      expect([request.url, request.credentials, request.headers]).toEqual([
        request.url,
        'same-origin',
        { 'content-type': 'application/json' }
      ]);
      expect(request.url).toMatch(/^\/api\//);
    }
    // a start sends {}; a finish sends only the authenticator's response; the code travels in a body
    expect(sent.map((request) => request.body)).toEqual([
      { code: 'c0de' },
      {},
      ATTESTED_JSON,
      {},
      ASSERTED_JSON,
      {},
      {}
    ]);
    expect(stored).not.toHaveBeenCalled();
  });
});
