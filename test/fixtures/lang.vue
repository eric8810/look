<!-- two-face language fixture (scenario L) -->
<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
</script>

<template>
  <button @click="count++">count is {{ count }}</button>
</template>

<style scoped>
button { color: red; }
</style>
