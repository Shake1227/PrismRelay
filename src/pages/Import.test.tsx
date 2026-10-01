import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { act } from "react";
import type { Root } from "react-dom/client";
import { JSDOM } from "jsdom";
import type {
  BackupManifest,
  DecodedShare,
  ImportArguments,
  ImportPreview,
  ScanReport,
  Setting,
} from "../models";
import { I18nProvider } from "../i18n";
import { Import } from "./Import";

const bridge = vi.hoisted(() => ({
  decode: vi.fn(),
  preview: vi.fn(),
  apply: vi.fn(),
}));

vi.mock("../services/backend", () => ({ isDesktop: true, backend: bridge }));

const minecraftPath = "/Users/sample/.lunarclient/profiles/1.21/optionsLC.txt";
const lunarPath = "/Users/sample/.lunarclient/settings/game/default/mods.json";
const setting = (
  id: string,
  source: Setting["source"],
  value: number | boolean,
): Setting => ({
  id,
  source,
  value,
  profile: "profile-1",
  fileKind: source === "minecraft" ? "options" : "mods",
  pointer: source === "minecraft" ? "fov" : "/mods/fps/enabled",
  label: source === "minecraft" ? "FOV" : "Enabled",
  category: source === "minecraft" ? "video" : "mods",
  group: source === "minecraft" ? "Display" : "FPS",
});
const envelope = (platform: string, value: number): DecodedShare => ({
  formatVersion: 2,
  createdAt: "2026-10-01T00:00:00Z",
  applicationVersion: "1.0.0",
  metadata: { platform, minecraftVersion: "1.21.11" },
  settings: [
    setting(`${platform}-fov`, "minecraft", value),
    setting(`${platform}-fps`, "lunar", true),
  ],
});
const scan: ScanReport = {
  settings: [],
  minecraftDetected: true,
  lunarDetected: true,
  minecraftVersions: ["1.21", "1.8.9"],
  lunarProfiles: ["default", "my-game-profile"],
  warnings: [],
  platform: "macOS",
  runningProcesses: [],
  files: [
    {
      id: "vanilla",
      source: "minecraft",
      profile: "Vanilla",
      fileKind: "options",
      path: "/Users/sample/Library/Application Support/minecraft/options.txt",
    },
    {
      id: "lunar-mc",
      source: "minecraft",
      profile: "Lunar 1.21",
      fileKind: "options",
      path: minecraftPath,
    },
    {
      id: "lunar-default",
      source: "lunar",
      profile: "default",
      fileKind: "mods",
      path: lunarPath,
    },
    {
      id: "lunar-custom",
      source: "lunar",
      profile: "my-game-profile",
      fileKind: "mods",
      path: "/Users/sample/.lunarclient/settings/game/my-game-profile/mods.json",
    },
  ],
};

let dom: JSDOM;
let root: Root;
let container: HTMLElement;
let mount: (typeof import("react-dom/client"))["createRoot"];
let currentScan: ScanReport;
let revision: number;
let currentFov: number;
let reviewed: Map<string, { args: ImportArguments; preview: ImportPreview }>;
const onError = vi.fn();
const onNotice = vi.fn();
const navigate = vi.fn();
const onRefresh = vi.fn(async () => {
  currentScan = {
    ...currentScan,
    settings: [setting("local-fov", "minecraft", currentFov)],
  };
  render();
});

function render() {
  root.render(
    <I18nProvider language="en">
      <Import
        scan={currentScan}
        request={{}}
        backups={[]}
        onError={onError}
        onNotice={onNotice}
        navigate={navigate}
        onRefresh={onRefresh}
      />
    </I18nProvider>,
  );
}

function button(label: string): HTMLButtonElement {
  const result = [
    ...container.querySelectorAll<HTMLButtonElement>("button"),
  ].find((element) => element.textContent?.trim() === label);
  expect(result, label).toBeDefined();
  return result!;
}

async function click(label: string) {
  const element = button(label);
  expect(element.disabled).toBe(false);
  await act(async () => element.click());
}

async function enter(code: string) {
  const input = container.querySelector<HTMLTextAreaElement>("textarea")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLTextAreaElement.prototype,
      "value",
    )!.set!.call(input, code);
    input.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  });
  await act(async () => {
    vi.advanceTimersByTime(500);
  });
}

async function choose(minecraft: string, lunar: string) {
  await act(async () => {
    const selects = container.querySelectorAll<HTMLSelectElement>("select");
    selects[0].value = minecraft;
    selects[0].dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    selects[1].value = lunar;
    selects[1].dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
}

beforeAll(async () => {
  dom = new JSDOM(
    "<!doctype html><main class='main-content'><div id='test-root'></div></main>",
    { url: "http://localhost", pretendToBeVisual: true },
  );
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("navigator", dom.window.navigator);
  vi.stubGlobal("HTMLElement", dom.window.HTMLElement);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  Object.defineProperty(dom.window, "matchMedia", {
    value: () => ({ matches: true }),
  });
  Object.defineProperty(dom.window.HTMLElement.prototype, "scrollTo", {
    value: vi.fn(),
  });
  Object.defineProperty(dom.window.HTMLDialogElement.prototype, "showModal", {
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  Object.defineProperty(dom.window.HTMLDialogElement.prototype, "close", {
    value: function (this: HTMLDialogElement) {
      this.open = false;
    },
  });
  mount = (await import("react-dom/client")).createRoot;
});

beforeEach(async () => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  currentScan = structuredClone(scan);
  revision = 0;
  currentFov = 70;
  reviewed = new Map();
  bridge.decode.mockImplementation(async (code: string) =>
    envelope(
      code.startsWith("windows") ? "windows" : "macos",
      code.endsWith("second") ? 90 : 80,
    ),
  );
  bridge.preview.mockImplementation(async (args: ImportArguments) => {
    const decoded = (await bridge.decode(args.code)) as DecodedShare;
    const preview: ImportPreview = {
      fingerprint: (++revision).toString(16).padStart(64, "0"),
      warnings: [],
      selectedCount: args.selectedIds.length,
      targetFiles: currentScan.files
        .filter(
          (file) =>
            file.profile ===
            (file.source === "minecraft"
              ? args.target.minecraftProfile
              : args.target.lunarProfile),
        )
        .map(({ source, profile, fileKind, path }) => ({
          source,
          profile,
          fileKind,
          path,
        })),
      changes: decoded.settings
        .filter((item) => args.selectedIds.includes(item.id))
        .map((item) => {
          const current = item.source === "minecraft" ? currentFov : true;
          return {
            id: item.id,
            label: item.label,
            source: item.source,
            category: item.category,
            current,
            incoming: item.value,
            changed: current !== item.value,
          };
        }),
    };
    reviewed.set(preview.fingerprint, { args, preview });
    return preview;
  });
  bridge.apply.mockImplementation(
    async (
      args: ImportArguments,
      fingerprint: string,
    ): Promise<BackupManifest> => {
      const review = reviewed.get(fingerprint)!;
      expect(args).toEqual(review.args);
      const changes = review.preview.changes.filter((change) => change.changed);
      expect(changes).not.toHaveLength(0);
      currentFov = changes.find((change) => change.source === "minecraft")!
        .incoming as number;
      return {
        id: `backup-${revision}`,
        createdAt: "2026-10-01T00:00:00Z",
        platform: "macOS",
        minecraftVersions: ["1.21"],
        lunarProfiles: ["default"],
        reason: "import",
        status: "ready",
        files: [
          {
            originalPath: minecraftPath,
            relativeName: "0000.snapshot",
            checksum: "synthetic",
            size: 10,
          },
        ],
      };
    },
  );
  container = dom.window.document.getElementById("test-root")!;
  root = mount(container);
  await act(async () => render());
});

afterEach(async () => {
  await act(async () => root.unmount());
  vi.useRealTimers();
});
afterAll(() => {
  dom.window.close();
  vi.unstubAllGlobals();
});

describe("desktop import destination and repeated imports", () => {
  it("requires explicit targets and retains them for Windows-to-Mac then Mac-to-Mac imports", async () => {
    await enter("windows-first");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual(["", ""]);
    expect(button("Preview differences").disabled).toBe(true);
    await choose("Lunar 1.21", "default");
    await click("Preview differences");
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({
        code: "windows-first",
        selectedIds: ["windows-fov", "windows-fps"],
        target: { minecraftProfile: "Lunar 1.21", lunarProfile: "default" },
      }),
    );
    expect(container.textContent).toContain(minecraftPath);
    expect(container.textContent).toContain(lunarPath);
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(container.textContent).toContain("Settings applied.");
    expect(container.textContent).toContain("Minecraft · Lunar 1.21");
    expect(container.textContent).toContain(minecraftPath);
    expect(container.textContent).not.toContain(lunarPath);
    await click("Import another code");
    await enter("macos-second");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual(["Lunar 1.21", "default"]);
    await click("Preview differences");
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(bridge.apply).toHaveBeenCalledTimes(2);
    expect(bridge.apply.mock.calls[0][1]).not.toBe(
      bridge.apply.mock.calls[1][1],
    );
    expect(bridge.apply.mock.calls[1][0]).toEqual({
      code: "macos-second",
      selectedIds: ["macos-fov", "macos-fps"],
      target: { minecraftProfile: "Lunar 1.21", lunarProfile: "default" },
      request: {},
    });
    expect(currentFov).toBe(90);
    expect(onRefresh).toHaveBeenCalledTimes(2);
    expect(onError).not.toHaveBeenCalled();
  });

  it("requires a new preview after a failed apply and uses the fresh fingerprint", async () => {
    await enter("windows-first");
    await choose("Lunar 1.21", "default");
    await click("Preview differences");
    const stale = bridge.preview.mock.results[0].value;
    bridge.apply.mockRejectedValueOnce(
      new Error("Settings changed since the preview"),
    );
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: "Settings changed since the preview",
      }),
    );
    expect(container.querySelector(".diff-panel")).toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
    await click("Preview differences");
    const fresh = (await bridge.preview.mock.results[1].value) as ImportPreview;
    expect(fresh.fingerprint).not.toBe((await stale).fingerprint);
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(bridge.apply).toHaveBeenLastCalledWith(
      expect.objectContaining({
        target: { minecraftProfile: "Lunar 1.21", lunarProfile: "default" },
      }),
      fresh.fingerprint,
      false,
    );
    expect(container.textContent).toContain("Settings applied.");
  });

  it("shows an identical repeated code as already matching and prevents another write", async () => {
    currentScan = {
      ...scan,
      files: scan.files.filter(
        (file) => file.profile === "Lunar 1.21" || file.profile === "default",
      ),
      lunarProfiles: ["default"],
    };
    await act(async () => render());
    await enter("macos-first");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual(["Lunar 1.21", "default"]);
    await click("Preview differences");
    await click("Apply selected settings");
    await click("Back up and apply");
    await click("Import another code");
    await enter("macos-first");
    await click("Preview differences");
    expect(container.textContent).toContain("Settings already match");
    expect(button("Apply selected settings").disabled).toBe(true);
    expect(container.textContent).toContain(minecraftPath);
    expect(bridge.apply).toHaveBeenCalledTimes(1);
    expect(onError).not.toHaveBeenCalled();
  });
});
