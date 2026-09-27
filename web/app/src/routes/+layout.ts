import { redirect } from '@sveltejs/kit';
import { routeFor } from '$lib/startapp';
import { telegram } from '$lib/telegram.svelte';
import type { LayoutLoad } from './$types';

// A single-page app: nothing is rendered on a server or prerendered. adapter-static writes one
// fallback page, and the client renders every route from it.
export const ssr = false;
export const prerender = false;

// The launch link's startapp token routes the first navigation only; after it, the owner's own
// navigation decides.
let routed = false;

/**
 * Opens the screen the launch link's startapp token names (SPEC-028 R3, R4; ADR-028): a listed
 * token opens its screen, and any other token opens Today. A launch with no token opens the path
 * it asked for.
 */
export const load: LayoutLoad = ({ url, untrack }) => {
  if (routed) return;
  routed = true;
  if (telegram.startParam === null) return;
  const route = routeFor(telegram.startParam);
  if (untrack(() => url.pathname) !== route) redirect(307, route);
};
