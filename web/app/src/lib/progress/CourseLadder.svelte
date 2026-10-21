<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { CourseProgress } from './progress';

  // One course's Road to C2 (SPEC-077 R17): its flag and name, its band and mastery, its current
  // unit, and its ladder as six band cells, A1 to C2, in the order the route answers them. Each cell
  // states its band, its mastery and its mature cards as text beside a meter of the mastery, and
  // the current band's cell is marked. Every figure is the server's, rounded to the whole percent.
  let { course }: { course: CourseProgress } = $props();
</script>

<section aria-labelledby="course-{course.code}" class="mt-6">
  <h3 id="course-{course.code}" class="font-medium">
    <span aria-hidden="true">{course.flag}</span>
    {course.name}
  </h3>
  <p class="text-sm" data-course-summary>
    <span>{m.progress_summary({ band: course.currentBand, mastery: Math.round(course.masteryPct) })}</span>
    <span>
      {course.currentUnit === null ? m.progress_no_unit() : m.progress_unit({ unit: course.currentUnit })}
    </span>
  </p>
  <p class="text-sm" data-mastery-about="course">{m.progress_mastery_about()}</p>
  <ol class="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3" aria-label={m.progress_bands({ name: course.name })}>
    {#each course.bands as cell (cell.band)}
      <li
        data-band={cell.band}
        aria-current={cell.band === course.currentBand ? 'step' : undefined}
        class="rounded-md border p-2 aria-[current=step]:border-2 aria-[current=step]:border-primary"
      >
        <span class="font-semibold">{cell.band}</span>
        <span class="block text-sm">{m.progress_band_mastery({ pct: Math.round(cell.pct) })}</span>
        <meter
          class="block h-2 w-full"
          min="0"
          max="100"
          value={cell.pct}
          aria-label={m.progress_band_meter({ band: cell.band })}
        ></meter>
        <span class="block text-sm">{m.progress_band_mature({ mature: cell.mature, total: cell.total })}</span>
        {#if cell.achieved}
          <span class="block text-sm">{m.progress_band_achieved()}</span>
        {/if}
        {#if cell.band === course.currentBand}
          <span class="block text-sm font-medium">{m.progress_band_current()}</span>
        {/if}
      </li>
    {/each}
  </ol>
</section>
