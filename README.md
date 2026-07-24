# 乔巴便笺（qiaoba-notes）

**乔巴便笺（qiaoba-notes）** 是一个用于日常记录多任务工作进度的桌面便笺工具，尤其适合需要同时跟踪多个 AI Agent 并行任务的场景——快速记录、清晰追踪每个任务的状态和进度，不在多头任务之间迷失。

---

## 功能列表

- **自动编号** — 有序列表增删内容后自动全量重新计算编号，支持跨内容中断后延续编号（飞书规则：段落/无序列表不重置计数，H1/H2/H3 标题重置计数）
- **"-" + 空格自动转无序列表** — 输入 `- ` 即可快速进入列表模式
- **Tab / Shift+Tab 缩进调整，自动继承编号** — 光标在列表行按 Tab 降一级、Shift+Tab 升一级，缩进变化后编号自动正确重算
- **Ctrl+A 渐进式全选（仿飞书）** — 第一次选中当前行，再按扩大为全选，光标移动后重置
- **H1 模块化拖拽排序** — 将 H1 标题及其下方所有内容视为一个任务模块，通过拖拽手柄整体上移/下移，调整任务优先级顺序，支持 Ctrl+Z 撤销
- **便笺列表右键置顶** — 右键单击便笺卡片，可置顶或取消置顶；置顶便笺固定显示在列表最上方，左侧橙色边线 + 📌 图标标记
- **置顶区域自由拖拽排序** — 置顶区内多个便笺可通过鼠标拖拽自由调整顺序，跨区域拖拽被阻止，排序持久化保存

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri 2.x](https://tauri.app/) |
| 前端框架 | Vue 3.5 |
| 富文本编辑器 | Tiptap 3.28（基于 ProseMirror） |
| Markdown 解析 | marked 18.x |
| Markdown 序列化 | turndown 7.x |
| 后端语言 | Rust |
| 打包格式 | NSIS 安装包（Windows） |

---

## 如何运行（开发环境）

**前置条件：** 已安装 [Node.js](https://nodejs.org/)、[Rust](https://www.rust-lang.org/tools/install)、[Tauri CLI 2.x 的系统依赖](https://tauri.app/start/prerequisites/)

```bash
# 安装前端依赖
npm install

# 启动开发版本（热更新）
npm run tauri dev
```

---

## 如何打包

```bash
# 生成 Windows NSIS 安装包
npm run tauri build
```

打包产物位于 `src-tauri/target/release/bundle/nsis/`。

---

## 截图

（截图待补充）

---

## License

本项目基于 [MIT License](LICENSE) 开源。

本项目开发已完成（满足个人日常使用需求），欢迎 fork 二次开发，使用时请保留原作者信息。
