import { useEffect, useMemo, useState } from "react";
import {
  deleteLocalCapabilityMatrix,
  auditXm5UnverifiedProperties,
  isDesktop,
  listLocalCapabilityMatrices,
  probeCameraDeviceInfo,
  saveLocalCapabilityMatrix,
  type CameraDiscovery,
  type LocalCapabilityMatrix,
  type LocalCapabilityMatrixProperty,
  type LocalCapabilityPropertyStatus,
} from "./lib/camera";
import type { CopyKey, Locale } from "./lib/i18n";

type Translator = (key: CopyKey) => string;

const propertyTemplates: Array<
  Pick<LocalCapabilityMatrixProperty, "key" | "labelZh" | "labelEn" | "scope">
> = [
  { key: "filmSimulation", labelZh: "軟片模擬", labelEn: "Film Simulation", scope: "custom_slot" },
  { key: "dynamicRange", labelZh: "動態範圍", labelEn: "Dynamic Range", scope: "custom_slot" },
  { key: "whiteBalance", labelZh: "白平衡", labelEn: "White Balance", scope: "custom_slot" },
  { key: "grainEffect", labelZh: "顆粒效果", labelEn: "Grain Effect", scope: "custom_slot" },
  { key: "colorChromeEffect", labelZh: "色彩效果", labelEn: "Color Chrome Effect", scope: "custom_slot" },
  { key: "colorChromeFxBlue", labelZh: "彩色 FX 藍色", labelEn: "Color Chrome FX Blue", scope: "custom_slot" },
  { key: "highlight", labelZh: "高光", labelEn: "Highlight Tone", scope: "custom_slot" },
  { key: "shadow", labelZh: "陰影", labelEn: "Shadow Tone", scope: "custom_slot" },
  { key: "color", labelZh: "色彩", labelEn: "Color", scope: "custom_slot" },
  { key: "sharpness", labelZh: "銳利度", labelEn: "Sharpness", scope: "custom_slot" },
  { key: "highIsoNoiseReduction", labelZh: "高 ISO 降噪", labelEn: "High ISO NR", scope: "custom_slot" },
  { key: "clarity", labelZh: "清晰度", labelEn: "Clarity", scope: "custom_slot" },
  { key: "colorSpace", labelZh: "色彩空間", labelEn: "Color Space", scope: "custom_slot" },
  { key: "imageSize", labelZh: "影像尺寸", labelEn: "Image Size", scope: "custom_slot" },
  { key: "imageQuality", labelZh: "影像品質", labelEn: "Image Quality", scope: "custom_slot" },
];

const localStatuses: LocalCapabilityPropertyStatus[] = [
  "read_detected_unverified",
  "write_rejected",
  "blocked_unknown",
];

function blankProperty(
  template?: (typeof propertyTemplates)[number],
): LocalCapabilityMatrixProperty {
  return {
    key: template?.key ?? "newProperty",
    labelZh: template?.labelZh ?? "新欄位",
    labelEn: template?.labelEn ?? "New property",
    code: "",
    scope: template?.scope ?? "unknown",
    status: "blocked_unknown",
    notes: "",
  };
}

function blankMatrix(locale: Locale): LocalCapabilityMatrix {
  const id = `local-${Date.now().toString(36)}`;
  return {
    schemaVersion: 1,
    id,
    manufacturer: "FUJIFILM",
    model: locale === "zh-TW" ? "未命名 Fujifilm 相機" : "Unnamed Fujifilm camera",
    firmware: "unknown",
    usbIds: ["04CB:0000"],
    customSlots: [],
    sourceUrl: "",
    sourceKind: "local_probe",
    evidenceSummary: "",
    lastVerifiedAt: undefined,
    createdAt: 0,
    updatedAt: 0,
    properties: propertyTemplates.map(blankProperty),
  };
}

function formatStatus(locale: Locale, status: LocalCapabilityPropertyStatus) {
  const labels =
    locale === "zh-TW"
      ? {
          read_detected_unverified: "已探測，尚未驗證",
          write_rejected: "相機拒絕",
          blocked_unknown: "未知／禁止寫入",
        }
      : {
          read_detected_unverified: "Detected, unverified",
          write_rejected: "Camera rejected",
          blocked_unknown: "Unknown / blocked",
        };
  return labels[status];
}

export function CapabilityMatrixPanel({
  devices,
  locale,
  onScan,
  t,
}: {
  devices: CameraDiscovery[];
  locale: Locale;
  onScan: () => Promise<CameraDiscovery[]>;
  t: Translator;
}) {
  const [matrices, setMatrices] = useState<LocalCapabilityMatrix[]>([]);
  const [draft, setDraft] = useState<LocalCapabilityMatrix>(() => blankMatrix(locale));
  const [selectedUsbId, setSelectedUsbId] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const cameraCandidates = useMemo(
    () => devices.filter((device) => device.isFujifilm && device.ptpInterfaceDetected),
    [devices],
  );

  useEffect(() => {
    if (!isDesktop) return;
    void listLocalCapabilityMatrices()
      .then((records) => {
        setMatrices(records);
        if (records[0]) setDraft(records[0]);
      })
      .catch(() => setMessage(t("matrixLoadFailed")));
    // `t` is recreated by the parent on every render; reloading on it would
    // continuously query SQLite after this effect updates component state.
    // The matrix contents are locale-neutral, so load once on mount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selectedUsbId && cameraCandidates[0]) {
      setSelectedUsbId(cameraCandidates[0].usbId);
    }
  }, [cameraCandidates, selectedUsbId]);

  function updateDraft(patch: Partial<LocalCapabilityMatrix>) {
    setDraft((current) => ({ ...current, ...patch }));
  }

  function updateProperty(index: number, patch: Partial<LocalCapabilityMatrixProperty>) {
    setDraft((current) => ({
      ...current,
      properties: current.properties.map((property, propertyIndex) =>
        propertyIndex === index ? { ...property, ...patch } : property,
      ),
    }));
  }

  async function scanAndReadIdentity() {
    setBusy(true);
    setMessage("");
    try {
      const scanned = cameraCandidates.length ? devices : await onScan();
      const candidates = scanned.filter(
        (device) => device.isFujifilm && device.ptpInterfaceDetected,
      );
      const usbId = selectedUsbId || candidates[0]?.usbId;
      if (!usbId) throw new Error(t("matrixNoCamera"));
      const identity = await probeCameraDeviceInfo(usbId);
      const sourceUrl =
        identity.model === "X-M5"
          ? "https://www.fujifilm-x.com/en-us/products/cameras/x-m5/specifications/"
          : draft.sourceUrl;
      setDraft((current) => ({
        ...current,
        id: current.createdAt ? current.id : `local-${identity.usbId.toLowerCase().replace(":", "-")}-${identity.deviceVersion}`,
        manufacturer: "FUJIFILM",
        model: identity.model,
        firmware: identity.deviceVersion,
        usbIds: [identity.usbId],
        customSlots: identity.model === "X-M5" ? ["C1", "C2", "C3", "C4"] : current.customSlots,
        sourceUrl,
        lastVerifiedAt: Date.now(),
      }));
      setSelectedUsbId(usbId);
      setMessage(t("matrixIdentityRead"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function auditUnverifiedProperties() {
    setBusy(true);
    setMessage("");
    try {
      const usbId = selectedUsbId || cameraCandidates[0]?.usbId;
      if (!usbId) throw new Error(t("matrixNoCamera"));
      const audit = await auditXm5UnverifiedProperties(usbId);
      const auditTimestamp = new Date().toISOString();
      setDraft((current) => {
        const byKey = new Map(current.properties.map((property) => [property.key, property]));
        for (const entry of audit.properties) {
          const [labelEn, labelZh = labelEn] = entry.label.split(" / ");
          const observation = [
            entry.valueHex ? `value=${entry.valueHex}` : `value error=${entry.valueError ?? "unavailable"}`,
            entry.descriptorDataType
              ? `descriptor type=${entry.descriptorDataType}, access=${entry.descriptorWritable ? "writable" : "read-only"}`
              : `descriptor error=${entry.descriptorError ?? "unavailable"}`,
          ].join("; ");
          const existing = byKey.get(entry.key);
          const previousNotes = existing?.notes.split("\nRead-only audit:")[0].trim() ?? "";
          byKey.set(entry.key, {
            key: entry.key,
            labelZh: existing?.labelZh ?? labelZh,
            labelEn: existing?.labelEn ?? labelEn,
            code: entry.propertyCode,
            scope: existing?.scope ?? (entry.key.startsWith("reservedPreset") ? "custom_slot" : "unknown"),
            status: existing?.status ?? (entry.key.startsWith("reservedPreset") ? "read_detected_unverified" : "blocked_unknown"),
            notes: `${previousNotes}${previousNotes ? "\n" : ""}Read-only audit: ${auditTimestamp}; ${observation}`,
          });
        }
        const properties = Array.from(byKey.values());
        const evidence = current.evidenceSummary.trim();
        return {
          ...current,
          model: audit.model,
          firmware: audit.firmware,
          usbIds: [audit.usbId],
          sourceKind: "local_probe",
          lastVerifiedAt: Date.now(),
          evidenceSummary: `${evidence}${evidence ? "\n\n" : ""}Read-only unverified-property audit ${auditTimestamp}: ${audit.properties.length} fixed-scope codes; no slot selection or writes.`.trim(),
          properties,
        };
      });
      setSelectedUsbId(usbId);
      setMessage(t("matrixAuditComplete"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    setMessage("");
    try {
      const saved = await saveLocalCapabilityMatrix({
        ...draft,
        usbIds: draft.usbIds.map((value) => value.trim()).filter(Boolean),
        customSlots: draft.customSlots.map((value) => value.trim()).filter(Boolean),
        sourceUrl: draft.sourceUrl?.trim() || undefined,
      });
      setDraft(saved);
      setMatrices((current) => [saved, ...current.filter((item) => item.id !== saved.id)]);
      setMessage(t("matrixSaved"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!draft.createdAt) {
      setDraft(blankMatrix(locale));
      return;
    }
    setBusy(true);
    try {
      await deleteLocalCapabilityMatrix(draft.id);
      const remaining = matrices.filter((matrix) => matrix.id !== draft.id);
      setMatrices(remaining);
      setDraft(remaining[0] ?? blankMatrix(locale));
      setMessage(t("matrixDeleted"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  if (!isDesktop) {
    return <section className="capability-database"><p>{t("matrixDesktopOnly")}</p></section>;
  }

  return (
    <section className="capability-database">
      <div className="capability-database-intro">
        <div>
          <p className="eyebrow">{t("capabilityDatabase")}</p>
          <h2>{t("capabilityDatabaseTitle")}</h2>
          <p>{t("capabilityDatabaseIntro")}</p>
        </div>
        <button className="primary" disabled={busy} onClick={() => setDraft(blankMatrix(locale))}>
          {t("matrixNew")}
        </button>
      </div>
      <p className="matrix-safety-note">{t("matrixSafety")}</p>
      <div className="matrix-layout">
        <aside className="matrix-records" aria-label={t("capabilityDatabase")}>
          {matrices.length ? matrices.map((matrix) => (
            <button
              key={matrix.id}
              className={matrix.id === draft.id ? "active" : ""}
              onClick={() => setDraft(matrix)}
            >
              <strong>{matrix.model}</strong>
              <small>{matrix.firmware} · {matrix.usbIds.join(", ")}</small>
            </button>
          )) : <p>{t("matrixNoRecords")}</p>}
        </aside>
        <div className="matrix-editor">
          <div className="matrix-identity-actions">
            <label>
              <span>{t("matrixConnectedCamera")}</span>
              <select value={selectedUsbId} onChange={(event) => setSelectedUsbId(event.target.value)} disabled={busy}>
                <option value="">{t("matrixSelectCamera")}</option>
                {cameraCandidates.map((device) => (
                  <option key={device.usbId} value={device.usbId}>{device.profile ?? device.product ?? device.usbId} · {device.usbId}</option>
                ))}
              </select>
            </label>
            <div className="matrix-identity-buttons">
              <button className="secondary" disabled={busy} onClick={scanAndReadIdentity}>{t("matrixReadIdentity")}</button>
              <button className="secondary" disabled={busy || !selectedUsbId} onClick={auditUnverifiedProperties}>{t("matrixAuditUnverified")}</button>
            </div>
          </div>
          <div className="matrix-identity-grid">
            <label><span>{t("matrixModel")}</span><input value={draft.model} disabled={busy} onChange={(event) => updateDraft({ model: event.target.value })} /></label>
            <label><span>{t("matrixFirmware")}</span><input value={draft.firmware} disabled={busy} onChange={(event) => updateDraft({ firmware: event.target.value })} /></label>
            <label><span>{t("matrixUsbIds")}</span><input value={draft.usbIds.join(", ")} disabled={busy} onChange={(event) => updateDraft({ usbIds: event.target.value.split(",") })} placeholder="04CB:030C" /></label>
            <label><span>{t("matrixSlots")}</span><input value={draft.customSlots.join(", ")} disabled={busy} onChange={(event) => updateDraft({ customSlots: event.target.value.split(",") })} placeholder="C1, C2, C3, C4" /></label>
            <label><span>{t("matrixSourceKind")}</span><select value={draft.sourceKind} disabled={busy} onChange={(event) => updateDraft({ sourceKind: event.target.value as LocalCapabilityMatrix["sourceKind"] })}>
              <option value="official">{t("matrixSourceOfficial")}</option>
              <option value="community">{t("matrixSourceCommunity")}</option>
              <option value="local_probe">{t("matrixSourceProbe")}</option>
            </select></label>
            <label className="matrix-source"><span>{t("matrixSource")}</span><input type="url" value={draft.sourceUrl ?? ""} disabled={busy} onChange={(event) => updateDraft({ sourceUrl: event.target.value })} placeholder="https://…" /></label>
            <label className="matrix-source matrix-evidence"><span>{t("matrixEvidence")}</span><textarea value={draft.evidenceSummary} disabled={busy} onChange={(event) => updateDraft({ evidenceSummary: event.target.value })} placeholder={t("matrixNotes")} /></label>
          </div>
          <div className="matrix-properties-heading">
            <div><h3>{t("matrixProperties")}</h3><p>{t("matrixPropertiesHelp")}</p></div>
            <button className="secondary" disabled={busy} onClick={() => updateDraft({ properties: [...draft.properties, blankProperty()] })}>{t("matrixAddProperty")}</button>
          </div>
          <div className="matrix-properties">
            {draft.properties.map((property, index) => (
              <article key={`${property.key}-${index}`}>
                <div className="matrix-property-labels">
                  <input aria-label={`${t("matrixPropertyKey")} ${index + 1}`} value={property.key} disabled={busy} onChange={(event) => updateProperty(index, { key: event.target.value })} placeholder="fieldKey" />
                  <input aria-label={`${t("matrixPropertyChinese")} ${index + 1}`} value={property.labelZh} disabled={busy} onChange={(event) => updateProperty(index, { labelZh: event.target.value })} placeholder={t("matrixPropertyChinese")} />
                  <input aria-label={`${t("matrixPropertyEnglish")} ${index + 1}`} value={property.labelEn} disabled={busy} onChange={(event) => updateProperty(index, { labelEn: event.target.value })} placeholder={t("matrixPropertyEnglish")} />
                </div>
                <div className="matrix-property-controls">
                  <input aria-label={`${t("property")} ${index + 1}`} value={property.code} disabled={busy} onChange={(event) => updateProperty(index, { code: event.target.value })} placeholder="PTP code (optional)" />
                  <select aria-label={`${t("matrixScope")} ${index + 1}`} value={property.scope} disabled={busy} onChange={(event) => updateProperty(index, { scope: event.target.value as LocalCapabilityMatrixProperty["scope"] })}>
                    <option value="custom_slot">{t("matrixScopeCustom")}</option>
                    <option value="global">{t("matrixScopeGlobal")}</option>
                    <option value="unknown">{t("matrixScopeUnknown")}</option>
                  </select>
                  <select aria-label={`${t("status")} ${index + 1}`} value={property.status} disabled={busy} onChange={(event) => updateProperty(index, { status: event.target.value as LocalCapabilityPropertyStatus })}>
                    {localStatuses.map((status) => <option key={status} value={status}>{formatStatus(locale, status)}</option>)}
                  </select>
                  <button className="danger" aria-label={`${t("delete")} ${property.labelEn}`} disabled={busy || draft.properties.length === 1} onClick={() => updateDraft({ properties: draft.properties.filter((_, propertyIndex) => propertyIndex !== index) })}>×</button>
                </div>
                <input className="matrix-property-notes" value={property.notes} disabled={busy} onChange={(event) => updateProperty(index, { notes: event.target.value })} placeholder={t("matrixNotes")} />
              </article>
            ))}
          </div>
          {message && <p className="probe-message" role="status">{message}</p>}
          <footer className="matrix-footer">
            <button className="danger" disabled={busy} onClick={remove}>{t("delete")}</button>
            <button className="primary" disabled={busy} onClick={save}>{t("matrixSave")}</button>
          </footer>
        </div>
      </div>
    </section>
  );
}
