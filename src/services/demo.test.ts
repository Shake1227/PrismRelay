import { describe, expect, it } from "vitest";
import { decodeDemo, demoScan, encodeDemo, previewDemo } from "./demo";
import type { ImportArguments } from "../models";

const positions = demoScan.settings.filter(
  (setting) => setting.id === "demo-hud-x" || setting.id === "demo-hud-y",
);
const shared = encodeDemo(positions, { platform: "Sample" });
const argumentsFor = (code = shared.code): ImportArguments => ({
  code,
  selectedIds: positions.map((setting) => setting.id),
  target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
  request: {},
  hudLayout: { mode: "auto" },
});

describe("synthetic HUD viewport sharing", () => {
  it("adds deterministic source size and shows adjusted coordinates for a maximized sample window", () => {
    const before = JSON.stringify(demoScan);
    expect(decodeDemo(shared.code).metadata.hudViewport).toEqual({
      width: 960,
      height: 540,
    });
    const preview = previewDemo(argumentsFor());
    expect(preview.hudLayout).toEqual({
      status: "adjusted",
      adjustedCount: 2,
      windowSize: { width: 1600, height: 1000 },
    });
    expect(preview.changes[0].incoming).toBeCloseTo((320 * 800) / 960);
    expect(preview.changes[1].incoming).toBeCloseTo((180 * 500) / 540);
    const manual = previewDemo({
      ...argumentsFor(),
      hudLayout: { mode: "manual", windowSize: { width: 1920, height: 1080 } },
    });
    expect(manual.hudLayout?.status).toBe("unchanged");
    expect(manual.changes.every((change) => !change.changed)).toBe(true);
    expect(JSON.stringify(demoScan)).toBe(before);
  });
  it("reports missing source context for legacy samples and preserves original positions when requested", () => {
    const decoded = decodeDemo(shared.code);
    delete decoded.metadata.hudViewport;
    const legacy = "PRDEMO1:" + btoa(JSON.stringify(decoded));
    expect(previewDemo(argumentsFor(legacy)).hudLayout?.status).toBe(
      "missing-source",
    );
    const original = previewDemo({
      ...argumentsFor(legacy),
      hudLayout: { mode: "preserve" },
    });
    expect(original.hudLayout).toEqual({
      status: "unchanged",
      adjustedCount: 0,
    });
    expect(original.changes.map((change) => change.incoming)).toEqual(
      positions.map((setting) => setting.value),
    );
  });
});
