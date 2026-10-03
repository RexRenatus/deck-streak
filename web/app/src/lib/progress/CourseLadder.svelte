<script lang="ts">
  import type { CourseProgress } from './progress';

  let { course }: { course: CourseProgress } = $props();
</script>

<section>
  <h3>{course.name}</h3>
</section>
