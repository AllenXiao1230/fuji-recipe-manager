//! X-M5 profile and recipe codec.
//!
//! USB ID `04CB:030C` and the vendor-property encodings below were observed on
//! a physical X-M5 with firmware 1.30. They remain *experimental*: callers
//! must create a durable backup, write only after explicit confirmation, and
//! read every property back before reporting success.

use camera_core::{CameraCapability, CameraProfile, SupportLevel, UsbId};
use fuji_ptp::FUJIFILM_VENDOR_ID;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const X_M5_USB_IDS: &[UsbId] = &[UsbId::new(FUJIFILM_VENDOR_ID, 0x030C)];
/// Firmware revisions which passed the current experimental codec checklist.
/// New firmware is probe-only until its observed encodings and recovery flow
/// have been added to the hardware validation record.
pub const X_M5_EXPERIMENTAL_WRITE_FIRMWARES: &[&str] = &["1.30"];
pub const X_M5_CAPABILITY: CameraCapability = CameraCapability {
    model: "FUJIFILM X-M5",
    usb_ids: X_M5_USB_IDS,
    custom_slots: 4,
    custom_slot_labels: &["C1", "C2", "C3", "C4"],
    support: SupportLevel::Experimental,
};

pub struct Xm5;
impl CameraProfile for Xm5 {
    fn capability() -> &'static CameraCapability {
        &X_M5_CAPABILITY
    }
}

pub fn is_fujifilm(device: UsbId) -> bool {
    device.vendor_id == FUJIFILM_VENDOR_ID
}

pub fn supports_experimental_write_firmware(device_version: &str) -> bool {
    X_M5_EXPERIMENTAL_WRITE_FIRMWARES.contains(&device_version.trim())
}

/// Versioned, hardware-backed X-M5 capability data. This is deliberately
/// separate from the encoder: the UI can report "verified", "detected but not
/// verified", and "rejected" without treating an observed property as write
/// permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5CapabilityRecord {
    pub schema_version: u16,
    pub record_id: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware: String,
    pub usb_ids: Vec<String>,
    pub custom_slots: Vec<String>,
    pub validation: Xm5CapabilityValidation,
    pub properties: Vec<Xm5CapabilityProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5CapabilityValidation {
    pub method: String,
    pub tested_on: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5CapabilityProperty {
    pub key: String,
    pub code: String,
    pub scope: String,
    pub label_zh: String,
    pub label_en: String,
    pub status: String,
    pub verified_values: String,
    pub dependencies: Vec<String>,
    pub notes: String,
}

/// Return the bundled record, validated on first use. A malformed checked-in
/// manifest is a build-time regression, so failing loudly is preferable to
/// silently unlocking an unknown property.
pub fn capability_record() -> &'static Xm5CapabilityRecord {
    static RECORD: OnceLock<Xm5CapabilityRecord> = OnceLock::new();
    RECORD.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../data/capabilities/fujifilm-xm5-1.30.json"
        ))
        .expect("bundled X-M5 capability record must be valid JSON")
    })
}

/// Recipe values accepted by the X-M5 encoder. This lives in the camera crate
/// rather than the Tauri application so model-specific wire encodings cannot
/// accidentally be reused for a different Fujifilm generation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5RecipeSettings {
    pub film_simulation: String,
    pub dynamic_range: String,
    pub white_balance: Xm5WhiteBalance,
    pub grain: Xm5Grain,
    pub color_chrome_effect: String,
    pub color_chrome_fx_blue: String,
    pub smooth_skin_effect: String,
    #[serde(default)]
    pub monochromatic_color: Xm5MonochromaticColor,
    pub color_space: String,
    pub image_size: String,
    pub image_quality: String,
    pub highlight: f64,
    pub shadow: f64,
    pub color: i16,
    pub sharpness: i16,
    pub high_iso_noise_reduction: i16,
    pub clarity: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5WhiteBalance {
    pub mode: String,
    pub color_temperature_k: i16,
    pub shift_r: i16,
    pub shift_b: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5Grain {
    pub strength: String,
    pub size: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Xm5MonochromaticColor {
    pub warm_cool: i16,
    pub magenta_green: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeProperty {
    pub code: u16,
    pub write_value: Vec<u8>,
    /// Some cameras canonicalise a command value when it is read back. Each
    /// permitted canonical form is listed explicitly instead of weakening
    /// verification for every property.
    pub accepted_read_values: Vec<Vec<u8>>,
}

impl RecipeProperty {
    fn exact(code: u16, value: u16) -> Self {
        let value = value.to_le_bytes().to_vec();
        Self {
            code,
            write_value: value.clone(),
            accepted_read_values: vec![value],
        }
    }

    fn signed_tenths(code: u16, value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("tone value must be finite".to_string());
        }
        let scaled = value * 10.0;
        if (scaled - scaled.round()).abs() > f64::EPSILON
            || scaled < f64::from(i16::MIN)
            || scaled > f64::from(i16::MAX)
        {
            return Err("tone value must use a supported tenth-step encoding".to_string());
        }
        let value = (scaled.round() as i16).to_le_bytes().to_vec();
        Ok(Self {
            code,
            write_value: value.clone(),
            accepted_read_values: vec![value],
        })
    }

    fn signed(code: u16, value: i16) -> Self {
        let value = value.to_le_bytes().to_vec();
        Self {
            code,
            write_value: value.clone(),
            accepted_read_values: vec![value],
        }
    }

    pub fn matches_read_back(&self, observed: &[u8]) -> bool {
        self.accepted_read_values
            .iter()
            .any(|expected| expected.as_slice() == observed)
    }
}

/// Translate user-facing X-M5 values to the observed vendor PTP properties.
/// This function has no USB side effects and is deliberately easy to test.
pub fn encode_recipe(settings: &Xm5RecipeSettings) -> Result<Vec<RecipeProperty>, String> {
    if settings.monochromatic_color.warm_cool != 0
        || settings.monochromatic_color.magenta_green != 0
    {
        return Err(
            "X-M5 monochromatic warm/cool and magenta/green writes were rejected by the camera and remain locked"
                .to_string(),
        );
    }
    if !is_half_step_in_range(settings.highlight, -2.0, 4.0)
        || !is_half_step_in_range(settings.shadow, -2.0, 4.0)
    {
        return Err("X-M5 highlight and shadow must be between -2 and +4 in 0.5 steps".to_string());
    }
    let film: u16 = match settings.film_simulation.as_str() {
        "PROVIA" => 1,
        "VELVIA" => 2,
        "ASTIA" => 3,
        "PRO_NEG_HI" => 4,
        "PRO_NEG_STD" => 5,
        "MONOCHROME" => 6,
        "MONOCHROME_YE" => 7,
        "MONOCHROME_R" => 8,
        "MONOCHROME_G" => 9,
        "SEPIA" => 10,
        "CLASSIC_CHROME" => 11,
        "ACROS" => 12,
        "ACROS_YE" => 13,
        "ACROS_R" => 14,
        "ACROS_G" => 15,
        "ETERNA" => 16,
        "CLASSIC_NEGATIVE" => 17,
        "ETERNA_BLEACH_BYPASS" => 18,
        "NOSTALGIC_NEGATIVE" => 19,
        "REALA_ACE" => 20,
        _ => return Err("unsupported X-M5 film simulation".to_string()),
    };
    let dynamic_range: u16 = match settings.dynamic_range.as_str() {
        "AUTO" => 0xFFFF,
        "DR100" => 100,
        "DR200" => 200,
        "DR400" => 400,
        _ => return Err("unsupported X-M5 dynamic range".to_string()),
    };
    let white_balance: u16 = match settings.white_balance.mode.as_str() {
        "WHITE_PRIORITY" => 0x8020,
        "AUTO" => 2,
        "AMBIENCE_PRIORITY" => 0x8021,
        "DAYLIGHT" => 4,
        "INCANDESCENT" => 6,
        "UNDERWATER" => 8,
        "FLUORESCENT_1" => 0x8001,
        "FLUORESCENT_2" => 0x8002,
        "FLUORESCENT_3" => 0x8003,
        "SHADE" => 0x8006,
        "COLOR_TEMPERATURE" => 0x8007,
        _ => return Err("this white-balance mode is not verified for X-M5 writing".to_string()),
    };
    if !(-9..=9).contains(&settings.white_balance.shift_r)
        || !(-9..=9).contains(&settings.white_balance.shift_b)
    {
        return Err("X-M5 white-balance shifts must be between -9 and +9".to_string());
    }
    if settings.white_balance.mode == "COLOR_TEMPERATURE"
        && (!(2500..=10000).contains(&settings.white_balance.color_temperature_k)
            || settings.white_balance.color_temperature_k % 10 != 0)
    {
        return Err(
            "X-M5 color temperature must be between 2500 and 10000 K in 10 K steps".to_string(),
        );
    }
    if !(-4..=4).contains(&settings.color) || !(-4..=4).contains(&settings.sharpness) {
        return Err("X-M5 color and sharpness must be between -4 and +4".to_string());
    }
    if !(-5..=5).contains(&settings.clarity) {
        return Err("X-M5 clarity must be between -5 and +5".to_string());
    }
    let effect = |value: &str| match value {
        "OFF" => Ok(1_u16),
        "WEAK" => Ok(2_u16),
        "STRONG" => Ok(3_u16),
        _ => Err("unsupported X-M5 Color Chrome setting".to_string()),
    };
    let nr: u16 = match settings.high_iso_noise_reduction {
        -4 => 0x8000,
        -3 => 0x7000,
        -2 => 0x4000,
        -1 => 0x3000,
        0 => 0x2000,
        1 => 0x1000,
        2 => 0,
        3 => 0x6000,
        4 => 0x5000,
        _ => return Err("X-M5 high ISO NR must be between -4 and +4".to_string()),
    };
    let grain_properties = match (
        settings.grain.strength.as_str(),
        settings.grain.size.as_str(),
    ) {
        ("WEAK", "SMALL") => vec![RecipeProperty::exact(0xD195, 2)],
        ("STRONG", "SMALL") => vec![RecipeProperty::exact(0xD195, 3)],
        ("WEAK", "LARGE") => vec![RecipeProperty::exact(0xD195, 4)],
        ("STRONG", "LARGE") => vec![RecipeProperty::exact(0xD195, 5)],
        // X-M5 firmware 1.30 stores the last grain size even while Grain is
        // Off. Establish that size first, then send its Off command (1).
        ("OFF", "SMALL") => grain_off_properties(2, 6),
        ("OFF", "LARGE") => grain_off_properties(4, 7),
        _ => return Err("unsupported X-M5 grain setting".to_string()),
    };
    let smooth_skin: u16 = match settings.smooth_skin_effect.as_str() {
        "OFF" => 1,
        "WEAK" => 2,
        "STRONG" => 3,
        _ => return Err("unsupported X-M5 Smooth Skin setting".to_string()),
    };
    let color_space: u16 = match settings.color_space.as_str() {
        "SRGB" => 1,
        "ADOBE_RGB" => 2,
        _ => return Err("unsupported X-M5 Color Space setting".to_string()),
    };
    // These are intentionally narrow. The property transport itself passed
    // hardware validation, but only the payloads below have a reversible
    // write/read-back record and are safe to expose in this release.
    let image_size: u16 = match settings.image_size.as_str() {
        "S_3_2" => 1,
        "S_16_9" => 2,
        "S_1_1" => 3,
        "M_3_2" => 4,
        "M_16_9" => 5,
        "M_1_1" => 6,
        "L_3_2" => 7,
        "L_16_9" => 8,
        "L_1_1" => 9,
        _ => {
            return Err("this image size is detected but not verified for X-M5 writing".to_string())
        }
    };
    let image_quality: u16 = match settings.image_quality.as_str() {
        "FINE" => 2,
        "NORMAL" => 3,
        "FINE_PLUS_RAW" => 4,
        "NORMAL_PLUS_RAW" => 5,
        "RAW" => 1,
        _ => {
            return Err(
                "this image quality is detected but not verified for X-M5 writing".to_string(),
            )
        }
    };

    let mut properties = vec![
        RecipeProperty::exact(0xD190, dynamic_range),
        RecipeProperty::exact(0xD192, film),
    ];
    properties.extend(grain_properties);
    properties.extend([
        RecipeProperty::exact(0xD196, effect(&settings.color_chrome_effect)?),
        RecipeProperty::exact(0xD197, effect(&settings.color_chrome_fx_blue)?),
        RecipeProperty::exact(0xD198, smooth_skin),
        RecipeProperty::exact(0xD199, white_balance),
        // WB shift is an unscaled signed 16-bit value; tone controls below use ×10.
        RecipeProperty::signed(0xD19A, settings.white_balance.shift_r),
        RecipeProperty::signed(0xD19B, settings.white_balance.shift_b),
        RecipeProperty::signed_tenths(0xD19D, settings.highlight)?,
        RecipeProperty::signed_tenths(0xD19E, settings.shadow)?,
        RecipeProperty::signed_tenths(0xD19F, f64::from(settings.color))?,
        RecipeProperty::signed_tenths(0xD1A0, f64::from(settings.sharpness))?,
        RecipeProperty::exact(0xD1A1, nr),
        RecipeProperty::signed_tenths(0xD1A2, f64::from(settings.clarity))?,
        RecipeProperty::exact(0xD1A4, color_space),
        RecipeProperty::exact(0xD18E, image_size),
        RecipeProperty::exact(0xD18F, image_quality),
    ]);
    if settings.white_balance.mode == "COLOR_TEMPERATURE" {
        let temperature =
            RecipeProperty::exact(0xD19C, settings.white_balance.color_temperature_k as u16);
        // The camera requires the white-balance mode to be selected before
        // colour temperature. Insert immediately after D199, preserving the
        // safe dependency order during both write and read-back verification.
        let wb_index = properties
            .iter()
            .position(|property| property.code == 0xD199)
            .expect("white-balance property is always encoded");
        properties.insert(wb_index + 1, temperature);
    }
    Ok(properties)
}

fn grain_off_properties(size_command: u16, expected_off_value: u16) -> Vec<RecipeProperty> {
    vec![
        RecipeProperty::exact(0xD195, size_command),
        RecipeProperty {
            code: 0xD195,
            write_value: 1_u16.to_le_bytes().to_vec(),
            accepted_read_values: vec![expected_off_value.to_le_bytes().to_vec()],
        },
    ]
}

fn is_half_step_in_range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite()
        && value >= min
        && value <= max
        && ((value * 2.0) - (value * 2.0).round()).abs() <= f64::EPSILON
}

/// Build all verified restore writes from a captured raw value.
///
/// Grain Off is represented by `06 00` (small) or `07 00` (large). The X-M5
/// rejects either canonical value when sent back verbatim, so recovery first
/// establishes its retained size and only then sends the Off command (`01 00`).
pub fn restore_property_steps(code: u16, captured_value: &[u8]) -> Vec<RecipeProperty> {
    if code == 0xD195 {
        return match captured_value {
            [6, 0] => grain_off_properties(2, 6),
            [7, 0] => grain_off_properties(4, 7),
            _ => vec![RecipeProperty {
                code,
                write_value: captured_value.to_vec(),
                accepted_read_values: vec![captured_value.to_vec()],
            }],
        };
    }
    vec![RecipeProperty {
        code,
        write_value: captured_value.to_vec(),
        accepted_read_values: vec![captured_value.to_vec()],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declares_four_custom_slots() {
        assert_eq!(Xm5::capability().custom_slots, 4);
    }

    #[test]
    fn firmware_gate_is_exact() {
        assert!(supports_experimental_write_firmware("1.30"));
        assert!(!supports_experimental_write_firmware("1.31"));
    }

    fn recipe() -> Xm5RecipeSettings {
        Xm5RecipeSettings {
            film_simulation: "CLASSIC_CHROME".into(),
            dynamic_range: "DR400".into(),
            white_balance: Xm5WhiteBalance {
                mode: "AUTO".into(),
                color_temperature_k: 6500,
                shift_r: 3,
                shift_b: -5,
            },
            grain: Xm5Grain {
                strength: "OFF".into(),
                size: "SMALL".into(),
            },
            color_chrome_effect: "STRONG".into(),
            color_chrome_fx_blue: "WEAK".into(),
            smooth_skin_effect: "OFF".into(),
            monochromatic_color: Xm5MonochromaticColor::default(),
            color_space: "SRGB".into(),
            image_size: "L_3_2".into(),
            image_quality: "FINE".into(),
            highlight: -1.0,
            shadow: -2.0,
            color: 2,
            sharpness: -2,
            high_iso_noise_reduction: -4,
            clarity: -2,
        }
    }

    #[test]
    fn grain_off_accepts_xm5_canonical_readback() {
        let properties = encode_recipe(&recipe()).unwrap();
        let grain = properties
            .iter()
            .filter(|property| property.code == 0xD195)
            .collect::<Vec<_>>();
        assert_eq!(grain.len(), 2);
        assert_eq!(grain[0].write_value, [2, 0]);
        assert_eq!(grain[1].write_value, [1, 0]);
        assert!(grain[1].matches_read_back(&[6, 0]));
        assert!(!grain[1].matches_read_back(&[7, 0]));
        assert!(!grain[1].matches_read_back(&[5, 0]));
    }

    #[test]
    fn white_balance_shift_uses_unscaled_signed_values() {
        let properties = encode_recipe(&recipe()).unwrap();
        let red = properties
            .iter()
            .find(|property| property.code == 0xD19A)
            .unwrap();
        let blue = properties
            .iter()
            .find(|property| property.code == 0xD19B)
            .unwrap();
        assert_eq!(red.write_value, [3, 0]);
        assert_eq!(blue.write_value, [251, 255]);
    }

    #[test]
    fn rejects_out_of_range_white_balance_shift() {
        let mut invalid = recipe();
        invalid.white_balance.shift_r = 10;
        assert!(encode_recipe(&invalid)
            .unwrap_err()
            .contains("white-balance shifts"));
    }

    #[test]
    fn rejects_unverified_white_balance() {
        let mut invalid = recipe();
        invalid.white_balance.mode = "KELVIN".into();
        assert!(encode_recipe(&invalid)
            .unwrap_err()
            .contains("not verified"));
    }

    #[test]
    fn color_temperature_follows_white_balance_mode() {
        let mut recipe = recipe();
        recipe.white_balance.mode = "COLOR_TEMPERATURE".into();
        recipe.white_balance.color_temperature_k = 6500;
        let properties = encode_recipe(&recipe).unwrap();
        let wb = properties
            .iter()
            .position(|property| property.code == 0xD199)
            .unwrap();
        assert_eq!(properties[wb].write_value, [7, 128]);
        assert_eq!(properties[wb + 1].code, 0xD19C);
        assert_eq!(properties[wb + 1].write_value, 6500_u16.to_le_bytes());
    }

    #[test]
    fn rejects_color_temperature_outside_the_hardware_record() {
        let mut recipe = recipe();
        recipe.white_balance.mode = "COLOR_TEMPERATURE".into();
        recipe.white_balance.color_temperature_k = 10005;
        assert!(encode_recipe(&recipe)
            .unwrap_err()
            .contains("color temperature"));
    }

    #[test]
    fn only_hardware_verified_image_payloads_are_encoded() {
        let mut recipe = recipe();
        recipe.image_size = "M_3_2".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "M_16_9".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "M_1_1".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "S_3_2".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "S_16_9".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "S_1_1".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "L_3_2".into();
        recipe.image_size = "L_16_9".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "L_1_1".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_size = "L_3_2".into();
        recipe.image_quality = "NORMAL".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_quality = "FINE_PLUS_RAW".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_quality = "NORMAL_PLUS_RAW".into();
        assert!(encode_recipe(&recipe).is_ok());
        recipe.image_quality = "RAW".into();
        assert!(encode_recipe(&recipe).is_ok());
    }

    #[test]
    fn all_hardware_verified_additional_enums_are_encoded() {
        let mut recipe = recipe();
        for film_simulation in [
            "PRO_NEG_HI",
            "PRO_NEG_STD",
            "MONOCHROME",
            "MONOCHROME_YE",
            "MONOCHROME_R",
            "MONOCHROME_G",
            "SEPIA",
            "ACROS_YE",
            "ACROS_R",
            "ACROS_G",
            "ETERNA_BLEACH_BYPASS",
            "NOSTALGIC_NEGATIVE",
        ] {
            recipe.film_simulation = film_simulation.into();
            assert!(encode_recipe(&recipe).is_ok(), "{film_simulation}");
        }
        recipe.dynamic_range = "AUTO".into();
        assert!(encode_recipe(&recipe).is_ok());
        for white_balance in ["WHITE_PRIORITY", "AMBIENCE_PRIORITY"] {
            recipe.white_balance.mode = white_balance.into();
            assert!(encode_recipe(&recipe).is_ok(), "{white_balance}");
        }
    }

    #[test]
    fn rejects_monochromatic_values_that_xm5_hardware_rejected() {
        let mut recipe = recipe();
        recipe.monochromatic_color.warm_cool = 1;
        assert!(encode_recipe(&recipe)
            .unwrap_err()
            .contains("monochromatic"));
    }

    #[test]
    fn bundled_capability_record_matches_xm5_identity() {
        let record = capability_record();
        assert_eq!(record.model, "X-M5");
        assert_eq!(record.firmware, "1.30");
        assert!(record.usb_ids.iter().any(|id| id == "04CB:030C"));
        assert!(record
            .properties
            .iter()
            .any(|property| property.key == "longExposureNoiseReduction"
                && property.status == "write_rejected"));
    }

    #[test]
    fn grain_off_backup_restores_the_captured_size_before_off() {
        let small = restore_property_steps(0xD195, &[6, 0]);
        assert_eq!(small.len(), 2);
        assert_eq!(small[0].write_value, [2, 0]);
        assert_eq!(small[1].write_value, [1, 0]);
        assert!(small[1].matches_read_back(&[6, 0]));

        let large = restore_property_steps(0xD195, &[7, 0]);
        assert_eq!(large[0].write_value, [4, 0]);
        assert_eq!(large[1].write_value, [1, 0]);
        assert!(large[1].matches_read_back(&[7, 0]));
    }

    #[test]
    fn tone_curve_uses_half_step_encoding() {
        let mut recipe = recipe();
        recipe.highlight = 0.5;
        recipe.shadow = -1.5;
        let properties = encode_recipe(&recipe).unwrap();
        let highlight = properties
            .iter()
            .find(|property| property.code == 0xD19D)
            .unwrap();
        let shadow = properties
            .iter()
            .find(|property| property.code == 0xD19E)
            .unwrap();
        assert_eq!(highlight.write_value, [5, 0]);
        assert_eq!(shadow.write_value, [241, 255]);
    }

    #[test]
    fn rejects_non_half_step_tone_curve_values() {
        let mut recipe = recipe();
        recipe.highlight = 0.25;
        assert!(encode_recipe(&recipe).unwrap_err().contains("0.5 steps"));
    }
}
