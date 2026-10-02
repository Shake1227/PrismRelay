import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { APP_VERSION } from "../version";
import type {
  AppInfo,
  BackupManifest,
  DecodedShare,
  EncodedShare,
  ImportArguments,
  ImportPreview,
  ScanReport,
  ScanRequest,
  Setting,
  ShareMetadata,
  Source,
  UpdateInfo,
} from "../models";
import {
  decodeDemo,
  demoBackup,
  demoScan,
  encodeDemo,
  previewDemo,
} from "./demo";

export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const backend = {
  scan: (request: ScanRequest) =>
    isDesktop
      ? invoke<ScanReport>("scan_configuration", { request })
      : Promise.resolve(demoScan),
  encode: (
    settings: Setting[],
    metadata: ShareMetadata,
    request: ScanRequest = {},
  ) =>
    isDesktop
      ? invoke<EncodedShare>("encode_share", { settings, metadata, request })
      : Promise.resolve(encodeDemo(settings, metadata)),
  decode: (code: string) =>
    isDesktop
      ? invoke<DecodedShare>("decode_share", { code })
      : Promise.resolve().then(() => decodeDemo(code)),
  preview: (args: ImportArguments) =>
    isDesktop
      ? invoke<ImportPreview>("preview_import", {
          ...args,
          target: {
            minecraftProfile: args.target.minecraftProfile || "",
            lunarProfile: args.target.lunarProfile || "",
          },
        })
      : Promise.resolve().then(() => previewDemo(args)),
  apply: (
    args: ImportArguments,
    previewFingerprint: string,
    allowRunning = false,
  ) =>
    isDesktop
      ? invoke<BackupManifest>("apply_import", {
          ...args,
          target: {
            minecraftProfile: args.target.minecraftProfile || "",
            lunarProfile: args.target.lunarProfile || "",
          },
          previewFingerprint,
          allowRunning,
        })
      : Promise.resolve(demoBackup),
  backups: () =>
    isDesktop
      ? invoke<BackupManifest[]>("list_backups")
      : Promise.resolve([demoBackup]),
  createBackup: (request: ScanRequest) =>
    isDesktop
      ? invoke<BackupManifest>("create_backup", { request })
      : Promise.resolve(demoBackup),
  restore: (id: string, allowRunning = false) =>
    isDesktop
      ? invoke<BackupManifest>("restore_backup", { id, allowRunning })
      : Promise.resolve(demoBackup),
  deleteBackup: (id: string) =>
    isDesktop ? invoke<void>("delete_backup", { id }) : Promise.resolve(),
  openBackupFolder: () =>
    isDesktop ? invoke<void>("open_backup_folder") : Promise.resolve(),
  info: () =>
    isDesktop
      ? invoke<AppInfo>("get_app_info")
      : Promise.resolve({
          name: "Prism Relay",
          version: APP_VERSION,
          platform: "Sample",
          repository: "https://github.com/Shake1227/PrismRelay",
          license: "GPL-3.0-or-later",
        }),
  updates: () =>
    isDesktop
      ? invoke<UpdateInfo>("check_updates")
      : Promise.resolve({
          currentVersion: APP_VERSION,
          latestVersion: APP_VERSION,
          updateAvailable: false,
          releaseUrl: "https://github.com/Shake1227/PrismRelay/releases",
        }),
  openProjectPage: (page: "repository" | "releases" | "license" | "x") => {
    if (isDesktop) return invoke<void>("open_project_page", { page });
    const suffix =
      page === "repository"
        ? ""
        : page === "releases"
          ? "/releases"
          : "/blob/main/LICENSE";
    window.open(
      page === "x"
        ? "https://x.com/shake_1227"
        : `https://github.com/Shake1227/PrismRelay${suffix}`,
      "_blank",
      "noopener,noreferrer",
    );
    return Promise.resolve();
  },
  async chooseDirectory(
    source: Source,
    title?: string,
  ): Promise<string | null> {
    if (!isDesktop) return null;
    const path = await open({
      directory: true,
      multiple: false,
      title:
        title ||
        (source === "minecraft"
          ? "Minecraft 設定フォルダ"
          : "Lunar Client 設定フォルダ"),
    });
    return typeof path === "string" ? path : null;
  },
  async saveCode(code: string): Promise<boolean> {
    if (!isDesktop) {
      const url = URL.createObjectURL(new Blob([code], { type: "text/plain" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "prism-relay-sample.prism";
      anchor.click();
      URL.revokeObjectURL(url);
      return true;
    }
    const path = await save({
      defaultPath: "settings.prism",
      filters: [{ name: "Prism Relay", extensions: ["prism"] }],
    });
    if (!path) return false;
    await invoke<void>("save_share_file", { path, code });
    return true;
  },
  async saveQrImage(dataUrl: string): Promise<boolean> {
    if (!isDesktop) {
      const anchor = document.createElement("a");
      anchor.href = dataUrl;
      anchor.download = "prism-relay-qr.png";
      anchor.click();
      return true;
    }
    const path = await save({
      defaultPath: "prism-relay-qr.png",
      filters: [{ name: "PNG", extensions: ["png"] }],
    });
    if (!path) return false;
    await invoke<void>("save_qr_image", { path, dataUrl });
    return true;
  },
  async loadCode(): Promise<string | null> {
    if (!isDesktop) return null;
    const path = await open({
      multiple: false,
      filters: [{ name: "Prism Relay", extensions: ["prism", "txt"] }],
    });
    if (typeof path !== "string") return null;
    return invoke<string>("load_share_file", { path });
  },
};
