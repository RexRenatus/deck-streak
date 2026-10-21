<script lang="ts">
  // SPEC-377 R15 to R17; ADR-388 D3, D5, D14. The browser's backups, as the Worker listed them after
  // its retention: each by its kind and age, newest first, with an export that saves its bytes as a
  // file named by kind and age, and each backup retention removed named once.
  import type { BackupKind, BackupsListed } from '$lib/engine/protocol';
  import type { BackupsClient } from '$lib/engine/backups';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { snapshotAge } from './status';

  let { listed, client }: { listed: BackupsListed; client: () => Promise<BackupsClient> } = $props();

  /** Whether the last export failed: the engine stopped, and nothing was saved. */
  let failed = $state(false);

  /** How a backup reads on the list: whose collection it holds, and how long ago it was saved. */
  function described(kind: BackupKind, age: number): string {
    const ago = snapshotAge(age, getLocale());
    return kind === 'backup' ? m.sync_backup_device({ age: ago }) : m.sync_backup_server({ age: ago });
  }

  /** Saves one backup's bytes, as the Worker sent them, in a file named by its kind and age. The
   * anchor is only the browser's means of saving: it never joins the page, and its address is
   * revoked once the save has begun. */
  async function save(id: string, kind: BackupKind, age: number): Promise<void> {
    failed = false;
    try {
      const bytes = await (await client()).backupExport(id);
      const url = URL.createObjectURL(new Blob([bytes]));
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `deck-streak-${kind}-${age}s.anki2`;
      anchor.click();
      setTimeout(() => URL.revokeObjectURL(url), 0);
    } catch {
      failed = true;
    }
  }
</script>

<section class="flex flex-col gap-2">
  {#if listed.backups.length === 0}
    <div>{m.sync_backups_none()}</div>
  {:else}
    <table class="w-full text-left">
      <caption class="text-left font-medium">{m.sync_backups_title()}</caption>
      <tbody>
        {#each listed.backups as backup (backup.id)}
          <tr>
            <th id="backup-row-{backup.id}" scope="row" class="py-1 pr-3 font-normal">
              {described(backup.kind, backup.age)}
            </th>
            <td class="py-1">
              <button
                type="button"
                class="min-h-11 rounded-md border px-4"
                aria-describedby="backup-row-{backup.id}"
                onclick={() => save(backup.id, backup.kind, backup.age)}>{m.sync_backup_export()}</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if listed.removed.length > 0}
    <ul class="list-disc pl-6">
      {#each listed.removed as removal}
        <li>{m.sync_backup_removed({ backup: described(removal.kind, removal.age) })}</li>
      {/each}
    </ul>
  {/if}
  {#if failed}
    <div role="alert">{m.study_refused_engine()}</div>
  {/if}
</section>
