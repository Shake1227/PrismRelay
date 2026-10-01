import { useI18n } from "../i18n";
import { AlertCircle, X } from "lucide-react";
export function ErrorBanner({
  error,
  onClose,
}: {
  error: string | null;
  onClose: () => void;
}) {
  const { t } = useI18n();
  if (!error) return null;
  return (
    <div className="error-banner" role="alert">
      <AlertCircle size={18} />
      <div>
        <strong>{t("操作を完了できませんでした")}</strong>
        <p>
          {t(
            "設定は適用されていません。内容や対象フォルダを確認して、もう一度お試しください。",
          )}
        </p>
        <details>
          <summary>{t("詳細を表示")}</summary>
          <pre>{t(error)}</pre>
        </details>
      </div>
      <button
        className="icon-button"
        onClick={onClose}
        aria-label={t("エラーを閉じる")}
      >
        <X size={16} />
      </button>
    </div>
  );
}
