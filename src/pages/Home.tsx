import { useI18n } from "../i18n";
import {
  ArrowDownToLine,
  ArrowRight,
  ArrowUpRight,
  Check,
  CircleDashed,
  Clock3,
  HardDrive,
  Layers3,
  ShieldCheck,
} from "lucide-react";
import { PrismMark } from "../components/PrismMark";
import { ApplicationIcon } from "../components/ApplicationIcon";
import { formatDate } from "../utils/format";
import { isDesktop } from "../services/backend";
import type { WorkspaceProps } from "./types";
export function Home({
  scan,
  backups,
  navigate,
  lastExport,
  scanFailed,
}: WorkspaceProps & {
  lastExport: string | null;
  scanFailed: boolean;
}) {
  const { t, locale } = useI18n();
  const latest = backups[0];
  return (
    <div className="home-page">
      <section className="hero glass-panel">
        <div className="hero-content">
          <span className="eyebrow">
            <span className="sparkle-dot" />
            {t("SHARE SETTINGS")}
          </span>
          <h2>
            {t("設定を選んで、")}
            <br />
            <span>{t("共有コードに。")}</span>
          </h2>
          <p>
            {t("Minecraft と Lunar Client の設定を選択し、")}
            <br />
            {t("別のデバイスへ共有できます。")}
          </p>
          <div className="hero-actions">
            <button
              className="button primary"
              onClick={() => navigate("export")}
            >
              <ArrowUpRight size={17} />
              {t("共有コードを作成")}
              <ArrowRight size={16} />
            </button>
            <button className="button quiet" onClick={() => navigate("import")}>
              {t("コードを読み込む")}
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
          <span className="hero-art-caption">
            {t("EXPORT \xB7 IMPORT \xB7 RESTORE")}
          </span>
        </div>
      </section>
      <div className="section-heading">
        <h3>{t("接続されたプレイ環境")}</h3>
        <span>
          {scanFailed
            ? scan.settings.length
              ? t("前回の検出結果 · {0} 個の設定", {
                  "0": scan.settings.length,
                })
              : t("設定の検出を確認してください")
            : t("{0} settings discovered", {
                "0": scan.settings.length,
              })}
        </span>
      </div>
      <div className="source-cards">
        <section className="glass-panel source-card">
          <div className="source-icon lunar">
            <ApplicationIcon
              source="lunar"
              data={scan.applicationIcons?.lunar}
            />
          </div>
          <div className="source-info">
            <h3>Lunar Client</h3>
            <p>
              {scan.lunarProfiles.length
                ? t("{0} プロフィール · {1}", {
                    "0": scan.lunarProfiles.length,
                    "1": scan.lunarProfiles[0],
                  })
                : scanFailed
                  ? t("プロフィール未確認")
                  : t("プロフィール未検出")}
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
                ? t("前回の結果")
                : t("未確認")
              : scan.lunarDetected
                ? isDesktop
                  ? t("検出済み")
                  : t("サンプル")
                : t("未検出")}
          </span>
          <div className="source-bottom">
            <span>
              {scanFailed && !scan.lunarDetected ? (
                t("未確認")
              ) : (
                <>
                  {
                    scan.settings.filter(
                      (setting) => setting.source === "lunar",
                    ).length
                  }{" "}
                  {t("個の設定")}
                </>
              )}
            </span>
            <button
              className="text-button"
              onClick={() =>
                navigate(scan.lunarDetected ? "export" : "settings")
              }
            >
              {scan.lunarDetected ? t("設定を見る") : t("フォルダを指定")}
              <ArrowRight size={13} />
            </button>
          </div>
        </section>
        <section className="glass-panel source-card">
          <div className="source-icon minecraft">
            <ApplicationIcon
              source="minecraft"
              data={scan.applicationIcons?.minecraft}
            />
          </div>
          <div className="source-info">
            <h3>Minecraft</h3>
            <p>
              {scan.minecraftVersions.length
                ? scan.minecraftVersions.join(" · ")
                : scanFailed
                  ? t("バージョン未確認")
                  : t("バージョン未検出")}
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
                ? t("前回の結果")
                : t("未確認")
              : scan.minecraftDetected
                ? isDesktop
                  ? t("検出済み")
                  : t("サンプル")
                : t("未検出")}
          </span>
          <div className="source-bottom">
            <span>
              {scanFailed && !scan.minecraftDetected ? (
                t("未確認")
              ) : (
                <>
                  {
                    scan.settings.filter(
                      (setting) => setting.source === "minecraft",
                    ).length
                  }{" "}
                  {t("個の設定")}
                </>
              )}
            </span>
            <button
              className="text-button"
              onClick={() =>
                navigate(scan.minecraftDetected ? "export" : "settings")
              }
            >
              {scan.minecraftDetected ? t("設定を見る") : t("フォルダを指定")}
              <ArrowRight size={13} />
            </button>
          </div>
        </section>
      </div>
      <div className="home-bottom">
        <section className="glass-panel activity-card">
          <div className="panel-heading">
            <h3>{t("最近のアクティビティ")}</h3>
            <Clock3 size={16} />
          </div>
          <div className="activity-row">
            <div className="activity-icon">
              <HardDrive size={17} />
            </div>
            <div>
              <strong>
                {latest
                  ? t("バックアップを保存")
                  : t("バックアップはまだありません")}
              </strong>
              <p>
                {latest
                  ? t("{0} ファイル · {1}", {
                      "0": latest.files.length,
                      "1": formatDate(latest.createdAt, locale),
                    })
                  : t("変更する前に、元の設定を残します")}
              </p>
            </div>
            <button
              className="icon-button"
              onClick={() => navigate("backups")}
              aria-label={t("バックアップを開く")}
            >
              <ArrowUpRight size={17} />
            </button>
          </div>
          <div className="activity-row">
            <div className="activity-icon violet">
              <ArrowUpRight size={17} />
            </div>
            <div>
              <strong>{t("共有コードを作成")}</strong>
              <p>
                {lastExport
                  ? formatDate(lastExport, locale)
                  : t("設定を選択してコードを作成")}
              </p>
            </div>
            <button
              className="icon-button"
              onClick={() => navigate("export")}
              aria-label={t("共有コード作成を開く")}
            >
              <ArrowUpRight size={17} />
            </button>
          </div>
        </section>
        <section className="privacy-card">
          <ShieldCheck size={26} />
          <h3>{t("バックアップ")}</h3>
          <p>
            {t(
              "適用前に元の設定を保存します。変更した項目は差分で確認できます。",
            )}
          </p>
          <div>
            <span>
              <Layers3 size={12} />
              {t("選んだ項目だけ")}
            </span>
            <span>
              <ArrowDownToLine size={12} />
              {t("バックアップから復元")}
            </span>
          </div>
        </section>
      </div>
    </div>
  );
}
