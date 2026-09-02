import { describe, it, expect } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { ContinuousOrderedList } from './continuousOrderedList.js'

function createEditor() {
  return new Editor({
    extensions: [StarterKit, ContinuousOrderedList],
  })
}

function olStarts(editor) {
  const starts = []
  editor.state.doc.descendants((node) => {
    if (node.type.name === 'orderedList') starts.push(node.attrs.start)
  })
  return starts
}

describe('ContinuousOrderedList 编号重算', () => {
  it('段落隔断不重置编号，后续有序列表延续', () => {
    const editor = createEditor()
    editor.commands.setContent(
      '<ol><li><p>第一项</p></li></ol><p>中间段落</p><ol><li><p>第二项</p></li></ol>'
    )
    expect(olStarts(editor)).toEqual([1, 2])
    editor.destroy()
  })

  it('H1 标题隔断后编号重置为 1', () => {
    const editor = createEditor()
    editor.commands.setContent(
      '<ol><li><p>第一项</p></li></ol><h1>标题</h1><ol><li><p>新项</p></li></ol>'
    )
    expect(olStarts(editor)).toEqual([1, 1])
    editor.destroy()
  })

  it('无序列表隔断不重置编号', () => {
    const editor = createEditor()
    editor.commands.setContent(
      '<ol><li><p>第一项</p></li></ol><ul><li><p>无序</p></li></ul><ol><li><p>第二项</p></li></ol>'
    )
    expect(olStarts(editor)).toEqual([1, 2])
    editor.destroy()
  })
})
