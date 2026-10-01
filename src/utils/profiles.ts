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

export function defaultMinecraftProfile(scan: ScanReport): string | undefined {
  const available = minecraftProfiles(scan);
  return available.length === 1 ? available[0] : undefined;
}

export function defaultLunarProfile(
  scan: ScanReport,
  selected?: string,
): string | undefined {
  if (selected && scan.lunarProfiles.includes(selected)) return selected;
  if (
    scan.activeLunarProfile &&
    scan.lunarProfiles.includes(scan.activeLunarProfile)
  )
    return scan.activeLunarProfile;
  return scan.lunarProfiles.length === 1 ? scan.lunarProfiles[0] : undefined;
}

export function reconcileProfiles(
  scan: ScanReport,
  current: TargetProfiles,
): TargetProfiles {
  const minecraftProfile = defaultMinecraftProfile(scan);
  const lunarProfile = defaultLunarProfile(scan, current.lunarProfile);
  return minecraftProfile === current.minecraftProfile &&
    lunarProfile === current.lunarProfile
    ? current
    : { minecraftProfile, lunarProfile };
}

export function reconcileImportProfiles(
  scan: ScanReport,
  current: TargetProfiles = {},
): TargetProfiles {
  return reconcileProfiles(scan, current);
}
