import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync("package.json", "utf8")).version;
const tauriVersion = JSON.parse(
  readFileSync("src-tauri/tauri.conf.json", "utf8"),
).version;
const cargoVersion = readFileSync("src-tauri/Cargo.toml", "utf8").match(
  /^version = "([^"]+)"/m,
)?.[1];
const tagVersion =
  process.env.GITHUB_REF_TYPE === "tag"
    ? process.env.GITHUB_REF_NAME?.replace(/^v/, "")
    : packageVersion;
if (
  ![tauriVersion, cargoVersion, tagVersion].every(
    (version) => version === packageVersion,
  )
) {
  throw new Error(
    `Version mismatch: package=${packageVersion}, Tauri=${tauriVersion}, Cargo=${cargoVersion}, tag=${tagVersion}`,
  );
}
console.log(`Prism Relay ${packageVersion}`);
