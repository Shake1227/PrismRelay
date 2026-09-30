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
  const [paths, setPaths] = useState({
    minecraftRoot: preferences.request.minecraftRoot || "",
    lunarRoot: preferences.request.lunarRoot || "",
  });
  const [busy, setBusy] = useState(false);
  const themes: { id: Theme; label: string; icon: typeof Moon }[] = [
    { id: "dark", label: "ダーク", icon: Moon },
    { id: "light", label: "ライト", icon: Sun },
    { id: "system", label: "システム", icon: Laptop },
  ];
  const choose = async (source: Source) => {
    try {
      const path = await backend.chooseDirectory(source);
      if (path) {
        const key = source === "minecraft" ? "minecraftRoot" : "lunarRoot";
        setPaths((current) => ({ ...current, [key]: path }));
        onChange({
          ...preferences,
          request: { ...preferences.request, [key]: path },
        });
      }
    } catch (error) {
      onError(error);
    }
  };
  return (
    <div className="settings-page">
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>あなたに合う見た目に</h3>
            <p>心地よく使える表示を選べます。</p>
          </div>
        </div>
        <div className="theme-options">
          {themes.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`theme-option theme-${id} ${preferences.theme === id ? "selected" : ""}`}
              onClick={() => onChange({ ...preferences, theme: id })}
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
            <strong>アニメーションを抑える</strong>
            <p>動きや画面の切り替えを控えめにします。OS の設定も反映します。</p>
          </div>
          <input
            className="switch"
            type="checkbox"
            role="switch"
            aria-label="アニメーションを抑える"
            checked={preferences.reduceMotion}
            onChange={(event) =>
              onChange({ ...preferences, reduceMotion: event.target.checked })
            }
          />
        </div>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>設定フォルダ</h3>
            <p>自動検出できない場合は、保存先を指定できます。</p>
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
            再検出
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
                  {detected ? "検出済み" : "未検出"}
                </span>
              </div>
              <div className="folder-input">
                <input
                  aria-label={`${source} 設定フォルダ`}
                  value={paths[key]}
                  placeholder="自動検出を使用"
                  disabled={!isDesktop}
                  onChange={(event) =>
                    setPaths({ ...paths, [key]: event.target.value })
                  }
                />
                <button
                  className="icon-button"
                  aria-label={`${source} フォルダを選択`}
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
                  使用
                </button>
              </div>
              {scan.files
                .filter((file) => file.source === source)
                .slice(0, 2)
                .map((file) => (
                  <p className="detected-path" key={file.id}>
                    {file.path}
                  </p>
                ))}
            </div>
          );
        })}
      </section>
      <section className="glass-panel settings-section privacy-setting">
        <ShieldCheck size={24} />
        <div>
          <h3>プライバシーは、標準装備。</h3>
          <p>
            設定の処理はローカルで行います。テレメトリやアクセス解析はありません。更新確認は、About
            から手動で行う場合のみ GitHub に接続します。
          </p>
        </div>
        <span className="status-pill">
          <Check size={12} />
          LOCAL ONLY
        </span>
      </section>
    </div>
  );
}
