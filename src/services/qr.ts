export function qrShareCode(text: string): string {
  const code = text.trim();
  if (code.length > 2 * 1024 * 1024 || !/^(PRS[12]:|PRDEMO1:)/.test(code))
    throw new Error("Prism Relayの共有コードを含むQR画像を選択してください。");
  return code;
}

export async function decodeQrPixels(
  data: Uint8ClampedArray,
  width: number,
  height: number,
): Promise<string> {
  if (
    width < 1 ||
    height < 1 ||
    width > 4096 ||
    height > 4096 ||
    data.length !== width * height * 4
  )
    throw new Error("4096ピクセル以下のQR画像を選択してください。");
  const {
    BinaryBitmap,
    HybridBinarizer,
    RGBLuminanceSource,
    QRCodeReader,
    DecodeHintType,
  } = await import("@zxing/library");
  const pixels = new Uint8ClampedArray(width * height);
  for (let i = 0; i < pixels.length; i++) {
    const offset = i * 4;
    const alpha = data[offset + 3] / 255;
    pixels[i] = Math.round(
      ((data[offset] + 2 * data[offset + 1] + data[offset + 2]) / 4) * alpha +
        255 * (1 - alpha),
    );
  }
  const image = new BinaryBitmap(
    new HybridBinarizer(new RGBLuminanceSource(pixels, width, height)),
  );
  const result = new QRCodeReader().decode(
    image,
    new Map([[DecodeHintType.TRY_HARDER, true]]),
  );
  return qrShareCode(result.getText());
}

export async function decodeQrImage(url: string): Promise<string> {
  if (!/^(data:image\/(png|jpeg|webp);base64,|blob:)/.test(url))
    throw new Error("PNG、JPEG、WebP画像を選択してください。");
  const image = new Image();
  image.src = url;
  await image.decode();
  if (image.naturalWidth > 4096 || image.naturalHeight > 4096)
    throw new Error("4096ピクセル以下のQR画像を選択してください。");
  const canvas = document.createElement("canvas");
  canvas.width = image.naturalWidth;
  canvas.height = image.naturalHeight;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("画像を読み込めませんでした。");
  context.drawImage(image, 0, 0);
  return decodeQrPixels(
    context.getImageData(0, 0, canvas.width, canvas.height).data,
    canvas.width,
    canvas.height,
  );
}
