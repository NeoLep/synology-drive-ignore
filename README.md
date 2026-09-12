# Synology Drive Ignore

中文 | [English](#english)

一个适用于 macOS 和 Windows 的 Synology Drive 忽略文件夹规则管理器，使用 Tauri、Vue 和 Rust 构建。

## 功能

- 自动检测当前操作系统与 Synology Drive 配置文件
- 读取并展示已经配置的忽略文件夹
- 添加、删除文件夹名并安全写回原配置
- Windows 支持识别多个同步任务
- macOS 同时维护 `blacklist.filter` 与 `filter-v4150`
- 支持共享 JSON 配置，在多台设备间同步同一份忽略规则
- 使用文件 watcher 实时检测共享配置变化，并提供可配置的兜底轮询间隔
- 支持托盘运行，关闭窗口后仍可在后台检测文件更新并自动写入配置
- 支持中文 / English 界面切换

## 多设备共享

在“共享配置”区域可以选择已有的 `.json` 文件，或用当前规则创建一份新文件。建议把文件保存在 Synology Drive 已同步的文件夹内。

连接共享配置后，新增忽略文件夹时可以选择规则来源：

- **本机规则**仅保存在当前设备。
- **云规则**会写入共享配置文件，并同步给使用该文件的其他设备。
- 已有的本机规则可点击“提升为云规则”。新增、删除和提升操作均需点击“保存更改”后提交。

共享配置开启后：

- 应用会监听共享配置文件变化，更新后自动写入本机 Synology Drive 配置。
- 当写入发生时，应用会显示“已更新禁用同步文件夹配置”的提示。
- 默认每 60 分钟进行一次兜底轮询，可在应用内修改，范围为 1 到 1440 分钟。
- 最终配置始终是 `本机规则 ∪ 云规则`，相同名称自动去重且云规则优先标记。
- 应用会记住共享文件路径；断开连接不会删除共享文件，当前已经写入的规则也会保留在本机。

共享文件采用易于查看和手动编辑的格式：

```json
{
  "version": 1,
  "ignoredFolders": ["node_modules", ".git", "dist"]
}
```

## 配置位置

- macOS：`~/Library/Application Support/SynologyDrive/SynologyDrive.app/Contents/Resources/conf`
- Windows：`%LOCALAPPDATA%\SynologyDrive\data\session\*\conf`

建议修改前退出 Synology Drive，保存后重新启动客户端。Synology Drive 更新后可能重置配置，需要再次保存。

### macOS 权限

macOS 会保护其他应用的资源文件。首次保存遇到权限提示时，请打开：

`系统设置 → 隐私与安全性 → App 管理`

- 安装后的应用：允许 **Synology Drive Ignore** 修改其他应用。
- 使用 `pnpm tauri dev`：允许启动开发服务的 Terminal、iTerm 或 Codex。

授权后需要完全退出并重新打开应用。

## 开发

```sh
pnpm install
pnpm tauri dev
```

运行检查：

```sh
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

在配置好 LLVM、`cargo-xwin`、Windows Rust target 和 NSIS 的 macOS/Linux 主机上，可交叉构建 Windows x64 安装包：

```sh
pnpm build:windows
```

## English

A macOS and Windows manager for Synology Drive ignored-folder rules, built with Tauri, Vue, and Rust.

For a fuller guide, see [docs/USER_GUIDE.md](/Users/leep/Desktop/Workspace2/synology-drive-ignore/docs/USER_GUIDE.md).

## Features

- Detects the current operating system and Synology Drive configuration files automatically
- Reads and displays existing ignored folders
- Adds or removes folder names and writes the original configuration files safely
- Supports multiple sync tasks on Windows
- Maintains both `blacklist.filter` and `filter-v4150` on macOS
- Supports a shared JSON configuration for syncing one ignore-rule set across devices
- Uses a file watcher for real-time shared configuration updates, with a configurable fallback polling interval
- Supports tray/background operation, so closing the window keeps update detection and syncing active
- Supports Chinese / English UI switching

## Multi-device sharing

In the “Shared configuration” area, choose an existing `.json` file or create a new one from the current rules. It is best to place this file inside a Synology Drive synced folder.

After connecting a shared configuration, new rules can be saved as:

- **Local rules**, saved only on the current device.
- **Cloud rules**, written to the shared configuration and synced to other devices using that file.
- Existing local rules can be promoted to cloud rules. Add, delete, and promote operations are committed after clicking “Save changes”.

When shared configuration is enabled:

- The app watches the shared configuration file and writes changes to local Synology Drive configuration automatically.
- When an automatic write happens, the app shows “Disabled-sync folder configuration has been updated.”
- The fallback polling interval defaults to 60 minutes and can be configured in the app from 1 to 1440 minutes.
- The final configuration is always `local rules ∪ cloud rules`; duplicate names are deduplicated and cloud rules are marked first.
- The app remembers the shared file path. Disconnecting does not delete the shared file, and rules already written locally remain on this device.

The shared file uses a simple, readable JSON format:

```json
{
  "version": 1,
  "ignoredFolders": ["node_modules", ".git", "dist"]
}
```

## Configuration locations

- macOS: `~/Library/Application Support/SynologyDrive/SynologyDrive.app/Contents/Resources/conf`
- Windows: `%LOCALAPPDATA%\SynologyDrive\data\session\*\conf`

It is recommended to quit Synology Drive before saving, then restart the client. Synology Drive updates may reset these files, so you may need to save again afterward.

### macOS permission

macOS protects other apps’ resource files. If the first save shows a permission warning, open:

`System Settings → Privacy & Security → App Management`

- Installed app: allow **Synology Drive Ignore** to modify other apps.
- `pnpm tauri dev`: also allow the Terminal, iTerm, or Codex app that launched the dev process.

After granting permission, fully quit and reopen the app.

## Development

```sh
pnpm install
pnpm tauri dev
```

Run checks:

```sh
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

To cross-build a Windows x64 installer from macOS/Linux after installing LLVM, `cargo-xwin`, the Windows Rust target, and NSIS:

```sh
pnpm build:windows
```