// A single-page app: nothing is rendered on a server or prerendered. adapter-static writes one
// fallback page, and the client renders every route from it.
export const ssr = false;
export const prerender = false;
