import { useI18n } from "../i18n";
import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Copy,
  FileDown,
  Plus,
  QrCode,
  Share2,
} from "lucide-react";
import QRCode from "qrcode";
import type { EncodedShare, Setting, TargetProfiles } from "../models";
import { TreePicker } from "../components/TreePicker";
import { ProfileSelect } from "../components/ProfileSelect";
import { Modal } from "../components/Modal";
import { backend, isDesktop } from "../services/backend";
import { formatBytes } from "../utils/format";
import { presets, selectPreset } from "../utils/tree";
import { minecraftVersion, reconcileProfiles } from "../utils/profiles";
import { createRequestGuard } from "../utils/requestGuard";
import type { WorkspaceProps } from "./types";
interface SavedPreset {
  name: string;
  pointers: string[];
}
function loadPresets(): SavedPreset[] {
  try {
    const stored: unknown = JSON.parse(
      localStorage.getItem("prism-presets") || "[]",
    );
    return Array.isArray(stored)
      ? stored.filter(
          (item): item is SavedPreset =>
            typeof item === "object" &&
            item !== null &&
            typeof item.name === "string" &&
            Array.isArray(item.pointers) &&
            item.pointers.every(
              (pointer: unknown) => typeof pointer === "string",
            ),
        )
      : [];
  } catch {
    return [];
  }
}
export function Export({
  scan,
  request,
  onError,
  onNotice,
  onCreated,
}: WorkspaceProps & {
  onCreated: () => void;
}) {
  const { t } = useI18n();
  const [profiles, setProfiles] = useState<TargetProfiles>(() =>
    reconcileProfiles(scan, {}),
  );
  const settings = scan.settings.filter((setting) =>
    setting.source === "minecraft"
      ? setting.profile === profiles.minecraftProfile
      : setting.profile === profiles.lunarProfile,
  );
  const [selected, setSelected] = useState(() =>
    selectPreset(settings, "everything"),
  );
  const [preset, setPreset] = useState("everything");
  const [savedPresets, setSavedPresets] = useState(loadPresets);
  const [presetName, setPresetName] = useState("");
  const [savingPreset, setSavingPreset] = useState(false);
  const [encoded, setEncoded] = useState<EncodedShare | null>(null);
  const [created, setCreated] = useState(false);
  const [busy, setBusy] = useState(false);
  const [qr, setQr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const previewGuard = useRef(createRequestGuard());
  const selectedSettings = settings.filter((setting) =>
    selected.has(setting.id),
  );
  useEffect(() => {
    previewGuard.current.invalidate();
    setSelected(
      selectPreset(
        scan.settings.filter((setting) =>
          setting.source === "minecraft"
            ? setting.profile === profiles.minecraftProfile
            : setting.profile === profiles.lunarProfile,
        ),
        "everything",
      ),
    );
    setPreset("everything");
    setEncoded(null);
    setCreated(false);
  }, [scan, profiles]);
  useEffect(() => {
    setProfiles((current) => reconcileProfiles(scan, current));
  }, [scan]);
  const changeSelection = (value: Set<string>) => {
    previewGuard.current.invalidate();
    setSelected(value);
    setPreset("custom");
    setEncoded(null);
  };
  const applyPreset = (value: string) => {
    previewGuard.current.invalidate();
    setPreset(value);
    const saved = savedPresets.find((item) => `saved:${item.name}` === value);
    setSelected(
      saved
        ? new Set(
            settings
              .filter((setting) =>
                saved.pointers.includes(`${setting.source}:${setting.pointer}`),
              )
              .map((setting) => setting.id),
          )
        : selectPreset(settings, value),
    );
    setEncoded(null);
  };
  const preview = async () => {
    const isCurrent = previewGuard.current.begin();
    setBusy(true);
    try {
      const result = await backend.encode(
        selectedSettings,
        {
          minecraftVersion: minecraftVersion(scan, profiles.minecraftProfile),
          platform: scan.platform.toLowerCase(),
        },
        request,
      );
      if (isCurrent()) {
        setEncoded(result);
        setCreated(false);
      }
    } catch (error) {
      if (isCurrent()) onError(error);
    } finally {
      setBusy(false);
    }
  };
  const copy = async () => {
    if (!encoded) return;
    try {
      await navigator.clipboard.writeText(encoded.code);
      setCopied(true);
      onNotice(t("共有コードをコピーしました"));
      window.setTimeout(() => setCopied(false), 2500);
    } catch (error) {
      onError(error);
    }
  };
  const makeQr = async () => {
    if (!encoded) return;
    if (encoded.code.length > 2200) {
      onNotice(
        t(
          "コードが長いため QR にできません。コピーまたはファイル保存をご利用ください。",
        ),
      );
      return;
    }
    try {
      setQr(
        await QRCode.toDataURL(encoded.code, {
          width: 768,
          margin: 2,
          errorCorrectionLevel: "M",
        }),
      );
    } catch (error) {
      onError(error);
    }
  };
  const savePreset = () => {
    if (!presetName.trim()) return;
    const next = [
      ...savedPresets.filter((item) => item.name !== presetName.trim()),
      {
        name: presetName.trim(),
        pointers: selectedSettings.map(
          (setting) => `${setting.source}:${setting.pointer}`,
        ),
      },
    ];
    localStorage.setItem("prism-presets", JSON.stringify(next));
    setSavedPresets(next);
    setPreset(`saved:${presetName.trim()}`);
    setSavingPreset(false);
    setPresetName("");
    onNotice(t("プリセットを保存しました"));
  };
  const groups = selectedSettings.reduce<Record<string, number>>(
    (result, setting: Setting) => {
      const key = `${setting.source === "lunar" ? "Lunar" : "Minecraft"} · ${t(setting.category)}`;
      result[key] = (result[key] || 0) + 1;
      return result;
    },
    {},
  );
  if (created && encoded)
    return (
      <section className="glass-panel code-result">
        <div className="result-icon">
          <Check size={26} />
        </div>
        <h2>{t("共有コードを作成しました。")}</h2>
        <p>
          {encoded.settingCount} {t("項目")}
        </p>
        <div className="code-meta">
          <span>
            {encoded.code.length} {t("文字")}
          </span>
          {!isDesktop && <span>{t("サンプル")}</span>}
        </div>
        {encoded.code.length > 2000 && (
          <p className="share-file-hint">
            {t("コードが長い場合はファイルとして保存して共有できます。")}
          </p>
        )}
        <textarea
          className="share-code"
          value={encoded.code}
          readOnly
          aria-label={t("作成した共有コード")}
        />
        <div className="result-actions">
          <button className="button primary" onClick={() => void copy()}>
            {copied ? <Check size={17} /> : <Copy size={17} />}
            {t("コードをコピー")}
          </button>
          <button
            className="button secondary"
            onClick={() =>
              void backend
                .saveCode(encoded.code)
                .then((saved) => saved && onNotice(t("ファイルを保存しました")))
                .catch(onError)
            }
          >
            <FileDown size={17} />
            {t("保存")}
          </button>
          <button className="button secondary" onClick={() => void makeQr()}>
            <QrCode size={17} />
            QR
          </button>
          <button
            className="button secondary"
            onClick={() =>
              void (navigator.share
                ? navigator
                    .share({
                      title: "Prism Relay",
                      text: encoded.code,
                    })
                    .catch(onError)
                : copy())
            }
          >
            <Share2 size={17} />
            {t("共有")}
          </button>
        </div>
        <button
          className="text-button back-button"
          onClick={() => setCreated(false)}
        >
          <ArrowLeft size={14} />
          {t("選択に戻る")}
        </button>
        {qr && (
          <Modal title={t("共有コードの QR")} onClose={() => setQr(null)}>
            <img
              className="qr-image"
              src={qr}
              alt={t("設定共有コードの QR コード")}
            />
            <p className="modal-copy">
              {t(
                "受け取り側でコードを読み取り、適用前に内容をご確認ください。",
              )}
            </p>
            <div className="modal-actions">
              <button
                className="button primary"
                onClick={() =>
                  void backend
                    .saveQrImage(qr)
                    .then(
                      (saved) => saved && onNotice(t("QR画像を保存しました。")),
                    )
                    .catch(onError)
                }
              >
                <FileDown size={16} />
                {t("QR画像を保存")}
              </button>
            </div>
          </Modal>
        )}
      </section>
    );
  return (
    <div className="export-page">
      <div className="workflow-steps">
        <span className="active">
          <b>1</b>
          {t("設定を選ぶ")}
        </span>
        <i />
        <span className={encoded ? "active" : ""}>
          <b>2</b>
          {t("プレビュー")}
        </span>
        <i />
        <span>
          <b>3</b>
          {t("コードを作成")}
        </span>
      </div>
      <div className="export-layout">
        <section className="glass-panel selection-panel">
          <div className="panel-heading">
            <div>
              <h3>{t("共有する設定")}</h3>
              <p>{t("必要な設定だけを選択してください。")}</p>
            </div>
          </div>
          <ProfileSelect
            scan={scan}
            value={profiles}
            onChange={(value) => {
              previewGuard.current.invalidate();
              setProfiles(value);
            }}
          />
          <div className="preset-bar">
            <label>
              <span>{t("プリセット")}</span>
              <select
                aria-label={t("プリセット")}
                value={preset}
                onChange={(event) => applyPreset(event.target.value)}
              >
                {presets.map((item) => (
                  <option key={item.id} value={item.id}>
                    {t(item.label)}
                  </option>
                ))}
                <option value="custom">{t("カスタム")}</option>
                {savedPresets.map((item) => (
                  <option key={item.name} value={`saved:${item.name}`}>
                    {item.name}
                  </option>
                ))}
              </select>
            </label>
            <button
              className="icon-button"
              onClick={() => setSavingPreset(true)}
              aria-label={t("プリセットを保存")}
              disabled={!selectedSettings.length}
            >
              <Plus size={17} />
            </button>
          </div>
          <TreePicker
            settings={settings}
            applicationIcons={scan.applicationIcons}
            selected={selected}
            onChange={changeSelection}
          />
        </section>
        <aside className="export-aside">
          <section className="glass-panel summary-panel">
            <span className="eyebrow">{t("EXPORT SUMMARY")}</span>
            <div className="selection-total">
              <strong>{selectedSettings.length}</strong>
              <span>{t("選択した設定")}</span>
            </div>
            <div className="summary-groups">
              {Object.entries(groups).map(([label, count]) => (
                <div key={label}>
                  <span>{label}</span>
                  <b>{count}</b>
                </div>
              ))}
            </div>
            {encoded && (
              <div className="size-estimate">
                <span>
                  {isDesktop ? t("圧縮後のサイズ") : t("サンプルサイズ")}
                </span>
                <strong>{formatBytes(encoded.compressedBytes)}</strong>
              </div>
            )}
            <button
              className="button primary full"
              disabled={!selectedSettings.length || busy}
              onClick={() =>
                encoded ? (setCreated(true), onCreated()) : void preview()
              }
            >
              {busy
                ? t("確認中…")
                : encoded
                  ? t("共有コードを作成")
                  : t("プレビューを確認")}
              <ArrowRight size={16} />
            </button>
          </section>
        </aside>
      </div>
      {savingPreset && (
        <Modal
          title={t("プリセットを保存")}
          onClose={() => setSavingPreset(false)}
        >
          <p className="modal-copy">
            {selectedSettings.length} {t("項目")}
          </p>
          <input
            className="text-input"
            autoFocus
            placeholder={t("プリセット名")}
            maxLength={40}
            value={presetName}
            onChange={(event) => setPresetName(event.target.value)}
            onKeyDown={(event) => event.key === "Enter" && savePreset()}
          />
          <div className="modal-actions">
            <button
              className="button secondary"
              onClick={() => setSavingPreset(false)}
            >
              {t("キャンセル")}
            </button>
            <button
              className="button primary"
              disabled={!presetName.trim()}
              onClick={savePreset}
            >
              {t("保存")}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
