# Selection AI Assistant

<p align="center">
  <strong>Windows 桌面 AI 划词助手</strong><br />
  选中文本后，通过悬浮入口快速解释、翻译、总结代码或分析报错。
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.77%2B-000000?logo=rust&logoColor=white" />
  <img alt="React" src="https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=20232A" />
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white" />
</p>

---

## 项目状态

当前处于 **run-first-version / MVP 首版链路打磨** 阶段。

已完成：

- Tauri 2 + Rust 后端 + React 18 + TypeScript + Vite 前端骨架
- 设置窗口、悬浮按钮窗口、AI 面板窗口与后台托盘生命周期
- 中文设置页、悬浮按钮、AI 面板和可见操作反馈
- AI 面板上下文事件、流式输出事件、结构化错误事件与超时兜底
- AI 面板 header 拖动、屏幕边界自动避让和最新划词上下文同步
- OpenAI-compatible Chat Completions streaming client
- Provider 配置表单、本机 settings 文件持久化、模型加载和连通测试
- Windows 低层鼠标 hook MVP、热键状态机和 hover-gated 悬浮按钮
- 保守剪贴板 fallback：只在鼠标移动到选区中心/anchor 附近并停留后读取，不在拖拽释放时立即弹窗或读取
- 前端单测、Rust 集成测试、TypeScript 构建与 Rust 编译检查

尚未完整实现：

- 真实 UI Automation 选中文本读取
- 非文本/不可快照剪贴板格式的完整兼容
- 系统凭据存储接入
- AI 面板追问链路
- 真实 Windows 桌面手工验收、安装包与发布流程

## 功能概览

| 功能 | 状态 |
|---|---|
| 设置窗口 | 已实现：中文 UI、provider 保存、模型加载、连通测试 |
| 后台运行 | 已实现：主窗口关闭后托盘驻留，可重新打开设置 |
| 悬浮 AI 入口 | 已实现：选区后移动到中心/anchor 附近并停留后才显示 |
| AI 面板 | 已实现：中文 UI、header 拖动、屏幕边界避让、最新文本同步 |
| 解释 / 翻译解释 / 总结 / 代码解释 / 报错解释 | 已实现：动作选择与“执行当前动作”分离 |
| 流式 AI 输出 | 已实现：delta / error / done 事件、stale request 过滤和超时兜底 |
| OpenAI-compatible provider | 已实现：配置持久化、模型加载、连通测试 |
| 选中文本检测 | MVP 已实现：Windows mouse hook + hover-gated clipboard fallback；UI Automation 待接入 |
| 密钥保存 | 本机 settings 文件明文保存 + `SELECTION_AI_API_KEY` fallback；系统凭据存储待接入 |

## 技术栈

- [Tauri 2](https://tauri.app/)：Windows 桌面应用壳与后端 command
- [Rust](https://www.rust-lang.org/)：后端逻辑、AI streaming client、选择模型
- [React 18](https://react.dev/)：窗口 UI
- [TypeScript](https://www.typescriptlang.org/)：前端类型与状态管理
- [Vite](https://vite.dev/)：前端开发与构建
- [Vitest](https://vitest.dev/)：前端测试

## 目录结构

```text
.
├── README.md
├── CLAUDE.md
└── selection-ai-assistant/
    ├── src/
    │   ├── App.tsx
    │   ├── api/
    │   ├── stores/
    │   ├── windows/
    │   └── __tests__/
    └── src-tauri/
        ├── src/
        │   ├── ai/
        │   ├── commands/
        │   ├── config/
        │   ├── floating_window/
        │   ├── input_monitor/
        │   ├── security/
        │   └── selection/
        └── tests/
```

## 快速开始

### 环境要求

- Windows 11
- Node.js + npm
- Rust stable MSVC toolchain，Rust 版本 `1.77.2+`
- Visual Studio Build Tools C++ workload
- Windows SDK
- Microsoft Edge WebView2 Runtime

> Tauri Windows 构建需要 MSVC 编译链。普通终端没有加载 MSVC 环境时，请先进入 Visual Studio Developer Command Prompt，或手动调用 `vcvars64.bat`。

### 安装依赖

```bash
cd selection-ai-assistant
npm install
```

### 启动前端开发服务

```bash
npm run dev
```

Vite 开发服务固定端口：

```text
http://localhost:5173/
```

### 启动 Tauri 桌面应用

```bash
npm run tauri:dev
```

如果当前 shell 没有 MSVC 环境，请先在 Windows cmd 中执行类似命令：

```cmd
call <Visual Studio Build Tools>\VC\Auxiliary\Build\vcvars64.bat
cd /d <repo>\selection-ai-assistant
npm run tauri:dev
```

## 常用脚本

在 `selection-ai-assistant/` 目录执行：

| 命令 | 说明 |
|---|---|
| `npm run dev` | 启动 Vite 开发服务 |
| `npm run build` | TypeScript 检查并构建前端 |
| `npm test` | 运行前端测试 |
| `npm run preview` | 预览前端构建产物 |
| `npm run tauri:dev` | 启动 Tauri 开发应用 |
| `npm run tauri:build` | 构建 Tauri 应用 |

在 `selection-ai-assistant/src-tauri/` 目录执行：

| 命令 | 说明 |
|---|---|
| `cargo fmt` | 格式化 Rust 代码 |
| `cargo check` | 检查 Rust/Tauri 后端 |
| `cargo test` | 运行 Rust 测试 |
| `cargo build` | 构建 Rust 后端 |

## AI provider

运行 AI 动作前需要在设置页配置 OpenAI-compatible provider；API key 可以保存到设置页，也可以通过环境变量 `SELECTION_AI_API_KEY` 作为 fallback 提供。当前设置页会把 provider 配置与 API key 保存到本机 settings 文件（明文，位于系统本地应用配置目录）中。

设置页还提供：

- **加载模型**：调用 OpenAI-compatible `/models` 接口加载可用模型，减少手填模型名。
- **测试连接**：保存前测试 provider 是否可连通。

请不要把真实 API key 写入代码、README、`.env.example`、提交记录、日志或其他文档。

## 隐私边界

核心规则：**只有在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后，选中文本才应发送给 AI provider。**

仓库默认忽略：

- `temp/`
- `.env`
- `.env.*`
- `node_modules/`
- 构建输出目录

## 测试与验证

前端：

```bash
cd selection-ai-assistant
npm test
npm run build
```

Rust/Tauri：

```bash
cd selection-ai-assistant/src-tauri
cargo check
cargo test
```

最近一次本地验证结果：

| 命令 | 结果 |
|---|---|
| `npm test` | 39 tests passed |
| `npm run build` | passed |
| `cargo fmt -- --check` | passed |
| `CARGO_TARGET_DIR=target/claude-test cargo check` | passed |
| `CARGO_TARGET_DIR=target/claude-test cargo test` | 86 tests passed |

> 已完成自动化验证；仍建议继续做真实 Windows 桌面手工验收，实际验证“划词 → 移动到选区中心附近并停留 → 悬浮按钮 → 面板拖动/定位 → AI provider”的完整体验。

## Roadmap

- [x] 接入 Windows 低层鼠标 hook MVP 和 hover-gated 悬浮按钮
- [x] 接入 provider 本机 settings 持久化、模型加载和连接测试
- [x] 中文化设置页与 AI 面板关键交互
- [x] 支持 AI 面板拖动、屏幕边界避让和生成超时兜底
- [ ] 接入真实 UI Automation 选中文本读取
- [ ] 接入系统凭据存储
- [ ] 优化 AI 面板追问体验
- [ ] 替换临时 MVP 图标
- [ ] 完成真实 Windows 桌面手工验收与发布说明

## License

当前尚未添加许可证文件。正式公开分发前需要补充 `LICENSE`。
