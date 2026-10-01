import { describe, expect, it } from "vitest";
import type { ScanReport } from "../models";
import {
  minecraftProfiles,
  minecraftVersion,
  reconcileProfiles,
  reconcileImportProfiles,
} from "./profiles";

const scan: ScanReport = {
  settings: [],
  minecraftDetected: true,
  lunarDetected: false,
  minecraftVersions: ["1.21", "1.8.9"],
  lunarProfiles: [],
  warnings: [],
  platform: "macOS",
  runningProcesses: [],
  files: [
    {
      id: "vanilla",
      source: "minecraft",
      profile: "Vanilla",
      path: "/sample/options.txt",
      fileKind: "options",
    },
    {
      id: "lunar",
      source: "minecraft",
      profile: "Lunar 1.21",
      path: "/sample/lunar/options.txt",
      fileKind: "options",
    },
    {
      id: "optifine",
      source: "minecraft",
      profile: "Lunar 1.21",
      path: "/sample/lunar/optionsof.txt",
      fileKind: "optionsof",
    },
  ],
};

describe("profile selection", () => {
  it("offers real target profile identities rather than version labels", () => {
    expect(minecraftProfiles(scan)).toEqual(["Vanilla", "Lunar 1.21"]);
  });
  it("does not place a user profile name in shared version metadata", () => {
    expect(minecraftVersion(scan, "Vanilla")).toBeUndefined();
    expect(minecraftVersion(scan, "My Private Profile")).toBeUndefined();
    expect(minecraftVersion(scan, "Lunar 1.21")).toBe("1.21");
    expect(minecraftVersion(scan, "Lunar 1.21.4")).toBeUndefined();
  });
  it("keeps valid profile choices and replaces profiles removed by a new scan", () => {
    const valid = { minecraftProfile: "Lunar 1.21" };
    expect(reconcileProfiles(scan, valid)).toBe(valid);
    expect(reconcileProfiles(scan, { minecraftProfile: "removed" })).toEqual({
      minecraftProfile: "Vanilla",
      lunarProfile: undefined,
    });
  });
  it("requires an explicit import target when several profiles exist and after a selected profile disappears", () => {
    expect(reconcileImportProfiles(scan)).toEqual({});
    const chosen = { minecraftProfile: "Lunar 1.21" };
    expect(reconcileImportProfiles({ ...scan }, chosen)).toBe(chosen);
    expect(
      reconcileImportProfiles(scan, { minecraftProfile: "missing" }),
    ).toEqual({ minecraftProfile: undefined, lunarProfile: undefined });
    const single = {
      ...scan,
      files: scan.files.filter((file) => file.profile === "Lunar 1.21"),
    };
    expect(reconcileImportProfiles(single)).toEqual({
      minecraftProfile: "Lunar 1.21",
      lunarProfile: undefined,
    });
  });
});
