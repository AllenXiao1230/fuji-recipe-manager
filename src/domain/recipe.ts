export const filmSimulations = [
  "AUTO",
  "PROVIA",
  "VELVIA",
  "ASTIA",
  "CLASSIC_CHROME",
  "REALA_ACE",
  "PRO_NEG_HI",
  "PRO_NEG_STD",
  "CLASSIC_NEGATIVE",
  "NOSTALGIC_NEGATIVE",
  "ETERNA",
  "ETERNA_BLEACH_BYPASS",
  "ACROS",
  "ACROS_YE",
  "ACROS_R",
  "ACROS_G",
  "MONOCHROME",
  "MONOCHROME_YE",
  "MONOCHROME_R",
  "MONOCHROME_G",
  "SEPIA",
] as const;
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
  "M_3_2_1_25X_CROP",
  "M_16_9_1_25X_CROP",
  "M_1_1_1_25X_CROP",
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

export const xm5WritableFilmSimulations = [
  "PROVIA",
  "VELVIA",
  "ASTIA",
  "CLASSIC_CHROME",
  "ACROS",
  "ETERNA",
  "CLASSIC_NEGATIVE",
  "REALA_ACE",
] as const;
export const xm5WritableDynamicRanges = ["DR100", "DR200", "DR400"] as const;
export const xm5WritableWhiteBalances = [
  "AUTO",
  "DAYLIGHT",
  "INCANDESCENT",
  "UNDERWATER",
  "FLUORESCENT_1",
  "FLUORESCENT_2",
  "FLUORESCENT_3",
  "SHADE",
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
  exposureCompensation: number;
  meteringMode: (typeof meteringModes)[number];
  focusMode: (typeof focusModes)[number];
  afMode: (typeof afModes)[number];
  driveMode: (typeof driveModes)[number];
  shutterType: (typeof shutterTypes)[number];
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
  return blockers;
}

export const emptySettings: RecipeSettings = {
  filmSimulation: "CLASSIC_CHROME",
  dynamicRange: "DR400",
  whiteBalance: { mode: "AUTO", colorTemperatureK: 6500, shiftR: 0, shiftB: 0 },
  grain: { strength: "OFF", size: "SMALL" },
  colorChromeEffect: "OFF",
  colorChromeFxBlue: "OFF",
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
