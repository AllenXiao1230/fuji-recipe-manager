import {
  makeRecipe,
  type FpInterchangeMetadata,
  type Recipe,
  type RecipeSettings,
} from "../domain/recipe";
import { optionValueFromText } from "./i18n";
import { normalizeRecipe } from "./recipeCodec";

export type FpFormat = "FP1" | "FP2" | "FP3";

export type FpInterchangeReport = {
  format: FpFormat;
  mappedFields: string[];
  unmappedProperties: Array<{ name: string; value: string }>;
  omittedSensitiveProperties: string[];
  warnings: string[];
};

export type ParsedFpProfile = {
  recipe: Recipe;
  report: FpInterchangeReport;
};

const knownPropertyNames = new Set([
  "TetherRAWConditonCode",
  "Editable",
  "SourceFileName",
  "Fileerror",
  "RotationAngle",
  "StructVer",
  "IOPCode",
  "ShootingCondition",
  "FileType",
  "ImageSize",
  "ImageQuality",
  "ExposureBias",
  "DynamicRange",
  "WideDRange",
  "FilmSimulation",
  "BlackImageTone",
  "MonochromaticColor_RG",
  "GrainEffect",
  "GrainEffectSize",
  "ChromeEffect",
  "ColorChromeBlue",
  "SmoothSkinEffect",
  "WBShootCond",
  "WhiteBalance",
  "WBShiftR",
  "WBShiftB",
  "WBColorTemp",
  "HighlightTone",
  "ShadowTone",
  "Color",
  "Sharpness",
  "NoisReduction",
  "Clarity",
  "LensModulationOpt",
  "ColorSpace",
  "HDR",
  "DigitalTeleConv",
  "PortraitEnhancer",
]);

function normaliseKey(value: string) {
  return value.toUpperCase().replace(/[^A-Z0-9]+/g, "");
}

function formatFromFilename(filename: string): FpFormat {
  const extension = filename.split(".").pop()?.toUpperCase();
  if (extension === "FP2" || extension === "FP3") return extension;
  return "FP1";
}

function parseNumber(value: string) {
  const number = Number(value.replace(/K$/i, "").trim());
  return Number.isFinite(number) ? number : undefined;
}

function enumValue(value: string) {
  return optionValueFromText(value.trim().replace(/\s+/g, "_"));
}

function imageSizeValue(value: string): RecipeSettings["imageSize"] | undefined {
  const values: Record<string, RecipeSettings["imageSize"]> = {
    L3X2: "L_3_2",
    L16X9: "L_16_9",
    L1X1: "L_1_1",
    M3X2: "M_3_2",
    M16X9: "M_16_9",
    M1X1: "M_1_1",
    S3X2: "S_3_2",
    S16X9: "S_16_9",
    S1X1: "S_1_1",
  };
  return values[normaliseKey(value)];
}

function filmSimulationValue(value: string): RecipeSettings["filmSimulation"] | undefined {
  const values: Record<string, RecipeSettings["filmSimulation"]> = {
    PROVIA: "PROVIA",
    STANDARD: "PROVIA",
    PROVIASTANDARD: "PROVIA",
    VELVIA: "VELVIA",
    VIVID: "VELVIA",
    ASTIA: "ASTIA",
    SOFT: "ASTIA",
    PRONEGHI: "PRO_NEG_HI",
    PRONEGSTD: "PRO_NEG_STD",
    CLASSIC: "CLASSIC_CHROME",
    CLASSICCHROME: "CLASSIC_CHROME",
    CLASSICNEG: "CLASSIC_NEGATIVE",
    CLASSICNEGATIVE: "CLASSIC_NEGATIVE",
    NOSTALGICNEG: "NOSTALGIC_NEGATIVE",
    NOSTALGICNEGATIVE: "NOSTALGIC_NEGATIVE",
    ETERNA: "ETERNA",
    ETERNACINEMA: "ETERNA",
    ETERNABLEACHBYPASS: "ETERNA_BLEACH_BYPASS",
    ACROS: "ACROS",
    ACROSYE: "ACROS_YE",
    ACROSR: "ACROS_R",
    ACROSG: "ACROS_G",
    MONOCHROME: "MONOCHROME",
    MONOCHROMEYE: "MONOCHROME_YE",
    MONOCHROMER: "MONOCHROME_R",
    MONOCHROMEG: "MONOCHROME_G",
    SEPIA: "SEPIA",
    REALAACE: "REALA_ACE",
  };
  return values[normaliseKey(value)];
}

function whiteBalanceValue(value: string): RecipeSettings["whiteBalance"]["mode"] | undefined {
  const values: Record<string, RecipeSettings["whiteBalance"]["mode"]> = {
    AUTO: "AUTO",
    DAYLIGHT: "DAYLIGHT",
    SHADE: "SHADE",
    INCANDESCENT: "INCANDESCENT",
    UNDERWATER: "UNDERWATER",
    COLORTEMPERATURE: "COLOR_TEMPERATURE",
    FLUORESCENT1: "FLUORESCENT_1",
    FLUORESCENT2: "FLUORESCENT_2",
    FLUORESCENT3: "FLUORESCENT_3",
  };
  return values[normaliseKey(value)];
}

function strengthValue(value: string): RecipeSettings["grain"]["strength"] | undefined {
  const option = enumValue(value);
  return option === "OFF" || option === "WEAK" || option === "STRONG"
    ? option
    : undefined;
}

function colorSpaceValue(value: string): RecipeSettings["colorSpace"] | undefined {
  const key = normaliseKey(value);
  return key === "SRGB" ? "SRGB" : key === "ADOBERGB" ? "ADOBE_RGB" : undefined;
}

function qualityValue(value: string): RecipeSettings["imageQuality"] | undefined {
  const key = normaliseKey(value);
  const values: Record<string, RecipeSettings["imageQuality"]> = {
    FINE: "FINE",
    NORMAL: "NORMAL",
    FINERAW: "FINE_PLUS_RAW",
    NORMALRAW: "NORMAL_PLUS_RAW",
    RAW: "RAW",
  };
  return values[key];
}

function setNumeric(
  recipe: Recipe,
  field: "highlight" | "shadow" | "color" | "sharpness" | "highIsoNoiseReduction" | "clarity",
  value: string,
) {
  const number = parseNumber(value);
  if (number !== undefined) recipe.settings[field] = number;
}

function directPropertyChildren(group: Element) {
  return Array.from(group.children).flatMap((element) =>
    element.tagName === "RejectedValue"
      ? Array.from(element.children).map((child) => ({
          name: `RejectedValue/${child.tagName}`,
          value: child.textContent?.trim() ?? "",
        }))
      : [{ name: element.tagName, value: element.textContent?.trim() ?? "" }],
  );
}

/** Parse X RAW Studio's XML FP1, FP2, and FP3 profile containers. */
export function parseFpProfile(xml: string, filename = "profile.FP1"): ParsedFpProfile {
  const document = new DOMParser().parseFromString(xml, "application/xml");
  const parserError = document.querySelector("parsererror");
  if (parserError) throw new Error("The FP profile is not valid XML.");
  const root = document.documentElement;
  if (root.tagName !== "ConversionProfile")
    throw new Error("The FP profile root must be ConversionProfile.");
  const group = Array.from(root.children).find(
    (child) => child.tagName === "PropertyGroup",
  );
  if (!group) throw new Error("The FP profile has no PropertyGroup.");

  const format = formatFromFilename(filename);
  const properties = directPropertyChildren(group);
  const propertyMap = new Map(
    properties
      .filter((property) => !property.name.startsWith("RejectedValue/"))
      .map((property) => [property.name, property.value]),
  );
  const recipe = makeRecipe(group.getAttribute("label")?.trim() || "Imported FP Recipe");
  const mappedFields: string[] = [];
  const map = (field: string, action: () => boolean) => {
    if (action()) mappedFields.push(field);
  };
  const value = (name: string) => propertyMap.get(name) ?? "";

  map("Film Simulation", () => {
    const simulation = filmSimulationValue(value("FilmSimulation"));
    if (!simulation) return false;
    recipe.settings.filmSimulation = simulation;
    return true;
  });
  map("Dynamic Range", () => {
    const number = parseNumber(value("DynamicRange"));
    if (number !== 100 && number !== 200 && number !== 400) return false;
    recipe.settings.dynamicRange = `DR${number}`;
    return true;
  });
  map("Image Size", () => {
    const imageSize = imageSizeValue(value("ImageSize"));
    if (!imageSize) return false;
    recipe.settings.imageSize = imageSize;
    return true;
  });
  map("Image Quality", () => {
    const imageQuality = qualityValue(value("ImageQuality"));
    if (!imageQuality) return false;
    recipe.settings.imageQuality = imageQuality;
    return true;
  });
  map("Grain Effect", () => {
    const strength = strengthValue(value("GrainEffect"));
    const size = enumValue(value("GrainEffectSize"));
    if (!strength || (size !== "SMALL" && size !== "LARGE")) return false;
    recipe.settings.grain = { strength, size };
    return true;
  });
  map("Color Chrome Effect", () => {
    const effect = strengthValue(value("ChromeEffect"));
    if (!effect) return false;
    recipe.settings.colorChromeEffect = effect;
    return true;
  });
  map("Color Chrome FX Blue", () => {
    const effect = strengthValue(value("ColorChromeBlue"));
    if (!effect) return false;
    recipe.settings.colorChromeFxBlue = effect;
    return true;
  });
  map("Smooth Skin Effect", () => {
    const effect = strengthValue(value("SmoothSkinEffect"));
    if (!effect) return false;
    recipe.settings.smoothSkinEffect = effect;
    return true;
  });
  for (const [name, field, label] of [
    ["BlackImageTone", "warmCool", "Monochromatic Warm/Cool"],
    ["MonochromaticColor_RG", "magentaGreen", "Monochromatic Magenta/Green"],
  ] as const) {
    map(label, () => {
      const number = parseNumber(value(name));
      if (number === undefined || number < -18 || number > 18) return false;
      recipe.settings.monochromaticColor = {
        ...recipe.settings.monochromaticColor,
        [field]: number,
      };
      return true;
    });
  }
  map("White Balance", () => {
    const mode = whiteBalanceValue(value("WhiteBalance"));
    if (!mode) return false;
    recipe.settings.whiteBalance = { ...recipe.settings.whiteBalance, mode };
    return true;
  });
  for (const [name, field] of [
    ["WBShiftR", "shiftR"],
    ["WBShiftB", "shiftB"],
  ] as const) {
    map(name, () => {
      const number = parseNumber(value(name));
      if (number === undefined) return false;
      recipe.settings.whiteBalance = { ...recipe.settings.whiteBalance, [field]: number };
      return true;
    });
  }
  map("Color Temperature", () => {
    const number = parseNumber(value("WBColorTemp"));
    if (number === undefined) return false;
    recipe.settings.whiteBalance = {
      ...recipe.settings.whiteBalance,
      colorTemperatureK: number,
    };
    return true;
  });
  for (const [name, field] of [
    ["HighlightTone", "highlight"],
    ["ShadowTone", "shadow"],
    ["Color", "color"],
    ["Sharpness", "sharpness"],
    ["NoisReduction", "highIsoNoiseReduction"],
    ["Clarity", "clarity"],
  ] as const) {
    map(name, () => {
      const number = parseNumber(value(name));
      if (number === undefined) return false;
      setNumeric(recipe, field, number.toString());
      return true;
    });
  }
  map("Color Space", () => {
    const colorSpace = colorSpaceValue(value("ColorSpace"));
    if (!colorSpace) return false;
    recipe.settings.colorSpace = colorSpace;
    return true;
  });

  const omittedSensitiveProperties = properties
    .filter((property) => property.name === "SerialNumber")
    .map((property) => property.name);
  const unmappedProperties = properties.filter(
    (property) =>
      property.name !== "SerialNumber" &&
      (!knownPropertyNames.has(property.name) || property.name.startsWith("RejectedValue/")),
  );
  const metadata: FpInterchangeMetadata = {
    format,
    application: root.getAttribute("application")?.trim() || "XRFC",
    profileVersion: root.getAttribute("version")?.trim() || "1.12.0.0",
    device: group.getAttribute("device")?.trim() || "",
    deviceVersion: group.getAttribute("version")?.trim() || "",
    sourceProperties: properties.filter((property) => property.name !== "SerialNumber"),
    unmappedProperties,
  };
  recipe.cameraCompatibility = metadata.device ? [metadata.device] : ["Fujifilm"];
  recipe.interoperability = { fp: metadata };
  const report: FpInterchangeReport = {
    format,
    mappedFields,
    unmappedProperties,
    omittedSensitiveProperties,
    warnings: [
      ...(format === "FP2" || format === "FP3"
        ? ["FP2/FP3 are image-linked X RAW Studio profiles; source-file fields are retained locally but do not make a Recipe writable to a camera."]
        : []),
      ...(unmappedProperties.length
        ? ["Unmapped XML properties are retained for FP export and are not sent to a camera."]
        : []),
    ],
  };
  return { recipe: normalizeRecipe(recipe), report };
}

function escapeXml(value: string) {
  return value.replace(/[<>&"']/g, (character) =>
    ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;", "'": "&apos;" })[
      character
    ]!,
  );
}

function fpFilmSimulation(value: RecipeSettings["filmSimulation"]) {
  const values: Partial<Record<RecipeSettings["filmSimulation"], string>> = {
    PROVIA: "Provia",
    VELVIA: "Velvia",
    ASTIA: "Astia",
    PRO_NEG_HI: "ProNegHi",
    PRO_NEG_STD: "ProNegStd",
    CLASSIC_CHROME: "Classic",
    MONOCHROME: "Monochrome",
    MONOCHROME_YE: "MonochromeYe",
    MONOCHROME_R: "MonochromeR",
    MONOCHROME_G: "MonochromeG",
    SEPIA: "Sepia",
    ACROS: "Acros",
    ACROS_YE: "AcrosYe",
    ACROS_R: "AcrosR",
    ACROS_G: "AcrosG",
    ETERNA: "Eterna",
    CLASSIC_NEGATIVE: "ClassicNeg",
    ETERNA_BLEACH_BYPASS: "EternaBleachBypass",
    NOSTALGIC_NEGATIVE: "NostalgicNeg",
    REALA_ACE: "RealaAce",
  };
  return values[value];
}

function fpImageSize(value: RecipeSettings["imageSize"]) {
  const values: Partial<Record<RecipeSettings["imageSize"], string>> = {
    L_3_2: "L3x2",
    L_16_9: "L16x9",
    L_1_1: "L1x1",
    M_3_2: "M3x2",
    M_16_9: "M16x9",
    M_1_1: "M1x1",
    S_3_2: "S3x2",
    S_16_9: "S16x9",
    S_1_1: "S1x1",
  };
  return values[value];
}

function fpImageQuality(value: RecipeSettings["imageQuality"]) {
  return value
    .replace("_PLUS_", "+")
    .split("_")
    .map((part) => part[0] + part.slice(1).toLowerCase())
    .join("");
}

function fpWhiteBalance(value: RecipeSettings["whiteBalance"]["mode"]) {
  const values: Partial<Record<RecipeSettings["whiteBalance"]["mode"], string>> = {
    AUTO: "Auto",
    DAYLIGHT: "Daylight",
    SHADE: "Shade",
    INCANDESCENT: "Incandescent",
    UNDERWATER: "Underwater",
    COLOR_TEMPERATURE: "ColorTemperature",
    FLUORESCENT_1: "Fluorescent1",
    FLUORESCENT_2: "Fluorescent2",
    FLUORESCENT_3: "Fluorescent3",
  };
  return values[value];
}

function fpColorSpace(value: RecipeSettings["colorSpace"]) {
  return value === "ADOBE_RGB" ? "AdobeRGB" : "sRGB";
}

function xmlProperty(name: string, value: string) {
  return value ? `        <${name}>${escapeXml(value)}</${name}>` : `        <${name}/>`;
}

/**
 * Create an XML FP profile. Existing FP imports preserve their X RAW Studio
 * device/version metadata and unmapped XML. New Recipes use an explicitly
 * marked generic profile which must be tested with the user's camera before it
 * is trusted by X RAW Studio.
 */
export function exportFpProfile(recipe: Recipe, format: FpFormat): {
  xml: string;
  report: FpInterchangeReport;
} {
  const template = recipe.interoperability?.fp;
  const base = template?.sourceProperties ?? [];
  const rejectedProperties = base.filter((property) =>
    property.name.startsWith("RejectedValue/"),
  );
  const values = new Map(
    base
      .filter((property) => !property.name.startsWith("RejectedValue/"))
      .map((property) => [property.name, property.value]),
  );
  const settings = recipe.settings;
  const mappedFields: string[] = [];
  const set = (name: string, value: string | undefined, field: string) => {
    if (value === undefined) return;
    values.set(name, value);
    mappedFields.push(field);
  };
  set("Editable", format === "FP3" ? "FALSE" : "TRUE", "Editable");
  set("ImageSize", fpImageSize(settings.imageSize), "Image Size");
  set("ImageQuality", fpImageQuality(settings.imageQuality), "Image Quality");
  set("DynamicRange", settings.dynamicRange.replace("DR", ""), "Dynamic Range");
  set("FilmSimulation", fpFilmSimulation(settings.filmSimulation), "Film Simulation");
  set("GrainEffect", settings.grain.strength, "Grain Effect");
  set("GrainEffectSize", settings.grain.size, "Grain Effect Size");
  set("ChromeEffect", settings.colorChromeEffect, "Color Chrome Effect");
  set("ColorChromeBlue", settings.colorChromeFxBlue, "Color Chrome FX Blue");
  set("SmoothSkinEffect", settings.smoothSkinEffect, "Smooth Skin Effect");
  const isMonochrome = [
    "ACROS",
    "ACROS_YE",
    "ACROS_R",
    "ACROS_G",
    "MONOCHROME",
    "MONOCHROME_YE",
    "MONOCHROME_R",
    "MONOCHROME_G",
    "SEPIA",
  ].includes(settings.filmSimulation);
  if (isMonochrome || values.has("BlackImageTone")) {
    set(
      "BlackImageTone",
      String(settings.monochromaticColor.warmCool),
      "Monochromatic Warm/Cool",
    );
  }
  if (isMonochrome || values.has("MonochromaticColor_RG")) {
    set(
      "MonochromaticColor_RG",
      String(settings.monochromaticColor.magentaGreen),
      "Monochromatic Magenta/Green",
    );
  }
  set("WhiteBalance", fpWhiteBalance(settings.whiteBalance.mode), "White Balance");
  set("WBShiftR", String(settings.whiteBalance.shiftR), "WB Shift R");
  set("WBShiftB", String(settings.whiteBalance.shiftB), "WB Shift B");
  set("WBColorTemp", `${settings.whiteBalance.colorTemperatureK}K`, "Color Temperature");
  set("HighlightTone", String(settings.highlight), "Highlight Tone");
  set("ShadowTone", String(settings.shadow), "Shadow Tone");
  set("Color", String(settings.color), "Color");
  set("Sharpness", String(settings.sharpness), "Sharpness");
  set("NoisReduction", String(settings.highIsoNoiseReduction), "High ISO NR");
  set("Clarity", String(settings.clarity), "Clarity");
  set("ColorSpace", fpColorSpace(settings.colorSpace), "Color Space");

  const sensitive = ["SerialNumber"];
  for (const name of sensitive) values.delete(name);
  const device = template?.device || recipe.cameraCompatibility[0] || "X-M5";
  const deviceVersion = template?.deviceVersion || "";
  const application = template?.application || "XRFC";
  const profileVersion = template?.profileVersion || "1.12.0.0";
  const properties = Array.from(values, ([name, value]) => xmlProperty(name, value));
  if (rejectedProperties.length) {
    properties.push("        <RejectedValue>");
    for (const property of rejectedProperties) {
      const name = property.name.slice("RejectedValue/".length);
      if (/^[A-Za-z][A-Za-z0-9_.-]*$/.test(name))
        properties.push(`            ${xmlProperty(name, property.value).trim()}`);
    }
    properties.push("        </RejectedValue>");
  }
  const warnings = [
    ...(template
      ? []
      : ["This Recipe has no imported X RAW Studio template. Device/version metadata is generic and must be checked in X RAW Studio before use."]),
    ...(format === "FP2" || format === "FP3"
      ? ["FP2/FP3 exports contain Recipe settings only; they do not create or link a RAF image conversion job."]
      : []),
  ];
  return {
    xml: [
      '<?xml version="1.0" encoding="utf-8"?>',
      `<ConversionProfile application="${escapeXml(application)}" version="${escapeXml(profileVersion)}">`,
      `    <PropertyGroup device="${escapeXml(device)}" version="${escapeXml(deviceVersion)}" label="${escapeXml(recipe.name)}">`,
      ...properties,
      "    </PropertyGroup>",
      "</ConversionProfile>",
      "",
    ].join("\n"),
    report: {
      format,
      mappedFields,
      unmappedProperties: template?.unmappedProperties ?? [],
      omittedSensitiveProperties: sensitive,
      warnings,
    },
  };
}
