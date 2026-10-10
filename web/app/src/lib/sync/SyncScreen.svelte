<script lang="ts">
  // SPEC-377 R8, R10, R11; ADR-388 D5, D11, D13. The sync screen: a sign-in that posts the sync user
  // and password to the Worker once and keeps neither, the store's status word in the owner's words,
  // the sign-out, the device's unsynced reviews as the Worker counts them (offline too), and the
  // choice when a sync answers that only a full sync can go on. A browser that lost its copy of the
  // collection is told so, and its choice names the download as the restore. Every value shown is
  // a reply of the Worker's (D5). The browser's backups are listed at the start and once each choice has
  // ended, never while one is held (SPEC-377 R15 to R17), and the storage status is the page's own
  // `persisted()` answer, since a Worker cannot ask (R18).
  import { onMount } from 'svelte';
  import type { BackupsClient } from '$lib/engine/backups';
  import type { BackupsListed, Required, StatusWord } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import BackupList from './BackupList.svelte';
  import ChoiceScreen from './ChoiceScreen.svelte';
  import { signOut } from './sign-out';
  import { choosing, signsIn, statusText, type ChoiceClient, type SyncClient } from './status';

  let {
    client,
    lost,
    fetch = globalThis.fetch,
    storage = globalThis.navigator?.storage
  }: {
    client: () => Promise<SyncClient & BackupsClient>;
    lost: () => boolean;
    fetch?: typeof globalThis.fetch;
    storage?: Pick<StorageManager, 'persisted'>;
  } = $props();

  /** The engine the screen asks: the sync and the backups. */
  type Engine = SyncClient & BackupsClient;
  /** What the browser answered about keeping this site's storage. */
  type Kept = 'persisted' | 'not_persisted' | 'unknown';

  /** The store's word as the Worker last answered it; null until it has. */
  let word = $state<StatusWord | null>(null);
  /** The device's unsynced reviews as the Worker last counted them; null until it has. */
  let pending = $state<number | null>(null);
  /** What the last sync answered the collections need; null before a sync answered. */
  let required = $state<Required | null>(null);
  /** The choice's client while the choice is open; null when no choice is open. */
  let chooser = $state.raw<ChoiceClient | null>(null);
  /** Whether the session's open found no copy of the collection in the browser's storage. */
  let gone = $state(false);
  let written = $state(false);
  let working = $state(false);
  let failed = $state(false);
  /** The browser's backups as the Worker last listed them; null until it has. */
  let listed = $state.raw<BackupsListed | null>(null);
  /** The browser's answer on keeping this site's storage; null until it has answered. */
  let kept = $state<Kept | null>(null);

  /** Runs one of the owner's acts against the engine, one at a time; a failure says the engine
   * stopped. */
  async function act(task: (sync: Engine) => Promise<void>): Promise<void> {
    working = true;
    failed = false;
    try {
      await task(await client());
    } catch {
      failed = true;
    } finally {
      working = false;
    }
  }

  /** Reads the device's unsynced reviews from the Worker. */
  async function count(sync: SyncClient): Promise<void> {
    pending = (await sync.unsynced()).reviews;
  }

  /** Reads the browser's backups from the Worker, which runs its retention first. */
  async function list(sync: Engine): Promise<void> {
    listed = await sync.backups();
  }

  /** Asks the browser whether it keeps this site's storage. A browser that cannot answer cannot
   * tell, and neither can a page with no storage to ask: asking it throws, and reads as unknown. */
  async function keeps(): Promise<void> {
    try {
      kept = (await storage!.persisted()) ? 'persisted' : 'not_persisted';
    } catch {
      kept = 'unknown';
    }
  }

  /** The storage status in the owner's words, one for each answer. */
  function keptText(answer: Kept): string {
    switch (answer) {
      case 'persisted':
        return m.sync_storage_persisted();
      case 'not_persisted':
        return m.sync_storage_not_persisted();
      case 'unknown':
        return m.sync_storage_unknown();
    }
  }

  /** Posts the sync user and password once. The fields are emptied before the post, and the
   * screen keeps neither value. */
  function signIn(event: SubmitEvent & { currentTarget: EventTarget & HTMLFormElement }): void {
    event.preventDefault();
    const fields = new FormData(event.currentTarget);
    event.currentTarget.reset();
    void act(async (sync) => {
      word = await sync.syncLogin(String(fields.get('user')), String(fields.get('password')));
    });
  }

  function syncNow(): void {
    written = false;
    void act(async (sync) => {
      const answer = await sync.sync();
      word = answer.status;
      required = answer.required;
      chooser = choosing(answer.required) ? sync : null;
      await count(sync);
    });
  }

  /** Forgets the sync key, then ends the web session (SPEC-363 R14). */
  function leave(): void {
    void act(async (sync) => {
      await signOut(sync, fetch);
      word = await sync.credentialStatus();
    });
  }

  /** The choice ended: a write leaves the browser holding the server's copy or the server holding
   * this device's, and the unsynced count is read again; a cancel closes the choice. Either way the
   * backups are listed again, now that no choice is held. */
  function chosen(outcome: 'written' | 'cancelled'): void {
    chooser = null;
    if (outcome === 'cancelled') {
      void act(list);
      return;
    }
    written = true;
    gone = false;
    required = null;
    void act(async (sync) => {
      await count(sync);
      await list(sync);
    });
  }

  onMount(() => {
    void keeps();
    void act(async (sync) => {
      gone = lost();
      word = await sync.credentialStatus();
      await count(sync);
      await list(sync);
    });
  });
</script>

<main class="mx-auto flex max-w-prose flex-col gap-4 px-6 py-12">
  <h1 class="text-2xl font-semibold tracking-tight">{m.sync_title()}</h1>
  <p role="status" class="min-h-6">
    {#if word !== null}{statusText(word)}{/if}
  </p>
  {#if gone}
    <p>{m.sync_lost()}</p>
  {/if}
  {#if failed}
    <p>{m.study_refused_engine()}</p>
  {/if}
  {#if word !== null}
    {#if signsIn(word)}
      <form class="flex flex-col gap-3" onsubmit={signIn}>
        <label for="sync-user" class="font-medium">{m.sync_user()}</label>
        <input
          id="sync-user"
          name="user"
          type="text"
          autocomplete="username"
          autocapitalize="none"
          spellcheck="false"
          required
          class="min-h-11 rounded-md border bg-background px-3"
        />
        <label for="sync-password" class="font-medium">{m.sync_password()}</label>
        <input
          id="sync-password"
          name="password"
          type="password"
          autocomplete="current-password"
          required
          class="min-h-11 rounded-md border bg-background px-3"
        />
        <button
          type="submit"
          class="min-h-11 self-start rounded-md bg-foreground px-4 font-medium text-background disabled:opacity-60"
          disabled={working}>{m.sync_sign_in()}</button
        >
      </form>
    {:else}
      <div class="flex flex-wrap gap-3">
        <button
          type="button"
          class="min-h-11 rounded-md bg-foreground px-4 font-medium text-background disabled:opacity-60"
          disabled={working || chooser !== null}
          onclick={syncNow}>{m.sync_now()}</button
        >
        <button
          type="button"
          class="min-h-11 rounded-md border px-4 disabled:opacity-60"
          disabled={working || chooser !== null}
          onclick={leave}>{m.sync_sign_out()}</button
        >
      </div>
    {/if}
  {/if}
  {#if working}
    <p>{m.sync_working()}</p>
  {/if}
  {#if pending !== null}
    <p>{m.sync_unsynced({ count: pending })}</p>
  {/if}
  {#if required === 'no-changes'}
    <p>{m.sync_in_step()}</p>
  {/if}
  {#if choosing(required)}
    <p>{m.sync_full_required()}</p>
  {/if}
  {#if chooser !== null}
    <ChoiceScreen client={chooser} lost={gone} ondone={chosen} />
  {/if}
  {#if written}
    <p>{m.sync_choice_written()}</p>
  {/if}
  {#if listed !== null}
    <BackupList {listed} {client} />
  {/if}
  {#if kept !== null}
    <div data-storage class="flex flex-col gap-1 text-sm">
      <span>{keptText(kept)}</span>
      <span>{m.sync_storage_evicted()}</span>
    </div>
  {/if}
</main>
