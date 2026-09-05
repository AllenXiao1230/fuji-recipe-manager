use std::{
    fs,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use camera_core::UsbId;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use usb_transport::UsbBackend;

struct LibraryDb(Mutex<Connection>);

static RECORD_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const LIBRARY_DB_SCHEMA_VERSION: i64 = 2;

fn default_local_matrix_schema_version() -> u32 {
    1
}

fn default_local_matrix_source_kind() -> String {
    "local_probe".to_string()
}

#[derive(Deserialize)]
struct RecipeDocument(serde_json::Value);

/// A user-maintained, local-only capability matrix. It is intentionally kept
/// separate from the compiled capability record that gates camera writes:
/// saving a matrix can document observations, but can never grant write access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalCapabilityMatrix {
    #[serde(default = "default_local_matrix_schema_version")]
    schema_version: u32,
    id: String,
    manufacturer: String,
    model: String,
    firmware: String,
    usb_ids: Vec<String>,
    custom_slots: Vec<String>,
    source_url: Option<String>,
    #[serde(default = "default_local_matrix_source_kind")]
    source_kind: String,
    #[serde(default)]
    evidence_summary: String,
    #[serde(default)]
    last_verified_at: Option<u128>,
    created_at: u128,
    updated_at: u128,
    properties: Vec<LocalCapabilityMatrixProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalCapabilityMatrixProperty {
    key: String,
    label_zh: String,
    label_en: String,
    code: String,
    scope: String,
    status: String,
    notes: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CameraDiscovery {
    usb_id: String,
    manufacturer: Option<String>,
    product: Option<String>,
    is_fujifilm: bool,
    family: Option<&'static str>,
    profile: Option<&'static str>,
    recipe_access: Option<&'static str>,
    ptp_interface_detected: bool,
    write_enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CameraCapabilityCatalogEntry {
    model: &'static str,
    family: &'static str,
    access: &'static str,
    trusted_record_id: Option<String>,
    trusted_firmware: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PtpDeviceInfoResult {
    usb_id: String,
    interface_number: u8,
    bulk_in_endpoint: String,
    bulk_out_endpoint: String,
    data_bytes: usize,
    response_code: String,
    manufacturer: String,
    model: String,
    device_version: String,
    capability_state_key: String,
    capability_state: String,
    capability_record_id: Option<String>,
    capability_next_action: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PtpPropertyValueResult {
    usb_id: String,
    interface_number: u8,
    property_code: String,
    value_hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Xm5UnverifiedPropertyAuditResult {
    usb_id: String,
    model: String,
    firmware: String,
    properties: Vec<Xm5UnverifiedPropertyAuditEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Xm5UnverifiedPropertyAuditEntry {
    property_code: String,
    key: String,
    label: String,
    value_hex: Option<String>,
    value_error: Option<String>,
    descriptor_data_type: Option<String>,
    descriptor_writable: Option<bool>,
    descriptor_default: Option<String>,
    descriptor_current: Option<String>,
    descriptor_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SlotSelectionResult {
    usb_id: String,
    slot: u16,
    read_back_hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeWriteResult {
    usb_id: String,
    slot: u16,
    verified_properties: Vec<String>,
    backup_id: String,
    journal_id: String,
    preset_name: String,
}

/// Read-only preservation data for a single C slot. The values are emitted as
/// display-safe hexadecimal strings and are never accepted by a write command.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawPtpPresetSnapshot {
    manufacturer: &'static str,
    model: String,
    firmware: String,
    usb_id: String,
    slot: u16,
    captured_at: String,
    restoration_policy: &'static str,
    properties: Vec<RawPtpPresetSnapshotProperty>,
    unreadable_property_codes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawPtpPresetSnapshotProperty {
    code: String,
    value_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SnapshotProperty {
    code: u16,
    value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraBackup {
    id: String,
    usb_id: String,
    slot: u16,
    captured_at: u128,
    properties: Vec<SnapshotProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriteJournal {
    id: String,
    backup_id: String,
    usb_id: String,
    slot: u16,
    created_at: u128,
    state: String,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CameraBackupSummary {
    id: String,
    usb_id: String,
    slot: u16,
    captured_at: u128,
    property_count: usize,
}

/// A durable indicator that an interrupted write still needs human review.
/// The associated backup remains the only source for a restoration attempt.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteJournalSummary {
    id: String,
    backup_id: String,
    usb_id: String,
    slot: u16,
    created_at: u128,
    state: String,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstalledPreset {
    slot: u16,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageRecipeFieldStatus {
    key: String,
    status: String,
    detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageRecipeImport {
    file_name: String,
    model: String,
    settings: serde_json::Value,
    shooting_settings: serde_json::Value,
    field_statuses: Vec<ImageRecipeFieldStatus>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RafPreviewStageResult {
    state: raf_preview::RafPreviewState,
    recovery_action: Option<String>,
}

#[tauri::command]
fn discover_cameras() -> Result<Vec<CameraDiscovery>, String> {
    usb_transport::PlatformUsbBackend
        .list_devices()
        .map_err(|error| error.to_string())
        .map(|devices| {
            devices
                .into_iter()
                .map(|device| {
                    let recognition =
                        camera_fujifilm::recognise(device.id, device.product.as_deref());
                    CameraDiscovery {
                        usb_id: device.id.to_string(),
                        manufacturer: device.manufacturer,
                        product: device.product,
                        is_fujifilm: recognition.is_some(),
                        family: recognition.map(|device| device.family.label()),
                        profile: recognition.and_then(|device| device.model),
                        recipe_access: recognition.map(|device| device.recipe_access.label()),
                        ptp_interface_detected: device
                            .interfaces
                            .iter()
                            .any(usb_transport::UsbInterface::is_ptp),
                        write_enabled: device.id == UsbId::new(0x04CB, 0x030C),
                    }
                })
                .collect()
        })
}

#[tauri::command]
fn app_status() -> &'static str {
    "Phase 1: experimental X-M5 recipe writes with durable backup and read-back recovery"
}

#[tauri::command]
fn xm5_capability_record() -> camera_xm5::Xm5CapabilityRecord {
    camera_xm5::capability_record().clone()
}

/// The catalogue is intentionally recognition-oriented rather than a write
/// allow-list. A model appears here to help users create a matrix, while only
/// an exact trusted record can ever enable a writer.
#[tauri::command]
fn list_camera_capability_catalog() -> Vec<CameraCapabilityCatalogEntry> {
    let xm5_record = camera_xm5::capability_record();
    camera_fujifilm::KNOWN_MODELS
        .iter()
        .map(|profile| {
            let is_xm5 = profile.model == "X-M5";
            CameraCapabilityCatalogEntry {
                model: profile.model,
                family: profile.family.label(),
                access: profile.recipe_access.label(),
                trusted_record_id: is_xm5.then(|| xm5_record.record_id.clone()),
                trusted_firmware: is_xm5.then(|| xm5_record.firmware.clone()),
            }
        })
        .collect()
}

#[tauri::command]
fn probe_camera_device_info(usb_id: String) -> Result<PtpDeviceInfoResult, String> {
    let id = parse_usb_id(&usb_id)?;
    let probe = usb_transport::probe_ptp_device_info(id).map_err(|error| error.to_string())?;
    let capability =
        camera_fujifilm::resolve_capability_record(id, &probe.model, &probe.device_version);
    Ok(PtpDeviceInfoResult {
        usb_id: id.to_string(),
        interface_number: probe.interface_number,
        bulk_in_endpoint: format!("{:02X}", probe.bulk_in_endpoint),
        bulk_out_endpoint: format!("{:02X}", probe.bulk_out_endpoint),
        data_bytes: probe.data_bytes,
        response_code: format!("{:04X}", probe.response_code),
        manufacturer: probe.manufacturer,
        model: probe.model,
        device_version: probe.device_version,
        capability_state_key: capability.state.key().to_string(),
        capability_state: capability.state.label().to_string(),
        capability_record_id: capability.record_id.map(str::to_string),
        capability_next_action: capability.state.next_action().to_string(),
    })
}

/// Collect fixed-scope evidence for X-M5 properties that are not writable by
/// the application. This command only performs standard PTP reads and is
/// intentionally gated to the exact model/firmware record. A successful
/// descriptor or value read never changes the capability write allow-list.
#[tauri::command]
fn audit_xm5_unverified_properties(
    usb_id: String,
) -> Result<Xm5UnverifiedPropertyAuditResult, String> {
    let id = parse_usb_id(&usb_id)?;
    let device_info =
        usb_transport::probe_ptp_device_info(id).map_err(|error| error.to_string())?;
    let capability = camera_fujifilm::resolve_capability_record(
        id,
        &device_info.model,
        &device_info.device_version,
    );
    if capability.state != camera_fujifilm::CapabilityRecordState::ExactExperimental {
        return Err(format!(
            "{}: {}",
            capability.state.label(),
            capability.state.next_action()
        ));
    }

    let properties = camera_xm5::unverified_property_audit_candidates()
        .iter()
        .map(|(code, key, label)| {
            let (value_hex, value_error) = match usb_transport::probe_ptp_property_value(id, *code)
            {
                Ok(probe) => (Some(format_hex(&probe.value)), None),
                Err(error) => (None, Some(error.to_string())),
            };
            let (
                descriptor_data_type,
                descriptor_writable,
                descriptor_default,
                descriptor_current,
                descriptor_error,
            ) = match usb_transport::probe_ptp_property_descriptor(id, *code) {
                Ok(probe) => (
                    Some(format!("{:04X}", probe.data_type)),
                    Some(probe.writable),
                    Some(format!("{:?}", probe.factory_default)),
                    Some(format!("{:?}", probe.current_value)),
                    None,
                ),
                Err(error) => (None, None, None, None, Some(error.to_string())),
            };
            Xm5UnverifiedPropertyAuditEntry {
                property_code: format!("{:04X}", code),
                key: (*key).to_string(),
                label: (*label).to_string(),
                value_hex,
                value_error,
                descriptor_data_type,
                descriptor_writable,
                descriptor_default,
                descriptor_current,
                descriptor_error,
            }
        })
        .collect();
    Ok(Xm5UnverifiedPropertyAuditResult {
        usb_id: id.to_string(),
        model: device_info.model,
        firmware: device_info.device_version,
        properties,
    })
}

/// Build a local Recipe draft from JPEG or RAF metadata. ExifTool is used here
/// because it understands Fujifilm MakerNote data in both containers; the app
/// never uploads the image and never writes metadata back to it.
#[tauri::command]
fn import_fujifilm_image_recipe(path: String) -> Result<ImageRecipeImport, String> {
    let file = Path::new(&path);
    let extension = file
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("select a JPEG or RAF file")?;
    if !matches!(extension.as_str(), "jpg" | "jpeg" | "raf") {
        return Err("only JPG, JPEG, and RAF files can be imported as an EXIF Recipe".to_string());
    }
    if !file.is_file() {
        return Err("the selected image file no longer exists".to_string());
    }
    let output = Command::new("exiftool")
        .args(["-j", "-n", "-G1"])
        .arg(file)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "ExifTool is required for JPEG/RAF Recipe import. Install it for development, or use a release that bundles the verified metadata adapter.".to_string()
            } else {
                format!("could not start ExifTool: {error}")
            }
        })?;
    if !output.status.success() {
        return Err(format!(
            "ExifTool could not read this image: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let documents: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("ExifTool returned invalid metadata JSON: {error}"))?;
    let metadata = documents
        .first()
        .and_then(serde_json::Value::as_object)
        .ok_or("ExifTool returned no image metadata")?;
    let make = exif_string(metadata, "Make").unwrap_or_default();
    if !make.eq_ignore_ascii_case("FUJIFILM") {
        return Err("the selected image does not identify itself as FUJIFILM".to_string());
    }
    let file_name = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Imported image")
        .to_string();
    Ok(recipe_from_fujifilm_exif(metadata, file_name))
}

/// Perform an offline RAF preview preflight. This command deliberately does
/// not enumerate, open, or send data to a camera. Camera transport remains
/// unavailable until a model-specific adapter has a hardware-validated record.
#[tauri::command]
fn stage_raf_preview(path: String, recipe_id: String) -> Result<RafPreviewStageResult, String> {
    let file = Path::new(&path);
    let metadata = fs::metadata(file).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "the selected RAF file no longer exists".to_string()
        } else {
            format!("could not inspect the RAF file: {error}")
        }
    })?;
    if !metadata.is_file() {
        return Err("select a RAF file, not a folder".to_string());
    }
    let source_name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the RAF file name is not valid UTF-8")?
        .to_string();
    let progress = raf_preview::stage_request(raf_preview::RafPreviewRequest {
        source_name,
        source_bytes: metadata.len(),
        recipe_id,
        target: None,
    })
    .map_err(|error| error.to_string())?;
    Ok(RafPreviewStageResult {
        state: progress.state,
        recovery_action: progress.recovery_action,
    })
}

fn exif_value<'a>(
    metadata: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a serde_json::Value> {
    metadata.get(name).or_else(|| {
        metadata
            .iter()
            .find(|(key, _)| key.rsplit(':').next() == Some(name))
            .map(|(_, value)| value)
    })
}

fn exif_string(
    metadata: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<String> {
    match exif_value(metadata, name)? {
        serde_json::Value::String(value) => Some(value.trim().to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn exif_i64(metadata: &serde_json::Map<String, serde_json::Value>, name: &str) -> Option<i64> {
    match exif_value(metadata, name)? {
        serde_json::Value::Number(value) => value.as_i64(),
        serde_json::Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

fn exif_f64(metadata: &serde_json::Map<String, serde_json::Value>, name: &str) -> Option<f64> {
    match exif_value(metadata, name)? {
        serde_json::Value::Number(value) => value.as_f64(),
        serde_json::Value::String(value) => value.trim().trim_end_matches('K').parse().ok(),
        _ => None,
    }
}

fn add_exif_status(
    statuses: &mut Vec<ImageRecipeFieldStatus>,
    key: &str,
    status: &str,
    detail: impl Into<String>,
) {
    statuses.push(ImageRecipeFieldStatus {
        key: key.to_string(),
        status: status.to_string(),
        detail: detail.into(),
    });
}

fn recipe_from_fujifilm_exif(
    metadata: &serde_json::Map<String, serde_json::Value>,
    file_name: String,
) -> ImageRecipeImport {
    use serde_json::{json, Map, Value};

    let model = exif_string(metadata, "Model").unwrap_or_else(|| "Fujifilm".to_string());
    let mut settings = Map::new();
    let mut white_balance = Map::new();
    let mut shooting_settings = Map::new();
    let mut statuses = Vec::new();
    let mut warnings = Vec::new();

    match exif_i64(metadata, "FilmMode").and_then(map_exif_film_simulation) {
        Some(value) => {
            settings.insert("filmSimulation".to_string(), json!(value));
            add_exif_status(
                &mut statuses,
                "Film Simulation",
                "recognized",
                "MakerNotes:FilmMode",
            );
        }
        None => add_exif_status(
            &mut statuses,
            "Film Simulation",
            "unavailable",
            "MakerNote missing or model-specific value is not mapped",
        ),
    }
    match exif_i64(metadata, "DynamicRangeSetting").and_then(map_exif_dynamic_range) {
        Some(value) => {
            settings.insert("dynamicRange".to_string(), json!(value));
            add_exif_status(
                &mut statuses,
                "Dynamic Range",
                "recognized",
                "MakerNotes:DynamicRangeSetting",
            );
        }
        None => add_exif_status(
            &mut statuses,
            "Dynamic Range",
            "unavailable",
            "MakerNote missing or automatic/development value is not mapped",
        ),
    }
    match exif_i64(metadata, "WhiteBalance").and_then(map_exif_white_balance) {
        Some(value) => {
            white_balance.insert("mode".to_string(), json!(value));
            add_exif_status(
                &mut statuses,
                "White Balance",
                "recognized",
                "MakerNotes:WhiteBalance",
            );
        }
        None => add_exif_status(
            &mut statuses,
            "White Balance",
            "unavailable",
            "MakerNote missing or unsupported white-balance code",
        ),
    }
    if let Some(value) = exif_f64(metadata, "ColorTemperature") {
        if (2500.0..=10000.0).contains(&value) {
            white_balance.insert("colorTemperatureK".to_string(), json!(value.round() as i64));
            add_exif_status(
                &mut statuses,
                "Color Temperature",
                "recognized",
                "MakerNotes:ColorTemperature",
            );
        } else {
            add_exif_status(
                &mut statuses,
                "Color Temperature",
                "unavailable",
                "value is outside the Recipe range",
            );
        }
    } else {
        add_exif_status(
            &mut statuses,
            "Color Temperature",
            "unavailable",
            "MakerNote is absent",
        );
    }
    if let Some((red, blue)) = exif_white_balance_shift(metadata) {
        white_balance.insert("shiftR".to_string(), json!(red));
        white_balance.insert("shiftB".to_string(), json!(blue));
        add_exif_status(
            &mut statuses,
            "WB Shift",
            "recognized",
            "MakerNotes:WhiteBalanceFineTune",
        );
    } else {
        add_exif_status(
            &mut statuses,
            "WB Shift",
            "unavailable",
            "MakerNote is absent or uses an unknown representation",
        );
    }
    if !white_balance.is_empty() {
        settings.insert("whiteBalance".to_string(), Value::Object(white_balance));
    }

    match (
        exif_i64(metadata, "GrainEffectRoughness"),
        exif_i64(metadata, "GrainEffectSize"),
    ) {
        (Some(roughness), Some(size)) => match map_exif_grain(roughness, size) {
            Some((strength, size)) => {
                settings.insert(
                    "grain".to_string(),
                    json!({ "strength": strength, "size": size }),
                );
                add_exif_status(
                    &mut statuses,
                    "Grain Effect",
                    "recognized",
                    "MakerNotes:GrainEffectRoughness / GrainEffectSize",
                );
            }
            None => add_exif_status(
                &mut statuses,
                "Grain Effect",
                "unavailable",
                "MakerNote values are not mapped for this model",
            ),
        },
        _ => add_exif_status(
            &mut statuses,
            "Grain Effect",
            "unavailable",
            "MakerNotes are absent",
        ),
    }
    for (source, target, label) in [
        (
            "ColorChromeEffect",
            "colorChromeEffect",
            "Color Chrome Effect",
        ),
        (
            "ColorChromeFXBlue",
            "colorChromeFxBlue",
            "Color Chrome FX Blue",
        ),
        ("SmoothSkinEffect", "smoothSkinEffect", "Smooth Skin Effect"),
    ] {
        match exif_i64(metadata, source).and_then(map_exif_effect) {
            Some(value) => {
                settings.insert(target.to_string(), json!(value));
                add_exif_status(
                    &mut statuses,
                    label,
                    "recognized",
                    format!("MakerNotes:{source}"),
                );
            }
            None => add_exif_status(
                &mut statuses,
                label,
                "unavailable",
                "MakerNote missing or unrecognised",
            ),
        }
    }
    for (source, target, label, mapper) in [
        (
            "HighlightTone",
            "highlight",
            "Highlight Tone",
            map_exif_tone as fn(i64) -> Option<f64>,
        ),
        (
            "ShadowTone",
            "shadow",
            "Shadow Tone",
            map_exif_tone as fn(i64) -> Option<f64>,
        ),
        (
            "Sharpness",
            "sharpness",
            "Sharpness",
            map_exif_sharpness as fn(i64) -> Option<f64>,
        ),
        (
            "NoiseReduction",
            "highIsoNoiseReduction",
            "High ISO NR",
            map_exif_noise_reduction as fn(i64) -> Option<f64>,
        ),
    ] {
        match exif_i64(metadata, source).and_then(mapper) {
            Some(value) => {
                settings.insert(target.to_string(), json!(value));
                add_exif_status(
                    &mut statuses,
                    label,
                    "recognized",
                    format!("MakerNotes:{source}"),
                );
            }
            None => add_exif_status(
                &mut statuses,
                label,
                "unavailable",
                "MakerNote missing or unrecognised",
            ),
        }
    }
    match exif_i64(metadata, "Saturation").and_then(map_exif_color) {
        Some(value) => {
            settings.insert("color".to_string(), json!(value));
            add_exif_status(
                &mut statuses,
                "Color",
                "recognized",
                "MakerNotes:Saturation",
            );
        }
        None => add_exif_status(
            &mut statuses,
            "Color",
            "unavailable",
            "MakerNote is absent or belongs to a monochrome simulation",
        ),
    }
    match exif_i64(metadata, "Clarity") {
        Some(value) if (-5000..=5000).contains(&value) && value % 1000 == 0 => {
            settings.insert("clarity".to_string(), json!(value / 1000));
            add_exif_status(&mut statuses, "Clarity", "recognized", "MakerNotes:Clarity");
        }
        _ => add_exif_status(
            &mut statuses,
            "Clarity",
            "unavailable",
            "MakerNote missing or unrecognised",
        ),
    }
    match exif_i64(metadata, "ISO") {
        Some(value) if value > 0 => {
            shooting_settings.insert("isoSensitivity".to_string(), json!(format!("ISO_{value}")));
            add_exif_status(&mut statuses, "ISO Sensitivity", "recognized", "EXIF:ISO");
        }
        _ => add_exif_status(
            &mut statuses,
            "ISO Sensitivity",
            "unavailable",
            "EXIF ISO is absent",
        ),
    }
    if !metadata
        .keys()
        .any(|key| key.rsplit(':').next() == Some("FilmMode"))
    {
        warnings.push("No Fujifilm FilmMode MakerNote was found. This may be a processed image rather than a straight-out-of-camera JPEG/RAF.".to_string());
    }
    warnings.push("Only fields marked recognized are used to build the Recipe. Unavailable fields retain safe local defaults and are never guessed.".to_string());

    ImageRecipeImport {
        file_name,
        model,
        settings: Value::Object(settings),
        shooting_settings: Value::Object(shooting_settings),
        field_statuses: statuses,
        warnings,
    }
}

fn map_exif_film_simulation(value: i64) -> Option<&'static str> {
    match value {
        0x000 => Some("PROVIA"),
        0x120 => Some("ASTIA"),
        0x200 | 0x400 => Some("VELVIA"),
        0x500 => Some("PRO_NEG_STD"),
        0x501 => Some("PRO_NEG_HI"),
        0x600 => Some("CLASSIC_CHROME"),
        0x700 => Some("ETERNA"),
        0x800 => Some("CLASSIC_NEGATIVE"),
        0x900 => Some("ETERNA_BLEACH_BYPASS"),
        0xA00 => Some("NOSTALGIC_NEGATIVE"),
        0xB00 => Some("REALA_ACE"),
        _ => None,
    }
}

fn map_exif_dynamic_range(value: i64) -> Option<&'static str> {
    match value {
        0 => Some("AUTO"),
        100 => Some("DR100"),
        200 => Some("DR200"),
        400 => Some("DR400"),
        _ => None,
    }
}

fn map_exif_white_balance(value: i64) -> Option<&'static str> {
    match value {
        0x000 => Some("AUTO"),
        0x001 => Some("WHITE_PRIORITY"),
        0x002 => Some("AMBIENCE_PRIORITY"),
        0x100 => Some("DAYLIGHT"),
        0x200 => Some("SHADE"),
        0x300 => Some("FLUORESCENT_1"),
        0x301 => Some("FLUORESCENT_2"),
        0x302 => Some("FLUORESCENT_3"),
        0x400 => Some("INCANDESCENT"),
        0x600 => Some("UNDERWATER"),
        0xFF0 => Some("COLOR_TEMPERATURE"),
        _ => None,
    }
}

fn map_exif_grain(roughness: i64, size: i64) -> Option<(&'static str, &'static str)> {
    match (roughness, size) {
        (0, _) => Some(("OFF", "SMALL")),
        (32, 16) => Some(("WEAK", "SMALL")),
        (32, 32) => Some(("WEAK", "LARGE")),
        (64, 16) => Some(("STRONG", "SMALL")),
        (64, 32) => Some(("STRONG", "LARGE")),
        _ => None,
    }
}

fn map_exif_effect(value: i64) -> Option<&'static str> {
    match value {
        0 => Some("OFF"),
        32 => Some("WEAK"),
        64 => Some("STRONG"),
        _ => None,
    }
}

fn map_exif_tone(value: i64) -> Option<f64> {
    if (-64..=32).contains(&value) && value % 8 == 0 {
        Some(-(value as f64) / 16.0)
    } else {
        None
    }
}

fn map_exif_sharpness(value: i64) -> Option<f64> {
    match value {
        0 => Some(-4.0),
        1 => Some(-3.0),
        2 => Some(-2.0),
        3 => Some(0.0),
        4 => Some(2.0),
        5 => Some(3.0),
        6 => Some(4.0),
        0x82 => Some(-1.0),
        0x84 => Some(1.0),
        _ => None,
    }
}

fn map_exif_noise_reduction(value: i64) -> Option<f64> {
    match value {
        0x000 => Some(0.0),
        0x100 => Some(2.0),
        0x180 => Some(1.0),
        0x1C0 => Some(3.0),
        0x1E0 => Some(4.0),
        0x200 => Some(-2.0),
        0x280 => Some(-1.0),
        0x2C0 => Some(-3.0),
        0x2E0 => Some(-4.0),
        _ => None,
    }
}

fn map_exif_color(value: i64) -> Option<i64> {
    match value {
        0x000 => Some(0),
        0x080 => Some(1),
        0x100 => Some(2),
        0x0C0 => Some(3),
        0x0E0 => Some(4),
        0x180 => Some(-1),
        0x400 => Some(-2),
        0x4C0 => Some(-3),
        0x4E0 => Some(-4),
        _ => None,
    }
}

fn exif_white_balance_shift(
    metadata: &serde_json::Map<String, serde_json::Value>,
) -> Option<(i64, i64)> {
    let raw = exif_string(metadata, "WhiteBalanceFineTune")?;
    let numbers = raw
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|value| !value.is_empty())
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if numbers.len() != 2 {
        return None;
    }
    let (red, blue) = (numbers[0], numbers[1]);
    let (red, blue) = if red.abs() >= 20 || blue.abs() >= 20 {
        (red / 20, blue / 20)
    } else {
        (red, blue)
    };
    ((-9..=9).contains(&red) && (-9..=9).contains(&blue)).then_some((red, blue))
}

/// Every X-M5 operation below this line can change the active C-slot selector
/// or a setting. It therefore requires the complete USB + PTP model + firmware
/// identity, not merely a Fujifilm vendor ID or product string.
fn exact_xm5_write_identity(id: UsbId) -> Result<usb_transport::PtpDeviceInfoProbe, String> {
    let device_info =
        usb_transport::probe_ptp_device_info(id).map_err(|error| error.to_string())?;
    let capability = camera_fujifilm::resolve_capability_record(
        id,
        &device_info.model,
        &device_info.device_version,
    );
    if capability.state != camera_fujifilm::CapabilityRecordState::ExactExperimental {
        return Err(format!(
            "{}: {}",
            capability.state.label(),
            capability.state.next_action()
        ));
    }
    Ok(device_info)
}

#[tauri::command]
fn read_camera_slot_selector(usb_id: String) -> Result<PtpPropertyValueResult, String> {
    let id = parse_usb_id(&usb_id)?;
    // The GUI intentionally exposes one hardware-observed, read-only probe.
    // It does not let arbitrary UI input select or write PTP properties.
    const CUSTOM_SLOT_SELECTOR: u16 = 0xD18C;
    let probe = usb_transport::probe_ptp_property_value(id, CUSTOM_SLOT_SELECTOR)
        .map_err(|error| error.to_string())?;
    Ok(PtpPropertyValueResult {
        usb_id: id.to_string(),
        interface_number: probe.interface_number,
        property_code: format!("{:04X}", probe.property_code),
        value_hex: probe
            .value
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
    })
}

#[tauri::command]
fn select_camera_slot(usb_id: String, slot: u16) -> Result<SlotSelectionResult, String> {
    if !(1..=4).contains(&slot) {
        return Err("slot must be between C1 and C4".to_string());
    }
    let id = parse_usb_id(&usb_id)?;
    exact_xm5_write_identity(id)?;
    usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;
    let read_back =
        usb_transport::probe_ptp_property_value(id, 0xD18C).map_err(|error| error.to_string())?;
    if read_back.value != slot.to_le_bytes() {
        return Err(format!(
            "slot read-back mismatch: expected C{slot}, received {}",
            read_back
                .value
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    Ok(SlotSelectionResult {
        usb_id: id.to_string(),
        slot,
        read_back_hex: read_back
            .value
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
    })
}

/// Capture the observed custom-slot property window without writing a Recipe
/// property. Selecting the requested C slot is the one necessary transport
/// side effect; the previous active slot is always read first and verified on
/// restoration. Failed reads are recorded by code, not retried as writes.
#[tauri::command]
fn capture_xm5_raw_preset_snapshot(
    usb_id: String,
    slot: u16,
) -> Result<RawPtpPresetSnapshot, String> {
    if !(1..=4).contains(&slot) {
        return Err("slot must be between C1 and C4".to_string());
    }
    let id = parse_usb_id(&usb_id)?;
    let device_info = exact_xm5_write_identity(id)?;
    let previously_selected = read_selected_slot(id)?;
    usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;

    let outcome = (|| -> Result<RawPtpPresetSnapshot, String> {
        let mut properties = Vec::new();
        let mut unreadable_property_codes = Vec::new();
        for code in xm5_raw_snapshot_property_codes() {
            match usb_transport::probe_ptp_property_value(id, *code) {
                Ok(probe) => properties.push(RawPtpPresetSnapshotProperty {
                    code: format!("{code:04X}"),
                    value_hex: format_hex(&probe.value),
                }),
                Err(_) => unreadable_property_codes.push(format!("{code:04X}")),
            }
        }
        if properties.is_empty() {
            return Err("camera did not return any readable custom-slot properties".to_string());
        }
        Ok(RawPtpPresetSnapshot {
            manufacturer: "FUJIFILM",
            model: device_info.model,
            firmware: device_info.device_version,
            usb_id: id.to_string(),
            slot,
            captured_at: timestamp_millis().to_string(),
            restoration_policy: "read_only_preserved",
            properties,
            unreadable_property_codes,
        })
    })();

    let selector_result = restore_selected_slot(id, previously_selected);
    match (outcome, selector_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(_), Err(error)) => Err(format!(
            "raw snapshot was captured, but the prior active slot could not be restored: {error}"
        )),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(selector_error)) => Err(format!(
            "{error}; also failed to restore the active slot: {selector_error}"
        )),
    }
}

/// This bounded property window is specific to the observed X-M5 custom-slot
/// protocol. It includes rejected/unknown values so a snapshot can preserve
/// them, but it is intentionally not the application restore allow-list.
fn xm5_raw_snapshot_property_codes() -> &'static [u16] {
    &[
        0xD18D, 0xD18E, 0xD18F, 0xD190, 0xD191, 0xD192, 0xD193, 0xD194, 0xD195, 0xD196, 0xD197,
        0xD198, 0xD199, 0xD19A, 0xD19B, 0xD19C, 0xD19D, 0xD19E, 0xD19F, 0xD1A0, 0xD1A1, 0xD1A2,
        0xD1A3, 0xD1A4, 0xD1A5,
    ]
}

#[tauri::command]
fn write_xm5_recipe(
    usb_id: String,
    slot: u16,
    settings: camera_xm5::Xm5RecipeSettings,
    name: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<RecipeWriteResult, String> {
    write_xm5_recipe_inner(usb_id, slot, settings, name, false, database.inner())
}

/// Clear the Recipe values which have an X-M5 firmware-1.30 write/read-back
/// record. This deliberately is not advertised as a factory reset: unverified
/// camera settings are left untouched and every changed value gets a durable
/// restore point first.
#[tauri::command]
fn clear_xm5_custom_slot(
    usb_id: String,
    slot: u16,
    database: tauri::State<'_, LibraryDb>,
) -> Result<RecipeWriteResult, String> {
    write_xm5_recipe_inner(
        usb_id,
        slot,
        cleared_xm5_recipe_settings(),
        String::new(),
        true,
        database.inner(),
    )
}

fn write_xm5_recipe_inner(
    usb_id: String,
    slot: u16,
    settings: camera_xm5::Xm5RecipeSettings,
    name: String,
    allow_empty_name: bool,
    database: &LibraryDb,
) -> Result<RecipeWriteResult, String> {
    if !(1..=4).contains(&slot) {
        return Err("slot must be between C1 and C4".to_string());
    }
    let id = parse_usb_id(&usb_id)?;
    exact_xm5_write_identity(id)?;
    let preset_name = camera_preset_name(&name, allow_empty_name)?;
    let preset_name_value =
        ptp_core::encode_string(&preset_name).map_err(|error| error.to_string())?;
    let mut properties = camera_xm5::encode_recipe(&settings)?;
    properties.insert(
        0,
        camera_xm5::RecipeProperty {
            code: 0xD18D,
            write_value: preset_name_value.clone(),
            accepted_read_values: vec![preset_name_value],
        },
    );
    let previously_selected = read_selected_slot(id)?;
    usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;

    let outcome = (|| -> Result<RecipeWriteResult, String> {
        let mut captured = Vec::new();
        for property in &properties {
            if captured
                .iter()
                .any(|snapshot: &SnapshotProperty| snapshot.code == property.code)
            {
                continue;
            }
            let probe = usb_transport::probe_ptp_property_value(id, property.code)
                .map_err(|error| error.to_string())?;
            captured.push(SnapshotProperty {
                code: property.code,
                value: probe.value,
            });
        }
        let backup = CameraBackup {
            id: new_record_id("backup"),
            usb_id: id.to_string(),
            slot,
            captured_at: timestamp_millis(),
            properties: captured,
        };
        save_camera_backup(database, &backup)?;
        let mut journal = WriteJournal {
            id: new_record_id("write"),
            backup_id: backup.id.clone(),
            usb_id: id.to_string(),
            slot,
            created_at: timestamp_millis(),
            state: "writing".to_string(),
            error: None,
        };
        save_write_journal(database, &journal)?;

        for property in &properties {
            let result =
                usb_transport::set_ptp_property_value(id, property.code, &property.write_value)
                    .and_then(|_| usb_transport::probe_ptp_property_value(id, property.code));
            let verified = result
                .as_ref()
                .map(|probe| property.matches_read_back(&probe.value))
                .unwrap_or(false);
            if !verified {
                let original_error = match result {
                    Ok(probe) => format!(
                        "property {:04X} read-back mismatch (received {})",
                        property.code,
                        format_hex(&probe.value)
                    ),
                    Err(error) => {
                        format!("property {:04X} write/read failed: {error}", property.code)
                    }
                };
                let restore_result = restore_captured_properties(id, &backup.properties);
                journal.error = Some(original_error.clone());
                journal.state = if restore_result.is_ok() {
                    "rolled_back".to_string()
                } else {
                    "recovery_failed".to_string()
                };
                update_write_journal(database, &journal)?;
                return match restore_result {
                    Ok(()) => Err(format!("{original_error}; captured values were restored and verified")),
                    Err(restore_error) => Err(format!(
                        "{original_error}; automatic recovery failed: {restore_error}. Backup {} is retained locally.",
                        backup.id
                    )),
                };
            }
        }
        journal.state = "committed".to_string();
        update_write_journal(database, &journal)?;
        Ok(RecipeWriteResult {
            usb_id: id.to_string(),
            slot,
            verified_properties: properties
                .iter()
                .map(|property| format!("{:04X}", property.code))
                .collect(),
            backup_id: backup.id,
            journal_id: journal.id,
            preset_name,
        })
    })();

    let selector_result = restore_selected_slot(id, previously_selected);
    match (outcome, selector_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(_), Err(error)) => Err(format!(
            "recipe was written, but the prior active slot could not be restored: {error}"
        )),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(selector_error)) => Err(format!(
            "{error}; also failed to restore the active slot: {selector_error}"
        )),
    }
}

fn camera_preset_name(name: &str, allow_empty: bool) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() && !allow_empty {
        return Err("X-M5 preset name cannot be empty".to_string());
    }
    if name.encode_utf16().count() > 31 {
        return Err("X-M5 preset name must be at most 31 UTF-16 characters".to_string());
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return Err(
            "X-M5 firmware 1.30 preset-name writing is verified only for printable ASCII"
                .to_string(),
        );
    }
    Ok(name.to_string())
}

fn cleared_xm5_recipe_settings() -> camera_xm5::Xm5RecipeSettings {
    camera_xm5::Xm5RecipeSettings {
        film_simulation: "PROVIA".to_string(),
        dynamic_range: "DR100".to_string(),
        white_balance: camera_xm5::Xm5WhiteBalance {
            mode: "AUTO".to_string(),
            color_temperature_k: 6500,
            shift_r: 0,
            shift_b: 0,
        },
        grain: camera_xm5::Xm5Grain {
            strength: "OFF".to_string(),
            size: "SMALL".to_string(),
        },
        color_chrome_effect: "OFF".to_string(),
        color_chrome_fx_blue: "OFF".to_string(),
        smooth_skin_effect: "OFF".to_string(),
        monochromatic_color: camera_xm5::Xm5MonochromaticColor::default(),
        color_space: "SRGB".to_string(),
        image_size: "L_3_2".to_string(),
        image_quality: "FINE".to_string(),
        highlight: 0.0,
        shadow: 0.0,
        color: 0,
        sharpness: 0,
        high_iso_noise_reduction: 0,
        clarity: 0,
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn new_record_id(prefix: &str) -> String {
    let sequence = RECORD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{sequence}", timestamp_millis())
}

fn format_hex(value: &[u8]) -> String {
    value
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_selected_slot(id: UsbId) -> Result<u16, String> {
    let probe =
        usb_transport::probe_ptp_property_value(id, 0xD18C).map_err(|error| error.to_string())?;
    let bytes: [u8; 2] = probe
        .value
        .as_slice()
        .try_into()
        .map_err(|_| "camera returned an invalid active-slot value".to_string())?;
    let slot = u16::from_le_bytes(bytes);
    if !(1..=4).contains(&slot) {
        return Err(format!(
            "camera returned unsupported active custom slot C{slot}"
        ));
    }
    Ok(slot)
}

fn restore_selected_slot(id: UsbId, slot: u16) -> Result<(), String> {
    usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;
    let restored = read_selected_slot(id)?;
    if restored == slot {
        Ok(())
    } else {
        Err(format!("expected C{slot}, received C{restored}"))
    }
}

fn restore_captured_properties(id: UsbId, properties: &[SnapshotProperty]) -> Result<(), String> {
    let mut failures = Vec::new();
    for captured in properties.iter().rev() {
        if let Err(error) = validate_restorable_property(captured) {
            failures.push(error);
            continue;
        }
        for restore in camera_xm5::restore_property_steps(captured.code, &captured.value) {
            let result =
                usb_transport::set_ptp_property_value(id, restore.code, &restore.write_value)
                    .and_then(|_| usb_transport::probe_ptp_property_value(id, restore.code));
            match result {
                Ok(probe) if restore.matches_read_back(&probe.value) => {}
                Ok(probe) => {
                    failures.push(format!(
                        "{:04X} read-back mismatch ({})",
                        restore.code,
                        format_hex(&probe.value)
                    ));
                    break;
                }
                Err(error) => {
                    failures.push(format!("{:04X} {error}", restore.code));
                    break;
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Backups are local, but restore is still a write path. Restrict it to the
/// exact X-M5 Recipe properties captured by this application so a modified
/// database cannot turn recovery into an arbitrary vendor-property write.
fn validate_restorable_property(property: &SnapshotProperty) -> Result<(), String> {
    let known = matches!(
        property.code,
        0xD18D
            | 0xD18E
            | 0xD18F
            | 0xD190
            | 0xD192
            | 0xD195
            | 0xD196
            | 0xD197
            | 0xD198
            | 0xD199
            | 0xD19A
            | 0xD19B
            | 0xD19C
            | 0xD19D
            | 0xD19E
            | 0xD19F
            | 0xD1A0
            | 0xD1A1
            | 0xD1A2
            | 0xD1A4
    );
    if !known {
        return Err(format!(
            "{:04X} is not an approved X-M5 Recipe property",
            property.code
        ));
    }
    if property.code == 0xD18D {
        if !(3..=65).contains(&property.value.len()) || decode_ptp_string(&property.value).is_none()
        {
            return Err("D18D preset name has an invalid captured length".to_string());
        }
    } else if property.value.len() != 2 {
        return Err(format!(
            "{:04X} has an invalid captured value length",
            property.code
        ));
    }
    Ok(())
}

fn save_camera_backup(database: &LibraryDb, backup: &CameraBackup) -> Result<(), String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    connection
        .execute(
            "INSERT INTO camera_backups (id, usb_id, slot, captured_at, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &backup.id,
                &backup.usb_id,
                backup.slot,
                backup.captured_at.to_string(),
                serde_json::to_string(backup).map_err(|error| error.to_string())?,
            ),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn save_write_journal(database: &LibraryDb, journal: &WriteJournal) -> Result<(), String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    connection
        .execute(
            "INSERT INTO write_journals (id, backup_id, usb_id, slot, created_at, state, error, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &journal.id,
                &journal.backup_id,
                &journal.usb_id,
                journal.slot,
                journal.created_at.to_string(),
                &journal.state,
                &journal.error,
                serde_json::to_string(journal).map_err(|error| error.to_string())?,
            ),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn update_write_journal(database: &LibraryDb, journal: &WriteJournal) -> Result<(), String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    connection
        .execute(
            "UPDATE write_journals SET state = ?2, error = ?3, payload = ?4 WHERE id = ?1",
            (
                &journal.id,
                &journal.state,
                &journal.error,
                serde_json::to_string(journal).map_err(|error| error.to_string())?,
            ),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_local_capability_matrix(matrix: &mut LocalCapabilityMatrix) -> Result<(), String> {
    if matrix.schema_version != 1 {
        return Err("unsupported local capability matrix schema version".to_string());
    }
    matrix.id = matrix.id.trim().to_string();
    matrix.manufacturer = matrix.manufacturer.trim().to_ascii_uppercase();
    matrix.model = matrix.model.trim().to_string();
    matrix.firmware = matrix.firmware.trim().to_string();
    matrix.usb_ids = matrix
        .usb_ids
        .iter()
        .map(|id| id.trim().to_ascii_uppercase())
        .filter(|id| !id.is_empty())
        .collect();
    matrix.custom_slots = matrix
        .custom_slots
        .iter()
        .map(|slot| slot.trim().to_ascii_uppercase())
        .filter(|slot| !slot.is_empty())
        .collect();
    matrix.source_kind = matrix.source_kind.trim().to_ascii_lowercase();
    matrix.evidence_summary = matrix.evidence_summary.trim().to_string();
    if let Some(source_url) = &matrix.source_url {
        let source_url = source_url.trim();
        matrix.source_url = (!source_url.is_empty()).then(|| source_url.to_string());
    }

    if matrix.id.is_empty() || matrix.model.is_empty() || matrix.firmware.is_empty() {
        return Err("capability matrix requires an ID, model, and firmware".to_string());
    }
    if matrix.manufacturer != "FUJIFILM" {
        return Err("local capability matrices are limited to FUJIFILM cameras".to_string());
    }
    if matrix.usb_ids.is_empty() {
        return Err("capability matrix requires at least one USB ID".to_string());
    }
    if !matches!(
        matrix.source_kind.as_str(),
        "official" | "community" | "local_probe"
    ) {
        return Err(
            "capability matrix source kind must be official, community, or local_probe".to_string(),
        );
    }
    if let Some(source_url) = &matrix.source_url {
        if !is_safe_external_url(source_url) {
            return Err("capability matrix source URL must use http or https".to_string());
        }
    }
    for id in &matrix.usb_ids {
        parse_usb_id(id)?;
    }
    if matrix.custom_slots.iter().any(|slot| {
        !matches!(
            slot.as_str(),
            "C1" | "C2" | "C3" | "C4" | "C5" | "C6" | "C7"
        )
    }) {
        return Err("custom slots must use C1 through C7 labels".to_string());
    }

    let mut keys = std::collections::HashSet::new();
    for property in &mut matrix.properties {
        property.key = property.key.trim().to_string();
        property.label_zh = property.label_zh.trim().to_string();
        property.label_en = property.label_en.trim().to_string();
        property.code = property.code.trim().to_ascii_uppercase();
        property.scope = property.scope.trim().to_ascii_lowercase();
        property.status = property.status.trim().to_ascii_lowercase();
        property.notes = property.notes.trim().to_string();
        if property.key.is_empty() || property.label_zh.is_empty() || property.label_en.is_empty() {
            return Err("every capability property needs a key and bilingual labels".to_string());
        }
        if !property.code.is_empty()
            && (property.code.len() != 4
                || !property.code.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(format!(
                "{} PTP code must be four hexadecimal characters",
                property.key
            ));
        }
        if !keys.insert(property.key.clone()) {
            return Err(format!(
                "duplicate capability property key: {}",
                property.key
            ));
        }
        if !matches!(
            property.scope.as_str(),
            "custom_slot" | "global" | "unknown"
        ) {
            return Err(format!("{} has an invalid property scope", property.key));
        }
        if property.status == "write_verified" {
            return Err(format!(
                "{} cannot be locally marked write verified; complete a hardware write/read-back/restore validation and update the trusted capability record",
                property.key
            ));
        }
        if !matches!(
            property.status.as_str(),
            "read_detected_unverified" | "write_rejected" | "blocked_unknown"
        ) {
            return Err(format!(
                "{} has an invalid local capability status",
                property.key
            ));
        }
    }
    Ok(())
}

fn list_local_capability_matrices_inner(
    database: &LibraryDb,
) -> Result<Vec<LocalCapabilityMatrix>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let mut statement = connection
        .prepare("SELECT payload FROM local_capability_matrices ORDER BY updated_at DESC")
        .map_err(|error| error.to_string())?;
    let matrices = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .map(|payload| {
            payload
                .map_err(|error| error.to_string())
                .and_then(|payload| {
                    serde_json::from_str(&payload).map_err(|error| error.to_string())
                })
        })
        .collect();
    matrices
}

fn save_local_capability_matrix_inner(
    mut matrix: LocalCapabilityMatrix,
    database: &LibraryDb,
) -> Result<LocalCapabilityMatrix, String> {
    validate_local_capability_matrix(&mut matrix)?;
    let now = timestamp_millis();
    if matrix.created_at == 0 {
        matrix.created_at = now;
    }
    matrix.updated_at = now;
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    connection
        .execute(
            "INSERT INTO local_capability_matrices (id, manufacturer, model, firmware, usb_ids, updated_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                manufacturer = excluded.manufacturer,
                model = excluded.model,
                firmware = excluded.firmware,
                usb_ids = excluded.usb_ids,
                updated_at = excluded.updated_at,
                payload = excluded.payload",
            (
                &matrix.id,
                &matrix.manufacturer,
                &matrix.model,
                &matrix.firmware,
                serde_json::to_string(&matrix.usb_ids).map_err(|error| error.to_string())?,
                matrix.updated_at.to_string(),
                serde_json::to_string(&matrix).map_err(|error| error.to_string())?,
            ),
        )
        .map_err(|error| error.to_string())?;
    Ok(matrix)
}

#[tauri::command]
fn list_local_capability_matrices(
    database: tauri::State<'_, LibraryDb>,
) -> Result<Vec<LocalCapabilityMatrix>, String> {
    list_local_capability_matrices_inner(database.inner())
}

#[tauri::command]
fn save_local_capability_matrix(
    matrix: LocalCapabilityMatrix,
    database: tauri::State<'_, LibraryDb>,
) -> Result<LocalCapabilityMatrix, String> {
    save_local_capability_matrix_inner(matrix, database.inner())
}

#[tauri::command]
fn delete_local_capability_matrix(
    id: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<(), String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    connection
        .execute("DELETE FROM local_capability_matrices WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn list_pending_write_journals_inner(
    database: &LibraryDb,
) -> Result<Vec<WriteJournalSummary>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let mut statement = connection
        .prepare(
            "SELECT payload FROM write_journals
             WHERE state IN ('writing', 'recovery_failed')
             ORDER BY created_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let journals = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .map(|payload| {
            let journal: WriteJournal =
                serde_json::from_str(&payload.map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            Ok(WriteJournalSummary {
                id: journal.id,
                backup_id: journal.backup_id,
                usb_id: journal.usb_id,
                slot: journal.slot,
                created_at: journal.created_at,
                state: journal.state,
                error: journal.error,
            })
        })
        .collect();
    journals
}

#[tauri::command]
fn list_pending_write_journals(
    database: tauri::State<'_, LibraryDb>,
) -> Result<Vec<WriteJournalSummary>, String> {
    list_pending_write_journals_inner(database.inner())
}

/// A manual restore is only acknowledged after the model-specific restore
/// command has read every captured value back successfully.
fn mark_pending_journals_recovered(database: &LibraryDb, backup_id: &str) -> Result<(), String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let journals = {
        let mut statement = connection
            .prepare(
                "SELECT payload FROM write_journals
                 WHERE backup_id = ?1 AND state IN ('writing', 'recovery_failed')",
            )
            .map_err(|error| error.to_string())?;
        let journals = statement
            .query_map([backup_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .map(|payload| {
                serde_json::from_str::<WriteJournal>(&payload.map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        journals
    };
    for mut journal in journals {
        journal.state = "recovered_manually".to_string();
        connection
            .execute(
                "UPDATE write_journals SET state = ?2, payload = ?3 WHERE id = ?1",
                (
                    &journal.id,
                    &journal.state,
                    serde_json::to_string(&journal).map_err(|error| error.to_string())?,
                ),
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn list_camera_backups(
    usb_id: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<Vec<CameraBackupSummary>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let mut statement = connection
        .prepare("SELECT payload FROM camera_backups WHERE usb_id = ?1 ORDER BY captured_at DESC")
        .map_err(|error| error.to_string())?;
    let backups = statement
        .query_map([usb_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .map(|payload| {
            let backup: CameraBackup =
                serde_json::from_str(&payload.map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            Ok(CameraBackupSummary {
                id: backup.id,
                usb_id: backup.usb_id,
                slot: backup.slot,
                captured_at: backup.captured_at,
                property_count: backup.properties.len(),
            })
        })
        .collect();
    backups
}

#[tauri::command]
fn restore_xm5_backup(
    usb_id: String,
    backup_id: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<CameraBackupSummary, String> {
    let id = parse_usb_id(&usb_id)?;
    exact_xm5_write_identity(id)?;
    let backup = {
        let connection = database
            .0
            .lock()
            .map_err(|_| "recipe database lock failed".to_string())?;
        let payload: String = connection
            .query_row(
                "SELECT payload FROM camera_backups WHERE id = ?1 AND usb_id = ?2",
                (&backup_id, &usb_id),
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_str::<CameraBackup>(&payload).map_err(|error| error.to_string())?
    };
    let previous_slot = read_selected_slot(id)?;
    usb_transport::select_custom_slot(id, backup.slot).map_err(|error| error.to_string())?;
    let restored = restore_captured_properties(id, &backup.properties);
    let selector_result = restore_selected_slot(id, previous_slot);
    match (restored, selector_result) {
        (Ok(()), Ok(())) => {
            mark_pending_journals_recovered(database.inner(), &backup.id).map_err(|error| {
                format!(
                    "backup was restored and verified, but its recovery journal could not be acknowledged: {error}"
                )
            })?;
            Ok(CameraBackupSummary {
                id: backup.id,
                usb_id: backup.usb_id,
                slot: backup.slot,
                captured_at: backup.captured_at,
                property_count: backup.properties.len(),
            })
        }
        (Err(error), Ok(())) => Err(format!("backup restore failed: {error}")),
        (Ok(()), Err(error)) => Err(format!(
            "backup was restored, but active slot recovery failed: {error}"
        )),
        (Err(error), Err(selector_error)) => Err(format!(
            "backup restore failed: {error}; active slot recovery also failed: {selector_error}"
        )),
    }
}

#[tauri::command]
fn read_xm5_installed_presets(usb_id: String) -> Result<Vec<InstalledPreset>, String> {
    let id = parse_usb_id(&usb_id)?;
    exact_xm5_write_identity(id)?;
    let selected =
        usb_transport::probe_ptp_property_value(id, 0xD18C).map_err(|error| error.to_string())?;
    if selected.value.len() != 2 {
        return Err("camera returned an invalid active-slot value".to_string());
    }
    let previous_slot = u16::from_le_bytes([selected.value[0], selected.value[1]]);
    let result = (|| {
        let mut presets = Vec::new();
        for slot in 1..=4 {
            usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;
            let value = usb_transport::probe_ptp_property_value(id, 0xD18D)
                .map_err(|error| error.to_string())?
                .value;
            presets.push(InstalledPreset {
                slot,
                name: decode_ptp_string(&value).unwrap_or_default(),
            });
        }
        Ok(presets)
    })();
    if (1..=4).contains(&previous_slot) {
        let _ = usb_transport::select_custom_slot(id, previous_slot);
    }
    result
}

#[tauri::command]
fn list_recipes(database: tauri::State<'_, LibraryDb>) -> Result<Vec<serde_json::Value>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let mut statement = connection
        .prepare("SELECT payload FROM recipes ORDER BY updated_at DESC")
        .map_err(|error| error.to_string())?;
    let recipes = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .map(|payload| {
            payload
                .map_err(|error| error.to_string())
                .and_then(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        })
        .collect();
    recipes
}

/// Persist changed documents without clearing the library. The updatedAt value
/// is an optimistic revision: a delayed save from another window cannot
/// overwrite a newer local edit.
#[tauri::command]
fn upsert_recipes(
    recipes: Vec<RecipeDocument>,
    database: tauri::State<'_, LibraryDb>,
) -> Result<(), String> {
    upsert_recipes_inner(recipes, database.inner())
}

fn upsert_recipes_inner(recipes: Vec<RecipeDocument>, database: &LibraryDb) -> Result<(), String> {
    let mut connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    for RecipeDocument(recipe) in recipes {
        let id = recipe
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or("recipe missing id")?;
        let updated_at = recipe
            .get("updatedAt")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or("recipe missing updatedAt")?;
        let deleted_at: Option<String> = transaction
            .query_row(
                "SELECT deleted_at FROM recipe_tombstones WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if deleted_at
            .as_deref()
            .is_some_and(|deleted_at| deleted_at >= updated_at)
        {
            continue;
        }
        transaction
            .execute("DELETE FROM recipe_tombstones WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO recipes (id, updated_at, payload) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                    updated_at = excluded.updated_at,
                    payload = excluded.payload
                 WHERE excluded.updated_at >= recipes.updated_at",
                (
                    id,
                    updated_at,
                    serde_json::to_string(&recipe).map_err(|error| error.to_string())?,
                ),
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

/// A deletion is stored as a tombstone so a delayed save cannot silently bring
/// a Recipe back from a second application window.
#[tauri::command]
fn delete_recipe(
    id: String,
    deleted_at: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<(), String> {
    delete_recipe_inner(id, deleted_at, database.inner())
}

fn delete_recipe_inner(id: String, deleted_at: String, database: &LibraryDb) -> Result<(), String> {
    let id = id.trim();
    let deleted_at = deleted_at.trim();
    if id.is_empty() || deleted_at.is_empty() {
        return Err("recipe deletion requires an ID and timestamp".to_string());
    }
    let mut connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO recipe_tombstones (id, deleted_at) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET deleted_at =
                CASE WHEN excluded.deleted_at > recipe_tombstones.deleted_at
                THEN excluded.deleted_at ELSE recipe_tombstones.deleted_at END",
            (id, deleted_at),
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM recipes WHERE id = ?1 AND updated_at <= ?2",
            (id, deleted_at),
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

fn open_library_database(app: &tauri::App) -> Result<LibraryDb, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(directory.join("recipes.sqlite3")).map_err(|error| error.to_string())?;
    initialise_library_database(&connection)?;
    Ok(LibraryDb(Mutex::new(connection)))
}

fn initialise_library_database(connection: &Connection) -> Result<(), String> {
    // One desktop process owns the connection today, but WAL and a bounded
    // busy timeout make accidental double launches and future background work
    // fail predictably rather than corrupting the local library.
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )
        .map_err(|error| error.to_string())?;
    let schema_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if schema_version > LIBRARY_DB_SCHEMA_VERSION {
        return Err(format!(
            "recipe database schema {schema_version} is newer than this app supports"
        ));
    }
    connection
        .execute_batch(
            "
        CREATE TABLE IF NOT EXISTS recipes (
            id TEXT PRIMARY KEY NOT NULL,
            updated_at TEXT NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS camera_backups (
            id TEXT PRIMARY KEY NOT NULL,
            usb_id TEXT NOT NULL,
            slot INTEGER NOT NULL,
            captured_at TEXT NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS camera_backups_by_device
            ON camera_backups (usb_id, captured_at DESC);
        CREATE TABLE IF NOT EXISTS write_journals (
            id TEXT PRIMARY KEY NOT NULL,
            backup_id TEXT NOT NULL,
            usb_id TEXT NOT NULL,
            slot INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            state TEXT NOT NULL,
            error TEXT,
            payload TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS write_journals_by_backup
            ON write_journals (backup_id);
        CREATE TABLE IF NOT EXISTS local_capability_matrices (
            id TEXT PRIMARY KEY NOT NULL,
            manufacturer TEXT NOT NULL,
            model TEXT NOT NULL,
            firmware TEXT NOT NULL,
            usb_ids TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS local_capability_matrices_by_identity
            ON local_capability_matrices (manufacturer, model, firmware, updated_at DESC);
        ",
        )
        .map_err(|error| error.to_string())?;
    if schema_version < 2 {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS recipe_tombstones (
                    id TEXT PRIMARY KEY NOT NULL,
                    deleted_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS write_journals_by_state
                    ON write_journals (state, created_at DESC);
                ",
            )
            .map_err(|error| error.to_string())?;
    }
    connection
        .pragma_update(None, "user_version", LIBRARY_DB_SCHEMA_VERSION)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn parse_usb_id(value: &str) -> Result<UsbId, String> {
    let (vendor_id, product_id) = value
        .split_once(':')
        .ok_or_else(|| "USB device ID must use VENDOR:PRODUCT format".to_string())?;
    let vendor_id = u16::from_str_radix(vendor_id, 16)
        .map_err(|_| "USB vendor ID is not hexadecimal".to_string())?;
    let product_id = u16::from_str_radix(product_id, 16)
        .map_err(|_| "USB product ID is not hexadecimal".to_string())?;
    Ok(UsbId::new(vendor_id, product_id))
}

/// Source links are attribution only. Keep their scheme constrained here as
/// well as in the frontend because local capability matrices are persisted and
/// may later be displayed by a different view.
fn is_safe_external_url(value: &str) -> bool {
    let value = value.trim();
    let authority = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    authority.is_some_and(|authority| {
        !authority.is_empty()
            && !authority.starts_with('/')
            && authority
                .split('/')
                .next()
                .is_some_and(|host| !host.is_empty())
            && !value.chars().any(char::is_whitespace)
    })
}

fn decode_ptp_string(value: &[u8]) -> Option<String> {
    let count = *value.first()? as usize;
    if count == 0 {
        return (value.len() == 1).then(String::new);
    }
    let expected_len = 1usize.checked_add(count.checked_mul(2)?)?;
    if value.len() != expected_len || value[expected_len - 2..] != [0, 0] {
        return None;
    }
    let bytes = value.get(1..expected_len - 2)?;
    let (pairs, _) = bytes.as_chunks::<2>();
    let units = pairs
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    fn memory_database() -> LibraryDb {
        let connection = Connection::open_in_memory().unwrap();
        initialise_library_database(&connection).unwrap();
        LibraryDb(Mutex::new(connection))
    }

    #[test]
    fn write_recovery_records_are_persisted_before_a_camera_operation() {
        let database = memory_database();
        let backup = CameraBackup {
            id: "backup-test".into(),
            usb_id: "04CB:030C".into(),
            slot: 2,
            captured_at: 1,
            properties: vec![SnapshotProperty {
                code: 0xD195,
                value: vec![7, 0],
            }],
        };
        save_camera_backup(&database, &backup).unwrap();
        let journal = WriteJournal {
            id: "write-test".into(),
            backup_id: backup.id.clone(),
            usb_id: backup.usb_id.clone(),
            slot: backup.slot,
            created_at: 2,
            state: "writing".into(),
            error: None,
        };
        save_write_journal(&database, &journal).unwrap();
        let mut updated = journal.clone();
        updated.state = "rolled_back".into();
        updated.error = Some("D195 mismatch".into());
        update_write_journal(&database, &updated).unwrap();

        let connection = database.0.lock().unwrap();
        let saved_backup: String = connection
            .query_row(
                "SELECT payload FROM camera_backups WHERE id = 'backup-test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let saved_journal: String = connection
            .query_row(
                "SELECT payload FROM write_journals WHERE id = 'write-test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<CameraBackup>(&saved_backup)
                .unwrap()
                .properties,
            backup.properties
        );
        assert_eq!(
            serde_json::from_str::<WriteJournal>(&saved_journal)
                .unwrap()
                .state,
            "rolled_back"
        );
    }

    fn local_matrix() -> LocalCapabilityMatrix {
        LocalCapabilityMatrix {
            schema_version: 1,
            id: "fujifilm-xm5-1.30-local".into(),
            manufacturer: "FUJIFILM".into(),
            model: "X-M5".into(),
            firmware: "1.30".into(),
            usb_ids: vec!["04CB:030C".into()],
            custom_slots: vec!["C1".into(), "C2".into(), "C3".into(), "C4".into()],
            source_url: Some("https://fujifilm-x.com/".into()),
            source_kind: "official".into(),
            evidence_summary: "Read-only observation.".into(),
            last_verified_at: None,
            created_at: 0,
            updated_at: 0,
            properties: vec![LocalCapabilityMatrixProperty {
                key: "filmSimulation".into(),
                label_zh: "軟片模擬".into(),
                label_en: "Film Simulation".into(),
                code: "D192".into(),
                scope: "custom_slot".into(),
                status: "read_detected_unverified".into(),
                notes: "Read-only observation.".into(),
            }],
        }
    }

    #[test]
    fn local_capability_matrix_persists_without_unlocking_writes() {
        let database = memory_database();
        let saved = save_local_capability_matrix_inner(local_matrix(), &database).unwrap();
        assert!(saved.created_at > 0);
        assert!(saved.updated_at >= saved.created_at);
        let listed = list_local_capability_matrices_inner(&database).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].model, "X-M5");
        assert_eq!(listed[0].properties[0].status, "read_detected_unverified");

        let mut unsafe_matrix = local_matrix();
        unsafe_matrix.properties[0].status = "write_verified".into();
        assert!(save_local_capability_matrix_inner(unsafe_matrix, &database)
            .unwrap_err()
            .contains("cannot be locally marked write verified"));
    }

    #[test]
    fn local_capability_matrix_rejects_unsafe_sources_and_invalid_ptp_codes() {
        let database = memory_database();
        let mut unsafe_source = local_matrix();
        unsafe_source.source_url = Some("file:///tmp/evidence".into());
        assert!(save_local_capability_matrix_inner(unsafe_source, &database)
            .unwrap_err()
            .contains("must use http or https"));

        let mut invalid_code = local_matrix();
        invalid_code.properties[0].code = "D19".into();
        assert!(save_local_capability_matrix_inner(invalid_code, &database)
            .unwrap_err()
            .contains("four hexadecimal"));
    }

    #[test]
    fn journal_recovery_is_visible_until_a_verified_manual_restore_acknowledges_it() {
        let database = memory_database();
        let journal = WriteJournal {
            id: "pending-journal".into(),
            backup_id: "backup-pending".into(),
            usb_id: "04CB:030C".into(),
            slot: 4,
            created_at: 10,
            state: "recovery_failed".into(),
            error: Some("cable disconnected".into()),
        };
        save_write_journal(&database, &journal).unwrap();
        let pending = list_pending_write_journals_inner(&database).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].backup_id, "backup-pending");

        mark_pending_journals_recovered(&database, "backup-pending").unwrap();
        assert!(list_pending_write_journals_inner(&database)
            .unwrap()
            .is_empty());
    }

    fn recipe_document(id: &str, updated_at: &str, name: &str) -> RecipeDocument {
        RecipeDocument(serde_json::json!({
            "id": id,
            "updatedAt": updated_at,
            "name": name,
        }))
    }

    #[test]
    fn recipe_upserts_preserve_newer_edits_and_tombstones_block_stale_resurrection() {
        let database = memory_database();
        upsert_recipes_inner(
            vec![recipe_document(
                "recipe-1",
                "2026-09-05T10:00:00.000Z",
                "first",
            )],
            &database,
        )
        .unwrap();
        upsert_recipes_inner(
            vec![recipe_document(
                "recipe-1",
                "2026-09-05T11:00:00.000Z",
                "newer",
            )],
            &database,
        )
        .unwrap();
        upsert_recipes_inner(
            vec![recipe_document(
                "recipe-1",
                "2026-09-05T10:30:00.000Z",
                "stale",
            )],
            &database,
        )
        .unwrap();
        let connection = database.0.lock().unwrap();
        let payload: String = connection
            .query_row(
                "SELECT payload FROM recipes WHERE id = 'recipe-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).unwrap()["name"],
            "newer"
        );
        drop(connection);

        delete_recipe_inner(
            "recipe-1".into(),
            "2026-09-05T12:00:00.000Z".into(),
            &database,
        )
        .unwrap();
        upsert_recipes_inner(
            vec![recipe_document(
                "recipe-1",
                "2026-09-05T11:30:00.000Z",
                "stale",
            )],
            &database,
        )
        .unwrap();
        let connection = database.0.lock().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM recipes WHERE id = 'recipe-1'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        drop(connection);

        upsert_recipes_inner(
            vec![recipe_document(
                "recipe-1",
                "2026-09-05T12:01:00.000Z",
                "restored",
            )],
            &database,
        )
        .unwrap();
        let connection = database.0.lock().unwrap();
        let payload: String = connection
            .query_row(
                "SELECT payload FROM recipes WHERE id = 'recipe-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).unwrap()["name"],
            "restored"
        );
    }

    #[test]
    fn library_database_initialization_records_a_schema_version() {
        let connection = Connection::open_in_memory().unwrap();
        initialise_library_database(&connection).unwrap();
        let schema_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(schema_version, LIBRARY_DB_SCHEMA_VERSION);
    }

    #[test]
    fn capability_catalog_lists_known_models_without_promoting_probe_only_bodies() {
        let catalog = list_camera_capability_catalog();
        assert!(catalog.len() > 10);
        let xm5 = catalog.iter().find(|entry| entry.model == "X-M5").unwrap();
        assert_eq!(xm5.access, "Experimental recipe writes");
        assert!(xm5.trusted_record_id.is_some());
        let xs20 = catalog.iter().find(|entry| entry.model == "X-S20").unwrap();
        assert_eq!(xs20.access, "Probe required");
        assert!(xs20.trusted_record_id.is_none());
    }

    #[test]
    fn restore_accepts_only_expected_recipe_properties() {
        assert!(validate_restorable_property(&SnapshotProperty {
            code: 0xD195,
            value: vec![6, 0],
        })
        .is_ok());
        assert!(validate_restorable_property(&SnapshotProperty {
            code: 0xD199,
            value: vec![2],
        })
        .is_err());
        assert!(validate_restorable_property(&SnapshotProperty {
            code: 0xD1FF,
            value: vec![0, 0],
        })
        .is_err());
    }

    #[test]
    fn ptp_string_requires_its_declared_length_and_terminator() {
        assert_eq!(
            decode_ptp_string(&[3, b'C', 0, b'1', 0, 0, 0]),
            Some("C1".into())
        );
        assert_eq!(decode_ptp_string(&[3, b'C', 0, b'1', 0]), None);
        assert_eq!(decode_ptp_string(&[3, b'C', 0, b'1', 0, 1, 0]), None);
    }

    #[test]
    fn camera_preset_name_accepts_printable_ascii() {
        assert_eq!(
            camera_preset_name("Night Chrome 400", false).unwrap(),
            "Night Chrome 400"
        );
    }

    #[test]
    fn clearing_a_slot_allows_only_the_explicit_empty_name_path() {
        assert!(camera_preset_name("", false).is_err());
        assert_eq!(camera_preset_name("", true).unwrap(), "");
    }

    #[test]
    fn cleared_slot_uses_only_verified_neutral_values() {
        let properties = camera_xm5::encode_recipe(&cleared_xm5_recipe_settings()).unwrap();
        assert_eq!(properties.len(), 19);
        assert_eq!(
            properties
                .iter()
                .filter(|property| property.code == 0xD195)
                .count(),
            2
        );
        assert!(properties.iter().all(|property| {
            matches!(
                property.code,
                0xD18E
                    | 0xD18F
                    | 0xD190
                    | 0xD192
                    | 0xD195
                    | 0xD196
                    | 0xD197
                    | 0xD198
                    | 0xD199
                    | 0xD19A
                    | 0xD19B
                    | 0xD19D
                    | 0xD19E
                    | 0xD19F
                    | 0xD1A0
                    | 0xD1A1
                    | 0xD1A2
                    | 0xD1A4
            )
        }));
    }

    #[test]
    fn raw_snapshot_window_includes_rejected_properties_without_approving_restore() {
        let codes = xm5_raw_snapshot_property_codes();
        assert_eq!(codes.first(), Some(&0xD18D));
        assert_eq!(codes.last(), Some(&0xD1A5));
        assert!(codes.contains(&0xD193));
        assert!(codes.contains(&0xD194));
        assert!(codes.contains(&0xD1A3));
        assert!(validate_restorable_property(&SnapshotProperty {
            code: 0xD193,
            value: vec![0, 0],
        })
        .is_err());
    }

    #[test]
    fn camera_preset_name_rejects_unverified_unicode() {
        assert!(camera_preset_name("台北夜景", false)
            .unwrap_err()
            .contains("printable ASCII"));
    }

    #[test]
    fn camera_preset_name_rejects_more_than_31_utf16_units() {
        assert!(camera_preset_name(&"A".repeat(32), false)
            .unwrap_err()
            .contains("31 UTF-16"));
    }

    #[test]
    fn image_recipe_import_maps_only_known_fujifilm_makernotes() {
        let metadata = serde_json::json!({
            "EXIF:Make": "FUJIFILM",
            "EXIF:Model": "X-M5",
            "EXIF:ISO": 640,
            "MakerNotes:FilmMode": 0x600,
            "MakerNotes:DynamicRangeSetting": 400,
            "MakerNotes:WhiteBalance": 0xFF0,
            "MakerNotes:ColorTemperature": 6500,
            "MakerNotes:WhiteBalanceFineTune": "60 -40",
            "MakerNotes:GrainEffectRoughness": 64,
            "MakerNotes:GrainEffectSize": 16,
            "MakerNotes:ColorChromeEffect": 64,
            "MakerNotes:ColorChromeFXBlue": 32,
            "MakerNotes:HighlightTone": 16,
            "MakerNotes:ShadowTone": -32,
            "MakerNotes:Saturation": 0x100,
            "MakerNotes:Sharpness": 0x82,
            "MakerNotes:NoiseReduction": 0x2E0,
            "MakerNotes:Clarity": -2000
        });
        let draft =
            recipe_from_fujifilm_exif(metadata.as_object().unwrap(), "test.JPG".to_string());
        assert_eq!(draft.model, "X-M5");
        assert_eq!(draft.settings["filmSimulation"], "CLASSIC_CHROME");
        assert_eq!(draft.settings["dynamicRange"], "DR400");
        assert_eq!(draft.settings["whiteBalance"]["mode"], "COLOR_TEMPERATURE");
        assert_eq!(draft.settings["whiteBalance"]["colorTemperatureK"], 6500);
        assert_eq!(draft.settings["whiteBalance"]["shiftR"], 3);
        assert_eq!(draft.settings["whiteBalance"]["shiftB"], -2);
        assert_eq!(draft.settings["grain"]["strength"], "STRONG");
        assert_eq!(draft.settings["grain"]["size"], "SMALL");
        assert_eq!(draft.settings["highlight"], -1.0);
        assert_eq!(draft.settings["shadow"], 2.0);
        assert_eq!(draft.settings["clarity"], -2);
        assert!(draft
            .field_statuses
            .iter()
            .any(|field| field.key == "Smooth Skin Effect" && field.status == "unavailable"));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(open_library_database(app).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            xm5_capability_record,
            list_camera_capability_catalog,
            discover_cameras,
            probe_camera_device_info,
            audit_xm5_unverified_properties,
            import_fujifilm_image_recipe,
            stage_raf_preview,
            read_camera_slot_selector,
            select_camera_slot,
            capture_xm5_raw_preset_snapshot,
            write_xm5_recipe,
            clear_xm5_custom_slot,
            read_xm5_installed_presets,
            list_camera_backups,
            list_pending_write_journals,
            restore_xm5_backup,
            list_recipes,
            upsert_recipes,
            delete_recipe,
            list_local_capability_matrices,
            save_local_capability_matrix,
            delete_local_capability_matrix
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fuji Recipe Manager");
}
