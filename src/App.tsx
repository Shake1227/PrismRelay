import { I18nProvider, useI18n } from "./i18n";
import type { Dispatch, SetStateAction } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpRight,
  Check,
  ChevronRight,
  FlaskConical,
  HardDrive,
  Home as HomeIcon,
  Info,
  RefreshCw,
  Settings2,
  ShieldCheck,
  X,
} from "lucide-react";
import type {
  AppInfo,
  BackupManifest,
  Page,
  ScanReport,
  Preferences,
} from "./models";
import { backend, isDesktop } from "./services/backend";
import { APP_VERSION } from "./version";
import { usePreferences } from "./hooks/usePreferences";
import { PrismMark } from "./components/PrismMark";
import { ErrorBanner } from "./components/ErrorBanner";
import { Home } from "./pages/Home";
import { Export } from "./pages/Export";
import { Import } from "./pages/Import";
import { Backups } from "./pages/Backups";
import { Settings } from "./pages/Settings";
import { About } from "./pages/About";
import { createRequestGuard } from "./utils/requestGuard";
const navigation = [
  {
    id: "home",
    label: "ホーム",
    title: "概要",
    icon: HomeIcon,
    subtitle: "検出したアプリと設定を確認します。",
  },
  {
    id: "export",
    label: "エクスポート",
    title: "共有コードの作成",
    icon: ArrowUpRight,
    subtitle: "共有する設定を選択してコードを作成します。",
  },
  {
    id: "import",
    label: "インポート",
    title: "設定の読み込み",
    icon: ArrowDownToLine,
    subtitle: "共有コードを読み込み、設定の差分を確認します。",
  },
  {
    id: "backups",
    label: "バックアップ",
    title: "バックアップ",
    icon: HardDrive,
    subtitle: "保存した設定の確認と復元を行います。",
  },
  {
    id: "settings",
    label: "設定",
    title: "環境設定",
    icon: Settings2,
    subtitle: "表示言語、テーマ、設定フォルダを変更します。",
  },
  {
    id: "about",
    label: "アプリについて",
    title: "Prism Relay について",
    icon: Info,
    subtitle: "バージョン、作者、ライセンスを確認します。",
  },
] as const;
const emptyScan: ScanReport = {
  settings: [],
  files: [],
  minecraftDetected: false,
  lunarDetected: false,
  minecraftVersions: [],
  lunarProfiles: [],
  warnings: [],
  platform: "",
  runningProcesses: [],
};
function WorkspaceApp({
  preferences,
  setPreferences,
}: {
  preferences: Preferences;
  setPreferences: Dispatch<SetStateAction<Preferences>>;
}) {
  const { t } = useI18n();
  const mainRef = useRef<HTMLElement>(null);
  const [page, setPage] = useState<Page>("home");
  const [scan, setScan] = useState<ScanReport>(emptyScan);
  const [backups, setBackups] = useState<BackupManifest[]>([]);
  const [info, setInfo] = useState<AppInfo>({
    name: "Prism Relay",
    version: APP_VERSION,
    platform: "",
    repository: "https://github.com/SHake1227/prism-relay",
    license: "GPL-3.0-or-later",
  });
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanFailed, setScanFailed] = useState(false);
  const scanGuard = useRef(createRequestGuard());
  const [lastExport, setLastExport] = useState<string | null>(null);
  const onError = useCallback(
    (value: unknown) =>
      setError(
        value instanceof Error
          ? value.message
          : typeof value === "string"
            ? value
            : JSON.stringify(value),
      ),
    [],
  );
  const onNotice = useCallback((message: string) => setNotice(message), []);
  const refresh = useCallback(async () => {
    const isCurrent = scanGuard.current.begin();
    setLoading(true);
    const results = await Promise.allSettled([
      backend.scan(preferences.request),
      backend.backups(),
    ]);
    if (!isCurrent()) return;
    if (results[0].status === "fulfilled") {
      setScan(results[0].value);
      setScanFailed(false);
    } else {
      onError(results[0].reason);
      setScanFailed(true);
    }
    if (results[1].status === "fulfilled") setBackups(results[1].value);
    else onError(results[1].reason);
    setLoading(false);
  }, [preferences.request, onError]);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    mainRef.current?.scrollTo({
      top: 0,
    });
  }, [page]);
  useEffect(() => {
    void backend.info().then(setInfo).catch(onError);
  }, [onError]);
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(null), 5000);
    return () => window.clearTimeout(timer);
  }, [notice]);
  const active = navigation.find((item) => item.id === page)!;
  const shared = {
    scan,
    backups,
    request: preferences.request,
    onError,
    onNotice,
    onRefresh: refresh,
    navigate: setPage,
  };
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <PrismMark className="brand-mark" />
          <div>
            <strong>
              Prism<span>Relay</span>
            </strong>
            <small>v{info.version}</small>
          </div>
        </div>
        <div className="workspace-label">{t("WORKSPACE")}</div>
        <nav aria-label={t("メインナビゲーション")}>
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`nav-item ${page === id ? "active" : ""}`}
              onClick={() => {
                setPage(id);
                setError(null);
              }}
              aria-current={page === id ? "page" : undefined}
              aria-label={t(label)}
            >
              <Icon size={18} />
              <span>{t(label)}</span>
              {page === id && <ChevronRight size={13} />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-status">
            <span className="status-dot" />
            <div>
              <strong>
                {isDesktop ? t("ローカルで動作中") : t("サンプルモード")}
              </strong>
              <small>
                {isDesktop
                  ? t("設定ファイルをローカルで処理")
                  : t("実際の設定は変更しません")}
              </small>
            </div>
            <ShieldCheck size={16} />
          </div>
          <div className="sidebar-version">
            <span>PRISM RELAY</span>
            <span>v{info.version}</span>
          </div>
        </div>
      </aside>
      <main className="main-content" ref={mainRef}>
        <header className="page-header">
          <div>
            <div className="header-breadcrumb">
              {t("Workspace")}
              <ChevronRight size={11} /> {t(active.label)}
            </div>
            <h1>{t(active.title)}</h1>
            <p>{t(active.subtitle)}</p>
          </div>
          <div className="header-actions">
            <span className="offline-badge">
              <span />
              {isDesktop ? t("LOCAL FIRST") : t("PREVIEW")}
            </span>
            <button
              className="icon-button refresh-button"
              onClick={() => void refresh()}
              disabled={loading}
              aria-label={t("設定を再検出")}
            >
              <RefreshCw size={17} className={loading ? "spinning" : ""} />
            </button>
          </div>
        </header>
        {!isDesktop && (
          <div className="demo-banner">
            <FlaskConical size={15} />
            <span>
              {t(
                "サンプルモード — デモ用の設定を表示しています。実際のファイルにはアクセスしません。",
              )}
            </span>
            <span className="demo-label">{t("DEMO")}</span>
          </div>
        )}
        <ErrorBanner error={error} onClose={() => setError(null)} />
        {scan.warnings.length > 0 && (
          <details className="scan-warnings">
            <summary>
              {scan.warnings.length} {t("件の検出メッセージ")}
            </summary>
            {scan.warnings.map((warning, index) => (
              <p key={index}>{warning}</p>
            ))}
          </details>
        )}
        {loading && !scan.settings.length ? (
          <div className="loading-screen">
            <PrismMark className="loading-prism" />
            <h2>{t("プレイ環境を探しています…")}</h2>
            <p>{t("設定ファイルを読み取り専用で確認しています。")}</p>
          </div>
        ) : (
          <div className="page-content" key={page}>
            {page === "home" && (
              <Home
                {...shared}
                lastExport={lastExport}
                scanFailed={scanFailed}
              />
            )}{" "}
            {page === "export" && (
              <Export
                {...shared}
                onCreated={() => setLastExport(new Date().toISOString())}
              />
            )}{" "}
            {page === "import" && <Import {...shared} />}{" "}
            {page === "backups" && <Backups {...shared} />}{" "}
            {page === "settings" && (
              <Settings
                {...shared}
                preferences={preferences}
                onChange={setPreferences}
              />
            )}{" "}
            {page === "about" && <About info={info} onError={onError} />}
          </div>
        )}
        <footer className="main-footer">
          <span>{t("Minecraft & Lunar Client settings")}</span>
          <span>
            <ShieldCheck size={11} />
            {t("設定データは送信されません")}
          </span>
        </footer>
      </main>
      {notice && (
        <div className="toast" role="status">
          <Check size={17} />
          <span>{notice}</span>
          <button
            className="icon-button"
            onClick={() => setNotice(null)}
            aria-label={t("通知を閉じる")}
          >
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
export default function App() {
  const { preferences, setPreferences } = usePreferences();
  useEffect(() => {
    document.documentElement.lang = preferences.language;
  }, [preferences.language]);
  return (
    <I18nProvider language={preferences.language}>
      <WorkspaceApp preferences={preferences} setPreferences={setPreferences} />
    </I18nProvider>
  );
}
