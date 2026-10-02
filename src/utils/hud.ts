import type { JsonValue, Setting, WindowSize } from "../models";
import { formatValue } from "./format";

const rootHudModules = new Set([
  "ARMORSTATUS",
  "BOSSBAR",
  "CLOCK",
  "COMBO",
  "COOLDOWNS",
  "COORDINATES",
  "CPS",
  "DIRECTION_HUD",
  "FPS",
  "ITEM_COUNTER",
  "ITEM_TRACKER",
  "KEYSTROKES",
  "MEMORY",
  "MOMENTUM",
  "PING",
  "PLAYTIME",
  "POTION_EFFECTS",
  "REACH_DISPLAY",
  "STOPWATCH",
]);

export function hasRootHudCoordinates(settings: Setting[]): boolean {
  return settings.some(
    (setting) =>
      setting.source === "lunar" &&
      setting.fileKind === "mods" &&
      /^\/[^/]+\/[xy]$/.test(setting.pointer) &&
      rootHudModules.has(setting.pointer.split("/")[1]),
  );
}

export function formatHudCoordinate(value: JsonValue | undefined): string {
  return typeof value === "number" && Number.isFinite(value)
    ? String(Number(value.toFixed(6)))
    : formatValue(value);
}

export function parseWindowSize(
  width: string,
  height: string,
): WindowSize | undefined {
  if (!/^\d+$/.test(width.trim()) || !/^\d+$/.test(height.trim()))
    return undefined;
  const size = { width: Number(width), height: Number(height) };
  return Number.isInteger(size.width) &&
    Number.isInteger(size.height) &&
    size.width >= 320 &&
    size.width <= 32768 &&
    size.height >= 240 &&
    size.height <= 32768
    ? size
    : undefined;
}
