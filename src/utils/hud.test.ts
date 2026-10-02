import { describe, expect, it } from "vitest";
import type { Setting } from "../models";
import {
  formatHudCoordinate,
  hasRootHudCoordinates,
  parseWindowSize,
} from "./hud";

const hud: Setting = {
  id: "fixture",
  source: "lunar",
  fileKind: "mods",
  profile: "profile-1",
  pointer: "/FPS/x",
  category: "HUD",
  group: "FPS",
  label: "X",
  value: 10,
};

describe("HUD adjustment selection and manual pixel bounds", () => {
  it("removes floating point tails from HUD preview text without changing coordinate values", () => {
    const value = 163.79999999999998;
    expect(formatHudCoordinate(value)).toBe("163.8");
    expect(value).toBe(163.79999999999998);
    expect(formatHudCoordinate(1.23456789)).toBe("1.234568");
    expect(formatHudCoordinate(320)).toBe("320");
    expect(formatHudCoordinate(-0.00000001)).toBe("0");
    expect(formatHudCoordinate("163.79999999999998")).toBe(
      "163.79999999999998",
    );
  });
  it("only enables automatic adjustment for selected root Lunar HUD coordinates", () => {
    expect(hasRootHudCoordinates([hud])).toBe(true);
    expect(hasRootHudCoordinates([{ ...hud, pointer: "/FPS/y" }])).toBe(true);
    expect(hasRootHudCoordinates([{ ...hud, pointer: "/FPS/CHILD/x" }])).toBe(
      false,
    );
    expect(hasRootHudCoordinates([{ ...hud, pointer: "/FPS/enabled" }])).toBe(
      false,
    );
    expect(hasRootHudCoordinates([{ ...hud, pointer: "/UNVERIFIED/x" }])).toBe(
      false,
    );
    expect(hasRootHudCoordinates([{ ...hud, source: "minecraft" }])).toBe(
      false,
    );
    expect(hasRootHudCoordinates([{ ...hud, fileKind: "general" }])).toBe(
      false,
    );
  });
  it("requires a complete integer size within the physical window bounds", () => {
    expect(parseWindowSize("320", "240")).toEqual({ width: 320, height: 240 });
    expect(parseWindowSize("32768", "32768")).toEqual({
      width: 32768,
      height: 32768,
    });
    expect(parseWindowSize("1920", "1080")).toEqual({
      width: 1920,
      height: 1080,
    });
    for (const pair of [
      ["", "1080"],
      ["1920", ""],
      ["319", "1080"],
      ["1920", "239"],
      ["32769", "1080"],
      ["1920", "32769"],
      ["1920.5", "1080"],
      ["NaN", "1080"],
      ["Infinity", "1080"],
      ["1e3", "1080"],
    ])
      expect(parseWindowSize(...(pair as [string, string]))).toBeUndefined();
  });
});
