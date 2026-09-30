import { describe, expect, it } from "vitest";
import { createRequestGuard } from "./requestGuard";

describe("preview request ordering", () => {
  it("rejects a pending result after the reviewed selection changes", () => {
    const guard = createRequestGuard();
    const isCurrent = guard.begin();
    guard.invalidate();
    expect(isCurrent()).toBe(false);
  });
  it("accepts only the latest request when responses arrive out of order", async () => {
    const guard = createRequestGuard();
    const first = guard.begin();
    const second = guard.begin();
    const accepted: string[] = [];
    await Promise.resolve().then(() => {
      if (second()) accepted.push("latest");
    });
    await Promise.resolve().then(() => {
      if (first()) accepted.push("stale");
    });
    expect(accepted).toEqual(["latest"]);
  });
});
