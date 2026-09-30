import { useContext, useMemo } from "react";
import type { Language } from "../models";
import { LanguageContext } from "./context";
import { japaneseMessages, messages } from "./messages";

export { I18nProvider } from "./provider";
export type TranslationParameters = Record<string, string | number>;
const locales: Record<Language, string> = {
  en: "en-US",
  ja: "ja-JP",
  ko: "ko-KR",
  zh: "zh-CN",
  fi: "fi-FI",
  es: "es-ES",
  de: "de-DE",
};

export function translate(
  language: Language,
  key: string,
  parameters: TranslationParameters = {},
): string {
  const text =
    language === "ja"
      ? japaneseMessages[key] || key
      : messages[key]?.[language] || key;
  return text.replace(/\{(\w+)\}/g, (token, name: string) =>
    parameters[name] === undefined ? token : String(parameters[name]),
  );
}

export function useI18n() {
  const language = useContext(LanguageContext);
  return useMemo(
    () => ({
      language,
      locale: locales[language],
      t: (key: string, parameters?: TranslationParameters) =>
        translate(language, key, parameters),
    }),
    [language],
  );
}
