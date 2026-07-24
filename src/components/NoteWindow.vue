<script setup>
import { ref, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import NoteEditor from './NoteEditor.vue'

const appWindow = getCurrentWindow()
const props = defineProps({ noteId: String })

const isPinned = ref(false)
const initialContent = ref('')
const isLoaded = ref(false)
const noteTitle = ref('')
const isTitleEditing = ref(false)
const titleInputRef = ref(null)
let titleBeforeEdit = ''

let saveTimer = null
let latestMd = ''

async function loadNote() {
  const data = await invoke('read_note', { id: props.noteId })
  initialContent.value = data.content
  isPinned.value = data.meta.pinned
  noteTitle.value = data.meta.title
  latestMd = data.content
  isLoaded.value = true
}

async function saveNote() {
  if (!isLoaded.value) return
  try {
    const pos = await invoke('get_window_position')
    await invoke('save_note', {
      id: props.noteId,
      title: noteTitle.value,
      content: latestMd,
      x: pos.x,
      y: pos.y,
      width: pos.width,
      height: pos.height,
      pinned: isPinned.value,
    })
  } catch (e) {
    console.error('save error', e)
  }
}

function onContentChange(md) {
  latestMd = md
  clearTimeout(saveTimer)
  saveTimer = setTimeout(saveNote, 500)
}

async function togglePin() {
  isPinned.value = !isPinned.value
  await invoke('set_always_on_top', { value: isPinned.value })
  saveNote()
}

async function newNote() {
  const id = await invoke('create_note')
  await invoke('open_note_window', { id })
}

async function startDrag(e) {
  if (e.button === 0) {
    await appWindow.startDragging()
  }
}

async function startEditTitle() {
  titleBeforeEdit = noteTitle.value
  isTitleEditing.value = true
  await nextTick()
  titleInputRef.value?.select()
}

async function finishEditTitle() {
  if (!isTitleEditing.value) return
  isTitleEditing.value = false
  noteTitle.value = noteTitle.value.trim() || titleBeforeEdit
  await saveNote()
}

function cancelEditTitle() {
  noteTitle.value = titleBeforeEdit
  isTitleEditing.value = false
}

async function closeWindow() {
  clearTimeout(saveTimer)
  await saveNote()
  await invoke('close_window')
}

onMounted(loadNote)

onBeforeUnmount(() => {
  clearTimeout(saveTimer)
})
</script>

<template>
  <div class="note-window">
    <div class="note-titlebar">
      <!-- 左侧拖动把手 -->
      <div class="titlebar-drag" @mousedown="startDrag"></div>

      <!-- 标题区域：拖动 = 移动窗口；点 ✎ = 编辑标题 -->
      <div class="titlebar-title" @mousedown="!isTitleEditing && startDrag($event)">
        <template v-if="!isTitleEditing">
          <span class="title-text">{{ noteTitle || '便签' }}</span>
          <button class="btn-title-edit" @mousedown.stop @click.stop="startEditTitle" title="编辑标题">✎</button>
        </template>
        <input
          v-else
          ref="titleInputRef"
          class="title-input"
          v-model="noteTitle"
          @blur="finishEditTitle"
          @keydown.enter.prevent="finishEditTitle"
          @keydown.escape="cancelEditTitle"
          @mousedown.stop
          @click.stop
        />
      </div>

      <div class="titlebar-actions">
        <button
          class="btn-icon pin-btn"
          :class="{ pinned: isPinned }"
          @click="togglePin"
          :title="isPinned ? '取消置顶' : '置顶'"
        >📌</button>
        <button class="btn-icon" @click="newNote" title="新建便签">＋</button>
        <button class="btn-icon btn-close" @click="closeWindow" title="关闭">✕</button>
      </div>
    </div>

    <NoteEditor
      v-if="isLoaded"
      :initial-content="initialContent"
      @change="onContentChange"
    />
  </div>
</template>
