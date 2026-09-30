import type { ReactNode } from "react";
import type { Language } from "../models";
import { LanguageContext } from "./context";

export function I18nProvider({
  language,
  children,
}: {
  language: Language;
  children: ReactNode;
}) {
  return (
    <LanguageContext.Provider value={language}>
      {children}
    </LanguageContext.Provider>
  );
}
