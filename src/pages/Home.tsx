import {
  ArrowDownToLine,
  ArrowRight,
  ArrowUpRight,
  Check,
  CircleDashed,
  Clock3,
  HardDrive,
  Layers3,
  Moon,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import { PrismMark } from "../components/PrismMark";
import { formatDate } from "../utils/format";
import { isDesktop } from "../services/backend";
import type { WorkspaceProps } from "./types";

export function Home({
  scan,
  backups,
  navigate,
  lastExport,
  scanFailed,
}: WorkspaceProps & { lastExport: string | null; scanFailed: boolean }) {
  const latest = backups[0];
  return (
    <div className="home-page">
      <section className="hero glass-panel">
        <div className="hero-content">
          <span className="eyebrow">
            <span className="sparkle-dot" /> YOUR SETUP, IN SYNC
          </span>
          <h2>
            いつもの設定を、
            <br />
            <span>次の場所へ。</span>
          </h2>
          <p>
            大切なプレイ環境を、ひとつの共有コードに。
            <br />
            必要な設定だけを、安全に持ち運べます。
          </p>
          <div className="hero-actions">
            <button
              className="button primary"
              onClick={() => navigate("export")}
            >
              <ArrowUpRight size={17} />
              共有コードを作成
              <ArrowRight size={16} />
            </button>
            <button className="button quiet" onClick={() => navigate("import")}>
              コードを読み込む
            </button>
          </div>
        </div>
        <div className="hero-art" aria-hidden="true">
          <div className="prism-orbit orbit-one" />
          <div className="prism-orbit orbit-two" />
          <div className="prism-glow" />
          <PrismMark className="hero-prism" />
          <span className="orbit-spark one" />
          <span className="orbit-spark two" />
          <span className="hero-art-caption">LOCAL. PRIVATE. YOURS.</span>
        </div>
      </section>
      <div className="section-heading">
        <h3>接続されたプレイ環境</h3>
        <span>
          {scanFailed
            ? scan.settings.length
              ? `前回の検出結果 · ${scan.settings.length} 個の設定`
              : "設定の検出を確認してください"
            : `${scan.settings.length} settings discovered`}
        </span>
      </div>
      <div className="source-cards">
        <section className="glass-panel source-card">
          <div className="source-icon lunar">
            <Moon size={25} />
          </div>
          <div className="source-info">
            <h3>Lunar Client</h3>
            <p>
              {scan.lunarProfiles.length
                ? `${scan.lunarProfiles.length} プロフィール · ${scan.lunarProfiles[0]}`
                : scanFailed
                  ? "プロフィール未確認"
                  : "プロフィール未検出"}
            </p>
          </div>
          <span className={`status-pill ${scan.lunarDetected ? "" : "muted"}`}>
            {scan.lunarDetected ? (
              <Check size={12} />
            ) : (
              <CircleDashed size={12} />
            )}{" "}
            {scanFailed
              ? scan.lunarDetected
                ? "前回の結果"
                : "未確認"
              : scan.lunarDetected
                ? isDesktop
                  ? "検出済み"
                  : "サンプル"
                : "未検出"}
          </span>
          <div className="source-bottom">
            <span>
              {scanFailed && !scan.lunarDetected ? (
                "未確認"
              ) : (
                <>
                  {
                    scan.settings.filter(
                      (setting) => setting.source === "lunar",
                    ).length
                  }{" "}
                  個の設定
                </>
              )}
            </span>
            <button
              className="text-button"
              onClick={() =>
                navigate(scan.lunarDetected ? "export" : "settings")
              }
            >
              {scan.lunarDetected ? "設定を見る" : "フォルダを指定"}
              <ArrowRight size={13} />
            </button>
          </div>
        </section>
        <section className="glass-panel source-card">
          <div className="source-icon minecraft">
            <SlidersHorizontal size={24} />
          </div>
          <div className="source-info">
            <h3>Minecraft</h3>
            <p>
              {scan.minecraftVersions.length
                ? scan.minecraftVersions.join(" · ")
                : scanFailed
                  ? "バージョン未確認"
                  : "バージョン未検出"}
            </p>
          </div>
          <span
            className={`status-pill ${scan.minecraftDetected ? "" : "muted"}`}
          >
            {scan.minecraftDetected ? (
              <Check size={12} />
            ) : (
              <CircleDashed size={12} />
            )}{" "}
            {scanFailed
              ? scan.minecraftDetected
                ? "前回の結果"
                : "未確認"
              : scan.minecraftDetected
                ? isDesktop
                  ? "検出済み"
                  : "サンプル"
                : "未検出"}
          </span>
          <div className="source-bottom">
            <span>
              {scanFailed && !scan.minecraftDetected ? (
                "未確認"
              ) : (
                <>
                  {
                    scan.settings.filter(
                      (setting) => setting.source === "minecraft",
                    ).length
                  }{" "}
                  個の設定
                </>
              )}
            </span>
            <button
              className="text-button"
              onClick={() =>
                navigate(scan.minecraftDetected ? "export" : "settings")
              }
            >
              {scan.minecraftDetected ? "設定を見る" : "フォルダを指定"}
              <ArrowRight size={13} />
            </button>
          </div>
        </section>
      </div>
      <div className="home-bottom">
        <section className="glass-panel activity-card">
          <div className="panel-heading">
            <h3>最近のアクティビティ</h3>
            <Clock3 size={16} />
          </div>
          <div className="activity-row">
            <div className="activity-icon">
              <HardDrive size={17} />
            </div>
            <div>
              <strong>
                {latest ? "バックアップを保存" : "バックアップはまだありません"}
              </strong>
              <p>
                {latest
                  ? `${latest.files.length} ファイル · ${formatDate(latest.createdAt)}`
                  : "変更する前に、元の設定を残します"}
              </p>
            </div>
            <button
              className="icon-button"
              onClick={() => navigate("backups")}
              aria-label="バックアップを開く"
            >
              <ArrowUpRight size={17} />
            </button>
          </div>
          <div className="activity-row">
            <div className="activity-icon violet">
              <ArrowUpRight size={17} />
            </div>
            <div>
              <strong>
                {lastExport ? "共有コードを作成" : "はじめての設定共有"}
              </strong>
              <p>
                {lastExport
                  ? formatDate(lastExport)
                  : "あなたのセットアップを持ち運ぼう"}
              </p>
            </div>
            <button
              className="icon-button"
              onClick={() => navigate("export")}
              aria-label="共有コード作成を開く"
            >
              <ArrowUpRight size={17} />
            </button>
          </div>
        </section>
        <section className="privacy-card">
          <ShieldCheck size={26} />
          <h3>
            あなたの設定は、
            <br />
            あなたの手元に。
          </h3>
          <p>
            ローカルで完結。アカウント情報を共有せず、適用前にはバックアップ。
          </p>
          <div>
            <span>
              <Layers3 size={12} /> 選んだ項目だけ
            </span>
            <span>
              <ArrowDownToLine size={12} /> 安全に復元
            </span>
          </div>
        </section>
      </div>
    </div>
  );
}
