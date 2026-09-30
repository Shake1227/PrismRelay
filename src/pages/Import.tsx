import { useEffect, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowRight,
  Check,
  FileUp,
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
import { Modal } from "../components/Modal";
import { formatDate, formatValue } from "../utils/format";
import { minecraftProfiles, reconcileProfiles } from "../utils/profiles";
import { createRequestGuard } from "../utils/requestGuard";
import type { WorkspaceProps } from "./types";

export function Import({
  scan,
  request,
  onError,
  onNotice,
  onRefresh,
}: WorkspaceProps) {
  const [code, setCode] = useState("");
  const [decoded, setDecoded] = useState<DecodedShare | null>(null);
  const [selected, setSelected] = useState(new Set<string>());
  const [profiles, setProfiles] = useState<TargetProfiles>({
    minecraftProfile: minecraftProfiles(scan)[0],
    lunarProfile: scan.lunarProfiles[0],
  });
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [changedOnly, setChangedOnly] = useState(true);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [done, setDone] = useState(false);
  const [appliedCount, setAppliedCount] = useState(0);
  const previewGuard = useRef(createRequestGuard());
  const fileInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    previewGuard.current.invalidate();
    setDecoded(null);
    setPreview(null);
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
    setProfiles((current) => reconcileProfiles(scan, current));
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
      await backend.apply(args, preview?.fingerprint || "", allowRunning);
      setAppliedCount(
        preview.changes.filter((change) => change.changed).length,
      );
      setDone(true);
      setConfirm(false);
      onNotice(
        isDesktop
          ? "選択した設定を適用しました。元の設定はバックアップ済みです。"
          : "サンプルの適用を確認しました。実際の設定は変更されません。",
      );
      await onRefresh();
    } catch (error) {
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
            setting.id === "demo-mc-0" ? { ...setting, value: 80 } : setting,
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
        <span className="eyebrow">SETUP RESTORED</span>
        <h2>
          {isDesktop
            ? "いつもの環境が、ここに。"
            : "適用の流れを確認できました。"}
        </h2>
        <p>
          {isDesktop
            ? `${appliedCount} 項目を適用しました。ゲームを再起動すると設定が反映されます。`
            : "サンプルモードではファイルは変更されません。"}
        </p>
        <div className="help-card">
          <ShieldCheck size={20} />
          <div>
            <strong>
              {isDesktop
                ? "元の設定も保存済みです"
                : "実際の操作はデスクトップアプリで"}
            </strong>
            <p>バックアップ画面から、以前の設定に戻せます。</p>
          </div>
        </div>
        <button
          className="button primary"
          onClick={() => {
            setCode("");
            setDone(false);
          }}
        >
          別のコードを読み込む
          <ArrowRight size={16} />
        </button>
      </section>
    );
  return (
    <div className="import-page">
      <section className="glass-panel paste-panel">
        <div className="panel-heading">
          <div>
            <h3>共有コードを貼り付け</h3>
            <p>内容を確認してから、必要な項目だけ適用できます。</p>
          </div>
          <button
            className="button secondary compact"
            onClick={() => void loadFile()}
          >
            <FileUp size={15} />
            ファイルを開く
          </button>
        </div>
        <textarea
          className="paste-code"
          spellCheck={false}
          placeholder={
            isDesktop
              ? "PRS1: で始まる共有コードをここに…"
              : "サンプルコードをここに…"
          }
          aria-label="読み込む共有コード"
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
            貼り付けだけでは設定は変わりません
          </span>
          {busy && !decoded ? (
            <span>解析中…</span>
          ) : (
            !isDesktop && (
              <button className="text-button" onClick={sample}>
                <Sparkles size={13} />
                サンプルを試す
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
              コードを確認
            </span>
            <span>{decoded.settings.length} 項目</span>
            <span>{decoded.metadata.platform}</span>
            <span>Minecraft {decoded.metadata.minecraftVersion || "不明"}</span>
            <span>{formatDate(decoded.createdAt)}</span>
          </div>
          <section className="glass-panel selection-panel">
            <div className="panel-heading">
              <div>
                <h3>適用する設定を選択</h3>
                <p>受け取った設定から、さらに絞り込めます。</p>
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
            <TreePicker
              settings={decoded.settings}
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
                  ? "適用先のプロフィールを選択してください"
                  : `${selected.size} 項目の適用先を確認します`}
              </span>
              <button
                className="button primary"
                disabled={!selected.size || missingTarget || busy}
                onClick={() => void makePreview()}
              >
                差分をプレビュー
                <ArrowRight size={16} />
              </button>
            </div>
          </section>
          {preview && (
            <section className="glass-panel diff-panel">
              <div className="panel-heading">
                <div>
                  <h3>変更内容を確認</h3>
                  <p>
                    {preview.changes.filter((change) => change.changed).length}{" "}
                    項目が変更されます。
                  </p>
                </div>
                <label className="toggle-label">
                  <input
                    type="checkbox"
                    checked={changedOnly}
                    onChange={(event) => setChangedOnly(event.target.checked)}
                  />
                  変更のみ
                </label>
              </div>
              {preview.warnings.map((warning, index) => (
                <div className="inline-warning" key={index}>
                  {warning}
                </div>
              ))}
              <div className="diff-table">
                <div className="diff-table-header">
                  <span>設定</span>
                  <span>現在</span>
                  <span>読み込む設定</span>
                </div>
                {preview.changes
                  .filter((change) => !changedOnly || change.changed)
                  .map((change) => (
                    <div
                      className={`diff-row ${change.changed ? "changed" : ""}`}
                      key={change.id}
                    >
                      <div>
                        <strong>{change.label}</strong>
                        <small>
                          {change.source === "lunar" ? "Lunar" : "Minecraft"} ·{" "}
                          {change.category}
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
                    <p>現在の設定と一致しています</p>
                  </div>
                )}
              </div>
              <div className="panel-action-row">
                <span>
                  <ShieldCheck size={15} />
                  適用前に自動でバックアップ
                </span>
                <button
                  className="button primary"
                  disabled={
                    busy || preview.changes.every((change) => !change.changed)
                  }
                  onClick={() => setConfirm(true)}
                >
                  <ArrowDownToLine size={16} />
                  {isDesktop ? "選択した設定を適用" : "サンプルの適用を確認"}
                </button>
              </div>
            </section>
          )}
        </>
      )}
      {confirm && (
        <Modal
          title={
            scan.runningProcesses.length
              ? "ゲームが起動しています"
              : "設定を適用しますか？"
          }
          onClose={() => setConfirm(false)}
        >
          <div className="confirm-icon">
            <ShieldCheck size={27} />
          </div>
          <p className="modal-copy">
            {scan.runningProcesses.length
              ? `${scan.runningProcesses.join("、")} が起動しています。ゲーム側で設定が上書きされる場合があります。終了してからの適用をおすすめします。`
              : `${preview?.changes.filter((change) => change.changed).length || 0} 項目の変更を、選択したプロフィールに適用します。変更前の設定は自動でバックアップされます。`}
          </p>
          {!isDesktop && (
            <p className="inline-warning">
              サンプルの確認です。実際の設定ファイルは変更しません。
            </p>
          )}
          <div className="modal-actions">
            <button
              className="button secondary"
              onClick={() => setConfirm(false)}
            >
              キャンセル
            </button>
            <button
              className="button primary"
              disabled={busy}
              onClick={() => void apply(scan.runningProcesses.length > 0)}
            >
              {busy
                ? "適用中…"
                : scan.runningProcesses.length
                  ? "起動中でも続行"
                  : "バックアップして適用"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
