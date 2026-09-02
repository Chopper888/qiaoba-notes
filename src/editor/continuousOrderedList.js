import { Extension } from '@tiptap/core'
import { Plugin } from '@tiptap/pm/state'

// ── 有序列表全量重算编号 ──────────────────────────────────────────────────────
// 规则（对齐飞书行为）：
//   - 同层级的 <ol> 编号全程延续，中间夹着 <ul>/<p>/图片等不重置计数
//   - 只有 heading（H1/H2/H3）才重置同层级计数器
//   - 文档任何结构变化后（增删改/撤销/粘贴）立即全量重推，不做局部修补
//   - 递归处理嵌套层级：每个 <li> 内部独立维护自己的计数上下文
export const ContinuousOrderedList = Extension.create({
  name: 'continuousOrderedList',
  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction(transactions, _oldState, newState) {
          if (!transactions.some(tr => tr.docChanged)) return null
          const { schema, doc } = newState
          const olType = schema.nodes.orderedList
          const blType = schema.nodes.bulletList
          const hType  = schema.nodes.heading
          if (!olType) return null

          const tr = newState.tr
          let changed = false

          // contentStart：parent 节点内容区域的绝对起始位置
          // （doc 无开始 token 所以传 0；其他节点传 nodePos+1）
          function processLevel(parent, contentStart) {
            let prevOlEnd = -1  // -1 = 当前层级尚未出现有序列表

            parent.forEach((node, offset) => {
              const nodePos = contentStart + offset

              if (node.type === olType) {
                // 全量推导：前面有 ol 则延续，否则从 1 开始
                const thisStart = prevOlEnd >= 0 ? prevOlEnd + 1 : 1
                if ((node.attrs.start || 1) !== thisStart) {
                  tr.setNodeMarkup(nodePos, null, { ...node.attrs, start: thisStart })
                  changed = true
                }
                // 用 thisStart（期望值）而非旧属性值更新 prevOlEnd，避免累积误差
                prevOlEnd = thisStart + node.childCount - 1
                // 递归处理每个 <li> 内部的嵌套有序列表
                node.forEach((li, liOffset) => {
                  processLevel(li, nodePos + 1 + liOffset + 1)
                })

              } else if (hType && node.type === hType) {
                prevOlEnd = -1  // 标题隔断：新 section 的编号从 1 重新开始

              } else if (blType && node.type === blType) {
                // 无序列表：不重置 prevOlEnd（飞书规则：跳过不参与计数）
                // 但要递归进去，处理 <li> 内可能存在的嵌套有序列表
                node.forEach((li, liOffset) => {
                  processLevel(li, nodePos + 1 + liOffset + 1)
                })

              }
              // 段落、图片等其他节点：同样不重置 prevOlEnd，直接跳过
            })
          }

          processLevel(doc, 0)
          return changed ? tr : null
        },
      }),
    ]
  },
})
