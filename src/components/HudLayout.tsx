import type { HudLayoutPreview } from "../models";
import { useI18n } from "../i18n";
import "./HudLayout.css";

export function HudLayoutControls({
  enabled,
  onEnabledChange,
  manual,
  onAutomatic,
}: {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
  manual?: {
    width: string;
    height: string;
    valid: boolean;
    onChange: (width: string, height: string) => void;
  };
  onAutomatic: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="hud-layout-controls">
      <label className="toggle-label">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(event) => onEnabledChange(event.target.checked)}
        />
        {t("HUD位置をウインドウサイズに合わせる")}
      </label>
      {enabled && manual && (
        <fieldset className="hud-manual-size">
          <legend>{t("最大化時のウインドウサイズ")}</legend>
          <div className="hud-size-fields">
            <label>
              <span>{t("幅（px）")}</span>
              <input
                type="number"
                min="320"
                max="32768"
                step="1"
                inputMode="numeric"
                value={manual.width}
                onChange={(event) =>
                  manual.onChange(event.target.value, manual.height)
                }
              />
            </label>
            <span className="hud-size-times" aria-hidden="true">
              ×
            </span>
            <label>
              <span>{t("高さ（px）")}</span>
              <input
                type="number"
                min="240"
                max="32768"
                step="1"
                inputMode="numeric"
                value={manual.height}
                onChange={(event) =>
                  manual.onChange(manual.width, event.target.value)
                }
              />
            </label>
            <button className="text-button" type="button" onClick={onAutomatic}>
              {t("自動検出に戻す")}
            </button>
          </div>
          {!manual.valid && (manual.width || manual.height) && (
            <p className="hud-size-validation" role="status">
              {t("幅は320〜32768 px、高さは240〜32768 pxで入力してください。")}
            </p>
          )}
        </fieldset>
      )}
    </div>
  );
}

export function HudLayoutSummary({ layout }: { layout?: HudLayoutPreview }) {
  const { t } = useI18n();
  if (!layout || (layout.status === "unchanged" && !layout.windowSize))
    return null;
  const blocked =
    layout.status === "missing-source" || layout.status === "unavailable";
  return (
    <div
      className={`hud-layout-summary ${blocked ? "hud-layout-warning" : ""}`}
      role={blocked ? "alert" : undefined}
    >
      <strong>{t("HUD位置")}</strong>
      {layout.windowSize && (
        <p>
          <span className="hud-window-size">
            {layout.windowSize.width} × {layout.windowSize.height} px
          </span>
          <span>{t("最大化・ウインドウ表示")}</span>
        </p>
      )}
      {layout.status === "adjusted" && (
        <p>{t("{0}項目の位置を調整", { 0: layout.adjustedCount })}</p>
      )}
      {layout.status === "unchanged" && <p>{t("位置の調整は不要です。")}</p>}
      {layout.status === "missing-source" && (
        <p>
          {t(
            "このコードにはHUDのサイズ情報がありません。共有元でコードを作り直すか、自動調整をオフにしてください。",
          )}
        </p>
      )}
      {layout.status === "unavailable" && (
        <p>
          {t(
            "HUD位置の補正に必要なサイズや設定を確認できません。サイズを手動入力するか、自動調整をオフにしてください。",
          )}
        </p>
      )}
    </div>
  );
}
