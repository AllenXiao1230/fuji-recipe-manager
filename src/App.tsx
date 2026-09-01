import { ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  afModes,
  colorSpaces,
  dRangePriorities,
  display,
  driveModes,
  dynamicRanges,
  filmSimulations,
  focusModes,
  grainSizes,
  imageQualities,
  imageSizes,
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
  discoverCameras,
  isDesktop,
  listCameraBackups,
  loadDesktopRecipes,
  probeCameraDeviceInfo,
  readCameraSlotSelector,
  readXm5InstalledPresets,
  restoreXm5Backup,
  saveDesktopRecipes,
  selectCameraSlot,
  writeXm5Recipe,
  type CameraBackupSummary,
  type CameraDiscovery,
  type InstalledPreset,
  type PtpDeviceInfoResult,
  type PtpPropertyValueResult,
  type RecipeWriteResult,
  type SlotSelectionResult,
} from "./lib/camera";
import {
  localeOptions,
  translate,
  type CopyKey,
  type Locale,
} from "./lib/i18n";
import {
  loadRecipes,
  normalizeRecipe,
  parseRecipeText,
  saveRecipes,
} from "./lib/recipeCodec";

type Page = "library" | "camera" | "installed" | "import";
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
  highlight: { min: -4, max: 4, step: 0.5 },
  shadow: { min: -4, max: 4, step: 0.5 },
  color: { min: -4, max: 4 },
  sharpness: { min: -4, max: 4 },
  highIsoNoiseReduction: { min: -4, max: 4 },
  clarity: { min: -5, max: 5 },
};
type DeletedRecipe = { recipe: Recipe; index: number };

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
  const [devices, setDevices] = useState<CameraDiscovery[]>([]);
  const [status, setStatus] = useState("");
  const [paste, setPaste] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const [sourceAuthor, setSourceAuthor] = useState("");
  const [notice, setNotice] = useState("");
  const [deletedRecipe, setDeletedRecipe] = useState<DeletedRecipe>();
  const [importingRecipe, setImportingRecipe] = useState<Recipe>();
  const [backupRefresh, setBackupRefresh] = useState(0);
  const [desktopLibraryReady, setDesktopLibraryReady] = useState(!isDesktop);
  const fileInput = useRef<HTMLInputElement>(null);
  const desktopSaveTimer = useRef<number>();
  const selected =
    recipes.find((recipe) => recipe.id === selectedId) ?? recipes[0];
  const visible = useMemo(
    () =>
      recipes.filter((recipe) =>
        (recipe.name + " " + recipe.tags.join(" "))
          .toLowerCase()
          .includes(query.toLowerCase()),
      ),
    [recipes, query],
  );
  const t: Translator = (key) => translate(locale, key);

  useEffect(() => {
    if (!desktopLibraryReady) return;
    saveRecipes(recipes);
    if (!isDesktop) return;
    window.clearTimeout(desktopSaveTimer.current);
    desktopSaveTimer.current = window.setTimeout(() => {
      void saveDesktopRecipes(recipes).catch(() =>
        setNotice(t("savingLibraryFailed")),
      );
    }, 350);
    return () => window.clearTimeout(desktopSaveTimer.current);
  }, [desktopLibraryReady, recipes]);
  useEffect(() => {
    localStorage.setItem("fuji-recipe-manager/locale", locale);
    document.documentElement.lang = locale;
  }, [locale]);
  useEffect(() => {
    if (!isDesktop) return;
    void loadDesktopRecipes()
      .then((stored) => {
        if (stored.length) {
          const normalized = stored.map(normalizeRecipe);
          setRecipes(normalized);
          setSelectedId(normalized[0]?.id ?? "");
        }
      })
      .catch(() => setNotice(t("databaseUnavailable")))
      .finally(() => setDesktopLibraryReady(true));
  }, []);

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
    setDeletedRecipe({ recipe: selected, index });
    setNotice(t("recipeDeleted"));
  }
  function undoDelete() {
    if (!deletedRecipe) return;
    setRecipes((items) => {
      const restored = [...items];
      restored.splice(deletedRecipe.index, 0, deletedRecipe.recipe);
      return restored;
    });
    setSelectedId(deletedRecipe.recipe.id);
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
  function importRecipe(value = paste) {
    try {
      const recipe = parseRecipeText(value, {
        url: sourceUrl,
        author: sourceAuthor,
      });
      setRecipes((items) => [recipe, ...items]);
      setSelectedId(recipe.id);
      setPaste("");
      setSourceUrl("");
      setSourceAuthor("");
      setPage("library");
      setNotice(locale === "zh-TW" ? "Recipe 已匯入。" : "Recipe imported.");
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
      .then(importRecipe)
      .catch(() => setNotice(t("fileReadFailure")));
    event.target.value = "";
  }

  const title =
    page === "library"
      ? t("buildLibrary")
      : page === "camera"
        ? t("connectCare")
        : page === "installed"
          ? t("installedTitle")
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
          onClick={() => setPage("library")}
        >
          {t("library")} <b>{recipes.length}</b>
        </button>
        <button
          className={"nav " + (page === "camera" ? "active" : "")}
          onClick={() => setPage("camera")}
        >
          {t("camera")}{" "}
          <i
            className={
              devices.some((device) => device.isFujifilm) ? "connected" : ""
            }
          />
        </button>
        <button
          className={"nav " + (page === "installed" ? "active" : "")}
          onClick={() => setPage("installed")}
        >
          {t("installed")}
        </button>
        <button
          className={"nav " + (page === "import" ? "active" : "")}
          onClick={() => setPage("import")}
        >
          {t("importRecipe")}
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
      <main className="workspace">
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
                <button className="primary" onClick={createRecipe}>
                  {t("newRecipe")}
                </button>
                <button className="secondary" onClick={() => setPage("import")}>
                  {t("importRecipe")}
                </button>
              </div>
              <div className="cards">
                {visible.map((recipe) => (
                  <button
                    key={recipe.id}
                    className={
                      "recipe-card " +
                      (recipe.id === selected?.id ? "selected" : "")
                    }
                    onClick={() => setSelectedId(recipe.id)}
                  >
                    <span>{recipe.favorite ? "★" : "☆"}</span>
                    <strong>{recipe.name}</strong>
                    <small>
                      {display(recipe.settings.filmSimulation)} ·{" "}
                      {recipe.tags.join(" / ") || t("untaged")}
                    </small>
                  </button>
                ))}
              </div>
            </div>
            {selected && (
              <RecipeEditor
                recipe={selected}
                t={t}
                onChange={updateRecipe}
                onExport={exportRecipe}
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
            t={t}
          />
        )}
        {page === "installed" && (
          <InstalledPresetsPanel devices={devices} onScan={scanCamera} t={t} />
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
                accept=".frecipe,.json,.txt,application/json,text/plain"
                onChange={importFile}
              />
              <button className="primary" onClick={() => importRecipe()}>
                {t("parseSave")}
              </button>
            </div>
          </section>
        )}
        {importingRecipe && (
          <CameraImportDialog
            recipe={importingRecipe}
            devices={devices}
            t={t}
            onScan={scanCamera}
            onClose={() => setImportingRecipe(undefined)}
            onCompleted={(result) => {
              setBackupRefresh((value) => value + 1);
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

function RecipeEditor({
  recipe,
  t,
  onChange,
  onExport,
  onDelete,
  onImport,
}: {
  recipe: Recipe;
  t: Translator;
  onChange: (recipe: Recipe) => void;
  onExport: () => void;
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
        {recipe.source.url && (
          <a href={recipe.source.url} target="_blank" rel="noreferrer">
            {t("openSource")}
          </a>
        )}
      </div>
      <section className="recipe-settings-section">
        <div className="settings-section-heading">
          <p className="eyebrow">{t("imageQualitySettings")}</p>
          <p>{t("imageQualitySettingsHelp")}</p>
        </div>
        <div className="settings-grid">
          <Select
            label={t("filmSimulation")}
            value={recipe.settings.filmSimulation}
            options={filmSimulations}
            onChange={(value) =>
              setting(
                "filmSimulation",
                value as RecipeSettings["filmSimulation"],
              )
            }
          />
          <Select
            label={t("dynamicRange")}
            value={recipe.settings.dynamicRange}
            options={dynamicRanges}
            onChange={(value) =>
              setting("dynamicRange", value as RecipeSettings["dynamicRange"])
            }
          />
          <Select
            label={t("dRangePriority")}
            value={recipe.settings.dRangePriority}
            options={dRangePriorities}
            onChange={(value) =>
              setting(
                "dRangePriority",
                value as RecipeSettings["dRangePriority"],
              )
            }
          />
          <Select
            label={t("whiteBalance")}
            value={recipe.settings.whiteBalance.mode}
            options={whiteBalances}
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
            onChange={(value) =>
              setting(
                "colorChromeFxBlue",
                value as RecipeSettings["colorChromeFxBlue"],
              )
            }
          />
          <Select
            label={t("portraitEnhancer")}
            value={recipe.settings.portraitEnhancer}
            options={portraitEnhancerLevels}
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
            onChange={(value) =>
              setting("colorSpace", value as RecipeSettings["colorSpace"])
            }
          />
          <Select
            label={t("imageSize")}
            value={recipe.settings.imageSize}
            options={imageSizes}
            onChange={(value) =>
              setting("imageSize", value as RecipeSettings["imageSize"])
            }
          />
          <Select
            label={t("imageQuality")}
            value={recipe.settings.imageQuality}
            options={imageQualities}
            onChange={(value) =>
              setting("imageQuality", value as RecipeSettings["imageQuality"])
            }
          />
          <Select
            label={t("rawRecording")}
            value={recipe.settings.rawRecording}
            options={rawRecordingOptions}
            onChange={(value) =>
              setting("rawRecording", value as RecipeSettings["rawRecording"])
            }
          />
          <Select
            label={t("stillFormat")}
            value={recipe.settings.stillFormat}
            options={stillFormats}
            onChange={(value) =>
              setting("stillFormat", value as RecipeSettings["stillFormat"])
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
            onChange={(value) =>
              shootingSetting(
                "isoSensitivity",
                value as ShootingSettings["isoSensitivity"],
              )
            }
          />
          <NumberControl
            label={t("exposureCompensation")}
            value={recipe.shootingSettings.exposureCompensation}
            min={-5}
            max={5}
            t={t}
            onChange={(value) => shootingSetting("exposureCompensation", value)}
          />
          <Select
            label={t("meteringMode")}
            value={recipe.shootingSettings.meteringMode}
            options={meteringModes}
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
            onChange={(value) =>
              shootingSetting("afMode", value as ShootingSettings["afMode"])
            }
          />
          <Select
            label={t("driveMode")}
            value={recipe.shootingSettings.driveMode}
            options={driveModes}
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
  onChange,
}: {
  label: string;
  value: string;
  options: readonly string[];
  onChange: (value: string) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((option) => (
          <option key={option} value={option}>
            {display(option)}
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
  t,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
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
          {value > 0 ? "+" : ""}
          {value}
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
  t,
  onScan,
  onClose,
  onCompleted,
}: {
  recipe: Recipe;
  devices: CameraDiscovery[];
  t: Translator;
  onScan: () => Promise<CameraDiscovery[]>;
  onClose: () => void;
  onCompleted: (result: RecipeWriteResult) => void;
}) {
  const [slot, setSlot] = useState<number>();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [cameraName, setCameraName] = useState(recipe.name.slice(0, 31));
  const writeBlockers = xm5WriteBlockers(recipe.settings);
  const cameraNameIsVerified = /^[\x20-\x7E]+$/.test(cameraName.trim());
  const camera = devices.find(
    (device) =>
      device.isFujifilm && device.ptpInterfaceDetected && device.writeEnabled,
  );

  async function scan() {
    setBusy(true);
    setMessage(t("scanUsb") + "…");
    try {
      await onScan();
      setMessage("");
    } finally {
      setBusy(false);
    }
  }

  async function confirmImport() {
    if (!camera || !slot) return;
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

  return (
    <div className="modal-backdrop">
      <section
        className="camera-import-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="camera-import-title"
      >
        <header>
          <div>
            <p className="eyebrow">{t("importToCamera")}</p>
            <h2 id="camera-import-title">{t("importDialogTitle")}</h2>
          </div>
          <button
            className="notice-dismiss"
            aria-label={t("cancel")}
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
          </div>
          <button className="secondary" disabled={busy} onClick={scan}>
            {t("scanCameraAgain")}
          </button>
        </div>
        <div className="slot-overwrite-picker">
          <span>{t("selectTargetSlot")}</span>
          <div>
            {[1, 2, 3, 4].map((candidate) => (
              <button
                key={candidate}
                className={slot === candidate ? "primary" : "secondary"}
                aria-pressed={slot === candidate}
                disabled={busy || !camera}
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
        {writeBlockers.length > 0 && (
          <p className="probe-message" role="alert">
            {t("writeValueUnsupported")} {writeBlockers.join(", ")}
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
              writeBlockers.length > 0
            }
            onClick={confirmImport}
          >
            {t("confirmOverwrite")}
            {slot ? ` C${slot}` : ""}
          </button>
        </footer>
      </section>
    </div>
  );
}

function InstalledPresetsPanel({
  devices,
  onScan,
  t,
}: {
  devices: CameraDiscovery[];
  onScan: () => Promise<CameraDiscovery[]>;
  t: Translator;
}) {
  const [presets, setPresets] = useState<InstalledPreset[]>([]);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const camera = devices.find(
    (device) => device.isFujifilm && device.ptpInterfaceDetected,
  );
  async function read() {
    const activeCamera =
      camera ??
      (await onScan()).find(
        (device) => device.isFujifilm && device.ptpInterfaceDetected,
      );
    if (!activeCamera) {
      setPresets([]);
      setMessage(t("noFuji"));
      return;
    }
    setBusy(true);
    setMessage("");
    try {
      setPresets(await readXm5InstalledPresets(activeCamera.usbId));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  function reset() {
    setPresets([]);
    setMessage(t("installedReset"));
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
          <button className="secondary" disabled={busy} onClick={reset}>
            {t("resetInstalled")}
          </button>
          <button className="primary" disabled={busy} onClick={read}>
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
    </section>
  );
}

function CameraPanel({
  devices,
  status,
  onScan,
  refreshKey,
  t,
}: {
  devices: CameraDiscovery[];
  status: string;
  onScan: () => void;
  refreshKey: number;
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
  useEffect(() => {
    if (camera) void loadBackups();
    else setBackups([]);
  }, [camera?.usbId, refreshKey]);

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
            </div>
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
              <p className="eyebrow">{t("backupHistory")}</p>
              {backups.length ? (
                <div className="device-table">
                  {backups.map((backup) => (
                    <div key={backup.id}>
                      <strong>C{backup.slot}</strong>
                      <span>
                        {new Date(backup.capturedAt).toLocaleString()}
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
        <div className="modal-backdrop">
          <section
            className="camera-import-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="restore-backup-title"
          >
            <header>
              <div>
                <p className="eyebrow">{t("restorePoint")}</p>
                <h2 id="restore-backup-title">{t("restoreDialogTitle")}</h2>
              </div>
              <button
                className="notice-dismiss"
                aria-label={t("cancel")}
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
                {new Date(restoreCandidate.capturedAt).toLocaleString()} ·{" "}
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
          </section>
        </div>
      )}
    </section>
  );
}

export default App;
