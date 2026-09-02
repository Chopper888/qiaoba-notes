import { Extension } from '@tiptap/core'
import { sinkListItem, liftListItem } from '@tiptap/pm/schema-list'
import { Fragment } from '@tiptap/pm/model'

const LIST_TYPES = new Set(['orderedList', 'bulletList'])
const isList = node => node && LIST_TYPES.has(node.type.name)

// 自定义 sinkListItem：当光标所在列表项是其所在 <ol>/<ul> 的首项（startIndex === 0），
// 且该列表前面隔着图片/段落等内容、后面还有上一个列表时，内置 sinkListItem 会直接返回 false。
// 这里改为：找到上一个列表的最后一项，把当前项移进去成为嵌套子列表，跨越"内容隔断"实现 Tab 缩进。
export function sinkListItemCrossList(itemType) {
  return (state, dispatch) => {
    const type = typeof itemType === 'string' ? state.schema.nodes[itemType] : itemType
    if (!type) return false
    const base = sinkListItem(type)
    const { $from, $to } = state.selection
    const range = $from.blockRange($to, node => node.childCount > 0 && node.firstChild.type === type)
    if (!range) return false
    // 非首项：走内置逻辑（同一列表内缩进）
    if (range.startIndex !== 0) {
      return base(state, dispatch)
    }

    const list = range.parent        // 当前项所在的列表（<ol>/<ul>）
    const listPos = range.start - 1  // 该列表节点在文档中的位置
    // 找上一个列表项：遍历 listPos 之前所有 listItem，取最后一个
    let prevItemPos = -1
    state.doc.nodesBetween(0, listPos, (node, pos) => {
      if (node.type.name === 'listItem') prevItemPos = pos
    })
    if (prevItemPos < 0) return false
    const prevItem = state.doc.nodeAt(prevItemPos)
    if (!prevItem || prevItem.type !== type) return false
    if (!dispatch) return true

    const firstItem = list.firstChild
    const nestedList = list.type.create(null, Fragment.from(firstItem))
    const insertPos = prevItemPos + prevItem.nodeSize - 1  // 上一个列表项的内容末尾
    const tr = state.tr
    if (list.childCount === 1) {
      // 列表只有这一项：整体移入上一个列表项
      tr.delete(listPos, listPos + list.nodeSize)
      tr.insert(insertPos, nestedList)
    } else {
      // 列表有多项：只移动首项，其余项留在原列表（编号由 ContinuousOrderedList 重算）
      tr.delete(range.start, range.end)
      tr.insert(insertPos, nestedList)
    }
    dispatch(tr.scrollIntoView())
    return true
  }
}

// 返回"空段落且夹在两个列表之间"时的段落范围，否则返回 null。
function emptyParagraphBetweenLists(state) {
  const { $from } = state.selection
  const para = $from.parent
  if (para.type.name !== 'paragraph' || para.content.size !== 0) return null
  const paraPos = $from.before($from.depth)
  const prev = state.doc.resolve(paraPos).nodeBefore
  const next = state.doc.resolve(paraPos + para.nodeSize).nodeAfter
  if (!isList(prev) || !isList(next)) return null
  return { start: paraPos, end: paraPos + para.nodeSize }
}

// 空段落夹在两个列表之间时，删除该空段落（合并列表），返回 true；否则返回 false。
export function deleteEmptyParagraphBetweenLists(state, dispatch) {
  const range = emptyParagraphBetweenLists(state)
  if (!range) return false
  if (dispatch) dispatch(state.tr.delete(range.start, range.end).scrollIntoView())
  return true
}

export const TabIndentList = Extension.create({
  name: 'tabIndentList',
  priority: 200,
  addKeyboardShortcuts() {
    const sink = sinkListItemCrossList('listItem')
    const lift = liftListItem('listItem')
    return {
      Tab: () => sink(this.editor.state, this.editor.view.dispatch),
      'Shift-Tab': () => lift(this.editor.state, this.editor.view.dispatch),
      Backspace: () => deleteEmptyParagraphBetweenLists(this.editor.state, this.editor.view.dispatch),
      Delete: () => deleteEmptyParagraphBetweenLists(this.editor.state, this.editor.view.dispatch),
    }
  },
})
