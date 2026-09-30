import { describe, expect, it } from "vitest";
import QRCode from "qrcode";
import { decodeQrPixels, qrShareCode } from "./qr";

describe("QR share-code reading", () => {
  it("reads generated share-code QR pixels and preserves the exact code", async () => {
    const code = "PRS2:synthetic-round-trip-test-1234567890";
    const qr = QRCode.create(code, { errorCorrectionLevel: "M" });
    const scale = 5;
    const width = (qr.modules.size + 8) * scale;
    const pixels = new Uint8ClampedArray(width * width * 4).fill(255);
    for (let y = 0; y < width; y++)
      for (let x = 0; x < width; x++) {
        const mx = Math.floor(x / scale) - 4;
        const my = Math.floor(y / scale) - 4;
        if (
          mx >= 0 &&
          my >= 0 &&
          mx < qr.modules.size &&
          my < qr.modules.size &&
          qr.modules.get(my, mx)
        ) {
          const offset = (y * width + x) * 4;
          pixels[offset] = pixels[offset + 1] = pixels[offset + 2] = 0;
        }
      }
    expect(await decodeQrPixels(pixels, width, width)).toBe(code);
  });
  it("rejects unrelated QR contents and oversized images", async () => {
    expect(() => qrShareCode("https://example.com")).toThrow();
    expect(() => qrShareCode("PRS2:" + "a".repeat(2 * 1024 * 1024))).toThrow();
    await expect(
      decodeQrPixels(new Uint8ClampedArray(4), 4097, 1),
    ).rejects.toThrow();
    expect(qrShareCode("  PRS1:example\n")).toBe("PRS1:example");
  });
});
