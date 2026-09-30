import { useEffect, useState } from "react";
import { Moon, SlidersHorizontal } from "lucide-react";
import type { Source } from "../models";
export function ApplicationIcon({
  source,
  data,
  size,
  decorative = false,
}: {
  source: Source;
  data?: string | null;
  size?: number;
  decorative?: boolean;
}) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [data]);
  if (!failed && data?.startsWith("data:image/png;base64,"))
    return (
      <img
        className="application-icon-image"
        src={data}
        alt={
          decorative
            ? ""
            : source === "minecraft"
              ? "Minecraft"
              : "Lunar Client"
        }
        style={size ? { width: size, height: size } : undefined}
        onError={() => setFailed(true)}
      />
    );
  return source === "minecraft" ? (
    <SlidersHorizontal size={size ?? 24} />
  ) : (
    <Moon size={size ?? 25} />
  );
}
