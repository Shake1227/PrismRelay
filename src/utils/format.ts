import type { JsonValue } from "../models";

export function formatBytes(bytes: number): string {
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`;
}

export function formatDate(value: string, locale = "ja-JP"): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat(locale, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
}

export function formatValue(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return "—";
  if (typeof value === "boolean") return value ? "ON" : "OFF";
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}
