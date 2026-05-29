# Windows Selection Spike

## Apps tested

| App | UIA text | UIA rect | Clipboard text | Notes |
|---|---:|---:|---:|---|
| Chrome | No | No | No | Not tested yet |
| Edge | No | No | No | Not tested yet |
| VS Code | No | No | No | Not tested yet |
| WeChat | No | No | No | Not tested yet |
| Feishu | No | No | No | Not tested yet |
| Telegram | No | No | No | Not tested yet |

## Decision

Use clipboard fallback as the reliable P0 path. Use UIA as opportunistic rect/text probe.
