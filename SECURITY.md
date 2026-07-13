# Security Policy

## Supported versions

安全修复优先进入最新开发线和最新发布版本。旧版本可能不会单独回补。

| Version | Supported |
|---|---|
| Latest release / `main` | Yes |
| Older releases | Best effort |

## Reporting a vulnerability

请通过 GitHub 的私密安全报告入口提交：

- [Report a vulnerability](https://github.com/ZaunEkko/selection-ai-assistant/security/advisories/new)

请不要在公开 Issue、Discussion、PR 或社交平台披露尚未修复的漏洞。

报告中建议包含：

- 受影响版本、提交或安装包
- 可复现步骤和预期影响
- 涉及的窗口、Tauri command、provider 或平台
- 已脱敏的日志、截图或 PoC
- 你建议的修复方向（如有）

请勿附带真实 API key、token、Cookie、自定义 provider header value 或其他第三方凭据。维护者会尽力确认报告、评估影响并协调修复与披露时间，但当前项目不承诺固定响应 SLA。

## Current security boundaries

- 选中文本和截图只应在用户显式触发相应操作后发送给 AI provider。
- AI Markdown 默认阻止远程图片自动加载。
- Tauri renderer 按窗口使用最小 capability，敏感 command 还会验证调用窗口 label。
- 生产 WebView 使用 CSP；开发环境仅额外开放固定的 Vite `localhost:5173` 来源。
- 已保存 API key 和自定义 header value 不返回 renderer。

## Known limitations

- API key、自定义 provider header value 和 provider 配置当前仍以明文保存在本机 settings 文件；尚未接入系统凭据存储。
- Windows EXE、MSI 和 NSIS 安装器当前未进行代码签名。
- macOS / Linux backend 尚未实现完整系统级划词能力，不应视为受支持的安全边界。
- 用户选择的第三方 AI provider 会接收其显式提交的文本或截图；第三方的日志、保留和隐私政策不由本项目控制。

## Scope

欢迎报告以下问题：

- 未经显式操作发送文本、截图或秘密
- renderer 泄露已保存的 API key / header value
- Tauri capability、command caller 校验或 CSP 绕过
- 剪贴板、窗口焦点、截图范围或敏感应用禁用逻辑错误
- Provider 错误信息中的秘密泄露
- 安装包、更新或依赖供应链风险

普通功能缺陷和非敏感崩溃请使用 Issue 模板。
