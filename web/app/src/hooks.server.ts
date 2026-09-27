import type { Handle } from '@sveltejs/kit';
import { getTextDirection } from '$lib/paraglide/runtime';
import { paraglideMiddleware } from '$lib/paraglide/server';

// The app has no server at runtime. This hook runs once, at build, when SvelteKit renders the
// fallback page, and fills app.html's lang and dir placeholders for the base locale;
// hooks.client.ts sets them for the visitor's locale when the app starts.
const handleParaglide: Handle = ({ event, resolve }) =>
  paraglideMiddleware(event.request, ({ request, locale }) => {
    event.request = request;

    return resolve(event, {
      transformPageChunk: ({ html }) =>
        html
          .replace('%paraglide.lang%', locale)
          .replace('%paraglide.dir%', getTextDirection(locale))
    });
  });

export const handle: Handle = handleParaglide;
