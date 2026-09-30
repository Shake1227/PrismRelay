import { useI18n } from "../i18n";
import { useState } from "react";
import {
  Check,
  FolderOpen,
  Laptop,
  Moon,
  RefreshCw,
  ShieldCheck,
  Sun,
} from "lucide-react";
import type { Preferences, Source, Theme } from "../models";
import { backend, isDesktop } from "../services/backend";
import type { WorkspaceProps } from "./types";
export function Settings({
  scan,
  onError,
  onRefresh,
  preferences,
  onChange,
}: WorkspaceProps & {
  preferences: Preferences;
  onChange: (preferences: Preferences) => void;
}) {
  const { t } = useI18n();
  const [paths, setPaths] = useState({
    minecraftRoot: preferences.request.minecraftRoot || "",
    lunarRoot: preferences.request.lunarRoot || "",
  });
  const [busy, setBusy] = useState(false);
  const themes: {
    id: Theme;
    label: string;
    icon: typeof Moon;
  }[] = [
    {
      id: "dark",
      label: t("ダーク"),
      icon: Moon,
    },
    {
      id: "light",
      label: t("ライト"),
      icon: Sun,
    },
    {
      id: "system",
      label: t("システム"),
      icon: Laptop,
    },
  ];
  const choose = async (source: Source) => {
    try {
      const path = await backend.chooseDirectory(
        source,
        t("{0} 設定フォルダ", {
          0: source === "minecraft" ? "Minecraft" : "Lunar Client",
        }),
      );
      if (path) {
        const key = source === "minecraft" ? "minecraftRoot" : "lunarRoot";
        setPaths((current) => ({
          ...current,
          [key]: path,
        }));
        onChange({
          ...preferences,
          request: {
            ...preferences.request,
            [key]: path,
          },
        });
      }
    } catch (error) {
      onError(error);
    }
  };
  return (
    <div className="settings-page">
      <section className="glass-panel settings-section">
        <div className="setting-row language-setting">
          <div>
            <h3>{t("表示言語")}</h3>
            <p>{t("すべての画面に使用する言語を選択します。")}</p>
          </div>
          <select
            aria-label={t("表示言語")}
            value={preferences.language}
            onChange={(event) =>
              onChange({
                ...preferences,
                language: event.target.value as Preferences["language"],
              })
            }
          >
            <option value="en">English</option>
            <option value="ja">日本語</option>
            <option value="ko">한국어</option>
            <option value="zh">简体中文</option>
            <option value="fi">Suomi</option>
            <option value="es">Español</option>
            <option value="de">Deutsch</option>
          </select>
        </div>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>{t("テーマと表示")}</h3>
            <p>{t("画面のテーマとアニメーションを設定します。")}</p>
          </div>
        </div>
        <div className="theme-options">
          {themes.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`theme-option theme-${id} ${preferences.theme === id ? "selected" : ""}`}
              onClick={() =>
                onChange({
                  ...preferences,
                  theme: id,
                })
              }
            >
              <div className="theme-preview">
                <div />
                <div>
                  <i />
                  <i />
                  <i />
                </div>
              </div>
              <span>
                <Icon size={15} />
                {label}
                {preferences.theme === id && <Check size={14} />}
              </span>
            </button>
          ))}
        </div>
        <div className="setting-row">
          <div>
            <strong>{t("アニメーションを抑える")}</strong>
            <p>
              {t(
                "動きや画面の切り替えを控えめにします。OS の設定も反映します。",
              )}
            </p>
          </div>
          <input
            className="switch"
            type="checkbox"
            role="switch"
            aria-label={t("アニメーションを抑える")}
            checked={preferences.reduceMotion}
            onChange={(event) =>
              onChange({
                ...preferences,
                reduceMotion: event.target.checked,
              })
            }
          />
        </div>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>{t("設定フォルダ")}</h3>
            <p>{t("自動検出できない場合は、保存先を指定できます。")}</p>
          </div>
          <button
            className="button secondary compact"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void onRefresh().finally(() => setBusy(false));
            }}
          >
            <RefreshCw size={14} className={busy ? "spinning" : ""} />
            {t("再検出")}
          </button>
        </div>
        {(["minecraft", "lunar"] as const).map((source) => {
          const detected =
            source === "minecraft"
              ? scan.minecraftDetected
              : scan.lunarDetected;
          const key = source === "minecraft" ? "minecraftRoot" : "lunarRoot";
          return (
            <div className="folder-setting" key={source}>
              <div>
                <strong>
                  {source === "minecraft" ? "Minecraft" : "Lunar Client"}
                </strong>
                <span className={`status-pill ${detected ? "" : "muted"}`}>
                  {detected ? t("検出済み") : t("未検出")}
                </span>
              </div>
              <div className="folder-input">
                <input
                  aria-label={t("{0} 設定フォルダ", {
                    "0": source,
                  })}
                  value={paths[key]}
                  placeholder={t("自動検出を使用")}
                  disabled={!isDesktop}
                  onChange={(event) =>
                    setPaths({
                      ...paths,
                      [key]: event.target.value,
                    })
                  }
                />
                <button
                  className="icon-button"
                  aria-label={t("{0} フォルダを選択", {
                    "0": source,
                  })}
                  disabled={!isDesktop}
                  onClick={() => void choose(source)}
                >
                  <FolderOpen size={17} />
                </button>
                <button
                  className="text-button"
                  disabled={
                    !isDesktop ||
                    paths[key] === (preferences.request[key] || "")
                  }
                  onClick={() =>
                    onChange({
                      ...preferences,
                      request: {
                        ...preferences.request,
                        [key]: paths[key].trim() || undefined,
                      },
                    })
                  }
                >
                  {t("使用")}
                </button>
              </div>
              {scan.files
                .filter((file) => file.source === source)
                .slice(0, 2)
                .map((file) => (
                  <p className="detected-path" key={file.id}>
                    {isDesktop
                      ? file.path
                      : file.path.replace(/^サンプル/, t("サンプル"))}
                  </p>
                ))}
            </div>
          );
        })}
      </section>
      <section className="glass-panel settings-section privacy-setting">
        <ShieldCheck size={24} />
        <div>
          <h3>{t("ローカル処理と通信")}</h3>
          <p>
            {t(
              "設定の処理はローカルで行います。テレメトリやアクセス解析はありません。更新確認は、About から手動で行う場合のみ GitHub に接続します。",
            )}
          </p>
        </div>
        <span className="status-pill">
          <Check size={12} />
          {t("LOCAL ONLY")}
        </span>
      </section>
    </div>
  );
}
