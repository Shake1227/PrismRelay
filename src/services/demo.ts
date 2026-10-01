import type {
  BackupManifest,
  DecodedShare,
  EncodedShare,
  ImportArguments,
  ImportPreview,
  JsonValue,
  ScanReport,
  Setting,
  ShareMetadata,
} from "../models";
import { APP_VERSION } from "../version";

const fields: [string, string, string, JsonValue][] = [
  ["Video", "Display", "FOV", 90],
  ["Video", "Display", "GUI Scale", 3],
  ["Video", "Display", "Fullscreen", false],
  ["Video", "Quality", "Graphics", "fancy"],
  ["Video", "Quality", "Clouds", "fast"],
  ["Video", "Quality", "Particles", "all"],
  ["Video", "Performance", "Render Distance", 12],
  ["Video", "Performance", "Simulation Distance", 8],
  ["Video", "Performance", "Max Framerate", 144],
  ["Video", "Performance", "VSync", false],
  ["Mouse", "", "Sensitivity", 0.42],
  ["Mouse", "", "Raw Input", true],
  ["Mouse", "", "Scroll Sensitivity", 1],
  ["Controls", "Movement", "Forward", "key.keyboard.w"],
  ["Controls", "Movement", "Jump", "key.keyboard.space"],
  ["Controls", "Movement", "Sprint", "key.keyboard.left.control"],
  ["Controls", "Gameplay", "Attack", "key.mouse.left"],
  ["Controls", "Gameplay", "Use Item", "key.mouse.right"],
  ["Controls", "Inventory", "Inventory", "key.keyboard.e"],
  ["Audio", "", "Master Volume", 0.7],
  ["Audio", "", "Music", 0],
  ["Audio", "", "Players", 1],
  ["Chat", "", "Chat Opacity", 0.85],
  ["Chat", "", "Chat Scale", 1],
  ["Accessibility", "", "Auto Jump", false],
  ["Language", "", "Language", "ja_jp"],
];

const minecraft: Setting[] = fields.map(
  ([category, group, label, value], index) => ({
    id: `demo-mc-${index}`,
    source: "minecraft",
    category,
    group,
    label,
    value,
    fileKind: "options",
    profile: "Minecraft",
    pointer: label.toLowerCase().replaceAll(" ", "_"),
  }),
);
const lunar: Setting[] = [
  "Coordinates",
  "FPS",
  "CPS",
  "Keystrokes",
  "Potion Effects",
  "Armor Status",
  "Crosshair",
  "Scoreboard",
]
  .flatMap((label, index) => [
    {
      id: `demo-lunar-${index}-enabled`,
      source: "lunar" as const,
      category: "HUD",
      group: label,
      label: "Enabled",
      value: true,
      fileKind: "json",
      profile: "default",
      pointer: `/hud/${label.toLowerCase().replaceAll(" ", "_")}/enabled`,
    },
    {
      id: `demo-lunar-${index}-scale`,
      source: "lunar" as const,
      category: "HUD",
      group: label,
      label: "Scale",
      value: 1,
      fileKind: "json",
      profile: "default",
      pointer: `/hud/${label.toLowerCase().replaceAll(" ", "_")}/scale`,
    },
  ])
  .concat(
    ["Toggle Sprint", "Zoom", "Freelook"].map((label, index) => ({
      id: `demo-mod-${index}`,
      source: "lunar" as const,
      category: "Mods",
      group: label,
      label: "Enabled",
      value: true,
      fileKind: "json",
      profile: "default",
      pointer: `/mods/${label.toLowerCase().replaceAll(" ", "_")}/enabled`,
    })),
  );

export const demoScan: ScanReport = {
  settings: [...lunar, ...minecraft],
  files: [
    {
      id: "demo-options",
      source: "minecraft",
      path: "サンプル / Minecraft / options.txt",
      fileKind: "options",
      profile: "Minecraft",
    },
    {
      id: "demo-lunar",
      source: "lunar",
      path: "サンプル / Lunar / default / mods.json",
      fileKind: "json",
      profile: "default",
    },
  ],
  minecraftDetected: true,
  lunarDetected: true,
  minecraftVersions: ["1.21.4"],
  lunarProfiles: ["default"],
  warnings: [],
  platform: "Sample",
  runningProcesses: [],
};

export const demoBackup: BackupManifest = {
  id: "sample-backup",
  createdAt: "2026-09-30T03:24:00Z",
  platform: "Sample",
  minecraftVersions: ["1.21.4"],
  lunarProfiles: ["default"],
  reason: "manual",
  status: "complete",
  files: [
    {
      relativeName: "options.txt",
      originalPath: "サンプル / Minecraft / options.txt",
      checksum: "sample-checksum",
      size: 3180,
    },
    {
      relativeName: "mods.json",
      originalPath: "サンプル / Lunar / mods.json",
      checksum: "sample-checksum",
      size: 6520,
    },
  ],
};

export function encodeDemo(
  settings: Setting[],
  metadata: ShareMetadata,
): EncodedShare {
  const payload: DecodedShare = {
    formatVersion: 1,
    applicationVersion: APP_VERSION,
    createdAt: "2026-09-30T03:24:00Z",
    metadata,
    settings,
  };
  const bytes = new TextEncoder().encode(JSON.stringify(payload));
  const base64 = btoa(
    Array.from(bytes, (byte) => String.fromCharCode(byte)).join(""),
  );
  return {
    code: `PRDEMO1:${base64}`,
    compressedBytes: bytes.length,
    uncompressedBytes: bytes.length,
    settingCount: settings.length,
  };
}

export function decodeDemo(code: string): DecodedShare {
  if (!code.startsWith("PRDEMO1:"))
    throw new Error(
      "サンプルモードでは PRDEMO1: で始まるサンプルコードのみ確認できます。実際の共有コードはデスクトップアプリで開いてください。",
    );
  try {
    const bytes = Uint8Array.from(atob(code.slice(8)), (letter) =>
      letter.charCodeAt(0),
    );
    const value = JSON.parse(new TextDecoder().decode(bytes)) as DecodedShare;
    if (value.formatVersion !== 1 || !Array.isArray(value.settings))
      throw new Error("Invalid sample payload");
    return value;
  } catch {
    throw new Error("サンプルコードの形式が正しくありません。");
  }
}

export function previewDemo(args: ImportArguments): ImportPreview {
  const decoded = decodeDemo(args.code);
  return {
    fingerprint: "demo",
    targetFiles: demoScan.files
      .filter((file) => {
        const profile =
          file.source === "minecraft"
            ? args.target.minecraftProfile
            : args.target.lunarProfile;
        return (
          file.profile === profile &&
          decoded.settings.some(
            (setting) =>
              args.selectedIds.includes(setting.id) &&
              setting.source === file.source &&
              setting.fileKind === file.fileKind,
          )
        );
      })
      .map(({ source, profile, fileKind, path }) => ({
        source,
        profile,
        fileKind,
        path,
      })),
    changes: decoded.settings
      .filter((setting) => args.selectedIds.includes(setting.id))
      .map((setting) => {
        const current =
          demoScan.settings.find(
            (item) =>
              item.source === setting.source &&
              item.pointer === setting.pointer,
          )?.value ?? null;
        return {
          id: setting.id,
          label: `${setting.group ? `${setting.group} · ` : ""}${setting.label}`,
          source: setting.source,
          category: setting.category,
          current,
          incoming: setting.value,
          changed: JSON.stringify(current) !== JSON.stringify(setting.value),
        };
      }),
    warnings: ["サンプルの差分です。実際の設定ファイルにはアクセスしません。"],
    selectedCount: args.selectedIds.length,
  };
}
