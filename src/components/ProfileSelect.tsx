import { useI18n } from "../i18n";
import type { ScanReport, TargetProfiles } from "../models";
import { minecraftProfiles } from "../utils/profiles";
export function ProfileSelect({
  scan,
  value,
  onChange,
  target = false,
}: {
  scan: ScanReport;
  value: TargetProfiles;
  onChange: (value: TargetProfiles) => void;
  target?: boolean;
}) {
  const { t } = useI18n();
  return (
    <div className="profile-selects">
      <label>
        <span>Minecraft {target ? t("の適用先") : t("プロフィール")}</span>
        <select
          value={value.minecraftProfile || ""}
          onChange={(event) =>
            onChange({
              ...value,
              minecraftProfile: event.target.value || undefined,
            })
          }
        >
          <option value="">{t("未選択")}</option>
          {minecraftProfiles(scan).map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
      </label>
      <label>
        <span>Lunar {target ? t("の適用先") : t("プロフィール")}</span>
        <select
          value={value.lunarProfile || ""}
          onChange={(event) =>
            onChange({
              ...value,
              lunarProfile: event.target.value || undefined,
            })
          }
        >
          <option value="">{t("未選択")}</option>
          {scan.lunarProfiles.map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
      </label>
    </div>
  );
}
