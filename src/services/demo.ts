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
import { hasRootHudCoordinates, parseWindowSize } from "../utils/hud";

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
      fileKind: "mods",
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
      fileKind: "mods",
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
      fileKind: "mods",
      profile: "default",
      pointer: `/mods/${label.toLowerCase().replaceAll(" ", "_")}/enabled`,
    })),
  );
const hudPositions: Setting[] = ["x", "y"].map((axis) => ({
  id: `demo-hud-${axis}`,
  source: "lunar",
  category: "HUD",
  group: "Coordinates",
  label: axis.toUpperCase(),
  value: axis === "x" ? 320 : 180,
  fileKind: "mods",
  profile: "default",
  pointer: `/COORDINATES/${axis}`,
}));

export const demoScan: ScanReport = {
  settings: [...lunar, ...hudPositions, ...minecraft],
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
      fileKind: "mods",
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
    metadata: hasRootHudCoordinates(settings)
      ? {
          ...metadata,
          hudViewport: metadata.hudViewport || { width: 960, height: 540 },
        }
      : metadata,
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
  const settings = decoded.settings.filter((setting) =>
    args.selectedIds.includes(setting.id),
  );
  const coordinates = hasRootHudCoordinates(settings);
  const mode = args.hudLayout?.mode || "auto";
  const windowSize =
    mode === "manual"
      ? args.hudLayout?.windowSize &&
        parseWindowSize(
          String(args.hudLayout.windowSize.width),
          String(args.hudLayout.windowSize.height),
        )
      : { width: 1600, height: 1000 };
  const source = decoded.metadata.hudViewport;
  const validSource =
    source &&
    Number.isFinite(source.width) &&
    Number.isFinite(source.height) &&
    source.width > 0 &&
    source.height > 0;
  const canAdjust =
    coordinates && mode !== "preserve" && !!validSource && !!windowSize;
  let adjustedCount = 0;
  const changes = settings.map((setting) => {
    const current =
      demoScan.settings.find(
        (item) =>
          item.source === setting.source && item.pointer === setting.pointer,
      )?.value ?? null;
    let incoming = setting.value;
    if (
      canAdjust &&
      hasRootHudCoordinates([setting]) &&
      typeof incoming === "number"
    ) {
      const axis = setting.pointer.endsWith("/x") ? "width" : "height";
      incoming = (incoming * (windowSize![axis] / 2)) / source![axis];
      if (incoming !== setting.value) adjustedCount += 1;
    }
    return {
      id: setting.id,
      label: `${setting.group ? `${setting.group} · ` : ""}${setting.label}`,
      source: setting.source,
      category: setting.category,
      current,
      incoming,
      changed: JSON.stringify(current) !== JSON.stringify(incoming),
    };
  });
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
    changes,
    ...(coordinates
      ? {
          hudLayout: {
            status:
              mode === "preserve"
                ? "unchanged"
                : !validSource
                  ? "missing-source"
                  : !windowSize
                    ? "unavailable"
                    : adjustedCount
                      ? "adjusted"
                      : "unchanged",
            ...(mode !== "preserve" && windowSize ? { windowSize } : {}),
            adjustedCount,
          } as ImportPreview["hudLayout"],
        }
      : {}),
    warnings: ["サンプルの差分です。実際の設定ファイルにはアクセスしません。"],
    selectedCount: args.selectedIds.length,
  };
}
