# Synology Drive Ignore 使用说明 / User Guide

中文 | [English](#english)

## 这个应用做什么

Synology Drive Ignore 用来管理 Synology Drive 的“不同步文件夹”配置。它可以读取本机 Synology Drive 配置，添加或删除文件夹名，并把结果安全写回配置文件。

它还支持把忽略规则保存到一个共享 JSON 文件中。把这个文件放进 Synology Drive 同步目录后，多台设备可以共用同一份“禁用同步文件夹”配置。

## 语言切换

应用标题栏右侧提供 `中文 / EN` 切换。选择会保存在当前设备，下次启动时自动沿用。

## 基本使用

1. 打开应用后等待自动扫描。
2. 如果有多个同步任务，先选择要修改的任务。
3. 在“本机配置中的忽略文件夹”中添加、删除或提升规则。
4. 点击“保存更改”。
5. 如果 Synology Drive 正在运行，退出并重新启动 Synology Drive，让新规则生效。

规则只需要填写文件夹名，例如：

- `node_modules`
- `.git`
- `dist`
- `.venv`

不要输入完整路径、逗号、引号或多行内容。

## 共享配置

在“共享配置”区域可以：

- 选择已有 JSON 配置文件
- 用当前列表创建新的 JSON 配置文件
- 断开共享配置
- 调整兜底轮询间隔

建议把共享配置文件放在 Synology Drive 已同步的文件夹里。连接后，应用会把云规则和本机规则合并写入 Synology Drive 配置。

最终写入结果为：

```text
本机规则 ∪ 云规则
```

本机规则只影响当前设备。云规则会写入共享 JSON，并同步到其他连接同一配置文件的设备。

## 自动检测更新

应用主要依靠文件 watcher 监听共享配置变化。当其他设备更新了共享 JSON 文件后，本机应用会自动读取并写入 Synology Drive 配置。

为了防止系统文件事件偶发丢失，应用还会按可配置间隔进行兜底轮询。默认值为 60 分钟，可在“共享配置”区域修改，支持 1 到 1440 分钟。

当自动写入完成后，应用会显示提示：

```text
已更新禁用同步文件夹配置。
```

## 托盘与后台运行

应用支持托盘功能。关闭主窗口时，程序不会立即退出，而是隐藏到后台继续运行：

- 继续监听共享配置文件变化
- 继续按配置间隔进行兜底轮询
- 检测到更新后自动写入本机配置
- 可从托盘菜单重新打开窗口或退出应用

## macOS 权限

macOS 会保护其他应用的资源文件。首次保存时如果提示没有权限，请打开：

```text
系统设置 → 隐私与安全性 → App 管理
```

然后允许：

- 安装版：`Synology Drive Ignore`
- 开发版：启动 `pnpm tauri dev` 的 Terminal、iTerm 或 Codex

授权后需要完全退出并重新打开应用。

## Windows 构建

在 macOS 或 Linux 上交叉构建 Windows x64 安装包前，需要准备 LLVM、`cargo-xwin`、Windows Rust target 和 NSIS。准备好后执行：

```sh
pnpm build:windows
```

生成物通常位于 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`，项目也可以把最终安装包复制到 `release/`。

## GitHub Release

仓库已配置 `.github/workflows/release.yml`。推送 `v*.*.*` 格式的 tag 后，GitHub Actions 会分别构建：

- macOS：DMG
- Windows：NSIS 安装包

发布命令示例：

```sh
git tag v0.1.0
git push origin v0.1.0
```

构建产物会上传到 GitHub 草稿 Release，确认说明和附件后再手动发布即可。也可以在 GitHub Actions 页面手动运行 `Release` workflow。

## English

## What this app does

Synology Drive Ignore manages Synology Drive ignored-folder configuration. It reads the local Synology Drive configuration, lets you add or remove folder names, and writes the result back safely.

It can also store ignore rules in a shared JSON file. Put that file inside a Synology Drive synced folder, and multiple devices can share the same disabled-sync folder configuration.

## Language switching

The title bar includes a `中文 / EN` switch. The selected language is saved on this device and reused on the next launch.

## Basic usage

1. Open the app and wait for the automatic scan.
2. If there are multiple sync tasks, select the task you want to edit.
3. Add, remove, or promote rules in “Ignored folders in local configuration”.
4. Click “Save changes”.
5. If Synology Drive is running, quit and restart Synology Drive for the new rules to take effect.

Rules should be folder names only, for example:

- `node_modules`
- `.git`
- `dist`
- `.venv`

Do not enter full paths, commas, quotes, or multi-line content.

## Shared configuration

In the “Shared configuration” area, you can:

- Choose an existing JSON configuration file
- Create a new JSON configuration file from the current list
- Disconnect the shared configuration
- Adjust the fallback polling interval

Place the shared configuration file inside a Synology Drive synced folder. After connecting it, the app merges cloud rules and local rules, then writes the merged result to Synology Drive configuration.

The final written result is:

```text
local rules ∪ cloud rules
```

Local rules affect only the current device. Cloud rules are written to the shared JSON file and synced to other devices connected to the same file.

## Automatic update detection

The app primarily uses a file watcher to monitor the shared configuration. When another device updates the shared JSON file, this app reads it and writes the update into local Synology Drive configuration automatically.

To guard against occasional missed file events, the app also runs fallback polling at a configurable interval. The default is 60 minutes, and it can be changed in the “Shared configuration” area from 1 to 1440 minutes.

When an automatic write completes, the app shows:

```text
Disabled-sync folder configuration has been updated.
```

## Tray and background operation

The app supports tray operation. Closing the main window hides it instead of quitting, so it keeps running in the background:

- Watches the shared configuration file
- Runs fallback polling at the configured interval
- Writes local configuration automatically when updates are detected
- Can be reopened or quit from the tray menu

## macOS permission

macOS protects other apps’ resource files. If saving fails with a permission prompt, open:

```text
System Settings → Privacy & Security → App Management
```

Then allow:

- Installed app: `Synology Drive Ignore`
- Dev mode: the Terminal, iTerm, or Codex app that launched `pnpm tauri dev`

After granting permission, fully quit and reopen the app.

## Windows build

Before cross-building a Windows x64 installer from macOS or Linux, install LLVM, `cargo-xwin`, the Windows Rust target, and NSIS. Then run:

```sh
pnpm build:windows
```

The generated installer is usually under `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`; this project may also copy final release artifacts into `release/`.

## GitHub Release

The repository includes `.github/workflows/release.yml`. After pushing a `v*.*.*` tag, GitHub Actions builds:

- macOS: DMG
- Windows: NSIS installer

Example release command:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Artifacts are uploaded to a draft GitHub Release, so you can review the notes and assets before publishing. You can also run the `Release` workflow manually from the GitHub Actions page.
