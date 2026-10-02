import registry from "../../src-tauri/src/lunar_appearance.json";
import type { JsonValue, Setting } from "../models";
import { japaneseMessages } from "../i18n/messages";

interface AppearanceField {
  file_kind: string;
  pointer: string;
  component: string;
  kind: string;
  role: string;
  min?: number;
  max?: number;
  label?: string;
  allowed?: string[];
}

const fields: AppearanceField[] = registry.fields;
const indexedFields = new Map(
  fields.map((field) => [`${field.file_kind}:${field.pointer}`, field]),
);
const aliases: Record<string, string> = {
  "Background Pressed Color": "Pressed Background Color",
  "Text Pressed Color": "Pressed Text Color",
};
const roleLabels: Record<string, string> = {
  chroma: "Chroma",
  "chroma-speed": "Chroma Speed",
  "chroma-mode": "Chroma Mode",
  "keybind-value": "Key Binding",
  "keybind-modifier": "Modifier Key",
};

function humanize(token: string): string {
  const words = token
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replaceAll("_", " ")
    .split(/\s+/)
    .filter(Boolean);
  const label = words
    .map((word) =>
      [
        "HUD",
        "GUI",
        "FPS",
        "CPS",
        "TPS",
        "RGB",
        "NPC",
        "FOV",
        "RAM",
        "XP",
        "ID",
      ].includes(word.toUpperCase())
        ? word.toUpperCase()
        : word[0].toUpperCase() + word.slice(1).toLowerCase(),
    )
    .join(" ");
  return aliases[label] || label;
}

export function appearanceField(
  setting?: Setting,
): AppearanceField | undefined {
  return setting?.source === "lunar"
    ? indexedFields.get(`${setting.fileKind}:${setting.pointer}`)
    : undefined;
}

const standardPrefixes = [
  "Always Show",
  "Show",
  "Hide",
  "Enable",
  "Disable",
  "Use",
  "Animate",
  "Bold",
];
const standardSuffixes = [
  "Scale Factor",
  "Font Size",
  "Color",
  "Width",
  "Height",
  "Scale",
  "Padding",
  "Spacing",
  "Thickness",
  "Opacity",
  "Size",
  "Radius",
  "Length",
  "Speed",
  "Offset",
  "Duration",
  "Distance",
  "Mode",
  "Rows",
];
function captionParts(caption: string): string[] {
  if (Object.hasOwn(japaneseMessages, caption)) return [caption];
  for (const prefix of standardPrefixes) {
    if (caption.startsWith(`${prefix} `))
      return [
        prefix === "Use" ? "Use Option" : prefix,
        ...captionParts(caption.slice(prefix.length + 1)),
      ];
  }
  for (const suffix of standardSuffixes) {
    if (caption.endsWith(` ${suffix}`))
      return [...captionParts(caption.slice(0, -suffix.length - 1)), suffix];
  }
  return [caption];
}
function fieldLabelParts(field: AppearanceField): string[] {
  const tokens = field.pointer.split("/").filter(Boolean);
  const component = field.component.split("/").filter(Boolean);
  const leaf = tokens.at(-1) || "";
  const property =
    field.role.startsWith("keybind-") ||
    ["value", "chroma", "chromaSpeed", "chromaType"].includes(leaf)
      ? tokens.at(-2) || leaf
      : leaf;
  return [
    ...component.slice(1).flatMap((value) => captionParts(humanize(value))),
    ...captionParts(humanize(field.label || property)),
    ...(field.role === "keybind-flag"
      ? [
          (
            {
              alt: "Alt Key",
              control: "Control Key",
              shift: "Shift Key",
            } as Record<string, string>
          )[leaf] || humanize(leaf),
        ]
      : roleLabels[field.role]
        ? [roleLabels[field.role]]
        : []),
  ];
}
export function appearanceLabelKeys(): string[] {
  return [
    ...new Set(
      fields
        .flatMap(fieldLabelParts)
        .filter((key) => Object.hasOwn(japaneseMessages, key)),
    ),
  ];
}
export function appearanceEnumLabel(
  setting: Setting | undefined,
  value: JsonValue | undefined,
  translate: (key: string, values?: Record<string, string | number>) => string,
): string | undefined {
  const field = appearanceField(setting);
  if (
    field?.kind !== "enum" ||
    typeof value !== "string" ||
    !field.allowed?.includes(value)
  )
    return undefined;
  if (field.role === "keybind-value" || field.role === "keybind-modifier") {
    const key = value.replace(/^KEY_/, "");
    if (key === "NONE") return translate("未割り当て");
    const mouse = key.match(/^MOUSE(\d+)$/);
    if (mouse) return translate("マウス {0}", { 0: mouse[1] });
    const numpad = key.match(/^NUMPAD(\d+)$/);
    if (numpad) return translate("テンキー {0}", { 0: numpad[1] });
    const modifiers: Record<string, [string, string]> = {
      LSHIFT: ["Left", "Shift Key"],
      RSHIFT: ["Right", "Shift Key"],
      LCONTROL: ["Left", "Control Key"],
      RCONTROL: ["Right", "Control Key"],
    };
    const modifier = modifiers[key];
    if (modifier) return modifier.map((part) => translate(part)).join(" · ");
    const names: Record<string, string> = {
      RETURN: "Enter",
      BACK: "Backspace",
      CAPITAL: "Caps Lock",
      PRINTSC: "Print Screen",
      LCOMMAND: "Left Command",
      RCOMMAND: "Right Command",
      LWINDOWS: "Left Windows",
      RWINDOWS: "Right Windows",
      LMENU: "Left Alt",
      RMENU: "Right Alt",
    };
    return names[key] || key;
  }
  const caption = humanize(value);
  return Object.hasOwn(japaneseMessages, caption) ? translate(caption) : value;
}

export function settingLabel(
  setting: Setting,
  translate: (key: string) => string,
): string {
  const field = appearanceField(setting);
  return (field ? fieldLabelParts(field) : setting.label.split(" · "))
    .map(translate)
    .join(" · ");
}

export function appearanceColor(
  setting: Setting | undefined,
  value: JsonValue | undefined,
): { hex: string; css: string; alpha: number; opacity: number } | undefined {
  const field = appearanceField(setting);
  if (
    field?.role !== "color-value" ||
    field.kind !== "number" ||
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < (field.min ?? -2147483648) ||
    value > (field.max ?? 2147483647)
  )
    return undefined;
  const argb = value >>> 0;
  const alpha = argb >>> 24;
  const hex = `#${(argb & 0xffffff).toString(16).padStart(6, "0").toUpperCase()}`;
  return {
    hex,
    alpha,
    opacity: Math.round((alpha / 255) * 1000) / 10,
    css: `${hex}${alpha.toString(16).padStart(2, "0").toUpperCase()}`,
  };
}
