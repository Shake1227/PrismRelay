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
import { Export } from "./Export";

const bridge = vi.hoisted(() => ({
  decode: vi.fn(),
  preview: vi.fn(),
  apply: vi.fn(),
  encode: vi.fn(),
  scan: vi.fn(),
}));

vi.mock("../services/backend", () => ({ isDesktop: true, backend: bridge }));

const minecraftPath =
  "/Users/sample/Library/Application Support/minecraft/optionsLC.txt";
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
      profile: "Minecraft",
      fileKind: "options",
      path: "/Users/sample/Library/Application Support/minecraft/options.txt",
    },
    {
      id: "lunar-mc",
      source: "minecraft",
      profile: "Minecraft",
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
    settings: [
      {
        ...setting("local-fov", "minecraft", currentFov),
        profile: "Minecraft",
      },
    ],
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

async function choose(lunar: string) {
  await act(async () => {
    const selects = container.querySelectorAll<HTMLSelectElement>("select");
    selects[0].value = lunar;
    selects[0].dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
}

async function changeSize(axis: "width" | "height", value: string) {
  const input = container.querySelectorAll<HTMLInputElement>(
    ".hud-size-fields input",
  )[axis === "width" ? 0 : 1];
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, value);
    input.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  });
}

async function prepareHud(
  status: "adjusted" | "missing-source" | "unavailable" = "adjusted",
) {
  const decoded = envelope("windows", 80);
  decoded.metadata.hudViewport = { width: 480, height: 270 };
  decoded.settings.push(
    {
      ...setting("hud-x", "lunar", 200),
      pointer: "/FPS/x",
      label: "X",
      category: "HUD",
      group: "FPS",
    },
    {
      ...setting("hud-y", "lunar", 81.89999999999999),
      pointer: "/FPS/y",
      label: "Y",
      category: "HUD",
      group: "FPS",
    },
  );
  if (status === "missing-source") delete decoded.metadata.hudViewport;
  bridge.decode.mockResolvedValue(decoded);
  const ordinary = bridge.preview.getMockImplementation()!;
  bridge.preview.mockImplementation(async (args: ImportArguments) => {
    const result = (await ordinary(args)) as ImportPreview;
    const preserve = args.hudLayout?.mode === "preserve";
    const effectiveStatus = preserve
      ? "unchanged"
      : args.hudLayout?.mode === "manual"
        ? "adjusted"
        : status;
    const adjusted = effectiveStatus === "adjusted";
    result.hudLayout = {
      status: effectiveStatus,
      ...(adjusted
        ? {
            windowSize: args.hudLayout?.windowSize || {
              width: 1920,
              height: 1080,
            },
          }
        : {}),
      adjustedCount: adjusted ? 2 : 0,
    };
    result.changes = result.changes.map((change) =>
      change.id.startsWith("hud-") && adjusted
        ? { ...change, incoming: Number(change.incoming) * 2, changed: true }
        : change,
    );
    return result;
  });
  await enter("windows-hud");
  await choose("default");
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
  it("previews verified text color with opacity while keeping size values and wire values exact", async () => {
    const shared = envelope("windows", 80);
    const packed = 0x806633ff | 0;
    shared.settings.push(
      {
        ...setting("text-color", "lunar", packed),
        pointer: "/KEYSTROKES/options/textColor/value",
        category: "HUD",
        group: "Keystrokes",
        label: "Text Color",
      },
      {
        ...setting("size-value", "lunar", 123),
        pointer: "/KEYSTROKES/options/width",
        category: "HUD",
        group: "Keystrokes",
        label: "Width",
      },
    );
    bridge.decode.mockResolvedValue(shared);
    const ordinary = bridge.preview.getMockImplementation()!;
    bridge.preview.mockImplementation(async (args: ImportArguments) => {
      const preview = (await ordinary(args)) as ImportPreview;
      preview.changes = preview.changes.map((change) =>
        change.id === "text-color" ? { ...change, current: -1 } : change,
      );
      return preview;
    });
    await enter("windows-colors");
    await choose("default");
    await click("Preview differences");
    const rows = [...container.querySelectorAll(".diff-row")];
    const colors = rows
      .find(
        (row) =>
          row.querySelector("strong")?.textContent ===
          "Keystrokes · Text color",
      )!
      .querySelectorAll("code");
    expect(colors[0].textContent).toContain("#FFFFFF");
    expect(colors[1].textContent).toContain("#6633FF");
    expect(colors[1].textContent).toContain("50.2%");
    expect(colors[1].title).toBe(String(packed));
    expect(
      colors[1].querySelector(".setting-color")?.getAttribute("aria-label"),
    ).toBe("Color #6633FF, opacity 50.2%");
    const size = rows.find((row) =>
      row.querySelector("strong")?.textContent?.endsWith("Width"),
    )!;
    expect(size.querySelectorAll("code")[1].textContent).toBe("123");
    expect(size.querySelector(".setting-color")).toBeNull();
    const preview = (await bridge.preview.mock.results[0]
      .value) as ImportPreview;
    expect(
      preview.changes.find((change) => change.id === "text-color")!.incoming,
    ).toBe(packed);
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({
        selectedIds: shared.settings.map((item) => item.id),
      }),
    );
  });
  it("exports a freshly scanned preset and ignores an older preset scan arriving later", async () => {
    const request = { lunarRoot: "/fixture/lunar" };
    const game = {
      ...setting("local-game", "minecraft", 70),
      profile: "Minecraft",
    };
    const active = {
      ...setting("active-option", "lunar", true),
      profile: "default",
    };
    const common = {
      ...scan,
      activeLunarProfile: "default",
      lunarProfiles: ["default", "my-game-profile", "third"],
      settings: [game, active],
    };
    const pending = new Map<string, (value: ScanReport) => void>();
    bridge.scan.mockImplementation(
      (_request: unknown, profile: string) =>
        new Promise<ScanReport>((resolve) => pending.set(profile, resolve)),
    );
    bridge.encode.mockResolvedValue({
      code: "PRS2:fixture",
      compressedBytes: 100,
      uncompressedBytes: 200,
      settingCount: 2,
    });
    await act(async () =>
      root.render(
        <I18nProvider language="en">
          <Export
            scan={common}
            request={request}
            backups={[]}
            onError={onError}
            onNotice={onNotice}
            navigate={navigate}
            onRefresh={onRefresh}
            onCreated={vi.fn()}
          />
        </I18nProvider>,
      ),
    );
    await choose("my-game-profile");
    expect(bridge.scan).toHaveBeenLastCalledWith(request, "my-game-profile");
    expect(button("Review preview").disabled).toBe(true);
    expect(bridge.encode).not.toHaveBeenCalled();
    await choose("third");
    expect(bridge.scan).toHaveBeenLastCalledWith(request, "third");
    const complete = {
      ...setting("complete-option", "lunar", 123),
      profile: "third",
      pointer: "/KEYSTROKES/options/width",
      label: "Width",
    };
    await act(async () =>
      pending.get("third")!({ ...common, settings: [game, complete] }),
    );
    await click("Review preview");
    expect(bridge.encode).toHaveBeenLastCalledWith(
      [game, complete],
      expect.any(Object),
      request,
    );
    await act(async () =>
      pending.get("my-game-profile")!({
        ...common,
        settings: [
          { ...complete, id: "stale-option", profile: "my-game-profile" },
        ],
      }),
    );
    expect(button("Create share code").disabled).toBe(false);
    expect(container.querySelector<HTMLSelectElement>("select")!.value).toBe(
      "third",
    );
    expect(bridge.encode).toHaveBeenCalledTimes(1);
    expect(onError).not.toHaveBeenCalled();
  });
  it("passes the configured source folders to HUD export encoding", async () => {
    const request = {
      minecraftRoot: "/fixture/game",
      lunarRoot: "/fixture/lunar",
    };
    const hud = {
      ...setting("local-x", "lunar", 200),
      profile: "default",
      pointer: "/FPS/x",
    };
    bridge.encode.mockResolvedValue({
      code: "PRS3:synthetic",
      compressedBytes: 100,
      uncompressedBytes: 200,
      settingCount: 1,
    });
    await act(async () =>
      root.render(
        <I18nProvider language="en">
          <Export
            scan={{ ...scan, activeLunarProfile: "default", settings: [hud] }}
            request={request}
            backups={[]}
            onError={onError}
            onNotice={onNotice}
            navigate={navigate}
            onRefresh={onRefresh}
            onCreated={vi.fn()}
          />
        </I18nProvider>,
      ),
    );
    await click("Review preview");
    expect(bridge.encode).toHaveBeenCalledWith(
      [hud],
      { platform: "macos", minecraftVersion: undefined },
      request,
    );
  });
  it("defaults HUD adjustment on and confirms the measured window and adjusted values before applying", async () => {
    await prepareHud();
    expect(
      container.querySelector<HTMLInputElement>(
        ".hud-layout-controls input[type=checkbox]",
      )!.checked,
    ).toBe(true);
    expect(container.querySelector(".hud-size-fields")).toBeNull();
    await click("Preview differences");
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({ hudLayout: { mode: "auto" } }),
    );
    expect(
      container.querySelector(".hud-layout-summary")!.textContent,
    ).toContain("1920 × 1080 px");
    expect(
      container.querySelector(".hud-layout-summary")!.textContent,
    ).toContain("Adjusted 2 positions");
    const row = [...container.querySelectorAll(".diff-row")].find(
      (item) => item.querySelector("strong")?.textContent === "X",
    )!;
    expect(row.querySelectorAll("code")[1].textContent).toBe("400");
    const y = [...container.querySelectorAll(".diff-row")]
      .find((item) => item.querySelector("strong")?.textContent === "Y")!
      .querySelectorAll("code")[1];
    expect(y.textContent).toBe("163.8");
    expect(y.title).toBe("163.79999999999998");
    const exact = (await bridge.preview.mock.results[0].value) as ImportPreview;
    expect(
      exact.changes.find((change) => change.id === "hud-y")!.incoming,
    ).toBe(163.79999999999998);
    await click("Apply selected settings");
    expect(
      container.querySelector("dialog .hud-layout-summary")!.textContent,
    ).toContain("1920 × 1080 px");
    await click("Back up and apply");
    expect(bridge.apply).toHaveBeenCalledWith(
      expect.objectContaining({ hudLayout: { mode: "auto" } }),
      expect.any(String),
      false,
    );
  });
  it("keeps exact HUD coordinate display when importing the original positions", async () => {
    await prepareHud();
    await act(async () =>
      container
        .querySelector<HTMLInputElement>(
          ".hud-layout-controls input[type=checkbox]",
        )!
        .click(),
    );
    await click("Preview differences");
    const y = [...container.querySelectorAll(".diff-row")]
      .find((item) => item.querySelector("strong")?.textContent === "Y")!
      .querySelectorAll("code")[1];
    expect(y.textContent).toBe("81.89999999999999");
    expect(y.title).toBe("81.89999999999999");
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({ hudLayout: { mode: "preserve" } }),
    );
  });
  it("blocks legacy HUD scaling, then requires a new preview when keeping the original values", async () => {
    await prepareHud("missing-source");
    await click("Preview differences");
    expect(button("Apply selected settings").disabled).toBe(true);
    expect(container.textContent).toContain(
      "Create a new code on the source device",
    );
    const first = (await bridge.preview.mock.results[0].value) as ImportPreview;
    await act(async () =>
      container
        .querySelector<HTMLInputElement>(
          ".hud-layout-controls input[type=checkbox]",
        )!
        .click(),
    );
    expect(container.querySelector(".diff-panel")).toBeNull();
    await click("Preview differences");
    const fresh = (await bridge.preview.mock.results[1].value) as ImportPreview;
    expect(fresh.fingerprint).not.toBe(first.fingerprint);
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({ hudLayout: { mode: "preserve" } }),
    );
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(bridge.apply).toHaveBeenCalledWith(
      expect.objectContaining({ hudLayout: { mode: "preserve" } }),
      fresh.fingerprint,
      false,
    );
  });
  it("keeps manual inputs available after detection fails and binds a fresh preview to valid physical dimensions", async () => {
    await prepareHud("unavailable");
    await click("Preview differences");
    const old = (await bridge.preview.mock.results[0].value) as ImportPreview;
    expect(button("Apply selected settings").disabled).toBe(true);
    expect(button("Preview differences").disabled).toBe(true);
    await changeSize("width", "319");
    expect(container.querySelector(".diff-panel")).toBeNull();
    await changeSize("height", "1080");
    expect(button("Preview differences").disabled).toBe(true);
    await changeSize("width", "1920");
    await click("Preview differences");
    const fresh = (await bridge.preview.mock.results[1].value) as ImportPreview;
    expect(fresh.fingerprint).not.toBe(old.fingerprint);
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({
        hudLayout: {
          mode: "manual",
          windowSize: { width: 1920, height: 1080 },
        },
      }),
    );
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(bridge.apply).toHaveBeenCalledWith(
      expect.objectContaining({
        hudLayout: {
          mode: "manual",
          windowSize: { width: 1920, height: 1080 },
        },
      }),
      fresh.fingerprint,
      false,
    );
  });
  it("discards an in-flight adjusted preview after the user changes the HUD mode", async () => {
    await prepareHud();
    const original = bridge.preview.getMockImplementation()!;
    let complete!: (value: ImportPreview) => void;
    bridge.preview.mockImplementationOnce(
      () =>
        new Promise<ImportPreview>((resolve) => {
          complete = resolve;
        }),
    );
    await click("Preview differences");
    await act(async () =>
      container
        .querySelector<HTMLInputElement>(
          ".hud-layout-controls input[type=checkbox]",
        )!
        .click(),
    );
    await act(async () =>
      complete(
        await original({
          code: "windows-hud",
          selectedIds: ["windows-fov", "windows-fps", "hud-x", "hud-y"],
          target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
          request: {},
          hudLayout: { mode: "auto" },
        }),
      ),
    );
    expect(container.querySelector(".diff-panel")).toBeNull();
    expect(bridge.apply).not.toHaveBeenCalled();
    await click("Preview differences");
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({ hudLayout: { mode: "preserve" } }),
    );
  });
  it("starts with the active Lunar preset and keeps the user's different choice across scans", async () => {
    currentScan = { ...scan, activeLunarProfile: "my-game-profile" };
    await act(async () => render());
    await enter("windows-first");
    expect(
      container.querySelector<HTMLSelectElement>(".profile-selects select")!
        .value,
    ).toBe("my-game-profile");
    await choose("default");
    currentScan = { ...currentScan };
    await act(async () => render());
    expect(
      container.querySelector<HTMLSelectElement>(".profile-selects select")!
        .value,
    ).toBe("default");
    await click("Preview differences");
    expect(bridge.preview).toHaveBeenCalledWith(
      expect.objectContaining({
        target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
      }),
    );
  });
  it("rejects several shared game profiles before planning a write", async () => {
    const shared = envelope("windows", 80);
    bridge.decode.mockResolvedValueOnce({
      ...shared,
      settings: [
        ...shared.settings,
        { ...setting("other-fov", "minecraft", 90), profile: "profile-2" },
      ],
    });
    await enter("windows-multiple");
    expect(container.textContent).toContain("one profile per app");
    expect(container.querySelector(".tree-picker")).toBeNull();
    expect(bridge.preview).not.toHaveBeenCalled();
    expect(bridge.apply).not.toHaveBeenCalled();
  });
  it("uses the default game folder and retains the Lunar preset for Windows-to-Mac then Mac-to-Mac imports", async () => {
    await enter("windows-first");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual([""]);
    expect(button("Preview differences").disabled).toBe(true);
    await choose("default");
    await click("Preview differences");
    expect(bridge.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({
        code: "windows-first",
        selectedIds: ["windows-fov", "windows-fps"],
        target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
      }),
    );
    expect(container.textContent).toContain(minecraftPath);
    expect(container.textContent).toContain(lunarPath);
    await click("Apply selected settings");
    await click("Back up and apply");
    expect(container.textContent).toContain("Settings applied.");
    expect(container.textContent).not.toContain("Minecraft · Minecraft");
    expect(container.textContent).toContain(minecraftPath);
    expect(container.textContent).not.toContain(lunarPath);
    await click("Import another code");
    await enter("macos-second");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual(["default"]);
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
      target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
      request: {},
    });
    expect(currentFov).toBe(90);
    expect(onRefresh).toHaveBeenCalledTimes(2);
    expect(onError).not.toHaveBeenCalled();
  });

  it("requires a new preview after a failed apply and uses the fresh fingerprint", async () => {
    await enter("windows-first");
    await choose("default");
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
        target: { minecraftProfile: "Minecraft", lunarProfile: "default" },
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
        (file) => file.profile === "Minecraft" || file.profile === "default",
      ),
      lunarProfiles: ["default"],
    };
    await act(async () => render());
    await enter("macos-first");
    expect(
      [...container.querySelectorAll<HTMLSelectElement>("select")].map(
        (select) => select.value,
      ),
    ).toEqual(["default"]);
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
