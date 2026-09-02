//! Minimal PTP vocabulary. Transport transactions arrive in a later phase.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OperationCode {
    GetDeviceInfo = 0x1001,
    OpenSession = 0x1002,
    CloseSession = 0x1003,
    GetDevicePropDesc = 0x1014,
    GetDevicePropValue = 0x1015,
    SetDevicePropValue = 0x1016,
}

pub const RESPONSE_OK: u16 = 0x2001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DataType {
    I8 = 0x0001,
    U8 = 0x0002,
    I16 = 0x0003,
    U16 = 0x0004,
    I32 = 0x0005,
    U32 = 0x0006,
    String = 0xFFFF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ContainerType {
    Command = 1,
    Data = 2,
    Response = 3,
    Event = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerHeader {
    pub length: u32,
    pub container_type: u16,
    pub code: u16,
    pub transaction_id: u32,
}

pub fn command(operation: OperationCode, transaction_id: u32) -> [u8; 12] {
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&12u32.to_le_bytes());
    bytes[4..6].copy_from_slice(&(ContainerType::Command as u16).to_le_bytes());
    bytes[6..8].copy_from_slice(&(operation as u16).to_le_bytes());
    bytes[8..12].copy_from_slice(&transaction_id.to_le_bytes());
    bytes
}

/// Build a PTP command container with zero or more 32-bit parameters.
/// Standard PTP uses u32 transport parameters even when a property code itself
/// is a u16 value.
pub fn command_with_params(
    operation: OperationCode,
    transaction_id: u32,
    parameters: &[u32],
) -> Vec<u8> {
    let length = 12 + parameters.len() * 4;
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(&(length as u32).to_le_bytes());
    bytes.extend_from_slice(&(ContainerType::Command as u16).to_le_bytes());
    bytes.extend_from_slice(&(operation as u16).to_le_bytes());
    bytes.extend_from_slice(&transaction_id.to_le_bytes());
    for parameter in parameters {
        bytes.extend_from_slice(&parameter.to_le_bytes());
    }
    bytes
}

/// Build a PTP data container for the second phase of a command such as
/// `SetDevicePropValue`.
pub fn data_container(operation: OperationCode, transaction_id: u32, payload: &[u8]) -> Vec<u8> {
    let length = 12 + payload.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(&(length as u32).to_le_bytes());
    bytes.extend_from_slice(&(ContainerType::Data as u16).to_le_bytes());
    bytes.extend_from_slice(&(operation as u16).to_le_bytes());
    bytes.extend_from_slice(&transaction_id.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// Encode a PTP UTF-16LE string, including its terminating NUL. The leading
/// byte is the count of UTF-16 code units including the terminator.
pub fn encode_string(value: &str) -> Result<Vec<u8>, PtpError> {
    let units: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let count = u8::try_from(units.len())
        .map_err(|_| PtpError::InvalidPayload("PTP string is too long"))?;
    let mut bytes = Vec::with_capacity(1 + units.len() * 2);
    bytes.push(count);
    for unit in units {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(bytes)
}

pub fn parse_container_header(bytes: &[u8]) -> Result<ContainerHeader, PtpError> {
    if bytes.len() < 12 {
        return Err(PtpError::InvalidPayload(
            "PTP container header is shorter than 12 bytes",
        ));
    }
    let header = ContainerHeader {
        length: u32::from_le_bytes(bytes[0..4].try_into().expect("fixed slice length")),
        container_type: u16::from_le_bytes(bytes[4..6].try_into().expect("fixed slice length")),
        code: u16::from_le_bytes(bytes[6..8].try_into().expect("fixed slice length")),
        transaction_id: u32::from_le_bytes(bytes[8..12].try_into().expect("fixed slice length")),
    };
    if header.length < 12 || header.length as usize > bytes.len() {
        return Err(PtpError::InvalidPayload("PTP container length is invalid"));
    }
    Ok(header)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub device_version: String,
    pub operations_supported: Vec<u16>,
    pub events_supported: Vec<u16>,
    pub device_properties_supported: Vec<u16>,
    pub capture_formats: Vec<u16>,
    pub image_formats: Vec<u16>,
}

/// Parse the stable, non-sensitive fields from the standard PTP DeviceInfo dataset.
/// The camera serial number is deliberately not returned or logged.
pub fn parse_device_info(bytes: &[u8]) -> Result<DeviceInfo, PtpError> {
    let mut cursor = 0;
    take_u16(bytes, &mut cursor)?; // StandardVersion
    take_u32(bytes, &mut cursor)?; // VendorExtensionID
    take_u16(bytes, &mut cursor)?; // VendorExtensionVersion
    take_string(bytes, &mut cursor)?; // VendorExtensionDesc
    take_u16(bytes, &mut cursor)?; // FunctionalMode
    let operations_supported = take_u16_array(bytes, &mut cursor)?;
    let events_supported = take_u16_array(bytes, &mut cursor)?;
    let device_properties_supported = take_u16_array(bytes, &mut cursor)?;
    let capture_formats = take_u16_array(bytes, &mut cursor)?;
    let image_formats = take_u16_array(bytes, &mut cursor)?;
    let manufacturer = take_string(bytes, &mut cursor)?;
    let model = take_string(bytes, &mut cursor)?;
    let device_version = take_string(bytes, &mut cursor)?;
    let _serial_number = take_string(bytes, &mut cursor)?;
    Ok(DeviceInfo {
        manufacturer,
        model,
        device_version,
        operations_supported,
        events_supported,
        device_properties_supported,
        capture_formats,
        image_formats,
    })
}

fn take_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, PtpError> {
    let value = bytes
        .get(*cursor..*cursor + 2)
        .ok_or(PtpError::InvalidPayload("unexpected end of PTP dataset"))?;
    *cursor += 2;
    Ok(u16::from_le_bytes(
        value.try_into().expect("fixed slice length"),
    ))
}

fn take_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, PtpError> {
    let value = bytes
        .get(*cursor..*cursor + 4)
        .ok_or(PtpError::InvalidPayload("unexpected end of PTP dataset"))?;
    *cursor += 4;
    Ok(u32::from_le_bytes(
        value.try_into().expect("fixed slice length"),
    ))
}

fn take_u16_array(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u16>, PtpError> {
    let count = take_u32(bytes, cursor)? as usize;
    let length = count
        .checked_mul(2)
        .ok_or(PtpError::InvalidPayload("PTP array length overflow"))?;
    let end = cursor
        .checked_add(length)
        .ok_or(PtpError::InvalidPayload("PTP array length overflow"))?;
    if end > bytes.len() {
        return Err(PtpError::InvalidPayload(
            "unexpected end of PTP DeviceInfo array",
        ));
    }
    let (pairs, remainder) = bytes[*cursor..end].as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    let values = pairs
        .iter()
        .map(|value| u16::from_le_bytes(*value))
        .collect();
    *cursor = end;
    Ok(values)
}

fn take_string(bytes: &[u8], cursor: &mut usize) -> Result<String, PtpError> {
    let character_count = *bytes
        .get(*cursor)
        .ok_or(PtpError::InvalidPayload("unexpected end of PTP string"))?
        as usize;
    *cursor += 1;
    if character_count == 0 {
        return Ok(String::new());
    }
    let byte_length = character_count
        .checked_mul(2)
        .ok_or(PtpError::InvalidPayload("PTP string length overflow"))?;
    let encoded = bytes
        .get(*cursor..*cursor + byte_length)
        .ok_or(PtpError::InvalidPayload("unexpected end of PTP string"))?;
    *cursor += byte_length;
    let (chunks, _) = encoded.as_chunks::<2>();
    let code_units: Vec<u16> = chunks
        .iter()
        .map(|chunk| u16::from_le_bytes(*chunk))
        .take_while(|unit| *unit != 0)
        .collect();
    String::from_utf16(&code_units)
        .map_err(|_| PtpError::InvalidPayload("invalid UTF-16 PTP string"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DevicePropertyCode(pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevicePropertyValue {
    U8(u8),
    U16(u16),
    U32(u32),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicePropertyDescriptor {
    pub property: DevicePropertyCode,
    pub data_type: u16,
    pub writable: bool,
    pub factory_default: DevicePropertyValue,
    pub current_value: DevicePropertyValue,
}

/// Parse the stable part of a standard PTP DevicePropDesc dataset. The optional
/// form section is purposefully left as opaque protocol data for now: it varies
/// by property and is not required to decide whether a property can be backed up.
pub fn parse_device_property_descriptor(
    bytes: &[u8],
) -> Result<DevicePropertyDescriptor, PtpError> {
    let mut cursor = 0;
    let property = DevicePropertyCode(take_u16(bytes, &mut cursor)?);
    let data_type = take_u16(bytes, &mut cursor)?;
    let writable = *bytes.get(cursor).ok_or(PtpError::InvalidPayload(
        "unexpected end of PTP property descriptor",
    ))? != 0;
    cursor += 1;
    let factory_default = take_property_value(bytes, &mut cursor, data_type)?;
    let current_value = take_property_value(bytes, &mut cursor, data_type)?;
    Ok(DevicePropertyDescriptor {
        property,
        data_type,
        writable,
        factory_default,
        current_value,
    })
}

fn take_property_value(
    bytes: &[u8],
    cursor: &mut usize,
    data_type: u16,
) -> Result<DevicePropertyValue, PtpError> {
    match data_type {
        value if value == DataType::I8 as u16 => {
            let value = *bytes.get(*cursor).ok_or(PtpError::InvalidPayload(
                "unexpected end of PTP property value",
            ))?;
            *cursor += 1;
            Ok(DevicePropertyValue::U8(value))
        }
        value if value == DataType::U8 as u16 => {
            let value = *bytes.get(*cursor).ok_or(PtpError::InvalidPayload(
                "unexpected end of PTP property value",
            ))?;
            *cursor += 1;
            Ok(DevicePropertyValue::U8(value))
        }
        value if value == DataType::I16 as u16 || value == DataType::U16 as u16 => {
            Ok(DevicePropertyValue::U16(take_u16(bytes, cursor)?))
        }
        value if value == DataType::I32 as u16 || value == DataType::U32 as u16 => {
            Ok(DevicePropertyValue::U32(take_u32(bytes, cursor)?))
        }
        value if value == DataType::String as u16 => {
            Ok(DevicePropertyValue::String(take_string(bytes, cursor)?))
        }
        _ => Err(PtpError::UnsupportedDataType(data_type)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PtpError {
    #[error("PTP operation is not implemented in Phase 0")]
    NotImplemented,
    #[error("invalid PTP payload: {0}")]
    InvalidPayload(&'static str),
    #[error("unsupported PTP property data type: 0x{0:04X}")]
    UnsupportedDataType(u16),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_standard_get_device_info_command() {
        assert_eq!(
            command(OperationCode::GetDeviceInfo, 0),
            [12, 0, 0, 0, 1, 0, 1, 0x10, 0, 0, 0, 0]
        );
    }
    #[test]
    fn parses_response_header() {
        let header = parse_container_header(&[12, 0, 0, 0, 3, 0, 1, 0x20, 0, 0, 0, 0]).unwrap();
        assert_eq!(header.code, 0x2001);
        assert_eq!(header.container_type, ContainerType::Response as u16);
    }
    #[test]
    fn parses_standard_device_info_without_returning_serial() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&100u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        for _ in 0..5 {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }
        for text in ["FUJIFILM", "X-M5", "1.00", "secret"] {
            let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            bytes.push(units.len() as u8);
            for unit in units {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
        }
        let info = parse_device_info(&bytes).unwrap();
        assert_eq!(info.manufacturer, "FUJIFILM");
        assert_eq!(info.model, "X-M5");
        assert_eq!(info.device_version, "1.00");
        assert!(info.device_properties_supported.is_empty());
    }

    #[test]
    fn builds_a_parameterised_property_descriptor_command() {
        assert_eq!(
            command_with_params(OperationCode::GetDevicePropDesc, 1, &[0xD18C]),
            [16, 0, 0, 0, 1, 0, 0x14, 0x10, 1, 0, 0, 0, 0x8C, 0xD1, 0, 0,]
        );
    }

    #[test]
    fn parses_a_scalar_property_descriptor() {
        let bytes = [
            0x8C, 0xD1, // D18C
            0x04, 0x00, // UINT16
            0x01, // writable
            0x01, 0x00, // factory default C1
            0x02, 0x00, // current C2
            0x00, // no form
        ];
        let descriptor = parse_device_property_descriptor(&bytes).unwrap();
        assert_eq!(descriptor.property, DevicePropertyCode(0xD18C));
        assert!(descriptor.writable);
        assert_eq!(descriptor.factory_default, DevicePropertyValue::U16(1));
        assert_eq!(descriptor.current_value, DevicePropertyValue::U16(2));
    }

    #[test]
    fn encodes_a_ptp_string() {
        assert_eq!(encode_string("C1").unwrap(), [3, b'C', 0, b'1', 0, 0, 0]);
    }
}
