import { useEffect, useState } from "react";
import type { Preferences } from "../models";

const defaults: Preferences = {
  language: "ja",
  theme: "dark",
  reduceMotion: false,
  sidebarCollapsed: false,
  request: {},
};

function loadPreferences(): Preferences {
  try {
    const stored = JSON.parse(
      localStorage.getItem("prism-preferences") || "{}",
    ) as Partial<Preferences>;
    return {
      language: ["en", "ja", "ko", "zh", "fi", "es", "de"].includes(
        stored.language || "",
      )
        ? stored.language!
        : defaults.language,
      theme: ["dark", "light", "system"].includes(stored.theme || "")
        ? stored.theme!
        : defaults.theme,
      reduceMotion: stored.reduceMotion === true,
      sidebarCollapsed: stored.sidebarCollapsed === true,
      request: {
        minecraftRoot:
          typeof stored.request?.minecraftRoot === "string"
            ? stored.request.minecraftRoot
            : undefined,
        lunarRoot:
          typeof stored.request?.lunarRoot === "string"
            ? stored.request.lunarRoot
            : undefined,
      },
    };
  } catch {
    return defaults;
  }
}

export function usePreferences() {
  const [preferences, setPreferences] = useState(loadPreferences);
  useEffect(() => {
    localStorage.setItem("prism-preferences", JSON.stringify(preferences));
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const updateTheme = () =>
      (document.documentElement.dataset.theme =
        preferences.theme === "system"
          ? query.matches
            ? "dark"
            : "light"
          : preferences.theme);
    updateTheme();
    document.documentElement.dataset.reduceMotion = String(
      preferences.reduceMotion,
    );
    query.addEventListener("change", updateTheme);
    return () => query.removeEventListener("change", updateTheme);
  }, [preferences]);
  return { preferences, setPreferences };
}
