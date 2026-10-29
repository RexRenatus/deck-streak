import type { RequestEvent } from '@sveltejs/kit';
import type { ResolveOptions } from '@sveltejs/kit/hooks';
import { describe, expect, it, vi } from 'vitest';
import { handle } from './hooks.server';
import { reroute } from './hooks';

// SPEC-057 A24. The build renders the fallback page once, in the base locale, and the server hook
// fills app.html's language and direction placeholders; the reroute hook hands the router the path
// with the locale prefix taken off.
describe('the server hook', () => {
  it('fills the language and direction placeholders of the page with the base locale', async () => {
    const url = new URL('http://localhost/about');
    const event = { request: new Request(url), url } as unknown as RequestEvent;
    const original = event.request;
    const resolve = vi.fn(async (_event: RequestEvent, options?: ResolveOptions) => {
      const html = await options?.transformPageChunk?.({
        html: '<html lang="%paraglide.lang%" dir="%paraglide.dir%">',
        done: true
      });
      return new Response(html);
    });

    const response = await handle({ event, resolve });

    expect(await response.text()).toBe('<html lang="en" dir="ltr">');
    expect(resolve).toHaveBeenCalledTimes(1);
    // SvelteKit 3's event is read-only: the hook leaves it alone and resolves a copy of it that
    // carries the middleware's request
    const resolved = resolve.mock.calls[0][0];
    expect(event.request).toBe(original);
    expect(resolved.url).toBe(url);
    expect(resolved.request).toBeInstanceOf(Request);
    expect(resolved.request.url).toBe(original.url);
  });
});

describe('the reroute hook', () => {
  it('routes on the path with the locale prefix taken off', () => {
    const route = (path: string) => reroute({ url: new URL(`http://localhost${path}`), fetch });

    expect(route('/about')).toBe('/about');
    expect(route('/')).toBe('/');
    expect(route('/fr/about')).toBe('/about');
    expect(route('/ja')).toBe('/');
  });
});
