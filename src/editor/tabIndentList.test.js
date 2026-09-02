import { describe, it, expect } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import { ContinuousOrderedList } from './continuousOrderedList.js'
import { sinkListItemCrossList, deleteEmptyParagraphBetweenLists } from './tabIndentList.js'

function makeEditor() {
  return new Editor({
    extensions: [StarterKit, Image, ContinuousOrderedList],
  })
}

function posOfText(editor, text) {
  let found = -1
  editor.state.doc.descendants((node, pos) => {
    if (node.isText && node.text === text) found = pos + 1
  })
  return found
}

function olStarts(editor) {
  const starts = []
  editor.state.doc.descendants((node) => {
    if (node.type.name === 'orderedList') starts.push(node.attrs.start)
  })
  return starts
}

describe('sinkListItemCrossList（Tab 跨列表缩进）', () => {
  it('图片隔断的两个列表：Tab 把后续项缩进到上一项', () => {
    const editor = makeEditor()
    editor.commands.setContent('<ol><li><p>A</p></li></ol><img src="x"><ol><li><p>B</p></li></ol>')
    editor.commands.setTextSelection(posOfText(editor, 'B'))
    const ok = sinkListItemCrossList('listItem')(editor.state, editor.view.dispatch)
    expect(ok).toBe(true)
    const json = editor.getJSON().content
    // 第一个有序列表应包含嵌套的 B
    const firstOl = json[0]
    expect(firstOl.type).toBe('orderedList')
    expect(firstOl.content[0].type).toBe('listItem')
    const nested = firstOl.content[0].content.find(n => n.type === 'orderedList')
    expect(nested).toBeTruthy()
    expect(nested.content[0].content[0].content[0].text).toBe('B')
    // 图片仍在顶层
    expect(json[1].type).toBe('image')
    editor.destroy()
  })

  it('段落隔断的两个列表：Tab 把后续项缩进到上一项', () => {
    const editor = makeEditor()
    editor.commands.setContent('<ol><li><p>A</p></li></ol><p>x</p><ol><li><p>B</p></li></ol>')
    editor.commands.setTextSelection(posOfText(editor, 'B'))
    const ok = sinkListItemCrossList('listItem')(editor.state, editor.view.dispatch)
    expect(ok).toBe(true)
    const firstOl = editor.getJSON().content[0]
    const nested = firstOl.content[0].content.find(n => n.type === 'orderedList')
    expect(nested.content[0].content[0].content[0].text).toBe('B')
    editor.destroy()
  })

  it('多项列表：只缩进首项，其余项留在原列表', () => {
    const editor = makeEditor()
    editor.commands.setContent('<ol><li><p>A</p></li></ol><p>x</p><ol><li><p>B</p></li><li><p>C</p></li></ol>')
    editor.commands.setTextSelection(posOfText(editor, 'B'))
    const ok = sinkListItemCrossList('listItem')(editor.state, editor.view.dispatch)
    expect(ok).toBe(true)
    const json = editor.getJSON().content
    // A 下嵌套 B
    const firstOl = json[0]
    const nested = firstOl.content[0].content.find(n => n.type === 'orderedList')
    expect(nested.content[0].content[0].content[0].text).toBe('B')
    // C 仍在第二个顶层列表里
    expect(json[2].type).toBe('orderedList')
    expect(json[2].content[0].content[0].content[0].text).toBe('C')
    // 编号：A=1，嵌套 B=1，C 延续为 2
    expect(olStarts(editor)).toEqual([1, 1, 2])
    editor.destroy()
  })

  it('同一列表内的普通缩进仍走内置逻辑', () => {
    const editor = makeEditor()
    editor.commands.setContent('<ol><li><p>A</p></li><li><p>B</p></li></ol>')
    editor.commands.setTextSelection(posOfText(editor, 'B'))
    const ok = sinkListItemCrossList('listItem')(editor.state, editor.view.dispatch)
    expect(ok).toBe(true)
    const firstOl = editor.getJSON().content[0]
    const nested = firstOl.content[0].content.find(n => n.type === 'orderedList')
    expect(nested).toBeTruthy()
    expect(nested.content[0].content[0].content[0].text).toBe('B')
    editor.destroy()
  })
})

describe('deleteEmptyParagraphBetweenLists（删空行）', () => {
  it('空段落在两个有序列表之间：删除后合并，前后编号不变', () => {
    const editor = makeEditor()
    editor.commands.setContent('<ol><li><p>A</p></li></ol><p></p><ol><li><p>B</p></li></ol>')
    // 光标定位到空段落
    let emptyP = null
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'paragraph' && node.childCount === 0 && !emptyP) emptyP = { node, pos }
    })
    editor.commands.setTextSelection(emptyP.pos + 1)
    const ok = deleteEmptyParagraphBetweenLists(editor.state, editor.view.dispatch)
    expect(ok).toBe(true)
    // 空段落被删除，两个列表相邻，编号仍为 [1, 2]
    const json = editor.getJSON().content
    expect(json[0].type).toBe('orderedList')
    expect(json[1].type).toBe('orderedList')
    expect(olStarts(editor)).toEqual([1, 2])
    editor.destroy()
  })

  it('空段落不夹在两个列表之间：不处理', () => {
    const editor = makeEditor()
    editor.commands.setContent('<p>a</p><p></p><p>b</p>')
    let emptyP = null
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'paragraph' && node.childCount === 0 && !emptyP) emptyP = { node, pos }
    })
    editor.commands.setTextSelection(emptyP.pos + 1)
    const ok = deleteEmptyParagraphBetweenLists(editor.state, editor.view.dispatch)
    expect(ok).toBe(false)
    editor.destroy()
  })
})
