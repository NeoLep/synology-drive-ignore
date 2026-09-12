<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import appLogo from "./assets/app-logo.png";

type ConfigLocation = {
  id: string;
  label: string;
  path: string;
  relatedFiles: string[];
  ignoredFolders: string[];
};

type ConfigSnapshot = {
  platform: string;
  locations: ConfigLocation[];
  searchedPaths: string[];
  driveStatus: DriveStatus;
};

type DriveStatus = {
  running: boolean;
  detectionAvailable: boolean;
  processes: string[];
};
type UpdateResult = { location: ConfigLocation; message: string; driveStatus: DriveStatus };
type SharedConfig = { path: string; ignoredFolders: string[] };
type SyncSettings = { pollingIntervalSeconds: number };
type SharedConfigChangedEvent = { path: string };
type ManagedConfig = {
  path: string;
  localFolders: string[];
  cloudFolders: string[];
  effectiveFolders: string[];
  changed: boolean;
  driveStatus: DriveStatus;
};
type DisplayRule = { name: string; cloud: boolean };
type RuleSource = "local" | "cloud";
type Locale = "zh" | "en";

const LOCALE_STORAGE_KEY = "synology-drive-ignore-locale";
const messages = {
  zh: {
    subtitle: "同步忽略规则管理器",
    localConfig: "本机配置",
    heroTitle: "管理不同步的文件夹",
    heroDescription: "自动读取 Synology Drive 配置。添加或移除文件夹名后，一次保存即可写回对应文件。",
    scanning: "正在扫描",
    rescan: "重新扫描",
    runningTitle: "Synology Drive 正在运行。",
    runningText: "可以保存配置，但需要退出并重新启动客户端后规则才会生效。",
    detected: "检测到：",
    stoppedTitle: "Synology Drive 当前未运行。",
    stoppedText: "保存的配置会在下次启动客户端时生效。",
    unknownTitle: "无法检测 Synology Drive 状态。",
    unknownText: "保存后请手动重启客户端，确保新规则生效。",
    loadingTitle: "正在查找配置文件",
    loadingText: "正在检查当前系统的 Synology Drive 目录…",
    emptyTitle: "没有找到 Synology Drive 配置",
    emptyText: "请确认已经安装并至少创建过一个同步任务，然后重新扫描。",
    checkedLocations: "查看已检查的位置",
    syncTask: "同步任务",
    sharedConfig: "共享配置",
    sharedIntro: "把配置文件放在 Synology Drive 文件夹中，即可在多台设备间共享规则。",
    managedBadge: (count: number) => `托管中 · ${count} 个云规则`,
    guideManagedTitle: "共享配置托管已开启",
    guideIntroTitle: "这是做什么的？",
    guideManagedText: "应用会监听共享配置文件的变化并实时同步；兜底轮询用于处理系统文件事件偶发丢失的情况。",
    guideIntroText: "把忽略文件夹列表保存成一个小型 JSON 文件。将它放进 Synology Drive 云文件夹后，多台电脑就可以使用同一份规则。",
    step1: "选择已有配置，或用当前列表创建",
    step2: "云规则会自动应用，并以云朵图标标记",
    step3: "最终写入结果是本机规则与云规则的并集",
    pollingInterval: "兜底轮询间隔",
    minutes: "分钟",
    save: "保存",
    cloudRulesTitle: "云配置中的忽略文件夹",
    autoApply: "文件更新后自动应用",
    noCloudRules: "这个云配置目前没有忽略规则",
    mergeExplanation: "新增时可以选择本机或云规则；普通规则也可提升为云规则。删除带云朵的规则会更新云配置文件，删除普通规则只影响当前设备。所有更改均在点击“保存更改”后提交。",
    sourceRules: "来源规则：",
    checkCloudUpdates: "立即检查云端更新",
    changeFile: "更换文件",
    disconnect: "断开",
    chooseExisting: "选择已有配置",
    createFromCurrent: "用当前列表创建",
    found: "已找到",
    mergedCount: (count: number) => `${count} 个合并规则`,
    localRulesTitle: "本机配置中的忽略文件夹",
    effectiveHint: "最终结果 = 本机规则 ∪ 云规则",
    fromCloud: "来自共享云配置",
    cloudConfig: "云配置",
    promoteTitle: "保存后写入共享配置，并同步到其他设备",
    promote: "提升为云规则",
    promoteAria: (name: string) => `将 ${name} 提升为云规则`,
    deleteAria: (name: string) => `删除 ${name}`,
    noRules: "当前没有自定义忽略规则",
    newRuleSource: "新规则保存位置",
    addAs: "添加为",
    localRule: "本机规则",
    cloudRule: "云规则",
    cloudRuleEnabledTitle: "保存到共享配置并同步到其他设备",
    cloudRuleDisabledTitle: "请先连接共享配置",
    connectRequired: "连接共享配置后可选",
    cloudRuleHint: "会同步到使用此配置的设备",
    localRuleHint: "仅保存在当前设备",
    cloudPlaceholder: "输入要共享的文件夹名，例如 .venv",
    localPlaceholder: "输入本机文件夹名，例如 node_modules",
    add: "添加",
    permissionTitle: "需要允许修改其他应用",
    permissionHelp: "如果通过 pnpm tauri dev 运行，还需要允许启动它的终端或 Codex；安装版请允许 Synology Drive Ignore。",
    openSettings: "打开系统设置",
    writeFiles: (count: number) => `将写入 ${count} 个配置文件`,
    saving: "正在保存…",
    saveChanges: "保存更改",
    upToDate: "已是最新",
    pollingUpdated: (minutes: number) => `兜底轮询间隔已更新为 ${minutes} 分钟。`,
    notificationBody: "已更新禁用同步文件夹配置。",
    syncUpdated: "已更新禁用同步文件夹配置。",
    syncAlreadySynced: "云配置与本机托管结果已经同步。",
    syncUpdatedRestart: "云配置已自动更新本机规则；Synology Drive 正在运行，请重启客户端后生效。",
    sharedCreated: "共享配置已创建并开始托管，当前列表已写入云配置。",
    sharedConnected: (count: number) => `已启用托管，其中 ${count} 个云规则已自动应用到本机。`,
    sharedDisconnected: "已断开共享配置，文件本身没有被删除。",
    invalidFolder: "请输入单个文件夹名，不能包含路径、逗号或引号。",
    alreadyIgnored: (name: string) => `“${name}”已经在忽略列表中。`,
    addedCloud: (name: string) => `已将“${name}”添加为云规则，点击“保存更改”后会写入共享配置。`,
    addedLocal: (name: string) => `已将“${name}”添加为本机规则，点击“保存更改”后生效。`,
    promotedCloud: (name: string) => `已将“${name}”提升为云规则，点击“保存更改”后会同步到其他设备。`,
    managedSavedShared: "云端规则和本机规则已按各自来源保存，合并结果已写入 Synology Drive。",
    managedSavedRunning: "托管配置已保存。检测到 Synology Drive 仍在运行，请退出并重新启动客户端后使规则生效。",
    managedSavedStopped: "托管配置已保存。Synology Drive 当前未运行，规则会在下次启动时生效。",
    managedSavedUnknown: "托管配置已保存。无法确认客户端状态，请重启 Synology Drive 后使规则生效。",
    updatedLocalConfig: "已更新 Synology Drive 忽略配置",
    updateRunning: (message: string) => `${message}。检测到 Synology Drive 仍在运行，请退出并重新启动客户端后使规则生效。`,
    updateStopped: (message: string) => `${message}。Synology Drive 当前未运行，规则会在下次启动时生效。`,
    updateUnknown: (message: string) => `${message}。无法确认客户端状态，请重启 Synology Drive 后使规则生效。`,
    languageLabel: "语言",
    systemApp: "Synology Drive（系统应用）",
    syncTaskNamed: (name: string) => `同步任务 ${name}`,
  },
  en: {
    subtitle: "Ignore rules manager",
    localConfig: "Local configuration",
    heroTitle: "Manage folders that should not sync",
    heroDescription: "Automatically reads Synology Drive configuration. Add or remove folder names, then save once to write the matching files.",
    scanning: "Scanning",
    rescan: "Rescan",
    runningTitle: "Synology Drive is running.",
    runningText: "You can save the configuration, but you need to quit and restart Synology Drive before the rules take effect.",
    detected: "Detected: ",
    stoppedTitle: "Synology Drive is not running.",
    stoppedText: "Saved rules will take effect the next time the client starts.",
    unknownTitle: "Unable to detect Synology Drive status.",
    unknownText: "After saving, restart Synology Drive manually to make sure the new rules take effect.",
    loadingTitle: "Searching for configuration files",
    loadingText: "Checking Synology Drive folders on this system…",
    emptyTitle: "No Synology Drive configuration found",
    emptyText: "Make sure Synology Drive is installed and at least one sync task has been created, then rescan.",
    checkedLocations: "Show checked locations",
    syncTask: "Sync task",
    sharedConfig: "Shared configuration",
    sharedIntro: "Put the configuration file inside Synology Drive to share rules across devices.",
    managedBadge: (count: number) => `Managed · ${count} cloud rule${count === 1 ? "" : "s"}`,
    guideManagedTitle: "Shared configuration management is enabled",
    guideIntroTitle: "What is this for?",
    guideManagedText: "The app watches the shared configuration file and syncs changes in real time. Fallback polling handles occasional missed file events.",
    guideIntroText: "Save the ignored folder list as a small JSON file. Put it in a Synology Drive folder so multiple computers can use the same rules.",
    step1: "Select an existing file, or create one from the current list",
    step2: "Cloud rules are applied automatically and marked with a cloud icon",
    step3: "The final written result is the union of local and cloud rules",
    pollingInterval: "Fallback polling interval",
    minutes: "minutes",
    save: "Save",
    cloudRulesTitle: "Ignored folders in cloud configuration",
    autoApply: "Applied automatically after file updates",
    noCloudRules: "This cloud configuration has no ignore rules yet",
    mergeExplanation: "When adding a rule, choose local or cloud. Local rules can also be promoted to cloud rules. Deleting a cloud-marked rule updates the shared file; deleting a normal rule only affects this device. All changes are committed after clicking “Save changes”.",
    sourceRules: "Source rules: ",
    checkCloudUpdates: "Check cloud updates now",
    changeFile: "Change file",
    disconnect: "Disconnect",
    chooseExisting: "Choose existing file",
    createFromCurrent: "Create from current list",
    found: "Found",
    mergedCount: (count: number) => `${count} merged rule${count === 1 ? "" : "s"}`,
    localRulesTitle: "Ignored folders in local configuration",
    effectiveHint: "Final result = local rules ∪ cloud rules",
    fromCloud: "From shared cloud configuration",
    cloudConfig: "Cloud config",
    promoteTitle: "Save to the shared configuration and sync to other devices",
    promote: "Promote to cloud rule",
    promoteAria: (name: string) => `Promote ${name} to a cloud rule`,
    deleteAria: (name: string) => `Delete ${name}`,
    noRules: "No custom ignore rules yet",
    newRuleSource: "New rule save location",
    addAs: "Add as",
    localRule: "Local rule",
    cloudRule: "Cloud rule",
    cloudRuleEnabledTitle: "Save to the shared configuration and sync to other devices",
    cloudRuleDisabledTitle: "Connect a shared configuration first",
    connectRequired: "Connect a shared configuration to enable this",
    cloudRuleHint: "Syncs to devices using this configuration",
    localRuleHint: "Only saved on this device",
    cloudPlaceholder: "Folder name to share, e.g. .venv",
    localPlaceholder: "Local folder name, e.g. node_modules",
    add: "Add",
    permissionTitle: "Permission required to modify another app",
    permissionHelp: "If you run with pnpm tauri dev, also allow the terminal app or Codex that launched it. For the installed app, allow Synology Drive Ignore.",
    openSettings: "Open System Settings",
    writeFiles: (count: number) => `Will write ${count} configuration file${count === 1 ? "" : "s"}`,
    saving: "Saving…",
    saveChanges: "Save changes",
    upToDate: "Up to date",
    pollingUpdated: (minutes: number) => `Fallback polling interval updated to ${minutes} minute${minutes === 1 ? "" : "s"}.`,
    notificationBody: "Disabled-sync folder configuration has been updated.",
    syncUpdated: "Disabled-sync folder configuration has been updated.",
    syncAlreadySynced: "Cloud configuration and local managed result are already in sync.",
    syncUpdatedRestart: "Cloud configuration updated local rules automatically. Synology Drive is running; restart the client for the rules to take effect.",
    sharedCreated: "Shared configuration created and management started. The current list has been written to cloud configuration.",
    sharedConnected: (count: number) => `Management enabled. ${count} cloud rule${count === 1 ? "" : "s"} applied locally.`,
    sharedDisconnected: "Shared configuration disconnected. The file itself was not deleted.",
    invalidFolder: "Enter a single folder name. Paths, commas, quotes, and line breaks are not allowed.",
    alreadyIgnored: (name: string) => `"${name}" is already in the ignore list.`,
    addedCloud: (name: string) => `"${name}" was added as a cloud rule. Click “Save changes” to write it to the shared configuration.`,
    addedLocal: (name: string) => `"${name}" was added as a local rule. Click “Save changes” to apply it.`,
    promotedCloud: (name: string) => `"${name}" was promoted to a cloud rule. Click “Save changes” to sync it to other devices.`,
    managedSavedShared: "Cloud rules and local rules were saved by source, and the merged result was written to Synology Drive.",
    managedSavedRunning: "Managed configuration saved. Synology Drive is still running; quit and restart the client for the rules to take effect.",
    managedSavedStopped: "Managed configuration saved. Synology Drive is not running, so the rules will take effect on next launch.",
    managedSavedUnknown: "Managed configuration saved. Client status could not be confirmed; restart Synology Drive for the rules to take effect.",
    updatedLocalConfig: "Synology Drive ignore configuration updated",
    updateRunning: (message: string) => `${message}. Synology Drive is still running; quit and restart the client for the rules to take effect.`,
    updateStopped: (message: string) => `${message}. Synology Drive is not running, so the rules will take effect on next launch.`,
    updateUnknown: (message: string) => `${message}. Client status could not be confirmed; restart Synology Drive for the rules to take effect.`,
    languageLabel: "Language",
    systemApp: "Synology Drive (system app)",
    syncTaskNamed: (name: string) => `Sync task ${name}`,
  },
} as const;

const storedLocale = localStorage.getItem(LOCALE_STORAGE_KEY);
const locale = ref<Locale>(storedLocale === "en" ? "en" : "zh");
const snapshot = ref<ConfigSnapshot | null>(null);
const selectedId = ref("");
const draftNames = ref<string[]>([]);
const newName = ref("");
const newRuleSource = ref<RuleSource>("local");
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const notice = ref("");
const permissionDenied = ref(false);
const sharedConfig = ref<SharedConfig | null>(null);
const sharedBusy = ref(false);
const sharedError = ref("");
const sharedNotice = ref("");
const managedConfig = ref<ManagedConfig | null>(null);
const cloudDraftNames = ref<string[]>([]);
const syncSettings = ref<SyncSettings>({ pollingIntervalSeconds: 3600 });
const pollingIntervalMinutes = ref(60);
let syncTimer: number | undefined;
let watcherDebounceTimer: number | undefined;
let unlistenSharedConfig: (() => void) | undefined;

const selected = computed(() =>
  snapshot.value?.locations.find((location) => location.id === selectedId.value),
);
const text = computed(() => messages[locale.value]);
const platformName = computed(() => snapshot.value?.platform === "windows" ? "Windows" : "macOS");
const effectiveNames = computed(() => Array.from(new Set([
  ...draftNames.value,
  ...cloudDraftNames.value,
])));
const displayRules = computed<DisplayRule[]>(() => effectiveNames.value.map((name) => ({
  name,
  cloud: cloudDraftNames.value.includes(name),
})));
const hasChanges = computed(() => {
  const original = selected.value?.ignoredFolders ?? [];
  const cloudChanged = managedConfig.value
    ? JSON.stringify(managedConfig.value.cloudFolders) !== JSON.stringify(cloudDraftNames.value)
    : false;
  return cloudChanged || JSON.stringify(original) !== JSON.stringify(effectiveNames.value);
});

function setLocale(next: Locale) {
  locale.value = next;
  localStorage.setItem(LOCALE_STORAGE_KEY, next);
  document.documentElement.lang = next === "zh" ? "zh-CN" : "en";
}

function locationLabel(location: ConfigLocation) {
  if (location.label === messages.zh.systemApp) return text.value.systemApp;
  const syncTaskPrefix = "同步任务 ";
  if (location.label.startsWith(syncTaskPrefix)) {
    return text.value.syncTaskNamed(location.label.slice(syncTaskPrefix.length));
  }
  return location.label;
}

function selectLocation(id: string) {
  selectedId.value = id;
  const location = snapshot.value?.locations.find((item) => item.id === id);
  draftNames.value = [...(location?.ignoredFolders ?? [])];
  cloudDraftNames.value = [];
  managedConfig.value = null;
  newName.value = "";
  newRuleSource.value = "local";
  error.value = "";
  notice.value = "";
  permissionDenied.value = false;
}

async function changeLocation(id: string) {
  selectLocation(id);
  if (sharedConfig.value) await syncManagedConfig(false);
}

async function scan() {
  loading.value = true;
  error.value = "";
  notice.value = "";
  permissionDenied.value = false;
  try {
    snapshot.value = await invoke<ConfigSnapshot>("discover_configs");
    const currentStillExists = snapshot.value.locations.some((item) => item.id === selectedId.value);
    selectLocation(currentStillExists ? selectedId.value : (snapshot.value.locations[0]?.id ?? ""));
    if (sharedConfig.value && selected.value) await syncManagedConfig(false);
  } catch (reason) {
    const message = String(reason);
    permissionDenied.value = message.includes("APP_MANAGEMENT_REQUIRED|");
    error.value = message.replace("APP_MANAGEMENT_REQUIRED|", "");
  } finally {
    loading.value = false;
  }
}

async function openPermissionSettings() {
  try {
    await invoke("open_permission_settings");
  } catch (reason) {
    error.value = String(reason);
  }
}

async function loadSharedConfig() {
  try {
    sharedConfig.value = await invoke<SharedConfig | null>("get_shared_config");
  } catch (reason) {
    sharedError.value = String(reason);
  }
}

async function loadSyncSettings() {
  try {
    syncSettings.value = await invoke<SyncSettings>("get_sync_settings");
    pollingIntervalMinutes.value = Math.round(syncSettings.value.pollingIntervalSeconds / 60);
  } catch (reason) {
    sharedError.value = String(reason);
  }
}

function restartSyncTimer() {
  if (syncTimer !== undefined) window.clearInterval(syncTimer);
  syncTimer = window.setInterval(() => {
    if (sharedConfig.value && selected.value && !hasChanges.value) {
      void syncManagedConfig(false);
    }
  }, syncSettings.value.pollingIntervalSeconds * 1000);
}

function queueWatcherSync() {
  if (!sharedConfig.value || !selected.value || hasChanges.value || saving.value) return;
  if (watcherDebounceTimer !== undefined) window.clearTimeout(watcherDebounceTimer);
  watcherDebounceTimer = window.setTimeout(() => {
    void syncManagedConfig(false);
  }, 700);
}

async function savePollingInterval() {
  const minutes = Math.min(24 * 60, Math.max(1, Math.round(pollingIntervalMinutes.value || 60)));
  pollingIntervalMinutes.value = minutes;
  try {
    syncSettings.value = await invoke<SyncSettings>("update_sync_settings", {
      pollingIntervalSeconds: minutes * 60,
    });
    restartSyncTimer();
    sharedNotice.value = text.value.pollingUpdated(minutes);
  } catch (reason) {
    sharedError.value = String(reason);
  }
}

async function showSyncUpdatedNotification() {
  if (!("Notification" in window)) return;
  try {
    let permission = Notification.permission;
    if (permission === "default") {
      permission = await Notification.requestPermission();
    }
    if (permission === "granted") {
      new Notification("Synology Drive Ignore", {
        body: text.value.notificationBody,
      });
    }
  } catch {
    // Some WebView environments disable system notifications; the in-app notice still updates.
  }
}

function applyManagedConfig(result: ManagedConfig, updateDriveStatus = true) {
  managedConfig.value = result;
  sharedConfig.value = { path: result.path, ignoredFolders: [...result.cloudFolders] };
  draftNames.value = [...result.localFolders];
  cloudDraftNames.value = [...result.cloudFolders];
  if (selected.value) {
    selected.value.ignoredFolders = [...result.effectiveFolders];
  }
  if (snapshot.value && updateDriveStatus) {
    snapshot.value.driveStatus = result.driveStatus;
  }
}

async function syncManagedConfig(showNotice = true, refreshDriveStatus = showNotice) {
  if (!selected.value || !sharedConfig.value || sharedBusy.value || saving.value) return;
  sharedBusy.value = true;
  sharedError.value = "";
  try {
    const result = await invoke<ManagedConfig>("sync_managed_config", {
      targetId: selected.value.id,
      refreshDriveStatus,
    });
    applyManagedConfig(result, result.changed || refreshDriveStatus);
    if (result.changed) {
      sharedNotice.value = text.value.syncUpdated;
      await showSyncUpdatedNotification();
    } else if (showNotice) {
      sharedNotice.value = text.value.syncAlreadySynced;
    }
    if (result.changed && result.driveStatus.running) {
      notice.value = text.value.syncUpdatedRestart;
    } else if (result.changed) {
      notice.value = text.value.syncUpdated;
    }
  } catch (reason) {
    const message = String(reason);
    permissionDenied.value = message.includes("APP_MANAGEMENT_REQUIRED|");
    sharedError.value = message.replace("APP_MANAGEMENT_REQUIRED|", "");
    if (permissionDenied.value) error.value = sharedError.value;
  } finally {
    sharedBusy.value = false;
  }
}

async function chooseSharedConfig() {
  try {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Synology Drive Ignore", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    await connectSharedConfig(path, false);
  } catch (reason) {
    sharedError.value = String(reason);
  }
}

async function createSharedConfig() {
  try {
    const path = await saveDialog({
      defaultPath: "synology-drive-ignore.json",
      filters: [{ name: "Synology Drive Ignore", extensions: ["json"] }],
    });
    if (!path) return;
    await connectSharedConfig(path, true);
  } catch (reason) {
    sharedError.value = String(reason);
  }
}

async function connectSharedConfig(path: string, create: boolean) {
  sharedBusy.value = true;
  sharedError.value = "";
  sharedNotice.value = "";
  try {
    sharedConfig.value = await invoke<SharedConfig>("connect_shared_config", {
      path,
      create,
      names: draftNames.value,
    });
    sharedBusy.value = false;
    await syncManagedConfig(false);
    sharedNotice.value = create
      ? text.value.sharedCreated
      : text.value.sharedConnected(sharedConfig.value.ignoredFolders.length);
  } catch (reason) {
    sharedError.value = String(reason);
  } finally {
    sharedBusy.value = false;
  }
}

async function disconnectSharedConfig() {
  sharedBusy.value = true;
  try {
    await invoke("disconnect_shared_config");
    sharedConfig.value = null;
    managedConfig.value = null;
    cloudDraftNames.value = [];
    newRuleSource.value = "local";
    draftNames.value = [...(selected.value?.ignoredFolders ?? [])];
    sharedError.value = "";
    sharedNotice.value = text.value.sharedDisconnected;
  } catch (reason) {
    sharedError.value = String(reason);
  } finally {
    sharedBusy.value = false;
  }
}

function addFolder() {
  const name = newName.value.trim();
  if (!name) return;
  if (/["\\/,\r\n]/.test(name) || name === "." || name === "..") {
    error.value = text.value.invalidFolder;
    return;
  }
  if (effectiveNames.value.includes(name)) {
    error.value = text.value.alreadyIgnored(name);
    return;
  }
  if (newRuleSource.value === "cloud" && sharedConfig.value) {
    cloudDraftNames.value.push(name);
    sharedNotice.value = text.value.addedCloud(name);
  } else {
    draftNames.value.push(name);
    notice.value = text.value.addedLocal(name);
  }
  newName.value = "";
  error.value = "";
}

function promoteToCloud(rule: DisplayRule) {
  if (rule.cloud || !sharedConfig.value) return;
  const localIndex = draftNames.value.indexOf(rule.name);
  if (localIndex >= 0) draftNames.value.splice(localIndex, 1);
  if (!cloudDraftNames.value.includes(rule.name)) cloudDraftNames.value.push(rule.name);
  notice.value = "";
  sharedNotice.value = text.value.promotedCloud(rule.name);
}

function removeFolder(rule: DisplayRule) {
  const list = rule.cloud ? cloudDraftNames.value : draftNames.value;
  const index = list.indexOf(rule.name);
  if (index >= 0) list.splice(index, 1);
  notice.value = "";
}

async function save() {
  if (!selected.value || !hasChanges.value) return;
  saving.value = true;
  error.value = "";
  notice.value = "";
  permissionDenied.value = false;
  try {
    if (managedConfig.value && sharedConfig.value) {
      const result = await invoke<ManagedConfig>("save_managed_config", {
        targetId: selected.value.id,
        localNames: draftNames.value,
        cloudNames: cloudDraftNames.value,
      });
      applyManagedConfig(result);
      sharedNotice.value = text.value.managedSavedShared;
      if (result.driveStatus.running) {
        notice.value = text.value.managedSavedRunning;
      } else if (result.driveStatus.detectionAvailable) {
        notice.value = text.value.managedSavedStopped;
      } else {
        notice.value = text.value.managedSavedUnknown;
      }
      return;
    }
    const result = await invoke<UpdateResult>("update_ignored_folders", {
      targetId: selected.value.id,
      names: effectiveNames.value,
    });
    const index = snapshot.value!.locations.findIndex((item) => item.id === result.location.id);
    snapshot.value!.locations[index] = result.location;
    snapshot.value!.driveStatus = result.driveStatus;
    draftNames.value = [...result.location.ignoredFolders];
    const resultMessage = locale.value === "zh" ? result.message : text.value.updatedLocalConfig;
    if (result.driveStatus.running) {
      notice.value = text.value.updateRunning(resultMessage);
    } else if (result.driveStatus.detectionAvailable) {
      notice.value = text.value.updateStopped(resultMessage);
    } else {
      notice.value = text.value.updateUnknown(resultMessage);
    }
  } catch (reason) {
    const message = String(reason);
    permissionDenied.value = message.includes("APP_MANAGEMENT_REQUIRED|");
    error.value = message.replace("APP_MANAGEMENT_REQUIRED|", "");
  } finally {
    saving.value = false;
  }
}

onMounted(async () => {
  setLocale(locale.value);
  await loadSyncSettings();
  await scan();
  await loadSharedConfig();
  if (sharedConfig.value && selected.value) await syncManagedConfig(false);
  unlistenSharedConfig = await listen<SharedConfigChangedEvent>("shared-config-changed", (event) => {
    if (event.payload.path === sharedConfig.value?.path) {
      queueWatcherSync();
    }
  });
  restartSyncTimer();
});

onUnmounted(() => {
  if (syncTimer !== undefined) window.clearInterval(syncTimer);
  if (watcherDebounceTimer !== undefined) window.clearTimeout(watcherDebounceTimer);
  if (unlistenSharedConfig) unlistenSharedConfig();
});
</script>

<template>
  <main class="app-shell">
    <header class="topbar">
      <img class="brand-mark" :src="appLogo" alt="Synology Drive Ignore" />
      <div>
        <h1>Synology Drive Ignore</h1>
        <p>{{ text.subtitle }}</p>
      </div>
      <div class="topbar-actions">
        <div class="language-switch" :aria-label="text.languageLabel">
          <button type="button" :class="{ active: locale === 'zh' }" @click="setLocale('zh')">中文</button>
          <button type="button" :class="{ active: locale === 'en' }" @click="setLocale('en')">EN</button>
        </div>
        <span v-if="snapshot" class="platform-badge">{{ platformName }}</span>
      </div>
    </header>

    <section class="workspace">
      <div class="hero">
        <div>
          <span class="eyebrow">{{ text.localConfig }}</span>
          <h2>{{ text.heroTitle }}</h2>
          <p>{{ text.heroDescription }}</p>
        </div>
        <button class="secondary-button" :disabled="loading || saving" @click="scan">
          <svg viewBox="0 0 24 24"><path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 5v6h-6"/></svg>
          {{ loading ? text.scanning : text.rescan }}
        </button>
      </div>

      <aside v-if="snapshot && !loading" class="tip-card drive-status-card" :class="{ 'running-warning': snapshot.driveStatus.running }">
        <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 11v5m0-8h.01"/></svg>
        <p v-if="snapshot.driveStatus.running">
          <strong>{{ text.runningTitle }}</strong>{{ text.runningText }}
          <span v-if="snapshot.driveStatus.processes.length" class="process-list">{{ text.detected }}{{ snapshot.driveStatus.processes.join(locale === "zh" ? "、" : ", ") }}</span>
        </p>
        <p v-else-if="snapshot.driveStatus.detectionAvailable"><strong>{{ text.stoppedTitle }}</strong>{{ text.stoppedText }}</p>
        <p v-else><strong>{{ text.unknownTitle }}</strong>{{ text.unknownText }}</p>
      </aside>

      <div v-if="loading" class="state-card loading-state">
        <span class="spinner"></span>
        <div><strong>{{ text.loadingTitle }}</strong><p>{{ text.loadingText }}</p></div>
      </div>

      <div v-else-if="!snapshot?.locations.length" class="state-card empty-state">
        <div class="empty-icon"><svg viewBox="0 0 24 24"><path d="M4 19.5V7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v10.5"/><path d="m9 13 6 6m0-6-6 6"/></svg></div>
        <div>
          <strong>{{ text.emptyTitle }}</strong>
          <p>{{ text.emptyText }}</p>
          <details v-if="snapshot?.searchedPaths.length">
            <summary>{{ text.checkedLocations }}</summary>
            <code v-for="path in snapshot.searchedPaths" :key="path">{{ path }}</code>
          </details>
        </div>
      </div>

      <template v-else>
        <div v-if="snapshot.locations.length > 1" class="target-tabs" :aria-label="text.syncTask">
          <button v-for="location in snapshot.locations" :key="location.id"
            :class="{ active: selectedId === location.id }" @click="changeLocation(location.id)">
            <span class="status-dot"></span>{{ locationLabel(location) }}
          </button>
        </div>

        <section class="sync-card">
          <div class="sync-icon">
            <svg viewBox="0 0 24 24"><path d="M7 18.5H6a4 4 0 0 1-.7-7.9A7 7 0 0 1 19 12a3.3 3.3 0 0 1-.3 6.5H17"/><path d="m9 15 3-3 3 3m-3-3v8"/></svg>
          </div>
          <div class="sync-content">
            <div class="sync-heading">
              <div>
                <h3>{{ text.sharedConfig }}</h3>
                <p v-if="sharedConfig" class="shared-path" :title="sharedConfig.path">{{ sharedConfig.path }}</p>
                <p v-else>{{ text.sharedIntro }}</p>
              </div>
              <span v-if="sharedConfig" class="sync-count managed-badge"><span></span>{{ text.managedBadge(cloudDraftNames.length) }}</span>
            </div>

            <div class="sync-guide">
              <strong>{{ sharedConfig ? text.guideManagedTitle : text.guideIntroTitle }}</strong>
              <p v-if="sharedConfig">{{ text.guideManagedText }}</p>
              <p v-else>{{ text.guideIntroText }}</p>
              <div v-if="!sharedConfig" class="sync-steps">
                <span><b>1</b> {{ text.step1 }}</span>
                <span><b>2</b> {{ text.step2 }}</span>
                <span><b>3</b> {{ text.step3 }}</span>
              </div>
              <form v-else class="polling-settings" @submit.prevent="savePollingInterval">
                <label for="polling-interval">{{ text.pollingInterval }}</label>
                <input id="polling-interval" v-model.number="pollingIntervalMinutes" type="number" min="1" max="1440" step="1" />
                <span>{{ text.minutes }}</span>
                <button type="submit" :disabled="sharedBusy">{{ text.save }}</button>
              </form>
            </div>

            <div v-if="sharedConfig" class="cloud-rules">
              <div class="cloud-rules-heading">
                <strong>{{ text.cloudRulesTitle }}</strong>
                <small>{{ text.autoApply }}</small>
              </div>
              <div v-if="cloudDraftNames.length" class="cloud-rule-list">
                <span v-for="name in cloudDraftNames" :key="name">{{ name }}</span>
              </div>
              <div v-else class="cloud-rules-empty">{{ text.noCloudRules }}</div>
              <p class="merge-explanation"><b>{{ text.sourceRules }}</b>{{ text.mergeExplanation }}</p>
            </div>

            <div v-if="sharedConfig" class="sync-actions">
              <button class="primary-sync" :disabled="sharedBusy || hasChanges" @click="syncManagedConfig()">{{ text.checkCloudUpdates }}</button>
              <button class="link-button" :disabled="sharedBusy" @click="chooseSharedConfig">{{ text.changeFile }}</button>
              <button class="link-button danger-link" :disabled="sharedBusy" @click="disconnectSharedConfig">{{ text.disconnect }}</button>
            </div>
            <div v-else class="sync-actions">
              <button class="primary-sync" :disabled="sharedBusy" @click="chooseSharedConfig">{{ text.chooseExisting }}</button>
              <button :disabled="sharedBusy" @click="createSharedConfig">{{ text.createFromCurrent }}</button>
            </div>

            <p v-if="sharedError" class="sync-feedback sync-error">{{ sharedError }}</p>
            <p v-if="sharedNotice" class="sync-feedback sync-success">{{ sharedNotice }}</p>
          </div>
        </section>

        <section v-if="selected" class="config-card">
          <div class="card-header">
            <div>
              <div class="title-row"><span class="status-dot"></span><h3>{{ locationLabel(selected) }}</h3><span class="found-badge">{{ text.found }}</span></div>
              <p class="config-path" :title="selected.path">{{ selected.path }}</p>
            </div>
            <span class="count-badge">{{ text.mergedCount(displayRules.length) }}</span>
          </div>

          <div class="rule-area">
            <div class="section-label"><span>{{ text.localRulesTitle }}</span><small>{{ text.effectiveHint }}</small></div>
            <div v-if="displayRules.length" class="rules-list">
              <div v-for="rule in displayRules" :key="rule.name" class="rule-row">
                <div class="folder-icon"><svg viewBox="0 0 24 24"><path d="M3.5 7.5v9A2.5 2.5 0 0 0 6 19h12a2.5 2.5 0 0 0 2.5-2.5v-7A2.5 2.5 0 0 0 18 7h-6l-2-2H6a2.5 2.5 0 0 0-2.5 2.5Z"/></svg></div>
                <span>{{ rule.name }}</span>
                <span v-if="rule.cloud" class="cloud-source" :title="text.fromCloud">
                  <svg viewBox="0 0 24 24"><path d="M7 18h10a3.5 3.5 0 0 0 .5-7A6 6 0 0 0 6 9.5 4.3 4.3 0 0 0 7 18Z"/></svg>
                  {{ text.cloudConfig }}
                </span>
                <button v-else-if="sharedConfig" class="promote-button" type="button"
                  :aria-label="text.promoteAria(rule.name)" :title="text.promoteTitle"
                  @click="promoteToCloud(rule)">
                  <svg viewBox="0 0 24 24"><path d="M7 18h10a3.5 3.5 0 0 0 .5-7A6 6 0 0 0 6 9.5 4.3 4.3 0 0 0 7 18Z"/><path d="m9 14 3-3 3 3m-3-3v6"/></svg>
                  {{ text.promote }}
                </button>
                <button class="delete-button" :aria-label="text.deleteAria(rule.name)" @click="removeFolder(rule)">
                  <svg viewBox="0 0 24 24"><path d="M5 7h14M9 7V4h6v3m-8 0 1 13h8l1-13M10 11v5m4-5v5"/></svg>
                </button>
              </div>
            </div>
            <div v-else class="no-rules">{{ text.noRules }}</div>

            <div class="add-rule-panel">
              <div class="source-picker" :aria-label="text.newRuleSource">
                <span>{{ text.addAs }}</span>
                <button type="button" :class="{ active: newRuleSource === 'local' }" @click="newRuleSource = 'local'">{{ text.localRule }}</button>
                <button type="button" :class="{ active: newRuleSource === 'cloud' }" :disabled="!sharedConfig"
                  :title="sharedConfig ? text.cloudRuleEnabledTitle : text.cloudRuleDisabledTitle"
                  @click="newRuleSource = 'cloud'">
                  <svg viewBox="0 0 24 24"><path d="M7 18h10a3.5 3.5 0 0 0 .5-7A6 6 0 0 0 6 9.5 4.3 4.3 0 0 0 7 18Z"/></svg>
                  {{ text.cloudRule }}
                </button>
                <small v-if="!sharedConfig">{{ text.connectRequired }}</small>
                <small v-else>{{ newRuleSource === "cloud" ? text.cloudRuleHint : text.localRuleHint }}</small>
              </div>
              <form class="add-row" @submit.prevent="addFolder">
                <div class="input-wrap">
                  <svg viewBox="0 0 24 24"><path d="M3.5 8v8.5A2.5 2.5 0 0 0 6 19h12a2.5 2.5 0 0 0 2.5-2.5v-7A2.5 2.5 0 0 0 18 7h-6l-2-2H6a2.5 2.5 0 0 0-2.5 2.5"/></svg>
                  <input v-model="newName" :placeholder="newRuleSource === 'cloud' ? text.cloudPlaceholder : text.localPlaceholder" autocomplete="off" />
                </div>
                <button class="add-button" type="submit" :disabled="!newName.trim()">{{ text.add }}</button>
              </form>
            </div>
          </div>

          <div v-if="permissionDenied" class="message permission-message">
            <div class="permission-copy">
              <strong>{{ text.permissionTitle }}</strong>
              <span>{{ error }}</span>
              <small>{{ text.permissionHelp }}</small>
            </div>
            <button type="button" @click="openPermissionSettings">{{ text.openSettings }}</button>
          </div>
          <div v-else-if="error" class="message error-message">{{ error }}</div>
          <div v-if="notice" class="message" :class="snapshot.driveStatus.running ? 'restart-message' : 'success-message'">{{ notice }}</div>

          <footer class="card-footer">
            <div class="file-note">
              <svg viewBox="0 0 24 24"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z"/><path d="M14 3v6h6"/></svg>
              <span>{{ text.writeFiles(selected.relatedFiles.length) }}</span>
            </div>
            <button class="save-button" :disabled="!hasChanges || saving" @click="save">
              {{ saving ? text.saving : hasChanges ? text.saveChanges : text.upToDate }}
            </button>
          </footer>
        </section>

      </template>

      <div v-if="error && !selected" class="message error-message page-message">{{ error }}</div>
    </section>
  </main>
</template>

<style>
:root {
  font-family: Inter, "SF Pro Display", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif;
  color: #172136;
  background: #f3f6fa;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}
* { box-sizing: border-box; }
body { margin: 0; min-width: 620px; min-height: 100vh; }
button, input { font: inherit; }
button { cursor: pointer; }
button:disabled { cursor: default; }
svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.app-shell { min-height: 100vh; background: radial-gradient(circle at 70% 0%, #fff 0, transparent 35%), #f3f6fa; }
.topbar { height: 76px; padding: 0 38px; display: flex; align-items: center; gap: 13px; color: #fff; background: linear-gradient(108deg, #1874e8 0%, #2463d5 56%, #174ba9 100%); box-shadow: 0 2px 12px #0b418536; }
.brand-mark { width: 46px; height: 46px; flex: none; object-fit: contain; filter: drop-shadow(0 5px 8px #0a38885c); }
.topbar h1 { margin: 0; font-size: 18px; line-height: 22px; letter-spacing: -.2px; }
.topbar p { margin: 2px 0 0; font-size: 12px; opacity: .72; }
.topbar-actions { margin-left: auto; display: flex; align-items: center; gap: 10px; }
.language-switch { display: inline-flex; align-items: center; padding: 2px; border: 1px solid #ffffff3d; border-radius: 999px; background: #ffffff18; }
.language-switch button { height: 24px; padding: 0 9px; color: #ffffffb8; border: 0; border-radius: 999px; background: transparent; font-size: 11px; font-weight: 700; }
.language-switch button.active { color: #1458b4; background: #fff; box-shadow: 0 2px 8px #0a388833; }
.platform-badge { padding: 5px 10px; border: 1px solid #ffffff45; border-radius: 999px; font-size: 12px; background: #ffffff18; }
.workspace { width: min(900px, calc(100% - 64px)); margin: 0 auto; padding: 46px 0 48px; }
.hero { display: flex; align-items: end; justify-content: space-between; gap: 24px; margin-bottom: 28px; }
.eyebrow { display: block; margin-bottom: 8px; color: #2871d8; font-size: 11px; font-weight: 750; letter-spacing: 1.8px; }
.hero h2 { margin: 0; font-size: 30px; line-height: 1.2; letter-spacing: -.8px; color: #17233a; }
.hero p { max-width: 630px; margin: 10px 0 0; color: #708097; font-size: 14px; line-height: 1.7; }
.secondary-button { height: 38px; flex: none; display: flex; align-items: center; gap: 8px; padding: 0 14px; color: #446078; border: 1px solid #d6dee8; border-radius: 9px; background: #fff; box-shadow: 0 2px 7px #24364c0a; }
.secondary-button:hover:not(:disabled) { border-color: #9bb5d4; color: #2167c5; }
.secondary-button svg { width: 16px; }
.state-card, .config-card, .sync-card { border: 1px solid #dde4ec; border-radius: 14px; background: #fff; box-shadow: 0 9px 30px #2744640d; }
.state-card { min-height: 180px; padding: 38px; display: flex; align-items: center; justify-content: center; gap: 18px; color: #66778b; }
.state-card strong { display: block; color: #24354b; }
.state-card p { margin: 5px 0; font-size: 13px; }
.spinner { width: 24px; height: 24px; border: 2px solid #d6e3f5; border-top-color: #2672dc; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.empty-state { align-items: flex-start; justify-content: flex-start; }
.empty-icon { width: 44px; height: 44px; display: grid; place-items: center; flex: none; color: #7d91a9; border-radius: 12px; background: #f0f4f8; }
.empty-icon svg { width: 25px; }
details { margin-top: 14px; font-size: 12px; }
details summary { cursor: pointer; color: #47709e; }
details code { display: block; margin-top: 8px; white-space: normal; word-break: break-all; color: #738196; }
.target-tabs { display: flex; gap: 8px; margin-bottom: 12px; overflow-x: auto; }
.target-tabs button { display: flex; align-items: center; gap: 7px; padding: 9px 13px; color: #6f8094; border: 1px solid #dfe5ec; border-radius: 9px; background: #f8fafc; white-space: nowrap; }
.target-tabs button.active { color: #2169ce; border-color: #a9c8ef; background: #ecf4ff; }
.status-dot { width: 7px; height: 7px; flex: none; border-radius: 50%; background: #31b36b; box-shadow: 0 0 0 3px #31b36b20; }
.sync-card { margin-bottom: 14px; padding: 18px 20px; display: flex; align-items: flex-start; gap: 15px; }
.sync-icon { width: 38px; height: 38px; display: grid; place-items: center; flex: none; color: #2b70d2; border-radius: 10px; background: #eaf3ff; }
.sync-icon svg { width: 22px; }
.sync-content { min-width: 0; flex: 1; }
.sync-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
.sync-heading h3 { margin: 1px 0 0; color: #2c3d53; font-size: 14px; }
.sync-heading p { margin: 4px 0 0; color: #8795a7; font-size: 11px; line-height: 1.55; }
.shared-path { max-width: 620px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.sync-count { flex: none; padding: 4px 8px; color: #3970ad; border-radius: 7px; background: #edf5ff; font-size: 10px; }
.managed-badge { display: inline-flex; align-items: center; gap: 5px; color: #24734d; background: #eaf8f0; }
.managed-badge > span { width: 6px; height: 6px; border-radius: 50%; background: #2ea866; box-shadow: 0 0 0 3px #2ea8661c; }
.sync-guide { margin-top: 13px; padding: 11px 12px; color: #67798e; border: 1px solid #e4ebf3; border-radius: 8px; background: #f8fafc; }
.sync-guide > strong { display: block; margin-bottom: 3px; color: #3e536b; font-size: 11px; }
.sync-guide > p { margin: 0; font-size: 10px; line-height: 1.6; }
.sync-steps { display: flex; flex-wrap: wrap; gap: 6px 18px; margin-top: 8px; }
.sync-steps span { display: flex; align-items: center; gap: 5px; color: #73859a; font-size: 10px; }
.sync-steps b { width: 16px; height: 16px; display: inline-grid; place-items: center; flex: none; color: #2b6fc9; border-radius: 50%; background: #e4effd; font-size: 9px; }
.polling-settings { display: flex; flex-wrap: wrap; align-items: center; gap: 7px; margin-top: 10px; color: #6f8195; font-size: 10px; }
.polling-settings label { color: #485f78; font-weight: 650; }
.polling-settings input { width: 70px; height: 28px; padding: 0 8px; color: #26364a; border: 1px solid #d5e0ec; border-radius: 7px; outline: 0; background: #fff; font-size: 11px; }
.polling-settings input:focus { border-color: #8ab6e9; box-shadow: 0 0 0 3px #2877df12; }
.polling-settings button { height: 28px; padding: 0 9px; color: #356da9; border: 1px solid #cfe0f4; border-radius: 7px; background: #fff; font-size: 10px; }
.polling-settings button:hover:not(:disabled) { background: #edf5ff; }
.cloud-rules { margin-top: 11px; padding: 12px; border: 1px solid #dce7f4; border-radius: 9px; background: #fbfdff; }
.cloud-rules-heading { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
.cloud-rules-heading strong { color: #38516c; font-size: 11px; }
.cloud-rules-heading small { color: #91a0b1; font-size: 9px; }
.cloud-rule-list { max-height: 112px; margin-top: 9px; display: flex; flex-wrap: wrap; gap: 6px; overflow-y: auto; }
.cloud-rule-list span { padding: 4px 8px; color: #2866ad; border: 1px solid #cfe0f4; border-radius: 6px; background: #edf5ff; font: 10px/1.4 ui-monospace, SFMono-Regular, Menlo, monospace; }
.cloud-rules-empty { margin-top: 8px; padding: 9px; color: #97a4b3; border: 1px dashed #dce4ed; border-radius: 6px; text-align: center; font-size: 10px; }
.merge-explanation { margin: 10px 0 0; padding-top: 9px; color: #6f8195; border-top: 1px solid #e8eef5; font-size: 10px; line-height: 1.55; }
.merge-explanation b { color: #45617e; }
.sync-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 13px; }
.sync-actions button { height: 32px; padding: 0 11px; color: #51677f; border: 1px solid #d7e0e9; border-radius: 7px; background: #fff; font-size: 11px; font-weight: 620; }
.sync-actions button:hover:not(:disabled) { color: #2267c7; border-color: #9dbde5; }
.sync-actions button:disabled { opacity: .55; }
.sync-actions .primary-sync { color: #fff; border-color: #2871d4; background: #2871d4; }
.sync-actions .primary-sync:hover:not(:disabled) { color: #fff; border-color: #1f61bd; background: #1f67c9; }
.sync-actions .link-button { padding: 0 4px; border-color: transparent; background: transparent; }
.sync-actions .danger-link:hover:not(:disabled) { color: #c64949; border-color: transparent; }
.sync-feedback { margin: 10px 0 0; padding: 7px 9px; border-radius: 6px; font-size: 10px; line-height: 1.5; }
.sync-error { color: #a63838; background: #fff0f0; }
.sync-success { color: #24734d; background: #eaf8f0; }
.card-header { min-height: 88px; padding: 20px 24px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid #edf0f4; }
.title-row { display: flex; align-items: center; gap: 9px; }
.title-row h3 { margin: 0; font-size: 16px; }
.found-badge { padding: 2px 7px; color: #238555; border-radius: 999px; font-size: 10px; font-weight: 650; background: #e8f7ef; }
.config-path { max-width: 670px; margin: 7px 0 0 16px; color: #8996a8; font: 11px/1.4 ui-monospace, SFMono-Regular, Menlo, monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.count-badge { flex: none; padding: 5px 9px; color: #607387; border-radius: 7px; font-size: 11px; background: #f1f4f7; }
.rule-area { padding: 24px; }
.section-label { margin-bottom: 12px; display: flex; justify-content: space-between; align-items: baseline; }
.section-label span { font-size: 13px; font-weight: 700; color: #3d4b5f; }
.section-label small { color: #96a2b1; font-size: 11px; }
.rules-list { border: 1px solid #e3e8ee; border-radius: 10px; overflow: hidden; }
.rule-row { min-height: 50px; padding: 0 11px 0 14px; display: flex; align-items: center; gap: 12px; border-bottom: 1px solid #edf1f4; background: #fff; }
.rule-row:last-child { border-bottom: 0; }
.rule-row:hover { background: #fbfcfe; }
.rule-row > span { flex: 1; color: #2c3a4d; font: 13px ui-monospace, SFMono-Regular, Menlo, monospace; }
.rule-row > .cloud-source { flex: none; display: inline-flex; align-items: center; gap: 4px; padding: 3px 7px; color: #2d70bd; border-radius: 999px; background: #eaf3ff; font: 9px/1.4 Inter, "PingFang SC", sans-serif; }
.cloud-source svg { width: 12px; }
.promote-button { height: 28px; flex: none; display: inline-flex; align-items: center; gap: 5px; padding: 0 8px; color: #3673b9; border: 1px solid #cfe0f4; border-radius: 7px; background: #f6faff; font-size: 10px; }
.promote-button:hover { color: #1f62b1; border-color: #9fc2ea; background: #eaf3ff; }
.promote-button svg { width: 14px; }
.folder-icon { width: 28px; height: 28px; display: grid; place-items: center; color: #2a73d5; border-radius: 7px; background: #eaf3ff; }
.folder-icon svg { width: 17px; }
.delete-button { width: 31px; height: 31px; display: grid; place-items: center; color: #a4aeba; border: 0; border-radius: 7px; background: transparent; }
.delete-button:hover { color: #d94a4a; background: #fff0f0; }
.delete-button svg { width: 16px; }
.no-rules { padding: 22px; text-align: center; color: #9aa6b4; border: 1px dashed #dce3eb; border-radius: 10px; font-size: 13px; }
.add-rule-panel { margin-top: 14px; padding: 12px; border: 1px solid #e2e8ef; border-radius: 10px; background: #f9fbfd; }
.source-picker { display: flex; align-items: center; gap: 7px; margin-bottom: 10px; }
.source-picker > span { margin-right: 2px; color: #65778c; font-size: 11px; font-weight: 650; }
.source-picker button { height: 27px; display: inline-flex; align-items: center; gap: 4px; padding: 0 9px; color: #60758c; border: 1px solid #d9e2ec; border-radius: 7px; background: #fff; font-size: 10px; }
.source-picker button.active { color: #2168c8; border-color: #a9c9ef; background: #eaf3ff; box-shadow: 0 0 0 2px #2877df0c; }
.source-picker button:disabled { color: #abb5c1; border-color: #e4e9ef; background: #f2f4f7; }
.source-picker button svg { width: 13px; }
.source-picker small { margin-left: auto; color: #8c9aab; font-size: 10px; }
.add-row { display: flex; gap: 10px; }
.input-wrap { height: 42px; flex: 1; display: flex; align-items: center; gap: 10px; padding: 0 13px; color: #9aa9ba; border: 1px solid #dbe2ea; border-radius: 9px; background: #fbfcfd; transition: .18s; }
.input-wrap:focus-within { color: #3376d2; border-color: #7facE9; background: #fff; box-shadow: 0 0 0 3px #2877df15; }
.input-wrap svg { width: 17px; flex: none; }
.input-wrap input { width: 100%; color: #26364a; border: 0; outline: 0; background: transparent; font-size: 13px; }
.input-wrap input::placeholder { color: #a8b3c0; }
.add-button, .save-button { border: 0; border-radius: 9px; font-weight: 650; }
.add-button { width: 74px; color: #1761c5; background: #eaf3ff; }
.add-button:hover:not(:disabled) { background: #dcecff; }
.add-button:disabled { color: #9baabe; background: #eff2f5; }
.message { margin: 0 24px 16px; padding: 10px 12px; border-radius: 8px; font-size: 12px; }
.error-message { color: #a63838; background: #fff0f0; }
.success-message { color: #24734d; background: #eaf8f0; }
.restart-message { color: #82570a; border: 1px solid #efd694; background: #fff8e6; }
.permission-message { display: flex; align-items: center; justify-content: space-between; gap: 18px; color: #81580c; border: 1px solid #f0d89e; background: #fff9e9; }
.permission-copy { display: flex; flex-direction: column; gap: 3px; }
.permission-copy strong { color: #654407; font-size: 13px; }
.permission-copy span { line-height: 1.55; }
.permission-copy small { color: #9a762e; line-height: 1.5; }
.permission-copy code { font-size: 10px; }
.permission-message button { flex: none; padding: 7px 10px; color: #79520b; border: 1px solid #e2c77f; border-radius: 7px; background: #fff; font-size: 11px; }
.permission-message button:hover { border-color: #c99e39; }
.page-message { margin: 16px 0 0; }
.card-footer { min-height: 72px; padding: 14px 24px; display: flex; align-items: center; justify-content: space-between; border-top: 1px solid #edf0f4; background: #fafbfd; border-radius: 0 0 14px 14px; }
.file-note { display: flex; align-items: center; gap: 8px; color: #8795a7; font-size: 11px; }
.file-note svg { width: 16px; }
.save-button { min-width: 110px; height: 39px; padding: 0 17px; color: #fff; background: linear-gradient(#2e7ae1, #2168ce); box-shadow: 0 4px 10px #2168ce33; }
.save-button:hover:not(:disabled) { filter: brightness(1.05); }
.save-button:disabled { color: #8998a8; background: #e6ebf0; box-shadow: none; }
.tip-card { margin-top: 14px; padding: 13px 16px; display: flex; align-items: flex-start; gap: 10px; color: #66788d; border: 1px solid #dae6f4; border-radius: 10px; background: #f7fbff; }
.drive-status-card { margin: -8px 0 18px; }
.tip-card svg { width: 17px; flex: none; margin-top: 1px; color: #3177d6; }
.tip-card p { margin: 0; font-size: 11px; line-height: 1.65; }
.tip-card strong { color: #4a6079; }
.tip-card.running-warning { color: #7c5a1d; border-color: #ead49e; background: #fff9ea; }
.tip-card.running-warning svg { color: #c18216; }
.tip-card.running-warning strong { color: #68480f; }
.process-list { display: block; margin-top: 2px; color: #9a793e; font: 10px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace; }
@media (max-width: 700px) {
  .workspace { width: calc(100% - 36px); padding-top: 32px; }
  .topbar { padding: 0 22px; }
  .hero h2 { font-size: 25px; }
  .section-label small { display: none; }
}
</style>
