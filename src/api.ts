import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export type Loader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";
export type ContentKind = "mod" | "resourcepack" | "shader";

export interface Art {
  kind: "url" | "file";
  value: string;
}

export interface Instance {
  id: string;
  name: string;
  mcVersion: string;
  loader: Loader;
  loaderVersion?: string | null;
  art?: Art | null;
  memoryMb?: number | null;
  jvmArgs?: string | null;
  created: number;
  lastPlayed?: number | null;
  playSeconds: number;
  width?: number | null;
  height?: number | null;
  fullscreen?: boolean;
  server?: string | null;
  javaPath?: string | null;
  pinned?: boolean;
}

export interface CrashHint {
  kind: "memory" | "java" | "missingMod" | "mixin" | "duplicate" | "driver" | "modError";
  detail: string;
}

export interface PublicAccount {
  id: string;
  name: string;
  kind: "msa" | "offline";
  uuid: string;
  skinUrl?: string | null;
  needsName: boolean;
  active: boolean;
}

export interface AccountsView {
  accounts: PublicAccount[];
  hasLicense: boolean;
  clientIdSet: boolean;
}

export interface DeviceCode {
  userCode: string;
  deviceCode: string;
  verificationUri: string;
  interval: number;
  expiresIn: number;
}

export interface GameVersion {
  id: string;
  kind: "release" | "snapshot";
}

export interface NewsItem {
  id: string;
  title: string;
  tag: string;
  date: string;
  image?: string | null;
  link?: string | null;
  text: string;
}

export interface Feed {
  news: NewsItem[];
  patchNotes: NewsItem[];
}

export interface LocalProfile {
  nick: string;
  avatar?: string | null;
  frame: string;
}

export interface Hit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  iconUrl?: string | null;
  downloads: number;
  author: string;
  categories: string[];
}

export interface InstalledContent {
  filename: string;
  enabled: boolean;
  title: string;
  projectId?: string | null;
  versionNumber?: string | null;
  icon?: string | null;
  size: number;
}

export interface ContentVersion {
  id: string;
  name: string;
  versionNumber: string;
  versionType: string;
  date: string;
  gameVersions: string[];
  loaders: string[];
}

export interface ContentUpdate {
  kind: ContentKind;
  filename: string;
  title: string;
  from: string;
  to: string;
  versionId: string;
}

export interface Shot {
  path: string;
  name: string;
  instanceId: string;
  instanceName: string;
  modified: number;
  size: number;
}

export interface World {
  name: string;
  size: number;
  modified: number;
  icon?: string | null;
}

export interface Backup {
  file: string;
  world: string;
  created: number;
  size: number;
}

export interface SavedSkin {
  id: string;
  name: string;
  file: string;
  variant: "classic" | "slim";
  added: number;
}

export interface Cape {
  id: string;
  alias: string;
  url: string;
  active: boolean;
}

export interface SkinProfile {
  skinUrl?: string | null;
  variant: string;
  capes: Cape[];
}

export interface CustomTheme {
  id: string;
  name: string;
  base: string;
  vars: Record<string, string>;
  radius: number;
  border: number;
  glow: boolean;
  soundPack?: string;
}

export interface ModProblem {
  severity: "error" | "warning";
  kind: string;
  modName: string;
  modFile: string;
  other: string;
  detail: string;
}

export interface Snapshot {
  file: string;
  created: number;
  reason: string;
  label: string;
  size: number;
  mods: number;
}

export interface Session {
  instanceId: string;
  start: number;
  end: number;
}

export interface QuickPlay {
  kind: "world" | "server";
  target: string;
}

export interface QuickItem extends QuickPlay {
  instanceId: string;
  instanceName: string;
  time: number;
  icon?: string | null;
}

export interface ControlProfile {
  name: string;
  keys: Record<string, string>;
  created: number;
  from: string;
}

export interface SharedFound {
  projectId: string;
  slug: string;
  title: string;
  icon?: string | null;
  projectType: string;
  source: string;
  available: boolean;
}

export interface DropInfo {
  path: string;
  name: string;
  kind: "mod" | "resourcepack" | "shader" | "world" | "modpack" | "unknown";
  loaders: string[];
  gameVersions: string[];
  suggested: string[];
}

export interface Background {
  mode: "art" | "live" | "shots" | "custom";
  file?: string | null;
  weather: "auto" | "clear" | "rain" | "snow" | "fireflies";
}

export type HomeBlock = "quick" | "builds" | "widgets" | "news";
export type NavPage = "home" | "builds" | "mods" | "skins" | "shots" | "news";

/** Look and behaviour of the launcher itself. */
export interface UiPrefs {
  scale: number;
  accent: string | null;
  radius: number | null;
  bodyFont: "inter" | "pixel" | "system";
  sidebar: "full" | "compact";
  hiddenNav: NavPage[];
  startPage: NavPage | "last";
  lastPage: NavPage;
  heroHeight: "compact" | "normal" | "tall";
  heroShade: number;
  homeOrder: HomeBlock[];
  homeHidden: HomeBlock[];
  homeBuilds: number;
  playLabel: string;
  toastPos: "br" | "tr" | "bl" | "tl";
  toastSeconds: number;
  hoverSounds: boolean;
  clickSounds: boolean;
  confirmDelete: boolean;
  consoleOnLaunch: boolean;
  closeToTray: boolean;
  autostart: boolean;
  autoUpdateCheck: boolean;
  notifications: boolean;
  glass: boolean;
}

/** Which celebrations show and how. */
export interface EventPrefs {
  newyear: boolean;
  halloween: boolean;
  birthday: boolean;
  particles: number;
  decor: boolean;
  everywhere: boolean;
}

export const defaultUi: UiPrefs = {
  scale: 1,
  accent: null,
  radius: null,
  bodyFont: "inter",
  sidebar: "full",
  hiddenNav: [],
  startPage: "home",
  lastPage: "home",
  heroHeight: "normal",
  heroShade: 0.7,
  homeOrder: ["quick", "builds", "widgets", "news"],
  homeHidden: [],
  homeBuilds: 3,
  playLabel: "",
  toastPos: "br",
  toastSeconds: 3.5,
  hoverSounds: true,
  clickSounds: true,
  confirmDelete: true,
  consoleOnLaunch: false,
  closeToTray: false,
  autostart: false,
  autoUpdateCheck: true,
  notifications: true,
  glass: false,
};

/** What the game reports through AniLauncher Link. */
export interface LinkWorld {
  state: "enter" | "leave";
  kind?: "singleplayer" | "multiplayer" | "other";
  id?: string;
  name?: string;
}
export interface ShotMeta {
  world?: string;
  kind?: string;
  x?: number;
  y?: number;
  z?: number;
  dim?: string;
}
export interface GameOptions {
  fov: number;
  renderDistance: number;
  simulationDistance: number;
  guiScale: number;
  gamma: number;
  maxFps: number;
  vsync: boolean;
  master: number;
  music: number;
}
export interface LinkPrefs {
  mirrorToasts: boolean;
  notifyUpdates: boolean;
  notifyProblems: boolean;
  packsLive: boolean;
}
export interface UpdateInfo {
  version: string;
  current: string;
  notes?: string | null;
  date?: string | null;
}

export interface DiscordPrefs {
  enabled: boolean;
  appId: string;
  showIdle: boolean;
  showBuild: boolean;
  showWorld: boolean;
  showTime: boolean;
}
export const defaultDiscord: DiscordPrefs = { enabled: true, appId: "", showIdle: true, showBuild: true, showWorld: true, showTime: true };

export const defaultLink: LinkPrefs = { mirrorToasts: true, notifyUpdates: true, notifyProblems: true, packsLive: true };

export const defaultEvents: EventPrefs = { newyear: true, halloween: true, birthday: true, particles: 1, decor: true, everywhere: true };

export interface Settings {
  lang: "ru" | "en";
  theme: string;
  sounds: boolean;
  animations: boolean;
  memoryMb: number;
  clientId: string;
  fontPath?: string | null;
  profile: LocalProfile;
  selectedInstance?: string | null;
  edition: "java" | "bedrock";
  autoUpdateMods: boolean;
  customThemes: CustomTheme[];
  onLaunch: "keep" | "minimize" | "hide";
  background: Background;
  soundPack: string;
  volume: number;
  ambient: boolean;
  checkConflicts: boolean;
  widgets: { week: boolean; shots: boolean; updates: boolean; calendar: boolean; quick?: boolean };
  seasonal: boolean;
  birthday?: string | null;
  streamer: boolean;
  streamerNote: string;
  baseMods: Record<string, Array<{ projectId: string; title: string; icon?: string | null }>>;
  ui: UiPrefs;
  events: EventPrefs;
  globalJvmArgs: string;
  companion: boolean;
  link: LinkPrefs;
  discord: DiscordPrefs;
  dataDir?: string;
  builtInClientId?: boolean;
}

export const defaultSettings: Settings = {
  lang: "ru",
  theme: "official",
  sounds: true,
  animations: true,
  memoryMb: 4096,
  clientId: "",
  fontPath: null,
  profile: { nick: "", avatar: null, frame: "gold" },
  selectedInstance: null,
  edition: "java",
  autoUpdateMods: false,
  customThemes: [],
  onLaunch: "keep",
  background: { mode: "art", file: null, weather: "auto" },
  soundPack: "theme",
  volume: 0.7,
  ambient: true,
  checkConflicts: true,
  widgets: { week: true, shots: true, updates: true, calendar: true, quick: true },
  seasonal: true,
  birthday: null,
  streamer: false,
  streamerNote: "",
  baseMods: {},
  ui: defaultUi,
  events: defaultEvents,
  globalJvmArgs: "",
  companion: true,
  link: defaultLink,
  discord: defaultDiscord,
};

export const api = {
  getSettings: () => invoke<Partial<Settings>>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  importMedia: (path: string) => invoke<string>("import_media", { path }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),

  listAccounts: () => invoke<AccountsView>("list_accounts"),
  loginStart: () => invoke<DeviceCode>("login_start"),
  loginWindow: () => invoke<AccountsView>("login_window"),
  loginPoll: (deviceCode: string, interval: number) =>
    invoke<AccountsView>("login_poll", { deviceCode, interval }),
  nameAvailable: (accountId: string, name: string) => invoke<boolean>("name_available", { accountId, name }),
  setMinecraftName: (accountId: string, name: string) =>
    invoke<AccountsView>("set_minecraft_name", { accountId, name }),
  addOffline: (name: string) => invoke<AccountsView>("add_offline", { name }),
  setActiveAccount: (accountId: string) => invoke<AccountsView>("set_active_account", { accountId }),
  removeAccount: (accountId: string) => invoke<AccountsView>("remove_account", { accountId }),

  listGameVersions: (loader: Loader) => invoke<GameVersion[]>("list_game_versions", { loader }),
  listLoaderVersions: (loader: Loader, mc: string) => invoke<string[]>("list_loader_versions", { loader, mc }),

  listInstances: () => invoke<Instance[]>("list_instances"),
  createInstance: (data: { name: string; mcVersion: string; loader: Loader; loaderVersion?: string | null; art?: Art | null }) =>
    invoke<Instance>("create_instance", { data }),
  updateInstance: (instance: Instance) => invoke<Instance>("update_instance", { instance }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  openInstanceFolder: (id: string, sub?: string) => invoke<void>("open_instance_folder", { id, sub }),

  launchInstance: (id: string, quick?: QuickPlay | null) => invoke<void>("launch_instance", { id, quick: quick ?? null }),
  runningInstances: () => invoke<string[]>("running_instances"),
  stopInstance: (id: string) => invoke<void>("stop_instance", { id }),
  launchBedrock: () => invoke<void>("launch_bedrock"),

  getFeed: () => invoke<Feed>("get_feed"),

  search: (query: string, kind: ContentKind | "modpack", mc: string | null, loader: string | null, offset: number, sort: string) =>
    invoke<{ hits: Hit[]; total: number }>("modrinth_search", { query, kind, mc, loader, offset, sort }),
  contentVersions: (instanceId: string, projectId: string, kind: ContentKind) =>
    invoke<ContentVersion[]>("modrinth_versions", { instanceId, projectId, kind }),
  installContent: (instanceId: string, kind: ContentKind, projectId: string, versionId?: string | null) =>
    invoke<string[]>("install_content", { instanceId, kind, projectId, versionId }),
  listContent: (instanceId: string, kind: ContentKind) => invoke<InstalledContent[]>("list_content", { instanceId, kind }),
  toggleContent: (instanceId: string, kind: ContentKind, filename: string, enabled: boolean) =>
    invoke<void>("toggle_content", { instanceId, kind, filename, enabled }),
  removeContent: (instanceId: string, kind: ContentKind, filename: string) =>
    invoke<void>("remove_content", { instanceId, kind, filename }),
  checkUpdates: (instanceId: string) => invoke<ContentUpdate[]>("check_updates", { instanceId }),
  applyUpdates: (instanceId: string, only: string[]) => invoke<number>("apply_updates", { instanceId, only }),

  importPack: (path: string) => invoke<Instance>("import_pack", { path }),
  installModpack: (projectId: string, versionId?: string | null) => invoke<Instance>("install_modpack", { projectId, versionId }),
  optimizeInstance: (instanceId: string) => invoke<{ installed: string[]; skipped: string[] }>("optimize_instance", { instanceId }),
  duplicateInstance: (id: string) => invoke<Instance>("duplicate_instance", { id }),
  readLaunchLog: (id: string) => invoke<string[]>("read_launch_log", { id }),
  systemMemory: () => invoke<number>("system_memory"),
  openFile: (path: string) => invoke<void>("open_file", { path }),
  checkMods: (instanceId: string) => invoke<ModProblem[]>("check_mods", { instanceId }),
  listSnapshots: (instanceId: string) => invoke<Snapshot[]>("list_snapshots", { instanceId }),
  createSnapshot: (instanceId: string, label: string) => invoke<Snapshot>("create_snapshot", { instanceId, label }),
  restoreSnapshot: (instanceId: string, file: string) => invoke<void>("restore_snapshot", { instanceId, file }),
  deleteSnapshot: (instanceId: string, file: string) => invoke<void>("delete_snapshot", { instanceId, file }),
  playSessions: (since: number) => invoke<Session[]>("play_sessions", { since }),
  quickStart: () => invoke<QuickItem[]>("quick_start"),
  estimateLauncherBackup: (options: { worlds: boolean; history: boolean; accounts: boolean }) =>
    invoke<{ files: number; bytes: number; builds: number }>("estimate_launcher_backup", { options }),
  createLauncherBackup: (dest: string, options: { worlds: boolean; history: boolean; accounts: boolean }) =>
    invoke<number>("create_launcher_backup", { dest, options }),
  inspectLauncherBackup: (path: string) =>
    invoke<{ created: number; version: string; builds: number; worlds: boolean; accounts: boolean }>("inspect_launcher_backup", { path }),
  restoreLauncherBackup: (path: string) => invoke<number>("restore_launcher_backup", { path }),
  bugReport: (dest: string, instanceId: string | null, note: string) => invoke<void>("bug_report", { dest, instanceId, note }),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  updatesConfigured: () => invoke<boolean>("updates_configured"),
  discordPresence: (presence: { details: string; state: string; start: number | null; largeText: string } | null) =>
    invoke<void>("discord_presence", { presence }),
  trayUpdate: (state: { builds: { id: string; name: string }[]; running: { id: string; name: string }[]; linked: string[]; streamer: boolean; lang: string }) =>
    invoke<void>("tray_update", { state }),
  hideToTray: () => invoke<void>("hide_to_tray"),
  setAutostart: (on: boolean) => invoke<boolean>("set_autostart", { on }),
  revealFile: (path: string) => invoke<void>("reveal_file", { path }),
  linkSend: (instanceId: string, cmd: Record<string, unknown>) => invoke<Record<string, unknown>>("link_send", { instanceId, cmd }),
  linkStatus: () => invoke<Record<string, LinkWorld | null>>("link_status"),
  screenshotMeta: () => invoke<Record<string, ShotMeta>>("screenshot_meta"),
  worldTimes: (instanceId: string) => invoke<Record<string, { seconds: number; name?: string; kind: string; last: number }>>("world_times", { instanceId }),
  listControls: () => invoke<ControlProfile[]>("list_controls"),
  saveControls: (instanceId: string, name: string) => invoke<ControlProfile[]>("save_controls", { instanceId, name }),
  applyControls: (instanceId: string, name: string) => invoke<number>("apply_controls", { instanceId, name }),
  deleteControls: (name: string) => invoke<ControlProfile[]>("delete_controls", { name }),
  resolveShare: (text: string, loader?: string | null, mc?: string | null) =>
    invoke<{ items: SharedFound[]; notFound: string[] }>("resolve_share", { text, loader, mc }),
  analyzeDrop: (path: string) => invoke<DropInfo>("analyze_drop", { path }),
  placeDrop: (path: string, instanceId: string, kind: string) => invoke<string>("place_drop", { path, instanceId, kind }),
  savePng: (path: string, data: string) => invoke<void>("save_png", { path, data }),
  copyPng: (data: string) => invoke<void>("copy_png", { data }),
  minecraftLatest: () => invoke<{ release: string; snapshot: string; releaseTime: string; snapshotTime: string }>("minecraft_latest"),
  exportPack: (instanceId: string, dest: string) => invoke<void>("export_pack", { instanceId, dest }),

  listScreenshots: () => invoke<Shot[]>("list_screenshots"),
  copyImage: (path: string) => invoke<void>("copy_image", { path }),
  saveCopy: (path: string, dest: string) => invoke<void>("save_copy", { path, dest }),
  deleteScreenshot: (path: string) => invoke<void>("delete_screenshot", { path }),
  listWorlds: (instanceId: string) => invoke<World[]>("list_worlds", { instanceId }),
  listBackups: (instanceId: string) => invoke<Backup[]>("list_backups", { instanceId }),
  backupWorld: (instanceId: string, world: string) => invoke<string>("backup_world", { instanceId, world }),
  restoreBackup: (instanceId: string, file: string) => invoke<void>("restore_backup", { instanceId, file }),
  deleteBackup: (instanceId: string, file: string) => invoke<void>("delete_backup", { instanceId, file }),

  listSkins: () => invoke<SavedSkin[]>("list_skins"),
  importSkin: (path: string, name: string, variant: string) => invoke<SavedSkin>("import_skin", { path, name, variant }),
  saveSkinData: (id: string | null, name: string, variant: string, data: string) =>
    invoke<SavedSkin>("save_skin_data", { id, name, variant, data }),
  updateSkin: (id: string, name: string, variant: string) => invoke<void>("update_skin", { id, name, variant }),
  deleteSkin: (id: string) => invoke<void>("delete_skin", { id }),
  textureData: (source: string) => invoke<string>("texture_data", { source }),
  skinProfile: () => invoke<SkinProfile>("skin_profile"),
  uploadSkin: (id: string) => invoke<SkinProfile>("upload_skin", { id }),
  setCape: (capeId: string | null) => invoke<SkinProfile>("set_cape", { capeId }),
  openNameMC: () => invoke<void>("open_namemc"),
  readPng: (path: string) => invoke<string>("read_png", { path }),
  writePng: (path: string, data: string) => invoke<void>("write_png", { path, data }),

  readTextFile: (path: string) => invoke<string>("read_text_file", { path }),
  writeTextFile: (path: string, content: string) => invoke<void>("write_text_file", { path, content }),
};

/** Image source for an instance art / avatar (remote URL or a file in the launcher folder). */
export function artSrc(art?: Art | null): string | null {
  if (!art) return null;
  return art.kind === "file" ? convertFileSrc(art.value) : art.value;
}

export function fileSrc(path?: string | null): string | null {
  return path ? convertFileSrc(path) : null;
}

export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}
