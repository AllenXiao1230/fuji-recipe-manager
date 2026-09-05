//! Fujifilm-wide recognition and safety classification.
//! Model recognition is deliberately separate from recipe-write support: a recognised
//! Fujifilm body is safe to inspect, but never safe to write until its PTP profile has
//! passed hardware validation.

use camera_core::UsbId;
use fuji_ptp::FUJIFILM_VENDOR_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FujiFamily {
    XSeries,
    X100,
    Gfx,
    FinePix,
    Other,
}

impl FujiFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::XSeries => "X Series",
            Self::X100 => "X100 Series",
            Self::Gfx => "GFX Series",
            Self::FinePix => "FinePix Series",
            Self::Other => "Fujifilm camera",
        }
    }
}

impl RecipeAccess {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ProbeRequired => "Probe required",
            Self::ReadOnlyValidated => "Read-only PTP verified",
            Self::WriteExperimental => "Experimental recipe writes",
            Self::WriteValidated => "Recipe writes verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeAccess {
    /// Detection is safe; protocol capability is not known for this model.
    ProbeRequired,
    /// Read-only PTP property access has been confirmed on this model.
    ReadOnlyValidated,
    /// A model-specific encoder and recovery workflow exist, but the complete
    /// hardware/release matrix is not yet sufficient for a production claim.
    WriteExperimental,
    /// Recipe writes passed a model-specific hardware test plan.
    WriteValidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FujiModelProfile {
    pub model: &'static str,
    pub family: FujiFamily,
    pub recipe_access: RecipeAccess,
}

/// Model names that the UI can recognise from USB/PTP strings. Every entry still
/// starts probe-required, because the product string alone cannot prove firmware or
/// property encoding compatibility.
pub const KNOWN_MODELS: &[FujiModelProfile] = &[
    FujiModelProfile {
        model: "X-M5",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::WriteExperimental,
    },
    FujiModelProfile {
        model: "X-S20",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-S10",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-T5",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-T4",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-T3",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-T50",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-T30",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-H2S",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-H2",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-H1",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-PRO3",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-E4",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X-E3",
        family: FujiFamily::XSeries,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X100VI",
        family: FujiFamily::X100,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X100V",
        family: FujiFamily::X100,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "X100F",
        family: FujiFamily::X100,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "GFX100 II",
        family: FujiFamily::Gfx,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "GFX100S II",
        family: FujiFamily::Gfx,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "GFX100S",
        family: FujiFamily::Gfx,
        recipe_access: RecipeAccess::ProbeRequired,
    },
    FujiModelProfile {
        model: "GFX50S II",
        family: FujiFamily::Gfx,
        recipe_access: RecipeAccess::ProbeRequired,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FujiRecognition {
    pub family: FujiFamily,
    pub model: Option<&'static str>,
    pub recipe_access: RecipeAccess,
}

/// The decision produced after pairing a USB ID with PTP DeviceInfo model and
/// firmware. Recognition is not enough: this result is the sole model-level
/// gate used by write entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityRecordState {
    /// USB ID, PTP model and firmware exactly match a checked-in record.
    ExactExperimental,
    /// The body matches a known record but its firmware does not.
    FirmwareProbeOnly,
    /// The USB ID and firmware do not identify a checked-in model record.
    ProbeOnly,
    /// This is not a Fujifilm USB device.
    NotFujifilm,
}

impl CapabilityRecordState {
    /// Stable machine-readable state for frontend gates and stored diagnostics.
    /// `label` may be translated or clarified without changing program logic.
    pub const fn key(self) -> &'static str {
        match self {
            Self::ExactExperimental => "exact_experimental",
            Self::FirmwareProbeOnly => "firmware_probe_only",
            Self::ProbeOnly => "probe_only",
            Self::NotFujifilm => "not_fujifilm",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ExactExperimental => "Exact experimental capability record",
            Self::FirmwareProbeOnly => "Known model, unverified firmware (probe only)",
            Self::ProbeOnly => "No exact capability record (probe only)",
            Self::NotFujifilm => "Not a Fujifilm camera",
        }
    }

    pub const fn next_action(self) -> &'static str {
        match self {
            Self::ExactExperimental => {
                "A disposable C slot may use the backup, write, read-back, restore workflow."
            }
            Self::FirmwareProbeOnly | Self::ProbeOnly => {
                "Run read-only scanning first; add a model and firmware capability record before any write test."
            }
            Self::NotFujifilm => "No Fujifilm PTP workflow is available.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRecordResolution {
    pub state: CapabilityRecordState,
    pub record_id: Option<&'static str>,
}

/// Resolve a physical Fujifilm identity to a checked-in capability record.
///
/// This is intentionally conservative: a matching USB product ID without the
/// matching PTP model and firmware remains probe-only. Additional per-model
/// crates can register records here without changing USB transport code.
pub fn resolve_capability_record(
    id: UsbId,
    device_model: &str,
    device_firmware: &str,
) -> CapabilityRecordResolution {
    if !is_fujifilm(id) {
        return CapabilityRecordResolution {
            state: CapabilityRecordState::NotFujifilm,
            record_id: None,
        };
    }
    let record = camera_xm5::capability_record();
    let same_usb_id = record
        .usb_ids
        .iter()
        .any(|usb_id| usb_id == &id.to_string());
    let normalised_model = device_model
        .to_ascii_uppercase()
        .replace("FUJIFILM", "")
        .trim()
        .to_string();
    let same_model = normalised_model == record.model.to_ascii_uppercase();
    if same_usb_id && same_model {
        return CapabilityRecordResolution {
            state: if device_firmware.trim() == record.firmware {
                CapabilityRecordState::ExactExperimental
            } else {
                CapabilityRecordState::FirmwareProbeOnly
            },
            record_id: Some(record.record_id.as_str()),
        };
    }
    CapabilityRecordResolution {
        state: CapabilityRecordState::ProbeOnly,
        record_id: None,
    }
}

pub const fn is_fujifilm(id: UsbId) -> bool {
    id.vendor_id == FUJIFILM_VENDOR_ID
}

pub fn recognise(id: UsbId, product: Option<&str>) -> Option<FujiRecognition> {
    if !is_fujifilm(id) {
        return None;
    }
    let name = product.unwrap_or_default().to_ascii_uppercase();
    if let Some(profile) = KNOWN_MODELS
        .iter()
        .find(|profile| name.contains(profile.model))
    {
        return Some(FujiRecognition {
            family: profile.family,
            model: Some(profile.model),
            recipe_access: profile.recipe_access,
        });
    }
    let family = if name.contains("X100") {
        FujiFamily::X100
    } else if name.contains("GFX") {
        FujiFamily::Gfx
    } else if name.contains("FINEPIX") {
        FujiFamily::FinePix
    } else if name.contains("X-") || name.starts_with('X') {
        FujiFamily::XSeries
    } else {
        FujiFamily::Other
    };
    Some(FujiRecognition {
        family,
        model: None,
        recipe_access: RecipeAccess::ProbeRequired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognises_known_model_without_unlocking_writes() {
        let device = recognise(UsbId::new(FUJIFILM_VENDOR_ID, 1), Some("FUJIFILM X-T5")).unwrap();
        assert_eq!(device.model, Some("X-T5"));
        assert_eq!(device.recipe_access, RecipeAccess::ProbeRequired);
    }

    #[test]
    fn marks_xm5_writes_as_experimental_not_production_verified() {
        let device = recognise(UsbId::new(FUJIFILM_VENDOR_ID, 1), Some("FUJIFILM X-M5")).unwrap();
        assert_eq!(device.recipe_access, RecipeAccess::WriteExperimental);
    }
    #[test]
    fn classifies_unlisted_fujifilm_as_probe_required() {
        let device = recognise(
            UsbId::new(FUJIFILM_VENDOR_ID, 1),
            Some("FUJIFILM GFX Future"),
        )
        .unwrap();
        assert_eq!(device.family, FujiFamily::Gfx);
        assert_eq!(device.model, None);
    }
    #[test]
    fn ignores_other_vendors() {
        assert_eq!(recognise(UsbId::new(0x05ac, 1), Some("X-T5")), None);
    }

    #[test]
    fn capability_record_requires_usb_model_and_firmware_match() {
        let exact = resolve_capability_record(
            UsbId::new(FUJIFILM_VENDOR_ID, 0x030C),
            "FUJIFILM X-M5",
            "1.30",
        );
        assert_eq!(exact.state, CapabilityRecordState::ExactExperimental);
        assert_eq!(exact.record_id, Some("fujifilm-xm5-04cb-030c-fw-1.30"));

        let firmware_mismatch =
            resolve_capability_record(UsbId::new(FUJIFILM_VENDOR_ID, 0x030C), "X-M5", "1.31");
        assert_eq!(
            firmware_mismatch.state,
            CapabilityRecordState::FirmwareProbeOnly
        );

        let unlisted = resolve_capability_record(
            UsbId::new(FUJIFILM_VENDOR_ID, 0x1234),
            "FUJIFILM X-T5",
            "2.00",
        );
        assert_eq!(unlisted.state, CapabilityRecordState::ProbeOnly);
    }
}
