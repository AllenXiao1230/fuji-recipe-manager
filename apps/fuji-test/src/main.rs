//! Read-only Phase 0 USB discovery tool.

use usb_transport::{PlatformUsbBackend, UsbBackend};

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let request_ptp_probe = arguments.iter().any(|argument| argument == "ptp-info");
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
    {
        eprintln!("Usage: fuji-test ptp-descriptor D18C | ptp-read D18C | ptp-select-slot C1 | ptp-set-name C1 FRM-TEST | ptp-test-name C2 FRM-TEST | ptp-test-dynamic-range C2 DR100 | ptp-test-signed C2 D19D 0 | ptp-test-u16 C2 D196 1");
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

fn parse_property_code(value: &str) -> Option<u16> {
    let normalized = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or(value.trim());
    u16::from_str_radix(normalized, 16).ok()
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
