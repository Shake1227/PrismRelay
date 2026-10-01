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
import type { ScanReport } from "./models";
import App from "./App";

const bridge = vi.hoisted(() => ({
  scan: vi.fn(),
  backups: vi.fn(),
  info: vi.fn(),
  decode: vi.fn(),
  preview: vi.fn(),
  apply: vi.fn(),
}));
vi.mock("./services/backend", () => ({ isDesktop: true, backend: bridge }));
const gamePath = "/fixture/minecraft/options.txt";
const scan: ScanReport = {
  settings: [],
  files: [
    {
      id: "game",
      source: "minecraft",
      profile: "Minecraft",
      path: gamePath,
      fileKind: "options",
    },
  ],
  minecraftDetected: true,
  lunarDetected: false,
  minecraftVersions: [],
  lunarProfiles: [],
  warnings: [],
  platform: "macOS",
  runningProcesses: [],
};
let dom: JSDOM;
let root: Root;
let container: HTMLElement;
let mount: (typeof import("react-dom/client"))["createRoot"];

async function click(label: string) {
  const control = [
    ...container.querySelectorAll<HTMLButtonElement>("button"),
  ].find((button) => button.textContent?.trim() === label)!;
  expect(control, label).toBeDefined();
  expect(control.disabled).toBe(false);
  await act(async () => control.click());
}

beforeAll(async () => {
  dom = new JSDOM("<!doctype html><div id='app'></div>", {
    url: "http://localhost",
    pretendToBeVisual: true,
  });
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("navigator", dom.window.navigator);
  vi.stubGlobal("localStorage", dom.window.localStorage);
  vi.stubGlobal("HTMLElement", dom.window.HTMLElement);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  Object.defineProperty(dom.window, "matchMedia", {
    value: () => ({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
  Object.defineProperty(dom.window.HTMLElement.prototype, "scrollTo", {
    value: vi.fn(function (this: HTMLElement, options: ScrollToOptions) {
      this.scrollTop = options.top ?? 0;
    }),
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
  dom.window.localStorage.setItem(
    "prism-preferences",
    JSON.stringify({ language: "en", reduceMotion: false }),
  );
  bridge.scan.mockResolvedValue(scan);
  bridge.backups.mockResolvedValue([]);
  bridge.info.mockResolvedValue({
    name: "Prism Relay",
    version: "1.0.1",
    platform: "macOS",
    repository: "https://github.com/Shake1227/PrismRelay",
    license: "GPL-3.0-or-later",
  });
  bridge.decode.mockResolvedValue({
    formatVersion: 2,
    createdAt: "2026-10-01T00:00:00Z",
    applicationVersion: "1.0.1",
    metadata: { platform: "windows" },
    settings: [
      {
        id: "incoming-fov",
        source: "minecraft",
        profile: "profile-1",
        fileKind: "options",
        pointer: "fov",
        label: "FOV",
        category: "Video",
        group: "Display",
        value: 0.5,
      },
    ],
  });
  bridge.preview.mockResolvedValue({
    fingerprint: "0".repeat(64),
    targetFiles: [
      {
        source: "minecraft",
        profile: "Minecraft",
        fileKind: "options",
        path: gamePath,
      },
    ],
    selectedCount: 1,
    warnings: [],
    changes: [
      {
        id: "incoming-fov",
        source: "minecraft",
        category: "Video",
        label: "FOV",
        current: 0,
        incoming: 0.5,
        changed: true,
      },
    ],
  });
  bridge.apply.mockResolvedValue({
    id: "fixture",
    createdAt: "2026-10-01T00:00:00Z",
    platform: "macOS",
    minecraftVersions: [],
    lunarProfiles: [],
    reason: "import",
    status: "ready",
    files: [
      {
        originalPath: gamePath,
        relativeName: "0000.snapshot",
        checksum: "fixture",
        size: 20,
      },
    ],
  });
  container = dom.window.document.getElementById("app")!;
  root = mount(container);
  await act(async () => root.render(<App />));
});
afterEach(async () => {
  await act(async () => root.unmount());
  vi.useRealTimers();
});
afterAll(() => {
  dom.window.close();
  vi.unstubAllGlobals();
});

describe("apply completion during configuration refresh", () => {
  it("keeps the completion mounted and visible while refreshing an initially empty settings scan", async () => {
    await click("Import");
    const input = container.querySelector<HTMLTextAreaElement>("textarea")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(
        dom.window.HTMLTextAreaElement.prototype,
        "value",
      )!.set!.call(input, "PRS2:synthetic");
      input.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
    });
    await act(async () => vi.advanceTimersByTime(500));
    expect(container.querySelector("select")).toBeNull();
    await click("Preview differences");
    let finishRefresh!: (value: ScanReport) => void;
    bridge.scan.mockImplementationOnce(
      () =>
        new Promise<ScanReport>((resolve) => {
          finishRefresh = resolve;
        }),
    );
    await click("Apply selected settings");
    const scroller = container.querySelector<HTMLElement>(".main-content")!;
    scroller.scrollTop = 400;
    await click("Back up and apply");
    const animation = container.querySelector<HTMLElement>(
      ".crystal-completion",
    )!;
    expect(animation).not.toBeNull();
    expect(animation.dataset.motion).toBe("running");
    expect(scroller.scrollTop).toBe(0);
    expect(container.querySelector(".loading-screen")).toBeNull();
    expect(container.textContent).toContain("Settings applied.");
    expect(bridge.apply).toHaveBeenCalledWith(
      expect.objectContaining({
        target: { minecraftProfile: "Minecraft", lunarProfile: undefined },
      }),
      "0".repeat(64),
      false,
    );
    await act(async () => finishRefresh({ ...scan }));
    expect(container.querySelector(".crystal-completion")).toBe(animation);
    expect(container.textContent).toContain("Settings applied.");
    expect(
      [...container.querySelectorAll<HTMLButtonElement>("button")].find(
        (button) => button.textContent?.trim() === "Import another code",
      )?.disabled,
    ).toBe(false);
  });
});
