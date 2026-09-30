import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import ts from "typescript";
import { messages } from "./messages";
import { translate } from ".";
import type { Language } from "../models";

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? sourceFiles(join(directory, entry.name))
      : /\.tsx?$/.test(entry.name)
        ? [join(directory, entry.name)]
        : [],
  );
}

function usedKeys(): Set<string> {
  const keys = new Set<string>();
  for (const file of sourceFiles(join(process.cwd(), "src"))) {
    if (file.includes("i18n")) continue;
    const source = ts.createSourceFile(
      file,
      readFileSync(file, "utf8"),
      ts.ScriptTarget.Latest,
      true,
      file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
    );
    const visit = (node: ts.Node) => {
      if (
        ts.isCallExpression(node) &&
        ts.isIdentifier(node.expression) &&
        node.expression.text === "t" &&
        node.arguments[0] &&
        ts.isStringLiteral(node.arguments[0])
      )
        keys.add(node.arguments[0].text);
      if (
        (file.endsWith("App.tsx") || file.endsWith("tree.ts")) &&
        ts.isPropertyAssignment(node) &&
        ts.isIdentifier(node.name) &&
        ["label", "title", "subtitle"].includes(node.name.text) &&
        ts.isStringLiteral(node.initializer)
      )
        keys.add(node.initializer.text);
      ts.forEachChild(node, visit);
    };
    visit(source);
  }
  return keys;
}

const languages: Language[] = ["en", "ja", "ko", "zh", "fi", "es", "de"];
const placeholders = (text: string) =>
  [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

describe("application translations", () => {
  it("includes a complete translation for every page, action, and dialog key", () => {
    for (const key of usedKeys()) {
      expect(messages[key], key).toBeDefined();
      for (const language of languages.filter((language) => language !== "ja"))
        expect(messages[key][language], `${key}: ${language}`).toBeTruthy();
    }
  });
  it("preserves interpolation fields in every language", () => {
    for (const [key, translations] of Object.entries(messages))
      for (const text of Object.values(translations))
        expect(placeholders(text), key).toEqual(placeholders(key));
  });
  it("translates page titles and safely inserts profile names and counts", () => {
    expect(translate("ja", "概要")).toBe("概要");
    expect(translate("en", "概要")).toBe("Overview");
    expect(translate("fi", "概要")).toBe("Yleiskatsaus");
    expect(
      translate("de", "{0} プロフィール · {1}", { 0: 2, 1: "<local>" }),
    ).toBe("2 Profile · <local>");
    expect(translate("ja", "Display")).toBe("画面");
    expect(translate("de", "Controls")).toBe("Steuerung");
    expect(translate("ko", "Resource Packs")).toBe("리소스 팩");
  });
});
