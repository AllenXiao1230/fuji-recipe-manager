//! X-M5 profile and recipe codec.
//!
//! USB ID `04CB:030C` and the vendor-property encodings below were observed on
//! a physical X-M5 with firmware 1.30. They remain *experimental*: callers
//! must create a durable backup, write only after explicit confirmation, and
//! read every property back before reporting success.

use camera_core::{CameraCapability, CameraProfile, SupportLevel, UsbId};
use fuji_ptp::FUJIFILM_VENDOR_ID;
use serde::Deserialize;

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
    pub shift_r: i16,
    pub shift_b: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Xm5Grain {
    pub strength: String,
    pub size: String,
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
    if !is_half_step_in_range(settings.highlight, -4.0, 4.0)
        || !is_half_step_in_range(settings.shadow, -4.0, 4.0)
    {
        return Err("X-M5 highlight and shadow must be between -4 and +4 in 0.5 steps".to_string());
    }
    let film: u16 = match settings.film_simulation.as_str() {
        "PROVIA" => 1,
        "VELVIA" => 2,
        "ASTIA" => 3,
        "CLASSIC_CHROME" => 11,
        "ACROS" => 12,
        "ETERNA" => 16,
        "CLASSIC_NEGATIVE" => 17,
        "REALA_ACE" => 20,
        _ => return Err("unsupported X-M5 film simulation".to_string()),
    };
    let dynamic_range: u16 = match settings.dynamic_range.as_str() {
        "DR100" => 100,
        "DR200" => 200,
        "DR400" => 400,
        _ => return Err("unsupported X-M5 dynamic range".to_string()),
    };
    let white_balance: u16 = match settings.white_balance.mode.as_str() {
        "AUTO" => 2,
        "DAYLIGHT" => 4,
        "INCANDESCENT" => 6,
        "UNDERWATER" => 8,
        "FLUORESCENT_1" => 0x8001,
        "FLUORESCENT_2" => 0x8002,
        "FLUORESCENT_3" => 0x8003,
        "SHADE" => 0x8006,
        _ => return Err("this white-balance mode is not verified for X-M5 writing".to_string()),
    };
    if !(-9..=9).contains(&settings.white_balance.shift_r)
        || !(-9..=9).contains(&settings.white_balance.shift_b)
    {
        return Err("X-M5 white-balance shifts must be between -9 and +9".to_string());
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
    let grain = match (
        settings.grain.strength.as_str(),
        settings.grain.size.as_str(),
    ) {
        ("WEAK", "SMALL") => RecipeProperty::exact(0xD195, 2),
        ("STRONG", "SMALL") => RecipeProperty::exact(0xD195, 3),
        ("WEAK", "LARGE") => RecipeProperty::exact(0xD195, 4),
        ("STRONG", "LARGE") => RecipeProperty::exact(0xD195, 5),
        // Firmware 1.30 accepts 1 for Off and reads it back as 6.
        ("OFF", _) => RecipeProperty {
            code: 0xD195,
            write_value: 1_u16.to_le_bytes().to_vec(),
            accepted_read_values: vec![1_u16.to_le_bytes().to_vec(), 6_u16.to_le_bytes().to_vec()],
        },
        _ => return Err("unsupported X-M5 grain setting".to_string()),
    };

    Ok(vec![
        RecipeProperty::exact(0xD190, dynamic_range),
        RecipeProperty::exact(0xD192, film),
        grain,
        RecipeProperty::exact(0xD196, effect(&settings.color_chrome_effect)?),
        RecipeProperty::exact(0xD197, effect(&settings.color_chrome_fx_blue)?),
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
    ])
}

fn is_half_step_in_range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite()
        && value >= min
        && value <= max
        && ((value * 2.0) - (value * 2.0).round()).abs() <= f64::EPSILON
}

/// Build a verified restore operation from raw values captured before a write.
/// X-M5 firmware 1.30 cannot be restored by sending its canonical Grain Off
/// read-back (`06 00`) verbatim; it must receive the command form (`01 00`).
pub fn restore_property(code: u16, captured_value: &[u8]) -> RecipeProperty {
    if code == 0xD195 && captured_value == [6, 0] {
        return RecipeProperty {
            code,
            write_value: 1_u16.to_le_bytes().to_vec(),
            accepted_read_values: vec![6_u16.to_le_bytes().to_vec()],
        };
    }
    RecipeProperty {
        code,
        write_value: captured_value.to_vec(),
        accepted_read_values: vec![captured_value.to_vec()],
    }
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
                shift_r: 3,
                shift_b: -5,
            },
            grain: Xm5Grain {
                strength: "OFF".into(),
                size: "SMALL".into(),
            },
            color_chrome_effect: "STRONG".into(),
            color_chrome_fx_blue: "WEAK".into(),
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
            .find(|property| property.code == 0xD195)
            .unwrap();
        assert_eq!(grain.write_value, [1, 0]);
        assert!(grain.matches_read_back(&[6, 0]));
        assert!(!grain.matches_read_back(&[7, 0]));
        assert!(!grain.matches_read_back(&[5, 0]));
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
    fn grain_off_backup_uses_the_command_form_for_restore() {
        let restore = restore_property(0xD195, &[6, 0]);
        assert_eq!(restore.write_value, [1, 0]);
        assert!(restore.matches_read_back(&[6, 0]));
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
