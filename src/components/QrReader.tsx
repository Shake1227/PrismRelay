import { useEffect, useRef, useState } from "react";
import { Camera, FileImage, Square } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { IScannerControls } from "@zxing/browser";
import { Modal } from "./Modal";
import { useI18n } from "../i18n";
import { isDesktop } from "../services/backend";
import { decodeQrImage, qrShareCode } from "../services/qr";

export function QrReader({
  onCode,
  onError,
  onClose,
}: {
  onCode: (code: string) => void;
  onError: (error: unknown) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const video = useRef<HTMLVideoElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const controls = useRef<IScannerControls | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const generation = useRef(0);
  const [camera, setCamera] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const stop = () => {
    generation.current += 1;
    controls.current?.stop();
    controls.current = null;
    stream.current?.getTracks().forEach((track) => track.stop());
    stream.current = null;
    if (video.current) video.current.srcObject = null;
    setCamera(false);
    setBusy(false);
  };
  useEffect(
    () => () => {
      generation.current += 1;
      controls.current?.stop();
      stream.current?.getTracks().forEach((track) => track.stop());
    },
    [],
  );
  const accept = (text: string) => {
    const code = qrShareCode(text);
    stop();
    onCode(code);
    onClose();
  };
  const readImage = async (url: string) => {
    stop();
    const current = generation.current;
    setBusy(true);
    setError(null);
    try {
      const result = await decodeQrImage(url);
      if (current === generation.current) accept(result);
    } catch {
      if (current === generation.current)
        setError(
          t(
            "QRコードを読み取れませんでした。共有コードのQR画像を選ぶか、QR部分を大きく切り取ってください。",
          ),
        );
    } finally {
      if (current === generation.current) setBusy(false);
      if (url.startsWith("blob:")) URL.revokeObjectURL(url);
    }
  };
  const chooseImage = async () => {
    stop();
    const current = generation.current;
    if (!isDesktop) {
      fileInput.current?.click();
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const path = await open({
        multiple: false,
        title: t("QR画像を選択"),
        filters: [
          { name: "QR image", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
      });
      if (current !== generation.current || typeof path !== "string") return;
      const image = await invoke<string>("load_qr_image", { path });
      if (current === generation.current) await readImage(image);
    } catch (reason) {
      if (current === generation.current) onError(reason);
    } finally {
      if (current === generation.current) setBusy(false);
    }
  };
  const startCamera = async () => {
    stop();
    const current = generation.current;
    setBusy(true);
    setError(null);
    try {
      if (!navigator.mediaDevices?.getUserMedia) throw new Error("unsupported");
      const media = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: {
          facingMode: { ideal: "environment" },
          width: { ideal: 1280 },
          height: { ideal: 720 },
        },
      });
      if (current !== generation.current) {
        media.getTracks().forEach((track) => track.stop());
        return;
      }
      stream.current = media;
      setCamera(true);
      const { BrowserQRCodeReader } = await import("@zxing/browser");
      if (current !== generation.current || !video.current) return;
      const reader = new BrowserQRCodeReader();
      const scanner = await reader.decodeFromStream(
        media,
        video.current,
        (result, _reason, scannerControls) => {
          if (!result || current !== generation.current) return;
          try {
            const code = qrShareCode(result.getText());
            scannerControls.stop();
            accept(code);
          } catch {
            setError(
              t("Prism Relayの共有コードを含むQRコードを写してください。"),
            );
          }
        },
      );
      if (current === generation.current) controls.current = scanner;
      else scanner.stop();
    } catch {
      if (current === generation.current) {
        stop();
        setError(
          t(
            "カメラを使用できませんでした。カメラの接続とアクセス許可を確認するか、QR画像を選択してください。",
          ),
        );
      }
    } finally {
      if (current === generation.current) setBusy(false);
    }
  };
  return (
    <Modal
      title={t("QRコードを読み取る")}
      onClose={() => {
        stop();
        onClose();
      }}
    >
      <p>
        {t(
          "保存したQR画像を選択するか、カメラにQRコードを写してください。読み取り後に内容を確認できます。",
        )}
      </p>
      <div className="qr-reader-actions">
        <button
          className="button secondary"
          onClick={() => void chooseImage()}
          disabled={busy}
        >
          <FileImage size={17} />
          {t("QR画像を選択")}
        </button>
        {camera ? (
          <button className="button secondary" onClick={stop}>
            <Square size={17} />
            {t("カメラを停止")}
          </button>
        ) : (
          <button
            className="button secondary"
            onClick={() => void startCamera()}
            disabled={busy}
          >
            <Camera size={17} />
            {t("カメラで読み取る")}
          </button>
        )}
      </div>
      <input
        ref={fileInput}
        type="file"
        hidden
        accept="image/png,image/jpeg,image/webp"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.currentTarget.value = "";
          if (!file) return;
          if (file.size > 8 * 1024 * 1024) {
            setError(t("8 MB以下のQR画像を選択してください。"));
            return;
          }
          void readImage(URL.createObjectURL(file));
        }}
      />
      <video
        ref={video}
        className={`qr-camera ${camera ? "active" : ""}`}
        muted
        playsInline
        autoPlay
        aria-label={t("QRスキャン用カメラ")}
      />
      {busy && <p role="status">{t("読み取り中…")}</p>}
      {error && (
        <p className="qr-reader-error" role="alert">
          {error}
        </p>
      )}
      <p className="subtle">{t("画像とカメラ映像は端末内で処理されます。")}</p>
    </Modal>
  );
}
