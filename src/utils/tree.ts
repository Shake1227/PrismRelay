import type { Setting } from "../models";
import { settingLabel } from "./appearance";

export interface SettingNode {
  key: string;
  label: string;
  children: SettingNode[];
  setting?: Setting;
  ids: string[];
}
export interface SelectionState {
  checked: boolean;
  indeterminate: boolean;
}

export function buildTree(
  settings: Setting[],
  search = "",
  translateLabel: (label: string) => string = (label) => label,
): SettingNode[] {
  const roots: SettingNode[] = [];
  const query = search.trim().toLocaleLowerCase();
  for (const setting of settings) {
    const source =
      setting.source === "minecraft" ? "Minecraft" : "Lunar Client";
    if (
      query &&
      !`${source} ${setting.category} ${setting.group} ${setting.label} ${setting.pointer} ${translateLabel(setting.category)} ${translateLabel(setting.group)} ${settingLabel(setting, translateLabel)}`
        .toLocaleLowerCase()
        .includes(query)
    )
      continue;
    const levels = [
      source,
      setting.category || "General",
      ...(setting.group && setting.group !== setting.category
        ? [setting.group]
        : []),
    ];
    let children = roots;
    let path = "";
    for (const label of levels) {
      path += `/${label}`;
      let node = children.find((candidate) => candidate.key === path);
      if (!node) {
        node = { key: path, label, children: [], ids: [] };
        children.push(node);
      }
      node.ids.push(setting.id);
      children = node.children;
    }
    children.push({
      key: setting.id,
      label: setting.label,
      children: [],
      setting,
      ids: [setting.id],
    });
  }
  return roots.sort((a, b) =>
    a.label === "Lunar Client"
      ? -1
      : b.label === "Lunar Client"
        ? 1
        : a.label.localeCompare(b.label),
  );
}

export function selectionState(
  ids: string[],
  selected: Set<string>,
): SelectionState {
  const count = ids.filter((id) => selected.has(id)).length;
  return {
    checked: ids.length > 0 && count === ids.length,
    indeterminate: count > 0 && count < ids.length,
  };
}

export function toggleSelection(
  ids: string[],
  selected: Set<string>,
): Set<string> {
  const next = new Set(selected);
  const remove = ids.every((id) => selected.has(id));
  for (const id of ids) {
    if (remove) next.delete(id);
    else next.add(id);
  }
  return next;
}

export const presets = [
  { id: "everything", label: "すべて", accepts: () => true },
  {
    id: "hud",
    label: "HUD のみ",
    accepts: (setting: Setting) =>
      setting.source === "lunar" && /hud/i.test(setting.category),
  },
  {
    id: "pvp",
    label: "PvP",
    accepts: (setting: Setting) =>
      /hud|control|mouse|keybind|mod/i.test(setting.category),
  },
  {
    id: "performance",
    label: "パフォーマンス",
    accepts: (setting: Setting) => /video/i.test(setting.category),
  },
  {
    id: "controls",
    label: "操作",
    accepts: (setting: Setting) =>
      /control|mouse|keybind/i.test(setting.category),
  },
  {
    id: "minecraft",
    label: "Minecraft",
    accepts: (setting: Setting) => setting.source === "minecraft",
  },
  {
    id: "lunar",
    label: "Lunar Client",
    accepts: (setting: Setting) => setting.source === "lunar",
  },
];

export function selectPreset(settings: Setting[], preset: string): Set<string> {
  const rule = presets.find((item) => item.id === preset);
  return new Set(
    settings
      .filter((setting) => rule?.accepts(setting))
      .map((setting) => setting.id),
  );
}
