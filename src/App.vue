<script setup>
import { ref, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { check } from '@tauri-apps/plugin-updater'
import ListView from './components/ListView.vue'
import NoteWindow from './components/NoteWindow.vue'
import UpdateDialog from './components/UpdateDialog.vue'

const noteId = ref(null)
const isDev = import.meta.env.DEV
const pendingUpdate = ref(null)

onMounted(async () => {
  const label = getCurrentWindow().label
  if (label.startsWith('note-')) {
    noteId.value = label.replace('note-', '')
  } else {
    try {
      const update = await check()
      if (update?.available) {
        pendingUpdate.value = update
      }
    } catch (e) {
      console.warn('[updater] check failed:', e)
    }
  }
})
</script>

<template>
  <div v-if="isDev" class="dev-banner">DEV</div>
  <NoteWindow v-if="noteId" :note-id="noteId" />
  <ListView v-else />
  <UpdateDialog
    v-if="pendingUpdate"
    :update="pendingUpdate"
    @close="pendingUpdate = null"
  />
</template>
