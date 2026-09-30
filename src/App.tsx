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
import type { AppInfo, BackupManifest, Page, ScanReport } from "./models";
import { backend, isDesktop } from "./services/backend";
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
    title: "Overview",
    icon: HomeIcon,
    subtitle: "あなたのプレイ環境を、ひと目で。",
  },
  {
    id: "export",
    label: "エクスポート",
    title: "Create share code",
    icon: ArrowUpRight,
    subtitle: "あなたのセットアップを、ひとつのコードに。",
  },
  {
    id: "import",
    label: "インポート",
    title: "Import settings",
    icon: ArrowDownToLine,
    subtitle: "次のデバイスにも、いつものプレイ環境を。",
  },
  {
    id: "backups",
    label: "バックアップ",
    title: "Backups",
    icon: HardDrive,
    subtitle: "大切な設定の、セーフティネット。",
  },
  {
    id: "settings",
    label: "設定",
    title: "Preferences",
    icon: Settings2,
    subtitle: "あなたらしく、心地よく。",
  },
  {
    id: "about",
    label: "アプリについて",
    title: "About Prism Relay",
    icon: Info,
    subtitle: "セットアップをつなぐ、小さなプリズム。",
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

export default function App() {
  const mainRef = useRef<HTMLElement>(null);
  const [page, setPage] = useState<Page>("home");
  const [scan, setScan] = useState<ScanReport>(emptyScan);
  const [backups, setBackups] = useState<BackupManifest[]>([]);
  const [info, setInfo] = useState<AppInfo>({
    name: "Prism Relay",
    version: "0.1.0",
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
  const { preferences, setPreferences } = usePreferences();
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
    mainRef.current?.scrollTo({ top: 0 });
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
            <small>YOUR SETUP, EVERYWHERE</small>
          </div>
        </div>
        <div className="workspace-label">WORKSPACE</div>
        <nav aria-label="メインナビゲーション">
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`nav-item ${page === id ? "active" : ""}`}
              onClick={() => {
                setPage(id);
                setError(null);
              }}
              aria-current={page === id ? "page" : undefined}
              aria-label={label}
            >
              <Icon size={18} />
              <span>{label}</span>
              {page === id && <ChevronRight size={13} />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-status">
            <span className="status-dot" />
            <div>
              <strong>
                {isDesktop ? "ローカルで動作中" : "サンプルモード"}
              </strong>
              <small>
                {isDesktop
                  ? "あなたのデータは、あなたのもの。"
                  : "実際の設定は変更しません"}
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
              Workspace <ChevronRight size={11} /> {active.label}
            </div>
            <h1>{active.title}</h1>
            <p>{active.subtitle}</p>
          </div>
          <div className="header-actions">
            <span className="offline-badge">
              <span />
              {isDesktop ? "LOCAL FIRST" : "PREVIEW"}
            </span>
            <button
              className="icon-button refresh-button"
              onClick={() => void refresh()}
              disabled={loading}
              aria-label="設定を再検出"
            >
              <RefreshCw size={17} className={loading ? "spinning" : ""} />
            </button>
          </div>
        </header>
        {!isDesktop && (
          <div className="demo-banner">
            <FlaskConical size={15} />
            <span>
              サンプルモード —
              デモ用の設定を表示しています。実際のファイルにはアクセスしません。
            </span>
            <span className="demo-label">DEMO</span>
          </div>
        )}
        <ErrorBanner error={error} onClose={() => setError(null)} />
        {scan.warnings.length > 0 && (
          <details className="scan-warnings">
            <summary>{scan.warnings.length} 件の検出メッセージ</summary>
            {scan.warnings.map((warning, index) => (
              <p key={index}>{warning}</p>
            ))}
          </details>
        )}
        {loading && !scan.settings.length ? (
          <div className="loading-screen">
            <PrismMark className="loading-prism" />
            <h2>プレイ環境を探しています…</h2>
            <p>設定ファイルを読み取り専用で確認しています。</p>
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
          <span>Made for your next session.</span>
          <span>
            <ShieldCheck size={11} />
            プライベートに、安全に。
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
            aria-label="通知を閉じる"
          >
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
