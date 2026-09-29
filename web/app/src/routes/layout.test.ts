import { afterEach, describe, expect, it, vi } from 'vitest';

// SPEC-057 A24; SPEC-028 R3, R4. The root layout's load routes the first navigation by the launch
// link's startapp token and no other. The module keeps its "already routed" flag at load, so each
// scenario imports a fresh copy over a wrapper that answers the token the scenario names.
async function load(startParam: string | null) {
  vi.resetModules();
  vi.doMock('$lib/telegram.svelte', () => ({ telegram: { startParam } }));
  return import('./+layout');
}

function navigate(module: Awaited<ReturnType<typeof load>>, path: string) {
  const untrack = vi.fn(<T>(read: () => T) => read());
  try {
    module.load({ url: new URL(`http://localhost${path}`), untrack } as never);
  } catch (thrown) {
    return { thrown, untrack };
  }
  return { thrown: undefined, untrack };
}

afterEach(() => {
  vi.doUnmock('$lib/telegram.svelte');
});

describe('the root layout', () => {
  it('renders in the browser only, and never at build', async () => {
    const module = await load(null);

    expect(module.ssr).toBe(false);
    expect(module.prerender).toBe(false);
  });

  it('opens the screen a listed token names, once, from another path', async () => {
    const module = await load('about');

    const first = navigate(module, '/');

    expect(first.thrown).toMatchObject({ status: 307, location: '/about' });
    expect(first.untrack).toHaveBeenCalledTimes(1);
    // the owner's own navigation decides after the first
    expect(navigate(module, '/').thrown).toBeUndefined();
  });

  it('does not redirect a launch that is already on the screen its token names', async () => {
    const module = await load('about');

    const outcome = navigate(module, '/about');

    expect(outcome.thrown).toBeUndefined();
    expect(outcome.untrack).toHaveBeenCalledTimes(1);
    expect(navigate(module, '/').thrown).toBeUndefined();
  });

  it('opens Today for a token that is not listed', async () => {
    const module = await load('unlisted');

    expect(navigate(module, '/about').thrown).toMatchObject({ status: 307, location: '/' });
  });

  it('opens the path a launch with no token asked for', async () => {
    const module = await load(null);

    const outcome = navigate(module, '/about');

    expect(outcome.thrown).toBeUndefined();
    expect(outcome.untrack).not.toHaveBeenCalled();
  });
});
