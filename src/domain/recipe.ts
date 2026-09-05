/**
 * The 20 concrete Fujifilm film simulations validated for X-M5 firmware 1.30.
 * `AUTO` is deliberately excluded: it is a camera mode, not a reusable Recipe
 * look, and its custom-slot encoding has not been write-verified.
 */
export const supportedFilmSimulations = [
  "PROVIA",
  "VELVIA",
  "ASTIA",
  "PRO_NEG_HI",
  "PRO_NEG_STD",
  "MONOCHROME",
  "MONOCHROME_YE",
  "MONOCHROME_R",
  "MONOCHROME_G",
  "SEPIA",
  "CLASSIC_CHROME",
  "ACROS",
  "ACROS_YE",
  "ACROS_R",
  "ACROS_G",
  "ETERNA",
  "CLASSIC_NEGATIVE",
  "ETERNA_BLEACH_BYPASS",
  "NOSTALGIC_NEGATIVE",
  "REALA_ACE",
] as const;

export const filmSimulations = ["AUTO", ...supportedFilmSimulations] as const;
export type FilmSimulation = (typeof filmSimulations)[number];
export const dynamicRanges = ["AUTO", "DR100", "DR200", "DR400"] as const;
export const strengths = ["OFF", "WEAK", "STRONG"] as const;
export const grainSizes = ["SMALL", "LARGE"] as const;
export const whiteBalances = [
  "WHITE_PRIORITY",
  "AUTO",
  "AMBIENCE_PRIORITY",
  "CUSTOM_1",
  "CUSTOM_2",
  "CUSTOM_3",
  "COLOR_TEMPERATURE",
  "DAYLIGHT",
  "SHADE",
  "FLUORESCENT_1",
  "FLUORESCENT_2",
  "FLUORESCENT_3",
  "INCANDESCENT",
  "UNDERWATER",
] as const;
export const dRangePriorities = ["OFF", "AUTO", "WEAK", "STRONG"] as const;
export const portraitEnhancerLevels = [
  "OFF",
  "WEAK",
  "MEDIUM",
  "STRONG",
] as const;
export const onOff = ["ON", "OFF"] as const;
export const colorSpaces = ["SRGB", "ADOBE_RGB"] as const;
export const imageSizes = [
  "L_3_2",
  "L_16_9",
  "L_1_1",
  "M_3_2",
  "M_16_9",
  "M_1_1",
  "S_3_2",
  "S_16_9",
  "S_1_1",
  "P_3_2_1_25X_CROP",
  "P_16_9_1_25X_CROP",
  "P_1_1_1_25X_CROP",
] as const;
export const imageQualities = [
  "FINE",
  "NORMAL",
  "FINE_PLUS_RAW",
  "NORMAL_PLUS_RAW",
  "RAW",
] as const;
export const rawRecordingOptions = [
  "UNCOMPRESSED",
  "LOSSLESS_COMPRESSED",
  "COMPRESSED",
] as const;
export const stillFormats = ["JPEG", "HEIF"] as const;
export const isoSensitivities = [
  "AUTO_1",
  "AUTO_2",
  "AUTO_3",
  "ISO_80",
  "ISO_100",
  "ISO_125",
  "ISO_160",
  "ISO_200",
  "ISO_250",
  "ISO_320",
  "ISO_400",
  "ISO_500",
  "ISO_640",
  "ISO_800",
  "ISO_1000",
  "ISO_1250",
  "ISO_1600",
  "ISO_2000",
  "ISO_2500",
  "ISO_3200",
  "ISO_4000",
  "ISO_5000",
  "ISO_6400",
  "ISO_8000",
  "ISO_10000",
  "ISO_12800",
  "ISO_25600",
  "ISO_51200",
] as const;
export const isoAutoMaximums = [
  80, 100, 125, 160, 200, 250, 320, 400, 500, 640, 800, 1000, 1250, 1600,
  2000, 2500, 3200, 4000, 5000, 6400, 8000, 10000, 12800, 25600, 51200,
] as const;
export const meteringModes = [
  "MULTI",
  "SPOT",
  "AVERAGE",
  "CENTER_WEIGHTED",
] as const;
export const focusModes = [
  "SINGLE_AF",
  "CONTINUOUS_AF",
  "MANUAL_FOCUS",
] as const;
export const afModes = [
  "SINGLE_POINT",
  "ZONE",
  "WIDE_TRACKING",
  "ALL",
] as const;
export const driveModes = [
  "SINGLE",
  "CONTINUOUS_LOW",
  "CONTINUOUS_HIGH",
  "SELF_TIMER",
  "BRACKETING",
] as const;
export const shutterTypes = [
  "MECHANICAL",
  "ELECTRONIC",
  "ELECTRONIC_FRONT_CURTAIN",
  "MECHANICAL_PLUS_ELECTRONIC",
  "E_FRONT_PLUS_MECHANICAL",
  "E_FRONT_PLUS_MECHANICAL_PLUS_ELECTRONIC",
] as const;

export const xm5WritableFilmSimulations = supportedFilmSimulations;
export const xm5WritableDynamicRanges = ["AUTO", "DR100", "DR200", "DR400"] as const;
export const xm5WritableWhiteBalances = [
  "WHITE_PRIORITY",
  "AUTO",
  "AMBIENCE_PRIORITY",
  "COLOR_TEMPERATURE",
  "DAYLIGHT",
  "INCANDESCENT",
  "UNDERWATER",
  "FLUORESCENT_1",
  "FLUORESCENT_2",
  "FLUORESCENT_3",
  "SHADE",
] as const;
export const xm5WritableImageSizes = [
  "L_3_2",
  "L_16_9",
  "L_1_1",
  "M_3_2",
  "M_16_9",
  "M_1_1",
  "S_3_2",
  "S_16_9",
  "S_1_1",
] as const;
export const xm5WritableImageQualities = [
  "FINE",
  "NORMAL",
  "FINE_PLUS_RAW",
  "NORMAL_PLUS_RAW",
  "RAW",
] as const;

export interface RecipeSettings {
  filmSimulation: FilmSimulation;
  dynamicRange: (typeof dynamicRanges)[number];
  whiteBalance: {
    mode: (typeof whiteBalances)[number];
    colorTemperatureK: number;
    shiftR: number;
    shiftB: number;
  };
  grain: {
    strength: (typeof strengths)[number];
    size: (typeof grainSizes)[number];
  };
  colorChromeEffect: (typeof strengths)[number];
  colorChromeFxBlue: (typeof strengths)[number];
  smoothSkinEffect: (typeof strengths)[number];
  /**
   * Stored for FP/Recipe interchange only on X-M5 firmware 1.30. The camera
   * rejected every tested PTP encoding for these controls, so the X-M5 codec
   * blocks non-zero values rather than silently dropping them.
   */
  monochromaticColor: {
    warmCool: number;
    magentaGreen: number;
  };
  dRangePriority: (typeof dRangePriorities)[number];
  portraitEnhancer: (typeof portraitEnhancerLevels)[number];
  longExposureNoiseReduction: (typeof onOff)[number];
  lensModulationOptimizer: (typeof onOff)[number];
  colorSpace: (typeof colorSpaces)[number];
  imageSize: (typeof imageSizes)[number];
  imageQuality: (typeof imageQualities)[number];
  rawRecording: (typeof rawRecordingOptions)[number];
  stillFormat: (typeof stillFormats)[number];
  highlight: number;
  shadow: number;
  color: number;
  sharpness: number;
  highIsoNoiseReduction: number;
  clarity: number;
}

export interface ShootingSettings {
  isoSensitivity: (typeof isoSensitivities)[number];
  isoAutoMaximum: (typeof isoAutoMaximums)[number];
  exposureCompensation: number;
  meteringMode: (typeof meteringModes)[number];
  focusMode: (typeof focusModes)[number];
  afMode: (typeof afModes)[number];
  driveMode: (typeof driveModes)[number];
  shutterType: (typeof shutterTypes)[number];
}

/**
 * Loss-aware interchange metadata. It is optional so existing JSON Recipes
 * remain valid, but FP imports retain their X RAW Studio identity and any
 * unmodelled XML properties for a later FP round trip. Serial numbers are
 * deliberately excluded before this object is persisted.
 */
export interface FpInterchangeMetadata {
  format: "FP1" | "FP2" | "FP3";
  application: string;
  profileVersion: string;
  device: string;
  deviceVersion: string;
  sourceProperties: Array<{ name: string; value: string }>;
  unmappedProperties: Array<{ name: string; value: string }>;
}

/**
 * A byte-for-byte record of the readable vendor PTP properties in one custom
 * slot. It is intentionally an interchange/audit artifact, not a write
 * payload: unknown raw values must never flow into the camera writer.
 */
export interface RawPtpPresetSnapshot {
  manufacturer: "FUJIFILM";
  model: string;
  firmware: string;
  usbId: string;
  slot: number;
  capturedAt: string;
  restorationPolicy: "read_only_preserved";
  properties: Array<{ code: string; valueHex: string }>;
  unreadablePropertyCodes: string[];
}

export interface RecipeInteroperability {
  fp?: FpInterchangeMetadata;
  rawPtpPresetSnapshot?: RawPtpPresetSnapshot;
}

export interface Recipe {
  schemaVersion: 1;
  id: string;
  name: string;
  description: string;
  tags: string[];
  favorite: boolean;
  cameraCompatibility: string[];
  createdAt: string;
  updatedAt: string;
  source: {
    author: string;
    url: string;
  };
  interoperability?: RecipeInteroperability;
  settings: RecipeSettings;
  shootingSettings: ShootingSettings;
}

/**
 * UI-side preflight for the experimental X-M5 codec. The Rust encoder makes
 * the authoritative check again before it opens a write transaction.
 */
export function xm5WriteBlockers(settings: RecipeSettings): string[] {
  const blockers: string[] = [];
  if (!xm5WritableFilmSimulations.includes(settings.filmSimulation as never))
    blockers.push("Film Simulation");
  if (!xm5WritableDynamicRanges.includes(settings.dynamicRange as never))
    blockers.push("Dynamic Range");
  if (!xm5WritableWhiteBalances.includes(settings.whiteBalance.mode as never))
    blockers.push("White Balance");
  if (
    settings.whiteBalance.mode === "COLOR_TEMPERATURE" &&
    (settings.whiteBalance.colorTemperatureK < 2500 ||
      settings.whiteBalance.colorTemperatureK > 10000 ||
      settings.whiteBalance.colorTemperatureK % 10 !== 0)
  )
    blockers.push("Color Temperature");
  if (!strengths.includes(settings.smoothSkinEffect))
    blockers.push("Smooth Skin Effect");
  if (
    settings.monochromaticColor.warmCool !== 0 ||
    settings.monochromaticColor.magentaGreen !== 0
  )
    blockers.push("Monochromatic Color");
  if (!colorSpaces.includes(settings.colorSpace)) blockers.push("Color Space");
  if (!xm5WritableImageSizes.includes(settings.imageSize as never))
    blockers.push("Image Size");
  if (!xm5WritableImageQualities.includes(settings.imageQuality as never))
    blockers.push("Image Quality");
  return blockers;
}

export const emptySettings: RecipeSettings = {
  filmSimulation: "CLASSIC_CHROME",
  dynamicRange: "DR400",
  whiteBalance: { mode: "AUTO", colorTemperatureK: 6500, shiftR: 0, shiftB: 0 },
  grain: { strength: "OFF", size: "SMALL" },
  colorChromeEffect: "OFF",
  colorChromeFxBlue: "OFF",
  smoothSkinEffect: "OFF",
  monochromaticColor: { warmCool: 0, magentaGreen: 0 },
  dRangePriority: "OFF",
  portraitEnhancer: "OFF",
  longExposureNoiseReduction: "OFF",
  lensModulationOptimizer: "ON",
  colorSpace: "SRGB",
  imageSize: "L_3_2",
  imageQuality: "FINE",
  rawRecording: "LOSSLESS_COMPRESSED",
  stillFormat: "JPEG",
  highlight: 0,
  shadow: 0,
  color: 0,
  sharpness: 0,
  highIsoNoiseReduction: 0,
  clarity: 0,
};

export const emptyShootingSettings: ShootingSettings = {
  isoSensitivity: "AUTO_1",
  isoAutoMaximum: 6400,
  exposureCompensation: 0,
  meteringMode: "MULTI",
  focusMode: "SINGLE_AF",
  afMode: "SINGLE_POINT",
  driveMode: "SINGLE",
  shutterType: "MECHANICAL",
};

export function makeRecipe(name = "Untitled Recipe"): Recipe {
  const now = new Date().toISOString();
  return {
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name,
    description: "",
    tags: [],
    favorite: false,
    cameraCompatibility: ["Fujifilm"],
    createdAt: now,
    updatedAt: now,
    source: { author: "", url: "" },
    settings: structuredClone(emptySettings),
    shootingSettings: structuredClone(emptyShootingSettings),
  };
}

export function display(value: string) {
  return value
    .replaceAll("_", " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}
