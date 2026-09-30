import { useEffect, useState } from "react";
import { Moon, SlidersHorizontal } from "lucide-react";
import type { Source } from "../models";
export function ApplicationIcon({
  source,
  data,
}: {
  source: Source;
  data?: string | null;
}) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [data]);
  if (!failed && data?.startsWith("data:image/png;base64,"))
    return (
      <img
        className="application-icon-image"
        src={data}
        alt={source === "minecraft" ? "Minecraft" : "Lunar Client"}
        onError={() => setFailed(true)}
      />
    );
  return source === "minecraft" ? (
    <SlidersHorizontal size={24} />
  ) : (
    <Moon size={25} />
  );
}
