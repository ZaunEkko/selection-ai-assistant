---
name: verify
description: 在真实 Windows Tauri 窗口中验证桌面交互并捕获截图与原生窗口状态
---

# 桌面交互验证

1. 在 `selection-ai-assistant/` 运行 `npm run tauri:dev`。若 debug exe 被占用，先确认并结束旧的 `selection-ai-assistant.exe` 开发实例，再重启。
2. 必须操作真实 Tauri 窗口，不用 Vitest、普通浏览器页面或直接调用内部函数代替。
3. Windows 鼠标流程使用 `SendInput` 或 `mouse_event` 生成低级鼠标事件；`SetCursorPos` 只能移动光标，不能验证项目的 `WH_MOUSE_LL` 路径。
4. 使用 `EnumWindows`、`IsWindowVisible` 和 `GetWindowRect` 记录目标窗口状态，并用 Pillow `ImageGrab` 保存关键步骤截图。
5. 注意 Windows DPI 虚拟化：验证联合窗口区域时，以应用低级 hook 和 Tauri `outer_position` / `outer_size` 使用的物理坐标为准。
6. 至少覆盖正常路径和一个邻近探测，例如延迟关闭前快速返回目标窗口。
