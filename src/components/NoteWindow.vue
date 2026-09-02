<script setup>
import { ref, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import NoteEditor from './NoteEditor.vue'

const appWindow = getCurrentWindow()
const props = defineProps({ noteId: String })

const isFloating = ref(false)
const initialContent = ref('')
const isLoaded = ref(false)
const noteTitle = ref('')
const isTitleEditing = ref(false)
const titleInputRef = ref(null)
let titleBeforeEdit = ''
const loadError = ref('')
const actionError = ref('')

let saveTimer = null
let latestMd = ''
let unlistenClose = null

async function loadNote() {
  try {
    const data = await invoke('read_note', { id: props.noteId })
    initialContent.value = data.content
    isFloating.value = data.meta.floating
    noteTitle.value = data.meta.title
    latestMd = data.content
    isLoaded.value = true
  } catch (e) {
    console.error('load note error', e)
    loadError.value = '该便签读取失败'
  }
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
      floating: isFloating.value,
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

async function toggleFloating() {
  isFloating.value = !isFloating.value
  await invoke('set_always_on_top', { value: isFloating.value })
  saveNote()
}

async function newNote() {
  try {
    const id = await invoke('create_note')
    await invoke('open_note_window', { id })
  } catch (e) {
    console.error('create note error', e)
    actionError.value = '新建便签失败'
  }
}

function minimize() {
  appWindow.minimize()
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

onMounted(async () => {
  await loadNote()
  // 任何关闭路径（含整体退出时窗口被关闭）都先强制落盘，再放行关闭
  unlistenClose = await appWindow.onCloseRequested(async () => {
    clearTimeout(saveTimer)
    await saveNote()
  })
})

onBeforeUnmount(() => {
  clearTimeout(saveTimer)
  unlistenClose?.()
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
          <span class="title-text">{{ noteTitle || '无标题' }}</span>
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
          :class="{ floating: isFloating }"
          @click="toggleFloating"
          :title="isFloating ? '取消置顶' : '置顶'"
        >📌</button>
        <button class="btn-icon" @click="newNote" title="新建便签">＋</button>
        <button class="btn-icon" @click="minimize" title="最小化">—</button>
        <button class="btn-icon btn-close" @click="closeWindow" title="关闭">✕</button>
      </div>
    </div>

    <div v-if="actionError" class="note-action-error">{{ actionError }}</div>

    <div v-if="loadError" class="note-error">{{ loadError }}</div>
    <NoteEditor
      v-else-if="isLoaded"
      :initial-content="initialContent"
      @change="onContentChange"
    />
  </div>
</template>

<style scoped>
.note-action-error {
  padding: 8px 14px;
  color: #b91c1c;
  background: #fef2f2;
  border-bottom: 1px solid #fecaca;
  font-size: 13px;
}
.note-error {
  padding: 24px;
  text-align: center;
  color: #b91c1c;
  font-size: 14px;
}
</style>
