import { useI18n } from "../i18n";
import { useState } from "react";
import {
  ArrowUpRight,
  Check,
  Github,
  RefreshCw,
  Scale,
  ShieldCheck,
} from "lucide-react";
import type { AppInfo, UpdateInfo } from "../models";
import { backend, isDesktop } from "../services/backend";
import { PrismMark } from "../components/PrismMark";
import { Modal } from "../components/Modal";
export function About({
  info,
  onError,
}: {
  info: AppInfo;
  onError: (error: unknown) => void;
}) {
  const { t } = useI18n();
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [licenses, setLicenses] = useState<string | null>(null);
  const [licensesLoading, setLicensesLoading] = useState(false);
  const check = async () => {
    setBusy(true);
    try {
      setUpdate(await backend.updates());
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };
  const showLicenses = async () => {
    setLicensesLoading(true);
    try {
      const response = await fetch("/third-party-notices.txt");
      if (!response.ok) throw new Error("License notices could not be loaded");
      setLicenses(await response.text());
    } catch (error) {
      onError(error);
    } finally {
      setLicensesLoading(false);
    }
  };
  return (
    <div className="about-page">
      <section className="glass-panel about-brand">
        <PrismMark className="about-prism" />
        <span className="eyebrow">{t("MINECRAFT SETTINGS SHARING")}</span>
        <h2>
          Prism Relay<span>v{info.version}</span>
        </h2>
        <p>
          {t("Minecraft と Lunar Client のセットアップを、")}
          <br />
          {t("選択した項目だけ共有する設定ツールです。")}
        </p>
        <div className="about-badges">
          <span>{t("作者 · Shake_1227")}</span>
          <span>
            <ShieldCheck size={13} />
            {t("Local first")}
          </span>
          <span>
            <Scale size={13} />
            GPL v3
          </span>
          <span>{t("Open source")}</span>
        </div>
        <div className="about-links">
          <button
            className="button secondary"
            onClick={() => void backend.openProjectPage("x").catch(onError)}
            aria-label={t("作者の X を開く")}
          >
            X
            <ArrowUpRight size={14} />
          </button>
          <button
            className="button secondary"
            onClick={() =>
              void backend.openProjectPage("repository").catch(onError)
            }
          >
            <Github size={17} />
            GitHub
            <ArrowUpRight size={14} />
          </button>
          <button
            className="button secondary"
            onClick={() =>
              void backend.openProjectPage("releases").catch(onError)
            }
          >
            {t("リリースノート")}
            <ArrowUpRight size={14} />
          </button>
        </div>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>{t("更新を確認")}</h3>
            <p>{t("GitHub Releases の最新バージョンを確認します。")}</p>
          </div>
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void check()}
          >
            <RefreshCw size={15} className={busy ? "spinning" : ""} />
            {busy ? t("確認中…") : t("更新を確認")}
          </button>
        </div>
        {update && (
          <div className="update-result">
            <Check size={16} />
            <span>
              {!isDesktop
                ? t(
                    "サンプルモードです。更新確認はデスクトップアプリで実行できます。",
                  )
                : update.updateAvailable
                  ? t("新しいバージョン {0} を利用できます。", {
                      "0": update.latestVersion,
                    })
                  : t("最新のバージョンです。")}
            </span>
            {update.updateAvailable && (
              <button
                className="text-button"
                onClick={() =>
                  void backend.openProjectPage("releases").catch(onError)
                }
              >
                {t("ダウンロード")}
                <ArrowUpRight size={13} />
              </button>
            )}
          </div>
        )}
        <p className="fine-print">
          {t("自動ダウンロードや自動適用は行いません。")}
        </p>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <h3>{t("オープンソースライセンス")}</h3>
          <button
            className="text-button"
            onClick={() =>
              void backend.openProjectPage("license").catch(onError)
            }
          >
            {t("GPL を読む")}
            <ArrowUpRight size={13} />
          </button>
        </div>
        <p className="license-copy">
          {t("Prism Relay は")} {info.license || "GPL-3.0-or-later"}{" "}
          {t(
            "で公開されています。React / MIT、Tauri / MIT・Apache-2.0、Lucide / ISC、QRCode / MIT をはじめ、オープンソースライブラリを使用しています。",
          )}
        </p>
        <button
          className="text-button license-button"
          disabled={licensesLoading}
          onClick={() => void showLicenses()}
        >
          {licensesLoading
            ? t("読み込み中…")
            : t("使用ライブラリのライセンス一覧")}
        </button>
      </section>
      <p className="disclaimer">
        {t(
          "Minecraft、Lunar Client、Microsoft、Mojang とは独立した非公式ツールです。",
        )}
        <br />
        {t("各製品名・商標はそれぞれの権利者に帰属します。")}
      </p>
      {licenses && (
        <Modal
          title={t("オープンソースライセンス")}
          onClose={() => setLicenses(null)}
        >
          <pre className="license-notices">{licenses}</pre>
        </Modal>
      )}
    </div>
  );
}
