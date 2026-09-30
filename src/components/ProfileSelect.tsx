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
  return (
    <div className="profile-selects">
      <label>
        <span>Minecraft {target ? "の適用先" : "プロフィール"}</span>
        <select
          value={value.minecraftProfile || ""}
          onChange={(event) =>
            onChange({
              ...value,
              minecraftProfile: event.target.value || undefined,
            })
          }
        >
          <option value="">未選択</option>
          {minecraftProfiles(scan).map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
      </label>
      <label>
        <span>Lunar {target ? "の適用先" : "プロフィール"}</span>
        <select
          value={value.lunarProfile || ""}
          onChange={(event) =>
            onChange({
              ...value,
              lunarProfile: event.target.value || undefined,
            })
          }
        >
          <option value="">未選択</option>
          {scan.lunarProfiles.map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
      </label>
    </div>
  );
}
