import { describe, expect, it } from "vitest";
import type { Setting } from "../models";
import { translate } from "../i18n";
import {
  appearanceColor,
  appearanceEnumLabel,
  settingLabel,
} from "./appearance";
import { buildTree } from "./tree";

const color: Setting = {
  id: "fixture-color",
  source: "lunar",
  fileKind: "mods",
  profile: "Default",
  pointer: "/KEYSTROKES/options/textColor/value",
  category: "HUD",
  group: "Keystrokes",
  label: "Text Color",
  value: 0x806633ff | 0,
};

describe("verified Lunar appearance display", () => {
  it("shows signed packed ARGB as RGB and opacity without changing its value", () => {
    expect(appearanceColor(color, color.value)).toEqual({
      hex: "#6633FF",
      css: "#6633FF80",
      alpha: 128,
      opacity: 50.2,
    });
    expect(appearanceColor(color, -1)?.css).toBe("#FFFFFFFF");
    expect(appearanceColor(color, 0)?.css).toBe("#00000000");
    expect(color.value).toBe(0x806633ff | 0);
  });
  it("never labels an unrelated integer, another file, or an invalid packed value as a color", () => {
    expect(
      appearanceColor({ ...color, pointer: "/KEYSTROKES/x" }, -1),
    ).toBeUndefined();
    expect(
      appearanceColor({ ...color, fileKind: "general" }, -1),
    ).toBeUndefined();
    expect(
      appearanceColor({ ...color, source: "minecraft" }, -1),
    ).toBeUndefined();
    for (const value of [-2147483649, 2147483648, -1.5, "-1", null])
      expect(appearanceColor(color, value)).toBeUndefined();
  });
  it("keeps rainbow controls associated with their color and searches translated appearance labels", () => {
    const chroma = {
      ...color,
      id: "fixture-chroma",
      pointer: "/KEYSTROKES/options/textColor/chroma",
      label: "Chroma",
      value: false,
    };
    const t = (key: string) => translate("ja", key);
    expect(settingLabel(chroma, t)).toBe("文字色 · 虹色");
    expect(buildTree([color, chroma], "文字色", t)[0].ids).toEqual([
      "fixture-color",
      "fixture-chroma",
    ]);
  });
  it("distinguishes key modifiers from hue shift and leaves key codes unchanged on the wire", () => {
    const t = (key: string, values?: Record<string, string | number>) =>
      translate("ja", key, values);
    const key = {
      ...color,
      pointer: "/CHAT/options/copyChatBind/value",
      value: "KEY_LSHIFT",
    };
    const shift = {
      ...key,
      pointer: "/CHAT/options/copyChatBind/shift",
      value: false,
    };
    expect(settingLabel(shift, t)).toBe("チャットのコピー · Shift キー");
    expect(appearanceEnumLabel(key, "KEY_NONE", t)).toBe("未割り当て");
    expect(appearanceEnumLabel(key, key.value, t)).toBe("左 · Shift キー");
    expect(key.value).toBe("KEY_LSHIFT");
    expect(
      appearanceEnumLabel({ ...key, pointer: "/unverified/key" }, key.value, t),
    ).toBeUndefined();
  });
  it("describes verified box sizes and closed display modes without treating them as colors", () => {
    const t = (key: string) => translate("ja", key);
    const padding = {
      ...color,
      pointer: "/WAYPOINTS/options/boxPadding",
      value: 4,
    };
    const width = {
      ...color,
      fileKind: "general",
      pointer: "/backgroundWidth",
      value: 72,
    };
    const mode = { ...width, pointer: "/cosmeticRenderMode", value: "friends" };
    expect(settingLabel(padding, t)).toBe("枠内の余白");
    expect(settingLabel(width, t)).toBe("背景の幅");
    expect(appearanceColor(width, width.value)).toBeUndefined();
    expect(appearanceEnumLabel(mode, "unsupported", t)).toBeUndefined();
  });
});
