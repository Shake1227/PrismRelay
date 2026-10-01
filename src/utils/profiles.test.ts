import { describe, expect, it } from "vitest";
import type { ScanReport } from "../models";
import {
  defaultMinecraftProfile,
  defaultLunarProfile,
  minecraftProfiles,
  minecraftVersion,
  reconcileProfiles,
  reconcileImportProfiles,
} from "./profiles";

const scan: ScanReport = {
  settings: [],
  minecraftDetected: true,
  lunarDetected: true,
  minecraftVersions: ["1.21", "1.8.9"],
  lunarProfiles: ["Default", "PvP"],
  warnings: [],
  platform: "macOS",
  runningProcesses: [],
  files: [
    {
      id: "game",
      source: "minecraft",
      profile: "Minecraft",
      path: "/sample/options.txt",
      fileKind: "options",
    },
    {
      id: "game-lc",
      source: "minecraft",
      profile: "Minecraft",
      path: "/sample/optionsLC.txt",
      fileKind: "options",
    },
    {
      id: "optifine",
      source: "minecraft",
      profile: "Minecraft",
      path: "/sample/optionsof.txt",
      fileKind: "optionsof",
    },
  ],
};

describe("default game folder and Lunar preset selection", () => {
  it("prefers the active Lunar preset, preserves a valid explicit choice, and leaves uncertain multiple presets unselected", () => {
    const active = { ...scan, activeLunarProfile: "PvP" };
    expect(defaultLunarProfile(active)).toBe("PvP");
    expect(reconcileProfiles(active, {})).toEqual({
      minecraftProfile: "Minecraft",
      lunarProfile: "PvP",
    });
    expect(reconcileImportProfiles(active)).toEqual({
      minecraftProfile: "Minecraft",
      lunarProfile: "PvP",
    });
    expect(defaultLunarProfile(active, "Default")).toBe("Default");
    expect(
      reconcileImportProfiles(active, { lunarProfile: "Default" }),
    ).toEqual({ minecraftProfile: "Minecraft", lunarProfile: "Default" });
    expect(defaultLunarProfile(scan)).toBeUndefined();
    expect(
      defaultLunarProfile({ ...scan, activeLunarProfile: null }),
    ).toBeUndefined();
    expect(
      defaultLunarProfile({ ...scan, activeLunarProfile: "not-registered" }),
    ).toBeUndefined();
    expect(
      defaultLunarProfile({
        ...scan,
        lunarProfiles: ["Default"],
        activeLunarProfile: "not-registered",
      }),
    ).toBe("Default");
    expect(defaultLunarProfile({ ...scan, lunarProfiles: [] })).toBeUndefined();
  });
  it("uses one game profile automatically when several files belong to the same folder", () => {
    expect(minecraftProfiles(scan)).toEqual(["Minecraft"]);
    expect(defaultMinecraftProfile(scan)).toBe("Minecraft");
    expect(reconcileImportProfiles(scan)).toEqual({
      minecraftProfile: "Minecraft",
      lunarProfile: undefined,
    });
  });
  it("never puts a local game profile name in shared version metadata", () => {
    expect(minecraftVersion(scan, "Minecraft")).toBeUndefined();
    expect(minecraftVersion(scan, "My Private Profile")).toBeUndefined();
    expect(minecraftVersion(scan, "Lunar 1.21")).toBe("1.21");
    expect(minecraftVersion(scan, "Lunar 1.21.4")).toBeUndefined();
  });
  it("updates the game folder automatically while retaining a valid Lunar preset", () => {
    expect(
      reconcileProfiles(scan, {
        minecraftProfile: "removed",
        lunarProfile: "PvP",
      }),
    ).toEqual({ minecraftProfile: "Minecraft", lunarProfile: "PvP" });
    const valid = { minecraftProfile: "Minecraft", lunarProfile: "PvP" };
    expect(reconcileProfiles(scan, valid)).toBe(valid);
    expect(
      reconcileImportProfiles(scan, {
        minecraftProfile: "removed",
        lunarProfile: "PvP",
      }),
    ).toEqual(valid);
    expect(reconcileImportProfiles(scan, { lunarProfile: "removed" })).toEqual({
      minecraftProfile: "Minecraft",
      lunarProfile: undefined,
    });
    expect(
      reconcileImportProfiles({ ...scan, lunarProfiles: ["Default"] }),
    ).toEqual({ minecraftProfile: "Minecraft", lunarProfile: "Default" });
  });
  it("does not silently choose a game folder from an ambiguous older scan", () => {
    const ambiguous: ScanReport = {
      ...scan,
      files: [
        ...scan.files,
        {
          id: "older-game",
          source: "minecraft",
          profile: "Lunar 1.21",
          path: "/other/options.txt",
          fileKind: "options",
        },
      ],
    };
    expect(defaultMinecraftProfile(ambiguous)).toBeUndefined();
    expect(
      reconcileImportProfiles(ambiguous, {
        minecraftProfile: "Lunar 1.21",
        lunarProfile: "PvP",
      }),
    ).toEqual({ minecraftProfile: undefined, lunarProfile: "PvP" });
  });
});
