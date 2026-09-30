import { afterEach, describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import App from "./App";

afterEach(() => vi.unstubAllGlobals());

describe("saved sidebar layout", () => {
  it("restores the icon rail with labeled page controls and a menu expansion control", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => JSON.stringify({ language: "en", sidebarCollapsed: true }),
    });
    const markup = renderToStaticMarkup(createElement(App));
    expect(markup).toContain('class="app-shell sidebar-collapsed"');
    expect(markup).toContain('<aside class="sidebar" id="app-sidebar">');
    expect(markup).toContain('aria-label="Export" title="Export"');
    expect(markup).toContain('aria-label="Backups" title="Backups"');
    expect(markup).toContain('aria-label="Expand menu"');
    expect(markup).toContain('title="Expand menu"');
    expect(markup).toContain(
      'aria-expanded="false" aria-controls="app-sidebar"',
    );
    expect(markup.indexOf('aria-label="Expand menu"')).toBeLessThan(
      markup.indexOf("</aside>"),
    );
  });
  it("keeps legacy saved preferences expanded and offers the close control", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => JSON.stringify({ language: "en", theme: "light" }),
    });
    const markup = renderToStaticMarkup(createElement(App));
    expect(markup).toContain('<aside class="sidebar" id="app-sidebar">');
    expect(markup).toContain('aria-label="Collapse menu"');
    expect(markup).toContain(
      'aria-expanded="true" aria-controls="app-sidebar"',
    );
  });
});
