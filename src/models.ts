export type JsonValue =
  string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };
export type Source = "minecraft" | "lunar";
export interface Setting {
  id: string;
  label: string;
  source: Source;
  category: string;
  group: string;
  fileKind: string;
  profile: string;
  pointer: string;
  value: JsonValue;
}
export interface ScanRequest {
  minecraftRoot?: string;
  lunarRoot?: string;
}
export interface ScanReport {
  applicationIcons?: {
    minecraft?: string | null;
    lunar?: string | null;
  } | null;
  settings: Setting[];
  files: {
    id: string;
    source: Source;
    path: string;
    fileKind: string;
    profile: string;
  }[];
  minecraftDetected: boolean;
  lunarDetected: boolean;
  minecraftVersions: string[];
  lunarProfiles: string[];
  warnings: string[];
  platform: string;
  runningProcesses: string[];
}
export interface ShareMetadata {
  minecraftVersion?: string;
  lunarVersion?: string;
  platform: string;
}
export interface EncodedShare {
  code: string;
  compressedBytes: number;
  uncompressedBytes: number;
  settingCount: number;
}
export interface DecodedShare {
  formatVersion: number;
  createdAt: string;
  applicationVersion: string;
  metadata: ShareMetadata;
  settings: Setting[];
}
export interface TargetProfiles {
  minecraftProfile?: string;
  lunarProfile?: string;
}
export interface ImportArguments {
  code: string;
  selectedIds: string[];
  target: TargetProfiles;
  request: ScanRequest;
}
export interface ImportPreview {
  fingerprint: string;
  changes: {
    id: string;
    label: string;
    source: Source;
    category: string;
    current: JsonValue;
    incoming: JsonValue;
    changed: boolean;
  }[];
  warnings: string[];
  selectedCount: number;
}
export interface BackupManifest {
  id: string;
  createdAt: string;
  platform: string;
  minecraftVersions: string[];
  lunarProfiles: string[];
  reason: string;
  files: {
    relativeName: string;
    originalPath: string;
    checksum: string;
    size: number;
  }[];
  status: string;
}
export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  repository: string;
  license: string;
}
export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  releaseUrl: string;
}
export type Page =
  "home" | "export" | "import" | "backups" | "settings" | "about";
export type Theme = "dark" | "light" | "system";
export type Language = "en" | "ja" | "ko" | "zh" | "fi" | "es" | "de";
export interface Preferences {
  language: Language;
  theme: Theme;
  reduceMotion: boolean;
  sidebarCollapsed: boolean;
  request: ScanRequest;
}
