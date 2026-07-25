# CHANGELOG

## [1.2.1] - 2026-07-26

### Bug 修复

**修复列表卡片预览文字显示原始 HTML 代码的问题**

- 现象：正文中含有任务状态标记（🟢🟡✅）的便笺，在主列表卡片的预览摘要处会出现 `<span data-status="...">` 等原始 HTML 代码，而非纯文字内容
- 原因：后端生成预览文字时直接截取了正文的原始内容，未先剥离 HTML 标签
- 修复：在 `lib.rs` 的 preview 生成逻辑中，先通过 `strip_html` 函数去除所有 HTML 标签，再截取纯文字内容；搜索功能复用同一字段，同步受益
- 影响文件：`src-tauri/src/lib.rs`

---

## [1.2.0] - 2026-07-25

### 新功能

**热更新（自动检测新版本并提示）**

- 软件启动时自动在后台检测 GitHub Releases 是否有新版本，有则弹窗提示，无则静默不打扰
- 弹窗显示新版本号及更新说明，提供"立即更新"和"稍后再说"两个选项；点击"稍后再说"后本次会话不再重复弹出
- 点击"立即更新"后显示下载进度条，下载安装完成后自动重启应用
- 使用 Tauri 官方 updater 插件 + minisign 签名机制，更新包经签名验证，防止篡改
- 更新源对接 GitHub Releases（`latest.json` 作为元数据文件），无需额外服务器
- 影响文件：`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src-tauri/src/lib.rs`、`src/components/UpdateDialog.vue`（新增）、`src/App.vue`

> **发版注意**：每次发布新版本到 GitHub Releases 时，需同步上传 `latest.json`（含版本号、下载链接、.sig 签名内容）。打包时设置环境变量 `TAURI_SIGNING_PRIVATE_KEY_PATH=C:\Users\a\.tauri\qiaoba-notes.key`，签名文件会自动生成在 `target/release/bundle/nsis/` 目录下。

---

## [1.1.0] - 2026-07-24

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
- 实现：新增 `TabIndentList` Tiptap Extension（`priority: 200`），以高于内置 ListItem 的优先级显式注册 `sinkListItem` / `liftListItem` 命令
- 影响文件：`src/components/NoteEditor.vue`

**便笺内部"未完成事项悬浮总览"**

- 便笺编辑区顶部（工具栏下方）新增常驻总览面板，自动汇总当前便笺中所有标记为 🟢进行中 或 🟡待处理 的内容，无需滚动翻找即可一眼看清全局剩余任务
- 按 H1 模块分组折叠展示：第一层列出含未完成事项的 H1 标题 + 数量徽标，点击展开查看该模块下的具体条目（保留 🟢/🟡 状态图标）；再点一次收起
- 点击展开的具体条目，编辑区自动平滑滚动定位到正文对应位置，并触发短暂高亮闪烁，方便快速跳转继续编辑
- 实时联动：正文中任意状态标记变化（如 🟡→✅）后，总览立刻同步更新；H1 模块整段删除时幽灵数据同步清除
- 无 H1 的简单便笺不显示总览面板；所有模块均无未完成事项时显示"当前没有待处理事项 ✓"
- 兼容 H1 模块化拖拽排序：拖动调整 H1 顺序后，总览列表顺序同步更新
- 影响文件：`src/components/NoteEditor.vue`、`src/style.css`

---

## [1.0.0] - 2026-07-18

### 首次发布

**便笺基础功能**

- 便签列表面板：展示 / 搜索 / 新建 / 删除
- 无边框便签窗口（黄色）+ 自定义标题栏，窗口位置 / 尺寸 / 置顶跨会话恢复
- Tiptap 富文本编辑：H1/H2/H3、有序 / 无序列表、加粗 / 斜体 / 下划线 / 删除线
- 任务状态标记：🟢进行中 / 🟡待处理 / ✅已解决（气泡菜单，选中文字后浮现）
- 插入超链接（默认浏览器打开）、插入图片（base64 内嵌）
- 自动保存（500ms 防抖）
- 数据存为标准 Markdown（`~/StickyNotesData/`）

**编辑器增强**

- 有序列表全量自动重算编号（新增 `ContinuousOrderedList` ProseMirror 插件）：H1 重置计数、非有序内容跳过、嵌套层级独立推导
- Ctrl+A 渐进式全选（仿飞书）：第一次选中当前行，再次按扩大为全选
- H1 模块化拖拽排序：H1 标题及其下方内容视为独立模块，可通过左侧手柄（⠿）整体拖动调整顺序，支持 Ctrl+Z 撤销；蓝色插入指示线（4px，光晕效果）指示落点

**打包**

- 打包为 QB便签 NSIS 安装包（Windows x64）
