import { describe, expect, it } from "vitest";
import type { Setting } from "../models";
import {
  buildTree,
  selectPreset,
  selectionState,
  toggleSelection,
} from "./tree";

const settings: Setting[] = [
  {
    id: "fov",
    label: "FOV",
    source: "minecraft",
    category: "Video",
    group: "",
    fileKind: "options",
    profile: "default",
    pointer: "fov",
    value: 90,
  },
  {
    id: "vsync",
    label: "VSync",
    source: "minecraft",
    category: "Video",
    group: "",
    fileKind: "options",
    profile: "default",
    pointer: "vsync",
    value: true,
  },
  {
    id: "fps",
    label: "FPS",
    source: "lunar",
    category: "HUD",
    group: "FPS",
    fileKind: "json",
    profile: "default",
    pointer: "/fps/enabled",
    value: true,
  },
];

describe("setting selection", () => {
  it("cascades parent selection and deselection without changing another source", () => {
    const selected = toggleSelection(["fov", "vsync"], new Set(["fps"]));
    expect([...selected]).toEqual(["fps", "fov", "vsync"]);
    expect([...toggleSelection(["fov", "vsync"], selected)]).toEqual(["fps"]);
  });
  it("reports partial, complete, and empty states", () => {
    expect(selectionState(["fov", "vsync"], new Set(["fov"]))).toEqual({
      checked: false,
      indeterminate: true,
    });
    expect(selectionState(["fov", "vsync"], new Set(["fov", "vsync"]))).toEqual(
      { checked: true, indeterminate: false },
    );
    expect(selectionState([], new Set())).toEqual({
      checked: false,
      indeterminate: false,
    });
  });
  it("retains hierarchy and limits search selection to matching descendants", () => {
    const tree = buildTree(settings, "fov");
    expect(tree).toHaveLength(1);
    expect(tree[0].ids).toEqual(["fov"]);
    expect(tree[0].children[0].children[0].setting?.id).toBe("fov");
    expect(toggleSelection(tree[0].ids, new Set(["fps"]))).toEqual(
      new Set(["fps", "fov"]),
    );
  });
  it("uses discovered categories for presets and does not assume a fixed file structure", () => {
    expect(selectPreset(settings, "hud")).toEqual(new Set(["fps"]));
    expect(selectPreset(settings, "performance")).toEqual(
      new Set(["fov", "vsync"]),
    );
    expect(buildTree(settings, "no such setting")).toEqual([]);
  });
  it("searches translated headings while preserving stable setting identities", () => {
    const tree = buildTree(settings, "画面", (label) =>
      label === "Video" ? "画面" : label,
    );
    expect(tree[0].ids).toEqual(["fov", "vsync"]);
    expect(tree[0].children[0].label).toBe("Video");
  });
});
