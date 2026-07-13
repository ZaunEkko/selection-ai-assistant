# Selection AI Assistant

<p align="center">
  <strong>Windows 桌面 AI 划词助手</strong><br />
  选中文本后，通过悬浮操作条快速翻译、替换、解释、总结代码或分析报错。
</p>

<p align="center">
  <a href="https://github.com/ZaunEkko/selection-ai-assistant/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ZaunEkko/selection-ai-assistant/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-blue.svg" /></a>
  <img alt="Platform: Windows 11" src="https://img.shields.io/badge/Platform-Windows%2011-0078D4" />
</p>

> 当前开发线为 **v0.3.x**。Windows 11 是正式支持的平台；macOS 和 Linux 目前只有能力探测与 backend stub，不支持完整的系统级自动划词。

## 功能

- Windows 低层鼠标监听、UI Automation 选区读取与悬浮操作条
- 翻译并替换、仅翻译、解释、总结、代码解释、报错解释和提示词扩写
- OpenAI-compatible、Anthropic Messages API、Gemini Generative Language API
- 流式响应、追问、原文窗口和翻译结果浮窗
- 截图翻译：显式热键打开取景层，手动框选后发送给视觉模型
- Provider 预设、模型加载、连接测试和环境变量 fallback
- 开机自启、后台启动、托盘驻留和关闭行为设置
- Tauri 窗口级 capability、命令调用方校验与生产 CSP

## 隐私与安全边界

- 选中文本只会在用户显式点击 AI 动作、悬浮按钮、热键或面板操作后发送给 AI provider。
- 截图内容只会在用户按下截图翻译热键并手动框选区域后读取和发送。
- 自动鼠标划词路径不使用剪贴板 fallback，避免普通点击或拖动意外触发 `Ctrl+C`。
- AI Markdown 中的远程图片默认不会加载；仅允许受限的 base64 raster 图片。普通外链仍需用户主动点击。
- 已保存的 API key 和自定义 header value 不会返回 renderer；但它们当前仍以**明文**保存在本机 settings 文件中。系统凭据存储尚未接入。
- Windows 安装包和 EXE 当前**未进行代码签名**，下载运行时可能触发 SmartScreen 提示。
- 默认禁用部分密码管理器和远程控制应用，包括 1Password、KeePassXC、Bitwarden、mstsc、AnyDesk 和 TeamViewer。

发现安全问题时，请不要创建公开 Issue，参见 [SECURITY.md](SECURITY.md)。

## 安装

从 [GitHub Releases](https://github.com/ZaunEkko/selection-ai-assistant/releases) 下载最新 Windows 构建：

| 文件 | 用途 |
|---|---|
| `Selection AI Assistant_*_x64-setup.exe` | 推荐给普通用户的 NSIS 安装器 |
| `Selection AI Assistant_*_x64_en-US.msi` | 适合 Windows Installer / 统一部署环境 |
| `selection-ai-assistant.exe` | 便携验证或开发用途 |

安装后打开设置页，选择 provider、填写 Base URL / 模型 / API key，并使用“加载模型”或“测试连接”确认配置。

也可以仅通过环境变量提供 fallback API key：

```text
SELECTION_AI_API_KEY
```

请勿将真实密钥写入仓库、Issue、日志或截图。

## 使用

### 自动划词

```text
拖拽选中文本
  → 移动到选区中心或 anchor 附近
  → 悬浮操作条出现
  → 选择替换、翻译或更多 AI 动作
```

### 截图翻译

按设置中的截图翻译热键，拖拽框选不可选中的文字区域，确认后由 OpenAI-compatible 视觉模型识别并翻译。

## 平台支持

| 平台 | 状态 | 说明 |
|---|---|---|
| Windows 11 | 支持 | 自动划词、UI Automation、截图翻译、托盘和开机自启 |
| macOS | 未完成 | 前端与 AI provider 可复用，系统级选区读取和权限接入尚未实现 |
| Linux | 未完成 | 前端与 AI provider 可复用，X11 / Wayland 系统层能力尚未实现 |

## 从源码开发

### 环境要求

- Windows 11
- Node.js `22.16.0`，npm `11.x`
- Rust `1.88.0` 或以上，MSVC toolchain
- Visual Studio Build Tools C++ workload、Windows SDK
- Microsoft Edge WebView2 Runtime

### 安装与启动

```bash
cd selection-ai-assistant
npm ci
npm run tauri:dev
```

Vite 开发端口固定为 `5173`。

### 测试与构建

```bash
cd selection-ai-assistant
npm test
npm run build
npm audit --audit-level=high
npm run tauri:build
```

```bash
cd selection-ai-assistant/src-tauri
cargo fmt --check
cargo check --locked --jobs 1
cargo test --locked --jobs 1
```

RustSec 检查需要先安装 [`cargo-audit`](https://github.com/rustsec/rustsec/tree/main/cargo-audit)：

```bash
cargo audit --no-yanked
```

CI 在 Windows 上执行前端测试、构建、npm audit、Rust fmt/check/test、Rust 1.88 MSRV 检查和 cargo audit。

## 项目结构

```text
.
├── .github/
├── CONTRIBUTING.md
├── LICENSE
├── README.md
├── SECURITY.md
└── selection-ai-assistant/
    ├── src/                 # React / TypeScript 多窗口前端
    └── src-tauri/
        ├── capabilities/    # 按窗口拆分的 Tauri capability
        ├── src/             # Rust 后端、平台实现和 commands
        └── tests/           # Rust 集成测试
```

## 参与贡献

提交代码前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。请保持改动聚焦，并为行为变化补充相应的前端或 Rust 测试。

## 已知限制

- API key 和自定义 header value 尚未迁移到 Windows Credential Manager / DPAPI。
- Windows 二进制尚未签名。
- macOS / Linux 不具备完整的系统级自动划词能力。
- 浏览器选区贴附将作为独立浏览器扩展方向继续探索。

## License

[MIT License](LICENSE)
