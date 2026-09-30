import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sizes = [16, 24, 32, 48, 64, 128, 256];
const source = join(root, "src-tauri", "icons", "source-windows.svg");
const output = join(root, "src-tauri", "icons", "icon.ico");
const args = process.argv.slice(2);
let previewDirectory;
let check = false;

for (let index = 0; index < args.length; index += 1) {
  if (args[index] === "--check") {
    check = true;
  } else if (args[index] === "--preview-dir" && args[index + 1]) {
    previewDirectory = resolve(args[index + 1]);
    index += 1;
  } else {
    throw new Error(
      "Usage: node scripts/generate-windows-icon.mjs [--check] [--preview-dir directory]",
    );
  }
}

const temporaryDirectory = mkdtempSync(join(tmpdir(), "prism-windows-icon-"));

try {
  const render = spawnSync(
    process.execPath,
    [
      join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"),
      "icon",
      source,
      "--output",
      temporaryDirectory,
      ...sizes.flatMap((size) => ["--png", String(size)]),
    ],
    { cwd: root, stdio: "inherit" },
  );

  if (render.error) throw render.error;
  if (render.status !== 0)
    throw new Error("Windows icon SVG rendering failed.");

  const images = sizes.map((size) => {
    const path = join(temporaryDirectory, `${size}x${size}.png`);
    const image = readFileSync(path);
    const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

    if (
      !image.subarray(0, 8).equals(signature) ||
      image.toString("ascii", 12, 16) !== "IHDR" ||
      image.readUInt32BE(16) !== size ||
      image.readUInt32BE(20) !== size ||
      image[24] !== 8 ||
      image[25] !== 6
    ) {
      throw new Error(`Expected an ${size}px RGBA PNG from the SVG renderer.`);
    }

    return { size, path, image };
  });

  const directory = Buffer.alloc(6 + 16 * images.length);
  directory.writeUInt16LE(1, 2);
  directory.writeUInt16LE(images.length, 4);
  let offset = directory.length;

  images.forEach(({ size, image }, index) => {
    const entry = 6 + 16 * index;
    directory[entry] = size === 256 ? 0 : size;
    directory[entry + 1] = size === 256 ? 0 : size;
    directory.writeUInt16LE(1, entry + 4);
    directory.writeUInt16LE(32, entry + 6);
    directory.writeUInt32LE(image.length, entry + 8);
    directory.writeUInt32LE(offset, entry + 12);
    offset += image.length;
  });

  const icon = Buffer.concat([directory, ...images.map(({ image }) => image)]);

  if (check) {
    if (!existsSync(output) || !readFileSync(output).equals(icon)) {
      throw new Error("The Windows ICO does not match source-windows.svg.");
    }
    process.stdout.write("Windows ICO matches its SVG source.\n");
  } else {
    writeFileSync(output, icon);
    process.stdout.write("Generated src-tauri/icons/icon.ico with 7 sizes.\n");
  }

  if (previewDirectory) {
    mkdirSync(previewDirectory, { recursive: true });
    images.forEach(({ size, path }) =>
      copyFileSync(path, join(previewDirectory, `${size}x${size}.png`)),
    );
    process.stdout.write(`PNG previews: ${previewDirectory}\n`);
  }
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
