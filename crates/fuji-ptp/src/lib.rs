//! Fujifilm vendor PTP identifiers. Values are represented, not written, in Phase 0.

use ptp_core::DevicePropertyCode;

pub const FUJIFILM_VENDOR_ID: u16 = 0x04CB;
pub const CUSTOM_SLOT: DevicePropertyCode = DevicePropertyCode(0xD18C);
pub const CUSTOM_PRESET_NAME: DevicePropertyCode = DevicePropertyCode(0xD18D);
pub const RECIPE_PARAMETERS_START: DevicePropertyCode = DevicePropertyCode(0xD18E);
pub const RECIPE_PARAMETERS_END: DevicePropertyCode = DevicePropertyCode(0xD1A5);

pub fn is_recipe_property(property: DevicePropertyCode) -> bool {
    (CUSTOM_SLOT.0..=RECIPE_PARAMETERS_END.0).contains(&property.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_slot_is_a_recipe_property() {
        assert!(is_recipe_property(CUSTOM_SLOT));
        assert!(is_recipe_property(RECIPE_PARAMETERS_END));
    }
}
