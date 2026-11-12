### Changed

- The Mini App moves to SvelteKit 3.1, with Svelte 5.57, Vite 8.3 and adapter-static 4. SvelteKit 3
  refuses `svelte.config.js`, so SvelteKit's options move to `web/app/sveltekit.config.js`, which
  `vite.config.ts` passes to SvelteKit's Vite plugin, and the library's imports move from `$lib` to
  `#lib`. The workspace moves to pnpm 12, Vitest 5, Mermaid 12 and StrykerJS 10.1.

### Security

- The web advisory audit clears 19 of its 20 advisories. SvelteKit 3 clears GHSA-pfhg-q3qv-9q6r and
  GHSA-27mw-82g8-phjj. The new lock moves `dompurify` to 3.4.17 (GHSA-cg2g-xh64-v4q7,
  GHSA-mrv2-3pqr-j5jr and GHSA-c6gr-qvv4-5v3f), `zod` to 4.6.7 (GHSA-mrhx-c7xc-m3w5), `undici` to
  8.12.3 (GHSA-4qv8-2p9j-2685, GHSA-f72v-6w45-q472, GHSA-xjj6-f33q-929c, GHSA-4w7w-5xrc-9h26,
  GHSA-2m5v-pch6-74hx, GHSA-qrxm-fqhp-jwvm, GHSA-9fm3-97px-rjf6 and GHSA-55wc-3pg8-jfw6) and
  `fast-uri` to 3.1.9 (GHSA-w74x-v6f9-j3xg, GHSA-xhq7-xmqj-769h, GHSA-wjr9-j795-jxcr and
  GHSA-333x-w4r6-6crp), and the `qs` override's floor rises to 6.16.1 (GHSA-6pq8-jhm2-2vcw).
  `deepmerge` 4.3.1 (GHSA-gcqx-w48m-w2h4, low) has no fixed release and stays.
