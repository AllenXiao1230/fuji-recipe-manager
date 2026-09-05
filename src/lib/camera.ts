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
export interface CameraCapabilityCatalogEntry {
  model: string;
  family: string;
  access:
    | "Probe required"
    | "Read-only PTP verified"
    | "Experimental recipe writes"
    | "Recipe writes verified";
  trustedRecordId?: string;
  trustedFirmware?: string;
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
  capabilityStateKey:
    | "exact_experimental"
    | "firmware_probe_only"
    | "probe_only"
    | "not_fujifilm";
  capabilityState: string;
  capabilityRecordId?: string;
  capabilityNextAction: string;
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
  usbId: string;
  slot: number;
  verifiedProperties: string[];
  backupId: string;
  journalId: string;
  presetName: string;
}
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
export type CameraPropertyStatus =
  | "write_verified"
  | "write_rejected"
  | "read_detected_unverified"
  | "blocked_unknown";
export interface CameraCapabilityProperty {
  key: string;
  code: string;
  scope: "custom_slot" | "global" | "unknown";
  labelZh: string;
  labelEn: string;
  status: CameraPropertyStatus;
  verifiedValues: string;
  dependencies: string[];
  notes: string;
}
export interface CameraCapabilityRecord {
  schemaVersion: number;
  recordId: string;
  manufacturer: string;
  model: string;
  firmware: string;
  usbIds: string[];
  customSlots: string[];
  validation: { method: string; testedOn: string; scope: string };
  properties: CameraCapabilityProperty[];
}
export type LocalCapabilityPropertyStatus =
  | "read_detected_unverified"
  | "write_rejected"
  | "blocked_unknown";
export interface LocalCapabilityMatrixProperty {
  key: string;
  labelZh: string;
  labelEn: string;
  code: string;
  scope: "custom_slot" | "global" | "unknown";
  status: LocalCapabilityPropertyStatus;
  notes: string;
}
/**
 * Locally documented observations for a camera identity. These records cannot
 * enable PTP writes; only a trusted, hardware-verified capability record can.
 */
export interface LocalCapabilityMatrix {
  schemaVersion: 1;
  id: string;
  manufacturer: "FUJIFILM";
  model: string;
  firmware: string;
  usbIds: string[];
  customSlots: string[];
  sourceUrl?: string;
  sourceKind: "official" | "community" | "local_probe";
  evidenceSummary: string;
  lastVerifiedAt?: number;
  createdAt: number;
  updatedAt: number;
  properties: LocalCapabilityMatrixProperty[];
}
export interface ImageRecipeFieldStatus {
  key: string;
  status: "recognized" | "unavailable";
  detail: string;
}
export interface ImageRecipeImport {
  fileName: string;
  model: string;
  settings: Record<string, unknown>;
  shootingSettings: Record<string, unknown>;
  fieldStatuses: ImageRecipeFieldStatus[];
  warnings: string[];
}
export interface RafPreviewStageResult {
  state: "staged";
  recoveryAction?: string;
}
export interface WriteJournalSummary {
  id: string;
  backupId: string;
  usbId: string;
  slot: number;
  createdAt: number;
  state: "writing" | "recovery_failed";
  error?: string;
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

/**
 * Captures readable custom-slot PTP values for audit/interchange only. The
 * Rust command never uses these bytes as a future write payload.
 */
export async function captureXm5RawPresetSnapshot(
  usbId: string,
  slot: number,
): Promise<RawPtpPresetSnapshot> {
  const snapshot = await invoke<RawPtpPresetSnapshot>(
    "capture_xm5_raw_preset_snapshot",
    {
      usbId,
      slot,
    },
  );
  const capturedAtMillis = Number(snapshot.capturedAt);
  if (!Number.isFinite(capturedAtMillis)) {
    throw new Error("camera returned an invalid raw snapshot timestamp");
  }
  return {
    ...snapshot,
    capturedAt: new Date(capturedAtMillis).toISOString(),
  };
}

export async function getXm5CapabilityRecord(): Promise<CameraCapabilityRecord> {
  return invoke<CameraCapabilityRecord>("xm5_capability_record");
}

export async function listCameraCapabilityCatalog(): Promise<
  CameraCapabilityCatalogEntry[]
> {
  return invoke<CameraCapabilityCatalogEntry[]>("list_camera_capability_catalog");
}

export async function listLocalCapabilityMatrices(): Promise<
  LocalCapabilityMatrix[]
> {
  return invoke<LocalCapabilityMatrix[]>("list_local_capability_matrices");
}

export async function saveLocalCapabilityMatrix(
  matrix: LocalCapabilityMatrix,
): Promise<LocalCapabilityMatrix> {
  return invoke<LocalCapabilityMatrix>("save_local_capability_matrix", {
    matrix,
  });
}

export async function deleteLocalCapabilityMatrix(id: string): Promise<void> {
  return invoke<void>("delete_local_capability_matrix", { id });
}

export async function importFujifilmImageRecipe(
  path: string,
): Promise<ImageRecipeImport> {
  return invoke<ImageRecipeImport>("import_fujifilm_image_recipe", { path });
}

export async function stageRafPreview(
  path: string,
  recipeId: string,
): Promise<RafPreviewStageResult> {
  return invoke<RafPreviewStageResult>("stage_raf_preview", { path, recipeId });
}

export async function clearXm5CustomSlot(
  usbId: string,
  slot: number,
): Promise<RecipeWriteResult> {
  return invoke<RecipeWriteResult>("clear_xm5_custom_slot", { usbId, slot });
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

export async function listPendingWriteJournals(): Promise<WriteJournalSummary[]> {
  return invoke<WriteJournalSummary[]>("list_pending_write_journals");
}

export const isDesktop = "__TAURI_INTERNALS__" in window;

export async function loadDesktopRecipes(): Promise<Recipe[]> {
  return invoke<Recipe[]>("list_recipes");
}

export async function saveDesktopRecipes(recipes: Recipe[]): Promise<void> {
  return invoke("upsert_recipes", { recipes });
}

export async function deleteDesktopRecipe(
  id: string,
  deletedAt: string,
): Promise<void> {
  return invoke("delete_recipe", { id, deletedAt });
}
