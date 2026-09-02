<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

const notes = ref([])
const searchQuery = ref('')
const listError = ref('')
const deleteTargetId = ref(null)
let unlistenFocus = null

const ctxMenu = ref({ visible: false, x: 0, y: 0, note: null })

// 置顶区域拖拽排序状态
const pinnedSectionEl = ref(null)
const dragState = ref(null) // { fromIdx, startX, startY, isDragging, insertIdx, canDrag }
const pinInsertVisible = ref(false)
const pinInsertTop = ref(0)

const filteredNotes = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return notes.value
  return notes.value.filter(n =>
    n.title.toLowerCase().includes(q) || n.preview.toLowerCase().includes(q)
  )
})

const pinnedNotes = computed(() => filteredNotes.value.filter(n => n.pinned))
const unpinnedNotes = computed(() => filteredNotes.value.filter(n => !n.pinned))

async function loadNotes() {
  try {
    notes.value = await invoke('list_notes')
    listError.value = ''
  } catch (e) {
    console.error('load notes error', e)
    listError.value = '便签列表读取失败'
  }
}

async function newNote() {
  try {
    const id = await invoke('create_note')
    await invoke('open_note_window', { id })
    // create_note 已同步写盘，这里直接等待 list_notes 返回即可，无需固定延时猜测
    await loadNotes()
  } catch (e) {
    console.error('create note error', e)
    listError.value = '新建便签失败'
  }
}

async function openNote(id) {
  await invoke('open_note_window', { id })
}

async function deleteNote(id, e) {
  e.stopPropagation()
  deleteTargetId.value = id
}

function cancelDelete() {
  deleteTargetId.value = null
}

async function confirmDelete() {
  const id = deleteTargetId.value
  deleteTargetId.value = null
  if (!id) return
  await invoke('delete_note', { id })
  await loadNotes()
}

function formatDate(str) {
  if (!str) return ''
  return str.substring(0, 10)
}

function showCtxMenu(e, note) {
  e.preventDefault()
  e.stopPropagation()
  ctxMenu.value = { visible: true, x: e.clientX, y: e.clientY, note }
}

function hideCtxMenu() {
  ctxMenu.value = { ...ctxMenu.value, visible: false }
}

async function togglePin() {
  const note = ctxMenu.value.note
  hideCtxMenu()
  if (!note) return
  await invoke('toggle_pin', { id: note.id })
  await loadNotes()
}

function onDocClick() {
  if (ctxMenu.value.visible) hideCtxMenu()
}

function onDocKeydown(e) {
  if (e.key === 'Escape' && ctxMenu.value.visible) hideCtxMenu()
}

// --- 置顶区域拖拽排序 ---

function getPinnedCards() {
  if (!pinnedSectionEl.value) return []
  return Array.from(pinnedSectionEl.value.querySelectorAll('.note-card-pinned'))
}

function computeLineYs() {
  const cards = getPinnedCards()
  if (!pinnedSectionEl.value || cards.length === 0) return []
  const sectionRect = pinnedSectionEl.value.getBoundingClientRect()
  const ys = []
  cards.forEach((card, i) => {
    const r = card.getBoundingClientRect()
    if (i === 0) ys.push(r.top - sectionRect.top)
    ys.push(r.bottom - sectionRect.top)
  })
  return ys
}

function onPinnedMousedown(e, idx) {
  if (e.button !== 0) return
  const isSearchActive = !!searchQuery.value.trim()
  const canDrag = !isSearchActive && pinnedNotes.value.length > 1
  dragState.value = {
    fromIdx: idx,
    startX: e.clientX,
    startY: e.clientY,
    isDragging: false,
    insertIdx: idx,
    canDrag,
  }
  document.addEventListener('mousemove', onDragMousemove)
  document.addEventListener('mouseup', onDragMouseup)
}

function onDragMousemove(e) {
  if (!dragState.value) return
  const ds = dragState.value
  if (!ds.isDragging) {
    const dx = e.clientX - ds.startX
    const dy = e.clientY - ds.startY
    if (!ds.canDrag || (Math.abs(dx) <= 4 && Math.abs(dy) <= 4)) return
    ds.isDragging = true
    document.body.style.cursor = 'grabbing'
  }
  if (!pinnedSectionEl.value) return
  const sectionRect = pinnedSectionEl.value.getBoundingClientRect()
  const ys = computeLineYs()
  if (ys.length === 0) return
  const mouseRelY = e.clientY - sectionRect.top
  let bestIdx = 0
  let bestDist = Infinity
  ys.forEach((y, i) => {
    const dist = Math.abs(mouseRelY - y)
    if (dist < bestDist) {
      bestDist = dist
      bestIdx = i
    }
  })
  ds.insertIdx = bestIdx
  pinInsertTop.value = ys[bestIdx]
  pinInsertVisible.value = true
}

async function onDragMouseup() {
  document.removeEventListener('mousemove', onDragMousemove)
  document.removeEventListener('mouseup', onDragMouseup)
  document.body.style.cursor = ''
  pinInsertVisible.value = false
  if (!dragState.value) return
  const { fromIdx, isDragging, insertIdx } = dragState.value
  const noteToOpen = pinnedNotes.value[fromIdx]
  dragState.value = null
  if (!isDragging) {
    if (noteToOpen) openNote(noteToOpen.id)
    return
  }
  // 计算新顺序
  const newOrder = pinnedNotes.value.map(n => n.id)
  const [moved] = newOrder.splice(fromIdx, 1)
  const targetIdx = insertIdx > fromIdx ? insertIdx - 1 : insertIdx
  newOrder.splice(targetIdx, 0, moved)
  await invoke('reorder_pins', { ids: newOrder })
  await loadNotes()
}

onMounted(async () => {
  await loadNotes()
  unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) loadNotes()
  })
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onDocKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onDocKeydown)
  document.removeEventListener('mousemove', onDragMousemove)
  document.removeEventListener('mouseup', onDragMouseup)
  unlistenFocus?.()
})
</script>

<template>
  <div class="list-view">
    <div class="list-header">
      <span class="list-title">乔巴便笺</span>
      <button class="btn-new" @click="newNote">+ 新建</button>
    </div>

    <div class="search-wrap">
      <input
        v-model="searchQuery"
        class="search-input"
        type="text"
        placeholder="搜索便签..."
      />
    </div>

    <div class="notes-list">
      <div v-if="listError" class="list-error">{{ listError }}</div>
      <!-- 置顶区域（支持拖拽排序） -->
      <div v-if="pinnedNotes.length > 0" class="pinned-section" ref="pinnedSectionEl">
        <div
          v-for="(note, idx) in pinnedNotes"
          :key="note.id"
          class="note-card note-card-pinned"
          :class="{ 'note-card-dragging': dragState?.isDragging && dragState?.fromIdx === idx }"
          @mousedown="onPinnedMousedown($event, idx)"
          @contextmenu.prevent="showCtxMenu($event, note)"
        >
          <div class="note-card-top">
            <span class="pin-icon" title="已置顶，可拖拽排序">📌</span>
            <span class="note-card-title">{{ note.title || '无标题' }}</span>
            <button class="btn-card-del" @mousedown.stop @click="deleteNote(note.id, $event)" title="删除">✕</button>
          </div>
          <div class="note-card-preview">{{ note.preview }}</div>
          <div class="note-card-date">{{ formatDate(note.modified) }}</div>
        </div>
        <!-- 拖拽插入指示线 -->
        <div
          v-show="pinInsertVisible"
          class="pin-insert-line"
          :style="{ top: pinInsertTop + 'px' }"
        ></div>
      </div>

      <!-- 未置顶便签 -->
      <div
        v-for="note in unpinnedNotes"
        :key="note.id"
        class="note-card"
        :class="{ 'note-card-error': note.error }"
        @click="!note.error && openNote(note.id)"
        @contextmenu.prevent="showCtxMenu($event, note)"
      >
        <div class="note-card-top">
          <span class="note-card-title">{{ note.error ? '⚠ 读取失败' : (note.title || '无标题') }}</span>
          <button class="btn-card-del" @click="deleteNote(note.id, $event)" title="删除">✕</button>
        </div>
        <div class="note-card-preview">{{ note.error ? (note.id + ' · 文件损坏或编码异常') : note.preview }}</div>
        <div class="note-card-date">{{ note.error ? '' : formatDate(note.modified) }}</div>
      </div>

      <div v-if="filteredNotes.length === 0" class="empty-tip">
        {{ searchQuery ? '没有匹配的便签' : '暂无便签，点击「+ 新建」创建第一条' }}
      </div>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="ctxMenu.visible"
      class="ctx-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      @click.stop
    >
      <button class="ctx-menu-item" @click="togglePin">
        {{ ctxMenu.note?.pinned ? '取消置顶' : '📌 置顶' }}
      </button>
    </div>

    <!-- 删除确认弹窗 -->
    <div v-if="deleteTargetId" class="modal-mask" @click.self="cancelDelete">
      <div class="modal-card">
        <div class="modal-text">确定删除这条便签？</div>
        <div class="modal-actions">
          <button class="modal-btn cancel" @click="cancelDelete">取消</button>
          <button class="modal-btn danger" @click="confirmDelete">删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.list-error {
  padding: 10px 14px;
  margin-bottom: 8px;
  color: #b91c1c;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 6px;
  font-size: 13px;
}

.pinned-section {
  position: relative;
}

.note-card-pinned {
  cursor: grab;
  user-select: none;
}

.note-card-pinned .btn-card-del {
  cursor: pointer;
}

.note-card-dragging {
  opacity: 0.4;
}

.pin-insert-line {
  position: absolute;
  left: 8px;
  right: 8px;
  height: 4px;
  border-radius: 2px;
  background: #2563eb;
  box-shadow: 0 0 0 1.5px #fff, 0 0 0 3px #2563eb;
  pointer-events: none;
  z-index: 20;
  transform: translateY(-50%);
}

.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.32);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}
.modal-card {
  width: 260px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.2);
  padding: 20px 18px 16px;
}
.modal-text {
  font-size: 14px;
  color: #2c2c2c;
  text-align: center;
  margin-bottom: 16px;
}
.modal-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}
.modal-btn {
  flex: 1;
  padding: 7px 0;
  border-radius: 6px;
  border: 1px solid transparent;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
}
.modal-btn.cancel {
  background: #f5f5f5;
  color: #555;
  border-color: #ddd;
}
.modal-btn.cancel:hover { background: #ececec; }
.modal-btn.danger {
  background: #e53e3e;
  color: #fff;
}
.modal-btn.danger:hover { background: #c53030; }
</style>
