# CHANGELOG

## [0.3.0] - 2026-07-24

### 新功能

**便笺列表右键置顶 / 取消置顶**

- 在便笺列表主界面，右键单击任意便笺卡片，可通过上下文菜单置顶或取消置顶
- 置顶的便笺固定显示在列表最上方，卡片左侧显示橙色边线 + 📌 图标
- 数据存储：置顶状态单独存入 `~/StickyNotesData/_pins.json`（Key = 便笺 ID，Value = 置顶时间戳），独立于各便笺的 Markdown 正文；与 Markdown frontmatter 中的 `pinned` 字段完全解耦，老笔记无需迁移
- 影响文件：`src-tauri/src/lib.rs`、`src/components/ListView.vue`、`src/style.css`

**置顶区域自由拖拽排序**

- 置顶区域有多个便笺时，直接按住卡片拖动即可调整彼此之间的先后顺序
- 拖动过程中显示蓝色插入指示线（`#2563eb`，4px，白色内环 + 蓝色外环光晕），与编辑器内 H1 拖拽指示线风格一致
- 跨区域操作被正确阻止：置顶便笺不能拖到未置顶区域，反之亦然
- 排序持久化：手动顺序保存至 `~/StickyNotesData/_pin_order.json`（`Vec<String>`，存 ID 有序列表），重启后保持不变
- 新置顶的便笺默认插入置顶区域最顶部；搜索状态下禁用拖拽（避免过滤后顺序不完整）
- 影响文件：`src-tauri/src/lib.rs`（新增 `reorder_pins` 命令 + `_pin_order.json` 读写逻辑）、`src/components/ListView.vue`

**Tab 键缩进自动继承编号**

- 光标停在有序/无序列表的任意一行，按 **Tab** 将该行降一级（成为上一项的子项）；按 **Shift+Tab** 升一级
- 缩进变化后自动触发 `ContinuousOrderedList` 全量重算，所有受影响的编号（调整行本身、同级兄弟项、原层级后续项）均正确更新
- 不涉及数据存储变更，纯编辑器行为
- 实现：新增 `TabIndentList` Tiptap Extension（`priority: 200`），以高于内置 ListItem 的优先级显式注册 `sinkListItem` / `liftListItem` 命令，确保行为明确、不依赖 StarterKit 版本内部实现
- 影响文件：`src/components/NoteEditor.vue`

---

## [0.2.2] - 2026-07-24

### 新功能

**H1 模块化拖拽排序**

- 将 H1 标题及其下方所有内容视为一个独立模块，可通过拖拽手柄整体上移/下移，用于调整任务优先级顺序
- 交互细节：
  - 鼠标悬停 H1 时，左侧出现拖拽手柄（⠿），离开后隐藏
  - 拖动时标题以胶囊气泡形式跟随鼠标，源区域半透明遮罩标记原位置
  - 落点仅限模块之间的间隙（不支持嵌套进其他 H1 内部）
  - 拖拽后自动重新计算有序列表编号，与 `ContinuousOrderedList` 插件联动
  - 支持撤销（Ctrl+Z）：底层通过 Tiptap `setContent` 写入编辑器历史栈
- 插入指示线最终样式：蓝色（`#2563eb`）、线宽 4px、白色内环 + 蓝色外环光晕（`box-shadow: 0 0 0 1.5px #ffffff, 0 0 0 3px #2563eb`）、两端端点圆直径 9px

### 修复

**插入指示线显示不稳定**

- 根本原因：插入指示线底部位置计算使用了 `editorDom.getBoundingClientRect().bottom`（ProseMirror 内容 DOM 的实际底边），当笔记内容超出编辑器可视高度时，该值远大于 overlay 容器高度，导致底部指示线被 `.editor-wrap` 的 `overflow: hidden` 裁切而不可见
- 修复：改为使用 `scrollEl.getBoundingClientRect().bottom`（`.editor-content` 可视区域的实际底边），始终在屏幕内，不被裁切
- 同步修复：拖动开始时从 DOM 实时快照所有插入线位置（纯数字数组），`onMove` 改用快照做距离计算，避免读写 Vue 响应式数组的竞态

---

## [0.2.1] - 2026-07-23

### 新功能

**Ctrl+A 渐进式全选（仿飞书）**

- 第一次按 Ctrl+A：只选中光标所在的当前行/块内容，而非全选整篇
- 再次按 Ctrl+A（当前行已选中状态下）：扩大为全选整篇内容
- 全选状态再按：保持不变
- 光标移动或点击他处后，下次 Ctrl+A 重新从"选当前行"开始
- 实现方式：新增 `ProgressiveSelectAll` Tiptap Extension，覆盖 ProseMirror 默认的 `Mod-a` 行为
- 影响范围：`src/components/NoteEditor.vue`

### 维护

**清理未完成的"H1 模块化拖动"遗留代码**

- 原因：此功能当初未做完，但相关代码误留在正式包里
- 清理内容：
  - `NoteEditor.vue`：删除 6 个状态变量（`currentMd`/`currentPre`/`currentH1s`/`dragFromIdx`/`scrollEl`/`dragZones`）、6 个函数（`extractSections`/`joinSections`/`doReorder`/`clearDropIndicators`/`updateDragZones`/`startSectionDrag`）、dragHandle Turndown 规则、onUpdate/onMounted/onBeforeUnmount 里的拖动专属逻辑、模板里的 drag-zones-layer
  - `style.css`：删除 H1 拖拽排序相关 CSS（`.drag-zones-layer`/`.drag-zone`/`.drag-over-above`/`.drag-over-below`），去掉 H1 为手柄留出的 `padding-left: 18px`
- 已确认保留完整的功能：自动编号（`ContinuousOrderedList`）、Ctrl+A 渐进式全选（`ProgressiveSelectAll`）、气泡菜单及所有编辑功能
- 旧笔记兼容：完全兼容，无数据丢失风险

---

## [0.2.0] - 2026-07-23

### 修复

**有序列表自动编号 bug 修复**

- 问题：在有序列表中增删内容后，编号不会自动重新计算（只有被直接编辑的行附近会局部更新，其余行编号停留在旧数字）。
- 根本原因：原实现完全依赖浏览器 CSS counter，每个独立的 `<ol>` 元素从 1 重新计数，中间夹入无序列表或段落时计数链断裂。
- 修复方式：新增 `ContinuousOrderedList` ProseMirror 插件，在每次文档结构变化后（增删改/撤销/粘贴）对全文有序列表做全量重新推导，通过 `<ol start="N">` 属性修正编号：
  - 无序列表、段落、图片等非有序内容：跳过，不重置计数（飞书规则）
  - 标题（H1/H2/H3）：重置计数，新章节从 1 开始
  - 嵌套有序列表：递归独立推导，每个 `<li>` 内部维护自己的计数上下文
- 影响范围：`src/components/NoteEditor.vue`（新增约 55 行，无旧代码删除）
- 旧笔记兼容：完全兼容，加载时自动按新规则重算，无数据丢失风险

---

## [0.1.0] - 2026-07-18

### 首次发布

- 便签列表面板：展示/搜索/新建/删除
- 无边框便签窗口（黄色）+ 自定义标题栏
- Tiptap 富文本编辑：H1/H2/H3、有序/无序列表、加粗/斜体/下划线/删除线
- 任务状态标记：🟢进行中 / 🟡待处理 / ✅已解决（气泡菜单）
- 插入超链接（默认浏览器打开）、插入图片（base64 内嵌）
- 自动保存（500ms 防抖），窗口位置/尺寸/置顶跨会话恢复
- 数据存为标准 Markdown（`~/StickyNotesData/`）
- 打包为 QB便签 NSIS 安装包
