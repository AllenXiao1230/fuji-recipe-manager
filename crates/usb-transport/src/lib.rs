//! USB enumeration behind a small backend abstraction.
//! The default backend is `nusb`, which supports macOS and Windows. Native platform
//! fallbacks can implement `UsbBackend` without changing the camera layer.

use camera_core::UsbId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDevice {
    pub id: UsbId,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub interfaces: Vec<UsbInterface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbInterface {
    pub number: u8,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
}

impl UsbInterface {
    /// USB Still Image / PTP interface: class 06h, subclass 01h, protocol 01h.
    pub const fn is_ptp(&self) -> bool {
        self.class == 0x06 && self.subclass == 0x01 && self.protocol == 0x01
    }
}

pub trait UsbBackend {
    type Error: std::error::Error + Send + Sync + 'static;
    fn list_devices(&self) -> Result<Vec<UsbDevice>, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("USB enumeration failed: {0}")]
    Enumeration(String),
    #[error("PTP probe failed: {0}")]
    PtpProbe(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtpDeviceInfoProbe {
    pub interface_number: u8,
    pub bulk_in_endpoint: u8,
    pub bulk_out_endpoint: u8,
    pub data_bytes: usize,
    pub response_code: u16,
    pub manufacturer: String,
    pub model: String,
    pub device_version: String,
    pub operations_supported: Vec<u16>,
    pub events_supported: Vec<u16>,
    pub device_properties_supported: Vec<u16>,
    pub capture_formats: Vec<u16>,
    pub image_formats: Vec<u16>,
}

/// Read-only result of asking a camera to describe one PTP property. The
/// command opens and closes a standard PTP session but never reads the value of
/// a recipe field and never sends a setting-write operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtpPropertyDescriptorProbe {
    pub interface_number: u8,
    pub bulk_in_endpoint: u8,
    pub bulk_out_endpoint: u8,
    pub property_code: u16,
    pub data_type: u16,
    pub writable: bool,
    pub factory_default: ptp_core::DevicePropertyValue,
    pub current_value: ptp_core::DevicePropertyValue,
}

/// Read-only result of a standard PTP `GetDevicePropValue` transaction. The
/// raw bytes are retained because Fujifilm vendor properties use model-specific
/// encodings that must be validated before they are translated into a Recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtpPropertyValueProbe {
    pub interface_number: u8,
    pub bulk_in_endpoint: u8,
    pub bulk_out_endpoint: u8,
    pub property_code: u16,
    pub value: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct PlatformUsbBackend;

impl UsbBackend for PlatformUsbBackend {
    type Error = TransportError;

    fn list_devices(&self) -> Result<Vec<UsbDevice>, Self::Error> {
        list_devices()
    }
}

#[cfg(feature = "nusb-backend")]
pub fn list_devices() -> Result<Vec<UsbDevice>, TransportError> {
    let devices =
        nusb::list_devices().map_err(|error| TransportError::Enumeration(error.to_string()))?;
    Ok(devices
        .map(|device| UsbDevice {
            id: UsbId::new(device.vendor_id(), device.product_id()),
            manufacturer: device.manufacturer_string().map(str::to_owned),
            product: device.product_string().map(str::to_owned),
            serial_number: device.serial_number().map(str::to_owned),
            interfaces: device
                .interfaces()
                .map(|interface| UsbInterface {
                    number: interface.interface_number(),
                    class: interface.class(),
                    subclass: interface.subclass(),
                    protocol: interface.protocol(),
                })
                .collect(),
        })
        .collect())
}

/// Send only the standard PTP `GetDeviceInfo` command. This does not open a PTP
/// session and cannot change a camera setting. It is intentionally separate from
/// discovery because claiming a camera interface can interrupt other PTP apps.
#[cfg(feature = "nusb-backend")]
pub fn probe_ptp_device_info(id: UsbId) -> Result<PtpDeviceInfoProbe, TransportError> {
    use futures_lite::future::block_on;
    use nusb::transfer::RequestBuffer;
    use ptp_core::{
        command, parse_container_header, parse_device_info, ContainerType, OperationCode,
    };

    let info = nusb::list_devices()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?
        .find(|device| device.vendor_id() == id.vendor_id && device.product_id() == id.product_id)
        .ok_or_else(|| TransportError::PtpProbe(format!("device {id} is not connected")))?;
    let device = info
        .open()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let (interface_number, bulk_in, bulk_out, in_packet_size) = {
        let configuration = device
            .active_configuration()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let mut selected = None;
        for group in configuration.interfaces() {
            for interface in group.alt_settings() {
                if interface.class() != 0x06
                    || interface.subclass() != 0x01
                    || interface.protocol() != 0x01
                {
                    continue;
                }
                let mut bulk_in = None;
                let mut bulk_out = None;
                let mut in_packet_size = 0;
                for endpoint in interface.endpoints() {
                    if endpoint.attributes() & 0x03 != 0x02 {
                        continue;
                    }
                    if endpoint.address() & 0x80 == 0 {
                        bulk_out = Some(endpoint.address());
                    } else {
                        bulk_in = Some(endpoint.address());
                        in_packet_size = endpoint.max_packet_size();
                    }
                }
                if let (Some(bulk_in), Some(bulk_out)) = (bulk_in, bulk_out) {
                    selected = Some((
                        interface.interface_number(),
                        bulk_in,
                        bulk_out,
                        in_packet_size,
                    ));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        selected.ok_or_else(|| TransportError::PtpProbe("no PTP bulk interface found".into()))?
    };
    let interface = device
        .claim_interface(interface_number)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    block_on(interface.bulk_out(bulk_out, command(OperationCode::GetDeviceInfo, 0).to_vec()))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let request_size = 65_536_usize.next_multiple_of(in_packet_size.max(1));
    let data = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let data_header = parse_container_header(&data)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    if data_header.container_type != ContainerType::Data as u16
        || data_header.code != OperationCode::GetDeviceInfo as u16
    {
        return Err(TransportError::PtpProbe(format!(
            "unexpected PTP data container {:04X}/{:04X}",
            data_header.container_type, data_header.code
        )));
    }
    let device_info = parse_device_info(&data[12..data_header.length as usize])
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let response = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let response_header = parse_container_header(&response)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    if response_header.container_type != ContainerType::Response as u16 {
        return Err(TransportError::PtpProbe(format!(
            "unexpected PTP response container {:04X}",
            response_header.container_type
        )));
    }
    Ok(PtpDeviceInfoProbe {
        interface_number,
        bulk_in_endpoint: bulk_in,
        bulk_out_endpoint: bulk_out,
        data_bytes: data_header.length as usize - 12,
        response_code: response_header.code,
        manufacturer: device_info.manufacturer,
        model: device_info.model,
        device_version: device_info.device_version,
        operations_supported: device_info.operations_supported,
        events_supported: device_info.events_supported,
        device_properties_supported: device_info.device_properties_supported,
        capture_formats: device_info.capture_formats,
        image_formats: device_info.image_formats,
    })
}

/// Open a standard PTP session and ask for a single property's descriptor.
///
/// This is intentionally opt-in and should be used one property at a time
/// while building a model/firmware capability record. `GetDevicePropDesc`
/// returns metadata such as data type, access flag, and default/current values;
/// it does not select a custom slot or write to the camera.
#[cfg(feature = "nusb-backend")]
pub fn probe_ptp_property_descriptor(
    id: UsbId,
    property_code: u16,
) -> Result<PtpPropertyDescriptorProbe, TransportError> {
    use futures_lite::future::block_on;
    use nusb::transfer::RequestBuffer;
    use ptp_core::{
        command, command_with_params, parse_container_header, parse_device_property_descriptor,
        ContainerType, OperationCode, RESPONSE_OK,
    };

    let info = nusb::list_devices()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?
        .find(|device| device.vendor_id() == id.vendor_id && device.product_id() == id.product_id)
        .ok_or_else(|| TransportError::PtpProbe(format!("device {id} is not connected")))?;
    let device = info
        .open()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let (interface_number, bulk_in, bulk_out, in_packet_size) = {
        let configuration = device
            .active_configuration()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let mut selected = None;
        for group in configuration.interfaces() {
            for interface in group.alt_settings() {
                if !(UsbInterface {
                    number: interface.interface_number(),
                    class: interface.class(),
                    subclass: interface.subclass(),
                    protocol: interface.protocol(),
                })
                .is_ptp()
                {
                    continue;
                }
                let mut bulk_in = None;
                let mut bulk_out = None;
                let mut in_packet_size = 0;
                for endpoint in interface.endpoints() {
                    if endpoint.attributes() & 0x03 != 0x02 {
                        continue;
                    }
                    if endpoint.address() & 0x80 == 0 {
                        bulk_out = Some(endpoint.address());
                    } else {
                        bulk_in = Some(endpoint.address());
                        in_packet_size = endpoint.max_packet_size();
                    }
                }
                if let (Some(bulk_in), Some(bulk_out)) = (bulk_in, bulk_out) {
                    selected = Some((
                        interface.interface_number(),
                        bulk_in,
                        bulk_out,
                        in_packet_size,
                    ));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        selected.ok_or_else(|| TransportError::PtpProbe("no PTP bulk interface found".into()))?
    };
    let interface = device
        .claim_interface(interface_number)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let request_size = 65_536_usize.next_multiple_of(in_packet_size.max(1));
    let receive_response = || {
        let response = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        parse_container_header(&response)
            .map_err(|error| TransportError::PtpProbe(error.to_string()))
    };

    block_on(interface.bulk_out(
        bulk_out,
        command_with_params(OperationCode::OpenSession, 0, &[1]),
    ))
    .into_result()
    .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let open_response = receive_response()?;
    if open_response.container_type != ContainerType::Response as u16
        || open_response.code != RESPONSE_OK
    {
        return Err(TransportError::PtpProbe(format!(
            "OpenSession was rejected with PTP response {:04X}",
            open_response.code
        )));
    }

    let descriptor_result = (|| {
        block_on(interface.bulk_out(
            bulk_out,
            command_with_params(
                OperationCode::GetDevicePropDesc,
                1,
                &[u32::from(property_code)],
            ),
        ))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let data = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let data_header = parse_container_header(&data)
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        if data_header.container_type != ContainerType::Data as u16
            || data_header.code != OperationCode::GetDevicePropDesc as u16
        {
            return Err(TransportError::PtpProbe(format!(
                "unexpected PTP descriptor data container {:04X}/{:04X}",
                data_header.container_type, data_header.code
            )));
        }
        let response = receive_response()?;
        if response.container_type != ContainerType::Response as u16 || response.code != RESPONSE_OK
        {
            return Err(TransportError::PtpProbe(format!(
                "GetDevicePropDesc was rejected with PTP response {:04X}",
                response.code
            )));
        }
        let descriptor_bytes = &data[12..data_header.length as usize];
        let descriptor = parse_device_property_descriptor(descriptor_bytes).map_err(|error| {
            TransportError::PtpProbe(format!(
                "{error}; descriptor payload is {} bytes ({})",
                descriptor_bytes.len(),
                hex_preview(descriptor_bytes)
            ))
        })?;
        if descriptor.property.0 != property_code {
            return Err(TransportError::PtpProbe(format!(
                "camera described property {:04X}, expected {:04X}",
                descriptor.property.0, property_code
            )));
        }
        Ok(descriptor)
    })();

    // Always ask the camera to close the session we opened. A close failure is
    // reported only if the descriptor query itself succeeded, preserving the
    // more useful query error when both operations fail.
    let close_result =
        block_on(interface.bulk_out(bulk_out, command(OperationCode::CloseSession, 2).to_vec()))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))
            .and_then(|_| receive_response())
            .and_then(|response| {
                if response.container_type == ContainerType::Response as u16
                    && response.code == RESPONSE_OK
                {
                    Ok(())
                } else {
                    Err(TransportError::PtpProbe(format!(
                        "CloseSession was rejected with PTP response {:04X}",
                        response.code
                    )))
                }
            });

    let descriptor = descriptor_result?;
    close_result?;
    Ok(PtpPropertyDescriptorProbe {
        interface_number,
        bulk_in_endpoint: bulk_in,
        bulk_out_endpoint: bulk_out,
        property_code,
        data_type: descriptor.data_type,
        writable: descriptor.writable,
        factory_default: descriptor.factory_default,
        current_value: descriptor.current_value,
    })
}

/// Open a standard PTP session and read one property's raw value. This is a
/// read-only calibration primitive: it never chooses a custom slot and never
/// sends `SetDevicePropValue`. Callers must interpret the returned bytes only
/// after a model/firmware-specific capability record has been validated.
#[cfg(feature = "nusb-backend")]
pub fn probe_ptp_property_value(
    id: UsbId,
    property_code: u16,
) -> Result<PtpPropertyValueProbe, TransportError> {
    use futures_lite::future::block_on;
    use nusb::transfer::RequestBuffer;
    use ptp_core::{
        command, command_with_params, parse_container_header, ContainerType, OperationCode,
        RESPONSE_OK,
    };

    let info = nusb::list_devices()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?
        .find(|device| device.vendor_id() == id.vendor_id && device.product_id() == id.product_id)
        .ok_or_else(|| TransportError::PtpProbe(format!("device {id} is not connected")))?;
    let device = info
        .open()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let (interface_number, bulk_in, bulk_out, in_packet_size) = {
        let configuration = device
            .active_configuration()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let mut selected = None;
        for group in configuration.interfaces() {
            for interface in group.alt_settings() {
                if interface.class() != 0x06
                    || interface.subclass() != 0x01
                    || interface.protocol() != 0x01
                {
                    continue;
                }
                let mut bulk_in = None;
                let mut bulk_out = None;
                let mut in_packet_size = 0;
                for endpoint in interface.endpoints() {
                    if endpoint.attributes() & 0x03 != 0x02 {
                        continue;
                    }
                    if endpoint.address() & 0x80 == 0 {
                        bulk_out = Some(endpoint.address());
                    } else {
                        bulk_in = Some(endpoint.address());
                        in_packet_size = endpoint.max_packet_size();
                    }
                }
                if let (Some(bulk_in), Some(bulk_out)) = (bulk_in, bulk_out) {
                    selected = Some((
                        interface.interface_number(),
                        bulk_in,
                        bulk_out,
                        in_packet_size,
                    ));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        selected.ok_or_else(|| TransportError::PtpProbe("no PTP bulk interface found".into()))?
    };
    let interface = device
        .claim_interface(interface_number)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let request_size = 65_536_usize.next_multiple_of(in_packet_size.max(1));
    let receive_response = || {
        let response = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        parse_container_header(&response)
            .map_err(|error| TransportError::PtpProbe(error.to_string()))
    };

    block_on(interface.bulk_out(
        bulk_out,
        command_with_params(OperationCode::OpenSession, 0, &[1]),
    ))
    .into_result()
    .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let open_response = receive_response()?;
    if open_response.container_type != ContainerType::Response as u16
        || open_response.code != RESPONSE_OK
    {
        return Err(TransportError::PtpProbe(format!(
            "OpenSession was rejected with PTP response {:04X}",
            open_response.code
        )));
    }

    let value_result = (|| {
        block_on(interface.bulk_out(
            bulk_out,
            command_with_params(
                OperationCode::GetDevicePropValue,
                1,
                &[u32::from(property_code)],
            ),
        ))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let data = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let data_header = parse_container_header(&data)
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        if data_header.container_type != ContainerType::Data as u16
            || data_header.code != OperationCode::GetDevicePropValue as u16
        {
            return Err(TransportError::PtpProbe(format!(
                "unexpected PTP property-value data container {:04X}/{:04X}",
                data_header.container_type, data_header.code
            )));
        }
        let response = receive_response()?;
        if response.container_type != ContainerType::Response as u16 || response.code != RESPONSE_OK
        {
            return Err(TransportError::PtpProbe(format!(
                "GetDevicePropValue was rejected with PTP response {:04X}",
                response.code
            )));
        }
        Ok(data[12..data_header.length as usize].to_vec())
    })();

    let close_result =
        block_on(interface.bulk_out(bulk_out, command(OperationCode::CloseSession, 2).to_vec()))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))
            .and_then(|_| receive_response())
            .and_then(|response| {
                if response.container_type == ContainerType::Response as u16
                    && response.code == RESPONSE_OK
                {
                    Ok(())
                } else {
                    Err(TransportError::PtpProbe(format!(
                        "CloseSession was rejected with PTP response {:04X}",
                        response.code
                    )))
                }
            });
    let value = value_result?;
    close_result?;
    Ok(PtpPropertyValueProbe {
        interface_number,
        bulk_in_endpoint: bulk_in,
        bulk_out_endpoint: bulk_out,
        property_code,
        value,
    })
}

/// Select a Fujifilm custom slot through the vendor property. This is the only
/// write primitive exposed during calibration: it accepts C1-C4 values only and
/// is always followed by a separate read-back by the caller.
#[cfg(feature = "nusb-backend")]
pub fn select_custom_slot(id: UsbId, slot: u16) -> Result<(), TransportError> {
    if !(1..=4).contains(&slot) {
        return Err(TransportError::PtpProbe(
            "custom slot must be C1 through C4".into(),
        ));
    }
    set_ptp_property_value(id, 0xD18C, &slot.to_le_bytes())
}

/// Write one raw PTP property value. Callers must use a model-validated
/// property/payload pair and always read the property back afterwards.
#[cfg(feature = "nusb-backend")]
pub fn set_ptp_property_value(
    id: UsbId,
    property_code: u16,
    value: &[u8],
) -> Result<(), TransportError> {
    use futures_lite::future::block_on;
    use nusb::transfer::RequestBuffer;
    use ptp_core::{
        command, command_with_params, data_container, parse_container_header, ContainerType,
        OperationCode, RESPONSE_OK,
    };
    let info = nusb::list_devices()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?
        .find(|device| device.vendor_id() == id.vendor_id && device.product_id() == id.product_id)
        .ok_or_else(|| TransportError::PtpProbe(format!("device {id} is not connected")))?;
    let device = info
        .open()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let (number, bulk_in, bulk_out, packet) = {
        let config = device
            .active_configuration()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let mut result = None;
        for group in config.interfaces() {
            for interface in group.alt_settings() {
                if interface.class() != 0x06
                    || interface.subclass() != 0x01
                    || interface.protocol() != 0x01
                {
                    continue;
                }
                let mut input = None;
                let mut output = None;
                let mut packet = 0;
                for endpoint in interface.endpoints() {
                    if endpoint.attributes() & 0x03 != 0x02 {
                        continue;
                    }
                    if endpoint.address() & 0x80 == 0 {
                        output = Some(endpoint.address());
                    } else {
                        input = Some(endpoint.address());
                        packet = endpoint.max_packet_size();
                    }
                }
                if let (Some(input), Some(output)) = (input, output) {
                    result = Some((interface.interface_number(), input, output, packet));
                    break;
                }
            }
            if result.is_some() {
                break;
            }
        }
        result.ok_or_else(|| TransportError::PtpProbe("no PTP bulk interface found".into()))?
    };
    let interface = device
        .claim_interface(number)
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let request_size = 65_536_usize.next_multiple_of(packet.max(1));
    let receive = || {
        let bytes = block_on(interface.bulk_in(bulk_in, RequestBuffer::new(request_size)))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        parse_container_header(&bytes).map_err(|error| TransportError::PtpProbe(error.to_string()))
    };
    block_on(interface.bulk_out(
        bulk_out,
        command_with_params(OperationCode::OpenSession, 0, &[1]),
    ))
    .into_result()
    .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
    let open = receive()?;
    if open.container_type != ContainerType::Response as u16 || open.code != RESPONSE_OK {
        return Err(TransportError::PtpProbe(format!(
            "OpenSession was rejected with PTP response {:04X}",
            open.code
        )));
    }
    let operation = (|| {
        block_on(interface.bulk_out(
            bulk_out,
            command_with_params(
                OperationCode::SetDevicePropValue,
                1,
                &[u32::from(property_code)],
            ),
        ))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        block_on(interface.bulk_out(
            bulk_out,
            data_container(OperationCode::SetDevicePropValue, 1, value),
        ))
        .into_result()
        .map_err(|error| TransportError::PtpProbe(error.to_string()))?;
        let response = receive()?;
        if response.container_type == ContainerType::Response as u16 && response.code == RESPONSE_OK
        {
            Ok(())
        } else {
            Err(TransportError::PtpProbe(format!(
                "SetDevicePropValue was rejected with PTP response {:04X}",
                response.code
            )))
        }
    })();
    let close =
        block_on(interface.bulk_out(bulk_out, command(OperationCode::CloseSession, 2).to_vec()))
            .into_result()
            .map_err(|error| TransportError::PtpProbe(error.to_string()))
            .and_then(|_| receive())
            .and_then(|response| {
                if response.container_type == ContainerType::Response as u16
                    && response.code == RESPONSE_OK
                {
                    Ok(())
                } else {
                    Err(TransportError::PtpProbe(format!(
                        "CloseSession was rejected with PTP response {:04X}",
                        response.code
                    )))
                }
            });
    operation?;
    close
}

fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(24)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(not(feature = "nusb-backend"))]
pub fn list_devices() -> Result<Vec<UsbDevice>, TransportError> {
    Err(TransportError::Enumeration("no USB backend enabled".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_keeps_identity_data() {
        let device = UsbDevice {
            id: UsbId::new(1, 2),
            manufacturer: None,
            product: None,
            serial_number: None,
            interfaces: vec![UsbInterface {
                number: 0,
                class: 0x06,
                subclass: 0x01,
                protocol: 0x01,
            }],
        };
        assert_eq!(device.id.to_string(), "0001:0002");
        assert!(device.interfaces[0].is_ptp());
    }
}
