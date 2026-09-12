use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

const BLACKLIST_FILE: &str = "blacklist.filter";
const DEFAULT_POLLING_INTERVAL_SECONDS: u64 = 60 * 60;
#[cfg(target_os = "macos")]
const VERSION_FILTER_FILE: &str = "filter-v4150";

struct SharedConfigWatcher(Mutex<Option<RecommendedWatcher>>);

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[derive(Clone)]
struct ConfigTarget {
    id: String,
    label: String,
    blacklist_path: PathBuf,
    version_filter_path: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigLocation {
    id: String,
    label: String,
    path: String,
    related_files: Vec<String>,
    ignored_folders: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigSnapshot {
    platform: String,
    locations: Vec<ConfigLocation>,
    searched_paths: Vec<String>,
    drive_status: DriveStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateResult {
    location: ConfigLocation,
    message: String,
    drive_status: DriveStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DriveStatus {
    running: bool,
    detection_available: bool,
    processes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedConfigDocument {
    version: u8,
    ignored_folders: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedConfigInfo {
    path: String,
    ignored_folders: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncSettingsInfo {
    polling_interval_seconds: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedConfigChangedEvent {
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedConfigInfo {
    path: String,
    local_folders: Vec<String>,
    cloud_folders: Vec<String>,
    effective_folders: Vec<String>,
    changed: bool,
    drive_status: DriveStatus,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    shared_config_path: Option<String>,
    #[serde(default = "default_polling_interval_seconds")]
    polling_interval_seconds: u64,
    #[serde(default)]
    last_cloud_by_target: BTreeMap<String, Vec<String>>,
}

fn default_polling_interval_seconds() -> u64 {
    DEFAULT_POLLING_INTERVAL_SECONDS
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            shared_config_path: None,
            polling_interval_seconds: DEFAULT_POLLING_INTERVAL_SECONDS,
            last_cloud_by_target: BTreeMap::new(),
        }
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn detect_drive_status() -> DriveStatus {
    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "cloud-drive-daemon",
            "cloud-drive-ui",
            "cloud-drive-connect",
            "SynologyDrive",
        ];
        let mut processes = Vec::new();
        let mut detection_available = false;
        for candidate in candidates {
            if let Ok(output) = std::process::Command::new("pgrep")
                .args(["-x", candidate])
                .output()
            {
                detection_available = true;
                if output.status.success() {
                    processes.push(candidate.to_string());
                }
            }
        }
        return DriveStatus {
            running: !processes.is_empty(),
            detection_available,
            processes,
        };
    }

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut command = std::process::Command::new("tasklist");
        command
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW);

        let output = command.output();
        return match output {
            Ok(output) if output.status.success() => {
                let listing = String::from_utf8_lossy(&output.stdout).to_lowercase();
                let candidates = [
                    "cloud-drive-daemon.exe",
                    "cloud-drive-ui.exe",
                    "cloud-drive-connect.exe",
                    "synologydrive.exe",
                ];
                let processes = candidates
                    .iter()
                    .filter(|name| listing.contains(**name))
                    .map(|name| (*name).to_string())
                    .collect::<Vec<_>>();
                DriveStatus {
                    running: !processes.is_empty(),
                    detection_available: true,
                    processes,
                }
            }
            _ => DriveStatus {
                running: false,
                detection_available: false,
                processes: Vec::new(),
            },
        };
    }

    #[allow(unreachable_code)]
    DriveStatus {
        running: false,
        detection_available: false,
        processes: Vec::new(),
    }
}

fn config_targets() -> (Vec<ConfigTarget>, Vec<PathBuf>) {
    #[cfg(target_os = "macos")]
    {
        let mut roots = Vec::new();
        if let Some(home) = env::var_os("HOME") {
            roots.push(PathBuf::from(home).join(
                "Library/Application Support/SynologyDrive/SynologyDrive.app/Contents/Resources/conf",
            ));
        }
        roots.push(PathBuf::from(
            "/Applications/Synology Drive Client.app/Contents/Resources/conf",
        ));

        let targets = roots
            .iter()
            .enumerate()
            .filter(|(_, root)| root.join(BLACKLIST_FILE).is_file())
            .map(|(index, root)| ConfigTarget {
                id: format!("macos-{index}"),
                label: if index == 0 {
                    "Synology Drive".to_string()
                } else {
                    "Synology Drive（系统应用）".to_string()
                },
                blacklist_path: root.join(BLACKLIST_FILE),
                version_filter_path: root
                    .join(VERSION_FILTER_FILE)
                    .is_file()
                    .then(|| root.join(VERSION_FILTER_FILE)),
            })
            .collect();
        return (targets, roots);
    }

    #[cfg(target_os = "windows")]
    {
        let root = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\<用户名>\AppData\Local"))
            .join("SynologyDrive")
            .join("data")
            .join("session");
        let mut targets = Vec::new();
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let session_name = entry.file_name().to_string_lossy().into_owned();
                let blacklist_path = entry.path().join("conf").join(BLACKLIST_FILE);
                if blacklist_path.is_file() {
                    targets.push(ConfigTarget {
                        id: format!("windows-session-{session_name}"),
                        label: format!("同步任务 {session_name}"),
                        blacklist_path,
                        version_filter_path: None,
                    });
                }
            }
        }
        targets.sort_by(|a, b| a.label.cmp(&b.label));
        return (targets, vec![root]);
    }

    #[allow(unreachable_code)]
    (Vec::new(), Vec::new())
}

fn parse_list(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in value.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                let item = current.trim().trim_matches('"').trim().to_string();
                if !item.is_empty() {
                    values.push(item);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let item = current.trim().trim_matches('"').trim().to_string();
    if !item.is_empty() {
        values.push(item);
    }
    values
}

fn read_key(content: &str, section_name: &str, key_name: &str) -> Vec<String> {
    let mut current_section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }
        if current_section == section_name {
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim() == key_name {
                    return parse_list(value);
                }
            }
        }
    }
    Vec::new()
}

fn line_ending(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn set_key(content: &str, section_name: &str, key_name: &str, values: &[String]) -> String {
    let ending = line_ending(content);
    let had_final_newline = content.ends_with('\n');
    let serialized = values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let replacement = format!("{key_name}={serialized}");
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let mut section_start = None;
    let mut section_end = lines.len();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if section_start.is_some() {
                section_end = index;
                break;
            }
            if &trimmed[1..trimmed.len() - 1] == section_name {
                section_start = Some(index);
            }
        }
    }

    match section_start {
        Some(start) => {
            let existing = (start + 1..section_end).find(|index| {
                lines[*index]
                    .trim()
                    .split_once('=')
                    .is_some_and(|(key, _)| key.trim() == key_name)
            });
            if let Some(index) = existing {
                let indent = lines[index]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();
                lines[index] = format!("{indent}{replacement}");
            } else {
                lines.insert(section_end, replacement);
            }
        }
        None => {
            if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("[{section_name}]"));
            lines.push(replacement);
        }
    }

    let mut output = lines.join(ending);
    if had_final_newline || content.is_empty() {
        output.push_str(ending);
    }
    output
}

fn read_location(target: &ConfigTarget) -> Result<ConfigLocation, String> {
    let content = fs::read_to_string(&target.blacklist_path)
        .map_err(|error| format!("无法读取 {}：{error}", path_text(&target.blacklist_path)))?;
    let mut related_files = vec![path_text(&target.blacklist_path)];
    if let Some(path) = &target.version_filter_path {
        related_files.push(path_text(path));
    }
    Ok(ConfigLocation {
        id: target.id.clone(),
        label: target.label.clone(),
        path: path_text(target.blacklist_path.parent().unwrap_or(Path::new(""))),
        related_files,
        ignored_folders: read_key(&content, "Directory", "black_name"),
    })
}

#[tauri::command]
fn discover_configs() -> Result<ConfigSnapshot, String> {
    let (targets, searched_paths) = config_targets();
    let locations = targets
        .iter()
        .map(read_location)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ConfigSnapshot {
        platform: env::consts::OS.to_string(),
        locations,
        searched_paths: searched_paths.iter().map(|path| path_text(path)).collect(),
        drive_status: detect_drive_status(),
    })
}

fn validate_names(names: Vec<String>) -> Result<Vec<String>, String> {
    let mut cleaned = Vec::new();
    for name in names {
        let name = name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        if name == "." || name == ".." || name.contains(['"', ',', '/', '\\', '\r', '\n']) {
            return Err(format!("“{name}”不是有效的文件夹名"));
        }
        if !cleaned.iter().any(|existing| existing == &name) {
            cleaned.push(name);
        }
    }
    Ok(cleaned)
}

fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("settings.json"))
        .map_err(|error| format!("无法确定应用设置目录：{error}"))
}

fn read_preferences(app: &tauri::AppHandle) -> Result<Preferences, String> {
    let path = preferences_path(app)?;
    if !path.is_file() {
        return Ok(Preferences::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("无法读取应用设置 {}：{error}", path_text(&path)))?;
    serde_json::from_str(&content).map_err(|error| format!("应用设置格式无效：{error}"))
}

fn write_preferences(app: &tauri::AppHandle, preferences: &Preferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory).map_err(|error| format!("无法创建应用设置目录：{error}"))?;
    }
    let content = serde_json::to_string_pretty(preferences)
        .map_err(|error| format!("无法生成应用设置：{error}"))?;
    fs::write(&path, content).map_err(|error| format!("无法保存应用设置：{error}"))
}

fn stop_shared_config_watcher(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<SharedConfigWatcher>();
    let mut watcher = state
        .0
        .lock()
        .map_err(|_| "无法更新共享配置监听器".to_string())?;
    *watcher = None;
    Ok(())
}

fn start_shared_config_watcher(app: &tauri::AppHandle, path: &Path) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "共享配置路径没有所在目录，无法监听变化".to_string())?;
    let target_name = path
        .file_name()
        .ok_or_else(|| "共享配置路径没有文件名，无法监听变化".to_string())?
        .to_os_string();
    let target_path = path.to_path_buf();
    let event_path = path_text(path);
    let app_handle = app.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else {
            return;
        };
        let touches_target = event.paths.iter().any(|changed_path| {
            changed_path == &target_path
                || changed_path
                    .file_name()
                    .is_some_and(|name| name == target_name.as_os_str())
        });
        if touches_target {
            let _ = app_handle.emit(
                "shared-config-changed",
                SharedConfigChangedEvent {
                    path: event_path.clone(),
                },
            );
        }
    })
    .map_err(|error| format!("无法创建共享配置监听器：{error}"))?;
    watcher
        .watch(directory, RecursiveMode::NonRecursive)
        .map_err(|error| format!("无法监听共享配置目录 {}：{error}", path_text(directory)))?;

    let state = app.state::<SharedConfigWatcher>();
    let mut active = state
        .0
        .lock()
        .map_err(|_| "无法更新共享配置监听器".to_string())?;
    *active = Some(watcher);
    Ok(())
}

fn read_shared_document(path: &Path) -> Result<SharedConfigDocument, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("无法读取共享配置 {}：{error}", path_text(path)))?;
    let mut document: SharedConfigDocument = serde_json::from_str(&content)
        .map_err(|error| format!("共享配置不是有效的 JSON 文件：{error}"))?;
    if document.version != 1 {
        return Err(format!("暂不支持共享配置版本 {}", document.version));
    }
    document.ignored_folders = validate_names(document.ignored_folders)?;
    Ok(document)
}

fn write_shared_document(path: &Path, names: Vec<String>) -> Result<SharedConfigDocument, String> {
    let document = SharedConfigDocument {
        version: 1,
        ignored_folders: validate_names(names)?,
    };
    let content = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("无法生成共享配置：{error}"))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("无法写入共享配置 {}：{error}", path_text(path)))?;
    Ok(document)
}

fn shared_info(path: &Path, document: SharedConfigDocument) -> SharedConfigInfo {
    SharedConfigInfo {
        path: path_text(path),
        ignored_folders: document.ignored_folders,
    }
}

fn sync_settings_info(preferences: &Preferences) -> SyncSettingsInfo {
    SyncSettingsInfo {
        polling_interval_seconds: preferences.polling_interval_seconds.max(60),
    }
}

#[tauri::command]
fn get_sync_settings(app: tauri::AppHandle) -> Result<SyncSettingsInfo, String> {
    let preferences = read_preferences(&app)?;
    Ok(sync_settings_info(&preferences))
}

#[tauri::command]
fn update_sync_settings(
    app: tauri::AppHandle,
    polling_interval_seconds: u64,
) -> Result<SyncSettingsInfo, String> {
    let mut preferences = read_preferences(&app)?;
    preferences.polling_interval_seconds = polling_interval_seconds.clamp(60, 24 * 60 * 60);
    write_preferences(&app, &preferences)?;
    Ok(sync_settings_info(&preferences))
}

#[tauri::command]
fn get_shared_config(app: tauri::AppHandle) -> Result<Option<SharedConfigInfo>, String> {
    let Some(path) = read_preferences(&app)?.shared_config_path else {
        stop_shared_config_watcher(&app)?;
        return Ok(None);
    };
    let path = PathBuf::from(path);
    let document = read_shared_document(&path)?;
    start_shared_config_watcher(&app, &path)?;
    Ok(Some(shared_info(&path, document)))
}

#[tauri::command]
fn connect_shared_config(
    app: tauri::AppHandle,
    path: String,
    create: bool,
    names: Vec<String>,
) -> Result<SharedConfigInfo, String> {
    let mut path = PathBuf::from(path);
    if create && path.extension().is_none() {
        path.set_extension("json");
    }
    let document = if create {
        write_shared_document(&path, names)?
    } else {
        read_shared_document(&path)?
    };
    let mut preferences = read_preferences(&app)?;
    preferences.shared_config_path = Some(path_text(&path));
    write_preferences(&app, &preferences)?;
    start_shared_config_watcher(&app, &path)?;
    Ok(shared_info(&path, document))
}

#[tauri::command]
fn reload_shared_config(app: tauri::AppHandle) -> Result<SharedConfigInfo, String> {
    let path = read_preferences(&app)?
        .shared_config_path
        .ok_or_else(|| "尚未选择共享配置文件".to_string())?;
    let path = PathBuf::from(path);
    read_shared_document(&path).map(|document| shared_info(&path, document))
}

#[tauri::command]
fn save_shared_config(
    app: tauri::AppHandle,
    names: Vec<String>,
) -> Result<SharedConfigInfo, String> {
    let path = read_preferences(&app)?
        .shared_config_path
        .ok_or_else(|| "尚未选择共享配置文件".to_string())?;
    let path = PathBuf::from(path);
    write_shared_document(&path, names).map(|document| shared_info(&path, document))
}

fn union_folders(local: &[String], cloud: &[String]) -> Vec<String> {
    let mut effective = local.to_vec();
    for name in cloud {
        if !effective.contains(name) {
            effective.push(name.clone());
        }
    }
    effective
}

fn current_folders(target_id: &str) -> Result<Vec<String>, String> {
    let (targets, _) = config_targets();
    let target = targets
        .iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| "配置文件已移动或不存在，请重新扫描".to_string())?;
    read_location(target).map(|location| location.ignored_folders)
}

fn shared_path(preferences: &Preferences) -> Result<PathBuf, String> {
    preferences
        .shared_config_path
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| "尚未选择共享配置文件".to_string())
}

fn managed_info(
    path: &Path,
    local_folders: Vec<String>,
    cloud_folders: Vec<String>,
    changed: bool,
    drive_status: DriveStatus,
) -> ManagedConfigInfo {
    let effective_folders = union_folders(&local_folders, &cloud_folders);
    ManagedConfigInfo {
        path: path_text(path),
        local_folders,
        cloud_folders,
        effective_folders,
        changed,
        drive_status,
    }
}

fn unavailable_drive_status() -> DriveStatus {
    DriveStatus {
        running: false,
        detection_available: false,
        processes: Vec::new(),
    }
}

#[tauri::command]
fn sync_managed_config(
    app: tauri::AppHandle,
    target_id: String,
    refresh_drive_status: Option<bool>,
) -> Result<ManagedConfigInfo, String> {
    let mut preferences = read_preferences(&app)?;
    let path = shared_path(&preferences)?;
    let cloud = read_shared_document(&path)?.ignored_folders;
    let current = current_folders(&target_id)?;
    let previous_cloud = preferences.last_cloud_by_target.get(&target_id).cloned();
    let ownership_cloud = previous_cloud.as_ref().unwrap_or(&cloud);
    let local = current
        .iter()
        .filter(|name| !ownership_cloud.contains(name))
        .cloned()
        .collect::<Vec<_>>();
    let effective = union_folders(&local, &cloud);
    let changed = current != effective;
    let drive_status = if changed {
        update_ignored_folders(target_id.clone(), effective)?.drive_status
    } else if refresh_drive_status.unwrap_or(false) {
        detect_drive_status()
    } else {
        unavailable_drive_status()
    };
    if previous_cloud.as_ref() != Some(&cloud) {
        preferences
            .last_cloud_by_target
            .insert(target_id, cloud.clone());
        write_preferences(&app, &preferences)?;
    }
    Ok(managed_info(&path, local, cloud, changed, drive_status))
}

#[tauri::command]
fn save_local_managed_config(
    app: tauri::AppHandle,
    target_id: String,
    names: Vec<String>,
) -> Result<ManagedConfigInfo, String> {
    let mut preferences = read_preferences(&app)?;
    let path = shared_path(&preferences)?;
    let cloud = read_shared_document(&path)?.ignored_folders;
    let local = validate_names(names)?
        .into_iter()
        .filter(|name| !cloud.contains(name))
        .collect::<Vec<_>>();
    let effective = union_folders(&local, &cloud);
    let result = update_ignored_folders(target_id.clone(), effective)?;
    preferences
        .last_cloud_by_target
        .insert(target_id, cloud.clone());
    write_preferences(&app, &preferences)?;
    Ok(managed_info(&path, local, cloud, true, result.drive_status))
}

#[tauri::command]
fn save_cloud_managed_config(
    app: tauri::AppHandle,
    target_id: String,
    names: Vec<String>,
) -> Result<ManagedConfigInfo, String> {
    let mut preferences = read_preferences(&app)?;
    let path = shared_path(&preferences)?;
    let previous_document = read_shared_document(&path)?;
    let current = current_folders(&target_id)?;
    let ownership_cloud = preferences
        .last_cloud_by_target
        .get(&target_id)
        .unwrap_or(&previous_document.ignored_folders);
    let local = current
        .iter()
        .filter(|name| !ownership_cloud.contains(name))
        .cloned()
        .collect::<Vec<_>>();
    let cloud = validate_names(names)?;
    write_shared_document(&path, cloud.clone())?;
    let effective = union_folders(&local, &cloud);
    let result = match update_ignored_folders(target_id.clone(), effective) {
        Ok(result) => result,
        Err(error) => {
            let _ = write_shared_document(&path, previous_document.ignored_folders);
            return Err(format!("{error}（云配置的修改已撤销）"));
        }
    };
    preferences
        .last_cloud_by_target
        .insert(target_id, cloud.clone());
    write_preferences(&app, &preferences)?;
    Ok(managed_info(&path, local, cloud, true, result.drive_status))
}

#[tauri::command]
fn save_managed_config(
    app: tauri::AppHandle,
    target_id: String,
    local_names: Vec<String>,
    cloud_names: Vec<String>,
) -> Result<ManagedConfigInfo, String> {
    let mut preferences = read_preferences(&app)?;
    let path = shared_path(&preferences)?;
    let previous_document = read_shared_document(&path)?;
    let cloud = validate_names(cloud_names)?;
    let local = validate_names(local_names)?
        .into_iter()
        .filter(|name| !cloud.contains(name))
        .collect::<Vec<_>>();
    let cloud_changed = previous_document.ignored_folders != cloud;
    if cloud_changed {
        write_shared_document(&path, cloud.clone())?;
    }
    let effective = union_folders(&local, &cloud);
    let result = match update_ignored_folders(target_id.clone(), effective) {
        Ok(result) => result,
        Err(error) => {
            if cloud_changed {
                let _ = write_shared_document(&path, previous_document.ignored_folders);
            }
            return Err(format!("{error}（云配置的修改已撤销）"));
        }
    };
    preferences
        .last_cloud_by_target
        .insert(target_id, cloud.clone());
    write_preferences(&app, &preferences)?;
    Ok(managed_info(&path, local, cloud, true, result.drive_status))
}

#[tauri::command]
fn disconnect_shared_config(app: tauri::AppHandle) -> Result<(), String> {
    let polling_interval_seconds = read_preferences(&app)?.polling_interval_seconds;
    stop_shared_config_watcher(&app)?;
    write_preferences(
        &app,
        &Preferences {
            polling_interval_seconds,
            ..Preferences::default()
        },
    )
}

fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|error| {
        #[cfg(target_os = "macos")]
        if error.raw_os_error() == Some(1) || error.raw_os_error() == Some(13) {
            return format!(
                "APP_MANAGEMENT_REQUIRED|macOS 已阻止修改 Synology Drive。请在“系统设置 → 隐私与安全性 → App 管理”中允许本应用修改其他应用，然后完全退出并重新打开本应用。目标文件：{}",
                path_text(path)
            );
        }
        format!("无法写入 {}：{error}", path_text(path))
    })
}

#[tauri::command]
fn open_permission_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_AppBundles")
            .status()
            .map_err(|error| format!("无法打开系统设置：{error}"))?;
        return status
            .success()
            .then_some(())
            .ok_or_else(|| "无法打开系统设置，请手动打开“隐私与安全性 → App 管理”".to_string());
    }

    #[cfg(not(target_os = "macos"))]
    Err("当前系统不需要 macOS App 管理权限".to_string())
}

#[tauri::command]
fn update_ignored_folders(target_id: String, names: Vec<String>) -> Result<UpdateResult, String> {
    let names = validate_names(names)?;
    let (targets, _) = config_targets();
    let target = targets
        .into_iter()
        .find(|candidate| candidate.id == target_id)
        .ok_or_else(|| "配置文件已移动或不存在，请重新扫描".to_string())?;
    let blacklist_original = fs::read_to_string(&target.blacklist_path)
        .map_err(|error| format!("无法读取配置：{error}"))?;
    let blacklist_updated = set_key(&blacklist_original, "Directory", "black_name", &names);

    let version_update = if let Some(path) = &target.version_filter_path {
        let original = fs::read_to_string(path)
            .map_err(|error| format!("无法读取 {}：{error}", path_text(path)))?;
        let common_existing = read_key(&original, "Common", "black_name");
        let directory_existing = read_key(&original, "Directory", "black_name");
        let previous_managed = read_key(&blacklist_original, "Directory", "black_name");
        let merge = |existing: Vec<String>| {
            let mut next = existing
                .into_iter()
                .filter(|item| !previous_managed.contains(item))
                .collect::<Vec<_>>();
            for name in &names {
                if !next.contains(name) {
                    next.push(name.clone());
                }
            }
            next
        };
        let updated = set_key(&original, "Common", "black_name", &merge(common_existing));
        let updated = set_key(
            &updated,
            "Directory",
            "black_name",
            &merge(directory_existing),
        );
        Some((path.clone(), updated))
    } else {
        None
    };

    write_file(&target.blacklist_path, blacklist_updated)?;
    if let Some((path, updated)) = &version_update {
        if let Err(error) = write_file(path, updated) {
            let _ = fs::write(&target.blacklist_path, blacklist_original);
            return Err(format!("{error}（blacklist.filter 的修改已撤销）"));
        }
    }

    Ok(UpdateResult {
        location: read_location(&target)?,
        message: if target.version_filter_path.is_some() {
            "已同步更新 blacklist.filter 和 filter-v4150".to_string()
        } else {
            "已更新 blacklist.filter".to_string()
        },
        drive_status: detect_drive_status(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SharedConfigWatcher(Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let menu = MenuBuilder::new(app)
                .text("show", "打开 / Open Synology Drive Ignore")
                .separator()
                .text("quit", "退出 / Quit")
                .build()?;
            let mut tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("Synology Drive Ignore 正在后台运行 / Running in background")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "show" {
                        show_main_window(app);
                    } else if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    let should_show = match event {
                        TrayIconEvent::Click {
                            button,
                            button_state,
                            ..
                        } => button == MouseButton::Left && button_state == MouseButtonState::Up,
                        TrayIconEvent::DoubleClick { button, .. } => button == MouseButton::Left,
                        _ => false,
                    };
                    if should_show {
                        show_main_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            discover_configs,
            update_ignored_folders,
            open_permission_settings,
            get_sync_settings,
            update_sync_settings,
            get_shared_config,
            connect_shared_config,
            reload_shared_config,
            save_shared_config,
            disconnect_shared_config,
            sync_managed_config,
            save_local_managed_config,
            save_cloud_managed_config,
            save_managed_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{read_key, set_key, union_folders};

    #[test]
    fn reads_and_replaces_directory_names_without_touching_other_sections() {
        let input = "[Common]\r\nblack_name=\"system\"\r\n\r\n[Directory]\r\nblack_name = \"node_modules\", \".git\"\r\n";
        assert_eq!(
            read_key(input, "Directory", "black_name"),
            vec!["node_modules", ".git"]
        );
        let output = set_key(input, "Directory", "black_name", &["dist".into()]);
        assert!(output.contains("[Common]\r\nblack_name=\"system\""));
        assert!(output.contains("[Directory]\r\nblack_name=\"dist\""));
        assert!(output.ends_with("\r\n"));
    }

    #[test]
    fn creates_a_missing_section() {
        let output = set_key(
            "[Version]\nmajor=1\n",
            "Directory",
            "black_name",
            &["build".into()],
        );
        assert!(output.contains("\n[Directory]\nblack_name=\"build\"\n"));
    }

    #[test]
    fn merges_local_and_cloud_folders_without_duplicates() {
        let local = vec!["node_modules".to_string(), "dist".to_string()];
        let cloud = vec![".venv".to_string(), "dist".to_string()];
        assert_eq!(
            union_folders(&local, &cloud),
            vec!["node_modules", "dist", ".venv"]
        );
    }
}
