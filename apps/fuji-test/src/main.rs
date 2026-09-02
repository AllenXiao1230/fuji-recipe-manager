//! Read-only Phase 0 USB discovery tool.

use usb_transport::{PlatformUsbBackend, UsbBackend};

#[derive(Debug, Clone)]
struct VerificationCase {
    label: String,
    property: u16,
    write_value: Vec<u8>,
    accepted_read_values: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct ImagePayloadCalibration {
    slot: u16,
    property: u16,
    kind: &'static str,
    setting: &'static str,
}

impl VerificationCase {
    fn exact(label: impl Into<String>, property: u16, value: u16) -> Self {
        let value = value.to_le_bytes().to_vec();
        Self {
            label: label.into(),
            property,
            accepted_read_values: vec![value.clone()],
            write_value: value,
        }
    }

    fn signed(label: impl Into<String>, property: u16, value: i16) -> Self {
        let value = value.to_le_bytes().to_vec();
        Self {
            label: label.into(),
            property,
            accepted_read_values: vec![value.clone()],
            write_value: value,
        }
    }
}

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let request_ptp_probe = arguments.iter().any(|argument| argument == "ptp-info");
    let readonly_xm5_scan_request = arguments
        .iter()
        .any(|argument| argument == "ptp-scan-xm5-readonly");
    let readonly_fujifilm_scan_request = arguments
        .iter()
        .any(|argument| argument == "ptp-scan-readonly");
    let xm5_descriptor_scan_request = arguments
        .iter()
        .any(|argument| argument == "ptp-scan-xm5-descriptors");
    let extended_xm5_verification_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-verify-xm5-extended")
        .and_then(|arguments| {
            arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())
                .filter(|slot| (1..=4).contains(slot))
        });
    let global_current_value_verification_request = arguments
        .iter()
        .any(|argument| argument == "ptp-verify-xm5-global-current");
    let additional_enum_verification_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-verify-xm5-additional-enums")
        .and_then(|arguments| {
            arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())
                .filter(|slot| (1..=4).contains(slot))
        });
    let image_payload_calibration_request = arguments
        .windows(4)
        .find(|arguments| arguments[0] == "ptp-capture-xm5-image-payload")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            if !(1..=4).contains(&slot) {
                return None;
            }
            parse_xm5_image_payload_calibration(slot, &arguments[2], &arguments[3])
        });
    let image_payload_verification_request = arguments
        .windows(5)
        .find(|arguments| arguments[0] == "ptp-verify-xm5-image-payload")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            let request = parse_xm5_image_payload_calibration(slot, &arguments[2], &arguments[3])?;
            let observed_raw_value = parse_u16_value(&arguments[4])?;
            Some((request, observed_raw_value))
        });
    let descriptor_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-descriptor")
        .and_then(|arguments| parse_property_code(&arguments[1]));
    let property_value_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-read")
        .and_then(|arguments| parse_property_code(&arguments[1]));
    let slot_selection_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-select-slot")
        .and_then(|arguments| arguments[1].strip_prefix('C'))
        .and_then(|value| value.parse::<u16>().ok());
    let name_write_request = arguments
        .windows(3)
        .find(|arguments| arguments[0] == "ptp-set-name")
        .and_then(|arguments| {
            arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())
                .map(|slot| (slot, arguments[2].clone()))
        });
    let name_test_request = arguments
        .windows(3)
        .find(|arguments| arguments[0] == "ptp-test-name")
        .and_then(|arguments| {
            arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())
                .map(|slot| (slot, arguments[2].clone()))
        });
    let dynamic_range_test_request = arguments
        .windows(3)
        .find(|arguments| arguments[0] == "ptp-test-dynamic-range")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            let raw_value = match arguments[2].as_str() {
                "DR100" => 100_u16,
                "DR200" => 200_u16,
                "DR400" => 400_u16,
                _ => return None,
            };
            Some((slot, raw_value))
        });
    let signed_property_test_request = arguments
        .windows(4)
        .find(|arguments| arguments[0] == "ptp-test-signed")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            let property = parse_property_code(&arguments[2])?;
            let value = arguments[3].parse::<i16>().ok()?;
            if !matches!(property, 0xD19A..=0xD1A0 | 0xD1A2) {
                return None;
            }
            Some((slot, property, value))
        });
    let enum_property_test_request = arguments
        .windows(4)
        .find(|arguments| arguments[0] == "ptp-test-u16")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            let property = parse_property_code(&arguments[2])?;
            let value = parse_u16_value(&arguments[3])?;
            if !matches!(property, 0xD192 | 0xD195..=0xD199 | 0xD1A1) {
                return None;
            }
            Some((slot, property, value))
        });
    let grain_restore_request = arguments
        .windows(3)
        .find(|arguments| arguments[0] == "ptp-restore-grain")
        .and_then(|arguments| {
            let slot = arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())?;
            let value = parse_u16_value(&arguments[2])?;
            Some((slot, value))
        });
    let full_verification_request = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "ptp-verify-xm5-recipe")
        .and_then(|arguments| {
            arguments[1]
                .strip_prefix('C')
                .and_then(|slot| slot.parse::<u16>().ok())
                .filter(|slot| (1..=4).contains(slot))
        });
    if (arguments
        .iter()
        .any(|argument| argument == "ptp-descriptor")
        && descriptor_request.is_none())
        || (arguments.iter().any(|argument| argument == "ptp-read")
            && property_value_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-select-slot")
            && slot_selection_request.is_none())
        || (arguments.iter().any(|argument| argument == "ptp-set-name")
            && name_write_request.is_none())
        || (arguments.iter().any(|argument| argument == "ptp-test-name")
            && name_test_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-test-dynamic-range")
            && dynamic_range_test_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-test-signed")
            && signed_property_test_request.is_none())
        || (arguments.iter().any(|argument| argument == "ptp-test-u16")
            && enum_property_test_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-restore-grain")
            && grain_restore_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-verify-xm5-recipe")
            && full_verification_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-verify-xm5-extended")
            && extended_xm5_verification_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-capture-xm5-image-payload")
            && image_payload_calibration_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-verify-xm5-image-payload")
            && image_payload_verification_request.is_none())
        || (arguments
            .iter()
            .any(|argument| argument == "ptp-verify-xm5-additional-enums")
            && additional_enum_verification_request.is_none())
    {
        eprintln!("Usage: fuji-test ptp-info | ptp-scan-readonly | ptp-scan-xm5-readonly | ptp-scan-xm5-descriptors | ptp-descriptor D18C | ptp-read D18C | ptp-select-slot C1 | ptp-set-name C1 FRM-TEST | ptp-test-name C2 FRM-TEST | ptp-test-dynamic-range C2 DR100 | ptp-test-signed C2 D19D 0 | ptp-test-u16 C2 D196 1 | ptp-verify-xm5-recipe C4 | ptp-verify-xm5-extended C4 | ptp-verify-xm5-global-current | ptp-verify-xm5-additional-enums C4 | ptp-capture-xm5-image-payload C4 image-size L_16_9 | ptp-verify-xm5-image-payload C4 image-size L_16_9 0x0008");
        std::process::exit(2);
    }
    let backend = PlatformUsbBackend;
    match backend.list_devices() {
        Ok(devices) => {
            println!("Found {} USB device(s).", devices.len());
            let ptp_candidates: Vec<_> = devices
                .iter()
                .filter(|device| {
                    camera_fujifilm::is_fujifilm(device.id)
                        && device
                            .interfaces
                            .iter()
                            .any(usb_transport::UsbInterface::is_ptp)
                })
                .collect();
            for device in &devices {
                let recognition = camera_fujifilm::recognise(device.id, device.product.as_deref());
                let label = recognition
                    .map(|camera| match camera.model {
                        Some(model) => format!(
                            "FUJIFILM {} ({}, {})",
                            model,
                            camera.family.label(),
                            camera.recipe_access.label(),
                        ),
                        None => format!(
                            "FUJIFILM {} (unlisted model, probe required)",
                            camera.family.label()
                        ),
                    })
                    .unwrap_or_else(|| "not a Fujifilm camera".to_owned());
                println!("{} — {}", device.id, label);
                if let Some(name) = device.product.as_deref().or(device.manufacturer.as_deref()) {
                    println!("  {}", name);
                }
                for interface in &device.interfaces {
                    println!(
                        "  interface {}: {:02X}/{:02X}/{:02X}{}",
                        interface.number,
                        interface.class,
                        interface.subclass,
                        interface.protocol,
                        if interface.is_ptp() {
                            " (PTP candidate)"
                        } else {
                            ""
                        },
                    );
                }
            }
            println!("Fujifilm devices are discoverable across model families; each model/firmware remains probe-only unless its capability record explicitly permits an experimental write.");
            if let Some(slot) = full_verification_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no write test was run.");
                    std::process::exit(1);
                };
                run_xm5_recipe_verification(device.id, slot);
                return;
            }
            if let Some(slot) = extended_xm5_verification_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no extended write test was run.");
                    std::process::exit(1);
                };
                run_xm5_extended_verification(device.id, slot);
                return;
            }
            if global_current_value_verification_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no global write test was run.");
                    std::process::exit(1);
                };
                run_xm5_global_current_value_verification(device.id);
                return;
            }
            if let Some(slot) = additional_enum_verification_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no additional enum verification was run.");
                    std::process::exit(1);
                };
                run_xm5_additional_enum_verification(device.id, slot);
                return;
            }
            if let Some(request) = image_payload_calibration_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no calibration capture was run.");
                    std::process::exit(1);
                };
                run_xm5_image_payload_capture(device.id, request);
                return;
            }
            if let Some((request, observed_raw_value)) = image_payload_verification_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!("X-M5 (04CB:030C) with a PTP interface was not found; no image payload write verification was run.");
                    std::process::exit(1);
                };
                run_xm5_image_payload_verification(device.id, request, observed_raw_value);
                return;
            }
            if readonly_xm5_scan_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!(
                        "X-M5 (04CB:030C) with a PTP interface was not found; no scan was run."
                    );
                    std::process::exit(1);
                };
                run_xm5_readonly_property_scan(device.id);
                return;
            }
            if readonly_fujifilm_scan_request {
                if ptp_candidates.is_empty() {
                    println!("No Fujifilm PTP interface is available for a read-only scan.");
                }
                for device in &ptp_candidates {
                    run_fujifilm_readonly_property_scan(device.id);
                }
                return;
            }
            if xm5_descriptor_scan_request {
                let Some(device) = ptp_candidates
                    .iter()
                    .copied()
                    .find(|device| device.id.vendor_id == 0x04CB && device.id.product_id == 0x030C)
                else {
                    eprintln!(
                        "X-M5 (04CB:030C) with a PTP interface was not found; no descriptor scan was run."
                    );
                    std::process::exit(1);
                };
                run_xm5_descriptor_scan(device.id);
                return;
            }
            if request_ptp_probe {
                if ptp_candidates.is_empty() {
                    println!("No Fujifilm PTP interface is available for a DeviceInfo probe.");
                }
                for device in &ptp_candidates {
                    println!("Probing standard PTP DeviceInfo on {}…", device.id);
                    match usb_transport::probe_ptp_device_info(device.id) {
                        Ok(probe) => println!("  PTP response {:04X}; {} {} firmware {}; data {} bytes; interface {} / bulk OUT {:02X} / bulk IN {:02X}", probe.response_code, probe.manufacturer, probe.model, probe.device_version, probe.data_bytes, probe.interface_number, probe.bulk_out_endpoint, probe.bulk_in_endpoint),
                        Err(error) => {
                            println!("  probe failed: {error}");
                            if error.to_string().contains("exclusive access") {
                                println!("  action: close Image Capture, Photos, X RAW Studio, and any tethering app; then reconnect the camera and retry.");
                            }
                        }
                    }
                }
            } else {
                println!(
                    "Run `fuji-test ptp-info` to send the standard read-only GetDeviceInfo probe."
                );
            }
            if let Some(property_code) = descriptor_request {
                for device in &ptp_candidates {
                    println!(
                        "Reading PTP descriptor {:04X} on {} (opens then closes a read-only session)…",
                        property_code, device.id
                    );
                    match usb_transport::probe_ptp_property_descriptor(device.id, property_code) {
                        Ok(probe) => println!(
                            "  descriptor {:04X}; type {:04X}; {}; default {:?}; current {:?}; interface {} / bulk OUT {:02X} / bulk IN {:02X}",
                            probe.property_code,
                            probe.data_type,
                            if probe.writable { "camera marks it writable" } else { "camera marks it read-only" },
                            probe.factory_default,
                            probe.current_value,
                            probe.interface_number,
                            probe.bulk_out_endpoint,
                            probe.bulk_in_endpoint,
                        ),
                        Err(error) => {
                            println!("  descriptor probe failed: {error}");
                            if error.to_string().contains("exclusive access") {
                                println!("  action: close Image Capture, Photos, X RAW Studio, and any tethering app; then reconnect the camera and retry.");
                            }
                        }
                    }
                }
            }
            if let Some(property_code) = property_value_request {
                for device in &ptp_candidates {
                    println!(
                        "Reading raw PTP value {:04X} on {} (opens then closes a read-only session)…",
                        property_code, device.id
                    );
                    match usb_transport::probe_ptp_property_value(device.id, property_code) {
                        Ok(probe) => println!(
                            "  value {:04X}; {} byte(s): {}; interface {} / bulk OUT {:02X} / bulk IN {:02X}",
                            probe.property_code,
                            probe.value.len(),
                            format_hex(&probe.value),
                            probe.interface_number,
                            probe.bulk_out_endpoint,
                            probe.bulk_in_endpoint,
                        ),
                        Err(error) => {
                            println!("  value probe failed: {error}");
                            if error.to_string().contains("exclusive access") {
                                println!("  action: close Image Capture, Photos, X RAW Studio, and any tethering app; then reconnect the camera and retry.");
                            }
                        }
                    }
                }
            }
            if let Some(slot) = slot_selection_request {
                for device in &ptp_candidates {
                    println!("Selecting C{slot} on {} and reading it back…", device.id);
                    match usb_transport::select_custom_slot(device.id, slot) {
                        Ok(()) => {
                            match usb_transport::probe_ptp_property_value(device.id, 0xD18C) {
                                Ok(probe) if probe.value == slot.to_le_bytes() => {
                                    println!("  C{slot} selected and verified by read-back.")
                                }
                                Ok(probe) => println!(
                                    "  selection read-back mismatch: {}",
                                    format_hex(&probe.value)
                                ),
                                Err(error) => {
                                    println!("  selection succeeded but read-back failed: {error}")
                                }
                            }
                        }
                        Err(error) => println!("  slot selection failed: {error}"),
                    }
                }
            }
            if let Some((slot, name)) = name_write_request {
                let encoded = match ptp_core::encode_string(&name) {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("Invalid preset name: {error}");
                        std::process::exit(2);
                    }
                };
                for device in &ptp_candidates {
                    println!("Selecting C{slot}, writing its preset name, then reading it back…");
                    let result = usb_transport::select_custom_slot(device.id, slot)
                        .and_then(|_| {
                            usb_transport::set_ptp_property_value(device.id, 0xD18D, &encoded)
                        })
                        .and_then(|_| usb_transport::probe_ptp_property_value(device.id, 0xD18D));
                    match result {
                        Ok(probe) if probe.value == encoded => {
                            println!("  preset name write/read-back verified for C{slot}.")
                        }
                        Ok(probe) => println!(
                            "  preset-name read-back mismatch: {}",
                            format_hex(&probe.value)
                        ),
                        Err(error) => println!("  preset-name validation failed: {error}"),
                    }
                }
            }
            if let Some((slot, name)) = name_test_request {
                let encoded = match ptp_core::encode_string(&name) {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("Invalid preset name: {error}");
                        std::process::exit(2);
                    }
                };
                for device in &ptp_candidates {
                    println!(
                        "Testing C{slot} preset name with a reversible write/read-back/restore sequence…"
                    );
                    let baseline = usb_transport::select_custom_slot(device.id, slot)
                        .and_then(|_| usb_transport::probe_ptp_property_value(device.id, 0xD18D));
                    let Ok(baseline) = baseline else {
                        println!("  could not read C{slot}'s original preset name.");
                        continue;
                    };
                    let original = baseline.value;
                    let target_result =
                        usb_transport::set_ptp_property_value(device.id, 0xD18D, &encoded)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, 0xD18D)
                            });
                    let restore_result =
                        usb_transport::set_ptp_property_value(device.id, 0xD18D, &original)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, 0xD18D)
                            });
                    match (target_result, restore_result) {
                        (Ok(target_probe), Ok(restore_probe))
                            if target_probe.value == encoded && restore_probe.value == original =>
                        {
                            println!(
                                "  preset name write/read-back verified; original value restored and verified."
                            );
                        }
                        (target_result, restore_result) => {
                            println!(
                                "  preset-name test did not fully verify; target={:?}; restore={:?}.",
                                target_result.map(|probe| format_hex(&probe.value)),
                                restore_result.map(|probe| format_hex(&probe.value)),
                            );
                        }
                    }
                }
            }
            if let Some((slot, raw_value)) = dynamic_range_test_request {
                const DYNAMIC_RANGE_PROPERTY: u16 = 0xD190;
                let target = raw_value.to_le_bytes();
                for device in &ptp_candidates {
                    println!(
                        "Testing C{slot} dynamic range with a reversible write/read-back/restore sequence…"
                    );
                    let baseline =
                        usb_transport::select_custom_slot(device.id, slot).and_then(|_| {
                            usb_transport::probe_ptp_property_value(
                                device.id,
                                DYNAMIC_RANGE_PROPERTY,
                            )
                        });
                    let Ok(baseline) = baseline else {
                        println!("  could not read C{slot}'s original dynamic-range value.");
                        continue;
                    };
                    if baseline.value.len() != 2 {
                        println!(
                            "  unexpected dynamic-range value length {}; test skipped.",
                            baseline.value.len()
                        );
                        continue;
                    }
                    let original = baseline.value;
                    let target_result = usb_transport::set_ptp_property_value(
                        device.id,
                        DYNAMIC_RANGE_PROPERTY,
                        &target,
                    )
                    .and_then(|_| {
                        usb_transport::probe_ptp_property_value(device.id, DYNAMIC_RANGE_PROPERTY)
                    });
                    let restore_result = usb_transport::set_ptp_property_value(
                        device.id,
                        DYNAMIC_RANGE_PROPERTY,
                        &original,
                    )
                    .and_then(|_| {
                        usb_transport::probe_ptp_property_value(device.id, DYNAMIC_RANGE_PROPERTY)
                    });
                    match (target_result, restore_result) {
                        (Ok(target_probe), Ok(restore_probe))
                            if target_probe.value == target && restore_probe.value == original =>
                        {
                            println!(
                                "  DR{} write/read-back verified; original {} restored and verified.",
                                raw_value,
                                format_hex(&original)
                            );
                        }
                        (target_result, restore_result) => {
                            println!(
                                "  dynamic-range test did not fully verify; target={:?}; restore={:?}.",
                                target_result.map(|probe| format_hex(&probe.value)),
                                restore_result.map(|probe| format_hex(&probe.value)),
                            );
                        }
                    }
                }
            }
            if let Some((slot, property, value)) = signed_property_test_request {
                let target = value.to_le_bytes();
                for device in &ptp_candidates {
                    println!(
                        "Testing C{slot} property {property:04X} with a reversible write/read-back/restore sequence…"
                    );
                    let baseline = usb_transport::select_custom_slot(device.id, slot)
                        .and_then(|_| usb_transport::probe_ptp_property_value(device.id, property));
                    let Ok(baseline) = baseline else {
                        println!("  could not read C{slot}'s original property value.");
                        continue;
                    };
                    if baseline.value.len() != 2 {
                        println!(
                            "  unexpected property value length {}; test skipped.",
                            baseline.value.len()
                        );
                        continue;
                    }
                    let original = baseline.value;
                    let target_result =
                        usb_transport::set_ptp_property_value(device.id, property, &target)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, property)
                            });
                    let restore_value = if property == 0xD195 && original == [0x06, 0x00] {
                        // The X-M5 reports grain-off as 6 but rejects 6 when it is sent
                        // back verbatim. Its accepted grain-off command is 1 and reads back 6.
                        vec![0x01, 0x00]
                    } else {
                        original.clone()
                    };
                    let restore_result =
                        usb_transport::set_ptp_property_value(device.id, property, &restore_value)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, property)
                            });
                    match (target_result, restore_result) {
                        (Ok(target_probe), Ok(restore_probe))
                            if target_probe.value == target && restore_probe.value == original =>
                        {
                            println!(
                                "  property {property:04X} write/read-back verified; original {} restored and verified.",
                                format_hex(&original)
                            );
                        }
                        (target_result, restore_result) => {
                            println!(
                                "  property {property:04X} test did not fully verify; target={:?}; restore={:?}.",
                                target_result.map(|probe| format_hex(&probe.value)),
                                restore_result.map(|probe| format_hex(&probe.value)),
                            );
                        }
                    }
                }
            }
            if let Some((slot, property, value)) = enum_property_test_request {
                let target = value.to_le_bytes();
                for device in &ptp_candidates {
                    println!(
                        "Testing C{slot} property {property:04X} with a reversible write/read-back/restore sequence…"
                    );
                    let baseline = usb_transport::select_custom_slot(device.id, slot)
                        .and_then(|_| usb_transport::probe_ptp_property_value(device.id, property));
                    let Ok(baseline) = baseline else {
                        println!("  could not read C{slot}'s original property value.");
                        continue;
                    };
                    if baseline.value.len() != 2 {
                        println!(
                            "  unexpected property value length {}; test skipped.",
                            baseline.value.len()
                        );
                        continue;
                    }
                    let original = baseline.value;
                    let target_result =
                        usb_transport::set_ptp_property_value(device.id, property, &target)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, property)
                            });
                    let restore_result =
                        usb_transport::set_ptp_property_value(device.id, property, &original)
                            .and_then(|_| {
                                usb_transport::probe_ptp_property_value(device.id, property)
                            });
                    match (target_result, restore_result) {
                        (Ok(target_probe), Ok(restore_probe))
                            if target_probe.value == target && restore_probe.value == original =>
                        {
                            println!(
                                "  property {property:04X} write/read-back verified; original {} restored and verified.",
                                format_hex(&original)
                            );
                        }
                        (target_result, restore_result) => {
                            println!(
                                "  property {property:04X} test did not fully verify; target={:?}; restore={:?}.",
                                target_result.map(|probe| format_hex(&probe.value)),
                                restore_result.map(|probe| format_hex(&probe.value)),
                            );
                        }
                    }
                }
            }
            if let Some((slot, value)) = grain_restore_request {
                for device in &ptp_candidates {
                    println!("Applying X-M5 grain recovery value {value} to C{slot} and reading it back…");
                    let result = usb_transport::select_custom_slot(device.id, slot)
                        .and_then(|_| {
                            usb_transport::set_ptp_property_value(
                                device.id,
                                0xD195,
                                &value.to_le_bytes(),
                            )
                        })
                        .and_then(|_| usb_transport::probe_ptp_property_value(device.id, 0xD195));
                    match result {
                        Ok(probe) => println!("  grain read-back: {}", format_hex(&probe.value)),
                        Err(error) => println!("  grain recovery attempt failed: {error}"),
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("Could not enumerate USB devices: {error}");
            std::process::exit(1);
        }
    }
}

/// Uses only GetDeviceInfo and GetDevicePropValue. In particular, this does
/// not select a C slot, open a descriptor request, or invoke a write command.
/// A successful read says that a property is observable; it is not evidence
/// that its encoding or ownership (global versus C-slot) is safe to write.
fn run_xm5_readonly_property_scan(id: camera_core::UsbId) {
    use std::collections::BTreeMap;

    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) => info,
        Err(error) => {
            eprintln!("Cannot begin read-only scan: {error}");
            return;
        }
    };
    println!(
        "Read-only X-M5 scan: {} {} firmware {}; {} advertised device property code(s).",
        info.manufacturer,
        info.model,
        info.device_version,
        info.device_properties_supported.len()
    );
    println!(
        "DeviceInfo: {} operation(s), {} event(s), {} capture format(s), {} image format(s).",
        info.operations_supported.len(),
        info.events_supported.len(),
        info.capture_formats.len(),
        info.image_formats.len()
    );

    let mut candidates = BTreeMap::<u16, String>::new();
    for code in info.device_properties_supported {
        candidates.insert(code, format!("DeviceInfo advertised property {code:04X}"));
    }
    for (code, label) in xm5_readonly_probe_candidates() {
        candidates
            .entry(code)
            .and_modify(|existing| existing.push_str(&format!("; {label}")))
            .or_insert_with(|| label.to_string());
    }

    let total = candidates.len();
    let mut readable = 0_usize;
    let mut unavailable = 0_usize;
    println!("Scanning {total} candidate properties with standard GetDevicePropValue…");
    for (code, label) in candidates {
        match usb_transport::probe_ptp_property_value(id, code) {
            Ok(probe) => {
                readable += 1;
                println!("  READ {code:04X} {label}: {}", format_hex(&probe.value));
            }
            Err(error) => {
                unavailable += 1;
                println!("  NO   {code:04X} {label}: {error}");
            }
        }
    }
    println!(
        "Read-only scan complete: {readable}/{total} properties returned a value; {unavailable} were unavailable or rejected. No settings were changed."
    );
}

/// Baseline discovery for every Fujifilm body. It sends GetDeviceInfo followed
/// only by GetDevicePropValue for property codes advertised by the camera.
/// There is no custom-slot selection, SetDevicePropValue, or proprietary write
/// in this workflow. Use it before adding a new model or firmware record.
fn run_fujifilm_readonly_property_scan(id: camera_core::UsbId) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) => info,
        Err(error) => {
            eprintln!("Cannot begin Fujifilm read-only scan: {error}");
            return;
        }
    };
    let capability =
        camera_fujifilm::resolve_capability_record(id, &info.model, &info.device_version);
    println!(
        "Read-only Fujifilm scan: {} {} firmware {} on {}; {}.",
        info.manufacturer,
        info.model,
        info.device_version,
        id,
        capability.state.label()
    );
    if let Some(record_id) = capability.record_id {
        println!("  capability record: {record_id}");
    }
    println!("  next action: {}", capability.state.next_action());
    println!(
        "  DeviceInfo advertises {} device property code(s), {} operation(s), {} event(s), {} capture format(s), and {} image format(s).",
        info.device_properties_supported.len(),
        info.operations_supported.len(),
        info.events_supported.len(),
        info.capture_formats.len(),
        info.image_formats.len()
    );
    let total = info.device_properties_supported.len();
    let mut readable = 0_usize;
    let mut unavailable = 0_usize;
    for code in info.device_properties_supported {
        match usb_transport::probe_ptp_property_value(id, code) {
            Ok(probe) => {
                readable += 1;
                println!("  READ {code:04X}: {}", format_hex(&probe.value));
            }
            Err(error) => {
                unavailable += 1;
                println!("  NO   {code:04X}: {error}");
            }
        }
    }
    println!(
        "Read-only Fujifilm scan complete: {readable}/{total} values returned; {unavailable} unavailable or rejected. No settings were changed."
    );
}

fn xm5_readonly_probe_candidates() -> Vec<(u16, &'static str)> {
    let mut candidates = vec![
        (0x5001, "Battery Level / 電池電量"),
        (0x5003, "Image Size / 影像尺寸"),
        (0x5004, "Compression / 壓縮"),
        (0x5005, "White Balance / 白平衡"),
        (0x500A, "Focus Mode / 對焦模式"),
        (0x500B, "Exposure Metering / 測光"),
        (0x500D, "Exposure Time / 快門速度"),
        (0x500E, "Exposure Program / 曝光模式"),
        (0x500F, "Exposure Index / ISO"),
        (0x5010, "Exposure Bias / 曝光補償"),
        (0x5013, "Still Capture Mode / 靜態拍攝模式"),
        (0x5014, "Contrast / 對比"),
        (0x5015, "Sharpness / 銳利度"),
    ];
    candidates.extend([
        (0xD001, "Film Simulation / 底片模擬 (global candidate)"),
        (0xD002, "Film Simulation Tune / 底片模擬微調"),
        (0xD003, "D Range Mode / 動態範圍模式"),
        (0xD007, "Color Temperature / 色溫"),
        (0xD008, "White Balance Fine Tune / 白平衡微調"),
        (0xD00A, "Noise Reduction / 雜訊抑制"),
        (0xD00B, "Image Quality / 影像品質"),
        (0xD00C, "Recording Mode / 記錄模式"),
        (0xD00F, "Focus Mode / 對焦模式"),
        (0xD017, "Grain Effect / 顆粒效果 (global candidate)"),
        (0xD019, "Shadow/Highlight / 陰影高光"),
        (0xD100, "Exposure Index / ISO"),
        (0xD104, "Focus Metering Mode / 對焦區域"),
        (0xD10A, "Shutter Speed / 快門速度"),
        (0xD10B, "Image Aspect Ratio / 長寬比"),
        (0xD171, "RAW Conversion Edit / RAW 轉換編輯"),
        (0xD183, "Start RAW Conversion / 開始 RAW 轉換"),
        (0xD184, "IOP Codes / IOP 代碼"),
        (0xD185, "RAW Conversion Profile / RAW 轉換設定檔"),
        (0xD186, "Firmware Version / 韌體版本"),
        (0xD187, "Firmware Version 2 / 韌體版本 2"),
        (0xD18C, "Custom Slot / 自定義槽位"),
        (0xD18D, "Custom Preset Name / 自定義檔名稱"),
    ]);
    for code in 0xD18E..=0xD1A5 {
        candidates.push((code, xm5_preset_property_label(code)));
    }
    candidates
}

/// Runs only standard GetDeviceInfo and GetDevicePropDesc requests. Descriptor
/// metadata is useful evidence, but a writable flag alone never enables an
/// application write: vendor encodings must still be proven with recovery.
fn run_xm5_descriptor_scan(id: camera_core::UsbId) {
    use std::collections::BTreeMap;

    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) => info,
        Err(error) => {
            eprintln!("Cannot begin descriptor scan: {error}");
            return;
        }
    };
    let labels = xm5_readonly_probe_candidates()
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let total = info.device_properties_supported.len();
    let mut described = 0_usize;
    let mut writable = 0_usize;
    let mut unavailable = 0_usize;
    println!(
        "Read-only X-M5 descriptor scan: {total} DeviceInfo-advertised properties; no settings will be changed."
    );
    for code in info.device_properties_supported {
        let label = labels
            .get(&code)
            .copied()
            .unwrap_or("DeviceInfo advertised property");
        match usb_transport::probe_ptp_property_descriptor(id, code) {
            Ok(probe) => {
                described += 1;
                writable += usize::from(probe.writable);
                println!(
                    "  DESC {code:04X} {label}: type {:04X}; {}; default {:?}; current {:?}",
                    probe.data_type,
                    if probe.writable {
                        "writable"
                    } else {
                        "read-only"
                    },
                    probe.factory_default,
                    probe.current_value,
                );
            }
            Err(error) => {
                unavailable += 1;
                println!("  NO   {code:04X} {label}: {error}");
            }
        }
    }
    println!(
        "Descriptor scan complete: {described}/{total} descriptors returned; {writable} marked writable; {unavailable} unavailable or rejected. No settings were changed."
    );
}

fn xm5_preset_property_label(code: u16) -> &'static str {
    match code {
        0xD18E => "Preset Image Size / 自定義檔影像尺寸",
        0xD18F => "Preset Image Quality / 自定義檔影像品質",
        0xD190 => "Preset Dynamic Range / 自定義檔動態範圍",
        0xD191 => "Preset Reserved / 自定義檔保留欄位",
        0xD192 => "Preset Film Simulation / 自定義檔底片模擬",
        0xD193 => "Preset Monochrome Warm/Cool / 黑白暖冷色",
        0xD194 => "Preset Monochrome Magenta/Green / 黑白洋紅綠色",
        0xD195 => "Preset Grain Effect / 自定義檔顆粒效果",
        0xD196 => "Preset Color Chrome / 自定義檔色彩效果",
        0xD197 => "Preset Color Chrome FX Blue / 自定義檔藍色彩色效果",
        0xD198 => "Preset Smooth Skin / 自定義檔平滑膚色效果",
        0xD199 => "Preset White Balance / 自定義檔白平衡",
        0xD19A => "Preset WB Shift R / 自定義檔白平衡偏移 R",
        0xD19B => "Preset WB Shift B / 自定義檔白平衡偏移 B",
        0xD19C => "Preset Color Temperature / 自定義檔色溫",
        0xD19D => "Preset Highlight Tone / 自定義檔高光色調",
        0xD19E => "Preset Shadow Tone / 自定義檔陰影色調",
        0xD19F => "Preset Color / 自定義檔色彩",
        0xD1A0 => "Preset Sharpness / 自定義檔銳利度",
        0xD1A1 => "Preset High ISO NR / 自定義檔高 ISO 降噪",
        0xD1A2 => "Preset Clarity / 自定義檔清晰度",
        0xD1A3 => "Preset Long Exposure NR / 自定義檔長時間曝光降噪",
        0xD1A4 => "Preset Color Space / 自定義檔色彩空間",
        0xD1A5 => "Preset Reserved / 自定義檔保留欄位",
        _ => "Preset Vendor Property / 自定義檔 vendor 欄位",
    }
}

fn parse_property_code(value: &str) -> Option<u16> {
    let normalized = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or(value.trim());
    u16::from_str_radix(normalized, 16).ok()
}

/// Parses a declared, human-selected camera value for the controlled image
/// payload workflow. This parser never derives a write payload: it only labels
/// the bytes which the camera reports after the user changes that setting in
/// the camera menu.
fn parse_xm5_image_payload_calibration(
    slot: u16,
    kind: &str,
    setting: &str,
) -> Option<ImagePayloadCalibration> {
    const IMAGE_SIZES: &[&str] = &[
        "L_3_2",
        "L_16_9",
        "L_1_1",
        "M_3_2",
        "M_16_9",
        "M_1_1",
        "S_3_2",
        "S_16_9",
        "S_1_1",
        "M_3_2_1_25X_CROP",
        "M_16_9_1_25X_CROP",
        "M_1_1_1_25X_CROP",
    ];
    const IMAGE_QUALITIES: &[&str] = &["FINE", "NORMAL", "FINE_PLUS_RAW", "NORMAL_PLUS_RAW", "RAW"];
    match kind {
        "image-size" if IMAGE_SIZES.contains(&setting) => Some(ImagePayloadCalibration {
            slot,
            property: 0xD18E,
            kind: "image-size",
            setting: IMAGE_SIZES
                .iter()
                .copied()
                .find(|value| *value == setting)?,
        }),
        "image-quality" if IMAGE_QUALITIES.contains(&setting) => Some(ImagePayloadCalibration {
            slot,
            property: 0xD18F,
            kind: "image-quality",
            setting: IMAGE_QUALITIES
                .iter()
                .copied()
                .find(|value| *value == setting)?,
        }),
        _ => None,
    }
}

/// Capture an image-size or image-quality candidate after the user manually
/// selects that exact value on the camera. It may select the requested C slot
/// to observe the slot-local value, but it sends no `SetDevicePropValue` for
/// the candidate and verifies restoration of the previously active C slot.
fn run_xm5_image_payload_capture(id: camera_core::UsbId, request: ImagePayloadCalibration) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) if info.device_version == "1.30" => info,
        Ok(info) => {
            eprintln!(
                "Refusing calibration capture: firmware {} is not the validated 1.30 target.",
                info.device_version
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot start calibration capture: {error}");
            return;
        }
    };
    let original_slot = match read_u16_property(id, 0xD18C) {
        Ok(slot) if (1..=4).contains(&slot) => slot,
        Ok(slot) => {
            eprintln!("Cannot safely preserve active custom slot D18C={slot}.");
            return;
        }
        Err(error) => {
            eprintln!("Cannot preserve active custom slot: {error}");
            return;
        }
    };
    if let Err(error) = select_and_verify_slot(id, request.slot) {
        eprintln!("Cannot select calibration slot C{}: {error}", request.slot);
        return;
    }
    println!(
        "Capturing manual X-M5 {} candidate {} from C{}; no image-setting write will be sent.",
        request.kind, request.setting, request.slot
    );
    let result = usb_transport::probe_ptp_property_value(id, request.property);
    let active_slot_restored = restore_active_slot(id, original_slot);
    match result {
        Ok(probe) => println!(
            "  candidate: model={} firmware={} slot=C{} setting={} property={:04X} raw={} status=read_only_unverified; active-slot restoration {}.",
            info.model,
            info.device_version,
            request.slot,
            request.setting,
            probe.property_code,
            format_hex(&probe.value),
            if active_slot_restored { "verified" } else { "FAILED" },
        ),
        Err(error) => eprintln!(
            "  could not read candidate property {:04X}: {error}; active-slot restoration {}.",
            request.property,
            if active_slot_restored { "verified" } else { "FAILED" },
        ),
    }
}

/// Reversible validation for one image-payload candidate observed by
/// `ptp-capture-xm5-image-payload`. The raw value must be supplied explicitly
/// from that preceding read-only capture; the test preserves the target C slot
/// value and restores the previously active C slot before reporting success.
fn run_xm5_image_payload_verification(
    id: camera_core::UsbId,
    request: ImagePayloadCalibration,
    observed_raw_value: u16,
) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) if info.device_version == "1.30" => info,
        Ok(info) => {
            eprintln!(
                "Refusing image payload write verification: firmware {} is not the validated 1.30 target.",
                info.device_version
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot start image payload write verification: {error}");
            return;
        }
    };
    let original_slot = match read_u16_property(id, 0xD18C) {
        Ok(slot) if (1..=4).contains(&slot) => slot,
        Ok(slot) => {
            eprintln!("Cannot safely preserve active custom slot D18C={slot}.");
            return;
        }
        Err(error) => {
            eprintln!("Cannot preserve active custom slot: {error}");
            return;
        }
    };
    if let Err(error) = select_and_verify_slot(id, request.slot) {
        eprintln!("Cannot select test slot C{}: {error}", request.slot);
        return;
    }
    let case = VerificationCase::exact(
        format!(
            "Image payload / {}: {} (observed {:04X})",
            request.kind, request.setting, observed_raw_value
        ),
        request.property,
        observed_raw_value,
    );
    println!(
        "Starting reversible X-M5 image payload verification on {} {} firmware {} C{}; it will restore the captured field and active C{}.",
        info.manufacturer, info.model, info.device_version, request.slot, original_slot
    );
    let result = run_reversible_case(id, &case);
    let active_slot_restored = restore_active_slot(id, original_slot);
    match result {
        Ok(()) if active_slot_restored => println!(
            "  PASS {} — write/read-back/restore and active-slot recovery verified.",
            case.label
        ),
        Ok(()) => eprintln!(
            "  FAIL {} — target field restored, but active-slot restoration FAILED.",
            case.label
        ),
        Err(error) => eprintln!(
            "  FAIL {} — {error}; active-slot restoration {}.",
            case.label,
            if active_slot_restored {
                "verified"
            } else {
                "FAILED"
            },
        ),
    }
}

/// Validates only documented/reference-backed enum candidates which have not
/// yet passed an X-M5-specific reversible record. It does not test custom
/// white balance slots, image payloads, global controls, or unknown vendor
/// properties because their value mapping or recovery boundary is incomplete.
fn run_xm5_additional_enum_verification(id: camera_core::UsbId, slot: u16) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) if info.device_version == "1.30" => info,
        Ok(info) => {
            eprintln!(
                "Refusing additional enum verification: firmware {} is not the validated 1.30 target.",
                info.device_version
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot start additional enum verification: {error}");
            return;
        }
    };
    let original_slot = match read_u16_property(id, 0xD18C) {
        Ok(slot) if (1..=4).contains(&slot) => slot,
        Ok(slot) => {
            eprintln!("Cannot safely preserve active custom slot D18C={slot}.");
            return;
        }
        Err(error) => {
            eprintln!("Cannot preserve active custom slot: {error}");
            return;
        }
    };
    if let Err(error) = select_and_verify_slot(id, slot) {
        eprintln!("Cannot select test slot C{slot}: {error}");
        return;
    }
    let cases = xm5_additional_enum_verification_cases();
    println!(
        "Starting {} additional reversible X-M5 enum checks on {} {} firmware {} C{}; active C{} will be restored afterwards.",
        cases.len(), info.manufacturer, info.model, info.device_version, slot, original_slot
    );
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    for (index, case) in cases.iter().enumerate() {
        match run_reversible_case(id, case) {
            Ok(()) => {
                passed += 1;
                println!("  [{}/{}] PASS {}", index + 1, cases.len(), case.label);
            }
            Err(error) => {
                failed += 1;
                println!(
                    "  [{}/{}] FAIL {} — {error}",
                    index + 1,
                    cases.len(),
                    case.label
                );
            }
        }
    }
    let active_slot_restored = restore_active_slot(id, original_slot);
    println!(
        "Additional enum verification complete: {passed}/{} passed; {failed} failed; active-slot restoration {}.",
        cases.len(),
        if active_slot_restored { "verified" } else { "FAILED" },
    );
}

fn xm5_additional_enum_verification_cases() -> Vec<VerificationCase> {
    let mut cases = vec![VerificationCase::exact(
        "Dynamic Range / 動態範圍: AUTO",
        0xD190,
        0xFFFF,
    )];
    for (label, value) in [
        ("PRO Neg. Hi", 4),
        ("PRO Neg. Std", 5),
        ("MONOCHROME", 6),
        ("MONOCHROME+Ye FILTER", 7),
        ("MONOCHROME+R FILTER", 8),
        ("MONOCHROME+G FILTER", 9),
        ("SEPIA", 10),
        ("ACROS+Ye FILTER", 13),
        ("ACROS+R FILTER", 14),
        ("ACROS+G FILTER", 15),
        ("ETERNA BLEACH BYPASS", 18),
        ("NOSTALGIC Neg.", 19),
    ] {
        cases.push(VerificationCase::exact(
            format!("Film Simulation / 軟片模擬: {label}"),
            0xD192,
            value,
        ));
    }
    for (label, value) in [
        ("White Priority / 白色優先", 0x8020),
        ("Ambience Priority / 氛圍優先", 0x8021),
    ] {
        cases.push(VerificationCase::exact(
            format!("White Balance / 白平衡: {label}"),
            0xD199,
            value,
        ));
    }
    cases
}

/// Tests every X-M5 recipe field that the application is currently allowed to
/// write.  Each case captures the selected C slot's current value, writes a
/// supported test value, verifies the camera's response, and then verifies its
/// captured value was restored.  Enumerated controls test every supported
/// value; numeric controls test their minimum, neutral, and maximum values.
fn run_xm5_recipe_verification(id: camera_core::UsbId, slot: u16) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) => info,
        Err(error) => {
            eprintln!("Cannot start write verification: PTP DeviceInfo failed: {error}");
            return;
        }
    };
    if info.device_version != "1.30" {
        eprintln!(
            "Refusing write verification: X-M5 firmware {} is not the validated 1.30 target.",
            info.device_version
        );
        return;
    }

    let original_slot = match usb_transport::probe_ptp_property_value(id, 0xD18C) {
        Ok(probe) if probe.value.len() == 2 => u16::from_le_bytes([probe.value[0], probe.value[1]]),
        Ok(probe) => {
            eprintln!(
                "Cannot preserve the active custom slot: D18C returned {}. No write test was run.",
                format_hex(&probe.value)
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot preserve the active custom slot: {error}. No write test was run.");
            return;
        }
    };
    if !(1..=4).contains(&original_slot) {
        eprintln!(
            "Cannot safely restore active slot D18C={original_slot}; select C1-C4 on the camera first. No write test was run."
        );
        return;
    }

    println!(
        "Starting reversible X-M5 recipe verification on C{slot}; active C{original_slot} will be restored afterwards."
    );
    let selection_result = usb_transport::select_custom_slot(id, slot).and_then(|_| {
        usb_transport::probe_ptp_property_value(id, 0xD18C).and_then(|probe| {
            if probe.value == slot.to_le_bytes() {
                Ok(probe)
            } else {
                Err(usb_transport::TransportError::PtpProbe(format!(
                    "C{slot} selection read back as {}",
                    format_hex(&probe.value)
                )))
            }
        })
    });
    if let Err(error) = selection_result {
        eprintln!("Cannot select test slot C{slot}: {error}. No recipe property was written.");
        return;
    }

    let mut cases = xm5_recipe_verification_cases();
    let test_name = match ptp_core::encode_string("FRM-VERIFY") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Could not encode test preset name: {error}");
            restore_active_slot(id, original_slot);
            return;
        }
    };
    cases.insert(
        0,
        VerificationCase {
            label: "Preset name / 自定義名稱: FRM-VERIFY".into(),
            property: 0xD18D,
            write_value: test_name.clone(),
            accepted_read_values: vec![test_name],
        },
    );

    let total = cases.len();
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    for (index, case) in cases.iter().enumerate() {
        match run_reversible_case(id, case) {
            Ok(()) => {
                passed += 1;
                println!("  [{}/{}] PASS {}", index + 1, total, case.label);
            }
            Err(error) => {
                failed += 1;
                println!("  [{}/{}] FAIL {} — {error}", index + 1, total, case.label);
            }
        }
    }

    let active_slot_restored = restore_active_slot(id, original_slot);
    println!(
        "Verification complete: {passed}/{total} cases passed; {failed} failed; active slot restoration {}.",
        if active_slot_restored { "verified" } else { "FAILED" }
    );
    if failed > 0 || !active_slot_restored {
        eprintln!("The test slot's captured values were restored after each case; review FAIL lines before enabling that field for production writes.");
    }
}

/// Verifies the additional, identified C-slot properties found during the
/// read-only X-M5 scan. It deliberately excludes unknown/global properties
/// and operation triggers such as RAW conversion.
fn run_xm5_extended_verification(id: camera_core::UsbId, slot: u16) {
    let info = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) if info.device_version == "1.30" => info,
        Ok(info) => {
            eprintln!(
                "Refusing extended write verification: firmware {} is not the validated 1.30 target.",
                info.device_version
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot start extended write verification: {error}");
            return;
        }
    };
    let original_slot = match read_u16_property(id, 0xD18C) {
        Ok(slot) if (1..=4).contains(&slot) => slot,
        Ok(slot) => {
            eprintln!("Cannot safely preserve active custom slot D18C={slot}.");
            return;
        }
        Err(error) => {
            eprintln!("Cannot preserve active custom slot: {error}");
            return;
        }
    };
    if let Err(error) = select_and_verify_slot(id, slot) {
        eprintln!("Cannot select test slot C{slot}: {error}");
        return;
    }

    println!(
        "Starting reversible extended X-M5 C-slot verification on C{slot}; active C{original_slot} will be restored afterwards."
    );
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    let mut report = |label: &str, result: Result<(), String>| match result {
        Ok(()) => {
            passed += 1;
            println!("  PASS {label}");
        }
        Err(error) => {
            failed += 1;
            println!("  FAIL {label} — {error}");
        }
    };

    for (code, label) in [
        (0xD18E, "Image Size / 影像尺寸 (captured value)"),
        (0xD18F, "Image Quality / 影像品質 (captured value)"),
    ] {
        let case = read_captured_value_case(id, code, label);
        report(label, case.and_then(|case| run_reversible_case(id, &case)));
    }
    for (label, value) in [("OFF", 1), ("WEAK", 2), ("STRONG", 3)] {
        let case = VerificationCase::exact(
            format!("Smooth Skin Effect / 平滑膚色效果: {label}"),
            0xD198,
            value,
        );
        report(&case.label, run_reversible_case(id, &case));
    }
    for (label, value) in [("OFF", 0), ("ON", 1)] {
        let case = VerificationCase::exact(
            format!("Long Exposure NR / 長時間曝光降噪: {label}"),
            0xD1A3,
            value,
        );
        report(&case.label, run_reversible_case(id, &case));
    }
    for (label, value) in [("sRGB", 1), ("Adobe RGB", 2)] {
        let case =
            VerificationCase::exact(format!("Color Space / 色彩空間: {label}"), 0xD1A4, value);
        report(&case.label, run_reversible_case(id, &case));
    }
    report(
        "Color Temperature / 色溫: 2500K, 6500K, 10000K",
        verify_color_temperature_property(id),
    );
    report(
        "Monochrome Warm/Cool and Magenta/Green / 黑白暖冷與洋紅綠",
        verify_monochrome_color_properties(id),
    );

    let active_slot_restored = restore_active_slot(id, original_slot);
    println!(
        "Extended verification complete for {} {}: {passed} passed; {failed} failed; active slot restoration {}.",
        info.model,
        info.device_version,
        if active_slot_restored { "verified" } else { "FAILED" }
    );
}

/// Tests whether documented, live-camera controls accept an exact write of
/// their already-observed value. This detects transport support without asking
/// the camera to transition to another shooting state. Unknown, status-only,
/// firmware, and action/RAW-conversion properties are intentionally excluded.
fn run_xm5_global_current_value_verification(id: camera_core::UsbId) {
    let firmware = match usb_transport::probe_ptp_device_info(id) {
        Ok(info) if info.device_version == "1.30" => info.device_version,
        Ok(info) => {
            eprintln!(
                "Refusing global current-value verification: firmware {} is not the validated 1.30 target.",
                info.device_version
            );
            return;
        }
        Err(error) => {
            eprintln!("Cannot start global current-value verification: {error}");
            return;
        }
    };
    let candidates = [
        (0x5005, "White Balance / 白平衡"),
        (0x5015, "Sharpness / 銳利度"),
        (0xD001, "Film Simulation / 底片模擬"),
        (0xD007, "Color Temperature / 色溫"),
        (0xD008, "White Balance Fine Tune / 白平衡微調"),
        (0xD00A, "Noise Reduction / 雜訊抑制"),
        (0xD00B, "Image Quality / 影像品質"),
        (0xD00C, "Recording Mode / 記錄模式"),
        (0xD017, "Grain Effect / 顆粒效果"),
        (0xD104, "Focus Metering Mode / 對焦區域"),
    ];
    println!(
        "Starting current-value-only global control verification on X-M5 firmware {firmware}; no test changes a value."
    );
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    for (property, label) in candidates {
        let result = read_property(id, property).and_then(|current| {
            write_and_expect(id, property, &current)
                .and_then(|_| verify_property(id, property, &current))
        });
        match result {
            Ok(()) => {
                passed += 1;
                println!("  PASS {property:04X} {label}: current value accepted and unchanged");
            }
            Err(error) => {
                failed += 1;
                println!("  FAIL {property:04X} {label}: {error}");
            }
        }
    }
    println!(
        "Global current-value verification complete: {passed} passed; {failed} rejected. Unknown/action properties were not written."
    );
}

fn read_captured_value_case(
    id: camera_core::UsbId,
    property: u16,
    label: &str,
) -> Result<VerificationCase, String> {
    let value = usb_transport::probe_ptp_property_value(id, property)
        .map_err(|error| format!("could not capture current value: {error}"))?
        .value;
    Ok(VerificationCase {
        label: label.to_string(),
        property,
        write_value: value.clone(),
        accepted_read_values: vec![value],
    })
}

fn verify_color_temperature_property(id: camera_core::UsbId) -> Result<(), String> {
    const WHITE_BALANCE: u16 = 0xD199;
    const COLOR_TEMPERATURE: u16 = 0xD19C;
    const COLOR_TEMPERATURE_MODE: u16 = 0x8007;
    let original_wb = read_property(id, WHITE_BALANCE)?;
    let original_temperature = read_property(id, COLOR_TEMPERATURE)?;
    let mut target_error =
        write_and_expect(id, WHITE_BALANCE, &COLOR_TEMPERATURE_MODE.to_le_bytes()).err();
    if target_error.is_none() {
        for temperature in [2500_u16, 6500, 10_000] {
            if let Err(error) = write_and_expect(id, COLOR_TEMPERATURE, &temperature.to_le_bytes())
            {
                target_error = Some(format!("{temperature}K: {error}"));
                break;
            }
        }
    }
    let restore_result = write_and_expect(id, COLOR_TEMPERATURE, &original_temperature)
        .and_then(|_| write_and_expect(id, WHITE_BALANCE, &original_wb))
        .and_then(|_| verify_property(id, WHITE_BALANCE, &original_wb))
        .and_then(|_| verify_property(id, COLOR_TEMPERATURE, &original_temperature));
    combine_target_and_restore(target_error, restore_result)
}

fn verify_monochrome_color_properties(id: camera_core::UsbId) -> Result<(), String> {
    const FILM_SIMULATION: u16 = 0xD192;
    const MONO_WARM_COOL: u16 = 0xD193;
    const MONO_MAGENTA_GREEN: u16 = 0xD194;
    const ACROS: u16 = 12;
    let original_film = read_property(id, FILM_SIMULATION)?;
    let original_warm_cool = read_property(id, MONO_WARM_COOL)?;
    let original_magenta_green = read_property(id, MONO_MAGENTA_GREEN)?;
    let mut target_errors = Vec::new();
    if let Err(error) = write_and_expect(id, FILM_SIMULATION, &ACROS.to_le_bytes()) {
        target_errors.push(format!("select ACROS: {error}"));
    } else {
        for (code, title) in [
            (MONO_WARM_COOL, "warm/cool"),
            (MONO_MAGENTA_GREEN, "magenta/green"),
        ] {
            // Test direct and ×10 interpretations at both ends. The X-M5
            // firmware rejected the borrowed ×10 mapping during validation,
            // so results are retained rather than assumed.
            for value in [1_i16, 10, 90, -1, -10, -90] {
                if let Err(error) = write_and_expect(id, code, &value.to_le_bytes()) {
                    target_errors.push(format!("{title} {value}: {error}"));
                }
            }
        }
    }
    // A non-monochrome original film makes the two adjustments inactive and
    // reads them back as their captured zero value, which is the only safe way
    // to restore a zero because the camera rejects writing zero directly.
    let restore_result = write_and_expect(id, FILM_SIMULATION, &original_film)
        .and_then(|_| verify_property(id, FILM_SIMULATION, &original_film))
        .and_then(|_| verify_property(id, MONO_WARM_COOL, &original_warm_cool))
        .and_then(|_| verify_property(id, MONO_MAGENTA_GREEN, &original_magenta_green));
    combine_target_and_restore(
        (!target_errors.is_empty()).then(|| target_errors.join("; ")),
        restore_result,
    )
}

fn read_property(id: camera_core::UsbId, property: u16) -> Result<Vec<u8>, String> {
    usb_transport::probe_ptp_property_value(id, property)
        .map(|probe| probe.value)
        .map_err(|error| error.to_string())
}

fn read_u16_property(id: camera_core::UsbId, property: u16) -> Result<u16, String> {
    let value = read_property(id, property)?;
    let value: [u8; 2] = value.as_slice().try_into().map_err(|_| {
        format!(
            "{property:04X} returned {} byte(s), expected 2",
            value.len()
        )
    })?;
    Ok(u16::from_le_bytes(value))
}

fn select_and_verify_slot(id: camera_core::UsbId, slot: u16) -> Result<(), String> {
    usb_transport::select_custom_slot(id, slot).map_err(|error| error.to_string())?;
    let observed = read_u16_property(id, 0xD18C)?;
    (observed == slot)
        .then_some(())
        .ok_or_else(|| format!("C{slot} selection read back as C{observed}"))
}

fn write_and_expect(id: camera_core::UsbId, property: u16, expected: &[u8]) -> Result<(), String> {
    usb_transport::set_ptp_property_value(id, property, expected)
        .map_err(|error| error.to_string())?;
    verify_property(id, property, expected)
}

fn verify_property(id: camera_core::UsbId, property: u16, expected: &[u8]) -> Result<(), String> {
    let observed = read_property(id, property)?;
    (observed == expected).then_some(()).ok_or_else(|| {
        format!(
            "{property:04X} read back {} instead of {}",
            format_hex(&observed),
            format_hex(expected)
        )
    })
}

fn combine_target_and_restore(
    target_error: Option<String>,
    restore_result: Result<(), String>,
) -> Result<(), String> {
    match (target_error, restore_result) {
        (None, Ok(())) => Ok(()),
        (Some(target), Ok(())) => Err(format!("{target}; captured values restored")),
        (None, Err(restore)) => Err(format!("target accepted but restore failed: {restore}")),
        (Some(target), Err(restore)) => Err(format!("{target}; restore failed: {restore}")),
    }
}

fn run_reversible_case(id: camera_core::UsbId, case: &VerificationCase) -> Result<(), String> {
    let original = usb_transport::probe_ptp_property_value(id, case.property)
        .map_err(|error| format!("could not capture original value: {error}"))?
        .value;

    let target_result = usb_transport::set_ptp_property_value(id, case.property, &case.write_value)
        .and_then(|_| usb_transport::probe_ptp_property_value(id, case.property));
    let target_result = match target_result {
        Ok(probe)
            if case
                .accepted_read_values
                .iter()
                .any(|value| value == &probe.value) =>
        {
            Ok(())
        }
        Ok(probe) => Err(format!(
            "read-back {} was not one of the accepted values",
            format_hex(&probe.value)
        )),
        Err(error) => Err(format!("write/read-back failed: {error}")),
    };

    // A rejected SetDevicePropValue should not trigger a second, identical
    // write merely to "restore" the captured value. First read the property:
    // if it is still captured, preserve that evidence and avoid a redundant
    // command that would only create a misleading restore failure.
    let observed_after_target = read_property(id, case.property);
    let restore_needed = !matches!(
        observed_after_target.as_deref(),
        Ok(value) if value == original.as_slice()
    );
    let restore_result = if restore_needed {
        let mut result = Ok(());
        for restore in camera_xm5::restore_property_steps(case.property, &original) {
            result =
                match usb_transport::set_ptp_property_value(id, restore.code, &restore.write_value)
                    .and_then(|_| usb_transport::probe_ptp_property_value(id, restore.code))
                {
                    Ok(probe) if restore.matches_read_back(&probe.value) => Ok(()),
                    Ok(probe) => Err(format!(
                        "restore read-back was {} instead of {}",
                        format_hex(&probe.value),
                        format_hex(&original)
                    )),
                    Err(error) => Err(format!("restore failed: {error}")),
                };
            if result.is_err() {
                break;
            }
        }
        result
    } else {
        Ok(())
    };

    match (target_result, restore_result, restore_needed) {
        (Ok(()), Ok(()), false) => Ok(()),
        (Ok(()), Ok(()), true) => Ok(()),
        (Err(target), Ok(()), false) => Err(format!(
            "{target}; captured value confirmed unchanged after rejection"
        )),
        (Err(target), Ok(()), true) => Err(format!("{target}; original value restored")),
        (Ok(()), Err(restore), _) => Err(format!("target accepted but {restore}")),
        (Err(target), Err(restore), _) => Err(format!("{target}; additionally {restore}")),
    }
}

fn restore_active_slot(id: camera_core::UsbId, slot: u16) -> bool {
    match usb_transport::select_custom_slot(id, slot)
        .and_then(|_| usb_transport::probe_ptp_property_value(id, 0xD18C))
    {
        Ok(probe) if probe.value == slot.to_le_bytes() => true,
        Ok(probe) => {
            eprintln!(
                "Could not restore active C{slot}: D18C read back as {}",
                format_hex(&probe.value)
            );
            false
        }
        Err(error) => {
            eprintln!("Could not restore active C{slot}: {error}");
            false
        }
    }
}

fn xm5_recipe_verification_cases() -> Vec<VerificationCase> {
    let mut cases = Vec::new();
    for (label, value) in [("DR100", 100), ("DR200", 200), ("DR400", 400)] {
        cases.push(VerificationCase::exact(
            format!("Dynamic Range / 動態範圍: {label}"),
            0xD190,
            value,
        ));
    }
    for (label, value) in [
        ("PROVIA / 標準", 1),
        ("VELVIA / 鮮艷", 2),
        ("ASTIA / 柔和", 3),
        ("CLASSIC CHROME / 經典正片", 11),
        ("ACROS / 黑白", 12),
        ("ETERNA / 電影", 16),
        ("CLASSIC NEGATIVE / 經典負片", 17),
        ("REALA ACE / REALA ACE", 20),
    ] {
        cases.push(VerificationCase::exact(
            format!("Film Simulation / 底片模擬: {label}"),
            0xD192,
            value,
        ));
    }
    cases.push(VerificationCase {
        label: "Grain Effect / 顆粒效果: OFF".into(),
        property: 0xD195,
        write_value: 1_u16.to_le_bytes().to_vec(),
        accepted_read_values: vec![1_u16.to_le_bytes().to_vec(), 6_u16.to_le_bytes().to_vec()],
    });
    for (label, value) in [
        ("WEAK SMALL", 2),
        ("STRONG SMALL", 3),
        ("WEAK LARGE", 4),
        ("STRONG LARGE", 5),
    ] {
        cases.push(VerificationCase::exact(
            format!("Grain Effect / 顆粒效果: {label}"),
            0xD195,
            value,
        ));
    }
    for (property, title) in [
        (0xD196, "Color Chrome Effect / 色彩效果"),
        (0xD197, "Color Chrome FX Blue / 藍色彩色效果"),
    ] {
        for (label, value) in [("OFF", 1), ("WEAK", 2), ("STRONG", 3)] {
            cases.push(VerificationCase::exact(
                format!("{title}: {label}"),
                property,
                value,
            ));
        }
    }
    for (label, value) in [
        ("AUTO / 自動", 2),
        ("DAYLIGHT / 晴天", 4),
        ("INCANDESCENT / 白熾燈", 6),
        ("UNDERWATER / 水下", 8),
        ("FLUORESCENT 1 / 日光燈 1", 0x8001),
        ("FLUORESCENT 2 / 日光燈 2", 0x8002),
        ("FLUORESCENT 3 / 日光燈 3", 0x8003),
        ("SHADE / 陰天", 0x8006),
    ] {
        cases.push(VerificationCase::exact(
            format!("White Balance / 白平衡: {label}"),
            0xD199,
            value,
        ));
    }
    for (property, title, values) in [
        (
            0xD19A,
            "White Balance Shift R / 白平衡偏移 R",
            vec![-9, 0, 9],
        ),
        (
            0xD19B,
            "White Balance Shift B / 白平衡偏移 B",
            vec![-9, 0, 9],
        ),
        (0xD19D, "Highlight Tone / 高光色調", vec![-20, 0, 40]),
        (0xD19E, "Shadow Tone / 陰影色調", vec![-20, 0, 40]),
        (0xD19F, "Color / 色彩", vec![-40, 0, 40]),
        (0xD1A0, "Sharpness / 銳利度", vec![-40, 0, 40]),
        (0xD1A2, "Clarity / 清晰度", vec![-50, 0, 50]),
    ] {
        for value in values {
            cases.push(VerificationCase::signed(
                format!("{title}: {value}"),
                property,
                value,
            ));
        }
    }
    for (label, value) in [
        ("-4", 0x8000),
        ("-3", 0x7000),
        ("-2", 0x4000),
        ("-1", 0x3000),
        ("0", 0x2000),
        ("+1", 0x1000),
        ("+2", 0),
        ("+3", 0x6000),
        ("+4", 0x5000),
    ] {
        cases.push(VerificationCase::exact(
            format!("High ISO NR / 高 ISO 降噪: {label}"),
            0xD1A1,
            value,
        ));
    }
    cases
}

fn parse_u16_value(value: &str) -> Option<u16> {
    let normalized = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"));
    match normalized {
        Some(hex) => u16::from_str_radix(hex, 16).ok(),
        None => value.trim().parse::<u16>().ok(),
    }
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
