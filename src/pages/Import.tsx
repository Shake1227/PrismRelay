import { useI18n } from "../i18n";
import { useEffect, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowDownToLine,
  ArrowRight,
  Check,
  FileUp,
  QrCode,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type {
  DecodedShare,
  ImportArguments,
  ImportPreview,
  TargetProfiles,
} from "../models";
import { backend, isDesktop } from "../services/backend";
import { demoScan, encodeDemo } from "../services/demo";
import { TreePicker } from "../components/TreePicker";
import { ProfileSelect } from "../components/ProfileSelect";
import { ImportDestinations } from "../components/ImportDestinations";
import type { ImportDestinationFile } from "../components/ImportDestinations";
import { QrReader } from "../components/QrReader";
import { Modal } from "../components/Modal";
import { formatDate, formatValue } from "../utils/format";
import { reconcileImportProfiles } from "../utils/profiles";
import { createRequestGuard } from "../utils/requestGuard";
import type { WorkspaceProps } from "./types";
export function Import({
  scan,
  request,
  onError,
  onNotice,
  onRefresh,
}: WorkspaceProps) {
  const { t, locale } = useI18n();
  const [code, setCode] = useState("");
  const [decoded, setDecoded] = useState<DecodedShare | null>(null);
  const [selected, setSelected] = useState(new Set<string>());
  const [profiles, setProfiles] = useState<TargetProfiles>(() =>
    reconcileImportProfiles(scan),
  );
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [changedOnly, setChangedOnly] = useState(true);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [readingQr, setReadingQr] = useState(false);
  const [done, setDone] = useState(false);
  const [appliedCount, setAppliedCount] = useState(0);
  const [appliedTargets, setAppliedTargets] = useState<ImportDestinationFile[]>(
    [],
  );
  const previewGuard = useRef(createRequestGuard());
  const fileInput = useRef<HTMLInputElement>(null);
  const previewPanel = useRef<HTMLElement>(null);
  useEffect(() => {
    if (!preview) return;
    const frame = window.requestAnimationFrame(() => {
      const panel = previewPanel.current;
      const scroller = panel?.closest<HTMLElement>(".main-content");
      if (!panel || !scroller) return;
      const header = scroller.querySelector<HTMLElement>(".page-header");
      const reduceMotion =
        document.documentElement.dataset.reduceMotion === "true" ||
        window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      const top =
        scroller.scrollTop +
        panel.getBoundingClientRect().top -
        scroller.getBoundingClientRect().top -
        scroller.clientTop -
        (header?.getBoundingClientRect().height ?? 0) -
        12;
      scroller.scrollTo({
        top: Math.max(0, top),
        behavior: reduceMotion ? "instant" : "smooth",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [preview]);
  useEffect(() => {
    previewGuard.current.invalidate();
    setDecoded(null);
    setPreview(null);
    setConfirm(false);
    setDone(false);
    setBusy(false);
    if (!code.trim()) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      setBusy(true);
      void backend
        .decode(code.trim())
        .then((value) => {
          if (!cancelled) {
            setDecoded(value);
            setSelected(new Set(value.settings.map((setting) => setting.id)));
          }
        })
        .catch((error) => {
          if (!cancelled) onError(error);
        })
        .finally(() => {
          if (!cancelled) setBusy(false);
        });
    }, 450);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [code, onError]);
  useEffect(() => {
    previewGuard.current.invalidate();
    setPreview(null);
    setConfirm(false);
    setProfiles((current) => reconcileImportProfiles(scan, current));
  }, [scan, request]);
  const args: ImportArguments = {
    code: code.trim(),
    selectedIds: [...selected],
    target: profiles,
    request,
  };
  const selectedSettings =
    decoded?.settings.filter((setting) => selected.has(setting.id)) || [];
  const missingTarget = selectedSettings.some((setting) =>
    setting.source === "minecraft"
      ? !profiles.minecraftProfile
      : !profiles.lunarProfile,
  );
  const makePreview = async () => {
    const isCurrent = previewGuard.current.begin();
    setBusy(true);
    try {
      const result = await backend.preview(args);
      if (isCurrent()) setPreview(result);
    } catch (error) {
      if (isCurrent()) onError(error);
    } finally {
      setBusy(false);
    }
  };
  const apply = async (allowRunning: boolean) => {
    if (!preview) return;
    setBusy(true);
    try {
      const backup = await backend.apply(
        args,
        preview.fingerprint,
        allowRunning,
      );
      setAppliedCount(
        preview.changes.filter((change) => change.changed).length,
      );
      setAppliedTargets(
        isDesktop
          ? backup.files.map((file) => ({
              ...preview.targetFiles.find(
                (target) => target.path === file.originalPath,
              ),
              path: file.originalPath,
            }))
          : preview.targetFiles.filter((file) =>
              preview.changes.some(
                (change) => change.changed && change.source === file.source,
              ),
            ),
      );
      previewGuard.current.invalidate();
      setPreview(null);
      setDone(true);
      setConfirm(false);
      onNotice(
        isDesktop
          ? t("選択した設定を適用しました。元の設定はバックアップ済みです。")
          : t("サンプルの適用を確認しました。実際の設定は変更されません。"),
      );
      await onRefresh();
    } catch (error) {
      previewGuard.current.invalidate();
      setPreview(null);
      onError(error);
      setConfirm(false);
    } finally {
      setBusy(false);
    }
  };
  const loadFile = async () => {
    if (!isDesktop) {
      fileInput.current?.click();
      return;
    }
    try {
      const content = await backend.loadCode();
      if (content) setCode(content);
    } catch (error) {
      onError(error);
    }
  };
  const sample = () =>
    setCode(
      encodeDemo(
        demoScan.settings
          .filter((setting) =>
            [
              "demo-mc-0",
              "demo-mc-1",
              "demo-lunar-0-enabled",
              "demo-mod-0",
            ].includes(setting.id),
          )
          .map((setting) =>
            setting.id === "demo-mc-0"
              ? {
                  ...setting,
                  value: 80,
                }
              : setting,
          ),
        {
          platform: "Sample",
          minecraftVersion: "1.21.4",
          lunarVersion: "default",
        },
      ).code,
    );
  if (done)
    return (
      <section className="glass-panel code-result">
        <div className="result-icon">
          <Check size={26} />
        </div>
        <h2>
          {isDesktop
            ? t("設定を適用しました。")
            : t("適用の流れを確認できました。")}
        </h2>
        <p>
          {isDesktop
            ? t(
                "{0} 項目を適用しました。ゲームを再起動すると設定が反映されます。",
                {
                  "0": appliedCount,
                },
              )
            : t("サンプルモードではファイルは変更されません。")}
        </p>
        <ImportDestinations files={appliedTargets} />
        <div className="help-card">
          <ShieldCheck size={20} />
          <div>
            <strong>
              {isDesktop
                ? t("元の設定も保存済みです")
                : t("実際の操作はデスクトップアプリで")}
            </strong>
            <p>{t("バックアップ画面から、以前の設定に戻せます。")}</p>
          </div>
        </div>
        <button
          className="button primary"
          disabled={busy}
          onClick={() => {
            setCode("");
            setDone(false);
          }}
        >
          {t("別のコードを読み込む")}
          <ArrowRight size={16} />
        </button>
      </section>
    );
  return (
    <div className="import-page">
      <section className="glass-panel paste-panel">
        <div className="panel-heading">
          <div>
            <h3>{t("共有コードを貼り付け")}</h3>
            <p>{t("内容を確認してから、必要な項目だけ適用できます。")}</p>
          </div>
          <div className="import-input-actions">
            <button
              className="button secondary compact"
              onClick={() => setReadingQr(true)}
            >
              <QrCode size={15} />
              {t("QRコードを読み取る")}
            </button>
            <button
              className="button secondary compact"
              onClick={() => void loadFile()}
            >
              <FileUp size={15} />
              {t("ファイルを開く")}
            </button>
          </div>
        </div>
        <textarea
          className="paste-code"
          spellCheck={false}
          placeholder={
            isDesktop
              ? t("PRS1: または PRS2: で始まる共有コードをここに…")
              : t("サンプルコードをここに…")
          }
          aria-label={t("読み込む共有コード")}
          value={code}
          onChange={(event) => setCode(event.target.value)}
        />
        <input
          type="file"
          accept=".prism,.txt"
          hidden
          ref={fileInput}
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) {
              if (file.size > 2_000_000) onError("File exceeds the 2 MB limit");
              else void file.text().then(setCode).catch(onError);
            }
          }}
        />
        <div className="paste-footer">
          <span>
            <ShieldCheck size={13} />
            {t("貼り付けだけでは設定は変わりません")}
          </span>
          {busy && !decoded ? (
            <span>{t("解析中…")}</span>
          ) : (
            !isDesktop && (
              <button className="text-button" onClick={sample}>
                <Sparkles size={13} />
                {t("サンプルを試す")}
              </button>
            )
          )}
        </div>
      </section>
      {decoded && (
        <>
          <div className="decoded-meta">
            <span className="status-pill">
              <Check size={12} />
              {t("コードを確認")}
            </span>
            <span>
              {decoded.settings.length} {t("項目")}
            </span>
            <span>{decoded.metadata.platform}</span>
            <span>
              Minecraft {decoded.metadata.minecraftVersion || t("不明")}
            </span>
            <span>{formatDate(decoded.createdAt, locale)}</span>
          </div>
          <section className="glass-panel selection-panel">
            <div className="panel-heading">
              <div>
                <h3>{t("適用する設定を選択")}</h3>
                <p>{t("受け取った設定から、さらに絞り込めます。")}</p>
              </div>
            </div>
            <ProfileSelect
              scan={scan}
              value={profiles}
              target
              onChange={(value) => {
                previewGuard.current.invalidate();
                setProfiles(value);
                setPreview(null);
              }}
            />
            <p className="import-profile-help">
              {t(
                "Lunar Clientの操作・画面設定は、Minecraftの適用先で使用中のLunarバージョンを選んでください。HUD・MOD設定はLunarのプロフィールを選びます。",
              )}
            </p>
            <TreePicker
              settings={decoded.settings}
              applicationIcons={scan.applicationIcons}
              selected={selected}
              onChange={(value) => {
                previewGuard.current.invalidate();
                setSelected(value);
                setPreview(null);
              }}
            />
            <div className="panel-action-row">
              <span>
                {missingTarget
                  ? t("適用先のプロフィールを選択してください")
                  : t("{0} 項目の適用先を確認します", {
                      "0": selected.size,
                    })}
              </span>
              <button
                className="button primary"
                disabled={!selected.size || missingTarget || busy}
                onClick={() => void makePreview()}
              >
                {t("差分をプレビュー")}
                <ArrowDown size={16} />
              </button>
            </div>
          </section>
          {preview && (
            <section className="glass-panel diff-panel" ref={previewPanel}>
              <div className="panel-heading">
                <div>
                  <h3>{t("変更内容を確認")}</h3>
                  <p>
                    {preview.changes.filter((change) => change.changed).length}{" "}
                    {t("項目が変更されます。")}
                  </p>
                </div>
                <label className="toggle-label">
                  <input
                    type="checkbox"
                    checked={changedOnly}
                    onChange={(event) => setChangedOnly(event.target.checked)}
                  />
                  {t("変更のみ")}
                </label>
              </div>
              <ImportDestinations files={preview.targetFiles} />
              {preview.warnings.map((warning, index) => (
                <div className="inline-warning" key={index}>
                  {t(warning)}
                </div>
              ))}
              <div className="diff-table">
                <div className="diff-table-header">
                  <span>{t("設定")}</span>
                  <span>{t("現在")}</span>
                  <span>{t("読み込む設定")}</span>
                </div>
                {preview.changes
                  .filter((change) => !changedOnly || change.changed)
                  .map((change) => (
                    <div
                      className={`diff-row ${change.changed ? "changed" : ""}`}
                      key={change.id}
                    >
                      <div>
                        <strong>
                          {change.label
                            .split(" · ")
                            .map((part) => t(part))
                            .join(" · ")}
                        </strong>
                        <small>
                          {change.source === "lunar" ? "Lunar" : "Minecraft"} ·{" "}
                          {t(change.category)}
                        </small>
                      </div>
                      <code title={formatValue(change.current)}>
                        {formatValue(change.current)}
                      </code>
                      <code title={formatValue(change.incoming)}>
                        {formatValue(change.incoming)}
                      </code>
                    </div>
                  ))}
                {preview.changes.filter(
                  (change) => !changedOnly || change.changed,
                ).length === 0 && (
                  <div className="empty-state small">
                    <Check size={23} />
                    <p>{t("現在の設定と一致しています")}</p>
                  </div>
                )}
              </div>
              <div className="panel-action-row">
                <span>
                  <ShieldCheck size={15} />
                  {t("適用前に自動でバックアップ")}
                </span>
                <button
                  className="button primary"
                  disabled={
                    busy || preview.changes.every((change) => !change.changed)
                  }
                  onClick={() => setConfirm(true)}
                >
                  <ArrowDownToLine size={16} />
                  {isDesktop
                    ? t("選択した設定を適用")
                    : t("サンプルの適用を確認")}
                </button>
              </div>
            </section>
          )}
        </>
      )}
      {readingQr && (
        <QrReader
          onCode={setCode}
          onError={onError}
          onClose={() => setReadingQr(false)}
        />
      )}
      {confirm && (
        <Modal
          title={
            scan.runningProcesses.length
              ? t("ゲームが起動しています")
              : t("設定を適用しますか？")
          }
          onClose={() => {
            if (!busy) setConfirm(false);
          }}
        >
          <div className="confirm-icon">
            <ShieldCheck size={27} />
          </div>
          <p className="modal-copy">
            {scan.runningProcesses.length
              ? t(
                  "{0} が起動しています。ゲーム側で設定が上書きされる場合があります。終了してからの適用をおすすめします。",
                  {
                    "0": scan.runningProcesses.join("、"),
                  },
                )
              : t(
                  "{0} 項目の変更を、選択したプロフィールに適用します。変更前の設定は自動でバックアップされます。",
                  {
                    "0":
                      preview?.changes.filter((change) => change.changed)
                        .length || 0,
                  },
                )}
          </p>
          <ImportDestinations files={preview?.targetFiles || []} />
          {!isDesktop && (
            <p className="inline-warning">
              {t("サンプルの確認です。実際の設定ファイルは変更しません。")}
            </p>
          )}
          <div className="modal-actions">
            <button
              className="button secondary"
              disabled={busy}
              onClick={() => setConfirm(false)}
            >
              {t("キャンセル")}
            </button>
            <button
              className="button primary"
              disabled={busy}
              onClick={() => void apply(scan.runningProcesses.length > 0)}
            >
              {busy
                ? t("適用中…")
                : scan.runningProcesses.length
                  ? t("起動中でも続行")
                  : t("バックアップして適用")}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
