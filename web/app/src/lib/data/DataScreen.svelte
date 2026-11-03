<script lang="ts">
  // SPEC-516 R1, R2, R4, R6 to R8; ADR-534 D1, D4 to D6. The data screen: a JSON copy of the owner's
  // data from the one export route, saved as a file and never read by the page; back up now, which
  // the Worker's engine makes through the core, then the list again, which runs the core's retention
  // first; the backups this browser keeps, by the sync screen's own list and export; and the
  // server's backup age as the snapshot route answers it, unknown said as unknown. Inside Telegram
  // on a phone, where a download may not save, it names the bot's `/export` beside the download.
  import { onMount } from 'svelte';
  import type { BackupsListed } from '#lib/engine/protocol.js';
  import { m } from '#lib/paraglide/messages.js';
  import { getLocale } from '#lib/paraglide/runtime.js';
  import BackupList from '#lib/sync/BackupList.svelte';
  import { snapshotAge } from '#lib/sync/status.js';
  import {
    EXPORT_FILE,
    onPhone,
    saveFile,
    serverAge,
    type DataApi,
    type DataClient,
    type ServerAge
  } from './data';

  let {
    client,
    data,
    platform,
    save = saveFile
  }: {
    client: () => Promise<DataClient>;
    data: DataApi;
    platform: string;
    save?: (blob: Blob, name: string) => void;
  } = $props();

  /** How the last JSON copy ended; null before the first. */
  let copied = $state<'saved' | 'failed' | 'reopen' | null>(null);
  /** Whether a JSON copy is being fetched. */
  let copying = $state(false);
  /** The browser's backups as the Worker last listed them; null until it has. */
  let listed = $state.raw<BackupsListed | null>(null);
  /** Whether a back up now runs. */
  let working = $state(false);
  /** Whether the last back up now made a backup. */
  let made = $state(false);
  /** Whether the engine stopped during the list or a back up now. */
  let failed = $state(false);
  /** The server's backup as the screen says it; null until the route answered. */
  let server = $state<ServerAge | null>(null);

  /** Fetches the JSON copy and saves it as the export's file; a refusal or no answer saves nothing. */
  async function download(): Promise<void> {
    if (copying) return;
    copying = true;
    copied = null;
    try {
      const answer = await data.dataExport();
      if (answer.kind === 'ok') {
        save(answer.value, EXPORT_FILE);
        copied = 'saved';
      } else {
        copied = answer.kind === 'reopen' ? 'reopen' : 'failed';
      }
    } catch {
      copied = 'failed';
    } finally {
      copying = false;
    }
  }

  /** Reads the browser's backups from the Worker, which runs the core's retention first. */
  async function list(engine: DataClient): Promise<void> {
    listed = await engine.backups();
  }

  /** Backs this device up now, once at a time, then lists the backups again. */
  async function backUp(): Promise<void> {
    if (working) return;
    working = true;
    made = false;
    failed = false;
    try {
      const engine = await client();
      await engine.backUpNow();
      made = true;
      await list(engine);
    } catch {
      failed = true;
    } finally {
      working = false;
    }
  }

  /** The server's backup age in the owner's words: unknown in words, never as a number. */
  function serverText(age: ServerAge): string {
    switch (age.kind) {
      case 'found':
        return m.data_server_age({ age: snapshotAge(age.age, getLocale()) });
      case 'none':
        return m.data_server_none();
      case 'unknown':
        return m.data_server_unknown();
    }
  }

  onMount(() => {
    void data.serverBackup().then(
      (answer) => (server = serverAge(answer)),
      () => (server = { kind: 'unknown' })
    );
    void client()
      .then(list)
      .catch(() => (failed = true));
  });
</script>

<main class="mx-auto flex max-w-prose flex-col gap-6 px-6 py-12">
  <h1 class="text-2xl font-semibold tracking-tight">{m.data_title()}</h1>

  <section class="flex flex-col gap-3" aria-labelledby="data-json-title">
    <h2 id="data-json-title" class="text-lg font-medium">{m.data_json_title()}</h2>
    <p>{m.data_json_about()}</p>
    <button
      type="button"
      class="min-h-11 self-start rounded-md bg-foreground px-4 font-medium text-background disabled:opacity-60"
      disabled={copying}
      onclick={download}>{m.data_json_download()}</button
    >
    {#if onPhone(platform)}
      <p>{m.data_telegram_phone()}</p>
    {/if}
    {#if copied === 'saved'}
      <p role="status">{m.data_json_saved()}</p>
    {:else if copied === 'failed'}
      <p role="status">{m.data_json_failed()}</p>
    {:else if copied === 'reopen'}
      <p role="status">{m.data_json_reopen()}</p>
    {/if}
  </section>

  <section class="flex flex-col gap-3" aria-labelledby="data-device-title">
    <h2 id="data-device-title" class="text-lg font-medium">{m.data_device_title()}</h2>
    <p>{m.data_device_about()}</p>
    <button
      type="button"
      class="min-h-11 self-start rounded-md border px-4 disabled:opacity-60"
      disabled={working}
      onclick={backUp}>{m.data_back_up_now()}</button
    >
    {#if working}
      <p role="status">{m.data_working()}</p>
    {:else if made}
      <p role="status">{m.data_backup_made()}</p>
    {/if}
    {#if failed}
      <div role="alert">{m.study_refused_engine()}</div>
    {/if}
    {#if listed !== null}
      <BackupList {listed} {client} />
    {/if}
  </section>

  <section class="flex flex-col gap-3" aria-labelledby="data-server-title">
    <h2 id="data-server-title" class="text-lg font-medium">{m.data_server_title()}</h2>
    {#if server !== null}
      <p data-server-age>{serverText(server)}</p>
    {/if}
  </section>
</main>
