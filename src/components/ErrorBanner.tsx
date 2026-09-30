import { AlertCircle, X } from "lucide-react";

export function ErrorBanner({
  error,
  onClose,
}: {
  error: string | null;
  onClose: () => void;
}) {
  if (!error) return null;
  return (
    <div className="error-banner" role="alert">
      <AlertCircle size={18} />
      <div>
        <strong>操作を完了できませんでした</strong>
        <p>
          設定は適用されていません。内容や対象フォルダを確認して、もう一度お試しください。
        </p>
        <details>
          <summary>詳細を表示</summary>
          <pre>{error}</pre>
        </details>
      </div>
      <button
        className="icon-button"
        onClick={onClose}
        aria-label="エラーを閉じる"
      >
        <X size={16} />
      </button>
    </div>
  );
}
