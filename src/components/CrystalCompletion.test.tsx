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
import { act, StrictMode } from "react";
import type { Root } from "react-dom/client";
import { JSDOM } from "jsdom";
import { CrystalCompletion } from "./CrystalCompletion";

let dom: JSDOM;
let root: Root;
let container: HTMLElement;
let mount: (typeof import("react-dom/client"))["createRoot"];
let reduced: boolean;
let nextFrame: number;
let frames: Map<number, FrameRequestCallback>;
let visibility: DocumentVisibilityState;
const mediaListeners = new Set<() => void>();
const query = {
  get matches() {
    return reduced;
  },
  addEventListener: vi.fn((_event: string, listener: () => void) =>
    mediaListeners.add(listener),
  ),
  removeEventListener: vi.fn((_event: string, listener: () => void) =>
    mediaListeners.delete(listener),
  ),
};
function tick(timestamp: number) {
  const current = [...frames.values()];
  frames.clear();
  current.forEach((callback) => callback(timestamp));
}
beforeAll(async () => {
  dom = new JSDOM("<!doctype html><div id='completion'></div>", {
    url: "http://localhost",
  });
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("navigator", dom.window.navigator);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  Object.defineProperty(dom.window, "matchMedia", { value: () => query });
  Object.defineProperty(dom.window.document, "visibilityState", {
    get: () => visibility,
  });
  Object.defineProperty(dom.window, "requestAnimationFrame", {
    value: (callback: FrameRequestCallback) => {
      const id = ++nextFrame;
      frames.set(id, callback);
      return id;
    },
  });
  Object.defineProperty(dom.window, "cancelAnimationFrame", {
    value: (id: number) => frames.delete(id),
  });
  mount = (await import("react-dom/client")).createRoot;
});
beforeEach(() => {
  reduced = false;
  visibility = "visible";
  mediaListeners.clear();
  nextFrame = 0;
  frames = new Map();
  delete dom.window.document.documentElement.dataset.reduceMotion;
  container = dom.window.document.getElementById("completion")!;
  root = mount(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  expect(frames.size).toBe(0);
});
afterAll(() => {
  dom.window.close();
  vi.unstubAllGlobals();
});

describe("completion animation lifecycle", () => {
  it("updates SVG coordinates directly, finishes once, and replays on a new completion", async () => {
    await act(async () => root.render(<CrystalCompletion key="first" />));
    const element = container.querySelector<HTMLElement>(
      ".crystal-completion",
    )!;
    const core = container.querySelector(".completion-crystal")!;
    expect(element.dataset.motion).toBe("running");
    tick(0);
    const initial = core.getAttribute("transform");
    tick(1800);
    expect(core.getAttribute("transform")).not.toBe(initial);
    const particle = container.querySelector(".completion-particle")!;
    const position = () => {
      const match = particle
        .getAttribute("transform")!
        .match(/^translate\(([-.\d]+) ([-.\d]+)\)/)!;
      return Math.hypot(Number(match[1]), Number(match[2]));
    };
    const approaching = position();
    tick(3400);
    expect(position()).toBeLessThan(approaching);
    expect(core.getAttribute("opacity")).toBe("1");
    expect(
      Number(
        container.querySelector(".completion-charge")!.getAttribute("opacity"),
      ),
    ).toBeGreaterThan(0.5);
    expect(
      container.querySelector(".completion-ring, .completion-orbits"),
    ).toBeNull();
    tick(4530);
    expect(
      Number(
        container.querySelector(".completion-field")!.getAttribute("opacity"),
      ),
    ).toBeGreaterThan(0.4);
    expect(
      Number(
        container.querySelector(".completion-shard")!.getAttribute("opacity"),
      ),
    ).toBeGreaterThan(0.9);
    expect(container.querySelectorAll(".completion-shard > path")).toHaveLength(
      16,
    );
    tick(6500);
    expect(element.dataset.motion).toBe("finished");
    expect(core.getAttribute("opacity")).toBe("0");
    for (const node of container.querySelectorAll(
      ".completion-shard, .completion-particle, .completion-particle-trail, .completion-filament, .completion-charge, .completion-seams, .completion-field, .completion-afterglow",
    ))
      expect(node.getAttribute("opacity")).toBe("0");
    expect(frames.size).toBe(0);
    await act(async () => root.render(<CrystalCompletion key="first" />));
    expect(frames.size).toBe(0);
    await act(async () => root.render(<CrystalCompletion key="second" />));
    expect(frames.size).toBe(1);
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("running");
    expect(container.querySelector("use")).toBeNull();
  });
  it("keeps a static crystal for OS motion reduction and restarts after the app preference is re-enabled", async () => {
    reduced = true;
    await act(async () => root.render(<CrystalCompletion key="os" />));
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("reduced");
    expect(frames.size).toBe(0);
    reduced = false;
    await act(async () => root.render(<CrystalCompletion key="app" />));
    tick(0);
    tick(2700);
    expect(frames.size).toBe(1);
    await act(async () => {
      document.documentElement.dataset.reduceMotion = "true";
    });
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("reduced");
    expect(
      container.querySelector(".completion-crystal")!.getAttribute("opacity"),
    ).toBe("1");
    expect(
      container.querySelector(".completion-crystal")!.getAttribute("transform"),
    ).toBeNull();
    expect(frames.size).toBe(0);
    await act(async () => {
      document.documentElement.dataset.reduceMotion = "false";
    });
    expect(frames.size).toBe(1);
    tick(10000);
    expect(
      container.querySelector(".completion-crystal")!.getAttribute("opacity"),
    ).toBe("1");
  });
  it("begins on the first visible frame and pauses time while hidden", async () => {
    visibility = "hidden";
    await act(async () => root.render(<CrystalCompletion />));
    expect(frames.size).toBe(0);
    visibility = "visible";
    document.dispatchEvent(new dom.window.Event("visibilitychange"));
    tick(10000);
    tick(11800);
    const core = container.querySelector(".completion-crystal")!;
    const beforeHidden = core.getAttribute("transform");
    visibility = "hidden";
    document.dispatchEvent(new dom.window.Event("visibilitychange"));
    expect(frames.size).toBe(0);
    visibility = "visible";
    document.dispatchEvent(new dom.window.Event("visibilitychange"));
    tick(50000);
    expect(core.getAttribute("transform")).toBe(beforeHidden);
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("running");
    tick(52500);
    expect(
      Number(
        container.querySelector(".completion-field")!.getAttribute("opacity"),
      ),
    ).toBeGreaterThan(0.8);
  });
  it("cancels and restarts for OS motion changes without exposing an override or duplicating StrictMode frames", async () => {
    await act(async () =>
      root.render(
        <StrictMode>
          <CrystalCompletion />
        </StrictMode>,
      ),
    );
    expect(frames.size).toBe(1);
    tick(0);
    tick(1800);
    reduced = true;
    mediaListeners.forEach((listener) => listener());
    expect(frames.size).toBe(0);
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("reduced");
    reduced = false;
    mediaListeners.forEach((listener) => listener());
    expect(frames.size).toBe(1);
    tick(30000);
    expect(
      container.querySelector(".completion-crystal")!.getAttribute("opacity"),
    ).toBe("1");
    reduced = true;
    mediaListeners.forEach((listener) => listener());
    await act(async () =>
      root.render(<CrystalCompletion key="new-completion" />),
    );
    expect(frames.size).toBe(0);
    expect(
      container.querySelector<HTMLElement>(".crystal-completion")!.dataset
        .motion,
    ).toBe("reduced");
    expect(container.querySelector("button")).toBeNull();
  });
});
