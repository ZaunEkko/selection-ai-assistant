# AGENTS.md

本文件用于为 Claude 之外的代码代理在本仓库中工作时提供指导；规则效果应与 `CLAUDE.md` 保持一致，只是面向其他 agent。

## 项目上下文

本仓库实现 Windows 桌面 AI 划词助手（划词器）。主应用位于 `selection-ai-assistant/`，技术栈为 Tauri 2 + Rust 后端 + React 18 + TypeScript + Vite 前端。

默认用中文回复；更新本文件或新增项目说明时也优先使用中文，除非用户明确要求其他语言。

当前版本线为 v0.3.x：Windows 自动划词、截图翻译、开机自启、后台启动、AI provider 配置和多窗口 UI 已可体验；macOS/Linux 系统层 backend 仍为 stub。

## 关键边界

- 永远不要远程推送，除非用户明确说“推送到远程仓库”。包括 `git push`、force push、推送分支、推送 tag 或任何等价远程写入动作。
- 用户说“提交本地”时，默认走当前代理所在环境的代提交流程：优先使用 `/commit-commands:commit` 或等价提交流程生成提交信息并附带对应 attribution；不要裸用 `git commit -m`。
- 分支工作流遵循 `main` / `dev` / `feature/*` / `release/*` / `hotfix/*`；`main` 和 `dev` 合并走 PR，不直接 push。
- 选中文本只应在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后发送给 AI provider。
- 截图翻译必须由用户显式按快捷键并手动框选区域后才读取屏幕内容；不要做自动截图或后台截图。
- 自动鼠标划词路径不要恢复剪贴板 fallback；该路径不应因为普通点击、拖窗口或不可选中文本而模拟 `Ctrl+C`。
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
  - `replacement-preset`：`ReplacementPresetPanel`
  - `ai-panel`：`AiPanel`
  - `source-text`：`SourceTextWindow`
  - `translate-result`：`TranslateResult`
  - `screenshot-overlay`：`ScreenshotOverlay`
- Tauri API wrapper：`selection-ai-assistant/src/api/tauri.ts`，集中封装 config、AI action、窗口显示/隐藏、截图翻译和错误格式化。
- AI 面板状态：`selection-ai-assistant/src/stores/panelStore.ts`，用 `requestId` 过滤 stale delta/done，避免旧请求覆盖新请求。
- `AiPanel` 监听后端事件：`panel_context`、`ai_stream_delta`、`ai_stream_done`。
- `ScreenshotOverlay` 是截图翻译取景层；Tauri 窗口 hide/show 不会卸载 React 组件，连续截图时必须显式清空拖拽状态。
- `Settings` 负责 provider、截图翻译快捷键、开机自启、后台启动和关闭按钮行为配置；provider 配置和 API key 当前保存到本机明文 settings 文件。运行时 AI 请求优先使用已保存的 `provider.apiKey`，未保存时回退读取 `SELECTION_AI_API_KEY`；系统凭据存储仍是未来工作。

### Tauri/Rust 后端

- 入口：`selection-ai-assistant/src-tauri/src/lib.rs` 注册通过 `AppState::load_or_default()` 初始化的应用状态、Tauri commands、托盘、窗口生命周期和 autostart plugin。
- `commands/`：Tauri command 层。
  - `config.rs` 读取内存中的 `AppConfig`；保存 provider 或行为配置时更新内存，并在可用时持久化到本机 settings 文件；开机自启设置会同步 OS autostart。
  - `panel.rs` 显示/隐藏/定位 `floating-button`、`ai-panel`、`source-text`、`translate-result` 等辅助窗口。
  - `selection.rs` 根据手动文本或当前选区生成 `panel_context` 并 emit 给前端。
  - `ai.rs` 构造 prompt，按已保存 `provider.apiKey` 优先且 `SELECTION_AI_API_KEY` fallback 读取密钥，启动 provider stream 并 emit `ai_stream_delta` / `ai_stream_done`。
  - `screenshot.rs` 显示截图取景层、把 CSS 选区坐标换算为物理屏幕坐标、截图并发起 OpenAI-compatible 视觉模型流式请求。
- `config/`：`AiProviderConfig`、`AppConfig`、开机自启、后台启动、关闭按钮行为和替换语言预设等结构与默认值。
- `ai/`：动作分类与 OpenAI-compatible / Claude / Gemini streaming client；OpenAI-compatible 还支持截图翻译用的 vision chat request。
- `selection/`：选择状态机、选择候选、UI Automation 结果模型、剪贴板 fallback 策略。
- `input_monitor/`：输入事件、拖拽距离、hover 判断、浏览器滚动跟随策略和选区几何匹配模型。
- `platform/windows.rs`：Windows 低层鼠标 hook、热键监听、前台窗口识别、UI Automation 选区读取、视觉高亮定位、剪贴板读写与 GDI 截屏实现。
- `floating_window/`：根据 anchor point、选区 rect 和屏幕边界计算窗口位置。
- `security/`：密钥存储 trait 和内存实现；当前 AI command 尚未接入系统凭据存储。
- `types.rs`：共享 `Point`、`Rect`、`PublicError` 等类型。

`selection-ai-assistant/src-tauri/tauri.conf.json` 定义当前窗口：`main`、`floating-button`、`replacement-preset`、`ai-panel`、`source-text`、`translate-result`、`screenshot-overlay`。

## 当前行为要点

- 自动鼠标划词：鼠标拖拽释放后，后端尝试通过 UI Automation 读取选中文本，并用 UIA rect 或本次拖拽产生的视觉高亮选区确认几何关系；确认后才显示悬浮入口。
- 自动路径禁用剪贴板 fallback：不要在自动鼠标划词失败时模拟 `Ctrl+C` 兜底。
- 浏览器划词：Chrome/Edge 等浏览器可能能读到 UIA 文本但 rect 不稳定，当前允许用视觉高亮选区作为几何证明。
- 截图翻译：快捷键触发 `screenshot-overlay`，用户框选区域后隐藏 overlay/旧翻译窗，等待短暂 settle，再用物理屏幕坐标截图。
- 开机自启：`launch_at_startup` 控制 OS 登录自启；`start_minimized_to_tray` 控制启动后是否显示主设置窗口，两者相互独立。

## 测试与完成标准

- 前端测试在 `selection-ai-assistant/src/__tests__/`，覆盖窗口路由、AI 面板事件流、stale request 过滤、panel reducer、Settings/provider 表单、截图 overlay 和错误格式化。
- Rust 集成测试在 `selection-ai-assistant/src-tauri/tests/`，覆盖配置、动作分类、prompt、OpenAI-compatible client/SSE、vision request、选择状态机、剪贴板 fallback、UIA 模型、窗口定位、输入距离判断、开机自启配置和 capabilities。
- 修改 TypeScript/React 代码后，至少运行相关单测；跨前端入口或共享状态时运行 `npm test`。
- 修改 Rust 逻辑后，至少运行相关 `cargo test ...`；环境未配置 MSVC/Windows SDK 时，如实说明未运行或失败原因。
- 修改 release 版本时，同步检查 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json`。

## 参考文件

如后续新增 `README.md`、`CLAUDE.md`、`.cursor/rules/`、`.cursorrules`、`.github/copilot-instructions.md` 等项目指导文件，先读取并合并其中仍有效的规则。
