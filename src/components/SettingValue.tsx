import type { JsonValue, Setting } from "../models";
import { useI18n } from "../i18n";
import {
  appearanceColor,
  appearanceEnumLabel,
  appearanceField,
} from "../utils/appearance";
import { formatValue } from "../utils/format";
import "./SettingValue.css";

export function SettingValue({
  setting,
  value,
  text,
}: {
  setting?: Setting;
  value?: JsonValue;
  text?: string;
}) {
  const { t } = useI18n();
  const color = appearanceColor(setting, value);
  if (color)
    return (
      <span
        className="setting-color"
        aria-label={t("色 {0}、不透明度 {1}%", {
          0: color.hex,
          1: color.opacity,
        })}
      >
        <span className="setting-color-swatch" aria-hidden="true">
          <span style={{ backgroundColor: color.css }} />
        </span>
        <span>{color.hex}</span>
        <span className="setting-color-opacity">{color.opacity}%</span>
      </span>
    );
  if (appearanceField(setting)?.role === "chroma-mode") {
    if (value === "wave") return <>{t("Wave")}</>;
    if (value === "shift") return <>{t("Shift")}</>;
  }
  return (
    <>{text ?? appearanceEnumLabel(setting, value, t) ?? formatValue(value)}</>
  );
}
