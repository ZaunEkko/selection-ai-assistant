# Contributing

感谢你参与 Selection AI Assistant。

## Before you start

- 普通缺陷和功能建议请先查看现有 Issues。
- 安全问题不要公开提交，使用 [SECURITY.md](SECURITY.md) 中的私密报告入口。
- 保持 PR 聚焦，不要把无关重构、依赖大版本迁移和功能改动混在一起。

## Development environment

当前主要开发和验收平台为 Windows 11。

- Node.js `22.16.0`
- npm `11.x`
- Rust `1.88.0` 或以上，MSVC toolchain
- Visual Studio Build Tools C++ workload 与 Windows SDK
- Microsoft Edge WebView2 Runtime

安装依赖：

```bash
cd selection-ai-assistant
npm ci
```

启动桌面应用：

```bash
npm run tauri:dev
```

## Tests

前端：

```bash
cd selection-ai-assistant
npm test
npm run build
npm audit --audit-level=high
```

Rust / Tauri：

```bash
cd selection-ai-assistant/src-tauri
cargo fmt --check
cargo check --locked --jobs 1
cargo test --locked --jobs 1
```

当前没有 `lint` 脚本，请不要在 PR 中声称运行了不存在的 lint 命令。

行为变化应至少覆盖相关测试：

- React / TypeScript 改动：更新 `selection-ai-assistant/src/__tests__/`
- Rust 逻辑改动：更新 `selection-ai-assistant/src-tauri/tests/`
- Tauri 配置、CSP 或 capability 改动：补充静态配置测试，并验证真实 Tauri WebView

## Privacy and secret handling

- 不要提交 API key、token、Cookie、私有 provider header、真实用户配置或未脱敏日志。
- `.env`、`.env.*` 和 `temp/` 不应进入 Git；只有 `.env.example` 可以提交。
- 选中文本和截图只能在用户显式触发后发送给 provider。
- 自动鼠标划词路径不要重新引入剪贴板 fallback。
- 测试本机 settings 前必须备份；验证完成后逐字节恢复，并删除临时敏感材料。

## Branches and pull requests

仓库使用以下分支约定：

- `main`：发布线
- `dev`：集成线
- `feature/*`：功能与普通修复
- `release/*`：发布准备
- `hotfix/*`：已发布版本的紧急修复

通常向 `dev` 提交功能 PR，再通过 release PR 合入 `main`。请在 PR 中说明：

- 改动目的和用户影响
- 安全或隐私边界变化
- 已运行的精确测试命令及结果
- 真实桌面验收是否执行；未执行时说明原因
- UI 变化的截图（不得包含秘密或私人内容）

## Code style

- 遵循现有 TypeScript、React 和 Rust 命名与结构。
- 优先修改现有模块，避免无必要的抽象和大范围重构。
- Rust 提交前运行 `cargo fmt`。
- 不要把内部工作记录、代理提示词或个人验证材料加入仓库。

## License

提交贡献即表示你同意按仓库的 [MIT License](LICENSE) 提供相应代码和文档。
