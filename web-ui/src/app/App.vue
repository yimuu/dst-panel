<template>
  <RouterView />
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<style scoped>
:deep(.app-shell) {
  min-height: 100vh;
  display: grid;
  place-content: center;
  gap: 0.75rem;
  padding: 2rem;
  text-align: center;
  color: #1f2937;
  background: #f8fafc;
}

:deep(h1),
:deep(p) {
  margin: 0;
}

:deep(h1) {
  font-size: 2rem;
  font-weight: 700;
}

:deep(p) {
  color: #4b5563;
}
</style>
