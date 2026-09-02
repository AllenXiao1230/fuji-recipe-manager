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
  isoAutoMaximums,
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
  type RecipeInteroperability,
  type RawPtpPresetSnapshot,
  type RecipeSettings,
  type ShootingSettings,
} from "../domain/recipe";
import { optionValueFromText } from "./i18n";

const storageKey = "fuji-recipe-manager/recipes/v1";

const legacyCropImageSizeAliases: Record<string, RecipeSettings["imageSize"]> = {
  M_3_2_1_25X_CROP: "P_3_2_1_25X_CROP",
  M_16_9_1_25X_CROP: "P_16_9_1_25X_CROP",
  M_1_1_1_25X_CROP: "P_1_1_1_25X_CROP",
};

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
  const monochromaticColor = recordAs<RecipeSettings["monochromaticColor"]>(
    settings.monochromaticColor,
  );
  const whiteBalanceMode =
    String(whiteBalance.mode) === "KELVIN"
      ? "COLOR_TEMPERATURE"
      : whiteBalance.mode;
  const imageSizeValue =
    typeof settings.imageSize === "string"
      ? (legacyCropImageSizeAliases[settings.imageSize] ?? settings.imageSize)
      : settings.imageSize;
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
  const interoperability = recordAs<RecipeInteroperability>(
    candidate.interoperability,
  );
  const fp = recordAs<NonNullable<RecipeInteroperability["fp"]>>(
    interoperability.fp,
  );
  const rawPtpPresetSnapshot = recordAs<
    NonNullable<RecipeInteroperability["rawPtpPresetSnapshot"]>
  >(interoperability.rawPtpPresetSnapshot);
  const safeProperties = (value: unknown) =>
    Array.isArray(value)
      ? value.flatMap((item) => {
          const property = recordAs<{ name: unknown; value: unknown }>(item);
          return typeof property.name === "string" &&
            typeof property.value === "string" &&
            property.name !== "SerialNumber"
            ? [{ name: property.name, value: property.value }]
            : [];
        })
      : [];
  const safeRawProperties = (value: unknown) =>
    Array.isArray(value)
      ? value.flatMap((item) => {
          const property = recordAs<{ code: unknown; valueHex: unknown }>(item);
          return typeof property.code === "string" &&
            /^[0-9A-F]{4}$/.test(property.code) &&
            typeof property.valueHex === "string" &&
            /^(?:[0-9A-F]{2})(?: [0-9A-F]{2})*$/.test(property.valueHex) &&
            property.valueHex.length <= 512
            ? [{ code: property.code, valueHex: property.valueHex }]
            : [];
        })
      : [];
  const safePropertyCodes = (value: unknown) =>
    Array.isArray(value)
      ? value.filter(
          (code): code is string =>
            typeof code === "string" && /^[0-9A-F]{4}$/.test(code),
        )
      : [];
  const normalizedInteroperability: RecipeInteroperability = {};
  if (
    fp.format === "FP1" ||
    fp.format === "FP2" ||
    fp.format === "FP3"
  ) {
    normalizedInteroperability.fp = {
      format: fp.format,
      application: typeof fp.application === "string" ? fp.application : "XRFC",
      profileVersion:
        typeof fp.profileVersion === "string" ? fp.profileVersion : "1.12.0.0",
      device: typeof fp.device === "string" ? fp.device : "",
      deviceVersion:
        typeof fp.deviceVersion === "string" ? fp.deviceVersion : "",
      sourceProperties: safeProperties(fp.sourceProperties),
      unmappedProperties: safeProperties(fp.unmappedProperties),
    };
  }
  const snapshotProperties = safeRawProperties(rawPtpPresetSnapshot.properties);
  if (
    rawPtpPresetSnapshot.manufacturer === "FUJIFILM" &&
    typeof rawPtpPresetSnapshot.model === "string" &&
    rawPtpPresetSnapshot.model.length > 0 &&
    rawPtpPresetSnapshot.model.length <= 128 &&
    typeof rawPtpPresetSnapshot.firmware === "string" &&
    rawPtpPresetSnapshot.firmware.length > 0 &&
    rawPtpPresetSnapshot.firmware.length <= 64 &&
    typeof rawPtpPresetSnapshot.usbId === "string" &&
    /^[0-9A-F]{4}:[0-9A-F]{4}$/.test(rawPtpPresetSnapshot.usbId) &&
    typeof rawPtpPresetSnapshot.slot === "number" &&
    Number.isInteger(rawPtpPresetSnapshot.slot) &&
    rawPtpPresetSnapshot.slot >= 1 &&
    rawPtpPresetSnapshot.slot <= 4 &&
    typeof rawPtpPresetSnapshot.capturedAt === "string" &&
    !Number.isNaN(Date.parse(rawPtpPresetSnapshot.capturedAt)) &&
    rawPtpPresetSnapshot.restorationPolicy === "read_only_preserved" &&
    snapshotProperties.length > 0
  ) {
    normalizedInteroperability.rawPtpPresetSnapshot = {
      manufacturer: "FUJIFILM",
      model: rawPtpPresetSnapshot.model,
      firmware: rawPtpPresetSnapshot.firmware,
      usbId: rawPtpPresetSnapshot.usbId,
      slot: rawPtpPresetSnapshot.slot,
      capturedAt: rawPtpPresetSnapshot.capturedAt,
      restorationPolicy: "read_only_preserved",
      properties: snapshotProperties,
      unreadablePropertyCodes: safePropertyCodes(
        rawPtpPresetSnapshot.unreadablePropertyCodes,
      ),
    } satisfies RawPtpPresetSnapshot;
  }
  if (Object.keys(normalizedInteroperability).length) {
    recipe.interoperability = normalizedInteroperability;
  }
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
    smoothSkinEffect: strengths.includes(
      settings.smoothSkinEffect as (typeof strengths)[number],
    )
      ? (settings.smoothSkinEffect as (typeof strengths)[number])
      : emptySettings.smoothSkinEffect,
    monochromaticColor: {
      warmCool: normaliseNumber(
        monochromaticColor.warmCool,
        emptySettings.monochromaticColor.warmCool,
        -18,
        18,
      ),
      magentaGreen: normaliseNumber(
        monochromaticColor.magentaGreen,
        emptySettings.monochromaticColor.magentaGreen,
        -18,
        18,
      ),
    },
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
    imageSize: imageSizes.includes(imageSizeValue as (typeof imageSizes)[number])
      ? (imageSizeValue as (typeof imageSizes)[number])
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
    highlight: normaliseNumber(settings.highlight, emptySettings.highlight, -2, 4, 0.5),
    shadow: normaliseNumber(settings.shadow, emptySettings.shadow, -2, 4, 0.5),
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
    isoAutoMaximum: isoAutoMaximums.includes(
      shootingSettings.isoAutoMaximum as (typeof isoAutoMaximums)[number],
    )
      ? (shootingSettings.isoAutoMaximum as (typeof isoAutoMaximums)[number])
      : emptyShootingSettings.isoAutoMaximum,
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
        1 / 3,
      ),
  };
  return recipe;
}

export interface RecipeSource {
  author?: string;
  url?: string;
}

function normaliseEnum(value: string) {
  return optionValueFromText(value);
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
    else if (
      key.includes("film simulation") ||
      key.includes("底片模擬") ||
      key.includes("軟片模擬")
    )
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
      key.includes("chrome fx blue") ||
      key.includes("色彩漸變特效藍色")
    )
      recipe.settings.colorChromeFxBlue = normaliseEnum(
        value,
      ) as RecipeSettings["colorChromeFxBlue"];
    else if (key.includes("color chrome") || key.includes("彩色效果"))
      recipe.settings.colorChromeEffect = normaliseEnum(
        value,
      ) as RecipeSettings["colorChromeEffect"];
    else if (
      key.includes("smooth skin") ||
      key.includes("平滑膚色") ||
      key.includes("平滑皮膚")
    )
      recipe.settings.smoothSkinEffect = normaliseEnum(
        value,
      ) as RecipeSettings["smoothSkinEffect"];
    else if (
      (key.includes("monochromatic") &&
        !key.includes("magenta") &&
        !key.includes(" mg")) ||
      key.includes("mono warm") ||
      key.includes("黑白暖冷")
    )
      recipe.settings.monochromaticColor = {
        ...recipe.settings.monochromaticColor,
        warmCool:
          parseSignedNumber(value) ?? recipe.settings.monochromaticColor.warmCool,
      };
    else if (
      key.includes("mono magenta") ||
      key.includes("monochromatic mg") ||
      key.includes("黑白洋紅")
    )
      recipe.settings.monochromaticColor = {
        ...recipe.settings.monochromaticColor,
        magentaGreen:
          parseSignedNumber(value) ?? recipe.settings.monochromaticColor.magentaGreen,
      };
    else if (
      key.includes("high iso") ||
      key === "nr" ||
      key.includes("雜訊") ||
      key.includes("降噪")
    )
      recipe.settings.highIsoNoiseReduction =
        parseSignedNumber(value) ?? recipe.settings.highIsoNoiseReduction;
    else if (
      ["highlight", "highlights", "高光", "亮部", "亮部色調"].includes(key)
    )
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

/**
 * Imports either one portable Recipe object or a JSON array of Recipes. A
 * collection is deliberately JSON-only: line-oriented text remains one
 * Recipe, preventing a pasted prose note from unexpectedly creating items.
 */
export function parseRecipeCollectionText(
  text: string,
  source: RecipeSource = {},
): Recipe[] {
  const trimmed = text.trim();
  if (!trimmed.startsWith("[")) return [parseRecipeText(text, source)];
  const parsed: unknown = JSON.parse(trimmed);
  if (!Array.isArray(parsed) || parsed.length === 0) {
    throw new Error("Recipe collection must contain at least one item");
  }
  return parsed.map(normalizeRecipe);
}
