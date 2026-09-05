import {
  ChangeEvent,
  memo,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { SafeDialog } from "./SafeDialog";
import { CapabilityMatrixPanel } from "./CapabilityMatrixPanel";
import { CameraCapabilityCatalog } from "./CameraCapabilityCatalog";
import {
  afModes,
  colorSpaces,
  dRangePriorities,
  driveModes,
  dynamicRanges,
  filmSimulations,
  supportedFilmSimulations,
  focusModes,
  grainSizes,
  imageQualities,
  imageSizes,
  isoAutoMaximums,
  isoSensitivities,
  makeRecipe,
  meteringModes,
  onOff,
  portraitEnhancerLevels,
  rawRecordingOptions,
  shutterTypes,
  strengths,
  stillFormats,
  whiteBalances,
  xm5WriteBlockers,
  type Recipe,
  type RecipeSettings,
  type ShootingSettings,
} from "./domain/recipe";
import {
  captureXm5RawPresetSnapshot,
  clearXm5CustomSlot,
  deleteDesktopRecipe,
  discoverCameras,
  isDesktop,
  listCameraBackups,
  listPendingWriteJournals,
  loadDesktopRecipes,
  probeCameraDeviceInfo,
  readCameraSlotSelector,
  readXm5InstalledPresets,
  restoreXm5Backup,
  saveDesktopRecipes,
  selectCameraSlot,
  writeXm5Recipe,
  getXm5CapabilityRecord,
  importFujifilmImageRecipe,
  stageRafPreview,
  type CameraBackupSummary,
  type CameraCapabilityRecord,
  type CameraPropertyStatus,
  type CameraDiscovery,
  type InstalledPreset,
  type ImageRecipeImport,
  type RafPreviewStageResult,
  type PtpDeviceInfoResult,
  type PtpPropertyValueResult,
  type RecipeWriteResult,
  type RawPtpPresetSnapshot,
  type SlotSelectionResult,
  type WriteJournalSummary,
} from "./lib/camera";
import {
  localeOptions,
  formatOption,
  translate,
  type CopyKey,
  type Locale,
} from "./lib/i18n";
import {
  loadRecipes,
  normalizeRecipe,
  parseRecipeCollectionText,
  saveRecipes,
} from "./lib/recipeCodec";
import { exportFpProfile, parseFpProfile, type FpFormat } from "./lib/fpProfile";
import { isSafeExternalUrl } from "./lib/externalUrl";

type Page =
  | "library"
  | "camera"
  | "installed"
  | "import"
  | "capabilities"
  | "catalog";
type Translator = (key: CopyKey) => string;
const numericKeys = [
  "highlight",
  "shadow",
  "color",
  "sharpness",
  "highIsoNoiseReduction",
  "clarity",
] as const;
const numericLimits: Partial<
  Record<
    (typeof numericKeys)[number],
    { min: number; max: number; step?: number }
  >
> = {
  highlight: { min: -2, max: 4, step: 0.5 },
  shadow: { min: -2, max: 4, step: 0.5 },
  color: { min: -4, max: 4 },
  sharpness: { min: -4, max: 4 },
  highIsoNoiseReduction: { min: -4, max: 4 },
  clarity: { min: -5, max: 5 },
};
const monochromeFilmSimulations = new Set<RecipeSettings["filmSimulation"]>([
  "ACROS",
  "ACROS_YE",
  "ACROS_R",
  "ACROS_G",
  "MONOCHROME",
  "MONOCHROME_YE",
  "MONOCHROME_R",
  "MONOCHROME_G",
  "SEPIA",
]);
type DeletedRecipe = { recipe: Recipe; index: number; deletedAt: string };
type SlotAssignments = Record<string, string>;
const slotAssignmentsStorageKey = "fuji-recipe-manager/slot-assignments/v1";
const isoAutoMaximumOptions = isoAutoMaximums.map(String);

function formatExposureCompensation(value: number) {
  const thirds = Math.round(value * 3);
  if (Math.abs(value * 3 - thirds) < 0.01 && thirds % 3 !== 0) {
    const sign = thirds > 0 ? "+" : "−";
    return `${sign}${Math.abs(thirds) % 3}/3`;
  }
  return `${value > 0 ? "+" : ""}${value}`;
}

function loadSlotAssignments(): SlotAssignments {
  try {
    const stored = localStorage.getItem(slotAssignmentsStorageKey);
    const parsed = stored ? JSON.parse(stored) : {};
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as SlotAssignments)
      : {};
  } catch {
    return {};
  }
}

function slotAssignmentKey(usbId: string, slot: number) {
  return `${usbId}:C${slot}`;
}

function formatDateTime(locale: Locale, value: string | number | Date) {
  return new Intl.DateTimeFormat(locale === "zh-TW" ? "zh-TW" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function App() {
  const [recipes, setRecipes] = useState<Recipe[]>(loadRecipes);
  const [selectedId, setSelectedId] = useState(recipes[0]?.id ?? "");
  const [page, setPage] = useState<Page>("library");
  const [locale, setLocale] = useState<Locale>(
    localStorage.getItem("fuji-recipe-manager/locale") === "zh-TW"
      ? "zh-TW"
      : "en",
  );
  const [query, setQuery] = useState("");
  const [filmSimulationFilter, setFilmSimulationFilter] = useState("ALL");
  const [tagFilter, setTagFilter] = useState("ALL");
  const [favoriteFilter, setFavoriteFilter] = useState("ALL");
  const [assignmentFilter, setAssignmentFilter] = useState("ALL");
  const [cameraCompatibilityFilter, setCameraCompatibilityFilter] =
    useState("ALL");
  const [devices, setDevices] = useState<CameraDiscovery[]>([]);
  const [status, setStatus] = useState("");
  const [paste, setPaste] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const [sourceAuthor, setSourceAuthor] = useState("");
  const [imageImport, setImageImport] = useState<ImageRecipeImport>();
  const [rafPreviewStage, setRafPreviewStage] =
    useState<RafPreviewStageResult>();
  const [notice, setNotice] = useState("");
  const [deletedRecipe, setDeletedRecipe] = useState<DeletedRecipe>();
  const [importingRecipe, setImportingRecipe] = useState<Recipe>();
  const [backupRefresh, setBackupRefresh] = useState(0);
  const [pendingWriteJournals, setPendingWriteJournals] = useState<WriteJournalSummary[]>([]);
  const [slotAssignments, setSlotAssignments] =
    useState<SlotAssignments>(loadSlotAssignments);
  const [desktopLibraryReady, setDesktopLibraryReady] = useState(!isDesktop);
  const fileInput = useRef<HTMLInputElement>(null);
  const desktopSaveTimer = useRef<number>();
  const desktopSaveQueue = useRef(Promise.resolve());
  const pendingRecipeDeletes = useRef(new Map<string, string>());
  const selected =
    recipes.find((recipe) => recipe.id === selectedId) ?? recipes[0];
  const assignedSlotsByRecipe = useMemo(() => {
    const assignments: Record<string, string[]> = {};
    for (const [key, recipeId] of Object.entries(slotAssignments)) {
      const slot = key.split(":C")[1];
      if (slot) (assignments[recipeId] ??= []).push(slot);
    }
    return assignments;
  }, [slotAssignments]);
  const availableTags = useMemo(
    () =>
      Array.from(
        new Set(
          recipes.flatMap((recipe) =>
            recipe.tags.map((tag) => tag.trim()).filter(Boolean),
          ),
        ),
      ).sort((left, right) => left.localeCompare(right, locale)),
    [locale, recipes],
  );
  const availableCameraCompatibility = useMemo(
    () =>
      Array.from(
        new Set(
          recipes.flatMap((recipe) =>
            recipe.cameraCompatibility.map((camera) => camera.trim()).filter(Boolean),
          ),
        ),
      ).sort((left, right) => left.localeCompare(right, locale)),
    [locale, recipes],
  );
  // Always show the complete, verified set in the filter. A simulation that
  // does not occur in the local library simply produces an empty result.
  const availableFilmSimulations = supportedFilmSimulations;
  const visible = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase(locale);
    return recipes.filter((recipe) => {
      const searchable = [
        recipe.name,
        recipe.description,
        recipe.tags.join(" "),
        recipe.source.author,
        recipe.cameraCompatibility.join(" "),
      ]
        .join(" ")
        .toLocaleLowerCase(locale);
      const assigned = (assignedSlotsByRecipe[recipe.id] ?? []).length > 0;
      return (
        (!normalizedQuery || searchable.includes(normalizedQuery)) &&
        (filmSimulationFilter === "ALL" ||
          recipe.settings.filmSimulation === filmSimulationFilter) &&
        (tagFilter === "ALL" ||
          (tagFilter === "UNTAGGED"
            ? recipe.tags.length === 0
            : recipe.tags.some((tag) => tag.trim() === tagFilter))) &&
        (favoriteFilter === "ALL" || recipe.favorite) &&
        (assignmentFilter === "ALL" ||
          (assignmentFilter === "ASSIGNED" ? assigned : !assigned)) &&
        (cameraCompatibilityFilter === "ALL" ||
          recipe.cameraCompatibility.some(
            (camera) => camera.trim() === cameraCompatibilityFilter,
          ))
      );
    });
  }, [
    assignedSlotsByRecipe,
    assignmentFilter,
    cameraCompatibilityFilter,
    favoriteFilter,
    filmSimulationFilter,
    locale,
    query,
    recipes,
    tagFilter,
  ]);
  const hasActiveLibraryFilters =
    Boolean(query.trim()) ||
    filmSimulationFilter !== "ALL" ||
    tagFilter !== "ALL" ||
    favoriteFilter !== "ALL" ||
    assignmentFilter !== "ALL" ||
    cameraCompatibilityFilter !== "ALL";
  const t: Translator = (key) => translate(locale, key);
  const selectedVisible =
    selected && visible.some((recipe) => recipe.id === selected.id)
      ? selected
      : undefined;

  function clearLibraryFilters() {
    setQuery("");
    setFilmSimulationFilter("ALL");
    setTagFilter("ALL");
    setFavoriteFilter("ALL");
    setAssignmentFilter("ALL");
    setCameraCompatibilityFilter("ALL");
  }

  useEffect(() => {
    if (!desktopLibraryReady) return;
    if (!isDesktop) {
      saveRecipes(recipes);
      return;
    }
    window.clearTimeout(desktopSaveTimer.current);
    desktopSaveTimer.current = window.setTimeout(() => {
      const recipesToSave = recipes;
      const deletions = Array.from(pendingRecipeDeletes.current.entries());
      pendingRecipeDeletes.current.clear();
      desktopSaveQueue.current = desktopSaveQueue.current
        .catch(() => undefined)
        .then(async () => {
          await saveDesktopRecipes(recipesToSave);
          for (const [id, deletedAt] of deletions) {
            await deleteDesktopRecipe(id, deletedAt);
          }
        })
        .catch(() => setNotice(t("savingLibraryFailed")));
    }, 350);
    return () => window.clearTimeout(desktopSaveTimer.current);
  }, [desktopLibraryReady, recipes]);
  useEffect(() => {
    if (
      visible.length > 0 &&
      !visible.some((recipe) => recipe.id === selectedId)
    )
      setSelectedId(visible[0].id);
  }, [selectedId, visible]);
  useEffect(() => {
    localStorage.setItem("fuji-recipe-manager/locale", locale);
    document.documentElement.lang = locale;
  }, [locale]);
  useEffect(() => {
    localStorage.setItem(
      slotAssignmentsStorageKey,
      JSON.stringify(slotAssignments),
    );
  }, [slotAssignments]);
  useEffect(() => {
    if (!isDesktop) return;
    void (async () => {
      try {
        const stored = await loadDesktopRecipes();
        if (stored.length) {
          const normalized = stored.map(normalizeRecipe);
          setRecipes(normalized);
          setSelectedId(normalized[0]?.id ?? "");
        } else {
          // localStorage is a one-time migration source for desktop users.
          // SQLite becomes authoritative after this completes.
          await saveDesktopRecipes(recipes);
        }
      } catch {
        setNotice(t("databaseUnavailable"));
      } finally {
        setDesktopLibraryReady(true);
      }
    })();
  }, []);
  useEffect(() => {
    if (!isDesktop) return;
    void listPendingWriteJournals()
      .then((journals) => {
        setPendingWriteJournals(journals);
        if (journals.length) {
          setNotice(
            locale === "zh-TW"
              ? `偵測到 ${journals.length} 筆未完成的相機寫入；請到「相機」頁面檢查並回復。`
              : `${journals.length} incomplete camera write(s) need review. Open Camera to inspect and restore.`,
          );
        }
      })
      .catch(() => undefined);
  }, [backupRefresh, locale]);

  function updateRecipe(recipe: Recipe) {
    const updated = { ...recipe, updatedAt: new Date().toISOString() };
    setRecipes((items) =>
      items.map((item) => (item.id === updated.id ? updated : item)),
    );
  }
  function createRecipe() {
    const recipe = makeRecipe(
      locale === "zh-TW" ? "新的 Fujifilm Recipe" : "New Fujifilm Recipe",
    );
    setRecipes((items) => [recipe, ...items]);
    setSelectedId(recipe.id);
    setDeletedRecipe(undefined);
    setNotice(
      locale === "zh-TW"
        ? "已在本機建立新 Recipe。"
        : "New recipe created locally.",
    );
  }
  function removeRecipe() {
    if (!selected || recipes.length < 2) return;
    const index = recipes.findIndex((item) => item.id === selected.id);
    const remaining = recipes.filter((item) => item.id !== selected.id);
    setRecipes(remaining);
    setSelectedId(remaining[0].id);
    const deletedAt = new Date().toISOString();
    pendingRecipeDeletes.current.set(selected.id, deletedAt);
    setDeletedRecipe({ recipe: selected, index, deletedAt });
    setNotice(t("recipeDeleted"));
  }
  function undoDelete() {
    if (!deletedRecipe) return;
    const restoredRecipe = {
      ...deletedRecipe.recipe,
      // A delete can already be queued when Undo is clicked. Give the restore
      // a newer revision so the durable tombstone cannot suppress it.
      updatedAt: new Date().toISOString(),
    };
    setRecipes((items) => {
      const restored = [...items];
      restored.splice(deletedRecipe.index, 0, restoredRecipe);
      return restored;
    });
    setSelectedId(restoredRecipe.id);
    pendingRecipeDeletes.current.delete(deletedRecipe.recipe.id);
    setDeletedRecipe(undefined);
    setNotice(t("recipeRestored"));
  }
  async function scanCamera(): Promise<CameraDiscovery[]> {
    setStatus(
      locale === "zh-TW" ? "正在掃描 USB 裝置…" : "Scanning USB devices…",
    );
    try {
      const found = await discoverCameras();
      setDevices(found);
      setStatus(
        found.some((device) => device.isFujifilm)
          ? t("readOnlyDetected")
          : t("noFuji"),
      );
      return found;
    } catch {
      setStatus(t("scanUnavailable"));
      return [];
    }
  }
  function exportRecipe() {
    if (!selected) return;
    const blob = new Blob([JSON.stringify(selected, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download =
      (selected.name.replaceAll(/[^a-z0-9]+/gi, "-").toLowerCase() ||
        "recipe") + ".frecipe";
    link.click();
    URL.revokeObjectURL(url);
    setNotice(locale === "zh-TW" ? "Recipe 已匯出。" : "Recipe exported.");
  }
  function exportFpRecipe(format: FpFormat) {
    if (!selected) return;
    const { xml, report } = exportFpProfile(selected, format);
    const blob = new Blob([xml], { type: "application/xml" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download =
      (selected.name.replaceAll(/[^a-z0-9]+/gi, "-").toLowerCase() ||
        "recipe") +
      `.${format}`;
    link.click();
    URL.revokeObjectURL(url);
    const warning = report.warnings[0];
    setNotice(
      locale === "zh-TW"
        ? `${format} 已匯出。${warning ? ` 注意：${warning}` : ""}`
        : `${format} exported.${warning ? ` Note: ${warning}` : ""}`,
    );
  }
  function importRecipe(value = paste) {
    try {
      const imported = parseRecipeCollectionText(value, {
        url: sourceUrl,
        author: sourceAuthor,
      });
      setRecipes((items) => {
        const importedIds = new Set(imported.map((recipe) => recipe.id));
        return [...imported, ...items.filter((recipe) => !importedIds.has(recipe.id))];
      });
      setSelectedId(imported[0].id);
      setPaste("");
      setSourceUrl("");
      setSourceAuthor("");
      setPage("library");
      setNotice(
        locale === "zh-TW"
          ? `已匯入 ${imported.length} 組 Recipe。`
          : `${imported.length} Recipe(s) imported.`,
      );
    } catch {
      setNotice(
        locale === "zh-TW"
          ? "無法解析 Recipe。"
          : "Could not parse that recipe.",
      );
    }
  }
  function importFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    file
      .text()
      .then((text) => {
        if (/\.fp[123]$/i.test(file.name)) {
          const { recipe, report } = parseFpProfile(text, file.name);
          setRecipes((items) => [recipe, ...items]);
          setSelectedId(recipe.id);
          setPage("library");
          const preservation =
            report.unmappedProperties.length || report.omittedSensitiveProperties.length
              ? locale === "zh-TW"
                ? `；保留 ${report.unmappedProperties.length} 個未映射欄位，移除 ${report.omittedSensitiveProperties.length} 個敏感欄位。`
                : `; retained ${report.unmappedProperties.length} unmapped field(s) and removed ${report.omittedSensitiveProperties.length} sensitive field(s).`
              : "";
          setNotice(`${file.name} ${locale === "zh-TW" ? "已匯入" : "imported"}${preservation}`);
          return;
        }
        importRecipe(text);
      })
      .catch(() => setNotice(t("fileReadFailure")));
    event.target.value = "";
  }
  async function importImageRecipe() {
    if (!isDesktop) {
      setNotice(t("imageImportDesktopOnly"));
      return;
    }
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "Fujifilm image metadata", extensions: ["jpg", "jpeg", "raf"] },
        ],
      });
      if (!path || Array.isArray(path)) return;
      const imported = await importFujifilmImageRecipe(path);
      const base = makeRecipe(imported.fileName.replace(/\.[^.]+$/, "") || "EXIF Recipe");
      const settingsPatch = imported.settings as Partial<RecipeSettings>;
      const whiteBalancePatch =
        settingsPatch.whiteBalance as Partial<RecipeSettings["whiteBalance"]> | undefined;
      const shootingPatch = imported.shootingSettings as Partial<ShootingSettings>;
      const recipe = normalizeRecipe({
        ...base,
        cameraCompatibility: [imported.model],
        settings: {
          ...base.settings,
          ...settingsPatch,
          whiteBalance: {
            ...base.settings.whiteBalance,
            ...whiteBalancePatch,
          },
        },
        shootingSettings: { ...base.shootingSettings, ...shootingPatch },
      });
      setRecipes((items) => [recipe, ...items]);
      setSelectedId(recipe.id);
      setImageImport(imported);
      const recognized = imported.fieldStatuses.filter(
        (field) => field.status === "recognized",
      ).length;
      setNotice(
        locale === "zh-TW"
          ? `已由 ${imported.fileName} 建立 Recipe；已辨識 ${recognized} 個欄位。`
          : `Recipe created from ${imported.fileName}; ${recognized} field(s) recognized.`,
      );
    } catch (error) {
      setNotice(String(error));
    }
  }
  async function stageRafPreviewFile() {
    if (!isDesktop) {
      setNotice(t("rafPreviewDesktopOnly"));
      return;
    }
    if (!selected) return;
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Fujifilm RAF", extensions: ["raf"] }],
      });
      if (!path || Array.isArray(path)) return;
      const staged = await stageRafPreview(path, selected.id);
      setRafPreviewStage(staged);
      setNotice(t("rafPreviewStaged"));
    } catch (error) {
      setNotice(String(error));
    }
  }
  function assignRecipeToSlot(usbId: string, slot: number, recipeId: string) {
    setSlotAssignments((assignments) => ({
      ...assignments,
      [slotAssignmentKey(usbId, slot)]: recipeId,
    }));
  }
  function clearRecipeSlotAssignment(usbId: string, slot: number) {
    setSlotAssignments((assignments) => {
      const next = { ...assignments };
      delete next[slotAssignmentKey(usbId, slot)];
      return next;
    });
  }
  function syncInstalledPresetAssignments(
    usbId: string,
    presets: InstalledPreset[],
  ) {
    setSlotAssignments((assignments) => {
      const next = { ...assignments };
      for (const preset of presets) {
        const key = slotAssignmentKey(usbId, preset.slot);
        const matches = recipes.filter(
          (recipe) => recipe.name.trim() === preset.name.trim(),
        );
        if (preset.name && matches.length === 1) next[key] = matches[0].id;
        else delete next[key];
      }
      return next;
    });
  }

  const title =
    page === "library"
      ? t("buildLibrary")
      : page === "camera"
        ? t("connectCare")
        : page === "installed"
          ? t("installedTitle")
          : page === "capabilities"
            ? t("capabilityDatabaseTitle")
            : page === "catalog"
              ? t("capabilityCatalogTitle")
            : t("bringRecipe");
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">F</span>
          <span>
            FUJI
            <br />
            RECIPE
          </span>
        </div>
        <button
          className={"nav " + (page === "library" ? "active" : "")}
          aria-current={page === "library" ? "page" : undefined}
          onClick={() => setPage("library")}
        >
          {t("library")} <b>{recipes.length}</b>
        </button>
        <button
          className={"nav " + (page === "camera" ? "active" : "")}
          aria-current={page === "camera" ? "page" : undefined}
          onClick={() => setPage("camera")}
        >
          {t("camera")}{" "}
          <i
            aria-label={
              devices.some((device) => device.isFujifilm)
                ? t("cameraConnected")
                : t("cameraDisconnected")
            }
            className={
              devices.some((device) => device.isFujifilm) ? "connected" : ""
            }
          />
        </button>
        <button
          className={"nav " + (page === "installed" ? "active" : "")}
          aria-current={page === "installed" ? "page" : undefined}
          onClick={() => setPage("installed")}
        >
          {t("installed")}
        </button>
        <button
          className={"nav " + (page === "import" ? "active" : "")}
          aria-current={page === "import" ? "page" : undefined}
          onClick={() => setPage("import")}
        >
          {t("importRecipe")}
        </button>
        <button
          className={"nav " + (page === "capabilities" ? "active" : "")}
          aria-current={page === "capabilities" ? "page" : undefined}
          onClick={() => setPage("capabilities")}
        >
          {t("capabilityDatabase")}
        </button>
        <button
          className={"nav " + (page === "catalog" ? "active" : "")}
          aria-current={page === "catalog" ? "page" : undefined}
          onClick={() => setPage("catalog")}
        >
          {t("capabilityCatalog")}
        </button>
        <label className="language-picker">
          <span>{t("settings")}</span>
          <select
            value={locale}
            onChange={(event) => setLocale(event.target.value as Locale)}
          >
            {localeOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <small>{t("languageHelp")}</small>
        </label>
        <div className="sidebar-bottom">
          <span className="phase">PHASE 1 · {t("safeMode")}</span>
          <p>{t("recipesStayLocal")}</p>
        </div>
      </aside>
      <main className={`workspace workspace-${page}`}>
        <header>
          <div>
            <h1>{title}</h1>
          </div>
          {notice && (
            <div className="notice" role="status" aria-live="polite">
              <span>{notice}</span>
              {deletedRecipe && (
                <button className="notice-action" onClick={undoDelete}>
                  {t("undo")}
                </button>
              )}
              <button
                className="notice-dismiss"
                aria-label={t("dismissNotice")}
                onClick={() => {
                  setNotice("");
                  setDeletedRecipe(undefined);
                }}
              >
                ×
              </button>
            </div>
          )}
        </header>
        {page === "library" && (
          <section className="library-layout">
            <div className="recipe-list">
              <div className="list-actions">
                <input
                  aria-label={t("search")}
                  placeholder={t("search")}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
                <div className="library-filters" aria-label={t("filterRecipes")}>
                  <div className="filter-heading">
                    <span>{t("filterRecipes")}</span>
                    {hasActiveLibraryFilters && (
                      <button
                        className="filter-reset"
                        type="button"
                        onClick={clearLibraryFilters}
                      >
                        {t("clearFilters")}
                      </button>
                    )}
                  </div>
                  <label>
                    <span>{t("filterFilmSimulation")}</span>
                    <select
                      aria-label={t("filterFilmSimulation")}
                      value={filmSimulationFilter}
                      onChange={(event) =>
                        setFilmSimulationFilter(event.target.value)
                      }
                    >
                      <option value="ALL">{t("allFilmSimulations")}</option>
                      {availableFilmSimulations.map((filmSimulation) => (
                        <option key={filmSimulation} value={filmSimulation}>
                          {formatOption(locale, filmSimulation)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t("filterTag")}</span>
                    <select
                      aria-label={t("filterTag")}
                      value={tagFilter}
                      onChange={(event) => setTagFilter(event.target.value)}
                    >
                      <option value="ALL">{t("allTags")}</option>
                      <option value="UNTAGGED">{t("untaged")}</option>
                      {availableTags.map((tag) => (
                        <option key={tag} value={tag}>
                          {tag}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t("filterFavorites")}</span>
                    <select
                      aria-label={t("filterFavorites")}
                      value={favoriteFilter}
                      onChange={(event) =>
                        setFavoriteFilter(event.target.value)
                      }
                    >
                      <option value="ALL">{t("allRecipes")}</option>
                      <option value="FAVORITES">{t("favoritesOnly")}</option>
                    </select>
                  </label>
                  <label>
                    <span>{t("filterAssignment")}</span>
                    <select
                      aria-label={t("filterAssignment")}
                      value={assignmentFilter}
                      onChange={(event) =>
                        setAssignmentFilter(event.target.value)
                      }
                    >
                      <option value="ALL">{t("allAssignments")}</option>
                      <option value="ASSIGNED">{t("assignedToSlot")}</option>
                      <option value="UNASSIGNED">{t("notAssignedToSlot")}</option>
                    </select>
                  </label>
                  <label>
                    <span>{t("filterCameraCompatibility")}</span>
                    <select
                      aria-label={t("filterCameraCompatibility")}
                      value={cameraCompatibilityFilter}
                      onChange={(event) =>
                        setCameraCompatibilityFilter(event.target.value)
                      }
                    >
                      <option value="ALL">{t("allCameras")}</option>
                      {availableCameraCompatibility.map((camera) => (
                        <option key={camera} value={camera}>
                          {camera}
                        </option>
                      ))}
                    </select>
                  </label>
                  <p className="filter-summary" aria-live="polite">
                    {visible.length} / {recipes.length} {t("recipes")}
                  </p>
                </div>
                <button className="primary" onClick={createRecipe}>
                  {t("newRecipe")}
                </button>
                <button className="secondary" onClick={() => setPage("import")}>
                  {t("importRecipe")}
                </button>
              </div>
              <div className="cards">
                {visible.length ? (
                  visible.map((recipe) => (
                    <RecipeCard
                      key={recipe.id}
                      recipe={recipe}
                      selected={recipe.id === selectedVisible?.id}
                      assignedSlots={assignedSlotsByRecipe[recipe.id] ?? []}
                      locale={locale}
                      untagged={t("untaged")}
                      onSelect={setSelectedId}
                    />
                  ))
                ) : (
                  <div className="empty-library" role="status">
                    <strong>{t("noRecipeResults")}</strong>
                    <button className="secondary" onClick={clearLibraryFilters}>
                      {t("clearFilters")}
                    </button>
                  </div>
                )}
              </div>
            </div>
            {selectedVisible && (
              <RecipeEditor
                recipe={selectedVisible}
                t={t}
                formatOption={(value) => formatOption(locale, value)}
                onChange={updateRecipe}
                onExport={exportRecipe}
                onExportFp={exportFpRecipe}
                onDelete={removeRecipe}
                onImport={() => setImportingRecipe(selected)}
              />
            )}
          </section>
        )}
        {page === "camera" && (
          <CameraPanel
            devices={devices}
            status={status || t("noCamera")}
            onScan={scanCamera}
            refreshKey={backupRefresh}
            pendingWriteJournals={pendingWriteJournals}
            onRecoveryUpdated={() => setBackupRefresh((value) => value + 1)}
            locale={locale}
            t={t}
          />
        )}
        {page === "installed" && (
          <InstalledPresetsPanel
            devices={devices}
            onScan={scanCamera}
            onPresetsRead={syncInstalledPresetAssignments}
            onSlotCleared={clearRecipeSlotAssignment}
            t={t}
          />
        )}
        {page === "capabilities" && (
          <CapabilityMatrixPanel
            devices={devices}
            locale={locale}
            onScan={scanCamera}
            t={t}
          />
        )}
        {page === "catalog" && (
          <CameraCapabilityCatalog locale={locale} t={t} />
        )}
        {page === "import" && (
          <section className="import-panel">
            <p>{t("importHelp")}</p>
            <div className="source-fields">
              <label>
                {t("sourceUrl")}
                <input
                  type="url"
                  value={sourceUrl}
                  onChange={(event) => setSourceUrl(event.target.value)}
                  placeholder="https://example.com/recipe"
                />
              </label>
              <label>
                {t("sourceAuthor")}
                <input
                  value={sourceAuthor}
                  onChange={(event) => setSourceAuthor(event.target.value)}
                  placeholder={t("optional")}
                />
              </label>
            </div>
            <p className="import-note">{t("sourceNote")}</p>
            <textarea
              value={paste}
              onChange={(event) => setPaste(event.target.value)}
              placeholder="Film Simulation: Classic Chrome"
            />
            <div className="import-actions">
              <button
                className="secondary"
                onClick={() => fileInput.current?.click()}
              >
                {t("chooseFile")}
              </button>
              <input
                ref={fileInput}
                hidden
                type="file"
                accept=".frecipe,.json,.txt,.fp1,.fp2,.fp3,application/json,text/plain,application/xml"
                onChange={importFile}
              />
              <button
                className="secondary"
                disabled={!isDesktop}
                onClick={() => void importImageRecipe()}
              >
                {t("importImageRecipe")}
              </button>
              <button
                className="secondary"
                disabled={!isDesktop || !selected}
                onClick={() => void stageRafPreviewFile()}
              >
                {t("stageRafPreview")}
              </button>
              <button className="primary" onClick={() => importRecipe()}>
                {t("parseSave")}
              </button>
            </div>
            {imageImport && (
              <section className="image-import-summary">
                <strong>{t("imageImportSummary")}</strong>
                <p>
                  {imageImport.fileName} · {imageImport.model}
                </p>
                <ul>
                  {imageImport.fieldStatuses.map((field) => (
                    <li key={field.key}>
                      <span>{field.key}</span>
                      <em className={field.status}>{
                        field.status === "recognized"
                          ? t("imageFieldRecognized")
                          : t("imageFieldUnavailable")
                      }</em>
                      <small>{field.detail}</small>
                    </li>
                  ))}
                </ul>
                {imageImport.warnings.map((warning) => (
                  <small key={warning}>{warning}</small>
                ))}
              </section>
            )}
            {rafPreviewStage && (
              <section className="image-import-summary">
                <strong>{t("rafPreviewStatus")}</strong>
                <p>{rafPreviewStage.state}</p>
                {rafPreviewStage.recoveryAction && (
                  <small>{rafPreviewStage.recoveryAction}</small>
                )}
              </section>
            )}
          </section>
        )}
        {importingRecipe && (
          <CameraImportDialog
            recipe={importingRecipe}
            devices={devices}
            locale={locale}
            t={t}
            onScan={scanCamera}
            onClose={() => setImportingRecipe(undefined)}
            onSnapshot={(snapshot) => {
              updateRecipe({
                ...importingRecipe,
                interoperability: {
                  ...importingRecipe.interoperability,
                  rawPtpPresetSnapshot: snapshot,
                },
              });
              setNotice(
                `${t("rawSnapshotCaptured")}: C${snapshot.slot} · ${snapshot.properties.length} ${t("rawSnapshotProperties")}.`,
              );
            }}
            onCompleted={(result) => {
              setBackupRefresh((value) => value + 1);
              assignRecipeToSlot(
                result.usbId,
                result.slot,
                importingRecipe.id,
              );
              setNotice(
                `C${result.slot} “${result.presetName}” — ${t("importCompleted")}: ${result.backupId}.`,
              );
              setImportingRecipe(undefined);
            }}
          />
        )}
      </main>
    </div>
  );
}

const RecipeCard = memo(function RecipeCard({
  recipe,
  selected,
  assignedSlots,
  locale,
  untagged,
  onSelect,
}: {
  recipe: Recipe;
  selected: boolean;
  assignedSlots: string[];
  locale: Locale;
  untagged: string;
  onSelect: (recipeId: string) => void;
}) {
  return (
    <button
      className={"recipe-card " + (selected ? "selected" : "")}
      onClick={() => onSelect(recipe.id)}
    >
      <span>{recipe.favorite ? "★" : "☆"}</span>
      <div className="recipe-card-title">
        <strong>{recipe.name}</strong>
        {assignedSlots.map((slot) => (
          <span key={slot} className="slot-assignment">
            C{slot}
          </span>
        ))}
      </div>
      <small>
        {formatOption(locale, recipe.settings.filmSimulation)} ·{" "}
        {recipe.tags.join(" / ") || untagged}
      </small>
    </button>
  );
});

function RecipeEditor({
  recipe,
  t,
  formatOption,
  onChange,
  onExport,
  onExportFp,
  onDelete,
  onImport,
}: {
  recipe: Recipe;
  t: Translator;
  formatOption: (value: string) => string;
  onChange: (recipe: Recipe) => void;
  onExport: () => void;
  onExportFp: (format: FpFormat) => void;
  onDelete: () => void;
  onImport: () => void;
}) {
  const update = <K extends keyof Recipe>(key: K, value: Recipe[K]) =>
    onChange({ ...recipe, [key]: value });
  const setting = <K extends keyof RecipeSettings>(
    key: K,
    value: RecipeSettings[K],
  ) => onChange({ ...recipe, settings: { ...recipe.settings, [key]: value } });
  const shootingSetting = <K extends keyof ShootingSettings>(
    key: K,
    value: ShootingSettings[K],
  ) =>
    onChange({
      ...recipe,
      shootingSettings: { ...recipe.shootingSettings, [key]: value },
    });
  const selectedIso = Number(
    recipe.shootingSettings.isoSensitivity.replace("ISO_", ""),
  );
  const dynamicRangeNeedsHigherIso =
    (recipe.settings.dynamicRange === "DR200" && selectedIso > 0 && selectedIso < 320) ||
    (recipe.settings.dynamicRange === "DR400" && selectedIso > 0 && selectedIso < 640);
  const settingWarnings = [
    recipe.settings.stillFormat === "HEIF" &&
      (recipe.settings.clarity !== 0 || recipe.settings.colorSpace !== "SRGB")
      ? t("heifDependency")
      : undefined,
    recipe.settings.dRangePriority !== "OFF"
      ? t("dRangePriorityDependency")
      : undefined,
    dynamicRangeNeedsHigherIso ? t("dynamicRangeIsoDependency") : undefined,
  ].filter((warning): warning is string => Boolean(warning));
  return (
    <article className="editor">
      <div className="editor-title">
        <div>
          <input
            className="recipe-name"
            aria-label={t("recipeName")}
            value={recipe.name}
            onChange={(event) => update("name", event.target.value)}
          />
          <input
            className="description"
            aria-label={t("description")}
            value={recipe.description}
            placeholder={t("description")}
            onChange={(event) => update("description", event.target.value)}
          />
        </div>
        <button
          className={"favorite " + (recipe.favorite ? "is-favorite" : "")}
          aria-label={
            recipe.favorite ? t("unfavoriteRecipe") : t("favoriteRecipe")
          }
          aria-pressed={recipe.favorite}
          onClick={() => update("favorite", !recipe.favorite)}
        >
          {recipe.favorite ? "★" : "☆"}
        </button>
      </div>
      <div className="tag-row">
        <label>
          {t("tags")}{" "}
          <input
            value={recipe.tags.join(", ")}
            placeholder="Portrait, Daylight"
            onChange={(event) =>
              update(
                "tags",
                event.target.value
                  .split(",")
                  .map((tag) => tag.trim())
                  .filter(Boolean),
              )
            }
          />
        </label>
        <span className="compatibility">✓ {t("compatible")}</span>
      </div>
      <div className="source-row">
        <label>
          {t("sourceUrl")}
          <input
            type="url"
            value={recipe.source.url}
            placeholder="https://…"
            onChange={(event) =>
              update("source", { ...recipe.source, url: event.target.value })
            }
          />
        </label>
        <label>
          {t("sourceAuthor")}
          <input
            value={recipe.source.author}
            placeholder={t("optional")}
            onChange={(event) =>
              update("source", { ...recipe.source, author: event.target.value })
            }
          />
        </label>
        {isSafeExternalUrl(recipe.source.url) && (
          <a href={recipe.source.url} target="_blank" rel="noreferrer">
            {t("openSource")}
          </a>
        )}
        {recipe.source.url && !isSafeExternalUrl(recipe.source.url) && (
          <small className="field-error">{t("sourceUrlInvalid")}</small>
        )}
      </div>
      <section className="recipe-settings-section">
        <div className="settings-section-heading">
          <p className="eyebrow">{t("creativeRecipeSettings")}</p>
          <p>{t("creativeRecipeSettingsHelp")}</p>
        </div>
        <div className="settings-grid">
          <Select
            label={t("filmSimulation")}
            value={recipe.settings.filmSimulation}
            options={filmSimulations}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "filmSimulation",
                value as RecipeSettings["filmSimulation"],
              )
            }
          />
          {monochromeFilmSimulations.has(recipe.settings.filmSimulation) && (
            <>
              <NumberControl
                label={t("monochromaticWarmCool")}
                value={recipe.settings.monochromaticColor.warmCool}
                min={-18}
                max={18}
                t={t}
                onChange={(warmCool) =>
                  setting("monochromaticColor", {
                    ...recipe.settings.monochromaticColor,
                    warmCool,
                  })
                }
              />
              <NumberControl
                label={t("monochromaticMagentaGreen")}
                value={recipe.settings.monochromaticColor.magentaGreen}
                min={-18}
                max={18}
                t={t}
                onChange={(magentaGreen) =>
                  setting("monochromaticColor", {
                    ...recipe.settings.monochromaticColor,
                    magentaGreen,
                  })
                }
              />
              <small className="field-error mono-write-lock">
                {t("monochromaticWriteLocked")}
              </small>
            </>
          )}
          <Select
            label={t("dynamicRange")}
            value={recipe.settings.dynamicRange}
            options={dynamicRanges}
            formatOption={formatOption}
            onChange={(value) =>
              setting("dynamicRange", value as RecipeSettings["dynamicRange"])
            }
          />
          <Select
            label={t("whiteBalance")}
            value={recipe.settings.whiteBalance.mode}
            options={whiteBalances}
            formatOption={formatOption}
            onChange={(value) =>
              setting("whiteBalance", {
                ...recipe.settings.whiteBalance,
                mode: value as RecipeSettings["whiteBalance"]["mode"],
              })
            }
          />
          {recipe.settings.whiteBalance.mode === "COLOR_TEMPERATURE" ? (
            <NumberControl
              label={t("colorTemperature")}
              value={recipe.settings.whiteBalance.colorTemperatureK}
              min={2500}
              max={10000}
              step={10}
              t={t}
              onChange={(colorTemperatureK) =>
                setting("whiteBalance", {
                  ...recipe.settings.whiteBalance,
                  colorTemperatureK,
                })
              }
            />
          ) : null}
          <NumberPair
            label={t("wbShift")}
            first="R"
            second="B"
            a={recipe.settings.whiteBalance.shiftR}
            b={recipe.settings.whiteBalance.shiftB}
            t={t}
            min={-9}
            max={9}
            onChange={(a, b) =>
              setting("whiteBalance", {
                ...recipe.settings.whiteBalance,
                shiftR: a,
                shiftB: b,
              })
            }
          />
          <Select
            label={t("grainStrength")}
            value={recipe.settings.grain.strength}
            options={strengths}
            formatOption={formatOption}
            onChange={(value) =>
              setting("grain", {
                ...recipe.settings.grain,
                strength: value as RecipeSettings["grain"]["strength"],
              })
            }
          />
          <Select
            label={t("grainSize")}
            value={recipe.settings.grain.size}
            options={grainSizes}
            formatOption={formatOption}
            onChange={(value) =>
              setting("grain", {
                ...recipe.settings.grain,
                size: value as RecipeSettings["grain"]["size"],
              })
            }
          />
          <Select
            label={t("colorChrome")}
            value={recipe.settings.colorChromeEffect}
            options={strengths}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "colorChromeEffect",
                value as RecipeSettings["colorChromeEffect"],
              )
            }
          />
          <Select
            label={t("colorChromeBlue")}
            value={recipe.settings.colorChromeFxBlue}
            options={strengths}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "colorChromeFxBlue",
                value as RecipeSettings["colorChromeFxBlue"],
              )
            }
          />
          <Select
            label={t("smoothSkinEffect")}
            value={recipe.settings.smoothSkinEffect}
            options={strengths}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "smoothSkinEffect",
                value as RecipeSettings["smoothSkinEffect"],
              )
            }
          />
          {numericKeys.map((key) => (
            <NumberControl
              key={key}
              label={
                key === "highlight"
                  ? t("highlightTone")
                  : key === "shadow"
                    ? t("shadowTone")
                    : key === "color"
                      ? t("color")
                      : key === "sharpness"
                        ? t("sharpness")
                        : key === "highIsoNoiseReduction"
                          ? t("highIsoNr")
                          : t("clarity")
              }
              value={recipe.settings[key]}
              t={t}
              {...numericLimits[key]}
              onChange={(value) => setting(key, value)}
            />
          ))}
        </div>
      </section>
      <section className="recipe-settings-section capture-settings-section">
        <div className="settings-section-heading">
          <p className="eyebrow">{t("cameraCaptureSettings")}</p>
          <p>{t("cameraCaptureSettingsHelp")}</p>
        </div>
        {settingWarnings.length > 0 && (
          <aside className="setting-warnings" aria-live="polite">
            <strong>{t("settingDependencyTitle")}</strong>
            <ul>
              {settingWarnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </aside>
        )}
        <div className="settings-grid">
          <Select
            label={t("dRangePriority")}
            value={recipe.settings.dRangePriority}
            options={dRangePriorities}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "dRangePriority",
                value as RecipeSettings["dRangePriority"],
              )
            }
          />
          <Select
            label={t("portraitEnhancer")}
            value={recipe.settings.portraitEnhancer}
            options={portraitEnhancerLevels}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "portraitEnhancer",
                value as RecipeSettings["portraitEnhancer"],
              )
            }
          />
          <Select
            label={t("longExposureNr")}
            value={recipe.settings.longExposureNoiseReduction}
            options={onOff}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "longExposureNoiseReduction",
                value as RecipeSettings["longExposureNoiseReduction"],
              )
            }
          />
          <Select
            label={t("lensModulationOptimizer")}
            value={recipe.settings.lensModulationOptimizer}
            options={onOff}
            formatOption={formatOption}
            onChange={(value) =>
              setting(
                "lensModulationOptimizer",
                value as RecipeSettings["lensModulationOptimizer"],
              )
            }
          />
          <Select
            label={t("colorSpace")}
            value={recipe.settings.colorSpace}
            options={colorSpaces}
            formatOption={formatOption}
            onChange={(value) =>
              setting("colorSpace", value as RecipeSettings["colorSpace"])
            }
          />
          <Select
            label={t("imageSize")}
            value={recipe.settings.imageSize}
            options={imageSizes}
            formatOption={formatOption}
            onChange={(value) =>
              setting("imageSize", value as RecipeSettings["imageSize"])
            }
          />
          <Select
            label={t("imageQuality")}
            value={recipe.settings.imageQuality}
            options={imageQualities}
            formatOption={formatOption}
            onChange={(value) =>
              setting("imageQuality", value as RecipeSettings["imageQuality"])
            }
          />
          <Select
            label={t("rawRecording")}
            value={recipe.settings.rawRecording}
            options={rawRecordingOptions}
            formatOption={formatOption}
            onChange={(value) =>
              setting("rawRecording", value as RecipeSettings["rawRecording"])
            }
          />
          <Select
            label={t("stillFormat")}
            value={recipe.settings.stillFormat}
            options={stillFormats}
            formatOption={formatOption}
            onChange={(value) =>
              setting("stillFormat", value as RecipeSettings["stillFormat"])
            }
          />
        </div>
      </section>
      <section className="recipe-settings-section shooting-settings-section">
        <div className="settings-section-heading">
          <p className="eyebrow">{t("shootingSettings")}</p>
          <p>{t("shootingSettingsHelp")}</p>
        </div>
        <div className="settings-grid">
          <Select
            label={t("isoSensitivity")}
            value={recipe.shootingSettings.isoSensitivity}
            options={isoSensitivities}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting(
                "isoSensitivity",
                value as ShootingSettings["isoSensitivity"],
              )
            }
          />
          <Select
            label={t("isoAutoMaximum")}
            value={String(recipe.shootingSettings.isoAutoMaximum)}
            options={isoAutoMaximumOptions}
            formatOption={(value) => `ISO ${value}`}
            onChange={(value) =>
              shootingSetting(
                "isoAutoMaximum",
                Number(value) as ShootingSettings["isoAutoMaximum"],
              )
            }
          />
          <NumberControl
            label={t("exposureCompensation")}
            value={recipe.shootingSettings.exposureCompensation}
            min={-5}
            max={5}
            step={1 / 3}
            formatValue={formatExposureCompensation}
            t={t}
            onChange={(value) => shootingSetting("exposureCompensation", value)}
          />
          <Select
            label={t("meteringMode")}
            value={recipe.shootingSettings.meteringMode}
            options={meteringModes}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting(
                "meteringMode",
                value as ShootingSettings["meteringMode"],
              )
            }
          />
          <Select
            label={t("focusMode")}
            value={recipe.shootingSettings.focusMode}
            options={focusModes}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting(
                "focusMode",
                value as ShootingSettings["focusMode"],
              )
            }
          />
          <Select
            label={t("afMode")}
            value={recipe.shootingSettings.afMode}
            options={afModes}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting("afMode", value as ShootingSettings["afMode"])
            }
          />
          <Select
            label={t("driveMode")}
            value={recipe.shootingSettings.driveMode}
            options={driveModes}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting(
                "driveMode",
                value as ShootingSettings["driveMode"],
              )
            }
          />
          <Select
            label={t("shutterType")}
            value={recipe.shootingSettings.shutterType}
            options={shutterTypes}
            formatOption={formatOption}
            onChange={(value) =>
              shootingSetting(
                "shutterType",
                value as ShootingSettings["shutterType"],
              )
            }
          />
        </div>
      </section>
      <footer className="editor-actions">
        <button className="secondary danger" onClick={onDelete}>
          {t("deleteRecipe")}
        </button>
        <button className="secondary" onClick={onExport}>
          {t("exportJson")}
        </button>
        <button className="secondary" onClick={() => onExportFp("FP1")}>
          {t("exportFp1")}
        </button>
        <button className="secondary" onClick={() => onExportFp("FP2")}>
          {t("exportFp2")}
        </button>
        <button className="secondary" onClick={() => onExportFp("FP3")}>
          {t("exportFp3")}
        </button>
        <button className="primary" onClick={onImport}>
          {t("importToCamera")}
        </button>
      </footer>
    </article>
  );
}

function Select({
  label,
  value,
  options,
  formatOption,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly string[];
  formatOption: (value: string) => string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((option) => (
          <option key={option} value={option}>
            {formatOption(option)}
          </option>
        ))}
      </select>
    </label>
  );
}
function NumberControl({
  label,
  value,
  min,
  max,
  step = 1,
  formatValue,
  t,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  formatValue?: (value: number) => string;
  t: Translator;
  onChange: (value: number) => void;
}) {
  const adjust = (direction: -1 | 1) => {
    const next = Math.round((value + direction * step) * 100) / 100;
    onChange(
      Math.min(
        max ?? Number.POSITIVE_INFINITY,
        Math.max(min ?? Number.NEGATIVE_INFINITY, next),
      ),
    );
  };
  return (
    <label className="field">
      <span>{label}</span>
      <div className="stepper">
        <button
          aria-label={`${t("decrease")} ${label}`}
          disabled={min !== undefined && value <= min}
          onClick={() => adjust(-1)}
        >
          −
        </button>
        <output aria-live="polite">
          {formatValue ? formatValue(value) : `${value > 0 ? "+" : ""}${value}`}
        </output>
        <button
          aria-label={`${t("increase")} ${label}`}
          disabled={max !== undefined && value >= max}
          onClick={() => adjust(1)}
        >
          +
        </button>
      </div>
    </label>
  );
}
function NumberPair({
  label,
  first,
  second,
  a,
  b,
  t,
  min,
  max,
  onChange,
}: {
  label: string;
  first: string;
  second: string;
  a: number;
  b: number;
  t: Translator;
  min?: number;
  max?: number;
  onChange: (a: number, b: number) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <div className="pair">
        <button
          aria-label={`${t("decrease")} ${label} ${first}`}
          disabled={min !== undefined && a <= min}
          onClick={() => onChange(a - 1, b)}
        >
          {first}−
        </button>
        <output aria-live="polite">
          {first}
          {a >= 0 ? "+" : ""}
          {a}
        </output>
        <button
          aria-label={`${t("increase")} ${label} ${first}`}
          disabled={max !== undefined && a >= max}
          onClick={() => onChange(a + 1, b)}
        >
          {first}+
        </button>
        <button
          aria-label={`${t("decrease")} ${label} ${second}`}
          disabled={min !== undefined && b <= min}
          onClick={() => onChange(a, b - 1)}
        >
          {second}−
        </button>
        <output aria-live="polite">
          {second}
          {b >= 0 ? "+" : ""}
          {b}
        </output>
        <button
          aria-label={`${t("increase")} ${label} ${second}`}
          disabled={max !== undefined && b >= max}
          onClick={() => onChange(a, b + 1)}
        >
          {second}+
        </button>
      </div>
    </label>
  );
}

function CameraImportDialog({
  recipe,
  devices,
  locale,
  t,
  onScan,
  onClose,
  onSnapshot,
  onCompleted,
}: {
  recipe: Recipe;
  devices: CameraDiscovery[];
  locale: Locale;
  t: Translator;
  onScan: () => Promise<CameraDiscovery[]>;
  onClose: () => void;
  onSnapshot: (snapshot: RawPtpPresetSnapshot) => void;
  onCompleted: (result: RecipeWriteResult) => void;
}) {
  const [slot, setSlot] = useState<number>();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [cameraName, setCameraName] = useState(recipe.name.slice(0, 31));
  const [capability, setCapability] = useState<CameraCapabilityRecord>();
  const [capabilityError, setCapabilityError] = useState("");
  const [rawSnapshot, setRawSnapshot] = useState<RawPtpPresetSnapshot>();
  const [deviceInfo, setDeviceInfo] = useState<PtpDeviceInfoResult>();
  const writeBlockers = xm5WriteBlockers(recipe.settings);
  const cropImageSizeSelected = recipe.settings.imageSize.endsWith("_1_25X_CROP");
  const cameraNameIsVerified = /^[\x20-\x7E]+$/.test(cameraName.trim());
  const camera = devices.find(
    (device) =>
      device.isFujifilm && device.ptpInterfaceDetected && device.writeEnabled,
  );

  useEffect(() => {
    if (!isDesktop) return;
    getXm5CapabilityRecord()
      .then(setCapability)
      .catch(() => setCapabilityError("Unable to load the local capability record."));
  }, []);

  async function scan() {
    setBusy(true);
    setMessage(t("scanUsb") + "…");
    try {
      await onScan();
      setDeviceInfo(undefined);
      setMessage("");
    } finally {
      setBusy(false);
    }
  }

  async function verifyCameraIdentity() {
    if (!camera) return;
    setBusy(true);
    setMessage(t("verifyWorking"));
    try {
      const identity = await probeCameraDeviceInfo(camera.usbId);
      setDeviceInfo(identity);
      setMessage(
        identity.capabilityStateKey === "exact_experimental"
          ? t("writeIdentityVerified")
          : identity.capabilityNextAction,
      );
    } catch (error) {
      const detail = String(error);
      setMessage(detail.includes("exclusive access") ? t("ptpBusy") : detail);
    } finally {
      setBusy(false);
    }
  }
  const writeIdentityVerified =
    deviceInfo?.capabilityStateKey === "exact_experimental";

  async function confirmImport() {
    if (!camera || !slot || !writeIdentityVerified) return;
    setBusy(true);
    setMessage(`${t("importWorking")} C${slot}…`);
    try {
      const result = await writeXm5Recipe(
        camera.usbId,
        slot,
        recipe.settings,
        cameraName,
      );
      onCompleted(result);
    } catch (error) {
      const detail = String(error);
      setMessage(detail.includes("exclusive access") ? t("ptpBusy") : detail);
    } finally {
      setBusy(false);
    }
  }

  async function captureRawSnapshot() {
    if (!camera || !slot || !writeIdentityVerified) return;
    setBusy(true);
    setMessage(`${t("captureRawSnapshot")} C${slot}…`);
    try {
      const snapshot = await captureXm5RawPresetSnapshot(camera.usbId, slot);
      setRawSnapshot(snapshot);
      onSnapshot(snapshot);
      setMessage(
        `${t("rawSnapshotCaptured")}: ${snapshot.properties.length} ${t("rawSnapshotProperties")}.`,
      );
    } catch (error) {
      const detail = String(error);
      setMessage(detail.includes("exclusive access") ? t("ptpBusy") : detail);
    } finally {
      setBusy(false);
    }
  }

  return (
    <SafeDialog
      labelledBy="camera-import-title"
      onClose={onClose}
      closeDisabled={busy}
    >
        <header>
          <div>
            <p className="eyebrow">{t("importToCamera")}</p>
            <h2 id="camera-import-title">{t("importDialogTitle")}</h2>
          </div>
          <button
            className="notice-dismiss"
            aria-label={t("cancel")}
            data-dialog-initial-focus
            disabled={busy}
            onClick={onClose}
          >
            ×
          </button>
        </header>
        <p className="dialog-intro">{t("importDialogIntro")}</p>
        <div className="dialog-recipe">
          <span>{t("recipeName")}</span>
          <strong>{recipe.name}</strong>
        </div>
        <label className="camera-name-field">
          <span>{t("cameraPresetName")}</span>
          <input
            value={cameraName}
            maxLength={31}
            disabled={busy}
            onChange={(event) => setCameraName(event.target.value)}
          />
          <small>{t("cameraPresetNameHelp")}</small>
          {!cameraNameIsVerified && (
            <small className="field-error">{t("cameraPresetNameAscii")}</small>
          )}
        </label>
        <div className="dialog-camera">
          <div>
            <span>{t("selectedCamera")}</span>
            <strong>{camera?.profile ?? t("noCompatibleCamera")}</strong>
            {deviceInfo && <small>{deviceInfo.model} · {deviceInfo.deviceVersion}</small>}
          </div>
          <div className="probe-actions">
            <button className="secondary" disabled={busy || !camera} onClick={verifyCameraIdentity}>
              {t("verify")}
            </button>
            <button className="secondary" disabled={busy} onClick={scan}>
              {t("scanCameraAgain")}
            </button>
          </div>
        </div>
        <div className="slot-overwrite-picker">
          <span>{t("selectTargetSlot")}</span>
          <div>
            {[1, 2, 3, 4].map((candidate) => (
              <button
                key={candidate}
                className={slot === candidate ? "primary" : "secondary"}
                aria-pressed={slot === candidate}
                disabled={busy || !camera || !writeIdentityVerified}
                onClick={() => setSlot(candidate)}
              >
                C{candidate}
              </button>
            ))}
          </div>
          {slot && (
            <small>
              {t("selectedTarget")}: C{slot}
            </small>
          )}
        </div>
        <div className="restore-point-card">
          <strong>{t("restorePoint")}</strong>
          <p>{t("restorePointDetail")}</p>
        </div>
        {slot && (
          <div className="raw-snapshot-card">
            <strong>{t("rawSnapshot")}</strong>
            <p>{t("rawSnapshotDetail")}</p>
            <button
              className="secondary"
              disabled={busy || !camera || !writeIdentityVerified}
              onClick={captureRawSnapshot}
            >
              {t("captureRawSnapshot")} C{slot}
            </button>
            {rawSnapshot && (
              <small>
                {t("rawSnapshotCaptured")}: {rawSnapshot.properties.length}{" "}
                {t("rawSnapshotProperties")};{" "}
                {rawSnapshot.unreadablePropertyCodes.length}{" "}
                {t("rawSnapshotUnavailable")}.
              </small>
            )}
          </div>
        )}
        {capability && (
          <CapabilityStatusPanel capability={capability} locale={locale} />
        )}
        {capabilityError && <p className="field-error">{capabilityError}</p>}
        {writeBlockers.length > 0 && (
          <p className="probe-message" role="alert">
            {t("writeValueUnsupported")} {writeBlockers.join(", ")}
          </p>
        )}
        {cropImageSizeSelected && (
          <p className="probe-message" role="alert">
            {t("cropImageSizeLocked")}
          </p>
        )}
        {message && (
          <p className="probe-message" role="status" aria-live="polite">
            {message}
          </p>
        )}
        <footer>
          <button className="secondary" disabled={busy} onClick={onClose}>
            {t("cancel")}
          </button>
          <button
            className="primary"
            disabled={
              busy ||
              !camera ||
              !slot ||
              !cameraName.trim() ||
              !cameraNameIsVerified ||
              !writeIdentityVerified ||
              writeBlockers.length > 0
            }
            onClick={confirmImport}
          >
            {t("confirmOverwrite")}
            {slot ? ` C${slot}` : ""}
          </button>
        </footer>
    </SafeDialog>
  );
}

function CapabilityStatusPanel({
  capability,
  locale,
}: {
  capability: CameraCapabilityRecord;
  locale: Locale;
}) {
  const statusLabel: Record<CameraPropertyStatus, string> =
    locale === "zh-TW"
      ? {
          write_verified: "已驗證可寫入",
          write_rejected: "相機拒絕（201C）",
          read_detected_unverified: "已探測，尚未驗證",
          blocked_unknown: "未知／禁止寫入",
        }
      : {
          write_verified: "Write verified",
          write_rejected: "Rejected (201C)",
          read_detected_unverified: "Detected, unverified",
          blocked_unknown: "Unknown / blocked",
        };
  return (
    <section className="capability-status-card">
      <div>
        <strong>
          {locale === "zh-TW" ? "X-M5 能力矩陣" : "X-M5 capability matrix"}
        </strong>
        <small>
          {capability.usbIds.join(", ")} · firmware {capability.firmware}
        </small>
      </div>
      <p>
        {locale === "zh-TW"
          ? "下列狀態由實機寫入、讀回與還原測試記錄；未驗證欄位不會送至相機。"
          : "Statuses come from hardware write, read-back, and restore tests; unverified fields are never sent to the camera."}
      </p>
      <ul>
        {capability.properties.map((property) => (
          <li key={property.key}>
            <div>
              <strong>
                {property.labelZh} <span lang="en">{property.labelEn}</span>
              </strong>
              <small>
                {property.code} · {property.verifiedValues}
              </small>
              {property.dependencies.map((dependency) => (
                <small key={dependency}>{dependency}</small>
              ))}
              {property.notes && <small>{property.notes}</small>}
            </div>
            <span className={`capability-status ${property.status}`}>
              {statusLabel[property.status]}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function InstalledPresetsPanel({
  devices,
  onScan,
  onPresetsRead,
  onSlotCleared,
  t,
}: {
  devices: CameraDiscovery[];
  onScan: () => Promise<CameraDiscovery[]>;
  onPresetsRead: (usbId: string, presets: InstalledPreset[]) => void;
  onSlotCleared: (usbId: string, slot: number) => void;
  t: Translator;
}) {
  const [presets, setPresets] = useState<InstalledPreset[]>([]);
  const [resetCandidate, setResetCandidate] = useState<InstalledPreset>();
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<PtpDeviceInfoResult>();
  const camera = devices.find(
    (device) => device.isFujifilm && device.ptpInterfaceDetected,
  );
  const writeIdentityVerified =
    deviceInfo?.capabilityStateKey === "exact_experimental";
  useEffect(() => setDeviceInfo(undefined), [camera?.usbId]);
  async function verifyCameraIdentity() {
    if (!camera) return;
    setBusy(true);
    setMessage(t("verifyWorking"));
    try {
      const identity = await probeCameraDeviceInfo(camera.usbId);
      setDeviceInfo(identity);
      setMessage(
        identity.capabilityStateKey === "exact_experimental"
          ? t("writeIdentityVerified")
          : identity.capabilityNextAction,
      );
    } catch (error) {
      const detail = String(error);
      setMessage(detail.includes("exclusive access") ? t("ptpBusy") : detail);
    } finally {
      setBusy(false);
    }
  }
  async function read() {
    const activeCamera =
      camera ??
      (await onScan()).find(
        (device) => device.isFujifilm && device.ptpInterfaceDetected,
      );
    if (!activeCamera || !writeIdentityVerified) {
      setPresets([]);
      setMessage(activeCamera ? t("verifyIdentityBeforeWrite") : t("noFuji"));
      return;
    }
    setBusy(true);
    setMessage("");
    try {
      const readPresets = await readXm5InstalledPresets(activeCamera.usbId);
      setPresets(readPresets);
      onPresetsRead(activeCamera.usbId, readPresets);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  function resetReadResults() {
    setPresets([]);
    setMessage(t("installedReset"));
  }
  async function resetSlot(preset: InstalledPreset) {
    if (!camera) return;
    setBusy(true);
    setMessage(t("resetSlotWorking") + ` C${preset.slot}…`);
    try {
      await clearXm5CustomSlot(camera.usbId, preset.slot);
      setPresets((current) =>
        current.map((item) =>
          item.slot === preset.slot ? { ...item, name: "" } : item,
        ),
      );
      onSlotCleared(camera.usbId, preset.slot);
      setMessage(`C${preset.slot} ${t("resetSlotVerified")}`);
    } catch (error) {
      const detail = String(error);
      setMessage(detail.includes("exclusive access") ? t("ptpBusy") : detail);
    } finally {
      setBusy(false);
      setResetCandidate(undefined);
    }
  }
  return (
    <section className="camera-panel">
      <div className="camera-status">
        <span className={camera ? "status-dot online" : "status-dot"} />
        <div>
          <strong>{camera?.profile ?? t("noCamera")}</strong>
          <p>{t("installedReadHelp")}</p>
        </div>
        <div className="probe-actions">
          <button className="secondary" disabled={busy} onClick={resetReadResults}>
            {t("resetInstalled")}
          </button>
          <button className="secondary" disabled={busy || !camera} onClick={verifyCameraIdentity}>
            {t("verify")}
          </button>
          <button className="primary" disabled={busy || !writeIdentityVerified} onClick={read}>
            {t("readInstalled")}
          </button>
        </div>
      </div>
      {message && (
        <p className="probe-message" role="status" aria-live="polite">
          {message}
        </p>
      )}
      {presets.length > 0 ? (
        <div className="device-table">
          {presets.map((preset) => (
            <div key={preset.slot}>
              <strong>C{preset.slot}</strong>
              <span>{preset.name || "—"}</span>
              <em>{preset.name ? t("installedLabel") : t("emptyLabel")}</em>
              <button
                className="secondary danger"
                disabled={busy || !camera?.writeEnabled || !writeIdentityVerified}
                onClick={() => setResetCandidate(preset)}
              >
                {t("resetSlot")}
              </button>
            </div>
          ))}
        </div>
      ) : (
        camera && (
          <div className="safety-card">
            <p>{t("noInstalled")}</p>
          </div>
        )
      )}
      {resetCandidate && (
        <SafeDialog
          labelledBy="reset-slot-title"
          onClose={() => setResetCandidate(undefined)}
          closeDisabled={busy}
        >
            <header>
              <div>
                <p className="eyebrow">{t("restorePoint")}</p>
                <h2 id="reset-slot-title">{t("resetSlotTitle")}</h2>
              </div>
              <button
                className="notice-dismiss"
                aria-label={t("cancel")}
                data-dialog-initial-focus
                disabled={busy}
                onClick={() => setResetCandidate(undefined)}
              >
                ×
              </button>
            </header>
            <p className="dialog-intro">
              {t("resetSlotIntro")} C{resetCandidate.slot}.
            </p>
            <div className="restore-point-card">
              <strong>C{resetCandidate.slot}</strong>
              <p>{t("resetSlotDetail")}</p>
            </div>
            {message && (
              <p className="probe-message" role="status" aria-live="polite">
                {message}
              </p>
            )}
            <footer>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setResetCandidate(undefined)}
              >
                {t("cancel")}
              </button>
              <button
                className="primary danger"
                disabled={busy}
                onClick={() => resetSlot(resetCandidate)}
              >
                {t("confirmResetSlot")}
              </button>
            </footer>
        </SafeDialog>
      )}
    </section>
  );
}

function CameraPanel({
  devices,
  status,
  onScan,
  refreshKey,
  pendingWriteJournals,
  onRecoveryUpdated,
  locale,
  t,
}: {
  devices: CameraDiscovery[];
  status: string;
  onScan: () => void;
  refreshKey: number;
  pendingWriteJournals: WriteJournalSummary[];
  onRecoveryUpdated: () => void;
  locale: Locale;
  t: Translator;
}) {
  const [deviceInfo, setDeviceInfo] = useState<PtpDeviceInfoResult>();
  const [slotValue, setSlotValue] = useState<PtpPropertyValueResult>();
  const [slotSelection, setSlotSelection] = useState<SlotSelectionResult>();
  const [backups, setBackups] = useState<CameraBackupSummary[]>([]);
  const [restoreCandidate, setRestoreCandidate] =
    useState<CameraBackupSummary>();
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const camera = devices.find(
    (device) => device.isFujifilm && device.ptpInterfaceDetected,
  );
  const pendingForCamera = pendingWriteJournals.filter(
    (journal) => journal.usbId === camera?.usbId,
  );
  const writeIdentityVerified =
    deviceInfo?.capabilityStateKey === "exact_experimental";
  useEffect(() => {
    if (camera) void loadBackups();
    else setBackups([]);
  }, [camera?.usbId, refreshKey]);
  useEffect(() => {
    setDeviceInfo(undefined);
    setSlotValue(undefined);
    setSlotSelection(undefined);
  }, [camera?.usbId]);

  async function loadBackups() {
    if (!camera) return;
    try {
      setBackups(await listCameraBackups(camera.usbId));
    } catch {
      setBackups([]);
    }
  }
  async function verify() {
    if (!camera) return;
    setBusy(true);
    setMessage(t("verifyWorking"));
    try {
      setDeviceInfo(await probeCameraDeviceInfo(camera.usbId));
      setMessage(t("verifySuccess"));
    } catch (error) {
      setMessage(t("couldNotVerify") + ": " + String(error));
    } finally {
      setBusy(false);
    }
  }
  async function readValue() {
    if (!camera) return;
    setBusy(true);
    setMessage(t("selectorWorking"));
    try {
      setSlotValue(await readCameraSlotSelector(camera.usbId));
      setMessage(t("selectorSuccess"));
    } catch (error) {
      setMessage(t("couldNotRead") + ": " + String(error));
    } finally {
      setBusy(false);
    }
  }
  async function selectSlot(slot: number) {
    if (!camera) return;
    setBusy(true);
    setMessage(t("selectingSlot") + " C" + slot + "…");
    try {
      const result = await selectCameraSlot(camera.usbId, slot);
      setSlotSelection(result);
      setMessage("C" + result.slot + " " + t("slotSelected"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  async function restoreBackup(backup: CameraBackupSummary) {
    if (!camera) return;
    setBusy(true);
    setMessage("");
    try {
      await restoreXm5Backup(camera.usbId, backup.id);
      setMessage(t("backupRestored"));
      onRecoveryUpdated();
    } catch (error) {
      setMessage(t("restoreFailed") + ": " + String(error));
    } finally {
      setBusy(false);
      setRestoreCandidate(undefined);
    }
  }
  return (
    <section className="camera-panel">
      <div className="camera-status">
        <span className={camera ? "status-dot online" : "status-dot"} />
        <div>
          <strong>{status}</strong>
          <p>{t("explicitActions")}</p>
        </div>
        <button className="primary" onClick={onScan}>
          {t("scanUsb")}
        </button>
      </div>
      {devices.length > 0 && (
        <div className="device-table">
          {devices.map((device) => (
            <div key={device.usbId}>
              <strong>
                {device.profile ?? device.product ?? t("usbDevice")}
              </strong>
              <span>{device.usbId}</span>
              <em>
                {device.isFujifilm
                  ? (device.family ?? "Fujifilm") +
                    " · " +
                    (device.recipeAccess ?? "Probe required")
                  : t("notConfigured")}
              </em>
            </div>
          ))}
        </div>
      )}
      {camera && (
        <div className="probe-card">
          <div>
            <p className="eyebrow">{t("readOnlyChecks")}</p>
            <h2>{camera.profile ?? t("unknownCamera")}</h2>
            <p>{t("cameraIntro")}</p>
          </div>
          <div className="probe-actions">
            <button className="secondary" disabled={busy} onClick={verify}>
              {t("verify")}
            </button>
            <button className="secondary" disabled={busy} onClick={readValue}>
              {t("readSelector")}
            </button>
          </div>
          {camera.writeEnabled && !writeIdentityVerified && (
            <p className="probe-message" role="status">
              {t("verifyIdentityBeforeWrite")}
            </p>
          )}
          {camera.writeEnabled && writeIdentityVerified && (
            <div className="slot-picker">
              <span>{t("selectSlot")}</span>
              <div>
                {[1, 2, 3, 4].map((slot) => (
                  <button
                    key={slot}
                    className="secondary"
                    disabled={busy}
                    onClick={() => selectSlot(slot)}
                  >
                    C{slot}
                  </button>
                ))}
              </div>
              <small>{t("selectSlotHelp")}</small>
            </div>
          )}
          {message && (
            <p className="probe-message" role="status" aria-live="polite">
              {message}
            </p>
          )}
          {deviceInfo && (
            <div className="probe-result">
              <span>
                {t("model")}{" "}
                <b>
                  {deviceInfo.manufacturer} {deviceInfo.model}
                </b>
              </span>
              <span>
                {t("firmware")} <b>{deviceInfo.deviceVersion}</b>
              </span>
              <span>
                {t("response")} <b>0x{deviceInfo.responseCode}</b>
              </span>
              <span>
                {t("status")} <b>{deviceInfo.capabilityState}</b>
              </span>
              {deviceInfo.capabilityRecordId && (
                <span>
                  Capability record <b>{deviceInfo.capabilityRecordId}</b>
                </span>
              )}
            </div>
          )}
          {deviceInfo && (
            <p className="empty-slot-note">{deviceInfo.capabilityNextAction}</p>
          )}
          {slotValue && (
            <div className="probe-result">
              <span>
                {t("property")} <b>0x{slotValue.propertyCode}</b>
              </span>
              <span>
                {t("rawValue")} <b>{slotValue.valueHex}</b>
              </span>
              <span>
                {t("status")} <b>{t("uninterpreted")}</b>
              </span>
            </div>
          )}
          {slotSelection && (
            <div className="probe-result">
              <span>
                {t("selectSlot")} <b>C{slotSelection.slot}</b>
              </span>
              <span>
                {t("response")} <b>{slotSelection.readBackHex}</b>
              </span>
            </div>
          )}
          {
            <div className="backup-panel">
              {pendingForCamera.length > 0 && (
                <section className="recovery-panel" aria-label={t("pendingRecoveryTitle")}>
                  <strong>{t("pendingRecoveryTitle")}</strong>
                  <p>{t("pendingRecoveryIntro")}</p>
                  {pendingForCamera.map((journal) => {
                    const backup = backups.find((item) => item.id === journal.backupId);
                    return (
                      <div key={journal.id}>
                        <span>C{journal.slot} · {formatDateTime(locale, journal.createdAt)}</span>
                        <em>{journal.state === "recovery_failed" ? t("pendingRecoveryFailed") : t("pendingRecoveryWriting")}</em>
                        {journal.error && <small>{journal.error}</small>}
                        <button
                          className="secondary"
                          disabled={busy || !backup || !writeIdentityVerified}
                          onClick={() => backup && setRestoreCandidate(backup)}
                        >
                          {t("recoveryBackup")}
                        </button>
                      </div>
                    );
                  })}
                </section>
              )}
              <p className="eyebrow">{t("backupHistory")}</p>
              {backups.length ? (
                <div className="device-table">
                  {backups.map((backup) => (
                    <div key={backup.id}>
                      <strong>C{backup.slot}</strong>
                      <span>
                        {formatDateTime(locale, backup.capturedAt)}
                      </span>
                      <em>
                        {backup.propertyCount} {t("properties")}
                      </em>
                      <button
                        className="secondary"
                        disabled={busy}
                        onClick={() => setRestoreCandidate(backup)}
                      >
                        {t("restoreBackup")}
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="empty-slot-note">{t("noBackups")}</p>
              )}
            </div>
          }
        </div>
      )}
      <div className="safety-card">
        <p className="eyebrow">{t("writeRequirements")}</p>
        <h2>{t("everyCamera")}</h2>
        <p>{t("writeRequirementText")}</p>
        <ol>
          <li>{t("safety1")}</li>
          <li>{t("safety2")}</li>
          <li>{t("safety3")}</li>
          <li>{t("safety4")}</li>
        </ol>
      </div>
      {restoreCandidate && (
        <SafeDialog
          labelledBy="restore-backup-title"
          onClose={() => setRestoreCandidate(undefined)}
          closeDisabled={busy}
        >
            <header>
              <div>
                <p className="eyebrow">{t("restorePoint")}</p>
                <h2 id="restore-backup-title">{t("restoreDialogTitle")}</h2>
              </div>
              <button
                className="notice-dismiss"
                aria-label={t("cancel")}
                data-dialog-initial-focus
                disabled={busy}
                onClick={() => setRestoreCandidate(undefined)}
              >
                ×
              </button>
            </header>
            <p className="dialog-intro">
              {t("restoreDialogIntro")} C{restoreCandidate.slot}.
            </p>
            <div className="restore-point-card">
              <strong>C{restoreCandidate.slot}</strong>
              <p>
                {formatDateTime(locale, restoreCandidate.capturedAt)} ·{" "}
                {restoreCandidate.propertyCount} {t("properties")}
              </p>
            </div>
            <footer>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setRestoreCandidate(undefined)}
              >
                {t("cancel")}
              </button>
              <button
                className="primary"
                disabled={busy}
                onClick={() => restoreBackup(restoreCandidate)}
              >
                {t("restoreBackup")}
              </button>
            </footer>
        </SafeDialog>
      )}
    </section>
  );
}

export default App;
