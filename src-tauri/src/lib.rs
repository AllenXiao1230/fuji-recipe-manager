use std::{
    fs,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use camera_core::UsbId;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use usb_transport::UsbBackend;

struct LibraryDb(Mutex<Connection>);

static RECORD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
struct RecipeDocument(serde_json::Value);

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
struct SlotSelectionResult {
    usb_id: String,
    slot: u16,
    read_back_hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeWriteResult {
    slot: u16,
    verified_properties: Vec<String>,
    backup_id: String,
    journal_id: String,
    preset_name: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstalledPreset {
    slot: u16,
    name: String,
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
fn probe_camera_device_info(usb_id: String) -> Result<PtpDeviceInfoResult, String> {
    let id = parse_usb_id(&usb_id)?;
    let probe = usb_transport::probe_ptp_device_info(id).map_err(|error| error.to_string())?;
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
    })
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

#[tauri::command]
fn write_xm5_recipe(
    usb_id: String,
    slot: u16,
    settings: camera_xm5::Xm5RecipeSettings,
    name: String,
    database: tauri::State<'_, LibraryDb>,
) -> Result<RecipeWriteResult, String> {
    if !(1..=4).contains(&slot) {
        return Err("slot must be between C1 and C4".to_string());
    }
    let id = parse_usb_id(&usb_id)?;
    if id != UsbId::new(0x04CB, 0x030C) {
        return Err("writing is currently verified only for FUJIFILM X-M5 (04CB:030C)".to_string());
    }
    let device_info =
        usb_transport::probe_ptp_device_info(id).map_err(|error| error.to_string())?;
    if !camera_xm5::supports_experimental_write_firmware(&device_info.device_version) {
        return Err(format!(
            "X-M5 firmware {} is probe-only. Experimental writing is currently gated to firmware {}.",
            device_info.device_version,
            camera_xm5::X_M5_EXPERIMENTAL_WRITE_FIRMWARES.join(", ")
        ));
    }
    let preset_name = camera_preset_name(&name)?;
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
        let captured = properties
            .iter()
            .map(|property| {
                usb_transport::probe_ptp_property_value(id, property.code)
                    .map(|probe| SnapshotProperty {
                        code: property.code,
                        value: probe.value,
                    })
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let backup = CameraBackup {
            id: new_record_id("backup"),
            usb_id: id.to_string(),
            slot,
            captured_at: timestamp_millis(),
            properties: captured,
        };
        save_camera_backup(database.inner(), &backup)?;
        let mut journal = WriteJournal {
            id: new_record_id("write"),
            backup_id: backup.id.clone(),
            usb_id: id.to_string(),
            slot,
            created_at: timestamp_millis(),
            state: "writing".to_string(),
            error: None,
        };
        save_write_journal(database.inner(), &journal)?;

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
                update_write_journal(database.inner(), &journal)?;
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
        update_write_journal(database.inner(), &journal)?;
        Ok(RecipeWriteResult {
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

fn camera_preset_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
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
        let restore = camera_xm5::restore_property(captured.code, &captured.value);
        let result = usb_transport::set_ptp_property_value(id, restore.code, &restore.write_value)
            .and_then(|_| usb_transport::probe_ptp_property_value(id, restore.code));
        match result {
            Ok(probe) if restore.matches_read_back(&probe.value) => {}
            Ok(probe) => failures.push(format!(
                "{:04X} read-back mismatch ({})",
                restore.code,
                format_hex(&probe.value)
            )),
            Err(error) => failures.push(format!("{:04X} {error}", restore.code)),
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
            | 0xD190
            | 0xD192
            | 0xD195
            | 0xD196
            | 0xD197
            | 0xD199
            | 0xD19A
            | 0xD19B
            | 0xD19D
            | 0xD19E
            | 0xD19F
            | 0xD1A0
            | 0xD1A1
            | 0xD1A2
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
    if id != UsbId::new(0x04CB, 0x030C) {
        return Err(
            "backup restoration is currently verified only for FUJIFILM X-M5 (04CB:030C)"
                .to_string(),
        );
    }
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
        (Ok(()), Ok(())) => Ok(CameraBackupSummary {
            id: backup.id,
            usb_id: backup.usb_id,
            slot: backup.slot,
            captured_at: backup.captured_at,
            property_count: backup.properties.len(),
        }),
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
    if id != UsbId::new(0x04CB, 0x030C) {
        return Err(
            "installed-preset reading is currently verified only for FUJIFILM X-M5".to_string(),
        );
    }
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

#[tauri::command]
fn replace_recipes(
    recipes: Vec<RecipeDocument>,
    database: tauri::State<'_, LibraryDb>,
) -> Result<(), String> {
    let mut connection = database
        .0
        .lock()
        .map_err(|_| "recipe database lock failed".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM recipes", [])
        .map_err(|error| error.to_string())?;
    for RecipeDocument(recipe) in recipes {
        let id = recipe
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("recipe missing id")?;
        let updated_at = recipe
            .get("updatedAt")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        transaction
            .execute(
                "INSERT INTO recipes (id, updated_at, payload) VALUES (?1, ?2, ?3)",
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

fn open_library_database(app: &tauri::App) -> Result<LibraryDb, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(directory.join("recipes.sqlite3")).map_err(|error| error.to_string())?;
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
        ",
        )
        .map_err(|error| error.to_string())?;
    Ok(LibraryDb(Mutex::new(connection)))
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
        connection
            .execute_batch(
                "
                CREATE TABLE camera_backups (id TEXT PRIMARY KEY, usb_id TEXT, slot INTEGER, captured_at TEXT, payload TEXT);
                CREATE TABLE write_journals (id TEXT PRIMARY KEY, backup_id TEXT, usb_id TEXT, slot INTEGER, created_at TEXT, state TEXT, error TEXT, payload TEXT);
                ",
            )
            .unwrap();
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
            camera_preset_name("Night Chrome 400").unwrap(),
            "Night Chrome 400"
        );
    }

    #[test]
    fn camera_preset_name_rejects_unverified_unicode() {
        assert!(camera_preset_name("台北夜景")
            .unwrap_err()
            .contains("printable ASCII"));
    }

    #[test]
    fn camera_preset_name_rejects_more_than_31_utf16_units() {
        assert!(camera_preset_name(&"A".repeat(32))
            .unwrap_err()
            .contains("31 UTF-16"));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(open_library_database(app).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            discover_cameras,
            probe_camera_device_info,
            read_camera_slot_selector,
            select_camera_slot,
            write_xm5_recipe,
            read_xm5_installed_presets,
            list_camera_backups,
            restore_xm5_backup,
            list_recipes,
            replace_recipes
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fuji Recipe Manager");
}
