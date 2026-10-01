import { useI18n } from "../i18n";
import type { ImportPreview } from "../models";
import { isDesktop } from "../services/backend";

export type ImportDestinationFile = Pick<
  ImportPreview["targetFiles"][number],
  "path"
> &
  Partial<Pick<ImportPreview["targetFiles"][number], "source" | "profile">>;

export function ImportDestinations({
  files,
}: {
  files: ImportDestinationFile[];
}) {
  const { t } = useI18n();
  if (!files.length) return null;
  return (
    <div className="import-destinations">
      <strong>{t("適用先の設定ファイル")}</strong>
      {files.map((file) => {
        const path = isDesktop
          ? file.path
          : file.path.replace(/^サンプル/, t("サンプル"));
        return (
          <div
            className="import-destination"
            key={`${file.source}:${file.path}`}
          >
            {file.source && (
              <span>
                {file.source === "minecraft" ? "Minecraft" : "Lunar"}
                {file.source === "lunar" && file.profile
                  ? ` · ${file.profile}`
                  : ""}
              </span>
            )}
            <code>{path}</code>
          </div>
        );
      })}
    </div>
  );
}
