# Selection AI Assistant

<p align="center">
  <strong>Windows 桌面 AI 划词助手</strong><br />
  选中文本后，在选区中心附近呼出悬浮入口，快速解释、翻译、总结代码或分析报错。
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.77%2B-000000?logo=rust&logoColor=white" />
  <img alt="React" src="https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=20232A" />
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white" />
  <img alt="Windows" src="https://img.shields.io/badge/Windows-11-0078D4?logo=windows&logoColor=white" />
</p>

<p align="center">
  <a href="#快速开始">快速开始</a> ·
  <a href="#功能概览">功能概览</a> ·
  <a href="#平台支持现状">平台支持</a> ·
  <a href="#ai-provider-配置">AI Provider</a> ·
  <a href="#隐私与安全边界">隐私边界</a> ·
  <a href="#开发者指南">开发者指南</a> ·
  <a href="#roadmap">Roadmap</a>
</p>

---

## 项目状态

当前处于 **MVP 可体验版 / run-first-version** 阶段，正式支持 Windows；macOS/Linux 系统层 backend 已预留 stub，但暂未支持自动划词。

已完成：

- Tauri 2 + Rust 后端 + React 18 + TypeScript + Vite 前端骨架
- 设置窗口、悬浮按钮窗口、AI 面板窗口、原文窗口与后台托盘生命周期
- 中文设置页、悬浮按钮、AI 面板和可见操作反馈
- OpenAI-compatible、Claude Messages API、Gemini Generative Language API streaming client
- Provider 配置保存、厂商预设、模型加载、连接测试和环境变量 fallback
- AI 面板上下文事件、流式输出事件、结构化错误事件与超时兜底
- AI 面板 header 拖动、屏幕边界避让、原文窗口和最新划词上下文同步
- Windows 低层鼠标 hook MVP、热键状态机和 hover-gated 悬浮按钮
- 平台能力抽象、Windows backend 封装、macOS/Linux stub backend
- 悬浮按钮中心区域交互：进入中心区域显示、离开隐藏、再次返回可重新显示
- 保守剪贴板 fallback：仅在用户把鼠标移动到选区中心/anchor 附近后读取选中文本
- 前端单测、Rust 集成测试、TypeScript 构建、Tauri release 打包链路

仍需继续完善：

- 将 UI Automation 读取接入主选中文本读取链路
- 非文本/不可快照剪贴板格式的完整兼容
- 系统凭据存储接入
- AI 面板追问链路体验
- 正式图标、签名、发布说明和更完整的 Windows 手工验收矩阵

## 快速开始

### 1. 下载 Windows 安装包

普通用户优先从 GitHub Releases 下载 Windows 安装包：

1. 打开本项目的 **Releases** 页面。
2. 找到最新版本。
3. Windows 用户推荐下载 `Selection AI Assistant_版本号_x64-setup.exe`。
4. 如果你所在环境更偏企业/标准安装流程，也可以下载 `Selection AI Assistant_版本号_x64_en-US.msi`。
5. 双击安装包并按提示完成安装。

> 当前正式支持 Windows 11。macOS/Linux backend 已预留，但暂未支持自动划词，不建议普通用户在这些平台上作为可用版本安装。

安装产物区别：

| 产物 | 推荐人群 | 说明 |
|---|---|---|
| `*-setup.exe` | 普通 Windows 用户 | NSIS 安装器，适合双击安装。 |
| `*.msi` | 企业、系统管理员、标准化安装环境 | Windows Installer 安装包，更适合统一部署。 |
| `selection-ai-assistant.exe` | 开发者、本机验证 | release 主程序；直接发给别人不一定包含完整安装体验。 |

### 2. 配置 AI 模型厂商

首次运行后，先打开设置页配置 AI provider，否则 AI 动作无法正常生成结果。

基本步骤：

1. 打开 **设置**。
2. 在 **Provider 配置** 中选择厂商预设，或手动填写协议类型、Base URL、模型名。
3. 填入对应厂商的 API key。
4. 点击 **加载模型** 或手动确认模型名。
5. 点击 **测试连接**，确认 provider 可用。
6. 保存配置。

当前支持三类协议：

| 协议类型 | 用途 |
|---|---|
| OpenAI-compatible | OpenAI，以及智谱、DeepSeek、阿里百炼、Kimi、Minimax、SiliconFlow、AWS Bedrock、火山方舟、AgentPlan、OpenCode 等兼容入口。 |
| Claude / Anthropic | 官方 Anthropic Messages API。 |
| Gemini | 官方 Gemini Generative Language API。 |

> 当前设置页会把 provider 配置与 API key 保存到本机 settings 文件中，仍是明文存储；系统凭据存储是后续工作。请不要把真实 API key 写入代码、README、`.env.example`、提交记录、日志或其他文档。

### 3. 使用划词助手

配置好模型后，可以按下面流程使用：

```text
选中文本
  ↓
移动到选区中心 / anchor 附近
  ↓
显示悬浮 AI 按钮
  ↓
点击悬浮按钮打开 AI 面板并执行推荐动作
```

AI 面板中可以继续：

- 切换 **解释**、**翻译解释**、**总结**、**代码解释**、**报错解释** 等动作。
- 点击 **执行当前动作** 重新生成。
- 打开独立 **原文窗口** 查看完整选中文本。
- 缩小窗口后通过滚动查看完整内容。

核心隐私边界：**选中文本只应在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后发送给 AI provider。**

## 功能概览

| 功能 | 状态 |
|---|---|
| 设置窗口 | 已实现：中文 UI、provider 保存、模型加载、连通测试 |
| 后台运行 | 已实现：主窗口关闭后托盘驻留，可重新打开设置 |
| 启动 / 关闭偏好 | 已实现：可设置启动时最小化到后台；关闭按钮可询问、最小化到后台或直接退出 |
| 悬浮 AI 入口 | 已实现：划词后移动到选区中心/anchor 附近显示；离开隐藏，返回可再次显示 |
| AI 面板 | 已实现：中文 UI、header 拖动、屏幕边界避让、最新文本同步、Markdown 加粗渲染 |
| 原文窗口 | 已实现：从 AI 面板打开独立原文窗口，支持长文本滚动与最新原文恢复 |
| 解释 / 翻译解释 / 总结 / 代码解释 / 报错解释 | 已实现：动作选择与“执行当前动作”分离 |
| 流式 AI 输出 | 已实现：delta / error / done 事件、stale request 过滤和超时兜底 |
| OpenAI-compatible / Claude / Gemini provider | 已实现：配置持久化、模型加载、连通测试；模型列表不可用时可用当前模型 probe |
| 选中文本检测 | Windows MVP 已实现：mouse hook + hover-gated clipboard fallback；UI Automation 待接入主读取链路 |
| 跨平台 backend | 已预留：Windows backend 已封装，macOS/Linux 目前为 stub，不声明自动划词可用 |
| 密钥保存 | 本机 settings 文件明文保存 + `SELECTION_AI_API_KEY` fallback；系统凭据存储待接入 |
| 安装包 | 已实现：Tauri build 生成 exe、MSI 与 NSIS 安装包 |

## 平台支持现状

| 平台 | 当前状态 | 说明 |
|---|---|---|
| Windows 11 | 正式支持 / MVP 可体验 | 当前主要目标平台；自动划词入口基于 Windows 低层鼠标 hook + hover-gated clipboard fallback。 |
| macOS | backend stub 已预留 | 前端、AI provider、prompt 和面板流程可复用；后续主要补 Accessibility / Input Monitoring 权限、原生选区读取和系统层 backend。 |
| Linux | backend stub 已预留 | 前端、AI provider、prompt 和面板流程可复用；后续需要区分 X11 / Wayland，全局输入监听和其他窗口选区读取在 Wayland 下限制更强。 |

> 目前公开分发时应说明：**Windows 可用，macOS/Linux 暂未支持自动划词**。macOS/Linux 贡献者主要需要实现系统层 backend，不需要重写 AI 面板、provider、prompt 和事件流。

## AI provider 配置

运行 AI 动作前，需要在设置页配置 AI provider。API key 可以：

1. 在设置页保存到本机 settings 文件；或
2. 通过环境变量 `SELECTION_AI_API_KEY` 作为 fallback 提供。

设置页提供：

- **厂商预设**：快速填入常见服务商的 base URL、模型名和协议类型。
- **加载模型**：按当前协议调用模型列表接口加载可用模型，减少手填模型名。
- **测试连接**：保存前测试 provider 是否可连通；模型列表不可用时，会在已填写模型名的情况下使用当前模型做聊天 probe。

## 隐私与安全边界

- 不在拖拽释放瞬间读取或发送选中文本。
- 用户移动到选区中心/anchor 附近后，应用才尝试读取当前选中文本并显示悬浮入口。
- 只有用户点击悬浮按钮、AI 动作、热键或面板操作后，选中文本才应发送给 AI provider。
- 剪贴板 fallback 会短暂模拟复制选中文本；应用会尝试恢复原剪贴板，但 Windows 剪贴板历史或第三方剪贴板管理器仍可能短暂记录内容。
- 默认禁用部分敏感/远程控制应用的剪贴板 fallback，例如 1Password、KeePassXC、Bitwarden、mstsc、AnyDesk、TeamViewer。
- `.env`、`.env.*`、`temp/`、`node_modules/` 和构建输出默认不应提交。

## 开发者指南

### 技术栈

- [Tauri 2](https://tauri.app/)：Windows 桌面应用壳、窗口管理和后端 command
- [Rust](https://www.rust-lang.org/)：后端逻辑、AI streaming client、输入监听和选择模型
- [React 18](https://react.dev/)：窗口 UI
- [TypeScript](https://www.typescriptlang.org/)：前端类型、状态管理和 Tauri API wrapper
- [Vite](https://vite.dev/)：前端开发与构建
- [Vitest](https://vitest.dev/)：前端测试

### 目录结构

```text
.
├── README.md
├── CLAUDE.md
└── selection-ai-assistant/
    ├── src/
    │   ├── App.tsx
    │   ├── api/
    │   ├── components/
    │   ├── stores/
    │   ├── windows/
    │   └── __tests__/
    └── src-tauri/
        ├── icons/
        ├── src/
        │   ├── ai/
        │   ├── commands/
        │   ├── config/
        │   ├── floating_window/
        │   ├── input_monitor/
        │   ├── platform/
        │   ├── security/
        │   └── selection/
        └── tests/
```

### 开发环境要求

- Windows 11
- Node.js + npm
- Rust stable MSVC toolchain，Rust 版本 `1.77.2+`
- Visual Studio Build Tools C++ workload
- Windows SDK
- Microsoft Edge WebView2 Runtime

> Tauri Windows 构建需要 MSVC 编译链。若链接阶段失败，请先确认当前终端能访问 MSVC linker 和 Windows SDK。

### 本地开发启动

在 `selection-ai-assistant/` 目录执行：

```bash
npm install
npm run tauri:dev
```

也可以在仓库根目录执行：

```bash
npm --prefix selection-ai-assistant run tauri:dev
```

Vite 开发服务固定端口：

```text
http://localhost:5173/
```

### 本地打包

在 `selection-ai-assistant/` 目录执行：

```bash
npm run tauri:build
```

也可以在仓库根目录执行：

```bash
npm --prefix selection-ai-assistant run tauri:build
```

当前构建产物位置：

```text
selection-ai-assistant/src-tauri/target/release/selection-ai-assistant.exe
selection-ai-assistant/src-tauri/target/release/bundle/msi/Selection AI Assistant_0.1.0_x64_en-US.msi
selection-ai-assistant/src-tauri/target/release/bundle/nsis/Selection AI Assistant_0.1.0_x64-setup.exe
```

### 常用脚本

在 `selection-ai-assistant/` 目录执行：

| 命令 | 说明 |
|---|---|
| `npm run dev` | 启动 Vite 开发服务 |
| `npm run build` | TypeScript 检查并构建前端 |
| `npm test` | 运行前端测试 |
| `npm run preview` | 预览前端构建产物 |
| `npm run tauri:dev` | 启动 Tauri 开发应用 |
| `npm run tauri:build` | 构建 Tauri release 应用与安装包 |

在 `selection-ai-assistant/src-tauri/` 目录执行：

| 命令 | 说明 |
|---|---|
| `cargo fmt` | 格式化 Rust 代码 |
| `cargo check` | 检查 Rust/Tauri 后端 |
| `cargo test` | 运行 Rust 测试 |
| `cargo build` | 构建 Rust 后端 |

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
cargo fmt
cargo check
cargo test
```

最近一次本地验证记录：

| 命令 | 结果 |
|---|---|
| `cargo fmt --manifest-path selection-ai-assistant/src-tauri/Cargo.toml` | passed |
| `cargo test --manifest-path selection-ai-assistant/src-tauri/Cargo.toml --test platform_tests` | 4 passed |
| `cargo test --manifest-path selection-ai-assistant/src-tauri/Cargo.toml --jobs 1` | passed |
| `npm --prefix selection-ai-assistant test` | 8 test files / 78 tests passed |
| `npm --prefix selection-ai-assistant run build` | passed |
| `npm --prefix selection-ai-assistant run tauri:build` | passed，生成 exe、MSI 与 NSIS 安装包 |

仍建议继续做真实 Windows 桌面手工验收，重点覆盖：

- 划词后移动到选区中心附近显示悬浮按钮
- 离开中心区域隐藏悬浮按钮
- 再次回到中心区域重新显示悬浮按钮
- 点击悬浮按钮打开 AI 面板并自动执行动作
- 设置 provider、加载模型、测试连接和流式输出
- 启动时最小化到后台、托盘恢复设置窗口、关闭按钮行为选择
- 原文窗口打开、刷新、长文本滚动

## Roadmap

- [x] 接入 Windows 低层鼠标 hook MVP 和 hover-gated 悬浮按钮
- [x] 悬浮按钮支持离开中心区域隐藏、返回中心区域重新显示
- [x] 接入 provider 本机 settings 持久化、模型加载和连接测试
- [x] 支持 OpenAI-compatible、Claude、Gemini 多协议 provider
- [x] 中文化设置页与 AI 面板关键交互
- [x] 支持 AI 面板拖动、屏幕边界避让、原文窗口和生成超时兜底
- [x] 支持启动最小化后台、托盘生命周期和关闭按钮行为设置
- [x] 输出 Windows exe / MSI / NSIS 安装包
- [x] 抽象平台能力并预留 macOS/Linux stub backend
- [ ] 接入 UI Automation 到主选中文本读取链路
- [ ] 接入系统凭据存储
- [ ] 优化 AI 面板追问体验
- [ ] 实现 macOS 系统层 backend
- [ ] 调研并实现 Linux X11/Wayland 可行 backend
- [ ] 替换临时 MVP 图标并完善应用签名
- [ ] 完成发布说明与完整手工验收记录

## License

当前尚未添加许可证文件。正式公开分发前需要补充 `LICENSE`。
