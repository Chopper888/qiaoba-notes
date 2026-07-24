<script setup>
import { ref, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ListView from './components/ListView.vue'
import NoteWindow from './components/NoteWindow.vue'

const noteId = ref(null)
const isDev = import.meta.env.DEV

onMounted(() => {
  const label = getCurrentWindow().label
  if (label.startsWith('note-')) {
    noteId.value = label.replace('note-', '')
  }
})
</script>

<template>
  <div v-if="isDev" class="dev-banner">DEV</div>
  <NoteWindow v-if="noteId" :note-id="noteId" />
  <ListView v-else />
</template>
