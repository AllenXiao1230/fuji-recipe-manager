import { invoke } from "@tauri-apps/api/core";
import type { Recipe, RecipeSettings } from "../domain/recipe";

export interface CameraDiscovery {
  usbId: string;
  manufacturer?: string;
  product?: string;
  isFujifilm: boolean;
  family?: string;
  profile?: string;
  recipeAccess?: string;
  ptpInterfaceDetected: boolean;
  writeEnabled: boolean;
}
export interface PtpDeviceInfoResult {
  usbId: string;
  interfaceNumber: number;
  bulkInEndpoint: string;
  bulkOutEndpoint: string;
  dataBytes: number;
  responseCode: string;
  manufacturer: string;
  model: string;
  deviceVersion: string;
}
export interface PtpPropertyValueResult {
  usbId: string;
  interfaceNumber: number;
  propertyCode: string;
  valueHex: string;
}
export interface SlotSelectionResult {
  usbId: string;
  slot: number;
  readBackHex: string;
}
export interface RecipeWriteResult {
  slot: number;
  verifiedProperties: string[];
  backupId: string;
  journalId: string;
  presetName: string;
}
export interface InstalledPreset {
  slot: number;
  name: string;
}
export interface CameraBackupSummary {
  id: string;
  usbId: string;
  slot: number;
  capturedAt: number;
  propertyCount: number;
}

export async function discoverCameras(): Promise<CameraDiscovery[]> {
  if (!("__TAURI_INTERNALS__" in window)) return [];
  return invoke<CameraDiscovery[]>("discover_cameras");
}

export async function probeCameraDeviceInfo(
  usbId: string,
): Promise<PtpDeviceInfoResult> {
  return invoke<PtpDeviceInfoResult>("probe_camera_device_info", { usbId });
}

export async function readCameraSlotSelector(
  usbId: string,
): Promise<PtpPropertyValueResult> {
  return invoke<PtpPropertyValueResult>("read_camera_slot_selector", { usbId });
}

export async function selectCameraSlot(
  usbId: string,
  slot: number,
): Promise<SlotSelectionResult> {
  return invoke<SlotSelectionResult>("select_camera_slot", { usbId, slot });
}

export async function writeXm5Recipe(
  usbId: string,
  slot: number,
  settings: RecipeSettings,
  name: string,
): Promise<RecipeWriteResult> {
  return invoke<RecipeWriteResult>("write_xm5_recipe", {
    usbId,
    slot,
    settings,
    name,
  });
}

export async function readXm5InstalledPresets(
  usbId: string,
): Promise<InstalledPreset[]> {
  return invoke<InstalledPreset[]>("read_xm5_installed_presets", { usbId });
}

export async function listCameraBackups(
  usbId: string,
): Promise<CameraBackupSummary[]> {
  return invoke<CameraBackupSummary[]>("list_camera_backups", { usbId });
}

export async function restoreXm5Backup(
  usbId: string,
  backupId: string,
): Promise<CameraBackupSummary> {
  return invoke<CameraBackupSummary>("restore_xm5_backup", { usbId, backupId });
}

export const isDesktop = "__TAURI_INTERNALS__" in window;

export async function loadDesktopRecipes(): Promise<Recipe[]> {
  return invoke<Recipe[]>("list_recipes");
}

export async function saveDesktopRecipes(recipes: Recipe[]): Promise<void> {
  return invoke("replace_recipes", { recipes });
}
