//! Shared camera domain types.  This crate is intentionally independent of USB and PTP.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UsbId {
    pub vendor_id: u16,
    pub product_id: u16,
}

impl UsbId {
    pub const fn new(vendor_id: u16, product_id: u16) -> Self {
        Self {
            vendor_id,
            product_id,
        }
    }
}

impl fmt::Display for UsbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04X}:{:04X}", self.vendor_id, self.product_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    Verified,
    Experimental,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraCapability {
    pub model: &'static str,
    pub usb_ids: &'static [UsbId],
    pub custom_slots: u8,
    pub custom_slot_labels: &'static [&'static str],
    pub support: SupportLevel,
}

pub trait CameraProfile {
    fn capability() -> &'static CameraCapability;
}

pub fn identify_by_usb_id(
    capabilities: &'static [&'static CameraCapability],
    id: UsbId,
) -> Option<&'static CameraCapability> {
    capabilities
        .iter()
        .copied()
        .find(|capability| capability.usb_ids.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_id_is_uppercase_hex() {
        assert_eq!(UsbId::new(0x04cb, 0x123).to_string(), "04CB:0123");
    }
}
