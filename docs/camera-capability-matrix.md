# Camera capability matrix

This matrix prevents a USB product name from being mistaken for write support.
Every Fujifilm USB device is discoverable; only a model and firmware record that
has completed the listed gates may expose a Recipe write action.

| Support level | Meaning | UI behaviour |
|---|---|---|
| Probe required | Fujifilm device recognised, but PTP property encoding is unknown. | Discovery and explicit read-only identity probe only. |
| Read-only validated | Standard PTP probe and selected raw-property reads recorded. | Read-only camera inspection. |
| Experimental write | Model, USB ID, firmware and field codec have a reversible hardware record. | Explicit write confirmation, durable backup, journal, read-back and restore only. |
| Production write | Full multi-slot hardware matrix, Windows/macOS recovery, release QA and documentation complete. | Normal write workflow. |

## Current records

| Family | Models recognised | Highest current support |
|---|---|---|
| X Series | X-M5, X-S10, X-S20, X-T3, X-T4, X-T5, X-T30, X-T50, X-H1, X-H2, X-H2S, X-Pro3, X-E3, X-E4 and unknown X bodies | X-M5 firmware 1.30: Experimental write; all other bodies: Probe required |
| X100 | X100F, X100V, X100VI and unknown X100 bodies | Probe required |
| GFX | GFX50S II, GFX100S, GFX100S II, GFX100 II and unknown GFX bodies | Probe required |
| FinePix and other Fujifilm | Vendor-recognised devices not listed above | Probe required |

## Required record for a new model or firmware

1. Record product ID, PTP interface, DeviceInfo model and firmware without storing a serial number.
2. Store sanitised `GetDevicePropValue` fixtures for custom-slot selection, preset name and every candidate Recipe property.
3. Define the model codec in a dedicated crate: user value, write command bytes, accepted read-back bytes, and restore command bytes.
4. Add unit tests for normal values, rejected values, and every canonicalisation exception.
5. On a disposable slot, run backup → write → read-back → restore → read-back on macOS and Windows.
6. Mark the record Experimental write only after the exact model/firmware test is retained. Production write additionally requires repeated multi-slot QA and release documentation.

## Compatibility policy

Recipe import is always local. A Recipe may be edited on any machine, but the
app must show unsupported fields and refuse the camera write until the connected
model/firmware record supports every selected field. New camera firmware starts
at Probe required even when the USB product ID is unchanged.
