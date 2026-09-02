import { describe, it, expect } from 'vitest'
import { fixOrderedListStructure } from './fixOrderedListStructure.js'

describe('fixOrderedListStructure 有序列表结构修正', () => {
  it('空输入返回空字符串', () => {
    expect(fixOrderedListStructure('')).toBe('')
  })

  it('纯文本不改变', () => {
    expect(fixOrderedListStructure('普通文本')).toBe('普通文本')
  })

  it('紧跟 <ol> 的 <ul> 移入最后一个 <li> 成为嵌套子列表', () => {
    const input = '<ol><li><p>第一项</p></li></ol><ul><li><p>子项</p></li></ul>'
    const out = fixOrderedListStructure(input)
    // <ul> 应嵌套进 <ol> 的最后一个 <li> 内
    expect(out).toBe('<ol><li><p>第一项</p><ul><li><p>子项</p></li></ul></li></ol>')
  })

  it('相邻 <ol> 合并使编号连续', () => {
    const input = '<ol><li><p>一</p></li></ol><ol><li><p>二</p></li></ol>'
    const out = fixOrderedListStructure(input)
    expect(out).toBe('<ol><li><p>一</p></li><li><p>二</p></li></ol>')
  })

  it('空 <ol>（无 <li>）后跟 <ul> 不崩溃、结构不变', () => {
    const input = '<ol></ol><ul><li><p>子项</p></li></ul>'
    expect(fixOrderedListStructure(input)).toBe(input)
  })

  it('已正确的嵌套结构保持幂等', () => {
    const input = '<ol><li><p>第一项</p><ul><li><p>子项</p></li></ul></li></ol>'
    expect(fixOrderedListStructure(input)).toBe(input)
  })
})
