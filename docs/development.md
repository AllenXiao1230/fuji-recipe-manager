# Development notes

## Hardware safety boundary

Phase 1 contains one experimental write path: X-M5 firmware 1.30 through USB ID `04CB:030C`. Before it sends a property, the app saves a durable SQLite snapshot and write journal; it reads each property back and verifies rollback if any write fails. No other model may call `SetDevicePropValue` until it has its own capability record and hardware checklist.

## macOS PTP interface ownership

macOS may allow the system photo-import service, Image Capture, Photos, X RAW Studio, or another tethering app to claim a camera's PTP interface before this app can open it. A `could not be opened for exclusive access` result is an environmental contention error, not permission to detach or force-control the camera. Close those applications, disconnect the camera, reconnect it directly, and retry the read-only probe. The application must never forcibly detach a macOS owner because this can interrupt an active import or tethering session.

## X-M5 validation checklist

1. Set the camera's USB mode to its documented PTP/tethering-compatible mode and connect it directly by USB-C.
2. Run `cargo run -p fuji-test` on macOS and Windows.
3. Record the displayed vendor/product ID and confirm the product ID against at least two physical-device observations or an authoritative source. The first observed X-M5 is `04CB:030C`; retain the raw discovery output in local test notes.
4. Add that ID to `camera-xm5::X_M5_USB_IDS`. The first physical X-M5 has passed the standard DeviceInfo probe as `FUJIFILM X-M5`, firmware `1.30`, PTP response `0x2001`; retain `Experimental` until the complete multi-field app workflow and Windows recovery are validated.
5. Implement read-only PTP property descriptors, then save anonymized logs as test fixtures. `fuji-test ptp-descriptor D18C` now opens and closes a standard PTP session to inspect one descriptor only; it must remain opt-in and must never be used to infer that a write is safe.
6. Preserve a user-visible backup list, write journal, and explicit restore action. A rollback is not successful unless each restored property is read back and matches its captured representation.

## Transport design

`UsbBackend` deliberately returns neutral `UsbDevice` records. Camera recognition happens in `camera-core`; PTP operations belong above the transport layer. This lets a future Windows-specific driver backend or a macOS entitlement-aware backend replace only transport code.

## Suggested verification commands

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run build
```

For a physical X-M5, the next safe manual check is:

```bash
cargo run -p fuji-test -- ptp-descriptor D18C
```

This reads the descriptor metadata for the custom-slot selector. It can reveal the camera's current selected-slot scalar as part of the standard descriptor dataset, but it does not select a slot, read C1-C4 recipes, or write a setting. Do not run property probes while another camera application is connected.

On the physical X-M5 firmware 1.30, `GetDevicePropDesc(D18C)` returns `0x2002` after an empty data phase. `GetDevicePropValue(D18C)` succeeds and returns two raw bytes (`01 00`). Treat that as a transport observation only: the value encoding and any relationship to a custom slot still need controlled, reversible calibration before being shown as C1-C4 data.

On a physical device, preserve the raw `fuji-test` output with the serial number removed before sharing it in an issue.
