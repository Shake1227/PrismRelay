import { useI18n } from "../i18n";
import { useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  FolderOpen,
  HardDrive,
  Plus,
  RotateCcw,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import type { BackupManifest } from "../models";
import { backend, isDesktop } from "../services/backend";
import { Modal } from "../components/Modal";
import { formatBytes, formatDate } from "../utils/format";
import type { WorkspaceProps } from "./types";
export function Backups({
  backups,
  scan,
  request,
  onError,
  onNotice,
  onRefresh,
}: WorkspaceProps) {
  const { t, locale } = useI18n();
  const [expanded, setExpanded] = useState<string | null>(null);
  const [action, setAction] = useState<{
    kind: "restore" | "delete";
    backup: BackupManifest;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const create = async () => {
    setBusy(true);
    try {
      await backend.createBackup(request);
      await onRefresh();
      onNotice(
        isDesktop
          ? t("バックアップを保存しました")
          : t("サンプルのバックアップを確認しました"),
      );
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };
  const perform = async () => {
    if (!action) return;
    setBusy(true);
    try {
      if (action.kind === "restore")
        await backend.restore(
          action.backup.id,
          scan.runningProcesses.length > 0,
        );
      else await backend.deleteBackup(action.backup.id);
      onNotice(
        isDesktop
          ? action.kind === "restore"
            ? t("設定を復元しました")
            : t("バックアップを削除しました")
          : t("サンプルの操作を確認しました。ファイルは変更されません。"),
      );
      setAction(null);
      await onRefresh();
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="backups-page">
      <div className="backup-intro">
        <div>
          <h2>{t("設定のバックアップ")}</h2>
          <p>{t("保存した設定を確認・復元できます。")}</p>
        </div>
        <div className="backup-toolbar">
          <button
            className="button secondary"
            onClick={() =>
              void backend
                .openBackupFolder()
                .then(
                  () =>
                    !isDesktop &&
                    onNotice(t("デスクトップアプリで保存先を開けます")),
                )
                .catch(onError)
            }
          >
            <FolderOpen size={16} />
            {t("保存先を開く")}
          </button>
          <button
            className="button primary"
            onClick={() => void create()}
            disabled={busy || !scan.files.length}
          >
            <Plus size={17} />
            {t("バックアップ")}
          </button>
        </div>
      </div>
      <div className="backup-summary">
        <ShieldCheck size={20} />
        <span>{t("インポート前のバックアップは自動で作成されます。")}</span>
        <b>
          {backups.length} {t("保存済み")}
        </b>
      </div>
      <section className="glass-panel backup-list">
        <div className="backup-list-label">
          <span>{t("バックアップ")}</span>
          <span>{t("ファイル / サイズ")}</span>
          <span>{t("操作")}</span>
        </div>
        {backups.map((backup) => (
          <div className="backup-entry" key={backup.id}>
            <div className="backup-row">
              <button
                className="backup-title"
                onClick={() =>
                  setExpanded(expanded === backup.id ? null : backup.id)
                }
                aria-expanded={expanded === backup.id}
              >
                <div className="backup-file-icon">
                  <HardDrive size={20} />
                </div>
                <div>
                  <strong>{formatDate(backup.createdAt, locale)}</strong>
                  <p>
                    {backup.reason === "manual"
                      ? t("手動バックアップ")
                      : backup.reason === "import"
                        ? t("インポート前")
                        : backup.reason === "restore"
                          ? t("復元前")
                          : backup.reason}{" "}
                    · {backup.platform}
                  </p>
                </div>
                {expanded === backup.id ? (
                  <ChevronDown size={14} />
                ) : (
                  <ChevronRight size={14} />
                )}
              </button>
              <div className="backup-size">
                <strong>
                  {backup.files.length} {t("ファイル")}
                </strong>
                <p>
                  {formatBytes(
                    backup.files.reduce((total, file) => total + file.size, 0),
                  )}
                </p>
              </div>
              <div className="backup-actions">
                <button
                  className="button secondary compact"
                  onClick={() =>
                    setAction({
                      kind: "restore",
                      backup,
                    })
                  }
                >
                  <RotateCcw size={14} />
                  {t("復元")}
                </button>
                <button
                  className="icon-button danger"
                  aria-label={t("{0} のバックアップを削除", {
                    "0": formatDate(backup.createdAt, locale),
                  })}
                  onClick={() =>
                    setAction({
                      kind: "delete",
                      backup,
                    })
                  }
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
            {expanded === backup.id && (
              <div className="backup-details">
                <div className="detail-tags">
                  <span>
                    Minecraft {backup.minecraftVersions.join(", ") || "—"}
                  </span>
                  <span>Lunar {backup.lunarProfiles.join(", ") || "—"}</span>
                  <span>{backup.status}</span>
                </div>
                {backup.files.map((file) => (
                  <div className="backup-detail-file" key={file.relativeName}>
                    <strong>{file.relativeName}</strong>
                    <span>{formatBytes(file.size)}</span>
                    <p>{file.originalPath}</p>
                    <code>SHA-256 · {file.checksum}</code>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
        {!backups.length && (
          <div className="empty-state">
            <HardDrive size={32} />
            <h3>{t("バックアップはまだありません")}</h3>
            <p>{t("現在の設定を保存すると、いつでも元に戻せます。")}</p>
          </div>
        )}
      </section>
      {action && (
        <Modal
          title={
            action.kind === "delete"
              ? t("バックアップを削除しますか？")
              : t("このバックアップから復元しますか？")
          }
          onClose={() => setAction(null)}
        >
          <p className="modal-copy">
            {action.kind === "delete"
              ? t(
                  "{0} のバックアップを削除します。この操作は取り消せません。",
                  {
                    "0": formatDate(action.backup.createdAt, locale),
                  },
                )
              : t(
                  "{0} の設定に戻します。現在の設定も、復元前にバックアップされます。",
                  {
                    "0": formatDate(action.backup.createdAt, locale),
                  },
                )}
          </p>
          {action.kind === "restore" && scan.runningProcesses.length > 0 && (
            <p className="inline-warning">
              {scan.runningProcesses.join("、")}{" "}
              {t("が起動中です。終了してからの復元をおすすめします。")}
            </p>
          )}
          {!isDesktop && (
            <p className="inline-warning">
              {t("サンプルの確認です。実際のファイルは変更しません。")}
            </p>
          )}
          <div className="modal-actions">
            <button
              className="button secondary"
              onClick={() => setAction(null)}
            >
              {t("キャンセル")}
            </button>
            <button
              className={`button ${action.kind === "delete" ? "destructive" : "primary"}`}
              disabled={busy}
              onClick={() => void perform()}
            >
              {busy
                ? t("処理中…")
                : action.kind === "delete"
                  ? t("削除")
                  : scan.runningProcesses.length
                    ? t("起動中でも復元")
                    : t("バックアップして復元")}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
