import type { ScanReport, TargetProfiles } from "../models";

export function minecraftProfiles(scan: ScanReport): string[] {
  return [
    ...new Set([
      ...scan.files
        .filter((file) => file.source === "minecraft")
        .map((file) => file.profile),
      ...scan.settings
        .filter((setting) => setting.source === "minecraft")
        .map((setting) => setting.profile),
    ]),
  ];
}

export function minecraftVersion(
  scan: ScanReport,
  profile: string | undefined,
): string | undefined {
  if (!profile) return undefined;
  return scan.minecraftVersions.find(
    (version) =>
      /^\d+(?:\.\d+)*(?:[-\w.]*)?$/.test(version) &&
      (profile === version || profile.endsWith(` ${version}`)),
  );
}

export function reconcileProfiles(
  scan: ScanReport,
  current: TargetProfiles,
): TargetProfiles {
  const availableMinecraft = minecraftProfiles(scan);
  const minecraftProfile = availableMinecraft.includes(
    current.minecraftProfile || "",
  )
    ? current.minecraftProfile
    : availableMinecraft[0];
  const lunarProfile = scan.lunarProfiles.includes(current.lunarProfile || "")
    ? current.lunarProfile
    : scan.lunarProfiles[0];
  return minecraftProfile === current.minecraftProfile &&
    lunarProfile === current.lunarProfile
    ? current
    : { minecraftProfile, lunarProfile };
}

export function reconcileImportProfiles(
  scan: ScanReport,
  current: TargetProfiles = {},
): TargetProfiles {
  const choose = (available: string[], selected?: string) =>
    selected && available.includes(selected)
      ? selected
      : available.length === 1
        ? available[0]
        : undefined;
  const minecraftProfile = choose(
    minecraftProfiles(scan),
    current.minecraftProfile,
  );
  const lunarProfile = choose(scan.lunarProfiles, current.lunarProfile);
  return minecraftProfile === current.minecraftProfile &&
    lunarProfile === current.lunarProfile
    ? current
    : { minecraftProfile, lunarProfile };
}
