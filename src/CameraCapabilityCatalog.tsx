import { useEffect, useMemo, useState } from "react";
import {
  getXm5CapabilityRecord,
  isDesktop,
  listCameraCapabilityCatalog,
  listLocalCapabilityMatrices,
  type CameraCapabilityCatalogEntry,
  type CameraCapabilityRecord,
  type LocalCapabilityMatrix,
} from "./lib/camera";
import type { CopyKey, Locale } from "./lib/i18n";
import { isSafeExternalUrl } from "./lib/externalUrl";

type Translator = (key: CopyKey) => string;

function accessLabel(locale: Locale, access: CameraCapabilityCatalogEntry["access"]) {
  const labels = locale === "zh-TW"
    ? {
        "Probe required": "需要唯讀探測",
        "Read-only PTP verified": "已驗證唯讀 PTP",
        "Experimental recipe writes": "實驗性配方寫入",
        "Recipe writes verified": "已驗證配方寫入",
      }
    : {
        "Probe required": "Probe required",
        "Read-only PTP verified": "Read-only PTP verified",
        "Experimental recipe writes": "Experimental Recipe writes",
        "Recipe writes verified": "Recipe writes verified",
      };
  return labels[access];
}

function statusLabel(locale: Locale, status: string) {
  const labels = locale === "zh-TW"
    ? {
        write_verified: "已驗證可寫入",
        write_rejected: "相機拒絕",
        read_detected_unverified: "已探測未驗證",
        blocked_unknown: "未知／鎖定",
      }
    : {
        write_verified: "Write verified",
        write_rejected: "Camera rejected",
        read_detected_unverified: "Detected, unverified",
        blocked_unknown: "Unknown / blocked",
      };
  return labels[status as keyof typeof labels] ?? status;
}

export function CameraCapabilityCatalog({ locale, t }: { locale: Locale; t: Translator }) {
  const [catalog, setCatalog] = useState<CameraCapabilityCatalogEntry[]>([]);
  const [trustedXm5, setTrustedXm5] = useState<CameraCapabilityRecord>();
  const [localMatrices, setLocalMatrices] = useState<LocalCapabilityMatrix[]>([]);
  const [selectedModel, setSelectedModel] = useState("X-M5");
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!isDesktop) return;
    void Promise.all([
      listCameraCapabilityCatalog(),
      getXm5CapabilityRecord(),
      listLocalCapabilityMatrices(),
    ])
      .then(([known, xm5, local]) => {
        setCatalog(known);
        setTrustedXm5(xm5);
        setLocalMatrices(local);
      })
      .catch(() => setError("load"));
  }, []);

  const visibleCatalog = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase(locale);
    return catalog.filter((entry) =>
      !normalized || `${entry.model} ${entry.family}`.toLocaleLowerCase(locale).includes(normalized),
    );
  }, [catalog, locale, query]);
  const selected = catalog.find((entry) => entry.model === selectedModel);
  const matricesForSelected = localMatrices.filter(
    (matrix) => matrix.model.trim().toLocaleUpperCase(locale) === selectedModel.toLocaleUpperCase(locale),
  );
  const propertyCounts = trustedXm5?.properties.reduce<Record<string, number>>((counts, property) => {
    counts[property.status] = (counts[property.status] ?? 0) + 1;
    return counts;
  }, {});

  if (!isDesktop) return <section className="capability-catalog"><p>{t("catalogDesktopOnly")}</p></section>;

  return (
    <section className="capability-catalog">
      <div className="catalog-intro">
        <div>
          <p className="eyebrow">{t("capabilityCatalog")}</p>
          <h2>{t("capabilityCatalogTitle")}</h2>
          <p>{t("capabilityCatalogIntro")}</p>
        </div>
      </div>
      <p className="matrix-safety-note">{t("catalogSafety")}</p>
      <input
        className="catalog-search"
        aria-label={t("catalogSearch")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder={t("catalogSearch")}
      />
      <div className="catalog-layout">
        <nav className="catalog-model-list" aria-label={t("capabilityCatalog")}>
          {visibleCatalog.map((entry) => (
            <button
              key={entry.model}
              className={entry.model === selectedModel ? "active" : ""}
              onClick={() => setSelectedModel(entry.model)}
            >
              <strong>{entry.model}</strong>
              <small>{entry.family}</small>
              <span className={entry.trustedRecordId ? "catalog-access trusted" : "catalog-access"}>
                {accessLabel(locale, entry.access)}
              </span>
            </button>
          ))}
          {!visibleCatalog.length && <p>{t("catalogNoMatches")}</p>}
        </nav>
        <article className="catalog-detail">
          {selected ? <>
            <header>
              <div>
                <p className="eyebrow">{selected.family}</p>
                <h2>{selected.model}</h2>
              </div>
              <span className={selected.trustedRecordId ? "catalog-access trusted" : "catalog-access"}>
                {accessLabel(locale, selected.access)}
              </span>
            </header>
            {selected.trustedRecordId && trustedXm5 ? <>
              <div className="catalog-identity">
                <div><span>{t("matrixFirmware")}</span><strong>{trustedXm5.firmware}</strong></div>
                <div><span>{t("matrixUsbIds")}</span><strong>{trustedXm5.usbIds.join(", ")}</strong></div>
                <div><span>{t("matrixSlots")}</span><strong>{trustedXm5.customSlots.join(", ")}</strong></div>
                <div><span>{t("catalogTrustedRecord")}</span><strong>{trustedXm5.recordId}</strong></div>
              </div>
              <div className="catalog-stat-grid">
                {Object.entries(propertyCounts ?? {}).map(([status, count]) => (
                  <div key={status} className={`capability-status ${status}`}>
                    <strong>{count}</strong><span>{statusLabel(locale, status)}</span>
                  </div>
                ))}
              </div>
              <h3>{t("catalogTrustedProperties")}</h3>
              <ul className="catalog-properties">
                {trustedXm5.properties.map((property) => (
                  <li key={property.key}>
                    <div><strong>{property.labelZh} <span lang="en">{property.labelEn}</span></strong><small>{property.code} · {property.scope}</small></div>
                    <span className={`capability-status ${property.status}`}>{statusLabel(locale, property.status)}</span>
                  </li>
                ))}
              </ul>
            </> : <p className="catalog-probe-only">{t("catalogProbeOnly")}</p>}
            <section className="catalog-local-matrices">
              <h3>{t("catalogLocalMatrices")}</h3>
              {matricesForSelected.length ? matricesForSelected.map((matrix) => (
                <article key={matrix.id}>
                  <header><div><strong>{matrix.firmware}</strong><small>{matrix.usbIds.join(", ")}</small></div><span>{matrix.properties.length} {t("properties")}</span></header>
                  {isSafeExternalUrl(matrix.sourceUrl) && <a href={matrix.sourceUrl} target="_blank" rel="noreferrer">{t("matrixSource")}</a>}
                  <small>{matrix.sourceKind === "official" ? t("matrixSourceOfficial") : matrix.sourceKind === "community" ? t("matrixSourceCommunity") : t("matrixSourceProbe")}</small>
                  {matrix.evidenceSummary && <p>{matrix.evidenceSummary}</p>}
                  <ul>{matrix.properties.map((property) => <li key={property.key}><span>{property.labelZh} <small lang="en">{property.labelEn}</small></span><em className={`capability-status ${property.status}`}>{statusLabel(locale, property.status)}</em></li>)}</ul>
                </article>
              )) : <p>{t("catalogNoLocalMatrix")}</p>}
            </section>
          </> : <p>{error ? t("catalogLoadFailed") : t("catalogSelectModel")}</p>}
        </article>
      </div>
    </section>
  );
}
