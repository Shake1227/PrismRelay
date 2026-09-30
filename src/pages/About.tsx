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
        <span className="eyebrow">YOUR SETTINGS. ANYWHERE.</span>
        <h2>
          Prism Relay<span>v{info.version}</span>
        </h2>
        <p>
          Minecraft と Lunar Client のセットアップを、
          <br />
          必要な分だけ、あなたの次のデバイスへ。
        </p>
        <div className="about-badges">
          <span>
            <ShieldCheck size={13} />
            Local first
          </span>
          <span>
            <Scale size={13} />
            GPL v3
          </span>
          <span>Open source</span>
        </div>
        <div className="about-links">
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
            リリースノート
            <ArrowUpRight size={14} />
          </button>
        </div>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <div>
            <h3>更新を確認</h3>
            <p>GitHub Releases の最新バージョンを確認します。</p>
          </div>
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void check()}
          >
            <RefreshCw size={15} className={busy ? "spinning" : ""} />
            {busy ? "確認中…" : "更新を確認"}
          </button>
        </div>
        {update && (
          <div className="update-result">
            <Check size={16} />
            <span>
              {!isDesktop
                ? "サンプルモードです。更新確認はデスクトップアプリで実行できます。"
                : update.updateAvailable
                  ? `新しいバージョン ${update.latestVersion} を利用できます。`
                  : "最新のバージョンです。"}
            </span>
            {update.updateAvailable && (
              <button
                className="text-button"
                onClick={() =>
                  void backend.openProjectPage("releases").catch(onError)
                }
              >
                ダウンロード
                <ArrowUpRight size={13} />
              </button>
            )}
          </div>
        )}
        <p className="fine-print">自動ダウンロードや自動適用は行いません。</p>
      </section>
      <section className="glass-panel settings-section">
        <div className="panel-heading">
          <h3>オープンソースライセンス</h3>
          <button
            className="text-button"
            onClick={() =>
              void backend.openProjectPage("license").catch(onError)
            }
          >
            GPL を読む
            <ArrowUpRight size={13} />
          </button>
        </div>
        <p className="license-copy">
          Prism Relay は {info.license || "GPL-3.0-or-later"}{" "}
          で公開されています。React / MIT、Tauri / MIT・Apache-2.0、Lucide /
          ISC、QRCode / MIT をはじめ、オープンソースライブラリを使用しています。
        </p>
        <button
          className="text-button license-button"
          disabled={licensesLoading}
          onClick={() => void showLicenses()}
        >
          {licensesLoading ? "読み込み中…" : "使用ライブラリのライセンス一覧"}
        </button>
      </section>
      <p className="disclaimer">
        Minecraft、Lunar Client、Microsoft、Mojang
        とは独立した非公式ツールです。
        <br />
        各製品名・商標はそれぞれの権利者に帰属します。
      </p>
      {licenses && (
        <Modal
          title="オープンソースライセンス"
          onClose={() => setLicenses(null)}
        >
          <pre className="license-notices">{licenses}</pre>
        </Modal>
      )}
    </div>
  );
}
