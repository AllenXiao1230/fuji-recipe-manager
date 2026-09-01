import {
  afModes,
  colorSpaces,
  dRangePriorities,
  driveModes,
  dynamicRanges,
  emptySettings,
  emptyShootingSettings,
  filmSimulations,
  focusModes,
  grainSizes,
  isoSensitivities,
  imageQualities,
  imageSizes,
  makeRecipe,
  meteringModes,
  onOff,
  portraitEnhancerLevels,
  rawRecordingOptions,
  shutterTypes,
  strengths,
  stillFormats,
  whiteBalances,
  type Recipe,
  type RecipeSettings,
  type ShootingSettings,
} from "../domain/recipe";

const storageKey = "fuji-recipe-manager/recipes/v1";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function recordAs<T>(value: unknown): Partial<T> {
  return isRecord(value) ? (value as Partial<T>) : {};
}

function normaliseNumber(
  value: unknown,
  fallback: number,
  min: number,
  max: number,
  step = 1,
) {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  const snapped = Math.round(value / step) * step;
  if (snapped < min || snapped > max) return fallback;
  return Math.round(snapped * 100) / 100;
}

export function seedRecipes(): Recipe[] {
  const portra = makeRecipe("Portra 400");
  portra.description = "Warm daylight portrait look.";
  portra.tags = ["Portrait", "Daylight"];
  portra.favorite = true;
  portra.settings = {
    ...structuredClone(emptySettings),
    whiteBalance: {
      ...emptySettings.whiteBalance,
      mode: "AUTO",
      shiftR: 3,
      shiftB: -5,
    },
    grain: { strength: "STRONG", size: "SMALL" },
    colorChromeEffect: "STRONG",
    colorChromeFxBlue: "WEAK",
    highlight: -1,
    shadow: -2,
    color: 2,
    sharpness: -2,
    highIsoNoiseReduction: -4,
    clarity: -2,
  };
  const night = makeRecipe("Night Chrome");
  night.description = "A restrained evening starting point.";
  night.tags = ["Night", "Street"];
  night.settings = {
    ...structuredClone(emptySettings),
    filmSimulation: "ETERNA",
    dynamicRange: "DR400",
    whiteBalance: {
      ...emptySettings.whiteBalance,
      mode: "INCANDESCENT",
      shiftR: 2,
      shiftB: -4,
    },
    colorChromeEffect: "STRONG",
    highlight: -2,
    shadow: 1,
    color: -1,
    sharpness: -1,
    highIsoNoiseReduction: -4,
    clarity: -2,
  };
  return [portra, night];
}

export function loadRecipes(): Recipe[] {
  try {
    const stored = localStorage.getItem(storageKey);
    if (!stored) return seedRecipes();
    const parsed = JSON.parse(stored);
    return Array.isArray(parsed) ? parsed.map(normalizeRecipe) : seedRecipes();
  } catch {
    return seedRecipes();
  }
}

export function saveRecipes(recipes: Recipe[]) {
  localStorage.setItem(storageKey, JSON.stringify(recipes));
}

export function normalizeRecipe(value: unknown): Recipe {
  const candidate = recordAs<Recipe>(value);
  const recipe = makeRecipe(
    typeof candidate.name === "string" ? candidate.name : "Imported Recipe",
  );
  const settings = recordAs<RecipeSettings>(candidate.settings);
  const whiteBalance = recordAs<RecipeSettings["whiteBalance"]>(
    settings.whiteBalance,
  );
  const grain = recordAs<RecipeSettings["grain"]>(settings.grain);
  const whiteBalanceMode =
    String(whiteBalance.mode) === "KELVIN"
      ? "COLOR_TEMPERATURE"
      : whiteBalance.mode;
  recipe.id = typeof candidate.id === "string" ? candidate.id : recipe.id;
  recipe.description =
    typeof candidate.description === "string" ? candidate.description : "";
  recipe.tags = Array.isArray(candidate.tags)
    ? candidate.tags.filter((tag): tag is string => typeof tag === "string")
    : [];
  recipe.favorite = Boolean(candidate.favorite);
  recipe.cameraCompatibility = Array.isArray(candidate.cameraCompatibility)
    ? candidate.cameraCompatibility.filter(
        (camera): camera is string => typeof camera === "string",
      )
    : ["X-M5"];
  recipe.createdAt =
    typeof candidate.createdAt === "string"
      ? candidate.createdAt
      : recipe.createdAt;
  recipe.updatedAt = new Date().toISOString();
  const source = recordAs<Recipe["source"]>(candidate.source);
  recipe.source = {
    author: typeof source.author === "string" ? source.author : "",
    url: typeof source.url === "string" ? source.url : "",
  };
  recipe.settings = {
    ...structuredClone(emptySettings),
    ...settings,
    filmSimulation: filmSimulations.includes(
      settings.filmSimulation as (typeof filmSimulations)[number],
    )
      ? (settings.filmSimulation as (typeof filmSimulations)[number])
      : emptySettings.filmSimulation,
    dynamicRange: dynamicRanges.includes(
      settings.dynamicRange as (typeof dynamicRanges)[number],
    )
      ? (settings.dynamicRange as (typeof dynamicRanges)[number])
      : emptySettings.dynamicRange,
    whiteBalance: {
      ...emptySettings.whiteBalance,
      ...whiteBalance,
      mode: whiteBalances.includes(
        whiteBalanceMode as (typeof whiteBalances)[number],
      )
        ? (whiteBalanceMode as (typeof whiteBalances)[number])
        : emptySettings.whiteBalance.mode,
      colorTemperatureK:
        typeof whiteBalance.colorTemperatureK === "number" &&
        whiteBalance.colorTemperatureK >= 2500 &&
        whiteBalance.colorTemperatureK <= 10000
          ? Math.round(whiteBalance.colorTemperatureK / 10) * 10
          : emptySettings.whiteBalance.colorTemperatureK,
      shiftR: normaliseNumber(
        whiteBalance.shiftR,
        emptySettings.whiteBalance.shiftR,
        -9,
        9,
      ),
      shiftB: normaliseNumber(
        whiteBalance.shiftB,
        emptySettings.whiteBalance.shiftB,
        -9,
        9,
      ),
    },
    grain: {
      ...emptySettings.grain,
      ...grain,
      strength: strengths.includes(
        grain.strength as (typeof strengths)[number],
      )
        ? (grain.strength as (typeof strengths)[number])
        : emptySettings.grain.strength,
      size: grainSizes.includes(grain.size as (typeof grainSizes)[number])
        ? (grain.size as (typeof grainSizes)[number])
        : emptySettings.grain.size,
    },
    colorChromeEffect: strengths.includes(
      settings.colorChromeEffect as (typeof strengths)[number],
    )
      ? (settings.colorChromeEffect as (typeof strengths)[number])
      : emptySettings.colorChromeEffect,
    colorChromeFxBlue: strengths.includes(
      settings.colorChromeFxBlue as (typeof strengths)[number],
    )
      ? (settings.colorChromeFxBlue as (typeof strengths)[number])
      : emptySettings.colorChromeFxBlue,
    dRangePriority: dRangePriorities.includes(
      settings.dRangePriority as (typeof dRangePriorities)[number],
    )
      ? (settings.dRangePriority as (typeof dRangePriorities)[number])
      : emptySettings.dRangePriority,
    portraitEnhancer: portraitEnhancerLevels.includes(
      settings.portraitEnhancer as (typeof portraitEnhancerLevels)[number],
    )
      ? (settings.portraitEnhancer as (typeof portraitEnhancerLevels)[number])
      : emptySettings.portraitEnhancer,
    longExposureNoiseReduction: onOff.includes(
      settings.longExposureNoiseReduction as (typeof onOff)[number],
    )
      ? (settings.longExposureNoiseReduction as (typeof onOff)[number])
      : emptySettings.longExposureNoiseReduction,
    lensModulationOptimizer: onOff.includes(
      settings.lensModulationOptimizer as (typeof onOff)[number],
    )
      ? (settings.lensModulationOptimizer as (typeof onOff)[number])
      : emptySettings.lensModulationOptimizer,
    colorSpace: colorSpaces.includes(
      settings.colorSpace as (typeof colorSpaces)[number],
    )
      ? (settings.colorSpace as (typeof colorSpaces)[number])
      : emptySettings.colorSpace,
    imageSize: imageSizes.includes(
      settings.imageSize as (typeof imageSizes)[number],
    )
      ? (settings.imageSize as (typeof imageSizes)[number])
      : emptySettings.imageSize,
    imageQuality: imageQualities.includes(
      settings.imageQuality as (typeof imageQualities)[number],
    )
      ? (settings.imageQuality as (typeof imageQualities)[number])
      : emptySettings.imageQuality,
    rawRecording: rawRecordingOptions.includes(
      settings.rawRecording as (typeof rawRecordingOptions)[number],
    )
      ? (settings.rawRecording as (typeof rawRecordingOptions)[number])
      : emptySettings.rawRecording,
    stillFormat: stillFormats.includes(
      settings.stillFormat as (typeof stillFormats)[number],
    )
      ? (settings.stillFormat as (typeof stillFormats)[number])
      : emptySettings.stillFormat,
    highlight: normaliseNumber(settings.highlight, emptySettings.highlight, -4, 4, 0.5),
    shadow: normaliseNumber(settings.shadow, emptySettings.shadow, -4, 4, 0.5),
    color: normaliseNumber(settings.color, emptySettings.color, -4, 4),
    sharpness: normaliseNumber(settings.sharpness, emptySettings.sharpness, -4, 4),
    highIsoNoiseReduction: normaliseNumber(
      settings.highIsoNoiseReduction,
      emptySettings.highIsoNoiseReduction,
      -4,
      4,
    ),
    clarity: normaliseNumber(settings.clarity, emptySettings.clarity, -5, 5),
  };
  const shootingSettings = recordAs<ShootingSettings>(candidate.shootingSettings);
  recipe.shootingSettings = {
    ...structuredClone(emptyShootingSettings),
    ...shootingSettings,
    isoSensitivity:
      String(shootingSettings.isoSensitivity) === "AUTO"
        ? "AUTO_1"
        : isoSensitivities.includes(
              shootingSettings.isoSensitivity as (typeof isoSensitivities)[number],
            )
          ? (shootingSettings.isoSensitivity as (typeof isoSensitivities)[number])
          : emptyShootingSettings.isoSensitivity,
    meteringMode: meteringModes.includes(
      shootingSettings.meteringMode as (typeof meteringModes)[number],
    )
      ? (shootingSettings.meteringMode as (typeof meteringModes)[number])
      : emptyShootingSettings.meteringMode,
    focusMode: focusModes.includes(
      shootingSettings.focusMode as (typeof focusModes)[number],
    )
      ? (shootingSettings.focusMode as (typeof focusModes)[number])
      : emptyShootingSettings.focusMode,
    afMode: afModes.includes(
      shootingSettings.afMode as (typeof afModes)[number],
    )
      ? (shootingSettings.afMode as (typeof afModes)[number])
      : emptyShootingSettings.afMode,
    driveMode: driveModes.includes(
      shootingSettings.driveMode as (typeof driveModes)[number],
    )
      ? (shootingSettings.driveMode as (typeof driveModes)[number])
      : emptyShootingSettings.driveMode,
    shutterType: shutterTypes.includes(
      shootingSettings.shutterType as (typeof shutterTypes)[number],
    )
      ? (shootingSettings.shutterType as (typeof shutterTypes)[number])
      : emptyShootingSettings.shutterType,
    exposureCompensation:
      normaliseNumber(
        shootingSettings.exposureCompensation,
        emptyShootingSettings.exposureCompensation,
        -5,
        5,
      ),
  };
  return recipe;
}

export interface RecipeSource {
  author?: string;
  url?: string;
}

function normaliseEnum(value: string) {
  return value
    .trim()
    .toUpperCase()
    .replaceAll(/[-\s.]+/g, "_")
    .replace(/_+/g, "_");
}

function parseSignedNumber(value: string) {
  const found = value.match(/[+-]?\d+(?:\.\d+)?/);
  return found ? Number(found[0]) : undefined;
}

function applyWhiteBalance(recipe: Recipe, value: string) {
  const shift =
    value.match(/(?:red|r)\s*([+-]\s*\d+).*?(?:blue|b)\s*([+-]\s*\d+)/i) ??
    value.match(/([+-]\s*\d+)\s*(?:red|r).*?([+-]\s*\d+)\s*(?:blue|b)/i);
  const mode = value.split(/[,;]|\s+(?:red|r)\s*[+-]/i)[0].trim();
  recipe.settings.whiteBalance = {
    ...recipe.settings.whiteBalance,
    mode: normaliseEnum(
      mode || "AUTO",
    ) as RecipeSettings["whiteBalance"]["mode"],
    shiftR: shift
      ? Number(shift[1].replace(/\s/g, ""))
      : recipe.settings.whiteBalance.shiftR,
    shiftB: shift
      ? Number(shift[2].replace(/\s/g, ""))
      : recipe.settings.whiteBalance.shiftB,
  };
}

export function parseRecipeText(
  text: string,
  source: RecipeSource = {},
): Recipe {
  if (text.trim().startsWith("{")) return normalizeRecipe(JSON.parse(text));
  const recipe = makeRecipe("Imported Recipe");
  recipe.source = {
    author: source.author?.trim() ?? "",
    url: source.url?.trim() ?? "",
  };
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (const line of lines) {
    const [rawKey, ...rawValue] = line.split(/\s*[:\t]\s*/);
    const key = rawKey.toLowerCase().replace(/[：]/g, ":");
    const value = rawValue.join(":").trim();
    if (!value) continue;
    if (["name", "recipe", "recipe name", "名稱"].includes(key))
      recipe.name = value;
    else if (["author", "creator", "作者"].includes(key))
      recipe.source.author = value;
    else if (["source", "source url", "來源", "來源網址"].includes(key))
      recipe.source.url = value;
    else if (key.includes("film simulation") || key.includes("底片模擬"))
      recipe.settings.filmSimulation = normaliseEnum(
        value,
      ) as RecipeSettings["filmSimulation"];
    else if (key.includes("dynamic range") || key.includes("動態範圍"))
      recipe.settings.dynamicRange = normaliseEnum(
        value,
      ) as RecipeSettings["dynamicRange"];
    else if (
      key.includes("white balance shift") ||
      key.includes("wb shift") ||
      key.includes("白平衡偏移")
    ) {
      const values = value.match(/[+-]\s*\d+/g);
      if (values && values.length >= 2)
        recipe.settings.whiteBalance = {
          ...recipe.settings.whiteBalance,
          shiftR: Number(values[0].replace(/\s/g, "")),
          shiftB: Number(values[1].replace(/\s/g, "")),
        };
    } else if (
      key.includes("white balance") ||
      key === "wb" ||
      key.includes("白平衡")
    )
      applyWhiteBalance(recipe, value);
    else if (key.includes("grain") || key.includes("顆粒")) {
      const [strength, size] = value.split(/,\s*/);
      recipe.settings.grain = {
        strength: normaliseEnum(
          strength,
        ) as RecipeSettings["grain"]["strength"],
        size: normaliseEnum(size ?? "Small") as RecipeSettings["grain"]["size"],
      };
    } else if (
      key.includes("color chrome fx") ||
      key.includes("chrome fx blue")
    )
      recipe.settings.colorChromeFxBlue = normaliseEnum(
        value,
      ) as RecipeSettings["colorChromeFxBlue"];
    else if (key.includes("color chrome"))
      recipe.settings.colorChromeEffect = normaliseEnum(
        value,
      ) as RecipeSettings["colorChromeEffect"];
    else if (key.includes("high iso") || key === "nr" || key.includes("雜訊"))
      recipe.settings.highIsoNoiseReduction =
        parseSignedNumber(value) ?? recipe.settings.highIsoNoiseReduction;
    else if (["highlight", "highlights", "高光"].includes(key))
      recipe.settings.highlight =
        parseSignedNumber(value) ?? recipe.settings.highlight;
    else if (["shadow", "shadows", "陰影"].includes(key))
      recipe.settings.shadow =
        parseSignedNumber(value) ?? recipe.settings.shadow;
    else if (["color", "colour", "色彩"].includes(key))
      recipe.settings.color = parseSignedNumber(value) ?? recipe.settings.color;
    else if (["sharpness", "銳利度"].includes(key))
      recipe.settings.sharpness =
        parseSignedNumber(value) ?? recipe.settings.sharpness;
    else if (["clarity", "清晰度"].includes(key))
      recipe.settings.clarity =
        parseSignedNumber(value) ?? recipe.settings.clarity;
  }
  return normalizeRecipe(recipe);
}
