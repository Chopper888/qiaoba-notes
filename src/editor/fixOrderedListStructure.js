// 修正 marked 输出的有序列表结构，使编号连续、嵌套子列表正确挂载：
// 1. 把紧跟 <ol> 之后的顶层 <ul> 移进该 <ol> 最后一个 <li>，成为嵌套子列表
// 2. 合并相邻 <ol>，使同层级编号连续
export function fixOrderedListStructure(html) {
  if (!html) return ''
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
