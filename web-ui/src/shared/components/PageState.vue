<template>
  <section class="page-state">
    <header class="page-state__header">
      <h1>{{ title }}</h1>
      <p v-if="description">{{ description }}</p>
    </header>

    <div class="page-state__body">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<style scoped>
.page-state {
  width: 100%;
  display: grid;
  gap: 16px;
}

.page-state__header {
  display: grid;
  gap: 6px;
}

.page-state__header h1,
.page-state__header p {
  margin: 0;
}

.page-state__header h1 {
  color: #111827;
  font-size: 22px;
  line-height: 1.25;
}

.page-state__header p {
  max-width: 760px;
  color: #667085;
  font-size: 14px;
  line-height: 1.6;
}

.page-state__body {
  display: grid;
  gap: 14px;
}

:global(.dark) .page-state__header h1 {
  color: #f8fafc;
}

:global(.dark) .page-state__header p {
  color: #9ca3af;
}
</style>
