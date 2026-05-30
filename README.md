# Selection AI Assistant

<div align="center">

**Windows 桌面 AI 划词助手，让选中文本变成即时解释、翻译、总结和排错入口。**

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=for-the-badge&logo=react&logoColor=20232A)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)

</div>

---

## Vision

Selection AI Assistant 目标是做一个轻量、可控、隐私边界清晰的 Windows AI 划词助手：

- 选中文本后，在合适位置显示悬浮 AI 入口。
- 用户明确点击动作后，再把文本发送给 AI provider。
- 支持解释、翻译解释、总结、代码解释和报错解释等常见场景。
- 使用 OpenAI-compatible Chat Completions，方便接入 OpenRouter、OpenAI 或兼容供应商。

当前仓库处于 **run-first-version** 阶段：前端、Rust/Tauri 编译链、测试和 Tauri dev 启动已跑通；系统级全局 hook、真实 UI Automation 读取和剪贴板读写仍是后续实现重点。

## What works now

| 能力 | 状态 |
|---|---|
| React/Vite 设置窗口 | 已实现 |
| Tauri 多窗口配置 | 已实现：`main`、`floating-button`、`ai-panel` |
| AI 面板事件流 | 已实现：`panel_context`、`ai_stream_delta`、`ai_stream_done` |
| OpenAI-compatible streaming client | 已实现基础版本 |
| Provider 配置表单 | 已实现内存配置；运行时 API key 读取 `SELECTION_AI_API_KEY` |
| 选择状态机 / UIA / 剪贴板 fallback 模型 | 已有模型与测试 |
| Windows 全局鼠标键盘 hook | 未完整实现 |
| 真实 UI Automation 读取 | 未完整实现 |
| 真实剪贴板读写 fallback | 未完整实现 |

## Architecture

```text
selection-ai-assistant/
├─ src/                      # React + TypeScript frontend
│  ├─ App.tsx                # 根据 Tauri window label 路由窗口
│  ├─ api/tauri.ts           # Tauri invoke wrapper
│  ├─ windows/               # Settings / FloatingButton / AiPanel
│  ├─ stores/panelStore.ts   # AI panel streaming state
│  └─ __tests__/             # Vitest frontend tests
└─ src-tauri/                # Rust + Tauri backend
   ├─ src/commands/          # Tauri commands
   ├─ src/ai/                # action classifier + OpenAI-compatible client
   ├─ src/config/            # AppConfig / provider config
   ├─ src/selection/         # selection state machine and fallback models
   ├─ src/floating_window/   # window placement helpers
   └─ tests/                 # Rust integration tests
```

### Frontend flow

```text
Tauri window label
  ├─ main            -> Settings
  ├─ floating-button -> FloatingButton
  └─ ai-panel        -> AiPanel
```

`AiPanel` 通过后端事件接收上下文和流式输出，并用 `requestId` 过滤旧请求，避免 stale delta/done 覆盖新结果。

### Backend flow

```text
run_ai_action
  -> validate selected text / request id
  -> load provider config
  -> read SELECTION_AI_API_KEY
  -> build prompt
  -> stream OpenAI-compatible response
  -> emit ai_stream_delta / ai_stream_done
```

## Quick start

### Prerequisites

- Windows 11
- Node.js + npm
- Rust stable MSVC toolchain
- Visual Studio Build Tools C++ workload + Windows SDK
- Microsoft Edge WebView2 Runtime

当前本机验证过的 Build Tools 路径：

```text
D:\env\Microsoft\VisualStudio\2022\BuildTools
```

> 这只是当前开发机路径，不是项目硬编码要求。Windows SDK 仍按 Microsoft 标准方式安装到 `C:\Program Files (x86)\Windows Kits\10`。

### Install dependencies

```bash
cd selection-ai-assistant
npm install
```

### Run frontend only

```bash
npm run dev
```

Vite 固定端口：

```text
http://localhost:5173/
```

### Run Tauri desktop app

普通 shell 如果没有加载 MSVC 环境，先进入 Visual Studio Build Tools 环境，再运行：

```cmd
call D:\env\Microsoft\VisualStudio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat
cd /d D:\project\coding\project\2026_5\word-selector\selection-ai-assistant
npm run tauri:dev
```

## Validation

前端：

```bash
cd selection-ai-assistant
npm test
npm run build
```

Rust/Tauri：

```cmd
call D:\env\Microsoft\VisualStudio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat
cd /d D:\project\coding\project\2026_5\word-selector\selection-ai-assistant\src-tauri
cargo check
cargo test
```

最近一次本地验证结果：

| 命令 | 结果 |
|---|---|
| `npm test` | 5 files / 17 tests passed |
| `npm run build` | passed |
| `cargo check` | passed |
| `cargo test` | passed |
| `npm run tauri:dev` | launched `target\debug\selection-ai-assistant.exe` |

## AI provider notes

Provider 表单当前保存的是配置和未来安全存储引用；运行 AI 请求时，后端实际读取：

```text
SELECTION_AI_API_KEY
```

不要把 API key 写入代码、README、`.env.example` 或提交记录。

## Privacy boundary

本项目的核心隐私约束是：**选中文本只应在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后发送给 AI provider。**

当前仓库不会把 `temp/`、`.env`、`.env.*` 纳入 Git；这些位置只用于本地临时材料和私密配置。

## Roadmap

- [ ] 接入真实 Windows 全局鼠标/键盘监听。
- [ ] 接入真实 UI Automation 选中文本读取。
- [ ] 实现安全的剪贴板 fallback 和恢复逻辑。
- [ ] 接入系统凭据存储，替代运行时只读环境变量的密钥方案。
- [ ] 优化 AI 面板追问体验。
- [ ] 替换临时 MVP 图标为正式图标。

## License

当前尚未添加许可证文件。正式公开分发前需要补充 `LICENSE`。
