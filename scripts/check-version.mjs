import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync("package.json", "utf8")).version;
const packageLock = JSON.parse(readFileSync("package-lock.json", "utf8"));
const tauriVersion = JSON.parse(
  readFileSync("src-tauri/tauri.conf.json", "utf8"),
).version;
const cargoVersion = readFileSync("src-tauri/Cargo.toml", "utf8").match(
  /^version = "([^"]+)"/m,
)?.[1];
const cargoLockVersion = readFileSync("src-tauri/Cargo.lock", "utf8").match(
  /\[\[package\]\]\r?\nname = "prism-relay"\r?\nversion = "([^"]+)"/,
)?.[1];
const releaseVersion = readFileSync("docs/release-notes.md", "utf8")
  .split("\n", 1)[0]
  .trimEnd()
  .replace(/^# Prism Relay /, "");
const tagVersion =
  process.env.GITHUB_REF_TYPE === "tag"
    ? process.env.GITHUB_REF_NAME?.replace(/^v/, "")
    : packageVersion;
if (
  ![
    tauriVersion,
    cargoVersion,
    cargoLockVersion,
    packageLock.version,
    packageLock.packages[""].version,
    releaseVersion,
    tagVersion,
  ].every(
    (version) => version === packageVersion,
  )
) {
  throw new Error(
    `Version mismatch: package=${packageVersion}, Tauri=${tauriVersion}, Cargo=${cargoVersion}, Cargo.lock=${cargoLockVersion}, package-lock=${packageLock.version}/${packageLock.packages[""].version}, release=${releaseVersion}, tag=${tagVersion}`,
  );
}
console.log(`Prism Relay ${packageVersion}`);
