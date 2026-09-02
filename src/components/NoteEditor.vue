<script setup>
import { onMounted, onBeforeUnmount, nextTick, ref } from 'vue'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Underline from '@tiptap/extension-underline'
import Image from '@tiptap/extension-image'
import Link from '@tiptap/extension-link'
import Placeholder from '@tiptap/extension-placeholder'
import { Mark, mergeAttributes, Extension } from '@tiptap/core'
import { TextSelection, AllSelection } from '@tiptap/pm/state'
import TurndownService from 'turndown'
import { marked } from 'marked'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ContinuousOrderedList } from '../editor/continuousOrderedList.js'
import { TabIndentList } from '../editor/tabIndentList.js'

const props = defineProps({ initialContent: String })
const emit = defineEmits(['change'])

// ── 自定义 Mark：任务状态标记 ────────────────────────────────────────────────
const StatusMark = Mark.create({
  name: 'status',
  addAttributes() {
    return {
      type: { default: null, parseHTML: el => el.getAttribute('data-status') },
    }
  },
  parseHTML() {
    return [{ tag: 'span[data-status]' }]
  },
  renderHTML({ mark, HTMLAttributes }) {
    const type = mark.attrs.type
    return ['span', mergeAttributes({ 'data-status': type, class: `status-mark status-${type}` }), 0]
  },
})

// ── Ctrl+A 渐进式全选（仿飞书） ──────────────────────────────────────────────
// 第一次：选中光标所在块（当前行/列表项的文字范围）
// 第二次（已选中当前块）：全选整个文档
// 全选状态再按：保持不变
const ProgressiveSelectAll = Extension.create({
  name: 'progressiveSelectAll',
  addKeyboardShortcuts() {
    return {
      'Mod-a': () => {
        const { state, view } = this.editor
        const { selection, doc } = state
        const { $from } = selection

        if (selection instanceof AllSelection) return true

        const from = $from.start()
        const to   = $from.end()
        const isBlockSelected = selection.from === from && selection.to === to

        if (isBlockSelected) {
          view.dispatch(state.tr.setSelection(new AllSelection(doc)))
        } else {
          view.dispatch(state.tr.setSelection(TextSelection.create(doc, from, to)))
        }
        return true
      },
    }
  },
})

// ── Turndown：HTML → Markdown ────────────────────────────────────────────────
const td = new TurndownService({ headingStyle: 'atx', bulletListMarker: '-', codeBlockStyle: 'fenced' })
td.addRule('underline', {
  filter: ['u'],
  replacement: content => `<u>${content}</u>`,
})
td.keep(['u'])
td.addRule('listItemParagraph', {
  filter: node => node.nodeName === 'P' && node.parentNode?.nodeName === 'LI',
  replacement: content => content,
})
td.addRule('orderedListItem', {
  filter: node => node.nodeName === 'LI' && node.parentNode?.nodeName === 'OL',
  replacement: (content) => {
    // 续行缩进3格（与"1. "等宽），防止嵌套 bullet 被 marked 解析为独立列表
    const body = content
      .replace(/^\n+/, '')
      .replace(/\n+$/, '')
      .replace(/\n/g, '\n   ')
    return '\n1. ' + body
  },
})
td.addRule('statusMark', {
  filter: node => node.nodeName === 'SPAN' && node.hasAttribute('data-status'),
  replacement: (content, node) =>
    `<span data-status="${node.getAttribute('data-status')}">${content}</span>`,
})

// ── H1 模块化拖拽排序 ────────────────────────────────────────────────────────
let currentPre = ''
let currentH1s = []
let dragFromIdx = -1
let scrollEl = null
const dragZones      = ref([])   // 每个 H1 左侧手柄的位置
const insertLines    = ref([])   // N+1 条插入指示线（top + active 标志）
const sectionFade    = ref(null) // 拖动中源区域半透明遮罩 { top, height }
const hoveredZoneIdx = ref(-1)   // 当前鼠标悬停的 H1 索引，用于显示手柄

// ── 未完成事项总览 ────────────────────────────────────────────────────────────
const todoSections     = ref([])        // [{ title, items: [{ type, text, node }] }]
const hasH1InDoc       = ref(false)     // 当前文档是否含有 H1
const expandedSections = ref(new Set()) // 已展开的 H1 标题集合

function updateTodoOverview() {
  if (!editor.value) { todoSections.value = []; hasH1InDoc.value = false; return }
  const children = Array.from(editor.value.view.dom.childNodes).filter(n => n.nodeType === 1)

  const hasH1 = children.some(c => c.tagName === 'H1')
  const all = []
  // 第一个标题之前的内容归入隐式分组"未分组"
  let cur = { title: '未分组', items: [] }
  for (const child of children) {
    if (child.tagName === 'H1') {
      all.push(cur)
      cur = { title: child.textContent.trim(), items: [] }
    } else {
      child.querySelectorAll('span[data-status="ing"], span[data-status="todo"]')
        .forEach(span => cur.items.push({
          type: span.getAttribute('data-status'),
          text: span.textContent,
          node: span,
        }))
    }
  }
  all.push(cur)

  hasH1InDoc.value = hasH1
  todoSections.value = all.filter(s => s.items.length > 0)
}

function toggleSection(title) {
  const s = new Set(expandedSections.value)
  s.has(title) ? s.delete(title) : s.add(title)
  expandedSections.value = s
}

function jumpToItem(item) {
  const node = item.node
  if (!node) return
  const editorContent = editorWrapRef.value?.querySelector('.editor-content')
  if (editorContent) {
    const nodeRect = node.getBoundingClientRect()
    const contRect = editorContent.getBoundingClientRect()
    editorContent.scrollTo({
      top: editorContent.scrollTop + nodeRect.top - contRect.top - contRect.height / 2 + nodeRect.height / 2,
      behavior: 'smooth',
    })
  }
  node.classList.add('todo-jump-highlight')
  setTimeout(() => node.classList.remove('todo-jump-highlight'), 1200)
}

// H1 hover 监听器：鼠标进入 H1 时显示对应手柄
let h1HoverCleanups = []
function bindH1HoverListeners() {
  h1HoverCleanups.forEach(fn => fn())
  h1HoverCleanups = []
  if (!editor.value) return
  const h1s = Array.from(editor.value.view.dom.querySelectorAll('h1'))
  h1s.forEach((h1, i) => {
    const enter = () => { hoveredZoneIdx.value = i }
    const leave = () => { if (hoveredZoneIdx.value === i) hoveredZoneIdx.value = -1 }
    h1.addEventListener('mouseenter', enter)
    h1.addEventListener('mouseleave', leave)
    h1HoverCleanups.push(() => { h1.removeEventListener('mouseenter', enter); h1.removeEventListener('mouseleave', leave) })
  })
}

function extractSections(md) {
  const idx = md.search(/^# /m)
  if (idx === -1) return { pre: md, sections: [] }
  const pre = md.slice(0, idx)
  const parts = md.slice(idx).split(/(?=^# )/m).filter(s => s.trim())
  return { pre, sections: parts }
}

function joinSections(pre, sections) {
  if (!sections.length) return pre
  return pre + sections.map(s => s.replace(/\n*$/, '\n')).join('\n')
}

async function doReorder(pre, sections) {
  currentPre = pre
  currentH1s = sections
  const md = joinSections(pre, sections)
  const rawHtml = String(await marked.parse(md))
  const html = fixOrderedListStructure(rawHtml)
  // emitUpdate=true：触发 onUpdate → emit change → 保存，同时刷新 currentH1s
  editor.value.commands.setContent(html, true)
  await nextTick()
  updateDragZones()
}

function updateDragZones() {
  if (dragFromIdx >= 0) return
  if (!editor.value || !editorWrapRef.value) return
  const wrapRect   = editorWrapRef.value.getBoundingClientRect()
  const editorDom  = editor.value.view.dom
  const h1s        = Array.from(editorDom.querySelectorAll('h1'))

  dragZones.value = h1s.map(h1 => {
    const r = h1.getBoundingClientRect()
    return {
      top:    (r.top  - wrapRect.top)  + 'px',
      left:   (r.left - wrapRect.left - 22) + 'px',
      height: r.height + 'px',
    }
  })

  // 用 scrollEl（.editor-content）的可视底边，而非 ProseMirror DOM 实际底边
  // editorDom.getBoundingClientRect().bottom 在内容超出可视区时会超过 overlay 高度，被 editor-wrap 的 overflow:hidden 裁掉
  const contentBottom = scrollEl ? scrollEl.getBoundingClientRect().bottom : editorDom.getBoundingClientRect().bottom
  insertLines.value = [
    ...h1s.map(h1 => ({
      top: (h1.getBoundingClientRect().top - wrapRect.top) + 'px',
      active: false,
    })),
    { top: (contentBottom - wrapRect.top) + 'px', active: false },
  ]

  bindH1HoverListeners()
}

function startSectionDrag(e, fromIdx) {
  if (dragFromIdx >= 0 || !editor.value || !editorWrapRef.value) return
  dragFromIdx = fromIdx

  const editorDom = editor.value.view.dom
  const wrapRect  = editorWrapRef.value.getBoundingClientRect()
  const h1s       = Array.from(editorDom.querySelectorAll('h1'))
  const fromH1    = h1s[fromIdx]
  if (!fromH1) { dragFromIdx = -1; return }

  // ① 拖动开始时从 DOM 新鲜计算所有插入线 Y 坐标（纯数字），
  //    同步写入 insertLines.value，确保显示与计算完全一致，消除竞态
  //    底部用 scrollEl 可视底边，而非 editorDom 实际底边（后者超出 overlay 会被裁掉）
  const editorRect = editorDom.getBoundingClientRect()
  const contentBottom = scrollEl ? scrollEl.getBoundingClientRect().bottom : editorRect.bottom
  const lineYs = [
    ...h1s.map(h1 => h1.getBoundingClientRect().top - wrapRect.top),
    contentBottom - wrapRect.top,
  ]
  insertLines.value = lineYs.map(y => ({ top: y + 'px', active: false }))

  // 胶囊：跟随鼠标的标题气泡
  const capsule = document.createElement('div')
  capsule.className = 'drag-capsule'
  capsule.textContent = fromH1.textContent || ''
  capsule.style.left = (e.clientX + 14) + 'px'
  capsule.style.top  = (e.clientY - 14) + 'px'
  document.body.appendChild(capsule)

  // 源区域半透明遮罩（从 H1 顶到下一个 H1 顶 / 编辑区底）
  const fromRect      = fromH1.getBoundingClientRect()
  const nextH1        = h1s[fromIdx + 1]
  const sectionBottom = nextH1
    ? nextH1.getBoundingClientRect().top
    : editorRect.bottom
  sectionFade.value = {
    top:    (fromRect.top    - wrapRect.top) + 'px',
    height: (sectionBottom   - fromRect.top) + 'px',
  }

  // 拖动期间禁止手柄拦截事件
  const zoneEls = editorWrapRef.value.querySelectorAll('.drag-zone')
  zoneEls.forEach(z => { z.style.pointerEvents = 'none' })

  let activeLineIdx = -1

  const onMove = mv => {
    capsule.style.left = (mv.clientX + 14) + 'px'
    capsule.style.top  = (mv.clientY - 14) + 'px'

    // ② 用快照 lineYs（纯数字）计算最近插入点，不再读 insertLines.value
    const relY = mv.clientY - wrapRect.top
    let bestIdx = -1, bestDist = Infinity
    lineYs.forEach((lineTop, i) => {
      // 跳过当前位置（fromIdx 前后各一条线是无效落点）
      if (i === fromIdx || i === fromIdx + 1) return
      const dist = Math.abs(relY - lineTop)
      if (dist < bestDist) { bestDist = dist; bestIdx = i }
    })
    // ③ 只在 activeLineIdx 变化时才更新响应式数组，减少不必要渲染
    if (activeLineIdx !== bestIdx) {
      activeLineIdx = bestIdx
      insertLines.value = insertLines.value.map((l, i) => ({ ...l, active: i === activeLineIdx }))
    }
  }

  const onUp = async () => {
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    capsule.remove()
    zoneEls.forEach(z => { z.style.pointerEvents = '' })
    sectionFade.value = null

    const toIdx = activeLineIdx
    dragFromIdx = -1
    insertLines.value = insertLines.value.map(l => ({ ...l, active: false }))

    if (toIdx < 0 || toIdx === fromIdx || toIdx === fromIdx + 1) {
      updateDragZones()
      return
    }

    const sections   = [...currentH1s]
    const [moved]    = sections.splice(fromIdx, 1)
    // toIdx > fromIdx 时删除操作使后续下标 -1，需补偿
    const insertAt   = toIdx > fromIdx ? toIdx - 1 : toIdx
    sections.splice(insertAt, 0, moved)
    await doReorder(currentPre, sections)
  }

  document.body.style.cursor = 'grabbing'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

// ── 气泡菜单状态 ─────────────────────────────────────────────────────────────
const bubbleVisible = ref(false)
const bubbleStyle = ref({})
const editorWrapRef = ref(null)

const BUBBLE_W = 240 // 气泡菜单估算宽度，用于边界钳制

function updateBubble(ed) {
  const { empty, from, to } = ed.state.selection
  if (empty) { bubbleVisible.value = false; return }
  const view = ed.view
  const startCoords = view.coordsAtPos(from)
  const endCoords = view.coordsAtPos(to)
  const wrap = editorWrapRef.value
  if (!wrap) { bubbleVisible.value = false; return }
  const rect = wrap.getBoundingClientRect()
  const rawMid = (startCoords.left + endCoords.left) / 2 - rect.left
  // 保证菜单不超出左右边界
  const clampedLeft = Math.max(BUBBLE_W / 2, Math.min(rawMid, rect.width - BUBBLE_W / 2))
  const topY = startCoords.top - rect.top - 46
  bubbleStyle.value = {
    top: `${topY}px`,
    left: `${clampedLeft}px`,
    transform: 'translateX(-50%)',
  }
  bubbleVisible.value = true
}

// ── 图片插入（工具栏/拖拽/粘贴共用） ─────────────────────────────────────────
function readImageAsDataURL(file, cb) {
  if (!file || !file.type.startsWith('image/')) return
  const reader = new FileReader()
  reader.onload = evt => cb(evt.target.result)
  reader.readAsDataURL(file)
}

function insertImageFile(file) {
  readImageAsDataURL(file, src => {
    editor.value?.chain().focus().setImage({ src }).run()
  })
}

function handleDrop(view, event) {
  const files = event.dataTransfer?.files
  if (!files || files.length === 0) return false
  const images = Array.from(files).filter(f => f.type.startsWith('image/'))
  if (images.length === 0) return false
  event.preventDefault()
  const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos
  images.forEach(file => {
    readImageAsDataURL(file, src => {
      if (pos != null) {
        editor.value?.chain().insertContentAt(pos, { type: 'image', attrs: { src } }).run()
      } else {
        editor.value?.chain().focus().setImage({ src }).run()
      }
    })
  })
  return true
}

function handlePaste(view, event) {
  const items = event.clipboardData?.items
  if (!items) return false
  const files = []
  for (const item of items) {
    if (item.kind === 'file' && item.type.startsWith('image/')) {
      const file = item.getAsFile()
      if (file) files.push(file)
    }
  }
  if (files.length === 0) return false
  event.preventDefault()
  files.forEach(file => insertImageFile(file))
  return true
}

// ── 编辑器 ───────────────────────────────────────────────────────────────────
const editor = useEditor({
  extensions: [
    StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
    Underline,
    Image.configure({ inline: false }),
    Link.configure({ openOnClick: false }),
    Placeholder.configure({ placeholder: '开始记录...\n\n# 标题   - 列表   1. 有序列表' }),
    StatusMark,
    TabIndentList,
    ContinuousOrderedList,
    ProgressiveSelectAll,
  ],
  content: '',
  editorProps: {
    handleDrop,
    handlePaste,
  },
  onUpdate({ editor }) {
    const md = td.turndown(editor.getHTML())
    const { pre, sections } = extractSections(md)
    currentPre = pre
    currentH1s = sections
    emit('change', md)
    nextTick(() => { updateDragZones(); updateTodoOverview() })
  },
  onSelectionUpdate({ editor }) {
    updateBubble(editor)
  },
  onBlur() {
    // 延迟隐藏，给气泡按钮的 mousedown 留出时间
    setTimeout(() => { bubbleVisible.value = false }, 150)
  },
  onFocus({ editor }) {
    updateBubble(editor)
  },
})

function fixOrderedListStructure(html) {
  const div = document.createElement('div')
  div.innerHTML = html

  // 把紧跟在 <ol> 后面的顶层 <ul> 移进该 <ol> 最后一个 <li>，使其成为嵌套子列表
  let changed = true
  while (changed) {
    changed = false
    const children = Array.from(div.children)
    for (let i = 1; i < children.length; i++) {
      if (children[i - 1].tagName === 'OL' && children[i].tagName === 'UL') {
        const lastLi = children[i - 1].lastElementChild
        if (lastLi) { lastLi.appendChild(children[i]); changed = true; break }
      }
    }
  }

  // 合并相邻 <ol>，使编号连续
  changed = true
  while (changed) {
    changed = false
    const children = Array.from(div.children)
    for (let i = 1; i < children.length; i++) {
      if (children[i - 1].tagName === 'OL' && children[i].tagName === 'OL') {
        while (children[i].firstChild) children[i - 1].appendChild(children[i].firstChild)
        children[i].remove(); changed = true; break
      }
    }
  }

  return div.innerHTML
}

onMounted(async () => {
  await nextTick()
  if (!editor.value) return
  const initMd = props.initialContent || ''
  const { pre, sections } = extractSections(initMd)
  currentPre = pre
  currentH1s = sections
  const rawHtml = String(await marked.parse(initMd))
  const html = fixOrderedListStructure(rawHtml)
  editor.value.commands.setContent(html, false)
  await nextTick()
  updateDragZones()
  updateTodoOverview()
  scrollEl = editorWrapRef.value?.querySelector('.editor-content')
  scrollEl?.addEventListener('scroll', updateDragZones)
})

// ── 状态标记操作 ─────────────────────────────────────────────────────────────
function applyStatus(type) {
  if (!editor.value) return
  const isActive = editor.value.isActive('status', { type })
  if (isActive) {
    editor.value.chain().focus().unsetMark('status').run()
  } else {
    editor.value.chain().focus().unsetMark('status').setMark('status', { type }).run()
  }
}
function clearStatus() {
  editor.value?.chain().focus().unsetMark('status').run()
}

// ── 链接点击 → 默认浏览器 ────────────────────────────────────────────────────
function handleEditorClick(e) {
  const a = e.target.closest('a')
  if (a?.href) { e.preventDefault(); openUrl(a.href) }
}

// ── 工具栏 ───────────────────────────────────────────────────────────────────
function bold()      { editor.value?.chain().focus().toggleBold().run() }
function italic()    { editor.value?.chain().focus().toggleItalic().run() }
function underline() { editor.value?.chain().focus().toggleUnderline().run() }
function strike()    { editor.value?.chain().focus().toggleStrike().run() }

function insertLink() {
  const prev = editor.value?.getAttributes('link').href || ''
  const url = window.prompt('输入链接地址：', prev)
  if (url === null) return
  url === ''
    ? editor.value?.chain().focus().unsetLink().run()
    : editor.value?.chain().focus().setLink({ href: url, target: '_blank' }).run()
}

function insertImage() {
  const input = document.createElement('input')
  input.type = 'file'; input.accept = 'image/*'
  input.onchange = e => insertImageFile(e.target.files[0])
  input.click()
}

onBeforeUnmount(() => { scrollEl?.removeEventListener('scroll', updateDragZones); h1HoverCleanups.forEach(fn => fn()); editor.value?.destroy() })
</script>

<template>
  <div class="editor-wrap" ref="editorWrapRef">
    <!-- 气泡菜单：选中文字后浮现 -->
    <div
      v-show="bubbleVisible"
      class="bubble-menu"
      :style="bubbleStyle"
    >
      <button
        class="bubble-btn"
        :class="{ active: editor?.isActive('status', { type: 'ing' }) }"
        @mousedown.prevent="applyStatus('ing')"
        title="进行中"
      >🟢 进行中</button>
      <button
        class="bubble-btn"
        :class="{ active: editor?.isActive('status', { type: 'todo' }) }"
        @mousedown.prevent="applyStatus('todo')"
        title="待处理"
      >🟡 待处理</button>
      <button
        class="bubble-btn"
        :class="{ active: editor?.isActive('status', { type: 'done' }) }"
        @mousedown.prevent="applyStatus('done')"
        title="已解决"
      >✅ 已解决</button>
      <div class="bubble-sep"></div>
      <button
        class="bubble-btn clear"
        @mousedown.prevent="clearStatus"
        title="清除标记"
      >✕</button>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <button class="tool-btn" :class="{ active: editor?.isActive('bold') }"
        @mousedown.prevent="bold" title="加粗 (Ctrl+B)"><b>B</b></button>
      <button class="tool-btn" :class="{ active: editor?.isActive('italic') }"
        @mousedown.prevent="italic" title="斜体 (Ctrl+I)"><i>I</i></button>
      <button class="tool-btn" :class="{ active: editor?.isActive('underline') }"
        @mousedown.prevent="underline" title="下划线 (Ctrl+U)"><u>U</u></button>
      <button class="tool-btn" :class="{ active: editor?.isActive('strike') }"
        @mousedown.prevent="strike" title="删除线"><s>S</s></button>
      <div class="tool-sep"></div>
      <button class="tool-btn" :class="{ active: editor?.isActive('link') }"
        @mousedown.prevent="insertLink" title="插入链接">🔗</button>
      <button class="tool-btn" @mousedown.prevent="insertImage" title="插入图片">🖼</button>
    </div>

    <!-- 未完成事项总览：仅在文档含 H1 时渲染 -->
    <div v-if="hasH1InDoc" class="todo-overview">
      <template v-if="todoSections.length > 0">
        <div v-for="sec in todoSections" :key="sec.title" class="todo-section">
          <div class="todo-section-header" @click="toggleSection(sec.title)">
            <span class="todo-section-title">{{ sec.title }}</span>
            <span class="todo-badge">{{ sec.items.length }}</span>
            <span class="todo-chevron">{{ expandedSections.has(sec.title) ? '▲' : '▼' }}</span>
          </div>
          <div v-if="expandedSections.has(sec.title)" class="todo-items">
            <div
              v-for="(item, j) in sec.items"
              :key="j"
              class="todo-item"
              @click="jumpToItem(item)"
            >
              <span class="todo-item-icon">{{ item.type === 'ing' ? '🟢' : '🟡' }}</span>
              <span class="todo-item-text">{{ item.text }}</span>
            </div>
          </div>
        </div>
      </template>
      <div v-else class="todo-empty">当前没有待处理事项 ✓</div>
    </div>

    <EditorContent :editor="editor" class="editor-content" @click="handleEditorClick" />

    <!-- H1 模块拖拽排序 overlays（在 contenteditable 外，不干扰编辑） -->
    <div class="drag-overlay-layer" aria-hidden="true">
      <!-- 拖拽手柄：悬停 H1 时浮现（hoveredZoneIdx 控制） -->
      <div
        v-for="(zone, i) in dragZones"
        :key="'hz-' + i"
        class="drag-zone"
        :class="{ visible: hoveredZoneIdx === i }"
        :style="{ top: zone.top, left: zone.left, height: zone.height }"
        @mouseenter="hoveredZoneIdx = i"
        @mouseleave="hoveredZoneIdx = -1"
        @mousedown.prevent.stop="startSectionDrag($event, i)"
      ></div>
      <!-- 源区域半透明遮罩 -->
      <div
        v-if="sectionFade"
        class="drag-section-fade"
        :style="{ top: sectionFade.top, height: sectionFade.height }"
      ></div>
      <!-- 插入指示线：渲染在遮罩之后，确保始终在最上层 -->
      <div
        v-for="(line, i) in insertLines"
        :key="'il-' + i"
        class="drag-insert-line"
        :class="{ active: line.active }"
        :style="{ top: line.top }"
      ></div>
    </div>
  </div>
</template>
