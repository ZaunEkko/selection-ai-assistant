# P0 Validation

## Manual scenarios

Manual desktop scenarios were not run in this environment because the Rust/Cargo toolchain and Windows Tauri build prerequisites are unavailable in the current shell. Keep these rows as the P0 checklist for validation on a Windows 11 machine with Rust, MSVC Build Tools, and Windows SDK installed.

| Scenario | Result | Notes |
|---|---|---|
| Chrome webpage selected English paragraph -> hover -> button -> translation/explanation | Not run | Requires `npm run tauri:dev` on a configured Windows desktop. |
| Edge webpage selected Chinese concept -> hover -> button -> explanation | Not run | Requires `npm run tauri:dev` on a configured Windows desktop. |
| VS Code selected code -> hover -> button -> code explanation | Not run | Requires `npm run tauri:dev` on a configured Windows desktop. |
| VS Code selected error -> hover -> button -> error explanation | Not run | Requires `npm run tauri:dev` on a configured Windows desktop. |
| WeChat selected text -> clipboard fallback -> button | Not run | Requires target app and clipboard fallback manual validation. |
| Feishu selected text -> clipboard fallback -> button | Not run | Requires target app and clipboard fallback manual validation. |
| Telegram selected text -> clipboard fallback -> button | Not run | Requires target app and clipboard fallback manual validation. |
| Ctrl+Alt+A opens panel for current selection or clipboard text | Not run | Global hotkey/hook implementation is represented by interfaces and needs desktop integration validation. |
| Missing API key shows settings guidance | Not run | Frontend error formatter is covered by automated tests; full Tauri command path needs desktop runtime. |
| Configured provider streams response | Not run | Requires provider config and `SELECTION_AI_API_KEY`. |
| ESC closes button/panel | Not run | Requires desktop window runtime. |
| Blacklisted app does not read or show button | Not run | Policy helpers are covered by Rust tests, but Rust tests were not executable in this shell. |

## Automated checks

| Check | Result | Notes |
|---|---|---|
| `npm test` | Pass | 5 test files / 14 tests passed. |
| `npm run build` | Pass | TypeScript and Vite production build passed. |
| `cargo test` | Blocked | `cargo: command not found` in current shell. |
| `cargo check` | Blocked | `cargo: command not found` in current shell. |
| `npx tauri info` | Blocked | Earlier run reported missing Rust/Cargo, rustup, MSVC Build Tools, and Windows SDK prerequisites. |

## Environment gaps before manual P0

- Install Rust/Cargo and ensure `cargo` is on `PATH`.
- Install Visual Studio/MSVC Build Tools and Windows SDK components required by Tauri on Windows.
- Re-run `cd selection-ai-assistant/src-tauri && cargo test && cargo check`.
- Re-run `cd selection-ai-assistant && npm run tauri:dev` and replace each `Not run` manual row with `Pass`, `Fail`, or `Blocked` plus concrete app/version notes.
