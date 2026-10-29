import type { Handle } from '@sveltejs/kit/hooks';
import { getTextDirection } from '#lib/paraglide/runtime.js';
import { paraglideMiddleware } from '#lib/paraglide/server.js';

// The app has no server at runtime. This hook runs once, at build, when SvelteKit renders the
// fallback page, and fills app.html's lang and dir placeholders for the base locale;
// hooks.client.ts sets them for the visitor's locale when the app starts. SvelteKit 3's event is
// read-only, so the page resolves on a copy that carries the middleware's request.
const handleParaglide: Handle = ({ event, resolve }) =>
  paraglideMiddleware(event.request, ({ request, locale }) =>
    resolve(
      { ...event, request },
      {
        transformPageChunk: ({ html }) =>
          html
            .replace('%paraglide.lang%', locale)
            .replace('%paraglide.dir%', getTextDirection(locale))
      }
    )
  );

export const handle: Handle = handleParaglide;
