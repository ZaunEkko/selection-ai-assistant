# CLAUDE.md

本文件用于为 Claude Code（claude.ai/code）在本仓库中工作时提供指导。

## 项目上下文

本仓库实现 Windows 桌面 AI 划词助手（划词器）。主应用位于 `selection-ai-assistant/`，技术栈为 Tauri 2 + Rust 后端 + React 18 + TypeScript + Vite 前端。

默认用中文回复；更新本文件或新增项目说明时也优先使用中文，除非用户明确要求其他语言。

## 关键边界

- 永远不要远程推送，除非用户明确说“推送到远程仓库”。包括 `git push`、force push、推送分支、推送 tag 或任何等价远程写入动作。
- 分支工作流遵循 `main` / `dev` / `feature/*` / `release/*` / `hotfix/*`；`main` 和 `dev` 合并走 PR，不直接 push。
- 选中文本只应在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后发送给 AI provider。
- `temp/` 是本地临时/敏感工作区，已被 `.gitignore` 忽略；不要提交其中的设计文档、过程记录或敏感材料。
- `.env`、`.env.*` 被忽略，`.env.example` 可以提交；不要把 API key、token、私有 provider 配置写入代码、文档或提交记录。

## 常用命令

前端/整体 Tauri 命令在 `selection-ai-assistant/` 目录执行：

```bash
npm install
npm run dev
npm run build
npm test
npm test -- src/__tests__/aiPanelEvents.test.tsx
npm run preview
npm run tauri -- <tauri-cli-args>
npm run tauri:dev
npm run tauri:build
```

- `npm run dev` 启动 Vite，端口固定为 `5173`。
- `npm run build` 执行 `tsc && vite build`。
- `npm test` 执行 `vitest run`。
- 当前没有 `lint` 脚本；不要声称存在 lint 命令。

Rust/Tauri 后端命令在 `selection-ai-assistant/src-tauri/` 目录执行：

```bash
cargo fmt
cargo check
cargo test
cargo test --test openai_client_tests
cargo test validates_provider_headers
cargo build
cargo run
```

- crate 要求 Rust `1.77.2` 或以上，默认运行目标为 `selection-ai-assistant`。
- Windows 上运行 Rust/Tauri 构建或测试需要 Rust MSVC toolchain 以及 Windows SDK / MSVC linker 环境；若链接阶段失败，要区分环境问题和代码问题。

## 架构地图

### 前端

- 入口：`selection-ai-assistant/src/main.tsx` 渲染 `App`。
- 窗口路由：`selection-ai-assistant/src/App.tsx` 按 Tauri window label 渲染：
  - `main` 或其他默认窗口：`Settings`
  - `floating-button`：`FloatingButton`
  - `ai-panel`：`AiPanel`
- Tauri API wrapper：`selection-ai-assistant/src/api/tauri.ts`，集中封装 config、AI action、窗口显示/隐藏和错误格式化。
- AI 面板状态：`selection-ai-assistant/src/stores/panelStore.ts`，用 `requestId` 过滤 stale delta/done，避免旧请求覆盖新请求。
- `AiPanel` 监听后端事件：`panel_context`、`ai_stream_delta`、`ai_stream_done`。
- `Settings` 负责 provider 配置；当前 `apiKeyRef` 只是未来安全存储引用，运行时 AI 请求实际读取 `SELECTION_AI_API_KEY`。

### Tauri/Rust 后端

- 入口：`selection-ai-assistant/src-tauri/src/lib.rs` 注册 `AppState::new(AppConfig::default())` 和 Tauri commands。
- `commands/`：Tauri command 层。
  - `config.rs` 读写内存中的 `AppConfig`。
  - `panel.rs` 显示/隐藏/定位 `floating-button` 和 `ai-panel`。
  - `selection.rs` 根据手动文本生成 `panel_context` 并 emit 给前端。
  - `ai.rs` 构造 prompt、读取 provider 和 `SELECTION_AI_API_KEY`，启动 OpenAI-compatible stream 并 emit `ai_stream_delta` / `ai_stream_done`。
- `config/`：`AiProviderConfig`、`AppConfig` 结构和默认值。
- `ai/`：动作分类与 OpenAI-compatible Chat Completions streaming client。
- `selection/`：选择状态机、选择候选、UI Automation 结果模型、剪贴板 fallback 策略。
- `input_monitor/`：输入事件和拖拽距离判断模型；当前不是完整 OS hook 实现。
- `floating_window/`：根据 anchor point 和屏幕边界计算窗口位置。
- `security/`：密钥存储 trait 和内存实现；当前 AI command 尚未接入系统凭据存储。
- `types.rs`：共享 `Point`、`Rect`、`PublicError` 等类型。

`selection-ai-assistant/src-tauri/tauri.conf.json` 定义三个窗口：`main`、`floating-button`、`ai-panel`。

不要把当前骨架误判成完整系统级实现：现有代码包含选择状态机、UIA/剪贴板策略模型和测试，但尚未看到真正的 Windows 全局鼠标/键盘 hook、UI Automation API 调用或剪贴板读写实现。

## 测试与完成标准

- 前端测试在 `selection-ai-assistant/src/__tests__/`，覆盖窗口路由、AI 面板事件流、stale request 过滤、panel reducer、Settings/provider 表单和错误格式化。
- Rust 集成测试在 `selection-ai-assistant/src-tauri/tests/`，覆盖配置、动作分类、prompt、OpenAI-compatible client/SSE、选择状态机、剪贴板 fallback、UIA 模型、窗口定位和输入距离判断。
- 修改 TypeScript/React 代码后，至少运行相关单测；跨前端入口或共享状态时运行 `npm test`。
- 修改 Rust 逻辑后，至少运行相关 `cargo test ...`；环境未配置 MSVC/Windows SDK 时，如实说明未运行或失败原因。

## 参考文件

如后续新增 `README.md`、`.cursor/rules/`、`.cursorrules`、`.github/copilot-instructions.md` 等项目指导文件，先读取并合并其中仍有效的规则。
